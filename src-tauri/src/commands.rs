use std::path::PathBuf;
use std::sync::atomic::Ordering;

use engine::boost::{BoostConfig, BoostState, ProcessOps, WinProcessOps};
use engine::catalog::catalog;
use engine::cleanup::{Category as CleanCategory, CategoryReport};
use engine::fps::FpsResult;
use engine::games::Game;
use engine::monitor::Stats;
use engine::restore::{RestorePoint, RestoreResult};
use engine::scan::SystemInfo;
use engine::score::LoadSample;
use engine::startup::{Source, StartupItem};
use engine::tweak::{Category, PcClass, Preset, Risk};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

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
pub async fn system_scan(state: State<'_, AppState>) -> CmdResult<SystemInfo> {
    let info = blocking(engine::scan::scan).await?;
    *state.pc_class.lock().unwrap() = Some(info.class);
    Ok(info)
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
    category: Category,
    name_en: String,
    name_ur: String,
    desc_en: String,
    desc_ur: String,
    risk: Risk,
    needs_restart: bool,
    recommended_for: Vec<PcClass>,
    preset: Option<Preset>,
    applied: bool,
    applied_at: Option<u64>,
}

#[tauri::command]
pub fn list_tweaks(state: State<'_, AppState>) -> Vec<TweakView> {
    let engine = state.engine.lock().unwrap();
    catalog()
        .into_iter()
        .map(|t| {
            let entry = engine.store.get(&t.id);
            TweakView {
                applied: entry.is_some(),
                applied_at: entry.and_then(|e| e.applied_at),
                id: t.id,
                category: t.category,
                name_en: t.name_en,
                name_ur: t.name_ur,
                desc_en: t.desc_en,
                desc_ur: t.desc_ur,
                risk: t.risk,
                needs_restart: t.needs_restart,
                recommended_for: t.recommended_for,
                preset: t.preset,
            }
        })
        .collect()
}

#[tauri::command]
pub fn preset_ids(state: State<'_, AppState>, preset: Preset) -> Vec<String> {
    let class = state.pc_class.lock().unwrap().unwrap_or(PcClass::Mid);
    engine::catalog::preset_ids(preset, class)
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
        state.booster.lock().unwrap().stop(&WinProcessOps).map_err(|e| e.0)?;
    }
    let json = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(state.data_dir.join("boost-config.json"), json).map_err(|e| e.to_string())?;
    *state.boost_config.lock().unwrap() = config;
    Ok(())
}

fn running_names() -> Vec<String> {
    let mut names: Vec<String> = WinProcessOps.list().into_iter().map(|p| p.name.to_lowercase()).collect();
    names.sort();
    names.dedup();
    names
}

/// Unique exe names of running user apps, for the Game Boost pickers.
#[tauri::command]
pub async fn running_apps() -> CmdResult<Vec<String>> {
    blocking(|| {
        let mut names: Vec<String> = WinProcessOps
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
pub async fn detect_games() -> CmdResult<Vec<Game>> {
    blocking(engine::games::detect).await
}

/// Dry run of one category. The UI scans all categories in parallel and fills
/// each row as it finishes, since a big Temp folder can take a while.
#[tauri::command]
pub async fn cleanup_scan(category: CleanCategory) -> CmdResult<CategoryReport> {
    blocking(move || engine::cleanup::clean_category(category, &running_names(), true)).await
}

#[tauri::command]
pub async fn cleanup_run(categories: Vec<CleanCategory>) -> CmdResult<Vec<CategoryReport>> {
    blocking(move || {
        let running = running_names();
        categories.iter().map(|c| engine::cleanup::clean_category(*c, &running, false)).collect()
    })
    .await
}

#[tauri::command]
pub async fn startup_list() -> CmdResult<Vec<StartupItem>> {
    blocking(|| {
        let (user, machine) = engine::startup::default_folders();
        engine::startup::list(&REG, user.as_deref(), machine.as_deref())
    })
    .await?
    .map_err(|e| e.0)
}

#[tauri::command]
pub fn startup_set(source: Source, name: String, enabled: bool) -> CmdResult<()> {
    if source.needs_admin() && !engine::sys::is_elevated() {
        return Err("Needs administrator rights.".into());
    }
    engine::startup::set_enabled(&REG, source, &name, enabled).map_err(|e| e.0)
}

#[tauri::command]
pub async fn monitor_sample(app: AppHandle) -> CmdResult<Stats> {
    blocking(move || app.state::<AppState>().monitor.lock().unwrap().sample()).await
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FpsRecord {
    id: u64,
    game: String,
    label: String,
    at: u64,
    result: FpsResult,
}

fn fps_file(state: &AppState) -> PathBuf {
    state.data_dir.join("fps-results.json")
}

fn load_fps(state: &AppState) -> Vec<FpsRecord> {
    std::fs::read_to_string(fps_file(state)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_fps(state: &AppState, records: &[FpsRecord]) -> CmdResult<()> {
    let json = serde_json::to_vec_pretty(records).map_err(|e| e.to_string())?;
    std::fs::write(fps_file(state), json).map_err(|e| e.to_string())
}

/// PresentMon ships next to raftaar.exe as a Tauri sidecar.
fn presentmon_path() -> CmdResult<PathBuf> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let p = exe.with_file_name("presentmon.exe");
    if p.exists() { Ok(p) } else { Err("PresentMon is missing from the Raftaar folder. Reinstall Raftaar.".into()) }
}

#[tauri::command]
pub async fn fps_record(app: AppHandle, game: String, seconds: u32, label: String) -> CmdResult<FpsRecord> {
    if !engine::sys::is_elevated() {
        return Err("The FPS Test needs Raftaar to run as administrator.".into());
    }
    let state = app.state::<AppState>();
    if state.fps_busy.swap(true, Ordering::SeqCst) {
        return Err("An FPS test is already running.".into());
    }
    let pm = presentmon_path();
    let csv = state.data_dir.join("fps-last.csv");
    let game2 = game.clone();
    let result = match pm {
        Ok(pm) => blocking(move || engine::fps::record(&SHELL, &pm, &game2, seconds, &csv)).await,
        Err(e) => Err(e),
    };
    state.fps_busy.store(false, Ordering::SeqCst);
    let result = result?.map_err(|e| e.0)?;
    let at = engine::snapshot::now_unix();
    let rec = FpsRecord { id: at * 1000 + u64::from(result.frames % 1000), game, label, at, result };
    let mut all = load_fps(&state);
    all.insert(0, rec.clone());
    all.truncate(50);
    save_fps(&state, &all)?;
    Ok(rec)
}

#[tauri::command]
pub fn fps_history(state: State<'_, AppState>) -> Vec<FpsRecord> {
    load_fps(&state)
}

#[tauri::command]
pub fn fps_delete(state: State<'_, AppState>, id: u64) -> CmdResult<()> {
    let mut all = load_fps(&state);
    all.retain(|r| r.id != id);
    save_fps(&state, &all)
}

#[tauri::command]
pub async fn restore_points() -> CmdResult<Vec<RestorePoint>> {
    blocking(|| engine::restore::list_restore_points(&SHELL)).await?.map_err(|e| e.0)
}

#[tauri::command]
pub async fn create_restore_point() -> CmdResult<RestoreResult> {
    blocking(|| engine::restore::create_restore_point(&SHELL, &REG, "Raftaar manual restore point")).await
}

#[tauri::command]
pub fn open_system_restore() -> CmdResult<()> {
    std::process::Command::new("rstrui.exe").spawn().map(|_| ()).map_err(|e| e.to_string())
}
