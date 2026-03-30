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

pub fn render_imported_song_select(
    frame: &mut ratatui::Frame<'_>,
    app: &DemoApp,
    theme: &ThemeTokens,
) {
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

    let selected = app
        .selected_imported_song()
        .unwrap_or_else(|| app.selected_song());
    let selected_record = app.song_profile_record(selected.id());
    let status_line = selection_status_line(
        selected_record.map(|record| record.best_score),
        selected_record.map(|record| record.play_count).unwrap_or(0),
    );
    let top_bar = vec![Line::from(vec![
        Span::styled(app.product_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled("Imported", label_style(theme)),
        Span::raw("  "),
        Span::styled(
            format!(
                "{}/{}",
                app.imported_selected_chart_index() + 1,
                app.imported_song_choices().len()
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
                    7
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
        let navigation = Paragraph::new(imported_navigation_rows(app, theme))
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled(
                        "Browse",
                        accent_style(theme),
                    )]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(raised_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(navigation, body[0]);

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
                pill("Imported", theme),
                Span::raw(" "),
                pill(format!("{} Keys", selected.difficulty()), theme),
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
                .title(Line::from(vec![
                    Span::styled("Featured", accent_style(theme)),
                    Span::raw("  "),
                    Span::styled("imported context", muted_style(theme)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(raised_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(featured, body[1]);

        let visible_window = visible_range(
            app.imported_song_choices().len(),
            app.imported_selected_chart_index(),
            visible_list_count(body[2].height.saturating_sub(2), 2),
        );
        let mut catalog_lines = Vec::new();
        for index in visible_window.clone() {
            let song = &app.imported_song_choices()[index];
            let selected_item = index == app.imported_selected_chart_index();
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
                        Span::styled("Imported Catalog", accent_style(theme)),
                        Span::raw("  "),
                        Span::styled(
                            format!(
                                "{} visible / {} total",
                                visible_window.len(),
                                app.imported_song_choices().len()
                            ),
                            muted_style(theme),
                        ),
                    ]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(layered_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(catalog, body[2]);
    } else {
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(8)])
            .split(body[0]);
        let navigation_header = Paragraph::new(vec![Line::from(vec![
            Span::styled("Browse", accent_style(theme)),
            Span::raw("  "),
            Span::styled("imported library", muted_style(theme)),
        ])])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        );
        frame.render_widget(navigation_header, left[0]);
        let navigation_menu = Paragraph::new(imported_navigation_rows(app, theme))
            .block(
                Block::default()
                    .title(Line::from(vec![Span::styled("Menu", label_style(theme))]))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .style(raised_panel_style(theme))
                    .border_style(separator_style(theme)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(navigation_menu, left[1]);

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
                pill("Imported", theme),
                Span::raw(" "),
                pill(format!("{} Keys", selected.difficulty()), theme),
                Span::raw(" "),
                pill(selected.mood(), theme),
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
                    "Imported",
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Library", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    format!("{} tracks", app.imported_song_choices().len()),
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Open", label_style(theme)),
                Span::raw("  "),
                Span::styled(
                    "Press B for bundled charts",
                    Style::default().fg(parse_color(&theme.foreground)),
                ),
            ]),
        ])
        .block(
            Block::default()
                .title(Line::from(vec![
                    Span::styled("Run", label_style(theme)),
                    Span::raw("  "),
                    Span::styled("imported context", muted_style(theme)),
                ]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(layered_panel_style(theme))
                .border_style(separator_style(theme)),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(support, center[1]);

        let song_count = app.imported_song_choices().len().max(1);
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(body[2]);
        let visible_count =
            visible_list_count(sidebar[1].height, if screen.compress_lists { 2 } else { 3 });
        let visible_window = visible_range(
            app.imported_song_choices().len(),
            app.imported_selected_chart_index(),
            visible_count,
        );
        let catalog_header = Paragraph::new(vec![Line::from(vec![
            Span::styled("Imported Catalog", accent_style(theme)),
            Span::raw("  "),
            Span::styled(format!("{} tracks", song_count), muted_style(theme)),
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
        frame.render_widget(catalog_header, sidebar[0]);

        let item_height = if screen.compress_lists { 2 } else { 3 };
        let mut item_constraints = vec![Constraint::Length(item_height); visible_window.len()];
        let visible_len = visible_window.len();
        item_constraints.push(Constraint::Min(0));
        let list_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(item_constraints)
            .split(sidebar[1]);

        for (row, index) in visible_window.enumerate() {
            let song = &app.imported_song_choices()[index];
            let selected_item = index == app.imported_selected_chart_index();
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
        Span::styled("Bundled", label_style(theme)),
        Span::raw("  B   "),
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

pub fn imported_song_select_cover_image_rect(area: Rect) -> Option<Rect> {
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

fn imported_navigation_rows(app: &DemoApp, theme: &ThemeTokens) -> Vec<Line<'static>> {
    vec![
        compact_row("󰍜 Library", "Bundled", theme, false),
        compact_row("󰙅 Imported", "I", theme, true),
        compact_row("󰍉 Search", "/", theme, false),
        compact_row("󰑓 Sort", app.browse_sort_mode().label(), theme, false),
        compact_row("󰒓 Settings", "S", theme, false),
    ]
}

fn catalog_row_lines<'a>(
    song: &'a crate::app::SongChoice,
    theme: &ThemeTokens,
    selected: bool,
    compact: bool,
) -> Vec<Line<'a>> {
    let metadata_style = if selected {
        selected_row_style(theme)
    } else {
        muted_style(theme)
    };
    let detail_style = if selected {
        selected_row_style(theme)
    } else {
        label_style(theme)
    };

    if compact {
        vec![
            compact_row(
                if selected { "Now" } else { "Imported" },
                song.chart_name(),
                theme,
                selected,
            ),
            Line::from(vec![
                Span::styled(song.title(), metadata_style),
                Span::raw("  "),
                Span::styled(song.artist(), detail_style),
                Span::raw("   "),
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
                if selected { "Now" } else { "Imported" },
                song.chart_name(),
                theme,
                selected,
            ),
            Line::from(vec![
                Span::styled(song.title(), metadata_style),
                Span::raw("  "),
                Span::styled(song.artist(), detail_style),
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

#[cfg(test)]
mod tests {
    use super::{catalog_row_lines, render_imported_song_select};
    use crate::app::DemoApp;
    use crate::content::{
        save_imported_song_catalog, ImportedSongCatalog, ImportedSongCatalogEntry,
        IMPORTED_ORIGIN_TYPE,
    };
    use crate::osu::import::import_osu_mania_folder;
    use crate::ui::ThemeTokens;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn imported_view_render_shows_the_hybrid_navigation_menu_and_live_actions() {
        let import_root = temp_import_root("imported-hybrid-menu");
        let source_folder =
            fixture_copy("tests/fixtures/osu/valid-6k", "imported-hybrid-menu-source");
        import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

        let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
            .expect("runtime app should load imported catalog");
        app.open_imported_view();

        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal should build");

        terminal
            .draw(|frame| render_imported_song_select(frame, &app, &theme))
            .expect("imported view should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Seythm"));
        assert!(buffer_contains_text(buffer, "Imported"));
        assert!(buffer_contains_text(buffer, "Library"));
        assert!(buffer_contains_text(buffer, "Search"));
        assert!(buffer_contains_text(buffer, "Sort"));
        assert!(buffer_contains_text(buffer, "Settings"));
        assert!(buffer_contains_text(buffer, "󰍜"));
        assert!(buffer_contains_text(buffer, "󰙅"));
        assert!(buffer_contains_text(buffer, " / "));
        assert!(buffer_contains_text(buffer, " R "));
        assert!(buffer_contains_text(buffer, "Default"));
        assert!(buffer_contains_text(buffer, "Navigate"));
        assert!(buffer_contains_text(buffer, "Play"));
        assert!(buffer_contains_text(buffer, "Bundled"));
        assert!(buffer_contains_text(buffer, "B"));
        assert!(buffer_contains_text(buffer, "Settings"));
        assert!(buffer_contains_text(buffer, "Theme"));
        assert!(buffer_contains_text(buffer, "Quit"));
    }

    #[test]
    fn imported_view_render_keeps_the_shell_readable_on_narrow_terminals() {
        let import_root = temp_import_root("imported-hybrid-compact");
        let source_folder = fixture_copy(
            "tests/fixtures/osu/valid-6k",
            "imported-hybrid-compact-source",
        );
        import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

        let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
            .expect("runtime app should load imported catalog");
        app.open_imported_view();

        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let mut terminal = Terminal::new(TestBackend::new(82, 24)).expect("terminal should build");

        terminal
            .draw(|frame| render_imported_song_select(frame, &app, &theme))
            .expect("imported view should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Seythm"));
        assert!(buffer_contains_text(buffer, "Imported"));
        assert!(buffer_contains_text(buffer, "Imported Catalog"));
        assert!(buffer_contains_text(buffer, "Featured"));
    }

    #[test]
    fn imported_catalog_rows_use_chart_name_as_the_primary_label() {
        let import_root = temp_import_root("imported-row-primary-label");
        let song_dir = import_root.join("demo-pack");
        fs::create_dir_all(&song_dir).expect("song dir");
        let chart_path = song_dir.join("chart.toml");
        let audio_path = song_dir.join("song.ogg");
        let source_osu_path = song_dir.join("map.osu");
        fs::write(
            &chart_path,
            r#"[metadata]
title = "6K Starter Practice Pack"
artist = "Various Artists"
chart_name = "[4] Uesugi Uta with HoshiA - Canon"
offset_ms = 0

[[timing]]
start_ms = 0
bpm = 145.0
beat_length = 4

[[notes]]
kind = "tap"
time_ms = 1000
lane = 0
"#,
        )
        .expect("write chart");
        fs::write(&audio_path, []).expect("write dummy audio");
        fs::write(&source_osu_path, "osu file format v14").expect("write dummy osu");
        save_imported_song_catalog(
            &import_root,
            &ImportedSongCatalog {
                songs: vec![ImportedSongCatalogEntry {
                    id: "demo-pack".to_string(),
                    title: "6K Starter Practice Pack".to_string(),
                    artist: "Various Artists".to_string(),
                    chart_name: "[4] Uesugi Uta with HoshiA - Canon".to_string(),
                    difficulty: 6,
                    bpm: 145,
                    mood: "Imported".to_string(),
                    chart_path,
                    audio_path,
                    artwork_path: None,
                    source_osu_path,
                    source_folder: "fixture".to_string(),
                    source_osu_filename: "map.osu".to_string(),
                    imported_at_unix_ms: 123,
                    origin_type: IMPORTED_ORIGIN_TYPE.to_string(),
                }],
            },
        )
        .expect("save catalog");
        let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
            .expect("runtime app should load imported catalog");
        app.open_imported_view();

        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");
        let song = app
            .selected_imported_song()
            .expect("imported selection should exist");
        let rows = catalog_row_lines(song, &theme, true, false);
        let rendered = rows[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("[4] Uesugi Uta with HoshiA - Canon"));
        assert!(!rendered.contains("6K Starter Practice Pack"));
    }

    fn temp_import_root(test_name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "code_m-{}-{}-{}",
            test_name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn fixture_copy(folder: &str, suffix: &str) -> PathBuf {
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder);
        let temp_root = temp_import_root(suffix);
        fs::copy(source_root.join("map.osu"), temp_root.join("map.osu")).expect("copy map");
        fs::copy(source_root.join("song.ogg"), temp_root.join("song.ogg")).expect("copy audio");
        temp_root
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
