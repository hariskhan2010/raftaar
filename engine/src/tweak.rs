//! Tweak model and the engine that applies and reverts tweaks.
//!
//! Safety contract: the prior state is written to disk *before* the first
//! system write, and a tweak that fails halfway is rolled back immediately.

use serde::Serialize;

use crate::power::{self, PowerTarget};
use crate::snapshot::{Entry, Prior, SnapshotStore, now_unix};
use crate::sys::{Hive, RegValue, Registry, Shell};
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Risk {
    Low,
    Medium,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum PcClass {
    Low,
    Mid,
    High,
}

/// Groups tweaks in the UI, like Hone's merged optimization groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Category {
    Fps,
    Power,
    Background,
    Privacy,
    Visuals,
    Network,
    Storage,
}

/// Presets are cumulative: Max includes everything in Balanced, which includes Lite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub enum Preset {
    Lite,
    Balanced,
    Max,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct RegChange {
    pub hive: Hive,
    pub path: String,
    pub name: String,
    pub value: RegValue,
}

#[derive(Clone, Debug, Serialize)]
pub enum TweakKind {
    Registry(Vec<RegChange>),
    PowerPlan(PowerTarget),
}

#[derive(Clone, Debug, Serialize)]
pub struct Tweak {
    pub id: String,
    pub category: Category,
    pub name_en: String,
    pub name_ur: String,
    pub desc_en: String,
    pub desc_ur: String,
    pub risk: Risk,
    /// Takes effect only after sign-out or restart.
    pub needs_restart: bool,
    pub recommended_for: Vec<PcClass>,
    /// The smallest preset that includes this tweak. `None` = only applied by hand (a fix, not a speed-up).
    pub preset: Option<Preset>,
    pub kind: TweakKind,
}

pub struct Engine<'a> {
    pub reg: &'a dyn Registry,
    pub shell: &'a dyn Shell,
    pub store: SnapshotStore,
}

impl<'a> Engine<'a> {
    pub fn new(reg: &'a dyn Registry, shell: &'a dyn Shell, store: SnapshotStore) -> Self {
        Engine { reg, shell, store }
    }

    pub fn is_applied(&self, id: &str) -> bool {
        self.store.get(id).is_some()
    }

    pub fn apply(&mut self, tweak: &Tweak) -> Result<()> {
        if self.is_applied(&tweak.id) {
            // Never overwrite the original snapshot with already-tweaked values.
            return Ok(());
        }
        match &tweak.kind {
            TweakKind::Registry(changes) => self.apply_registry(&tweak.id, changes),
            TweakKind::PowerPlan(target) => {
                let prior = power::apply(self.shell, *target)?;
                self.store.push(Entry { tweak_id: tweak.id.clone(), prior, applied_at: Some(now_unix()) })?;
                Ok(())
            }
        }
    }

    fn apply_registry(&mut self, id: &str, changes: &[RegChange]) -> Result<()> {
        let mut prior = Vec::with_capacity(changes.len());
        for c in changes {
            let old = self.reg.get(c.hive, &c.path, &c.name)?;
            prior.push(RegChange { value: old, ..c.clone() });
        }
        self.store.push(Entry {
            tweak_id: id.to_string(),
            prior: Prior::Registry(prior.clone()),
            applied_at: Some(now_unix()),
        })?;

        for (i, c) in changes.iter().enumerate() {
            if let Err(e) = self.reg.set(c.hive, &c.path, &c.name, &c.value) {
                // Roll back what this tweak already wrote, newest first.
                for p in prior[..i].iter().rev() {
                    let _ = self.reg.set(p.hive, &p.path, &p.name, &p.value);
                }
                self.store.remove(id)?;
                return Err(Error(format!("{id}: {e}")));
            }
        }
        Ok(())
    }

    pub fn revert(&mut self, id: &str) -> Result<()> {
        let Some(entry) = self.store.get(id).cloned() else {
            return Ok(());
        };
        match &entry.prior {
            Prior::Registry(prior) => {
                for p in prior.iter().rev() {
                    self.reg.set(p.hive, &p.path, &p.name, &p.value)?;
                }
            }
            Prior::PowerPlan { .. } => power::revert(self.shell, &entry.prior)?,
        }
        self.store.remove(id)
    }

