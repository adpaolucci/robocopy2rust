use crate::model::RobocopyJob;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub updated_at: String,
    pub job: RobocopyJob,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunHistoryEntry {
    pub id: String,
    pub timestamp: String,
    pub job_name: String,
    pub source: String,
    pub destination: String,
    pub dry_run: bool,
    pub command_line: String,
    pub exit_code: i32,
    pub exit_label: String,
    pub is_success: bool,
    pub duration_secs: f64,
    pub files_copied: u64,
    pub files_failed: u64,
    pub bytes_copied: u64,
    pub summary_text: String,
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    root_dir: PathBuf,
}

impl ProfileStore {
    pub fn default_store() -> Self {
        let root_dir = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("robocopy2rust");
        Self { root_dir }
    }

    pub fn with_dir(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn profiles_path(&self) -> PathBuf {
        self.root_dir.join("profiles.json")
    }

    pub fn history_path(&self) -> PathBuf {
        self.root_dir.join("history.json")
    }

    pub fn load_profiles(&self) -> Result<Vec<SavedProfile>> {
        let path = self.profiles_path();
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path)?;
        let list: Vec<SavedProfile> = serde_json::from_str(&text)?;
        Ok(list)
    }

    pub fn save_profiles(&self, profiles: &[SavedProfile]) -> Result<()> {
        atomic_write_json(&self.profiles_path(), profiles)
    }

    pub fn upsert_profile(&self, name: &str, description: &str, mut job: RobocopyJob) -> Result<Vec<SavedProfile>> {
        let clean_name = name.trim();
        job.name = clean_name.to_string();
        job.description = description.trim().to_string();

        let mut list = self.load_profiles()?;
        let now = Utc::now().to_rfc3339();
        let id = slugify(clean_name);

        if let Some(existing) = list
            .iter_mut()
            .find(|p| p.id.eq_ignore_ascii_case(&id) || p.name.eq_ignore_ascii_case(clean_name))
        {
            existing.name = clean_name.to_string();
            existing.description = description.trim().to_string();
            existing.updated_at = now;
            existing.job = job;
        } else {
            list.push(SavedProfile {
                id,
                name: clean_name.to_string(),
                description: description.trim().to_string(),
                updated_at: now,
                job,
            });
        }
        list.sort_by_key(|a| a.name.to_ascii_lowercase());
        self.save_profiles(&list)?;
        Ok(list)
    }

    pub fn delete_profile(&self, id_or_name: &str) -> Result<(bool, Vec<SavedProfile>)> {
        let mut list = self.load_profiles()?;
        let before = list.len();
        list.retain(|p| !p.id.eq_ignore_ascii_case(id_or_name) && !p.name.eq_ignore_ascii_case(id_or_name));
        let removed = list.len() != before;
        if removed {
            self.save_profiles(&list)?;
        }
        Ok((removed, list))
    }

    pub fn find_profile(&self, id_or_name: &str) -> Result<Option<SavedProfile>> {
        let list = self.load_profiles()?;
        Ok(list
            .into_iter()
            .find(|p| p.id.eq_ignore_ascii_case(id_or_name) || p.name.eq_ignore_ascii_case(id_or_name)))
    }

    pub fn load_history(&self) -> Result<Vec<RunHistoryEntry>> {
        let path = self.history_path();
        if !path.is_file() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(path)?;
        let list: Vec<RunHistoryEntry> = serde_json::from_str(&text)?;
        Ok(list)
    }

    pub fn append_history(&self, entry: RunHistoryEntry) -> Result<Vec<RunHistoryEntry>> {
        let mut list = self.load_history()?;
        list.insert(0, entry);
        list.truncate(200);
        atomic_write_json(&self.history_path(), &list)?;
        Ok(list)
    }

    pub fn clear_history(&self) -> Result<()> {
        atomic_write_json(&self.history_path(), &Vec::<RunHistoryEntry>::new())
    }
}

fn slugify(name: &str) -> String {
    use sha2::{Digest, Sha256};

    let clean = name.trim();
    let mut out = String::new();
    let mut dropped_non_ascii = false;
    for ch in clean.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            if !ch.is_ascii() {
                dropped_non_ascii = true;
            }
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() || dropped_non_ascii {
        let mut hasher = Sha256::new();
        hasher.update(clean.to_lowercase().as_bytes());
        let suffix = &crate::format::encode_hex_lower(&hasher.finalize())[..8];
        if trimmed.is_empty() {
            format!("profile-{suffix}")
        } else {
            format!("{trimmed}-{suffix}")
        }
    } else {
        trimmed
    }
}

fn atomic_write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn saves_updates_and_deletes_profiles_atomically() {
        let tmp = TempDir::new().unwrap();
        let store = ProfileStore::with_dir(tmp.path());

        let mut job = RobocopyJob {
            source: r"C:\A".to_string(),
            destination: r"D:\B".to_string(),
            ..Default::default()
        };

        let list = store
            .upsert_profile("Nightly Docs", "Documents backup", job.clone())
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "nightly-docs");

        job.copy.mirror = true;
        let list = store.upsert_profile("Nightly Docs", "Updated mirror", job).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].job.copy.mirror);

        let (removed, list) = store.delete_profile("nightly-docs").unwrap();
        assert!(removed);
        assert!(list.is_empty());
    }

    #[test]
    fn preserves_corrupt_store_and_avoids_non_ascii_slug_collisions() {
        let tmp = TempDir::new().unwrap();
        let store = ProfileStore::with_dir(tmp.path());

        let list1 = store.upsert_profile("café", "one", RobocopyJob::default()).unwrap();
        let list2 = store.upsert_profile("naïve", "two", RobocopyJob::default()).unwrap();
        assert_eq!(list2.len(), 2);
        assert_ne!(list1[0].id, list2.iter().find(|p| p.name == "naïve").unwrap().id);

        std::fs::write(store.profiles_path(), b"{corrupt json").unwrap();
        assert!(store.upsert_profile("another", "", RobocopyJob::default()).is_err());
        assert_eq!(std::fs::read_to_string(store.profiles_path()).unwrap(), "{corrupt json");
    }
}
