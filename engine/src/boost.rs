//! Game Boost: while a chosen game runs, raise its priority and close the
//! apps the user picked; reopen them when the game exits.
//!
//! State is saved to disk on every change, so if Raftaar crashes or the PC
//! restarts mid-game, the next tick still reopens the closed apps.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

/// Processes Raftaar will never close, whatever the user configures.
pub const DENY_LIST: &[&str] = &[
    "system", "registry", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe",
    "lsass.exe", "svchost.exe", "dwm.exe", "explorer.exe", "fontdrvhost.exe", "sihost.exe",
    "ctfmon.exe", "audiodg.exe", "msmpeng.exe", "securityhealthservice.exe", "raftaar.exe",
];

pub fn is_denied(name: &str) -> bool {
    DENY_LIST.iter().any(|d| d.eq_ignore_ascii_case(name))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoostConfig {
    pub enabled: bool,
    /// Game executable file names, e.g. "valorant.exe".
    pub games: Vec<String>,
    /// Executable file names to close while a game runs, e.g. "discord.exe".
    pub close_apps: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoostState {
    pub game_pid: Option<u32>,
    pub game_name: Option<String>,
    /// Full paths of closed apps, reopened when the game exits.
    pub closed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
}

pub trait ProcessOps {
    fn list(&self) -> Vec<Proc>;
    fn set_priority_high(&self, pid: u32) -> Result<()>;
    fn set_priority_normal(&self, pid: u32) -> Result<()>;
    fn kill(&self, pid: u32) -> Result<()>;
    fn launch(&self, path: &str) -> Result<()>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum BoostEvent {
    Started { game: String, closed: usize },
    Ended { game: String, reopened: usize },
}

pub struct Booster {
    path: PathBuf,
    pub state: BoostState,
}

impl Booster {
    pub fn load(path: PathBuf) -> Self {
        let state = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Booster { path, state }
    }

    fn save(&self) -> Result<()> {
        if let Some(d) = self.path.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&self.path, serde_json::to_vec(&self.state)?)?;
        Ok(())
    }

    pub fn tick(&mut self, cfg: &BoostConfig, ops: &dyn ProcessOps) -> Result<Option<BoostEvent>> {
        let procs = ops.list();
        if let Some(pid) = self.state.game_pid {
            if procs.iter().any(|p| p.pid == pid) {
                return Ok(None);
            }
            return self.end(ops).map(Some);
        }
        if !cfg.enabled {
            return Ok(None);
        }
        let Some(game) = procs.iter().find(|p| cfg.games.iter().any(|g| g.eq_ignore_ascii_case(&p.name))) else {
            return Ok(None);
        };
        let _ = ops.set_priority_high(game.pid);
        self.state.game_pid = Some(game.pid);
        self.state.game_name = Some(game.name.clone());
        self.save()?;

        for p in &procs {
            let wanted = cfg.close_apps.iter().any(|a| a.eq_ignore_ascii_case(&p.name));
            if !wanted || is_denied(&p.name) || p.pid == game.pid {
                continue;
            }
            if ops.kill(p.pid).is_ok()
                && let Some(path) = &p.path {
                    // Apps like Discord run many processes; reopen once.
                    if !self.state.closed.iter().any(|c| c.eq_ignore_ascii_case(path)) {
                        self.state.closed.push(path.clone());
                    }
                }
        }
        self.save()?;
        Ok(Some(BoostEvent::Started { game: game.name.clone(), closed: self.state.closed.len() }))
    }

    fn end(&mut self, ops: &dyn ProcessOps) -> Result<BoostEvent> {
        let reopened = self.state.closed.iter().filter(|p| ops.launch(p).is_ok()).count();
        let game = self.state.game_name.take().unwrap_or_default();
        self.state = BoostState::default();
        self.save()?;
        Ok(BoostEvent::Ended { game, reopened })
    }

    /// User turned Boost off mid-game: give the game normal priority back and reopen apps.
    pub fn stop(&mut self, ops: &dyn ProcessOps) -> Result<Option<BoostEvent>> {
        let Some(pid) = self.state.game_pid else { return Ok(None) };
        let _ = ops.set_priority_normal(pid);
        self.end(ops).map(Some)
    }
}

/// Asks Windows for a 1 ms timer while a game runs, for smoother frame pacing.
/// Calls must be paired; the caller tracks on/off so a request is never leaked.
#[cfg(windows)]
pub fn high_res_timer(on: bool) {
    use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
    unsafe {
        if on {
            timeBeginPeriod(1);
        } else {
            timeEndPeriod(1);
        }
    }
}

/// Real Windows process operations.
#[cfg(windows)]
pub struct WinProcessOps;

#[cfg(windows)]
impl WinProcessOps {
    fn set_class(pid: u32, class: u32) -> Result<()> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_INFORMATION, SetPriorityClass};
        unsafe {
            let h = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
            if h.is_null() {
                return Err(crate::Error(format!("cannot open process {pid}")));
            }
            let ok = SetPriorityClass(h, class);
            CloseHandle(h);
            if ok == 0 { Err(crate::Error(format!("cannot set priority of {pid}"))) } else { Ok(()) }
        }
    }
}

