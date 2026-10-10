//! Temp file cleanup. This is the one feature that cannot be undone, so the UI
//! always shows a dry run first.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CleanupReport {
    pub files: u64,
    pub bytes: u64,
    /// Files in use or without permission, left alone.
    pub skipped: u64,
}

pub fn default_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![std::env::temp_dir()];
    if let Ok(root) = std::env::var("SystemRoot") {
        dirs.push(Path::new(&root).join("Temp"));
    }
    dirs.dedup();
    dirs
}

pub fn clean(dirs: &[PathBuf], older_than: Duration, dry_run: bool) -> CleanupReport {
    let mut report = CleanupReport::default();
    let cutoff = SystemTime::now().checked_sub(older_than).unwrap_or(SystemTime::UNIX_EPOCH);
    for d in dirs {
        walk(d, cutoff, dry_run, &mut report);
    }
    report
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
        let old_enough = meta.modified().map(|m| m < cutoff).unwrap_or(false);
        if !old_enough {
            continue;
        }
        if dry_run || std::fs::remove_file(&path).is_ok() {
            r.files += 1;
            r.bytes += meta.len();
        } else {
            r.skipped += 1;
        }
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
}
