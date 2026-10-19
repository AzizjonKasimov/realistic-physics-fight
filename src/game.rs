//! A fight: two fighters on the engine, side by side as the sandbox stands
//! them, the left fighter driven by its player's punch and lean back, and
//! the right by a second player's, or by the sparring partner while nobody
//! plays it. Nobody wins yet: blows hurt the fighters as they hurt bodies in
//! the sandbox, and a fight goes on until it is started afresh.

use realistic_physics as rp;
use rp::scenarios::SCENARIO_WIDTH;

use crate::camera::Camera;
use crate::page::{self, Moment};
use crate::sparring::Sparring;

/// The fighter standing on the left, whom a player alone plays.
pub const LEFT: usize = 0;
/// The fighter standing on the right, a second player's or the sparring
/// partner's.
pub const RIGHT: usize = 1;

/// How long the right fighter's keys and buttons may rest before the
/// sparring partner takes it back from a second player, in seconds.
pub const SECOND_PLAYER_IDLE: f64 = 10.0;

/// The game's version as players see it: v0.1 for 0.1.0, v0.1.2 for 0.1.2.
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
    /// How much room the screen leaves each fighter, following them.
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
    /// A punch each fighter's player asked for since the last step.
    punch: [bool; 2],
    /// Whether each fighter's player holds lean back.
    lean: [bool; 2],
    sparring: Sparring,
    /// How long since a second player last pressed the right fighter's keys
    /// or buttons, in seconds; none while nobody has.
    second_player: Option<f64>,
    /// What the next fight's sparring partner's waits follow from.
    seed: u64,
}

impl Game {
    /// Two fighters ready to fight, the sparring partner's waits following
    /// from `seed`.
    pub fn new(seed: u64) -> Game {
        let world = new_fighters();
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
            punch: [false; 2],
            lean: [false; 2],
            sparring: Sparring::new(seed),
            second_player: None,
            seed,
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

    /// `fighter`'s player asks for a punch, thrown on the next step.
    pub fn punch(&mut self, fighter: usize) {
        self.punch[fighter] = true;
        self.played(fighter);
        if fighter == LEFT {
            page::tell(Moment::Punch);
        }
    }

    /// Whether `fighter`'s player holds lean back now.
    pub fn hold_lean(&mut self, fighter: usize, held: bool) {
        self.lean[fighter] = held;
        if held {
            self.played(fighter);
            if fighter == LEFT {
                page::tell(Moment::Lean);
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
            self.lean[RIGHT] = false;
            if self.sparring.tick(dt) {
                self.punch[RIGHT] = true;
            }
        }
        for fighter in [LEFT, RIGHT] {
            self.world.hold_lean(fighter, self.lean[fighter]);
            if std::mem::take(&mut self.punch[fighter]) {
                let arm = self.world.inner_arm(fighter);
                self.world.throw_punch(fighter, arm);
            }
        }
        self.world.capture_motion(&mut self.earlier);
        self.world
            .step(dt, &rp::InputState::default(), SCENARIO_WIDTH);
        self.rim.refresh(&self.world);
        self.camera.follow(&self.world, dt);
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

/// Two unhurt fighters as the sandbox stands them, the right built as the
/// left's mirror image.
fn new_fighters() -> rp::World {
    rp::create_fighters_at(rp::Materials::default(), rp::FIGHTER_SPACING)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(game: &mut Game, count: usize) {
        for _ in 0..count {
            game.step();
        }
    }

    /// The player's punch lands on the other fighter and bruises it.
    #[test]
    fn the_players_punch_lands() {
        let mut game = Game::new(1);
        game.second_player_joins();
        steps(&mut game, 10);
        game.punch(LEFT);
        steps(&mut game, 50);
        assert!(
            game.world.stats().contusion_events > 0,
            "the punch bruised nothing"
        );
    }

    /// Leaning back while the other fighter's punch comes, the player is not
    /// touched.
    #[test]
    fn leaning_back_makes_a_punch_fall_short() {
        let mut game = Game::new(1);
        game.second_player_joins();
        game.hold_lean(LEFT, true);
        steps(&mut game, 20);
        game.punch(RIGHT);
        steps(&mut game, 50);
        assert_eq!(
            game.world.stats().contusion_events,
            0,
            "the punch reached the leaning fighter"
        );
    }

    /// Left alone, the right fighter spars: it throws a punch within the
    /// longest wait, by its own arm nearer the player.
    #[test]
    fn the_sparring_partner_punches_on_its_own() {
        let mut game = Game::new(5);
        let arm = game.world.inner_arm(RIGHT);
        let mut punched = false;
        for _ in 0..(crate::sparring::PUNCH_GAP.1 * 60.0) as usize + 2 {
            game.step();
            punched |= game.world.punch(RIGHT, arm).is_some();
        }
        assert!(punched, "the sparring partner threw no punch");
    }

    /// A second player takes the right fighter by pressing its keys, and the
    /// sparring partner takes it back once they rest long enough.
    #[test]
    fn a_second_player_takes_over_and_hands_back() {
        let mut game = Game::new(1);
        assert!(!game.second_player_plays());
        game.hold_lean(RIGHT, true);
        assert!(game.second_player_plays());
        game.hold_lean(RIGHT, false);
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
