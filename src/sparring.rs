//! The sparring partner: until the game has a computer opponent, a fighter
//! nobody plays throws a punch every few seconds, at moments nobody can
//! foresee, so a player alone has punches to lean away from and to answer.
//! It never leans and never aims; the punch is the one a player throws.

/// The shortest and the longest wait between its punches, in seconds.
pub const PUNCH_GAP: (f64, f64) = (1.5, 3.5);

/// When the sparring partner throws its next punch.
#[derive(Clone, Debug, PartialEq)]
pub struct Sparring {
    /// The state of its random waits (xorshift64*, Vigna 2016), never zero.
    state: u64,
    /// Seconds until its next punch.
    wait: f64,
}

impl Sparring {
    /// A sparring partner whose waits follow from `seed`, the same waits for
    /// the same seed.
    pub fn new(seed: u64) -> Sparring {
        let mut sparring = Sparring {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            wait: 0.0,
        };
        sparring.wait = sparring.gap();
        sparring
    }

    /// Lets `dt` seconds pass; true when it throws a punch now.
    pub fn tick(&mut self, dt: f64) -> bool {
        self.wait -= dt;
        if self.wait > 0.0 {
            return false;
        }
        self.wait = self.gap();
        true
    }

    /// Seconds until its next punch.
    #[cfg(test)]
    pub fn wait(&self) -> f64 {
        self.wait
    }

    /// A wait drawn evenly between the shortest and the longest.
    fn gap(&mut self) -> f64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        let draw = self.state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
        let unit = draw as f64 / (1u64 << 53) as f64;
        PUNCH_GAP.0 + (PUNCH_GAP.1 - PUNCH_GAP.0) * unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The waits fall between the shortest and the longest, vary, and come
    /// out the same for the same seed.
    #[test]
    fn waits_vary_within_their_bounds_and_repeat_for_a_seed() {
        let waits = |seed| {
            let mut sparring = Sparring::new(seed);
            (0..200).map(|_| sparring.gap()).collect::<Vec<_>>()
        };
        let first = waits(7);
        assert_eq!(first, waits(7));
        assert_ne!(first, waits(8));
        assert!(first
            .iter()
            .all(|&wait| (PUNCH_GAP.0..=PUNCH_GAP.1).contains(&wait)));
        let (low, high) = first
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), &w| (lo.min(w), hi.max(w)));
        assert!(
            low < 1.8 && high > 3.2,
            "waits only from {low:.2} to {high:.2} s"
        );
    }

    /// It punches once a wait has passed, and not before.
    #[test]
    fn it_punches_when_its_wait_is_over() {
        let mut sparring = Sparring::new(3);
        let wait = sparring.wait();
        let dt = 1.0 / 60.0;
        let steps = (wait / dt).floor() as usize;
        assert!((0..steps - 1).all(|_| !sparring.tick(dt)));
        let fired = (0..3).any(|_| sparring.tick(dt));
        assert!(fired, "no punch once its {wait:.2} s wait was over");
    }
}
