use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{DemoApp, DemoMode};
use crate::chart::NoteKind;
use crate::runtime::input::key_for_lane;
use crate::ui::layout::{classify_screen, PlayfieldLayout, ShellDensity};
use crate::ui::theme::ThemeTokens;
use crate::ui::widgets::chrome::{
    accent_style, blend, block_card, bottom_shortcut_bar, label_style, muted_style,
    render_metric_card, top_status_bar,
};
use crate::ui::widgets::{format_judgment_label, lane_fill_glyph, parse_color};

const LANE_COUNT: usize = 6;
const LANE_WIDTH: u16 = 7;
const LANE_GAP: u16 = 1;
const HOLD_LENGTH_DIVISOR: f32 = 180.0;

pub fn render_gameplay(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let screen = classify_screen(area.width, area.height);
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if screen.stack_side_panels {
            vec![
                Constraint::Length(3),
                Constraint::Min(18),
                Constraint::Length(3),
            ]
        } else {
            vec![
                Constraint::Length(3),
                Constraint::Min(18),
                Constraint::Length(3),
            ]
        })
        .margin(1)
        .split(area);

    let summary = app.score_summary();
    let latest = app.latest_judgment();
    let progress = app.playback_progress_ratio();
    let combo_pulse = app.combo_feedback_strength();
    let judge_pulse = app.judgment_feedback_strength();
    let active = app.active_song();
    let mode_label = match app.mode() {
        DemoMode::Loading => "READY",
        DemoMode::Ready => "READY",
        DemoMode::Playing => "LIVE",
        DemoMode::Paused => "PAUSED",
        DemoMode::SongSelect => "SELECT",
        DemoMode::ImportedSelect => "SELECT",
        DemoMode::Search => "SEARCH",
        DemoMode::Settings => "SETTINGS",
        DemoMode::Calibration => "CALIBRATE",
        DemoMode::Replay => "REPLAY",
        DemoMode::Results => "RESULTS",
    };

    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled(mode_label, label_style(theme)),
        Span::raw("  "),
        Span::styled(active.title(), muted_style(theme)),
        Span::raw("  "),
        Span::styled(app.current_grade(), accent_style(theme)),
        Span::raw("  "),
        Span::styled(
            format!("{:>5.2}%", summary.accuracy * 100.0),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    frame.render_widget(top_status_bar(top_bar, theme), root[0]);

    let body = Layout::default()
        .direction(if screen.stack_side_panels {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if screen.stack_side_panels {
            vec![
                Constraint::Length(10),
                Constraint::Min(18),
                Constraint::Length(8),
            ]
        } else {
            vec![
                Constraint::Length(18),
                Constraint::Min(48),
                Constraint::Length(26),
            ]
        })
        .split(root[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if screen.stack_side_panels {
            vec![
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ]
        } else {
            vec![
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Min(8),
            ]
        })
        .split(body[0]);
    render_metric_card(frame, left[0], "Score", summary.score.to_string(), theme);
    render_metric_card(frame, left[1], "Combo", summary.combo.to_string(), theme);
    if screen.stack_side_panels {
        render_metric_card(
            frame,
            left[2],
            "Judge",
            latest.map(format_judgment_label).unwrap_or("READY"),
            theme,
        );
    } else {
        render_metric_card(
            frame,
            left[2],
            "Judge",
            latest.map(format_judgment_label).unwrap_or("READY"),
            theme,
        );
        let live = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(mode_label, accent_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{:>5.2}%", summary.accuracy * 100.0),
                    Style::default()
                        .fg(parse_color(&theme.foreground))
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("P/G/Gd/M", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!(
                        "{}/{}/{}/{}",
                        summary.judgments.perfect,
                        summary.judgments.great,
                        summary.judgments.good,
                        summary.judgments.miss
                    ),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "lane-first readability, atmosphere around impact.",
                muted_style(theme),
            )),
        ])
        .block(block_card("Session", theme))
        .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(live, left[3]);
    }

    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if matches!(screen.density, ShellDensity::Compact) {
            vec![
                Constraint::Length(3),
                Constraint::Min(12),
                Constraint::Length(3),
            ]
        } else {
            vec![
                Constraint::Length(4),
                Constraint::Min(16),
                Constraint::Length(3),
            ]
        })
        .split(body[1]);

    let key_labels = (0..LANE_COUNT)
        .filter_map(|lane| key_for_lane(app.keymap(), lane as u8))
        .collect::<Vec<_>>()
        .join(" ");
    let combo_style = Style::default()
        .fg(blend(
            parse_color(&theme.foreground),
            parse_color(&theme.accent),
            0.12 + combo_pulse * 0.72,
        ))
        .add_modifier(Modifier::BOLD);
    let judge_style = Style::default()
        .fg(blend(
            parse_color(&theme.foreground),
            parse_color(&theme.judgment_perfect),
            0.08 + judge_pulse * 0.82,
        ))
        .add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    active.title(),
                    Style::default()
                        .fg(parse_color(&theme.foreground))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(active.artist(), muted_style(theme)),
                Span::raw("  "),
                Span::styled(app.current_grade(), accent_style(theme)),
            ]),
            Line::from(vec![
                Span::styled(active.mood(), accent_style(theme)),
                Span::raw("  "),
                Span::styled(format!("{} BPM", active.bpm()), label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("Stage {:02}", active.difficulty()),
                    label_style(theme),
                ),
                Span::raw("   "),
                Span::styled(progress_bar(progress, 18), muted_style(theme)),
            ]),
            Line::from(vec![
                Span::styled("COMBO ", label_style(theme)),
                Span::styled(format!("{:>4}", summary.combo), combo_style),
                Span::raw("   "),
                Span::styled("JUDGE ", label_style(theme)),
                Span::styled(
                    latest.map(format_judgment_label).unwrap_or("READY"),
                    judge_style,
                ),
            ]),
        ])
        .block(block_card("Now Playing", theme)),
        center[0],
    );

    let playfield_block = block_card("Highway", theme);
    let inner = playfield_block.inner(center[1]);
    frame.render_widget(playfield_block, center[1]);
    frame.render_widget(
        GameplayWidget {
            app,
            theme,
            layout: PlayfieldLayout::new(LANE_COUNT as u16, LANE_WIDTH, LANE_GAP),
        },
        inner,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Chart", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    active.chart_name(),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Status", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    if matches!(app.mode(), DemoMode::Ready) {
                        app.loading_countdown_text()
                    } else {
                        "judgment live".to_string()
                    },
                    if matches!(app.mode(), DemoMode::Ready) {
                        accent_style(theme)
                    } else {
                        Style::default().fg(parse_color(&theme.foreground))
                    },
                ),
            ]),
        ])
        .block(block_card("Timing Gate", theme)),
        center[2],
    );

    let side_area = body[2];
    let right = Layout::default()
        .direction(if screen.stack_side_panels {
            Direction::Horizontal
        } else {
            Direction::Vertical
        })
        .constraints(if screen.stack_side_panels {
            vec![
                Constraint::Percentage(42),
                Constraint::Percentage(28),
                Constraint::Percentage(30),
            ]
        } else {
            vec![
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(8),
            ]
        })
        .split(side_area);

    let spotlight_lines = if matches!(screen.density, ShellDensity::Compact) {
        vec![
            Line::from(Span::styled("VISUAL PREVIEW", accent_style(theme))),
            Line::from(Span::styled(
                "Atmosphere stays around the highway.",
                muted_style(theme),
            )),
        ]
    } else {
        vec![
            Line::from(Span::styled("VISUAL PREVIEW", accent_style(theme))),
            Line::from(""),
            Line::from(Span::styled(
                "Gameplay stays restrained. Image-backed atmosphere belongs around the highway, not on top of it.",
                muted_style(theme),
            )),
        ]
    };
    let spotlight = Paragraph::new(spotlight_lines)
        .block(block_card("Spotlight", theme))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(spotlight, right[0]);

    let cadence = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Chart", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                active.chart_name(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Mood", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                active.mood(),
                Style::default().fg(parse_color(&theme.accent)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Theme", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.theme_name(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
    ])
    .block(block_card("Track Identity", theme));
    frame.render_widget(cadence, right[1]);

    let rail = blend(
        parse_color(&theme.lane_fill),
        parse_color(&theme.background),
        0.42,
    );
    let rail_glow = blend(
        parse_color(&theme.accent),
        parse_color(&theme.background),
        0.30,
    );
    let lane_limit = if matches!(screen.density, ShellDensity::Compact) {
        4
    } else {
        6
    };
    let mut lines = Vec::new();
    for idx in 0..lane_limit {
        lines.push(Line::from(Span::styled(
            format!(
                "Lane {}  {}",
                idx + 1,
                if app.active_lanes()[idx] {
                    "ACTIVE"
                } else {
                    "READY"
                }
            ),
            if app.active_lanes()[idx] {
                Style::default().fg(rail_glow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rail)
            },
        )));
    }
    if matches!(screen.density, ShellDensity::Compact) {
        lines.push(Line::from(Span::styled("…", muted_style(theme))));
    }
    let lane_state = Paragraph::new(lines).block(block_card("Lane State", theme));
    frame.render_widget(lane_state, right[2]);

    let footer = vec![Line::from(vec![
        Span::styled("Keys", label_style(theme)),
        Span::raw(format!("  {}   ", key_labels)),
        Span::styled("Pause", label_style(theme)),
        Span::raw("  P/Space   "),
        Span::styled("Back", label_style(theme)),
        Span::raw("  B   "),
        Span::styled("Restart", label_style(theme)),
        Span::raw("  R   "),
        Span::styled("Theme", label_style(theme)),
        Span::raw("  T   "),
        Span::styled("Quit", label_style(theme)),
        Span::raw("  Q"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

fn progress_bar(progress: f32, slots: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = (clamped * slots as f32).round() as usize;
    let mut bar = String::with_capacity(slots);
    for idx in 0..slots {
        bar.push(if idx < filled { '█' } else { '·' });
    }
    bar
}

struct GameplayWidget<'a> {
    app: &'a DemoApp,
    theme: &'a ThemeTokens,
    layout: PlayfieldLayout,
}

impl Widget for GameplayWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 8 || area.width < self.layout.playfield_width() {
            return;
        }

        let origin_x = area.x + self.layout.inner_origin(area.width);

        let lane_border = Style::default().fg(blend(
            parse_color(&self.theme.lane_border),
            parse_color(&self.theme.foreground),
            0.25,
        ));
        let lane_fill = Style::default().fg(blend(
            parse_color(&self.theme.lane_fill),
            parse_color(&self.theme.background),
            0.18,
        ));
        let note_style = Style::default()
            .fg(parse_color(&self.theme.accent))
            .add_modifier(Modifier::BOLD);
        let hold_style = Style::default().fg(parse_color(&self.theme.judgment_great));
        let hold_active_style = Style::default()
            .fg(parse_color(&self.theme.judgment_perfect))
            .add_modifier(Modifier::BOLD);
        let judge_feedback = self.app.judgment_feedback_strength();
        let judge_style = Style::default().fg(blend(
            parse_color(&self.theme.judgment_perfect),
            parse_color(&self.theme.foreground),
            0.35 + judge_feedback * 0.45,
        ));
        let ambient_style = Style::default().fg(blend(
            parse_color(&self.theme.background),
            parse_color(&self.theme.accent),
            0.10 + judge_feedback * 0.10,
        ));

        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let glyph = if (y - area.y) % 4 == 0 { "░" } else { " " };
                buf.set_string(x, y, glyph, ambient_style);
            }
        }

        let judge_y = area.y + area.height.saturating_sub(3);
        let top_y = area.y + 1;
        let usable_height = judge_y.saturating_sub(top_y);

        for lane in 0..LANE_COUNT {
            let lane_x = origin_x + self.layout.lane_left(lane as u16);
            let lane_feedback = self.app.lane_feedback_strength(lane);
            let lane_style = if self.app.active_lanes()[lane] || lane_feedback > 0.0 {
                Style::default()
                    .fg(blend(
                        parse_color(&self.theme.lane_active),
                        parse_color(&self.theme.judgment_perfect),
                        lane_feedback * 0.65,
                    ))
                    .add_modifier(Modifier::BOLD)
            } else {
                lane_border
            };
            let lane_fill_style = if lane_feedback > 0.0 {
                Style::default().fg(blend(
                    parse_color(&self.theme.lane_fill),
                    parse_color(&self.theme.judgment_perfect),
                    0.25 + lane_feedback * 0.55,
                ))
            } else {
                lane_fill
            };

            for y in area.y..area.bottom() {
                buf.set_string(lane_x, y, "│", lane_style);
                buf.set_string(lane_x + LANE_WIDTH - 1, y, "│", lane_style);
            }

            let fill = if lane_feedback > 0.0 {
                "█".to_string()
            } else {
                lane_fill_glyph(self.app.active_lanes()[lane]).to_string()
            };
            for y in top_y..judge_y {
                for x in (lane_x + 1)..(lane_x + LANE_WIDTH - 1) {
                    buf.set_string(x, y, &fill, lane_fill_style);
                }
            }

            if lane_feedback > 0.0 {
                let burst_style = Style::default()
                    .fg(blend(
                        parse_color(&self.theme.judgment_perfect),
                        parse_color(&self.theme.accent),
                        0.45 + lane_feedback * 0.4,
                    ))
                    .add_modifier(Modifier::BOLD);
                let burst_y = judge_y.saturating_sub(1);
                buf.set_string(lane_x + 1, burst_y, "▅▇▅", burst_style);
            }
        }

        for x in origin_x..(origin_x + self.layout.playfield_width()).min(area.right()) {
            let symbol = if x % 2 == 0 { "═" } else { "─" };
            buf.set_string(x, judge_y, symbol, judge_style);
        }

        for note in self.app.visible_notes() {
            let lane_x = origin_x + self.layout.lane_left(note.lane as u16);
            let note_x = lane_x + 2;
            let note_y = top_y
                + ((1.0 - note.approach_position).clamp(0.0, 1.0) * usable_height as f32) as u16;

            match note.kind {
                NoteKind::Tap => {
                    if note_y < judge_y {
                        buf.set_string(note_x, note_y, "▆█▆", note_style);
                    }
                }
                NoteKind::Hold => {
                    let (head_y, tail_y) =
                        hold_segment_bounds(note_y, judge_y, &note, usable_height);
                    let style = if note.is_active {
                        hold_active_style
                    } else {
                        hold_style
                    };

                    if head_y < judge_y {
                        buf.set_string(note_x, head_y, "╭█╮", style);
                    }

                    for y in (head_y + 1)..tail_y {
                        if y < judge_y {
                            buf.set_string(note_x, y, "│█│", style);
                        }
                    }

                    if tail_y < judge_y {
                        buf.set_string(note_x, tail_y, "╰█╯", style);
                    }
                }
            }
        }
    }
}

fn hold_segment_bounds(
    note_y: u16,
    judge_y: u16,
    note: &crate::chart::scheduler::LaneNoteProjection,
    usable_height: u16,
) -> (u16, u16) {
    if note.is_active {
        return (
            note_y.min(judge_y.saturating_sub(1)),
            judge_y.saturating_sub(1),
        );
    }

    let duration_ms = note
        .end_timestamp_ms
        .unwrap_or(note.timestamp_ms)
        .saturating_sub(note.timestamp_ms)
        .max(180);
    let visual_length =
        ((duration_ms as f32 / HOLD_LENGTH_DIVISOR).round() as u16).clamp(2, usable_height.max(2));
    let tail_y = note_y
        .saturating_add(visual_length)
        .min(judge_y.saturating_sub(1));

    (note_y.min(tail_y), tail_y)
}
