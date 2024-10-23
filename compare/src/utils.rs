use std::fmt::{Display, Formatter};

pub struct DurationDisplay(u64);

pub trait DurationExt {
    fn display_duration(&self) -> DurationDisplay;
}

impl DurationExt for f32 {
    fn display_duration(&self) -> DurationDisplay {
        DurationDisplay((self * 100.0).round() as _)
    }
}

impl Display for DurationDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let total_secs = self.0 / 100;
        let hours = total_secs / 3600;
        let rem = total_secs % 3600;
        let minutes = rem / 60;
        let seconds = rem % 60;
        let fraction = self.0 % 100;

        write!(f, "{}:{:02}:{:02}.{:02}", hours, minutes, seconds, fraction)
    }
}
