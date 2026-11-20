//! A fight: two fighters on the engine, seen from the side and facing each
//! other, the left one driven by its player's moves, and the right by a
//! second player's, or by the computer opponent while nobody plays it
//! (`opponent.rs`). The moves are the engine's: a jab, a cross, a step in
//! and a step back, and, held, a block, a duck, and a lean back. A punch
//! goes through the other fighter's open hands and lands on its head; a
//! held block stops it on the forearms. Blows hurt the fighters as they
//! hurt bodies in the sandbox, and a blow that turns the head hard enough
//! knocks a fighter out (the engine's `knockout.rs`): it goes down, lies
//! limp for longer the harder the blow, and gets up on its own, as a fighter
//! that falls does (the engine's `rise.rs`). The fight is a bout of rounds
//! (`bout.rs`): a fighter knocked down gets a referee's count while the
//! other goes back to its corner, and is knocked out if still down at ten;
//! three knockdowns end the bout; otherwise more clean punches wins when the
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

/// Something a fighter's player feels through a controller that rumbles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feel {
    /// The fighter's punch landed clean, on the head or the body.
    Landed(usize),
    /// The fighter was hit clean.
    Hit(usize),
    /// The fighter was knocked down.
    Down(usize),
}

impl Feel {
    /// The fighter whose player feels it.
    pub fn fighter(self) -> usize {
        match self {
            Feel::Landed(fighter) | Feel::Hit(fighter) | Feel::Down(fighter) => fighter,
        }
    }
}

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
    /// The step each fighter's player holds on a controller, which it keeps
    /// taking, a step at a time, while it is held.
    walking: [Option<Act>; 2],
    /// What the fighters' players have felt since the last frame took it.
    feels: Vec<Feel>,
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
            walking: [None; 2],
            feels: Vec::new(),
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

    /// Whether `fighter`'s player holds a step on a controller, toward the
    /// other fighter or away from it: the fighter keeps stepping that way, a
    /// step at a time, while it is held (toward wins if both are).
    pub fn walk(&mut self, fighter: usize, toward: bool, away: bool) {
        self.walking[fighter] = if toward {
            Some(Act::StepIn)
        } else if away {
            Some(Act::StepOut)
        } else {
            None
        };
        if self.walking[fighter].is_some() {
            self.played(fighter);
        }
    }

    /// What the fighters' players felt since this was last asked: punches
    /// landed and taken clean, and knockdowns, for the controllers to rumble.
    pub fn take_feels(&mut self) -> Vec<Feel> {
        std::mem::take(&mut self.feels)
    }

    /// Each fighter walking on a controller takes its next step once the
    /// last is done, unless a step back would leave the ring.
    fn keep_walking(&mut self) {
        for fighter in [LEFT, RIGHT] {
            let Some(act) = self.walking[fighter] else {
                continue;
            };
            let free = self
                .world
                .fighter_doing(fighter)
                .is_some_and(|doing| doing.step.is_none() && !doing.fallen);
            let pressed =
                self.acts[fighter][Act::StepIn.index()] || self.acts[fighter][Act::StepOut.index()];
            if free && !pressed && (act == Act::StepIn || self.room_behind(fighter)) {
                self.press(fighter, act);
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
            hit_body: landed
                .iter()
                .any(|punch| punch.on == RIGHT && punch.struck == rp::Struck::Body),
            their_tiredness: self.tiredness(LEFT),
        }
    }

    /// How much of `fighter`'s arms' strength it has lost to tiring, from 0
    /// fresh: what the bar under its tag shows (the engine's `fatigue.rs`).
    pub fn tiredness(&self, fighter: usize) -> f64 {
        self.world
            .fighter_stamina(fighter)
            .map_or(0.0, |stamina| 1.0 - stamina.both())
    }

    /// Between rounds both fighters go back to their corners, a step at a
    /// time, and stand there with their guards let go of; nobody's moves
    /// count.
    fn walk_to_corners(&mut self) {
        for fighter in [LEFT, RIGHT] {
            self.walk_to(fighter, self.corners[fighter]);
        }
    }

    /// While the referee counts over the other fighter, `fighter` goes back
    /// to the ring's edge on its side, as a boxer goes to a neutral corner,
    /// out of the way of the fighter getting up. Sent to where it began the
    /// fight instead, a step out of the other's reach, it stood over the
    /// fighter down, which got up under it.
    fn walk_to_neutral_corner(&mut self, fighter: usize) {
        let side = if fighter == LEFT { -1.0 } else { 1.0 };
        let edge =
            SCENARIO_WIDTH * 0.5 + side * (RING_HALF - 0.5 * STEP) * rp::world_frame().height;
        self.walk_to(fighter, edge);
    }

    /// `fighter` goes to `place` across the world, a step at a time, and
    /// stands there with its guard let go of; its player's moves do not
    /// count.
    fn walk_to(&mut self, fighter: usize, place: f64) {
        self.acts[fighter] = [false; 4];
        self.holds[fighter] = [false; 3];
        let (Some(stands), Some(doing)) = (
            self.world.fighter_stands(fighter),
            self.world.fighter_doing(fighter),
        ) else {
            return;
        };
        if doing.fallen || doing.step.is_some() || doing.lean > 0.2 || doing.duck > 0.2 {
            return;
        }
        let height = rp::world_frame().height;
        let middle = SCENARIO_WIDTH * 0.5;
        // How far nearer the ring's middle than that place it stands.
        let ahead = ((place - middle).abs() - (stands - middle).abs()) / height;
        if ahead > 0.5 * STEP {
            self.acts[fighter][Act::StepOut.index()] = true;
        } else if ahead < -0.5 * STEP {
            self.acts[fighter][Act::StepIn.index()] = true;
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
        self.keep_walking();
        if matches!(self.bout.stage(), bout::Stage::Break(_)) {
            self.walk_to_corners();
        } else if let Some(count) = self.bout.count() {
            // The referee counts over the fighter down, which gets up on its
            // own, while the other waits in its corner.
            self.acts[count.down] = [false; 4];
            self.holds[count.down] = [false; 3];
            self.walk_to_neutral_corner(1 - count.down);
        } else if !self.second_player_plays() {
            // Once the fight is over the opponent stops, its guard let go
            // of, and stands in its stance.
            if self.bout.outcome().is_some() {
                self.holds[RIGHT] = [false; 3];
            } else {
                let me = Itself {
                    doing: self.world.fighter_doing(RIGHT).unwrap_or_default(),
                    room_behind: self.room_behind(RIGHT),
                    tiredness: self.tiredness(RIGHT),
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
        let down = [LEFT, RIGHT].map(|fighter| self.world.fighter_fallen(fighter));
        for punch in self.world.landed_punches() {
            if bout::clean(punch.struck) {
                self.feels
                    .extend([Feel::Landed(punch.by), Feel::Hit(punch.on)]);
            }
        }
        let bell = self.bout.tick(dt, self.world.landed_punches(), down);
        match bell {
            Some(Bell::BoutOver) => {
                if matches!(self.bout.outcome(), Some((Outcome::KnockedOut { .. }, _))) {
                    page::tell(Moment::Knockout);
                }
            }
            // The fight goes on after a count, or a round begins: the
            // opponent starts afresh.
            Some(Bell::RoundBegins(_) | Bell::Box) => self.opponent.new_round(),
            Some(Bell::Knockdown(fighter)) => self.feels.push(Feel::Down(fighter)),
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

    /// The player's cross knocks the other fighter down, knocked out: the
    /// referee counts over it while the player goes back to its corner,
    /// moving on its own, whatever its keys say, and throws nothing at it.
    /// It gets up, and the fight goes on once the count has reached eight;
    /// the knockdown is counted against it.
    #[test]
    fn a_knockdown_is_counted_and_the_fighter_gets_up() {
        let mut game = knockouts_easy(1);
        game.second_player_joins();
        game.press(LEFT, Act::StepIn);
        steps(&mut game, 40);
        game.second_player_joins();
        let stood = game.world.fighter_stands(LEFT);
        game.press(LEFT, Act::Cross);
        let mut counted = false;
        for _ in 0..60 {
            game.step();
            counted |= game.bout.count().is_some_and(|count| count.down == RIGHT);
        }
        assert!(counted, "the cross knocked nobody down");
        assert!(
            game.world.knocked_out(RIGHT),
            "the right fighter is not out"
        );
        assert!(
            game.take_feels().contains(&Feel::Down(RIGHT)),
            "the knockdown was not felt"
        );
        let mut resumed = None;
        for step in 0..12 * 60 {
            // The player presses its punches while the other is down.
            if step % 20 == 0 {
                game.press(LEFT, Act::Cross);
                game.second_player_joins();
            }
            game.step();
            assert_ne!(
                game.world.striking_body(),
                Some(LEFT),
                "the player punched a fighter down, {step} steps on"
            );
            if game.bout.count().is_none() {
                resumed = Some(step);
                break;
            }
        }
        assert!(
            resumed.is_some_and(|step| step > 60) && !game.world.fighter_fallen(RIGHT),
            "the count ended on step {resumed:?}, the fighter down: {}",
            game.world.fighter_fallen(RIGHT)
        );
        assert!(
            game.world.fighter_stands(LEFT) != stood,
            "the player stayed where it stood during the count"
        );
        assert_eq!(game.bout.knockdowns(), [0, 1]);
        assert_eq!(game.outcome(), None, "the fight ended at the knockdown");
    }

    /// A fighter still down at the count of ten is knocked out and the
    /// fight is over; the opponent that knocked it out stops: it lets go of
    /// whatever it held and throws nothing more.
    #[test]
    fn still_down_at_ten_ends_the_fight_and_the_opponent_stops() {
        let mut materials = easy_knockouts();
        // Out for long enough that the count reaches ten.
        materials.knockout_seconds = 30.0;
        let mut game = Game::with(5, Level::Normal, Rules::default(), materials);
        for _ in 0..30 * 60 {
            game.step();
            if game.outcome().is_some() {
                break;
            }
        }
        let Some((outcome, _)) = game.outcome() else {
            panic!("the opponent knocked nobody out in thirty seconds");
        };
        assert!(
            matches!(outcome, Outcome::KnockedOut { down: LEFT, .. }),
            "{outcome:?}"
        );
        // A step it was in the middle of finishes.
        steps(&mut game, 50);
        for step in 0..180 {
            game.step();
            assert_eq!(game.holds[RIGHT], [false; 3], "it held something");
            assert_ne!(
                game.world.striking_body(),
                Some(RIGHT),
                "it threw a punch {step} steps on"
            );
        }
    }

    /// A clean punch is felt by both players' controllers: the one whose
    /// punch landed, and the one hit.
    #[test]
    fn a_clean_punch_is_felt_by_both_players() {
        let mut game = in_reach();
        game.take_feels();
        game.press(LEFT, Act::Cross);
        steps(&mut game, 45);
        let feels = game.take_feels();
        assert!(
            feels.contains(&Feel::Landed(LEFT)) && feels.contains(&Feel::Hit(RIGHT)),
            "{feels:?}"
        );
        assert_eq!(game.take_feels(), [], "felt twice");
    }

    /// A step held on a controller keeps stepping, a step at a time: back to
    /// the ring's edge, two steps behind, then in toward the other fighter;
    /// once let go, it takes no more.
    #[test]
    fn a_step_held_on_a_controller_keeps_stepping() {
        let mut game = Game::new(1, Level::Normal);
        // Holds a step for `count` steps and says how many it took.
        let walk = |game: &mut Game, toward: bool, away: bool, count: usize| {
            let (mut taken, mut stepping) = (0, false);
            for _ in 0..count {
                game.second_player_joins();
                game.walk(LEFT, toward, away);
                game.step();
                let now = game
                    .world
                    .fighter_doing(LEFT)
                    .is_some_and(|doing| doing.step.is_some());
                taken += usize::from(now && !stepping);
                stepping = now;
            }
            taken
        };
        let back = walk(&mut game, false, true, 240);
        assert_eq!(back, 2, "held back for four seconds, it took {back} steps");
        assert!(
            !game.room_behind(LEFT),
            "it stopped short of the ring's edge"
        );
        let toward = walk(&mut game, true, false, 240);
        assert!(
            toward >= 3,
            "held toward the other fighter for four seconds, it took {toward} steps"
        );
        let after = walk(&mut game, false, false, 120);
        assert_eq!(after, 0, "it kept stepping once let go");
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
