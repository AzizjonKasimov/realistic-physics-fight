//! The screen around the fighters: the game's name and version and its few
//! buttons along the top, a tag over each fighter saying who plays it, and
//! each fighter's LEAN BACK and PUNCH buttons along the bottom, the left
//! fighter's at the left and the right fighter's at the right, sized for
//! fingers. The fighters are drawn by the engine (`rp::draw`), as large as
//! fits between the two rows with the room the camera leaves them
//! (`camera.rs`).

use std::sync::atomic::{AtomicU32, Ordering};

use macroquad::prelude::*;
use realistic_physics as rp;
use rp::draw::{with_alpha, Palette};

use crate::camera::Camera;
use crate::controls::Button;
use crate::game::{version_label, Game, LEFT, RIGHT};

/// Room kept round the edges of the window and between the rows and the
/// fighters, in screen pixels.
const MARGIN: f32 = 10.0;
/// The height of the buttons along the top.
const CHIP_HEIGHT: f32 = 30.0;
/// The height of the fighters' buttons, for a mouse and for fingers.
const FIGHT_BUTTON_HEIGHT: [f32; 2] = [62.0, 78.0];
/// The least room left beside the fighters at each side of the window.
const FIGHTER_SIDE_MARGIN: f64 = 12.0;
/// Windows narrower than this get the buttons' short labels.
const NARROW_WIDTH: f32 = 640.0;
/// Every size the HUD writes text at.
const TEXT_SIZES: [f32; 6] = [14.0, 15.0, 16.0, 17.0, 18.0, 34.0];

/// The two lines the fighters are drawn between in a window `height` tall:
/// under the top row of buttons, and over the fighters' buttons.
fn fighters_between(height: f32, touch: bool) -> (f32, f32) {
    let tall = FIGHT_BUTTON_HEIGHT[usize::from(touch)];
    (
        MARGIN + CHIP_HEIGHT + MARGIN + 34.0,
        height - MARGIN - tall - MARGIN,
    )
}

/// Where everything around the fighters sits in a window.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    /// The fighters are drawn between these two lines.
    pub top: f32,
    pub bottom: f32,
    /// Each button and where it is.
    pub buttons: Vec<(Button, Rect)>,
    /// The buttons are sized for fingers and show no keys.
    pub touch: bool,
    /// The window is too narrow for the buttons' longer labels.
    pub narrow: bool,
}

impl Layout {
    /// The layout of a window `width` by `height`, with a Sandbox button
    /// where there is a sandbox to go back to.
    pub fn new(width: f32, height: f32, touch: bool, sandbox: bool) -> Layout {
        let narrow = width < NARROW_WIDTH;
        let mut buttons = Vec::new();
        // Along the top, right to left.
        let mut x = width - MARGIN;
        let top = [
            (Button::Sandbox, sandbox),
            (Button::Sound, true),
            (Button::Restart, true),
            (Button::Pause, true),
        ];
        for (button, shown) in top {
            if !shown {
                continue;
            }
            let w = chip_width(&top_label(button, narrow, false, false));
            x -= w;
            buttons.push((button, Rect::new(x, MARGIN, w, CHIP_HEIGHT)));
            x -= 6.0;
        }
        // Along the bottom: each fighter's two at its own side.
        let tall = FIGHT_BUTTON_HEIGHT[usize::from(touch)];
        let gap = 8.0;
        let each = ((width - 2.0 * MARGIN - 3.0 * gap - 24.0) / 4.0).clamp(60.0, 190.0);
        let (top, bottom) = fighters_between(height, touch);
        let y = bottom + MARGIN;
        let lefts = [
            MARGIN,
            MARGIN + each + gap,
            width - MARGIN - 2.0 * each - gap,
            width - MARGIN - each,
        ];
        let fighters = [
            Button::Lean(LEFT),
            Button::Punch(LEFT),
            Button::Punch(RIGHT),
            Button::Lean(RIGHT),
        ];
        for (button, left) in fighters.into_iter().zip(lefts) {
            buttons.push((button, Rect::new(left, y, each, tall)));
        }
        Layout {
            width,
            height,
            top,
            bottom,
            buttons,
            touch,
            narrow,
        }
    }

    /// The layout of a window without its buttons, which need the window
    /// to measure their labels: for tests of where the fighters stand.
    #[cfg(test)]
    pub fn without_buttons(width: f32, height: f32, touch: bool) -> Layout {
        let (top, bottom) = fighters_between(height, touch);
        Layout {
            width,
            height,
            top,
            bottom,
            buttons: Vec::new(),
            touch,
            narrow: width < NARROW_WIDTH,
        }
    }

