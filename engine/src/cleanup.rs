//! Temp file cleanup. This is the one feature that cannot be undone, so the UI
//! always shows a dry run first.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CleanupReport {
    pub files: u64,
    pub bytes: u64,
    /// Files in use or without permission, left alone.
    pub skipped: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    UserTemp,
    WindowsTemp,
    ShaderCache,
    CrashDumps,
    ErrorReports,
    BrowserCache,
}

pub const ALL: [Category; 6] = [
    Category::UserTemp,
    Category::WindowsTemp,
    Category::ShaderCache,
    Category::CrashDumps,
    Category::ErrorReports,
    Category::BrowserCache,
];

/// A folder (or single file) to clean. If `process` is running, it is skipped
/// so we never pull a cache out from under an open browser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub path: PathBuf,
    pub process: Option<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CategoryReport {
    pub category: Option<Category>,
    pub files: u64,
    pub bytes: u64,
    pub skipped: u64,
    /// Programs that must be closed before this category can be fully cleaned.
    pub blocked_by: Vec<String>,
}

impl Category {
    /// Temp files younger than a day may belong to a running installer; caches and dumps are fair game at any age.
    pub fn min_age(self) -> Duration {
        match self {
            Category::UserTemp | Category::WindowsTemp => Duration::from_secs(24 * 3600),
            _ => Duration::ZERO,
        }
    }
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from)
}

/// Every profile folder under a Chromium "User Data" dir has the same cache layout.
fn chromium(user_data: PathBuf, process: &'static str, out: &mut Vec<Target>) {
    let Ok(entries) = std::fs::read_dir(&user_data) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == "Default" || name.starts_with("Profile ") {
            for sub in ["Cache", "Code Cache", "GPUCache"] {
                out.push(Target { path: e.path().join(sub), process: Some(process) });
            }
        }
    }
}

pub fn targets(cat: Category) -> Vec<Target> {
    let local = env_path("LOCALAPPDATA");
    let root = env_path("SystemRoot");
    let program_data = env_path("ProgramData");
    let plain = |p: Option<PathBuf>| p.map(|path| Target { path, process: None });
    let mut out: Vec<Target> = vec![];
    match cat {
        Category::UserTemp => out.push(Target { path: std::env::temp_dir(), process: None }),
        Category::WindowsTemp => out.extend(plain(root.map(|r| r.join("Temp")))),
        Category::ShaderCache => {
            if let Some(l) = &local {
                for sub in [
                    "D3DSCache",
                    r"NVIDIA\DXCache",
                    r"NVIDIA\GLCache",
                    r"AMD\DxCache",
                    r"AMD\DxcCache",
                    r"AMD\GLCache",
                    r"AMD\VkCache",
                    r"Intel\ShaderCache",
                ] {
                    out.push(Target { path: l.join(sub), process: None });
                }
            }
        }
        Category::CrashDumps => {
            out.extend(plain(local.as_ref().map(|l| l.join("CrashDumps"))));
            out.extend(plain(root.as_ref().map(|r| r.join("Minidump"))));
            out.extend(plain(root.as_ref().map(|r| r.join("MEMORY.DMP"))));
        }
        Category::ErrorReports => {
            out.extend(plain(local.as_ref().map(|l| l.join(r"Microsoft\Windows\WER"))));
            if let Some(p) = &program_data {
                out.push(Target { path: p.join(r"Microsoft\Windows\WER\ReportArchive"), process: None });
                out.push(Target { path: p.join(r"Microsoft\Windows\WER\ReportQueue"), process: None });
            }
        }
        Category::BrowserCache => {
            if let Some(l) = &local {
                chromium(l.join(r"Google\Chrome\User Data"), "chrome.exe", &mut out);
                chromium(l.join(r"Microsoft\Edge\User Data"), "msedge.exe", &mut out);
                chromium(l.join(r"BraveSoftware\Brave-Browser\User Data"), "brave.exe", &mut out);
                if let Ok(profiles) = std::fs::read_dir(l.join(r"Mozilla\Firefox\Profiles")) {
                    for p in profiles.flatten() {
                        out.push(Target { path: p.path().join("cache2"), process: Some("firefox.exe") });
                    }
                }
            }
        }
    }
    out.dedup();
    out
}

/// `running` holds exe names of running processes (any case).
pub fn clean_targets(targets: &[Target], older_than: Duration, running: &[String], dry_run: bool) -> CategoryReport {
    let mut report = CategoryReport::default();
    let cutoff = SystemTime::now().checked_sub(older_than).unwrap_or(SystemTime::UNIX_EPOCH);
    for t in targets {
        if let Some(p) = t.process
            && running.iter().any(|r| r.eq_ignore_ascii_case(p))
        {
            if !report.blocked_by.iter().any(|b| b == p) {
                report.blocked_by.push(p.to_string());
            }
            continue;
        }
        let mut r = CleanupReport::default();
        if t.path.is_file() {
            clean_file(&t.path, cutoff, dry_run, &mut r);
        } else {
            walk(&t.path, cutoff, dry_run, &mut r);
        }
        report.files += r.files;
        report.bytes += r.bytes;
        report.skipped += r.skipped;
    }
    report
}

