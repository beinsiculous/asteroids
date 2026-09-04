# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # run the 42 headless tests (34 in src/gameplay_tests.rs, 4 in achievements.rs, 4 in spawning.rs)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` cargo feature). `glam` is the only other dependency.

## Architecture

This is a single-crate game (`insiculous_asteroids`) built on the in-house `insiculous_2d` ECS engine. `AsteroidsGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs` — `init()` loads the font, registers achievements, and spawns only the background + deforming grid (ships and rocks spawn fresh in `start_game()` because ship count depends on the picked mode). With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`; no game code changes between the two modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()`: `TitleScreen { selection }`, `ModeSelect { selection }`, and `Achievements` dispatch to input handlers in `menu.rs`; `Playing` and `GameOver` fall through to `update_gameplay()` in `gameplay/mod.rs`, which orchestrates the per-frame steps implemented across `gameplay/{ship,rocks,flow}.rs`. Flow is Title → ModeSelect (chaos pick, then `start_game`) → Playing ↔ GameOver (endless; the only ending is running out of ships). Match lifecycle (start, finish, reset-to-title, scoring/extra lives, theme application) lives in `gameplay/flow.rs`.

**Rendering is wireframe line art, not sprites.** The ship hull, thrust flame, and hash-jittered rock polygons are pushed into `ctx.lines` every frame by `emit_wireframes()` (`wireframe.rs` — same line-list pipeline as the deforming grid; grid first, wireframes on top). The ONLY gameplay sprites are the bullets (tiny emissive squares) and the background quad. Rock outlines are generated once at spawn (`asteroid_polygon(seed, radius)`) and stored on the `Asteroid` struct — never re-hashed per frame. Local space puts the ship's nose along +x, matching `Transform2D::forward()`.

**The game steps physics itself.** `AsteroidsGame` owns a `PhysicsSystem` (`PhysicsConfig::space()` — zero gravity) and calls `self.physics.update(ctx.world, ctx.delta_time)` inside `update_gameplay()`, followed by `LifetimeSystem` (expires bullets) and a single `take_collision_events()` drain shared by all consumers. Body recipes (spawning.rs):
- **Ship**: dynamic, free rotation, all damping zero, **sensor** circle collider (radius 10 < drawn hull 16) — rocks must never physically deflect a ship, and co-op ships pass through each other. Flight model is the pure `integrate_ship` in gameplay/ship.rs: read rapier velocity + heading, integrate thrust/drag/clamp, push back with ONE `set_velocity` per frame.
- **Rocks**: solid dynamic bodies, restitution 1.0 / friction 0.0 — they bounce elastically off each other. Circle collider = nominal radius × `ASTEROID_COLLIDER_SCALE` (0.9).
- **Bullets**: dynamic sensors with a `Lifetime` (1.1 s ≈ under one screen crossing); a bullet expiring via lifetime counts as a miss and resets that ship's sharpshooter streak (`prune_expired_bullets`).

**Two hit paths, deliberately different.** Bullet-vs-rock uses collision events through the pure `resolve_hit_pairs` (each bullet and rock used at most once per frame; ships are structurally excluded from the candidate set, so friendly fire is impossible). Ship-vs-rock is a game-side circle test in `check_ship_hit` — NOT events — because a rock that drifted over a ship during i-frames must still kill the instant mercy expires, which a stale `started` event can't do.

**Determinism convention: no RNG.** Every random-looking decision (rock shapes, speeds, spins, spawn points, split jitter) draws from `hash_f32` over a monotonic seed stream (`next_seed()` / `spawn_counter`). Wave rocks spawn on the window border via `spawn_position`, which walks 8 evenly spaced perimeter candidates from a hashed start and takes the first that clears `SAFE_SPAWN_DIST` (160 px) from every live ship, else the roomiest.

**Screen wrap** (`wrap_entities` / pure `wrap_position`): toroidal field with a per-entity margin so entities fully exit before re-entering. Writing `Transform2D.position` on a live dynamic body is a rapier teleport that preserves velocity (GPP-09) — proven by a sim test. Wrapping also runs during GameOver so leftover rocks keep circling behind the overlay.

**2-player co-op** (`GameMode::TwoPlayerCoop`): two ships share one rock field but keep per-ship lives, scores, bullets, streaks, and i-frames (`ShipState`; `entity: Option<EntityId>` becomes `None` when that ship is out — the survivor plays on; the match ends only when `coop_over` says every ship is done). Input goes through `ctx.players`: co-op ship `index` reads `PlayerId(index)`; single player merges P1+P2 (either keyboard cluster or pad flies the lone ship). Turn is `-move_x`, thrust is `move_y > 0` or MoveUp, fire is a fresh Action1. P2's hull/flame is `SHIP2_COLOR` green; P1 wears the chaos accent.

