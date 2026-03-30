use std::collections::HashMap;
use std::f32::consts::TAU;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::clock_sync::{PlaybackClock, PlaybackOffset};
use crate::audio::device::{AudioDeviceConfig, AudioDeviceInfo, AudioOutput};
use crate::chart::Chart;
use crate::runtime::GameTime;

const DEFAULT_DEMO_BPM: f32 = 120.0;
const CLICK_AMPLITUDE: f32 = 0.25;
const CLICK_DURATION_MS: u32 = 14;
const CLICK_FREQUENCY_HZ: f32 = 1_760.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Primed,
    Playing,
}

#[derive(Debug)]
pub struct PlaybackSession {
    device: AudioDeviceInfo,
    clock: PlaybackClock,
    state: PlaybackState,
    control: PlaybackControl,
    #[allow(dead_code)]
    output: Option<AudioOutput>,
}

#[derive(Debug, Clone)]
struct DecodedAudio {
    sample_rate_hz: u32,
    channels: u16,
    samples: Vec<f32>,
}

static AUDIO_FILE_CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<DecodedAudio>>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct PlaybackControl {
    inner: Arc<Mutex<PlaybackControlState>>,
}

#[derive(Debug, Clone, Copy)]
struct PlaybackControlState {
    music_volume: f32,
    hit_sound_volume: f32,
    pending_hit_sounds: u32,
}

impl PlaybackSession {
    pub fn new(device: AudioDeviceInfo, stream_started_at: Instant) -> Self {
        Self {
            device,
            clock: PlaybackClock::new(stream_started_at),
            state: PlaybackState::Primed,
            control: PlaybackControl::new(80, 70),
            output: None,
        }
    }

    pub fn with_offset(
        device: AudioDeviceInfo,
        stream_started_at: Instant,
        offset: PlaybackOffset,
    ) -> Self {
        Self {
            device,
            clock: PlaybackClock::with_offset(stream_started_at, offset),
            state: PlaybackState::Primed,
            control: PlaybackControl::new(80, 70),
            output: None,
        }
    }

    pub fn try_start_metronome(
        chart: &Chart,
        music_volume: u8,
        hit_sound_volume: u8,
    ) -> Result<Self> {
        Self::try_start_metronome_at(
            chart,
            GameTime::from_millis(0),
            music_volume,
            hit_sound_volume,
        )
    }

    pub fn try_start_metronome_at(
        chart: &Chart,
        start_time: GameTime,
        music_volume: u8,
        hit_sound_volume: u8,
    ) -> Result<Self> {
        let timing_point = chart
            .timing
            .first()
            .ok_or_else(|| anyhow!("chart has no timing points"))?;
        let bpm = if timing_point.bpm.is_finite() && timing_point.bpm > 0.0 {
            timing_point.bpm
        } else {
            DEFAULT_DEMO_BPM
        };
        let first_beat_ms = timing_point.start_ms as i64;
        let stream_started_at = Instant::now();
        let control = PlaybackControl::new(music_volume, hit_sound_volume);
        let mut renderer = MetronomeRenderer::new(bpm, first_beat_ms, control.clone());
        let output = AudioOutput::open_default(
            AudioDeviceConfig::default(),
            move |data, channels, sample_rate_hz| {
                renderer.render(data, channels, sample_rate_hz);
            },
        )
        .context("failed to initialize metronome audio output")?;
        let device = output.device().clone();
        output
            .play()
            .context("failed to start metronome audio output")?;

        let mut session = Self {
            device,
            clock: PlaybackClock::with_offset(
                stream_started_at,
                PlaybackOffset::from_millis(start_time.as_millis()),
            ),
            state: PlaybackState::Primed,
            control,
            output: Some(output),
        };
        session.start();
        Ok(session)
    }

    pub fn try_start_audio_file_at(
        path: &Path,
        start_time: GameTime,
        music_volume: u8,
        hit_sound_volume: u8,
    ) -> Result<Self> {
        let decoded = load_cached_audio_file(path)
            .with_context(|| format!("failed to decode audio file {}", path.display()))?;
        let stream_started_at = Instant::now();
        let control = PlaybackControl::new(music_volume, hit_sound_volume);
        let mut renderer = AudioFileRenderer::new(decoded, start_time, control.clone());
        let output = AudioOutput::open_default(
            AudioDeviceConfig::default(),
            move |data, channels, sample_rate_hz| {
                renderer.render(data, channels, sample_rate_hz);
            },
        )
        .context("failed to initialize audio file output")?;
        let device = output.device().clone();
        output.play().context("failed to start audio file output")?;

        let mut session = Self {
            device,
            clock: PlaybackClock::with_offset(
                stream_started_at,
                PlaybackOffset::from_millis(start_time.as_millis()),
            ),
            state: PlaybackState::Primed,
            control,
            output: Some(output),
        };
        session.start();
        Ok(session)
    }

