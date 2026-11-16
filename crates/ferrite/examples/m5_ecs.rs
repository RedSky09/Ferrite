//! M5 demo: a tiny entity system plus game states.
//!
//! Entities live in a `World`; their position, velocity, and color are plain
//! components. A `StateMachine` drives Menu -> Playing -> GameOver.
//!
//! Controls: Enter starts / restarts, Space squashes the nearest enemy, arrows
//! move the player.

use ferrite::{
    run, Anchor, Atlas, Config, Font, Game, Graphics, Input, Key, Sprite, State, StateMachine, World,
};

// --- components: plain data ------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Pos([f32; 2]);
#[derive(Clone, Copy, Debug)]
struct Vel([f32; 2]);
#[derive(Clone, Copy, Debug)]
struct Size([f32; 2]);
#[derive(Clone, Copy, Debug)]
struct Color([f32; 3]);
/// Tags the player-controlled entity.
#[derive(Clone, Copy, Debug)]
struct Player;
/// Tags enemies that chase the player.
#[derive(Clone, Copy, Debug)]
struct Enemy;

// --- states ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameState {
    Menu,
    Playing,
    GameOver,
}

// `State` is blanket-implemented for any `Copy + PartialEq` type.
fn _assert_state<S: State>() {}

// --- game ------------------------------------------------------------------

struct Demo {
    world: World,
    states: StateMachine<GameState>,
    player: Option<ferrite::Entity>,
    score: u32,
    spawn_timer: f32,
    rng: u32,
    font: Font,
    font_atlas: Option<Atlas>,
}

impl Demo {
    fn new() -> Self {
        _assert_state::<GameState>();
        Self {
            world: World::new(),
            states: StateMachine::new(GameState::Menu),
            player: None,
            score: 0,
            spawn_timer: 0.0,
            rng: 0x1234_5678,
            font: Font::builtin(),
            font_atlas: None,
        }
    }

