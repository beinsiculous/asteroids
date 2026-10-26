# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run the 82 headless tests (39 in src/gameplay_tests.rs, 15 in music_tests.rs, 10 in art_tests.rs, 7 in harness_tests.rs, 4 in spawning.rs, 4 in achievements.rs, 2 in title_screen_tests.rs, 1 in constants.rs)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` cargo feature). `glam` and `log` are the only other dependencies; the tests add `image` and the engine's `music_midi`.

## Architecture

This is a single-crate game (`insiculous_asteroids`, shown to players as **Meatieroids**) built on the in-house `insiculous_2d` ECS engine. `AsteroidsGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/lib.rs` — `init()` loads the font and the synced sheets and spawns only the star tiles and the deforming grid (ships and meatieroids spawn fresh in `start_game()` because ship count depends on the picked mode); achievements register in `register_achievements()`, which the engine calls before the window opens, and `cargo run -- --achievements-manifest <path>` exports the list. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`, and at `/playground/meatieroids/` in the browser (the same feature, built by the engine's `build_wasm.sh --kind editor` and served from the site); no game code changes between the modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()`: `TitleScreen { selection }`, `ModeSelect { selection }`, and `Achievements` dispatch to input handlers in `menu.rs`; `Playing` and `GameOver` fall through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{ship,rocks,flow}.rs`. Flow is Title → ModeSelect (chaos pick, then `start_game`) → Playing ↔ GameOver (endless; the only ending is running out of ships). Match lifecycle (start, finish, reset-to-title, scoring/extra lives, theme application) lives in `gameplay/flow.rs`.

**Everything is a sprite off a synced sheet** (`art.rs` holds one `SheetSpec` per sheet, the clip names, one draw depth per sheet and the colours; `art_tests.rs` re-measures every number from the PNGs). Back to front: the night is the clear colour (devil's-food `#241418`, decoded to linear by `night_clear_color()`), the engine's `GridBackdrop` draws behind the sprites, then the star `Tilemap` on its transparent ground, then the meatieroids, shots and Funks. The game draws no lines of its own except F1's collider overlay. **The Funk is drawn in sixteen headings**: its body's rotation is locked, `ShipState.heading` (radians, kept in [0, 2π)) turns, and the pure `ship_pose(heading, thrusting, play_time)` picks the nearest drawn heading's `idle_NN`/`thrust_NN` clip and a frame from the match clock, written onto the sprite every frame (`playing = false`). **Thrust and fire follow the drawn heading** (`heading_direction`), so a shot leaves in one of sixteen directions. **Meatieroids tumble at runtime**: their bodies spin, and each sheet's sprite offset puts the drawn body's centre on the entity so it turns about its middle. A wave's meatieroid loops `idle`; a split's children run `attack` → `idle`. A shattered meatieroid's `die` and a lost life's `crash` are detached one-shots kept in `transient_visuals`, ended by a restart or a quit.

**The game steps physics itself.** `AsteroidsGame` owns a `PhysicsSystem` (`PhysicsConfig::space()` — zero gravity) and calls `self.physics.update(ctx.world, ctx.delta_time)` inside `update_gameplay()`, followed by a single `take_collision_events()` drain shared by all consumers; stray bullets expire in the engine's frame tail (`ecs::LifetimeSystem`), which runs after the game's update — the game owns no lifetime system. Body recipes (spawning.rs):
- **Ship**: dynamic, **rotation locked**, all damping zero, **sensor** circle collider (radius 11, inside the drawn 26 × 20 hull) — meatieroids must never physically deflect a ship, and co-op ships pass through each other. Flight model is the pure `integrate_ship` in gameplay/ship.rs: read rapier velocity, integrate thrust along the drawn heading/drag/clamp, push back with ONE `set_velocity` per frame.
- **Meatieroids**: solid dynamic bodies, restitution 1.0 / friction 0.0 — they bounce elastically off each other. The circle collider is the drawn round body (29.5 / 18.5 / 11.5); the arms and fists are drawn, not collided. A split's children are placed along the unjittered fan (`split_offset`) so siblings are born clear of each other.
- **Shots**: dynamic sensors (radius 4) with a `Lifetime` (1.1 s ≈ under one screen crossing); a shot expiring via lifetime counts as a miss and resets that ship's sharpshooter streak (`prune_expired_bullets`). The drawn icicle points up, so a shot's rotation is its bearing − 90°, written ONCE at spawn: physics reads a later change to `Transform2D.rotation` as an outside edit and teleports the body.