pub fn clean_category(cat: Category, running: &[String], dry_run: bool) -> CategoryReport {
    CategoryReport { category: Some(cat), ..clean_targets(&targets(cat), cat.min_age(), running, dry_run) }
}

pub fn default_dirs() -> Vec<PathBuf> {
    [Category::UserTemp, Category::WindowsTemp].into_iter().flat_map(targets).map(|t| t.path).collect()
}

pub fn clean(dirs: &[PathBuf], older_than: Duration, dry_run: bool) -> CleanupReport {
    let mut report = CleanupReport::default();
    let cutoff = SystemTime::now().checked_sub(older_than).unwrap_or(SystemTime::UNIX_EPOCH);
    for d in dirs {
        walk(d, cutoff, dry_run, &mut report);
    }
    report
}

fn clean_file(path: &Path, cutoff: SystemTime, dry_run: bool, r: &mut CleanupReport) {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return };
    if is_link(&meta) || !meta.modified().map(|m| m < cutoff).unwrap_or(false) {
        return;
    }
    if dry_run || std::fs::remove_file(path).is_ok() {
        r.files += 1;
        r.bytes += meta.len();
    } else {
        r.skipped += 1;
    }
}

fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

fn walk(dir: &Path, cutoff: SystemTime, dry_run: bool, r: &mut CleanupReport) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            r.skipped += 1;
            continue;
        };
        // Never follow links or junctions out of the temp folder.
        if is_link(&meta) {
            continue;
        }
        if meta.is_dir() {
            walk(&path, cutoff, dry_run, r);
            continue;
        }
        clean_file(&path, cutoff, dry_run, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(dir: &Path, name: &str, size: usize, age: Duration) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, vec![b'x'; size]).unwrap();
        let f = std::fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(SystemTime::now() - age).unwrap();
        p
    }

    #[test]
    fn dry_run_deletes_nothing_but_reports_size() {
        let d = tempfile::tempdir().unwrap();
        let old = make(d.path(), "old.tmp", 100, Duration::from_secs(3 * 86400));
        let r = clean(&[d.path().to_path_buf()], Duration::from_secs(86400), true);
        assert_eq!(r, CleanupReport { files: 1, bytes: 100, skipped: 0 });
        assert!(old.exists());
    }

    #[test]
    fn deletes_old_files_and_keeps_new_ones_in_subfolders_too() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("sub")).unwrap();
        let old = make(&d.path().join("sub"), "old.tmp", 10, Duration::from_secs(3 * 86400));
        let new = make(d.path(), "new.tmp", 10, Duration::from_secs(60));
        let r = clean(&[d.path().to_path_buf()], Duration::from_secs(86400), false);
        assert_eq!(r.files, 1);
        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn target_owned_by_running_process_is_skipped_and_reported() {
        let d = tempfile::tempdir().unwrap();
        let f = make(d.path(), "cache_entry", 50, Duration::from_secs(10));
        let t = [Target { path: d.path().to_path_buf(), process: Some("chrome.exe") }];
        let r = clean_targets(&t, Duration::ZERO, &["Chrome.exe".into()], false);
        assert_eq!(r.files, 0);
        assert_eq!(r.blocked_by, vec!["chrome.exe".to_string()]);
        assert!(f.exists());
        let r = clean_targets(&t, Duration::ZERO, &[], false);
        assert_eq!((r.files, r.bytes), (1, 50));
        assert!(!f.exists());
    }

    #[test]
    fn single_file_target_and_missing_paths_are_handled() {
        let d = tempfile::tempdir().unwrap();
        let dump = make(d.path(), "MEMORY.DMP", 70, Duration::from_secs(10));
        let t = [
            Target { path: dump.clone(), process: None },
            Target { path: d.path().join("does-not-exist"), process: None },
        ];
        let r = clean_targets(&t, Duration::ZERO, &[], true);
        assert_eq!((r.files, r.bytes, r.skipped), (1, 70, 0));
        assert!(dump.exists());
    }

    #[test]
    fn chromium_profiles_expand_to_cache_folders() {
        let d = tempfile::tempdir().unwrap();
        for p in ["Default", "Profile 2", "System Profile", "Crashpad"] {
            std::fs::create_dir(d.path().join(p)).unwrap();
        }
        let mut out = vec![];
        chromium(d.path().to_path_buf(), "chrome.exe", &mut out);
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|t| t.process == Some("chrome.exe")));
    }

    #[test]
    fn temp_categories_keep_a_day_of_files_caches_do_not() {
        assert_eq!(Category::UserTemp.min_age(), Duration::from_secs(86400));
        assert_eq!(Category::ShaderCache.min_age(), Duration::ZERO);
    }
}
