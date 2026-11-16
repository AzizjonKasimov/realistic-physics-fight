//! A fight: two fighters on the engine, seen from the side and facing each
//! other, the left one driven by its player's moves, and the right by a
//! second player's, or by the sparring partner while nobody plays it. The
//! moves are the engine's: a jab, a cross, a step in and a step back, and,
//! held, a block, a duck, and a lean back. A punch goes through the other
//! fighter's open hands and lands on its head; a held block stops it on the
//! forearms. Blows hurt the fighters as they hurt bodies in the sandbox, and
//! a blow that turns the head hard enough knocks a fighter out (the
//! engine's `knockout.rs`): it goes down and stays down, and the fight is
//! over, as it is when a fighter falls. A fight is started afresh by hand.

use realistic_physics as rp;
use rp::scenarios::SCENARIO_WIDTH;

use crate::camera::Camera;
use crate::controls::{Act, Hold};
use crate::page::{self, Moment};
use crate::sparring::Sparring;

/// The fighter standing on the left, whom a player alone plays.
pub const LEFT: usize = 0;
/// The fighter standing on the right, a second player's or the sparring
/// partner's.
pub const RIGHT: usize = 1;

/// How long the right fighter's keys may rest before the sparring partner
/// takes it back from a second player, in seconds.
pub const SECOND_PLAYER_IDLE: f64 = 10.0;

/// How far from the ring's middle a fighter may stand, in body heights
/// between the middle and its hips: two steps back from where it starts.
/// A step back that would carry it past the ring's edge is not taken.
pub const RING_HALF: f64 = 0.66;
/// How far a step carries a fighter, in body heights, as the engine steps
/// it (its `mover.rs`).
const STEP: f64 = 0.13;

/// The game's version as players see it: v0.2 for 0.2.0, v0.1.2 for 0.1.2.
pub fn version_label() -> String {
    let version = env!("CARGO_PKG_VERSION");
    match version.strip_suffix(".0") {
        Some(short) if short.matches('.').count() == 1 => format!("v{short}"),
        _ => format!("v{version}"),
    }
}

/// How a fight ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ending {
    /// The fighter that went down.
    pub down: usize,
    /// It was knocked out by a blow to its head, rather than falling: a
    /// fighter shoved past where a step could catch it falls, and neither
    /// gets up again yet.
    pub knocked_out: bool,
}

/// A fight and everything that drives it.
pub struct Game {
    pub world: rp::World,
    /// The skin's silhouette edge, which the fighters are drawn with.
    pub rim: rp::draw::SkinRim,
    /// How much of the ring the screen shows, following the fighters.
    pub camera: Camera,
    /// Where everything was before the last step, and where it really is
    /// while a frame draws it between the two.
    earlier: rp::MotionSnapshot,
    actual: rp::MotionSnapshot,
    /// Time this frame's steps have not covered yet, in seconds.
    accumulator: f64,
    pub running: bool,
    /// The sound is switched off.
    pub muted: bool,
    /// The moves each fighter's player pressed since the last step, in
    /// [`Act::ALL`]'s order.
    acts: [[bool; 4]; 2],
    /// The moves each fighter's player holds, in [`Hold::ALL`]'s order, and
    /// what each fighter was last told to hold.
    holds: [[bool; 3]; 2],
    told: [[bool; 3]; 2],
    sparring: Sparring,
    /// How long since a second player last pressed the right fighter's
    /// keys, in seconds; none while nobody has.
    second_player: Option<f64>,
    /// What the next fight's sparring partner's decisions follow from.
    seed: u64,
    /// How the fight ended, once a fighter has gone down, and how many
    /// seconds ago.
    ending: Option<(Ending, f64)>,
}

impl Game {
    /// Two fighters ready to fight, the sparring partner's decisions
    /// following from `seed`.
    pub fn new(seed: u64) -> Game {
        Game::with_materials(seed, rp::Materials::default())
    }

