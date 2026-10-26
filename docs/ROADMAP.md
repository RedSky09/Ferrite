\# Ferrite roadmap



Target for v0.1: a 2D engine that runs one complete demo game, with a README,

a demo GIF, and architecture notes.



\## Milestones



\- \[x] M0: Cargo workspace

\- \[x] M1: Window and fixed-timestep game loop

\- \[ ] M2: 2D rendering

&#x20; - \[x] M2a: GPU init and clear color

&#x20; - \[x] M2b: First triangle (WGSL shader, render pipeline)

&#x20; - \[x] M2c: Indexed quad with vertex colors

&#x20; - \[ ] M2d: Textured quad (first sprite)

&#x20; - \[ ] M2e: 2D camera and sprite batcher

\- \[ ] M3: Input

\- \[ ] M4: Asset loading, spritesheets, animation, simple bitmap font

\- \[ ] M5: Entity system (ECS)

&#x20; - \[ ] M5b: Game states (Menu, Playing, GameOver)

\- \[ ] M6: Scene format (serialize/load scenes from files)

\- \[ ] M7: 2D collision (AABB), audio

\- \[ ] M8: Demo game with a real main menu

\- \[ ] M9: Polish, docs, devlog, v0.1



\## After v0.1



\- `ferrite new` CLI that creates a game project from a template

\- Ferrite editor v0 (viewport, hierarchy, inspector, save/load scenes)

\- Pixel art editor as a separate tool in the same workspace



\## Design rules



\- The engine stays a library; games and tools are separate programs using it.

\- No global state; everything lives in structs.

\- Components are plain data so they can be saved to files and edited by tools.

