# M2e: 2D camera and sprite batcher

**Result:** the hard-coded quad became a `Graphics` API a game can call
(`draw_sprite`), positions are in world units transformed by a `Camera2D`, and
every sprite in a frame is drawn in a single draw call. The demo
(`examples/m2_sprites.rs`) draws a wave of 960 sprites and sways the camera.

This milestone landed in two commits: M2e-1 (camera) and M2e-2 (batcher and
`Graphics`).

## Design decisions

**The camera lives in `ferrite-core`, not `ferrite-render`.** `Camera2D` is
pure math: a center, a `view_height`, and a matrix. It knows nothing about
wgpu, which means it can be unit-tested without a GPU (there are three tests,
including one that checks the matrix agrees with `world_to_ndc`). The renderer
only consumes the matrix.

**World units, y-up, width follows the aspect ratio.** The camera always shows
`view_height` world units vertically; the horizontal extent is derived from the
window's aspect ratio. That keeps the vertical framing stable when the window
is resized, which is usually what a 2D game wants.

**`Graphics` is a queue, not an immediate-mode renderer.** During `draw`, a
game only *pushes* `Sprite`s; it never touches the GPU. The engine calls
`begin_frame`, runs the game's `draw`, then hands the collected list to the
`Renderer`. This split is what makes the batcher possible: the renderer sees
the whole frame at once and can issue one draw call for all of it.

**One vertex buffer, one index buffer, one draw call.** Each sprite is four
vertices and six indices. The index pattern is identical for every sprite
(offset by four), so the index buffer is filled *once* when the buffers are
created rather than per frame. Per frame the renderer only rebuilds vertices,
into a reused `Vec` (`vertex_scratch`) so no allocation happens in the steady
state.

**Buffers grow by doubling.** `ensure_sprite_capacity` doubles the vertex and
index buffers until they fit the frame's sprite count. Growth is rare, so the
cost is amortized, and the common case is a length check and a return.

**The camera matrix is uploaded every frame.** It is a small uniform buffer
written with `write_buffer`; at this scale the cost is nothing, and it keeps the
code simple. A dirty flag can come later if it ever shows up in a profile.

## Observations

Growing the buffers rebuilt both of them from scratch, and the empty-frame case
needed guarding: if there are no sprites, `index_count` is zero and the draw
call is skipped entirely rather than issuing a zero-index draw. The 960-sprite
demo runs in a single call, which is the point of the batcher.

## Problems

- (write your own: whether the queued-`Graphics` split was hard to reason
  about, and how reading `App<G: Game>` felt while wiring all of this up)

## Technical notes

- `Sprite` is `{ position, size, color }` with a `with_color` builder; white
  leaves the texture unchanged.
- Vertex layout is `position: vec2, uv: vec2, color: vec3`. Color moved into
  the vertex so tinting is free and per-sprite.
- The camera matrix is column-major, matching WGSL's `mat4x4<f32>`.
- Versions: wgpu 30.0.1.

## Next

M3: keyboard input, so the demo can be driven instead of only swaying on a
timer.
