//! The players' keys and buttons, read once a frame into what they ask for.
//!
//! Each fighter has seven moves. Four are pressed: a jab, a cross, a step
//! toward the other fighter, and a step away from it. Three are held for as
//! long as the key or button is: block, duck, and lean back.
//!
//! The left fighter's player moves it with the left hand on W A S D (D steps
//! in, A steps back, W leans back, S ducks) and fights with the right on
//! J K L (jab, cross, block). A second player takes the right fighter with
//! the arrows (left steps in, right steps back, up leans back, down ducks)
//! and comma, full stop, and slash, or 1 2 3 on the number pad (jab, cross,
//! block). On the screen the left fighter's moves are buttons too, for a
//! mouse or for fingers: its steps, lean, and duck at the bottom left, under
//! a left thumb, and its jab, cross, and block at the bottom right, under a
//! right thumb; every finger on the screen counts. Game controllers drive
//! either fighter alongside all of these (`pads.rs`).

use macroquad::prelude::*;

use crate::game::LEFT;
use crate::hud::Layout;
use crate::pads::{Pad, PadIntents, PadKind, Pads};

/// A move a fighter makes once each time its key or button is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Jab,
    Cross,
    /// A step toward the other fighter, and one away from it.
    StepIn,
    StepOut,
}

impl Act {
    pub const ALL: [Act; 4] = [Act::Jab, Act::Cross, Act::StepIn, Act::StepOut];

    /// Its place in [`Act::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The name scripts and the page know it by.
    pub fn name(self) -> &'static str {
        match self {
            Act::Jab => "jab",
            Act::Cross => "cross",
            Act::StepIn => "in",
            Act::StepOut => "out",
        }
    }
}

/// A move a fighter makes for as long as its key or button is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    Block,
    Duck,
    /// Leaning back, out of a punch's reach.
    Lean,
}

impl Hold {
    pub const ALL: [Hold; 3] = [Hold::Block, Hold::Duck, Hold::Lean];

    /// Its place in [`Hold::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// The name scripts and the page know it by.
    pub fn name(self) -> &'static str {
        match self {
            Hold::Block => "block",
            Hold::Duck => "duck",
            Hold::Lean => "lean",
        }
    }
}

/// A button on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// A fighter's move, pressed or held: 0 the left fighter's, 1 the
    /// right's.
    Act(usize, Act),
    Hold(usize, Hold),
    Pause,
    Restart,
    Sound,
    /// Back to the sandbox's page, in the browser.
    Sandbox,
    /// The computer opponent's level: a press starts a fight against the
    /// next level.
    Level,
}

/// The keys for each fighter's pressed moves, in [`Act::ALL`]'s order, the
/// left fighter's first.
pub const ACT_KEYS: [[&[KeyCode]; 4]; 2] = [
    [&[KeyCode::J], &[KeyCode::K], &[KeyCode::D], &[KeyCode::A]],
    [
        &[KeyCode::Comma, KeyCode::Kp1],
        &[KeyCode::Period, KeyCode::Kp2],
        &[KeyCode::Left],
        &[KeyCode::Right],
    ],
];
/// The keys for each fighter's held moves, in [`Hold::ALL`]'s order.
pub const HOLD_KEYS: [[&[KeyCode]; 3]; 2] = [
    [&[KeyCode::L], &[KeyCode::S], &[KeyCode::W]],
    [
        &[KeyCode::Slash, KeyCode::Kp3],
        &[KeyCode::Down],
        &[KeyCode::Up],
    ],
];

/// What the players asked for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intents {
    /// The moves pressed for, by fighter, in [`Act::ALL`]'s order.
    pub acts: [[bool; 4]; 2],
    /// The moves held, by fighter, in [`Hold::ALL`]'s order.
    pub holds: [[bool; 3]; 2],
    /// A step held on a controller toward the other fighter, and one away
    /// from it, by fighter: the fighter keeps stepping while it is held.
    pub walks: [[bool; 2]; 2],
    pub pause: bool,
    pub restart: bool,
    /// A controller's bottom face button: the next fight, once this one is
    /// over.
    pub again: bool,
    pub mute: bool,
    pub sandbox: bool,
    /// A fight against the opponent's next level.
    pub level: bool,
    /// One of the game's own buttons or keys was pressed, which clicks.
    pub clicked: bool,
    /// A game controller pressed something for a fighter.
    pub controller: bool,
}

