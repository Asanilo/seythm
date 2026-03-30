use anyhow::Context;

use crate::config::{default_settings_path, save_settings, Keymap, Settings};
use crate::runtime::GameTime;
use crate::ui::ThemeTokens;

use super::{DemoApp, DemoMode, SettingsItem};

impl DemoApp {
    pub fn open_settings(&mut self) {
        if matches!(
            self.navigation.mode,
            DemoMode::Playing | DemoMode::Settings | DemoMode::Calibration
        ) {
            return;
        }

        self.navigation.settings_return_mode = self.navigation.mode;
        self.navigation.mode = DemoMode::Settings;
    }

    pub fn close_settings(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Settings) {
            self.navigation.mode = self.navigation.settings_return_mode;
        }
    }

    pub fn move_settings_selection(&mut self, delta: i32) {
        if !matches!(self.navigation.mode, DemoMode::Settings) {
            return;
        }

        let len = SettingsItem::ALL.len() as i32;
        let next =
            (self.navigation.settings_selection.index() as i32 + delta).rem_euclid(len) as usize;
        self.navigation.settings_selection = SettingsItem::from_index(next);
    }

    pub fn activate_settings_item(&mut self) {
        if !matches!(self.navigation.mode, DemoMode::Settings) {
            return;
        }

        match self.navigation.settings_selection {
            SettingsItem::Theme => self.toggle_theme(),
            SettingsItem::StartupSplash => self.toggle_startup_splash_enabled(),
            SettingsItem::Metronome => {
                self.settings.metronome_enabled = !self.settings.metronome_enabled;
                self.persist_settings();
            }
            SettingsItem::GlobalOffset => self.adjust_global_offset_ms(5),
            SettingsItem::InputOffset => self.adjust_input_offset_ms(5),
            SettingsItem::MusicVolume => self.adjust_music_volume(5),
            SettingsItem::HitSoundVolume => self.adjust_hit_sound_volume(5),
            SettingsItem::Keymap => self.cycle_keymap_preset(),
            SettingsItem::Calibration => self.open_calibration(),
            SettingsItem::Back => self.close_settings(),
        }
    }

    pub fn open_calibration(&mut self) {
        self.navigation.calibration_return_mode = self.navigation.mode;
        self.navigation.mode = DemoMode::Calibration;
    }

    pub fn adjust_calibration(&mut self, delta: i32) {
        if matches!(self.navigation.mode, DemoMode::Calibration) {
            self.adjust_global_offset_ms(delta);
        }
    }

    pub fn finish_calibration(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Calibration) {
            self.navigation.mode = self.navigation.calibration_return_mode;
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme_state.variant = self.theme_state.variant.next();
        self.theme_state.tokens = self.theme_state.variant.load().unwrap_or_else(|| {
            ThemeTokens::builtin("mocha-shell").expect("bundled mocha-shell theme should exist")
        });
        self.settings.theme_path = self.theme_state.variant.settings_theme_path().to_string();
        self.theme_state.label = theme_label_for(&self.settings, &self.theme_state.tokens);
        self.theme_state.active_source = ActiveThemeSource::BuiltinPreset(self.theme_state.variant);
        self.persist_settings();
    }

    pub fn theme_name(&self) -> &str {
        &self.theme_state.label
    }

    pub fn active_theme_name(&self) -> &str {
        self.theme_state.tokens.name.as_str()
    }

    pub fn theme_source_kind(&self) -> &'static str {
        if self.theme_state.active_source.builtin_variant().is_some() {
            "Built-in preset"
        } else {
            "Custom theme"
        }
    }

    pub fn theme_source_ref(&self) -> &str {
        self.theme_state.active_source.source_ref()
    }

    pub fn theme_cycle_summary(&self) -> String {
        self.theme_state
            .active_source
            .builtin_variant()
            .map(|variant: DemoThemeVariant| {
                format!(
                    "Preset {}/{}",
                    variant.index() + 1,
                    DemoThemeVariant::ALL.len()
                )
            })
            .unwrap_or_else(|| "Custom theme".to_string())
    }

    pub fn metronome_enabled(&self) -> bool {
        self.settings.metronome_enabled
    }

    pub fn toggle_startup_splash_enabled(&mut self) {
        self.settings.startup_splash_enabled = !self.settings.startup_splash_enabled;
        self.persist_settings();
    }

    pub fn keymap(&self) -> &Keymap {
        &self.settings.keymap
    }

    pub fn set_keymap_str(
        &mut self,
        value: &str,
    ) -> Result<(), crate::config::keymap::KeymapParseError> {
        self.settings.keymap = Keymap::parse(value)?;
        self.persist_settings();
        Ok(())
    }

    pub fn input_offset_ms(&self) -> i32 {
        self.settings.input_offset_ms
    }

    pub fn global_offset_ms(&self) -> i32 {
        self.settings.global_offset_ms
    }

    pub fn set_global_offset_ms(&mut self, value: i32) {
        self.settings.global_offset_ms = value.clamp(-240, 240);
        self.persist_settings();
    }

    pub fn set_input_offset_ms(&mut self, value: i32) {
        self.settings.input_offset_ms = value.clamp(-180, 180);
        self.persist_settings();
    }

    pub(super) fn cycle_keymap_preset(&mut self) {
        let next = match self.settings.keymap.as_string().as_str() {
            "S D F J K L" => "A S D J K L",
            "A S D J K L" => "Z X C , . /",
            _ => "S D F J K L",
        };
        if self.set_keymap_str(next).is_ok() {
            self.active_lanes = [false; 6];
        }
    }

    pub(super) fn persist_settings(&self) {
        let _ = save_settings(default_settings_path(), &self.settings);
    }

    pub(super) fn adjust_input_offset_ms(&mut self, delta: i32) {
        self.set_input_offset_ms(self.settings.input_offset_ms.saturating_add(delta));
    }

    pub(super) fn adjust_global_offset_ms(&mut self, delta: i32) {
        self.set_global_offset_ms(self.settings.global_offset_ms.saturating_add(delta));
    }

    pub(super) fn adjust_music_volume(&mut self, delta: i32) {
        let next = (i32::from(self.settings.music_volume) + delta).clamp(0, 100) as u8;
        self.settings.music_volume = next;
        self.persist_settings();
    }

    pub(super) fn adjust_hit_sound_volume(&mut self, delta: i32) {
        let next = (i32::from(self.settings.hit_sound_volume) + delta).clamp(0, 100) as u8;
        self.settings.hit_sound_volume = next;
        self.persist_settings();
    }

    pub(super) fn apply_input_offset(&self, time: GameTime) -> GameTime {
        GameTime::from_millis(time.as_millis() + i64::from(self.settings.input_offset_ms))
    }
}

