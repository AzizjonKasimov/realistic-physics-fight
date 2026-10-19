//! The camera: how much room the screen leaves each fighter. Standing, a
//! fighter needs room for its guard and its punch, as the sandbox leaves it
//! (`rp::FIGHTER_REACH`). Leaning back, its far arm, braced to its trunk,
//! swings out to about 0.53 body heights from where it stood, which on a
//! phone held upright, where the fighters fill the width, would leave the
//! screen. So the camera makes room as a fighter reaches out, at once, and
//! takes it back slowly once the fighter comes back: the fighters are drawn
//! as large as what they are doing allows.

use realistic_physics as rp;

use crate::game::{LEFT, RIGHT};

/// Room kept past a fighter's furthest flesh, in body heights, so the
/// flesh's own small sway does not move the camera.
const PAD: f64 = 0.02;
/// The most room the camera makes a fighter, in body heights from where it
/// stood: flesh flung further leaves the screen rather than shrink the
/// fight to nothing.
const MOST_REACH: f64 = 0.6;
/// How slowly the room comes back once a fighter stops reaching out: the
/// time it takes to close all but a third of the gap, in seconds.
const EASE_SECONDS: f64 = 0.5;

/// How much room the screen leaves each fighter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// How far out from where each fighter stood the screen leaves room, on
    /// its side away from the other, in body heights, the left fighter's
    /// first.
    pub reach: [f64; 2],
}

impl Default for Camera {
    /// Room for both fighters' guards and punches.
    fn default() -> Camera {
        Camera {
            reach: [rp::FIGHTER_REACH; 2],
        }
    }
}

impl Camera {
    /// Follows the fighters `dt` seconds on: out at once as far as each
    /// reaches, so nothing of them leaves the screen, and back in smoothly,
    /// never closer than room for the guard.
    pub fn follow(&mut self, world: &rp::World, dt: f64) {
        let ease = 1.0 - (-dt / EASE_SECONDS).exp();
        for (reach, out) in self.reach.iter_mut().zip(reaches(world)) {
            let need = (out + PAD).clamp(rp::FIGHTER_REACH, MOST_REACH);
            *reach = if need > *reach {
                need
            } else {
                *reach + (need - *reach) * ease
            };
        }
    }
}

/// How far each fighter's flesh reaches out from where it stood, on its
/// side away from the other, in body heights, the left fighter's first.
pub fn reaches(world: &rp::World) -> [f64; 2] {
    let [left, right] = rp::fighter_frames();
    let mut out = [f64::MIN; 2];
    for point in world.points() {
        match point.body as usize {
            LEFT => out[LEFT] = out[LEFT].max(left.origin.x - point.position.x),
            RIGHT => out[RIGHT] = out[RIGHT].max(point.position.x - right.origin.x),
            _ => {}
        }
    }
    [out[LEFT] / left.height, out[RIGHT] / right.height]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::hud::Layout;

    /// Standing, the fighters get the room for their guards and punches,
    /// as the sandbox gives them.
    #[test]
    fn standing_fighters_get_room_for_their_guards() {
        let mut game = Game::new(1);
        game.second_player_joins();
        for _ in 0..60 {
            game.step();
        }
        assert_eq!(game.camera, Camera::default());
    }

    /// On a phone held upright, where the fighters fill the width, a fighter
    /// leaning back stays on the screen, the other fighter too: the camera
    /// makes room at once, and gives it back once the lean is over.
    #[test]
    fn a_leaning_fighter_stays_on_a_phone_held_upright() {
        let width = 390.0;
        let layout = Layout::without_buttons(width, 844.0, true);
        let mut game = Game::new(1);
        game.second_player_joins();
        let mut widest = 0.0_f64;
        for step in 0..360 {
            game.hold_lean(LEFT, step < 90);
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
            widest = widest.max(game.camera.reach[LEFT]);
        }
        assert!(
            widest > 0.45,
            "the camera made only {widest:.3} body heights of room for the lean"
        );
        assert_eq!(
            game.camera.reach[RIGHT],
            rp::FIGHTER_REACH,
            "the fighter that did not lean kept its room"
        );
        assert!(
            (game.camera.reach[LEFT] - rp::FIGHTER_REACH).abs() < 1.0e-3,
            "the room for the lean was not given back: {:.3}",
            game.camera.reach[LEFT]
        );
    }
}
