//! The computer opponent: it fights the right fighter while nobody plays it.
//!
//! It sees only what is on the screen, and a moment late, as a person would:
//! how far apart the fighters stand, what the other fighter is doing as the
//! engine shows it (`rp::World::fighter_doing`: a punch on its way, a step, a
//! lean, a duck, a block), and which punches landed on what
//! (`rp::World::landed_punches`). It never reads the player's keys. A punch
//! arrives in about a tenth of a second, faster than anyone reacts (about a
//! quarter of a second; boxers dodge by reading the body before a punch
//! sets off), so like a boxer it defends by reading what comes before one:
//! it covers up, leans back, steps out, or meets the other with a jab as
//! the other steps in; covers up after it is hit, and the more the other
//! has been punching; and blocks the second punch of a pair, high against
//! punches to the head and low against punches to the body. It counters
//! while the other's arm comes back, punches more into a lowered guard,
//! goes to the body about one punch in four, and mostly against a high
//! guard, follows a fighter who steps back out of reach, and makes room
//! when crowded.
//!
//! It is built as simple fighting-game opponents are: a few rules, each
//! picking a move at random by weights, everything it reacts to seen
//! through a delay drawn afresh for each thing it notices (Shadow Fight 3's
//! bots pause before each choice; Street Fighter 6's opponent is made to
//! fumble as a person does). Its mistakes are those a person makes: it
//! notices late or not at all, picks the wrong answer, drops its guard to
//! punch, and cannot cover up while its own punch is out. Its three levels
//! differ in how late it sees, how often it notices, and how often it
//! picks well, so it can be beaten.

use std::collections::VecDeque;

use realistic_physics as rp;

use crate::controls::{Act, Hold};

/// How hard the opponent is to beat, chosen before a fight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Easy,
    Normal,
    Hard,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Easy, Level::Normal, Level::Hard];

    /// The name the page and the screen show it by.
    pub fn name(self) -> &'static str {
        match self {
            Level::Easy => "easy",
            Level::Normal => "normal",
            Level::Hard => "hard",
        }
    }

    /// The level after it, back to the first after the last: what the
    /// level button switches to.
    pub fn next(self) -> Level {
        match self {
            Level::Easy => Level::Normal,
            Level::Normal => Level::Hard,
            Level::Hard => Level::Easy,
        }
    }

    /// The level at `index` in [`Level::ALL`], as the page says it, if
    /// there is one.
    pub fn from_index(index: usize) -> Option<Level> {
        Level::ALL.get(index).copied()
    }

    /// How a fighter of this level fights.
    fn traits(self) -> Traits {
        match self {
            Level::Easy => Traits {
                reaction: (0.30, 0.45),
                notice: 0.55,
                read_step: 0.45,
                counter: 0.30,
                cover_up: 0.35,
                guard: 0.12,
                attack_gap: (1.0, 2.2),
                combo: 0.10,
            },
            Level::Normal => Traits {
                reaction: (0.20, 0.32),
                notice: 0.75,
                read_step: 0.70,
                counter: 0.55,
                cover_up: 0.55,
                guard: 0.25,
                attack_gap: (0.6, 1.5),
                combo: 0.25,
            },
            Level::Hard => Traits {
                reaction: (0.14, 0.22),
                notice: 0.90,
                read_step: 0.85,
                counter: 0.75,
                cover_up: 0.75,
                guard: 0.40,
                attack_gap: (0.4, 1.1),
                combo: 0.40,
            },
        }
    }
}

/// How a level fights: each a chance from 0 to 1, or seconds.
#[derive(Clone, Copy, Debug)]
struct Traits {
    /// How late it sees what happens, the shortest and the longest: people
    /// react to what they see in about a quarter of a second (0.25 s on
    /// average; Firoiu, Ju and Tenenbaum 2018 tested game agents at it).
    reaction: (f64, f64),
    /// The chance it takes in a thing it could react to at all.
    notice: f64,
    /// The chance it answers the other fighter stepping in.
    read_step: f64,
    /// The chance it counters a punch that came back.
    counter: f64,
    /// The chance it covers up after it is hit, or against the second
    /// punch of a pair.
    cover_up: f64,
    /// The chance that, with nothing else to do in reach, it keeps its
    /// guard closed for a while.
    guard: f64,
    /// How long it waits between punches of its own in reach.
    attack_gap: (f64, f64),
    /// The chance a punch of its own is a jab with a cross behind it.
    combo: f64,
}