    /// Deterministic-poor but sufficient random for a demo.
    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 1000) as f32 / 1000.0
    }

    fn start_game(&mut self) {
        self.world.clear();
        self.score = 0;
        self.spawn_timer = 0.0;

        // The player.
        let player = self.world.spawn();
        self.world.insert(player, Pos([0.0, 0.0]));
        self.world.insert(player, Vel([0.0, 0.0]));
        self.world.insert(player, Size([0.9, 0.9]));
        self.world.insert(player, Color([0.4, 0.8, 1.0]));
        self.world.insert(player, Player);
        self.player = Some(player);
    }

    fn spawn_enemy(&mut self) {
        // Enter from a random edge.
        let angle = self.rand() * std::f32::consts::TAU;
        let radius = 8.0;
        let pos = [angle.cos() * radius, angle.sin() * radius * 0.6];
        let e = self.world.spawn();
        self.world.insert(e, Pos(pos));
        self.world.insert(e, Vel([0.0, 0.0]));
        self.world.insert(e, Size([0.6, 0.6]));
        self.world.insert(e, Color([1.0, 0.35, 0.4]));
        self.world.insert(e, Enemy);
    }

    fn update_playing(&mut self, dt: f32, input: &Input) {
        // Move the player with the arrow keys.
        if let Some(player) = self.player {
            let mut dir = [0.0, 0.0];
            if input.is_down(Key::Left) {
                dir[0] -= 1.0;
            }
            if input.is_down(Key::Right) {
                dir[0] += 1.0;
            }
            if input.is_down(Key::Up) {
                dir[1] += 1.0;
            }
            if input.is_down(Key::Down) {
                dir[1] -= 1.0;
            }
            if let Some(pos) = self.world.get_mut::<Pos>(player) {
                pos.0[0] = (pos.0[0] + dir[0] * 5.0 * dt).clamp(-8.5, 8.5);
                pos.0[1] = (pos.0[1] + dir[1] * 5.0 * dt).clamp(-4.5, 4.5);
            }
        }

        // Spawn enemies over time.
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.spawn_timer = 1.2;
            self.spawn_enemy();
        }

        // Steer enemies toward the player. Only `Vel` needs to change, so this
        // gathers targets first, then writes velocities.
        let target = self
            .player
            .and_then(|p| self.world.get::<Pos>(p))
            .map(|p| p.0)
            .unwrap_or([0.0, 0.0]);
        let mut updates: Vec<(ferrite::Entity, [f32; 2])> = Vec::new();
        for (entity, (pos, _)) in self.world.query::<(&Pos, &Enemy)>() {
            let dx = target[0] - pos.0[0];
            let dy = target[1] - pos.0[1];
            let len = (dx * dx + dy * dy).sqrt().max(1e-3);
            updates.push((entity, [dx / len, dy / len]));
        }
        for (entity, dir) in updates {
            if let Some(vel) = self.world.get_mut::<Vel>(entity) {
                vel.0 = [dir[0] * 1.6, dir[1] * 1.6];
            }
        }

        // Integrate positions.
        let mut positions: Vec<(ferrite::Entity, [f32; 2])> = Vec::new();
        for (entity, (pos, vel)) in self.world.query::<(&Pos, &Vel)>() {
            positions.push((entity, [pos.0[0] + vel.0[0] * dt, pos.0[1] + vel.0[1] * dt]));
        }
        for (entity, new_pos) in positions {
            if let Some(pos) = self.world.get_mut::<Pos>(entity) {
                pos.0 = new_pos;
            }
        }

        // Space: remove the enemy closest to the player and add score.
        if input.just_pressed(Key::Space) {
            let mut closest: Option<(ferrite::Entity, f32)> = None;
            for (entity, (pos, _)) in self.world.query::<(&Pos, &Enemy)>() {
                let dx = pos.0[0] - target[0];
                let dy = pos.0[1] - target[1];
                let d = dx * dx + dy * dy;
                if closest.is_none_or(|(_, best)| d < best) {
                    closest = Some((entity, d));
                }
            }
            if let Some((entity, _)) = closest {
                self.world.despawn(entity);
                self.score += 1;
            }
        }

        // Enemies touching the player end the game.
        let mut dead = false;
        for (entity, (pos, _)) in self.world.query::<(&Pos, &Enemy)>() {
            let dx = pos.0[0] - target[0];
            let dy = pos.0[1] - target[1];
            if dx * dx + dy * dy < 0.5 * 0.5 {
                let _ = entity;
                dead = true;
                break;
            }
        }
        if dead {
            self.states.request(GameState::GameOver);
        }
    }

    fn draw_world(&self, gfx: &mut Graphics) {
        for (_, (pos, size, color)) in self.world.query::<(&Pos, &Size, &Color)>() {
            gfx.draw_sprite(Sprite::new(pos.0, size.0).with_color(color.0));
        }
    }

    /// Draws one line of text centered horizontally at a world y.
    fn text_centered(&self, gfx: &mut Graphics, text: &str, y: f32, pixel_size: f32, color: [f32; 3]) {
        if let Some(atlas) = self.font_atlas {
            gfx.draw_text_colored(
                &self.font,
                atlas,
                text,
                [0.0, y],
                pixel_size,
                Anchor::CENTER,
                color,
            );
        }
    }
}

impl Game for Demo {
    fn update(&mut self, dt: f32, input: &Input) {
        match self.states.current() {
            GameState::Menu => {
                if input.just_pressed(Key::Enter) {
                    self.states.request(GameState::Playing);
                }
            }
            GameState::Playing => {
                self.update_playing(dt, input);
            }
            GameState::GameOver => {
                if input.just_pressed(Key::Enter) {
                    self.states.request(GameState::Playing);
                }
            }
        }

        // Apply any transition requested this frame. On enter/restart, rebuild.
        if let Some((_from, to)) = self.states.apply() {
            if to == GameState::Playing {
                self.start_game();
            }
        }
    }

