use serde::Deserialize;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ThemeTokens {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    #[serde(default)]
    pub shell_top_bar_bg: String,
    #[serde(default)]
    pub shell_top_bar_fg: String,
    #[serde(default)]
    pub shell_footer_bg: String,
    #[serde(default)]
    pub shell_footer_fg: String,
    #[serde(default)]
    pub shell_panel: String,
    #[serde(default)]
    pub shell_panel_alt: String,
    #[serde(default)]
    pub shell_panel_raised: String,
    #[serde(default)]
    pub shell_row_selected_bg: String,
    #[serde(default)]
    pub shell_row_selected_fg: String,
    #[serde(default)]
    pub shell_separator: String,
    #[serde(default)]
    pub shell_border: String,
    #[serde(default)]
    pub shell_pill_bg: String,
    #[serde(default)]
    pub shell_title: String,
    #[serde(default)]
    pub shell_success: String,
    #[serde(default)]
    pub shell_warning: String,
    #[serde(default)]
    pub shell_muted: String,
    #[serde(default)]
    pub shell_error: String,
    #[serde(default)]
    pub lane_border: String,
    #[serde(default)]
    pub lane_fill: String,
    #[serde(default)]
    pub lane_active: String,
    #[serde(default)]
    pub hud_panel: String,
    #[serde(default)]
    pub hud_label: String,
    #[serde(default)]
    pub judgment_perfect: String,
    #[serde(default)]
    pub judgment_great: String,
    #[serde(default)]
    pub judgment_good: String,
    #[serde(default)]
    pub judgment_miss: String,
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("failed to read theme file {path}: {source}")]
    ReadTheme {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid theme color for {field}: {value}")]
    InvalidColor { field: &'static str, value: String },
    #[error("failed to parse theme tokens: {0}")]
    Parse(#[from] toml::de::Error),
}

impl ThemeTokens {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ThemeError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| ThemeError::ReadTheme {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_toml_str(&raw)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, ThemeError> {
        let theme = toml::from_str::<Self>(raw)?;
        theme.validate_core_colors()?;
        Ok(theme.normalized())
    }

    pub fn builtin(name: &str) -> Option<Self> {
        match normalize_builtin_name(name).as_str() {
            "minimal-professional" => Self::from_toml_str(include_str!(
                "../../assets/themes/minimal-professional.toml"
            ))
            .ok(),
            "ghostty-cold" => {
                Self::from_toml_str(include_str!("../../assets/themes/ghostty-cold.toml")).ok()
            }
            "soft-luxury" => {
                Self::from_toml_str(include_str!("../../assets/themes/soft-luxury.toml")).ok()
            }
            "neon-stage" => {
                Self::from_toml_str(include_str!("../../assets/themes/neon-stage.toml")).ok()
            }
            "mono-contrast" => {
                Self::from_toml_str(include_str!("../../assets/themes/mono-contrast.toml")).ok()
            }
            "mocha-shell" => {
                Self::from_toml_str(include_str!("../../assets/themes/mocha-shell.toml")).ok()
            }
            _ => None,
        }
    }

    fn normalized(mut self) -> Self {
        let background = self.background.clone();
        let foreground = self.foreground.clone();
        let accent = self.accent.clone();

        fill_if_empty(
            &mut self.shell_panel,
            blend_hex(&background, &foreground, 0.06),
        );
        fill_if_empty(
            &mut self.shell_panel_alt,
            blend_hex(&background, &foreground, 0.11),
        );
        fill_if_empty(
            &mut self.shell_panel_raised,
            blend_hex(&background, &accent, 0.18),
        );
        fill_if_empty(
            &mut self.shell_top_bar_bg,
            blend_hex(&self.shell_panel_raised, &background, 0.35),
        );
        fill_if_empty(&mut self.shell_top_bar_fg, foreground.clone());
        fill_if_empty(&mut self.shell_footer_bg, self.shell_panel.clone());
        fill_if_empty(&mut self.shell_footer_fg, foreground.clone());
        fill_if_empty(
            &mut self.shell_row_selected_bg,
            blend_hex(&accent, &background, 0.20),
        );
        fill_if_empty(&mut self.shell_row_selected_fg, foreground.clone());
        fill_if_empty(
            &mut self.shell_separator,
            blend_hex(&foreground, &background, 0.72),
        );
        fill_if_empty(
            &mut self.shell_border,
            blend_hex(&foreground, &background, 0.58),
        );
        fill_if_empty(
            &mut self.shell_pill_bg,
            blend_hex(&accent, &background, 0.72),
        );
        fill_if_empty(&mut self.shell_title, accent.clone());
        fill_if_empty(
            &mut self.shell_success,
            blend_hex(&accent, &foreground, 0.35),
        );
        fill_if_empty(
            &mut self.shell_warning,
            blend_hex(&accent, &background, 0.45),
        );
        fill_if_empty(
            &mut self.shell_muted,
            blend_hex(&foreground, &background, 0.45),
        );
        fill_if_empty(&mut self.shell_error, blend_hex(&accent, &background, 0.82));

        fill_if_empty(&mut self.lane_border, self.shell_border.clone());
        fill_if_empty(&mut self.lane_fill, self.shell_panel.clone());
        fill_if_empty(&mut self.lane_active, accent.clone());
        fill_if_empty(&mut self.hud_panel, self.shell_panel_alt.clone());
        fill_if_empty(&mut self.hud_label, self.shell_title.clone());
        fill_if_empty(&mut self.judgment_perfect, self.shell_success.clone());
        fill_if_empty(&mut self.judgment_great, accent.clone());
        fill_if_empty(&mut self.judgment_good, self.shell_warning.clone());
        fill_if_empty(&mut self.judgment_miss, self.shell_error.clone());

        self
    }

    fn validate_core_colors(&self) -> Result<(), ThemeError> {
        validate_color("background", &self.background)?;
        validate_color("foreground", &self.foreground)?;
        validate_color("accent", &self.accent)?;
        Ok(())
    }
}

fn normalize_builtin_name(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .replace(' ', "-")
        .replace('_', "-")
}

fn fill_if_empty(value: &mut String, fallback: String) {
    if value.is_empty() {
        *value = fallback;
    }
}

fn validate_color(field: &'static str, value: &str) -> Result<(), ThemeError> {
    parse_hex_rgb(value)
        .map(|_| ())
        .ok_or_else(|| ThemeError::InvalidColor {
            field,
            value: value.to_string(),
        })
}

fn blend_hex(a: &str, b: &str, amount: f32) -> String {
    match (parse_hex_rgb(a), parse_hex_rgb(b)) {
        (Some((ar, ag, ab)), Some((br, bg, bb))) => format!(
            "#{:02x}{:02x}{:02x}",
            mix_channel(ar, br, amount),
            mix_channel(ag, bg, amount),
            mix_channel(ab, bb, amount)
        ),
        _ => a.to_string(),
    }
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((red, green, blue))
}

fn mix_channel(a: u8, b: u8, amount: f32) -> u8 {
    let amount = amount.clamp(0.0, 1.0);
    ((a as f32 * (1.0 - amount)) + (b as f32 * amount)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::{ThemeError, ThemeTokens};

    #[test]
    fn theme_loads_minimal_professional_tokens() {
        let theme = ThemeTokens::load_from_path("assets/themes/minimal-professional.toml")
            .expect("theme should load");

        assert_eq!(theme.name, "minimal professional");
        assert_eq!(theme.background, "#0f1115");
        assert_eq!(theme.foreground, "#e6e8ee");
        assert_eq!(theme.accent, "#6ea8fe");
    }

    #[test]
    fn theme_builtin_ghostty_cold_is_available() {
        let theme = ThemeTokens::builtin("ghostty-cold").expect("builtin theme");

        assert_eq!(theme.name, "ghostty cold");
    }

    #[test]
    fn theme_builtin_soft_luxury_is_available() {
        let theme = ThemeTokens::builtin("soft-luxury").expect("builtin theme");

        assert_eq!(theme.name, "soft luxury");
        assert_eq!(theme.shell_title, "#f0cfb1");
        assert_eq!(theme.shell_success, "#aebf8a");
    }

    #[test]
    fn theme_builtin_neon_stage_is_available() {
        let theme = ThemeTokens::builtin("neon-stage").expect("builtin theme");

        assert_eq!(theme.name, "neon stage");
        assert_eq!(theme.shell_title, "#7cf7ff");
        assert_eq!(theme.shell_error, "#ff6a7a");
    }

    #[test]
    fn theme_builtin_mocha_shell_is_available() {
        let theme = ThemeTokens::builtin("mocha-shell").expect("builtin theme");

        assert_eq!(theme.name, "mocha shell");
        assert_eq!(theme.shell_title, "#f5c2e7");
        assert_eq!(theme.accent, "#89b4fa");
    }

    #[test]
    fn theme_builtin_mono_contrast_is_available() {
        let theme = ThemeTokens::builtin("mono-contrast").expect("builtin theme");

        assert_eq!(theme.name, "mono contrast");
        assert_eq!(theme.background, "#0c0c0c");
        assert_eq!(theme.foreground, "#f2f2f2");
        assert_eq!(theme.accent, "#89b4fa");
    }

    #[test]
    fn theme_missing_new_fields_fall_back_to_derived_values() {
        let theme = ThemeTokens::from_toml_str(
            r##"
name = "legacy"
background = "#101010"
foreground = "#f0f0f0"
accent = "#55ccff"
"##,
        )
        .expect("legacy theme should load");

        assert_eq!(theme.shell_panel, "#1d1d1d");
        assert_eq!(theme.shell_panel_alt, "#292929");
        assert_eq!(theme.shell_panel_raised, "#1c323b");
        assert_eq!(theme.shell_top_bar_bg, "#18262c");
        assert_eq!(theme.shell_footer_bg, "#1d1d1d");
        assert_eq!(theme.shell_row_selected_bg, "#47a6cf");
        assert_eq!(theme.shell_title, "#55ccff");
        assert_eq!(theme.shell_error, "#1c323b");
        assert_eq!(theme.lane_border, "#6e6e6e");
        assert_eq!(theme.hud_panel, "#292929");
    }

    #[test]
    fn theme_rejects_invalid_core_colors() {
        let err = ThemeTokens::from_toml_str(
            r##"
name = "broken"
background = "not-a-color"
foreground = "#f0f0f0"
accent = "#55ccff"
"##,
        )
        .expect_err("invalid base color should fail");

        assert!(matches!(
            err,
            ThemeError::InvalidColor {
                field: "background",
                ..
            }
        ));
    }
}