/// Farther apart than this, in body heights between the fighters' hips,
/// punches fall short: one step in from where the fighters start (0.78).
pub const REACH: f64 = 0.70;
/// Nearer than this it has little room, and makes room more often.
pub const CLOSE: f64 = 0.56;

/// How many steps of what it saw it remembers: more than its slowest
/// reaction.
const MEMORY: usize = 40;
/// The simulation's step, in seconds (`rp::Materials::fixed_dt`).
const STEP_SECONDS: f64 = 1.0 / 60.0;
/// How long the other fighter's punching stays on its mind, in seconds:
/// each punch it saw counts one, fading to nothing over this long.
const PRESSURE_SECONDS: f64 = 3.0;
/// How much of its own punches go to the body, against a guard that is not
/// up high, and against one that is: professional boxers land about 23% of
/// their punches to the body (CompuBox), and a high guard leaves it open.
const BODY_SHARE: f64 = 0.25;
const BODY_SHARE_INTO_HIGH_GUARD: f64 = 0.7;
/// The farthest apart it throws a punch to the body from, in body heights
/// between the fighters' hips: a body punch thrown ducking reaches less far,
/// and from 0.60 on its jabs fell short.
const BODY_REACH: f64 = 0.58;
/// How long it stays down after a punch to the body, in seconds: through
/// the punch's way out and back.
const CROUCH_SECONDS: f64 = 0.4;
/// How far into its duck it is, from 0 to 1, before it throws a punch to the
/// body: it drops level first, as a player holding duck does. Thrown as the
/// duck began, from near standing height, its punches went down onto a
/// blocking fighter's low elbows.
const CROUCHED: f64 = 0.5;
/// How long a punch to the body may wait for it to get down, in seconds.
const CROUCH_PATIENCE_SECONDS: f64 = 0.4;

/// How long a move it decided on may wait while it is busy (its own punch
/// still out, a step under way, or still leaning or ducked) before it gives
/// the move up, in seconds.
const PATIENCE_SECONDS: f64 = 0.25;
/// When the cross of a jab and a cross follows the jab, in seconds: once the
/// jab is back (0.33 s), since a fighter throws no punch while one is out.
const COMBO_CROSS: (f64, f64) = (0.34, 0.42);

/// What the opponent sees of the fight on a step: what was on the screen,
/// which it takes in a moment later.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sight {
    /// How far apart the fighters' hips stand, in body heights.
    pub apart: f64,
    /// What the other fighter is doing.
    pub them: rp::Doing,
    /// A punch of the other's landed on it clean, on its head or body, or
    /// on its arms.
    pub hit: bool,
    pub blocked: bool,
    /// The clean punch that landed on it went to its body.
    pub hit_body: bool,
}

/// What the opponent knows of itself, at once: what it is doing, and
/// whether there is room behind it in the ring for a step back.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Itself {
    pub doing: rp::Doing,
    pub room_behind: bool,
}

/// What the opponent does on a step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wish {
    /// A move it makes now.
    pub act: Option<Act>,
    /// What it holds, in [`Hold::ALL`]'s order.
    pub holds: [bool; 3],
}

/// What it holds for a while: its guard high in front of its face (a
/// block), low over its body (a block while ducking), or a lean back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Guard {
    High,
    Low,
    Lean,
}

impl Guard {
    /// The moves it holds, in [`Hold::ALL`]'s order.
    fn holds(self) -> [bool; 3] {
        let mut holds = [false; 3];
        match self {
            Guard::High => holds[Hold::Block.index()] = true,
            Guard::Low => {
                holds[Hold::Block.index()] = true;
                holds[Hold::Duck.index()] = true;
            }
            Guard::Lean => holds[Hold::Lean.index()] = true,
        }
        holds
    }
}

