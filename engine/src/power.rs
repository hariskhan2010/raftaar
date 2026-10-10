//! Power plan tweak. Remembers the user's active plan and deletes any plan
//! Raftaar created when reverting.

use serde::{Deserialize, Serialize};

use crate::snapshot::Prior;
use crate::sys::Shell;
use crate::{Error, Result};

pub const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerTarget {
    HighPerformance,
    Ultimate,
}

/// Finds the first GUID in powercfg output. Works on any Windows display
/// language because it matches the GUID shape, not the surrounding words.
pub fn find_guid(text: &str) -> Option<String> {
    let groups = [8, 4, 4, 4, 12];
    let total: usize = groups.iter().sum::<usize>() + 4;
    let chars: Vec<char> = text.chars().collect();
    'outer: for start in 0..chars.len().saturating_sub(total - 1) {
        let cand: String = chars[start..start + total].iter().collect();
        let mut parts = cand.split('-');
        for len in groups {
            match parts.next() {
                Some(p) if p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()) => {}
                _ => continue 'outer,
            }
        }
        return Some(cand.to_lowercase());
    }
    None
}

fn powercfg(shell: &dyn Shell, args: &[&str]) -> Result<String> {
    let out = shell.run("powercfg.exe", args)?;
    if out.success() {
        Ok(out.stdout)
    } else {
        Err(Error(format!("powercfg {}: {}{}", args.join(" "), out.stderr.trim(), out.stdout.trim())))
    }
}

pub fn active_guid(shell: &dyn Shell) -> Result<String> {
    find_guid(&powercfg(shell, &["/getactivescheme"])?).ok_or_else(|| Error::new("could not read the active power plan"))
}

pub fn apply(shell: &dyn Shell, target: PowerTarget) -> Result<Prior> {
    let previous_guid = active_guid(shell)?;
    let mut created_guid = None;

    let to_activate = match target {
        PowerTarget::HighPerformance if powercfg(shell, &["/setactive", HIGH_PERFORMANCE]).is_ok() => {
            return Ok(Prior::PowerPlan { previous_guid, created_guid });
        }
        // Some laptops hide the built-in High performance plan, so duplicate it.
        PowerTarget::HighPerformance => HIGH_PERFORMANCE,
        PowerTarget::Ultimate => ULTIMATE,
    };
    let out = powercfg(shell, &["-duplicatescheme", to_activate])?;
    let new_guid = find_guid(&out).ok_or_else(|| Error::new("powercfg did not return the new plan"))?;
    created_guid = Some(new_guid.clone());
    if let Err(e) = powercfg(shell, &["/setactive", &new_guid]) {
        let _ = powercfg(shell, &["/delete", &new_guid]);
        return Err(e);
    }
    Ok(Prior::PowerPlan { previous_guid, created_guid })
}

pub fn revert(shell: &dyn Shell, prior: &Prior) -> Result<()> {
    let Prior::PowerPlan { previous_guid, created_guid } = prior else {
        return Err(Error::new("not a power plan snapshot"));
    };
    powercfg(shell, &["/setactive", previous_guid])?;
    if let Some(g) = created_guid {
        powercfg(shell, &["/delete", g])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::{MockShell, Output};

    const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

    #[test]
    fn parses_getactivescheme_output() {
        let s = format!("Power Scheme GUID: {}  (Balanced)", BALANCED.to_uppercase());
        assert_eq!(find_guid(&s).as_deref(), Some(BALANCED));
        assert_eq!(find_guid("no guid here"), None);
    }

    #[test]
    fn high_performance_then_revert_reactivates_saved_guid() {
        let shell = MockShell::new().on("/getactivescheme", Output::ok(&format!("GUID: {BALANCED} (Balanced)")));
        let prior = apply(&shell, PowerTarget::HighPerformance).unwrap();
        assert_eq!(prior, Prior::PowerPlan { previous_guid: BALANCED.into(), created_guid: None });
        revert(&shell, &prior).unwrap();
        let calls = shell.calls();
        assert!(calls.last().unwrap().ends_with(&format!("/setactive {BALANCED}")));
    }

    #[test]
    fn ultimate_plan_is_created_and_deleted_on_revert() {
        let created = "11111111-2222-3333-4444-555555555555";
        let shell = MockShell::new()
            .on("/getactivescheme", Output::ok(&format!("GUID: {BALANCED}")))
            .on("-duplicatescheme", Output::ok(&format!("Power Scheme GUID: {created}  (Ultimate Performance)")));
        let prior = apply(&shell, PowerTarget::Ultimate).unwrap();
        assert_eq!(prior, Prior::PowerPlan { previous_guid: BALANCED.into(), created_guid: Some(created.into()) });
        revert(&shell, &prior).unwrap();
        assert!(shell.calls().last().unwrap().ends_with(&format!("/delete {created}")));
    }
}
