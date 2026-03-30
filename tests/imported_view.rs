use code_m::app::DemoApp;
use code_m::osu::import::import_osu_mania_folder;
use code_m::ui::widgets::render_imported_song_select;
use code_m::ui::ThemeTokens;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use std::fs;
use std::path::PathBuf;

#[test]
fn imported_view_render_uses_shell_bars_and_imported_metadata() {
    let import_root = temp_import_root("imported-view");
    let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "imported-view-source");
    import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

    let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
        .expect("runtime app should load imported catalog");
    app.open_imported_view();

    let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal should build");

    terminal
        .draw(|frame| render_imported_song_select(frame, &app, &theme))
        .expect("imported view should render");

    let buffer = terminal.backend().buffer();
    assert!(buffer_contains_text(buffer, "Seythm"));
    assert!(buffer_contains_text(buffer, "Imported"));
    assert!(buffer_contains_text(buffer, "Navigate"));
    assert!(buffer_contains_text(buffer, "Minimal Mania 6K"));
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

fn buffer_contains_text(buffer: &Buffer, needle: &str) -> bool {
    let text = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    text.contains(needle)
}
