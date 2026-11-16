# Realistic Physics: Fight

A two-fighter game on the [Realistic Physics](https://github.com/AzizjonKasimov/realistic-human-body-deformation-physics) engine, a 2D simulation of realistic injuries: skin, muscle and bone stretch, tear, break and bleed through real physics, checked against published injury research.

Play it in your browser: [azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/](https://azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/), or from the sandbox's start screen.

**Content warning:** graphic simulated injury, including blood, torn tissue and broken bones.

## How To Play

You play the left fighter. The two fighters are seen from the side, facing each other, a step out of each other's reach: step in to land a punch. They stand on their own legs and punch with a trained boxer's speed and power.

| Keys | Touch | What it does |
| --- | --- | --- |
| D / A | STEP IN / STEP BACK | Step toward the other fighter / away from it |
| W (hold) | LEAN BACK (hold) | Lean back, so a punch falls short; let go to come back upright |
| S (hold) | DUCK (hold) | Duck under a punch |
| J | JAB | A straight punch with your front hand |
| K | CROSS | A straight punch with your back hand, the hips and shoulders turning through |
| L (hold) | BLOCK (hold) | Close your forearms in front of your face: they stop punches |
| ← → ↑ ↓ and , . / | | A friend plays the right fighter: the arrows step in, step back, lean back, and duck; comma, full stop, and slash jab, cross, and block (1 2 3 on a number pad too) |
| Space / R / M | buttons at the top | Pause / start afresh / sound on or off |

Without a block, a punch goes between a fighter's open hands and lands on its head. A punch that turns the head hard enough knocks the fighter out, as the engine measures it (its `knockout.rs`, from research on punches and knockouts): it goes down and stays down, and the fight is over. A fighter shoved past where a step can catch it falls, which ends the fight too. The screen says who won; R, or the restart button, starts the next fight. Knockouts are rare, as they are in real fights. Fights decided by their other injuries, a computer opponent, and rounds are the next steps.

While nobody plays the right fighter, it spars on its own: it comes into reach, then every second or so picks a jab, a cross, a block, a duck, a lean back, or a step back at random. It never watches what you do. Pressing a friend's key hands it to them; after 10 seconds without them it spars again. Once a fighter is down it stops.

The fighters stand in a ring two steps deep behind each of them: a step back past its edge is not taken. They are drawn as large as the screen allows, and on a phone held upright, where they fill its width, the view pulls back while a fighter reaches out, so no part of either leaves the screen. On a touch screen your seven moves are buttons along the bottom, under your thumbs; a friend plays from the keyboard.

## Build And Run

The game is Rust, built with Cargo, on the engine as a library. The engine comes from its own repository at the commit `Cargo.toml` pins; move the pin to build on another engine version. The engine patches the `quad-snd` sound crate, and a Cargo `[patch]` only applies in the crate being built, so `Cargo.toml` repeats it, pointed at the engine's copy at the same commit.

```powershell
cargo run --release
```

The browser version needs the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`):

```powershell
.\tools\build_web.ps1 -Serve
```

It builds `target\web` and serves it at http://localhost:8081/ (add `?stats` for frame timings). The page reaches the site's visit counting, app manifest, and icons one folder up, which only the engine's site build brings: the engine's website builds this repository at the version it records and serves it at `/fight/`, beside the sandbox.

```powershell
cargo test --release
```

runs the game's tests: the controls, the sparring partner, the camera, the ring, and whole fights played without a window (a cross lands; a punch from where the fighters start falls short; a lean makes a punch miss; a knockout ends the fight, with the knockout limit lowered as the engine's own tests lower it).

Every text file has Unix line endings (LF), in the repository and in every checkout: `.gitattributes` says so whatever Git for Windows' `core.autocrlf` asks for, and `rustfmt.toml` has `cargo fmt` write them, as in the engine's repository. The engine's `.\tools\line_endings.ps1 -Path <this checkout>` switches a checkout made before them, without changing what git sees in the files.

### Film the game

`realistic_physics_fight --capture OUT.png` plays a script of the players' moves and saves the screen, for checking how the game looks without anyone at the keys:

```powershell
.\target\release\realistic_physics_fight.exe --capture output\block.png --script "in1:wait=40:block1=on:hold=block1:wait=60:block1=off:release:cross1:wait=60" --every 6
```

`--size 390x844 --touch` shows a phone's layout, `--sandbox` the browser version's Sandbox button, and `--seed S` another sparring partner. `src/capture.rs` lists every script step (`player2` has a script play the right fighter too). The capture prints the step a fighter was knocked out or fell on.

`.\tools\film.ps1` turns such a script into an MP4 film in one command, at the speed the game plays or slower (it needs ffmpeg: `winget install Gyan.FFmpeg`):

```powershell
.\tools\film.ps1 -Out output\round.mp4 -Script "wait=40:in1:wait=50:jab1:wait=30:cross1:wait=90" -Seed 3
.\tools\film.ps1 -Out output\cross_slow.mp4 -Script "in1:wait=40:cross1:wait=60" -Every 1 -Slow 2 -From 30
```

`-From N` starts the film at step N, `-Size 844x390 -Touch` films a phone held sideways, and `-SkipBuild` uses the game already built.

`--gesture` (`-Gesture`) plays both fighters' moves as the engine writes them instead of a script: a random fight from the engine's `print_random_fights` (with `RP_SEED` and `RP_GESTURE=1`) plays in the game step for step, so a knockout found there can be filmed here. Random fight 15 knocks the left fighter out on step 1170.

## License

The game's code is source-available under the [PolyForm Strict License 1.0.0](LICENSES/PolyForm-Strict-1.0.0.md): you may read it and play the game for personal and other noncommercial purposes, but not change it, share copies of it, or use it commercially ([LICENSE](LICENSE)). The engine it is built on keeps its own licenses, which let people and small businesses use it free; see its repository.
