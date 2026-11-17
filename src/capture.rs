//! Screenshot mode, so the game can be filmed without anyone at the keys:
//! `realistic_physics_fight --capture OUT.png [options]` opens the window,
//! plays a script of the players' moves one simulation step a frame, saves
//! the screen as a PNG, and exits. Native only.
//!
//! ```text
//! --size WXH        window size (default 1280x720)
//! --script SPEC     the players' moves, steps separated by `:` (below)
//! --gesture SPEC    both fighters' moves as the engine writes and films
//!                   them (below), in place of a script
//! --frames N        steps to play (default: the script's own, plus 60)
//! --every N         also save the screen every N steps, as OUT-NNNN.png
//! --seed S          what the computer opponent's decisions follow from
//!                   (default 1)
//! --level L         the opponent's level: easy, normal (the default), or
//!                   hard
//! --rounds N        how many rounds the bout has (default 3)
//! --round-seconds S how long a round lasts (default 60), and the rest
//! --break-seconds S between two (default 10): shorter for films of a
//!                   whole bout
//! --touch           draw the buttons as on a touch screen
//! --sandbox         show the Sandbox button, as the browser version does
//! ```
//!
//! Script steps: `wait=N` lets N steps pass. A fighter's move is its name
//! with the fighter's number, 1 the left and 2 the right: `jab1`, `cross1`,
//! `in1`, and `out1` press a jab, a cross, a step in, and a step back;
//! `block1=on`, `duck1=on`, and `lean1=on` hold a block, a duck, and a lean
//! back, and `=off` lets go. `player2` has a second player take the right
//! fighter, who then moves only as the script says. `hold=NAME` and
//! `release` draw one of the left fighter's buttons held down (`jab1`,
//! `block1`, ...), as a finger on it would, for pictures of the buttons in
//! use. For example
//! `in1:wait=30:jab1:wait=30:block1=on:hold=block1:wait=60:block1=off:release`.
//!
//! A gesture is the engine's way of writing both side-view fighters' moves,
//! as its capture mode films them (`--side --gesture`) and its random
//! fights print them (`print_random_fights` with `RP_SEED` and
//! `RP_GESTURE=1`): a tool and its place first, which the game passes over,
//! then `wait=N`, `jab`, `cross`, `step=in`, `step=out`, `back=on`,
//! `duck=on`, and `block=on` (and `=off`), the second fighter's with a 2
//! after the word (`cross2`, `step2=in`). Both fighters move only as it
//! says, so an engine fight plays in the game step for step as it played
//! there, unless a fighter punches while blocking or steps past the ring's
//! edge, which the game's rules change: random fight 90 knocks the left
//! fighter out on step 159.

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use macroquad::prelude::*;

use crate::bout::{Outcome, Rules};
use crate::controls::{Act, Button, Hold};
use crate::game::{Game, LEFT, RIGHT};
use crate::hud::{self, Layout};
use crate::opponent::Level;
use crate::page::button_name;

/// A capture asked for on the command line.
pub struct Request {
    path: PathBuf,
    script: Vec<Step>,
    frames: Option<usize>,
    every: Option<usize>,
    seed: u64,
    level: Level,
    rules: Rules,
    touch: bool,
    sandbox: bool,
}

