use std::time::Duration;

pub trait DurationExt {
    fn checked_mul_f64(self, rhs: f64) -> Option<Duration>;
}

impl DurationExt for Duration {
    fn checked_mul_f64(self, rhs: f64) -> Option<Duration> {
        let lhs = self.as_secs_f64();
        let new = lhs * rhs;

        Duration::try_from_secs_f64(new).ok()
    }
}
