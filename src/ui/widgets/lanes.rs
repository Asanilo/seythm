pub fn lane_fill_glyph(active: bool) -> char {
    if active {
        '#'
    } else {
        '.'
    }
}

#[cfg(test)]
mod tests {
    use super::lane_fill_glyph;

    #[test]
    fn theme_lane_glyphs_are_simple_and_high_contrast() {
        assert_eq!(lane_fill_glyph(false), '.');
        assert_eq!(lane_fill_glyph(true), '#');
    }
}
