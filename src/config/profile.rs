use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::settings::SettingsIoError;

pub const DEFAULT_PROFILE_PATH: &str = ".superpowers/code_m_profile.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResultProfile {
    #[serde(default)]
    pub songs: BTreeMap<String, ProfileRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileRecord {
    pub best_score: u32,
    pub best_accuracy: f32,
    pub last_score: u32,
    pub last_accuracy: f32,
    pub play_count: u32,
}

impl ResultProfile {
    pub fn record_run(&mut self, song_id: &str, score: u32, accuracy: f32) -> ProfileRecord {
        let entry = self.songs.entry(song_id.to_string()).or_default();
        entry.last_score = score;
        entry.last_accuracy = accuracy;
        entry.play_count = entry.play_count.saturating_add(1);
        if score >= entry.best_score {
            entry.best_score = score;
            entry.best_accuracy = accuracy.max(entry.best_accuracy);
        }
        entry.clone()
    }

    pub fn song(&self, song_id: &str) -> Option<&ProfileRecord> {
        self.songs.get(song_id)
    }
}

pub fn default_profile_path() -> PathBuf {
    PathBuf::from(DEFAULT_PROFILE_PATH)
}

pub fn load_profile(path: impl AsRef<Path>) -> Result<ResultProfile, SettingsIoError> {
    let path = path.as_ref();
    match fs::read_to_string(path) {
        Ok(raw) => Ok(toml::from_str(&raw)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ResultProfile::default()),
        Err(source) => Err(SettingsIoError::Read {
            path: path.display().to_string(),
            source,
        }),
    }
}

pub fn save_profile(
    path: impl AsRef<Path>,
    profile: &ResultProfile,
) -> Result<(), SettingsIoError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SettingsIoError::Write {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let raw = toml::to_string_pretty(profile)?;
    fs::write(path, raw).map_err(|source| SettingsIoError::Write {
        path: path.display().to_string(),
        source,
    })
}
