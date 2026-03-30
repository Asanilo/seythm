use std::time::Instant;

use crate::runtime::GameTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaybackOffset(i64);

impl PlaybackOffset {
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlaybackClock {
    stream_started_at: Instant,
    offset: PlaybackOffset,
}

impl PlaybackClock {
    pub fn new(stream_started_at: Instant) -> Self {
        Self::with_offset(stream_started_at, PlaybackOffset::default())
    }

    pub fn with_offset(stream_started_at: Instant, offset: PlaybackOffset) -> Self {
        Self {
            stream_started_at,
            offset,
        }
    }

    pub fn playback_time(&self, observed_at: Instant) -> GameTime {
        let elapsed_millis = observed_at
            .saturating_duration_since(self.stream_started_at)
            .as_millis() as i64;

        GameTime::from_millis(elapsed_millis + self.offset.as_millis())
    }

    pub fn offset(&self) -> PlaybackOffset {
        self.offset
    }

    pub fn stream_started_at(&self) -> Instant {
        self.stream_started_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockDriftCorrection {
    max_step_millis: i64,
}

impl ClockDriftCorrection {
    pub const fn new(max_step_millis: i64) -> Self {
        Self { max_step_millis }
    }

    pub const fn max_step_millis(self) -> i64 {
        self.max_step_millis
    }

    pub fn correct(&self, drift_millis: i64) -> i64 {
        drift_millis.clamp(-self.max_step_millis, self.max_step_millis)
    }

    pub fn corrected_offset(&self, offset: PlaybackOffset, drift_millis: i64) -> PlaybackOffset {
        PlaybackOffset::from_millis(offset.as_millis() + self.correct(drift_millis))
    }
}