**Two hit paths, deliberately different.** Bullet-vs-rock uses collision events through the pure `resolve_hit_pairs` (each bullet and rock used at most once per frame; ships are structurally excluded from the candidate set, so friendly fire is impossible). Ship-vs-rock is a game-side circle test in `check_ship_hit` — NOT events — because a rock that drifted over a ship during i-frames must still kill the instant mercy expires, which a stale `started` event can't do.

**Determinism convention: no RNG.** Every random-looking decision (speeds, spins, spawn points, split jitter) draws from `hash_f32` over a monotonic seed stream (`next_seed()` / `spawn_counter`). Wave rocks spawn on the window border via `spawn_position`, which walks 8 evenly spaced perimeter candidates from a hashed start and takes the first that clears `SAFE_SPAWN_DIST` (160 px) from every live ship, else the roomiest.

**Screen wrap** (`wrap_entities` / pure `wrap_position`): toroidal field with a per-entity margin — the farthest drawn pixel (the Funk's plume, a meatieroid's fist) — so entities fully exit before re-entering. The native window is fixed at 800 × 600 (`main.rs`, the bare game only) because wrap, spawns, the grid and the stars all read that field. Writing `Transform2D.position` on a live dynamic body is a rapier teleport that preserves velocity (GPP-09) — proven by a sim test. Wrapping also runs during GameOver so leftover rocks keep circling behind the overlay.

**2-player co-op** (`GameMode::TwoPlayerCoop`): two ships share one rock field but keep per-ship lives, scores, bullets, streaks, and i-frames (`ShipState`; `entity: Option<EntityId>` becomes `None` when that ship is out — the survivor plays on; the match ends only when `coop_over` says every ship is done). Input goes through `ctx.players`: co-op ship `index` reads `PlayerId(index)`; single player merges P1+P2 (either keyboard cluster or pad flies the lone ship). Turn is `-move_x`, thrust is `move_y > 0` or MoveUp, fire is a fresh Action1. `Pilot::for_ship` picks the art: P1 (and solo) is Deion's Funk firing icicles, P2 the recoloured Funk firing ice chips.

**Editor naming:** every spawned entity gets a `Name` component ("Flying Funk", "Shot", "Meatieroid Large #7") so the editor hierarchy is readable — keep this when adding entities.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — every sprite's scale is its `SheetSpec::scale()`, which draws the sheet at 1×. Pixel snap is on.
- Collider shapes use **absolute pixels** and ignore `Transform2D.scale` entirely. Sprites and colliders are sized through different paths, so they can silently diverge — `F1` in-game (or `C` in the editor) overlays magenta collider outlines to check, and `art_tests.rs` holds the numbers to the pixels. Note the deliberate mismatches: the ship's collider (11) inside its drawn hull, and the meatieroids' arms and fists outside their body circles.

**All tuning lives in `src/constants.rs`** (speeds, radii, split/wave tables, achievement thresholds, grid impulses), what is measured from the art lives in `src/art.rs`, and all entity creation lives in `src/spawning.rs`, spawned from those. Asteroid size tables (`ASTEROID_RADII/SCORES/SPEED_RANGES`) are indexed by `AsteroidSize::idx()` — keep index order Large, Medium, Small.

**Chaos modes** (engine `ChaosMode`, picked on the ModeSelect screen and mirrored into `ctx.chaos_mode`): Insane = rock speeds ×1.6 and respawn i-frames halved; Ridiculous = large rocks split into 3 (not 2) and +1 rock per wave; Insiculous = both (`is_insane()`/`is_ridiculous()` both fire). `ChaosTheme::for_mode` colours the grid, the menus and the banner, and scales the particle counts; `apply_theme` recolours the grid on match start.

