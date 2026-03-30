use code_m::app::{DemoApp, DemoMode};
use code_m::content::{
    save_imported_song_catalog, ImportedSongCatalog, ImportedSongCatalogEntry, IMPORTED_ORIGIN_TYPE,
};
use code_m::gameplay::Judgment;
use code_m::osu::import::import_osu_mania_folder;
use code_m::runtime::GameTime;
use std::fs;
use std::path::PathBuf;

fn start_loaded_chart(app: &mut DemoApp) {
    app.start_selected_chart()
        .expect("selected chart should start");
    app.skip_loading_intro();
}

#[test]
fn demo_state_advances_notes_and_scores_hits() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);

    app.update(GameTime::from_millis(1000));
    let before = app.visible_notes();
    assert!(!before.is_empty());

    let handled = app.handle_lane_press(0, GameTime::from_millis(1000));
    assert!(handled);
    assert_eq!(app.latest_judgment(), Some(Judgment::Perfect));
    assert_eq!(app.score_summary().score, Judgment::Perfect.points());
    assert_eq!(app.score_summary().combo, 1);
}

#[test]
fn demo_state_boots_into_song_select_and_launches_selected_chart() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    assert_eq!(app.mode(), DemoMode::SongSelect);
    assert_eq!(app.selected_chart_title(), "Demo Basic");

    app.move_selection(1);
    assert_eq!(app.selected_chart_title(), "Demo Hold");

    app.start_selected_chart()
        .expect("selected chart should start");
    assert_eq!(app.mode(), DemoMode::Loading);
    assert_eq!(app.chart_title(), "Demo Hold");

    app.update(GameTime::from_millis(1_999));
    assert_eq!(app.mode(), DemoMode::Loading);

    app.update(GameTime::from_millis(2_000));
    assert_eq!(app.mode(), DemoMode::Ready);

    app.update(GameTime::from_millis(3_000));
    assert_eq!(app.mode(), DemoMode::Playing);
}

#[test]
fn demo_state_pauses_restarts_finishes_and_returns_to_song_select() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.move_selection(1);
    start_loaded_chart(&mut app);

    app.update(GameTime::from_millis(1000));
    let before_pause = app.visible_notes();
    assert_eq!(app.mode(), DemoMode::Playing);

    app.toggle_pause();
    assert_eq!(app.mode(), DemoMode::Paused);

    app.update(GameTime::from_millis(2000));
    assert_eq!(app.visible_notes(), before_pause);

    app.toggle_pause();
    assert_eq!(app.mode(), DemoMode::Playing);

    app.handle_lane_press(2, GameTime::from_millis(1200));
    assert_eq!(app.score_summary().combo, 1);

    app.restart()
        .expect("restart should rebuild the selected chart");
    assert_eq!(app.mode(), DemoMode::Loading);
    assert_eq!(app.latest_judgment(), None);
    assert_eq!(app.score_summary().score, 0);
    assert_eq!(app.score_summary().combo, 0);
    assert_eq!(app.chart_title(), "Demo Hold");
    app.skip_loading_intro();
    assert_eq!(app.mode(), DemoMode::Playing);
    assert!(!app.visible_notes().is_empty());

    app.update(GameTime::from_millis(5000));
    assert_eq!(app.mode(), DemoMode::Results);

    app.return_to_song_select();
    assert_eq!(app.mode(), DemoMode::SongSelect);
    assert_eq!(app.selected_chart_title(), "Demo Hold");
}

#[test]
fn demo_state_cycles_theme_and_preserves_it_through_restart() {
    let seed = DemoApp::from_demo_chart().expect("demo app should load");
    let theme = code_m::ui::ThemeTokens::builtin("minimal-professional")
        .expect("built-in theme should load");
    let mut app = DemoApp::new(
        seed.song_choices().to_vec(),
        code_m::config::Settings::default(),
        theme,
    );

    let initial_theme = app.theme_name().to_string();

    app.toggle_theme();
    let toggled_theme = app.theme_name().to_string();
    assert_ne!(toggled_theme, initial_theme);

    start_loaded_chart(&mut app);
    app.toggle_pause();
    assert_eq!(app.mode(), DemoMode::Paused);

    app.restart()
        .expect("restart should preserve theme selection");
    assert_eq!(app.mode(), DemoMode::Loading);
    app.skip_loading_intro();
    assert_eq!(app.mode(), DemoMode::Playing);
    assert_eq!(app.theme_name(), toggled_theme);

    app.toggle_theme();
    assert_ne!(app.theme_name(), toggled_theme);
}

