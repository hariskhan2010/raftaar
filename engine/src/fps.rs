//! FPS Test: records a running game with Intel PresentMon and reports average
//! FPS and 1% / 0.1% lows. These are measured numbers, never estimates.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sys::Shell;
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpsResult {
    pub avg_fps: f32,
    /// FPS of the slowest 1% of frames (99th-percentile frame time).
    pub low1_fps: f32,
    pub low01_fps: f32,
    pub frames: u32,
    pub seconds: f32,
    /// Frame times in ms, thinned to at most 240 points for the chart.
    pub frame_times: Vec<f32>,
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    let idx = ((sorted.len() as f32 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Parses PresentMon `--v1_metrics` CSV. Uses the swap chain with the most
/// frames, since launchers and overlays present on their own swap chains.
pub fn parse_csv(text: &str, exe: &str) -> Option<FpsResult> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let col = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let (app, chain, ms) = (col("Application")?, col("SwapChainAddress")?, col("MsBetweenPresents")?);

    let mut chains: std::collections::HashMap<&str, Vec<f32>> = Default::default();
    for line in lines {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() <= ms.max(app).max(chain) || !f[app].trim().eq_ignore_ascii_case(exe) {
            continue;
        }
        if let Ok(v) = f[ms].trim().parse::<f32>()
            && v > 0.0
            && v < 1000.0
        {
            chains.entry(f[chain]).or_default().push(v);
        }
    }
    let times = chains.into_values().max_by_key(|v| v.len())?;
    if times.len() < 10 {
        return None;
    }
    let total_ms: f32 = times.iter().sum();
    let mut sorted = times.clone();
    sorted.sort_by(f32::total_cmp);
    let step = times.len().div_ceil(240);
    Some(FpsResult {
        avg_fps: 1000.0 * times.len() as f32 / total_ms,
        low1_fps: 1000.0 / percentile(&sorted, 0.99),
        low01_fps: 1000.0 / percentile(&sorted, 0.999),
        frames: times.len() as u32,
        seconds: total_ms / 1000.0,
        frame_times: times.chunks(step).map(|c| c.iter().copied().fold(0.0, f32::max)).collect(),
    })
}

/// Records `exe` for `seconds`. Blocks until PresentMon finishes. Needs administrator rights.
pub fn record(shell: &dyn Shell, presentmon: &Path, exe: &str, seconds: u32, csv: &Path) -> Result<FpsResult> {
    if !exe.to_lowercase().ends_with(".exe") || exe.contains(['\\', '/', '"']) {
        return Err(Error::new("not an exe name"));
    }
    let _ = std::fs::remove_file(csv);
    let secs = seconds.clamp(5, 300).to_string();
    let csv_s = csv.display().to_string();
    let pm = presentmon.display().to_string();
    let out = shell.run(
        &pm,
        &[
            "--process_name",
            exe,
            "--output_file",
            &csv_s,
            "--timed",
            &secs,
            "--terminate_after_timed",
            "--v1_metrics",
            "--no_console_stats",
            "--stop_existing_session",
            "--session_name",
            "RaftaarFpsTest",
        ],
    )?;
    let text = std::fs::read_to_string(csv).map_err(|_| {
        Error(format!("PresentMon wrote no data ({}). Is the game running?", out.stderr.trim()))
    })?;
    parse_csv(&text, exe).ok_or_else(|| Error::new("No frames were recorded. Keep the game in front and playing during the test."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::MockShell;

    fn csv(rows: &[(&str, &str, f32)]) -> String {
        let mut s = String::from("Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,MsBetweenPresents\n");
        for (app, chain, ms) in rows {
            s += &format!("{app},1,{chain},DXGI,0,0,{ms}\n");
        }
        s
    }

    #[test]
    fn steady_60_fps_with_two_spikes() {
        let mut rows = vec![("game.exe", "0xA", 16.6667); 998];
        rows.extend([("game.exe", "0xA", 100.0); 2]);
        let r = parse_csv(&csv(&rows), "Game.exe").unwrap();
        assert_eq!(r.frames, 1000);
        // 998 x 16.67 ms + 2 x 100 ms = 16.83 s for 1000 frames.
        assert!((r.avg_fps - 59.4).abs() < 0.1, "{}", r.avg_fps);
        assert!((r.low1_fps - 60.0).abs() < 0.1);
        assert!((r.low01_fps - 10.0).abs() < 0.1);
        assert!(r.frame_times.len() <= 240);
        assert_eq!(r.frame_times.iter().copied().fold(0.0, f32::max), 100.0);
    }

    #[test]
    fn picks_the_busiest_swap_chain_and_ignores_other_apps() {
        let mut rows = vec![("game.exe", "0xOVERLAY", 33.3); 20];
        rows.extend(vec![("game.exe", "0xMAIN", 10.0); 100]);
        rows.extend(vec![("discord.exe", "0xD", 5.0); 500]);
        let r = parse_csv(&csv(&rows), "game.exe").unwrap();
        assert_eq!(r.frames, 100);
        assert!((r.avg_fps - 100.0).abs() < 0.1);
    }

    #[test]
    fn too_few_frames_is_none() {
        assert!(parse_csv(&csv(&[("game.exe", "0xA", 16.0); 5]), "game.exe").is_none());
        assert!(parse_csv("garbage", "game.exe").is_none());
    }

    #[test]
    fn record_rejects_odd_exe_names_and_passes_safe_args() {
        let shell = MockShell::new();
        let d = tempfile::tempdir().unwrap();
        assert!(record(&shell, Path::new("pm.exe"), "a\" & calc", 10, &d.path().join("o.csv")).is_err());
        assert!(shell.calls().is_empty());
        let err = record(&shell, Path::new("pm.exe"), "game.exe", 10, &d.path().join("o.csv")).unwrap_err();
        assert!(err.0.contains("no data"));
        assert!(shell.calls()[0].contains("--process_name game.exe"));
        assert!(shell.calls()[0].contains("--timed 10"));
    }
}
