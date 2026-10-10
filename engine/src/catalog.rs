//! The tweaks Raftaar offers. Only well-known, documented settings that a user
//! could change by hand. Never Defender, Windows Update, UAC or core services.

use crate::power::PowerTarget;
use crate::sys::{Hive, RegValue};
use crate::tweak::{PcClass, RegChange, Risk, Tweak, TweakKind};

const ALL: &[PcClass] = &[PcClass::Low, PcClass::Mid, PcClass::High];
const MM_PROFILE: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";

fn reg(hive: Hive, path: &str, name: &str, value: RegValue) -> RegChange {
    RegChange { hive, path: path.into(), name: name.into(), value }
}

#[allow(clippy::too_many_arguments)]
fn tweak(
    id: &str,
    name_en: &str,
    name_ur: &str,
    desc_en: &str,
    desc_ur: &str,
    risk: Risk,
    needs_restart: bool,
    recommended_for: &[PcClass],
    kind: TweakKind,
) -> Tweak {
    Tweak {
        id: id.into(),
        name_en: name_en.into(),
        name_ur: name_ur.into(),
        desc_en: desc_en.into(),
        desc_ur: desc_ur.into(),
        risk,
        needs_restart,
        recommended_for: recommended_for.to_vec(),
        kind,
    }
}

pub fn catalog() -> Vec<Tweak> {
    use Hive::*;
    use RegValue::*;
    vec![
        tweak(
            "power_high",
            "High Performance power plan",
            "ہائی پرفارمنس پاور پلان",
            "Keeps the CPU at full speed instead of saving power. On laptops this uses more battery.",
            "سی پی یو کو بجلی بچانے کے بجائے پوری رفتار پر رکھتا ہے۔ لیپ ٹاپ پر بیٹری جلدی ختم ہوگی۔",
            Risk::Low,
            false,
            ALL,
            TweakKind::PowerPlan(PowerTarget::HighPerformance),
        ),
        tweak(
            "game_mode",
            "Windows Game Mode on",
            "ونڈوز گیم موڈ آن",
            "Tells Windows to give the running game priority over background work.",
            "ونڈوز کو بتاتا ہے کہ چلتی گیم کو پس منظر کے کاموں پر ترجیح دے۔",
            Risk::Low,
            false,
            ALL,
            TweakKind::Registry(vec![
                reg(Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled", Dword(1)),
                reg(Hkcu, r"Software\Microsoft\GameBar", "AllowAutoGameMode", Dword(1)),
            ]),
        ),
        tweak(
            "game_dvr_off",
            "Xbox background recording off",
            "ایکس باکس بیک گراؤنڈ ریکارڈنگ بند",
            "Stops Game Bar from silently recording gameplay in the background, which costs FPS.",
            "گیم بار کو پس منظر میں خاموشی سے ریکارڈنگ کرنے سے روکتا ہے، جو ایف پی ایس کھاتی ہے۔",
            Risk::Low,
            false,
            ALL,
            TweakKind::Registry(vec![
                reg(Hkcu, r"System\GameConfigStore", "GameDVR_Enabled", Dword(0)),
                reg(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", Dword(0)),
            ]),
        ),
        tweak(
            "multimedia_games",
            "Prioritise games in the scheduler",
            "گیمز کو ترجیح دیں",
            "Reserves less CPU for background media tasks and raises the priority Windows gives to games.",
            "پس منظر کے میڈیا کاموں کے لیے کم سی پی یو رکھتا ہے اور گیمز کی ترجیح بڑھاتا ہے۔",
            Risk::Medium,
            true,
            ALL,
            TweakKind::Registry(vec![
                reg(Hklm, MM_PROFILE, "SystemResponsiveness", Dword(10)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "GPU Priority", Dword(8)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "Priority", Dword(6)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "Scheduling Category", Str("High".into())),
            ]),
        ),
        tweak(
            "mouse_accel_off",
            "Mouse acceleration off",
            "ماؤس ایکسیلریشن بند",
            "Turns off \"Enhance pointer precision\" so aim moves the same distance at any speed.",
            "ماؤس کی رفتار کے حساب سے فاصلہ بدلنا بند کرتا ہے، تاکہ نشانہ ہر بار ایک جیسا ہو۔",
            Risk::Low,
            true,
            ALL,
            TweakKind::Registry(vec![
                reg(Hkcu, r"Control Panel\Mouse", "MouseSpeed", Str("0".into())),
                reg(Hkcu, r"Control Panel\Mouse", "MouseThreshold1", Str("0".into())),
                reg(Hkcu, r"Control Panel\Mouse", "MouseThreshold2", Str("0".into())),
            ]),
        ),
        tweak(
            "background_apps_off",
            "Background Store apps off",
            "پس منظر کی اسٹور ایپس بند",
            "Stops Microsoft Store apps from running when you are not using them. Frees RAM and CPU.",
            "مائیکروسافٹ اسٹور کی ایپس کو استعمال نہ ہونے پر چلنے سے روکتا ہے۔ ریم اور سی پی یو بچتا ہے۔",
            Risk::Low,
            false,
            &[PcClass::Low, PcClass::Mid],
            TweakKind::Registry(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
                Dword(1),
            )]),
        ),
        tweak(
            "visual_effects_perf",
            "Visual effects for performance",
            "کارکردگی کے لیے ویژول ایفیکٹس کم",
            "Turns off window animations and shadows. Windows looks plainer but feels faster on weak PCs.",
            "ونڈوز کی اینیمیشن اور سائے بند کرتا ہے۔ کمزور پی سی پر ونڈوز تیز محسوس ہوگی۔",
            Risk::Low,
            true,
            &[PcClass::Low],
            TweakKind::Registry(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
                "VisualFXSetting",
                Dword(2),
            )]),
        ),
    ]
}