    /// Reverts newest-first. Keeps going past failures and reports them all,
    /// so one stuck value never blocks the rest from being restored.
    pub fn revert_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.store.entries().iter().rev().map(|e| e.tweak_id.clone()).collect();
        let errors: Vec<String> = ids.iter().filter_map(|id| self.revert(id).err().map(|e| e.0)).collect();
        if errors.is_empty() { Ok(()) } else { Err(Error(errors.join("; "))) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::{MockRegistry, MockShell};

    fn tw(id: &str, changes: Vec<(&str, RegValue)>) -> Tweak {
        Tweak {
            id: id.into(),
            category: Category::Fps,
            name_en: id.into(),
            name_ur: id.into(),
            desc_en: String::new(),
            desc_ur: String::new(),
            risk: Risk::Low,
            needs_restart: false,
            recommended_for: vec![],
            preset: None,
            kind: TweakKind::Registry(
                changes
                    .into_iter()
                    .map(|(n, v)| RegChange { hive: Hive::Hkcu, path: "K".into(), name: n.into(), value: v })
                    .collect(),
            ),
        }
    }

    fn store() -> (tempfile::TempDir, SnapshotStore) {
        let dir = tempfile::tempdir().unwrap();
        let s = SnapshotStore::load(dir.path().join("snap.json")).unwrap();
        (dir, s)
    }

    #[test]
    fn apply_then_revert_restores_original_dword() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hkcu, "K", "A", &RegValue::Dword(3)).unwrap();
        let shell = MockShell::new();
        let (_d, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        e.apply(&tw("t", vec![("A", RegValue::Dword(9))])).unwrap();
        assert_eq!(reg.get(Hive::Hkcu, "K", "A").unwrap(), RegValue::Dword(9));
        e.revert("t").unwrap();
        assert_eq!(reg.get(Hive::Hkcu, "K", "A").unwrap(), RegValue::Dword(3));
        assert!(!e.is_applied("t"));
    }

    #[test]
    fn revert_restores_missing_by_deleting() {
        let reg = MockRegistry::new();
        let shell = MockShell::new();
        let (_d, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        e.apply(&tw("t", vec![("New", RegValue::Dword(1))])).unwrap();
        e.revert("t").unwrap();
        assert_eq!(reg.get(Hive::Hkcu, "K", "New").unwrap(), RegValue::Missing);
    }

    #[test]
    fn double_apply_keeps_first_snapshot() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hkcu, "K", "A", &RegValue::Dword(3)).unwrap();
        let shell = MockShell::new();
        let (_d, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        let t = tw("t", vec![("A", RegValue::Dword(9))]);
        e.apply(&t).unwrap();
        e.apply(&t).unwrap();
        e.revert("t").unwrap();
        assert_eq!(reg.get(Hive::Hkcu, "K", "A").unwrap(), RegValue::Dword(3));
    }

    #[test]
    fn revert_all_restores_everything_and_empties_store() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hkcu, "K", "A", &RegValue::Str("orig".into())).unwrap();
        let before = reg.snapshot();
        let shell = MockShell::new();
        let (_d, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        e.apply(&tw("one", vec![("A", RegValue::Str("x".into()))])).unwrap();
        e.apply(&tw("two", vec![("A", RegValue::Str("y".into())), ("B", RegValue::Dword(1))])).unwrap();
        e.revert_all().unwrap();
        assert_eq!(reg.snapshot(), before);
        assert!(e.store.entries().is_empty());
    }

    #[test]
    fn snapshot_is_on_disk_before_registry_write_and_failure_rolls_back() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hkcu, "K", "A", &RegValue::Dword(1)).unwrap();
        *reg.fail_set_on.lock().unwrap() = Some("B".into());
        let before = reg.snapshot();
        let shell = MockShell::new();
        let (dir, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        let err = e.apply(&tw("t", vec![("A", RegValue::Dword(2)), ("B", RegValue::Dword(2))]));
        assert!(err.is_err());
        // A was written then rolled back; nothing left behind.
        assert_eq!(reg.snapshot(), before);
        assert!(!e.is_applied("t"));
        let on_disk = SnapshotStore::load(dir.path().join("snap.json")).unwrap();
        assert!(on_disk.entries().is_empty());
    }

    #[test]
    fn snapshots_survive_restart() {
        let reg = MockRegistry::new();
        let shell = MockShell::new();
        let (dir, s) = store();
        let mut e = Engine::new(&reg, &shell, s);
        e.apply(&tw("t", vec![("A", RegValue::Dword(9))])).unwrap();
        drop(e);
        let reloaded = SnapshotStore::load(dir.path().join("snap.json")).unwrap();
        let mut e2 = Engine::new(&reg, &shell, reloaded);
        assert!(e2.is_applied("t"));
        e2.revert("t").unwrap();
        assert_eq!(reg.get(Hive::Hkcu, "K", "A").unwrap(), RegValue::Missing);
    }
}
