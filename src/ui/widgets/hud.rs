use crate::gameplay::judgment::Judgment;

pub fn format_judgment_label(judgment: Judgment) -> &'static str {
    match judgment {
        Judgment::Perfect => "PERFECT",
        Judgment::Great => "GREAT",
        Judgment::Good => "GOOD",
        Judgment::Miss => "MISS",
    }
}

#[cfg(test)]
mod tests {
    use crate::gameplay::judgment::Judgment;

    use super::format_judgment_label;

    #[test]
    fn theme_judgment_labels_are_short_and_consistent() {
        assert_eq!(format_judgment_label(Judgment::Perfect), "PERFECT");
        assert_eq!(format_judgment_label(Judgment::Great), "GREAT");
        assert_eq!(format_judgment_label(Judgment::Good), "GOOD");
        assert_eq!(format_judgment_label(Judgment::Miss), "MISS");
    }
}
