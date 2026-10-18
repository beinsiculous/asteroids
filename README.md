# Meatieroids

Asteroids, re-skinned for the Deion universe and built on the [insiculous_2d](../../insiculous_2d) engine (game 5 of the 20 Games Challenge). Deion flies the **Flying Funk** — a gravy boat seen from above, drawn in sixteen headings — through a belt of **meatieroids**: roided-out flexing meatballs that tumble, bounce off each other, and burst into smaller, angrier meatballs that come out flexing. Under the stars, the engine's deforming grid ripples with every burst and every crash. Endless waves, screen wrap, four chaos modes, 11 achievements, and full 2-player co-op. The site names it Meatieroids, at `/games/meatieroids/`; `/games/asteroids/` redirects there.

## Running

Requires the engine checked out side by side (`../../insiculous_2d` — the Cargo path dependency and the `deion_assets` symlink both assume it).

```bash
cargo run                     # play (the window is a fixed 800 x 600)
cargo run --features editor   # play inside the engine's scene editor
cargo test                    # 65 headless tests
```

The same build runs in the browser at [beinsiculous.com/playground/meatieroids/](https://beinsiculous.com/playground/meatieroids/): the game inside the editor, layout only — the rules are compiled in and nothing you change there persists.

## Controls

| Action | P1 keyboard | P2 keyboard | Gamepad (per player) |
|---|---|---|---|
| Rotate | A / D | ← / → | Left stick / d-pad X |
| Thrust | W (or stick up) | ↑ (or stick up) | Stick/d-pad up |
| Fire | Space | Enter | A |
| Pause | Esc | Esc | Start |
| Menus | W/S + confirm | ↑/↓ + confirm | D-pad + A |

In single player the lone ship listens to *both* control clusters and both pads. In co-op, ship 1 = P1 bindings (Deion's Funk, icicles), ship 2 = P2 bindings (the butter-and-berry Funk, ice chips). Bindings persist to `saves/input_settings.json` (hand-editable JSON). `F1` toggles the collider debug overlay.

## Mechanics

- **Waves**: start at 4 large meatieroids, +1 per wave (cap 9), spawned on the screen border never closer than 160 px to any ship. 2-second breather between waves.
- **Splitting**: Large → 2 Medium → 2 Small → gone. Scores 20 / 50 / 100 (small ones are the hardest targets, so they pay the most). Children are born flexing, fanned out and faster than the parent.
- **Flight**: the ship turns smoothly and is drawn on the nearest of sixteen headings; thrust and shots follow the drawn heading. Thrust + drag + speed clamp, classic drift. Max 4 live shots per ship, shots expire after ~one screen crossing (an expired shot is a "miss" for streak purposes).
- **Hits**: a meatieroid collides as its round body; its arms and fists are drawn, not collided.
- **Lives**: 3 per ship, +1 every 10,000 points (per ship). A lost life plays the Funk's crash; the respawn grants 3 s of blinking invincibility. In co-op each ship keeps its own lives and score; the match ends only when everyone is out.
- **Chaos modes**: Normal — the classic belt. Insane — meatballs 60% faster, respawn mercy halved. Ridiculous — large ones burst into *three* and every wave brings an extra one. Insiculous — all of the above.
- **Achievements** (11): wave-5 milestone per chaos mode, 10k/30k score tiers, Sharpshooter (10-hit streak), Close Call (a bust within 60 px), Untouchable (3 deathless waves), Double Tap (2 busts in 0.5 s), Living on the Edge (wave 5 on your last ship). Saved to `saves/asteroids_achievements.json`.

## The art

The meatieroids, both Funks, the shots and the star tile are drawn in Aseprite in `deion_assets` (style SSOT `deion_assets/DEION_STYLE.md`, via the repo-root symlink) and arrive only through the sync into `assets/sprites/` (`assets/sprites/sync.list` pins the commit). Everything ships at 1×, pixel-snapped: one art pixel is one world pixel. The art is AI-drawn and quarantined (`ai_` prefix): it may ship in free builds, never in paid ones, until a person's cleanup pass.

The hull's "FLYING FUNK" lettering lives on the side-view casting; at gameplay size the Funk is too small to carry it. Dr. Maxwell's cake saucer is the next batch.

### Open questions

- Meatball 'roid-rage behaviors: do small ones charge the ship? Flex taunts beyond the split?

Answered questions move up into the theme spec above and get DELETED from this list (live-docs convention).