**Editor naming:** every spawned entity gets a `Name` component ("Ship", "Bullet", "Asteroid Large #7") so the editor hierarchy is readable — keep this when adding entities.

**Coordinate and scale conventions (the main trap):**
- World origin is screen center; window is 800×600 (`WIN_W`/`WIN_H`).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — that's why the bullet sprite scale is `BULLET_SIZE_PX / RENDER_UNIT`.
- Collider shapes use **absolute pixels** and ignore `Transform2D.scale` entirely. Sprites/wireframes and colliders are sized through different paths, so they can silently diverge — `F1` in-game (or `C` in the editor) overlays magenta collider outlines to check. Note the deliberate mismatches: ship collider (10) < drawn hull (16), rock collider = 0.9 × polygon mean radius.

**All tuning lives in `src/constants.rs`** (sizes, speeds, split/wave tables, achievement thresholds, grid impulses) and all entity creation lives in `src/spawning.rs`, spawned from those constants. Asteroid size tables (`ASTEROID_RADII/SCORES/SPEED_RANGES`) are indexed by `AsteroidSize::idx()` — keep index order Large, Medium, Small.

**Chaos modes** (engine `ChaosMode`, picked on the ModeSelect screen and mirrored into `ctx.chaos_mode`): Insane = rock speeds ×1.6 and respawn i-frames halved; Ridiculous = large rocks split into 3 (not 2) and +1 rock per wave; Insiculous = both (`is_insane()`/`is_ridiculous()` both fire). `ChaosTheme::for_mode` colors the background/grid/wireframes; `apply_theme` refreshes live scenery on match start.

**Achievements** (11, defined in `achievements.rs`, registered in `init()`): wave-5 per mode ×4, score tiers 10k/30k, and skill (Sharpshooter 10-hit streak, Close Call ≤60 px, Untouchable 3 deathless waves, Double Tap 2 kills ≤0.5 s, Living on the Edge = wave 5 on the last total life). `DISPLAY_SECTIONS` drives the achievements page; a test enforces it covers every registered id. Extra life every 10,000 points, per ship.

**Pause** is the engine `PauseMenu`, gated to `GameState::Playing` only. While paused the whole update is frozen and the wireframes + grid are re-emitted (without advancing) so the frozen scene stays visible under the overlay.

**Paths:** `engine_core::game_root!()` anchors everything, so `cargo run` works from any cwd. Assets in `assets/` (just `fonts/font.ttf` today); achievements persist to `saves/asteroids_achievements.json`, input bindings to `saves/input_settings.json`.

**Tests** are fully headless: `src/gameplay_tests.rs` (34 — pure rules plus rapier sim tests using the exact spawn recipes) + inline mods in `achievements.rs` (4) and `spawning.rs` (4). Pure functions (`integrate_ship`, `wrap_position`, `split_spec`, `split_velocity`, `resolve_hit_pairs`, `spawn_position`, `coop_over`, …) exist precisely so behavior can be pinned without a window — keep new rules in that shape.

## The Deion Re-skin (Phase G): Meatieroids

The game today is the neon geometry-wars look; Phase G of the Deion pivot re-skins it as **Meatieroids**:

- Asteroids become **roided-out flexing meatballs** in the same 3 size classes; splitting a big one is a flex-burst into smaller, angrier meatballs. The **angry meatball** is a recurring cross-game character (hazard in Snake's "Hot Dog!").
- The ship stays the **Flying Funk**, Deion piloting, with the name painted on the hull as an easter egg. P2's co-op ship is Cubert's (name TBD by Jesse). Shots become **icicle spikes** — Deion's universal projectile. The UFO (when added) is Dr. Maxwell's cake saucer.
- Style SSOT: `deion_assets/DEION_STYLE.md` via the root symlink (the symlink assumes the standard side-by-side checkout — the same requirement the Cargo path dep already imposes). Settled metrics: 16 px base cell, nearest filtering, 5× integer scale to `RENDER_UNIT = 80`, one art cell = one world unit; never fake a footprint via `Transform2D.scale` (physics ignores it).
- Runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/` (F2, not yet built) — never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) — tiered ship rule (DEION_STYLE.md §6, Aug 19 2026): may ship in FREE web builds, never in paid/marketplace builds; `deion_assets/scripts/check_no_ai_assets.sh` must pass on any paid release's asset tree. Sheet clip names are the stable API between art and code.

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
