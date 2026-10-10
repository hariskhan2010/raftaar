//! Read-only system scan and PC classification.

use serde::Serialize;

use crate::tweak::PcClass;

#[derive(Clone, Debug, Serialize)]
pub struct SystemInfo {
    pub cpu: String,
    pub cores: usize,
    pub threads: usize,
    pub ram_gb: f32,
    pub gpu_name: String,
    pub vram_gb: f32,
    pub windows: String,
    pub startup_apps: u32,
    pub process_count: u32,
    pub class: PcClass,
}

/// Low: ≤ 8 GB RAM or no dedicated GPU (integrated GPUs report < 2 GB).
/// High: ≥ 16 GB RAM and ≥ 6 GB VRAM. Everything else is Mid.
pub fn classify(ram_gb: f32, vram_gb: f32) -> PcClass {
    // Windows reports a little less than the installed RAM (8 GB shows ~7.8).
    if ram_gb < 8.5 || vram_gb < 2.0 {
        PcClass::Low
    } else if ram_gb >= 15.5 && vram_gb >= 6.0 {
        PcClass::High
    } else {
        PcClass::Mid
    }
}

#[cfg(windows)]
pub fn startup_app_count() -> u32 {
    use winreg::RegKey;
    use winreg::enums::*;
    let run = r"Software\Microsoft\Windows\CurrentVersion\Run";
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
        .into_iter()
        .filter_map(|h| RegKey::predef(h).open_subkey_with_flags(run, KEY_READ).ok())
        .filter_map(|k| k.query_info().ok())
        .map(|i| i.values)
        .sum()
}

/// Best GPU by VRAM, read from the display adapter class key. WMI's
/// AdapterRAM is a 32-bit field and caps at 4 GB, so it can't be trusted.
#[cfg(windows)]
pub fn gpu() -> (String, f32) {
    use winreg::RegKey;
    use winreg::enums::*;
    const CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
    let mut best = (String::from("Unknown GPU"), 0.0f32);
    let Ok(class) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(CLASS, KEY_READ) else {
        return best;
    };
    for sub in class.enum_keys().flatten() {
        let Ok(k) = class.open_subkey_with_flags(&sub, KEY_READ) else { continue };
        let Ok(name) = k.get_value::<String, _>("DriverDesc") else { continue };
        let bytes = ["HardwareInformation.qwMemorySize", "HardwareInformation.MemorySize"]
            .iter()
            .find_map(|v| k.get_raw_value(v).ok())
            .map(|raw| raw.bytes.to_vec())
            .unwrap_or_default();
        let mem = match bytes.len() {
            8 => u64::from_le_bytes(bytes[..8].try_into().unwrap()),
            4 => u32::from_le_bytes(bytes[..4].try_into().unwrap()) as u64,
            _ => 0,
        };
        let gb = mem as f32 / 1024f32.powi(3);
        if gb > best.1 || best.1 == 0.0 {
            best = (name, gb);
        }
    }
    best
}

#[cfg(windows)]
pub fn scan() -> SystemInfo {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let ram_gb = sys.total_memory() as f32 / 1024f32.powi(3);
    let (gpu_name, vram_gb) = gpu();
    SystemInfo {
        cpu: sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
        cores: System::physical_core_count().unwrap_or(0),
        threads: sys.cpus().len(),
        ram_gb,
        gpu_name,
        vram_gb,
        windows: System::long_os_version().unwrap_or_default(),
        startup_apps: startup_app_count(),
        process_count: sys.processes().len() as u32,
        class: classify(ram_gb, vram_gb),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_gb_with_integrated_gpu_is_low() {
        assert_eq!(classify(3.9, 0.5), PcClass::Low);
    }

    #[test]
    fn sixteen_gb_with_8gb_vram_is_high() {
        assert_eq!(classify(15.9, 8.0), PcClass::High);
    }

    #[test]
    fn twelve_gb_with_4gb_vram_is_mid() {
        assert_eq!(classify(11.9, 4.0), PcClass::Mid);
    }

    #[test]
    fn plenty_of_ram_but_no_dedicated_gpu_is_low() {
        assert_eq!(classify(32.0, 0.5), PcClass::Low);
    }

    #[cfg(windows)]
    #[test]
    fn real_scan_returns_plausible_values() {
        let s = scan();
        assert!(s.ram_gb > 1.0, "{s:?}");
        assert!(s.threads >= 1 && s.process_count > 10, "{s:?}");
    }
}
