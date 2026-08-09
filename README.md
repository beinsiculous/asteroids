# Insiculous Asteroids

A neon-wireframe take on Asteroids, built on the [insiculous_2d](../../insiculous_2d) engine (game 5 of the 20 Games Challenge). The ship and rocks are glowing vector line art over a deforming spring-mass grid that ripples with every explosion; bullets and particle bursts feed the engine's HDR bloom. Endless waves, elastic rock-on-rock physics, screen wrap, four chaos modes, 11 achievements, and full 2-player co-op.

## Running

Requires the engine checked out side by side (`../../insiculous_2d` — the Cargo path dependency and the `deion_assets` symlink both assume it).

```bash
cargo run                     # play
cargo run --features editor   # play inside the engine's scene editor
cargo test                    # 42 headless tests
```

## Controls

| Action | P1 keyboard | P2 keyboard | Gamepad (per player) |
|---|---|---|---|
| Rotate | A / D | ← / → | Left stick / d-pad X |
| Thrust | W (or stick up) | ↑ (or stick up) | Stick/d-pad up |
| Fire | Space | Enter | A |
| Pause | Esc | Esc | Start |
| Menus | W/S + confirm | ↑/↓ + confirm | D-pad + A |

In single player the lone ship listens to *both* control clusters and both pads. In co-op, ship 1 = P1 bindings, ship 2 = P2 bindings (green ship). Bindings persist to `saves/input_settings.json` (hand-editable JSON). `F1` toggles the collider debug overlay.

## Mechanics

- **Waves**: start at 4 large rocks, +1 per wave (cap 9), spawned on the screen border never closer than 160 px to any ship. 2-second breather between waves.
- **Splitting**: Large → 2 Medium → 2 Small → gone. Scores 20 / 50 / 100 (small rocks are the hardest targets, so they pay the most). Children fan out faster than the parent.
- **Flight**: thrust + drag + speed clamp, classic drift. Max 4 live bullets per ship, bullets expire after ~one screen crossing (an expired bullet is a "miss" for streak purposes).
- **Lives**: 3 per ship, +1 every 10,000 points (per ship). Respawn grants 3 s of blinking invincibility. In co-op each ship keeps its own lives and score; the match ends only when everyone is out.
- **Chaos modes**: Normal — the classic belt. Insane — rocks 60% faster, respawn mercy halved. Ridiculous — large rocks split into *three* and every wave brings an extra rock. Insiculous — all of the above.
- **Achievements** (11): wave-5 milestone per chaos mode, 10k/30k score tiers, Sharpshooter (10-hit streak), Close Call (kill within 60 px), Untouchable (3 deathless waves), Double Tap (2 kills in 0.5 s), Living on the Edge (wave 5 on your last ship). Saved to `saves/asteroids_achievements.json`.

## The Deion Pivot: Meatieroids

When Phase G of the Deion re-skin reaches this game, the belt gets a protein upgrade: **Meatieroids**. The asteroids become **roided-out flexing meatballs** — three sizes, all of them mid-workout, all of them furious. Shooting a big one doesn't so much shatter it as trigger a *flex-burst*: it pops into smaller, even angrier meatballs. The angry meatball is a recurring villain across the whole Deion universe — it also shows up as a hazard in Snake's "Hot Dog!" and as a patty-rank enemy in "Burger Invaders".

The player ship is the **Flying Funk**, with Deion himself at the stick and the name painted on the hull (easter egg — squint at the sprite). Player 2 flies Cubert's ship. Bullets become **icicle spikes**, Deion's universal projectile. And if a UFO ever menaces the belt, it's Dr. Maxwell's cake saucer.

Art follows the style SSOT at `deion_assets/DEION_STYLE.md` (via the repo-root symlink): 16 px base cells, nearest filtering, 5× integer scale so one art cell = one world unit at `RENDER_UNIT = 80`. Runtime sprites only ever arrive through the deion_assets sync into `assets/sprites/` — and AI-generated stand-in art never ships.

### Open questions

- What is Cubert's ship called? (Jesse names it.)
- Drawn 16-angle rotation set vs. free runtime rotation for the meatballs and the Flying Funk — decide at re-skin time.
- Meatball 'roid-rage behaviors: do small ones charge the ship? Flex taunts on split?

Answered questions move up into the theme spec above and get DELETED from this list (live-docs convention).
