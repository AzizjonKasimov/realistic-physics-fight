//! What the game tells the web page it runs in (`web/index.html`), and asks
//! it: the things players do, each once per page load, for the site's visit
//! counts; a press of the Sandbox button, which goes back to the sandbox;
//! where its buttons are, for checks that touch them as a finger does
//! (`window.__fightButtons`); the opponent's level the player chose on the
//! start card; the game controllers connected, read each frame from the
//! browser's Gamepad API, and their rumble; and which fighter each
//! controller plays (`window.__fightPads`), for checks. The page provides
//! all of these through its `fight` gl.js plugin. The desktop game has no
//! page: it counts nothing, has no sandbox to go back to, starts at the
//! normal level, and reads no controllers.

use std::sync::atomic::{AtomicU16, Ordering};

use macroquad::math::Rect;

use crate::controls::Button;
use crate::opponent::Level;
use crate::pads::{Pad, PadKind};

/// Something a player does that the page counts the first time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// The player's fighter threw a jab or a cross.
    Punch,
    /// The player's fighter blocked, ducked, or leaned back.
    Block,
    Duck,
    Lean,
    /// The player's fighter took a step.
    Step,
    /// A second player took over the right fighter.
    SecondPlayer,
    /// The fight was started afresh.
    Restart,
    /// A fighter was knocked out, either one.
    Knockout,
    /// A game controller played a fighter.
    Controller,
}

impl Moment {
    fn bit(self) -> u16 {
        match self {
            Moment::Punch => 1,
            Moment::Lean => 2,
            Moment::SecondPlayer => 4,
            Moment::Restart => 8,
            Moment::Block => 16,
            Moment::Duck => 32,
            Moment::Step => 64,
            Moment::Knockout => 128,
            Moment::Controller => 256,
        }
    }

    /// The name the page counts it under, after its own `fight-` prefix.
    pub fn name(self) -> &'static str {
        match self {
            Moment::Punch => "punch",
            Moment::Block => "block",
            Moment::Duck => "duck",
            Moment::Lean => "lean",
            Moment::Step => "step",
            Moment::SecondPlayer => "two-players",
            Moment::Restart => "restart",
            Moment::Knockout => "knockout",
            Moment::Controller => "controller",
        }
    }
}

/// The moments already told, a bit each.
static TOLD: AtomicU16 = AtomicU16::new(0);

/// Tells the page about `moment` the first time it happens.
pub fn tell(moment: Moment) {
    let bit = moment.bit();
    if TOLD.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        platform::event(moment.name());
    }
}

/// Whether there is a sandbox to go back to: in the browser, the page the
/// Fight page sits under.
pub fn has_sandbox() -> bool {
    cfg!(target_arch = "wasm32")
}

/// Goes back to the sandbox's page.
pub fn back_to_sandbox() {
    platform::back();
}

/// The opponent's level the player chose on the page's start card: normal
/// where there is no page, or it says none.
pub fn chosen_level() -> Level {
    Level::from_index(platform::level()).unwrap_or(Level::Normal)
}

/// The most controllers the page reports at once.
const MOST_PADS: usize = 8;

/// The game controllers connected now, as the page read them this frame.
pub fn pads() -> Vec<Pad> {
    let mut records = [0u8; MOST_PADS * crate::pads::RECORD];
    let count = platform::pads(&mut records).min(MOST_PADS);
    records
        .chunks(crate::pads::RECORD)
        .take(count)
        .filter_map(Pad::from_record)
        .collect()
}

/// Rumbles the controller in `slot`: its heavy motor at `strong` and its
/// light one at `weak`, from 0 to 1, for `ms` milliseconds, where the
/// browser can.
pub fn rumble(slot: u8, strong: f32, weak: f32, ms: u32) {
    platform::rumble(slot, strong, weak, ms);
}

/// Tells the page which fighter each controller plays (0 the left, 1 the
/// right) and whose buttons the screen shows, for checks
/// (`window.__fightPads`).
pub fn show_pads(pads: &[(u8, PadKind, usize)], prompts: Option<PadKind>) {
    let list: Vec<String> = pads
        .iter()
        .map(|(slot, kind, fighter)| {
            format!(
                "{{\"slot\":{slot},\"kind\":\"{}\",\"fighter\":{fighter}}}",
                kind.name()
            )
        })
        .collect();
    let shown = prompts.map_or("keys", PadKind::name);
    platform::pads_seen(&format!(
        "{{\"pads\":[{}],\"prompts\":\"{shown}\"}}",
        list.join(",")
    ));
}

