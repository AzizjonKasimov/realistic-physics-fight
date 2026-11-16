//! Whole bouts played without a window against scripted players, for the
//! tests and for tuning the computer opponent: what each side threw and
//! landed, how the opponent answered, and how the bout ended.
//!
//! `cargo test --release print_bouts -- --ignored --nocapture` plays every
//! level against every scripted player and prints a table; `RP_BOUTS=N`
//! plays N seeds of each (default 2), `RP_ROUND=S` makes the bouts one
//! round S seconds long (default 30), and `RP_LEVEL` / `RP_PLAYER` keep one
//! level or player.

use realistic_physics as rp;

use crate::bout::{self, Outcome, Rules};
use crate::controls::{Act, Hold};
use crate::game::{Game, LEFT, RIGHT};
use crate::opponent::{Level, CLOSE, REACH};

/// A scripted player for the left fighter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// Stands in its stance and does nothing.
    Idle,
    /// Steps into reach and jabs about once a second, its guard never up.
    Jabber,
    /// Steps into reach, jabs and crosses, keeps its guard up between, and
    /// steps back out now and then.
    Boxer,
    /// Steps into reach and keeps its guard up, never punching.
    Blocker,
}

impl Player {
    pub const ALL: [Player; 4] = [Player::Idle, Player::Jabber, Player::Boxer, Player::Blocker];
}

/// A scripted player's mind: its own seeded draws, and the seconds until it
/// next decides.
struct Script {
    player: Player,
    state: u64,
    wait: f64,
    guard: f64,
}

impl Script {
    fn new(player: Player, seed: u64) -> Script {
        Script {
            player,
            state: seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1,
            wait: 0.5,
            guard: 0.0,
        }
    }

    fn unit(&mut self) -> f64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        (self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// What it presses and holds on a step `dt` long, the fighters `apart`.
    fn play(&mut self, game: &mut Game, dt: f64) {
        let apart = game.apart();
        let doing = game.world.fighter_doing(LEFT).unwrap_or_default();
        self.wait -= dt;
        self.guard -= dt;
        let busy = doing.step.is_some() || doing.punch.is_some();
        let guarding = matches!(self.player, Player::Blocker)
            || (self.player == Player::Boxer && self.guard > 0.0);
        game.hold(LEFT, Hold::Block, guarding);
        if self.player == Player::Idle || busy || self.wait > 0.0 {
            return;
        }
        if apart > REACH {
            game.press(LEFT, Act::StepIn);
            self.wait = 0.2;
            return;
        }
        match self.player {
            Player::Idle | Player::Blocker => {}
            Player::Jabber => {
                game.press(LEFT, Act::Jab);
                self.wait = 0.8 + 0.4 * self.unit();
            }
            Player::Boxer => {
                let draw = self.unit();
                if apart < CLOSE && draw < 0.15 {
                    game.press(LEFT, Act::StepOut);
                } else if draw < 0.55 {
                    game.press(LEFT, Act::Jab);
                } else {
                    game.press(LEFT, Act::Cross);
                }
                self.wait = 0.5 + 0.7 * self.unit();
                self.guard = self.wait;
            }
        }
    }
}

/// What a bout came to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    /// Punches each fighter threw, and landed clean and on the arms.
    pub thrown: [u32; 2],
    pub clean: [u32; 2],
    pub blocked: [u32; 2],
    /// Punches the opponent threw within half a second of one of the
    /// player's coming back: its counters.
    pub counters: u32,
    /// Seconds the opponent spent blocking, and in reach of the player.
    pub guarding: f64,
    pub in_reach: f64,
    pub outcome: Option<Outcome>,
    /// A fighter fell with no knockout.
    pub fell: bool,
}

