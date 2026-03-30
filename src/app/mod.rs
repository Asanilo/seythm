use std::io::{self, Stdout, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::audio::PlaybackSession;
use crate::chart::parser::parse_chart_file;
use crate::chart::scheduler::{ChartScheduler, LaneNoteProjection};
use crate::chart::Chart;
use crate::chart::Note;
use crate::config::{
    default_profile_path, default_settings_path, load_bundled_branding, load_profile,
    load_settings, save_profile, BrandingConfig, ProfileRecord, ResultProfile, Settings,
};
use crate::content::{
    default_import_root, load_bundled_song_catalog, load_imported_song_catalog,
    prepare_default_import_root,
};
use crate::gameplay::{GameplayState, HitEvent, HitPhase, Judgment, ScoreSummary};
use crate::runtime::input::lane_for_key;
use crate::runtime::GameTime;
use crate::runtime::{InputAction, InputEvent, ReplayInput};
use crate::ui::widgets::{render_demo, render_startup_splash};
use crate::ui::{
    image_protocol::{terminal_supports_graphics, ArtworkRenderer},
    ThemeTokens,
};

mod input;
mod flow;
mod session;
mod settings;

const APPROACH_WINDOW_MS: i64 = 2_000;
const FRAME_TIME: Duration = Duration::from_millis(16);
const STARTUP_SPLASH_MS: u64 = 1_250;
const LANE_FEEDBACK_WINDOW_MS: i64 = 180;
const JUDGMENT_FEEDBACK_WINDOW_MS: i64 = 240;
const COMBO_FEEDBACK_WINDOW_MS: i64 = 280;
const LOADING_TOTAL_MS: i64 = 2_000;
const READY_BUFFER_MS: i64 = 500;
const LOADING_AUDIO_READY_MS: i64 = 450;
const LOADING_CHART_READY_MS: i64 = 950;
const LOADING_INPUT_READY_MS: i64 = 1_350;

use input::poll_input;
use session::{current_demo_time, refresh_playback_session, DemoClockAnchor};
use settings::{load_theme_with_source, theme_label_for, ActiveThemeSource, DemoThemeVariant};

#[derive(Debug, Clone, Copy, Default)]
pub struct DemoLaunchOptions {
    pub autoplay: bool,
}

pub fn run_demo() -> anyhow::Result<()> {
    run_demo_with_options(DemoLaunchOptions::default())
}

pub fn run_demo_with_options(options: DemoLaunchOptions) -> anyhow::Result<()> {
    let mut app = DemoApp::from_runtime_chart()?;
    app.set_autoplay_enabled(options.autoplay);
    let mut terminal = TerminalGuard::enter()?;
    if app.startup_splash_enabled() {
        render_startup_sequence(&mut terminal.terminal, &app, options)?;
    }
    let mut clock_anchor = DemoClockAnchor::new(Instant::now(), GameTime::from_millis(0));
    let mut playback: Option<PlaybackSession> = None;
    let mut session_generation = app.session_generation();
    let mut mode = app.mode();
    let mut hit_sound_requests = app.pending_hit_sound_requests();
    let mut artwork_renderer = if terminal_supports_graphics() {
        Some(ArtworkRenderer::default())
    } else {
        None
    };

    loop {
        refresh_playback_session(
            &app,
            &mut playback,
            &mut clock_anchor,
            &mut session_generation,
            &mut mode,
        );

        if matches!(
            app.mode(),
            DemoMode::Loading | DemoMode::Ready | DemoMode::Playing | DemoMode::Replay
        ) {
            let now = current_demo_time(
                playback.as_ref(),
                clock_anchor,
                app.mode(),
                app.global_offset_ms(),
            );
            app.update(now);
        }

        if let Some(session) = playback.as_ref() {
            session.set_mix_levels(app.music_volume(), app.hit_sound_volume());
            while hit_sound_requests < app.pending_hit_sound_requests() {
                session.trigger_hit_sound();
                hit_sound_requests = hit_sound_requests.wrapping_add(1);
            }
        } else {
            hit_sound_requests = app.pending_hit_sound_requests();
        }

        refresh_playback_session(
            &app,
            &mut playback,
            &mut clock_anchor,
            &mut session_generation,
            &mut mode,
        );

        terminal
            .terminal
            .draw(|frame| render_demo(frame, &app, app.theme()))
            .context("failed to render terminal frame")?;

        if let Some(renderer) = artwork_renderer.as_mut() {
            let area = terminal
                .terminal
                .size()
                .context("failed to read terminal size for artwork sync")?;
            let area = ratatui::layout::Rect::new(0, 0, area.width, area.height);
            renderer
                .sync(terminal.terminal.backend_mut(), &app, area)
                .context("failed to render terminal artwork")?;
            terminal
                .terminal
                .backend_mut()
                .flush()
                .context("failed to flush terminal artwork")?;
        }

        let now = current_demo_time(
            playback.as_ref(),
            clock_anchor,
            app.mode(),
            app.global_offset_ms(),
        );
        if poll_input(FRAME_TIME, &mut app, now)? {
            break;
        }
    }

    Ok(())
}

fn render_startup_sequence(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &DemoApp,
    options: DemoLaunchOptions,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    loop {
        let elapsed_ms = started_at.elapsed().as_millis() as u64;
        let progress = (elapsed_ms as f32 / STARTUP_SPLASH_MS as f32).min(1.0);

        terminal
            .draw(|frame| {
                render_startup_splash(
                    frame,
                    app.theme(),
                    app.brand_name(),
                    app.brand_tagline(),
                    app.product_logo(),
                    app.startup_hint(),
                    progress,
                    options.autoplay,
                )
            })
            .context("failed to render startup splash")?;

        if progress >= 1.0 {
            return Ok(());
        }

        if event::poll(FRAME_TIME).context("failed to poll startup splash input")? {
            if matches!(
                event::read().context("failed to read startup splash input")?,
                Event::Key(_)
            ) {
                return Ok(());
            }
        }
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        stdout
            .execute(EnterAlternateScreen)
            .context("failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("failed to create terminal backend")?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = write!(self.terminal.backend_mut(), "\x1b_Ga=d,d=A\x1b\\");
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoMode {
    SongSelect,
    ImportedSelect,
    Search,
    Loading,
    Ready,
    Settings,
    Calibration,
    Replay,
    Playing,
    Paused,
    Results,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    Theme,
    StartupSplash,
    Metronome,
    GlobalOffset,
    InputOffset,
    MusicVolume,
    HitSoundVolume,
    Keymap,
    Calibration,
    Back,
}

impl SettingsItem {
    const ALL: [Self; 10] = [
        Self::Theme,
        Self::StartupSplash,
        Self::Metronome,
        Self::GlobalOffset,
        Self::InputOffset,
        Self::MusicVolume,
        Self::HitSoundVolume,
        Self::Keymap,
        Self::Calibration,
        Self::Back,
    ];

    fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    fn index(self) -> usize {
        match self {
            Self::Theme => 0,
            Self::StartupSplash => 1,
            Self::Metronome => 2,
            Self::GlobalOffset => 3,
            Self::InputOffset => 4,
            Self::MusicVolume => 5,
            Self::HitSoundVolume => 6,
            Self::Keymap => 7,
            Self::Calibration => 8,
            Self::Back => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartLibrary {
    Bundled,
    Imported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseSortMode {
    Default,
    Title,
    Bpm,
    Difficulty,
}

impl BrowseSortMode {
    fn next(self) -> Self {
        match self {
            Self::Default => Self::Title,
            Self::Title => Self::Bpm,
            Self::Bpm => Self::Difficulty,
            Self::Difficulty => Self::Default,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Title => "Title",
            Self::Bpm => "BPM",
            Self::Difficulty => "Stage",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SongChoice {
    id: String,
    title: String,
    artist: String,
    chart_name: String,
    difficulty: u8,
    bpm: u16,
    mood: String,
    artwork_path: Option<std::path::PathBuf>,
    audio_path: Option<std::path::PathBuf>,
    chart: Chart,
}

impl SongChoice {
    #[cfg(test)]
    fn from_chart(chart: Chart) -> Self {
        Self {
            id: chart.metadata.title.to_lowercase().replace(' ', "-"),
            title: chart.metadata.title.clone(),
            artist: chart.metadata.artist.clone(),
            chart_name: chart.metadata.chart_name.clone(),
            difficulty: 4,
            bpm: 120,
            mood: "Debug".to_string(),
            artwork_path: None,
            audio_path: None,
            chart,
        }
    }

    fn from_loaded_chart(
        id: String,
        title: String,
        artist: String,
        chart_name: String,
        difficulty: u8,
        bpm: u16,
        mood: String,
        artwork_path: Option<std::path::PathBuf>,
        audio_path: Option<std::path::PathBuf>,
        chart: Chart,
    ) -> Self {
        Self {
            id,
            title,
            artist,
            chart_name,
            difficulty,
            bpm,
            mood,
            artwork_path,
            audio_path,
            chart,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn chart_name(&self) -> &str {
        &self.chart_name
    }

    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn mood(&self) -> &str {
        &self.mood
    }

    pub fn artwork_path(&self) -> Option<&Path> {
        self.artwork_path.as_deref()
    }

    pub fn audio_path(&self) -> Option<&Path> {
        self.audio_path.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct DemoApp {
    chart_choices: Vec<SongChoice>,
    selected_chart_index: usize,
    imported_chart_choices: Vec<SongChoice>,
    imported_selected_chart_index: usize,
    active_chart_index: usize,
    active_chart_library: ChartLibrary,
    chart: Chart,
    scheduler: ChartScheduler,
    gameplay: GameplayState,
    settings: Settings,
    theme_state: ThemeState,
    latest_judgment: Option<Judgment>,
    current_time: GameTime,
    active_lanes: [bool; 6],
    lane_feedback_until: [Option<GameTime>; 6],
    judgment_feedback_until: Option<GameTime>,
    combo_feedback_until: Option<GameTime>,
    profile: ResultProfile,
    latest_result_record: Option<ProfileRecord>,
    current_run_events: Vec<InputEvent>,
    replay_state: ReplayState,
    autoplay_state: AutoplayState,
    search_state: SearchState,
    navigation: NavigationState,
    branding: BrandingConfig,
    session_generation: u64,
    hit_sound_requests: u64,
}

#[derive(Debug, Clone)]
struct ThemeState {
    tokens: ThemeTokens,
    label: String,
    active_source: ActiveThemeSource,
    variant: DemoThemeVariant,
}

#[derive(Debug, Clone, Default)]
struct ReplayState {
    latest_replay: Option<ReplayInput>,
    cursor: Option<usize>,
    preview_playing: bool,
    gameplay: Option<GameplayState>,
    input_cursor: usize,
    latest_judgment: Option<Judgment>,
}

#[derive(Debug, Clone, Default)]
struct AutoplayState {
    enabled: bool,
    events: Vec<InputEvent>,
    event_cursor: usize,
}

#[derive(Debug, Clone)]
struct SearchState {
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
    previous_mode: DemoMode,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            previous_mode: DemoMode::SongSelect,
        }
    }
}

#[derive(Debug, Clone)]
struct SearchResult {
    library: ChartLibrary,
    index: usize,
}

#[derive(Debug, Clone, Copy)]
struct NavigationState {
    mode: DemoMode,
    settings_selection: SettingsItem,
    settings_return_mode: DemoMode,
    calibration_return_mode: DemoMode,
    browse_return_mode: DemoMode,
    bundled_sort_mode: BrowseSortMode,
    imported_sort_mode: BrowseSortMode,
}

impl DemoApp {
    pub fn from_demo_chart() -> anyhow::Result<Self> {
        Self::from_settings(Settings::default())
    }

    pub fn from_runtime_chart() -> anyhow::Result<Self> {
        let settings =
            load_settings(default_settings_path()).unwrap_or_else(|_| Settings::default());
        let profile = load_profile(default_profile_path()).unwrap_or_default();
        let import_root = prepare_default_import_root().unwrap_or_else(|_| default_import_root());
        Self::from_settings_and_profile_with_import_root(settings, profile, import_root)
    }

    fn from_settings(settings: Settings) -> anyhow::Result<Self> {
        Self::from_settings_and_profile_with_import_root(
            settings,
            ResultProfile::default(),
            prepare_default_import_root().unwrap_or_else(|_| default_import_root()),
        )
    }

    pub fn from_runtime_chart_with_import_root(
        import_root: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let settings =
            load_settings(default_settings_path()).unwrap_or_else(|_| Settings::default());
        let profile = load_profile(default_profile_path()).unwrap_or_default();
        Self::from_settings_and_profile_with_import_root(settings, profile, import_root)
    }

    fn from_settings_and_profile_with_import_root(
        settings: Settings,
        profile: ResultProfile,
        import_root: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let bundled_chart_choices = load_bundled_song_catalog()?
            .songs()
            .iter()
            .map(|song| {
                let chart = parse_chart_file(song.chart_path()).with_context(|| {
                    format!("failed to parse chart {}", song.chart_path().display())
                })?;
                Ok(SongChoice::from_loaded_chart(
                    song.id().to_string(),
                    song.title().to_string(),
                    song.artist().to_string(),
                    song.chart_name().to_string(),
                    song.difficulty(),
                    song.bpm(),
                    song.mood().to_string(),
                    song.artwork_path().map(std::path::Path::to_path_buf),
                    song.audio_path().map(std::path::Path::to_path_buf),
                    chart,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let imported_chart_choices = load_imported_song_choices(import_root.as_ref())?;
        let (theme, active_theme_source) = load_theme_with_source(&settings)?;
        let branding = load_bundled_branding();
        Ok(Self::new_with_imports_and_profile(
            bundled_chart_choices,
            imported_chart_choices,
            settings,
            theme,
            active_theme_source,
            profile,
            branding,
        ))
    }

    pub fn new(chart_choices: Vec<SongChoice>, settings: Settings, theme: ThemeTokens) -> Self {
        let active_theme_source = ActiveThemeSource::from_settings_and_theme(&settings, &theme);
        Self::new_with_imports_and_profile(
            chart_choices,
            Vec::new(),
            settings,
            theme,
            active_theme_source,
            ResultProfile::default(),
            load_bundled_branding(),
        )
    }

    fn new_with_imports_and_profile(
        chart_choices: Vec<SongChoice>,
        imported_chart_choices: Vec<SongChoice>,
        settings: Settings,
        theme: ThemeTokens,
        active_theme_source: ActiveThemeSource,
        profile: ResultProfile,
        branding: BrandingConfig,
    ) -> Self {
        let selected_chart_index = 0;
        let active_chart_index = selected_chart_index;
        let chart = chart_choices[active_chart_index].chart.clone();
        let theme_variant = active_theme_source
            .builtin_variant()
            .unwrap_or(DemoThemeVariant::MochaShell);
        let theme_label = theme_label_for(&settings, &theme);

        Self {
            scheduler: ChartScheduler::new(chart.clone(), APPROACH_WINDOW_MS),
            gameplay: GameplayState::new(chart.clone()),
            chart_choices,
            imported_chart_choices,
            selected_chart_index,
            imported_selected_chart_index: 0,
            active_chart_index,
            active_chart_library: ChartLibrary::Bundled,
            chart,
            settings,
            theme_state: ThemeState {
                tokens: theme,
                label: theme_label,
                active_source: active_theme_source,
                variant: theme_variant,
            },
            latest_judgment: None,
            current_time: GameTime::from_millis(0),
            active_lanes: [false; 6],
            lane_feedback_until: [None; 6],
            judgment_feedback_until: None,
            combo_feedback_until: None,
            profile,
            latest_result_record: None,
            current_run_events: Vec::new(),
            replay_state: ReplayState::default(),
            autoplay_state: AutoplayState::default(),
            search_state: SearchState::default(),
            navigation: NavigationState {
                mode: DemoMode::SongSelect,
                settings_selection: SettingsItem::Theme,
                settings_return_mode: DemoMode::SongSelect,
                calibration_return_mode: DemoMode::Settings,
                browse_return_mode: DemoMode::SongSelect,
                bundled_sort_mode: BrowseSortMode::Default,
                imported_sort_mode: BrowseSortMode::Default,
            },
            branding,
            session_generation: 0,
            hit_sound_requests: 0,
        }
    }

    pub fn imported_song_choices(&self) -> &[SongChoice] {
        &self.imported_chart_choices
    }

    pub fn product_name(&self) -> &str {
        &self.branding.product_name
    }

    pub fn product_tagline(&self) -> &str {
        &self.branding.tagline
    }

    pub fn product_logo(&self) -> &str {
        &self.branding.ascii_logo
    }

    pub fn startup_hint(&self) -> &str {
        &self.branding.startup_hint
    }

    pub fn footer_hint(&self) -> &str {
        &self.branding.footer_hint
    }

    pub fn brand_name(&self) -> &str {
        self.product_name()
    }

    pub fn brand_tagline(&self) -> &str {
        self.product_tagline()
    }

    pub fn startup_splash_enabled(&self) -> bool {
        self.settings.startup_splash_enabled
    }

    pub fn browse_sort_mode(&self) -> BrowseSortMode {
        match self.navigation.mode {
            DemoMode::ImportedSelect => self.navigation.imported_sort_mode,
            _ => self.navigation.bundled_sort_mode,
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        if !matches!(self.navigation.mode, DemoMode::SongSelect) || self.chart_choices.is_empty() {
            return;
        }

        let len = self.chart_choices.len() as i32;
        let next = (self.selected_chart_index as i32 + delta).rem_euclid(len) as usize;
        self.selected_chart_index = next;
    }

    pub fn move_imported_selection(&mut self, delta: i32) {
        if !matches!(self.navigation.mode, DemoMode::ImportedSelect)
            || self.imported_chart_choices.is_empty()
        {
            return;
        }

        let len = self.imported_chart_choices.len() as i32;
        let next = (self.imported_selected_chart_index as i32 + delta).rem_euclid(len) as usize;
        self.imported_selected_chart_index = next;
    }

    pub fn open_search(&mut self) {
        if !matches!(self.navigation.mode, DemoMode::SongSelect | DemoMode::ImportedSelect) {
            return;
        }
        self.search_state.previous_mode = self.navigation.mode;
        self.search_state.query.clear();
        self.refresh_search_results();
        self.navigation.mode = DemoMode::Search;
    }

    pub fn close_search(&mut self) {
        if matches!(self.navigation.mode, DemoMode::Search) {
            self.navigation.mode = self.search_state.previous_mode;
        }
    }

    pub fn push_search_char(&mut self, ch: char) {
        if !matches!(self.navigation.mode, DemoMode::Search) || ch.is_control() {
            return;
        }
        self.search_state.query.push(ch);
        self.refresh_search_results();
    }

    pub fn pop_search_char(&mut self) {
        if !matches!(self.navigation.mode, DemoMode::Search) {
            return;
        }
        self.search_state.query.pop();
        self.refresh_search_results();
    }

    pub fn move_search_selection(&mut self, delta: i32) {
        if !matches!(self.navigation.mode, DemoMode::Search) || self.search_state.results.is_empty() {
            return;
        }
        let len = self.search_state.results.len() as i32;
        self.search_state.selected =
            (self.search_state.selected as i32 + delta).rem_euclid(len) as usize;
    }

    pub fn activate_search_selection(&mut self) -> anyhow::Result<()> {
        if !matches!(self.navigation.mode, DemoMode::Search) {
            return Ok(());
        }
        let Some(result) = self
            .search_state
            .results
            .get(self.search_state.selected)
            .cloned()
        else {
            return Ok(());
        };
        match result.library {
            ChartLibrary::Bundled => {
                self.selected_chart_index = result.index;
                self.navigation.mode = DemoMode::SongSelect;
            }
            ChartLibrary::Imported => {
                self.imported_selected_chart_index = result.index;
                self.navigation.mode = DemoMode::ImportedSelect;
            }
        }
        self.start_selected_chart()
    }

    pub fn open_imported_view(&mut self) {
        if matches!(self.navigation.mode, DemoMode::SongSelect)
            && !self.imported_chart_choices.is_empty()
        {
            self.navigation.mode = DemoMode::ImportedSelect;
        }
    }

    pub fn cycle_browse_sort(&mut self) {
        match self.navigation.mode {
            DemoMode::SongSelect => {
                let selected_id = self.selected_song().id().to_string();
                self.navigation.bundled_sort_mode = self.navigation.bundled_sort_mode.next();
                Self::sort_song_choices(
                    &mut self.chart_choices,
                    self.navigation.bundled_sort_mode,
                );
                self.selected_chart_index = Self::find_song_index(&self.chart_choices, &selected_id);
            }
            DemoMode::ImportedSelect => {
                if let Some(selected) = self.selected_imported_song() {
                    let selected_id = selected.id().to_string();
                    self.navigation.imported_sort_mode =
                        self.navigation.imported_sort_mode.next();
                    Self::sort_song_choices(
                        &mut self.imported_chart_choices,
                        self.navigation.imported_sort_mode,
                    );
                    self.imported_selected_chart_index =
                        Self::find_song_index(&self.imported_chart_choices, &selected_id);
                }
            }
            _ => {}
        }
    }

    pub fn start_selected_chart(&mut self) -> anyhow::Result<()> {
        let (library, chart_index) = self.current_selected_chart();
        self.active_chart_library = library;
        self.active_chart_index = chart_index;
        self.navigation.browse_return_mode = self.navigation.mode;
        self.reset_chart_state(library, chart_index)?;
        Ok(())
    }

    pub fn return_to_song_select(&mut self) {
        self.return_to_browse_view();
    }

    pub fn handle_key_char(&mut self, key: char, time: GameTime) -> bool {
        let normalized = key.to_ascii_uppercase().to_string();
        let Some(lane) = lane_for_key(&self.settings.keymap, &normalized) else {
            return false;
        };

        if matches!(self.navigation.mode, DemoMode::Playing) {
            self.current_run_events.push(InputEvent::new(
                normalized.clone(),
                time.as_millis(),
                InputAction::Press,
            ));
        }

        self.handle_lane_press(lane, self.apply_input_offset(time))
    }

    pub fn handle_key_release_char(&mut self, key: char, time: GameTime) -> bool {
        let normalized = key.to_ascii_uppercase().to_string();
        let Some(lane) = lane_for_key(&self.settings.keymap, &normalized) else {
            return false;
        };

        if matches!(self.navigation.mode, DemoMode::Playing) {
            self.current_run_events.push(InputEvent::new(
                normalized.clone(),
                time.as_millis(),
                InputAction::Release,
            ));
        }

        self.handle_lane_release(lane, self.apply_input_offset(time))
    }

    pub fn handle_lane_press(&mut self, lane: u8, time: GameTime) -> bool {
        if !matches!(self.navigation.mode, DemoMode::Playing) {
            return false;
        }

        self.current_time = time;
        self.active_lanes[lane as usize] = true;

        let hit = self
            .gameplay
            .press_lane(lane, time)
            .or_else(|| self.gameplay.release_lane(lane, time));

        if let Some(event) = hit {
            self.apply_hit_event(event);
            self.refresh_mode();
            return true;
        }

        self.active_lanes[lane as usize] = false;
        false
    }

    pub fn handle_lane_release(&mut self, lane: u8, time: GameTime) -> bool {
        if !matches!(self.navigation.mode, DemoMode::Playing) {
            return false;
        }

        self.current_time = time;
        self.active_lanes[lane as usize] = false;

        let Some(event) = self.gameplay.release_lane(lane, time) else {
            self.refresh_mode();
            return false;
        };

        self.apply_hit_event(event);
        self.refresh_mode();
        true
    }

    pub fn toggle_pause(&mut self) {
        self.navigation.mode = match self.navigation.mode {
            DemoMode::Playing => {
                self.active_lanes = [false; 6];
                DemoMode::Paused
            }
            DemoMode::Paused => DemoMode::Playing,
            DemoMode::SongSelect
            | DemoMode::ImportedSelect
            | DemoMode::Search
            | DemoMode::Loading
            | DemoMode::Ready
            | DemoMode::Results
            | DemoMode::Settings
            | DemoMode::Calibration
            | DemoMode::Replay => self.navigation.mode,
        };
    }

    pub fn restart(&mut self) -> anyhow::Result<()> {
        if matches!(self.navigation.mode, DemoMode::SongSelect) {
            return Ok(());
        }

        self.reset_chart_state(self.active_chart_library, self.active_chart_index)?;
        Ok(())
    }

    pub fn visible_notes(&self) -> Vec<LaneNoteProjection> {
        self.scheduler.visible_notes(self.current_time)
    }

    pub fn latest_judgment(&self) -> Option<Judgment> {
        self.latest_judgment
    }

    pub fn score_summary(&self) -> ScoreSummary {
        self.gameplay.summary()
    }

    pub fn result_record(&self) -> Option<&ProfileRecord> {
        self.latest_result_record.as_ref()
    }

    pub fn last_replay(&self) -> Option<&ReplayInput> {
        self.replay_state.latest_replay.as_ref()
    }

    pub fn replay_cursor(&self) -> Option<usize> {
        self.replay_state.cursor
    }

    pub fn replay_preview_playing(&self) -> bool {
        self.replay_state.preview_playing
    }

    pub fn replay_latest_judgment(&self) -> Option<Judgment> {
        self.replay_state.latest_judgment
    }

    pub fn replay_current_time_ms(&self) -> i64 {
        self.current_time.as_millis()
    }

    pub fn song_profile_record(&self, song_id: &str) -> Option<&ProfileRecord> {
        self.profile.song(song_id)
    }

    pub fn active_lanes(&self) -> &[bool; 6] {
        &self.active_lanes
    }

    pub fn lane_feedback_strength(&self, lane: usize) -> f32 {
        self.feedback_strength(
            self.lane_feedback_until.get(lane).copied().flatten(),
            LANE_FEEDBACK_WINDOW_MS,
        )
    }

    pub fn judgment_feedback_strength(&self) -> f32 {
        self.feedback_strength(self.judgment_feedback_until, JUDGMENT_FEEDBACK_WINDOW_MS)
    }

    pub fn combo_feedback_strength(&self) -> f32 {
        self.feedback_strength(self.combo_feedback_until, COMBO_FEEDBACK_WINDOW_MS)
    }

    pub fn mode(&self) -> DemoMode {
        self.navigation.mode
    }

    pub fn settings_selection(&self) -> SettingsItem {
        self.navigation.settings_selection
    }

    pub fn selected_chart_title(&self) -> &str {
        self.chart_choices[self.selected_chart_index].title()
    }

    pub fn selected_song(&self) -> &SongChoice {
        &self.chart_choices[self.selected_chart_index]
    }

    pub fn imported_selected_chart_index(&self) -> usize {
        self.imported_selected_chart_index
    }

    pub fn selected_imported_song(&self) -> Option<&SongChoice> {
        self.imported_chart_choices
            .get(self.imported_selected_chart_index)
    }

    pub fn active_song(&self) -> &SongChoice {
        self.song_for_library(self.active_chart_library, self.active_chart_index)
            .or_else(|| match self.active_chart_library {
                ChartLibrary::Bundled => self.chart_choices.first(),
                ChartLibrary::Imported => self
                    .imported_chart_choices
                    .first()
                    .or_else(|| self.chart_choices.first()),
            })
            .expect("demo app should always have at least one bundled song")
    }

    pub fn playback_song(&self) -> &SongChoice {
        match self.navigation.mode {
            DemoMode::SongSelect => &self.chart_choices[self.selected_chart_index],
            DemoMode::ImportedSelect => self
                .imported_chart_choices
                .get(self.imported_selected_chart_index)
                .unwrap_or(&self.chart_choices[self.selected_chart_index]),
            DemoMode::Calibration => {
                if matches!(self.navigation.calibration_return_mode, DemoMode::Settings) {
                    self.song_for_browse_selection()
                        .unwrap_or(&self.chart_choices[0])
                } else {
                    self.song_for_library(self.active_chart_library, self.active_chart_index)
                        .unwrap_or(&self.chart_choices[self.selected_chart_index])
                }
            }
            _ => self
                .song_for_library(self.active_chart_library, self.active_chart_index)
                .unwrap_or(&self.chart_choices[self.selected_chart_index]),
        }
    }

    pub fn playback_progress_ratio(&self) -> f32 {
        let total = chart_duration_ms(&self.chart).max(1) as f32;
        (self.current_time.as_millis().max(0) as f32 / total).clamp(0.0, 1.0)
    }


    pub fn current_grade(&self) -> &'static str {
        grade_label(live_accuracy(self.score_summary()))
    }

    pub fn selected_chart_index(&self) -> usize {
        self.selected_chart_index
    }

    pub fn song_choices(&self) -> &[SongChoice] {
        &self.chart_choices
    }

    fn find_song_index(choices: &[SongChoice], song_id: &str) -> usize {
        choices
            .iter()
            .position(|song| song.id() == song_id)
            .unwrap_or(0)
    }

    fn sort_song_choices(choices: &mut [SongChoice], mode: BrowseSortMode) {
        match mode {
            BrowseSortMode::Default => choices.sort_by(|left, right| left.id.cmp(&right.id)),
            BrowseSortMode::Title => choices.sort_by(|left, right| {
                left.title
                    .cmp(&right.title)
                    .then_with(|| left.artist.cmp(&right.artist))
                    .then_with(|| left.chart_name.cmp(&right.chart_name))
            }),
            BrowseSortMode::Bpm => choices.sort_by(|left, right| {
                left.bpm
                    .cmp(&right.bpm)
                    .then_with(|| left.title.cmp(&right.title))
                    .then_with(|| left.chart_name.cmp(&right.chart_name))
            }),
            BrowseSortMode::Difficulty => choices.sort_by(|left, right| {
                left.difficulty
                    .cmp(&right.difficulty)
                    .then_with(|| left.title.cmp(&right.title))
                    .then_with(|| left.chart_name.cmp(&right.chart_name))
            }),
        }
    }

    pub fn search_query(&self) -> &str {
        &self.search_state.query
    }

    pub fn search_selected_index(&self) -> usize {
        self.search_state.selected
    }

    pub fn search_results_len(&self) -> usize {
        self.search_state.results.len()
    }

    pub fn search_result_rows(&self) -> Vec<SearchResultRow<'_>> {
        self.search_state
            .results
            .iter()
            .map(|result| {
                let song = self
                    .song_for_library(result.library, result.index)
                    .expect("search result should point to a valid song");
                SearchResultRow {
                    source: match result.library {
                        ChartLibrary::Bundled => "Bundled",
                        ChartLibrary::Imported => "Imported",
                    },
                    title: song.title(),
                    artist: song.artist(),
                    chart_name: song.chart_name(),
                    mood: song.mood(),
                    bpm: song.bpm(),
                    difficulty: song.difficulty(),
                }
            })
            .collect()
    }

    fn current_selected_chart(&self) -> (ChartLibrary, usize) {
        match self.navigation.mode {
            DemoMode::ImportedSelect => {
                (ChartLibrary::Imported, self.imported_selected_chart_index)
            }
            _ => (ChartLibrary::Bundled, self.selected_chart_index),
        }
    }

    fn song_for_browse_selection(&self) -> Option<&SongChoice> {
        match self.navigation.settings_return_mode {
            DemoMode::SongSelect => self.chart_choices.get(self.selected_chart_index),
            DemoMode::ImportedSelect => self
                .imported_chart_choices
                .get(self.imported_selected_chart_index),
            _ => self.song_for_library(self.active_chart_library, self.active_chart_index),
        }
    }

    fn song_for_library(&self, library: ChartLibrary, index: usize) -> Option<&SongChoice> {
        match library {
            ChartLibrary::Bundled => self.chart_choices.get(index),
            ChartLibrary::Imported => self.imported_chart_choices.get(index),
        }
    }

    fn refresh_search_results(&mut self) {
        let query = self.search_state.query.trim().to_ascii_lowercase();
        let mut results = Vec::new();

        for (index, song) in self.chart_choices.iter().enumerate() {
            if search_matches(song, &query) {
                results.push(SearchResult {
                    library: ChartLibrary::Bundled,
                    index,
                });
            }
        }

        for (index, song) in self.imported_chart_choices.iter().enumerate() {
            if search_matches(song, &query) {
                results.push(SearchResult {
                    library: ChartLibrary::Imported,
                    index,
                });
            }
        }

        self.search_state.results = results;
        self.search_state.selected = self
            .search_state
            .selected
            .min(self.search_state.results.len().saturating_sub(1));
    }

    pub fn chart_title(&self) -> &str {
        &self.chart.metadata.title
    }

    pub fn chart_artist(&self) -> &str {
        &self.chart.metadata.artist
    }

    pub fn chart_name(&self) -> &str {
        &self.chart.metadata.chart_name
    }

    pub fn chart(&self) -> &Chart {
        &self.chart
    }

    pub fn playback_chart(&self) -> &Chart {
        &self.playback_song().chart
    }

    pub fn theme(&self) -> &ThemeTokens {
        &self.theme_state.tokens
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn music_volume(&self) -> u8 {
        self.settings.music_volume
    }

    pub fn hit_sound_volume(&self) -> u8 {
        self.settings.hit_sound_volume
    }

    pub fn pending_hit_sound_requests(&self) -> u64 {
        self.hit_sound_requests
    }

    fn apply_hit_event(&mut self, event: HitEvent) {
        self.latest_judgment = Some(event.judgment);
        if event.judgment.is_hit() {
            self.hit_sound_requests = self.hit_sound_requests.wrapping_add(1);
            self.lane_feedback_until[event.lane as usize] = Some(GameTime::from_millis(
                self.current_time.as_millis() + LANE_FEEDBACK_WINDOW_MS,
            ));
            self.judgment_feedback_until = Some(GameTime::from_millis(
                self.current_time.as_millis() + JUDGMENT_FEEDBACK_WINDOW_MS,
            ));
            self.combo_feedback_until = Some(GameTime::from_millis(
                self.current_time.as_millis() + COMBO_FEEDBACK_WINDOW_MS,
            ));
        }
        if !matches!(event.phase, HitPhase::HoldStart) {
            self.active_lanes[event.lane as usize] = false;
        }
    }

    fn bump_session(&mut self) {
        self.session_generation = self.session_generation.wrapping_add(1);
    }

    fn feedback_strength(&self, until: Option<GameTime>, window_ms: i64) -> f32 {
        let Some(until) = until else {
            return 0.0;
        };
        let remaining = until.as_millis() - self.current_time.as_millis();
        if remaining <= 0 {
            return 0.0;
        }
        (remaining as f32 / window_ms as f32).clamp(0.0, 1.0)
    }

    fn persist_profile(&self) {
        let _ = save_profile(default_profile_path(), &self.profile);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::Instant;

    use super::input::process_key_event;
    use super::settings::load_theme;
    use super::session::{current_demo_time, playback_transition, DemoClockAnchor, PlaybackTransition};
    use super::{DemoApp, DemoMode, SettingsItem, SongChoice};
    use crate::chart::{Chart, ChartMetadata, Note, NoteKind, TapNote, TimingPoint};
    use crate::config::Settings;
    use crate::gameplay::Judgment;
    use crate::runtime::GameTime;
    use crate::ui::ThemeTokens;

    fn test_chart() -> Chart {
        Chart {
            metadata: ChartMetadata {
                title: "Chord Test".to_string(),
                artist: "Code M".to_string(),
                chart_name: "Debug".to_string(),
                offset_ms: 0,
            },
            timing: vec![TimingPoint {
                start_ms: 0,
                bpm: 120.0,
                beat_length: 4,
            }],
            notes: vec![
                Note::Tap(TapNote {
                    kind: NoteKind::Tap,
                    time_ms: 1000,
                    lane: 0,
                }),
                Note::Tap(TapNote {
                    kind: NoteKind::Tap,
                    time_ms: 1000,
                    lane: 3,
                }),
            ],
        }
    }

    fn test_app(settings: Settings, theme: ThemeTokens) -> DemoApp {
        DemoApp::new(vec![SongChoice::from_chart(test_chart())], settings, theme)
    }

    #[test]
    fn processes_multiple_press_events_in_the_same_tick() {
        let theme = crate::ui::ThemeTokens::builtin("minimal-professional")
            .expect("built-in theme should load");
        let mut app = test_app(Settings::default(), theme);
        app.start_selected_chart()
            .expect("selected chart should start");
        app.skip_loading_intro();
        app.update(GameTime::from_millis(1000));

        let first = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let second = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);

        process_key_event(&mut app, GameTime::from_millis(1000), first)
            .expect("first key should process");
        process_key_event(&mut app, GameTime::from_millis(1000), second)
            .expect("second key should process");

        let summary = app.score_summary();
        assert_eq!(summary.combo, 2);
        assert_eq!(summary.judgments.perfect, 2);
        assert_eq!(app.mode(), DemoMode::Playing);
        assert_eq!(app.latest_judgment(), Some(Judgment::Perfect));
    }

    #[test]
    fn builtin_theme_selection_uses_builtin_label() {
        let settings = Settings {
            theme_path: "builtin:ghostty-cold".to_string(),
            ..Settings::default()
        };
        let theme = ThemeTokens::builtin("ghostty-cold").expect("built-in theme should load");
        let app = test_app(settings, theme);

        assert_eq!(app.theme_name(), "ghostty cold");
    }

    #[test]
    fn custom_theme_selection_uses_custom_label() {
        let settings = Settings {
            theme_path: "themes/custom.toml".to_string(),
            ..Settings::default()
        };
        let theme = ThemeTokens::builtin("mono-contrast").expect("built-in theme should load");
        let app = test_app(settings, theme);

        assert_eq!(app.theme_name(), "custom: themes/custom.toml");
    }

    #[test]
    fn toggle_theme_persists_builtin_selection_ids() {
        let settings = Settings {
            theme_path: "builtin:minimal-professional".to_string(),
            ..Settings::default()
        };
        let theme =
            ThemeTokens::builtin("minimal-professional").expect("built-in theme should load");
        let mut app = test_app(settings, theme);

        assert_eq!(app.settings.theme_path, "builtin:minimal-professional");
        app.toggle_theme();
        assert_eq!(app.settings.theme_path, "builtin:mono-contrast");
        assert_eq!(app.theme_name(), "mono contrast");
        app.toggle_theme();
        assert_eq!(app.settings.theme_path, "builtin:mocha-shell");
        assert_eq!(app.theme_name(), "mocha shell");
        app.toggle_theme();
        assert_eq!(app.settings.theme_path, "builtin:ghostty-cold");
        assert_eq!(app.theme_name(), "ghostty cold");
    }

    #[test]
    fn theme_cycle_visits_all_builtin_themes_before_wrapping() {
        let settings = Settings {
            theme_path: "builtin:minimal-professional".to_string(),
            ..Settings::default()
        };
        let theme =
            ThemeTokens::builtin("minimal-professional").expect("built-in theme should load");
        let mut app = test_app(settings, theme);

        let mut visited = vec![app.theme_name().to_string()];
        for _ in 0..5 {
            app.toggle_theme();
            visited.push(app.theme_name().to_string());
        }

        assert_eq!(
            visited,
            vec![
                "minimal professional".to_string(),
                "mono contrast".to_string(),
                "mocha shell".to_string(),
                "ghostty cold".to_string(),
                "soft luxury".to_string(),
                "neon stage".to_string(),
            ]
        );

        app.toggle_theme();
        assert_eq!(app.theme_name(), "minimal professional");
        assert_eq!(app.settings.theme_path, "builtin:minimal-professional");
    }

    #[test]
    fn invalid_builtin_theme_id_falls_back_without_builtin_label() {
        let settings = Settings {
            theme_path: "builtin:typo".to_string(),
            ..Settings::default()
        };
        let theme = load_theme(&settings).expect("fallback theme should load");
        assert_eq!(theme.name, "mocha shell");

        let app = test_app(settings, theme);
        assert_eq!(app.theme_name(), "custom: builtin:typo");
    }

    #[test]
    fn active_theme_metadata_reflects_loaded_builtin_fallback() {
        let settings = Settings {
            theme_path: "builtin:typo".to_string(),
            ..Settings::default()
        };
        let theme = load_theme(&settings).expect("fallback theme should load");
        let app = test_app(settings, theme);

        assert_eq!(app.theme().name, "mocha shell");
        assert_eq!(app.theme_source_kind(), "Built-in preset");
        assert_eq!(app.theme_source_ref(), "mocha-shell");
        assert_eq!(app.theme_cycle_summary(), "Preset 3/6");
    }

    #[test]
    fn active_theme_metadata_reflects_loaded_custom_theme() {
        let settings = Settings {
            theme_path: "themes/custom.toml".to_string(),
            ..Settings::default()
        };
        let theme = ThemeTokens::from_toml_str(
            r##"
name = "midnight signal"
background = "#101820"
foreground = "#f5f5f5"
accent = "#7dd3fc"
"##,
        )
        .expect("custom theme should parse");
        let app = test_app(settings, theme);

        assert_eq!(app.theme().name, "midnight signal");
        assert_eq!(app.theme_source_kind(), "Custom theme");
        assert_eq!(app.theme_source_ref(), "themes/custom.toml");
        assert_eq!(app.theme_cycle_summary(), "Custom theme");
    }

    #[test]
    fn custom_theme_named_like_builtin_still_reports_custom_source() {
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
        let app = test_app(settings, theme);

        assert_eq!(app.theme_source_kind(), "Custom theme");
        assert_eq!(app.theme_source_ref(), "themes/ghostty-cold.toml");
        assert_eq!(app.theme_cycle_summary(), "Custom theme");
    }

    #[test]
    fn playback_transition_stops_audio_when_leaving_gameplay() {
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::Paused, false),
            PlaybackTransition::StopAndFreeze
        );
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::Results, false),
            PlaybackTransition::StopAndFreeze
        );
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::SongSelect, false),
            PlaybackTransition::StopAndFreeze
        );
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::Settings, false),
            PlaybackTransition::StopAndFreeze
        );
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::Calibration, false),
            PlaybackTransition::StopAndFreeze
        );
    }

    #[test]
    fn playback_transition_restarts_audio_when_gameplay_begins_or_session_restarts() {
        assert_eq!(
            playback_transition(DemoMode::Paused, DemoMode::Playing, false),
            PlaybackTransition::StartOrResume
        );
        assert_eq!(
            playback_transition(DemoMode::Settings, DemoMode::Calibration, false),
            PlaybackTransition::StartOrResume
        );
        assert_eq!(
            playback_transition(DemoMode::Calibration, DemoMode::Playing, false),
            PlaybackTransition::StartOrResume
        );
        assert_eq!(
            playback_transition(DemoMode::Playing, DemoMode::Playing, true),
            PlaybackTransition::StartOrResume
        );
    }

    #[test]
    fn current_demo_time_applies_global_offset_while_playing() {
        let anchor = DemoClockAnchor::new(Instant::now(), GameTime::from_millis(120));

        let time = current_demo_time(None, anchor, DemoMode::Playing, 24);
        assert!(time.as_millis() >= 144);

        let paused = current_demo_time(None, anchor, DemoMode::Paused, 24);
        assert_eq!(paused, GameTime::from_millis(120));
    }

    #[test]
    fn settings_flow_mutates_theme_and_metronome_state() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = app.theme().clone();
        let mut app = DemoApp::new(app.song_choices().to_vec(), Settings::default(), theme);

        assert_eq!(app.mode(), DemoMode::SongSelect);
        assert!(app.metronome_enabled());

        app.open_settings();
        assert_eq!(app.mode(), DemoMode::Settings);
        assert_eq!(app.settings_selection(), SettingsItem::Theme);

        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::StartupSplash);
        app.activate_settings_item();
        assert!(!app.startup_splash_enabled());

        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::Metronome);
        app.activate_settings_item();
        assert!(!app.metronome_enabled());

        app.move_settings_selection(-2);
        let before = app.theme_name().to_string();
        app.activate_settings_item();
        assert_ne!(app.theme_name(), before);

        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::StartupSplash);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::Metronome);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::GlobalOffset);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::InputOffset);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::MusicVolume);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::HitSoundVolume);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::Keymap);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::Calibration);
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::Back);
        app.activate_settings_item();
        assert_eq!(app.mode(), DemoMode::SongSelect);
    }

    #[test]
    fn settings_flow_toggles_startup_splash() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = app.theme().clone();
        let mut app = DemoApp::new(app.song_choices().to_vec(), Settings::default(), theme);

        app.open_settings();
        app.move_settings_selection(1);
        assert_eq!(app.settings_selection(), SettingsItem::StartupSplash);
        assert!(app.startup_splash_enabled());

        app.activate_settings_item();
        assert!(!app.startup_splash_enabled());
    }

    #[test]
    fn keymap_preset_cycle_avoids_reserved_shell_shortcuts() {
        let app = DemoApp::from_demo_chart().expect("demo app should load");
        let theme = app.theme().clone();
        let mut app = DemoApp::new(app.song_choices().to_vec(), Settings::default(), theme);

        app.open_settings();
        while app.settings_selection() != SettingsItem::Keymap {
            app.move_settings_selection(1);
        }

        app.activate_settings_item();
        assert_eq!(app.keymap().as_string(), "A S D J K L");

        app.activate_settings_item();
        assert_eq!(app.keymap().as_string(), "Z X C , . /");

        app.activate_settings_item();
        assert_eq!(app.keymap().as_string(), "S D F J K L");

        for keymap in ["S D F J K L", "A S D J K L", "Z X C , . /"] {
            for reserved in ["Q", "I", "P", "T", "R", "V", "B"] {
                assert!(
                    !keymap.split_whitespace().any(|key| key == reserved),
                    "{keymap} should not contain reserved shell shortcut {reserved}"
                );
            }
        }
    }
}

