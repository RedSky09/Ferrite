# M2d: Textured quad (first sprite)

**Result:** the quad from M2c is now sampled from a texture. The fragment
shader reads a `texture_2d` at the interpolated `uv` and multiplies it by the
vertex color, so the quad can be tinted. The texture itself is an 8x8
checkerboard generated in code.

## Design decisions

**A real image, not a file.** The first texture is a procedurally generated 8x8
checkerboard rather than a loaded PNG. This keeps M2d focused on *the GPU path*
(create texture, upload pixels, bind it, sample it) without dragging asset
loading forward from M4. The top-left pixel is red on purpose: if the image
comes out flipped, it is obvious at a glance.

**`Texture` owns its view and sampler.** A texture is useless without a view to
bind and a sampler to read it with, so `Texture::from_rgba8` builds all three
and hands back one struct. Callers pass tightly packed RGBA8 bytes; the
function asserts the length matches `width * height * 4` so a mistake fails
loudly instead of rendering garbage.

**Nearest filtering.** `mag_filter` and `min_filter` are both `Nearest`. This
is pixel art; a smoothed sample would blur the checkerboard and every sprite
after it.

**A second bind group.** The camera is group 0; the texture and its sampler are
group 1. Keeping them separate (rather than packing everything into one group)
means the per-image data is isolated, which is what a sprite batcher needs
later: many textures, one camera.

**sRGB.** The texture format is `Rgba8UnormSrgb`, so 8-bit pixel values are
converted to linear space on read and back on present. Without it, colors would
look washed out.

## Observations

The checkerboard is exactly what the shader sees, and the red corner lands
top-left as expected, so the UV convention (0,0 at top-left, 1,1 at
bottom-right) matches the vertex layout from M2c. Encoding that convention in a
visible test image paid off: it turned a silent orientation bug into something
I could see.

## Problems

- (write your own: whether shuffling bind groups was intuitive, and whether
  the assert on pixel length caught anything)

## Technical notes

- The shader gained a `@group(1)` binding: `t_sprite: texture_2d<f32>` and
  `s_sprite: sampler`.
- `Texture::from_rgba8` uploads with a single `queue.write_texture`, so there
  is no staging buffer yet. Allocate one when textures get large or start
  coming from disk.
- Versions: wgpu 30.0.1.

## Next

M2e: a 2D camera and a sprite batcher, so scenes are not hard-coded to one
quad.
