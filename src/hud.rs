//! The screen around the fighters: the game's name and version and its few
//! buttons along the top, the opponent's level among them, the round and its
//! clock with each fighter's clean punches between them, a tag over each
//! fighter saying who plays it, and the left fighter's moves as buttons
//! along the bottom, sized for fingers on a touch screen: its steps, its
//! lean back, and its duck at the left, under a left thumb, and its jab, its
//! cross, and its block at the right, under a right thumb. A second player's
//! keys are written between them. Between rounds a card gives the score,
//! and once the fight is over a card says how it ended and who won.
//! The fighters are drawn by the engine (`rp::draw`), as large as fits
//! between the two rows with the room the camera leaves them (`camera.rs`).

use std::sync::atomic::{AtomicU32, Ordering};

use macroquad::prelude::*;
use realistic_physics as rp;
use rp::draw::{with_alpha, Palette};
use rp::scenarios::SCENARIO_WIDTH;

use crate::bout::{Outcome, Stage};
use crate::camera::Camera;
use crate::controls::{Act, Button, Hold};
use crate::game::{version_label, Game, LEFT, RIGHT};
use crate::opponent::Level;

/// The computer opponent's colour, on its tag and its count: a light red,
/// against the player's orange.
const OPPONENT: Color = Color::new(0.94, 0.45, 0.39, 0.96);

/// Room kept round the edges of the window and between the rows and the
/// fighters, in screen pixels.
const MARGIN: f32 = 10.0;
/// The height of the buttons along the top.
const CHIP_HEIGHT: f32 = 30.0;
/// The height of one row of the fighter's buttons, for a mouse and for
/// fingers; the buttons stand in two rows.
const ROW_HEIGHT: [f32; 2] = [50.0, 54.0];
/// The room between two of the fighter's buttons.
const BUTTON_GAP: f32 = 6.0;
/// The least room left beside the fighters at each side of the window.
const FIGHTER_SIDE_MARGIN: f64 = 12.0;
/// Windows narrower than this get the buttons' short labels.
const NARROW_WIDTH: f32 = 760.0;
/// Every size the HUD writes text at.
const TEXT_SIZES: [f32; 9] = [13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 20.0, 28.0, 34.0];