fn load_imported_song_choices(import_root: impl AsRef<Path>) -> anyhow::Result<Vec<SongChoice>> {
    let import_root = import_root.as_ref();
    let Ok(imported_catalog) = load_imported_song_catalog(import_root) else {
        return Ok(Vec::new());
    };

    Ok(imported_catalog
        .songs()
        .iter()
        .filter_map(|song| {
            let chart = parse_chart_file(song.chart_path()).ok()?;
            Some(SongChoice::from_loaded_chart(
                song.id.clone(),
                song.title.clone(),
                song.artist.clone(),
                song.chart_name.clone(),
                song.difficulty,
                song.bpm,
                song.mood.clone(),
                song.artwork_path.clone().or_else(|| {
                    discover_imported_artwork(song.chart_path.parent().unwrap_or(import_root))
                        .or_else(|| discover_imported_artwork(Path::new(&song.source_folder)))
                }),
                Some(song.audio_path.clone()),
                chart,
            ))
        })
        .collect())
}

fn discover_imported_artwork(folder: &Path) -> Option<std::path::PathBuf> {
    let mut entries = std::fs::read_dir(folder)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp"
                    )
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries.into_iter().next()
}

fn chart_duration_ms(chart: &Chart) -> i64 {
    chart
        .notes
        .iter()
        .map(|note| match note {
            Note::Tap(note) => note.time_ms as i64,
            Note::Hold(note) => note.end_ms as i64,
        })
        .max()
        .unwrap_or(0)
}

