use anyhow::Context;

use crate::chart::{Chart, Note};
use crate::chart::scheduler::ChartScheduler;
use crate::config::Keymap;
use crate::gameplay::GameplayState;
use crate::runtime::{InputAction, InputEvent, ReplayInput};
use crate::runtime::input::{key_for_lane, lane_for_key};
use crate::runtime::GameTime;

use super::{
    chart_duration_ms, grade_label, live_accuracy, zero_score_summary, ChartLibrary, DemoApp,
    DemoMode, APPROACH_WINDOW_MS, LOADING_AUDIO_READY_MS, LOADING_CHART_READY_MS,
    LOADING_INPUT_READY_MS, LOADING_TOTAL_MS, READY_BUFFER_MS,
};

impl DemoApp {
    pub fn update(&mut self, time: GameTime) {
        if matches!(self.navigation.mode, DemoMode::Loading) {
            self.current_time = GameTime::from_millis(time.as_millis().clamp(0, LOADING_TOTAL_MS));
            if self.current_time.as_millis() >= LOADING_TOTAL_MS {
                self.enter_ready_buffer();
            }
            return;
        }

        if matches!(self.navigation.mode, DemoMode::Ready) {
            self.current_time = GameTime::from_millis(time.as_millis().clamp(0, READY_BUFFER_MS));
            if self.current_time.as_millis() >= READY_BUFFER_MS {
                self.begin_playback();
            }
            return;
        }

        if matches!(self.navigation.mode, DemoMode::Replay) {
            self.current_time = time;
            self.advance_replay_preview();
            return;
        }

        if !matches!(self.navigation.mode, DemoMode::Playing) {
            return;
        }

        if self.autoplay_state.enabled {
            self.advance_with_autoplay(time);
        } else {
            self.current_time = time;
            for event in self.gameplay.advance_to(time) {
                self.apply_hit_event(event);
            }
        }
        self.refresh_mode();
    }

