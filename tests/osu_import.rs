use code_m::chart::model::{Note, NoteKind};
use code_m::content::load_imported_song_catalog;
use code_m::osu::import::{convert_osu_mania_chart, import_osu_mania_folder, OsuImportError};
use code_m::osu::model::{OsuBeatmap, OsuHitObject, OsuMetadata, OsuMode, OsuTimingPoint};
use code_m::osu::parser::parse_osu_file;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

mod osu_import {
    use super::*;

    fn importable_fixture(folder: &str) -> code_m::osu::OsuBeatmap {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(folder)
            .join("map.osu");
        parse_osu_file(path).expect("fixture should parse")
    }

    #[test]
    fn converts_6k_mania_notes_into_app_chart() {
        let imported = importable_fixture("tests/fixtures/osu/valid-6k");
        let chart = convert_osu_mania_chart(&imported).expect("convert");

        assert_eq!(chart.metadata.title, "Minimal Mania 6K");
        assert_eq!(chart.metadata.artist, "Code M");
        assert_eq!(chart.metadata.chart_name, "Starter");
        assert_eq!(chart.notes.len(), 3);
        assert_eq!(chart.notes[0].lane(), 0);
        assert_eq!(chart.notes[1].lane(), 3);
        assert_eq!(chart.notes[2].lane(), 5);
        assert_eq!(chart.timing.len(), 1);
        assert_eq!(chart.timing[0].start_ms, 0);
        assert_eq!(chart.timing[0].bpm, 120.0);
        assert_eq!(chart.timing[0].beat_length, 4);
        assert!(matches!(
            &chart.notes[0],
            Note::Tap(tap) if tap.kind == NoteKind::Tap && tap.time_ms == 1000 && tap.lane == 0
        ));
        assert!(matches!(
            &chart.notes[1],
            Note::Hold(hold) if hold.kind == NoteKind::Hold && hold.start_ms == 1500 && hold.end_ms == 2000 && hold.lane == 3
        ));
    }

    #[test]
    fn rejects_non_6k_mania_chart() {
        let imported = importable_fixture("tests/fixtures/osu/valid-4k");
        let err = convert_osu_mania_chart(&imported).expect_err("should fail");
        assert!(matches!(
            err,
            OsuImportError::UnsupportedKeyCount { key_count: 4 }
        ));
    }

    #[test]
    fn import_copies_required_files_into_app_storage() {
        let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "copy-files");
        let import_root = std::env::temp_dir().join(format!(
            "code_m-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&import_root).expect("import root");

        let entries = import_osu_mania_folder(&source_folder, &import_root).expect("import");
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];

        assert!(entry.chart_path().exists());
        assert!(entry.audio_path().exists());
        assert!(entry.source_osu_path().exists());
        assert!(entry.chart_path().ends_with("chart.toml"));
        assert!(entry.audio_path().ends_with("song.ogg"));
        assert!(entry.source_osu_path().ends_with("map.osu"));
        assert_eq!(entry.source_folder, source_folder.display().to_string());

        let catalog_path = import_root.join("catalog.toml");
        assert!(catalog_path.exists());
    }

