use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Chart {
    pub metadata: ChartMetadata,
    #[serde(default)]
    pub timing: Vec<TimingPoint>,
    #[serde(default)]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ChartMetadata {
    pub title: String,
    pub artist: String,
    pub chart_name: String,
    #[serde(default)]
    pub offset_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TimingPoint {
    pub start_ms: u32,
    pub bpm: f32,
    pub beat_length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteKind {
    Tap,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TapNote {
    pub kind: NoteKind,
    pub time_ms: u32,
    pub lane: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct HoldNote {
    pub kind: NoteKind,
    pub start_ms: u32,
    pub end_ms: u32,
    pub lane: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Note {
    Tap(TapNote),
    Hold(HoldNote),
}

impl Note {
    pub fn lane(&self) -> u8 {
        match self {
            Note::Tap(note) => note.lane,
            Note::Hold(note) => note.lane,
        }
    }

    pub fn timestamp_ms(&self) -> u32 {
        match self {
            Note::Tap(note) => note.time_ms,
            Note::Hold(note) => note.start_ms,
        }
    }
}