#[test]
fn demo_state_supports_explicit_release_and_press_fallback_for_holds() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(2500));
    assert!(app.handle_lane_press(4, GameTime::from_millis(2500)));
    assert!(app.active_lanes()[4]);

    assert!(app.handle_lane_release(4, GameTime::from_millis(4000)));
    assert_eq!(app.latest_judgment(), Some(Judgment::Perfect));
    assert_eq!(app.score_summary().score, Judgment::Perfect.points() * 2);
    assert_eq!(app.score_summary().combo, 2);
    assert!(!app.active_lanes()[4]);

    let mut fallback_app = DemoApp::from_demo_chart().expect("demo app should load");
    start_loaded_chart(&mut fallback_app);
    fallback_app.update(GameTime::from_millis(2500));
    assert!(fallback_app.handle_lane_press(4, GameTime::from_millis(2500)));
    assert!(fallback_app.handle_lane_press(4, GameTime::from_millis(4000)));
    assert_eq!(fallback_app.latest_judgment(), Some(Judgment::Perfect));
    assert_eq!(
        fallback_app.score_summary().score,
        Judgment::Perfect.points() * 2
    );
    assert!(!fallback_app.active_lanes()[4]);
}

#[test]
fn demo_state_reaches_results_after_unreleased_hold_expires() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(2500));
    assert!(app.handle_lane_press(4, GameTime::from_millis(2500)));

    app.update(GameTime::from_millis(4091));
    assert_eq!(app.latest_judgment(), Some(Judgment::Miss));
    assert_eq!(app.mode(), DemoMode::Results);
    assert!(!app.active_lanes()[4]);
}

#[test]
fn demo_state_exposes_product_settings_flow() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    let initial_theme = app.theme_name().to_string();
    assert!(app.metronome_enabled());

    app.open_settings();
    assert_eq!(app.mode(), DemoMode::Settings);

    app.move_settings_selection(1);
    app.activate_settings_item();
    assert!(!app.startup_splash_enabled());

    app.move_settings_selection(1);
    app.activate_settings_item();
    assert!(!app.metronome_enabled());

    app.move_settings_selection(-2);
    app.activate_settings_item();
    assert_ne!(app.theme_name(), initial_theme);
    let toggled_theme = app.theme_name().to_string();

    app.close_settings();
    assert_eq!(app.mode(), DemoMode::SongSelect);

    start_loaded_chart(&mut app);
    app.restart()
        .expect("restart should preserve product settings");
    assert_eq!(app.theme_name(), toggled_theme);
    assert!(!app.metronome_enabled());
}

#[test]
fn demo_state_enters_and_exits_calibration_from_settings() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.open_settings();
    while app.settings_selection() != code_m::app::SettingsItem::Calibration {
        app.move_settings_selection(1);
    }

    app.activate_settings_item();
    assert_eq!(app.mode(), DemoMode::Calibration);

    app.adjust_calibration(12);
    app.finish_calibration();

    assert_eq!(app.mode(), DemoMode::Settings);
    assert_eq!(app.global_offset_ms(), 12);
}

#[test]
fn demo_state_uses_updated_keymap_for_lane_input() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.set_keymap_str("A S D J K L")
        .expect("custom keymap should be accepted");
    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));

    assert!(app.handle_key_char('a', GameTime::from_millis(1000)));
    assert_eq!(app.latest_judgment(), Some(Judgment::Perfect));
}

#[test]
fn demo_state_applies_input_offset_to_judgment_time() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.set_input_offset_ms(20);
    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1020));

    assert!(app.handle_lane_press(0, GameTime::from_millis(980)));
    assert_eq!(app.latest_judgment(), Some(Judgment::Perfect));
}

#[test]
fn demo_state_tracks_global_offset_separately_from_input_offset() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.set_global_offset_ms(-24);
    app.set_input_offset_ms(18);

    assert_eq!(app.global_offset_ms(), -24);
    assert_eq!(app.input_offset_ms(), 18);
}

