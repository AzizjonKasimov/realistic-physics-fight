//! Game controllers. Each frame the page reads every connected controller
//! (`page::pads`, through the browser's Gamepad API) and the game turns what
//! is held on it into the fighters' moves, as `controls.rs` turns the keys.
//!
//! The D-pad or the left stick steps toward the other fighter and away from
//! it, leans back (up), and ducks (down); a step direction held keeps
//! stepping, a step at a time. The left trigger (L2, LT) jabs with the
//! front hand and the right trigger (R2, RT) crosses with the back hand, as
//! the left face button (square, X) and the top one (triangle, Y) do too
//! (the user's choice, 2026-11-17, after playing: the triggers are the two
//! fists); either shoulder button (R1 or L1, RB or LB) blocks while held.
//! The start button (OPTIONS, MENU) pauses, the bottom face button (cross,
//! A) fights again once a fight is over, and the select button (CREATE,
//! VIEW) moves the controller to the other fighter. The first controller
//! plays the left fighter and a second the right; the keyboard keeps both
//! of its halves.

use crate::controls::{Act, Hold};
use crate::game::{Feel, LEFT, RIGHT};

/// How a controller rumbles for what its player feels: its heavy motor's
/// strength and its light one's, from 0 to 1, and for how many
/// milliseconds. A punch of one's own landing is a light tick in the hands,
/// being hit a thump through both motors, and going down both at full for
/// most of a second.
pub fn rumble_for(feel: Feel) -> (f32, f32, u32) {
    match feel {
        Feel::Landed(_) => (0.0, 0.45, 90),
        Feel::Hit(_) => (0.7, 0.5, 200),
        Feel::Down(_) => (1.0, 1.0, 700),
    }
}

/// The kinds of controller the screen draws buttons for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadKind {
    /// Sony's (a DualSense, a DualShock 4): square, triangle, circle, and
    /// cross.
    PlayStation,
    /// Microsoft's: X, Y, B, and A.
    Xbox,
    /// Any other: its face buttons are shown by where they sit.
    Other,
}

impl PadKind {
    /// Every kind, for the capture mode's `--pad`.
    #[cfg(not(target_arch = "wasm32"))]
    pub const ALL: [PadKind; 3] = [PadKind::PlayStation, PadKind::Xbox, PadKind::Other];

    /// The kind the page tells by its number: 1, 2, or 3.
    pub fn from_code(code: u8) -> Option<PadKind> {
        match code {
            1 => Some(PadKind::PlayStation),
            2 => Some(PadKind::Xbox),
            3 => Some(PadKind::Other),
            _ => None,
        }
    }

    /// The name scripts and the page know it by.
    pub fn name(self) -> &'static str {
        match self {
            PadKind::PlayStation => "playstation",
            PadKind::Xbox => "xbox",
            PadKind::Other => "other",
        }
    }

    /// The name printed on its button that pauses.
    pub fn pause_name(self) -> &'static str {
        match self {
            PadKind::PlayStation => "OPTIONS",
            PadKind::Xbox => "MENU",
            PadKind::Other => "START",
        }
    }

    /// The name printed on its button that switches fighters.
    pub fn switch_name(self) -> &'static str {
        match self {
            PadKind::PlayStation => "CREATE",
            PadKind::Xbox => "VIEW",
            PadKind::Other => "SELECT",
        }
    }

    /// The names printed on its triggers, which jab and cross.
    pub fn jab_name(self) -> &'static str {
        match self {
            PadKind::Xbox => "LT",
            PadKind::PlayStation | PadKind::Other => "L2",
        }
    }

    pub fn cross_name(self) -> &'static str {
        match self {
            PadKind::Xbox => "RT",
            PadKind::PlayStation | PadKind::Other => "R2",
        }
    }

    /// The name printed on its right shoulder button, which blocks (as the
    /// left one does).
    pub fn block_name(self) -> &'static str {
        match self {
            PadKind::Xbox => "RB",
            PadKind::PlayStation | PadKind::Other => "R1",
        }
    }
}

