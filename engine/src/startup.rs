//! Startup manager. Enables and disables startup programs the same way Task
//! Manager does: through the `StartupApproved` keys. The program's own Run
//! entry or shortcut is never touched, so turning an item back on always works.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::sys::{Hive, RegValue, Registry};
use crate::{Error, Result};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    UserRun,
    MachineRun,
    MachineRun32,
    UserFolder,
    MachineFolder,
}

impl Source {
    fn approved(self) -> (Hive, String) {
        let (hive, sub) = match self {
            Source::UserRun => (Hive::Hkcu, "Run"),
            Source::MachineRun => (Hive::Hklm, "Run"),
            Source::MachineRun32 => (Hive::Hklm, "Run32"),
            Source::UserFolder => (Hive::Hkcu, "StartupFolder"),
            Source::MachineFolder => (Hive::Hklm, "StartupFolder"),
        };
        (hive, format!(r"{APPROVED}\{sub}"))
    }

    /// Changing machine-wide items needs administrator rights.
    pub fn needs_admin(self) -> bool {
        matches!(self, Source::MachineRun | Source::MachineRun32 | Source::MachineFolder)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StartupItem {
    pub name: String,
    pub command: String,
    pub source: Source,
    pub enabled: bool,
}

/// Task Manager writes 02 for enabled and 03 for disabled in the first byte
/// (06/07 are seen too); a missing value means enabled.
fn is_enabled(v: &RegValue) -> bool {
    match v {
        RegValue::Binary(b) => b.first().is_none_or(|x| x & 1 == 0),
        _ => true,
    }
}

pub fn list(reg: &dyn Registry, user_folder: Option<&Path>, machine_folder: Option<&Path>) -> Result<Vec<StartupItem>> {
    let mut items = vec![];
    for (hive, path, source) in
        [(Hive::Hkcu, RUN, Source::UserRun), (Hive::Hklm, RUN, Source::MachineRun), (Hive::Hklm, RUN32, Source::MachineRun32)]
    {
        for (name, value) in reg.values(hive, path)? {
            let RegValue::Str(command) = value else { continue };
            items.push(StartupItem { name, command, source, enabled: true });
        }
    }
    for (dir, source) in [(user_folder, Source::UserFolder), (machine_folder, Source::MachineFolder)] {
        let Some(dir) = dir else { continue };
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            items.push(StartupItem { command: e.path().display().to_string(), name, source, enabled: true });
        }
    }
    for item in &mut items {
        let (hive, path) = item.source.approved();
        item.enabled = is_enabled(&reg.get(hive, &path, &item.name)?);
    }
    items.sort_by_key(|i| i.name.to_lowercase());
    Ok(items)
}

pub fn set_enabled(reg: &dyn Registry, source: Source, name: &str, enabled: bool) -> Result<()> {
    if name.is_empty() {
        return Err(Error::new("empty startup item name"));
    }
    let (hive, path) = source.approved();
    let mut bytes = vec![0u8; 12];
    bytes[0] = if enabled { 0x02 } else { 0x03 };
    if !enabled {
        // Task Manager stores when the item was disabled, as a FILETIME.
        bytes[4..].copy_from_slice(&filetime_now().to_le_bytes());
    }
    reg.set(hive, &path, name, &RegValue::Binary(bytes))
}

fn filetime_now() -> u64 {
    // FILETIME counts 100 ns ticks since 1601-01-01.
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    (d.as_secs() + EPOCH_DIFF_SECS) * 10_000_000 + u64::from(d.subsec_nanos()) / 100
}

#[cfg(windows)]
pub fn default_folders() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    let user = std::env::var_os("APPDATA")
        .map(|a| Path::new(&a).join(r"Microsoft\Windows\Start Menu\Programs\Startup"));
    let machine = std::env::var_os("ProgramData")
        .map(|p| Path::new(&p).join(r"Microsoft\Windows\Start Menu\Programs\StartUp"));
    (user, machine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::MockRegistry;

    fn reg_with_items() -> MockRegistry {
        let r = MockRegistry::new();
        r.set(Hive::Hkcu, RUN, "discord", &RegValue::Str("discord.exe --start".into())).unwrap();
        r.set(Hive::Hklm, RUN, "nvidia", &RegValue::Str("nv.exe".into())).unwrap();
        r.set(Hive::Hkcu, RUN, "weird", &RegValue::Dword(1)).unwrap();
        r
    }

    #[test]
    fn lists_run_entries_as_enabled_by_default_and_skips_non_strings() {
        let items = list(&reg_with_items(), None, None).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.enabled));
        assert_eq!(items[0].name, "discord");
        assert_eq!(items[0].source, Source::UserRun);
    }

    #[test]
    fn disable_then_enable_round_trips_and_leaves_run_key_alone() {
        let r = reg_with_items();
        set_enabled(&r, Source::UserRun, "discord", false).unwrap();
        let items = list(&r, None, None).unwrap();
        assert!(!items.iter().find(|i| i.name == "discord").unwrap().enabled);
        assert_eq!(r.get(Hive::Hkcu, RUN, "discord").unwrap(), RegValue::Str("discord.exe --start".into()));
        set_enabled(&r, Source::UserRun, "discord", true).unwrap();
        assert!(list(&r, None, None).unwrap().iter().find(|i| i.name == "discord").unwrap().enabled);
    }

    #[test]
    fn disabled_value_matches_task_manager_format() {
        let r = MockRegistry::new();
        set_enabled(&r, Source::MachineRun, "x", false).unwrap();
        let RegValue::Binary(b) = r.get(Hive::Hklm, &format!(r"{APPROVED}\Run"), "x").unwrap() else { panic!() };
        assert_eq!(b.len(), 12);
        assert_eq!(b[0], 0x03);
        assert_ne!(&b[4..], &[0u8; 8]);
    }

    #[test]
    fn task_manager_06_and_07_variants_are_understood() {
        assert!(is_enabled(&RegValue::Binary(vec![0x06, 0, 0, 0])));
        assert!(!is_enabled(&RegValue::Binary(vec![0x07, 0, 0, 0])));
        assert!(is_enabled(&RegValue::Missing));
    }

    #[test]
    fn startup_folder_shortcuts_are_listed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Steam.lnk"), b"").unwrap();
        std::fs::write(d.path().join("desktop.ini"), b"").unwrap();
        let items = list(&MockRegistry::new(), Some(d.path()), None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Steam.lnk");
        assert_eq!(items[0].source, Source::UserFolder);
    }
}
