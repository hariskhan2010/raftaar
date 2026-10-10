//! Live system monitor: CPU, RAM, GPU load and the heaviest processes.
//! GPU load uses the same "GPU Engine" performance counters as Task Manager.

use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ProcUsage {
    pub name: String,
    pub pid: u32,
    pub ram_mb: f32,
    pub cpu_pct: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Stats {
    pub cpu_pct: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    /// `None` when the GPU counters are unavailable (old driver, or a VM).
    pub gpu_pct: Option<f32>,
    pub top: Vec<ProcUsage>,
}

/// Task Manager shows the busiest engine, not a sum. Per-engine values come per
/// process, so add up each engine type's instances, then take the maximum.
pub fn gpu_from_instances(instances: &[(String, f64)]) -> f32 {
    use std::collections::HashMap;
    let mut per_engine: HashMap<String, f64> = HashMap::new();
    for (name, v) in instances {
        // Instance names look like "pid_1234_luid_0x..._phys_0_eng_3_engtype_3D".
        let engine = name.split("_eng_").nth(1).unwrap_or(name).to_string();
        *per_engine.entry(engine).or_default() += v;
    }
    per_engine.values().copied().fold(0.0, f64::max).clamp(0.0, 100.0) as f32
}

pub fn top_by_ram(mut procs: Vec<ProcUsage>, n: usize) -> Vec<ProcUsage> {
    procs.sort_by(|a, b| b.ram_mb.total_cmp(&a.ram_mb));
    // One row per program: Chrome's 30 processes count as one Chrome.
    let mut merged: Vec<ProcUsage> = vec![];
    for p in procs {
        if let Some(m) = merged.iter_mut().find(|m| m.name.eq_ignore_ascii_case(&p.name)) {
            m.ram_mb += p.ram_mb;
            m.cpu_pct += p.cpu_pct;
        } else {
            merged.push(p);
        }
    }
    merged.sort_by(|a, b| b.ram_mb.total_cmp(&a.ram_mb));
    merged.truncate(n);
    merged
}

#[cfg(windows)]
mod pdh {
    use windows_sys::Win32::System::Performance::*;

    pub struct GpuCounter {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    // The PDH handles are only touched behind the Monitor's mutex.
    unsafe impl Send for GpuCounter {}

    impl GpuCounter {
        pub fn new() -> Option<Self> {
            let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage\0".encode_utf16().collect();
            unsafe {
                let mut query = 0 as PDH_HQUERY;
                if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 {
                    return None;
                }
                let mut counter = 0 as PDH_HCOUNTER;
                if PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) != 0 {
                    PdhCloseQuery(query);
                    return None;
                }
                PdhCollectQueryData(query);
                Some(GpuCounter { query, counter })
            }
        }

        pub fn sample(&self) -> Option<Vec<(String, f64)>> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let (mut size, mut count) = (0u32, 0u32);
                PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut());
                if size == 0 {
                    return Some(vec![]);
                }
                let mut buf = vec![0u8; size as usize];
                let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                    return None;
                }
                let mut out = Vec::with_capacity(count as usize);
                for i in 0..count as usize {
                    let item = &*items.add(i);
                    let mut len = 0;
                    while *item.szName.add(len) != 0 {
                        len += 1;
                    }
                    let name = String::from_utf16_lossy(std::slice::from_raw_parts(item.szName, len));
                    if item.FmtValue.CStatus == 0 {
                        out.push((name, item.FmtValue.Anonymous.doubleValue));
                    }
                }
                Some(out)
            }
        }
    }

    impl Drop for GpuCounter {
        fn drop(&mut self) {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}

/// Keeps state between samples: CPU % and GPU % are deltas since the last call.
#[cfg(windows)]
pub struct Monitor {
    sys: sysinfo::System,
    gpu: Option<pdh::GpuCounter>,
}

#[cfg(windows)]
impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
impl Monitor {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        Monitor { sys, gpu: pdh::GpuCounter::new() }
    }

    pub fn sample(&mut self) -> Stats {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_memory().with_cpu(),
        );
        let cores = self.sys.cpus().len().max(1) as f32;
        let procs = self
            .sys
            .processes()
            .values()
            .map(|p| ProcUsage {
                name: p.name().to_string_lossy().into_owned(),
                pid: p.pid().as_u32(),
                ram_mb: p.memory() as f32 / 1024.0 / 1024.0,
                cpu_pct: p.cpu_usage() / cores,
            })
            .collect();
        let gb = 1024f32.powi(3);
        Stats {
            cpu_pct: self.sys.global_cpu_usage(),
            ram_used_gb: self.sys.used_memory() as f32 / gb,
            ram_total_gb: self.sys.total_memory() as f32 / gb,
            gpu_pct: self.gpu.as_ref().and_then(|g| g.sample()).map(|v| gpu_from_instances(&v)),
            top: top_by_ram(procs, 8),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_load_is_busiest_engine_summed_across_processes() {
        let v = vec![
            ("pid_1_luid_0x1_phys_0_eng_0_engtype_3D".to_string(), 30.0),
            ("pid_2_luid_0x1_phys_0_eng_0_engtype_3D".to_string(), 25.0),
            ("pid_2_luid_0x1_phys_0_eng_5_engtype_VideoDecode".to_string(), 40.0),
        ];
        assert_eq!(gpu_from_instances(&v), 55.0);
        assert_eq!(gpu_from_instances(&[]), 0.0);
    }

    #[test]
    fn top_processes_merge_same_program() {
        let p = |n: &str, ram: f32| ProcUsage { name: n.into(), pid: 0, ram_mb: ram, cpu_pct: 1.0 };
        let top = top_by_ram(vec![p("chrome.exe", 300.0), p("game.exe", 500.0), p("Chrome.exe", 300.0), p("x", 1.0)], 2);
        assert_eq!(top[0].name, "chrome.exe");
        assert_eq!(top[0].ram_mb, 600.0);
        assert_eq!(top[1].name, "game.exe");
        assert_eq!(top.len(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn real_monitor_samples_plausible_values() {
        let mut m = Monitor::new();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let s = m.sample();
        assert!(s.ram_total_gb > 1.0 && s.ram_used_gb > 0.1, "{s:?}");
        assert!(!s.top.is_empty());
    }
}
