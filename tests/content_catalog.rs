use code_m::app::DemoApp;
use code_m::chart::parser::parse_chart_file;
use code_m::content::{
    imported_catalog_path, load_bundled_song_catalog, load_imported_song_catalog,
    prepare_import_root_with_legacy, save_imported_song_catalog, ImportedSongCatalog,
    ImportedSongCatalogEntry, IMPORTED_ORIGIN_TYPE,
};
use code_m::osu::import::import_osu_mania_folder;
use std::fs;
use std::path::{Path, PathBuf};

mod content_catalog {
    use super::*;

    fn temp_import_root(test_name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "code_m-{}-{}-{}",
            test_name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn bundled_catalog_lists_current_demo_songs() {
        let catalog = load_bundled_song_catalog().expect("bundled catalog should load");

        assert_eq!(catalog.songs().len(), 2);

        let first = &catalog.songs()[0];
        assert_eq!(first.id(), "demo-basic");
        assert_eq!(first.title(), "Demo Basic");
        assert_eq!(first.artist(), "Code M");
        assert_eq!(first.chart_name(), "Normal");
        assert_eq!(first.difficulty(), 4);
        assert_eq!(first.bpm(), 128);
        assert_eq!(first.mood(), "Clean pulse");
        assert!(first.artwork_path().is_some());
        assert!(first.audio_path().is_some());
        assert!(
            first
                .chart_path()
                .ends_with("assets/charts/demo_basic.toml"),
            "unexpected chart path: {}",
            first.chart_path().display()
        );

        let second = &catalog.songs()[1];
        assert_eq!(second.id(), "demo-hold");
        assert_eq!(second.title(), "Demo Hold");
        assert_eq!(second.artist(), "Code M");
        assert_eq!(second.chart_name(), "Normal");
        assert_eq!(second.difficulty(), 6);
        assert_eq!(second.bpm(), 143);
        assert_eq!(second.mood(), "Cold pulse");
        assert!(second.artwork_path().is_some());
        assert!(second.audio_path().is_some());
        assert!(
            second
                .chart_path()
                .ends_with("assets/charts/demo_hold.toml"),
            "unexpected chart path: {}",
            second.chart_path().display()
        );
    }

    #[test]
    fn demo_app_loads_song_choices_from_bundled_catalog() {
        let app = DemoApp::from_demo_chart().expect("demo app should load bundled songs");

        let songs = app.song_choices();
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title(), "Demo Basic");
        assert_eq!(songs[0].artist(), "Code M");
        assert_eq!(songs[0].chart_name(), "Normal");
        assert_eq!(songs[0].difficulty(), 4);
        assert_eq!(songs[0].bpm(), 128);
        assert_eq!(songs[0].mood(), "Clean pulse");
        assert_eq!(songs[1].title(), "Demo Hold");
        assert_eq!(songs[1].artist(), "Code M");
        assert_eq!(songs[1].chart_name(), "Normal");
        assert_eq!(songs[1].difficulty(), 6);
        assert_eq!(songs[1].bpm(), 143);
        assert_eq!(songs[1].mood(), "Cold pulse");
    }

    #[test]
    fn imported_catalog_round_trips_persisted_entry() {
        let root = temp_import_root("catalog-round-trip");
        let chart_path = root.join("demo-id/chart.toml");
        let audio_path = root.join("demo-id/song.ogg");
        let source_osu_path = root.join("demo-id/map.osu");

        let catalog = ImportedSongCatalog {
            songs: vec![ImportedSongCatalogEntry {
                id: "demo-id".to_string(),
                title: "Imported Demo".to_string(),
                artist: "Code M".to_string(),
                chart_name: "Starter".to_string(),
                difficulty: 6,
                bpm: 120,
                mood: "Imported".to_string(),
                chart_path: chart_path.clone(),
                audio_path: audio_path.clone(),
                artwork_path: None,
                source_osu_path: source_osu_path.clone(),
                source_folder: "/tmp/beatmap".to_string(),
                source_osu_filename: "map.osu".to_string(),
                imported_at_unix_ms: 123456789,
                origin_type: IMPORTED_ORIGIN_TYPE.to_string(),
            }],
        };

        save_imported_song_catalog(&root, &catalog).expect("save imported catalog");
        let loaded = load_imported_song_catalog(&root).expect("load imported catalog");

        assert_eq!(loaded.songs().len(), 1);
        let entry = &loaded.songs()[0];
        assert_eq!(entry.id, "demo-id");
        assert_eq!(entry.title, "Imported Demo");
        assert_eq!(entry.artist, "Code M");
        assert_eq!(entry.chart_name, "Starter");
        assert_eq!(entry.difficulty, 6);
        assert_eq!(entry.bpm, 120);
        assert_eq!(entry.mood, "Imported");
        assert!(entry.chart_path.ends_with(Path::new("demo-id/chart.toml")));
        assert!(entry.audio_path.ends_with(Path::new("demo-id/song.ogg")));
        assert!(entry
            .source_osu_path
            .ends_with(Path::new("demo-id/map.osu")));
        assert_eq!(entry.source_folder, "/tmp/beatmap");
        assert_eq!(entry.source_osu_filename, "map.osu");
        assert_eq!(entry.imported_at_unix_ms, 123456789);
        assert_eq!(entry.origin_type, IMPORTED_ORIGIN_TYPE);
    }

