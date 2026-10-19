# Realistic Physics: Fight

A two-fighter game on the [Realistic Physics](https://github.com/AzizjonKasimov/realistic-human-body-deformation-physics) engine, a 2D simulation of realistic injuries: skin, muscle and bone stretch, tear, break and bleed through real physics, checked against published injury research.

Play it in your browser: [azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/](https://azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/), or from the sandbox's start screen.

**Content warning:** graphic simulated injury, including blood, torn tissue and broken bones.

## How To Play

You play the left fighter. Both fighters face you, standing side by side; each punches with the arm nearer the other.

| Keys | Touch | What it does |
| --- | --- | --- |
| D or → | PUNCH (bottom left) | Punch the other fighter |
| A or ← (hold) | LEAN BACK (hold) | Lean away so a punch falls short; let go to come back upright |
| J | PUNCH (bottom right) | A friend punches with the right fighter |
| L (hold) | LEAN BACK (hold) | A friend leans the right fighter back |
| Space / R / M | buttons at the top | Pause / start afresh / sound on or off |

While nobody plays the right fighter, it spars on its own: it throws a punch every 1.5 to 3.5 seconds, at moments you cannot foresee, so you have punches to lean away from and to answer. Pressing J or L hands it to a friend; after 10 seconds without them it spars again.

The fighters are drawn as large as the screen allows. On a phone held upright, where they fill its width, the view pulls back while a fighter leans out, so no part of either leaves the screen, and comes back in once it stands upright again.

Punching opens your own guard: the arm that punches is the arm that guards your head on that side. Nobody wins yet. Blows hurt the fighters as they hurt bodies in the sandbox, and the fight goes on until you start it afresh. Injuries that decide the fight, a computer opponent, and rounds are the next steps.

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

runs the game's tests: the controls, the sparring partner, and whole fights played without a window.

### Film the game

`realistic_physics_fight --capture OUT.png` plays a script of the players' moves and saves the screen, for checking how the game looks without anyone at the keys:

```powershell
.\target\release\realistic_physics_fight.exe --capture output\lean.png --script "wait=30:lean1=on:hold=lean1:wait=60:lean1=off:release:punch1:wait=60" --every 6
```

`--size 390x844 --touch` shows a phone's layout, and `--sandbox` the browser version's Sandbox button. `src/capture.rs` lists every script step.

## License

The game's code is source-available under the [PolyForm Strict License 1.0.0](LICENSES/PolyForm-Strict-1.0.0.md): you may read it and play the game for personal and other noncommercial purposes, but not change it, share copies of it, or use it commercially ([LICENSE](LICENSE)). The engine it is built on keeps its own licenses, which let people and small businesses use it free; see its repository.
