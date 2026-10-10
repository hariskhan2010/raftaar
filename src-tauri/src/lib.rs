//! Thin Tauri layer: every command delegates to the `engine` crate.

mod commands;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use engine::boost::{BoostConfig, Booster};
use engine::snapshot::SnapshotStore;
use engine::sys::{RealShell, WinRegistry};
use engine::tweak::Engine;
use tauri::{Emitter, Manager};

static REG: WinRegistry = WinRegistry;
static SHELL: RealShell = RealShell;

pub struct AppState {
    pub data_dir: PathBuf,
    pub engine: Mutex<Engine<'static>>,
    /// A restore point is created once per app session, before the first tweak.
    pub restore_done: Mutex<bool>,
    pub boost_config: Mutex<BoostConfig>,
    pub booster: Mutex<Booster>,
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
            data_dir,
        })
    }
}

fn spawn_boost_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(3));
            let state = app.state::<AppState>();
            let cfg = state.boost_config.lock().unwrap().clone();
            let event = state.booster.lock().unwrap().tick(&cfg, &engine::boost::WinProcessOps);
            if let Ok(Some(ev)) = event {
                let _ = app.emit("boost-event", ev);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState::new(dir)?);
            spawn_boost_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::is_admin,
            commands::system_scan,
            commands::load_score,
            commands::list_tweaks,
            commands::apply_tweaks,
            commands::enable_protection,
            commands::revert_tweak,
            commands::revert_all,
            commands::boost_get,
            commands::boost_set,
            commands::running_apps,
            commands::cleanup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Raftaar");
}