    pub fn device(&self) -> &AudioDeviceInfo {
        &self.device
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn start(&mut self) {
        self.state = PlaybackState::Playing;
    }

    pub fn playback_time(&self, observed_at: Instant) -> GameTime {
        self.clock.playback_time(observed_at)
    }

    pub fn clock(&self) -> PlaybackClock {
        self.clock
    }

    pub fn trigger_hit_sound(&self) {
        self.control.trigger_hit_sound();
    }

    pub fn set_mix_levels(&self, music_volume: u8, hit_sound_volume: u8) {
        self.control.set_music_volume(music_volume);
        self.control.set_hit_sound_volume(hit_sound_volume);
    }
}

impl PlaybackControl {
    fn new(music_volume: u8, hit_sound_volume: u8) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlaybackControlState {
                music_volume: normalize_volume(music_volume),
                hit_sound_volume: normalize_volume(hit_sound_volume),
                pending_hit_sounds: 0,
            })),
        }
    }

    fn trigger_hit_sound(&self) {
        if let Ok(mut state) = self.inner.lock() {
            state.pending_hit_sounds = state.pending_hit_sounds.saturating_add(1);
        }
    }

    fn set_music_volume(&self, volume: u8) {
        if let Ok(mut state) = self.inner.lock() {
            state.music_volume = normalize_volume(volume);
        }
    }

    fn set_hit_sound_volume(&self, volume: u8) {
        if let Ok(mut state) = self.inner.lock() {
            state.hit_sound_volume = normalize_volume(volume);
        }
    }

    fn consume(&self) -> PlaybackControlState {
        if let Ok(mut state) = self.inner.lock() {
            let snapshot = *state;
            state.pending_hit_sounds = 0;
            snapshot
        } else {
            PlaybackControlState {
                music_volume: 1.0,
                hit_sound_volume: 0.7,
                pending_hit_sounds: 0,
            }
        }
    }
}

#[derive(Debug)]
struct AudioFileRenderer {
    audio: Arc<DecodedAudio>,
    start_frame: u64,
    rendered_frames: u64,
    control: PlaybackControl,
    hit_sound: HitSoundState,
}

impl AudioFileRenderer {
    fn new(audio: Arc<DecodedAudio>, start_time: GameTime, control: PlaybackControl) -> Self {
        let start_frame =
            ((start_time.as_millis().max(0) as u64) * audio.sample_rate_hz as u64) / 1_000;
        Self {
            audio,
            start_frame,
            rendered_frames: 0,
            control,
            hit_sound: HitSoundState::default(),
        }
    }

    fn render(&mut self, data: &mut [f32], output_channels: u16, sample_rate_hz: u32) {
        if output_channels == 0 || sample_rate_hz == 0 {
            data.fill(0.0);
            return;
        }

        let channels = output_channels as usize;
        let input_channels = self.audio.channels as usize;
        let input_rate = self.audio.sample_rate_hz as u64;
        let output_rate = sample_rate_hz as u64;
        let control = self.control.consume();
        self.hit_sound.trigger_many(control.pending_hit_sounds);

        for (frame_idx, frame) in data.chunks_mut(channels).enumerate() {
            let output_frame = self.rendered_frames + frame_idx as u64;
            let source_frame = self.start_frame + ((output_frame * input_rate) / output_rate);
            let base = source_frame as usize * input_channels;

            let hit = self.hit_sound.next_sample(sample_rate_hz) * control.hit_sound_volume;
            if base + input_channels <= self.audio.samples.len() {
                for (channel_idx, out) in frame.iter_mut().enumerate() {
                    let sample = if input_channels == 1 {
                        self.audio.samples[base]
                    } else {
                        let source_channel = channel_idx.min(input_channels - 1);
                        self.audio.samples[base + source_channel]
                    };
                    *out = (sample * control.music_volume + hit).clamp(-1.0, 1.0);
                }
            } else {
                frame.fill(hit);
            }
        }

        self.rendered_frames = self
            .rendered_frames
            .saturating_add((data.len() / channels) as u64);
    }
}

fn load_audio_file(path: &Path) -> Result<DecodedAudio> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
    {
        return load_wav_file(path);
    }

    load_symphonia_file(path)
}

fn load_cached_audio_file(path: &Path) -> Result<Arc<DecodedAudio>> {
    let cache_key = path.to_path_buf();
    let cache = AUDIO_FILE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(guard) = cache.lock() {
        if let Some(decoded) = guard.get(&cache_key) {
            return Ok(Arc::clone(decoded));
        }
    }

    let decoded = Arc::new(load_audio_file(path)?);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(cache_key, Arc::clone(&decoded));
    }
    Ok(decoded)
}

