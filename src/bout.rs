//! A bout: three rounds of a minute each, with ten seconds between them,
//! the fighters back in their corners. A fighter knocked down by a punch
//! gets a referee's count, a second at a time, while the other waits in a
//! corner: up by ten, it fights on once the count has reached eight (the
//! mandatory eight count), and still down at ten it is knocked out and the
//! bout is over. A third knockdown ends the bout too, as amateur boxing's
//! referees stop a bout after three counts in a round. A fighter that goes
//! down with no clean punch landing on it just before slipped, and is not
//! counted (the Association of Boxing Commissions' referee manual: slips
//! and pushes are not knockdowns); it gets up on its own. The bell does not
//! save a fighter: a round that runs out during a count ends once the count
//! does. Otherwise, when the last round ends, the fighter who landed more
//! clean punches wins, as Olympic boxing's computer scoring counted them
//! from 1992 to 2012: a punch counts that lands on the head or the body, not
//! one stopped on the arms, and lands hard. Amateur boxing's rules count a
//! blow only with the weight of the body or the shoulder behind it, and the
//! counting machine was dropped in 2013 for rewarding light, quick touches;
//! a fighter's arms tire as it punches (the engine's `fatigue.rs`), and a
//! tired arm's punch, landing slower than `counting_speed` says, does not
//! count, nor one that just reaches from the end of the arm's way.
//! Equal counts are a draw. What a punch struck, and how fast, is the
//! engine's to say (`rp::World::landed_punches`), and whether a fighter is
//! down (`rp::World::fighter_fallen`): the engine's fighters get up on their
//! own.

use realistic_physics as rp;

/// The slowest a clean jab and a clean cross may land and still count, in
/// metres a second: nine tenths of the slowest a fresh fighter's lands from
/// within its reach. Over the engine's 46 distances (`print_fighter_punches`)
/// fresh jabs landed at 7.6 to 8.8 m/s and crosses at 8.0 to 8.6, and both
/// at 5.0 to 5.6 thrown from the end of their reach, the arm all but
/// straight; a tired arm's punch lands slower by the square root of the
/// strength it has left (the engine's `Strike::pace`), under these once it
/// has lost about a fifth.
pub const COUNTING_SPEED: [f64; 2] = [6.9, 7.2];

/// The slowest a clean punch of `kind` may land and still count.
pub fn counting_speed(kind: rp::PunchKind) -> f64 {
    match kind {
        rp::PunchKind::Jab => COUNTING_SPEED[0],
        rp::PunchKind::Cross => COUNTING_SPEED[1],
    }
}

/// How many seconds after a clean punch lands a fighter going down counts
/// as knocked down by it, rather than slipping: a knocked-out fighter is on
/// the floor within about a second, and one pushed off its feet by a punch
/// sooner.
pub const KNOCKED_DOWN_SECONDS: f64 = 2.0;

/// The count: a fighter still down at its tenth second is knocked out, and
/// one up sooner fights on once it has reached eight.
pub const COUNT_OUT: f64 = 10.0;
pub const MANDATORY_COUNT: f64 = 8.0;

/// How many knockdowns end a bout.
pub const KNOCKDOWNS_STOP_IT: u32 = 3;

/// How long a bout is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    pub rounds: u32,
    /// How long a round lasts, and the rest between two, in seconds.
    pub round_seconds: f64,
    pub break_seconds: f64,
}

impl Default for Rules {
    /// Three rounds of a minute: real boxing's three minutes would make a
    /// bout about ten minutes long.
    fn default() -> Rules {
        Rules {
            rounds: 3,
            round_seconds: 60.0,
            break_seconds: 10.0,
        }
    }
}

/// Where a bout stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// Round `n`, counted from 1, under way.
    Round(u32),
    /// The rest after round `n`.
    Break(u32),
    /// Over, and how it ended.
    Over(Outcome),
}

/// How a bout ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A fighter was still down at the count of ten, in round `round`: a
    /// knockout, and the other won.
    KnockedOut { down: usize, round: u32 },
    /// A fighter went down for the third time, in round `round`: the
    /// referee stopped the bout, and the other won.
    Stopped { down: usize, round: u32 },
    /// The last round ended with both standing: the fighter with more clean
    /// punches won, or none, a draw.
    Decision { winner: Option<usize> },
}

