//! A game controller's buttons, drawn as their makers print them: a
//! PlayStation controller's square, triangle, circle, and cross in their
//! colours, an Xbox controller's lettered buttons, any other controller's
//! face buttons by where they sit (four dots, the one meant lit), the D-pad
//! with the way that makes a move lit, and the shoulder and middle buttons
//! by their printed names. The game's font has none of these signs.

use macroquad::prelude::*;
use realistic_physics::draw::Palette;

use crate::pads::PadKind;

/// A face button by where it sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Bottom,
    Right,
    Left,
    Top,
}

/// A way the D-pad points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Left,
    Right,
    Up,
    Down,
}

/// A controller's button as the screen draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Face(PadKind, Face),
    DPad(Way),
    /// A button drawn as its printed name: R1, RB, OPTIONS.
    Named(&'static str),
}

/// The height the glyphs are drawn at on the fighter's buttons.
pub const SIZE: f32 = 20.0;
/// The text sizes of a named button and of an Xbox button's letter: sizes
/// the HUD already writes at, whose letters it puts in the font's atlas
/// before each frame (`hud::cache_letters`).
const NAME_TEXT: f32 = 13.0;
const LETTER_TEXT: f32 = 16.0;

/// PlayStation's colours for its four signs, and Xbox's for its letters'
/// buttons.
const SQUARE: Color = Color::new(0.90, 0.59, 0.82, 1.0);
const TRIANGLE: Color = Color::new(0.25, 0.79, 0.64, 1.0);
const CIRCLE: Color = Color::new(1.0, 0.44, 0.44, 1.0);
const CROSS: Color = Color::new(0.53, 0.72, 1.0, 1.0);
const XBOX_A: Color = Color::new(0.36, 0.76, 0.12, 1.0);
const XBOX_B: Color = Color::new(0.89, 0.26, 0.23, 1.0);
const XBOX_X: Color = Color::new(0.23, 0.51, 0.88, 1.0);
const XBOX_Y: Color = Color::new(0.94, 0.77, 0.10, 1.0);
/// The dark ink of a letter on the yellow button, and the face of a button
/// with a sign on it.
const DARK: Color = Color::new(0.11, 0.11, 0.11, 1.0);
const BUTTON_FACE: Color = Color::new(0.15, 0.145, 0.165, 1.0);

impl Glyph {
    /// How wide it is drawn `size` tall.
    pub fn width(self, size: f32) -> f32 {
        match self {
            Glyph::Named(name) => {
                measure_text(name, None, NAME_TEXT as u16, 1.0).width + size * 0.6
            }
            Glyph::Face(..) | Glyph::DPad(_) => size,
        }
    }

    /// Draws it `size` tall with its middle at (`x`, `y`).
    pub fn draw(self, x: f32, y: f32, size: f32, palette: &Palette) {
        let r = size * 0.5;
        match self {
            Glyph::Face(PadKind::PlayStation, face) => {
                draw_circle(x, y, r, BUTTON_FACE);
                draw_circle_lines(x, y, r, 1.0, palette.hud_muted);
                let (s, line) = (size * 0.22, (size * 0.09).max(1.5));
                match face {
                    Face::Left => {
                        draw_rectangle_lines(x - s, y - s, 2.0 * s, 2.0 * s, line, SQUARE)
                    }
                    Face::Top => draw_triangle_lines(
                        vec2(x, y - s * 1.15),
                        vec2(x + s * 1.2, y + s * 0.85),
                        vec2(x - s * 1.2, y + s * 0.85),
                        line,
                        TRIANGLE,
                    ),
                    Face::Right => draw_circle_lines(x, y, s * 1.1, line, CIRCLE),
                    Face::Bottom => {
                        draw_line(x - s, y - s, x + s, y + s, line, CROSS);
                        draw_line(x + s, y - s, x - s, y + s, line, CROSS);
                    }
                }
            }
            Glyph::Face(PadKind::Xbox, face) => {
                let (letter, fill, ink) = match face {
                    Face::Bottom => ("A", XBOX_A, WHITE),
                    Face::Right => ("B", XBOX_B, WHITE),
                    Face::Left => ("X", XBOX_X, WHITE),
                    Face::Top => ("Y", XBOX_Y, DARK),
                };
                draw_circle(x, y, r, fill);
                let measure = measure_text(letter, None, LETTER_TEXT as u16, 1.0);
                // Twice, a pixel apart: the font has no bold, and its thin
                // letters were hard to read on the coloured button.
                for nudge in [-0.5, 0.5] {
                    draw_text(
                        letter,
                        x - measure.width * 0.5 + nudge,
                        y + LETTER_TEXT * 0.32,
                        LETTER_TEXT,
                        ink,
                    );
                }
            }
            Glyph::Face(PadKind::Other, face) => {
                let (spread, dot) = (size * 0.33, size * 0.15);
                for (at, dx, dy) in [
                    (Face::Bottom, 0.0, 1.0),
                    (Face::Right, 1.0, 0.0),
                    (Face::Left, -1.0, 0.0),
                    (Face::Top, 0.0, -1.0),
                ] {
                    let (cx, cy) = (x + dx * spread, y + dy * spread);
                    if at == face {
                        draw_circle(cx, cy, dot * 1.15, palette.tool_accent);
                    } else {
                        draw_circle_lines(cx, cy, dot, 1.2, palette.hud_muted);
                    }
                }
            }
            Glyph::DPad(way) => {
                let (arm, half) = (size * 0.32, size * 0.17);
                let unlit = with_alpha_of(palette.hud_muted, 0.55);
                draw_rectangle(x - half, y - half, 2.0 * half, 2.0 * half, unlit);
                for (at, dx, dy) in [
                    (Way::Left, -1.0, 0.0),
                    (Way::Right, 1.0, 0.0),
                    (Way::Up, 0.0, -1.0),
                    (Way::Down, 0.0, 1.0),
                ] {
                    let colour = if at == way {
                        palette.tool_accent
                    } else {
                        unlit
                    };
                    // Each arm reaches from the middle square out by `arm`.
                    let (w, h) = if dx != 0.0 {
                        (arm, 2.0 * half)
                    } else {
                        (2.0 * half, arm)
                    };
                    let ax = x + dx * (half + arm * 0.5) - w * 0.5;
                    let ay = y + dy * (half + arm * 0.5) - h * 0.5;
                    draw_rectangle(ax, ay, w, h, colour);
                }
            }
            Glyph::Named(name) => {
                let w = self.width(size);
                let h = size * 0.92;
                draw_rectangle(x - w * 0.5, y - h * 0.5, w, h, BUTTON_FACE);
                draw_rectangle_lines(x - w * 0.5, y - h * 0.5, w, h, 1.0, palette.hud_muted);
                let measure = measure_text(name, None, NAME_TEXT as u16, 1.0);
                draw_text(
                    name,
                    x - measure.width * 0.5,
                    y + NAME_TEXT * 0.32,
                    NAME_TEXT,
                    palette.hud_text,
                );
            }
        }
    }
}

fn with_alpha_of(colour: Color, alpha: f32) -> Color {
    Color { a: alpha, ..colour }
}
