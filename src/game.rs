//! A fight: two fighters on the engine, seen from the side and facing each
//! other, the left one driven by its player's moves, and the right by a
//! second player's, or by the computer opponent while nobody plays it
//! (`opponent.rs`). The moves are the engine's: a jab, a cross, a step in
//! and a step back, and, held, a block, a duck, and a lean back. A punch
//! goes through the other fighter's open hands and lands on its head; a
//! held block stops it on the forearms. Blows hurt the fighters as they
//! hurt bodies in the sandbox, and a blow that turns the head hard enough
//! knocks a fighter out (the engine's `knockout.rs`): it goes down and
//! stays down. The fight is a bout of rounds (`bout.rs`): a knockout or a
//! fall ends it at once, and otherwise more clean punches wins when the
//! last round ends. Between rounds both fighters go back to their corners.
//! A fight is started afresh by hand.

use realistic_physics as rp;
use rp::scenarios::SCENARIO_WIDTH;

use crate::bout::{self, Bell, Bout, Outcome, Rules};
use crate::camera::Camera;
use crate::controls::{Act, Hold};
use crate::opponent::{Itself, Level, Opponent, Sight};
use crate::page::{self, Moment};

/// The fighter standing on the left, whom a player alone plays.
pub const LEFT: usize = 0;
/// The fighter standing on the right, a second player's or the computer
/// opponent's.
pub const RIGHT: usize = 1;

/// How long the right fighter's keys may rest before the computer opponent
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
    opponent: Opponent,
    /// How long since a second player last pressed the right fighter's
    /// keys, in seconds; none while nobody has.
    second_player: Option<f64>,
    /// What the next fight's opponent's decisions follow from.
    seed: u64,
    /// The rounds, the clean punches, and how the fight ended.
    pub bout: Bout,
    /// Where each fighter stood as the fight began, across the world: its
    /// corner, where it goes back between rounds.
    corners: [f64; 2],
    /// The settings the world was built on, for the next fight.
    materials: rp::Materials,
}

impl Game {
    /// Two fighters ready to fight a bout against an opponent of `level`,
    /// its decisions following from `seed`.
    pub fn new(seed: u64, level: Level) -> Game {
        Game::with(seed, level, Rules::default(), rp::Materials::default())
    }