    /// A fight on the engine's settings `materials`, for tests that change
    /// one, as the engine's own tests lower the knockout limit.
    fn with_materials(seed: u64, materials: rp::Materials) -> Game {
        let world = new_fighters(materials);
        let mut earlier = rp::MotionSnapshot::default();
        world.capture_motion(&mut earlier);
        Game {
            rim: rp::draw::SkinRim::of(&world),
            camera: Camera::default(),
            world,
            earlier,
            actual: rp::MotionSnapshot::default(),
            accumulator: 0.0,
            running: true,
            muted: false,
            acts: [[false; 4]; 2],
            holds: [[false; 3]; 2],
            told: [[false; 3]; 2],
            sparring: Sparring::new(seed),
            second_player: None,
            seed,
            ending: None,
        }
    }

    /// How the fight ended and how many seconds ago, once a fighter has gone
    /// down.
    pub fn ending(&self) -> Option<(Ending, f64)> {
        self.ending
    }

    /// Notes the fight's end on the step a fighter goes down: knocked out
    /// by a blow, or fallen. Should both go down on one step, the left
    /// fighter's fall is the one shown.
    fn note_ending(&mut self, dt: f64) {
        if let Some((_, since)) = &mut self.ending {
            *since += dt;
            return;
        }
        let Some(down) = [LEFT, RIGHT]
            .into_iter()
            .find(|&fighter| self.world.fighter_fallen(fighter))
        else {
            return;
        };
        let knocked_out = self.world.knocked_out(down);
        self.ending = Some((Ending { down, knocked_out }, 0.0));
        if knocked_out {
            page::tell(Moment::Knockout);
        }
    }

    /// The fight afresh: unhurt fighters where they first stood. A second
    /// player playing keeps playing.
    pub fn restart(&mut self) {
        let second_player = self.second_player;
        let muted = self.muted;
        *self = Game::new(self.seed.wrapping_add(1));
        self.second_player = second_player;
        self.muted = muted;
        page::tell(Moment::Restart);
    }

    /// `fighter`'s player presses for `act`, made on the next step.
    pub fn press(&mut self, fighter: usize, act: Act) {
        self.acts[fighter][act.index()] = true;
        self.played(fighter);
        if fighter == LEFT {
            page::tell(match act {
                Act::Jab | Act::Cross => Moment::Punch,
                Act::StepIn | Act::StepOut => Moment::Step,
            });
        }
    }

    /// Whether `fighter`'s player holds `hold` now.
    pub fn hold(&mut self, fighter: usize, hold: Hold, held: bool) {
        self.holds[fighter][hold.index()] = held;
        if held {
            self.played(fighter);
            if fighter == LEFT {
                page::tell(match hold {
                    Hold::Block => Moment::Block,
                    Hold::Duck => Moment::Duck,
                    Hold::Lean => Moment::Lean,
                });
            }
        }
    }

    /// A second player takes the right fighter, as pressing its keys does:
    /// for the capture mode's scripts and the tests.
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub fn second_player_joins(&mut self) {
        self.played(RIGHT);
    }

    /// A player pressed `fighter`'s keys or buttons: the right fighter's
    /// make a second player its player.
    fn played(&mut self, fighter: usize) {
        if fighter == RIGHT {
            if !self.second_player_plays() {
                page::tell(Moment::SecondPlayer);
            }
            self.second_player = Some(0.0);
        }
    }

    /// Whether a second player plays the right fighter now, rather than the
    /// sparring partner.
    pub fn second_player_plays(&self) -> bool {
        self.second_player
            .is_some_and(|idle| idle < SECOND_PLAYER_IDLE)
    }

    /// How far apart the fighters stand, in body heights between their
    /// hips.
    pub fn apart(&self) -> f64 {
        let stands = |fighter| self.world.fighter_stands(fighter).unwrap_or(0.0);
        (stands(RIGHT) - stands(LEFT)).abs() / rp::world_frame().height
    }

