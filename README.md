# Realistic Physics: Fight

A two-fighter game on the [Realistic Physics](https://github.com/AzizjonKasimov/realistic-human-body-deformation-physics) engine, a 2D simulation of realistic injuries: skin, muscle and bone stretch, tear, break and bleed through real physics, checked against published injury research.

Play it in your browser: [azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/](https://azizjonkasimov.github.io/realistic-human-body-deformation-physics/fight/), or from the sandbox's start screen.

**Content warning:** graphic simulated injury, including blood, torn tissue and broken bones.

## How To Play

You play the left fighter. The two fighters are seen from the side, facing each other, a step out of each other's reach: step in to land a punch. They stand on their own legs and punch with a trained boxer's speed and power.

| Keys | Touch | Controller (PlayStation / Xbox) | What it does |
| --- | --- | --- | --- |
| D / A | STEP IN / STEP BACK | D-pad or left stick right / left; held, it keeps stepping | Step toward the other fighter / away from it |
| W (hold) | LEAN BACK (hold) | up (hold) | Lean back, so a punch falls short; let go to come back upright |
| S (hold) | DUCK (hold) | down (hold) | Duck under a punch; a jab or cross thrown while ducking goes to the body |
| J | JAB | L2 / LT, or square / X | A straight punch with your front hand |
| K | CROSS | R2 / RT, or triangle / Y | A straight punch with your back hand, the hips and shoulders turning through |
| L (hold) | BLOCK (hold) | R1 or L1 / RB or LB (hold) | Close your forearms in front of your face: they stop punches at your head. Held while ducking, they come down over your body instead (a low block), and your head is open |
| ← → ↑ ↓ and , . / | | a second controller; CREATE / VIEW switches a controller to the other fighter | A friend plays the right fighter: the arrows step in, step back, lean back, and duck; comma, full stop, and slash jab, cross, and block (1 2 3 on a number pad too) |
| Space / R / M | buttons at the top | OPTIONS / MENU pauses; cross / A fights again once a fight is over | Pause / start afresh / sound on or off |
| | level button at the top | | A new fight against the next level of opponent: easy, normal, hard |

Any game controller your browser reads works: a PS5 DualSense, a PS4 DualShock 4, an Xbox controller, or another, plugged in or paired at any time. A browser shows a controller to a page only once one of its buttons is pressed, and only on a secure page, so press any button to connect it; the start card then names it, lists its buttons beside the keys, and its cross (A) starts the fight. The first controller plays your fighter, the left one, and a second the right fighter; CREATE (VIEW, or SELECT on others) moves a controller to the other fighter, so a keyboard and a controller can fight each other either way round, and the keyboard keeps both of its halves. The triggers are your two fists: the left one (L2, LT) jabs with your front hand and the right one (R2, RT) crosses with your back hand, a pull past half way throwing one punch; the left and top face buttons punch too. The buttons at the bottom show the controller's own names for its buttons (L2, R2, R1 on a PlayStation controller, LT, RT, RB on an Xbox one, and the D-pad) once you use it, and the keys again once you press one of your fighter's keys. Your controller rumbles where the browser can: a light tick when your punch lands clean, a thump when you are hit clean, and both motors for most of a second when you go down. On Windows, Chrome and Edge rumble an Xbox controller and a PS4 DualShock 4 by themselves but not a PS5 DualSense (Chromium's controller reader there rumbles only the DualShock 4 among Sony's controllers); Firefox rumbles on some systems, and Safari not at all. For a DualSense, the first press of one of its buttons opens the browser's window for picking it (WebHID, in Chrome and Edge, which count a controller's press as the player's action a page needs to open that window); it opens by itself once a visit, and on no later visit once it has been shown, and the start card and a Turn on DualSense rumble button under the game's title during a fight offer it too. Picked once, the controller is remembered by the browser, and from then on the page drives the DualSense directly, reading its buttons and sticks from its own reports and sending it rumble (`web/dualsense.js`, its report layouts rebuilt from SDL's HIDAPI PS5 driver, zlib licence). Over Bluetooth the DualSense then stays in its full report mode until it is switched off and on, and Windows programs that read it as a plain game controller (DirectInput) see no buttons until then; Steam and most new games read that mode. The start card says when the browser cannot rumble a controller and the page cannot either. The page reads the controllers through the browser's Gamepad API (its `fight` plugin in `web/index.html`), and the game turns what each holds into its fighter's moves as it does the keys (`src/pads.rs`). The desktop version plays with the keyboard only.

Without a block, a punch goes between a fighter's open hands and lands on its head. Thrown while ducking, it goes to the body, under a block, and only a low block stops it there. Punching opens your guard until the punch is back, for both fighters: a forearm left up beside a punch would stop every answer. A punch that turns the head hard enough knocks the fighter out, as the engine measures it (its `knockout.rs`, from research on punches and knockouts): it goes down and lies limp, for longer the harder the punch, then gets up on its own (the engine's `rise.rs`): from its back it sits up onto its hands, crouches, and stands; from its front it comes up onto its knees, takes a knee, and stands. A fighter shoved past where a step can catch it falls, and gets up as soon as it comes to rest. Knockdowns are rare, as they are in real fights.

A fight is three rounds of a minute, with ten seconds' rest between them, while both fighters walk back to their corners and a card gives the round's score. A fighter knocked down by a punch gets a referee's count, a second at a time on a card, while the other fighter goes back to its edge of the ring, as a boxer goes to a neutral corner: up by ten, it fights on once the count has reached eight (the mandatory eight count), and still down at ten it is knocked out and the fight is over. Three knockdowns end the fight, as amateur referees stop a bout after three counts. A fall with no clean punch landing just before it is a slip: no count, and the fighter just gets up. The bell does not save a fighter: a round that runs out during a count ends once the count does. Otherwise, when the last round ends, the fighter who landed more clean punches wins: punches that land on the head or the body count, punches stopped on the arms do not, as Olympic boxing's computer scoring counted them from 1992 to 2012, and only punches that land hard, at nine tenths of the speed a fresh fighter's jab or cross lands with from within its reach or faster (6.9 and 7.2 m/s): amateur boxing counts a blow only with the weight of the body or the shoulder behind it, and the counting machine was dropped in 2013 for rewarding light, quick touches. Equal counts are a draw. Punching tires a fighter's arms (the engine's v0.20): the bar under each fighter's tag shows how much strength its arms have left, punching flat out wears it down within seconds, and tired arms punch slower and softer, so their punches stop counting, and come back to the guard later, while resting between punches and between rounds brings the strength back. The computer opponent paces itself as a boxer does, makes a fighter throwing punch after punch miss, stepping out of its reach or covering up, and goes after a fighter whose bar shows it tired. The top of the screen shows the round, its clock, and each fighter's clean punches; R, or the restart button, starts the next fight. Fights decided by their other injuries are the next step.

While nobody plays the right fighter, the computer opponent fights it (`src/opponent.rs`). It sees only what is on the screen, a moment late as a person would, never your keys: how far apart you stand, what your fighter is doing (a punch on its way, a step, a block, a duck, a lean), and where punches land, as the engine shows them. A punch arrives in about a tenth of a second, faster than anyone reacts, so like a boxer it defends by reading what comes before one: it covers up, leans back, steps out, or meets you with a jab as you step in; covers up after it is hit and the more you punch; and blocks the second punch of a pair, low against punches to the body. It counters into the guard your punch opens, punches more when your guard is down, goes to the body about one punch in four, and mostly when your guard is up, follows you when you back off, and makes room when crowded. It makes mistakes, more of them the easier its level: it notices late or not at all, picks the wrong answer, and drops its guard to punch. Pick its level on the page's start card (Normal by default, remembered on your device) or with the level button. Pressing a friend's key hands the right fighter to them; after 10 seconds without them the opponent takes it back. Once the fight is over it stops.

The fighters stand in a ring two steps deep behind each of them: a step back past its edge is not taken. They are drawn as large as the screen allows, and on a phone held upright, where they fill its width, the view pulls back while a fighter reaches out, so no part of either leaves the screen. On a touch screen your seven moves are buttons along the bottom, under your thumbs; a friend plays from the keyboard or a controller.

## Build And Run

The game is Rust, built with Cargo, on the engine as a library. The engine comes from its own repository at the commit `Cargo.toml` pins; move the pin to build on another engine version. The engine patches the `quad-snd` sound crate, and a Cargo `[patch]` only applies in the crate being built, so `Cargo.toml` repeats it, pointed at the engine's copy at the same commit.

```powershell
cargo run --release
```

The browser version needs the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`):

```powershell
.\tools\build_web.ps1 -Serve
```

It builds `target\web` and serves it at http://localhost:8081/ (add `?stats` for frame timings). The page reaches the site's visit counting, app manifest, and icons one folder up, which only the engine's site build brings: the engine's website builds this repository at the version it records and serves it at `/fight/`, beside the sandbox. In the engine's checkout, `.\tools\build_web.ps1 -Fight <this checkout>` builds that whole site, `.\tools\touch_check.ps1 -Fight` plays its buttons with real fingers on a phone, and `.\tools\pad_check.ps1` plays it with fake game controllers (a DualSense, then an Xbox controller for a friend) in place of the browser's own, all in headless Edge.

```powershell
cargo test --release
```

runs the game's tests: the controls, and game controllers fed through the same path as the keys (their triggers, buttons, and sticks play a fighter as its keys do, toward the other fighter on either side; the first plays the left fighter and a second the right; the select button switches; a Sony controller whose buttons come in its own order; the stick pushed past half and let go near the middle, and a trigger pulled past half and let go below a third; a key turns the buttons back to keys), the computer opponent (it never answers a punch sooner than a person could, and the harder it is the more it punches and answers), the rounds and their decision, the count (up by ten fights on after eight, still down at ten is a knockout, a fall with no punch is a slip, the third knockdown stops it, the bell saves no one), the camera, the ring, and whole fights played without a window (a cross lands, and both players feel it through their controllers; a step held on a controller keeps stepping, to the ring's edge and back in; a punch from where the fighters start falls short; a lean makes a punch miss; a knockdown is counted, the player goes back to its edge of the ring, throws nothing at the fighter down, and the fight goes on once that fighter is up; still down at ten ends the fight, with the knockout limit lowered as the engine's own tests lower it; the opponent beats a player who does nothing, counters and covers up against one who jabs, and loses on easy to a careful player whom the hard opponent hits far more; between rounds the fighters walk back to their corners).

```powershell
cargo test --release print_bouts -- --ignored --nocapture
```

plays every level of the opponent against four scripted players (one who does nothing, one who only jabs, a careful boxer whose guard is up but for its punches, and one who only blocks) and prints what each side threw and landed, the opponent's counters and guard, and how each bout ended (`src/trial.rs`; `RP_BOUTS=N` seeds each, `RP_ROUND=S` seconds a bout, `RP_LEVEL` and `RP_PLAYER` keep one). Use it after any change to the opponent, the rounds, or the engine's fighters.

Every text file has Unix line endings (LF), in the repository and in every checkout: `.gitattributes` says so whatever Git for Windows' `core.autocrlf` asks for, and `rustfmt.toml` has `cargo fmt` write them, as in the engine's repository. The engine's `.\tools\line_endings.ps1 -Path <this checkout>` switches a checkout made before them, without changing what git sees in the files.

### Film the game

`realistic_physics_fight --capture OUT.png` plays a script of the players' moves and saves the screen, for checking how the game looks without anyone at the keys:

```powershell
.\target\release\realistic_physics_fight.exe --capture output\block.png --script "in1:wait=40:block1=on:hold=block1:wait=60:block1=off:release:cross1:wait=60" --every 6
```

`--size 390x844 --touch` shows a phone's layout, `--sandbox` the browser version's Sandbox button, `--pad playstation|xbox|other` a game controller's buttons as when the player uses one, `--seed S` another opponent, `--level easy|normal|hard` its level, and `--rounds N --round-seconds S --break-seconds S` a shorter bout, to film its rests and decision. `src/capture.rs` lists every script step (`player2` has a script play the right fighter too). The capture prints each punch that lands, on which step and on what (the head, the body, or an arm), each knockdown as its count begins and ends, and how the fight ended and on which step: a knockout at the count of ten, a third knockdown, or the decision and its count. The steps are the saved frames' numbers, so `-From` can start a slow film just before a punch.

`.\tools\film.ps1` turns such a script into an MP4 film in one command, at the speed the game plays or slower, keeping the display on while the game plays and listing each punch that lands (it needs ffmpeg: `winget install Gyan.FFmpeg`):

```powershell
.\tools\film.ps1 -Out output\round.mp4 -Script "wait=40:in1:wait=50:jab1:wait=30:cross1:wait=90" -Seed 3
.\tools\film.ps1 -Out output\cross_slow.mp4 -Script "in1:wait=40:cross1:wait=60" -Every 1 -Slow 2 -From 30
```

`-From N` starts the film at step N, `-Size 844x390 -Touch` films a phone held sideways, `-Level`, `-Rounds`, `-RoundSeconds`, and `-BreakSeconds` pass the capture's options, and `-SkipBuild` uses the game already built.

`--gesture` (`-Gesture`) plays both fighters' moves as the engine writes them instead of a script: a random fight from the engine's `print_random_fights` (with `RP_SEED` and `RP_GESTURE=1`) plays in the game step for step, so a knockout found there can be filmed here, as long as neither fighter punches while blocking or steps past the ring's edge, which the game's rules change. Random fight 3 knocks the left fighter down on step 583 in the game, and it gets up and fights on after the count; random fight 90 knocks the left fighter out on step 159; random fight 42's knockout, which comes after punches thrown while blocking, does not happen in the game.

## License

The game's code is source-available under the [PolyForm Strict License 1.0.0](LICENSES/PolyForm-Strict-1.0.0.md): you may read it and play the game for personal and other noncommercial purposes, but not change it, share copies of it, or use it commercially ([LICENSE](LICENSE)). The engine it is built on keeps its own licenses, which let people and small businesses use it free; see its repository.
