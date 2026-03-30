use crate::chart::model::{Chart, ChartMetadata, HoldNote, Note, NoteKind, TapNote, TimingPoint};
use crate::content::{
    load_imported_song_catalog, save_imported_song_catalog_to_path, ImportedSongCatalogEntry,
    IMPORTED_ORIGIN_TYPE,
};
use crate::osu::model::{OsuBeatmap, OsuHitObject};
use crate::osu::parser::OsuParseError;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OsuImportError {
    #[error("unsupported key count {key_count}; expected 6")]
    UnsupportedKeyCount { key_count: u8 },
    #[error("unsupported hit object lane {lane}; expected 0..=5")]
    UnsupportedLane { lane: u8 },
    #[error("negative timing point start time {start_time_ms} is not supported")]
    NegativeTimingPointStart { start_time_ms: i32 },
    #[error("negative note time {time_ms} for lane {lane} is not supported")]
    NegativeNoteTime { lane: u8, time_ms: i32 },
    #[error(
        "invalid hold note duration: end {end_time_ms} must be greater than start {start_time_ms}"
    )]
    InvalidHoldDuration {
        start_time_ms: i32,
        end_time_ms: i32,
    },
    #[error(
        "negative hold time start {start_time_ms} or end {end_time_ms} for lane {lane} is not supported"
    )]
    NegativeHoldTime {
        lane: u8,
        start_time_ms: i32,
        end_time_ms: i32,
    },
    #[error("no usable timing points found in osu beatmap")]
    NoUsableTimingPoints,
    #[error("no usable osu!mania 6K beatmaps found in beatmap folder {folder}")]
    NoUsableBeatmaps { folder: String },
    #[error("missing audio file {path}")]
    MissingAudioFile { path: String },
    #[error("invalid audio filename {audio_filename}")]
    InvalidAudioFilename { audio_filename: String },
    #[error("storage failure at {path}: {message}")]
    StorageError { path: String, message: String },
}

pub fn convert_osu_mania_chart(imported: &OsuBeatmap) -> Result<Chart, OsuImportError> {
    if imported.key_count != 6 {
        return Err(OsuImportError::UnsupportedKeyCount {
            key_count: imported.key_count,
        });
    }

    let timing = convert_timing_points(&imported.timing_points)?;
    let mut notes = imported
        .hit_objects
        .iter()
        .map(convert_hit_object)
        .collect::<Result<Vec<_>, _>>()?;
    notes.sort_by(compare_notes);

    Ok(Chart {
        metadata: convert_metadata(&imported.metadata),
        timing,
        notes,
    })
}

pub fn import_osu_mania_folder(
    source_folder: impl AsRef<Path>,
    import_root: impl AsRef<Path>,
) -> Result<Vec<ImportedSongCatalogEntry>, OsuImportError> {
    let source_folder = source_folder.as_ref();
    let import_root = import_root.as_ref();
    let osu_files = find_osu_files(source_folder)?;
    let mut imported_entries = Vec::new();

    for source_osu_path in osu_files {
        match import_single_osu_file(source_folder, import_root, &source_osu_path) {
            Ok(Some(entry)) => imported_entries.push(entry),
            Ok(None) => {}
            Err(error) => return Err(error),
        }
    }

    if imported_entries.is_empty() {
        return Err(OsuImportError::NoUsableBeatmaps {
            folder: source_folder.display().to_string(),
        });
    }

    Ok(imported_entries)
}

fn convert_metadata(metadata: &crate::osu::model::OsuMetadata) -> ChartMetadata {
    ChartMetadata {
        title: metadata
            .title_unicode
            .as_deref()
            .or(metadata.title.as_deref())
            .unwrap_or_default()
            .to_string(),
        artist: metadata
            .artist_unicode
            .as_deref()
            .or(metadata.artist.as_deref())
            .unwrap_or_default()
            .to_string(),
        chart_name: metadata.version.clone().unwrap_or_default(),
        offset_ms: 0,
    }
}

