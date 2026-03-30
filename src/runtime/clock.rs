#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameTime(i64);

impl GameTime {
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

impl From<i64> for GameTime {
    fn from(millis: i64) -> Self {
        Self::from_millis(millis)
    }
}

impl From<u32> for GameTime {
    fn from(millis: u32) -> Self {
        Self::from_millis(millis as i64)
    }
}
