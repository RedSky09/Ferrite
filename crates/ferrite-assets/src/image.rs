use std::path::Path;

/// Decoded RGBA8 pixels plus their dimensions.
///
/// Pixels are tightly packed, 4 bytes per pixel, in row-major order starting at
/// the top-left corner. This is the format the renderer uploads directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    /// `width * height * 4` bytes.
    rgba: Vec<u8>,
}

impl Image {
    /// Wraps already-decoded RGBA8 pixels.
    ///
    /// # Panics
    /// Panics if `rgba.len() != width * height * 4`.
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(
            rgba.len(),
            (width * height * 4) as usize,
            "pixel buffer does not match {width}x{height} RGBA8"
        );
        Self {
            width,
            height,
            rgba,
        }
    }

    /// Builds a fully transparent image.
    pub fn transparent(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Loads a PNG from disk and decodes it to RGBA8.
    ///
    /// Every pixel is opaque unless the file has an alpha channel, so a plain
    /// RGB PNG gets an alpha of 255 rather than 0.
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let path = path.as_ref();
        let decoded = image::open(path).map_err(|source| ImageError::Open {
            path: path.display().to_string(),
            source,
        })?;
        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(Self {
            width,
            height,
            rgba: rgba.into_raw(),
        })
    }

    /// Decodes a PNG held in memory, for embedded or downloaded assets.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(ImageError::Decode)?;
        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(Self {
            width,
            height,
            rgba: rgba.into_raw(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The tightly packed RGBA8 bytes, ready to upload to the GPU.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Reads one pixel as (r, g, b, a).
    ///
    /// # Panics
    /// Panics if `x` or `y` is outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "pixel out of bounds");
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// Writes one pixel.
    ///
    /// # Panics
    /// Panics if `x` or `y` is outside the image.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        assert!(x < self.width && y < self.height, "pixel out of bounds");
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&color);
    }

    /// Copies a rectangular region into a new image of the same size.
    ///
    /// Used to pull one animation frame out of a spritesheet. The region must
    /// lie fully inside the image.
    pub fn sub_image(&self, rect: crate::spritesheet::Rect) -> Self {
        assert!(
            rect.x + rect.width <= self.width && rect.y + rect.height <= self.height,
            "region {rect:?} is outside a {}x{} image",
            self.width,
            self.height
        );
        let mut out = Vec::with_capacity((rect.width * rect.height * 4) as usize);
        for y in rect.y..rect.y + rect.height {
            let start = ((y * self.width + rect.x) * 4) as usize;
            let end = start + (rect.width * 4) as usize;
            out.extend_from_slice(&self.rgba[start..end]);
        }
        Image::from_rgba8(rect.width, rect.height, out)
    }
}

/// Why an image could not be loaded.
#[derive(Debug)]
pub enum ImageError {
    /// The file could not be opened or recognized as an image.
    Open {
        path: String,
        source: image::ImageError,
    },
    /// The in-memory bytes did not decode as a PNG.
    Decode(image::ImageError),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::Open { path, source } => {
                write!(f, "failed to open image '{path}': {source}")
            }
            ImageError::Decode(source) => write!(f, "failed to decode image: {source}"),
        }
    }
}

impl std::error::Error for ImageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spritesheet::Rect;

    #[test]
    fn stores_and_reads_pixels() {
        let mut img = Image::transparent(2, 2);
        img.set_pixel(1, 0, [10, 20, 30, 40]);
        assert_eq!(img.pixel(1, 0), [10, 20, 30, 40]);
        // Untouched pixels are still transparent.
        assert_eq!(img.pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn sub_image_extracts_the_right_region() {
        let mut img = Image::transparent(4, 2);
        img.set_pixel(2, 1, [1, 2, 3, 4]);
        img.set_pixel(3, 1, [5, 6, 7, 8]);
        let sub = img.sub_image(Rect::new(2, 1, 2, 1));
        assert_eq!(sub.width(), 2);
        assert_eq!(sub.height(), 1);
        assert_eq!(sub.pixel(0, 0), [1, 2, 3, 4]);
        assert_eq!(sub.pixel(1, 0), [5, 6, 7, 8]);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn sub_image_past_the_edge_panics() {
        let img = Image::transparent(2, 2);
        img.sub_image(Rect::new(1, 1, 2, 2));
    }

    #[test]
    #[should_panic(expected = "does not match")]
    fn rejects_a_wrong_sized_buffer() {
        Image::from_rgba8(2, 2, vec![0; 15]);
    }

    #[test]
    fn loads_the_demo_spinner_png() {
        // The example asset is committed next to the examples directory.
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/assets/spinner.png");
        let img = Image::load_png(&path).expect("spinner.png should load");
        assert_eq!((img.width(), img.height()), (64, 16));
        assert_eq!(img.rgba().len(), (64 * 16 * 4) as usize);

        // Frames grow, so the first frame should have fewer lit pixels than the
        // last once sliced into a 4x1 grid.
        let sheet = crate::spritesheet::Spritesheet::from_grid(img, 16, 16);
        assert_eq!(sheet.frame_count(), 4);
        let lit = |index: u32| {
            let rect = sheet.frame_rect_index(index);
            let mut count = 0;
            for y in 0..rect.height {
                for x in 0..rect.width {
                    if sheet.image().pixel(rect.x + x, rect.y + y)[3] != 0 {
                        count += 1;
                    }
                }
            }
            count
        };
        assert!(
            lit(3) > lit(0),
            "the last frame should be bigger than the first"
        );
    }
}