/// Plays a bout of `rules` against an opponent of `level`, its decisions
/// following from `seed`, with `player` playing the left fighter.
pub fn play_bout(level: Level, player: Player, seed: u64, rules: Rules) -> Tally {
    let mut game = Game::with(seed, level, rules, rp::Materials::default());
    let mut script = Script::new(player, seed ^ 0x5EED);
    let dt = game.world.materials().fixed_dt;
    let mut tally = Tally::default();
    let mut punching = [false; 2];
    let mut came_back: Option<f64> = None;
    let mut clock = 0.0;
    let length = rules.rounds as f64 * (rules.round_seconds + rules.break_seconds) + 1.0;
    while clock < length && game.outcome().is_none() {
        script.play(&mut game, dt);
        game.step();
        clock += dt;
        for fighter in [LEFT, RIGHT] {
            let doing = game.world.fighter_doing(fighter).unwrap_or_default();
            let now = doing.punch.is_some_and(|punch| punch.going_out);
            if now && !punching[fighter] {
                tally.thrown[fighter] += 1;
                if fighter == RIGHT && came_back.is_some_and(|at| clock - at < 0.5) {
                    tally.counters += 1;
                }
            }
            if fighter == LEFT && punching[LEFT] && !now {
                came_back = Some(clock);
            }
            punching[fighter] = now;
            if fighter == RIGHT && doing.guarded {
                tally.guarding += dt;
            }
        }
        if game.apart() <= REACH {
            tally.in_reach += dt;
        }
        for punch in game.world.landed_punches() {
            if bout::clean(punch.struck) {
                tally.clean[punch.by] += 1;
            } else if punch.struck == rp::Struck::Arm {
                tally.blocked[punch.by] += 1;
            }
        }
    }
    tally.outcome = game.outcome().map(|(outcome, _)| outcome);
    tally.fell = matches!(
        tally.outcome,
        Some(Outcome::Down {
            knocked_out: false,
            ..
        })
    );
    tally
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::Stage;

    /// A bout of one round of fifteen seconds.
    fn short_bout(level: Level, player: Player, seed: u64) -> Tally {
        let rules = Rules {
            rounds: 1,
            round_seconds: 15.0,
            break_seconds: 1.0,
        };
        play_bout(level, player, seed, rules)
    }

    /// Against a player who does nothing, the opponent comes into reach and
    /// wins on clean punches, landing most of what it throws, and nobody
    /// falls.
    #[test]
    fn the_opponent_beats_a_player_who_does_nothing() {
        let tally = short_bout(Level::Normal, Player::Idle, 1);
        assert!(
            tally.clean[RIGHT] >= 5 && tally.clean[LEFT] == 0,
            "{tally:?}"
        );
        assert_eq!(
            tally.outcome,
            Some(Outcome::Decision {
                winner: Some(RIGHT)
            })
        );
        assert!(!tally.fell);
    }

    /// Against a player who jabs about once a second and never covers up,
    /// the opponent punches back into the guard each jab opens, covers up
    /// some of the time, and wins.
    #[test]
    fn the_opponent_counters_and_covers_up() {
        let tally = short_bout(Level::Normal, Player::Jabber, 1);
        assert!(tally.counters >= 4, "{tally:?}");
        assert!(tally.guarding > 0.15 * tally.in_reach, "{tally:?}");
        assert!(tally.clean[RIGHT] > tally.clean[LEFT], "{tally:?}");
        assert!(!tally.fell);
    }

    /// A careful player, whose guard is up but for its own punches, beats
    /// the easy opponent, and the hard opponent lands far more on it.
    #[test]
    fn a_careful_player_beats_easy_and_hard_hits_it_harder() {
        let (easy, hard) = (
            short_bout(Level::Easy, Player::Boxer, 1),
            short_bout(Level::Hard, Player::Boxer, 1),
        );
        assert!(easy.clean[LEFT] > easy.clean[RIGHT], "easy: {easy:?}");
        assert!(
            hard.clean[RIGHT] >= easy.clean[RIGHT] + 3,
            "easy: {easy:?}
hard: {hard:?}"
        );
        assert!(!easy.fell && !hard.fell);
    }

    /// A bout of two short rounds: the bell ends the first; in the rest the
    /// fighters throw nothing and walk back to their corners, out of each
    /// other's reach; the second round begins; and the last bell gives a
    /// decision on clean punches.
    #[test]
    fn rounds_end_and_the_fighters_go_back_to_their_corners() {
        let rules = Rules {
            rounds: 2,
            round_seconds: 6.0,
            break_seconds: 4.0,
        };
        let mut game = Game::with(5, Level::Normal, rules, rp::Materials::default());
        let mut script = Script::new(Player::Jabber, 5);
        let dt = game.world.materials().fixed_dt;
        let (mut nearest_in_round, mut apart_at_bell) = (f64::INFINITY, None);
        let mut punched_in_rest = false;
        let mut was_punching = [false; 2];
        for _ in 0..(18.0 / dt) as usize {
            script.play(&mut game, dt);
            game.step();
            let stage = game.bout.stage();
            for fighter in [LEFT, RIGHT] {
                let punching = game
                    .world
                    .fighter_doing(fighter)
                    .is_some_and(|doing| doing.punch.is_some());
                if punching && !was_punching[fighter] && matches!(stage, Stage::Break(_)) {
                    punched_in_rest = true;
                }
                was_punching[fighter] = punching;
            }
            match stage {
                Stage::Round(1) => nearest_in_round = nearest_in_round.min(game.apart()),
                Stage::Round(2) if apart_at_bell.is_none() => apart_at_bell = Some(game.apart()),
                _ => {}
            }
        }
        assert!(
            nearest_in_round < REACH,
            "the fighters never came into reach"
        );
        let apart = apart_at_bell.expect("the second round never began");
        assert!(
            apart > REACH,
            "the second round began with the fighters {apart:.2} body heights apart"
        );
        assert!(!punched_in_rest, "a punch was thrown in the rest");
        assert!(
            matches!(game.outcome(), Some((Outcome::Decision { .. }, _))),
            "{:?}",
            game.outcome()
        );
    }

    fn env_number(name: &str, default: f64) -> f64 {
        std::env::var(name)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(default)
    }

    /// Prints what every level does against every scripted player: punches
    /// thrown, landed clean, and on the arms, by each side; the opponent's
    /// counters, its share of time in reach spent blocking; and how the
    /// bouts ended.
    #[test]
    #[ignore]
    fn print_bouts() {
        let bouts = env_number("RP_BOUTS", 2.0) as u64;
        let round = env_number("RP_ROUND", 30.0);
        let rules = Rules {
            rounds: 1,
            round_seconds: round,
            break_seconds: 1.0,
        };
        let only_level = std::env::var("RP_LEVEL").ok();
        let only_player = std::env::var("RP_PLAYER").ok();
        let mut jobs = Vec::new();
        for level in Level::ALL {
            if only_level
                .as_deref()
                .is_some_and(|name| name != level.name())
            {
                continue;
            }
            for player in Player::ALL {
                let name = format!("{player:?}").to_lowercase();
                if only_player.as_deref().is_some_and(|wanted| wanted != name) {
                    continue;
                }
                for seed in 1..=bouts {
                    jobs.push((level, player, seed));
                }
            }
        }
        let results: Vec<(Level, Player, u64, Tally)> = std::thread::scope(|scope| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|&(level, player, seed)| {
                    scope
                        .spawn(move || (level, player, seed, play_bout(level, player, seed, rules)))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect()
        });
        println!(
            "{:<7} {:<8} {:>4}  {:>13} {:>13} {:>13} {:>8} {:>7}  outcome",
            "level",
            "player",
            "seed",
            "thrown P/O",
            "clean P/O",
            "on arms P/O",
            "counters",
            "guard%"
        );
        for (level, player, seed, tally) in results {
            println!(
                "{:<7} {:<8} {:>4}  {:>6}/{:<6} {:>6}/{:<6} {:>6}/{:<6} {:>8} {:>6.0}%  {:?}{}",
                level.name(),
                format!("{player:?}").to_lowercase(),
                seed,
                tally.thrown[0],
                tally.thrown[1],
                tally.clean[0],
                tally.clean[1],
                tally.blocked[0],
                tally.blocked[1],
                tally.counters,
                100.0 * tally.guarding / tally.in_reach.max(1e-9),
                tally.outcome,
                if tally.fell { "  FELL" } else { "" }
            );
        }
    }
}
