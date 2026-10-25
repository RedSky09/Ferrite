use std::time::{Duration, Instant};

/// Fixed timestep dengan accumulator.
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
            // Batas agar loop tidak "mengejar ketertinggalan" tanpa henti
            // setelah jeda panjang (misal window digeser/di-debug).
            max_frame: Duration::from_millis(250),
        }
    }

    pub fn step_secs(&self) -> f32 {
        self.step.as_secs_f32()
    }

    /// Panggil sekali per frame. Mengembalikan jumlah update tetap yang harus dijalankan.
    pub fn advance(&mut self) -> u32 {
        let now = Instant::now();
        let elapsed = now - self.last;
        self.last = now;
        self.advance_by(elapsed)
    }

    /// Versi yang bisa dites: waktu diberikan dari luar.
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
        // sisa 5 ms + 5 ms = 1 step
        assert_eq!(t.advance_by(Duration::from_millis(5)), 1);
    }

    #[test]
    fn clamps_long_frames() {
        let mut t = FixedTimestep::new(100);
        // 10 detik dipotong ke 250 ms, jadi 25 step
        assert_eq!(t.advance_by(Duration::from_secs(10)), 25);
    }
}
