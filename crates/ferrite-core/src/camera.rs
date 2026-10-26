/// A 2D camera looking at the world from above.
///
/// World units are arbitrary (think "tiles" or "meters"). The camera always
/// shows `view_height` world units vertically; the horizontal extent follows
/// the window's aspect ratio. The world uses a y-up coordinate system.
#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    /// World position at the center of the screen.
    pub center: [f32; 2],
    /// How many world units are visible from the bottom to the top of the screen.
    pub view_height: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            view_height: 10.0,
        }
    }
}

impl Camera2D {
    pub fn new(center: [f32; 2], view_height: f32) -> Self {
        Self {
            center,
            view_height,
        }
    }

    /// Half of the visible area in world units: (half width, half height).
    fn half_extents(&self, aspect: f32) -> (f32, f32) {
        debug_assert!(aspect > 0.0 && self.view_height > 0.0);
        let half_h = self.view_height * 0.5;
        (half_h * aspect, half_h)
    }

    /// Column-major 4x4 matrix mapping world space to clip space.
    /// `aspect` is window width divided by window height.
    pub fn view_projection(&self, aspect: f32) -> [[f32; 4]; 4] {
        let (half_w, half_h) = self.half_extents(aspect);
        let [cx, cy] = self.center;
        [
            [1.0 / half_w, 0.0, 0.0, 0.0],
            [0.0, 1.0 / half_h, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-cx / half_w, -cy / half_h, 0.0, 1.0],
        ]
    }

    /// Maps a world point to normalized device coordinates (-1 to 1 on both
    /// axes). Mostly useful for tests and for picking later.
    pub fn world_to_ndc(&self, aspect: f32, p: [f32; 2]) -> [f32; 2] {
        let (half_w, half_h) = self.half_extents(aspect);
        [
            (p[0] - self.center[0]) / half_w,
            (p[1] - self.center[1]) / half_h,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn camera_center_maps_to_origin() {
        let cam = Camera2D::new([3.0, -2.0], 10.0);
        let ndc = cam.world_to_ndc(16.0 / 9.0, [3.0, -2.0]);
        assert!(approx(ndc[0], 0.0) && approx(ndc[1], 0.0));
    }

    #[test]
    fn visible_corner_maps_to_one() {
        // view_height 10 => half height 5; aspect 2 => half width 10
        let cam = Camera2D::new([0.0, 0.0], 10.0);
        let ndc = cam.world_to_ndc(2.0, [10.0, 5.0]);
        assert!(approx(ndc[0], 1.0) && approx(ndc[1], 1.0));
    }

    #[test]
    fn matrix_agrees_with_world_to_ndc() {
        let cam = Camera2D::new([1.5, 4.0], 8.0);
        let aspect = 1.6;
        let p = [2.0, 6.5];
        let m = cam.view_projection(aspect);
        // Column-major: m[column][row]. Apply to (x, y, 0, 1).
        let x = m[0][0] * p[0] + m[1][0] * p[1] + m[3][0];
        let y = m[0][1] * p[0] + m[1][1] * p[1] + m[3][1];
        let ndc = cam.world_to_ndc(aspect, p);
        assert!(approx(x, ndc[0]) && approx(y, ndc[1]));
    }
}