pub fn recommended_ids(class: PcClass) -> Vec<String> {
    catalog().into_iter().filter(|t| t.recommended_for.contains(&class)).map(|t| t.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotStore;
    use crate::sys::{MockRegistry, MockShell, Output, Registry};
    use crate::tweak::Engine;

    #[test]
    fn ids_are_unique_and_text_is_present() {
        let c = catalog();
        let mut ids: Vec<_> = c.iter().map(|t| t.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
        for t in &c {
            assert!(!t.name_en.is_empty() && !t.name_ur.is_empty(), "{}", t.id);
            assert!(!t.desc_en.is_empty() && !t.desc_ur.is_empty(), "{}", t.id);
        }
    }

    #[test]
    fn never_touches_forbidden_areas() {
        let forbidden = ["defender", "windowsupdate", "policies\\system", "enablelua", "\\services\\"];
        for t in catalog() {
            if let TweakKind::Registry(changes) = &t.kind {
                for c in changes {
                    let p = c.path.to_lowercase() + "\\" + &c.name.to_lowercase();
                    assert!(!forbidden.iter().any(|f| p.contains(f)), "{} touches {}", t.id, p);
                }
            }
        }
    }

    #[test]
    fn apply_all_then_revert_all_leaves_registry_identical() {
        let reg = MockRegistry::new();
        // Seed a few realistic pre-existing values.
        reg.set(Hive::Hkcu, r"Control Panel\Mouse", "MouseSpeed", &RegValue::Str("1".into())).unwrap();
        reg.set(Hive::Hklm, MM_PROFILE, "SystemResponsiveness", &RegValue::Dword(20)).unwrap();
        let before = reg.snapshot();
        let shell = MockShell::new().on("/getactivescheme", Output::ok("GUID: 381b4222-f694-41f0-9685-ff5bb260df2e"));
        let dir = tempfile::tempdir().unwrap();
        let mut e = Engine::new(&reg, &shell, SnapshotStore::load(dir.path().join("s.json")).unwrap());
        for t in catalog() {
            e.apply(&t).unwrap();
        }
        assert_ne!(reg.snapshot(), before);
        e.revert_all().unwrap();
        assert_eq!(reg.snapshot(), before);
    }

    #[test]
    fn low_pcs_get_more_recommendations_than_high() {
        assert!(recommended_ids(PcClass::Low).len() > recommended_ids(PcClass::High).len());
    }
}
