use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::app::{grade_label, DemoApp};
use crate::ui::layout::{classify_screen, ShellDensity};
use crate::ui::widgets::chrome::{
    accent_style, block_card, bottom_shortcut_bar, label_style, muted_style, render_metric_card,
    top_status_bar, CoverArtWidget,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_pause_overlay(frame: &mut ratatui::Frame<'_>, theme: &ThemeTokens) {
    let area = centered_rect(frame.area(), 48, 8);
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(Span::styled("PAUSED", accent_style(theme))),
        Line::from(""),
        Line::from("Resume with P or Space"),
        Line::from("Back B   Settings S   Theme T   Restart R   Quit Q"),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(block_card("Session Hold", theme)),
        area,
    );
}

pub fn render_results(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let screen = classify_screen(area.width, area.height);
    let summary = app.score_summary();
    let rank = grade_label(summary.accuracy);
    let record = app.result_record();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);

    let active = app.active_song();
    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled("Results", label_style(theme)),
        Span::raw("  "),
        Span::styled(active.title(), muted_style(theme)),
        Span::raw("  "),
        Span::styled(rank, accent_style(theme)),
    ])];
    frame.render_widget(top_status_bar(top_bar, theme), root[0]);

    let body = Layout::default()
        .direction(if screen.stack_side_panels {
            Direction::Vertical
        } else {
            Direction::Horizontal
        })
        .constraints(if screen.stack_side_panels {
            vec![Constraint::Percentage(42), Constraint::Percentage(58)]
        } else {
            vec![Constraint::Percentage(36), Constraint::Percentage(64)]
        })
        .split(root[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if matches!(screen.density, ShellDensity::Compact) {
            vec![Constraint::Percentage(54), Constraint::Percentage(46)]
        } else {
            vec![Constraint::Percentage(62), Constraint::Percentage(38)]
        })
        .split(body[0]);

    let cover_block = block_card("Track Cover", theme);
    let cover_area = cover_block.inner(left[0]);
    frame.render_widget(cover_block, left[0]);
    frame.render_widget(
        CoverArtWidget {
            title: active.title(),
            subtitle: active.artist(),
            tag: active.mood(),
            theme,
            emphasis: parse_color(&theme.judgment_perfect),
        },
        cover_area,
    );

    let grade = Paragraph::new(vec![
        Line::from(Span::styled(
            rank,
            Style::default()
                .fg(parse_color(&theme.accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("{:>5.2}%", summary.accuracy * 100.0),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("stage rating", label_style(theme))),
    ])
    .alignment(Alignment::Center)
    .block(block_card("Grade", theme));
    frame.render_widget(grade, left[1]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if matches!(screen.density, ShellDensity::Compact) {
            vec![
                Constraint::Length(6),
                Constraint::Min(11),
                Constraint::Length(4),
            ]
        } else {
            vec![
                Constraint::Length(7),
                Constraint::Min(10),
                Constraint::Length(5),
            ]
        })
        .split(body[1]);

    let mut summary_lines = vec![Line::from(vec![
        Span::styled("Accuracy ", label_style(theme)),
        Span::styled(
            format!("{:>5.2}%", summary.accuracy * 100.0),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Max Combo ", label_style(theme)),
        Span::styled(
            format!("{}", summary.max_combo),
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    if let Some(record) = record {
        summary_lines.push(Line::from(vec![
            Span::styled("PB ", label_style(theme)),
            Span::styled(
                record.best_score.to_string(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Plays ", label_style(theme)),
            Span::styled(
                record.play_count.to_string(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    if !matches!(screen.density, ShellDensity::Compact) {
        summary_lines.push(Line::from(Span::styled(
            "Results should feel tied to the track, not detached from it.",
            muted_style(theme),
        )));
    }
    frame.render_widget(
        Paragraph::new(summary_lines).block(block_card("Run Summary", theme)),
        right[0],
    );

    let stats_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if matches!(screen.density, ShellDensity::Compact) {
            vec![
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(4),
            ]
        } else {
            vec![
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(5),
            ]
        })
        .split(right[1]);

    let first_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(stats_rows[0]);
    let second_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(stats_rows[1]);

    render_metric_card(
        frame,
        first_row[0],
        "Score",
        summary.score.to_string(),
        theme,
    );
    render_metric_card(
        frame,
        first_row[1],
        "Perfect",
        summary.judgments.perfect.to_string(),
        theme,
    );
    render_metric_card(
        frame,
        second_row[0],
        "Great",
        summary.judgments.great.to_string(),
        theme,
    );
    render_metric_card(
        frame,
        second_row[1],
        "Good / Miss",
        format!("{} / {}", summary.judgments.good, summary.judgments.miss),
        theme,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Track", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    active.chart_name(),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("   "),
                Span::styled("Difficulty", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{:02}", active.difficulty()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Last", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    record
                        .map(|entry| entry.last_score.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("   "),
                Span::styled("Best", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    record
                        .map(|entry| entry.best_score.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Artwork", accent_style(theme)),
                Span::raw("  "),
                Span::styled(
                    "Replay data captured. Stage framing is ready for image preview mode.",
                    muted_style(theme),
                ),
            ]),
        ])
        .block(block_card("Track Identity", theme)),
        stats_rows[2],
    );

    let footer = vec![Line::from(vec![
        Span::styled("Replay", label_style(theme)),
        Span::raw("  V   "),
        Span::styled("Back", label_style(theme)),
        Span::raw("  Enter/B   "),
        Span::styled("Settings", label_style(theme)),
        Span::raw("  S   "),
        Span::styled("Theme", label_style(theme)),
        Span::raw("  T   "),
        Span::styled("Quit", label_style(theme)),
        Span::raw("  Q"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

pub fn results_cover_image_rect(area: Rect) -> Option<Rect> {
    let screen = classify_screen(area.width, area.height);
    if screen.stack_side_panels {
        return None;
    }

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
        .split(root[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if matches!(screen.density, ShellDensity::Compact) {
            vec![Constraint::Percentage(54), Constraint::Percentage(46)]
        } else {
            vec![Constraint::Percentage(62), Constraint::Percentage(38)]
        })
        .split(body[0]);

    Some(Rect {
        x: left[0].x + 1,
        y: left[0].y + 1,
        width: left[0].width.saturating_sub(2),
        height: left[0].height.saturating_sub(2),
    })
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
