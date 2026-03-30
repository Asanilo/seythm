use crate::chart::model::{Chart, ChartMetadata, HoldNote, Note, NoteKind, TapNote, TimingPoint};
use serde::Deserialize;
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ChartParseError {
    #[error("failed to parse chart TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("failed to read chart file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid hold note: {0}")]
    InvalidHold(String),
    #[error("invalid lane {lane}; expected 0..={max_lane}")]
    InvalidLane { lane: u8, max_lane: u8 },
}

#[derive(Debug, Deserialize)]
struct RawChart {
    metadata: ChartMetadata,
    #[serde(default)]
    timing: Vec<RawTimingPoint>,
    #[serde(default)]
    notes: Vec<RawNote>,
}

#[derive(Debug, Deserialize)]
struct RawTimingPoint {
    start_ms: u32,
    bpm: f32,
    #[serde(default = "default_beat_length")]
    beat_length: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum RawNote {
    Tap {
        time_ms: u32,
        lane: u8,
    },
    Hold {
        start_ms: u32,
        end_ms: u32,
        lane: u8,
    },
}

fn default_beat_length() -> u32 {
    4
}

pub fn parse_chart_str(input: &str) -> Result<Chart, ChartParseError> {
    let raw: RawChart = toml::from_str(input)?;
    Ok(convert_chart(raw)?)
}

pub fn parse_chart_file(path: impl AsRef<Path>) -> Result<Chart, ChartParseError> {
    let path_ref = path.as_ref();
    let contents = fs::read_to_string(path_ref).map_err(|source| ChartParseError::Io {
        path: path_ref.display().to_string(),
        source,
    })?;
    parse_chart_str(&contents)
}

fn convert_chart(raw: RawChart) -> Result<Chart, ChartParseError> {
    let mut timing: Vec<TimingPoint> = raw
        .timing
        .into_iter()
        .map(|point| TimingPoint {
            start_ms: point.start_ms,
            bpm: point.bpm,
            beat_length: point.beat_length,
        })
        .collect();
    timing.sort_by(|left, right| left.start_ms.cmp(&right.start_ms));

    let mut notes = raw
        .notes
        .into_iter()
        .map(convert_note)
        .collect::<Result<Vec<_>, _>>()?;
    notes.sort_by(compare_notes);

    Ok(Chart {
        metadata: raw.metadata,
        timing,
        notes,
    })
}

fn convert_note(raw: RawNote) -> Result<Note, ChartParseError> {
    match raw {
        RawNote::Tap { time_ms, lane } => Ok(Note::Tap(TapNote {
            kind: NoteKind::Tap,
            time_ms,
            lane: validate_lane(lane)?,
        })),
        RawNote::Hold {
            start_ms,
            end_ms,
            lane,
        } => {
            if end_ms <= start_ms {
                return Err(ChartParseError::InvalidHold(
                    "hold note end_ms must be greater than start_ms".to_string(),
                ));
            }

            Ok(Note::Hold(HoldNote {
                kind: NoteKind::Hold,
                start_ms,
                end_ms,
                lane: validate_lane(lane)?,
            }))
        }
    }
}

fn validate_lane(lane: u8) -> Result<u8, ChartParseError> {
    const MAX_LANE: u8 = 5;

    if lane <= MAX_LANE {
        Ok(lane)
    } else {
        Err(ChartParseError::InvalidLane {
            lane,
            max_lane: MAX_LANE,
        })
    }
}

fn compare_notes(left: &Note, right: &Note) -> Ordering {
    left.timestamp_ms()
        .cmp(&right.timestamp_ms())
        .then_with(|| left.lane().cmp(&right.lane()))
}
