use code_m::chart::model::{Chart, ChartMetadata, HoldNote, Note, NoteKind, TapNote};
use code_m::gameplay::judgment::Judgment;
use code_m::gameplay::state::{GameplayState, HitPhase};
use code_m::runtime::clock::GameTime;

fn chart(notes: Vec<Note>) -> Chart {
    Chart {
        metadata: ChartMetadata {
            title: "Gameplay Demo".to_string(),
            artist: "Code M".to_string(),
            chart_name: "Normal".to_string(),
            offset_ms: 0,
        },
        timing: Vec::new(),
        notes,
    }
}

fn tap(time_ms: u32, lane: u8) -> Note {
    Note::Tap(TapNote {
        kind: NoteKind::Tap,
        time_ms,
        lane,
    })
}

fn hold(start_ms: u32, end_ms: u32, lane: u8) -> Note {
    Note::Hold(HoldNote {
        kind: NoteKind::Hold,
        start_ms,
        end_ms,
        lane,
    })
}

#[test]
fn test_note_hits_are_resolved_by_lane_and_time() {
    let mut state = GameplayState::new(chart(vec![tap(1000, 0), tap(1000, 1)]));

    let wrong_lane = state.press_lane(2, GameTime::from_millis(1000));
    assert!(wrong_lane.is_none());

    let first = state.press_lane(1, GameTime::from_millis(1000)).unwrap();
    assert_eq!(first.lane, 1);
    assert_eq!(first.phase, HitPhase::Tap);
    assert_eq!(first.judgment, Judgment::Perfect);

    let second = state.press_lane(0, GameTime::from_millis(1000)).unwrap();
    assert_eq!(second.lane, 0);
    assert_eq!(second.phase, HitPhase::Tap);
    assert_eq!(second.judgment, Judgment::Perfect);
}

#[test]
fn test_combo_increments_and_breaks_on_miss() {
    let mut state = GameplayState::new(chart(vec![tap(1000, 0), tap(2000, 1)]));

    let first = state.press_lane(0, GameTime::from_millis(1000)).unwrap();
    assert_eq!(first.judgment, Judgment::Perfect);
    assert_eq!(state.summary().combo, 1);

    let misses = state.advance_to(GameTime::from_millis(2100));
    assert_eq!(misses.len(), 1);
    assert_eq!(misses[0].judgment, Judgment::Miss);
    assert_eq!(state.summary().combo, 0);
    assert_eq!(state.summary().max_combo, 1);
}

#[test]
fn test_accuracy_and_score_totals_include_judgment_weights() {
    let mut state = GameplayState::new(chart(vec![tap(1000, 0), tap(2000, 1)]));

    let first = state.press_lane(0, GameTime::from_millis(1000)).unwrap();
    assert_eq!(first.judgment, Judgment::Perfect);

    let second = state.press_lane(1, GameTime::from_millis(2085)).unwrap();
    assert_eq!(second.judgment, Judgment::Good);

    let summary = state.summary();
    assert_eq!(summary.score, 1500);
    assert_eq!(summary.combo, 2);
    assert_eq!(summary.max_combo, 2);
    assert!((summary.accuracy - 0.75).abs() < f32::EPSILON);
}

#[test]
fn test_hold_notes_score_start_and_release_separately() {
    let mut state = GameplayState::new(chart(vec![hold(1000, 2000, 3)]));

    let start = state.press_lane(3, GameTime::from_millis(1000)).unwrap();
    assert_eq!(start.phase, HitPhase::HoldStart);
    assert_eq!(start.judgment, Judgment::Perfect);
    assert_eq!(state.summary().score, 1000);
    assert_eq!(state.summary().combo, 1);

    let before_release = state.advance_to(GameTime::from_millis(2080));
    assert!(before_release.is_empty());
    assert_eq!(state.summary().score, 1000);
    assert_eq!(state.summary().combo, 1);

    let release = state.release_lane(3, GameTime::from_millis(2005)).unwrap();
    assert_eq!(release.phase, HitPhase::HoldRelease);
    assert_eq!(release.judgment, Judgment::Perfect);

    let summary = state.summary();
    assert_eq!(summary.score, 2000);
    assert_eq!(summary.combo, 2);
    assert_eq!(summary.max_combo, 2);
    assert!((summary.accuracy - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_hold_release_misses_when_release_window_expires() {
    let mut state = GameplayState::new(chart(vec![hold(1000, 2000, 3)]));

    let start = state.press_lane(3, GameTime::from_millis(1000)).unwrap();
    assert_eq!(start.phase, HitPhase::HoldStart);
    assert_eq!(start.judgment, Judgment::Perfect);

    let release_miss = state.advance_to(GameTime::from_millis(2091));
    assert_eq!(release_miss.len(), 1);
    assert_eq!(release_miss[0].phase, HitPhase::HoldRelease);
    assert_eq!(release_miss[0].judgment, Judgment::Miss);

    let summary = state.summary();
    assert_eq!(summary.score, 1000);
    assert_eq!(summary.combo, 0);
    assert_eq!(summary.max_combo, 1);
}
