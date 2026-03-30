use std::fmt;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    name: String,
    sample_rate_hz: u32,
    output_channels: u16,
}

impl AudioDeviceInfo {
    pub fn new(name: impl Into<String>, sample_rate_hz: u32, output_channels: u16) -> Self {
        Self {
            name: name.into(),
            sample_rate_hz,
            output_channels,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn output_channels(&self) -> u16 {
        self.output_channels
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioDeviceConfig {
    sample_rate_hz: u32,
    output_channels: u16,
}

impl AudioDeviceConfig {
    pub const fn new(sample_rate_hz: u32, output_channels: u16) -> Self {
        Self {
            sample_rate_hz,
            output_channels,
        }
    }

    pub const fn sample_rate_hz(self) -> u32 {
        self.sample_rate_hz
    }

    pub const fn output_channels(self) -> u16 {
        self.output_channels
    }
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self::new(48_000, 2)
    }
}

pub struct AudioOutput {
    info: AudioDeviceInfo,
    config: AudioDeviceConfig,
    stream: Stream,
}

impl AudioOutput {
    pub fn open_default<F>(_requested: AudioDeviceConfig, render: F) -> Result<Self>
    where
        F: FnMut(&mut [f32], u16, u32) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default audio output device available")?;
        let supported_config = device
            .default_output_config()
            .context("failed to query default audio output config")?;
        let stream_config = supported_config.config();
        let config = AudioDeviceConfig::new(stream_config.sample_rate.0, stream_config.channels);
        let info = AudioDeviceInfo::new(
            device
                .name()
                .unwrap_or_else(|_| "default output".to_string()),
            config.sample_rate_hz(),
            config.output_channels(),
        );
        let stream = build_output_stream(
            &device,
            supported_config.sample_format(),
            stream_config,
            render,
        )?;

        Ok(Self {
            info,
            config,
            stream,
        })
    }

    pub fn play(&self) -> Result<()> {
        self.stream
            .play()
            .context("failed to start audio output stream")
    }

    pub fn device(&self) -> &AudioDeviceInfo {
        &self.info
    }

    pub const fn config(&self) -> AudioDeviceConfig {
        self.config
    }
}

impl fmt::Debug for AudioOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AudioOutput")
            .field("info", &self.info)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

fn build_output_stream<F>(
    device: &cpal::Device,
    sample_format: SampleFormat,
    stream_config: StreamConfig,
    render: F,
) -> Result<Stream>
where
    F: FnMut(&mut [f32], u16, u32) + Send + 'static,
{
    match sample_format {
        SampleFormat::F32 => build_f32_stream(device, &stream_config, render),
        SampleFormat::I16 => build_i16_stream(device, &stream_config, render),
        SampleFormat::U16 => build_u16_stream(device, &stream_config, render),
        _ => anyhow::bail!("unsupported output sample format"),
    }
}

fn build_f32_stream<F>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut render: F,
) -> Result<Stream>
where
    F: FnMut(&mut [f32], u16, u32) + Send + 'static,
{
    let channels = config.channels;
    let sample_rate_hz = config.sample_rate.0;

    device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| render(data, channels, sample_rate_hz),
            |_| {},
            None,
        )
        .context("failed to build f32 audio output stream")
}

fn build_i16_stream<F>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut render: F,
) -> Result<Stream>
where
    F: FnMut(&mut [f32], u16, u32) + Send + 'static,
{
    let channels = config.channels;
    let sample_rate_hz = config.sample_rate.0;
    let mut scratch = Vec::<f32>::new();

    device
        .build_output_stream(
            config,
            move |data: &mut [i16], _| {
                scratch.resize(data.len(), 0.0);
                render(&mut scratch, channels, sample_rate_hz);

                for (output, sample) in data.iter_mut().zip(scratch.iter().copied()) {
                    *output = sample_to_i16(sample);
                }
            },
            |_| {},
            None,
        )
        .context("failed to build i16 audio output stream")
}

fn build_u16_stream<F>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut render: F,
) -> Result<Stream>
where
    F: FnMut(&mut [f32], u16, u32) + Send + 'static,
{
    let channels = config.channels;
    let sample_rate_hz = config.sample_rate.0;
    let mut scratch = Vec::<f32>::new();

    device
        .build_output_stream(
            config,
            move |data: &mut [u16], _| {
                scratch.resize(data.len(), 0.0);
                render(&mut scratch, channels, sample_rate_hz);

                for (output, sample) in data.iter_mut().zip(scratch.iter().copied()) {
                    *output = sample_to_u16(sample);
                }
            },
            |_| {},
            None,
        )
        .context("failed to build u16 audio output stream")
}

fn sample_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn sample_to_u16(sample: f32) -> u16 {
    (((sample.clamp(-1.0, 1.0) * 0.5) + 0.5) * u16::MAX as f32) as u16
}
