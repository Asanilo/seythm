use crate::osu::model::{OsuBeatmap, OsuHitObject, OsuMetadata, OsuMode, OsuTimingPoint};
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum OsuParseError {
    #[error("failed to parse osu file: {0}")]
    InvalidFormat(String),
    #[error("failed to read osu file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported osu mode {mode}; expected mania")]
    UnsupportedMode { mode: i32 },
}

#[derive(Debug, Default)]
struct BeatmapBuilder {
    mode: Option<i32>,
    audio_filename: Option<String>,
    metadata: OsuMetadata,
    key_count: Option<u8>,
    timing_points: Vec<OsuTimingPoint>,
    hit_objects: Vec<RawHitObject>,
}

#[derive(Debug, Clone)]
struct RawHitObject {
    x: i32,
    time_ms: i32,
    object_type: i32,
    hold_end_time_ms: Option<i32>,
}

pub fn parse_osu_file(path: impl AsRef<Path>) -> Result<OsuBeatmap, OsuParseError> {
    let path_ref = path.as_ref();
    let contents = fs::read_to_string(path_ref).map_err(|source| OsuParseError::Io {
        path: path_ref.display().to_string(),
        source,
    })?;
    parse_osu_str(&contents)
}

pub fn parse_osu_str(input: &str) -> Result<OsuBeatmap, OsuParseError> {
    let mut builder = BeatmapBuilder::default();
    let mut section = String::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(section_name) = parse_section_header(line) {
            section = section_name;
            continue;
        }

        match section.as_str() {
            "general" => parse_general_line(line, &mut builder)?,
            "metadata" => parse_metadata_line(line, &mut builder)?,
            "difficulty" => parse_difficulty_line(line, &mut builder)?,
            "timingpoints" => {
                builder
                    .timing_points
                    .push(parse_timing_point(line).map_err(|reason| {
                        OsuParseError::InvalidFormat(format!(
                            "line {}: invalid timing point: {reason}",
                            line_index + 1
                        ))
                    })?)
            }
            "hitobjects" => {
                builder.hit_objects.push(parse_hit_object(line)?);
            }
            _ => {}
        }
    }

    finalize(builder)
}

