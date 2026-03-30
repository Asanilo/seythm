use ratatui::style::Color;

use crate::app::{DemoApp, DemoMode};
use crate::ui::ThemeTokens;

pub mod calibration;
pub mod chrome;
pub mod gameplay;
pub mod hud;
pub mod imported_song_select;
pub mod lanes;
pub mod loading;
pub mod replay;
pub mod results;
pub mod search;
pub mod settings;
pub mod song_select;
pub mod startup;

pub use calibration::render_calibration;
pub use chrome::{
    accent_style, block_card, bottom_shortcut_bar, card_style, compact_row, cover_initials,
    footer_block, label_style, layered_panel_style, muted_style, pill, pill_style,
    raised_panel_style, render_metric, selected_row_style, separator_style, surface_block,
    surface_style, top_status_bar, CoverArtWidget,
};
pub use gameplay::render_gameplay;
pub use hud::format_judgment_label;
pub use imported_song_select::render_imported_song_select;
pub use lanes::lane_fill_glyph;
pub use loading::render_loading;
pub use replay::render_replay;
pub use results::{render_pause_overlay, render_results};
pub use search::render_search;
pub use settings::render_settings;
pub use song_select::render_song_select;
pub use startup::render_startup_splash;

pub fn render_demo(frame: &mut ratatui::Frame<'_>, app: &DemoApp, theme: &ThemeTokens) {
    match app.mode() {
        DemoMode::SongSelect => render_song_select(frame, app, theme),
        DemoMode::ImportedSelect => render_imported_song_select(frame, app, theme),
        DemoMode::Search => render_search(frame, app, theme),
        DemoMode::Loading => render_loading(frame, app, theme),
        DemoMode::Ready => render_gameplay(frame, app, theme),
        DemoMode::Settings => render_settings(frame, app, theme),
        DemoMode::Calibration => render_calibration(frame, app, theme),
        DemoMode::Replay => render_replay(frame, app, theme),
        DemoMode::Playing => render_gameplay(frame, app, theme),
        DemoMode::Paused => {
            render_gameplay(frame, app, theme);
            render_pause_overlay(frame, theme);
        }
        DemoMode::Results => render_results(frame, app, theme),
    }
}

pub(crate) fn parse_color(value: &str) -> Color {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16);
            let g = u8::from_str_radix(&hex[2..4], 16);
            let b = u8::from_str_radix(&hex[4..6], 16);
            if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                return Color::Rgb(r, g, b);
            }
        }
    }

    Color::Reset
}
