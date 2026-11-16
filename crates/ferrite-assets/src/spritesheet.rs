use crate::image::Image;

/// A rectangular region of an image, in pixels.
///
/// Used for spritesheet frames and for drawing a slice of an atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The region's uv rectangle within a texture of size `image_w` x `image_h`.
    ///
    /// Returns `[u0, v0, u1, v1]` with `v` growing downward, matching the
    /// engine's texture convention (0,0 is the top-left texel).
    pub fn uv(&self, image_w: u32, image_h: u32) -> [f32; 4] {
        let iw = image_w as f32;
        let ih = image_h as f32;
        [
            self.x as f32 / iw,
            self.y as f32 / ih,
            (self.x + self.width) as f32 / iw,
            (self.y + self.height) as f32 / ih,
        ]
    }
}

/// An image sliced into a grid of equally sized frames.
///
/// A spritesheet keeps the whole atlas in memory along with the pixel size of
/// one frame; frames are addressed by `(column, row)` or by index in row-major
/// order. The renderer uploads the atlas once and draws frames by uv.
#[derive(Clone, Debug)]
pub struct Spritesheet {
    image: Image,
    frame_width: u32,
    frame_height: u32,
    columns: u32,
    rows: u32,
}

impl Spritesheet {
    /// Slices `image` into a grid of `frame_width` x `frame_height` cells.
    ///
    /// # Panics
    /// Panics if the frame size is zero or does not divide the image evenly,
    /// which would leave partial frames and make frame indices ambiguous.
    pub fn from_grid(image: Image, frame_width: u32, frame_height: u32) -> Self {
        assert!(frame_width > 0 && frame_height > 0, "frame size must be nonzero");
        assert_eq!(
            image.width() % frame_width,
            0,
            "image width {} is not a multiple of the frame width {frame_width}",
            image.width()
        );
        assert_eq!(
            image.height() % frame_height,
            0,
            "image height {} is not a multiple of the frame height {frame_height}",
            image.height()
        );
        Self {
            columns: image.width() / frame_width,
            rows: image.height() / frame_height,
            image,
            frame_width,
            frame_height,
        }
    }

    /// Slices a sheet of `columns` x `rows` frames, ignoring any leftover edge.
    ///
    /// This is the forgiving counterpart to [`Spritesheet::from_grid`]: it
    /// picks the largest frame size that fits the requested grid.
    ///
    /// # Panics
    /// Panics if `columns` or `rows` is zero.
    pub fn from_columns_rows(image: Image, columns: u32, rows: u32) -> Self {
        assert!(columns > 0 && rows > 0, "grid must be at least 1x1");
        let frame_width = image.width() / columns;
        let frame_height = image.height() / rows;
        Self::from_grid(image, frame_width, frame_height)
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn frame_width(&self) -> u32 {
        self.frame_width
    }

    pub fn frame_height(&self) -> u32 {
        self.frame_height
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Total number of frames.
    pub fn frame_count(&self) -> u32 {
        self.columns * self.rows
    }

    /// The pixel region of the frame at `(column, row)`.
    ///
    /// # Panics
    /// Panics if the cell is outside the grid.
    pub fn frame_rect(&self, column: u32, row: u32) -> Rect {
        assert!(
            column < self.columns && row < self.rows,
            "frame ({column}, {row}) is outside a {}x{} grid",
            self.columns,
            self.rows
        );
        Rect::new(
            column * self.frame_width,
            row * self.frame_height,
            self.frame_width,
            self.frame_height,
        )
    }

    /// The frame at `index`, in row-major order (left to right, top to bottom).
    ///
    /// # Panics
    /// Panics if `index >= frame_count()`.
    pub fn frame_rect_index(&self, index: u32) -> Rect {
        assert!(
            index < self.frame_count(),
            "frame index {index} is out of range (count {})",
            self.frame_count()
        );
        self.frame_rect(index % self.columns, index / self.columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Spritesheet {
        // 4x4 image of 2x2 frames => 2 columns, 2 rows.
        Spritesheet::from_grid(Image::transparent(4, 4), 2, 2)
    }

    #[test]
    fn slices_a_grid() {
        let s = sheet();
        assert_eq!((s.columns(), s.rows()), (2, 2));
        assert_eq!(s.frame_count(), 4);
        assert_eq!(s.frame_rect(1, 1), Rect::new(2, 2, 2, 2));
    }

    #[test]
    fn index_is_row_major() {
        let s = sheet();
        assert_eq!(s.frame_rect_index(0), Rect::new(0, 0, 2, 2));
        assert_eq!(s.frame_rect_index(1), Rect::new(2, 0, 2, 2));
        assert_eq!(s.frame_rect_index(2), Rect::new(0, 2, 2, 2));
        assert_eq!(s.frame_rect_index(3), Rect::new(2, 2, 2, 2));
    }

    #[test]
    fn uv_maps_the_region_and_flips_to_v_down() {
        let rect = Rect::new(2, 2, 2, 2);
        let uv = rect.uv(4, 4);
        assert_eq!(uv, [0.5, 0.5, 1.0, 1.0]);
    }

    #[test]
    #[should_panic(expected = "not a multiple")]
    fn rejects_a_grid_that_does_not_divide() {
        Spritesheet::from_grid(Image::transparent(5, 4), 2, 2);
    }
}
