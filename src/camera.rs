//! The camera: how much of the ring the screen shows. The fighters step in
//! and out, lean back, and are knocked about, so the room they need across
//! the screen changes. On a wide screen there is room for the whole ring
//! with the fighters drawn as tall as fits, and nothing moves; on a narrow
//! one, a phone held upright, the fighters are drawn as large as the room
//! they need allows: the camera makes room at once as one moves out, and
//! takes it back slowly once it comes back in.

use realistic_physics as rp;
use rp::scenarios::SCENARIO_WIDTH;

use crate::game::RING_HALF;

/// Room kept past the fighters' furthest flesh, in body heights, so the
/// flesh's own small sway does not move the camera.
const PAD: f64 = 0.04;
/// The least the screen shows each side of the ring's middle, in body
/// heights: the fighters where they start, with their guards and room for
/// a punch.
pub const LEAST_HALF: f64 = 0.74;
/// The most it shows: a fighter at the ring's edge, leaning back with its
/// arms out. Flesh flung further leaves the screen rather than shrink the
/// fight to nothing.
pub const MOST_HALF: f64 = RING_HALF + 0.45;
/// How slowly the room comes back once the fighters need less: the time it
/// takes to close all but a third of the gap, in seconds.
const EASE_SECONDS: f64 = 0.5;

/// How much of the ring the screen shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// How far each side of the ring's middle the screen shows, in body
    /// heights.
    pub half: f64,
}

impl Default for Camera {
    /// Room for both fighters where they start.
    fn default() -> Camera {
        Camera { half: LEAST_HALF }
    }
}

impl Camera {
    /// Follows the fighters `dt` seconds on: out at once as far as either
    /// reaches, so nothing of them leaves the screen, and back in smoothly.
    pub fn follow(&mut self, world: &rp::World, dt: f64) {
        let ease = 1.0 - (-dt / EASE_SECONDS).exp();
        let need = (reach(world) + PAD).clamp(LEAST_HALF, MOST_HALF);
        self.half = if need > self.half {
            need
        } else {
            self.half + (need - self.half) * ease
        };
    }
}

/// How far the fighters' flesh reaches from the ring's middle, in body
/// heights, on the side it reaches furthest.
pub fn reach(world: &rp::World) -> f64 {
    let middle = SCENARIO_WIDTH * 0.5;
    world
        .points()
        .iter()
        .map(|point| (point.position.x - middle).abs())
        .fold(0.0, f64::max)
        / rp::world_frame().height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::{Act, Hold};
    use crate::game::{Game, LEFT};
    use crate::hud::Layout;

    /// Standing where they start, the fighters get the least room: the
    /// camera does not move for their stance's sway.
    #[test]
    fn standing_fighters_get_the_room_they_start_with() {
        let mut game = Game::new(1, crate::opponent::Level::Normal);
        game.second_player_joins();
        for _ in 0..60 {
            game.step();
            game.second_player_joins();
        }
        assert_eq!(game.camera, Camera::default());
    }

    /// On a phone held upright, a fighter stepping back to the ring's edge
    /// and leaning back there stays on the screen, the other fighter too:
    /// the camera makes room at once, and gives it back once the fighter
    /// comes back in. Each step is pressed once the one before is done: a
    /// step on the fighter's own legs takes about 0.6 s, and a step pressed
    /// in the middle of one is not taken.
    #[test]
    fn a_fighter_at_the_rings_edge_stays_on_a_phone_held_upright() {
        let width = 390.0;
        let layout = Layout::without_buttons(width, 844.0, true);
        let mut game = Game::new(1, crate::opponent::Level::Normal);
        let mut widest = 0.0_f64;
        for step in 0..480 {
            game.second_player_joins();
            match step {
                10 | 60 => game.press(LEFT, Act::StepOut),
                240 | 290 => game.press(LEFT, Act::StepIn),
                _ => {}
            }
            game.hold(LEFT, Hold::Lean, (110..200).contains(&step));
            game.step();
            let view = layout.view(&game.camera);
            for point in game.world.points() {
                let x = view.to_screen(point.position).x;
                assert!(
                    (0.0..=width as f64).contains(&x),
                    "step {step}: fighter {} drawn at x {x:.1}, off the {width} px screen",
                    point.body
                );
            }
            widest = widest.max(game.camera.half);
        }
        assert!(
            widest > LEAST_HALF + 0.15,
            "the camera made only {widest:.3} body heights of room"
        );
        assert!(
            game.camera.half < LEAST_HALF + 0.05,
            "the room was not given back: {:.3}",
            game.camera.half
        );
    }
}