pub struct SearchResultRow<'a> {
    pub source: &'a str,
    pub title: &'a str,
    pub artist: &'a str,
    pub chart_name: &'a str,
    pub mood: &'a str,
    pub bpm: u16,
    pub difficulty: u8,
}

fn search_matches(song: &SongChoice, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    [song.title(), song.artist(), song.chart_name(), song.mood()]
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

pub fn grade_label(accuracy: f32) -> &'static str {
    match accuracy {
        value if value >= 0.985 => "A",
        value if value >= 0.95 => "B",
        value if value >= 0.88 => "C",
        value if value >= 0.75 => "D",
        _ => "E",
    }
}

fn zero_score_summary() -> ScoreSummary {
    ScoreSummary {
        score: 0,
        combo: 0,
        max_combo: 0,
        accuracy: 0.0,
        judgments: Default::default(),
    }
}

fn live_accuracy(summary: ScoreSummary) -> f32 {
    let judged_notes = summary.judgments.perfect
        + summary.judgments.great
        + summary.judgments.good
        + summary.judgments.miss;
    if judged_notes == 0 {
        return 0.0;
    }

    let earned = summary.score as f32;
    let possible = (summary.judgments.perfect + summary.judgments.great + summary.judgments.good)
        .saturating_mul(Judgment::Perfect.points())
        .saturating_add(
            summary
                .judgments
                .miss
                .saturating_mul(Judgment::Perfect.points()),
        ) as f32;

    if possible <= f32::EPSILON {
        0.0
    } else {
        (earned / possible).clamp(0.0, 1.0)
    }
}
