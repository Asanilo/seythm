#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    Perfect,
    Great,
    Good,
    Miss,
}

impl Judgment {
    pub const fn points(self) -> u32 {
        match self {
            Judgment::Perfect => 1000,
            Judgment::Great => 800,
            Judgment::Good => 500,
            Judgment::Miss => 0,
        }
    }

    pub const fn is_hit(self) -> bool {
        !matches!(self, Judgment::Miss)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgmentWindows {
    perfect_ms: i64,
    great_ms: i64,
    good_ms: i64,
}

impl JudgmentWindows {
    pub const fn new(perfect_ms: i64, great_ms: i64, good_ms: i64) -> Self {
        Self {
            perfect_ms,
            great_ms,
            good_ms,
        }
    }

    pub const fn perfect_ms(self) -> i64 {
        self.perfect_ms
    }

    pub const fn great_ms(self) -> i64 {
        self.great_ms
    }

    pub const fn good_ms(self) -> i64 {
        self.good_ms
    }

    pub fn classify_offset(self, offset_ms: i64) -> Judgment {
        let distance = offset_ms.abs();

        if distance <= self.perfect_ms {
            Judgment::Perfect
        } else if distance <= self.great_ms {
            Judgment::Great
        } else if distance <= self.good_ms {
            Judgment::Good
        } else {
            Judgment::Miss
        }
    }
}

impl Default for JudgmentWindows {
    fn default() -> Self {
        Self::new(30, 60, 90)
    }
}