#[test]
fn demo_state_tracks_hit_sound_requests_for_successful_hits() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");
    let before = app.pending_hit_sound_requests();

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_lane_press(0, GameTime::from_millis(1000)));

    assert_eq!(app.pending_hit_sound_requests(), before + 1);
}

#[test]
fn demo_state_can_skip_loading_buffer_and_start_immediately() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    app.start_selected_chart()
        .expect("selected chart should start");
    assert_eq!(app.mode(), DemoMode::Loading);

    app.skip_loading_intro();
    assert_eq!(app.mode(), DemoMode::Playing);
}

#[test]
fn demo_state_autoplay_perfects_the_current_chart() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");
    app.set_autoplay_enabled(true);

    app.start_selected_chart()
        .expect("selected chart should start");
    app.skip_loading_intro();

    app.update(GameTime::from_millis(5_000));

    assert_eq!(app.mode(), DemoMode::Results);
    assert_eq!(app.score_summary().judgments.miss, 0);
    assert!(app.score_summary().judgments.perfect >= 3);
    assert!(app.last_replay().is_some());
}

#[test]
fn demo_state_loads_imported_catalog_separately_from_bundled_catalog() {
    let import_root = temp_import_root("runtime-imported-catalog");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "runtime-import-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");

    assert_eq!(app.song_choices().len(), 2);
    assert_eq!(app.imported_song_choices().len(), 1);
    assert_eq!(app.imported_song_choices()[0].title(), "Minimal Mania 6K");
    assert_eq!(app.mode(), DemoMode::SongSelect);
}

#[test]
fn demo_state_enters_imported_view_and_starts_imported_song() {
    let import_root = temp_import_root("imported-shell-flow");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "imported-shell-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");

    app.open_imported_view();
    assert_eq!(app.mode(), DemoMode::ImportedSelect);
    assert_eq!(app.imported_song_choices().len(), 1);

    start_loaded_chart(&mut app);
    assert_eq!(app.mode(), DemoMode::Playing);
    assert_eq!(app.chart_title(), "Minimal Mania 6K");
}

#[test]
fn demo_state_can_start_every_song_in_existing_local_import_root_when_present() {
    let import_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".superpowers")
        .join("code_m_imports");
    if !import_root.exists() {
        return;
    }

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");
    if app.imported_song_choices().is_empty() {
        return;
    }

    app.open_imported_view();
    let len = app.imported_song_choices().len();
    for index in 0..len {
        while app.imported_selected_chart_index() != index {
            app.move_imported_selection(1);
        }

        let expected_title = app
            .selected_imported_song()
            .expect("imported song")
            .title()
            .to_string();

        start_loaded_chart(&mut app);
        assert_eq!(app.active_song().title(), expected_title);
        app.return_to_browse_view();
        app.open_imported_view();
    }
}

#[test]
fn demo_state_calibration_from_bundled_browse_uses_selected_song() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");
    app.move_selection(1);
    assert_eq!(app.selected_chart_title(), "Demo Hold");

    app.open_settings();
    app.open_calibration();

    assert_eq!(app.mode(), DemoMode::Calibration);
    assert_eq!(app.playback_song().title(), "Demo Hold");
    assert_eq!(app.playback_chart().metadata.title, "Demo Hold");
}

#[test]
fn demo_state_calibration_from_imported_browse_uses_selected_song() {
    let import_root = temp_import_root("imported-calibration-browse");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "imported-calibration-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");

    app.open_imported_view();
    assert_eq!(app.mode(), DemoMode::ImportedSelect);
    assert_eq!(
        app.selected_imported_song()
            .expect("selected imported song")
            .title(),
        "Minimal Mania 6K"
    );

    app.open_settings();
    app.open_calibration();

    assert_eq!(app.mode(), DemoMode::Calibration);
    assert_eq!(app.playback_song().title(), "Minimal Mania 6K");
    assert_eq!(app.playback_chart().metadata.title, "Minimal Mania 6K");
}

#[test]
fn demo_state_preserves_imported_playback_context_through_settings_and_calibration() {
    let import_root = temp_import_root("imported-context");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "imported-context-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");
    app.open_imported_view();
    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(5000));
    assert_eq!(app.mode(), DemoMode::Results);

    app.open_settings();
    assert_eq!(app.mode(), DemoMode::Settings);
    app.open_calibration();
    assert_eq!(app.mode(), DemoMode::Calibration);
    assert_eq!(app.playback_song().title(), "Minimal Mania 6K");
    assert_eq!(app.playback_chart().metadata.title, "Minimal Mania 6K");
}