fn convert_timing_points(
    points: &[crate::osu::model::OsuTimingPoint],
) -> Result<Vec<TimingPoint>, OsuImportError> {
    let mut converted: Vec<TimingPoint> = points
        .iter()
        .map(|point| {
            if point.time_ms < 0 {
                return Err(OsuImportError::NegativeTimingPointStart {
                    start_time_ms: point.time_ms,
                });
            }

            if point.uninherited && point.beat_length > 0.0 {
                Ok(Some(TimingPoint {
                    start_ms: point.time_ms as u32,
                    bpm: (60000.0 / point.beat_length) as f32,
                    beat_length: point.meter as u32,
                }))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    converted.sort_by(|left, right| left.start_ms.cmp(&right.start_ms));

    if converted.is_empty() {
        return Err(OsuImportError::NoUsableTimingPoints);
    }

    Ok(converted)
}

fn convert_hit_object(hit_object: &OsuHitObject) -> Result<Note, OsuImportError> {
    match hit_object {
        OsuHitObject::Tap { lane, time_ms } => {
            let lane = validate_lane(*lane)?;
            if *time_ms < 0 {
                return Err(OsuImportError::NegativeNoteTime {
                    lane,
                    time_ms: *time_ms,
                });
            }

            Ok(Note::Tap(TapNote {
                kind: NoteKind::Tap,
                time_ms: *time_ms as u32,
                lane,
            }))
        }
        OsuHitObject::Hold {
            lane,
            start_time_ms,
            end_time_ms,
        } => {
            let lane = validate_lane(*lane)?;
            if *start_time_ms < 0 || *end_time_ms < 0 {
                return Err(OsuImportError::NegativeHoldTime {
                    lane,
                    start_time_ms: *start_time_ms,
                    end_time_ms: *end_time_ms,
                });
            }

            if end_time_ms <= start_time_ms {
                return Err(OsuImportError::InvalidHoldDuration {
                    start_time_ms: *start_time_ms,
                    end_time_ms: *end_time_ms,
                });
            }

            Ok(Note::Hold(HoldNote {
                kind: NoteKind::Hold,
                start_ms: *start_time_ms as u32,
                end_ms: *end_time_ms as u32,
                lane,
            }))
        }
    }
}

fn validate_lane(lane: u8) -> Result<u8, OsuImportError> {
    if lane <= 5 {
        Ok(lane)
    } else {
        Err(OsuImportError::UnsupportedLane { lane })
    }
}

fn compare_notes(left: &Note, right: &Note) -> Ordering {
    left.timestamp_ms()
        .cmp(&right.timestamp_ms())
        .then_with(|| left.lane().cmp(&right.lane()))
}

fn find_osu_files(folder: &Path) -> Result<Vec<PathBuf>, OsuImportError> {
    let mut osu_files = Vec::new();
    let read_dir = fs::read_dir(folder).map_err(|source| OsuImportError::StorageError {
        path: folder.display().to_string(),
        message: source.to_string(),
    })?;

    for entry in read_dir {
        let entry = entry.map_err(|source| OsuImportError::StorageError {
            path: folder.display().to_string(),
            message: source.to_string(),
        })?;
        let path = entry.path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("osu"))
            .unwrap_or(false)
        {
            osu_files.push(path);
        }
    }

    Ok(osu_files)
}

fn import_single_osu_file(
    source_folder: &Path,
    import_root: &Path,
    source_osu_path: &Path,
) -> Result<Option<ImportedSongCatalogEntry>, OsuImportError> {
    let beatmap = match crate::osu::parser::parse_osu_file(source_osu_path) {
        Ok(beatmap) => beatmap,
        Err(error) if should_skip_parse_error(&error) => return Ok(None),
        Err(error) => {
            return Err(OsuImportError::StorageError {
                path: source_osu_path.display().to_string(),
                message: error.to_string(),
            })
        }
    };
    let chart = match convert_osu_mania_chart(&beatmap) {
        Ok(chart) => chart,
        Err(error) if should_skip_import_error(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let import_id = derive_import_id(&chart.metadata, beatmap.key_count, &beatmap.audio_filename);
    let final_dir = import_root.join(&import_id);
    let stage_dir =
        import_root
            .join(".staging")
            .join(format!("{}-{}", import_id, imported_timestamp_ms()));
    let stage_entry = match stage_import_assets(
        source_folder,
        source_osu_path,
        &beatmap,
        &chart,
        &stage_dir,
        &final_dir,
    ) {
        Ok(entry) => entry,
        Err(error) if should_skip_import_error(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    commit_import_entry(import_root, &final_dir, &stage_dir, &stage_entry)?;
    Ok(Some(stage_entry))
}

fn derive_import_id(metadata: &ChartMetadata, key_count: u8, audio_filename: &str) -> String {
    let raw = format!(
        "{}-{}-{}-{}k-{}",
        metadata.artist, metadata.title, metadata.chart_name, key_count, audio_filename
    );
    slugify(&raw)
}

fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;

    for ch in input.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }

    output.trim_matches('-').to_string()
}

fn imported_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn stage_import_assets(
    source_folder: &Path,
    source_osu_path: &Path,
    beatmap: &OsuBeatmap,
    chart: &Chart,
    stage_dir: &Path,
    final_dir: &Path,
) -> Result<ImportedSongCatalogEntry, OsuImportError> {
    if stage_dir.exists() {
        fs::remove_dir_all(stage_dir).map_err(|source| OsuImportError::StorageError {
            path: stage_dir.display().to_string(),
            message: source.to_string(),
        })?;
    }
    fs::create_dir_all(stage_dir).map_err(|source| OsuImportError::StorageError {
        path: stage_dir.display().to_string(),
        message: source.to_string(),
    })?;

    let chart_path = stage_dir.join("chart.toml");
    let chart_raw =
        toml::to_string_pretty(chart).map_err(|source| OsuImportError::StorageError {
            path: chart_path.display().to_string(),
            message: source.to_string(),
        })?;
    fs::write(&chart_path, chart_raw).map_err(|source| OsuImportError::StorageError {
        path: chart_path.display().to_string(),
        message: source.to_string(),
    })?;

    let audio_filename = Path::new(&beatmap.audio_filename)
        .file_name()
        .ok_or_else(|| OsuImportError::InvalidAudioFilename {
            audio_filename: beatmap.audio_filename.clone(),
        })?;
    let source_audio_path = source_folder.join(audio_filename);
    if !source_audio_path.exists() {
        return Err(OsuImportError::MissingAudioFile {
            path: source_audio_path.display().to_string(),
        });
    }
    let audio_path = stage_dir.join(audio_filename);
    fs::copy(&source_audio_path, &audio_path).map_err(|source| OsuImportError::StorageError {
        path: audio_path.display().to_string(),
        message: source.to_string(),
    })?;

    let artwork_path = discover_artwork_path(source_folder, source_osu_path, &source_audio_path);
    let copied_artwork_path = if let Some(source_artwork_path) = artwork_path.as_ref() {
        let artwork_filename =
            source_artwork_path
                .file_name()
                .ok_or_else(|| OsuImportError::StorageError {
                    path: source_artwork_path.display().to_string(),
                    message: "artwork filename is not valid".to_string(),
                })?;
        let copied = stage_dir.join(artwork_filename);
        fs::copy(source_artwork_path, &copied).map_err(|source| OsuImportError::StorageError {
            path: copied.display().to_string(),
            message: source.to_string(),
        })?;
        Some(copied)
    } else {
        None
    };

    let source_osu_filename = source_osu_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| OsuImportError::StorageError {
            path: source_osu_path.display().to_string(),
            message: "source .osu filename is not valid UTF-8".to_string(),
        })?;
    let copied_osu_path = stage_dir.join(source_osu_filename);
    fs::copy(source_osu_path, &copied_osu_path).map_err(|source| OsuImportError::StorageError {
        path: copied_osu_path.display().to_string(),
        message: source.to_string(),
    })?;

    let imported_at_unix_ms = imported_timestamp_ms();
    Ok(ImportedSongCatalogEntry {
        id: derive_import_id(&chart.metadata, beatmap.key_count, &beatmap.audio_filename),
        title: chart.metadata.title.clone(),
        artist: chart.metadata.artist.clone(),
        chart_name: chart.metadata.chart_name.clone(),
        difficulty: beatmap.key_count,
        bpm: chart
            .timing
            .first()
            .map(|timing| timing.bpm.round().clamp(0.0, u16::MAX as f32) as u16)
            .unwrap_or(0),
        mood: "Imported".to_string(),
        chart_path: final_dir.join("chart.toml"),
        audio_path: final_dir.join(audio_filename),
        artwork_path: copied_artwork_path.map(|path| {
            final_dir.join(
                path.file_name()
                    .expect("copied artwork path should always have a file name"),
            )
        }),
        source_osu_path: final_dir.join(source_osu_filename),
        source_folder: source_folder.display().to_string(),
        source_osu_filename: source_osu_filename.to_string(),
        imported_at_unix_ms,
        origin_type: IMPORTED_ORIGIN_TYPE.to_string(),
    })
}

fn commit_import_entry(
    import_root: &Path,
    final_dir: &Path,
    stage_dir: &Path,
    entry: &ImportedSongCatalogEntry,
) -> Result<(), OsuImportError> {
    let catalog_path = crate::content::imported_catalog_path(import_root);
    let temp_catalog_path = catalog_path.with_extension("toml.new");
    let mut catalog =
        load_imported_song_catalog(import_root).map_err(|error| OsuImportError::StorageError {
            path: catalog_path.display().to_string(),
            message: error.to_string(),
        })?;
    catalog.upsert(entry.clone());
    save_imported_song_catalog_to_path(&temp_catalog_path, import_root, &catalog).map_err(
        |error| OsuImportError::StorageError {
            path: temp_catalog_path.display().to_string(),
            message: error.to_string(),
        },
    )?;

    let backup_dir = final_dir.with_extension("previous");
    let had_existing = final_dir.exists();
    if had_existing {
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).map_err(|source| OsuImportError::StorageError {
                path: backup_dir.display().to_string(),
                message: source.to_string(),
            })?;
        }
        fs::rename(final_dir, &backup_dir).map_err(|source| OsuImportError::StorageError {
            path: final_dir.display().to_string(),
            message: source.to_string(),
        })?;
    }

    if let Err(error) = fs::rename(stage_dir, final_dir) {
        if had_existing {
            let _ = fs::rename(&backup_dir, final_dir);
        }
        let _ = fs::remove_dir_all(stage_dir);
        let _ = fs::remove_file(&temp_catalog_path);
        return Err(OsuImportError::StorageError {
            path: final_dir.display().to_string(),
            message: error.to_string(),
        });
    }

    if let Err(error) = swap_catalog_file(&temp_catalog_path, &catalog_path, had_existing) {
        let _ = fs::remove_dir_all(final_dir);
        if had_existing {
            let _ = fs::rename(&backup_dir, final_dir);
        }
        let _ = fs::remove_file(&temp_catalog_path);
        return Err(error);
    }

    if had_existing {
        let _ = fs::remove_dir_all(&backup_dir);
    }

    Ok(())
}

fn swap_catalog_file(
    temp_catalog_path: &Path,
    catalog_path: &Path,
    had_existing: bool,
) -> Result<(), OsuImportError> {
    let backup_catalog_path = catalog_path.with_extension("toml.previous");

    if had_existing {
        if backup_catalog_path.exists() {
            fs::remove_file(&backup_catalog_path).map_err(|source| {
                OsuImportError::StorageError {
                    path: backup_catalog_path.display().to_string(),
                    message: source.to_string(),
                }
            })?;
        }
        fs::rename(catalog_path, &backup_catalog_path).map_err(|source| {
            OsuImportError::StorageError {
                path: catalog_path.display().to_string(),
                message: source.to_string(),
            }
        })?;
    }

    if let Err(error) = fs::rename(temp_catalog_path, catalog_path) {
        if had_existing {
            let _ = fs::rename(&backup_catalog_path, catalog_path);
        }
        return Err(OsuImportError::StorageError {
            path: catalog_path.display().to_string(),
            message: error.to_string(),
        });
    }

    if had_existing {
        let _ = fs::remove_file(&backup_catalog_path);
    }

    Ok(())
}

fn should_skip_parse_error(error: &OsuParseError) -> bool {
    matches!(
        error,
        OsuParseError::UnsupportedMode { .. } | OsuParseError::InvalidFormat(_)
    )
}

fn should_skip_import_error(error: &OsuImportError) -> bool {
    matches!(
        error,
        OsuImportError::UnsupportedKeyCount { .. }
            | OsuImportError::UnsupportedLane { .. }
            | OsuImportError::NegativeTimingPointStart { .. }
            | OsuImportError::NegativeNoteTime { .. }
            | OsuImportError::InvalidHoldDuration { .. }
            | OsuImportError::NegativeHoldTime { .. }
            | OsuImportError::NoUsableTimingPoints
            | OsuImportError::MissingAudioFile { .. }
            | OsuImportError::InvalidAudioFilename { .. }
    )
}

fn discover_artwork_path(
    source_folder: &Path,
    source_osu_path: &Path,
    source_audio_path: &Path,
) -> Option<PathBuf> {
    let supported = ["png", "jpg", "jpeg", "webp"];
    let audio_stem = source_audio_path.file_stem().and_then(|stem| stem.to_str());
    let osu_stem = source_osu_path.file_stem().and_then(|stem| stem.to_str());

    let mut images = fs::read_dir(source_folder)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    supported
                        .iter()
                        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    images.sort();

    images
        .into_iter()
        .find(|path| {
            let stem = path.file_stem().and_then(|value| value.to_str());
            stem == audio_stem || stem == osu_stem
        })
        .or_else(|| {
            fs::read_dir(source_folder)
                .ok()?
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| {
                            supported
                                .iter()
                                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
                        })
                        .unwrap_or(false)
                })
                .min()
        })
}