/// A referee's count over a fighter knocked down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Count {
    /// The fighter down.
    pub down: usize,
    /// Seconds since it went down.
    pub seconds: f64,
    /// It stands again, its knockdown beaten so far.
    pub up: bool,
}

impl Count {
    /// The number the referee has called: one a second, from one.
    pub fn called(&self) -> u32 {
        // A second's steps add up to a hair under it.
        ((self.seconds + 1.0e-6).floor() as u32).min(COUNT_OUT as u32)
    }
}

/// Something the bell or the referee marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bell {
    RoundOver(u32),
    RoundBegins(u32),
    BoutOver,
    /// A fighter was knocked down, and the count over it begins.
    Knockdown(usize),
    /// The count is over, the fighter up: the fight goes on.
    Box,
}

/// A bout under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Bout {
    pub rules: Rules,
    stage: Stage,
    /// Seconds left in the round or the rest, and seconds since the bout
    /// ended once it has.
    clock: f64,
    /// Clean punches each fighter landed in each round, the left fighter's
    /// first.
    clean: Vec<[u32; 2]>,
    /// The count under way, if a fighter is down.
    count: Option<Count>,
    /// Each fighter's knockdowns so far.
    knockdowns: [u32; 2],
    /// Seconds since a clean punch last landed on each fighter.
    hit: [Option<f64>; 2],
    /// Whether each fighter was down on the last step.
    was_down: [bool; 2],
}

