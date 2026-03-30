use code_m::chart::scheduler::ChartScheduler;
use code_m::chart::{Chart, ChartMetadata, HoldNote, Note, NoteKind, TapNote};
use code_m::runtime::clock::GameTime;

fn demo_chart(notes: Vec<Note>) -> Chart {
    Chart {
        metadata: ChartMetadata {
            title: "Scheduler Demo".to_string(),
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

fn projection_summary(scheduler: &ChartScheduler, time_ms: i64) -> Vec<(u8, NoteKind, f32, bool)> {
    scheduler
        .visible_notes(GameTime::from_millis(time_ms))
        .into_iter()
        .map(|note| (note.lane, note.kind, note.approach_position, note.is_active))
        .collect()
}

#[test]
fn test_project_tap_note_to_approach_position() {
    let scheduler = ChartScheduler::new(demo_chart(vec![tap(3000, 2)]), 1000);

    let notes = scheduler.visible_notes(GameTime::from_millis(2500));

    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].lane, 2);
    assert_eq!(notes[0].kind, NoteKind::Tap);
    assert!((notes[0].approach_position - 0.5).abs() < f32::EPSILON);
    assert!(!notes[0].is_active);
}

#[test]
fn test_visible_notes_are_selected_for_time_slice() {
    let scheduler = ChartScheduler::new(
        demo_chart(vec![tap(1000, 0), hold(2000, 3000, 1), tap(5000, 3)]),
        1000,
    );

    let notes = projection_summary(&scheduler, 1800);

    assert_eq!(notes, vec![(1, NoteKind::Hold, 0.2, false)]);
}

#[test]
fn test_visible_notes_preserve_deterministic_ordering() {
    let scheduler = ChartScheduler::new(
        demo_chart(vec![hold(3000, 4200, 4), tap(3000, 4), tap(3000, 1)]),
        1000,
    );

    let notes = projection_summary(&scheduler, 2500);

    assert_eq!(
        notes,
        vec![
            (1, NoteKind::Tap, 0.5, false),
            (4, NoteKind::Tap, 0.5, false),
            (4, NoteKind::Hold, 0.5, false),
        ]
    );
}

#[test]
fn test_hold_note_active_window_is_visible_through_release() {
    let scheduler = ChartScheduler::new(demo_chart(vec![hold(2000, 3500, 5)]), 1000);

    let lead_in = projection_summary(&scheduler, 1500);
    let active = projection_summary(&scheduler, 2500);
    let after_release = projection_summary(&scheduler, 3600);

    assert_eq!(lead_in, vec![(5, NoteKind::Hold, 0.5, false)]);
    assert_eq!(active, vec![(5, NoteKind::Hold, 0.0, true)]);
    assert!(after_release.is_empty());
}