    #[test]
    fn failed_reimport_preserves_existing_import() {
        let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "reimport-safety");
        let import_root = std::env::temp_dir().join(format!(
            "code_m-reimport-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&import_root).expect("import root");

        let first_entries =
            import_osu_mania_folder(&source_folder, &import_root).expect("first import");
        let first_entry = first_entries.into_iter().next().expect("one entry");
        let imported_dir = first_entry
            .chart_path()
            .parent()
            .expect("chart parent")
            .to_path_buf();
        assert!(imported_dir.exists());

        fs::remove_file(source_folder.join("song.ogg")).expect("remove audio to force failure");

        let err = import_osu_mania_folder(&source_folder, &import_root).expect_err("should fail");
        assert!(matches!(err, OsuImportError::NoUsableBeatmaps { .. }));

        assert!(
            imported_dir.exists(),
            "existing import directory should remain"
        );
        assert!(
            first_entry.chart_path().exists(),
            "existing chart should remain"
        );
        assert!(
            first_entry.audio_path().exists(),
            "existing audio should remain"
        );
        assert!(
            first_entry.source_osu_path().exists(),
            "existing source should remain"
        );
    }

    #[test]
    fn reimport_updates_existing_catalog_entry_safely() {
        let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "reimport-success");
        let import_root = std::env::temp_dir().join(format!(
            "code_m-reimport-success-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&import_root).expect("import root");

        let first_entries =
            import_osu_mania_folder(&source_folder, &import_root).expect("first import");
        let first_entry = first_entries.into_iter().next().expect("one entry");
        let second_entries =
            import_osu_mania_folder(&source_folder, &import_root).expect("second import");
        let second_entry = second_entries.into_iter().next().expect("one entry");

        assert_eq!(first_entry.id, second_entry.id);

        let loaded = load_imported_song_catalog(&import_root).expect("load imported catalog");
        assert_eq!(loaded.songs().len(), 1);
        assert_eq!(loaded.songs()[0].id, first_entry.id);
        assert!(loaded.songs()[0].chart_path.exists());
        assert!(loaded.songs()[0].audio_path.exists());
        assert!(loaded.songs()[0].source_osu_path.exists());
    }

    #[test]
    fn imports_usable_map_from_multi_osu_folder_and_skips_unsupported_maps() {
        let source_folder = temp_folder("multi-osu");
        copy_fixture_file(
            "tests/fixtures/osu/valid-6k/map.osu",
            source_folder.join("usable.osu"),
        );
        copy_fixture_file(
            "tests/fixtures/osu/valid-4k/map.osu",
            source_folder.join("unsupported.osu"),
        );
        copy_fixture_file(
            "tests/fixtures/osu/valid-6k/song.ogg",
            source_folder.join("song.ogg"),
        );

        let import_root = std::env::temp_dir().join(format!(
            "code_m-multi-osu-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&import_root).expect("import root");

        let entries = import_osu_mania_folder(&source_folder, &import_root).expect("import");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Minimal Mania 6K");
        assert!(entries[0].chart_path().exists());
        assert!(entries[0].audio_path().exists());
    }

    fn fixture_copy(folder: &str, suffix: &str) -> PathBuf {
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder);
        let temp_root = temp_folder(suffix);
        copy_fixture_file(source_root.join("map.osu"), temp_root.join("map.osu"));
        copy_fixture_file(source_root.join("song.ogg"), temp_root.join("song.ogg"));
        temp_root
    }

    fn temp_folder(suffix: &str) -> PathBuf {
        let temp_root = std::env::temp_dir().join(format!(
            "code_m-fixture-{}-{}-{}",
            suffix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&temp_root).expect("temp fixture root");
        temp_root
    }

    fn copy_fixture_file(source: impl AsRef<Path>, destination: impl AsRef<Path>) {
        fs::copy(source, destination).expect("copy fixture file");
    }

    #[test]
    fn preserves_non_four_meter_on_timing_points() {
        let imported = OsuBeatmap {
            mode: OsuMode::Mania,
            key_count: 6,
            audio_filename: "song.ogg".to_string(),
            metadata: OsuMetadata {
                title: Some("Meter Demo".to_string()),
                title_unicode: None,
                artist: Some("Code M".to_string()),
                artist_unicode: None,
                creator: Some("Code M".to_string()),
                version: Some("Starter".to_string()),
            },
            timing_points: vec![OsuTimingPoint {
                time_ms: 0,
                beat_length: 428.5714285714,
                meter: 7,
                uninherited: true,
                kiai: false,
            }],
            hit_objects: vec![OsuHitObject::Tap {
                lane: 0,
                time_ms: 1000,
            }],
        };

        let chart = convert_osu_mania_chart(&imported).expect("convert");
        assert_eq!(chart.timing[0].beat_length, 7);
    }

    #[test]
    fn rejects_negative_tap_times_explicitly() {
        let imported = OsuBeatmap {
            mode: OsuMode::Mania,
            key_count: 6,
            audio_filename: "song.ogg".to_string(),
            metadata: OsuMetadata {
                title: Some("Negative Notes".to_string()),
                title_unicode: None,
                artist: Some("Code M".to_string()),
                artist_unicode: None,
                creator: Some("Code M".to_string()),
                version: Some("Starter".to_string()),
            },
            timing_points: vec![OsuTimingPoint {
                time_ms: 0,
                beat_length: 500.0,
                meter: 4,
                uninherited: true,
                kiai: false,
            }],
            hit_objects: vec![OsuHitObject::Tap {
                lane: 0,
                time_ms: -10,
            }],
        };

        let err = convert_osu_mania_chart(&imported).expect_err("should fail");
        assert!(matches!(
            err,
            OsuImportError::NegativeNoteTime {
                lane: 0,
                time_ms: -10
            }
        ));
    }

    #[test]
    fn rejects_negative_hold_times_explicitly() {
        let imported = OsuBeatmap {
            mode: OsuMode::Mania,
            key_count: 6,
            audio_filename: "song.ogg".to_string(),
            metadata: OsuMetadata {
                title: Some("Negative Hold".to_string()),
                title_unicode: None,
                artist: Some("Code M".to_string()),
                artist_unicode: None,
                creator: Some("Code M".to_string()),
                version: Some("Starter".to_string()),
            },
            timing_points: vec![OsuTimingPoint {
                time_ms: 0,
                beat_length: 500.0,
                meter: 4,
                uninherited: true,
                kiai: false,
            }],
            hit_objects: vec![OsuHitObject::Hold {
                lane: 1,
                start_time_ms: 100,
                end_time_ms: -5,
            }],
        };

        let err = convert_osu_mania_chart(&imported).expect_err("should fail");
        assert!(matches!(
            err,
            OsuImportError::NegativeHoldTime {
                lane: 1,
                start_time_ms: 100,
                end_time_ms: -5
            }
        ));
    }

    #[test]
    fn rejects_negative_timing_point_start_explicitly() {
        let imported = OsuBeatmap {
            mode: OsuMode::Mania,
            key_count: 6,
            audio_filename: "song.ogg".to_string(),
            metadata: OsuMetadata {
                title: Some("Negative Timing".to_string()),
                title_unicode: None,
                artist: Some("Code M".to_string()),
                artist_unicode: None,
                creator: Some("Code M".to_string()),
                version: Some("Starter".to_string()),
            },
            timing_points: vec![OsuTimingPoint {
                time_ms: -250,
                beat_length: 500.0,
                meter: 4,
                uninherited: true,
                kiai: false,
            }],
            hit_objects: vec![OsuHitObject::Tap {
                lane: 0,
                time_ms: 1000,
            }],
        };

        let err = convert_osu_mania_chart(&imported).expect_err("should fail");
        assert!(matches!(
            err,
            OsuImportError::NegativeTimingPointStart {
                start_time_ms: -250
            }
        ));
    }
}