**Achievements** (11, defined in `achievements.rs`, registered in `register_achievements()`, which the engine calls before the window opens; `cargo run -- --achievements-manifest <path>` exports the list): wave-5 per mode ×4, score tiers 10k/30k, and skill (Sharpshooter 10-hit streak, Close Call ≤60 px, Untouchable 3 deathless waves, Double Tap 2 kills ≤0.5 s, Living on the Edge = wave 5 on the last total life). `DISPLAY_SECTIONS` drives the achievements page; a test enforces it covers every registered id. Extra life every 10,000 points, per ship.

**Pause** is the engine `PauseMenu`, gated to `GameState::Playing` only. While paused the whole update is frozen; the engine holds the sprites and the grid still, and only F1's overlay is re-emitted.

**Paths:** `engine_core::game_root!()` anchors everything, so `cargo run` works from any cwd. Assets in `assets/` (`fonts/font.ttf`, the synced `sprites/`, the song under `ai/music/` and its bank under `soundfonts/`); achievements persist to `saves/asteroids_achievements.json`, input bindings to `saves/input_settings.json`.

**Tests** are fully headless: `src/gameplay_tests.rs` (39 — pure rules plus rapier sim tests using the exact spawn recipes), `src/music_tests.rs` (15 — the song through the harness's virtual clock, § Music below), `src/art_tests.rs` (10 — every measured number and clip against the synced sheets), `src/harness_tests.rs` (7 — the whole game through the engine's `GameHarness`, fixtures in `test_support.rs`), `src/title_screen_tests.rs` (2) + inline mods in `spawning.rs` (4), `achievements.rs` (4) and `constants.rs` (1). Pure functions (`integrate_ship`, `ship_pose`, `displayed_heading`, `wrap_position`, `split_spec`, `split_offset`, `split_velocity`, `resolve_hit_pairs`, `spawn_position`, `coop_over`, …) exist precisely so behavior can be pinned without a window — keep new rules in that shape.

## The Deion Re-skin (Phase G): Meatieroids

Shipped: the asteroids are **roided-out flexing meatballs** in the same three sizes (a split is a flex-burst — the children are born flexing), Deion flies the **Flying Funk** from above, player 2 flies a recoloured Funk (no name needed), and the shots are Deion's icicles and Cubert's ice chips. The hull's "FLYING FUNK" lettering lives on the side-view casting only. **Ids, score keys and the native save files did not change** from Insiculous Asteroids; only what a player reads was re-cast. The site names it **Meatieroids** at `/games/meatieroids/`, the former `/games/asteroids/` redirecting there, and its web save keys are `beinsiculous.games.meatieroids.*`, with the former `beinsiculous.games.asteroids.*` keys read at boot. Dr. Maxwell's cake saucer is the next batch (`asteroids#2`).

