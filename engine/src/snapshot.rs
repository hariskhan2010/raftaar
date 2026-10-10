//! Persistent record of what every applied tweak replaced.
//! Written atomically (temp file + rename) so a crash never leaves half a file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::tweak::RegChange;
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prior {
    Registry(Vec<RegChange>),
    PowerPlan { previous_guid: String, created_guid: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub tweak_id: String,
    pub prior: Prior,
    /// Unix seconds. Missing in snapshots written by v0.1.
    #[serde(default)]
    pub applied_at: Option<u64>,
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Debug)]
pub struct SnapshotStore {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl SnapshotStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let entries = match std::fs::read_to_string(&path) {
            Ok(s) if !s.trim().is_empty() => serde_json::from_str(&s)?,
            Ok(_) => Vec::new(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(SnapshotStore { path, entries })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tweak_id == id)
    }

    pub fn push(&mut self, entry: Entry) -> Result<()> {
        self.entries.push(entry);
        self.save()
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        self.entries.retain(|e| e.tweak_id != id);
        self.save()
    }

    fn save(&self) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.entries)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
