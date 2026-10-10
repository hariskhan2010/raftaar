//! The tweaks Raftaar offers. Only well-known, documented settings that a user
//! could change by hand. Never Defender, Windows Update, UAC, security
//! mitigations or services.
//!
//! Ids are stored in users' snapshot files, so never rename one.

use crate::power::PowerTarget;
use crate::sys::{Hive, RegValue};
use crate::tweak::{Category, PcClass, Preset, RegChange, Risk, Tweak, TweakKind};

const ALL: &[PcClass] = &[PcClass::Low, PcClass::Mid, PcClass::High];
const LOW_MID: &[PcClass] = &[PcClass::Low, PcClass::Mid];
const MID_HIGH: &[PcClass] = &[PcClass::Mid, PcClass::High];
const LOW: &[PcClass] = &[PcClass::Low];
const NONE: &[PcClass] = &[];

const MM_PROFILE: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const GAME_CONFIG: &str = r"System\GameConfigStore";
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";

fn reg(hive: Hive, path: &str, name: &str, value: RegValue) -> RegChange {
    RegChange { hive, path: path.into(), name: name.into(), value }
}

/// `en` and `ur` are `[name, description]`.
#[allow(clippy::too_many_arguments)]
fn t(
    id: &str,
    category: Category,
    preset: Option<Preset>,
    risk: Risk,
    needs_restart: bool,
    recommended_for: &[PcClass],
    en: [&str; 2],
    ur: [&str; 2],
    kind: TweakKind,
) -> Tweak {
    Tweak {
        id: id.into(),
        category,
        name_en: en[0].into(),
        desc_en: en[1].into(),
        name_ur: ur[0].into(),
        desc_ur: ur[1].into(),
        risk,
        needs_restart,
        recommended_for: recommended_for.to_vec(),
        preset,
        kind,
    }
}

fn regs(changes: Vec<RegChange>) -> TweakKind {
    TweakKind::Registry(changes)
}

