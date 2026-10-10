//! System Restore Point creation. This is the last line of defence, so the
//! engine never enables System Protection on its own; it reports
//! `ProtectionOff` and lets the user decide.

use serde::Serialize;

use crate::sys::{Hive, RegValue, Registry, Shell};

const SR_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
const FREQ: &str = "SystemRestorePointCreationFrequency";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "detail")]
pub enum RestoreResult {
    Created,
    ProtectionOff,
    Failed(String),
}

fn powershell(shell: &dyn Shell, script: &str) -> crate::Result<crate::sys::Output> {
    shell.run("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
}

pub fn create_restore_point(shell: &dyn Shell, reg: &dyn Registry, description: &str) -> RestoreResult {
    // Windows sets RPSessionInterval to 0 when protection is off on every drive.
    if let Ok(RegValue::Dword(0)) = reg.get(Hive::Hklm, SR_KEY, "RPSessionInterval") {
        return RestoreResult::ProtectionOff;
    }

    let previous = match reg.get(Hive::Hklm, SR_KEY, FREQ) {
        Ok(v) => v,
        Err(e) => return RestoreResult::Failed(e.0),
    };
    // Windows allows one restore point per 24h by default. Lift that limit
    // just for this call, then put the user's value back exactly.
    if let Err(e) = reg.set(Hive::Hklm, SR_KEY, FREQ, &RegValue::Dword(0)) {
        return RestoreResult::Failed(e.0);
    }

    let desc: String = description.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').collect();
    let script = format!("Checkpoint-Computer -Description '{desc}' -RestorePointType MODIFY_SETTINGS -ErrorAction Stop");
    let out = powershell(shell, &script);

    let _ = reg.set(Hive::Hklm, SR_KEY, FREQ, &previous);

    match out {
        Ok(o) if o.success() => RestoreResult::Created,
        Ok(o) if o.stderr.to_lowercase().contains("disabled") => RestoreResult::ProtectionOff,
        Ok(o) => RestoreResult::Failed(o.stderr.trim().to_string()),
        Err(e) => RestoreResult::Failed(e.0),
    }
}

/// Turns on System Protection for the system drive. Only call after the user agrees.
pub fn enable_protection(shell: &dyn Shell) -> crate::Result<()> {
    let out = powershell(shell, r#"Enable-ComputerRestore -Drive "$env:SystemDrive\" -ErrorAction Stop"#)?;
    if out.success() { Ok(()) } else { Err(crate::Error(out.stderr.trim().to_string())) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::{MockRegistry, MockShell, Output};

    #[test]
    fn success_is_created_and_frequency_restored_to_missing() {
        let reg = MockRegistry::new();
        let shell = MockShell::new();
        assert_eq!(create_restore_point(&shell, &reg, "Raftaar"), RestoreResult::Created);
        assert_eq!(reg.get(Hive::Hklm, SR_KEY, FREQ).unwrap(), RegValue::Missing);
        assert!(shell.calls()[0].contains("Checkpoint-Computer"));
    }

    #[test]
    fn frequency_restored_to_user_value_even_on_failure() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hklm, SR_KEY, FREQ, &RegValue::Dword(1440)).unwrap();
        let shell = MockShell::new().on("Checkpoint-Computer", Output::fail("boom"));
        assert_eq!(create_restore_point(&shell, &reg, "Raftaar"), RestoreResult::Failed("boom".into()));
        assert_eq!(reg.get(Hive::Hklm, SR_KEY, FREQ).unwrap(), RegValue::Dword(1440));
    }

    #[test]
    fn disabled_in_stderr_is_protection_off() {
        let reg = MockRegistry::new();
        let shell = MockShell::new().on("Checkpoint-Computer", Output::fail("System Restore is disabled"));
        assert_eq!(create_restore_point(&shell, &reg, "x"), RestoreResult::ProtectionOff);
    }

    #[test]
    fn session_interval_zero_skips_the_attempt() {
        let reg = MockRegistry::new();
        reg.set(Hive::Hklm, SR_KEY, "RPSessionInterval", &RegValue::Dword(0)).unwrap();
        let shell = MockShell::new();
        assert_eq!(create_restore_point(&shell, &reg, "x"), RestoreResult::ProtectionOff);
        assert!(shell.calls().is_empty());
    }

    #[test]
    fn description_is_sanitized() {
        let reg = MockRegistry::new();
        let shell = MockShell::new();
        create_restore_point(&shell, &reg, "a'; rm -rf");
        assert!(shell.calls()[0].contains("'a rm rf'"));
    }
}