/// The players' keys, buttons, and controllers from one frame to the next.
#[derive(Debug, Default)]
pub struct Controls {
    /// The buttons held down last frame: a button counts as pressed on the
    /// frame a finger or the mouse first comes down on it.
    held: Vec<Button>,
    /// The screen has been touched, so the buttons are sized for fingers
    /// and show no keys.
    pub touch: bool,
    /// The controllers connected, and the fighter each plays.
    pub pads: Pads,
    /// The kind of controller the left fighter's player last used, whose
    /// buttons the screen shows; none once a key of theirs is pressed.
    pub prompts: Option<PadKind>,
}

impl Controls {
    /// The buttons held down now, drawn pressed.
    pub fn held(&self) -> &[Button] {
        &self.held
    }

    /// Whose buttons the screen shows for a second player and in its texts:
    /// the controller the left fighter's player uses, else the first
    /// controller connected, if any is.
    pub fn connected_kind(&self) -> Option<PadKind> {
        self.prompts
            .or_else(|| self.pads.connected().first().map(|&(_, kind, _)| kind))
    }

    /// Reads this frame's keys, the buttons under each finger, or under the
    /// mouse while its button is down, and the controllers connected now
    /// (`pads`).
    pub fn read(&mut self, layout: &Layout, pads: &[Pad]) -> Intents {
        let touches = touches();
        if !touches.is_empty() {
            self.touch = true;
        }
        let mut pointers: Vec<Vec2> = touches
            .iter()
            .filter(|touch| !matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled))
            // Touches come in the screen's own pixels, not the page's (the
            // mouse comes divided already): on a phone with three to a page
            // pixel, every finger missed its button.
            .map(|touch| touch.position / macroquad::miniquad::window::dpi_scale())
            .collect();
        if touches.is_empty() && is_mouse_button_down(MouseButton::Left) {
            pointers.push(mouse_position().into());
        }
        let held: Vec<Button> = pointers
            .iter()
            .filter_map(|&at| layout.button_at(at))
            .collect();
        let pressed: Vec<Button> = held
            .iter()
            .copied()
            .filter(|button| !self.held.contains(button))
            .collect();
        self.held = held;
        let keys = Keys {
            pressed: |key| is_key_pressed(key),
            down: |key| is_key_down(key),
        };
        let from_pads = self.pads.read(pads);
        self.prompts = prompts_after(self.prompts, left_keys_pressed(&keys), from_pads.used[LEFT]);
        intents_from(&self.held, &pressed, &keys, &from_pads)
    }
}

/// Whose buttons the screen shows for the left fighter after a frame:
/// a controller's once it is used for it, the keys once one of its keys is
/// pressed, the controller winning a frame with both.
pub fn prompts_after(
    was: Option<PadKind>,
    keys_pressed: bool,
    pad_used: Option<PadKind>,
) -> Option<PadKind> {
    match (pad_used, keys_pressed) {
        (Some(kind), _) => Some(kind),
        (None, true) => None,
        (None, false) => was,
    }
}

/// Whether one of the left fighter's own keys went down this frame.
pub fn left_keys_pressed<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool>(
    keys: &Keys<P, D>,
) -> bool {
    ACT_KEYS[LEFT]
        .iter()
        .chain(HOLD_KEYS[LEFT].iter())
        .flat_map(|keys| keys.iter())
        .any(|&key| (keys.pressed)(key))
}

/// How the keyboard stands this frame.
pub struct Keys<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool> {
    /// A key went down this frame.
    pub pressed: P,
    /// A key is down.
    pub down: D,
}