/// Tells the page where the buttons are, in the page's pixels, by name:
/// the left fighter's `jab1`, `cross1`, `in1`, `out1`, `block1`, `duck1`,
/// and `lean1`, then `pause`, `restart`, `sound`, `sandbox`.
pub fn show_buttons(buttons: &[(Button, Rect)]) {
    let list: Vec<String> = buttons
        .iter()
        .map(|(button, rect)| {
            format!(
                "{{\"name\":\"{}\",\"x\":{:.1},\"y\":{:.1},\"w\":{:.1},\"h\":{:.1}}}",
                button_name(*button),
                rect.x,
                rect.y,
                rect.w,
                rect.h
            )
        })
        .collect();
    platform::buttons(&format!("[{}]", list.join(",")));
}

/// A button's name: a fighter's move with the fighter's number after it,
/// as scripts write it (`capture.rs`).
pub fn button_name(button: Button) -> String {
    match button {
        Button::Act(fighter, act) => format!("{}{}", act.name(), fighter + 1),
        Button::Hold(fighter, hold) => format!("{}{}", hold.name(), fighter + 1),
        Button::Pause => "pause".to_string(),
        Button::Restart => "restart".to_string(),
        Button::Sound => "sound".to_string(),
        Button::Sandbox => "sandbox".to_string(),
        Button::Level => "level".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    extern "C" {
        fn fight_event(name: *const u8, len: usize);
        fn fight_back();
        fn fight_buttons(json: *const u8, len: usize);
        fn fight_level() -> u32;
        fn fight_pads(records: *mut u8, most: u32) -> u32;
        fn fight_rumble(slot: u32, strong: f32, weak: f32, ms: u32);
        fn fight_pads_seen(json: *const u8, len: usize);
    }

    /// Fills `records` with a record (`pads::RECORD` bytes) for each
    /// controller connected and returns how many it wrote.
    pub fn pads(records: &mut [u8]) -> usize {
        let most = (records.len() / crate::pads::RECORD) as u32;
        // SAFETY: the page provides this through its `fight` gl.js plugin;
        // it writes at most `most` records into the buffer it is given.
        unsafe { fight_pads(records.as_mut_ptr(), most) as usize }
    }

    pub fn rumble(slot: u8, strong: f32, weak: f32, ms: u32) {
        // SAFETY: as `pads`; it takes numbers only.
        unsafe { fight_rumble(u32::from(slot), strong, weak, ms) }
    }

    pub fn pads_seen(json: &str) {
        // SAFETY: as `event`.
        unsafe { fight_pads_seen(json.as_ptr(), json.len()) }
    }

    pub fn level() -> usize {
        // SAFETY: the page provides this through its `fight` gl.js plugin;
        // it takes nothing and returns a number.
        unsafe { fight_level() as usize }
    }

    pub fn buttons(json: &str) {
        // SAFETY: as `event`.
        unsafe { fight_buttons(json.as_ptr(), json.len()) }
    }

    pub fn event(name: &str) {
        // SAFETY: the page provides this through its `fight` gl.js plugin,
        // which copies the string before it returns.
        unsafe { fight_event(name.as_ptr(), name.len()) }
    }

    pub fn back() {
        // SAFETY: as above.
        unsafe { fight_back() }
    }

    /// gl.js checks the page's `fight` plugin version against this: 2 since
    /// the page tells the game the opponent's level, 3 since it reads the
    /// game controllers.
    #[no_mangle]
    pub extern "C" fn fight_crate_version() -> u32 {
        3
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    pub fn level() -> usize {
        1
    }

    pub fn pads(_records: &mut [u8]) -> usize {
        0
    }

    pub fn rumble(_slot: u8, _strong: f32, _weak: f32, _ms: u32) {}

    pub fn pads_seen(_json: &str) {}

    pub fn event(_name: &str) {}

    pub fn back() {}

    pub fn buttons(_json: &str) {}
}