/// The buttons as the browser's standard layout numbers them (the W3C
/// Gamepad API's "standard" mapping), which every kind is read into.
pub mod standard {
    /// The face buttons by where they sit: cross or A at the bottom, square
    /// or X at the left, triangle or Y at the top. The right one (1: circle
    /// or B) plays nothing.
    pub const BOTTOM: u32 = 0;
    pub const LEFT: u32 = 2;
    pub const TOP: u32 = 3;
    /// The shoulder buttons and the triggers.
    pub const L1: u32 = 4;
    pub const R1: u32 = 5;
    pub const L2: u32 = 6;
    pub const R2: u32 = 7;
    /// The two small buttons in the middle: CREATE or VIEW, OPTIONS or MENU.
    pub const SELECT: u32 = 8;
    pub const START: u32 = 9;
    /// The D-pad.
    pub const UP: u32 = 12;
    pub const DOWN: u32 = 13;
    pub const PAD_LEFT: u32 = 14;
    pub const PAD_RIGHT: u32 = 15;
}

/// Where a Sony controller's buttons land in the standard layout when the
/// browser gives them in the controller's own order (as Firefox did with a
/// DualSense before its bug 1849039): square, cross, circle, triangle, L1,
/// R1, L2, R2, CREATE, OPTIONS, L3, R3, the PS button, the touchpad.
const SONY_ORDER: [u32; 14] = [2, 0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 17];

/// One controller as the page read it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    /// The browser's number for it, its own while it stays connected.
    pub slot: u8,
    pub kind: PadKind,
    /// The browser lays its buttons out in the standard way; otherwise they
    /// come in the controller's own order.
    pub standard: bool,
    /// Each button down, a bit each, in the browser's order.
    pub buttons: u32,
    /// The left stick across (-1 left to 1 right) and up and down (-1 up to
    /// 1 down), then the right stick's.
    pub axes: [f32; 4],
    /// How far the left and right triggers are pulled, from 0 to 1, in the
    /// standard layout.
    pub triggers: [f32; 2],
}

/// The bytes the page writes for each controller (`web/index.html`): its
/// slot, its kind, 1 for a standard layout, one spare byte, its buttons as
/// a little-endian u32, four little-endian f32 axes, and how far its two
/// triggers are pulled as two more.
pub const RECORD: usize = 32;

impl Pad {
    /// A controller from the bytes the page wrote for it.
    pub fn from_record(record: &[u8]) -> Option<Pad> {
        if record.len() < RECORD {
            return None;
        }
        let word = |at: usize| [record[at], record[at + 1], record[at + 2], record[at + 3]];
        Some(Pad {
            slot: record[0],
            kind: PadKind::from_code(record[1])?,
            standard: record[2] & 1 == 1,
            buttons: u32::from_le_bytes(word(4)),
            axes: [8, 12, 16, 20].map(|at| f32::from_le_bytes(word(at))),
            triggers: [24, 28].map(|at| f32::from_le_bytes(word(at))),
        })
    }

    /// Its buttons in the standard layout, the triggers read by how far they
    /// are pulled against whether they were down last frame (`was`): a Sony
    /// controller's own order is known; any other's is taken as it comes,
    /// its triggers as buttons.
    fn standard_buttons(&self, was: u32) -> u32 {
        if self.standard {
            let mut bits = self.buttons & !(1 << standard::L2 | 1 << standard::R2);
            for (pull, button) in self.triggers.into_iter().zip([standard::L2, standard::R2]) {
                let down = was & (1 << button) != 0;
                if pull > if down { RELEASED } else { PULLED } {
                    bits |= 1 << button;
                }
            }
            return bits;
        }
        if self.kind != PadKind::PlayStation {
            return self.buttons;
        }
        SONY_ORDER
            .iter()
            .enumerate()
            .filter(|&(raw, _)| self.buttons & (1 << raw) != 0)
            .fold(0, |bits, (_, &to)| bits | 1 << to)
    }
}

