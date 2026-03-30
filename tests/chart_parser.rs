use code_m::chart::model::{Note, NoteKind};
use code_m::chart::parser::{parse_chart_file, parse_chart_str};
use std::path::PathBuf;

#[test]
fn test_parse_chart_metadata() {
    let chart = parse_chart_str(
        r#"
[metadata]
title = "Demo Basic"
artist = "Code M"
chart_name = "Normal"
offset_ms = 42

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[notes]]
kind = "tap"
time_ms = 1000
lane = 2
"#,
    )
    .expect("chart should parse");

    assert_eq!(chart.metadata.title, "Demo Basic");
    assert_eq!(chart.metadata.artist, "Code M");
    assert_eq!(chart.metadata.chart_name, "Normal");
    assert_eq!(chart.metadata.offset_ms, 42);
}

#[test]
fn test_parse_chart_timing_sections() {
    let chart = parse_chart_str(
        r#"
[metadata]
title = "Timing Demo"
artist = "Code M"
chart_name = "Normal"

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[timing]]
start_ms = 30000
bpm = 150.0
beat_length = 4

[[notes]]
kind = "tap"
time_ms = 1000
lane = 0
"#,
    )
    .expect("chart should parse");

    assert_eq!(chart.timing.len(), 2);
    assert_eq!(chart.timing[0].start_ms, 0);
    assert_eq!(chart.timing[0].bpm, 120.0);
    assert_eq!(chart.timing[1].start_ms, 30000);
    assert_eq!(chart.timing[1].bpm, 150.0);
}

#[test]
fn test_parse_tap_note() {
    let chart = parse_chart_str(
        r#"
[metadata]
title = "Tap Demo"
artist = "Code M"
chart_name = "Normal"

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[notes]]
kind = "tap"
time_ms = 1234
lane = 4
"#,
    )
    .expect("chart should parse");

    assert_eq!(chart.notes.len(), 1);
    let note = &chart.notes[0];
    assert!(
        matches!(note, Note::Tap(tap) if tap.kind == NoteKind::Tap && tap.time_ms == 1234 && tap.lane == 4)
    );
}

#[test]
fn test_parse_hold_note() {
    let chart = parse_chart_str(
        r#"
[metadata]
title = "Hold Demo"
artist = "Code M"
chart_name = "Normal"

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[notes]]
kind = "hold"
start_ms = 2000
end_ms = 3500
lane = 1
"#,
    )
    .expect("chart should parse");

    assert_eq!(chart.notes.len(), 1);
    let note = &chart.notes[0];
    assert!(
        matches!(note, Note::Hold(hold) if hold.kind == NoteKind::Hold && hold.start_ms == 2000 && hold.end_ms == 3500 && hold.lane == 1)
    );
}

#[test]
fn test_parse_hold_note_rejects_invalid_lane() {
    let err = parse_chart_str(
        r#"
[metadata]
title = "Hold Demo"
artist = "Code M"
chart_name = "Normal"

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[notes]]
kind = "hold"
start_ms = 2000
end_ms = 3500
lane = 6
"#,
    )
    .expect_err("chart should reject invalid lane");

    assert!(
        err.to_string().contains("invalid lane"),
        "unexpected error: {err}"
    );
}

#[test]
fn test_notes_are_sorted_by_time_then_lane() {
    let chart = parse_chart_str(
        r#"
[metadata]
title = "Ordering Demo"
artist = "Code M"
chart_name = "Normal"

[[timing]]
start_ms = 0
bpm = 120.0
beat_length = 4

[[notes]]
kind = "tap"
time_ms = 2000
lane = 3

[[notes]]
kind = "tap"
time_ms = 1000
lane = 4

[[notes]]
kind = "tap"
time_ms = 2000
lane = 1
"#,
    )
    .expect("chart should parse");

    let lanes: Vec<u8> = chart.notes.iter().map(|note| note.lane()).collect();
    let times: Vec<u32> = chart.notes.iter().map(|note| note.timestamp_ms()).collect();

    assert_eq!(times, vec![1000, 2000, 2000]);
    assert_eq!(lanes, vec![4, 1, 3]);
}

#[test]
fn test_parse_chart_file_entry_point() {
    let chart_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/charts/demo_basic.toml");
    let chart = parse_chart_file(chart_path).expect("chart file should parse");

    assert_eq!(chart.metadata.title, "Demo Basic");
    assert_eq!(chart.notes.len(), 4);
}
