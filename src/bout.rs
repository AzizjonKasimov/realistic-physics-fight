//! A bout: three rounds of a minute each, with ten seconds between them,
//! the fighters back in their corners. A knockout or a fall ends it at once,
//! as no fighter gets up yet. Otherwise, when the last round ends, the
//! fighter who landed more clean punches wins, as Olympic boxing's computer
//! scoring counted them from 1992 to 2012: a punch counts that lands on the
//! head or the body, not one stopped on the arms. Equal counts are a draw.
//! What a punch struck is the engine's to say (`rp::World::landed_punches`).

use realistic_physics as rp;

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
    /// A fighter went down, knocked out by a blow to its head or fallen,
    /// in round `round`, and the other won.
    Down {
        down: usize,
        knocked_out: bool,
        round: u32,
    },
    /// The last round ended with both standing: the fighter with more clean
    /// punches won, or none, a draw.
    Decision { winner: Option<usize> },
}

/// Something the bell marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bell {
    RoundOver(u32),
    RoundBegins(u32),
    BoutOver,
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
}

impl Bout {
    pub fn new(rules: Rules) -> Bout {
        Bout {
            rules,
            stage: Stage::Round(1),
            clock: rules.round_seconds,
            clean: vec![[0; 2]],
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

    /// Lets `dt` seconds of the bout pass, the punches `landed` this step
    /// counted while a round is under way, and the fighter `down` (and
    /// whether knocked out), if one has gone down: what the bell marks, if
    /// anything.
    pub fn tick(
        &mut self,
        dt: f64,
        landed: &[rp::LandedPunch],
        down: Option<(usize, bool)>,
    ) -> Option<Bell> {
        if let Stage::Over(_) = self.stage {
            self.clock += dt;
            return None;
        }
        if let Some((down, knocked_out)) = down {
            self.stage = Stage::Over(Outcome::Down {
                down,
                knocked_out,
                round: self.round(),
            });
            self.clock = 0.0;
            return Some(Bell::BoutOver);
        }
        if let Stage::Round(round) = self.stage {
            if let Some(tally) = self.clean.last_mut() {
                for punch in landed.iter().filter(|punch| clean(punch.struck)) {
                    if let Some(count) = tally.get_mut(punch.by) {
                        *count += 1;
                    }
                }
            }
            self.clock -= dt;
            if self.clock <= 0.0 {
                if round >= self.rules.rounds {
                    let [left, right] = self.clean();
                    let winner = match left.cmp(&right) {
                        std::cmp::Ordering::Greater => Some(0),
                        std::cmp::Ordering::Less => Some(1),
                        std::cmp::Ordering::Equal => None,
                    };
                    self.stage = Stage::Over(Outcome::Decision { winner });
                    self.clock = 0.0;
                    return Some(Bell::BoutOver);
                }
                // What the step ran past the bell goes on into the rest, so
                // that the bells keep time over a whole bout.
                self.stage = Stage::Break(round);
                self.clock += self.rules.break_seconds;
                return Some(Bell::RoundOver(round));
            }
            return None;
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
        None
    }
}

/// Whether a punch that struck `struck` counts: on the head or the body,
/// not stopped on the arms (nor, as never happens, on a leg).
pub fn clean(struck: rp::Struck) -> bool {
    matches!(struck, rp::Struck::Head | rp::Struck::Body)
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
            if let Some(bell) = bout.tick(DT, now, None) {
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

    /// A fighter going down ends the bout at once, in the round it went
    /// down in, and nothing changes after.
    #[test]
    fn going_down_ends_the_bout_at_once() {
        let mut bout = Bout::new(Rules::default());
        play(&mut bout, 75.0, &[]);
        assert_eq!(bout.tick(DT, &[], Some((1, true))), Some(Bell::BoutOver));
        let knocked_out = Outcome::Down {
            down: 1,
            knocked_out: true,
            round: 2,
        };
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(knocked_out)
        );
        assert_eq!(bout.tick(DT, &[], Some((0, false))), None);
        assert_eq!(
            bout.outcome().map(|(outcome, _)| outcome),
            Some(knocked_out)
        );
    }
}
