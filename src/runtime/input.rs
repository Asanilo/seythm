use crate::config::keymap::Keymap;
use crate::runtime::clock::GameTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Press,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    pub key: String,
    pub timestamp: GameTime,
    pub action: InputAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayInput {
    pub keymap: Keymap,
    pub events: Vec<InputEvent>,
}

impl InputEvent {
    pub fn new(key: impl Into<String>, timestamp_ms: i64, action: InputAction) -> Self {
        Self {
            key: key.into(),
            timestamp: GameTime::from_millis(timestamp_ms),
            action,
        }
    }
}

impl InputEvent {
    pub fn press(key: impl Into<String>, timestamp_ms: i64) -> Self {
        Self::new(key, timestamp_ms, InputAction::Press)
    }

    pub fn release(key: impl Into<String>, timestamp_ms: i64) -> Self {
        Self::new(key, timestamp_ms, InputAction::Release)
    }
}

impl ReplayInput {
    pub fn new(keymap: Keymap, events: Vec<InputEvent>) -> Self {
        Self { keymap, events }
    }
}

pub fn lane_for_key(keymap: &Keymap, key: &str) -> Option<u8> {
    keymap
        .keys()
        .iter()
        .position(|candidate| candidate == key)
        .and_then(|lane| u8::try_from(lane).ok())
}

pub fn key_for_lane(keymap: &Keymap, lane: u8) -> Option<&str> {
    keymap.keys().get(lane as usize).map(String::as_str)
}
