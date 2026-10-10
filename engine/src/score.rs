//! System Load Score: how much of the PC is free for a game, 0-100.
//! Deliberately not an FPS number. Raftaar never claims FPS it didn't measure.

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LoadSample {
    pub ram_used_pct: f32,
    pub process_count: u32,
    pub cpu_busy_pct: f32,
    pub startup_apps: u32,
}

/// Starts at 100 and subtracts capped penalties:
/// - RAM: 0.8 per % used above 40% (max 40)
/// - processes: 0.25 per process above 80 (max 25)
/// - idle CPU load: 0.3 per % busy (max 20)
/// - startup apps: 1.5 each (max 15)
pub fn score(s: &LoadSample) -> u8 {
    let ram = ((s.ram_used_pct - 40.0).max(0.0) * 0.8).min(40.0);
    let procs = ((s.process_count as f32 - 80.0).max(0.0) * 0.25).min(25.0);
    let cpu = (s.cpu_busy_pct.max(0.0) * 0.3).min(20.0);
    let startup = (s.startup_apps as f32 * 1.5).min(15.0);
    (100.0 - ram - procs - cpu - startup).clamp(0.0, 100.0).round() as u8
}

/// Samples the PC for about two seconds.
#[cfg(windows)]
pub fn measure() -> LoadSample {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    let mut busy = 0.0;
    const SAMPLES: u32 = 4;
    for _ in 0..SAMPLES {
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(std::time::Duration::from_millis(500)));
        sys.refresh_cpu_usage();
        busy += sys.global_cpu_usage();
    }
    sys.refresh_memory();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    LoadSample {
        ram_used_pct: sys.used_memory() as f32 / sys.total_memory().max(1) as f32 * 100.0,
        process_count: sys.processes().len() as u32,
        cpu_busy_pct: busy / SAMPLES as f32,
        startup_apps: crate::scan::startup_app_count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(ram: f32, procs: u32, cpu: f32, startup: u32) -> LoadSample {
        LoadSample { ram_used_pct: ram, process_count: procs, cpu_busy_pct: cpu, startup_apps: startup }
    }

    #[test]
    fn idle_machine_scores_higher_than_busy_one() {
        assert!(score(&s(30.0, 70, 2.0, 2)) > score(&s(85.0, 250, 40.0, 12)));
    }

    #[test]
    fn clean_machine_is_near_100_and_worst_case_is_0() {
        assert_eq!(score(&s(20.0, 50, 0.0, 0)), 100);
        assert_eq!(score(&s(100.0, 1000, 100.0, 50)), 0);
    }
}