pub fn catalog() -> Vec<Tweak> {
    use Category::*;
    use Hive::*;
    use Preset::*;
    use RegValue::*;
    use Risk::{Low as LowRisk, Medium};

    vec![
        // ---------------- FPS & input lag ----------------
        t(
            "game_mode",
            Fps,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            ["Windows Game Mode on", "Tells Windows to give the running game priority over background work."],
            ["ونڈوز گیم موڈ آن", "ونڈوز کو بتاتا ہے کہ چلتی گیم کو پس منظر کے کاموں پر ترجیح دے۔"],
            regs(vec![
                reg(Hkcu, r"Software\Microsoft\GameBar", "AutoGameModeEnabled", Dword(1)),
                reg(Hkcu, r"Software\Microsoft\GameBar", "AllowAutoGameMode", Dword(1)),
            ]),
        ),
        t(
            "game_dvr_off",
            Fps,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            [
                "Xbox background recording off",
                "Stops Game Bar from silently recording gameplay in the background, which costs FPS.",
            ],
            [
                "ایکس باکس بیک گراؤنڈ ریکارڈنگ بند",
                "گیم بار کو پس منظر میں خاموشی سے ریکارڈنگ کرنے سے روکتا ہے، جو ایف پی ایس کھاتی ہے۔",
            ],
            regs(vec![
                reg(Hkcu, GAME_CONFIG, "GameDVR_Enabled", Dword(0)),
                reg(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", Dword(0)),
            ]),
        ),
        t(
            "mouse_accel_off",
            Fps,
            Some(Lite),
            LowRisk,
            true,
            ALL,
            [
                "Mouse acceleration off",
                "Turns off \"Enhance pointer precision\" so aim moves the same distance at any speed.",
            ],
            ["ماؤس ایکسیلریشن بند", "ماؤس کی رفتار کے حساب سے فاصلہ بدلنا بند کرتا ہے، تاکہ نشانہ ہر بار ایک جیسا ہو۔"],
            regs(vec![
                reg(Hkcu, r"Control Panel\Mouse", "MouseSpeed", Str("0".into())),
                reg(Hkcu, r"Control Panel\Mouse", "MouseThreshold1", Str("0".into())),
                reg(Hkcu, r"Control Panel\Mouse", "MouseThreshold2", Str("0".into())),
            ]),
        ),
        t(
            "multimedia_games",
            Fps,
            Some(Balanced),
            Medium,
            true,
            ALL,
            [
                "Prioritise games in the scheduler",
                "Reserves less CPU for background media tasks and raises the priority Windows gives to games.",
            ],
            ["گیمز کو ترجیح دیں", "پس منظر کے میڈیا کاموں کے لیے کم سی پی یو رکھتا ہے اور گیمز کی ترجیح بڑھاتا ہے۔"],
            regs(vec![
                reg(Hklm, MM_PROFILE, "SystemResponsiveness", Dword(10)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "GPU Priority", Dword(8)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "Priority", Dword(6)),
                reg(Hklm, &format!(r"{MM_PROFILE}\Tasks\Games"), "Scheduling Category", Str("High".into())),
            ]),
        ),
        t(
            "foreground_boost",
            Fps,
            Some(Balanced),
            Medium,
            false,
            ALL,
            [
                "Favour the active window",
                "Gives the program in front (your game) longer, more frequent CPU turns than background programs.",
            ],
            ["سامنے والی ونڈو کو ترجیح", "سامنے چلنے والے پروگرام (آپ کی گیم) کو پس منظر کے پروگراموں سے زیادہ سی پی یو وقت دیتا ہے۔"],
            regs(vec![reg(Hklm, r"SYSTEM\CurrentControlSet\Control\PriorityControl", "Win32PrioritySeparation", Dword(0x26))]),
        ),
        t(
            "timer_resolution_global",
            Fps,
            Some(Balanced),
            LowRisk,
            true,
            ALL,
            [
                "Precise game timers (Windows 11)",
                "Lets a game's request for a 1 ms timer apply system-wide again, which smooths frame pacing. No effect on Windows 10.",
            ],
            [
                "گیم ٹائمر درست (ونڈوز 11)",
                "گیم کی 1 ملی سیکنڈ ٹائمر کی درخواست کو پورے سسٹم پر لاگو ہونے دیتا ہے، جس سے فریم ہموار آتے ہیں۔ ونڈوز 10 پر کوئی اثر نہیں۔",
            ],
            regs(vec![reg(
                Hklm,
                r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel",
                "GlobalTimerResolutionRequests",
                Dword(1),
            )]),
        ),
        t(
            "hags_on",
            Fps,
            Some(Max),
            Medium,
            true,
            MID_HIGH,
            [
                "Hardware-accelerated GPU scheduling",
                "Lets the graphics card manage its own memory, which can lower latency. Needs a GPU and driver that support it (GTX 1000 / RX 5000 or newer).",
            ],
            [
                "ہارڈویئر ایکسیلریٹڈ جی پی یو شیڈیولنگ",
                "گرافکس کارڈ کو اپنی میموری خود سنبھالنے دیتا ہے، جس سے تاخیر کم ہو سکتی ہے۔ نیا جی پی یو اور ڈرائیور ضروری ہے۔",
            ],
            regs(vec![reg(Hklm, r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", Dword(2))]),
        ),
        t(
            "fullscreen_opt_off",
            Fps,
            Some(Max),
            Medium,
            false,
            ALL,
            [
                "Fullscreen optimizations off",
                "Runs fullscreen games in true exclusive mode. Can lower input lag in older DirectX games, but Alt+Tab is slower and Auto HDR stops working.",
            ],
            [
                "فل اسکرین آپٹیمائزیشن بند",
                "فل اسکرین گیمز کو اصل ایکسکلوسیو موڈ میں چلاتا ہے۔ پرانی گیمز میں ان پٹ لیگ کم ہو سکتا ہے، لیکن Alt+Tab سست ہوگا۔",
            ],
            regs(vec![
                reg(Hkcu, GAME_CONFIG, "GameDVR_FSEBehaviorMode", Dword(2)),
                reg(Hkcu, GAME_CONFIG, "GameDVR_FSEBehavior", Dword(2)),
                reg(Hkcu, GAME_CONFIG, "GameDVR_HonorUserFSEBehaviorMode", Dword(1)),
                reg(Hkcu, GAME_CONFIG, "GameDVR_DXGIHonorFSEWindowsCompatible", Dword(1)),
            ]),
        ),
        t(
            "mpo_off",
            Fps,
            None,
            Medium,
            true,
            NONE,
            [
                "Fix flicker and stutter (MPO off)",
                "Turns off Multiplane Overlay. Only use this if you see screen flicker, black flashes or stutter in games or browsers; it is a known fix for some NVIDIA drivers.",
            ],
            [
                "اسکرین فلکر اور اٹکنے کا حل (MPO بند)",
                "ملٹی پلین اوورلے بند کرتا ہے۔ صرف تب استعمال کریں جب اسکرین جھلملائے یا گیم اٹکے؛ کچھ NVIDIA ڈرائیورز کا معروف حل ہے۔",
            ],
            regs(vec![reg(Hklm, r"SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", Dword(5))]),
        ),
        // ---------------- Power ----------------
        t(
            "power_high",
            Power,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            [
                "High Performance power plan",
                "Keeps the CPU at full speed instead of saving power. On laptops this uses more battery.",
            ],
            [
                "ہائی پرفارمنس پاور پلان",
                "سی پی یو کو بجلی بچانے کے بجائے پوری رفتار پر رکھتا ہے۔ لیپ ٹاپ پر بیٹری جلدی ختم ہوگی۔",
            ],
            TweakKind::PowerPlan(PowerTarget::HighPerformance),
        ),
        t(
            "power_throttling_off",
            Power,
            Some(Balanced),
            LowRisk,
            true,
            ALL,
            [
                "Power throttling off",
                "Stops Windows from slowing down programs it thinks are in the background, such as voice chat or a stream.",
            ],
            [
                "پاور تھروٹلنگ بند",
                "ونڈوز کو پس منظر کے پروگرام (جیسے وائس چیٹ یا اسٹریم) سست کرنے سے روکتا ہے۔",
            ],
            regs(vec![reg(Hklm, r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling", "PowerThrottlingOff", Dword(1))]),
        ),
        // ---------------- Background activity ----------------
        t(
            "background_apps_off",
            Background,
            Some(Lite),
            LowRisk,
            false,
            LOW_MID,
            [
                "Background Store apps off",
                "Stops Microsoft Store apps from running when you are not using them. Frees RAM and CPU.",
            ],
            [
                "پس منظر کی اسٹور ایپس بند",
                "مائیکروسافٹ اسٹور کی ایپس کو استعمال نہ ہونے پر چلنے سے روکتا ہے۔ ریم اور سی پی یو بچتا ہے۔",
            ],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
                "GlobalUserDisabled",
                Dword(1),
            )]),
        ),
        t(
            "startup_delay_off",
            Background,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            ["Faster startup apps", "Removes the delay Windows adds before starting your startup programs after sign-in."],
            ["اسٹارٹ اپ ایپس جلدی", "سائن اِن کے بعد اسٹارٹ اپ پروگرام چلانے سے پہلے ونڈوز کی انتظار ختم کرتا ہے۔"],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize",
                "StartupDelayInMSec",
                Dword(0),
            )]),
        ),
        t(
            "error_reporting_off",
            Background,
            Some(Balanced),
            LowRisk,
            false,
            ALL,
            [
                "Windows Error Reporting off",
                "Stops Windows from collecting and uploading crash reports in the background after a program crashes.",
            ],
            ["ونڈوز ایرر رپورٹنگ بند", "کسی پروگرام کے کریش ہونے پر ونڈوز کو پس منظر میں رپورٹ بنانے اور بھیجنے سے روکتا ہے۔"],
            regs(vec![reg(Hklm, r"SOFTWARE\Microsoft\Windows\Windows Error Reporting", "Disabled", Dword(1))]),
        ),
        t(
            "compat_assistant_off",
            Background,
            Some(Balanced),
            LowRisk,
            false,
            ALL,
            [
                "Program Compatibility Assistant off",
                "Stops Windows from watching every program you launch for compatibility problems.",
            ],
            ["پروگرام کمپیٹیبلٹی اسسٹنٹ بند", "ونڈوز کو ہر چلنے والے پروگرام کی نگرانی سے روکتا ہے۔"],
            regs(vec![reg(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\AppCompat", "DisablePCA", Dword(1))]),
        ),
        t(
            "browser_background_off",
            Background,
            Some(Max),
            LowRisk,
            false,
            ALL,
            [
                "Browsers don't run after closing",
                "Stops Edge and Chrome from staying in memory after you close them. The browser will show \"managed by your organization\"; that is this setting.",
            ],
            [
                "براؤزر بند ہونے کے بعد نہ چلیں",
                "ایج اور کروم کو بند کرنے کے بعد میموری میں رہنے سے روکتا ہے۔ براؤزر میں \"managed by your organization\" لکھا آئے گا، یہ اسی سیٹنگ کی وجہ سے ہے۔",
            ],
            regs(vec![
                reg(Hklm, r"SOFTWARE\Policies\Microsoft\Edge", "StartupBoostEnabled", Dword(0)),
                reg(Hklm, r"SOFTWARE\Policies\Microsoft\Edge", "BackgroundModeEnabled", Dword(0)),
                reg(Hklm, r"SOFTWARE\Policies\Google\Chrome", "BackgroundModeEnabled", Dword(0)),
            ]),
        ),
        // ---------------- Ads & tracking ----------------
        t(
            "ads_id_off",
            Privacy,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            ["Advertising ID off", "Stops apps from using your Windows advertising ID to show personalised ads."],
            ["ایڈورٹائزنگ آئی ڈی بند", "ایپس کو آپ کی ونڈوز ایڈورٹائزنگ آئی ڈی سے ذاتی اشتہار دکھانے سے روکتا ہے۔"],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
                "Enabled",
                Dword(0),
            )]),
        ),
        t(
            "suggestions_off",
            Privacy,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            [
                "Start menu ads and tips off",
                "Removes suggested apps in Start, tips, Settings suggestions, and stops Windows from silently installing promoted apps.",
            ],
            [
                "اسٹارٹ مینو کے اشتہار اور ٹپس بند",
                "اسٹارٹ میں تجویز کردہ ایپس، ٹپس اور سیٹنگز کی تجاویز ہٹاتا ہے، اور ونڈوز کو خود سے ایپس انسٹال کرنے سے روکتا ہے۔",
            ],
            regs(vec![
                reg(Hkcu, CDM, "SystemPaneSuggestionsEnabled", Dword(0)),
                reg(Hkcu, CDM, "SilentInstalledAppsEnabled", Dword(0)),
                reg(Hkcu, CDM, "SoftLandingEnabled", Dword(0)),
                reg(Hkcu, CDM, "SubscribedContent-338388Enabled", Dword(0)),
                reg(Hkcu, CDM, "SubscribedContent-338389Enabled", Dword(0)),
                reg(Hkcu, CDM, "SubscribedContent-353694Enabled", Dword(0)),
                reg(Hkcu, CDM, "SubscribedContent-353696Enabled", Dword(0)),
            ]),
        ),
        t(
            "tailored_experiences_off",
            Privacy,
            Some(Lite),
            LowRisk,
            false,
            ALL,
            [
                "Tailored experiences off",
                "Stops Microsoft from using your diagnostic data to show personalised tips and offers.",
            ],
            ["ذاتی تجربات بند", "مائیکروسافٹ کو آپ کے ڈیٹا سے ذاتی ٹپس اور آفرز دکھانے سے روکتا ہے۔"],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Privacy",
                "TailoredExperiencesWithDiagnosticDataEnabled",
                Dword(0),
            )]),
        ),
        t(
            "web_search_off",
            Privacy,
            Some(Balanced),
            LowRisk,
            true,
            ALL,
            [
                "Web results in Start search off",
                "Start menu search only looks at your PC, not Bing. Search becomes faster and stops sending what you type online.",
            ],
            [
                "اسٹارٹ سرچ میں ویب نتائج بند",
                "اسٹارٹ مینو سرچ صرف آپ کے پی سی میں ڈھونڈے گی، Bing میں نہیں۔ سرچ تیز ہوگی۔",
            ],
            regs(vec![
                reg(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Search", "BingSearchEnabled", Dword(0)),
                reg(Hkcu, r"Software\Microsoft\Windows\CurrentVersion\Search", "CortanaConsent", Dword(0)),
                reg(Hkcu, r"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", Dword(1)),
            ]),
        ),
        t(
            "telemetry_min",
            Privacy,
            Some(Balanced),
            LowRisk,
            false,
            ALL,
            [
                "Diagnostic data to minimum",
                "Sends only the required diagnostic data to Microsoft, the lowest level Windows Home and Pro allow.",
            ],
            ["تشخیصی ڈیٹا کم سے کم", "مائیکروسافٹ کو صرف ضروری تشخیصی ڈیٹا بھیجتا ہے، جو ونڈوز ہوم اور پرو میں سب سے کم سطح ہے۔"],
            regs(vec![reg(Hklm, r"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", Dword(1))]),
        ),
        // ---------------- Visuals ----------------
        t(
            "visual_effects_perf",
            Visuals,
            Some(Balanced),
            LowRisk,
            true,
            LOW,
            [
                "Visual effects for performance",
                "Turns off window animations and shadows. Windows looks plainer but feels faster on weak PCs.",
            ],
            ["کارکردگی کے لیے ویژول ایفیکٹس کم", "ونڈوز کی اینیمیشن اور سائے بند کرتا ہے۔ کمزور پی سی پر ونڈوز تیز محسوس ہوگی۔"],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
                "VisualFXSetting",
                Dword(2),
            )]),
        ),
        t(
            "transparency_off",
            Visuals,
            Some(Balanced),
            LowRisk,
            false,
            LOW_MID,
            ["Transparency effects off", "Makes the taskbar and Start menu solid. Saves GPU work on weak graphics."],
            ["شفافیت بند", "ٹاسک بار اور اسٹارٹ مینو کو ٹھوس بناتا ہے۔ کمزور گرافکس پر جی پی یو کا کام بچتا ہے۔"],
            regs(vec![reg(
                Hkcu,
                r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "EnableTransparency",
                Dword(0),
            )]),
        ),
        t(
            "menu_delay_short",
            Visuals,
            Some(Lite),
            LowRisk,
            true,
            ALL,
            ["Snappier menus", "Menus open after 100 ms instead of 400 ms."],
            ["تیز مینو", "مینو 400 کے بجائے 100 ملی سیکنڈ میں کھلتے ہیں۔"],
            regs(vec![reg(Hkcu, r"Control Panel\Desktop", "MenuShowDelay", Str("100".into()))]),
        ),
        // ---------------- Network ----------------
        t(
            "network_throttling_off",
            Network,
            Some(Max),
            Medium,
            true,
            ALL,
            [
                "Network throttling off",
                "Stops Windows from limiting network packets while media plays. Helps some online games; gains vary.",
            ],
            [
                "نیٹ ورک تھروٹلنگ بند",
                "میڈیا چلنے کے دوران ونڈوز کو نیٹ ورک پیکٹس محدود کرنے سے روکتا ہے۔ کچھ آن لائن گیمز میں مدد؛ فائدہ ہر پی سی پر مختلف۔",
            ],
            regs(vec![reg(Hklm, MM_PROFILE, "NetworkThrottlingIndex", Dword(0xFFFF_FFFF))]),
        ),
        // ---------------- Storage ----------------
        t(
            "ntfs_last_access_off",
            Storage,
            Some(Balanced),
            LowRisk,
            true,
            ALL,
            [
                "Skip last-access timestamps",
                "Windows stops writing a timestamp every time a file is read, which saves disk work while games load.",
            ],
            ["آخری رسائی کا وقت نہ لکھیں", "ہر فائل پڑھنے پر ونڈوز وقت لکھنا بند کرتی ہے، جس سے گیم لوڈ ہوتے وقت ڈسک کا کام بچتا ہے۔"],
            regs(vec![reg(
                Hklm,
                r"SYSTEM\CurrentControlSet\Control\FileSystem",
                "NtfsDisableLastAccessUpdate",
                Dword(0x8000_0001),
            )]),
        ),
    ]
}

