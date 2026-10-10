use engine::boost::{BoostConfig, BoostState};
use engine::catalog::catalog;
use engine::cleanup::CleanupReport;
use engine::restore::RestoreResult;
use engine::scan::SystemInfo;
use engine::score::LoadSample;
use engine::tweak::{PcClass, Risk};
use serde::Serialize;
use tauri::State;

use crate::{AppState, REG, SHELL};

type CmdResult<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_admin() -> bool {
    engine::sys::is_elevated()
}

#[tauri::command]
pub async fn system_scan() -> CmdResult<SystemInfo> {
    blocking(engine::scan::scan).await
}

#[derive(Serialize)]
pub struct Score {
    sample: LoadSample,
    score: u8,
}

#[tauri::command]
pub async fn load_score() -> CmdResult<Score> {
    blocking(|| {
        let sample = engine::score::measure();
        Score { score: engine::score::score(&sample), sample }
    })
    .await
}

#[derive(Serialize)]
pub struct TweakView {
    id: String,
    name_en: String,
    name_ur: String,
    desc_en: String,
    desc_ur: String,
    risk: Risk,
    needs_restart: bool,
    recommended_for: Vec<PcClass>,
    applied: bool,
}

#[tauri::command]
pub fn list_tweaks(state: State<'_, AppState>) -> Vec<TweakView> {
    let engine = state.engine.lock().unwrap();
    catalog()
        .into_iter()
        .map(|t| TweakView {
            applied: engine.is_applied(&t.id),
            id: t.id,
            name_en: t.name_en,
            name_ur: t.name_ur,
            desc_en: t.desc_en,
            desc_ur: t.desc_ur,
            risk: t.risk,
            needs_restart: t.needs_restart,
            recommended_for: t.recommended_for,
        })
        .collect()
}

#[derive(Serialize)]
pub struct ApplyResult {
    /// Set when a restore point was attempted. If it isn't `Created`, nothing
    /// was applied and the UI must ask the user how to continue.
    restore: Option<RestoreResult>,
    applied: Vec<String>,
    failed: Vec<(String, String)>,
}

#[tauri::command]
pub async fn apply_tweaks(state: State<'_, AppState>, ids: Vec<String>, skip_restore: bool) -> CmdResult<ApplyResult> {
    let mut restore = None;
    let need_restore = !*state.restore_done.lock().unwrap() && !skip_restore;
    if need_restore {
        let r = blocking(|| engine::restore::create_restore_point(&SHELL, &REG, "Raftaar before optimizing")).await?;
        if r != RestoreResult::Created {
            return Ok(ApplyResult { restore: Some(r), applied: vec![], failed: vec![] });
        }
        *state.restore_done.lock().unwrap() = true;
        restore = Some(r);
    }

    let mut engine = state.engine.lock().unwrap();
    let (mut applied, mut failed) = (vec![], vec![]);
    for t in catalog().into_iter().filter(|t| ids.contains(&t.id)) {
        match engine.apply(&t) {
            Ok(()) => applied.push(t.id),
            Err(e) => failed.push((t.id, e.0)),
        }
    }
    Ok(ApplyResult { restore, applied, failed })
}

#[tauri::command]
pub async fn enable_protection() -> CmdResult<()> {
    blocking(|| engine::restore::enable_protection(&SHELL)).await?.map_err(|e| e.0)
}

#[tauri::command]
pub fn revert_tweak(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    state.engine.lock().unwrap().revert(&id).map_err(|e| e.0)
}

#[tauri::command]
pub fn revert_all(state: State<'_, AppState>) -> CmdResult<()> {
    state.engine.lock().unwrap().revert_all().map_err(|e| e.0)
}

#[derive(Serialize)]
pub struct BoostView {
    config: BoostConfig,
    state: BoostState,
}

#[tauri::command]
pub fn boost_get(state: State<'_, AppState>) -> BoostView {
    BoostView {
        config: state.boost_config.lock().unwrap().clone(),
        state: state.booster.lock().unwrap().state.clone(),
    }
}

#[tauri::command]
pub fn boost_set(state: State<'_, AppState>, config: BoostConfig) -> CmdResult<()> {
    if !config.enabled {
        state.booster.lock().unwrap().stop(&engine::boost::WinProcessOps).map_err(|e| e.0)?;
    }
    let json = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(state.data_dir.join("boost-config.json"), json).map_err(|e| e.to_string())?;
    *state.boost_config.lock().unwrap() = config;
    Ok(())
}

/// Unique exe names of running user apps, for the Game Boost pickers.
#[tauri::command]
pub async fn running_apps() -> CmdResult<Vec<String>> {
    use engine::boost::ProcessOps;
    blocking(|| {
        let mut names: Vec<String> = engine::boost::WinProcessOps
            .list()
            .into_iter()
            .filter(|p| p.path.is_some() && !engine::boost::is_denied(&p.name))
            .map(|p| p.name.to_lowercase())
            .collect();
        names.sort();
        names.dedup();
        names
    })
    .await
}

#[tauri::command]
pub async fn cleanup(dry_run: bool) -> CmdResult<CleanupReport> {
    blocking(move || {
        engine::cleanup::clean(&engine::cleanup::default_dirs(), std::time::Duration::from_secs(24 * 3600), dry_run)
    })
    .await
}
