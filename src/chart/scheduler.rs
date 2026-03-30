use crate::chart::model::{Chart, Note, NoteKind};
use crate::runtime::clock::GameTime;

#[derive(Debug, Clone)]
pub struct ChartScheduler {
    chart: Chart,
    approach_window_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaneNoteProjection {
    pub lane: u8,
    pub kind: NoteKind,
    pub timestamp_ms: i64,
    pub end_timestamp_ms: Option<i64>,
    pub approach_position: f32,
    pub is_active: bool,
}

impl ChartScheduler {
    pub fn new(chart: Chart, approach_window_ms: i64) -> Self {
        Self {
            chart,
            approach_window_ms,
        }
    }

    pub fn visible_notes(&self, time: GameTime) -> Vec<LaneNoteProjection> {
        let mut notes = self
            .chart
            .notes
            .iter()
            .filter_map(|note| self.project_note(note, time))
            .collect::<Vec<_>>();

        notes.sort_by(compare_projection);
        notes
    }

    pub fn project_note(&self, note: &Note, time: GameTime) -> Option<LaneNoteProjection> {
        match note {
            Note::Tap(tap) => self.project_tap(tap.time_ms as i64, tap.lane, time),
            Note::Hold(hold) => {
                self.project_hold(hold.start_ms as i64, hold.end_ms as i64, hold.lane, time)
            }
        }
    }

    fn project_tap(
        &self,
        timestamp_ms: i64,
        lane: u8,
        time: GameTime,
    ) -> Option<LaneNoteProjection> {
        let visibility_start = timestamp_ms - self.approach_window_ms;
        let current_time = time.as_millis();

        if current_time < visibility_start || current_time > timestamp_ms {
            return None;
        }

        Some(LaneNoteProjection {
            lane,
            kind: NoteKind::Tap,
            timestamp_ms,
            end_timestamp_ms: None,
            approach_position: approach_position(timestamp_ms, time, self.approach_window_ms),
            is_active: false,
        })
    }

    fn project_hold(
        &self,
        start_ms: i64,
        end_ms: i64,
        lane: u8,
        time: GameTime,
    ) -> Option<LaneNoteProjection> {
        let visibility_start = start_ms - self.approach_window_ms;
        let current_time = time.as_millis();

        if current_time < visibility_start || current_time > end_ms {
            return None;
        }

        let is_active = current_time >= start_ms && current_time <= end_ms;
        let approach_position = if is_active {
            0.0
        } else {
            approach_position(start_ms, time, self.approach_window_ms)
        };

        Some(LaneNoteProjection {
            lane,
            kind: NoteKind::Hold,
            timestamp_ms: start_ms,
            end_timestamp_ms: Some(end_ms),
            approach_position,
            is_active,
        })
    }
}

pub fn approach_position(note_timestamp_ms: i64, time: GameTime, approach_window_ms: i64) -> f32 {
    if approach_window_ms <= 0 {
        return 0.0;
    }

    let remaining_ms = note_timestamp_ms - time.as_millis();
    let normalized = remaining_ms as f32 / approach_window_ms as f32;

    normalized.clamp(0.0, 1.0)
}

fn compare_projection(left: &LaneNoteProjection, right: &LaneNoteProjection) -> std::cmp::Ordering {
    left.timestamp_ms
        .cmp(&right.timestamp_ms)
        .then_with(|| left.lane.cmp(&right.lane))
        .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
        .then_with(|| left.end_timestamp_ms.cmp(&right.end_timestamp_ms))
}

fn kind_rank(kind: NoteKind) -> u8 {
    match kind {
        NoteKind::Tap => 0,
        NoteKind::Hold => 1,
    }
}
