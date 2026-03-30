use std::time::{Duration, Instant};

use code_m::audio::clock_sync::{ClockDriftCorrection, PlaybackClock, PlaybackOffset};

#[test]
fn clock_sync_converts_stream_start_and_elapsed_into_playback_time() {
    let stream_started_at = Instant::now();
    let clock = PlaybackClock::new(stream_started_at);
    let observed_at = stream_started_at + Duration::from_millis(375);

    let playback_time = clock.playback_time(observed_at);

    assert_eq!(playback_time.as_millis(), 375);
}

#[test]
fn clock_sync_applies_output_offset_to_playback_time() {
    let stream_started_at = Instant::now();
    let clock = PlaybackClock::with_offset(stream_started_at, PlaybackOffset::from_millis(-18));
    let observed_at = stream_started_at + Duration::from_millis(250);

    let playback_time = clock.playback_time(observed_at);

    assert_eq!(playback_time.as_millis(), 232);
}

#[test]
fn clock_sync_clamps_drift_correction_to_bounds() {
    let correction = ClockDriftCorrection::new(15);

    assert_eq!(correction.correct(42), 15);
    assert_eq!(correction.correct(-42), -15);
    assert_eq!(correction.correct(7), 7);
}
