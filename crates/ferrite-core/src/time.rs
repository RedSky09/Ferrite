use std::time::{Duration, Instant};

/// Fixed timestep driven by an accumulator.
///
/// Game logic runs in constant-size steps, no matter how fast or slow
/// frames are rendered.
pub struct FixedTimestep {
    step: Duration,
    accumulator: Duration,
    last: Instant,
    max_frame: Duration,
}

impl FixedTimestep {
    pub fn new(updates_per_second: u32) -> Self {
        Self {
            step: Duration::from_nanos(1_000_000_000 / updates_per_second as u64),
            accumulator: Duration::ZERO,
            last: Instant::now(),
            // Upper bound for a single frame. Without it, a long stall (for
            // example while the window is being dragged or a debugger is
            // attached) would make the loop try to catch up for a long time.
            max_frame: Duration::from_millis(250),
        }
    }

    /// Length of one fixed step, in seconds.
    pub fn step_secs(&self) -> f32 {
        self.step.as_secs_f32()
    }

    /// Call once per frame. Returns how many fixed updates should run.
    pub fn advance(&mut self) -> u32 {
        let now = Instant::now();
        let elapsed = now - self.last;
        self.last = now;
        self.advance_by(elapsed)
    }

    /// Testable variant: the elapsed time is supplied by the caller.
    pub fn advance_by(&mut self, elapsed: Duration) -> u32 {
        self.accumulator += elapsed.min(self.max_frame);
        let mut steps = 0;
        while self.accumulator >= self.step {
            self.accumulator -= self.step;
            steps += 1;
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_steps_and_keeps_remainder() {
        let mut t = FixedTimestep::new(100); // 10 ms per step
        assert_eq!(t.advance_by(Duration::from_millis(35)), 3);
        // 5 ms left over + 5 ms new = 1 more step
        assert_eq!(t.advance_by(Duration::from_millis(5)), 1);
    }

    #[test]
    fn clamps_long_frames() {
        let mut t = FixedTimestep::new(100);
        // 10 seconds is clamped to 250 ms, which is 25 steps
        assert_eq!(t.advance_by(Duration::from_secs(10)), 25);
    }
}