    /// Whether `fighter` has room in the ring for a step back.
    pub fn room_behind(&self, fighter: usize) -> bool {
        let Some(stands) = self.world.fighter_stands(fighter) else {
            return false;
        };
        let out = (stands - SCENARIO_WIDTH * 0.5).abs() / rp::world_frame().height;
        out + STEP <= RING_HALF
    }

    /// Runs the simulation steps `frame_dt` seconds cover, each at the
    /// engine's fixed step, `hear` hearing each.
    pub fn advance(&mut self, frame_dt: f64, mut hear: impl FnMut(&rp::World)) {
        if !self.running {
            return;
        }
        self.accumulator += frame_dt;
        let fixed_dt = self.world.materials().fixed_dt;
        while self.accumulator >= fixed_dt {
            self.step();
            hear(&self.world);
            self.accumulator -= fixed_dt;
        }
    }

    /// One simulation step: the players' and the sparring partner's moves,
    /// then the world moving on, and the camera following it.
    pub fn step(&mut self) {
        let dt = self.world.materials().fixed_dt;
        if let Some(idle) = &mut self.second_player {
            *idle += dt;
        }
        if !self.second_player_plays() {
            // Once a fighter is down the sparring partner stops, its guard
            // let go of, and stands in its stance.
            if self.ending.is_some() {
                self.holds[RIGHT] = [false; 3];
            } else {
                let wish = self.sparring.tick(dt, self.apart());
                self.holds[RIGHT] = wish.holds;
                if let Some(act) = wish.act {
                    self.acts[RIGHT][act.index()] = true;
                }
            }
        }
        // Every move reaches the engine in the step's input, a hold's start
        // and end too: the step notes which arms already lie across the
        // other fighter before any move changes what they do, and two
        // fighters stepping in at once share the room between them.
        let mut moves: Vec<(usize, rp::Move)> = Vec::new();
        for fighter in [LEFT, RIGHT] {
            for hold in Hold::ALL {
                let held = self.holds[fighter][hold.index()];
                if held != self.told[fighter][hold.index()] {
                    self.told[fighter][hold.index()] = held;
                    let made = match hold {
                        Hold::Block => rp::Move::Block(held),
                        Hold::Duck => rp::Move::Duck(held),
                        Hold::Lean => rp::Move::LeanBack(held),
                    };
                    moves.push((fighter, made));
                }
            }
            let acts = std::mem::take(&mut self.acts[fighter]);
            let pressed = |act: Act| acts[act.index()];
            let step = if pressed(Act::StepIn) {
                Some(rp::Move::StepIn)
            } else if pressed(Act::StepOut) && self.room_behind(fighter) {
                Some(rp::Move::StepOut)
            } else {
                None
            };
            let punch = if pressed(Act::Jab) {
                Some(rp::Move::Jab)
            } else if pressed(Act::Cross) {
                Some(rp::Move::Cross)
            } else {
                None
            };
            moves.extend(
                [step, punch]
                    .into_iter()
                    .flatten()
                    .map(|made| (fighter, made)),
            );
        }
        // Each fighter is told at most three holds, a step, and a punch.
        debug_assert!(moves.len() <= rp::MOVES_A_STEP);
        let mut input = rp::InputState::default();
        for (slot, made) in input.moves.iter_mut().zip(moves) {
            *slot = Some(made);
        }
        self.world.capture_motion(&mut self.earlier);
        self.world.step(dt, &input, SCENARIO_WIDTH);
        self.rim.refresh(&self.world);
        self.camera.follow(&self.world, dt);
        self.note_ending(dt);
    }

    /// Moves everything to where this frame shows it, between the last two
    /// steps, so it moves as smoothly as the screen refreshes; `restore`
    /// puts it back before the world steps again.
    pub fn blend(&mut self) {
        let alpha = self.accumulator / self.world.materials().fixed_dt;
        self.world
            .blend_motion(&self.earlier, alpha, &mut self.actual);
    }