    pub fn open_replay_view(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Results) && self.replay_state.latest_replay.is_some() {
            self.reset_replay_state();
            self.navigation.mode = DemoMode::Replay;
        }
    }

    pub fn close_replay_view(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Replay) {
            self.replay_state.cursor = None;
            self.replay_state.preview_playing = false;
            self.replay_state.gameplay = None;
            self.replay_state.input_cursor = 0;
            self.replay_state.latest_judgment = None;
            self.navigation.mode = DemoMode::Results;
        }
    }

    pub fn move_replay_cursor(&mut self, delta: i32) {
        if !matches!(self.navigation.mode, DemoMode::Replay) {
            return;
        }
        let Some(replay) = self.replay_state.latest_replay.as_ref() else {
            self.replay_state.cursor = None;
            return;
        };
        if replay.events.is_empty() {
            self.replay_state.cursor = None;
            return;
        }
        let current = self.replay_state.cursor.unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, replay.events.len() as i32 - 1) as usize;
        self.replay_state.cursor = Some(next);
    }

    pub fn toggle_replay_preview(&mut self) {
        if !matches!(self.navigation.mode, DemoMode::Replay)
            || self.replay_state.latest_replay.is_none()
        {
            return;
        }
        if !self.replay_state.preview_playing && self.replay_finished() {
            self.reset_replay_state();
        }
        self.replay_state.preview_playing = !self.replay_state.preview_playing;
    }

    pub fn return_to_browse_view(&mut self) {
        self.navigation.mode = self.navigation.browse_return_mode;
        self.active_lanes = [false; 6];
        self.latest_judgment = None;
        self.latest_result_record = None;
        self.replay_state.latest_replay = None;
        self.replay_state.cursor = None;
        self.replay_state.preview_playing = false;
        self.replay_state.gameplay = None;
        self.replay_state.input_cursor = 0;
        self.replay_state.latest_judgment = None;
        self.autoplay_state.event_cursor = 0;
        self.current_time = GameTime::from_millis(0);
        self.lane_feedback_until = [None; 6];
        self.judgment_feedback_until = None;
        self.combo_feedback_until = None;
        self.bump_session();
    }

    pub fn skip_loading_intro(&mut self) {
        if !matches!(self.navigation.mode, DemoMode::Loading | DemoMode::Ready) {
            return;
        }
        self.current_time = GameTime::from_millis(0);
        self.begin_playback();
    }

    pub fn set_autoplay_enabled(&mut self, enabled: bool) {
        self.autoplay_state.enabled = enabled;
    }

    pub fn loading_progress_ratio(&self) -> f32 {
        let total = if matches!(self.navigation.mode, DemoMode::Ready) {
            READY_BUFFER_MS
        } else {
            LOADING_TOTAL_MS
        };
        (self.current_time.as_millis().max(0) as f32 / total as f32).clamp(0.0, 1.0)
    }

    pub fn loading_stage_label(&self) -> &'static str {
        let elapsed = self.current_time.as_millis();
        if matches!(self.navigation.mode, DemoMode::Ready) {
            return "player ready";
        }
        if elapsed < LOADING_AUDIO_READY_MS {
            "loading audio"
        } else if elapsed < LOADING_CHART_READY_MS {
            "preparing chart"
        } else if elapsed < LOADING_INPUT_READY_MS {
            "arming input"
        } else if elapsed < LOADING_TOTAL_MS {
            "ready"
        } else {
            "live"
        }
    }

    pub fn loading_status_lines(&self) -> [&'static str; 3] {
        let elapsed = self.current_time.as_millis();
        if matches!(self.navigation.mode, DemoMode::Ready) {
            return ["audio ready", "chart ready", "start gate open"];
        }
        [
            if elapsed >= LOADING_AUDIO_READY_MS {
                "audio ready"
            } else {
                "loading audio"
            },
            if elapsed >= LOADING_CHART_READY_MS {
                "chart ready"
            } else {
                "preparing chart"
            },
            if elapsed >= LOADING_INPUT_READY_MS {
                "input armed"
            } else {
                "arming input"
            },
        ]
    }

    pub fn loading_countdown_text(&self) -> String {
        let total = if matches!(self.navigation.mode, DemoMode::Ready) {
            READY_BUFFER_MS
        } else {
            LOADING_TOTAL_MS
        };
        let remaining_ms = (total - self.current_time.as_millis()).max(0);
        if remaining_ms == 0 {
            "live".to_string()
        } else {
            format!("start in {:.1}s", remaining_ms as f32 / 1000.0)
        }
    }

    pub fn autoplay_enabled(&self) -> bool {
        self.autoplay_state.enabled
    }

    pub fn replay_score_summary(&self) -> crate::gameplay::ScoreSummary {
        self.replay_state.gameplay
            .as_ref()
            .map(|gameplay| gameplay.summary())
            .unwrap_or_else(zero_score_summary)
    }

    pub fn replay_current_grade(&self) -> &'static str {
        grade_label(live_accuracy(self.replay_score_summary()))
    }

    pub(super) fn refresh_mode(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Loading | DemoMode::Ready | DemoMode::Paused) {
            return;
        }

        if self.gameplay.all_notes_resolved() && self.visible_notes().is_empty() {
            let summary = self.score_summary();
            let song_id = self.active_song().id().to_string();
            let record = self
                .profile
                .record_run(&song_id, summary.score, summary.accuracy);
            self.latest_result_record = Some(record);
            self.replay_state.latest_replay = Some(ReplayInput::new(
                self.keymap().clone(),
                self.current_run_events.clone(),
            ));
            self.replay_state.cursor = None;
            self.replay_state.preview_playing = false;
            self.replay_state.gameplay = None;
            self.replay_state.input_cursor = 0;
            self.replay_state.latest_judgment = None;
            self.persist_profile();
            self.navigation.mode = DemoMode::Results;
            self.active_lanes = [false; 6];
        }
    }

    pub(super) fn reset_chart_state(
        &mut self,
        library: ChartLibrary,
        chart_index: usize,
    ) -> anyhow::Result<()> {
        let chart = self
            .song_for_library(library, chart_index)
            .context("selected chart index out of range")?
            .chart
            .clone();
        self.scheduler = ChartScheduler::new(chart.clone(), APPROACH_WINDOW_MS);
        self.gameplay = GameplayState::new(chart.clone());
        self.chart = chart;
        self.latest_judgment = None;
        self.latest_result_record = None;
        self.current_run_events.clear();
        self.replay_state.latest_replay = None;
        self.replay_state.cursor = None;
        self.replay_state.preview_playing = false;
        self.replay_state.gameplay = None;
        self.replay_state.input_cursor = 0;
        self.replay_state.latest_judgment = None;
        self.autoplay_state.events = if self.autoplay_state.enabled {
            build_autoplay_events(&self.chart, &self.settings.keymap)
        } else {
            Vec::new()
        };
        self.autoplay_state.event_cursor = 0;
        self.current_time = GameTime::from_millis(0);
        self.active_lanes = [false; 6];
        self.lane_feedback_until = [None; 6];
        self.judgment_feedback_until = None;
        self.combo_feedback_until = None;
        self.navigation.mode = DemoMode::Loading;
        self.bump_session();
        Ok(())
    }

    fn begin_playback(&mut self) {
        self.navigation.mode = DemoMode::Playing;
        self.current_time = GameTime::from_millis(0);
        self.active_lanes = [false; 6];
        self.bump_session();
    }

    fn enter_ready_buffer(&mut self) {
        self.navigation.mode = DemoMode::Ready;
        self.current_time = GameTime::from_millis(0);
        self.active_lanes = [false; 6];
    }

    fn advance_replay_preview(&mut self) {
        if !self.replay_state.preview_playing {
            return;
        }
        let Some(replay) = self.replay_state.latest_replay.clone() else {
            self.replay_state.preview_playing = false;
            return;
        };
        if replay.events.is_empty() {
            self.replay_state.preview_playing = false;
            self.replay_state.cursor = None;
            return;
        }

        if self.replay_state.gameplay.is_none() {
            self.reset_replay_state();
        }

        while self.replay_state.input_cursor < replay.events.len()
            && replay.events[self.replay_state.input_cursor]
                .timestamp
                .as_millis()
                <= self.current_time.as_millis()
        {
            let event = replay.events[self.replay_state.input_cursor].clone();
            self.advance_replay_gameplay_to(event.timestamp);

            if let Some(lane) = lane_for_key(&replay.keymap, &event.key) {
                match event.action {
                    InputAction::Press => self.apply_replay_press(lane, event.timestamp),
                    InputAction::Release => self.apply_replay_release(lane, event.timestamp),
                }
            }

            self.replay_state.cursor = Some(self.replay_state.input_cursor);
            self.replay_state.input_cursor += 1;
        }

        self.advance_replay_gameplay_to(self.current_time);

        let end_time = chart_duration_ms(&self.chart).saturating_add(240);
        if self.current_time.as_millis() >= end_time || self.replay_finished() {
            self.replay_state.preview_playing = false;
        }
    }

    fn replay_finished(&self) -> bool {
        self.replay_state.input_cursor
            >= self
                .replay_state
                .latest_replay
                .as_ref()
                .map(|replay| replay.events.len())
                .unwrap_or(0)
            && self
                .replay_state
                .gameplay
                .as_ref()
                .map(|gameplay| gameplay.all_notes_resolved())
                .unwrap_or(true)
    }

    fn reset_replay_state(&mut self) {
        self.current_time = GameTime::from_millis(0);
        self.replay_state.cursor = self
            .replay_state
            .latest_replay
            .as_ref()
            .and_then(|replay| (!replay.events.is_empty()).then_some(0));
        self.replay_state.gameplay = Some(GameplayState::new(self.chart.clone()));
        self.replay_state.input_cursor = 0;
        self.replay_state.latest_judgment = None;
    }

    fn advance_replay_gameplay_to(&mut self, time: GameTime) {
        let Some(gameplay) = self.replay_state.gameplay.as_mut() else {
            return;
        };
        for event in gameplay.advance_to(time) {
            self.replay_state.latest_judgment = Some(event.judgment);
        }
    }

    fn advance_with_autoplay(&mut self, target_time: GameTime) {
        while self.autoplay_state.event_cursor < self.autoplay_state.events.len()
            && self.autoplay_state.events[self.autoplay_state.event_cursor]
                .timestamp
                .as_millis()
                <= target_time.as_millis()
        {
            let event = self.autoplay_state.events[self.autoplay_state.event_cursor].clone();
            self.current_time = event.timestamp;
            for hit in self.gameplay.advance_to(event.timestamp) {
                self.apply_hit_event(hit);
            }
            self.apply_autoplay_event(event);
            self.autoplay_state.event_cursor += 1;
        }

        self.current_time = target_time;
        for event in self.gameplay.advance_to(target_time) {
            self.apply_hit_event(event);
        }
    }

    fn apply_autoplay_event(&mut self, event: InputEvent) {
        let Some(lane) = lane_for_key(&self.settings.keymap, &event.key) else {
            return;
        };

        self.current_run_events.push(event.clone());
        match event.action {
            InputAction::Press => {
                let _ = self.handle_lane_press(lane, event.timestamp);
            }
            InputAction::Release => {
                let _ = self.handle_lane_release(lane, event.timestamp);
            }
        }
    }

    fn apply_replay_press(&mut self, lane: u8, time: GameTime) {
        let Some(gameplay) = self.replay_state.gameplay.as_mut() else {
            return;
        };
        if let Some(event) = gameplay
            .press_lane(lane, time)
            .or_else(|| gameplay.release_lane(lane, time))
        {
            self.replay_state.latest_judgment = Some(event.judgment);
        }
    }

    fn apply_replay_release(&mut self, lane: u8, time: GameTime) {
        let Some(gameplay) = self.replay_state.gameplay.as_mut() else {
            return;
        };
        if let Some(event) = gameplay.release_lane(lane, time) {
            self.replay_state.latest_judgment = Some(event.judgment);
        }
    }
}

fn build_autoplay_events(chart: &Chart, keymap: &Keymap) -> Vec<InputEvent> {
    let mut events = Vec::new();

    for note in &chart.notes {
        let lane = note.lane();
        let Some(key) = key_for_lane(keymap, lane) else {
            continue;
        };
        match note {
            Note::Tap(note) => events.push(InputEvent::press(key, note.time_ms as i64)),
            Note::Hold(note) => {
                events.push(InputEvent::press(key, note.start_ms as i64));
                events.push(InputEvent::release(key, note.end_ms as i64));
            }
        }
    }

    events.sort_by(|left, right| {
        left.timestamp
            .as_millis()
            .cmp(&right.timestamp.as_millis())
            .then_with(|| match (left.action, right.action) {
                (InputAction::Press, InputAction::Release) => std::cmp::Ordering::Less,
                (InputAction::Release, InputAction::Press) => std::cmp::Ordering::Greater,
                _ => left.key.cmp(&right.key),
            })
    });
    events
}
