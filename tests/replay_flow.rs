use code_m::chart::model::{Chart, ChartMetadata, Note, NoteKind, TapNote};
use code_m::config::keymap::Keymap;
use code_m::gameplay::judgment::Judgment;
use code_m::runtime::{run_replay, InputAction, InputEvent, ReplayInput};

fn chart(notes: Vec<Note>) -> Chart {
    Chart {
        metadata: ChartMetadata {
            title: "Replay Demo".to_string(),
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

#[test]
fn test_replay_flow_maps_keys_orders_hits_and_summarizes_results() {
    let result = run_replay(
        chart(vec![tap(1000, 0), tap(1000, 1), tap(2000, 2)]),
        ReplayInput::new(
            Keymap::default(),
            vec![
                InputEvent::new("D", 1000, InputAction::Press),
                InputEvent::new("S", 1000, InputAction::Press),
                InputEvent::new("F", 2000, InputAction::Press),
            ],
        ),
    );

    let judgments: Vec<(u8, Judgment)> = result
        .events
        .iter()
        .map(|event| (event.lane, event.judgment))
        .collect();

    assert_eq!(
        judgments,
        vec![
            (1, Judgment::Perfect),
            (0, Judgment::Perfect),
            (2, Judgment::Perfect),
        ]
    );
    assert_eq!(result.summary.score, 3000);
    assert_eq!(result.summary.combo, 3);
    assert_eq!(result.summary.max_combo, 3);
    assert_eq!(result.summary.judgments.perfect, 3);
    assert_eq!(result.summary.judgments.great, 0);
    assert_eq!(result.summary.judgments.good, 0);
    assert_eq!(result.summary.judgments.miss, 0);
}