    /// Puts everything back where it really is after `blend`.
    pub fn restore(&mut self) {
        self.world.restore_motion(&self.actual);
    }
}

/// Two unhurt fighters facing each other, a step out of each other's reach,
/// the right built as the left's mirror image.
fn new_fighters(materials: rp::Materials) -> rp::World {
    rp::create_side_fighters(materials)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(game: &mut Game, count: usize) {
        for _ in 0..count {
            game.step();
        }
    }

    /// A fight between two players, so that the right fighter moves only as
    /// the test says, with the left fighter a step in, in reach.
    fn in_reach() -> Game {
        let mut game = Game::new(1);
        game.second_player_joins();
        game.press(LEFT, Act::StepIn);
        steps(&mut game, 30);
        game.second_player_joins();
        game
    }

    /// The player's cross lands on the other fighter and bruises it.
    #[test]
    fn the_players_cross_lands() {
        let mut game = in_reach();
        game.press(LEFT, Act::Cross);
        steps(&mut game, 45);
        assert!(
            game.world.stats().contusion_events > 0,
            "the cross bruised nothing"
        );
    }

    /// From where the fighters start, a step apart, a punch falls short:
    /// the player has to step in first.
    #[test]
    fn a_punch_from_where_the_fighters_start_falls_short() {
        let mut game = Game::new(1);
        game.second_player_joins();
        steps(&mut game, 10);
        game.press(LEFT, Act::Cross);
        steps(&mut game, 45);
        assert_eq!(
            game.world.stats().contusion_events,
            0,
            "a cross from out of reach landed"
        );
    }

    /// Leaning back while the other fighter's punch comes, the player is
    /// not touched.
    #[test]
    fn leaning_back_makes_a_punch_fall_short() {
        let mut game = in_reach();
        game.hold(LEFT, Hold::Lean, true);
        steps(&mut game, 25);
        game.press(RIGHT, Act::Cross);
        steps(&mut game, 45);
        assert_eq!(
            game.world.stats().contusion_events,
            0,
            "the punch reached the leaning fighter"
        );
    }

    /// A fighter steps back no further than the ring's edge, and forward no
    /// nearer the other than fighters stand.
    #[test]
    fn the_ring_and_the_other_fighter_bound_a_fighters_steps() {
        let mut game = Game::new(1);
        game.second_player_joins();
        let middle = SCENARIO_WIDTH * 0.5;
        let height = rp::world_frame().height;
        let out = |game: &Game| (middle - game.world.fighter_stands(LEFT).unwrap()) / height;
        for _ in 0..5 {
            game.press(LEFT, Act::StepOut);
            steps(&mut game, 30);
            game.second_player_joins();
        }
        assert!(
            out(&game) <= RING_HALF && out(&game) > RING_HALF - STEP,
            "stepping back five times, the fighter stands {:.2} body heights from the middle",
            out(&game)
        );
        for _ in 0..8 {
            game.press(LEFT, Act::StepIn);
            steps(&mut game, 30);
            game.second_player_joins();
        }
        assert!(
            (0.49..0.53).contains(&game.apart()),
            "stepping in eight times, the fighters stand {:.2} body heights apart",
            game.apart()
        );
    }

    /// Left alone, the right fighter spars: it comes into reach and lands a
    /// punch on a player who does nothing.
    #[test]
    fn the_sparring_partner_comes_in_and_punches() {
        let mut game = Game::new(5);
        let started = game.apart();
        let mut nearest = started;
        for _ in 0..12 * 60 {
            game.step();
            nearest = nearest.min(game.apart());
            if game.world.stats().contusion_events > 0 {
                break;
            }
        }
        assert!(
            nearest < started - 0.1,
            "the sparring partner never stepped in from {started:.2} body heights"
        );
        assert!(
            game.world.stats().contusion_events > 0,
            "the sparring partner landed nothing in twelve seconds"
        );
    }

    /// The engine's settings with the knockout limit lowered so that a
    /// landed cross knocks a fighter out, as the engine's own knockout tests
    /// lower it: at the real limit knockouts are rare, as in real fights.
    fn easy_knockouts() -> rp::Materials {
        let mut materials = rp::Materials::default();
        materials.knockout_turn = 10.0;
        materials
    }

    /// The player's cross knocks out the other fighter, and the fight is
    /// over: the right fighter is down, knocked out, and stays so.
    #[test]
    fn a_knockout_ends_the_fight() {
        let mut game = Game::with_materials(1, easy_knockouts());
        game.second_player_joins();
        game.press(LEFT, Act::StepIn);
        steps(&mut game, 40);
        game.second_player_joins();
        game.press(LEFT, Act::Cross);
        steps(&mut game, 60);
        let knocked_out = Ending {
            down: RIGHT,
            knocked_out: true,
        };
        assert_eq!(
            game.ending().map(|(ending, _)| ending),
            Some(knocked_out),
            "the cross did not end the fight"
        );
        game.second_player_joins();
        steps(&mut game, 120);
        assert_eq!(
            game.ending().map(|(ending, _)| ending),
            Some(knocked_out),
            "the fight's ending changed"
        );
        assert!(game.world.fighter_fallen(RIGHT));
    }

    /// Once the sparring partner has knocked the player out it stops: it
    /// lets go of whatever it held, takes no more steps, and throws nothing.
    #[test]
    fn the_sparring_partner_stops_once_a_fighter_is_down() {
        let mut game = Game::with_materials(5, easy_knockouts());
        for _ in 0..15 * 60 {
            game.step();
            if game.ending().is_some() {
                break;
            }
        }
        let Some((ending, _)) = game.ending() else {
            panic!("the sparring partner knocked nobody out in fifteen seconds");
        };
        assert_eq!(
            ending,
            Ending {
                down: LEFT,
                knocked_out: true
            }
        );
        // A step it was in the middle of finishes.
        steps(&mut game, 50);
        let stands = game.world.fighter_stands(RIGHT);
        for step in 0..180 {
            game.step();
            assert_eq!(game.holds[RIGHT], [false; 3], "it held something");
            assert_ne!(
                game.world.striking_body(),
                Some(RIGHT),
                "it threw a punch {step} steps on"
            );
        }
        assert_eq!(
            game.world.fighter_stands(RIGHT),
            stands,
            "it stepped once the player was down"
        );
    }

    /// A second player takes the right fighter by pressing its keys, and
    /// the sparring partner takes it back once they rest long enough.
    #[test]
    fn a_second_player_takes_over_and_hands_back() {
        let mut game = Game::new(1);
        assert!(!game.second_player_plays());
        game.hold(RIGHT, Hold::Block, true);
        assert!(game.second_player_plays());
        game.hold(RIGHT, Hold::Block, false);
        steps(&mut game, (SECOND_PLAYER_IDLE * 60.0) as usize + 1);
        assert!(!game.second_player_plays());
    }

    /// The page shows the newest entry of CHANGELOG.md under this version's
    /// label, so a bump needs its entry: `## v0.1.1 (2026-10-19)` for 0.1.1,
    /// `## v0.2 (...)` for 0.2.0, as `tools/build_web.ps1` reads it.
    #[test]
    fn the_version_reads_as_the_page_shows_it() {
        let label = version_label();
        let heading = include_str!("../CHANGELOG.md")
            .lines()
            .find(|line| line.starts_with("## "))
            .unwrap_or_default();
        assert!(
            heading.starts_with(&format!("## {label} (")),
            "CHANGELOG.md should start with an entry for {label}, not `{heading}`"
        );
    }
}
