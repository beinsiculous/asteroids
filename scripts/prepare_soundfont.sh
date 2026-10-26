#!/usr/bin/env bash
# Build Meatieroids' bank, assets/soundfonts/meatieroids.sf2, and the LICENSE beside it,
# through the engine's scripts/prepare_game_soundfont.sh: the FluidR3 GM presets Mars
# plays. The subset is committed; this script is how anyone rebuilds it byte for byte.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   The engine's script documents the fetch, the sha256 check and the 4 MiB cap.
#
# The presets, as bank:program: timpani (the ostinato, a shot's drum), tuba (the G pedal,
# the brass theme, a large meatieroid broken, the thrust's surge), trombone (a medium
# meatieroid broken), trumpet (the calls, a small meatieroid broken, the wave's fanfare,
# an extra life, the jam), orchestra hit (a ship lost). No strings and no horns: any of
# them beside the timpani is over 4 MiB.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"

exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" meatieroids 0:47 0:58 0:57 0:56 0:55
