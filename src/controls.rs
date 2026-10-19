//! The players' keys and buttons, read once a frame into what they ask for.
//!
//! Each fighter has two moves: a punch, with the arm nearer the other
//! fighter, and lean back, held for as long as the key or button is. The
//! left fighter's keys lie under a left hand (D punches, A leans back) and
//! on the arrows for a player alone (→ punches toward the other fighter, ←
//! leans away from it); the right fighter's lie under a right hand (J
//! punches toward the left, L leans away). On a touch screen each fighter's
//! two buttons sit at its own side of the bottom, and two players can hold
//! a phone between them, each a side; every finger on the screen counts.

use macroquad::prelude::*;

use crate::hud::Layout;

/// A button on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// A fighter's lean back, held, and its punch, pressed: 0 the left
    /// fighter's, 1 the right's.
    Lean(usize),
    Punch(usize),
    Pause,
    Restart,
    Sound,
    /// Back to the sandbox's page, in the browser.
    Sandbox,
}

/// The keys that punch with each fighter, the left fighter's first.
pub const PUNCH_KEYS: [&[KeyCode]; 2] = [&[KeyCode::D, KeyCode::Right], &[KeyCode::J]];
/// The keys that hold each fighter's lean back.
pub const LEAN_KEYS: [&[KeyCode]; 2] = [&[KeyCode::A, KeyCode::Left], &[KeyCode::L]];

/// What the players asked for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intents {
    /// A punch pressed for, by fighter.
    pub punch: [bool; 2],
    /// Lean back held, by fighter.
    pub lean: [bool; 2],
    pub pause: bool,
    pub restart: bool,
    pub mute: bool,
    pub sandbox: bool,
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
        intents.punch[fighter] = pressed.contains(&Button::Punch(fighter))
            || PUNCH_KEYS[fighter].iter().any(|&key| (keys.pressed)(key));
        intents.lean[fighter] = held.contains(&Button::Lean(fighter))
            || LEAN_KEYS[fighter].iter().any(|&key| (keys.down)(key));
    }
    intents.pause = pressed.contains(&Button::Pause)
        || (keys.pressed)(KeyCode::Space)
        || (keys.pressed)(KeyCode::P);
    intents.restart = pressed.contains(&Button::Restart) || (keys.pressed)(KeyCode::R);
    intents.mute = pressed.contains(&Button::Sound) || (keys.pressed)(KeyCode::M);
    intents.sandbox = pressed.contains(&Button::Sandbox) || (keys.pressed)(KeyCode::Escape);
    intents.clicked = intents.pause || intents.restart || intents.mute;
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

    /// A player alone plays the left fighter on the arrows as on D and A:
    /// toward the other fighter punches, away leans back.
    #[test]
    fn the_left_fighter_answers_its_letters_and_the_arrows() {
        for punch in [KeyCode::D, KeyCode::Right] {
            let intents = intents_from(&[], &[], &keys(&[punch], &[]));
            assert_eq!(intents.punch, [true, false], "{punch:?}");
        }
        for lean in [KeyCode::A, KeyCode::Left] {
            let intents = intents_from(&[], &[], &keys(&[], &[lean]));
            assert_eq!(intents.lean, [true, false], "{lean:?}");
        }
    }

    /// The right fighter answers J and L, and its keys never move the left.
    #[test]
    fn the_right_fighter_answers_j_and_l() {
        let intents = intents_from(&[], &[], &keys(&[KeyCode::J], &[KeyCode::L]));
        assert_eq!(
            (intents.punch, intents.lean),
            ([false, true], [false, true])
        );
    }

    /// A punch button punches once, as it is pressed; a lean button leans
    /// for as long as it is held.
    #[test]
    fn buttons_punch_when_pressed_and_lean_while_held() {
        let held = [Button::Punch(0), Button::Lean(1)];
        let pressed = intents_from(&held, &held, &keys(&[], &[]));
        assert_eq!(
            (pressed.punch, pressed.lean),
            ([true, false], [false, true])
        );
        let still_held = intents_from(&held, &[], &keys(&[], &[]));
        assert_eq!(
            (still_held.punch, still_held.lean),
            ([false, false], [false, true])
        );
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