/// How far a stick must go to count as pushed one way, and how far back
/// toward the middle it must come to count as let go: a stick held near
/// the line would otherwise press and let go many times a second.
const PUSHED: f32 = 0.5;
const LET_GO: f32 = 0.35;
/// The same for a trigger, which punches: pulled past half, and let go
/// below a third, so a trigger held near the line throws one punch.
const PULLED: f32 = 0.5;
const RELEASED: f32 = 0.3;

/// The four ways a stick or the D-pad points, in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Ways {
    left: bool,
    right: bool,
    up: bool,
    down: bool,
}

impl Ways {
    /// The ways `pad` points now, its stick read against how it pointed
    /// last frame (`was`).
    fn of(pad: &Pad, buttons: u32, was: Ways) -> Ways {
        let stick = |was: bool, along: f32| along > if was { LET_GO } else { PUSHED };
        let [across, down, ..] = pad.axes;
        let held = |button: u32| pad.standard && buttons & (1 << button) != 0;
        Ways {
            left: stick(was.left, -across) || held(standard::PAD_LEFT),
            right: stick(was.right, across) || held(standard::PAD_RIGHT),
            up: stick(was.up, -down) || held(standard::UP),
            down: stick(was.down, down) || held(standard::DOWN),
        }
    }
}

/// A controller the game has seen, and what it plays.
#[derive(Clone, Copy, Debug)]
struct Seen {
    slot: u8,
    kind: PadKind,
    /// The fighter it plays.
    fighter: usize,
    /// Its buttons last frame, in the standard layout, and the ways it
    /// pointed.
    buttons: u32,
    ways: Ways,
}

/// What the controllers ask for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadIntents {
    /// The punches pressed for, by fighter, in [`Act::ALL`]'s order; a step
    /// is asked for by `walks`.
    pub acts: [[bool; 4]; 2],
    /// The moves held, by fighter, in [`Hold::ALL`]'s order.
    pub holds: [[bool; 3]; 2],
    /// A step held toward the other fighter, and one away from it, by
    /// fighter: the fighter keeps stepping while it is held.
    pub walks: [[bool; 2]; 2],
    pub pause: bool,
    /// The bottom face button was pressed: once a fight is over, the next.
    pub again: bool,
    /// The kind of controller that pressed something for each fighter this
    /// frame, if one did.
    pub used: [Option<PadKind>; 2],
}

/// The controllers from one frame to the next.
#[derive(Debug, Default)]
pub struct Pads {
    seen: Vec<Seen>,
}

