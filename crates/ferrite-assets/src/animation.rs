use crate::spritesheet::Rect;

/// One animation frame: where it lives in the atlas, and how long it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The frame's region inside the spritesheet image.
    pub rect: Rect,
    /// How long this frame stays on screen, in milliseconds.
    pub duration_ms: u32,
}

impl Frame {
    pub const fn new(rect: Rect, duration_ms: u32) -> Self {
        Self { rect, duration_ms }
    }

    /// A frame that shows for `ms`.
    pub fn for_ms(rect: Rect, ms: u32) -> Self {
        Self::new(rect, ms)
    }
}

/// A looping or one-shot keyframe animation.
///
/// The animation does not store a clock; the caller keeps its own elapsed time
/// and asks [`Animation::sample`] which frame to show. That keeps it pure data
/// and trivial to test.
#[derive(Clone, Debug)]
pub struct Animation {
    frames: Vec<Frame>,
    /// Whether it wraps back to frame 0 after the last frame.
    pub looping: bool,
}

impl Animation {
    /// Builds an animation from frames. Panics on an empty list.
    pub fn new(frames: Vec<Frame>, looping: bool) -> Self {
        assert!(!frames.is_empty(), "animation needs at least one frame");
        Self { frames, looping }
    }

    /// Builds an animation where every frame shows for the same `frame_ms`.
    pub fn uniform(rects: impl IntoIterator<Item = Rect>, frame_ms: u32, looping: bool) -> Self {
        assert!(frame_ms > 0, "frame duration must be positive");
        Self::new(rects.into_iter().map(|r| Frame::for_ms(r, frame_ms)).collect(), looping)
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Total length of one pass through the frames, in milliseconds.
    pub fn duration_ms(&self) -> u32 {
        self.frames.iter().map(|f| f.duration_ms).sum()
    }

    /// The frame to show after `elapsed_ms`.
    ///
    /// A looping animation wraps; a one-shot animation holds its last frame
    /// once time runs out.
    pub fn sample(&self, elapsed_ms: u64) -> &Frame {
        let total = self.duration_ms() as u64;
        if total == 0 {
            return &self.frames[0];
        }
        let mut t = elapsed_ms;
        if self.looping {
            t %= total;
        } else if t >= total {
            return &self.frames[self.frames.len() - 1];
        }
        let mut acc = 0u64;
        for frame in &self.frames {
            acc += frame.duration_ms as u64;
            if t < acc {
                return frame;
            }
        }
        &self.frames[self.frames.len() - 1]
    }

    /// The index of the frame shown after `elapsed_ms`.
    pub fn sample_index(&self, elapsed_ms: u64) -> usize {
        let sampled = self.sample(elapsed_ms);
        self.frames
            .iter()
            .position(|f| std::ptr::eq(f, sampled))
            .unwrap_or(0)
    }

    /// True once a one-shot animation has run past its last frame.
    pub fn is_finished(&self, elapsed_ms: u64) -> bool {
        !self.looping && elapsed_ms >= self.duration_ms() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anim() -> Animation {
        // Three frames, 100 ms each => 300 ms per pass.
        Animation::uniform(
            [
                Rect::new(0, 0, 1, 1),
                Rect::new(1, 0, 1, 1),
                Rect::new(2, 0, 1, 1),
            ],
            100,
            true,
        )
    }

    #[test]
    fn samples_each_frame_in_order() {
        let a = anim();
        assert_eq!(a.sample_index(0), 0);
        assert_eq!(a.sample_index(99), 0);
        assert_eq!(a.sample_index(100), 1);
        assert_eq!(a.sample_index(250), 2);
    }

    #[test]
    fn looping_wraps_around() {
        let a = anim();
        assert_eq!(a.duration_ms(), 300);
        assert_eq!(a.sample_index(300), 0);
        assert_eq!(a.sample_index(450), 1);
    }

    #[test]
    fn one_shot_holds_the_last_frame() {
        let a = Animation::uniform([Rect::new(0, 0, 1, 1), Rect::new(1, 0, 1, 1)], 100, false);
        assert_eq!(a.sample_index(1_000), 1);
        assert!(!a.is_finished(50));
        assert!(a.is_finished(200));
    }

    #[test]
    fn uneven_durations_respect_their_bounds() {
        let a = Animation::new(
            vec![
                Frame::for_ms(Rect::new(0, 0, 1, 1), 50),
                Frame::for_ms(Rect::new(1, 0, 1, 1), 200),
            ],
            false,
        );
        assert_eq!(a.sample_index(49), 0);
        assert_eq!(a.sample_index(50), 1);
        assert_eq!(a.sample_index(249), 1);
    }

    #[test]
    #[should_panic(expected = "at least one frame")]
    fn rejects_an_empty_animation() {
        let _ = Animation::new(vec![], true);
    }
}
