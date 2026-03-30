use code_m::osu::model::OsuHitObject;
use code_m::osu::parser::{parse_osu_file, OsuParseError};
use code_m::osu::OsuMode;

mod osu_parser {
    use super::*;

    #[test]
    fn parses_minimal_osu_mania_6k_beatmap() {
        let beatmap = parse_osu_file("tests/fixtures/osu/valid-6k/map.osu").expect("parse");
        assert_eq!(beatmap.mode, OsuMode::Mania);
        assert_eq!(beatmap.key_count, 6);
        assert_eq!(beatmap.audio_filename, "song.ogg");
    }

    #[test]
    fn rejects_non_mania_mode() {
        let err = parse_osu_file("tests/fixtures/osu/not-mania/map.osu").expect_err("should fail");
        assert!(matches!(err, OsuParseError::UnsupportedMode { .. }));
    }

    #[test]
    fn parses_hit_objects_before_difficulty_and_resolves_lanes_later() {
        let beatmap = code_m::osu::parser::parse_osu_str(
            r#"
osu file format v14

[General]
AudioFilename: song.ogg
Mode: 3

[Metadata]
Title: Order Independent
Artist: Code M
Creator: Code M
Version: Starter

[TimingPoints]
0,500,4,2,1,60,1,0

[HitObjects]
0,192,1000,1,0,0:0:0:0:
427,192,1500,128,0,2000:0:0:0:0:

[Difficulty]
CircleSize: 6
"#,
        )
        .expect("parse");

        assert_eq!(beatmap.key_count, 6);
        assert_eq!(beatmap.timing_points.len(), 1);
        assert_eq!(beatmap.timing_points[0].time_ms, 0);
        assert_eq!(beatmap.timing_points[0].beat_length, 500.0);
        assert!(beatmap.timing_points[0].uninherited);
        assert!(!beatmap.timing_points[0].kiai);

        assert_eq!(beatmap.hit_objects.len(), 2);
        assert!(matches!(
            beatmap.hit_objects[0],
            OsuHitObject::Tap {
                lane: 0,
                time_ms: 1000
            }
        ));
        assert!(matches!(
            beatmap.hit_objects[1],
            OsuHitObject::Hold {
                lane: 5,
                start_time_ms: 1500,
                end_time_ms: 2000
            }
        ));
    }

    #[test]
    fn maps_x_positions_into_all_six_lanes() {
        let beatmap = code_m::osu::parser::parse_osu_str(
            r#"
osu file format v14

[General]
AudioFilename: song.ogg
Mode: 3

[Metadata]
Title: Lane Mapping
Artist: Code M
Creator: Code M
Version: Starter

[Difficulty]
CircleSize: 6

[TimingPoints]
0,500,4,2,1,60,1,0

[HitObjects]
0,192,1000,1,0,0:0:0:0:
86,192,1100,1,0,0:0:0:0:
171,192,1200,1,0,0:0:0:0:
256,192,1300,1,0,0:0:0:0:
342,192,1400,1,0,0:0:0:0:
427,192,1500,1,0,0:0:0:0:
"#,
        )
        .expect("parse");

        let lanes: Vec<u8> = beatmap
            .hit_objects
            .iter()
            .map(|object| match object {
                OsuHitObject::Tap { lane, .. } => *lane,
                OsuHitObject::Hold { lane, .. } => *lane,
            })
            .collect();

        assert_eq!(lanes, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn rejects_hit_objects_with_x_outside_the_playfield() {
        let err = code_m::osu::parser::parse_osu_str(
            r#"
osu file format v14

[General]
AudioFilename: song.ogg
Mode: 3

[Metadata]
Title: Invalid X
Artist: Code M
Creator: Code M
Version: Starter

[Difficulty]
CircleSize: 6

[TimingPoints]
0,500,4,2,1,60,1,0

[HitObjects]
600,192,1000,1,0,0:0:0:0:
"#,
        )
        .expect_err("should fail");

        assert!(matches!(err, OsuParseError::InvalidFormat(_)));
        assert!(
            err.to_string().contains("x coordinate"),
            "unexpected error: {err}"
        );
    }
}