- Style SSOT: `deion_assets/DEION_STYLE.md` via the `deion_assets -> ../../deion_assets` symlink (the working set's layout — the Cargo path dep `../../insiculous_2d` already requires it). The game ships at 1×, pixel-snapped; never fake a footprint via `Transform2D.scale` (physics ignores it).
- Runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/` (`python3 deion_assets/scripts/sync_sprites.py .`, pinned in `sync.list`) — never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix) — tiered ship rule (DEION_STYLE.md §6): may ship in FREE web builds, never in paid/marketplace builds; `deion_assets/scripts/check_no_ai_assets.sh` must pass on any paid release's asset tree. Sheet clip names are the stable API between art and code.

## Music (Mars, 2026-10)

Meatieroids plays Gustav Holst's *Mars, the Bringer of War* through the engine's conductor
(`training.md` § Conductor Pattern in the engine). The game's part is `src/music.rs` — its
constants, its moments, its counts and its pulse — standing on the engine's `GameMusic`
(`engine_core::game_music`), which carries the request, the match start's identity rule, the
jam's latch, the beat and the telemetry. Its tests are `src/music_tests.rs`.

- **The song** is `assets/ai/music/ai_meatieroids.mid` and its sidecar: thirty-two bars of 5/4 on G
  at 160 BPM, 60 s (Mars is a long crescendo and a match is long), looping on the ostinato's
  downbeat. An agent sequenced it from memory through the composer's command API (`song new`,
  `song edit …`, `song save ai/music/ai_meatieroids.mid`), so it sits under `ai/music` with the
  `ai_` prefix until a person's pass promotes it; the composer's Save wrote the sidecar and the
  stanza in `assets/ai/music/LICENSE`. Never hand-edit the sidecar — open the song in the composer
  (`cargo run --features editor`, File → Open Song) and save it again. The meter is written, not
  assumed: a song without one is 4/4, and the pulse's bars would drift a beat a bar. The loop ends
  where its last written note ends, so the tuba's last pedal note is held to the bar line.
- **The background** — the timpani's ostinato on G every bar (a triplet, two quarters, two eighths,
  a quarter; track `ostinato`), the tuba's G pedal (`pedal`) and the trumpet's sparse calls
  (`calls`) — plays by itself from `init` on, through the menus, under the pause menu and under the
  game-over panel. `song new` makes a track `melody` on channel 1 with program 0, and the composer
  changes no track's program; the bank has no preset 0, so a note there would sound on its lowest
  preset. That track stays empty.
- **Nine bindings, two source kinds**, every part `quantize: Off` (a quantized trigger in the
  loop's last beat would be parked on the seam and dropped):

  | binding | moment | part (mode) | instrument | source |
  |---|---|---|---|---|
  | `shot` | a ship fires (`update_firing`), once a shot | `drum` (AtSongPosition) | timpani | the ship's own |
  | `large` | a large meatieroid broken (`shatter_asteroid`, right after the destroy) | `theme` (Loop) | tuba | the game's — the meatieroid is gone in that call and its note rings on |
  | `medium` | a medium meatieroid broken | `theme` (Loop) | trombone | the game's |
  | `small` | a small meatieroid broken | `theme` (Loop) | trumpet | the game's |
  | `thrust` | a ship's thrust starting (`update_ship_control`, the rising edge) | `surge` (Loop, one low held note) | tuba | the ship's own |
  | `crash` | a ship lost (`kill_ship`) | `crash` (Loop, one chord) | orchestra hit | the game's |
  | `wave` | the next wave arriving (`check_wave_clear`'s spawn edge) | `fanfare` (Loop, one chord) | trumpet | the game's |
  | `life` | an extra ship (`award_points`'s count, sounded by the break that earned it) | `life` (Loop, one short high note) | trumpet | the game's |
  | `menu` | a move in the pause menu | `jam` (Loop, the calls as a counter-line) | trumpet | the menu's |

  The three sizes walk **one** brass theme, written in the octave the tuba, the trombone and the
  trumpet all carry (G3 to D4), so a wave is walked through Mars's first subject in the order its
  meatieroids break, the big ones low. A cue is one note or one chord: `Loop` plays one unit (the
  notes starting on one tick) a trigger. A shot drums the beat written for that moment, always on
  the ostinato; the drum line has no rest from tick 0 to the loop's end, which a test holds.
  Holding thrust sends nothing more, and releasing sends nothing. **No hyperspace binding**: the
  game has no hyperspace (the cake saucer, `asteroids#2`, is the next move). No binding for a
  wave's clear edge (the two-second breath is the signal; the fanfare lands with the rocks), a
  shot expiring, a meatieroid bounce or a close call.
- **The sources' lifecycle**: nothing is ever off screen — `wrap_entities` teleports everything at
  its margin — so there is no visibility exit and no `ctx.sources.end` call. A ship's last life
  despawns it in `kill_ship`, ending its source in that frame's tail; a surviving ship is
  teleported and keeps its source. A meatieroid is never a source; the game's is never ended.
- **The order within a frame** is `update_gameplay`'s: each ship's thrust then its shot
  (`update_ship_control`, in ship order), the breaks in the hit pairs' order (with a `life` inside
  the break that earned it), the crashes, the wave. `Music.triggered`, built only in tests,
  records the frame's calls.
