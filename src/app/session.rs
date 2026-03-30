use std::time::Instant;

use crate::audio::PlaybackSession;
use crate::runtime::GameTime;

use super::{DemoApp, DemoMode};

#[derive(Debug, Clone, Copy)]
pub(crate) struct DemoClockAnchor {
    instant: Instant,
    base_time: GameTime,
}

impl DemoClockAnchor {
    pub(crate) fn new(instant: Instant, base_time: GameTime) -> Self {
        Self { instant, base_time }
    }

    fn reset(&mut self, instant: Instant, base_time: GameTime) {
        self.instant = instant;
        self.base_time = base_time;
    }

    fn playback_time(self, observed_at: Instant) -> GameTime {
        GameTime::from_millis(
            self.base_time.as_millis()
                + observed_at
                    .saturating_duration_since(self.instant)
                    .as_millis() as i64,
        )
    }

    fn base_time(self) -> GameTime {
        self.base_time
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaybackTransition {
    Keep,
    StartOrResume,
    StopAndFreeze,
}

pub(crate) fn refresh_playback_session(
    app: &DemoApp,
    playback: &mut Option<PlaybackSession>,
    clock_anchor: &mut DemoClockAnchor,
    session_generation: &mut u64,
    mode: &mut DemoMode,
) {
    let generation_changed = app.session_generation() != *session_generation;
    let transition = playback_transition(*mode, app.mode(), generation_changed);

    match transition {
        PlaybackTransition::Keep => {}
        PlaybackTransition::StartOrResume => {
            clock_anchor.reset(Instant::now(), app.current_time);
            *playback = app
                .playback_song()
                .audio_path()
                .and_then(|path| {
                    PlaybackSession::try_start_audio_file_at(
                        path,
                        app.current_time,
                        app.music_volume(),
                        app.hit_sound_volume(),
                    )
                    .ok()
                })
                .or_else(|| {
                    if app.metronome_enabled() {
                        PlaybackSession::try_start_metronome_at(
                            app.playback_chart(),
                            app.current_time,
                            app.music_volume(),
                            app.hit_sound_volume(),
                        )
                        .ok()
                    } else {
                        None
                    }
                });
        }
        PlaybackTransition::StopAndFreeze => {
            clock_anchor.reset(Instant::now(), app.current_time);
            *playback = None;
        }
    }

    *session_generation = app.session_generation();
    *mode = app.mode();
}

pub(crate) fn current_demo_time(
    playback: Option<&PlaybackSession>,
    clock_anchor: DemoClockAnchor,
    mode: DemoMode,
    global_offset_ms: i32,
) -> GameTime {
    if !matches!(
        mode,
        DemoMode::Loading | DemoMode::Ready | DemoMode::Playing | DemoMode::Calibration
    ) {
        return clock_anchor.base_time();
    }

    let base = playback
        .map(|session| session.playback_time(Instant::now()))
        .unwrap_or_else(|| clock_anchor.playback_time(Instant::now()));

    if matches!(mode, DemoMode::Playing | DemoMode::Calibration) {
        GameTime::from_millis(base.as_millis() + i64::from(global_offset_ms))
    } else {
        base
    }
}

pub(crate) fn playback_transition(
    previous_mode: DemoMode,
    current_mode: DemoMode,
    generation_changed: bool,
) -> PlaybackTransition {
    if generation_changed {
        return if matches!(current_mode, DemoMode::Playing) {
            PlaybackTransition::StartOrResume
        } else {
            PlaybackTransition::StopAndFreeze
        };
    }

    match (previous_mode, current_mode) {
        (
            DemoMode::Playing,
            DemoMode::Loading
            | DemoMode::Ready
            | DemoMode::Paused
            | DemoMode::Results
            | DemoMode::SongSelect
            | DemoMode::ImportedSelect
            | DemoMode::Settings
            | DemoMode::Calibration
            | DemoMode::Replay,
        ) => PlaybackTransition::StopAndFreeze,
        (DemoMode::Ready, DemoMode::Playing) => PlaybackTransition::StartOrResume,
        (
            DemoMode::Settings
            | DemoMode::SongSelect
            | DemoMode::ImportedSelect
            | DemoMode::Results,
            DemoMode::Calibration,
        ) => PlaybackTransition::StartOrResume,
        (
            DemoMode::Paused
            | DemoMode::Results
            | DemoMode::SongSelect
            | DemoMode::ImportedSelect
            | DemoMode::Settings
            | DemoMode::Calibration
            | DemoMode::Replay,
            DemoMode::Playing,
        ) => PlaybackTransition::StartOrResume,
        _ => PlaybackTransition::Keep,
    }
}
