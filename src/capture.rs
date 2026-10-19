//! Screenshot mode, so the game can be filmed without anyone at the keys:
//! `realistic_physics_fight --capture OUT.png [options]` opens the window,
//! plays a script of the players' moves one simulation step a frame, saves
//! the screen as a PNG, and exits. Native only.
//!
//! ```text
//! --size WXH        window size (default 1280x720)
//! --script SPEC     the players' moves, steps separated by `:` (below)
//! --frames N        steps to play (default: the script's own, plus 60)
//! --every N         also save the screen every N steps, as OUT-NNNN.png
//! --seed S          what the sparring partner's waits follow from (default 1)
//! --touch           draw the buttons as on a touch screen
//! --sandbox         show the Sandbox button, as the browser version does
//! ```
//!
//! Script steps: `wait=N` lets N steps pass; `punch1` and `punch2` press the
//! left or right fighter's punch; `lean1=on`, `lean1=off`, `lean2=on`, and
//! `lean2=off` hold or let go of its lean back; `player2` has a second player
//! take the right fighter, who then moves only as the script says; `hold=NAME`
//! and `release` draw a button held down (`punch1`, `lean1`, `punch2`,
//! `lean2`), as a finger on it would, for pictures of the buttons in use. For
//! example `wait=30:lean1=on:hold=lean1:wait=40:lean1=off:release:punch1:wait=60`.

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use macroquad::prelude::*;

use crate::controls::Button;
use crate::game::{Game, LEFT, RIGHT};
use crate::hud::{self, Layout};

/// A capture asked for on the command line.
pub struct Request {
    path: PathBuf,
    script: Vec<Step>,
    frames: Option<usize>,
    every: Option<usize>,
    seed: u64,
    touch: bool,
    sandbox: bool,
}

/// One step of a capture's script.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Wait(usize),
    Punch(usize),
    Lean(usize, bool),
    SecondPlayer,
    Hold(Button),
    Release,
}

