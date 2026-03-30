use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::app::DemoApp;
use crate::ui::layout::{browse_columns, classify_screen, ShellDensity};
use crate::ui::widgets::chrome::{
    accent_style, bottom_shortcut_bar, compact_row, label_style, layered_panel_style, muted_style,
    pill, raised_panel_style, selected_row_style, separator_style, top_status_bar, CoverArtWidget,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_song_select(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
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

    let selected = app.selected_song();
    let selected_record = app.song_profile_record(selected.id());
    let status_line = selection_status_line(
        selected_record.map(|record| record.best_score),
        selected_record.map(|record| record.play_count).unwrap_or(0),
    );
    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled("Song Select", label_style(theme)),
        Span::raw("  "),
        Span::styled(
            format!(
                "{}/{}",
                app.selected_chart_index() + 1,
                app.song_choices().len()
            ),
            muted_style(theme),
        ),
        Span::raw("  "),
        Span::styled(app.theme_name(), accent_style(theme)),
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
                Constraint::Length(if matches!(screen.density, ShellDensity::Compact) {
                    5
                } else {
                    6
                }),
                Constraint::Min(if matches!(screen.density, ShellDensity::Compact) {
                    6
                } else {
                    8
                }),
                Constraint::Length(if matches!(screen.density, ShellDensity::Compact) {
                    5
                } else {
                    6
                }),
            ]
        } else {
            let columns = browse_columns(root[1].width, screen);
            vec![
                Constraint::Length(columns.left),
                Constraint::Length(columns.center),
                Constraint::Length(columns.right),
            ]
        })
        .split(root[1]);

    if screen.stack_side_panels {
        let navigation_menu = Paragraph::new(browse_navigation_rows(app, theme))
            .block(
                Block::default()
                    .title(Line::from(vec![
                        Span::styled("Browse", accent_style(theme)),
                        Span::raw("  "),
                        Span::styled("primary library", muted_style(theme)),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(raised_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(navigation_menu, body[0]);
    } else {
        let navigation = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(body[0]);
        let navigation_header = Paragraph::new(vec![Line::from(vec![
            Span::styled("Browse", accent_style(theme)),
            Span::raw("  "),
            Span::styled("primary library", muted_style(theme)),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        );
        frame.render_widget(navigation_header, navigation[0]);
        let navigation_menu = Paragraph::new(browse_navigation_rows(app, theme))
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled("Menu", label_style(theme))]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(raised_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(navigation_menu, navigation[1]);
    }

    if screen.stack_side_panels {
        let featured = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                selected.title(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(selected.artist(), muted_style(theme)),
                Span::raw("  "),
                Span::styled(selected.chart_name(), muted_style(theme)),
            ]),
            Line::from(vec![
                pill("6 Keys", theme),
                Span::raw(" "),
                pill(selected.mood(), theme),
            ]),
            Line::from(vec![
                Span::styled(
                    status_line,
                    Style::default()
                        .fg(parse_color(&theme.foreground))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Acc", label_style(theme)),
                Span::raw(" "),
                Span::styled(
                    selected_record
                        .map(|record| format!("{:>5.2}%", record.best_accuracy * 100.0))
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Chart", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{} BPM", selected.bpm()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("Stage {:02}", selected.difficulty()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Shell", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    shell_support_line(selected.artwork_path().is_some()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled("Theme", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    app.theme_name(),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
        ])
        .block(
            Block::default()
                .title(Line::from(vec![Span::styled(
                    "Featured",
                    accent_style(theme),
                )]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(raised_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(featured, body[1]);
    } else {
        let center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(20), Constraint::Min(7)])
            .split(body[1]);

        let hero = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(36), Constraint::Min(30)])
            .split(center[0]);

        let cover_block = Block::default()
            .title(Line::from(vec![
                Span::styled("Stage", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    shell_support_line(selected.artwork_path().is_some()),
                    muted_style(theme),
                ),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(raised_panel_style(theme))
            .border_style(separator_style(theme));
        let cover_area = cover_block.inner(hero[0]);
        frame.render_widget(cover_block, hero[0]);
        frame.render_widget(
            CoverArtWidget {
                title: selected.title(),
                subtitle: selected.artist(),
                tag: selected.mood(),
                theme,
                emphasis: parse_color(&theme.accent),
            },
            cover_area,
        );

        let hero_copy = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "Now Playing",
                label_style(theme).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                selected.title(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(selected.artist(), muted_style(theme)),
                Span::raw("  "),
                Span::styled(selected.chart_name(), muted_style(theme)),
            ]),
            Line::from(""),
            Line::from(vec![
                pill("6 Keys", theme),
                Span::raw(" "),
                pill(selected.mood(), theme),
                Span::raw(" "),
                pill(selected.chart_name(), theme),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    status_line,
                    Style::default()
                        .fg(parse_color(&theme.foreground))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Acc", label_style(theme)),
                Span::raw(" "),
                Span::styled(
                    selected_record
                        .map(|record| format!("{:>5.2}%", record.best_accuracy * 100.0))
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
        ])
        .block(
            Block::default()
                .title(Line::from(vec![Span::styled(
                    "Featured",
                    accent_style(theme),
                )]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(raised_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(hero_copy, hero[1]);
        let support = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Chart", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{} BPM", selected.bpm()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("Stage {:02}", selected.difficulty()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled(
                    selected.mood(),
                    Style::default().fg(parse_color(&theme.accent)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Last", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    selected_record
                        .map(|record| record.last_score.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled("Theme", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    app.theme_name(),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Shell", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    shell_support_line(selected.artwork_path().is_some()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled("Mode", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    "6 Keys",
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Library", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    "Bundled",
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
                Span::raw("  "),
                Span::styled("Imported", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{} tracks", app.imported_song_choices().len()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
        ])
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled("Run", label_style(theme)),
                    Span::raw("  "),
                    Span::styled("live context", muted_style(theme)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
        let center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(18), Constraint::Min(8)])
            .split(body[1]);
        frame.render_widget(support, center[1]);
    }

    let rail = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(6)])
        .split(body[2]);
    let song_count = app.song_choices().len();
    let visible_count_hint =
        visible_list_count(rail[1].height, if screen.compress_lists { 2 } else { 3 });
    let visible_window = visible_range(song_count, app.selected_chart_index(), visible_count_hint);

    if screen.stack_side_panels {
        let mut catalog_lines = Vec::new();
        for index in visible_window.clone() {
            let song = &app.song_choices()[index];
            let selected_item = index == app.selected_chart_index();
            if !catalog_lines.is_empty() {
                catalog_lines.push(Line::from(""));
            }
            catalog_lines.extend(catalog_row_lines(
                song,
                theme,
                selected_item,
                screen.compress_lists,
            ));
        }

        let catalog = Paragraph::new(catalog_lines)
            .block(
                Block::default()
                    .title(Line::from(vec![
                        Span::styled("Catalog", accent_style(theme)),
                        Span::raw("  "),
                        Span::styled(
                            format!("{} visible / {} total", visible_window.len(), song_count),
                            muted_style(theme),
                        ),
                        Span::raw("  "),
                        Span::styled(selected.chart_name(), muted_style(theme)),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(layered_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(catalog, body[2]);
    } else {
        let catalog_header = Paragraph::new(vec![Line::from(vec![
            Span::styled("Catalog", accent_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{} visible / {} total", visible_window.len(), song_count),
                muted_style(theme),
            ),
            Span::raw("  "),
            Span::styled(selected.chart_name(), muted_style(theme)),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        );
        frame.render_widget(catalog_header, rail[0]);

        let item_height = if screen.compress_lists { 2 } else { 3 };
        let mut item_constraints = vec![Constraint::Length(item_height); visible_window.len()];
        let visible_len = visible_window.len();
        item_constraints.push(Constraint::Min(0));
        let list_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(item_constraints)
            .split(rail[1]);

        for (row, index) in visible_window.enumerate() {
            let song = &app.song_choices()[index];
            let selected_item = index == app.selected_chart_index();
            let lines = catalog_row_lines(song, theme, selected_item, screen.compress_lists);

            let block = Block::default()
                .borders(if row == 0 {
                    Borders::ALL
                } else {
                    Borders::LEFT | Borders::RIGHT | Borders::BOTTOM
                })
                .border_type(BorderType::Rounded)
                .style(if selected_item {
                    selected_row_style(theme)
                } else {
                    layered_panel_style(theme)
                });
            frame.render_widget(
                Paragraph::new(lines)
                    .block(block.border_style(separator_style(theme)))
                    .wrap(Wrap { trim: true }),
                list_areas[row],
            );
        }

        if let Some(remainder_area) = list_areas.get(visible_len) {
            let filler = Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme));
            frame.render_widget(filler, *remainder_area);
        }
    }

    let footer = vec![Line::from(vec![
        Span::styled("Navigate", label_style(theme)),
        Span::raw("  ↑/↓ or J/K   "),
        Span::styled("Play", label_style(theme)),
        Span::raw("  Enter   "),
        Span::styled("Imported", label_style(theme)),
        Span::raw("  I   "),
        Span::styled("Search", label_style(theme)),
        Span::raw("  /   "),
        Span::styled("Sort", label_style(theme)),
        Span::raw("  R   "),
        Span::styled("Settings", label_style(theme)),
        Span::raw("  S   "),
        Span::styled("Theme", label_style(theme)),
        Span::raw("  T   "),
        Span::styled("Quit", label_style(theme)),
        Span::raw("  Q"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

pub fn song_select_cover_image_rect(area: Rect) -> Option<Rect> {
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
        .constraints({
            let columns = browse_columns(root[1].width, screen);
            [
                Constraint::Length(columns.left),
                Constraint::Length(columns.center),
                Constraint::Length(columns.right),
            ]
        })
        .split(root[1]);
    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(20), Constraint::Min(7)])
        .split(body[1]);
    let hero = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Length(36), Constraint::Min(30)])
        .split(center[0]);

    Some(Rect {
        x: hero[0].x + 1,
        y: hero[0].y + 1,
        width: hero[0].width.saturating_sub(2),
        height: hero[0].height.saturating_sub(2),
    })
}

fn selection_status_line(best_score: Option<u32>, play_count: u32) -> String {
    format!(
        "Best {}   {} plays",
        best_score
            .map(|score| score.to_string())
            .unwrap_or_else(|| "-".to_string()),
        play_count
    )
}

fn shell_support_line(has_artwork: bool) -> &'static str {
    if has_artwork {
        "Image preview ready"
    } else {
        "Text preview"
    }
}

fn browse_navigation_rows(app: &DemoApp, theme: &ThemeTokens) -> Vec<Line<'static>> {
    vec![
        compact_row("󰍜 Library", "Bundled", theme, true),
        compact_row("󰙅 Imported", "I", theme, false),
        compact_row("󰍉 Search", "/", theme, false),
        compact_row("󰑓 Sort", app.browse_sort_mode().label(), theme, false),
        compact_row("󰒓 Settings", "S", theme, false),
    ]
}

fn visible_list_count(list_height: u16, item_height: u16) -> usize {
    usize::from((list_height / item_height.max(1)).max(1))
}

fn visible_range(
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

fn catalog_row_lines<'a>(
    song: &'a crate::app::SongChoice,
    theme: &ThemeTokens,
    selected: bool,
    compact: bool,
) -> Vec<Line<'a>> {
    let metadata_style = catalog_metadata_style(theme, selected);
    let detail_style = catalog_detail_style(theme, selected);

    if compact {
        vec![
            compact_row(
                if selected { "Now" } else { "Song" },
                song.title(),
                theme,
                selected,
            ),
            Line::from(vec![
                Span::styled(song.artist(), metadata_style),
                Span::raw("  "),
                Span::styled(format!("{} BPM", song.bpm()), detail_style),
                Span::raw("   "),
                Span::styled(format!("Stage {:02}", song.difficulty()), metadata_style),
                Span::raw("   "),
                Span::styled(song.mood(), detail_style),
            ]),
        ]
    } else {
        vec![
            compact_row(
                if selected { "Now" } else { "Song" },
                song.title(),
                theme,
                selected,
            ),
            Line::from(vec![
                Span::styled(song.artist(), metadata_style),
                Span::raw("  "),
                Span::styled(format!("{} BPM", song.bpm()), detail_style),
                Span::raw("  "),
                Span::styled(format!("Stage {:02}", song.difficulty()), detail_style),
                Span::raw("  "),
                Span::styled(song.mood(), detail_style),
            ]),
        ]
    }
}

fn catalog_metadata_style(theme: &ThemeTokens, selected: bool) -> Style {
    if selected {
        selected_row_style(theme)
    } else {
        muted_style(theme)
    }
}

fn catalog_detail_style(theme: &ThemeTokens, selected: bool) -> Style {
    if selected {
        selected_row_style(theme)
    } else {
        label_style(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        browse_navigation_rows, catalog_row_lines, selection_status_line, shell_support_line,
        visible_range,
    };
    use crate::app::DemoApp;
    use crate::ui::widgets::selected_row_style;
    use crate::ui::ThemeTokens;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    #[test]
    fn song_select_formats_status_line_for_empty_history() {
        assert_eq!(selection_status_line(None, 0), "Best -   0 plays");
    }

    #[test]
    fn selection_status_line_reports_play_count_and_best_score() {
        assert_eq!(
            selection_status_line(Some(973240), 12),
            "Best 973240   12 plays"
        );
    }

    #[test]
    fn song_select_prefers_image_preview_message_when_art_exists() {
        assert_eq!(shell_support_line(true), "Image preview ready");
        assert_eq!(shell_support_line(false), "Text preview");
    }

    #[test]
    fn song_select_render_uses_shell_bars_and_selected_row_metadata_style() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let selected = app.selected_song();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal should build");

        terminal
            .draw(|frame| super::render_song_select(frame, &app, &theme))
            .expect("song select should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Seythm"));
        assert!(buffer_contains_text(buffer, "Navigate"));
        assert!(buffer_contains_text(buffer, selected.title()));
        assert!(buffer_contains_text(buffer, selected.artist()));
    }

    #[test]
    fn song_select_render_shows_the_hybrid_navigation_menu() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal should build");

        terminal
            .draw(|frame| super::render_song_select(frame, &app, &theme))
            .expect("song select should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Library"));
        assert!(buffer_contains_text(buffer, "Imported"));
        assert!(buffer_contains_text(buffer, "Search"));
        assert!(buffer_contains_text(buffer, "Sort"));
        assert!(buffer_contains_text(buffer, "Settings"));
        assert!(buffer_contains_text(buffer, "Imported"));
        assert!(buffer_contains_text(buffer, " I "));
        assert!(buffer_contains_text(buffer, " / "));
        assert!(buffer_contains_text(buffer, " R "));
        assert!(buffer_contains_text(buffer, "Default"));
    }

    #[test]
    fn song_select_render_keeps_the_shell_readable_on_narrow_terminals() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal should build");

        terminal
            .draw(|frame| super::render_song_select(frame, &app, &theme))
            .expect("song select should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Seythm"));
        assert!(buffer_contains_text(buffer, "Browse"));
        assert!(buffer_contains_text(buffer, "Catalog"));
        assert!(buffer_contains_text(buffer, "Featured"));
    }

    #[test]
    fn selected_catalog_rows_use_active_style_for_metadata_lines() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let lines = catalog_row_lines(app.selected_song(), &theme, true, false);
        let selected_style = selected_row_style(&theme);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].spans[0].style, selected_style);
        assert_eq!(lines[1].spans[2].style, selected_style);
        assert_eq!(lines[1].spans[4].style, selected_style);
        assert_eq!(lines[1].spans[6].style, selected_style);
    }

    #[test]
    fn roomy_catalog_rows_are_kept_to_two_lines() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let lines = catalog_row_lines(app.selected_song(), &theme, false, false);

        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn browse_navigation_rows_expose_primary_shell_destinations() {
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let rows = browse_navigation_rows(&app, &theme);
        let rendered = rows
            .into_iter()
            .map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Library"));
        assert!(rendered.contains("Imported"));
        assert!(rendered.contains("Search"));
        assert!(rendered.contains("Sort"));
        assert!(rendered.contains("Settings"));
        assert!(rendered.contains("󰍜"));
        assert!(rendered.contains("󰙅"));
        assert!(rendered.contains("/"));
        assert!(rendered.contains("Default"));
    }

    #[test]
    fn visible_range_keeps_the_catalog_window_to_the_visible_subset() {
        let range = visible_range(20, 10, 5);

        assert_eq!(range, 8..13);
        assert_eq!(range.len(), 5);
    }

    fn buffer_contains_text(buffer: &Buffer, needle: &str) -> bool {
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        text.contains(needle)
    }
}