/// The two lines the fighters are drawn between in a window `height` tall:
/// under the top row of buttons, and over the fighter's buttons.
fn fighters_between(height: f32, touch: bool) -> (f32, f32) {
    let rows = 2.0 * ROW_HEIGHT[usize::from(touch)] + BUTTON_GAP;
    (
        MARGIN + CHIP_HEIGHT + MARGIN + 34.0,
        height - MARGIN - rows - MARGIN,
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
            (Button::Level, true),
        ];
        for (button, shown) in top {
            if !shown {
                continue;
            }
            // The level's chip as wide as its longest name, so it does not
            // move the others as it changes.
            let w = chip_width(&top_label(
                button,
                narrow,
                touch,
                false,
                false,
                Level::Normal,
            ));
            x -= w;
            buttons.push((button, Rect::new(x, MARGIN, w, CHIP_HEIGHT)));
            x -= 6.0;
        }
        let (top, bottom) = fighters_between(height, touch);
        buttons.extend(fighter_buttons(width, bottom, touch));
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

    /// The layout of a window without the buttons along its top, which need
    /// the window to measure their labels: for tests of where the fighters
    /// and their buttons stand.
    #[cfg(test)]
    pub fn without_buttons(width: f32, height: f32, touch: bool) -> Layout {
        let (top, bottom) = fighters_between(height, touch);
        Layout {
            width,
            height,
            top,
            bottom,
            buttons: fighter_buttons(width, bottom, touch),
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

    /// Where the scenarios' body frame is drawn: the fighters as large as
    /// fits between the top row and the fighter's buttons, with the room
    /// `camera` leaves them each side of the ring's middle, which is drawn
    /// at the window's middle, standing just above the buttons.
    pub fn fighters_frame(&self, camera: &Camera) -> rp::BodyFrame {
        let between =
            rp::body_frame_between(self.width as f64, self.top as f64, self.bottom as f64);
        let world = rp::world_frame();
        let width = self.width as f64;
        let room = (width - 2.0 * FIGHTER_SIDE_MARGIN).max(1.0);
        let across = 2.0 * camera.half * world.height;
        let scale = (between.height / world.height).min(room / across);
        rp::BodyFrame {
            origin: rp::Vec2 {
                x: width * 0.5 + (world.origin.x - SCENARIO_WIDTH * 0.5) * scale,
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

/// The left fighter's buttons in a window `width` wide, under the line
/// `bottom` the fighters stand on: at the left its lean back over its step
/// back, its duck, and its step in, laid out as W over A S D; at the right
/// its jab, its cross, and its block, each as tall as both rows.
fn fighter_buttons(width: f32, bottom: f32, touch: bool) -> Vec<(Button, Rect)> {
    let tall = ROW_HEIGHT[usize::from(touch)];
    let each = ((width * 0.5 - MARGIN - 2.0 * BUTTON_GAP - 8.0) / 3.0).clamp(48.0, 118.0);
    let (upper, lower) = (bottom + MARGIN, bottom + MARGIN + tall + BUTTON_GAP);
    let from_left = |column: f32| MARGIN + column * (each + BUTTON_GAP);
    let from_right = |column: f32| width - MARGIN - each - column * (each + BUTTON_GAP);
    let both_rows = 2.0 * tall + BUTTON_GAP;
    vec![
        (
            Button::Hold(LEFT, Hold::Lean),
            Rect::new(from_left(1.0), upper, each, tall),
        ),
        (
            Button::Act(LEFT, Act::StepOut),
            Rect::new(from_left(0.0), lower, each, tall),
        ),
        (
            Button::Hold(LEFT, Hold::Duck),
            Rect::new(from_left(1.0), lower, each, tall),
        ),
        (
            Button::Act(LEFT, Act::StepIn),
            Rect::new(from_left(2.0), lower, each, tall),
        ),
        (
            Button::Act(LEFT, Act::Jab),
            Rect::new(from_right(2.0), upper, each, both_rows),
        ),
        (
            Button::Act(LEFT, Act::Cross),
            Rect::new(from_right(1.0), upper, each, both_rows),
        ),
        (
            Button::Hold(LEFT, Hold::Block),
            Rect::new(from_right(0.0), upper, each, both_rows),
        ),
    ]
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
    // A card between rounds or at the end stands where the tags are.
    let carded = matches!(game.bout.stage(), Stage::Break(_)) || game.outcome().is_some();
    if !carded {
        draw_tags(game, view, &palette);
    }
    draw_title(layout, &palette);
    draw_round(game, layout, &palette);
    for &(button, rect) in &layout.buttons {
        draw_button(game, layout, button, rect, held.contains(&button), &palette);
    }
    if !layout.touch {
        draw_second_players_keys(layout, &palette);
    }
    if let Stage::Break(round) = game.bout.stage() {
        draw_break(game, layout, round, &palette);
    }
    if let Some((outcome, since)) = game.outcome() {
        draw_ending(game, layout, outcome, since, &palette);
    }
    if !game.running {
        draw_paused(layout, &palette);
    }
}

/// How long the card that ends a fight takes to show, in seconds.
const ENDING_FADE_SECONDS: f64 = 0.25;

/// Who plays the right fighter, as the screen names it.
fn right_fighter(game: &Game) -> &'static str {
    if game.second_player_plays() {
        "PLAYER 2"
    } else {
        "THE OPPONENT"
    }
}

/// The card that ends a fight: KNOCKOUT, or DOWN for a fighter that fell,
/// with who won and in which round, or after the last round the DECISION on
/// clean punches; and how to fight again.
fn draw_ending(game: &Game, layout: &Layout, outcome: Outcome, since: f64, palette: &Palette) {
    let shown = (since / ENDING_FADE_SECONDS).min(1.0) as f32;
    let [yours, theirs] = game.bout.clean();
    let winner = |fighter: Option<usize>| match fighter {
        Some(LEFT) => "YOU WIN".to_string(),
        Some(_) => format!("{} WINS", right_fighter(game)),
        None => "A DRAW".to_string(),
    };
    let (title, result, detail) = match outcome {
        Outcome::Down {
            down,
            knocked_out,
            round,
        } => (
            if knocked_out { "KNOCKOUT" } else { "DOWN" },
            winner(Some(1 - down)),
            format!("in round {round}"),
        ),
        Outcome::Decision { winner: fighter } => {
            let (most, least) = (yours.max(theirs), yours.min(theirs));
            let detail = if fighter.is_some() {
                format!("{most} clean punches to {least}")
            } else {
                format!("{most} clean punches each")
            };
            ("DECISION", winner(fighter), detail)
        }
    };
    // Named as the restart button along the top says it.
    let again = if !layout.touch {
        "Press R to fight again"
    } else if layout.narrow {
        "Tap R to fight again"
    } else {
        "Tap restart to fight again"
    };
    let lines = [
        (title, 34.0, palette.tool_accent),
        (result.as_str(), 18.0, palette.hud_text),
        (detail.as_str(), 16.0, palette.hud_text),
        (again, 15.0, palette.hud_muted),
    ];
    draw_card(game, layout, &lines, shown, 0.92, palette);
}

/// The card between two rounds: the round that ended, each fighter's clean
/// punches in it, and when the next begins.
fn draw_break(game: &Game, layout: &Layout, round: u32, palette: &Palette) {
    let [yours, theirs] = game.bout.clean_in(round);
    let opponent = if game.second_player_plays() {
        "PLAYER 2"
    } else {
        "OPPONENT"
    };
    let title = format!("END OF ROUND {round}");
    let score = format!("YOU {yours} - {theirs} {opponent}, clean punches");
    let next = format!(
        "round {} in {}",
        round + 1,
        game.bout.seconds_left().unwrap_or(0.0).ceil() as u32
    );
    let lines = [
        (title.as_str(), 20.0, palette.tool_accent),
        (score.as_str(), 16.0, palette.hud_text),
        (next.as_str(), 15.0, palette.hud_muted),
    ];
    draw_card(game, layout, &lines, 1.0, 0.85, palette);
}

/// A card of `lines`, each a text, its size, and its colour, shown as far
/// as `shown` from 0 to 1, its back `back` opaque. It stands in the room
/// above the fighters where there is room for it, as on a phone held
/// upright, and otherwise over the top of the fight, clear of a fighter
/// lying on the floor.
fn draw_card(
    game: &Game,
    layout: &Layout,
    lines: &[(&str, f32, Color)],
    shown: f32,
    back: f32,
    palette: &Palette,
) {
    // A wider gap under the title than between the lines under it.
    let gap = |line: usize| match line {
        0 => 0.0,
        1 => 14.0,
        _ => 9.0,
    };
    let widest = lines
        .iter()
        .map(|&(text, size, _)| measure_text(text, None, size as u16, 1.0).width)
        .fold(0.0, f32::max);
    let (pad_x, pad_y) = (26.0, 12.0);
    let tall = lines
        .iter()
        .enumerate()
        .map(|(line, &(_, size, _))| size + gap(line))
        .sum::<f32>()
        + 2.0 * pad_y;
    let wide = widest + 2.0 * pad_x;
    let above = layout.fighters_frame(&game.camera).origin.y as f32 - layout.top;
    // Under the top row and the round's chip, where the tags it hides were.
    let chip = round_chip(layout);
    let under_the_top = chip.y + chip.h + 6.0;
    let y = if above >= tall + 2.0 * MARGIN {
        layout.top + (above - tall) * 0.5
    } else {
        under_the_top
    };
    let x = (layout.width - wide) * 0.5;
    draw_rectangle(x, y, wide, tall, with_alpha(palette.hud_back, back * shown));
    draw_rectangle_lines(
        x,
        y,
        wide,
        tall,
        2.0,
        with_alpha(palette.tool_accent, shown),
    );
    let mut baseline = y + pad_y;
    for (line, &(text, size, colour)) in lines.iter().enumerate() {
        baseline += gap(line) + size;
        let measure = measure_text(text, None, size as u16, 1.0);
        draw_text(
            text,
            (layout.width - measure.width) * 0.5,
            baseline - size * 0.2,
            size,
            with_alpha(colour, shown * colour.a),
        );
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

/// The game's name and version, and the engine's beside it where there is
/// room before the buttons along the top.
fn draw_title(layout: &Layout, palette: &Palette) {
    let (title, engine) = title_texts();
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
    if title_end(layout) > MARGIN + w {
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
}

/// The game's title chip's text and the engine's.
fn title_texts() -> (String, String) {
    (
        format!("FIGHT {}", version_label()),
        format!("engine {}", rp::version_label()),
    )
}

/// Where the title chips end along the top: after the engine's where it
/// fits before the buttons, else after the game's.
fn title_end(layout: &Layout) -> f32 {
    let (title, engine) = title_texts();
    let after_title = MARGIN + chip_width_at(&title, 17.0);
    let after_engine = after_title + 6.0 + chip_width_at(&engine, 14.0);
    if after_engine + 6.0 <= top_buttons_start(layout) {
        after_engine
    } else {
        after_title
    }
}

/// Where the buttons along the top begin, from the left.
fn top_buttons_start(layout: &Layout) -> f32 {
    layout
        .buttons
        .iter()
        .filter(|(_, rect)| rect.y < MARGIN + CHIP_HEIGHT)
        .map(|(_, rect)| rect.x)
        .fold(layout.width, f32::min)
}

/// The sizes of the round chip's clock and of the counts at its sides.
const ROUND_TEXT: f32 = 16.0;
const COUNT_TEXT: f32 = 20.0;

/// Where the round's chip stands: in the top row between the title and the
/// buttons where it fits, else in a row of its own under it, as on a phone
/// held upright. It is as wide as its longest text needs, so that it stays
/// put as the clock and the counts change.
fn round_chip(layout: &Layout) -> Rect {
    let measure = |text: &str, size: f32| measure_text(text, None, size as u16, 1.0).width;
    let w = measure("ROUND 3/3  0:59", ROUND_TEXT) + 2.0 * measure("88", COUNT_TEXT) + 48.0;
    let (from, to) = (title_end(layout) + 8.0, top_buttons_start(layout) - 8.0);
    if to - from >= w {
        let x = ((layout.width - w) * 0.5).clamp(from, to - w);
        Rect::new(x, MARGIN, w, CHIP_HEIGHT)
    } else {
        let x = (layout.width - w) * 0.5;
        Rect::new(x, MARGIN + CHIP_HEIGHT + 6.0, w, CHIP_HEIGHT)
    }
}

/// The round under way and its clock, or the rest's, the player's clean
/// punches at its left and the other fighter's at its right. Once the fight
/// is over, the last round and the counts stay.
fn draw_round(game: &Game, layout: &Layout, palette: &Palette) {
    let bout = &game.bout;
    let rounds = bout.rules.rounds;
    let middle = match (bout.stage(), bout.seconds_left()) {
        (Stage::Round(round), Some(left)) => {
            let seconds = left.ceil() as u32;
            format!(
                "ROUND {round}/{rounds}  {}:{:02}",
                seconds / 60,
                seconds % 60
            )
        }
        (Stage::Break(_), Some(left)) => format!("REST  0:{:02}", left.ceil() as u32),
        _ => format!("ROUND {}/{rounds}", bout.round()),
    };
    let [yours, theirs] = bout.clean();
    let (yours, theirs) = (yours.to_string(), theirs.to_string());
    let measure = |text: &str, size: f32| measure_text(text, None, size as u16, 1.0).width;
    let chip = round_chip(layout);
    let pad = 12.0;
    draw_rectangle(chip.x, chip.y, chip.w, chip.h, palette.hud_back);
    draw_rectangle_lines(chip.x, chip.y, chip.w, chip.h, 1.0, palette.hud_border);
    let baseline = |size: f32| chip.y + chip.h * 0.5 + size * 0.32;
    draw_text(
        &yours,
        chip.x + pad,
        baseline(COUNT_TEXT),
        COUNT_TEXT,
        palette.tool_accent,
    );
    draw_text(
        &middle,
        chip.x + (chip.w - measure(&middle, ROUND_TEXT)) * 0.5,
        baseline(ROUND_TEXT),
        ROUND_TEXT,
        palette.hud_text,
    );
    draw_text(
        &theirs,
        chip.x + chip.w - pad - measure(&theirs, COUNT_TEXT),
        baseline(COUNT_TEXT),
        COUNT_TEXT,
        OPPONENT,
    );
}

/// The height of the tag over each fighter's head, and the least room
/// between two tags.
const TAG_HEIGHT: f32 = 24.0;
const TAG_GAP: f32 = 6.0;

/// A tag over a fighter's head, where it is drawn.
struct Tag {
    text: &'static str,
    accent: Color,
    x: f32,
    y: f32,
    w: f32,
}

/// A tag over each fighter's head: who plays it. Over two fighters
/// standing close, as on a phone where they are drawn small, the two tags
/// would overlap: each moves aside by half as much as they would.
fn draw_tags(game: &Game, view: rp::View, palette: &Palette) {
    let size = 16.0;
    let mut tags: Vec<Tag> = Vec::new();
    for fighter in [LEFT, RIGHT] {
        let Some(top) = head_top(&game.world, fighter) else {
            continue;
        };
        let at = view.to_screen(top);
        let (text, accent) = match fighter {
            LEFT => ("YOU", palette.tool_accent),
            _ if game.second_player_plays() => ("PLAYER 2", palette.hud_text),
            _ => ("OPPONENT", OPPONENT),
        };
        let w = measure_text(text, None, size as u16, 1.0).width + 18.0;
        tags.push(Tag {
            text,
            accent,
            x: at.x as f32 - w * 0.5,
            y: at.y as f32 - TAG_HEIGHT - 10.0,
            w,
        });
    }
    if let [a, b] = &mut tags[..] {
        let (first, second) = if a.x <= b.x { (a, b) } else { (b, a) };
        let overlap = first.x + first.w + TAG_GAP - second.x;
        if overlap > 0.0 && (first.y - second.y).abs() < TAG_HEIGHT + TAG_GAP {
            first.x -= overlap * 0.5;
            second.x += overlap * 0.5;
        }
    }
    for tag in &tags {
        draw_rectangle(tag.x, tag.y, tag.w, TAG_HEIGHT, palette.hud_back);
        draw_rectangle_lines(tag.x, tag.y, tag.w, TAG_HEIGHT, 1.5, tag.accent);
        draw_text(
            tag.text,
            tag.x + 9.0,
            tag.y + TAG_HEIGHT * 0.5 + 5.5,
            size,
            tag.accent,
        );
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

/// What a fighter's button says: its name, shorter on a narrow window, the
/// left fighter's key for it, and, for a step, whether it goes toward the
/// other fighter.
fn fighter_label(button: Button, narrow: bool) -> (&'static str, &'static str, Option<bool>) {
    match button {
        Button::Act(_, Act::Jab) => ("JAB", "J", None),
        Button::Act(_, Act::Cross) => ("CROSS", "K", None),
        Button::Act(_, Act::StepIn) => (if narrow { "IN" } else { "STEP IN" }, "D", Some(true)),
        Button::Act(_, Act::StepOut) => {
            (if narrow { "BACK" } else { "STEP BACK" }, "A", Some(false))
        }
        Button::Hold(_, Hold::Block) => ("BLOCK", "L", None),
        Button::Hold(_, Hold::Duck) => ("DUCK", "S", None),
        Button::Hold(_, Hold::Lean) => (if narrow { "LEAN" } else { "LEAN BACK" }, "W", None),
        _ => ("", "", None),
    }
}

fn draw_button(
    game: &Game,
    layout: &Layout,
    button: Button,
    rect: Rect,
    held: bool,
    palette: &Palette,
) {
    let (fighter, accent) = match button {
        Button::Act(fighter, Act::Jab | Act::Cross) => (fighter, palette.blood_fresh),
        Button::Hold(fighter, Hold::Block) => (fighter, palette.tool_accent),
        Button::Act(fighter, _) | Button::Hold(fighter, _) => (fighter, palette.hud_border),
        _ => {
            let label = top_label(
                button,
                layout.narrow,
                layout.touch,
                game.muted,
                !game.running,
                game.level(),
            );
            // Once the fight is over, restart is what to press next.
            let accent = if button == Button::Restart && game.outcome().is_some() {
                palette.tool_accent
            } else {
                palette.hud_border
            };
            draw_chip(rect.x, rect.y, rect.w, &label, accent, palette, 15.0);
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
    let (title, key, toward) = fighter_label(button, layout.narrow);
    // Held down, a button lights up in its own colour.
    let fill = if held {
        with_alpha(accent, 0.5)
    } else {
        palette.hud_back
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.5, accent);
    let size = if layout.narrow { 14.0 } else { 16.0 };
    let measure = measure_text(title, None, size as u16, 1.0);
    let arrow = if toward.is_some() { size * 0.55 } else { 0.0 };
    let gap = if toward.is_some() { 6.0 } else { 0.0 };
    let show_keys = !layout.touch;
    let middle = rect.y + (rect.h - if show_keys { 24.0 } else { 0.0 }) * 0.5;
    let width = measure.width + gap + arrow;
    let left = rect.x + (rect.w - width) * 0.5;
    // A step's arrow points the way the fighter goes, on that side of its
    // name: toward the other fighter is to the right for the left fighter.
    let right = toward.is_some_and(|toward| toward == (fighter == LEFT));
    let (text_x, arrow_x) = if right || toward.is_none() {
        (left, left + measure.width + gap)
    } else {
        (left + arrow + gap, left)
    };
    draw_text(title, text_x, middle + size * 0.32, size, palette.hud_text);
    if toward.is_some() {
        draw_arrow(arrow_x, middle, arrow, right, palette.hud_text);
    }
    if show_keys {
        // The key that makes the move, on a key cap of its own, so a
        // player at a keyboard reads the keys before the names.
        let small = 16.0;
        let measure = measure_text(key, None, small as u16, 1.0);
        let (cap_w, cap_h) = (measure.width + 14.0, 18.0);
        let (cap_x, cap_y) = (
            rect.x + (rect.w - cap_w) * 0.5,
            rect.y + rect.h - cap_h - 5.0,
        );
        draw_rectangle(
            cap_x,
            cap_y,
            cap_w,
            cap_h,
            with_alpha(palette.hud_text, 0.12),
        );
        draw_rectangle_lines(cap_x, cap_y, cap_w, cap_h, 1.0, palette.hud_muted);
        draw_text(
            key,
            cap_x + 7.0,
            cap_y + cap_h * 0.5 + small * 0.30,
            small,
            palette.hud_text,
        );
    }
}

/// The second player's keys, written between the left fighter's two groups
/// of buttons where there is room: a friend takes the right fighter by
/// pressing them.
fn draw_second_players_keys(layout: &Layout, palette: &Palette) {
    let rect_of = |wanted: Button| {
        layout
            .buttons
            .iter()
            .find(|(button, _)| *button == wanted)
            .map(|&(_, rect)| rect)
    };
    let (Some(step_in), Some(jab)) = (
        rect_of(Button::Act(LEFT, Act::StepIn)),
        rect_of(Button::Act(LEFT, Act::Jab)),
    ) else {
        return;
    };
    let (from, to) = (step_in.x + step_in.w, jab.x);
    let lines = [
        "PLAYER 2 PLAYS THE RIGHT FIGHTER",
        "ARROWS  step, lean back, duck",
        ",  jab     .  cross     /  block",
    ];
    let size = 13.0;
    let widest = lines
        .iter()
        .map(|line| measure_text(line, None, size as u16, 1.0).width)
        .fold(0.0, f32::max);
    if to - from < widest + 24.0 {
        return;
    }
    let top = jab.y + (jab.h - 3.0 * (size + 5.0)) * 0.5;
    for (row, line) in lines.iter().enumerate() {
        let measure = measure_text(line, None, size as u16, 1.0);
        draw_text(
            line,
            (from + to - measure.width) * 0.5,
            top + (row as f32 + 0.8) * (size + 5.0),
            size,
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

/// The label of a button along the top: its key and what it does, what it
/// does alone on a touch screen, which has no keys, or on a narrow window, a
/// short sign. The game's font has no arrows. The level's button says the
/// opponent's level, and a press goes on to the next.
fn top_label(
    button: Button,
    narrow: bool,
    touch: bool,
    muted: bool,
    paused: bool,
    level: Level,
) -> String {
    if button == Button::Level {
        return if narrow {
            level.name().to_string()
        } else {
            format!("level  {}", level.name())
        };
    }
    let (short, keyed, plain) = match button {
        Button::Pause if paused => (">", "Space  go on", "go on"),
        Button::Pause => ("II", "Space  pause", "pause"),
        Button::Restart => ("R", "R  restart", "restart"),
        Button::Sound if muted => ("M off", "M  sound off", "sound off"),
        Button::Sound => ("M", "M  sound", "sound"),
        Button::Sandbox => ("<", "< Sandbox", "< Sandbox"),
        Button::Level | Button::Act(..) | Button::Hold(..) => ("", "", ""),
    };
    if narrow {
        short
    } else if touch {
        plain
    } else {
        keyed
    }
    .to_string()
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
    use crate::camera::{LEAST_HALF, MOST_HALF};

    const WINDOWS: [(f32, f32, bool); 4] = [
        (1280.0, 720.0, false),
        (800.0, 600.0, false),
        (390.0, 844.0, true),
        (844.0, 390.0, true),
    ];

    /// The left fighter has a button for each of its seven moves, all on
    /// the screen under the fighters, none over another, at every size.
    #[test]
    fn every_move_has_a_button_of_its_own() {
        for (width, height, touch) in WINDOWS {
            let layout = Layout::without_buttons(width, height, touch);
            let mut wanted: Vec<Button> = Act::ALL
                .into_iter()
                .map(|act| Button::Act(LEFT, act))
                .chain(Hold::ALL.into_iter().map(|hold| Button::Hold(LEFT, hold)))
                .collect();
            for (index, &(button, rect)) in layout.buttons.iter().enumerate() {
                wanted.retain(|&other| other != button);
                assert!(
                    rect.x >= 0.0
                        && rect.right() <= width
                        && rect.y >= layout.bottom
                        && rect.bottom() <= height,
                    "{width}x{height}: {button:?} at {rect:?} is off its place"
                );
                for &(other, other_rect) in &layout.buttons[index + 1..] {
                    assert!(
                        !rect.overlaps(&other_rect),
                        "{width}x{height}: {button:?} lies over {other:?}"
                    );
                }
                assert_eq!(layout.button_at(rect.center()), Some(button));
            }
            assert!(
                wanted.is_empty(),
                "{width}x{height}: no button for {wanted:?}"
            );
        }
    }

    /// The fighters stand on the line above their buttons, the ring's
    /// middle at the window's middle, with all the room the camera asks
    /// for on the screen, at every size.
    #[test]
    fn the_ring_is_drawn_in_the_middle_with_the_cameras_room() {
        let world = rp::world_frame();
        for (width, height, touch) in WINDOWS {
            let layout = Layout::without_buttons(width, height, touch);
            for half in [LEAST_HALF, MOST_HALF] {
                let camera = Camera { half };
                let frame = layout.fighters_frame(&camera);
                assert!((frame.origin.y + frame.height - layout.bottom as f64).abs() < 1.0e-9);
                assert!(frame.height <= (layout.bottom - layout.top) as f64 + 1.0e-9);
                let view = layout.view(&camera);
                let at = |x: f64| view.to_screen(rp::Vec2 { x, y: 0.0 }).x;
                let middle = SCENARIO_WIDTH * 0.5;
                assert!((at(middle) - width as f64 * 0.5).abs() < 1.0e-6);
                assert!(at(middle - half * world.height) >= FIGHTER_SIDE_MARGIN - 1.0e-6);
                assert!(
                    at(middle + half * world.height) <= width as f64 - FIGHTER_SIDE_MARGIN + 1.0e-6
                );
            }
        }
    }

    /// On a wide screen the fighters are as tall as fits whatever room the
    /// camera asks for; on a phone held upright they are drawn smaller to
    /// make it.
    #[test]
    fn room_across_shrinks_the_fight_only_where_it_must() {
        let (near, far) = (Camera { half: LEAST_HALF }, Camera { half: MOST_HALF });
        let wide = Layout::without_buttons(1280.0, 720.0, false);
        assert_eq!(wide.fighters_frame(&far), wide.fighters_frame(&near));
        let tall = Layout::without_buttons(390.0, 844.0, true);
        assert!(tall.fighters_frame(&far).height < tall.fighters_frame(&near).height);
    }
}
