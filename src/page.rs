//! What the game tells the web page it runs in (`web/index.html`): the things
//! players do, each once per page load, for the site's visit counts; a press
//! of the Sandbox button, which goes back to the sandbox; and where its
//! buttons are, for checks that touch them as a finger does
//! (`window.__fightButtons`). The page provides all three through its
//! `fight` gl.js plugin. The desktop game has no page: it counts nothing and
//! has no sandbox to go back to.

use std::sync::atomic::{AtomicU8, Ordering};

use macroquad::math::Rect;

use crate::controls::Button;

/// Something a player does that the page counts the first time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// The player's fighter threw a punch.
    Punch,
    /// The player's fighter leaned back.
    Lean,
    /// A second player took over the right fighter.
    SecondPlayer,
    /// The fight was started afresh.
    Restart,
}

impl Moment {
    fn bit(self) -> u8 {
        match self {
            Moment::Punch => 1,
            Moment::Lean => 2,
            Moment::SecondPlayer => 4,
            Moment::Restart => 8,
        }
    }

    /// The name the page counts it under, after its own `fight-` prefix.
    pub fn name(self) -> &'static str {
        match self {
            Moment::Punch => "punch",
            Moment::Lean => "lean",
            Moment::SecondPlayer => "two-players",
            Moment::Restart => "restart",
        }
    }
}

/// The moments already told, a bit each.
static TOLD: AtomicU8 = AtomicU8::new(0);

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

/// Tells the page where the buttons are, in the page's pixels, by name:
/// `lean1`, `punch1`, `punch2`, `lean2`, `pause`, `restart`, `sound`,
/// `sandbox`.
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

fn button_name(button: Button) -> String {
    match button {
        Button::Lean(fighter) => format!("lean{}", fighter + 1),
        Button::Punch(fighter) => format!("punch{}", fighter + 1),
        Button::Pause => "pause".to_string(),
        Button::Restart => "restart".to_string(),
        Button::Sound => "sound".to_string(),
        Button::Sandbox => "sandbox".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    extern "C" {
        fn fight_event(name: *const u8, len: usize);
        fn fight_back();
        fn fight_buttons(json: *const u8, len: usize);
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

    /// gl.js checks the page's `fight` plugin version against this.
    #[no_mangle]
    pub extern "C" fn fight_crate_version() -> u32 {
        1
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    pub fn event(_name: &str) {}

    pub fn back() {}

    pub fn buttons(_json: &str) {}
}