#[test]
fn demo_state_searches_bundled_and_imported_catalogs_together() {
    let import_root = temp_import_root("unified-search");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "unified-search-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");

    app.open_search();
    for ch in "minimal".chars() {
        app.push_search_char(ch);
    }

    assert_eq!(app.mode(), DemoMode::Search);
    assert_eq!(app.search_results_len(), 1);
    assert_eq!(app.search_result_rows()[0].source, "Imported");

    app.close_search();
    app.open_search();
    for ch in "demo".chars() {
        app.push_search_char(ch);
    }

    assert_eq!(app.search_results_len(), 2);
    assert_eq!(app.search_result_rows()[0].source, "Bundled");
}

#[test]
fn demo_state_cycles_browse_sort_without_losing_the_selected_song() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");
    app.move_selection(1);

    assert_eq!(app.selected_chart_title(), "Demo Hold");
    assert_eq!(app.browse_sort_mode().label(), "Default");

    app.cycle_browse_sort();
    assert_eq!(app.browse_sort_mode().label(), "Title");
    assert_eq!(app.selected_chart_title(), "Demo Hold");

    app.cycle_browse_sort();
    assert_eq!(app.browse_sort_mode().label(), "BPM");

    app.cycle_browse_sort();
    assert_eq!(app.browse_sort_mode().label(), "Stage");

    app.cycle_browse_sort();
    assert_eq!(app.browse_sort_mode().label(), "Default");
}

#[test]
fn demo_state_skips_corrupted_imported_entries_and_still_boots_bundled_runtime() {
    let import_root = temp_import_root("corrupted-imported-entry");
    let chart_path = import_root.join("broken-id/chart.toml");
    let audio_path = import_root.join("broken-id/song.ogg");
    let source_osu_path = import_root.join("broken-id/map.osu");

    let catalog = ImportedSongCatalog {
        songs: vec![ImportedSongCatalogEntry {
            id: "broken-id".to_string(),
            title: "Broken Imported".to_string(),
            artist: "Code M".to_string(),
            chart_name: "Starter".to_string(),
            difficulty: 6,
            bpm: 120,
            mood: "Imported".to_string(),
            chart_path: chart_path.clone(),
            audio_path: audio_path.clone(),
            artwork_path: None,
            source_osu_path: source_osu_path.clone(),
            source_folder: "/tmp/broken".to_string(),
            source_osu_filename: "map.osu".to_string(),
            imported_at_unix_ms: 0,
            origin_type: IMPORTED_ORIGIN_TYPE.to_string(),
        }],
    };

    save_imported_song_catalog(&import_root, &catalog).expect("save corrupt catalog");

    let app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should boot even with broken imported entry");
    assert_eq!(app.song_choices().len(), 2);
    assert!(app.imported_song_choices().is_empty());
    assert_eq!(app.mode(), DemoMode::SongSelect);
}

#[test]
fn demo_state_exposes_transient_hit_feedback_after_successful_hits() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_lane_press(0, GameTime::from_millis(1000)));

    assert!(app.lane_feedback_strength(0) > 0.0);
    assert!(app.judgment_feedback_strength() > 0.0);
    assert!(app.combo_feedback_strength() > 0.0);

    app.update(GameTime::from_millis(1300));
    assert_eq!(app.lane_feedback_strength(0), 0.0);
    assert_eq!(app.judgment_feedback_strength(), 0.0);
    assert_eq!(app.combo_feedback_strength(), 0.0);
}

#[test]
fn demo_state_reports_playback_progress_and_grade() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    assert_eq!(app.current_grade(), "E");
    assert_eq!(app.playback_progress_ratio(), 0.0);

    app.update(GameTime::from_millis(1000));
    assert!(app.handle_lane_press(0, GameTime::from_millis(1000)));
    assert!(app.playback_progress_ratio() > 0.20);
    assert!(app.playback_progress_ratio() < 0.40);
    assert_eq!(app.current_grade(), "A");
}

#[test]
fn demo_state_records_personal_best_when_entering_results() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_lane_press(0, GameTime::from_millis(1000)));
    app.update(GameTime::from_millis(5000));

    assert_eq!(app.mode(), DemoMode::Results);
    let record = app.result_record().expect("results should record run");
    assert_eq!(record.best_score, app.score_summary().score);
    assert_eq!(record.last_score, app.score_summary().score);
    assert_eq!(record.play_count, 1);
}

