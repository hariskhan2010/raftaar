//! Thin Tauri layer: every command delegates to the `engine` crate.

mod commands;

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use engine::boost::{BoostConfig, Booster};
use engine::license::{Licensing, UreqHttp};
use engine::monitor::Monitor;
use engine::snapshot::{SnapshotStore, now_unix};
use engine::sys::{RealShell, WinRegistry};
use engine::tweak::{Engine, PcClass};
use tauri::{Emitter, Manager};

static REG: WinRegistry = WinRegistry;
static SHELL: RealShell = RealShell;
static HTTP: UreqHttp = UreqHttp;

/// Lemon Squeezy store whose license keys unlock Premium (card payments).
/// Set it once the store exists: Lemon Squeezy dashboard → Settings → Stores → ID.
/// While it is None, only Raftaar keys (JazzCash / Easypaisa sales) are accepted.
pub const LEMON_SQUEEZY_STORE_ID: Option<u64> = None;

pub struct AppState {
    pub data_dir: PathBuf,
    pub engine: Mutex<Engine<'static>>,
    /// A restore point is created once per app session, before the first tweak.
    pub restore_done: Mutex<bool>,
    pub boost_config: Mutex<BoostConfig>,
    pub booster: Mutex<Booster>,
    pub monitor: Mutex<Monitor>,
    /// Set by the first scan; presets depend on it.
    pub pc_class: Mutex<Option<PcClass>>,
    /// Serialises FPS tests: PresentMon allows one session per name.
    pub fps_busy: AtomicBool,
    pub licensing: Mutex<Licensing>,
}

impl AppState {
    fn new(data_dir: PathBuf) -> engine::Result<Self> {
        let store = SnapshotStore::load(data_dir.join("snapshots.json"))?;
        let boost_config = std::fs::read_to_string(data_dir.join("boost-config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Ok(AppState {
            engine: Mutex::new(Engine::new(&REG, &SHELL, store)),
            restore_done: Mutex::new(false),
            boost_config: Mutex::new(boost_config),
            booster: Mutex::new(Booster::load(data_dir.join("boost-state.json"))),
            monitor: Mutex::new(Monitor::new()),
            pc_class: Mutex::new(None),
            fps_busy: AtomicBool::new(false),
            licensing: Mutex::new(Licensing::load(data_dir.join("license.json"), now_unix(), LEMON_SQUEEZY_STORE_ID)?),
            data_dir,
        })
    }

    pub fn premium(&self) -> bool {
        self.licensing.lock().unwrap().status(now_unix()).is_premium()
    }
}

fn spawn_boost_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut timer_on = false;
        loop {
            std::thread::sleep(Duration::from_secs(3));
            let state = app.state::<AppState>();
            let mut cfg = state.boost_config.lock().unwrap().clone();
            // Game Boost is Premium. When a trial or license ends mid-game, the
            // next tick ends the boost and reopens the closed apps as usual.
            cfg.enabled &= state.premium();
            let mut booster = state.booster.lock().unwrap();
            let event = booster.tick(&cfg, &engine::boost::WinProcessOps);
            // Hold a 1 ms timer exactly while a boosted game runs.
            let gaming = booster.state.game_pid.is_some();
            drop(booster);
            if gaming != timer_on {
                engine::boost::high_res_timer(gaming);
                timer_on = gaming;
            }
            if let Ok(Some(ev)) = event {
                let _ = app.emit("boost-event", ev);
            }
        }
    });
}

/// Re-checks a card-payment license in the background, at start and then daily.
fn spawn_license_check(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            let state = app.state::<AppState>();
            let _ = state.licensing.lock().unwrap().revalidate(&HTTP, now_unix());
            let _ = app.emit("license-changed", ());
            std::thread::sleep(Duration::from_secs(24 * 3600));
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState::new(dir)?);
            spawn_boost_watcher(app.handle().clone());
            spawn_license_check(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::is_admin,
            commands::system_scan,
            commands::load_score,
            commands::list_tweaks,
            commands::preset_ids,
            commands::apply_tweaks,
            commands::enable_protection,
            commands::revert_tweak,
            commands::revert_all,
            commands::boost_get,
            commands::boost_set,
            commands::running_apps,
            commands::detect_games,
            commands::cleanup_scan,
            commands::cleanup_run,
            commands::startup_list,
            commands::startup_set,
            commands::monitor_sample,
            commands::fps_record,
            commands::fps_history,
            commands::fps_delete,
            commands::restore_points,
            commands::create_restore_point,
            commands::open_system_restore,
            commands::license_status,
            commands::license_activate,
            commands::license_remove,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Raftaar");
}