    /// The button at screen position `at`, if any.
    pub fn button_at(&self, at: Vec2) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .map(|&(button, _)| button)
    }

    /// Where the scenarios' body frame is drawn: the two fighters as large as
    /// fits between the top row and the fighters' buttons with the room
    /// `camera` leaves each, standing just above the buttons (on a tall
    /// phone, near the thumbs, the room left over above them), the world's
    /// middle at the window's middle unless that would put a fighter's room
    /// off the screen. With room for the guards alone, as the fighters
    /// stand, it is the sandbox's fit (`rp::fighters_across`).
    pub fn fighters_frame(&self, camera: &Camera) -> rp::BodyFrame {
        let between =
            rp::body_frame_between(self.width as f64, self.top as f64, self.bottom as f64);
        let world = rp::world_frame();
        let [left, right] = rp::fighter_frames();
        let from = left.origin.x - camera.reach[LEFT] * world.height;
        let to = right.origin.x + camera.reach[RIGHT] * world.height;
        let width = self.width as f64;
        let room = (width - 2.0 * FIGHTER_SIDE_MARGIN).max(1.0);
        let scale = (between.height / world.height).min(room / (to - from));
        // A world position x lands on the screen at x * scale + offset.
        let middle = (left.origin.x + right.origin.x) * 0.5;
        let offset = (width * 0.5 - middle * scale)
            .max(FIGHTER_SIDE_MARGIN - from * scale)
            .min(width - FIGHTER_SIDE_MARGIN - to * scale);
        rp::BodyFrame {
            origin: rp::Vec2 {
                x: offset + world.origin.x * scale,
                y: self.bottom as f64 - world.height * scale,
            },
            height: world.height * scale,
        }
    }

    /// How the world is drawn in this window, with the room `camera` leaves
    /// the fighters.
    pub fn view(&self, camera: &Camera) -> rp::View {
        rp::View::fitting(rp::world_frame(), self.fighters_frame(camera))
    }
}

/// Draws the fight and everything around it, with the buttons in `held`
/// drawn pressed.
pub fn draw(game: &Game, layout: &Layout, held: &[Button]) {
    cache_letters();
    let palette = Palette::default();
    let view = layout.view(&game.camera);
    let floor_y = rp::draw::ground_on_screen(&game.world, view, layout.height);
    let world_floor_y = view
        .to_world(rp::Vec2 {
            x: 0.0,
            y: floor_y as f64,
        })
        .y as f32;
    let scene = rp::draw::Scene {
        world: &game.world,
        rim: &game.rim,
        palette,
        width: layout.width,
        height: layout.height,
        floor_y,
        world_floor_y,
        anatomy: false,
        debug: false,
        show_tool: false,
        tool_blood: 0.0,
        pressing: false,
    };
    rp::draw::draw_background(&scene);
    set_camera(&rp::draw::WorldCamera::of(view));
    rp::draw::draw_body_layers(&scene);
    rp::draw::draw_effects(&scene);
    set_default_camera();
    draw_tags(game, view, &palette);
    draw_title(&palette);
    for &(button, rect) in &layout.buttons {
        draw_button(game, layout, button, rect, held.contains(&button), &palette);
    }
    if !game.running {
        draw_paused(layout, &palette);
    }
}

/// Puts every letter the HUD can write, at every size it writes them, in
/// the font's atlas before the frame writes any, again whenever the
/// screen's density changes. Text is measured at its size and drawn at its
/// size times the density, and a letter new to the atlas can grow it.
/// Grown in the middle of a frame, macroquad deletes the atlas's texture
/// under the text the frame already wrote: the browser logs
/// "glBindTexture called with an already deleted texture" for each, and
/// that text is lost for the frame (v0.1.2 on a 390x844 phone, where
/// "engine v0.15.2" brought a new letter).
fn cache_letters() {
    static CACHED_AT: AtomicU32 = AtomicU32::new(0);
    let density = macroquad::miniquad::window::dpi_scale();
    if CACHED_AT.swap(density.to_bits(), Ordering::Relaxed) == density.to_bits() {
        return;
    }
    let font = get_default_font();
    let letters: Vec<char> = (' '..='~').collect();
    for size in TEXT_SIZES {
        font.populate_font_cache(&letters, size as u16);
        font.populate_font_cache(&letters, (size * density).ceil() as u16);
    }
}

/// The game's name and version, and the engine's, at the top left.
fn draw_title(palette: &Palette) {
    let title = format!("FIGHT {}", version_label());
    let w = chip_width_at(&title, 17.0);
    draw_chip(
        MARGIN,
        MARGIN,
        w,
        &title,
        palette.tool_accent,
        palette,
        17.0,
    );
    let engine = format!("engine {}", rp::version_label());
    draw_chip(
        MARGIN + w + 6.0,
        MARGIN,
        chip_width_at(&engine, 14.0),
        &engine,
        with_alpha(palette.hud_border, 0.0),
        palette,
        14.0,
    );
}