fn load_wav_file(path: &Path) -> Result<DecodedAudio> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to open wav file {}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1);
    let sample_rate_hz = spec.sample_rate;

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|sample| sample.context("failed to read f32 wav sample"))
            .collect::<Result<Vec<_>>>()?,
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| {
                sample
                    .map(|value| value as f32 / i16::MAX as f32)
                    .context("failed to read i16 wav sample")
            })
            .collect::<Result<Vec<_>>>()?,
        (hound::SampleFormat::Int, 24 | 32) => {
            let peak = ((1i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| (value as f32 / peak).clamp(-1.0, 1.0))
                        .context("failed to read i32 wav sample")
                })
                .collect::<Result<Vec<_>>>()?
        }
        _ => {
            return Err(anyhow!(
                "unsupported wav format: {:?} {} bits",
                spec.sample_format,
                spec.bits_per_sample
            ))
        }
    };

    Ok(DecodedAudio {
        sample_rate_hz,
        channels,
        samples,
    })
}

fn load_symphonia_file(path: &Path) -> Result<DecodedAudio> {
    let file = File::open(path)
        .with_context(|| format!("failed to open audio file {}", path.display()))?;
    let media_source = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media_source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("failed to probe audio file {}", path.display()))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("audio file has no default track"))?;

    let sample_rate_hz = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("audio file is missing sample rate metadata"))?;
    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| anyhow!("audio file is missing channel metadata"))?
        .count() as u16;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("failed to create audio decoder")?;

    let mut samples = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                return Err(anyhow!("audio decoder reset is not supported"));
            }
            Err(error) => return Err(error).context("failed to read audio packet"),
        };

        let decoded = decoder
            .decode(&packet)
            .context("failed to decode audio packet")?;
        let spec = *decoded.spec();
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buffer.samples());
    }

    Ok(DecodedAudio {
        sample_rate_hz,
        channels: channels.max(1),
        samples,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MetronomeConfig {
    sample_rate_hz: u32,
    bpm: f32,
    output_channels: u16,
    first_beat_ms: u32,
}

impl MetronomeConfig {
    fn new(sample_rate_hz: u32, bpm: f32, output_channels: u16) -> Self {
        Self {
            sample_rate_hz,
            bpm,
            output_channels,
            first_beat_ms: 0,
        }
    }

    fn with_first_beat_ms(mut self, first_beat_ms: u32) -> Self {
        self.first_beat_ms = first_beat_ms;
        self
    }

    fn beat_interval_samples(self) -> u64 {
        (((60.0 / self.bpm) * self.sample_rate_hz as f32).round() as u64).max(1)
    }

    fn click_duration_samples(self) -> u32 {
        ((self.sample_rate_hz as u64 * CLICK_DURATION_MS as u64) / 1_000)
            .max(1)
            .min(u32::MAX as u64) as u32
    }

    fn first_beat_sample(self) -> u64 {
        ((self.first_beat_ms as u64) * self.sample_rate_hz as u64) / 1_000
    }

    fn phase_step_radians(self) -> f32 {
        (TAU * CLICK_FREQUENCY_HZ) / self.sample_rate_hz as f32
    }
}

#[derive(Debug)]
struct MetronomeRenderer {
    bpm: f32,
    first_beat_ms: i64,
    control: PlaybackControl,
    config: Option<MetronomeConfig>,
    rendered_frames: u64,
    next_beat_frame: u64,
    click_frames_remaining: u32,
    click_phase_radians: f32,
    hit_sound: HitSoundState,
}

impl MetronomeRenderer {
    fn new(bpm: f32, first_beat_ms: i64, control: PlaybackControl) -> Self {
        Self {
            bpm,
            first_beat_ms,
            control,
            config: None,
            rendered_frames: 0,
            next_beat_frame: 0,
            click_frames_remaining: 0,
            click_phase_radians: 0.0,
            hit_sound: HitSoundState::default(),
        }
    }

    fn render(&mut self, data: &mut [f32], output_channels: u16, sample_rate_hz: u32) {
        if output_channels == 0 || sample_rate_hz == 0 {
            data.fill(0.0);
            return;
        }

        let expected = MetronomeConfig::new(sample_rate_hz, self.bpm, output_channels)
            .with_first_beat_ms(self.first_beat_ms.max(0) as u32);
        if self.config != Some(expected) {
            self.configure(expected);
        }

        let config = self.config.expect("metronome config should be initialized");
        let channels = config.output_channels as usize;
        let beat_interval_frames = config.beat_interval_samples();
        let control = self.control.consume();
        self.hit_sound.trigger_many(control.pending_hit_sounds);

        for frame in data.chunks_mut(channels) {
            while self.rendered_frames >= self.next_beat_frame {
                self.click_frames_remaining = config.click_duration_samples();
                self.click_phase_radians = 0.0;
                self.next_beat_frame = self.next_beat_frame.saturating_add(beat_interval_frames);
            }

            let sample = (self.next_click_sample(config) * control.music_volume)
                + self.hit_sound.next_sample(config.sample_rate_hz) * control.hit_sound_volume;
            frame.fill(sample);
            self.rendered_frames = self.rendered_frames.saturating_add(1);
        }
    }

    fn configure(&mut self, config: MetronomeConfig) {
        self.config = Some(config);
        self.rendered_frames = 0;
        self.next_beat_frame = config.first_beat_sample();
        self.click_frames_remaining = 0;
        self.click_phase_radians = 0.0;
    }

    fn next_click_sample(&mut self, config: MetronomeConfig) -> f32 {
        if self.click_frames_remaining == 0 {
            return 0.0;
        }

        let total = config.click_duration_samples() as f32;
        let progress = 1.0 - (self.click_frames_remaining as f32 / total);
        let envelope = (1.0 - progress).powi(2);
        let sample = self.click_phase_radians.sin() * envelope * CLICK_AMPLITUDE;

        self.click_phase_radians += config.phase_step_radians();
        self.click_frames_remaining -= 1;
        sample
    }
}

#[derive(Debug, Default)]
struct HitSoundState {
    pending_triggers: u32,
    frames_remaining: u32,
    phase_radians: f32,
}

impl HitSoundState {
    fn trigger_many(&mut self, count: u32) {
        self.pending_triggers = self.pending_triggers.saturating_add(count);
    }

    fn next_sample(&mut self, sample_rate_hz: u32) -> f32 {
        let click_frames = ((sample_rate_hz as u64 * 18) / 1_000).max(1) as u32;
        if self.frames_remaining == 0 && self.pending_triggers > 0 {
            self.pending_triggers -= 1;
            self.frames_remaining = click_frames;
            self.phase_radians = 0.0;
        }

        if self.frames_remaining == 0 {
            return 0.0;
        }

        let progress = 1.0 - (self.frames_remaining as f32 / click_frames as f32);
        let envelope = (1.0 - progress).powi(3);
        let sample = self.phase_radians.sin() * envelope * 0.32;
        self.phase_radians += (TAU * 2_650.0) / sample_rate_hz as f32;
        self.frames_remaining -= 1;
        sample
    }
}

fn normalize_volume(volume: u8) -> f32 {
    f32::from(volume.min(100)) / 100.0
}

#[cfg(test)]
mod tests {
    use super::{
        load_audio_file, load_cached_audio_file, load_wav_file, MetronomeConfig, MetronomeRenderer,
        PlaybackControl,
    };
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn metronome_uses_first_timing_point_bpm_to_compute_beat_samples() {
        let config = MetronomeConfig::new(48_000, 150.0, 2);

        assert_eq!(config.beat_interval_samples(), 19_200);
    }

    #[test]
    fn metronome_retriggers_clicks_on_beat_boundaries() {
        let mut renderer = MetronomeRenderer::new(60.0, 0, PlaybackControl::new(80, 70));
        let mut data = [0.0; 2_100];

        renderer.render(&mut data, 1, 1_000);

        assert_ne!(data[1], 0.0);
        assert_eq!(data[999], 0.0);
        assert_ne!(data[1001], 0.0);
        assert_ne!(data[2001], 0.0);
    }

    #[test]
    fn wav_loader_reads_basic_mono_pcm_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("code_m_test_{unique}.wav"));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("wav writer");
        for sample in [0i16, i16::MAX / 2, 0, -(i16::MAX / 2)] {
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize wav");

        let audio = load_wav_file(&path).expect("wav should load");

        assert_eq!(audio.channels, 1);
        assert_eq!(audio.sample_rate_hz, 8_000);
        assert_eq!(audio.samples.len(), 4);
        assert!(audio.samples[1] > 0.4);
        assert!(audio.samples[3] < -0.4);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn generic_audio_loader_reads_wav_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("code_m_test_generic_{unique}.wav"));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("wav writer");
        for sample in [0i16, i16::MAX / 2, 0, -(i16::MAX / 2)] {
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize wav");

        let audio = load_audio_file(&path).expect("generic loader should read wav");

        assert_eq!(audio.channels, 1);
        assert_eq!(audio.sample_rate_hz, 8_000);
        assert_eq!(audio.samples.len(), 4);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn cached_audio_loader_reuses_decoded_audio_for_the_same_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("code_m_test_cached_{unique}.wav"));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("wav writer");
        for sample in [0i16, i16::MAX / 2, 0, -(i16::MAX / 2)] {
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize wav");

        let first = load_cached_audio_file(&path).expect("first cached load");
        let second = load_cached_audio_file(&path).expect("second cached load");

        assert!(Arc::ptr_eq(&first, &second));

        let _ = fs::remove_file(path);
    }
}