/// Something it has decided to do.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Planned {
    Act(Act),
    /// A punch to the body: thrown while it ducks.
    Body(Act),
    /// Holds a guard for so many seconds.
    Guard(Guard, f64),
}

/// A move it has decided on: in how many seconds it is due, and how long it
/// may still wait once due while the fighter is busy.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Decided {
    wait: f64,
    patience: f64,
    planned: Planned,
}

/// The opponent's own mind.
#[derive(Clone, Debug)]
pub struct Opponent {
    level: Level,
    traits: Traits,
    /// The state of its random draws (xorshift64*, Vigna 2016), never zero.
    state: u64,
    /// What it saw on each of the last steps, the newest last, and how many
    /// steps it has seen in all.
    seen: VecDeque<Sight>,
    steps: usize,
    /// The newest step it has taken in, and what it saw there.
    taken: usize,
    last: Sight,
    /// How many steps late it takes things in now.
    late: usize,
    /// What it has decided to do.
    plan: Vec<Decided>,
    /// The guard it holds, and for how many seconds more.
    holding: Option<(Guard, f64)>,
    /// Seconds it stays down for a punch to the body.
    crouch: f64,
    /// Seconds until it attacks on its own, in reach.
    attack_in: f64,
    /// Seconds until it may next decide to keep its guard up.
    guard_in: f64,
    /// How much the other fighter has punched lately, a punch counting one
    /// and fading, and how much of that went to the body.
    pressure: f64,
    body_pressure: f64,
}

impl Opponent {
    /// An opponent of `level` whose decisions follow from `seed`, the same
    /// for the same seed.
    pub fn new(level: Level, seed: u64) -> Opponent {
        let mut opponent = Opponent {
            level,
            traits: level.traits(),
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            seen: VecDeque::with_capacity(MEMORY),
            steps: 0,
            taken: 0,
            last: Sight::default(),
            late: 0,
            plan: Vec::new(),
            holding: None,
            crouch: 0.0,
            attack_in: 0.0,
            guard_in: 0.0,
            pressure: 0.0,
            body_pressure: 0.0,
        };
        opponent.late = opponent.draw_late();
        opponent.attack_in = opponent.between(opponent.traits.attack_gap);
        opponent
    }

    pub fn level(&self) -> Level {
        self.level
    }

    /// A fresh round: it comes out of its corner with nothing on its mind,
    /// what it saw and decided before the rest forgotten.
    pub fn new_round(&mut self) {
        self.seen.clear();
        self.steps = 0;
        self.taken = 0;
        self.last = Sight::default();
        self.plan.clear();
        self.holding = None;
        self.crouch = 0.0;
        self.pressure = 0.0;
        self.body_pressure = 0.0;
        self.attack_in = self.between(self.traits.attack_gap);
        self.guard_in = 0.0;
    }

    /// Lets `dt` seconds pass with `sight` on the screen and itself as
    /// `me` is, and says what it does now.
    pub fn tick(&mut self, dt: f64, sight: Sight, me: Itself) -> Wish {
        if self.seen.len() == MEMORY {
            self.seen.pop_front();
        }
        self.seen.push_back(sight);
        self.steps += 1;
        self.pressure = (self.pressure - dt / PRESSURE_SECONDS).max(0.0);
        self.body_pressure = (self.body_pressure - dt / PRESSURE_SECONDS).max(0.0);
        // Everything it has not taken in yet that is old enough to be seen.
        let newest = self.steps.saturating_sub(self.late);
        while self.taken < newest {
            self.taken += 1;
            let age = self.steps - self.taken;
            // A step already out of its memory is passed over.
            let Some(&now) = self
                .seen
                .len()
                .checked_sub(1 + age)
                .and_then(|index| self.seen.get(index))
            else {
                continue;
            };
            let then = self.last;
            self.last = now;
            self.take_in(then, now, me);
        }
        self.act(dt, me)
    }

