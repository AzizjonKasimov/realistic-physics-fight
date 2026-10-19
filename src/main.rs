//! Fight: two fighters on the realistic_physics engine, each driven by a
//! player's punch and lean back, or the right one by a sparring partner
//! while nobody plays it. The engine does the bodies, their injuries, the
//! drawing of them, and their sound; this game adds the players' controls
//! and the screen around the fight. The browser version sits on the
//! engine's website, under the sandbox (`web/index.html`).

use macroquad::prelude::*;
use realistic_physics as rp;

mod camera;
#[cfg(not(target_arch = "wasm32"))]
mod capture;
mod controls;
mod game;
mod hud;
mod page;
mod sparring;

use controls::{Controls, Intents};
use game::{Game, LEFT, RIGHT};
use hud::Layout;

fn window_conf() -> Conf {
    #[cfg(not(target_arch = "wasm32"))]
    let (window_width, window_height) = capture::window_size().unwrap_or((1280, 720));
    #[cfg(target_arch = "wasm32")]
    let (window_width, window_height) = (1280, 720);
    Conf {
        window_title: "Realistic Physics: Fight".to_owned(),
        window_width,
        window_height,
        high_dpi: true,
        sample_count: 4,
        ..Conf::default()
    }
}

fn main() {
    // `--version` prints the version the game shows and exits before the
    // window opens.
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::args().skip(1).any(|arg| arg == "--version") {
        println!("{}", game::version_label());
        return;
    }
    // What `#[macroquad::main(window_conf)]` would write.
    macroquad::Window::from_config(window_conf(), run());
}

async fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(request) = capture::request() {
        capture::run(request).await;
        return;
    }
    // A different sparring partner every visit: its waits follow from the
    // moment the game starts.
    let seed = (macroquad::miniquad::date::now() * 1000.0) as u64;
    let mut game = Game::new(seed);
    let mut controls = Controls::default();
    let mut sound = rp::sound::Player::start();
    let mut shown: Option<Layout> = None;
    loop {
        let dt = get_frame_time().min(0.05) as f64;
        let layout = Layout::new(
            screen_width(),
            screen_height(),
            controls.touch,
            page::has_sandbox(),
        );
        if shown.as_ref() != Some(&layout) {
            page::show_buttons(&layout.buttons);
            shown = Some(layout.clone());
        }
        let intents = controls.read(&layout);
        if intents.clicked {
            sound.click();
        }
        apply(&mut game, &intents);
        game.advance(dt, |world| sound.hear(world));
        sound.play(dt as f32, game.muted, game.running);
        game.blend();
        hud::draw(&game, &layout, controls.held());
        game.restore();
        next_frame().await;
    }
}

/// Does what the players asked for this frame.
fn apply(game: &mut Game, intents: &Intents) {
    if intents.restart {
        game.restart();
    }
    if intents.pause {
        game.running = !game.running;
    }
    if intents.mute {
        game.muted = !game.muted;
    }
    if intents.sandbox {
        page::back_to_sandbox();
    }
    for fighter in [LEFT, RIGHT] {
        if intents.punch[fighter] {
            game.punch(fighter);
        }
        game.hold_lean(fighter, intents.lean[fighter]);
    }
}