#[cfg(test)]
pub(super) fn load_theme(settings: &Settings) -> anyhow::Result<ThemeTokens> {
    load_theme_with_source(settings).map(|(theme, _)| theme)
}

pub(super) fn load_theme_with_source(
    settings: &Settings,
) -> anyhow::Result<(ThemeTokens, ActiveThemeSource)> {
    if let Some(builtin_name) = settings.builtin_theme_name() {
        if let Some(variant) = DemoThemeVariant::from_builtin_name(builtin_name) {
            if let Some(theme) = variant.load() {
                return Ok((theme, ActiveThemeSource::BuiltinPreset(variant)));
            }
        }
    }

    if let Ok(theme) = ThemeTokens::load_from_path(settings.theme_path_buf()) {
        return Ok((
            theme,
            ActiveThemeSource::CustomPath(settings.theme_path.clone()),
        ));
    }

    let fallback = ThemeTokens::builtin("mocha-shell").context("builtin theme missing")?;
    Ok((
        fallback,
        ActiveThemeSource::BuiltinPreset(DemoThemeVariant::MochaShell),
    ))
}

pub(super) fn theme_label_for(settings: &Settings, theme: &ThemeTokens) -> String {
    if settings.is_builtin_theme() {
        theme.name.clone()
    } else {
        format!("custom: {}", settings.theme_path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DemoThemeVariant {
    MinimalProfessional,
    MonoContrast,
    MochaShell,
    GhosttyCold,
    SoftLuxury,
    NeonStage,
}

impl DemoThemeVariant {
    pub(super) const ALL: [Self; 6] = [
        Self::MinimalProfessional,
        Self::MonoContrast,
        Self::MochaShell,
        Self::GhosttyCold,
        Self::SoftLuxury,
        Self::NeonStage,
    ];

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn index(self) -> usize {
        match self {
            Self::MinimalProfessional => 0,
            Self::MonoContrast => 1,
            Self::MochaShell => 2,
            Self::GhosttyCold => 3,
            Self::SoftLuxury => 4,
            Self::NeonStage => 5,
        }
    }

    fn builtin_name(self) -> &'static str {
        match self {
            Self::MinimalProfessional => "minimal-professional",
            Self::MonoContrast => "mono-contrast",
            Self::MochaShell => "mocha-shell",
            Self::GhosttyCold => "ghostty-cold",
            Self::SoftLuxury => "soft-luxury",
            Self::NeonStage => "neon-stage",
        }
    }

    fn settings_theme_path(self) -> &'static str {
        match self {
            Self::MinimalProfessional => "builtin:minimal-professional",
            Self::MonoContrast => "builtin:mono-contrast",
            Self::MochaShell => "builtin:mocha-shell",
            Self::GhosttyCold => "builtin:ghostty-cold",
            Self::SoftLuxury => "builtin:soft-luxury",
            Self::NeonStage => "builtin:neon-stage",
        }
    }

    fn load(self) -> Option<ThemeTokens> {
        ThemeTokens::builtin(self.builtin_name())
    }

    fn from_builtin_name(name: &str) -> Option<Self> {
        match name {
            "minimal-professional" => Some(Self::MinimalProfessional),
            "mono-contrast" => Some(Self::MonoContrast),
            "mocha-shell" => Some(Self::MochaShell),
            "ghostty-cold" => Some(Self::GhosttyCold),
            "soft-luxury" => Some(Self::SoftLuxury),
            "neon-stage" => Some(Self::NeonStage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum ActiveThemeSource {
    BuiltinPreset(DemoThemeVariant),
    CustomPath(String),
}

impl ActiveThemeSource {
    pub(super) fn from_settings_and_theme(settings: &Settings, _theme: &ThemeTokens) -> Self {
        if let Some(builtin_name) = settings.builtin_theme_name() {
            if let Some(variant) = DemoThemeVariant::from_builtin_name(builtin_name) {
                return Self::BuiltinPreset(variant);
            }
        }

        if settings.theme_path.starts_with("builtin:") {
            return Self::BuiltinPreset(DemoThemeVariant::MochaShell);
        }

        Self::CustomPath(settings.theme_path.clone())
    }

    pub(super) fn builtin_variant(&self) -> Option<DemoThemeVariant> {
        match self {
            Self::BuiltinPreset(variant) => Some(*variant),
            Self::CustomPath(_) => None,
        }
    }

    fn source_ref(&self) -> &str {
        match self {
            Self::BuiltinPreset(variant) => variant.builtin_name(),
            Self::CustomPath(path) => path.as_str(),
        }
    }
}
