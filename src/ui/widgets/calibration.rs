use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::app::DemoApp;
use crate::ui::widgets::chrome::{accent_style, block_card, label_style, muted_style};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_calibration(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let panel = centered_rect(area, 84, 18);
    frame.render_widget(Clear, panel);

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Min(4),
        ])
        .split(panel);

    let active = app.selected_song();
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("TIMING CALIBRATION", accent_style(theme)),
            Span::raw("  "),
            Span::styled("align the stage clock before fine input tuning", label_style(theme)),
        ]),
        Line::from(vec![
            Span::styled(active.title(), Style::default().fg(parse_color(&theme.foreground)).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(active.artist(), muted_style(theme)),
            Span::raw("  "),
            Span::styled(format!("{} BPM", active.bpm()), label_style(theme)),
        ]),
        Line::from(Span::styled(
            "Use Left and Right to move the global sync offset, then confirm when the metronome feels locked to the chart.",
            muted_style(theme),
        )),
    ])
    .block(block_card("Calibration", theme));
    frame.render_widget(header, root[0]);

    let gauge = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{:+} ms", app.global_offset_ms()),
            Style::default()
                .fg(parse_color(&theme.accent))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            offset_bar(app.global_offset_ms()),
            Style::default().fg(parse_color(&theme.foreground)),
        )),
    ])
    .alignment(Alignment::Center)
    .block(block_card("Global Sync", theme));
    frame.render_widget(gauge, root[1]);

    let notes = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Current Input Bias", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{:+} ms", app.input_offset_ms()),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Global sync moves audio and stage together. Input bias stays separate for your personal hit timing.",
            muted_style(theme),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Adjust", label_style(theme)),
            Span::raw("  Left/Right   "),
            Span::styled("Confirm", label_style(theme)),
            Span::raw("  Enter/B"),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(block_card("Guide", theme))
    .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(notes, root[2]);
}

fn offset_bar(offset_ms: i32) -> String {
    let mut chars = vec!['·'; 25];
    chars[12] = '│';
    let index = (12 + (offset_ms / 10)).clamp(0, 24) as usize;
    chars[index] = '◆';
    chars.into_iter().collect()
}

fn centered_rect(area: ratatui::layout::Rect, width: u16, height: u16) -> ratatui::layout::Rect {
    let width = width.min(area.width.saturating_sub(2).max(1));
    let height = height.min(area.height.saturating_sub(2).max(1));

    ratatui::layout::Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
