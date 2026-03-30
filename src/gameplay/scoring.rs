use super::judgment::Judgment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JudgmentCounts {
    pub perfect: u32,
    pub great: u32,
    pub good: u32,
    pub miss: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreTotals {
    pub score: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub possible_points: u32,
    pub judgments: JudgmentCounts,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreSummary {
    pub score: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub accuracy: f32,
    pub judgments: JudgmentCounts,
}

impl ScoreTotals {
    pub const fn new(possible_points: u32) -> Self {
        Self {
            score: 0,
            combo: 0,
            max_combo: 0,
            possible_points,
            judgments: JudgmentCounts {
                perfect: 0,
                great: 0,
                good: 0,
                miss: 0,
            },
        }
    }

    pub fn apply(&mut self, judgment: Judgment) {
        self.score = self.score.saturating_add(judgment.points());

        match judgment {
            Judgment::Perfect => self.judgments.perfect += 1,
            Judgment::Great => self.judgments.great += 1,
            Judgment::Good => self.judgments.good += 1,
            Judgment::Miss => self.judgments.miss += 1,
        }

        if judgment.is_hit() {
            self.combo += 1;
            self.max_combo = self.max_combo.max(self.combo);
        } else {
            self.combo = 0;
        }
    }

    pub fn accuracy(&self) -> f32 {
        if self.possible_points == 0 {
            return 1.0;
        }

        self.score as f32 / self.possible_points as f32
    }

    pub fn summary(self) -> ScoreSummary {
        ScoreSummary {
            score: self.score,
            combo: self.combo,
            max_combo: self.max_combo,
            accuracy: self.accuracy(),
            judgments: self.judgments,
        }
    }
}
