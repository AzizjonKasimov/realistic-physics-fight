//! The sparring partner: until the game has a computer opponent, a fighter
//! nobody plays spars on its own, so a player alone has someone to hit and
//! punches to block, duck, lean away from, and answer. It comes into reach,
//! throws a jab or a cross at moments nobody can foresee, sometimes covers
//! up, ducks, or leans back for a moment, and now and then steps back out.
//! It decides by how far apart the fighters stand and by chance alone: it
//! never looks at what the player is doing.

use crate::controls::{Act, Hold};

/// The shortest and the longest wait between its decisions, in seconds.
pub const DECISION_GAP: (f64, f64) = (0.45, 1.3);
/// How long it holds a block, a duck, or a lean back once it decides to,
/// the shortest and the longest, in seconds.
pub const HOLD_SECONDS: (f64, f64) = (0.45, 1.1);
/// Farther apart than this, in body heights between the fighters' hips, its
/// punches would fall short, and it steps in.
pub const REACH: f64 = 0.70;
/// Nearer than this it has little room, and steps back out more often.
pub const CLOSE: f64 = 0.56;

/// What the sparring partner does on a step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wish {
    /// A move it makes now.
    pub act: Option<Act>,
    /// What it holds, in [`Hold::ALL`]'s order.
    pub holds: [bool; 3],
}

/// What it decides to do next.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Decision {
    Act(Act),
    Hold(Hold),
    Nothing,
}

/// The sparring partner's own mind: when it next decides, and what it is
/// holding.
#[derive(Clone, Debug, PartialEq)]
pub struct Sparring {
    /// The state of its random draws (xorshift64*, Vigna 2016), never zero.
    state: u64,
    /// Seconds until its next decision.
    wait: f64,
    /// What it is holding, and for how many seconds more.
    holding: Option<(Hold, f64)>,
}

impl Sparring {
    /// A sparring partner whose decisions follow from `seed`, the same for
    /// the same seed.
    pub fn new(seed: u64) -> Sparring {
        let mut sparring = Sparring {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            wait: 0.0,
            holding: None,
        };
        sparring.wait = sparring.between(DECISION_GAP);
        sparring
    }

    /// Lets `dt` seconds pass with the fighters' hips `apart` body heights
    /// apart, and says what it does now.
    pub fn tick(&mut self, dt: f64, apart: f64) -> Wish {
        let mut wish = Wish::default();
        if let Some((hold, left)) = &mut self.holding {
            *left -= dt;
            if *left > 0.0 {
                wish.holds[hold.index()] = true;
            } else {
                self.holding = None;
            }
        }
        self.wait -= dt;
        if self.wait > 0.0 {
            return wish;
        }
        self.wait = self.between(DECISION_GAP);
        match self.decide(apart) {
            Decision::Act(act) => wish.act = Some(act),
            Decision::Hold(hold) => {
                let seconds = self.between(HOLD_SECONDS);
                self.holding = Some((hold, seconds));
                wish.holds = [false; 3];
                wish.holds[hold.index()] = true;
            }
            Decision::Nothing => {}
        }
        wish
    }

    /// What it does next, `apart` body heights from the player: out of
    /// reach it comes in; in reach it mostly punches; close in it punches
    /// or makes room.
    fn decide(&mut self, apart: f64) -> Decision {
        use Decision::Nothing;
        let jab = Decision::Act(Act::Jab);
        let cross = Decision::Act(Act::Cross);
        let step_in = Decision::Act(Act::StepIn);
        let step_out = Decision::Act(Act::StepOut);
        let block = Decision::Hold(Hold::Block);
        let duck = Decision::Hold(Hold::Duck);
        let lean = Decision::Hold(Hold::Lean);
        // Each choice with its share of a hundred.
        let choices: &[(Decision, u32)] = if apart > REACH {
            &[(step_in, 80), (Nothing, 20)]
        } else if apart > CLOSE {
            &[
                (jab, 34),
                (cross, 22),
                (block, 14),
                (step_in, 8),
                (step_out, 6),
                (duck, 6),
                (lean, 4),
                (Nothing, 6),
            ]
        } else {
            &[
                (jab, 30),
                (cross, 24),
                (block, 14),
                (step_out, 18),
                (duck, 6),
                (Nothing, 8),
            ]
        };
        let mut draw = (self.unit() * 100.0) as u32;
        for &(decision, share) in choices {
            if draw < share {
                return decision;
            }
            draw -= share;
        }
        Nothing
    }