#[test]
fn demo_state_keeps_best_score_across_multiple_runs() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_lane_press(0, GameTime::from_millis(1000)));
    app.update(GameTime::from_millis(5000));
    let best = app.result_record().expect("first result").best_score;

    app.return_to_song_select();
    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(5000));

    let record = app.result_record().expect("second result");
    assert_eq!(record.best_score, best);
    assert!(record.last_score <= best);
    assert_eq!(record.play_count, 2);
}

#[test]
fn demo_state_exposes_recorded_replay_after_a_run() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_key_char('s', GameTime::from_millis(1000)));
    app.update(GameTime::from_millis(5000));

    let replay = app.last_replay().expect("run should expose replay");
    assert_eq!(replay.events.len(), 1);
    assert_eq!(replay.events[0].key, "S");
}

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

fn fixture_copy(folder: &str, suffix: &str) -> PathBuf {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder);
    let temp_root = temp_import_root(suffix);
    fs::copy(source_root.join("map.osu"), temp_root.join("map.osu")).expect("copy map");
    fs::copy(source_root.join("song.ogg"), temp_root.join("song.ogg")).expect("copy audio");
    temp_root
}

#[test]
fn demo_state_can_enter_and_exit_replay_view_from_results() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_key_char('s', GameTime::from_millis(1000)));
    app.update(GameTime::from_millis(5000));

    app.open_replay_view();
    assert_eq!(app.mode(), DemoMode::Replay);

    app.close_replay_view();
    assert_eq!(app.mode(), DemoMode::Results);
}

#[test]
fn demo_state_navigates_replay_timeline() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_key_char('s', GameTime::from_millis(1000)));
    assert!(app.handle_key_char('d', GameTime::from_millis(2000)));
    app.update(GameTime::from_millis(5000));

    app.open_replay_view();
    assert_eq!(app.replay_cursor(), Some(0));

    app.move_replay_cursor(1);
    assert_eq!(app.replay_cursor(), Some(1));

    app.move_replay_cursor(1);
    assert_eq!(app.replay_cursor(), Some(1));

    app.move_replay_cursor(-1);
    assert_eq!(app.replay_cursor(), Some(0));
}

#[test]
fn demo_state_can_play_replay_preview_and_advance_cursor() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_key_char('s', GameTime::from_millis(1000)));
    assert!(app.handle_key_char('d', GameTime::from_millis(2000)));
    app.update(GameTime::from_millis(5000));

    app.open_replay_view();
    assert_eq!(app.replay_cursor(), Some(0));
    assert!(!app.replay_preview_playing());

    app.toggle_replay_preview();
    assert!(app.replay_preview_playing());

    app.update(GameTime::from_millis(2000));
    assert_eq!(app.replay_cursor(), Some(1));

    app.update(GameTime::from_millis(5000));
    assert!(!app.replay_preview_playing());
}

#[test]
fn demo_state_replay_preview_replays_scoring_progress() {
    let mut app = DemoApp::from_demo_chart().expect("demo app should load");

    start_loaded_chart(&mut app);
    app.update(GameTime::from_millis(1000));
    assert!(app.handle_key_char('s', GameTime::from_millis(1000)));
    assert!(app.handle_key_char('d', GameTime::from_millis(2000)));
    app.update(GameTime::from_millis(5000));

    let live_summary = app.score_summary();
    app.open_replay_view();

    assert_eq!(app.replay_score_summary().score, 0);
    assert_eq!(app.replay_latest_judgment(), None);

    app.toggle_replay_preview();
    app.update(GameTime::from_millis(1000));
    assert_eq!(app.replay_score_summary().score, Judgment::Perfect.points());
    assert_eq!(app.replay_latest_judgment(), Some(Judgment::Perfect));

    app.update(GameTime::from_millis(2000));
    assert_eq!(
        app.replay_score_summary().score,
        Judgment::Perfect.points() * 2
    );

    app.update(GameTime::from_millis(5000));
    assert_eq!(app.replay_score_summary().score, live_summary.score);
    assert_eq!(app.replay_score_summary().accuracy, live_summary.accuracy);
}