impl Pads {
    /// What the controllers connected now (`pads`) ask for. A controller
    /// first seen plays the left fighter if no other controller does, and
    /// the right otherwise; one that is gone is forgotten.
    pub fn read(&mut self, pads: &[Pad]) -> PadIntents {
        self.seen.retain(|seen| {
            pads.iter()
                .any(|pad| pad.slot == seen.slot && pad.kind == seen.kind)
        });
        let mut sorted: Vec<&Pad> = pads.iter().collect();
        sorted.sort_by_key(|pad| pad.slot);
        for pad in sorted {
            if self.seen.iter().any(|seen| seen.slot == pad.slot) {
                continue;
            }
            let fighter = if self.seen.iter().any(|seen| seen.fighter == LEFT) {
                RIGHT
            } else {
                LEFT
            };
            self.seen.push(Seen {
                slot: pad.slot,
                kind: pad.kind,
                fighter,
                // Buttons already down when it is first seen, such as the
                // press that made the browser show it, count as pressed.
                buttons: 0,
                ways: Ways::default(),
            });
        }
        let mut intents = PadIntents::default();
        for seen in &mut self.seen {
            let Some(pad) = pads.iter().find(|pad| pad.slot == seen.slot) else {
                continue;
            };
            let buttons = pad.standard_buttons(seen.buttons);
            let ways = Ways::of(pad, buttons, seen.ways);
            let down = |button: u32| buttons & (1 << button) != 0;
            let pressed = |button: u32| down(button) && seen.buttons & (1 << button) == 0;
            if pressed(standard::SELECT) {
                seen.fighter = 1 - seen.fighter;
            }
            let fighter = seen.fighter;
            // Toward the other fighter is right for the left fighter.
            let (toward, away) = if fighter == LEFT {
                (ways.right, ways.left)
            } else {
                (ways.left, ways.right)
            };
            let acts = &mut intents.acts[fighter];
            acts[Act::Jab.index()] |= pressed(standard::L2) || pressed(standard::LEFT);
            acts[Act::Cross.index()] |= pressed(standard::R2) || pressed(standard::TOP);
            let holds = &mut intents.holds[fighter];
            holds[Hold::Block.index()] |= down(standard::R1) || down(standard::L1);
            holds[Hold::Duck.index()] |= ways.down;
            holds[Hold::Lean.index()] |= ways.up;
            intents.walks[fighter][0] |= toward;
            intents.walks[fighter][1] |= away;
            intents.pause |= pressed(standard::START);
            intents.again |= pressed(standard::BOTTOM);
            let newly = buttons & !seen.buttons != 0
                || (ways.left && !seen.ways.left)
                || (ways.right && !seen.ways.right)
                || (ways.up && !seen.ways.up)
                || (ways.down && !seen.ways.down);
            if newly {
                intents.used[fighter] = Some(seen.kind);
            }
            seen.buttons = buttons;
            seen.ways = ways;
        }
        intents
    }

