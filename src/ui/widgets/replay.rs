use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::app::DemoApp;
use crate::ui::widgets::chrome::{accent_style, block_card, label_style, muted_style};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_replay(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let panel = centered_rect(area, 92, 20);
    frame.render_widget(Clear, panel);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(panel);

    let replay = app.last_replay();
    let active = app.active_song();
    let replay_summary = app.replay_score_summary();
    let replay_judgment = app
        .replay_latest_judgment()
        .map(|judgment| format!("{judgment:?}"))
        .unwrap_or_else(|| "Waiting".to_string());

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("RUN REPLAY", accent_style(theme)),
            Span::raw("  "),
            Span::styled(
                active.title(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "This screen validates and inspects the recorded input stream for the current run.",
            muted_style(theme),
        )),
    ])
    .block(block_card("Replay", theme));
    frame.render_widget(header, root[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(root[1]);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Events", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                replay
                    .map(|r| r.events.len().to_string())
                    .unwrap_or_else(|| "0".to_string()),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Keymap", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.keymap().as_string(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Recorded", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                if replay.is_some() { "Ready" } else { "Empty" },
                accent_style(theme),
            ),
        ]),
        Line::from(vec![
            Span::styled("Preview", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                if app.replay_preview_playing() {
                    "Playing"
                } else {
                    "Paused"
                },
                if app.replay_preview_playing() {
                    accent_style(theme)
                } else {
                    muted_style(theme)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Clock", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{:>5}ms", app.replay_current_time_ms()),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Score", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                replay_summary.score.to_string(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Combo", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                replay_summary.combo.to_string(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Grade", label_style(theme)),
            Span::raw("  "),
            Span::styled(app.replay_current_grade(), accent_style(theme)),
        ]),
        Line::from(vec![
            Span::styled("Judgment", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                replay_judgment,
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
    ])
    .alignment(Alignment::Left)
    .block(block_card("Replay Summary", theme));
    frame.render_widget(summary, body[0]);

    let cursor = app.replay_cursor();
    let lines = replay
        .map(|input| {
            input
                .events
                .iter()
                .enumerate()
                .skip(cursor.unwrap_or(0).saturating_sub(4))
                .take(9)
                .map(|(index, event)| {
                    let active = cursor == Some(index);
                    let prefix = if active { ">" } else { " " };
                    Line::from(vec![
                        Span::styled(
                            prefix,
                            if active {
                                accent_style(theme)
                            } else {
                                muted_style(theme)
                            },
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:>5}ms", event.timestamp.as_millis()),
                            if active {
                                accent_style(theme)
                            } else {
                                label_style(theme)
                            },
                        ),
                        Span::raw("  "),
                        Span::styled(
                            event.key.clone(),
                            Style::default()
                                .fg(parse_color(&theme.foreground))
                                .add_modifier(if active {
                                    Modifier::BOLD
                                } else {
                                    Modifier::empty()
                                }),
                        ),
                        Span::raw("  "),
                        Span::styled(format!("{:?}", event.action), muted_style(theme)),
                    ])
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![Line::from(Span::styled(
                "No replay data captured for this run.",
                muted_style(theme),
            ))]
        });
    let events = Paragraph::new(lines)
        .block(block_card("Input Timeline", theme))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(events, body[1]);

    let footer = Paragraph::new("Space/P play  Up/Down or Left/Right browse  Enter/B close")
        .alignment(Alignment::Center)
        .block(block_card("", theme));
    frame.render_widget(footer, root[2]);
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2).max(1));
    let height = height.min(area.height.saturating_sub(2).max(1));

    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