pub fn recommended_ids(class: PcClass) -> Vec<String> {
    preset_ids(Preset::Balanced, class)
}

/// Tweaks a preset applies on a PC of this class.
pub fn preset_ids(preset: Preset, class: PcClass) -> Vec<String> {
    catalog()
        .into_iter()
        .filter(|t| t.preset.is_some_and(|p| p <= preset) && t.recommended_for.contains(&class))
        .map(|t| t.id)
        .collect()
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
    fn phase_one_ids_still_exist() {
        // Users have these in snapshots.json; removing one would orphan their revert data.
        let ids: Vec<String> = catalog().into_iter().map(|t| t.id).collect();
        for id in [
            "power_high",
            "game_mode",
            "game_dvr_off",
            "multimedia_games",
            "mouse_accel_off",
            "background_apps_off",
            "visual_effects_perf",
        ] {
            assert!(ids.iter().any(|i| i == id), "{id} missing");
        }
    }

    #[test]
    fn never_touches_forbidden_areas() {
        let forbidden = [
            "defender",
            "windowsupdate",
            "policies\\system",
            "enablelua",
            "\\services\\",
            "featuresettingsoverride",
            "firewall",
            "smartscreen",
        ];
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
        reg.set(Hive::Hkcu, r"Control Panel\Desktop", "MenuShowDelay", &RegValue::Str("400".into())).unwrap();
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
    fn state_diff_script_covers_every_catalog_value() {
        // The Sandbox end-to-end test only proves what this script records.
        let script = include_str!("../../scripts/state-diff.ps1");
        for t in catalog() {
            if let TweakKind::Registry(changes) = &t.kind {
                for c in changes {
                    assert!(script.contains(&format!("'{}'", c.name)), "state-diff.ps1 is missing {} ({})", c.name, t.id);
                }
            }
        }
    }

    #[test]
    fn presets_are_cumulative() {
        for class in [PcClass::Low, PcClass::Mid, PcClass::High] {
            let lite = preset_ids(Preset::Lite, class);
            let bal = preset_ids(Preset::Balanced, class);
            let max = preset_ids(Preset::Max, class);
            assert!(lite.iter().all(|i| bal.contains(i)));
            assert!(bal.iter().all(|i| max.contains(i)));
            assert!(lite.len() < bal.len() && bal.len() < max.len());
        }
    }

    #[test]
    fn fixes_are_never_in_a_preset() {
        for class in [PcClass::Low, PcClass::Mid, PcClass::High] {
            assert!(!preset_ids(Preset::Max, class).contains(&"mpo_off".to_string()));
        }
    }

    #[test]
    fn low_pcs_get_more_recommendations_than_high() {
        assert!(recommended_ids(PcClass::Low).len() > recommended_ids(PcClass::High).len());
    }
}