/// One step of a capture's script.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Wait(usize),
    Press(usize, Act),
    Hold(usize, Hold, bool),
    SecondPlayer,
    Show(Button),
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
    let script = match (value(&args, "--script"), value(&args, "--gesture")) {
        (Some(_), Some(_)) => fail("--script and --gesture each give the moves; pass one"),
        (Some(spec), None) => parse(&spec).unwrap_or_else(|error| fail(&error)),
        (None, Some(spec)) => parse_gesture(&spec).unwrap_or_else(|error| fail(&error)),
        (None, None) => Vec::new(),
    };
    let number = |name: &str| {
        value(&args, name).map(|text| {
            text.parse::<usize>()
                .unwrap_or_else(|_| fail(&format!("{name} wants a number, not `{text}`")))
        })
    };
    let level = match value(&args, "--level") {
        None => Level::Normal,
        Some(name) => Level::ALL
            .into_iter()
            .find(|level| level.name() == name)
            .unwrap_or_else(|| {
                fail(&format!(
                    "--level wants easy, normal, or hard, not `{name}`"
                ))
            }),
    };
    let seconds = |name: &str, default: f64| {
        value(&args, name).map_or(default, |text| {
            text.parse::<f64>()
                .ok()
                .filter(|seconds| *seconds > 0.0)
                .unwrap_or_else(|| fail(&format!("{name} wants seconds, not `{text}`")))
        })
    };
    let defaults = Rules::default();
    let rules = Rules {
        rounds: number("--rounds").map_or(defaults.rounds, |rounds| rounds.max(1) as u32),
        round_seconds: seconds("--round-seconds", defaults.round_seconds),
        break_seconds: seconds("--break-seconds", defaults.break_seconds),
    };
    Some(Request {
        path,
        script,
        frames: number("--frames"),
        every: number("--every"),
        seed: number("--seed").unwrap_or(1) as u64,
        level,
        rules,
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

/// The button of a fighter's move, by its name with the fighter's number.
fn fighter_button(name: &str) -> Option<Button> {
    [LEFT, RIGHT]
        .into_iter()
        .flat_map(|fighter| {
            Act::ALL
                .into_iter()
                .map(move |act| Button::Act(fighter, act))
                .chain(
                    Hold::ALL
                        .into_iter()
                        .map(move |hold| Button::Hold(fighter, hold)),
                )
        })
        .find(|&button| button_name(button) == name)
}

fn parse(spec: &str) -> Result<Vec<Step>, String> {
    spec.split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (name, setting) = match part.split_once('=') {
                Some((name, setting)) => (name, Some(setting)),
                None => (part, None),
            };
            match (name, setting, fighter_button(name)) {
                ("wait", Some(steps), _) => steps
                    .parse()
                    .map(Step::Wait)
                    .map_err(|_| format!("`{part}` wants a number of steps")),
                ("hold", Some(button), _) => fighter_button(button)
                    .map(Step::Show)
                    .ok_or_else(|| format!("no button `{button}`")),
                ("player2", None, _) => Ok(Step::SecondPlayer),
                ("release", None, _) => Ok(Step::Release),
                (_, None, Some(Button::Act(fighter, act))) => Ok(Step::Press(fighter, act)),
                (_, Some("on"), Some(Button::Hold(fighter, hold))) => {
                    Ok(Step::Hold(fighter, hold, true))
                }
                (_, Some("off"), Some(Button::Hold(fighter, hold))) => {
                    Ok(Step::Hold(fighter, hold, false))
                }
                _ => Err(format!("unknown script step `{part}`")),
            }
        })
        .collect()
}

/// The steps of an engine gesture (the module's notes): both fighters
/// played by it, the tool and its place passed over.
fn parse_gesture(spec: &str) -> Result<Vec<Step>, String> {
    let mut steps = vec![Step::SecondPlayer];
    let parts = spec
        .split(':')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    for (index, part) in parts.enumerate() {
        // The tool, first, and the place it starts at, which the side-view
        // fighters never hold.
        let tool = index == 0
            && !part.contains('=')
            && !matches!(part, "jab" | "jab2" | "cross" | "cross2");
        if tool || part.contains(',') {
            continue;
        }
        let (word, setting) = match part.split_once('=') {
            Some((word, setting)) => (word, Some(setting)),
            None => (part, None),
        };
        let (base, fighter) = match word.strip_suffix('2') {
            Some(base) => (base, RIGHT),
            None => (word, LEFT),
        };
        let held = |setting: Option<&str>| match setting {
            Some("on") => Ok(true),
            Some("off") => Ok(false),
            _ => Err(format!("`{part}` wants =on or =off")),
        };
        steps.push(match (base, setting) {
            ("wait", Some(frames)) => Step::Wait(
                frames
                    .parse()
                    .map_err(|_| format!("`{part}` wants a number of steps"))?,
            ),
            ("jab", None) => Step::Press(fighter, Act::Jab),
            ("cross", None) => Step::Press(fighter, Act::Cross),
            ("step", Some("in")) => Step::Press(fighter, Act::StepIn),
            ("step", Some("out")) => Step::Press(fighter, Act::StepOut),
            ("back", _) => Step::Hold(fighter, Hold::Lean, held(setting)?),
            ("duck", _) => Step::Hold(fighter, Hold::Duck, held(setting)?),
            ("block", _) => Step::Hold(fighter, Hold::Block, held(setting)?),
            ("shove", _) => return Err(format!("the game has no shoves: `{part}`")),
            _ => return Err(format!("unknown gesture step `{part}`")),
        });
    }
    Ok(steps)
}

