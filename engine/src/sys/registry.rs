use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hive {
    Hkcu,
    Hklm,
}

/// A registry value as Raftaar sees it. `Missing` is a real state: reverting a
/// value that did not exist before means deleting it, not writing a default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegValue {
    Dword(u32),
    Str(String),
    Binary(Vec<u8>),
    Missing,
}

pub trait Registry: Send + Sync {
    fn get(&self, hive: Hive, path: &str, name: &str) -> Result<RegValue>;
    /// Writes the value. `RegValue::Missing` deletes it (no error if already absent).
    fn set(&self, hive: Hive, path: &str, name: &str, value: &RegValue) -> Result<()>;
    /// All values directly under a key, skipping types Raftaar doesn't model. Missing key = empty.
    fn values(&self, hive: Hive, path: &str) -> Result<Vec<(String, RegValue)>>;
}

/// The real Windows registry (64-bit view).
pub struct WinRegistry;

#[cfg(windows)]
impl WinRegistry {
    fn root(hive: Hive) -> winreg::RegKey {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        winreg::RegKey::predef(match hive {
            Hive::Hkcu => HKEY_CURRENT_USER,
            Hive::Hklm => HKEY_LOCAL_MACHINE,
        })
    }
}

#[cfg(windows)]
impl Registry for WinRegistry {
    fn get(&self, hive: Hive, path: &str, name: &str) -> Result<RegValue> {
        use winreg::enums::*;
        let key = match Self::root(hive).open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(RegValue::Missing),
            Err(e) => return Err(e.into()),
        };
        let raw = match key.get_raw_value(name) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(RegValue::Missing),
            Err(e) => return Err(e.into()),
        };
        match raw.vtype {
            REG_DWORD => Ok(RegValue::Dword(key.get_value::<u32, _>(name)?)),
            REG_SZ | REG_EXPAND_SZ => Ok(RegValue::Str(key.get_value::<String, _>(name)?)),
            REG_BINARY => Ok(RegValue::Binary(raw.bytes.into_owned())),
            other => Err(Error(format!("{path}\\{name} has unsupported type {other:?}"))),
        }
    }

    fn values(&self, hive: Hive, path: &str) -> Result<Vec<(String, RegValue)>> {
        use winreg::enums::*;
        let key = match Self::root(hive).open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e.into()),
        };
        let mut out = vec![];
        for item in key.enum_values() {
            let (name, raw) = item?;
            let v = match raw.vtype {
                REG_DWORD => RegValue::Dword(key.get_value::<u32, _>(&name)?),
                REG_SZ | REG_EXPAND_SZ => RegValue::Str(key.get_value::<String, _>(&name)?),
                REG_BINARY => RegValue::Binary(raw.bytes.into_owned()),
                _ => continue,
            };
            out.push((name, v));
        }
        Ok(out)
    }

    fn set(&self, hive: Hive, path: &str, name: &str, value: &RegValue) -> Result<()> {
        use winreg::enums::*;
        match value {
            RegValue::Missing => {
                let key = match Self::root(hive).open_subkey_with_flags(path, KEY_SET_VALUE) {
                    Ok(k) => k,
                    Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
                    Err(e) => return Err(e.into()),
                };
                match key.delete_value(name) {
                    Err(e) if e.kind() != ErrorKind::NotFound => Err(e.into()),
                    _ => Ok(()),
                }
            }
            RegValue::Dword(v) => {
                let (key, _) = Self::root(hive).create_subkey(path)?;
                Ok(key.set_value(name, v)?)
            }
            RegValue::Str(v) => {
                let (key, _) = Self::root(hive).create_subkey(path)?;
                Ok(key.set_value(name, v)?)
            }
            RegValue::Binary(bytes) => {
                let (key, _) = Self::root(hive).create_subkey(path)?;
                Ok(key.set_raw_value(name, &winreg::RegValue { bytes: bytes.clone().into(), vtype: REG_BINARY })?)
            }
        }
    }
}

/// In-memory registry for tests. Paths and names are case-insensitive, like Windows.
#[derive(Default)]
pub struct MockRegistry {
    values: Mutex<HashMap<(Hive, String, String), RegValue>>,
    /// When set, any `set` on this value name fails, to test rollback paths.
    pub fail_set_on: Mutex<Option<String>>,
}

impl MockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn k(hive: Hive, path: &str, name: &str) -> (Hive, String, String) {
        (hive, path.to_lowercase(), name.to_lowercase())
    }

    pub fn snapshot(&self) -> HashMap<(Hive, String, String), RegValue> {
        self.values.lock().unwrap().clone()
    }
}

impl Registry for MockRegistry {
    fn get(&self, hive: Hive, path: &str, name: &str) -> Result<RegValue> {
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(&Self::k(hive, path, name))
            .cloned()
            .unwrap_or(RegValue::Missing))
    }

    fn set(&self, hive: Hive, path: &str, name: &str, value: &RegValue) -> Result<()> {
        if let Some(bad) = self.fail_set_on.lock().unwrap().as_deref()
            && bad.eq_ignore_ascii_case(name) {
                return Err(Error(format!("mock: access denied writing {name}")));
            }
        let mut map = self.values.lock().unwrap();
        match value {
            RegValue::Missing => {
                map.remove(&Self::k(hive, path, name));
            }
            v => {
                map.insert(Self::k(hive, path, name), v.clone());
            }
        }
        Ok(())
    }

    fn values(&self, hive: Hive, path: &str) -> Result<Vec<(String, RegValue)>> {
        let p = path.to_lowercase();
        let mut out: Vec<(String, RegValue)> = self
            .values
            .lock()
            .unwrap()
            .iter()
            .filter(|((h, kp, _), _)| *h == hive && *kp == p)
            .map(|((_, _, n), v)| (n.clone(), v.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_set_then_get_returns_value() {
        let r = MockRegistry::new();
        r.set(Hive::Hkcu, r"A\B", "X", &RegValue::Dword(5)).unwrap();
        assert_eq!(r.get(Hive::Hkcu, r"a\b", "x").unwrap(), RegValue::Dword(5));
    }

    #[test]
    fn get_of_absent_value_is_missing() {
        let r = MockRegistry::new();
        assert_eq!(r.get(Hive::Hklm, "Nope", "Nope").unwrap(), RegValue::Missing);
    }

    #[test]
    fn setting_missing_deletes() {
        let r = MockRegistry::new();
        r.set(Hive::Hkcu, "A", "X", &RegValue::Str("1".into())).unwrap();
        r.set(Hive::Hkcu, "A", "X", &RegValue::Missing).unwrap();
        assert_eq!(r.get(Hive::Hkcu, "A", "X").unwrap(), RegValue::Missing);
    }

    #[cfg(windows)]
    #[test]
    fn real_registry_round_trip_in_hkcu() {
        let r = WinRegistry;
        let path = r"Software\RaftaarTest";
        r.set(Hive::Hkcu, path, "Probe", &RegValue::Dword(7)).unwrap();
        assert_eq!(r.get(Hive::Hkcu, path, "Probe").unwrap(), RegValue::Dword(7));
        r.set(Hive::Hkcu, path, "Probe", &RegValue::Missing).unwrap();
        assert_eq!(r.get(Hive::Hkcu, path, "Probe").unwrap(), RegValue::Missing);
        let _ = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey(path);
    }
}
