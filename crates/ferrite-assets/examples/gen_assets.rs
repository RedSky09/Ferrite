// Generates the example's demo assets: a 4x1 sprite strip of growing circles
// and writes it as a PNG. Run with `cargo run -p ferrite-assets --example gen_assets`
// (or from the repo root: `cargo run --example gen_assets -p ferrite-assets`).
use ferrite_assets::Image;

fn main() {
    let frame = 16u32;
    let frames = 4u32;
    let width = frame * frames;
    let height = frame;
    let mut img = Image::transparent(width, height);

    for f in 0..frames {
        let radius = 3.0 + f as f32 * 1.6;
        let cx = (f * frame) as f32 + frame as f32 / 2.0;
        let cy = frame as f32 / 2.0;
        for y in 0..height {
            for x in f * frame..(f + 1) * frame {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if (dx * dx + dy * dy).sqrt() <= radius {
                    // Brighten each frame so the animation is obvious.
                    let t = f as f32 / (frames - 1) as f32;
                    let color = [(120.0 + 130.0 * t) as u8, (200.0 - 120.0 * t) as u8, 220, 255];
                    img.set_pixel(x, y, color);
                }
            }
        }
    }

    // Save via the image crate through ferrite-assets' bytes path helper is not
    // available, so write the PNG directly with the image crate.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/assets");
    std::fs::create_dir_all(&dir).expect("create assets dir");
    let path = dir.join("spinner.png");
    let buffer = image::RgbaImage::from_raw(width, height, img.rgba().to_vec())
        .expect("valid buffer");
    buffer.save(&path).expect("save png");
    println!("wrote {}", path.display());
}