/// A tag over each fighter's head: who plays it.
fn draw_tags(game: &Game, view: rp::View, palette: &Palette) {
    for fighter in [LEFT, RIGHT] {
        let Some(top) = head_top(&game.world, fighter) else {
            continue;
        };
        let at = view.to_screen(top);
        let (text, accent) = match fighter {
            LEFT => ("YOU".to_string(), palette.tool_accent),
            _ if game.second_player_plays() => ("PLAYER 2".to_string(), palette.hud_text),
            _ => ("SPARRING PARTNER".to_string(), palette.hud_muted),
        };
        let size = 16.0;
        let measure = measure_text(&text, None, size as u16, 1.0);
        let (w, h) = (measure.width + 18.0, 24.0);
        let (x, y) = (at.x as f32 - w * 0.5, at.y as f32 - h - 10.0);
        draw_rectangle(x, y, w, h, palette.hud_back);
        draw_rectangle_lines(x, y, w, h, 1.5, accent);
        draw_text(&text, x + 9.0, y + h * 0.5 + 5.5, size, accent);
    }
}

/// The top of `fighter`'s head in the world: its skull's highest end, out
/// by the skull's radius.
fn head_top(world: &rp::World, fighter: usize) -> Option<rp::Vec2> {
    let skull = world.bones().iter().find(|bone| {
        bone.part == rp::BonePart::Skull && bone.body as usize == fighter && !bone.splinter
    })?;
    let top = if skull.a.y < skull.b.y {
        skull.a
    } else {
        skull.b
    };
    Some(rp::Vec2 {
        x: top.x,
        y: top.y - skull.radius,
    })
}

fn draw_button(
    game: &Game,
    layout: &Layout,
    button: Button,
    rect: Rect,
    held: bool,
    palette: &Palette,
) {
    // A fighter's button: what it does, an arrow toward or away from the
    // other fighter, and its keys.
    let (title, toward, keys, accent) = match button {
        Button::Lean(fighter) => (
            if layout.narrow { "LEAN" } else { "LEAN BACK" },
            fighter == RIGHT,
            if fighter == LEFT { "A or LEFT" } else { "L" },
            palette.hud_border,
        ),
        Button::Punch(fighter) => (
            "PUNCH",
            fighter == LEFT,
            if fighter == LEFT { "D or RIGHT" } else { "J" },
            palette.blood_fresh,
        ),
        _ => {
            let label = top_label(button, layout.narrow, game.muted, !game.running);
            draw_chip(
                rect.x,
                rect.y,
                rect.w,
                &label,
                palette.hud_border,
                palette,
                15.0,
            );
            if held {
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    with_alpha(palette.hud_text, 0.15),
                );
            }
            return;
        }
    };
    // Held down, a button lights up: a punch red, a lean pale.
    let fill = match (held, button) {
        (false, _) => palette.hud_back,
        (true, Button::Punch(_)) => with_alpha(accent, 0.5),
        (true, _) => with_alpha(palette.hud_text, 0.28),
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.5, accent);
    let size = if layout.narrow { 16.0 } else { 18.0 };
    let measure = measure_text(title, None, size as u16, 1.0);
    let arrow = size * 0.55;
    let gap = 7.0;
    let show_keys = !layout.touch;
    let middle = rect.y + rect.h * if show_keys { 0.42 } else { 0.5 };
    let width = measure.width + gap + arrow;
    let left = rect.x + (rect.w - width) * 0.5;
    // The arrow on the side it points to: right for a move toward the right.
    let (text_x, arrow_x) = if toward {
        (left, left + measure.width + gap)
    } else {
        (left + arrow + gap, left)
    };
    draw_text(title, text_x, middle + size * 0.32, size, palette.hud_text);
    draw_arrow(arrow_x, middle, arrow, toward, palette.hud_text);
    if show_keys {
        let small = 14.0;
        let measure = measure_text(keys, None, small as u16, 1.0);
        draw_text(
            keys,
            rect.x + (rect.w - measure.width) * 0.5,
            rect.y + rect.h * 0.8,
            small,
            palette.hud_muted,
        );
    }
}

/// A solid arrowhead `size` wide from `x`, about the height `y`, pointing
/// right or left.
fn draw_arrow(x: f32, y: f32, size: f32, right: bool, color: Color) {
    let (back, tip) = if right { (x, x + size) } else { (x + size, x) };
    draw_triangle(
        vec2(back, y - size * 0.6),
        vec2(back, y + size * 0.6),
        vec2(tip, y),
        color,
    );
}