/// Plays the script and saves the screen.
pub async fn run(request: Request) {
    warm_glyphs().await;
    let mut game = Game::with(
        request.seed,
        request.level,
        request.rules,
        realistic_physics::Materials::default(),
    );
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
    let mut shown: Vec<Button> = Vec::new();
    let mut holds = [[false; 3]; 2];
    let mut second_player = false;
    for frame in 0..frames {
        // Every step of the script due by this frame.
        while waiting == 0 {
            let Some(step) = script.next() else {
                break;
            };
            match step {
                Step::Wait(frames) => waiting = frames,
                Step::Press(fighter, act) => game.press(fighter, act),
                Step::Hold(fighter, hold, on) => holds[fighter][hold.index()] = on,
                Step::SecondPlayer => second_player = true,
                Step::Show(button) => shown.push(button),
                Step::Release => shown.clear(),
            }
        }
        waiting = waiting.saturating_sub(1);
        if second_player {
            game.second_player_joins();
        }
        // While no second player has taken the right fighter, each step
        // gives it what the computer opponent holds instead (`Game::step`).
        for fighter in [LEFT, RIGHT] {
            for hold in Hold::ALL {
                game.hold(fighter, hold, holds[fighter][hold.index()]);
            }
        }
        let ended = game.outcome().is_some();
        game.step();
        let side = |fighter: usize| if fighter == LEFT { "left" } else { "right" };
        // Every punch that landed and on what, numbered as the saved frames
        // are, to find the moments worth filming slowly.
        for punch in game.world.landed_punches() {
            println!(
                "step {}: the {} fighter's {:?} landed on the {:?} at {:.1} m/s",
                frame + 1,
                side(punch.by),
                punch.kind,
                punch.struck,
                punch.speed
            );
        }
        if let (false, Some((outcome, _))) = (ended, game.outcome()) {
            // How the fight ended, for scripts that play one to its end.
            let [left, right] = game.bout.clean();
            let how = match outcome {
                Outcome::Down {
                    down,
                    knocked_out,
                    round,
                } => format!(
                    "the {} fighter {} in round {round}",
                    side(down),
                    if knocked_out {
                        "was knocked out"
                    } else {
                        "fell"
                    }
                ),
                Outcome::Decision {
                    winner: Some(winner),
                } => {
                    format!(
                        "the {} fighter won on points, {left} to {right}",
                        side(winner)
                    )
                }
                Outcome::Decision { winner: None } => format!("a draw on points, {left} each"),
            };
            println!("step {}: {how}", frame + 1);
        }
        if let Some(every) = request.every.filter(|&every| every > 0) {
            if (frame + 1) % every == 0 {
                save(&game, &request, &shown, Some(frame + 1)).await;
            }
        }
    }
    save(&game, &request, &shown, None).await;
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
    // The screen's alpha is whatever the last thing drawn on each pixel left
    // there (translucent buttons and flesh leave it as low as 106), not how
    // opaque the screen is: saved as it is, viewers drew the shot over their
    // own background, washed out. The colours are what the screen shows.
    let mut shot = get_screen_data();
    for pixel in shot.bytes.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    shot.export_png(&path.to_string_lossy());
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
    for size in [13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 34.0] {
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
        let steps = parse(
            "in1:wait=30:jab1:block1=on:hold=block1:wait=40:block1=off:release:player2:cross2:duck2=on:out2",
        )
        .unwrap();
        assert_eq!(
            steps,
            vec![
                Step::Press(LEFT, Act::StepIn),
                Step::Wait(30),
                Step::Press(LEFT, Act::Jab),
                Step::Hold(LEFT, Hold::Block, true),
                Step::Show(Button::Hold(LEFT, Hold::Block)),
                Step::Wait(40),
                Step::Hold(LEFT, Hold::Block, false),
                Step::Release,
                Step::SecondPlayer,
                Step::Press(RIGHT, Act::Cross),
                Step::Hold(RIGHT, Hold::Duck, true),
                Step::Press(RIGHT, Act::StepOut),
            ]
        );
        for wrong in ["jump", "wait=soon", "jab1=on", "block1", "hold=nothing"] {
            assert!(parse(wrong).is_err(), "`{wrong}` read as a step");
        }
    }

    /// An engine gesture plays both fighters: its tool and place are passed
    /// over, its words become the fighters' moves, the second fighter's
    /// with a 2, and a shove, which the game has no key for, is refused.
    #[test]
    fn an_engine_gesture_plays_both_fighters() {
        let steps =
            parse_gesture("hammer:-1.6,0.92:wait=17:block=on:wait=8:step=out:jab2:back2=off")
                .unwrap();
        assert_eq!(
            steps,
            vec![
                Step::SecondPlayer,
                Step::Wait(17),
                Step::Hold(LEFT, Hold::Block, true),
                Step::Wait(8),
                Step::Press(LEFT, Act::StepOut),
                Step::Press(RIGHT, Act::Jab),
                Step::Hold(RIGHT, Hold::Lean, false),
            ]
        );
        for wrong in [
            "hammer:-1.6,0.95:shove=60",
            "hammer:-1.6,0.95:jbb",
            "duck=up",
        ] {
            assert!(parse_gesture(wrong).is_err(), "`{wrong}` read as a gesture");
        }
    }
}