/// What the buttons `held` (`pressed` of them just now), the keys, and the
/// controllers (`pads`) ask for.
pub fn intents_from<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool>(
    held: &[Button],
    pressed: &[Button],
    keys: &Keys<P, D>,
    pads: &PadIntents,
) -> Intents {
    let mut intents = Intents::default();
    for fighter in 0..2 {
        for act in Act::ALL {
            intents.acts[fighter][act.index()] = pressed.contains(&Button::Act(fighter, act))
                || ACT_KEYS[fighter][act.index()]
                    .iter()
                    .any(|&key| (keys.pressed)(key))
                || pads.acts[fighter][act.index()];
        }
        for hold in Hold::ALL {
            intents.holds[fighter][hold.index()] = held.contains(&Button::Hold(fighter, hold))
                || HOLD_KEYS[fighter][hold.index()]
                    .iter()
                    .any(|&key| (keys.down)(key))
                || pads.holds[fighter][hold.index()];
        }
    }
    intents.walks = pads.walks;
    intents.pause = pressed.contains(&Button::Pause)
        || (keys.pressed)(KeyCode::Space)
        || (keys.pressed)(KeyCode::P)
        || pads.pause;
    intents.restart = pressed.contains(&Button::Restart) || (keys.pressed)(KeyCode::R);
    intents.again = pads.again;
    intents.mute = pressed.contains(&Button::Sound) || (keys.pressed)(KeyCode::M);
    intents.sandbox = pressed.contains(&Button::Sandbox) || (keys.pressed)(KeyCode::Escape);
    intents.level = pressed.contains(&Button::Level);
    intents.clicked = intents.pause || intents.restart || intents.mute || intents.level;
    intents.controller = pads.used.iter().any(Option::is_some);
    intents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pads::tests::{pad, pulled, STILL};
    use crate::pads::{standard, Pad};

    /// What the buttons and the keys ask for, with no controller connected.
    fn intents_from<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool>(
        held: &[Button],
        pressed: &[Button],
        keys: &Keys<P, D>,
    ) -> Intents {
        super::intents_from(held, pressed, keys, &PadIntents::default())
    }

    /// What the controllers connected now (`pads`) ask for, after the
    /// frames `seen` already read, with no key down: through the same path
    /// as the keys.
    fn from_pads(seen: &mut Pads, pads: &[Pad]) -> Intents {
        super::intents_from(&[], &[], &keys(&[], &[]), &seen.read(pads))
    }

    fn keys(
        pressed: &[KeyCode],
        down: &[KeyCode],
    ) -> Keys<impl Fn(KeyCode) -> bool, impl Fn(KeyCode) -> bool> {
        let (pressed, down) = (pressed.to_vec(), down.to_vec());
        Keys {
            pressed: move |key| pressed.contains(&key),
            down: move |key| down.contains(&key),
        }
    }

    /// What `intents` asks of `fighter`: its pressed moves and its held ones.
    fn asked(intents: &Intents, fighter: usize) -> (Vec<Act>, Vec<Hold>) {
        (
            Act::ALL
                .into_iter()
                .filter(|act| intents.acts[fighter][act.index()])
                .collect(),
            Hold::ALL
                .into_iter()
                .filter(|hold| intents.holds[fighter][hold.index()])
                .collect(),
        )
    }

    /// The left fighter moves on W A S D and fights on J K L, and none of
    /// its keys move the right fighter.
    #[test]
    fn the_left_fighter_answers_wasd_and_jkl() {
        for (key, act) in [
            (KeyCode::J, Act::Jab),
            (KeyCode::K, Act::Cross),
            (KeyCode::D, Act::StepIn),
            (KeyCode::A, Act::StepOut),
        ] {
            let intents = intents_from(&[], &[], &keys(&[key], &[]));
            assert_eq!(asked(&intents, 0), (vec![act], vec![]), "{key:?}");
            assert_eq!(asked(&intents, 1), (vec![], vec![]), "{key:?}");
        }
        for (key, hold) in [
            (KeyCode::L, Hold::Block),
            (KeyCode::S, Hold::Duck),
            (KeyCode::W, Hold::Lean),
        ] {
            let intents = intents_from(&[], &[], &keys(&[], &[key]));
            assert_eq!(asked(&intents, 0), (vec![], vec![hold]), "{key:?}");
            assert_eq!(asked(&intents, 1), (vec![], vec![]), "{key:?}");
        }
    }

    /// The right fighter moves on the arrows, toward the left fighter with
    /// the left arrow, and fights on comma, full stop, and slash, or the
    /// number pad's 1 2 3.
    #[test]
    fn the_right_fighter_answers_the_arrows_and_its_three_keys() {
        for (key, act) in [
            (KeyCode::Comma, Act::Jab),
            (KeyCode::Kp1, Act::Jab),
            (KeyCode::Period, Act::Cross),
            (KeyCode::Kp2, Act::Cross),
            (KeyCode::Left, Act::StepIn),
            (KeyCode::Right, Act::StepOut),
        ] {
            let intents = intents_from(&[], &[], &keys(&[key], &[]));
            assert_eq!(asked(&intents, 1), (vec![act], vec![]), "{key:?}");
            assert_eq!(asked(&intents, 0), (vec![], vec![]), "{key:?}");
        }
        for (key, hold) in [
            (KeyCode::Slash, Hold::Block),
            (KeyCode::Kp3, Hold::Block),
            (KeyCode::Down, Hold::Duck),
            (KeyCode::Up, Hold::Lean),
        ] {
            let intents = intents_from(&[], &[], &keys(&[], &[key]));
            assert_eq!(asked(&intents, 1), (vec![], vec![hold]), "{key:?}");
        }
    }

    /// A pressed move's button makes it once, as it is pressed; a held
    /// move's button holds it for as long as it is held.
    #[test]
    fn buttons_act_when_pressed_and_hold_while_held() {
        let held = [Button::Act(0, Act::Cross), Button::Hold(0, Hold::Block)];
        let pressed = intents_from(&held, &held, &keys(&[], &[]));
        assert_eq!(asked(&pressed, 0), (vec![Act::Cross], vec![Hold::Block]));
        let still_held = intents_from(&held, &[], &keys(&[], &[]));
        assert_eq!(asked(&still_held, 0), (vec![], vec![Hold::Block]));
    }

    /// The game's own keys pause, restart, mute, and go back.
    #[test]
    fn the_game_keys_do_what_they_say() {
        let intents = intents_from(
            &[],
            &[],
            &keys(
                &[KeyCode::Space, KeyCode::R, KeyCode::M, KeyCode::Escape],
                &[],
            ),
        );
        assert!(
            intents.pause && intents.restart && intents.mute && intents.sandbox && intents.clicked
        );
        let none = intents_from(&[], &[], &keys(&[], &[]));
        assert_eq!(none, Intents::default());
    }

    /// A controller's left trigger jabs and its right trigger crosses, once a
    /// pull, as its left and top face buttons do; either shoulder button
    /// blocks, and the D-pad or the left stick ducks down and leans back up
    /// while held: for the fighter it plays, the left for the first
    /// controller, as the keys do.
    #[test]
    fn a_controller_plays_its_fighter_as_the_keys_do() {
        let still = pad(0, PadKind::PlayStation, &[], STILL);
        for (down, act) in [
            (pulled(still, [1.0, 0.0]), Act::Jab),
            (pulled(still, [0.0, 1.0]), Act::Cross),
            (
                pad(0, PadKind::PlayStation, &[standard::LEFT], STILL),
                Act::Jab,
            ),
            (
                pad(0, PadKind::PlayStation, &[standard::TOP], STILL),
                Act::Cross,
            ),
        ] {
            let mut seen = Pads::default();
            from_pads(&mut seen, &[still]);
            let intents = from_pads(&mut seen, &[down]);
            assert_eq!(asked(&intents, 0), (vec![act], vec![]), "{down:?}");
            assert_eq!(asked(&intents, 1), (vec![], vec![]), "{down:?}");
            let held = from_pads(&mut seen, &[down]);
            assert_eq!(asked(&held, 0), (vec![], vec![]), "{down:?} held");
        }
        let stick = |down: f32| [0.0, down, 0.0, 0.0];
        for (buttons, axes, hold) in [
            (&[standard::R1][..], STILL, Hold::Block),
            (&[standard::L1][..], STILL, Hold::Block),
            (&[standard::DOWN][..], STILL, Hold::Duck),
            (&[][..], stick(0.9), Hold::Duck),
            (&[standard::UP][..], STILL, Hold::Lean),
            (&[][..], stick(-0.9), Hold::Lean),
        ] {
            let mut seen = Pads::default();
            let intents = from_pads(&mut seen, &[pad(0, PadKind::Xbox, buttons, axes)]);
            assert_eq!(
                asked(&intents, 0),
                (vec![], vec![hold]),
                "{buttons:?} {axes:?}"
            );
        }
    }

    /// Toward the other fighter is right on the left fighter's controller
    /// and left on the right fighter's, on the D-pad or the stick; a step
    /// held asks to keep stepping.
    #[test]
    fn a_controller_steps_toward_the_other_fighter() {
        let mut seen = Pads::default();
        let across = |x: f32| [x, 0.0, 0.0, 0.0];
        let toward = from_pads(
            &mut seen,
            &[
                pad(0, PadKind::PlayStation, &[standard::PAD_RIGHT], STILL),
                pad(1, PadKind::Xbox, &[], across(-0.8)),
            ],
        );
        assert_eq!(toward.walks, [[true, false], [true, false]]);
        assert_eq!(
            toward.acts, [[false; 4]; 2],
            "a step is walked, not pressed"
        );
        let away = from_pads(
            &mut seen,
            &[
                pad(0, PadKind::PlayStation, &[standard::PAD_LEFT], STILL),
                pad(1, PadKind::Xbox, &[], across(0.8)),
            ],
        );
        assert_eq!(away.walks, [[false, true], [false, true]]);
    }

    /// The keyboard plays the left fighter while a controller switched to
    /// the right fighter plays that one: each moves only its own.
    #[test]
    fn the_keyboard_and_a_controller_fight_each_other() {
        let mut seen = Pads::default();
        seen.read(&[pad(0, PadKind::PlayStation, &[], STILL)]);
        seen.read(&[pad(0, PadKind::PlayStation, &[standard::SELECT], STILL)]);
        let pads = seen.read(&[pad(
            0,
            PadKind::PlayStation,
            &[standard::LEFT, standard::R1],
            STILL,
        )]);
        let intents = super::intents_from(&[], &[], &keys(&[KeyCode::K], &[KeyCode::S]), &pads);
        assert_eq!(asked(&intents, 0), (vec![Act::Cross], vec![Hold::Duck]));
        assert_eq!(asked(&intents, 1), (vec![Act::Jab], vec![Hold::Block]));
    }

    /// A controller's start button pauses as Space does, and its bottom
    /// face button asks for the next fight.
    #[test]
    fn a_controllers_start_pauses_and_its_bottom_button_fights_again() {
        let mut seen = Pads::default();
        from_pads(&mut seen, &[pad(0, PadKind::Other, &[], STILL)]);
        let start = from_pads(
            &mut seen,
            &[pad(0, PadKind::Other, &[standard::START], STILL)],
        );
        assert!(start.pause && start.clicked && !start.again);
        let bottom = from_pads(
            &mut seen,
            &[pad(0, PadKind::Other, &[standard::BOTTOM], STILL)],
        );
        assert!(bottom.again && !bottom.pause);
        assert_eq!(asked(&bottom, 0), (vec![], vec![]));
    }

    /// A Sony controller whose buttons come in its own order (as Firefox
    /// gave a DualSense's before its bug 1849039) jabs on square, crosses on
    /// triangle, and blocks on R1 all the same.
    #[test]
    fn a_sony_controller_in_its_own_order_plays_the_same() {
        let mut seen = Pads::default();
        let own_order = |buttons: &[u32]| Pad {
            standard: false,
            ..pad(0, PadKind::PlayStation, buttons, STILL)
        };
        from_pads(&mut seen, &[own_order(&[])]);
        let square = from_pads(&mut seen, &[own_order(&[0])]);
        assert_eq!(asked(&square, 0), (vec![Act::Jab], vec![]));
        let triangle = from_pads(&mut seen, &[own_order(&[3, 5])]);
        assert_eq!(asked(&triangle, 0), (vec![Act::Cross], vec![Hold::Block]));
    }

    /// The screen shows a controller's buttons once it is used for the left
    /// fighter, and the keys again once one of the left fighter's keys is
    /// pressed; the right fighter's keys change nothing.
    #[test]
    fn the_screen_shows_the_buttons_last_used() {
        let xbox = Some(PadKind::Xbox);
        assert_eq!(prompts_after(None, false, xbox), xbox);
        assert_eq!(prompts_after(xbox, false, None), xbox);
        assert_eq!(prompts_after(xbox, true, None), None);
        assert_eq!(prompts_after(None, true, xbox), xbox);
        assert!(left_keys_pressed(&keys(&[KeyCode::J], &[])));
        assert!(!left_keys_pressed(&keys(&[KeyCode::Comma], &[])));
        let mut seen = Pads::default();
        seen.read(&[pad(0, PadKind::Xbox, &[], STILL)]);
        let used = seen.read(&[pad(0, PadKind::Xbox, &[], [0.0, 0.9, 0.0, 0.0])]);
        assert_eq!(used.used, [xbox, None]);
    }
}