    /// A draw spread evenly from 0 up to 1.
    fn unit(&mut self) -> f64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        let draw = self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
        draw as f64 / (1u64 << 53) as f64
    }

    /// A draw spread evenly between `bounds`.
    fn between(&mut self, bounds: (f64, f64)) -> f64 {
        bounds.0 + (bounds.1 - bounds.0) * self.unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    /// Everything it does over `seconds`, standing `apart`: each move it
    /// makes with the step it makes it on, and the steps it holds each hold.
    fn watch(seed: u64, seconds: f64, apart: f64) -> (Vec<(usize, Act)>, [usize; 3]) {
        let mut sparring = Sparring::new(seed);
        let (mut acts, mut held) = (Vec::new(), [0; 3]);
        for step in 0..(seconds / DT) as usize {
            let wish = sparring.tick(DT, apart);
            if let Some(act) = wish.act {
                acts.push((step, act));
            }
            for hold in Hold::ALL {
                held[hold.index()] += usize::from(wish.holds[hold.index()]);
            }
        }
        (acts, held)
    }

    /// What it does follows from its seed: the same for the same seed, and
    /// different for another.
    #[test]
    fn it_does_the_same_for_the_same_seed() {
        assert_eq!(watch(7, 60.0, 0.65), watch(7, 60.0, 0.65));
        assert_ne!(watch(7, 60.0, 0.65), watch(8, 60.0, 0.65));
    }

    /// Out of reach it only comes in: it throws no punch that would fall
    /// short, and holds nothing.
    #[test]
    fn out_of_reach_it_steps_in() {
        let (acts, held) = watch(3, 30.0, 0.78);
        assert!(!acts.is_empty(), "it never stepped in");
        assert!(
            acts.iter().all(|&(_, act)| act == Act::StepIn),
            "out of reach it did {acts:?}"
        );
        assert_eq!(held, [0; 3]);
    }

    /// In reach it throws both punches, more than anything else, with waits
    /// between its decisions inside their bounds, and sometimes blocks,
    /// ducks, and leans back, each for no longer than it holds one.
    #[test]
    fn in_reach_it_punches_and_sometimes_covers_up() {
        let seconds = 600.0;
        let (acts, held) = watch(11, seconds, 0.65);
        let count = |wanted: Act| acts.iter().filter(|&&(_, act)| act == wanted).count();
        let (jabs, crosses) = (count(Act::Jab), count(Act::Cross));
        let decisions = seconds / ((DECISION_GAP.0 + DECISION_GAP.1) * 0.5);
        let punches = (jabs + crosses) as f64 / decisions;
        assert!(
            jabs > crosses && crosses > 0 && (0.4..0.7).contains(&punches),
            "{jabs} jabs and {crosses} crosses in about {decisions:.0} decisions"
        );
        for pair in acts.windows(2) {
            let gap = (pair[1].0 - pair[0].0) as f64 * DT;
            assert!(
                gap > DECISION_GAP.0 - 2.0 * DT,
                "two moves {gap:.2} s apart"
            );
        }
        for hold in Hold::ALL {
            let share = held[hold.index()] as f64 * DT / seconds;
            assert!(
                share > 0.005 && share < 0.25,
                "it held {} for {:.0}% of the time",
                hold.name(),
                share * 100.0
            );
        }
    }

    /// Close in it makes room: it steps back out, and never further in.
    #[test]
    fn close_in_it_steps_back_out() {
        let (acts, _) = watch(5, 120.0, 0.52);
        assert!(acts.iter().any(|&(_, act)| act == Act::StepOut));
        assert!(acts.iter().all(|&(_, act)| act != Act::StepIn));
    }
}
