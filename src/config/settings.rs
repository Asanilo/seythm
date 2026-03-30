use crate::config::keymap::Keymap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_THEME_PATH: &str = "builtin:mocha-shell";
pub const BUILTIN_THEME_PREFIX: &str = "builtin:";
pub const DEFAULT_SETTINGS_PATH: &str = ".superpowers/code_m_settings.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub keymap: Keymap,
    pub theme_path: String,
    #[serde(default = "default_startup_splash_enabled")]
    pub startup_splash_enabled: bool,
    #[serde(default = "default_metronome_enabled")]
    pub metronome_enabled: bool,
    #[serde(default)]
    pub global_offset_ms: i32,
    #[serde(default)]
    pub input_offset_ms: i32,
    #[serde(default = "default_music_volume")]
    pub music_volume: u8,
    #[serde(default = "default_hit_sound_volume")]
    pub hit_sound_volume: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            keymap: Keymap::default(),
            theme_path: DEFAULT_THEME_PATH.to_string(),
            startup_splash_enabled: default_startup_splash_enabled(),
            metronome_enabled: default_metronome_enabled(),
            global_offset_ms: 0,
            input_offset_ms: 0,
            music_volume: default_music_volume(),
            hit_sound_volume: default_hit_sound_volume(),
        }
    }
}

impl Settings {
    pub fn theme_path_buf(&self) -> PathBuf {
        Path::new(&self.theme_path).to_path_buf()
    }

    pub fn builtin_theme_name(&self) -> Option<&str> {
        builtin_theme_name(&self.theme_path)
    }

    pub fn is_builtin_theme(&self) -> bool {
        self.builtin_theme_name().is_some()
    }
}

pub fn load_default_settings() -> Settings {
    Settings::default()
}

pub fn default_settings_path() -> PathBuf {
    PathBuf::from(DEFAULT_SETTINGS_PATH)
}

pub fn load_settings(path: impl AsRef<Path>) -> Result<Settings, SettingsIoError> {
    let path = path.as_ref();
    match fs::read_to_string(path) {
        Ok(raw) => Ok(toml::from_str(&raw)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(source) => Err(SettingsIoError::Read {
            path: path.display().to_string(),
            source,
        }),
    }
}

pub fn save_settings(path: impl AsRef<Path>, settings: &Settings) -> Result<(), SettingsIoError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SettingsIoError::Write {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let raw = toml::to_string_pretty(settings)?;
    fs::write(path, raw).map_err(|source| SettingsIoError::Write {
        path: path.display().to_string(),
        source,
    })
}

fn default_metronome_enabled() -> bool {
    true
}

fn default_startup_splash_enabled() -> bool {
    true
}

fn default_music_volume() -> u8 {
    80
}

fn default_hit_sound_volume() -> u8 {
    70
}

fn builtin_theme_name(theme_path: &str) -> Option<&str> {
    let name = theme_path.strip_prefix(BUILTIN_THEME_PREFIX)?;
    if is_builtin_theme_name(name) {
        Some(name)
    } else {
        None
    }
}

fn is_builtin_theme_name(name: &str) -> bool {
    matches!(
        name,
        "minimal-professional"
            | "mono-contrast"
            | "mocha-shell"
            | "ghostty-cold"
            | "soft-luxury"
            | "neon-stage"
    )
}

#[derive(Debug, Error)]
pub enum SettingsIoError {
    #[error("failed to read settings file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write settings file {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse settings TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize settings TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}
