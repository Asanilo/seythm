use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph};

use crate::ui::widgets::chrome::{
    accent_style, block_card, label_style, muted_style, raised_panel_style, separator_style,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_startup_splash(
    frame: &mut ratatui::Frame<'_>,
    theme: &ThemeTokens,
    brand_name: &str,
    brand_tagline: &str,
    ascii_logo: &str,
    startup_hint: &str,
    progress_ratio: f32,
    autoplay: bool,
) {
    let area = frame.area();
    let root = centered_rect(area, 72, 22);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(root);

    let shell = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(raised_panel_style(theme))
        .border_style(separator_style(theme));
    frame.render_widget(shell, root);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("BOOT", accent_style(theme)),
            Span::raw("  "),
            Span::styled("interactive shell rhythm", muted_style(theme)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            brand_name,
            Style::default()
                .fg(parse_color(&theme.foreground))
                .add_modifier(Modifier::BOLD),
        )),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let logo = Paragraph::new(
        ascii_logo
            .lines()
            .map(|line| Line::from(line.to_string()))
            .collect::<Vec<_>>(),
    )
    .alignment(Alignment::Center)
    .block(block_card("Brand Frame", theme));
    frame.render_widget(logo, layout[1]);

    let gauge = Gauge::default()
        .block(block_card("Startup", theme))
        .gauge_style(Style::default().fg(parse_color(&theme.accent)))
        .ratio(progress_ratio.clamp(0.0, 1.0) as f64)
        .label("warming up shell");
    frame.render_widget(gauge, layout[2]);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Tagline", label_style(theme)),
            Span::raw("  "),
            Span::styled(brand_tagline, Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(vec![
            Span::styled("Theme", label_style(theme)),
            Span::raw("  "),
            Span::styled(theme.name.as_str(), Style::default().fg(parse_color(&theme.foreground))),
        ]),
    ];
    if autoplay {
        lines.push(Line::from(vec![
            Span::styled("Mode", label_style(theme)),
            Span::raw("  "),
            Span::styled("autoplay demo armed", accent_style(theme)),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("Hint", label_style(theme)),
        Span::raw("  "),
        Span::styled(startup_hint, muted_style(theme)),
    ]));
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(block_card("Launch Card", theme)),
        layout[3],
    );
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