impl Bout {
    pub fn new(rules: Rules) -> Bout {
        Bout {
            rules,
            stage: Stage::Round(1),
            clock: rules.round_seconds,
            clean: vec![[0; 2]],
            count: None,
            knockdowns: [0; 2],
            hit: [None; 2],
            was_down: [false; 2],
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// How it ended, and how many seconds ago, once it has.
    pub fn outcome(&self) -> Option<(Outcome, f64)> {
        match self.stage {
            Stage::Over(outcome) => Some((outcome, self.clock)),
            _ => None,
        }
    }

    /// The count under way, if a fighter is down.
    pub fn count(&self) -> Option<Count> {
        self.count
    }

    /// Each fighter's knockdowns so far.
    pub fn knockdowns(&self) -> [u32; 2] {
        self.knockdowns
    }

    /// The round under way, or the last one fought.
    pub fn round(&self) -> u32 {
        match self.stage {
            Stage::Round(round) | Stage::Break(round) => round,
            Stage::Over(_) => self.clean.len() as u32,
        }
    }

    /// Seconds left in the round or the rest under way; none once over.
    pub fn seconds_left(&self) -> Option<f64> {
        match self.stage {
            Stage::Over(_) => None,
            _ => Some(self.clock.max(0.0)),
        }
    }

    /// Clean punches each fighter has landed in the whole bout.
    pub fn clean(&self) -> [u32; 2] {
        self.clean
            .iter()
            .fold([0; 2], |sum, round| [sum[0] + round[0], sum[1] + round[1]])
    }

    /// Clean punches each fighter landed in round `round`.
    pub fn clean_in(&self, round: u32) -> [u32; 2] {
        self.clean
            .get(round.saturating_sub(1) as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Ends the bout as `outcome`.
    fn end(&mut self, outcome: Outcome) -> Option<Bell> {
        self.stage = Stage::Over(outcome);
        self.clock = 0.0;
        self.count = None;
        Some(Bell::BoutOver)
    }

    /// Lets `dt` seconds of the bout pass, the punches `landed` this step
    /// counted while a round is under way, each fighter `down` or not as the
    /// engine has it: what the bell or the referee marks, if anything.
    pub fn tick(&mut self, dt: f64, landed: &[rp::LandedPunch], down: [bool; 2]) -> Option<Bell> {
        if let Stage::Over(_) = self.stage {
            self.clock += dt;
            return None;
        }
        for since in self.hit.iter_mut().flatten() {
            *since += dt;
        }
        for punch in landed.iter().filter(|punch| clean(punch.struck)) {
            if let Some(since) = self.hit.get_mut(punch.on) {
                *since = Some(0.0);
            }
            if !counts(punch) {
                continue;
            }
            if let (Stage::Round(_), Some(tally)) = (self.stage, self.clean.last_mut()) {
                if let Some(count) = tally.get_mut(punch.by) {
                    *count += 1;
                }
            }
        }
        // A fighter going down just after a clean punch landed on it is
        // knocked down, and counted; with none, it slipped.
        let went_down = (0..2).find(|&fighter| down[fighter] && !self.was_down[fighter]);
        self.was_down = down;
        let mut bell = None;
        if let (None, Some(fighter)) = (self.count, went_down) {
            let punched = self.hit[fighter].is_some_and(|since| since <= KNOCKED_DOWN_SECONDS);
            if punched {
                self.knockdowns[fighter] += 1;
                if self.knockdowns[fighter] >= KNOCKDOWNS_STOP_IT {
                    let round = self.round();
                    return self.end(Outcome::Stopped {
                        down: fighter,
                        round,
                    });
                }
                self.count = Some(Count {
                    down: fighter,
                    seconds: 0.0,
                    up: false,
                });
                bell = Some(Bell::Knockdown(fighter));
            }
        }
        if let Some(count) = &mut self.count {
            count.seconds += dt;
            // Up, or down again with no new punch: the count goes on.
            count.up = !down[count.down];
            if count.seconds >= COUNT_OUT && !count.up {
                let (fighter, round) = (count.down, self.round());
                return self.end(Outcome::KnockedOut {
                    down: fighter,
                    round,
                });
            }
            if count.seconds >= MANDATORY_COUNT && count.up {
                self.count = None;
                bell = Some(Bell::Box);
            }
        }
        if let Stage::Round(round) = self.stage {
            // The round's clock runs on through a count, but its bell waits
            // for the count to end: the bell saves no one.
            self.clock -= dt;
            if self.clock <= 0.0 && self.count.is_none() {
                if round >= self.rules.rounds {
                    let [left, right] = self.clean();
                    let winner = match left.cmp(&right) {
                        std::cmp::Ordering::Greater => Some(0),
                        std::cmp::Ordering::Less => Some(1),
                        std::cmp::Ordering::Equal => None,
                    };
                    return self.end(Outcome::Decision { winner });
                }
                // What the step ran past the bell goes on into the rest, so
                // that the bells keep time over a whole bout.
                self.stage = Stage::Break(round);
                self.clock = self.clock.max(-dt) + self.rules.break_seconds;
                return Some(Bell::RoundOver(round));
            }
            return bell;
        }
        if let Stage::Break(round) = self.stage {
            self.clock -= dt;
            if self.clock <= 0.0 {
                self.stage = Stage::Round(round + 1);
                self.clock += self.rules.round_seconds;
                self.clean.push([0; 2]);
                return Some(Bell::RoundBegins(round + 1));
            }
        }
        bell
    }
}

/// Whether a punch that struck `struck` landed clean: on the head or the
/// body, not stopped on the arms (nor, as never happens, on a leg).
pub fn clean(struck: rp::Struck) -> bool {
    matches!(struck, rp::Struck::Head | rp::Struck::Body)
}

/// Whether a landed punch counts toward the decision: clean, and hard, its
/// fist landing at its kind's `counting_speed` or faster.
pub fn counts(punch: &rp::LandedPunch) -> bool {
    clean(punch.struck) && punch.speed >= counting_speed(punch.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn punch(by: usize, struck: rp::Struck) -> rp::LandedPunch {
        rp::LandedPunch {
            by,
            kind: rp::PunchKind::Jab,
            on: 1 - by,
            struck,
            speed: 8.0,
        }
    }

    /// Plays `seconds` of `bout`, the punches `landed` landing on the first
    /// step: every bell it rang, with the step it rang on.
    fn play(bout: &mut Bout, seconds: f64, landed: &[rp::LandedPunch]) -> Vec<(usize, Bell)> {
        let mut bells = Vec::new();
        for step in 0..(seconds / DT).round() as usize {
            let now: &[rp::LandedPunch] = if step == 0 { landed } else { &[] };
            if let Some(bell) = bout.tick(DT, now, [false; 2]) {
                bells.push((step, bell));
            }
        }
        bells
    }

    /// Three rounds of a minute with ten seconds between, then a decision
    /// for the fighter who landed more clean punches: punches stopped on
    /// the arms do not count.
    #[test]
    fn three_rounds_then_a_decision_on_clean_punches() {
        let mut bout = Bout::new(Rules::default());
        let first = [
            punch(0, rp::Struck::Head),
            punch(0, rp::Struck::Arm),
            punch(1, rp::Struck::Body),
            punch(1, rp::Struck::Head),
        ];
        // A step either way: the clock counts down a sixtieth at a time.
        let near = |step: usize, wanted: usize| step.abs_diff(wanted) <= 1;
        let bells = play(&mut bout, 70.5, &first);
        assert!(
            matches!(bells[..], [(over, Bell::RoundOver(1)), (begins, Bell::RoundBegins(2))]
                if near(over, 3599) && near(begins, 4199)),
            "{bells:?}"
        );
        assert_eq!(bout.clean_in(1), [1, 2]);
        play(
            &mut bout,
            70.0,
            &[punch(0, rp::Struck::Head), punch(0, rp::Struck::Body)],
        );
        assert_eq!(bout.clean(), [3, 2]);
        assert_eq!(bout.stage(), Stage::Round(3));
        let bells = play(&mut bout, 61.0, &[]);
        assert!(
            matches!(bells[..], [(over, Bell::BoutOver)] if near(over, 3569)),
            "{bells:?}"
        );
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(Outcome::Decision { winner: Some(0) })
        );
    }

    /// A clean punch landing slower than a fresh arm's, as a tired arm's
    /// does, does not count; it still makes a fall just after it a
    /// knockdown.
    #[test]
    fn a_slow_punch_does_not_count() {
        let mut bout = Bout::new(Rules::default());
        let mut slow = punch(0, rp::Struck::Head);
        slow.speed = counting_speed(slow.kind) - 0.5;
        play(&mut bout, 1.0, &[slow, punch(1, rp::Struck::Head)]);
        assert_eq!(bout.clean(), [0, 1]);
        let mut slow = punch(1, rp::Struck::Head);
        slow.speed = counting_speed(slow.kind) - 0.5;
        let bells = (0..3)
            .filter_map(|step| {
                let landed = if step == 0 { vec![slow] } else { vec![] };
                bout.tick(DT, &landed, [step > 0, false])
            })
            .collect::<Vec<_>>();
        assert_eq!(bells, [Bell::Knockdown(0)]);
    }

    /// Nothing counts during the rest between rounds, and equal counts at
    /// the end are a draw.
    #[test]
    fn the_rest_counts_nothing_and_equal_counts_draw() {
        let rules = Rules {
            rounds: 2,
            round_seconds: 2.0,
            break_seconds: 1.0,
        };
        let mut bout = Bout::new(rules);
        play(&mut bout, 2.1, &[punch(1, rp::Struck::Head)]);
        assert_eq!(bout.stage(), Stage::Break(1));
        play(&mut bout, 0.5, &[punch(1, rp::Struck::Head)]);
        assert_eq!(bout.clean(), [0, 1]);
        play(&mut bout, 0.5, &[]);
        assert_eq!(bout.stage(), Stage::Round(2));
        play(&mut bout, 1.0, &[punch(0, rp::Struck::Body)]);
        play(&mut bout, 1.1, &[]);
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(Outcome::Decision { winner: None })
        );
    }

    /// Plays `seconds` of `bout` with each fighter `down` or not, a clean
    /// punch landing on `hit` first if any: every bell it rang, with the
    /// second it rang at.
    fn play_down(
        bout: &mut Bout,
        seconds: f64,
        hit: Option<usize>,
        down: [bool; 2],
    ) -> Vec<(f64, Bell)> {
        let mut bells = Vec::new();
        for step in 0..(seconds / DT).round() as usize {
            let landed: Vec<rp::LandedPunch> = hit
                .filter(|_| step == 0)
                .map(|on| punch(1 - on, rp::Struck::Head))
                .into_iter()
                .collect();
            if let Some(bell) = bout.tick(DT, &landed, down) {
                bells.push((step as f64 * DT, bell));
            }
        }
        bells
    }

    /// A fighter knocked down by a punch gets a count, and up by ten it
    /// fights on once the count has reached eight, though it stood at five;
    /// the knockdown is counted against it, and the bout goes on.
    #[test]
    fn a_fighter_up_by_ten_fights_on_after_a_count_of_eight() {
        let mut bout = Bout::new(Rules::default());
        play_down(&mut bout, 5.0, None, [false; 2]);
        let bells = play_down(&mut bout, 0.5, Some(1), [false, true]);
        assert_eq!(bells, [(0.0, Bell::Knockdown(1))]);
        play_down(&mut bout, 4.5, None, [false, true]);
        assert_eq!(bout.count().map(|count| count.called()), Some(5));
        let bells = play_down(&mut bout, 4.0, None, [false; 2]);
        assert!(
            matches!(bells[..], [(at, Bell::Box)] if (2.9..3.1).contains(&at)),
            "{bells:?}"
        );
        assert_eq!(bout.count(), None);
        assert_eq!(bout.knockdowns(), [0, 1]);
        assert_eq!(bout.stage(), Stage::Round(1));
    }

    /// A fighter still down at the count of ten is knocked out, and the
    /// bout is over in the round it went down in.
    #[test]
    fn still_down_at_ten_is_a_knockout() {
        let mut bout = Bout::new(Rules::default());
        play_down(&mut bout, 75.0, None, [false; 2]);
        let bells = play_down(&mut bout, 12.0, Some(0), [true, false]);
        assert!(
            matches!(bells[..], [(0.0, Bell::Knockdown(0)), (at, Bell::BoutOver)] if (9.9..10.1).contains(&at)),
            "{bells:?}"
        );
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(Outcome::KnockedOut { down: 0, round: 2 })
        );
    }

    /// A fighter going down with no punch landing on it slipped: no count,
    /// no knockdown, and the bout goes on as it gets up.
    #[test]
    fn a_fall_with_no_punch_is_a_slip() {
        let mut bout = Bout::new(Rules::default());
        play_down(&mut bout, 1.0, Some(1), [false; 2]);
        // A punch long before does not make it a knockdown.
        play_down(&mut bout, 3.0, None, [false; 2]);
        let bells = play_down(&mut bout, 6.0, None, [false, true]);
        assert!(bells.is_empty(), "{bells:?}");
        assert_eq!(bout.count(), None);
        assert_eq!(bout.knockdowns(), [0, 0]);
        play_down(&mut bout, 1.0, None, [false; 2]);
        assert_eq!(bout.stage(), Stage::Round(1));
    }

    /// The third knockdown ends the bout at once: the referee stops it.
    #[test]
    fn the_third_knockdown_stops_the_bout() {
        let mut bout = Bout::new(Rules::default());
        for _ in 0..2 {
            play_down(&mut bout, 2.0, Some(1), [false, true]);
            play_down(&mut bout, 7.0, None, [false; 2]);
            assert_eq!(bout.count(), None);
        }
        assert_eq!(bout.knockdowns(), [0, 2]);
        let bells = play_down(&mut bout, 1.0, Some(1), [false, true]);
        assert_eq!(bells, [(0.0, Bell::BoutOver)]);
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(Outcome::Stopped { down: 1, round: 1 })
        );
    }

    /// The bell saves no one: a round that runs out during a count ends once
    /// the count does, the fighter up.
    #[test]
    fn the_bell_does_not_save_a_fighter() {
        let mut bout = Bout::new(Rules::default());
        play_down(&mut bout, 57.0, None, [false; 2]);
        play_down(&mut bout, 6.0, Some(0), [true, false]);
        assert_eq!(bout.stage(), Stage::Round(1));
        assert!(bout.count().is_some());
        // The count ends at eight, and the round's bell rings with it.
        let bells = play_down(&mut bout, 3.0, None, [false; 2]);
        assert!(
            matches!(bells[..], [(at, Bell::RoundOver(1))] if (1.9..2.1).contains(&at)),
            "{bells:?}"
        );
        assert_eq!(bout.stage(), Stage::Break(1));
        assert_eq!(bout.count(), None);
    }
}