    /// A fight of `rules` on the engine's settings `materials`: for tests
    /// and films with short rounds, and tests that lower the knockout limit
    /// as the engine's own tests do.
    pub fn with(seed: u64, level: Level, rules: Rules, materials: rp::Materials) -> Game {
        let world = new_fighters(materials.clone());
        let mut earlier = rp::MotionSnapshot::default();
        world.capture_motion(&mut earlier);
        let corners = [LEFT, RIGHT].map(|fighter| world.fighter_stands(fighter).unwrap_or(0.0));
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
            opponent: Opponent::new(level, seed),
            second_player: None,
            seed,
            bout: Bout::new(rules),
            corners,
            materials,
        }
    }

    /// How the fight ended and how many seconds ago, once it has.
    pub fn outcome(&self) -> Option<(Outcome, f64)> {
        self.bout.outcome()
    }

    /// The computer opponent's level.
    pub fn level(&self) -> Level {
        self.opponent.level()
    }

    /// The fight afresh: unhurt fighters where they first stood, the first
    /// round to come, against an opponent of `level`. A second player
    /// playing keeps playing.
    pub fn restart_at(&mut self, level: Level) {
        let second_player = self.second_player;
        let muted = self.muted;
        let (rules, materials) = (self.bout.rules, self.materials.clone());
        *self = Game::with(self.seed.wrapping_add(1), level, rules, materials);
        self.second_player = second_player;
        self.muted = muted;
        page::tell(Moment::Restart);
    }

    /// The fight afresh against an opponent of the same level.
    pub fn restart(&mut self) {
        self.restart_at(self.level());
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
    /// computer opponent.
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

    /// What the computer opponent sees of the fight now: how far apart the
    /// fighters stand, what the left fighter is doing, and which of its
    /// punches landed on the right fighter on the last step.
    fn sight(&self) -> Sight {
        let landed = self.world.landed_punches();
        let on_it = |clean: bool| {
            landed
                .iter()
                .any(|punch| punch.on == RIGHT && bout::clean(punch.struck) == clean)
        };
        Sight {
            apart: self.apart(),
            them: self.world.fighter_doing(LEFT).unwrap_or_default(),
            hit: on_it(true),
            blocked: on_it(false),
        }
    }

    /// Between rounds both fighters go back to their corners, a step at a
    /// time, and stand there with their guards let go of; nobody's moves
    /// count.
    fn walk_to_corners(&mut self) {
        for fighter in [LEFT, RIGHT] {
            self.acts[fighter] = [false; 4];
            self.holds[fighter] = [false; 3];
            let (Some(stands), Some(doing)) = (
                self.world.fighter_stands(fighter),
                self.world.fighter_doing(fighter),
            ) else {
                continue;
            };
            if doing.step.is_some() || doing.lean > 0.2 || doing.duck > 0.2 {
                continue;
            }
            let height = rp::world_frame().height;
            let middle = SCENARIO_WIDTH * 0.5;
            // How far nearer the ring's middle than its corner it stands.
            let ahead = ((self.corners[fighter] - middle).abs() - (stands - middle).abs()) / height;
            if ahead > 0.5 * STEP {
                self.acts[fighter][Act::StepOut.index()] = true;
            } else if ahead < -0.5 * STEP {
                self.acts[fighter][Act::StepIn.index()] = true;
            }
        }
    }

    /// One simulation step: the players' and the computer opponent's moves,
    /// then the world moving on, the bout's clock and count, and the camera
    /// following it.
    pub fn step(&mut self) {
        let dt = self.world.materials().fixed_dt;
        if let Some(idle) = &mut self.second_player {
            *idle += dt;
        }
        if matches!(self.bout.stage(), bout::Stage::Break(_)) {
            self.walk_to_corners();
        } else if !self.second_player_plays() {
            // Once the fight is over the opponent stops, its guard let go
            // of, and stands in its stance.
            if self.bout.outcome().is_some() {
                self.holds[RIGHT] = [false; 3];
            } else {
                let me = Itself {
                    doing: self.world.fighter_doing(RIGHT).unwrap_or_default(),
                    room_behind: self.room_behind(RIGHT),
                };
                let wish = self.opponent.tick(dt, self.sight(), me);
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
            // A punch opens the guard until it is back, for either fighter:
            // the forearm that stays up beside a punch otherwise stopped
            // every answer, and blocking while punching beat every level of
            // the opponent, which drops its guard to punch.
            let punching = self.acts[fighter][Act::Jab.index()]
                || self.acts[fighter][Act::Cross.index()]
                || self
                    .world
                    .fighter_doing(fighter)
                    .is_some_and(|doing| doing.punch.is_some());
            for hold in Hold::ALL {
                let held = self.holds[fighter][hold.index()] && !(hold == Hold::Block && punching);
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
        // Should both go down on one step, the left fighter's fall is the
        // one that counts.
        let down = [LEFT, RIGHT]
            .into_iter()
            .find(|&fighter| self.world.fighter_fallen(fighter))
            .map(|fighter| (fighter, self.world.knocked_out(fighter)));
        let bell = self.bout.tick(dt, self.world.landed_punches(), down);
        match bell {
            Some(Bell::BoutOver) => {
                if matches!(
                    self.bout.outcome(),
                    Some((
                        Outcome::Down {
                            knocked_out: true,
                            ..
                        },
                        _
                    ))
                ) {
                    page::tell(Moment::Knockout);
                }
            }
            Some(Bell::RoundBegins(_)) => self.opponent.new_round(),
            Some(Bell::RoundOver(_)) | None => {}
        }
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
        let mut game = Game::new(1, Level::Normal);
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
        let mut game = Game::new(1, Level::Normal);
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
        let mut game = Game::new(1, Level::Normal);
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

    /// Left alone, the right fighter is the computer opponent's: it comes
    /// into reach and lands a punch on a player who does nothing.
    #[test]
    fn the_opponent_comes_in_and_punches() {
        let mut game = Game::new(5, Level::Normal);
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
            "the opponent never stepped in from {started:.2} body heights"
        );
        assert!(
            game.world.stats().contusion_events > 0,
            "the opponent landed nothing in twelve seconds"
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

    /// A fight against an opponent of `level` whose decisions follow from
    /// `seed`, with the knockout limit lowered (`easy_knockouts`).
    fn knockouts_easy(seed: u64) -> Game {
        Game::with(seed, Level::Normal, Rules::default(), easy_knockouts())
    }

    /// The player's cross knocks out the other fighter, and the fight is
    /// over: the right fighter is down, knocked out, and stays so.
    #[test]
    fn a_knockout_ends_the_fight() {
        let mut game = knockouts_easy(1);
        game.second_player_joins();
        game.press(LEFT, Act::StepIn);
        steps(&mut game, 40);
        game.second_player_joins();
        game.press(LEFT, Act::Cross);
        steps(&mut game, 60);
        let knocked_out = Outcome::Down {
            down: RIGHT,
            knocked_out: true,
            round: 1,
        };
        assert_eq!(
            game.outcome().map(|(outcome, _)| outcome),
            Some(knocked_out),
            "the cross did not end the fight"
        );
        game.second_player_joins();
        steps(&mut game, 120);
        assert_eq!(
            game.outcome().map(|(outcome, _)| outcome),
            Some(knocked_out),
            "the fight's ending changed"
        );
        assert!(game.world.fighter_fallen(RIGHT));
    }

    /// Once the opponent has knocked the player out it stops: it lets go of
    /// whatever it held, takes no more steps, and throws nothing.
    #[test]
    fn the_opponent_stops_once_a_fighter_is_down() {
        let mut game = knockouts_easy(5);
        for _ in 0..15 * 60 {
            game.step();
            if game.outcome().is_some() {
                break;
            }
        }
        let Some((outcome, _)) = game.outcome() else {
            panic!("the opponent knocked nobody out in fifteen seconds");
        };
        assert!(
            matches!(
                outcome,
                Outcome::Down {
                    down: LEFT,
                    knocked_out: true,
                    ..
                }
            ),
            "{outcome:?}"
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
    /// the computer opponent takes it back once they rest long enough.
    #[test]
    fn a_second_player_takes_over_and_hands_back() {
        let mut game = Game::new(1, Level::Normal);
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
