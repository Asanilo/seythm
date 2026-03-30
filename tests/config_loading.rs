use code_m::config::{
    load_default_settings, load_profile, load_settings, save_profile, save_settings, Keymap,
    ProfileRecord, ResultProfile, Settings,
};

#[test]
fn test_load_default_settings() {
    let settings = load_default_settings();

    assert_eq!(settings, Settings::default());
    assert_eq!(settings.keymap.as_string(), "S D F J K L");
    assert_eq!(settings.theme_path, "builtin:mocha-shell");
    assert!(settings.startup_splash_enabled);
}

#[test]
fn settings_round_trip_preserves_builtin_theme_choice() {
    let dir = std::env::temp_dir().join(format!("code_m_settings_builtin_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp settings dir");
    let path = dir.join("settings.toml");

    let settings = Settings {
        theme_path: "builtin:ghostty-cold".to_string(),
        ..Settings::default()
    };

    save_settings(&path, &settings).expect("settings should save");
    let reloaded = load_settings(&path).expect("settings should reload");

    assert_eq!(reloaded.theme_path, "builtin:ghostty-cold");
    assert_eq!(
        Settings::default().theme_path,
        "builtin:mocha-shell"
    );
}

#[test]
fn test_rejects_unrecognized_builtin_theme_ids() {
    let settings = Settings {
        theme_path: "builtin:typo".to_string(),
        ..Settings::default()
    };

    assert_eq!(settings.builtin_theme_name(), None);

    let empty = Settings {
        theme_path: "builtin:".to_string(),
        ..Settings::default()
    };

    assert_eq!(empty.builtin_theme_name(), None);
}

#[test]
fn test_parse_default_keymap() {
    let keymap = Keymap::parse("S D F J K L").expect("default keymap should parse");

    assert_eq!(
        keymap.keys(),
        &[
            "S".to_string(),
            "D".to_string(),
            "F".to_string(),
            "J".to_string(),
            "K".to_string(),
            "L".to_string(),
        ]
    );
    assert_eq!(keymap.as_string(), "S D F J K L");
}

#[test]
fn test_reject_non_six_key_keymap() {
    assert!(Keymap::parse("S D F J K").is_err());
    assert!(Keymap::parse("S D F J K L M").is_err());
}

#[test]
fn test_reject_invalid_keymap_in_settings_deserialization() {
    let config = r#"
keymap = "S D F J K"
theme_path = "builtin:mocha-shell"
"#;

    assert!(toml::from_str::<Settings>(config).is_err());
}

#[test]
fn test_theme_path_buf() {
    let settings = Settings {
        keymap: Keymap::default(),
        theme_path: "builtin:minimal-professional".to_string(),
        ..Settings::default()
    };

    assert_eq!(
        settings.theme_path_buf(),
        std::path::Path::new("builtin:minimal-professional")
    );
}

#[test]
fn test_load_missing_settings_path_falls_back_to_default() {
    let path = std::env::temp_dir().join(format!(
        "code_m_missing_{}_settings.toml",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    let settings = load_settings(&path).expect("missing settings should fall back to default");

    assert_eq!(settings, Settings::default());
}

#[test]
fn test_save_and_reload_settings_round_trip() {
    let dir = std::env::temp_dir().join(format!("code_m_settings_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp settings dir");
    let path = dir.join("settings.toml");

    let settings = Settings {
        keymap: Keymap::parse("A S D J K L").expect("custom keymap"),
        theme_path: "assets/themes/mono-contrast.toml".to_string(),
        metronome_enabled: false,
        global_offset_ms: -18,
        input_offset_ms: 12,
        music_volume: 65,
        hit_sound_volume: 55,
        startup_splash_enabled: false,
    };

    save_settings(&path, &settings).expect("settings should save");
    let reloaded = load_settings(&path).expect("settings should reload");

    assert_eq!(reloaded, settings);
}

#[test]
fn test_save_and_reload_profile_round_trip() {
    let dir = std::env::temp_dir().join(format!("code_m_profile_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp profile dir");
    let path = dir.join("profile.toml");

    let mut profile = ResultProfile::default();
    profile.songs.insert(
        "demo-basic".to_string(),
        ProfileRecord {
            best_score: 2_000,
            best_accuracy: 0.95,
            last_score: 1_500,
            last_accuracy: 0.75,
            play_count: 2,
        },
    );

    save_profile(&path, &profile).expect("profile should save");
    let reloaded = load_profile(&path).expect("profile should reload");

    assert_eq!(reloaded, profile);
}
