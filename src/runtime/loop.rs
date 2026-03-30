use crate::chart::model::Chart;
use crate::gameplay::{GameplayState, HitEvent, HitPhase, Judgment, ScoreSummary};
use crate::runtime::clock::GameTime;

use super::input::{lane_for_key, InputAction, ReplayInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvent {
    pub lane: u8,
    pub phase: HitPhase,
    pub judgment: Judgment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayResult {
    pub events: Vec<ReplayEvent>,
    pub summary: ScoreSummary,
}

pub fn run_replay(chart: Chart, input: ReplayInput) -> ReplayResult {
    let mut state = GameplayState::new(chart);
    let mut replay_result_events = Vec::new();
    let ReplayInput {
        keymap,
        events: input_events,
    } = input;

    let mut replay_events = input_events.into_iter().enumerate().collect::<Vec<_>>();
    replay_events.sort_by(|left, right| {
        left.1
            .timestamp
            .cmp(&right.1.timestamp)
            .then_with(|| left.0.cmp(&right.0))
    });

    for (_, event) in replay_events {
        replay_result_events.extend(
            state
                .advance_to(event.timestamp)
                .into_iter()
                .map(convert_hit_event),
        );

        let Some(lane) = lane_for_key(&keymap, &event.key) else {
            continue;
        };

        let hit = match event.action {
            InputAction::Press => state.press_lane(lane, event.timestamp),
            InputAction::Release => state.release_lane(lane, event.timestamp),
        };

        if let Some(hit) = hit {
            replay_result_events.push(convert_hit_event(hit));
        }
    }

    replay_result_events.extend(
        state
            .advance_to(GameTime::from_millis(i64::MAX / 4))
            .into_iter()
            .map(convert_hit_event),
    );

    ReplayResult {
        events: replay_result_events,
        summary: state.summary(),
    }
}

fn convert_hit_event(event: HitEvent) -> ReplayEvent {
    ReplayEvent {
        lane: event.lane,
        phase: event.phase,
        judgment: event.judgment,
    }
}
