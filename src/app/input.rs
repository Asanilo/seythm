use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::runtime::GameTime;

use super::{DemoApp, DemoMode, SettingsItem};

pub(crate) fn poll_input(
    timeout: Duration,
    app: &mut DemoApp,
    now: GameTime,
) -> anyhow::Result<bool> {
    if !event::poll(timeout).context("failed to poll terminal events")? {
        return Ok(false);
    }

    let first = event::read().context("failed to read terminal event")?;
    if process_terminal_event(app, now, first)? {
        return Ok(true);
    }

    while event::poll(Duration::from_millis(0)).context("failed to drain terminal events")? {
        let queued = event::read().context("failed to read queued terminal event")?;
        if process_terminal_event(app, now, queued)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn process_terminal_event(app: &mut DemoApp, now: GameTime, event: Event) -> anyhow::Result<bool> {
    match event {
        Event::Key(key) => process_key_event(app, now, key),
        _ => Ok(false),
    }
}

pub(crate) fn process_key_event(
    app: &mut DemoApp,
    now: GameTime,
    key: KeyEvent,
) -> anyhow::Result<bool> {
    if matches!(app.mode(), DemoMode::Search) {
        return process_search_key_event(app, key);
    }

    match key.kind {
        KeyEventKind::Press => match key.code {
            KeyCode::Esc => {
                Ok(true)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                Ok(true)
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if matches!(
                    app.mode(),
                    DemoMode::SongSelect
                        | DemoMode::ImportedSelect
                        | DemoMode::Paused
                        | DemoMode::Results
                ) {
                    app.open_settings();
                } else if let KeyCode::Char(c) = key.code {
                    app.handle_key_char(c, now);
                }
                Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if matches!(app.mode(), DemoMode::Settings) {
                    app.move_settings_selection(-1);
                } else if matches!(app.mode(), DemoMode::SongSelect) {
                    app.move_selection(-1);
                } else if matches!(app.mode(), DemoMode::ImportedSelect) {
                    app.move_imported_selection(-1);
                } else if matches!(app.mode(), DemoMode::Replay) {
                    app.move_replay_cursor(-1);
                } else if let KeyCode::Char(c) = key.code {
                    app.handle_key_char(c, now);
                }
                Ok(false)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if matches!(app.mode(), DemoMode::Settings) {
                    app.move_settings_selection(1);
                } else if matches!(app.mode(), DemoMode::SongSelect) {
                    app.move_selection(1);
                } else if matches!(app.mode(), DemoMode::ImportedSelect) {
                    app.move_imported_selection(1);
                } else if matches!(app.mode(), DemoMode::Replay) {
                    app.move_replay_cursor(1);
                } else if let KeyCode::Char(c) = key.code {
                    app.handle_key_char(c, now);
                }
                Ok(false)
            }
            KeyCode::Left | KeyCode::Right => {
                if matches!(app.mode(), DemoMode::Settings) {
                    let delta = if matches!(key.code, KeyCode::Left) {
                        -5
                    } else {
                        5
                    };
                    match app.settings_selection() {
                        SettingsItem::GlobalOffset => app.adjust_global_offset_ms(delta),
                        SettingsItem::InputOffset => app.adjust_input_offset_ms(delta),
                        SettingsItem::MusicVolume => app.adjust_music_volume(delta),
                        SettingsItem::HitSoundVolume => app.adjust_hit_sound_volume(delta),
                        _ => app.activate_settings_item(),
                    }
                } else if matches!(app.mode(), DemoMode::Calibration) {
                    let delta = if matches!(key.code, KeyCode::Left) {
                        -5
                    } else {
                        5
                    };
                    app.adjust_calibration(delta);
                } else if matches!(app.mode(), DemoMode::Replay) {
                    let delta = if matches!(key.code, KeyCode::Left) {
                        -1
                    } else {
                        1
                    };
                    app.move_replay_cursor(delta);
                }
                Ok(false)
            }
            KeyCode::Enter => {
                if matches!(app.mode(), DemoMode::Loading | DemoMode::Ready) {
                    app.skip_loading_intro();
                } else if matches!(app.mode(), DemoMode::Calibration) {
                    app.finish_calibration();
                } else if matches!(app.mode(), DemoMode::Replay) {
                    app.close_replay_view();
                } else if matches!(app.mode(), DemoMode::Settings) {
                    app.activate_settings_item();
                } else if matches!(app.mode(), DemoMode::SongSelect | DemoMode::ImportedSelect) {
                    app.start_selected_chart()?;
                } else if matches!(app.mode(), DemoMode::Results) {
                    app.return_to_browse_view();
                }
                Ok(false)
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                if matches!(app.mode(), DemoMode::Loading | DemoMode::Ready) {
                    app.return_to_browse_view();
                } else if matches!(app.mode(), DemoMode::Calibration) {
                    app.finish_calibration();
                } else if matches!(app.mode(), DemoMode::Replay) {
                    app.close_replay_view();
                } else if matches!(app.mode(), DemoMode::Paused) {
                    app.return_to_browse_view();
                } else if matches!(app.mode(), DemoMode::Settings) {
                    app.close_settings();
                } else if matches!(app.mode(), DemoMode::Results) {
                    app.return_to_browse_view();
                } else if matches!(app.mode(), DemoMode::ImportedSelect) {
                    app.return_to_song_select();
                }
                Ok(false)
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                if matches!(app.mode(), DemoMode::Results) {
                    app.open_replay_view();
                }
                Ok(false)
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if matches!(app.mode(), DemoMode::SongSelect | DemoMode::ImportedSelect) {
                    app.cycle_browse_sort();
                } else {
                    app.restart()?;
                }
                Ok(false)
            }
            KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Char(' ') => {
                if matches!(app.mode(), DemoMode::Loading | DemoMode::Ready) {
                    app.skip_loading_intro();
                } else if matches!(app.mode(), DemoMode::Replay) {
                    app.toggle_replay_preview();
                } else {
                    app.toggle_pause();
                }
                Ok(false)
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.toggle_theme();
                Ok(false)
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.open_imported_view();
                Ok(false)
            }
            KeyCode::Char('/') => {
                if matches!(app.mode(), DemoMode::SongSelect | DemoMode::ImportedSelect) {
                    app.open_search();
                }
                Ok(false)
            }
            KeyCode::Backspace => {
                Ok(false)
            }
            KeyCode::Char(c) => {
                app.handle_key_char(c, now);
                Ok(false)
            }
            _ => Ok(false),
        },
        KeyEventKind::Release => {
            if let KeyCode::Char(c) = key.code {
                app.handle_key_release_char(c, now);
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn process_search_key_event(app: &mut DemoApp, key: KeyEvent) -> anyhow::Result<bool> {
    match key.kind {
        KeyEventKind::Press => match key.code {
            KeyCode::Esc => {
                app.close_search();
                Ok(false)
            }
            KeyCode::Enter => {
                app.activate_search_selection()?;
                Ok(false)
            }
            KeyCode::Backspace => {
                app.pop_search_char();
                Ok(false)
            }
            KeyCode::Up => {
                app.move_search_selection(-1);
                Ok(false)
            }
            KeyCode::Down => {
                app.move_search_selection(1);
                Ok(false)
            }
            KeyCode::Char(c) => {
                app.push_search_char(c);
                Ok(false)
            }
            _ => Ok(false),
        },
        KeyEventKind::Release | KeyEventKind::Repeat => Ok(false),
    }
}