fn finalize(builder: BeatmapBuilder) -> Result<OsuBeatmap, OsuParseError> {
    let mode = builder
        .mode
        .ok_or_else(|| OsuParseError::InvalidFormat("missing General Mode".to_string()))?;

    if mode != 3 {
        return Err(OsuParseError::UnsupportedMode { mode });
    }

    let key_count = builder
        .key_count
        .ok_or_else(|| OsuParseError::InvalidFormat("missing Difficulty CircleSize".to_string()))?;

    let audio_filename = builder
        .audio_filename
        .ok_or_else(|| OsuParseError::InvalidFormat("missing General AudioFilename".to_string()))?;

    if audio_filename.trim().is_empty() {
        return Err(OsuParseError::InvalidFormat(
            "General AudioFilename cannot be empty".to_string(),
        ));
    }

    Ok(OsuBeatmap {
        mode: OsuMode::Mania,
        key_count,
        audio_filename,
        metadata: builder.metadata,
        timing_points: builder.timing_points,
        hit_objects: builder
            .hit_objects
            .into_iter()
            .map(|raw| resolve_hit_object(raw, key_count))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_section_header(line: &str) -> Option<String> {
    if line.starts_with('[') && line.ends_with(']') {
        Some(line[1..line.len() - 1].to_ascii_lowercase())
    } else {
        None
    }
}

fn parse_general_line(line: &str, builder: &mut BeatmapBuilder) -> Result<(), OsuParseError> {
    let (key, value) = split_key_value(line)?;
    match key.to_ascii_lowercase().as_str() {
        "audiofilename" => builder.audio_filename = Some(value.to_string()),
        "mode" => {
            builder.mode = Some(parse_i32(value).map_err(|reason| {
                OsuParseError::InvalidFormat(format!("invalid General Mode: {reason}"))
            })?)
        }
        _ => {}
    }
    Ok(())
}

fn parse_metadata_line(line: &str, builder: &mut BeatmapBuilder) -> Result<(), OsuParseError> {
    let (key, value) = split_key_value(line)?;
    match key.to_ascii_lowercase().as_str() {
        "title" => builder.metadata.title = Some(value.to_string()),
        "titleunicode" => builder.metadata.title_unicode = Some(value.to_string()),
        "artist" => builder.metadata.artist = Some(value.to_string()),
        "artistunicode" => builder.metadata.artist_unicode = Some(value.to_string()),
        "creator" => builder.metadata.creator = Some(value.to_string()),
        "version" => builder.metadata.version = Some(value.to_string()),
        _ => {}
    }
    Ok(())
}

fn parse_difficulty_line(line: &str, builder: &mut BeatmapBuilder) -> Result<(), OsuParseError> {
    let (key, value) = split_key_value(line)?;
    match key.to_ascii_lowercase().as_str() {
        "circlesize" => {
            let key_count = parse_u8(value).map_err(|reason| {
                OsuParseError::InvalidFormat(format!("invalid Difficulty CircleSize: {reason}"))
            })?;
            builder.key_count = Some(key_count);
        }
        _ => {}
    }
    Ok(())
}

fn parse_timing_point(line: &str) -> Result<OsuTimingPoint, String> {
    let parts = split_csv(line);
    if parts.len() < 8 {
        return Err("expected at least 8 comma-separated fields".to_string());
    }

    let time_ms = parse_i32(parts[0])?;
    let beat_length = parse_f64(parts[1])?;
    let meter = parse_u8(parts[2])?;
    let uninherited = parse_i32(parts[6])? != 0;
    let effects = parse_i32(parts[7])?;
    let kiai = effects & 1 != 0;

    Ok(OsuTimingPoint {
        time_ms,
        beat_length,
        meter,
        uninherited,
        kiai,
    })
}

fn parse_hit_object(line: &str) -> Result<RawHitObject, OsuParseError> {
    let parts = split_csv(line);
    if parts.len() < 5 {
        return Err(OsuParseError::InvalidFormat(
            "invalid hit object: expected at least 5 comma-separated fields".to_string(),
        ));
    }

    let x = parse_i32(parts[0]).map_err(|reason| {
        OsuParseError::InvalidFormat(format!("invalid hit object x coordinate: {reason}"))
    })?;
    let time_ms = parse_i32(parts[2]).map_err(|reason| {
        OsuParseError::InvalidFormat(format!("invalid hit object time: {reason}"))
    })?;
    let object_type = parse_i32(parts[3]).map_err(|reason| {
        OsuParseError::InvalidFormat(format!("invalid hit object type: {reason}"))
    })?;

    let hold_end_time_ms = if object_type & 128 != 0 {
        let params = parts.get(5).ok_or_else(|| {
            OsuParseError::InvalidFormat("hold note missing end time".to_string())
        })?;
        let end_time_str = params.split(':').next().unwrap_or("");
        Some(parse_i32(end_time_str).map_err(|reason| {
            OsuParseError::InvalidFormat(format!("invalid hold note end time: {reason}"))
        })?)
    } else {
        None
    };

    Ok(RawHitObject {
        x,
        time_ms,
        object_type,
        hold_end_time_ms,
    })
}

fn resolve_hit_object(raw: RawHitObject, key_count: u8) -> Result<OsuHitObject, OsuParseError> {
    let lane = lane_from_x(raw.x, key_count)?;

    if raw.object_type & 128 != 0 {
        let end_time_ms = raw.hold_end_time_ms.ok_or_else(|| {
            OsuParseError::InvalidFormat("hold note missing end time".to_string())
        })?;

        if end_time_ms <= raw.time_ms {
            return Err(OsuParseError::InvalidFormat(
                "hold note end time must be greater than start time".to_string(),
            ));
        }

        return Ok(OsuHitObject::Hold {
            lane,
            start_time_ms: raw.time_ms,
            end_time_ms,
        });
    }

    if raw.object_type & 1 != 0 {
        return Ok(OsuHitObject::Tap {
            lane,
            time_ms: raw.time_ms,
        });
    }

    Err(OsuParseError::InvalidFormat(
        "unsupported hit object type for mania import".to_string(),
    ))
}

fn lane_from_x(x: i32, key_count: u8) -> Result<u8, OsuParseError> {
    if key_count == 0 {
        return Err(OsuParseError::InvalidFormat(
            "Difficulty CircleSize must be greater than zero".to_string(),
        ));
    }

    if !(0..=511).contains(&x) {
        return Err(OsuParseError::InvalidFormat(format!(
            "hit object x coordinate out of range: {x}"
        )));
    }

    let lane = (x as u32 * key_count as u32) / 512;
    Ok(lane.min(key_count.saturating_sub(1) as u32) as u8)
}

fn split_key_value(line: &str) -> Result<(&str, &str), OsuParseError> {
    let (key, value) = line
        .split_once(':')
        .ok_or_else(|| OsuParseError::InvalidFormat(format!("invalid key/value line: {line}")))?;
    Ok((key.trim(), value.trim()))
}

fn split_csv(line: &str) -> Vec<&str> {
    line.split(',').map(str::trim).collect()
}

fn parse_i32(value: &str) -> Result<i32, String> {
    value.parse::<i32>().map_err(|error| error.to_string())
}

fn parse_u8(value: &str) -> Result<u8, String> {
    value.parse::<u8>().map_err(|error| error.to_string())
}

fn parse_f64(value: &str) -> Result<f64, String> {
    value.parse::<f64>().map_err(|error| error.to_string())
}