- **A match start decides by the song in the slot** (`GameMusic::restart`, at the end of
  `start_game`, which every match start takes): none → requested again; the game's own → back to
  its top; another song (the composer's audition in the editor) → left alone. The counts run for
  the session; a match start never resets them.
- **The pulse**: on each bar line the grid ripples under every live ship (one whose `entity` is
  `Some`) with `BEAT_PULSE_STRENGTH` over `BEAT_PULSE_RADIUS`, on any other beat
  `BEAT_PULSE_FRACTION` (a quarter; zero is bars only) of it — in 5/4 a bar every 1.875 s and four
  lesser beats between. It is the Playing frame's last step, after the wrap, so the ripple lands
  where the ship is drawn; nothing while paused (the frozen grid discards impulses), nothing in
  `GameOver`, nothing on the frame a last life ends the match. Never on the meatieroids: a harness
  test pins their sprite offsets.
- **The pause menu keeps the engine's labels**, and its hint says nothing of the jam: a hint that
  does overflows the engine's 300 px pause panel (insiculous_2d#202), and the text waits on that.
  Only the pause menu jams.
- **No recorded fallback.** A song that cannot play is logged once at `error` ("meatieroids: the
  song could not play: …"), every trigger is refused and counted, and the game is as silent as it
  was before it had music; each match start asks again.
- **Telemetry**: every five seconds a `conductor=on …` line with the game's `shots`, `broken` (the
  three sizes summed, then `large`, `medium` and `small`), `thrusts`, `crashes`, `waves`, `lives`,
  `jam_units`, `pulsed` and `refused`, at `info` when asked — `?telemetry=1` on the web page,
  `MEATIEROIDS_TELEMETRY=1` natively. `init` ends with `meatieroids: init complete`, the line the
  boot measurement stops on.
- **The bank** is `assets/soundfonts/meatieroids.sf2`, FluidR3 GM's timpani (0:47), tuba (0:58),
  trombone (0:57), trumpet (0:56) and orchestra hit (0:55), 2.40 MiB, cut by
  `scripts/prepare_soundfont.sh` — a wrapper over the engine's `scripts/prepare_game_soundfont.sh`
  with `--bank <path>` for a FluidR3_GM.sf2 got any other way; the source's and the output's
  sha256s are in the `LICENSE` beside it. No strings and no horns fit: any of them beside the
  timpani is over the cap, so the brass theme is the tuba's, the trombone's and the trumpet's.
  Budget: **the bank under 4 MiB** (the engine's script refuses to write one over it), the song
  under 256 KiB. On the web every asset, the bank included, is fetched before the first frame.
- The browser bundles moved to **v6** (game) and **v5** (editor) with the music
  (`src/web_entry.rs` names the four places each version lives).

## Work tracking

Open work lives on the **Studio Board** (https://github.com/orgs/beinsiculous/projects/1)
as issues in this repo. **Always pass `-R beinsiculous/asteroids`** — a bare `gh` command
resolves against the session's working directory, which is often the working-set root, so
it lists and files against the wrong repository.

```sh
gh issue list -R beinsiculous/asteroids
gh api repos/beinsiculous/asteroids/milestones --jq '.[] | "\(.title): \(.description)"'
```

Issues are grouped into **sprint milestones**; each description records the batch's
internal order and its gates. Take the next unblocked issue in a sprint, not an arbitrary
one. Claim by assigning yourself; close with `fixes beinsiculous/asteroids#N` in the commit.

**Unfinished work becomes an issue.** Anything you don't finish — work you deferred, debt
you created, a follow-up you spotted — is filed before you report done. Never buried in a
doc, never left as a bare `TODO:`, never dropped. The `file-issue` skill carries the shape;
`sprint-planning` groups issues into shippable batches.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/`.
- Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` BEFORE implementation.
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh`; the `ADV_REVIEWED=1` prefix is allowed only after a code-mode review adjudicated with the user, or when the user explicitly skipped review.
- `review/` holds gitignored transients.
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies — the canonical ones live in the working-set root, not in `insiculous_2d`. Never edit a copy: fix the root's and re-copy, and `scripts/check-skill-parity.sh` there reports any repo that drifted.