/// The label of a button along the top: its key and what it does, or on a
/// narrow window, a short sign. The game's font has no arrows.
fn top_label(button: Button, narrow: bool, muted: bool, paused: bool) -> String {
    let (short, long) = match button {
        Button::Pause if paused => (">", "Space  go on"),
        Button::Pause => ("II", "Space  pause"),
        Button::Restart => ("R", "R  restart"),
        Button::Sound if muted => ("M off", "M  sound off"),
        Button::Sound => ("M", "M  sound"),
        Button::Sandbox => ("<", "< Sandbox"),
        Button::Lean(_) | Button::Punch(_) => ("", ""),
    };
    if narrow { short } else { long }.to_string()
}

/// How wide a chip is round `label` in 15 px text.
fn chip_width(label: &str) -> f32 {
    chip_width_at(label, 15.0)
}

fn chip_width_at(label: &str, size: f32) -> f32 {
    measure_text(label, None, size as u16, 1.0).width + 18.0
}

fn draw_chip(x: f32, y: f32, w: f32, label: &str, accent: Color, palette: &Palette, size: f32) {
    draw_rectangle(x, y, w, CHIP_HEIGHT, palette.hud_back);
    draw_rectangle_lines(x, y, w, CHIP_HEIGHT, 1.0, palette.hud_border);
    draw_rectangle(x, y, 3.0, CHIP_HEIGHT, accent);
    draw_text(
        label,
        x + 9.0,
        y + CHIP_HEIGHT * 0.5 + size * 0.32,
        size,
        palette.hud_text,
    );
}

/// The fight stands still, and says how to go on.
fn draw_paused(layout: &Layout, palette: &Palette) {
    let text = "PAUSED";
    let size = 34.0;
    let measure = measure_text(text, None, size as u16, 1.0);
    let (x, y) = (
        (layout.width - measure.width) * 0.5,
        layout.top + (layout.bottom - layout.top) * 0.35,
    );
    draw_rectangle(
        x - 18.0,
        y - size,
        measure.width + 36.0,
        size + 18.0,
        with_alpha(palette.hud_back, 0.9),
    );
    draw_text(text, x, y, size, palette.hud_text);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With room for the guards alone, the fighters are drawn where the
    /// sandbox would fit them, standing on the buttons, at every size.
    #[test]
    fn standing_fighters_are_fitted_as_the_sandbox_fits_them() {
        for (width, height, touch) in [
            (1280.0, 720.0, false),
            (800.0, 600.0, false),
            (390.0, 844.0, true),
            (844.0, 390.0, true),
        ] {
            let layout = Layout::without_buttons(width, height, touch);
            let frame = layout.fighters_frame(&Camera::default());
            let between =
                rp::body_frame_between(width as f64, layout.top as f64, layout.bottom as f64);
            let sandbox = rp::fighters_across(
                between,
                rp::FIGHTER_SPACING,
                width as f64,
                FIGHTER_SIDE_MARGIN,
            );
            assert!(
                (frame.origin.x - sandbox.origin.x).abs() < 1.0e-9
                    && (frame.height - sandbox.height).abs() < 1.0e-9,
                "{width}x{height}: drawn at {frame:?}, the sandbox's fit {sandbox:?}"
            );
            assert_eq!(frame.origin.y + frame.height, layout.bottom as f64);
        }
    }

    /// On a wide screen there is room for a lean beside the fighters, so the
    /// camera's room moves nothing; on a phone held upright, the fighters
    /// are drawn smaller to make it.
    #[test]
    fn room_for_a_lean_shrinks_the_fight_only_where_it_must() {
        let leaning = Camera {
            reach: [0.55, rp::FIGHTER_REACH],
        };
        let wide = Layout::without_buttons(1280.0, 720.0, false);
        assert_eq!(
            wide.fighters_frame(&leaning),
            wide.fighters_frame(&Camera::default())
        );
        let tall = Layout::without_buttons(390.0, 844.0, true);
        let (standing, leant) = (
            tall.fighters_frame(&Camera::default()),
            tall.fighters_frame(&leaning),
        );
        assert!(leant.height < standing.height);
        let view = rp::View::fitting(rp::world_frame(), leant);
        let [left, right] = rp::fighter_frames();
        let height = rp::world_frame().height;
        let from = view.to_screen(rp::Vec2 {
            x: left.origin.x - 0.55 * height,
            y: 0.0,
        });
        let to = view.to_screen(rp::Vec2 {
            x: right.origin.x + rp::FIGHTER_REACH * height,
            y: 0.0,
        });
        assert!(from.x >= FIGHTER_SIDE_MARGIN - 1.0e-9);
        assert!(to.x <= 390.0 - FIGHTER_SIDE_MARGIN + 1.0e-9);
    }
}