    fn draw(&mut self, gfx: &mut Graphics) {
        gfx.camera.view_height = 12.0;
        gfx.camera.center = [0.0, 0.0];

        // Upload the font atlas on the first frame.
        if self.font_atlas.is_none() {
            self.font_atlas = Some(gfx.upload_image(self.font.atlas()));
        }

        match self.states.current() {
            GameState::Menu => {
                gfx.clear_color = [0.05, 0.08, 0.14];
                self.text_centered(gfx, "FERRITE", 2.2, 0.22, [0.55, 0.85, 1.0]);
                self.text_centered(gfx, "ECS DEMO", 0.9, 0.1, [0.4, 0.55, 0.75]);
                self.text_centered(gfx, "PRESS ENTER TO START", -1.6, 0.09, [0.85, 0.85, 0.9]);
                self.text_centered(gfx, "ARROWS MOVE   SPACE SHOOT", -2.6, 0.075, [0.5, 0.5, 0.6]);
            }
            GameState::Playing => {
                gfx.clear_color = [0.07, 0.07, 0.10];
                self.draw_world(gfx);
                let score = format!("SCORE {}", self.score);
                self.text_centered(gfx, &score, 5.2, 0.1, [0.85, 0.85, 0.9]);
            }
            GameState::GameOver => {
                gfx.clear_color = [0.16, 0.05, 0.06];
                self.draw_world(gfx);
                self.text_centered(gfx, "GAME OVER", 1.4, 0.24, [1.0, 0.55, 0.5]);
                let score = format!("SCORE {}", self.score);
                self.text_centered(gfx, &score, -0.2, 0.11, [0.9, 0.9, 0.9]);
                self.text_centered(gfx, "PRESS ENTER TO RETRY", -1.6, 0.08, [0.7, 0.7, 0.75]);
            }
        }
    }
}

fn main() {
    env_logger::builder().format_timestamp_millis().init();
    run(Config::default(), Demo::new()).expect("event loop failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(input: &mut Input, key: Key) {
        input.handle_key(key, true);
    }

    #[test]
    fn starts_in_the_menu() {
        let demo = Demo::new();
        assert!(demo.states.is(GameState::Menu));
        assert_eq!(demo.world.entity_count(), 0);
    }

    #[test]
    fn enter_starts_the_game_and_spawns_the_player() {
        let mut demo = Demo::new();
        let mut input = Input::new();
        press(&mut input, Key::Enter);
        demo.update(1.0 / 60.0, &input);
        input.clear_transient();

        assert!(demo.states.is(GameState::Playing));
        assert_eq!(demo.world.entity_count(), 1);
        assert!(demo.player.is_some());
        assert!(demo.world.has::<Player>(demo.player.unwrap()));
    }

    #[test]
    fn enemies_spawn_over_time_and_chase() {
        let mut demo = Demo::new();
        demo.start_game();
        // Run two seconds of updates with no input; enemies should appear.
        let input = Input::new();
        for _ in 0..120 {
            demo.update_playing(1.0 / 60.0, &input);
        }
        let enemies = demo.world.query::<&Enemy>().count();
        assert!(enemies >= 1, "expected at least one enemy, got {enemies}");
    }

    #[test]
    fn space_removes_an_enemy_and_scores() {
        let mut demo = Demo::new();
        demo.start_game();
        demo.spawn_enemy();
        demo.spawn_enemy();
        let before = demo.world.query::<&Enemy>().count();
        assert_eq!(before, 2);

        // Freeze spawning so only the Space press changes anything.
        demo.spawn_timer = 10.0;
        let mut input = Input::new();
        press(&mut input, Key::Space);
        demo.update_playing(1.0 / 60.0, &input);

        let after = demo.world.query::<&Enemy>().count();
        assert_eq!(after, before - 1);
        assert_eq!(demo.score, 1);
    }

    #[test]
    fn touching_an_enemy_requests_game_over() {
        let mut demo = Demo::new();
        demo.start_game();
        // Put an enemy right on top of the player.
        let player_pos = demo.world.get::<Pos>(demo.player.unwrap()).unwrap().0;
        let e = demo.world.spawn();
        demo.world.insert(e, Pos(player_pos));
        demo.world.insert(e, Vel([0.0, 0.0]));
        demo.world.insert(e, Size([0.6, 0.6]));
        demo.world.insert(e, Color([1.0, 0.0, 0.0]));
        demo.world.insert(e, Enemy);

        let input = Input::new();
        demo.update_playing(1.0 / 60.0, &input);
        // The request is queued; applying it moves to GameOver.
        assert!(demo.states.is_pending());
        let change = demo.states.apply();
        assert_eq!(change.map(|(_, to)| to), Some(GameState::GameOver));
    }

    #[test]
    fn restarting_clears_the_previous_world() {
        let mut demo = Demo::new();
        demo.start_game();
        demo.spawn_enemy();
        demo.spawn_enemy();
        let old_count = demo.world.entity_count();
        assert!(old_count >= 3);

        demo.start_game();
        // Only the fresh player remains.
        assert_eq!(demo.world.entity_count(), 1);
        assert_eq!(demo.score, 0);
    }
}