    /// The slots of the controllers that play `fighter`.
    pub fn playing(&self, fighter: usize) -> impl Iterator<Item = u8> + '_ {
        self.seen
            .iter()
            .filter(move |seen| seen.fighter == fighter)
            .map(|seen| seen.slot)
    }

    /// Each controller connected, in the order the game first saw them: its
    /// slot, its kind, and the fighter it plays.
    pub fn connected(&self) -> Vec<(u8, PadKind, usize)> {
        self.seen
            .iter()
            .map(|seen| (seen.slot, seen.kind, seen.fighter))
            .collect()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A standard controller of `kind` in `slot` with `buttons` down, its
    /// sticks at `axes`, and its triggers let go.
    pub fn pad(slot: u8, kind: PadKind, buttons: &[u32], axes: [f32; 4]) -> Pad {
        Pad {
            slot,
            kind,
            standard: true,
            buttons: buttons.iter().fold(0, |bits, button| bits | 1 << button),
            axes,
            triggers: [0.0; 2],
        }
    }

    /// The same controller with its left and right triggers pulled so far.
    pub fn pulled(pad: Pad, triggers: [f32; 2]) -> Pad {
        Pad { triggers, ..pad }
    }

    pub const STILL: [f32; 4] = [0.0; 4];

    /// The page's bytes for a controller read back as it wrote them.
    #[test]
    fn a_controllers_record_reads_back() {
        let mut record = vec![1, 2, 1, 0];
        record.extend((1u32 << 2 | 1 << 13).to_le_bytes());
        for axis in [0.75f32, -1.0, 0.0, 0.25, 0.6, 0.0] {
            record.extend(axis.to_le_bytes());
        }
        assert_eq!(record.len(), RECORD);
        assert_eq!(
            Pad::from_record(&record),
            Some(pulled(
                pad(1, PadKind::Xbox, &[2, 13], [0.75, -1.0, 0.0, 0.25]),
                [0.6, 0.0]
            ))
        );
        record[1] = 9;
        assert_eq!(Pad::from_record(&record), None, "an unknown kind");
        assert_eq!(Pad::from_record(&record[..10]), None, "a short record");
    }

    /// A trigger counts as pulled past half way and as let go only below a
    /// third, so a trigger held near the line throws one punch.
    #[test]
    fn a_trigger_pulls_past_half_and_lets_go_below_a_third() {
        let mut pads = Pads::default();
        let at = |pull: f32| pulled(pad(0, PadKind::PlayStation, &[], STILL), [pull, 0.0]);
        let jabs = |pads: &mut Pads, pull: f32| pads.read(&[at(pull)]).acts[LEFT][Act::Jab.index()];
        assert!(!jabs(&mut pads, 0.45));
        assert!(jabs(&mut pads, 0.55));
        assert!(!jabs(&mut pads, 0.4), "held, it jabbed again");
        assert!(!jabs(&mut pads, 0.6), "never let go, it jabbed again");
        assert!(!jabs(&mut pads, 0.2));
        assert!(
            jabs(&mut pads, 0.9),
            "let go and pulled again, it did not jab"
        );
    }

    /// The first controller plays the left fighter and the second the
    /// right, whichever slots the browser gives them; one that comes back
    /// after the first is gone plays the left fighter again.
    #[test]
    fn the_first_controller_plays_the_left_fighter_and_the_second_the_right() {
        let mut pads = Pads::default();
        pads.read(&[pad(3, PadKind::Xbox, &[], STILL)]);
        assert_eq!(pads.connected(), [(3, PadKind::Xbox, LEFT)]);
        pads.read(&[
            pad(3, PadKind::Xbox, &[], STILL),
            pad(0, PadKind::PlayStation, &[], STILL),
        ]);
        assert_eq!(
            pads.connected(),
            [(3, PadKind::Xbox, LEFT), (0, PadKind::PlayStation, RIGHT)]
        );
        pads.read(&[pad(0, PadKind::PlayStation, &[], STILL)]);
        assert_eq!(pads.connected(), [(0, PadKind::PlayStation, RIGHT)]);
        pads.read(&[
            pad(0, PadKind::PlayStation, &[], STILL),
            pad(1, PadKind::Other, &[], STILL),
        ]);
        assert_eq!(
            pads.connected(),
            [(0, PadKind::PlayStation, RIGHT), (1, PadKind::Other, LEFT)]
        );
    }

    /// The select button moves a controller to the other fighter and back.
    #[test]
    fn the_select_button_switches_fighters() {
        let mut pads = Pads::default();
        let select = pad(0, PadKind::PlayStation, &[standard::SELECT], STILL);
        let none = pad(0, PadKind::PlayStation, &[], STILL);
        pads.read(&[none]);
        pads.read(&[select]);
        assert_eq!(pads.playing(RIGHT).collect::<Vec<_>>(), [0]);
        // Held, it switches once.
        pads.read(&[select]);
        assert_eq!(pads.playing(RIGHT).collect::<Vec<_>>(), [0]);
        pads.read(&[none]);
        pads.read(&[select]);
        assert_eq!(pads.playing(LEFT).collect::<Vec<_>>(), [0]);
    }

    /// The stick counts as pushed past half way and as let go only once it
    /// comes back within about a third, so a stick held near the line does
    /// not flicker.
    #[test]
    fn the_stick_pushes_past_half_and_lets_go_near_the_middle() {
        let mut pads = Pads::default();
        let at = |down: f32| pad(0, PadKind::Xbox, &[], [0.0, down, 0.0, 0.0]);
        let ducking =
            |pads: &mut Pads, down: f32| pads.read(&[at(down)]).holds[LEFT][Hold::Duck.index()];
        assert!(!ducking(&mut pads, 0.45));
        assert!(ducking(&mut pads, 0.6));
        assert!(ducking(&mut pads, 0.4), "let go at 0.4");
        assert!(!ducking(&mut pads, 0.3));
        assert!(!ducking(&mut pads, 0.45), "pushed again at 0.45");
    }
}
