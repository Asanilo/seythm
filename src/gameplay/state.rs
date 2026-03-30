use crate::chart::model::{Chart, Note, NoteKind};
use crate::runtime::clock::GameTime;

use super::judgment::{Judgment, JudgmentWindows};
use super::scoring::{ScoreSummary, ScoreTotals};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitPhase {
    Tap,
    HoldStart,
    HoldRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitEvent {
    pub lane: u8,
    pub phase: HitPhase,
    pub judgment: Judgment,
}

#[derive(Debug, Clone)]
pub struct GameplayState {
    windows: JudgmentWindows,
    scoring: ScoreTotals,
    notes: Vec<RuntimeNote>,
}

#[derive(Debug, Clone)]
struct RuntimeNote {
    note: Note,
    start_hit: bool,
    release_hit: bool,
    missed_start: bool,
    missed_release: bool,
}

impl RuntimeNote {
    fn new(note: Note) -> Self {
        Self {
            note,
            start_hit: false,
            release_hit: false,
            missed_start: false,
            missed_release: false,
        }
    }

    fn lane(&self) -> u8 {
        self.note.lane()
    }

    fn timestamp_ms(&self) -> i64 {
        self.note.timestamp_ms() as i64
    }

    fn kind(&self) -> NoteKind {
        match &self.note {
            Note::Tap(_) => NoteKind::Tap,
            Note::Hold(_) => NoteKind::Hold,
        }
    }

    fn hold_end_ms(&self) -> Option<i64> {
        match &self.note {
            Note::Hold(note) => Some(note.end_ms as i64),
            Note::Tap(_) => None,
        }
    }

    fn is_fully_resolved(&self) -> bool {
        match self.kind() {
            NoteKind::Tap => self.start_hit || self.missed_start,
            NoteKind::Hold => {
                (self.start_hit && (self.release_hit || self.missed_release)) || self.missed_start
            }
        }
    }

    fn is_start_pending(&self) -> bool {
        !self.start_hit && !self.missed_start
    }

    fn is_hold_armed(&self) -> bool {
        self.kind() == NoteKind::Hold && self.start_hit && !self.release_hit && !self.missed_release
    }
}

impl GameplayState {
    pub fn new(chart: Chart) -> Self {
        let mut notes = chart
            .notes
            .into_iter()
            .map(RuntimeNote::new)
            .collect::<Vec<_>>();
        notes.sort_by(|left, right| {
            left.timestamp_ms()
                .cmp(&right.timestamp_ms())
                .then_with(|| left.lane().cmp(&right.lane()))
        });

        let possible_points = notes.iter().fold(0, |acc, note| {
            acc + match note.kind() {
                NoteKind::Tap => Judgment::Perfect.points(),
                NoteKind::Hold => Judgment::Perfect.points() * 2,
            }
        });

        Self {
            windows: JudgmentWindows::default(),
            scoring: ScoreTotals::new(possible_points),
            notes,
        }
    }

    pub fn press_lane(&mut self, lane: u8, time: GameTime) -> Option<HitEvent> {
        self.resolve_expired_notes(time);
        self.try_press_lane(lane, time)
    }

    pub fn release_lane(&mut self, lane: u8, time: GameTime) -> Option<HitEvent> {
        self.resolve_expired_notes(time);

        let now = time.as_millis();
        let index = self
            .notes
            .iter()
            .position(|note| note.lane() == lane && note.is_hold_armed())?;
        let note = &mut self.notes[index];
        let end_ms = note.hold_end_ms().expect("hold note");
        let judgment = self.windows.classify_offset(now - end_ms);
        note.release_hit = true;
        self.scoring.apply(judgment);

        Some(HitEvent {
            lane,
            phase: HitPhase::HoldRelease,
            judgment,
        })
    }

    pub fn advance_to(&mut self, time: GameTime) -> Vec<HitEvent> {
        self.resolve_expired_notes(time)
    }

    pub fn summary(&self) -> ScoreSummary {
        self.scoring.summary()
    }

    pub fn all_notes_resolved(&self) -> bool {
        self.notes.iter().all(RuntimeNote::is_fully_resolved)
    }

    fn try_press_lane(&mut self, lane: u8, time: GameTime) -> Option<HitEvent> {
        let now = time.as_millis();

        loop {
            let index = self
                .notes
                .iter()
                .position(|note| note.lane() == lane && !note.is_fully_resolved())?;

            if self.notes[index].is_hold_armed() {
                return None;
            }

            let note_kind = self.notes[index].kind();
            let timestamp_ms = self.notes[index].timestamp_ms();
            let judgment = self.windows.classify_offset(now - timestamp_ms);

            if judgment.is_hit() {
                let note = &mut self.notes[index];
                match note_kind {
                    NoteKind::Tap => note.start_hit = true,
                    NoteKind::Hold => note.start_hit = true,
                }
                self.scoring.apply(judgment);

                let phase = match note_kind {
                    NoteKind::Tap => HitPhase::Tap,
                    NoteKind::Hold => HitPhase::HoldStart,
                };

                return Some(HitEvent {
                    lane,
                    phase,
                    judgment,
                });
            }

            if now > timestamp_ms + self.windows.good_ms() {
                self.miss_note(index);
                continue;
            }

            return None;
        }
    }

    fn resolve_expired_notes(&mut self, time: GameTime) -> Vec<HitEvent> {
        let now = time.as_millis();
        let mut events = Vec::new();

        for index in 0..self.notes.len() {
            if self.notes[index].is_start_pending() {
                let note_kind = self.notes[index].kind();
                let timestamp_ms = self.notes[index].timestamp_ms();

                if now > timestamp_ms + self.windows.good_ms() {
                    let lane = self.notes[index].lane();
                    self.miss_note(index);
                    let phase = match note_kind {
                        NoteKind::Tap => HitPhase::Tap,
                        NoteKind::Hold => HitPhase::HoldStart,
                    };
                    events.push(HitEvent {
                        lane,
                        phase,
                        judgment: Judgment::Miss,
                    });
                }
                continue;
            }

            if self.notes[index].is_hold_armed() {
                let Some(end_ms) = self.notes[index].hold_end_ms() else {
                    continue;
                };

                if now > end_ms + self.windows.good_ms() {
                    let lane = self.notes[index].lane();
                    self.notes[index].missed_release = true;
                    self.scoring.apply(Judgment::Miss);
                    events.push(HitEvent {
                        lane,
                        phase: HitPhase::HoldRelease,
                        judgment: Judgment::Miss,
                    });
                }
            }
        }

        events
    }

    fn miss_note(&mut self, index: usize) {
        self.notes[index].missed_start = true;
        self.scoring.apply(Judgment::Miss);
    }
}