/// The window size `--size` asks for, if any.
pub fn window_size() -> Option<(i32, i32)> {
    let args: Vec<String> = env::args().collect();
    if !args.iter().any(|arg| arg == "--capture") {
        return None;
    }
    let size = value(&args, "--size")?;
    let (w, h) = size.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// The capture the command line asks for, if it asks for one.
pub fn request() -> Option<Request> {
    let args: Vec<String> = env::args().collect();
    let path = PathBuf::from(value(&args, "--capture")?);
    let script = match value(&args, "--script") {
        Some(spec) => parse(&spec).unwrap_or_else(|error| fail(&error)),
        None => Vec::new(),
    };
    let number = |name: &str| {
        value(&args, name).map(|text| {
            text.parse::<usize>()
                .unwrap_or_else(|_| fail(&format!("{name} wants a number, not `{text}`")))
        })
    };
    Some(Request {
        path,
        script,
        frames: number("--frames"),
        every: number("--every"),
        seed: number("--seed").unwrap_or(1) as u64,
        touch: args.iter().any(|arg| arg == "--touch"),
        sandbox: args.iter().any(|arg| arg == "--sandbox"),
    })
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

fn fail(message: &str) -> ! {
    eprintln!("capture: {message}");
    process::exit(2);
}

fn parse(spec: &str) -> Result<Vec<Step>, String> {
    let fighter_button = |name: &str| match name {
        "punch1" => Ok(Button::Punch(LEFT)),
        "lean1" => Ok(Button::Lean(LEFT)),
        "punch2" => Ok(Button::Punch(RIGHT)),
        "lean2" => Ok(Button::Lean(RIGHT)),
        other => Err(format!("no button `{other}`")),
    };
    spec.split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part {
            "punch1" => Ok(Step::Punch(LEFT)),
            "punch2" => Ok(Step::Punch(RIGHT)),
            "lean1=on" => Ok(Step::Lean(LEFT, true)),
            "lean1=off" => Ok(Step::Lean(LEFT, false)),
            "lean2=on" => Ok(Step::Lean(RIGHT, true)),
            "lean2=off" => Ok(Step::Lean(RIGHT, false)),
            "player2" => Ok(Step::SecondPlayer),
            "release" => Ok(Step::Release),
            _ => {
                if let Some(frames) = part.strip_prefix("wait=") {
                    frames
                        .parse()
                        .map(Step::Wait)
                        .map_err(|_| format!("`{part}` wants a number of steps"))
                } else if let Some(name) = part.strip_prefix("hold=") {
                    fighter_button(name).map(Step::Hold)
                } else {
                    Err(format!("unknown script step `{part}`"))
                }
            }
        })
        .collect()
}

/// Plays the script and saves the screen.
pub async fn run(request: Request) {
    warm_glyphs().await;
    let mut game = Game::new(request.seed);
    let length: usize = request
        .script
        .iter()
        .map(|step| match step {
            Step::Wait(frames) => *frames,
            _ => 0,
        })
        .sum();
    let frames = request.frames.unwrap_or(length + 60);
    let mut script = request.script.iter().copied().peekable();
    let mut waiting = 0;
    let mut held: Vec<Button> = Vec::new();
    let mut leans = [false; 2];
    let mut second_player = false;
    for frame in 0..frames {
        // Every step of the script due by this frame.
        while waiting == 0 {
            let Some(step) = script.next() else {
                break;
            };
            match step {
                Step::Wait(frames) => waiting = frames,
                Step::Punch(fighter) => game.punch(fighter),
                Step::Lean(fighter, on) => leans[fighter] = on,
                Step::SecondPlayer => second_player = true,
                Step::Hold(button) => held.push(button),
                Step::Release => held.clear(),
            }
        }
        waiting = waiting.saturating_sub(1);
        if second_player {
            game.second_player_joins();
        }
        for fighter in [LEFT, RIGHT] {
            game.hold_lean(fighter, leans[fighter]);
        }
        game.step();
        if let Some(every) = request.every.filter(|&every| every > 0) {
            if (frame + 1) % every == 0 {
                save(&game, &request, &held, Some(frame + 1)).await;
            }
        }
    }
    save(&game, &request, &held, None).await;
}

/// Draws the game as it is and saves the screen; `step` names a frame saved
/// on the way.
async fn save(game: &Game, request: &Request, held: &[Button], step: Option<usize>) {
    let layout = Layout::new(
        screen_width(),
        screen_height(),
        request.touch,
        request.sandbox,
    );
    hud::draw(game, &layout, held);
    next_frame().await;
    hud::draw(game, &layout, held);
    let path = step_path(&request.path, step);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| fail(&format!("cannot create {}: {error}", parent.display())));
    }
    get_screen_data().export_png(&path.to_string_lossy());
    println!("wrote {}", path.display());
    next_frame().await;
}

/// OUT.png, or OUT-0012.png for a frame saved on the way.
fn step_path(path: &Path, step: Option<usize>) -> PathBuf {
    let Some(step) = step else {
        return path.to_path_buf();
    };
    let stem = path.file_stem().map_or_else(
        || "capture".to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!("{stem}-{step:04}.png"))
}

/// Draws every character the screen uses at its text sizes once, before any
/// screen grab: text drawn for the first time grows the glyph atlas partway
/// through a frame and garbles it.
async fn warm_glyphs() {
    let characters: String = (' '..='~').collect();
    for size in [14.0, 15.0, 16.0, 17.0, 18.0, 34.0] {
        draw_text(&characters, 0.0, size, size, WHITE);
    }
    next_frame().await;
    clear_background(BLACK);
    next_frame().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_reads_as_its_steps() {
        let steps =
            parse("wait=30:lean1=on:hold=lean1:wait=40:lean1=off:release:punch1:player2:punch2")
                .unwrap();
        assert_eq!(
            steps,
            vec![
                Step::Wait(30),
                Step::Lean(LEFT, true),
                Step::Hold(Button::Lean(LEFT)),
                Step::Wait(40),
                Step::Lean(LEFT, false),
                Step::Release,
                Step::Punch(LEFT),
                Step::SecondPlayer,
                Step::Punch(RIGHT),
            ]
        );
        assert!(parse("jump").is_err());
        assert!(parse("wait=soon").is_err());
    }
}
