use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::app::DemoApp;
use crate::ui::layout::{classify_screen, ShellDensity};
use crate::ui::widgets::chrome::{
    accent_style, bottom_shortcut_bar, label_style, layered_panel_style, muted_style,
    raised_panel_style, selected_row_style, separator_style, top_status_bar,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_search(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    let area = frame.area();
    let screen = classify_screen(area.width, area.height);
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(area);

    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled("Search", label_style(theme)),
        Span::raw("  "),
        Span::styled(
            format!("{} results", app.search_results_len()),
            muted_style(theme),
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
            vec![Constraint::Length(6), Constraint::Min(12)]
        } else {
            vec![Constraint::Length(34), Constraint::Min(20)]
        })
        .split(root[1]);

    let query = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Query", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                if app.search_query().is_empty() {
                    "type title / artist / chart".to_string()
                } else {
                    app.search_query().to_string()
                },
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Unified search scans bundled and imported charts together.",
            muted_style(theme),
        )),
    ])
    .block(
        Block::default()
            .title(Line::from(vec![Span::styled("Finder", accent_style(theme))]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(raised_panel_style(theme))
            .border_style(separator_style(theme)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(query, body[0]);

    let rows = app.search_result_rows();
    let result_height = body[1].height.saturating_sub(2);
    let item_height = if matches!(screen.density, ShellDensity::Compact) {
        2
    } else {
        3
    };
    let visible_count = visible_result_count(result_height, item_height);
    let visible_window = visible_result_range(
        rows.len(),
        app.search_selected_index(),
        visible_count,
    );
    let mut lines = Vec::new();
    for index in visible_window {
        let row = &rows[index];
        let selected = index == app.search_selected_index();
        let style = if selected {
            selected_row_style(theme)
        } else {
            Style::default().fg(parse_color(&theme.foreground))
        };
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(vec![
            Span::styled(if selected { ">" } else { " " }, style),
            Span::raw(" "),
            Span::styled(row.source, if selected { style } else { accent_style(theme) }),
            Span::raw("  "),
            Span::styled(row.title, style),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(row.artist, muted_style(theme)),
            Span::raw("  "),
            Span::styled(row.chart_name, muted_style(theme)),
            Span::raw("  "),
            Span::styled(format!("{} BPM", row.bpm), muted_style(theme)),
            Span::raw("  "),
            Span::styled(format!("Stage {:02}", row.difficulty), muted_style(theme)),
        ]));
        if !matches!(screen.density, ShellDensity::Compact) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(row.mood, muted_style(theme)),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No results yet. Try title, artist, chart name, or mood.",
            muted_style(theme),
        )));
    }

    let results = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Line::from(vec![Span::styled("Results", label_style(theme))]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(results, body[1]);

    let footer = vec![Line::from(vec![
        Span::styled("Type", label_style(theme)),
        Span::raw("  query   "),
        Span::styled("Navigate", label_style(theme)),
        Span::raw("  ↑/↓   "),
        Span::styled("Open", label_style(theme)),
        Span::raw("  Enter   "),
        Span::styled("Back", label_style(theme)),
        Span::raw("  Esc   "),
        Span::styled("Delete", label_style(theme)),
        Span::raw("  Backspace"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

fn visible_result_count(result_height: u16, item_height: u16) -> usize {
    usize::from((result_height / item_height.max(1)).max(1))
}

fn visible_result_range(
    total: usize,
    selected_index: usize,
    visible_count: usize,
) -> std::ops::Range<usize> {
    if total <= visible_count {
        return 0..total;
    }

    let half = visible_count / 2;
    let mut start = selected_index.saturating_sub(half);
    let max_start = total.saturating_sub(visible_count);
    if start > max_start {
        start = max_start;
    }
    start..(start + visible_count)
}

#[cfg(test)]
mod tests {
    use super::{visible_result_count, visible_result_range};

    #[test]
    fn search_visible_range_tracks_selected_item() {
        assert_eq!(visible_result_range(20, 0, 5), 0..5);
        assert_eq!(visible_result_range(20, 10, 5), 8..13);
        assert_eq!(visible_result_range(20, 19, 5), 15..20);
    }

    #[test]
    fn search_visible_count_uses_available_result_height() {
        assert_eq!(visible_result_count(0, 3), 1);
        assert_eq!(visible_result_count(9, 3), 3);
        assert_eq!(visible_result_count(8, 2), 4);
    }
}
