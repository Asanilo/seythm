use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::app::{DemoApp, SettingsItem};
use crate::ui::layout::classify_screen;
use crate::ui::widgets::chrome::{
    accent_style, block_card, bottom_shortcut_bar, label_style, layered_panel_style, muted_style,
    pill, raised_panel_style, selected_row_style, separator_style, top_status_bar,
};
use crate::ui::widgets::parse_color;
use crate::ui::ThemeTokens;

pub fn render_settings(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
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
        Span::styled("Settings", label_style(theme)),
        Span::raw("  "),
        Span::styled(app.active_theme_name(), accent_style(theme)),
        Span::raw("  "),
        Span::styled(app.theme_source_kind(), muted_style(theme)),
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
                Constraint::Length(8),
                Constraint::Min(12),
                Constraint::Min(16),
            ]
        } else {
            vec![
                Constraint::Percentage(28),
                Constraint::Percentage(34),
                Constraint::Percentage(38),
            ]
        })
        .split(root[1]);

    let theme_focus = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                app.active_theme_name(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            pill(app.theme_cycle_summary(), theme),
        ]),
        Line::from(vec![
            Span::styled("Source", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.theme_source_kind(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Ref", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.theme_source_ref(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Theme changes apply immediately across the live shell. Use Left/Right, Enter, or T to cycle built-in presets.",
            muted_style(theme),
        )),
    ])
    .block(
        Block::default()
            .title(Line::from(vec![
                Span::styled("Theme Control", accent_style(theme)),
                Span::raw("  "),
                Span::styled("live runtime surface", muted_style(theme)),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(raised_panel_style(theme))
            .border_style(separator_style(theme)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(theme_focus, body[0]);

    let keymap = app.keymap().as_string();
    let controls = Paragraph::new(vec![
        section_line("Appearance", theme),
        setting_line(
            app.settings_selection(),
            SettingsItem::Theme,
            "Theme",
            app.active_theme_name(),
            theme,
        ),
        setting_line(
            app.settings_selection(),
            SettingsItem::StartupSplash,
            "Startup",
            if app.startup_splash_enabled() {
                "On"
            } else {
                "Off"
            },
            theme,
        ),
        Line::from(""),
        section_line("Audio", theme),
        setting_line(
            app.settings_selection(),
            SettingsItem::Metronome,
            "Metronome",
            if app.metronome_enabled() { "On" } else { "Off" },
            theme,
        ),
        setting_line(
            app.settings_selection(),
            SettingsItem::MusicVolume,
            "Music",
            &format!("{}%", app.music_volume()),
            theme,
        ),
        setting_line(
            app.settings_selection(),
            SettingsItem::HitSoundVolume,
            "Hit Sound",
            &format!("{}%", app.hit_sound_volume()),
            theme,
        ),
        Line::from(""),
        section_line("Timing", theme),
        setting_line(
            app.settings_selection(),
            SettingsItem::GlobalOffset,
            "Global Sync",
            &format!("{:+} ms", app.global_offset_ms()),
            theme,
        ),
        setting_line(
            app.settings_selection(),
            SettingsItem::InputOffset,
            "Input Bias",
            &format!("{:+} ms", app.input_offset_ms()),
            theme,
        ),
        Line::from(""),
        section_line("Input", theme),
        setting_line(
            app.settings_selection(),
            SettingsItem::Keymap,
            "Keymap",
            &keymap,
            theme,
        ),
        setting_line(
            app.settings_selection(),
            SettingsItem::Calibration,
            "Calibration",
            "Open",
            theme,
        ),
        Line::from(""),
        setting_line(
            app.settings_selection(),
            SettingsItem::Back,
            "Back",
            "Return",
            theme,
        ),
    ])
    .block(
        Block::default()
            .title(Line::from(vec![
                Span::styled("Control Surface", label_style(theme)),
                Span::raw("  "),
                Span::styled("active row is live", muted_style(theme)),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(layered_panel_style(theme))
            .border_style(separator_style(theme)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(controls, body[1]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(8),
        ])
        .split(body[2]);

    let active_theme = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Preset", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.active_theme_name(),
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Source", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.theme_source_kind(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Ref", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.theme_source_ref(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status", label_style(theme)),
            Span::raw("  "),
            Span::styled("Applied live", accent_style(theme)),
        ]),
    ])
    .block(block_card(
        Line::from(Span::styled("Active Theme", accent_style(theme))),
        theme,
    ))
    .wrap(Wrap { trim: true });
    frame.render_widget(active_theme, right[0]);

    let (selected_label, selected_value, selected_hint) = selected_setting_copy(app);
    let selected_state = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Selected", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                selected_label,
                Style::default()
                    .fg(parse_color(&theme.foreground))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Value", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                selected_value,
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(selected_hint, muted_style(theme))),
    ])
    .block(block_card(
        Line::from(Span::styled("Active Control", label_style(theme))),
        theme,
    ))
    .wrap(Wrap { trim: true });
    frame.render_widget(selected_state, right[1]);

    let system_summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Brand", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                app.brand_name(),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Music", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{}%", app.music_volume()),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
            Span::raw("  "),
            Span::styled("Hit FX", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{}%", app.hit_sound_volume()),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Metronome", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                if app.metronome_enabled() {
                    "Enabled"
                } else {
                    "Muted"
                },
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Global Sync", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{:+} ms", app.global_offset_ms()),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Input Bias", label_style(theme)),
            Span::raw("  "),
            Span::styled(
                format!("{:+} ms", app.input_offset_ms()),
                Style::default().fg(parse_color(&theme.foreground)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Keymap", label_style(theme)),
            Span::raw("  "),
            Span::styled(keymap, Style::default().fg(parse_color(&theme.foreground))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Global sync shifts the stage clock. Input bias shifts judgment timing. Keymap and mix changes persist immediately.",
            muted_style(theme),
        )),
    ])
    .block(
        Block::default()
            .title(Line::from(vec![
                Span::styled("System Summary", label_style(theme)),
                Span::raw("  "),
                Span::styled("timing and mix", muted_style(theme)),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(layered_panel_style(theme))
            .border_style(separator_style(theme)),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(system_summary, right[2]);

    let footer = vec![Line::from(vec![
        Span::styled("UP/DOWN", accent_style(theme)),
        Span::raw(" move  "),
        Span::styled("LEFT/RIGHT", accent_style(theme)),
        Span::raw(" adjust live  "),
        Span::styled("ENTER", accent_style(theme)),
        Span::raw(" apply/toggle  "),
        Span::styled("B", accent_style(theme)),
        Span::raw(" back  "),
        Span::styled("T", accent_style(theme)),
        Span::raw(" quick theme"),
    ])];
    frame.render_widget(bottom_shortcut_bar(footer, theme), root[2]);
}

fn setting_line(
    selected: SettingsItem,
    item: SettingsItem,
    label: &str,
    value: &str,
    theme: &ThemeTokens,
) -> Line<'static> {
    let active = selected == item;
    let row_style = if active {
        selected_row_style(theme)
    } else {
        Style::default().fg(parse_color(&theme.foreground))
    };
    let marker = if active { ">" } else { " " };

    Line::from(vec![
        Span::styled(format!("{marker} "), row_style),
        Span::styled(format!("{label:<11}"), row_style),
        Span::styled("  ", row_style),
        Span::styled(value.to_string(), row_style),
    ])
}

fn section_line(label: &str, theme: &ThemeTokens) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        accent_style(theme).add_modifier(Modifier::BOLD),
    ))
}

fn selected_setting_copy(app: &DemoApp) -> (&'static str, String, &'static str) {
    match app.settings_selection() {
        SettingsItem::Theme => (
            "Theme",
            format!(
                "{} ({})",
                app.active_theme_name(),
                app.theme_cycle_summary()
            ),
            "Cycle through bundled presets and apply the new shell tokens immediately.",
        ),
        SettingsItem::Metronome => (
            "Metronome",
            if app.metronome_enabled() {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            "Toggles the fallback timing pulse used for calibration and silent preview.",
        ),
        SettingsItem::StartupSplash => (
            "Startup Splash",
            if app.startup_splash_enabled() {
                "On".to_string()
            } else {
                "Off".to_string()
            },
            "Controls whether the branded startup card appears before entering the shell.",
        ),
        SettingsItem::GlobalOffset => (
            "Global Sync",
            format!("{:+} ms", app.global_offset_ms()),
            "Moves the shared stage clock relative to audio playback.",
        ),
        SettingsItem::InputOffset => (
            "Input Bias",
            format!("{:+} ms", app.input_offset_ms()),
            "Adjusts judgment timing without shifting the song clock.",
        ),
        SettingsItem::MusicVolume => (
            "Music",
            format!("{}%", app.music_volume()),
            "Controls the backing track mix for the current runtime session.",
        ),
        SettingsItem::HitSoundVolume => (
            "Hit Sound",
            format!("{}%", app.hit_sound_volume()),
            "Balances key-hit feedback against the music channel.",
        ),
        SettingsItem::Keymap => (
            "Keymap",
            app.keymap().as_string(),
            "Cycles the bundled six-key layouts and updates input handling immediately.",
        ),
        SettingsItem::Calibration => (
            "Calibration",
            "Open".to_string(),
            "Enter the calibration view to tune sync against the preview clock.",
        ),
        SettingsItem::Back => (
            "Back",
            "Return".to_string(),
            "Leave settings and return to the previous shell.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::render_settings;
    use crate::app::DemoApp;
    use crate::config::Settings;
    use crate::ui::ThemeTokens;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    #[test]
    fn settings_render_shows_shell_sections_and_active_theme_metadata() {
        let seed = DemoApp::from_demo_chart().expect("demo app should load");
        let settings = Settings {
            theme_path: "themes/ghostty-cold.toml".to_string(),
            ..Settings::default()
        };
        let theme = ThemeTokens::from_toml_str(
            r##"
name = "ghostty cold"
background = "#0b1120"
foreground = "#f8fafc"
accent = "#38bdf8"
"##,
        )
        .expect("custom theme should parse");
        let app = DemoApp::new(seed.song_choices().to_vec(), settings, theme.clone());
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).expect("terminal should build");

        terminal
            .draw(|frame| render_settings(frame, &app, &theme))
            .expect("settings should render");

        let buffer = terminal.backend().buffer();
        assert!(buffer_contains_text(buffer, "Seythm"));
        assert!(buffer_contains_text(buffer, "Settings"));
        assert!(buffer_contains_text(buffer, "Theme Control"));
        assert!(buffer_contains_text(buffer, "Active Theme"));
        assert!(buffer_contains_text(buffer, "ghostty cold"));
        assert!(buffer_contains_text(buffer, "Custom theme"));
        assert!(buffer_contains_text(buffer, "themes/ghostty-cold.toml"));
        assert!(buffer_contains_text(buffer, "Startup"));
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
