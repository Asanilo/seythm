use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::DemoApp;
use crate::ui::widgets::chrome::{
    accent_style, block_card, bottom_shortcut_bar, compact_row, label_style, muted_style,
    top_status_bar, CoverArtWidget,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_loading(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);

    let song = app.active_song();
    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled("Loading", label_style(theme)),
        Span::raw("  "),
        Span::styled(song.title(), muted_style(theme)),
        Span::raw("  "),
        Span::styled(app.loading_stage_label(), accent_style(theme)),
    ])];
    frame.render_widget(top_status_bar(top_bar, theme), root[0]);

    let shell = loading_shell_rect(root[1]);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(34)])
        .split(shell);

    let cover_block = block_card("Ready Room", theme);
    let cover_area = cover_block.inner(body[0]);
    frame.render_widget(cover_block, body[0]);
    frame.render_widget(
        CoverArtWidget {
            title: song.title(),
            subtitle: song.artist(),
            tag: song.mood(),
            theme,
            emphasis: parse_color(&theme.accent),
        },
        cover_area,
    );

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(6),
        ])
        .split(body[1]);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                song.title(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(song.chart_name(), label_style(theme)),
        ]),
        Line::from(vec![
            Span::styled(song.artist(), muted_style(theme)),
            Span::raw("  "),
            Span::styled(format!("{}K", app.keymap().keys().len()), label_style(theme)),
            Span::raw("  "),
            Span::styled(app.loading_countdown_text(), accent_style(theme)),
            if app.autoplay_enabled() {
                Span::styled("  AUTOPLAY", accent_style(theme))
            } else {
                Span::raw("")
            },
        ]),
    ])
    .block(block_card("Track Load", theme));
    frame.render_widget(header, right[0]);

    let info = Paragraph::new(vec![
        compact_row("Artist", song.artist(), theme, false),
        compact_row("Difficulty", song.chart_name(), theme, false),
        compact_row("BPM", song.bpm().to_string(), theme, false),
        compact_row("Keys", app.keymap().as_string(), theme, false),
        compact_row("Library", song.mood(), theme, false),
    ])
    .block(block_card("Song Card", theme));
    frame.render_widget(info, right[1]);

    let gauge = Gauge::default()
        .block(block_card("System", theme))
        .gauge_style(Style::default().fg(parse_color(&theme.accent)))
        .label(app.loading_countdown_text())
        .ratio(app.loading_progress_ratio() as f64);
    frame.render_widget(gauge, right[2]);

    let stages = app.loading_status_lines();
    let stage_lines = vec![
        compact_row("Audio", stages[0], theme, false),
        compact_row("Chart", stages[1], theme, false),
        compact_row("Input", stages[2], theme, false),
        Line::from(""),
        Line::from(Span::styled(
            "Enter / Space to start immediately. B to return.",
            muted_style(theme).add_modifier(Modifier::ITALIC),
        )),
    ];
    frame.render_widget(
        Paragraph::new(stage_lines)
            .alignment(Alignment::Left)
            .block(block_card("Launch Prep", theme)),
        right[3],
    );

    let footer = vec![Line::from(vec![
        Span::styled("Ready", label_style(theme)),
        Span::raw("  "),
        Span::styled(app.loading_countdown_text(), accent_style(theme)),
        Span::raw("   "),
        Span::styled("Start", label_style(theme)),
        Span::raw("  Enter/Space   "),
        Span::styled("Back", label_style(theme)),
        Span::raw("  B"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

pub fn loading_cover_image_rect(area: Rect) -> Option<Rect> {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);
    let shell = loading_shell_rect(root[1]);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(34)])
        .split(shell);
    Some(Rect {
        x: body[0].x + 1,
        y: body[0].y + 1,
        width: body[0].width.saturating_sub(2),
        height: body[0].height.saturating_sub(2),
    })
}

fn loading_shell_rect(area: Rect) -> Rect {
    centered_rect(
        area,
        94.min(area.width.saturating_sub(2).max(1)),
        20.min(area.height.saturating_sub(1).max(1)),
    )
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