    #[test]
    fn imported_folder_persists_and_reload_points_to_usable_files() {
        let source_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/osu/valid-6k");
        let source_folder = temp_import_source("persisted-folder", &source_root);
        let import_root = temp_import_root("catalog-reload");

        let imported_entries =
            import_osu_mania_folder(&source_folder, &import_root).expect("import");
        assert_eq!(imported_entries.len(), 1);

        let loaded = load_imported_song_catalog(&import_root).expect("reload imported catalog");
        assert_eq!(loaded.songs().len(), 1);
        let entry = &loaded.songs()[0];

        assert!(entry.chart_path.exists());
        assert!(entry.audio_path.exists());
        assert!(entry.source_osu_path.exists());

        let chart = parse_chart_file(&entry.chart_path).expect("persisted chart should parse");
        assert_eq!(chart.metadata.title, "Minimal Mania 6K");
        assert_eq!(chart.notes.len(), 3);
        assert_eq!(chart.timing.len(), 1);
    }

    #[test]
    fn imported_catalog_saves_paths_relative_to_import_root() {
        let root = temp_import_root("catalog-relative-paths");
        let chart_path = root.join("demo-id/chart.toml");
        let audio_path = root.join("demo-id/song.ogg");
        let source_osu_path = root.join("demo-id/map.osu");

        let catalog = ImportedSongCatalog {
            songs: vec![ImportedSongCatalogEntry {
                id: "demo-id".to_string(),
                title: "Imported Demo".to_string(),
                artist: "Code M".to_string(),
                chart_name: "Starter".to_string(),
                difficulty: 6,
                bpm: 120,
                mood: "Imported".to_string(),
                chart_path,
                audio_path,
                artwork_path: None,
                source_osu_path,
                source_folder: "/tmp/beatmap".to_string(),
                source_osu_filename: "map.osu".to_string(),
                imported_at_unix_ms: 123456789,
                origin_type: IMPORTED_ORIGIN_TYPE.to_string(),
            }],
        };

        save_imported_song_catalog(&root, &catalog).expect("save imported catalog");
        let raw = fs::read_to_string(imported_catalog_path(&root)).expect("read raw catalog");

        assert!(raw.contains("chart_path = \"demo-id/chart.toml\""));
        assert!(raw.contains("audio_path = \"demo-id/song.ogg\""));
        assert!(raw.contains("source_osu_path = \"demo-id/map.osu\""));
    }

    #[test]
    fn imported_catalog_load_normalizes_legacy_prefixed_relative_paths() {
        let root = temp_import_root("catalog-legacy-normalize");
        let catalog_raw = r#"
[[songs]]
id = "demo-id"
title = "Imported Demo"
artist = "Code M"
chart_name = "Starter"
difficulty = 6
bpm = 120
mood = "Imported"
chart_path = ".superpowers/code_m_imports/.superpowers/code_m_imports/demo-id/chart.toml"
audio_path = ".superpowers/code_m_imports/.superpowers/code_m_imports/demo-id/song.ogg"
source_osu_path = ".superpowers/code_m_imports/.superpowers/code_m_imports/demo-id/map.osu"
source_folder = "/tmp/beatmap"
source_osu_filename = "map.osu"
imported_at_unix_ms = 123456789
origin_type = "osu!mania import"
"#;

        fs::write(imported_catalog_path(&root), catalog_raw).expect("write legacy catalog");

        let loaded = load_imported_song_catalog(&root).expect("load normalized catalog");
        let entry = &loaded.songs()[0];

        assert_eq!(entry.chart_path, root.join("demo-id/chart.toml"));
        assert_eq!(entry.audio_path, root.join("demo-id/song.ogg"));
        assert_eq!(entry.source_osu_path, root.join("demo-id/map.osu"));
    }

    #[test]
    fn prepare_import_root_with_legacy_moves_superpowers_imports_into_code_m_dir() {
        let sandbox = temp_import_root("import-root-migration");
        let new_root = sandbox.join(".code_m/imports");
        let legacy_root = sandbox.join(".superpowers/code_m_imports");
        fs::create_dir_all(legacy_root.join("demo-id")).expect("legacy import dir");
        fs::write(legacy_root.join("catalog.toml"), "legacy").expect("legacy catalog");
        fs::write(legacy_root.join("demo-id/chart.toml"), "chart").expect("legacy chart");

        let resolved =
            prepare_import_root_with_legacy(&new_root, &legacy_root).expect("prepare import root");

        assert_eq!(resolved, new_root);
        assert!(new_root.join("catalog.toml").exists());
        assert!(new_root.join("demo-id/chart.toml").exists());
        assert!(!legacy_root.exists());
    }

    fn temp_import_source(test_name: &str, source_root: &Path) -> PathBuf {
        let root = temp_import_root(test_name);
        fs::copy(source_root.join("map.osu"), root.join("map.osu")).expect("copy map");
        fs::copy(source_root.join("song.ogg"), root.join("song.ogg")).expect("copy audio");
        root
    }
}