    /// Takes in what changed from `then` to `now`, one step it saw, and
    /// decides what to do about it.
    fn take_in(&mut self, then: Sight, now: Sight, me: Itself) {
        let started = |was: Option<rp::Punching>, is: Option<rp::Punching>| match (was, is) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(was), Some(is)) => is.age < was.age,
        };
        let their_punch = started(then.them.punch, now.them.punch);
        let came_back = then.them.punch.is_some_and(|punch| punch.going_out)
            && !now.them.punch.is_some_and(|punch| punch.going_out);
        let step_began = |forward: bool| {
            now.them.step.is_some_and(|step| {
                step.forward == forward
                    && !step.catching
                    && then
                        .them
                        .step
                        .is_none_or(|was| was.forward != forward || step.progress < was.progress)
            })
        };
        let in_reach = now.apart <= REACH;
        let to_body = now.them.punch.is_some_and(|punch| punch.to_body);
        if their_punch {
            self.pressure += 1.0;
            if to_body {
                self.body_pressure += 1.0;
            }
        }
        // The other fighter dropping its level, as it does before a punch
        // to the body (and to duck under one).
        let dropped = now.them.ducking && !then.them.ducking;
        if now.hit && self.notices() {
            self.hit_me(me, now.hit_body);
        } else if their_punch && in_reach && self.notices() {
            // Too late for this punch: it punches back into the guard the
            // punch opened, or covers up against the next, low against a
            // punch to the body.
            if !self.counter(now) && self.chance(self.traits.cover_up * 0.6) {
                self.cover(to_body, 0.35, 0.6);
            }
        } else if dropped && in_reach && self.notices() && self.chance(self.traits.read_step * 0.5)
        {
            self.cover(true, 0.4, 0.7);
        } else if came_back && in_reach && self.notices() && self.chance(0.5) {
            // A punch it saw too late to answer as it came: one more chance
            // as it comes back.
            self.counter(now);
        }
        if step_began(true) && self.notices() && self.chance(self.traits.read_step) {
            self.read_step_in(me);
        }
        if step_began(false) && self.notices() {
            // It follows a fighter who steps back out of its reach.
            if now.apart + 0.13 > REACH && self.chance(0.8) {
                let wait = self.between((0.05, 0.25));
                self.decide(wait, Planned::Act(Act::StepIn));
            }
        }
    }

    /// It was hit clean, to the body or the head: it covers up where it
    /// was hit, makes room, or fires back.
    fn hit_me(&mut self, me: Itself, body: bool) {
        let draw = self.unit();
        let cover = self.traits.cover_up;
        if draw < cover {
            self.cover(body, 0.5, 0.9);
        } else if draw < cover + 0.2 && me.room_behind {
            self.forget_plan();
            self.decide(0.0, Planned::Act(Act::StepOut));
        } else if draw < cover + 0.35 {
            self.forget_plan();
            let punch = self.pick_punch();
            self.decide(0.0, Planned::Act(punch));
        }
    }

    /// The other fighter steps in: it covers up, leans back, makes room,
    /// or meets it with a jab.
    fn read_step_in(&mut self, me: Itself) {
        let choices: &[(u32, u8)] = &[(40, 0), (15, 1), (20, 2), (25, 3)];
        match self.weighted(choices) {
            0 => self.cover(false, 0.5, 0.8),
            1 => {
                self.forget_plan();
                let seconds = self.between((0.35, 0.55));
                self.decide(0.0, Planned::Guard(Guard::Lean, seconds));
            }
            2 if me.room_behind => {
                self.forget_plan();
                self.decide(0.0, Planned::Act(Act::StepOut));
            }
            _ => {
                self.forget_plan();
                self.decide(0.0, Planned::Act(Act::Jab));
            }
        }
    }

    /// The other fighter punched, opening its guard until the punch is
    /// back: it punches into the opening, letting go of its own guard to,
    /// at its level's chance. Whether it does.
    fn counter(&mut self, now: Sight) -> bool {
        let chance = if now.them.guarded {
            self.traits.counter * 0.4
        } else {
            self.traits.counter
        };
        if !self.chance(chance) {
            return false;
        }
        self.forget_plan();
        let punch = if self.chance(0.6) {
            Act::Cross
        } else {
            Act::Jab
        };
        let wait = self.between((0.0, 0.05));
        self.decide(wait, Planned::Act(punch));
        true
    }

    /// Covers up behind its forearms, low over its body or high in front of
    /// its face, for between `shortest` and `longest` seconds.
    fn cover(&mut self, low: bool, shortest: f64, longest: f64) {
        self.forget_plan();
        let seconds = self.between((shortest, longest));
        let guard = if low { Guard::Low } else { Guard::High };
        self.decide(0.0, Planned::Guard(guard, seconds));
    }

    /// What it does this step: what it decided that is due, or, with
    /// nothing to do, what its place in the fight asks.
    fn act(&mut self, dt: f64, me: Itself) -> Wish {
        let mut wish = Wish::default();
        if me.doing.fallen {
            self.holding = None;
            self.crouch = 0.0;
            self.plan.clear();
            return wish;
        }
        if let Some((_, left)) = &mut self.holding {
            *left -= dt;
            if *left <= 0.0 {
                self.holding = None;
            }
        }
        self.crouch = (self.crouch - dt).max(0.0);
        for decided in &mut self.plan {
            if decided.wait > 0.0 {
                decided.wait -= dt;
            } else {
                decided.patience -= dt;
            }
        }
        self.plan.retain(|decided| decided.patience > 0.0);
        let due = self.plan.iter().position(|decided| decided.wait <= 0.0);
        if let Some(due) = due {
            let planned = self.plan[due].planned;
            match planned {
                Planned::Guard(guard, seconds) => {
                    self.plan.remove(due);
                    self.holding = Some((guard, seconds));
                }
                Planned::Act(act) => {
                    // A punch or a step lets go of what it held: it drops
                    // its guard to punch, and comes upright before it steps
                    // and out of a crouch to punch, so that after a jab to
                    // the body the cross goes up to the head.
                    self.holding = None;
                    self.crouch = 0.0;
                    if free_to(act, me.doing) {
                        self.plan.remove(due);
                        wish.act = Some(act);
                    }
                }
                Planned::Body(act) => {
                    // Down first, then the punch, the duck held through it.
                    self.holding = None;
                    self.crouch = self.crouch.max(CROUCH_SECONDS);
                    if me.doing.duck >= CROUCHED && free_to(act, me.doing) {
                        self.plan.remove(due);
                        wish.act = Some(act);
                    }
                }
            }
        } else if self.holding.is_none() && self.plan.is_empty() {
            wish.act = self.on_its_own(dt, me);
        }
        if let Some((guard, _)) = self.holding {
            wish.holds = guard.holds();
        }
        if self.crouch > 0.0 {
            wish.holds[Hold::Duck.index()] = true;
        }
        self.attack_in -= dt;
        self.guard_in -= dt;
        wish
    }

    /// With nothing decided: out of reach it comes in; in reach it punches
    /// when its time comes, more readily into a lowered guard, keeps its
    /// own guard up for a while now and then, and makes room when crowded.
    fn on_its_own(&mut self, dt: f64, me: Itself) -> Option<Act> {
        let seen = self.last;
        if me.doing.step.is_some() || me.doing.punch.is_some() {
            return None;
        }
        if seen.apart > REACH {
            let they_come = seen.them.step.is_some_and(|step| step.forward);
            return (!they_come && self.chance(dt / 0.25)).then_some(Act::StepIn);
        }
        if seen.apart < CLOSE && me.room_behind && self.chance(dt / 2.5) {
            return Some(Act::StepOut);
        }
        if self.guard_in <= 0.0 {
            self.guard_in = self.between((0.6, 1.4));
            let guard = (self.traits.guard + 0.08 * self.pressure).min(0.8);
            if self.chance(guard) {
                let seconds = self.between((0.5, 1.2));
                // Low as often as the other's punches lately went low.
                let low = self.pressure > 0.0 && self.chance(self.body_pressure / self.pressure);
                self.holding = Some((if low { Guard::Low } else { Guard::High }, seconds));
                return None;
            }
        }
        if self.attack_in > 0.0 {
            return None;
        }
        self.attack_in = self.between(self.traits.attack_gap);
        // To the body about one punch in four, and mostly into a high guard,
        // which leaves the body open; from near enough, as a body punch from
        // a duck reaches less far than one at the head.
        let high_guard = seen.them.guarded && !seen.them.guard_low;
        let body = !seen.them.guard_low
            && self.chance(if high_guard {
                BODY_SHARE_INTO_HIGH_GUARD
            } else {
                BODY_SHARE
            });
        if body && seen.apart > BODY_REACH {
            // Nearer first, the punch to the body after the step.
            let wait = self.between((0.6, 0.75));
            self.decide(wait, Planned::Body(Act::Cross));
            return Some(Act::StepIn);
        }
        // A punch at the head into a closed guard often waits for the guard
        // to open.
        if !body && seen.them.guarded && self.chance(0.5) {
            return None;
        }
        let punch = self.pick_punch();
        if punch == Act::Jab && self.chance(self.traits.combo) {
            // A jab and a cross; after a jab to the body, the cross goes up
            // to the head the jab brought the guard down from.
            let wait = self.between(COMBO_CROSS);
            self.decide(wait, Planned::Act(Act::Cross));
        }
        if body {
            // Planned, so that the duck is held on the step it goes.
            self.decide(0.0, Planned::Body(punch));
            return None;
        }
        // At the head: up out of the crouch of a punch to the body before,
        // which would have sent this one to the body too.
        self.crouch = 0.0;
        Some(punch)
    }

    /// A punch of its own: mostly jabs.
    fn pick_punch(&mut self) -> Act {
        if self.chance(0.6) {
            Act::Jab
        } else {
            Act::Cross
        }
    }

    /// Decides to do `planned` in `wait` seconds.
    fn decide(&mut self, wait: f64, planned: Planned) {
        let patience = match planned {
            Planned::Body(_) => CROUCH_PATIENCE_SECONDS,
            _ => PATIENCE_SECONDS,
        };
        self.plan.push(Decided {
            wait,
            patience,
            planned,
        });
    }

    /// Lets go of what it had decided and of what it holds, for something
    /// that matters more now.
    fn forget_plan(&mut self) {
        self.plan.clear();
        self.holding = None;
    }

    /// Whether it takes in something it could react to, and if it does,
    /// how late it sees the next thing.
    fn notices(&mut self) -> bool {
        self.late = self.draw_late();
        self.chance(self.traits.notice)
    }

    /// How many steps late it takes things in, drawn from its reaction.
    fn draw_late(&mut self) -> usize {
        let seconds = self.between(self.traits.reaction);
        ((seconds / STEP_SECONDS).round() as usize).clamp(1, MEMORY - 1)
    }

    /// One of `choices`' second halves, each picked by its weight.
    fn weighted<T: Copy>(&mut self, choices: &[(u32, T)]) -> T {
        let total: u32 = choices.iter().map(|&(weight, _)| weight).sum();
        let mut draw = (self.unit() * f64::from(total)) as u32;
        for &(weight, choice) in choices {
            if draw < weight {
                return choice;
            }
            draw -= weight;
        }
        choices[choices.len() - 1].1
    }

    fn chance(&mut self, chance: f64) -> bool {
        self.unit() < chance
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

/// Whether a fighter doing `doing` can make `act` now: no punch while its
/// own is still out, which the engine would not throw, and no step during
/// another or while it still leans or ducks. Stepping with moves stacked on
/// it can tip a fighter over (the engine's random fight 23).
fn free_to(act: Act, doing: rp::Doing) -> bool {
    match act {
        Act::Jab | Act::Cross => doing.punch.is_none(),
        Act::StepIn | Act::StepOut => doing.step.is_none() && doing.lean < 0.2 && doing.duck < 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    /// The other fighter standing `apart`, doing nothing.
    fn standing(apart: f64) -> Sight {
        Sight {
            apart,
            ..Sight::default()
        }
    }

    /// The other fighter's punch, `age` seconds into its way out.
    fn punching(apart: f64, age: f64) -> Sight {
        Sight {
            apart,
            them: rp::Doing {
                punch: Some(rp::Punching {
                    kind: rp::PunchKind::Jab,
                    to_body: false,
                    age,
                    out_seconds: 0.11,
                    going_out: age < 0.11,
                    landed: false,
                    reach: (age / 0.11).min(1.0),
                }),
                ..rp::Doing::default()
            },
            ..Sight::default()
        }
    }

    /// Everything an opponent of `level` with `seed` does over the sights
    /// `sights`, a step each, its own fighter standing free with room
    /// behind it.
    fn watch(level: Level, seed: u64, sights: &[Sight]) -> Vec<Wish> {
        let mut opponent = Opponent::new(level, seed);
        let mut me = Itself {
            doing: rp::Doing::default(),
            room_behind: true,
        };
        sights
            .iter()
            .map(|&sight| {
                let wish = opponent.tick(DT, sight, me);
                // Its own body ducks as it holds duck, most of the way in
                // about a sixth of a second, and comes up as it lets go.
                let ducking = wish.holds[Hold::Duck.index()];
                me.doing.ducking = ducking;
                me.doing.duck = if ducking {
                    (me.doing.duck + DT / 0.15).min(1.0)
                } else {
                    (me.doing.duck - DT / 0.15).max(0.0)
                };
                wish
            })
            .collect()
    }

    fn acts(wishes: &[Wish]) -> Vec<Act> {
        wishes.iter().filter_map(|wish| wish.act).collect()
    }

    /// What it does follows from its seed: the same for the same seed, and
    /// different for another.
    #[test]
    fn it_does_the_same_for_the_same_seed() {
        let sights = vec![standing(0.65); 60 * 60];
        assert_eq!(
            watch(Level::Normal, 7, &sights),
            watch(Level::Normal, 7, &sights)
        );
        assert_ne!(
            watch(Level::Normal, 7, &sights),
            watch(Level::Normal, 8, &sights)
        );
    }

    /// It never answers what it has not seen yet: shown a punch at a moment,
    /// it does exactly what it would have done without it until its quickest
    /// reaction has passed, and then, more often than not, answers it.
    #[test]
    fn it_sees_a_punch_no_sooner_than_a_person_would() {
        let at = 120;
        let calm = vec![standing(0.62); 240];
        let mut punched = calm.clone();
        for (step, sight) in punched.iter_mut().enumerate().skip(at).take(20) {
            *sight = punching(0.62, (step - at) as f64 * DT);
        }
        for level in Level::ALL {
            let quickest = (level.traits().reaction.0 / DT).round() as usize;
            let mut answered = 0;
            for seed in 1..=100 {
                let (before, after) = (watch(level, seed, &calm), watch(level, seed, &punched));
                let parted = (0..before.len()).find(|&step| before[step] != after[step]);
                if let Some(parted) = parted {
                    assert!(
                        parted >= at + quickest,
                        "{level:?} {seed}: answered a punch {} steps after it set off",
                        parted - at
                    );
                    answered += 1;
                }
            }
            assert!(
                answered > 40,
                "{level:?} answered {answered} of 100 punches"
            );
        }
    }

    /// Out of reach it only comes in: it throws no punch that would fall
    /// short, and holds nothing.
    #[test]
    fn out_of_reach_it_steps_in() {
        let wishes = watch(Level::Normal, 3, &vec![standing(0.78); 30 * 60]);
        let acts = acts(&wishes);
        assert!(!acts.is_empty(), "it never stepped in");
        assert!(
            acts.iter().all(|&act| act == Act::StepIn),
            "out of reach it did {acts:?}"
        );
        assert!(wishes.iter().all(|wish| wish.holds == [false; 3]));
    }

    /// In reach of a fighter doing nothing it punches, more jabs than
    /// crosses, faster the harder it is, and keeps its guard up some of the
    /// time.
    #[test]
    fn in_reach_it_punches_and_sometimes_covers_up() {
        let sights = vec![standing(0.65); 120 * 60];
        let mut punches = Vec::new();
        for level in Level::ALL {
            let wishes = watch(level, 11, &sights);
            let acts = acts(&wishes);
            let count = |wanted: Act| acts.iter().filter(|&&act| act == wanted).count();
            let (jabs, crosses) = (count(Act::Jab), count(Act::Cross));
            assert!(
                jabs > 0 && crosses > 0,
                "{level:?}: {jabs} jabs, {crosses} crosses"
            );
            // Some go to the body, thrown with the duck held: about one in
            // four into a guard that is not up.
            let to_body = wishes
                .iter()
                .filter(|wish| {
                    matches!(wish.act, Some(Act::Jab | Act::Cross))
                        && wish.holds[Hold::Duck.index()]
                })
                .count() as f64
                / (jabs + crosses) as f64;
            assert!(
                (0.1..0.45).contains(&to_body),
                "{level:?} threw {:.0}% of its punches to the body",
                to_body * 100.0
            );
            let blocking = wishes
                .iter()
                .filter(|wish| wish.holds[Hold::Block.index()])
                .count() as f64
                / wishes.len() as f64;
            assert!(
                (0.03..0.6).contains(&blocking),
                "{level:?} blocked {:.0}% of the time",
                blocking * 100.0
            );
            punches.push(jabs + crosses);
        }
        assert!(
            punches[0] < punches[1] && punches[1] < punches[2],
            "punches thrown in two minutes, easy to hard: {punches:?}"
        );
    }

    /// Hit clean, it covers up, makes room, or fires back, at its level's
    /// chances: the harder it is, the more often it answers at all.
    #[test]
    fn hit_it_answers_more_often_the_harder_it_is() {
        let at = 60;
        let mut sights = vec![standing(0.62); 150];
        sights[at].hit = true;
        let calm = vec![standing(0.62); 150];
        let mut answered = Vec::new();
        for level in Level::ALL {
            let count = (1..=200)
                .filter(|&seed| watch(level, seed, &sights) != watch(level, seed, &calm))
                .count();
            answered.push(count);
        }
        assert!(
            answered[0] < answered[1] && answered[1] < answered[2],
            "of 200 hits, easy to hard answered {answered:?}"
        );
    }

    /// A fighter stepping back out of its reach is followed in.
    #[test]
    fn it_follows_a_fighter_who_steps_back() {
        let at = 60;
        let mut sights = vec![standing(0.62); 120];
        for (step, sight) in sights.iter_mut().enumerate().skip(at) {
            let progress = ((step - at) as f64 * DT / 0.6).min(1.0);
            sight.apart = 0.62 + 0.13 * progress;
            if progress < 1.0 {
                sight.them.step = Some(rp::Stepping {
                    forward: false,
                    progress,
                    catching: false,
                });
            }
        }
        let followed = (1..=100)
            .filter(|&seed| acts(&watch(Level::Normal, seed, &sights)[at..]).contains(&Act::StepIn))
            .count();
        assert!(followed > 60, "followed {followed} of 100 steps back");
    }

    /// It never steps while it leans or ducks, nor punches while its own
    /// punch is out: a move decided then waits, or is given up.
    #[test]
    fn it_waits_to_be_free_before_it_moves() {
        let leaning = rp::Doing {
            lean: 0.8,
            ..rp::Doing::default()
        };
        assert!(!free_to(Act::StepOut, leaning));
        assert!(free_to(Act::Jab, leaning));
        let punching = punching(0.6, 0.05).them;
        assert!(!free_to(Act::Cross, punching));
        assert!(free_to(Act::StepIn, punching));
    }
}
