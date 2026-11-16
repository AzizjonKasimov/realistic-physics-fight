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
//! right thumb; every finger on the screen counts.

use macroquad::prelude::*;

use crate::hud::Layout;

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
    pub pause: bool,
    pub restart: bool,
    pub mute: bool,
    pub sandbox: bool,
    /// A fight against the opponent's next level.
    pub level: bool,
    /// One of the game's own buttons or keys was pressed, which clicks.
    pub clicked: bool,
}

/// The players' keys and buttons from one frame to the next.
#[derive(Debug, Default)]
pub struct Controls {
    /// The buttons held down last frame: a button counts as pressed on the
    /// frame a finger or the mouse first comes down on it.
    held: Vec<Button>,
    /// The screen has been touched, so the buttons are sized for fingers
    /// and show no keys.
    pub touch: bool,
}

impl Controls {
    /// The buttons held down now, drawn pressed.
    pub fn held(&self) -> &[Button] {
        &self.held
    }

    /// Reads this frame's keys and the buttons under each finger, or under
    /// the mouse while its button is down.
    pub fn read(&mut self, layout: &Layout) -> Intents {
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
        intents_from(&self.held, &pressed, &keys)
    }
}

/// How the keyboard stands this frame.
pub struct Keys<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool> {
    /// A key went down this frame.
    pub pressed: P,
    /// A key is down.
    pub down: D,
}

/// What the buttons `held` (`pressed` of them just now) and the keys ask for.
pub fn intents_from<P: Fn(KeyCode) -> bool, D: Fn(KeyCode) -> bool>(
    held: &[Button],
    pressed: &[Button],
    keys: &Keys<P, D>,
) -> Intents {
    let mut intents = Intents::default();
    for fighter in 0..2 {
        for act in Act::ALL {
            intents.acts[fighter][act.index()] = pressed.contains(&Button::Act(fighter, act))
                || ACT_KEYS[fighter][act.index()]
                    .iter()
                    .any(|&key| (keys.pressed)(key));
        }
        for hold in Hold::ALL {
            intents.holds[fighter][hold.index()] = held.contains(&Button::Hold(fighter, hold))
                || HOLD_KEYS[fighter][hold.index()]
                    .iter()
                    .any(|&key| (keys.down)(key));
        }
    }
    intents.pause = pressed.contains(&Button::Pause)
        || (keys.pressed)(KeyCode::Space)
        || (keys.pressed)(KeyCode::P);
    intents.restart = pressed.contains(&Button::Restart) || (keys.pressed)(KeyCode::R);
    intents.mute = pressed.contains(&Button::Sound) || (keys.pressed)(KeyCode::M);
    intents.sandbox = pressed.contains(&Button::Sandbox) || (keys.pressed)(KeyCode::Escape);
    intents.level = pressed.contains(&Button::Level);
    intents.clicked = intents.pause || intents.restart || intents.mute || intents.level;
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
