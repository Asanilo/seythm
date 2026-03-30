#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayfieldLayout {
    lane_count: u16,
    lane_width: u16,
    lane_gap: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudAnchor {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellDensity {
    Compact,
    Regular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenClass {
    pub density: ShellDensity,
    pub stack_side_panels: bool,
    pub compress_lists: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowseColumns {
    pub left: u16,
    pub center: u16,
    pub right: u16,
}

impl PlayfieldLayout {
    pub const fn new(lane_count: u16, lane_width: u16, lane_gap: u16) -> Self {
        Self {
            lane_count,
            lane_width,
            lane_gap,
        }
    }

    pub const fn playfield_width(self) -> u16 {
        if self.lane_count == 0 {
            return 0;
        }

        self.lane_count * self.lane_width + self.lane_gap * (self.lane_count - 1)
    }

    pub fn centered_left(self, terminal_width: u16) -> u16 {
        terminal_width.saturating_sub(self.playfield_width()) / 2
    }

    pub const fn lane_left(self, lane_index: u16) -> u16 {
        lane_index * (self.lane_width + self.lane_gap)
    }

    pub fn inner_origin(self, container_width: u16) -> u16 {
        self.centered_left(container_width)
    }

    pub fn hud_anchor(self, terminal_width: u16, hud_height: u16) -> HudAnchor {
        HudAnchor {
            x: self.centered_left(terminal_width),
            y: 0,
            width: self.playfield_width(),
            height: hud_height,
        }
    }
}

pub fn classify_screen(width: u16, height: u16) -> ScreenClass {
    let density = if width < 110 || height < 30 {
        ShellDensity::Compact
    } else {
        ShellDensity::Regular
    };

    ScreenClass {
        density,
        stack_side_panels: width < 120,
        compress_lists: height < 32,
    }
}

pub const fn shell_header_height(screen: ScreenClass) -> u16 {
    match screen.density {
        ShellDensity::Compact => 3,
        ShellDensity::Regular => 4,
    }
}

pub const fn shell_footer_height(screen: ScreenClass) -> u16 {
    match screen.density {
        ShellDensity::Compact => 2,
        ShellDensity::Regular => 3,
    }
}

pub fn browse_columns(body_width: u16, screen: ScreenClass) -> BrowseColumns {
    let left_min = if matches!(screen.density, ShellDensity::Compact) {
        18
    } else {
        20
    };
    let right_min = if matches!(screen.density, ShellDensity::Compact) {
        28
    } else {
        32
    };
    let center_min = 66;
    let extra = body_width.saturating_sub(left_min + center_min + right_min);
    let left_extra = extra / 2;
    let right_extra = extra - left_extra;

    BrowseColumns {
        left: left_min + left_extra,
        center: center_min,
        right: right_min + right_extra,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        browse_columns, classify_screen, shell_footer_height, shell_header_height,
        BrowseColumns, PlayfieldLayout, ScreenClass, ShellDensity,
    };

    #[test]
    fn theme_fixed_width_playfield_is_centered_without_stretching() {
        let layout = PlayfieldLayout::new(6, 7, 1);

        assert_eq!(layout.playfield_width(), 47);
        assert_eq!(layout.centered_left(120), 36);
        assert_eq!(layout.centered_left(80), 16);
    }

    #[test]
    fn theme_lane_spacing_is_stable_across_all_six_lanes() {
        let layout = PlayfieldLayout::new(6, 7, 1);

        assert_eq!(layout.lane_left(0), 0);
        assert_eq!(layout.lane_left(1), 8);
        assert_eq!(layout.lane_left(5), 40);
    }

    #[test]
    fn theme_playfield_recenters_inside_wider_container() {
        let layout = PlayfieldLayout::new(6, 7, 1);

        assert_eq!(layout.inner_origin(47), 0);
        assert_eq!(layout.inner_origin(63), 8);
        assert_eq!(layout.inner_origin(80), 16);
    }

    #[test]
    fn theme_hud_anchors_to_the_playfield_without_changing_lane_width() {
        let layout = PlayfieldLayout::new(6, 7, 1);
        let hud = layout.hud_anchor(120, 4);

        assert_eq!(hud.x, 36);
        assert_eq!(hud.y, 0);
        assert_eq!(hud.width, 47);
        assert_eq!(hud.height, 4);
    }

    #[test]
    fn classifies_large_terminal_as_regular_shell() {
        assert_eq!(
            classify_screen(160, 42),
            ScreenClass {
                density: ShellDensity::Regular,
                stack_side_panels: false,
                compress_lists: false,
            }
        );
    }

    #[test]
    fn classifies_smaller_terminal_as_compact_and_stacked() {
        assert_eq!(
            classify_screen(100, 26),
            ScreenClass {
                density: ShellDensity::Compact,
                stack_side_panels: true,
                compress_lists: true,
            }
        );
    }

    #[test]
    fn classify_screen_marks_narrow_shell_for_stacked_rails() {
        let shell = classify_screen(100, 28);

        assert!(shell.stack_side_panels);
        assert_eq!(shell.density, ShellDensity::Compact);
        assert!(shell.compress_lists);
    }

    #[test]
    fn shell_header_and_footer_measurements_follow_density() {
        let compact = classify_screen(100, 28);
        let regular = classify_screen(160, 42);

        assert_eq!(shell_header_height(compact), 3);
        assert_eq!(shell_footer_height(compact), 2);
        assert_eq!(shell_header_height(regular), 4);
        assert_eq!(shell_footer_height(regular), 3);
    }

    #[test]
    fn browse_columns_keep_the_center_stable_and_expand_both_sidebars() {
        let regular = classify_screen(160, 42);
        let compact = classify_screen(120, 28);

        assert_eq!(
            browse_columns(118, regular),
            BrowseColumns {
                left: 20,
                center: 66,
                right: 32,
            }
        );
        assert_eq!(
            browse_columns(158, regular),
            BrowseColumns {
                left: 40,
                center: 66,
                right: 52,
            }
        );
        assert_eq!(
            browse_columns(118, compact),
            BrowseColumns {
                left: 21,
                center: 66,
                right: 31,
            }
        );
    }
}