#[cfg(windows)]
impl ProcessOps for WinProcessOps {
    fn list(&self) -> Vec<Proc> {
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.processes()
            .iter()
            .map(|(pid, p)| Proc {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                path: p.exe().map(|e| e.to_string_lossy().into_owned()),
            })
            .collect()
    }

    fn set_priority_high(&self, pid: u32) -> Result<()> {
        // HIGH, never REALTIME: realtime can starve mouse and keyboard input.
        Self::set_class(pid, windows_sys::Win32::System::Threading::HIGH_PRIORITY_CLASS)
    }

    fn set_priority_normal(&self, pid: u32) -> Result<()> {
        Self::set_class(pid, windows_sys::Win32::System::Threading::NORMAL_PRIORITY_CLASS)
    }

    fn kill(&self, pid: u32) -> Result<()> {
        let mut sys = sysinfo::System::new();
        let spid = sysinfo::Pid::from_u32(pid);
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[spid]), true);
        match sys.process(spid) {
            Some(p) if p.kill() => Ok(()),
            _ => Err(crate::Error(format!("cannot close process {pid}"))),
        }
    }

    fn launch(&self, path: &str) -> Result<()> {
        // Raftaar runs as admin. Launching through explorer starts the app as
        // the normal user instead of handing it admin rights.
        std::process::Command::new("explorer.exe").arg(path).spawn()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeOps {
        procs: RefCell<Vec<Proc>>,
        killed: RefCell<Vec<u32>>,
        launched: RefCell<Vec<String>>,
        high: RefCell<Vec<u32>>,
    }

    impl FakeOps {
        fn with(procs: &[(u32, &str)]) -> Self {
            let f = FakeOps::default();
            *f.procs.borrow_mut() = procs
                .iter()
                .map(|(pid, n)| Proc { pid: *pid, name: n.to_string(), path: Some(format!(r"C:\Apps\{n}")) })
                .collect();
            f
        }
    }

    impl ProcessOps for FakeOps {
        fn list(&self) -> Vec<Proc> {
            self.procs.borrow().clone()
        }
        fn set_priority_high(&self, pid: u32) -> Result<()> {
            self.high.borrow_mut().push(pid);
            Ok(())
        }
        fn set_priority_normal(&self, _: u32) -> Result<()> {
            Ok(())
        }
        fn kill(&self, pid: u32) -> Result<()> {
            self.killed.borrow_mut().push(pid);
            self.procs.borrow_mut().retain(|p| p.pid != pid);
            Ok(())
        }
        fn launch(&self, path: &str) -> Result<()> {
            self.launched.borrow_mut().push(path.into());
            Ok(())
        }
    }

    fn cfg() -> BoostConfig {
        BoostConfig {
            enabled: true,
            games: vec!["Game.exe".into()],
            close_apps: vec!["discord.exe".into(), "explorer.exe".into()],
        }
    }

    #[test]
    fn denylisted_process_is_never_closed() {
        let d = tempfile::tempdir().unwrap();
        let ops = FakeOps::with(&[(1, "game.exe"), (2, "explorer.exe"), (3, "discord.exe")]);
        let mut b = Booster::load(d.path().join("b.json"));
        b.tick(&cfg(), &ops).unwrap();
        assert_eq!(*ops.killed.borrow(), vec![3]);
        assert_eq!(*ops.high.borrow(), vec![1]);
    }

    #[test]
    fn closed_apps_reopened_once_on_game_exit() {
        let d = tempfile::tempdir().unwrap();
        let ops = FakeOps::with(&[(1, "game.exe"), (3, "discord.exe"), (4, "Discord.exe")]);
        let mut b = Booster::load(d.path().join("b.json"));
        let ev = b.tick(&cfg(), &ops).unwrap();
        assert_eq!(ev, Some(BoostEvent::Started { game: "game.exe".into(), closed: 1 }));
        assert_eq!(b.tick(&cfg(), &ops).unwrap(), None); // still running
        ops.procs.borrow_mut().clear(); // game exits
        let ev = b.tick(&cfg(), &ops).unwrap();
        assert_eq!(ev, Some(BoostEvent::Ended { game: "game.exe".into(), reopened: 1 }));
        assert_eq!(*ops.launched.borrow(), vec![r"C:\Apps\discord.exe".to_string()]);
    }

    #[test]
    fn state_file_restores_apps_after_crash() {
        let d = tempfile::tempdir().unwrap();
        let ops = FakeOps::with(&[(1, "game.exe"), (3, "discord.exe")]);
        let mut b = Booster::load(d.path().join("b.json"));
        b.tick(&cfg(), &ops).unwrap();
        drop(b); // Raftaar crashes mid-game
        ops.procs.borrow_mut().clear(); // game ended while we were dead
        let mut b2 = Booster::load(d.path().join("b.json"));
        let ev = b2.tick(&BoostConfig::default(), &ops).unwrap(); // even with boost now disabled
        assert!(matches!(ev, Some(BoostEvent::Ended { reopened: 1, .. })));
    }

    #[test]
    fn disabled_config_does_nothing() {
        let d = tempfile::tempdir().unwrap();
        let ops = FakeOps::with(&[(1, "game.exe"), (3, "discord.exe")]);
        let mut b = Booster::load(d.path().join("b.json"));
        assert_eq!(b.tick(&BoostConfig { enabled: false, ..cfg() }, &ops).unwrap(), None);
        assert!(ops.killed.borrow().is_empty());
    }
}
