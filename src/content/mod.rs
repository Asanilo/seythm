use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CatalogLoadError {
    #[error("failed to read catalog file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse catalog TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

pub const DEFAULT_IMPORTED_CATALOG_PATH: &str = ".code_m/imports/catalog.toml";
pub const DEFAULT_IMPORTED_ROOT: &str = ".code_m/imports";
pub const LEGACY_IMPORTED_ROOT: &str = ".superpowers/code_m_imports";
pub const IMPORTED_ORIGIN_TYPE: &str = "osu!mania import";

#[derive(Debug, Clone)]
pub struct SongCatalog {
    songs: Vec<SongCatalogEntry>,
}

impl SongCatalog {
    pub fn songs(&self) -> &[SongCatalogEntry] {
        &self.songs
    }
}

#[derive(Debug, Clone)]
pub struct SongCatalogEntry {
    id: String,
    title: String,
    artist: String,
    chart_name: String,
    difficulty: u8,
    bpm: u16,
    mood: String,
    chart_path: PathBuf,
    audio_path: Option<PathBuf>,
    artwork_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSongCatalog {
    pub songs: Vec<ImportedSongCatalogEntry>,
}

impl ImportedSongCatalog {
    pub fn songs(&self) -> &[ImportedSongCatalogEntry] {
        &self.songs
    }

    pub fn upsert(&mut self, entry: ImportedSongCatalogEntry) {
        if let Some(slot) = self.songs.iter_mut().find(|song| song.id == entry.id) {
            *slot = entry;
        } else {
            self.songs.push(entry);
        }
    }

    pub fn song(&self, id: &str) -> Option<&ImportedSongCatalogEntry> {
        self.songs.iter().find(|song| song.id == id)
    }
}

impl Default for ImportedSongCatalog {
    fn default() -> Self {
        Self { songs: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSongCatalogEntry {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub chart_name: String,
    pub difficulty: u8,
    pub bpm: u16,
    pub mood: String,
    pub chart_path: PathBuf,
    pub audio_path: PathBuf,
    pub artwork_path: Option<PathBuf>,
    pub source_osu_path: PathBuf,
    pub source_folder: String,
    pub source_osu_filename: String,
    pub imported_at_unix_ms: u64,
    pub origin_type: String,
}

impl ImportedSongCatalogEntry {
    pub fn chart_path(&self) -> &Path {
        &self.chart_path
    }

    pub fn audio_path(&self) -> &Path {
        &self.audio_path
    }

    pub fn artwork_path(&self) -> Option<&Path> {
        self.artwork_path.as_deref()
    }

    pub fn source_osu_path(&self) -> &Path {
        &self.source_osu_path
    }
}

impl SongCatalogEntry {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn chart_name(&self) -> &str {
        &self.chart_name
    }

    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn mood(&self) -> &str {
        &self.mood
    }

    pub fn chart_path(&self) -> &Path {
        &self.chart_path
    }

    pub fn audio_path(&self) -> Option<&Path> {
        self.audio_path.as_deref()
    }

    pub fn artwork_path(&self) -> Option<&Path> {
        self.artwork_path.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct RawSongCatalog {
    #[serde(default)]
    songs: Vec<RawSongCatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct RawSongCatalogEntry {
    id: String,
    title: String,
    artist: String,
    chart_name: String,
    difficulty: u8,
    bpm: u16,
    mood: String,
    chart_path: PathBuf,
    #[serde(default)]
    audio_path: Option<PathBuf>,
    #[serde(default)]
    artwork_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct RawImportedSongCatalog {
    #[serde(default)]
    songs: Vec<RawImportedSongCatalogEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawImportedSongCatalogEntry {
    id: String,
    title: String,
    artist: String,
    chart_name: String,
    difficulty: u8,
    bpm: u16,
    mood: String,
    chart_path: PathBuf,
    audio_path: PathBuf,
    #[serde(default)]
    artwork_path: Option<PathBuf>,
    source_osu_path: PathBuf,
    source_folder: String,
    source_osu_filename: String,
    imported_at_unix_ms: u64,
    origin_type: String,
}

pub fn load_bundled_song_catalog() -> Result<SongCatalog, CatalogLoadError> {
    let assets_root = bundled_assets_root();
    let catalog_path = assets_root.join("songs/catalog.toml");
    load_song_catalog(&catalog_path, &assets_root)
}

pub fn default_import_root() -> PathBuf {
    PathBuf::from(DEFAULT_IMPORTED_ROOT)
}

pub fn prepare_default_import_root() -> Result<PathBuf, ImportedCatalogError> {
    prepare_import_root_with_legacy(DEFAULT_IMPORTED_ROOT, LEGACY_IMPORTED_ROOT)
}

pub fn prepare_import_root_with_legacy(
    default_root: impl AsRef<Path>,
    legacy_root: impl AsRef<Path>,
) -> Result<PathBuf, ImportedCatalogError> {
    let default_root = default_root.as_ref().to_path_buf();
    let legacy_root = legacy_root.as_ref().to_path_buf();

    if default_root.exists() {
        return Ok(default_root);
    }

    if legacy_root.exists() {
        if let Some(parent) = default_root.parent() {
            fs::create_dir_all(parent).map_err(|source| ImportedCatalogError::Write {
                path: parent.display().to_string(),
                source,
            })?;
        }
        fs::rename(&legacy_root, &default_root).map_err(|source| ImportedCatalogError::Write {
            path: default_root.display().to_string(),
            source,
        })?;
        return Ok(default_root);
    }

    fs::create_dir_all(&default_root).map_err(|source| ImportedCatalogError::Write {
        path: default_root.display().to_string(),
        source,
    })?;
    Ok(default_root)
}

pub fn imported_catalog_path(import_root: impl AsRef<Path>) -> PathBuf {
    import_root.as_ref().join("catalog.toml")
}

pub fn load_imported_song_catalog(
    import_root: impl AsRef<Path>,
) -> Result<ImportedSongCatalog, ImportedCatalogError> {
    let import_root = import_root.as_ref();
    let catalog_path = imported_catalog_path(import_root);
    match fs::read_to_string(&catalog_path) {
        Ok(raw) => {
            let raw_catalog: RawImportedSongCatalog = toml::from_str(&raw)?;
            Ok(ImportedSongCatalog {
                songs: raw_catalog
                    .songs
                    .into_iter()
                    .map(|song| ImportedSongCatalogEntry {
                        id: song.id,
                        title: song.title,
                        artist: song.artist,
                        chart_name: song.chart_name,
                        difficulty: song.difficulty,
                        bpm: song.bpm,
                        mood: song.mood,
                        chart_path: resolve_catalog_path(import_root, song.chart_path),
                        audio_path: resolve_catalog_path(import_root, song.audio_path),
                        artwork_path: song
                            .artwork_path
                            .map(|path| resolve_catalog_path(import_root, path)),
                        source_osu_path: resolve_catalog_path(import_root, song.source_osu_path),
                        source_folder: song.source_folder,
                        source_osu_filename: song.source_osu_filename,
                        imported_at_unix_ms: song.imported_at_unix_ms,
                        origin_type: song.origin_type,
                    })
                    .collect(),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ImportedSongCatalog::default())
        }
        Err(source) => Err(ImportedCatalogError::Read {
            path: catalog_path.display().to_string(),
            source,
        }),
    }
}

pub fn save_imported_song_catalog(
    import_root: impl AsRef<Path>,
    catalog: &ImportedSongCatalog,
) -> Result<(), ImportedCatalogError> {
    let import_root = import_root.as_ref();
    save_imported_song_catalog_to_path(imported_catalog_path(import_root), import_root, catalog)
}

pub fn save_imported_song_catalog_to_path(
    catalog_path: impl AsRef<Path>,
    import_root: impl AsRef<Path>,
    catalog: &ImportedSongCatalog,
) -> Result<(), ImportedCatalogError> {
    let import_root = import_root.as_ref();
    let catalog_path = catalog_path.as_ref();
    if let Some(parent) = catalog_path.parent() {
        fs::create_dir_all(parent).map_err(|source| ImportedCatalogError::Write {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let raw = RawImportedSongCatalog {
        songs: catalog
            .songs
            .iter()
            .map(|song| {
                Ok(RawImportedSongCatalogEntry {
                    id: song.id.clone(),
                    title: song.title.clone(),
                    artist: song.artist.clone(),
                    chart_name: song.chart_name.clone(),
                    difficulty: song.difficulty,
                    bpm: song.bpm,
                    mood: song.mood.clone(),
                    chart_path: relative_catalog_path(import_root, &song.chart_path)?,
                    audio_path: relative_catalog_path(import_root, &song.audio_path)?,
                    artwork_path: song
                        .artwork_path
                        .as_ref()
                        .map(|path| relative_catalog_path(import_root, path))
                        .transpose()?,
                    source_osu_path: relative_catalog_path(import_root, &song.source_osu_path)?,
                    source_folder: song.source_folder.clone(),
                    source_osu_filename: song.source_osu_filename.clone(),
                    imported_at_unix_ms: song.imported_at_unix_ms,
                    origin_type: song.origin_type.clone(),
                })
            })
            .collect::<Result<Vec<_>, ImportedCatalogError>>()?,
    };

    let contents = toml::to_string_pretty(&raw)?;
    fs::write(catalog_path, contents).map_err(|source| ImportedCatalogError::Write {
        path: catalog_path.display().to_string(),
        source,
    })
}

fn resolve_catalog_path(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(normalize_catalog_relative_path(root, path))
    }
}

fn relative_catalog_path(root: &Path, path: &Path) -> Result<PathBuf, ImportedCatalogError> {
    if path.is_absolute() {
        path.strip_prefix(root).map(PathBuf::from).map_err(|_| {
            ImportedCatalogError::PathOutsideRoot {
                path: path.display().to_string(),
                root: root.display().to_string(),
            }
        })
    } else {
        Ok(normalize_catalog_relative_path(root, path.to_path_buf()))
    }
}

fn normalize_catalog_relative_path(root: &Path, path: PathBuf) -> PathBuf {
    let mut normalized = path;
    let default_root = Path::new(DEFAULT_IMPORTED_ROOT);
    let legacy_root = Path::new(LEGACY_IMPORTED_ROOT);

    while let Ok(stripped) = normalized.strip_prefix(root) {
        normalized = stripped.to_path_buf();
    }

    while let Ok(stripped) = normalized.strip_prefix(default_root) {
        normalized = stripped.to_path_buf();
    }

    while let Ok(stripped) = normalized.strip_prefix(legacy_root) {
        normalized = stripped.to_path_buf();
    }

    normalized
}

fn load_song_catalog(path: &Path, assets_root: &Path) -> Result<SongCatalog, CatalogLoadError> {
    let contents = fs::read_to_string(path).map_err(|source| CatalogLoadError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let raw: RawSongCatalog = toml::from_str(&contents)?;

    Ok(SongCatalog {
        songs: raw
            .songs
            .into_iter()
            .map(|song| SongCatalogEntry {
                id: song.id,
                title: song.title,
                artist: song.artist,
                chart_name: song.chart_name,
                difficulty: song.difficulty,
                bpm: song.bpm,
                mood: song.mood,
                chart_path: assets_root.join(song.chart_path),
                audio_path: song.audio_path.map(|path| assets_root.join(path)),
                artwork_path: song.artwork_path.map(|path| assets_root.join(path)),
            })
            .collect(),
    })
}

fn bundled_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[derive(Debug, thiserror::Error)]
pub enum ImportedCatalogError {
    #[error("failed to read imported catalog file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write imported catalog file {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse imported catalog TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize imported catalog TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("catalog path {path} is not inside import root {root}")]
    PathOutsideRoot { path: String, root: String },
}
