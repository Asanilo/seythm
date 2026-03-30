#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsuMode {
    Mania,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OsuBeatmap {
    pub mode: OsuMode,
    pub key_count: u8,
    pub audio_filename: String,
    pub metadata: OsuMetadata,
    pub timing_points: Vec<OsuTimingPoint>,
    pub hit_objects: Vec<OsuHitObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OsuMetadata {
    pub title: Option<String>,
    pub title_unicode: Option<String>,
    pub artist: Option<String>,
    pub artist_unicode: Option<String>,
    pub creator: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OsuTimingPoint {
    pub time_ms: i32,
    pub beat_length: f64,
    pub meter: u8,
    pub uninherited: bool,
    pub kiai: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsuHitObject {
    Tap {
        lane: u8,
        time_ms: i32,
    },
    Hold {
        lane: u8,
        start_time_ms: i32,
        end_time_ms: i32,
    },
}
