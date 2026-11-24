# Meteor Sizzle

Original AI-authored F-sharp-minor space-rock for Meatieroids, CC0-1.0. No borrowed
melody, ostinato, recording or sample bank. The five-beat riff groups three plus two:
clipped repeated fifths answer falling phrases over a driving octave bass. Explicit
bass waveshaping (drive2.5,contour0,level0.65) supplies distortion before its filtered
sound enters the mix. Punchy drums and high lead answers complete the backing.
Bars1–8 introduce,9–16 expand register,17–24 reduce alternate lead bars and rhythm,
25–32 reprise; fills mark each eight-bar return. This is a distinct hook and
arrangement, not a transposition of another game's score.

160BPM, explicit5/4,480PPQ,32bars. Tempo375000us gives exactly60seconds. The actual
final EngineRoom bass note-off and loaded end_tick are76800; no cue extends it.
Every pitched backing/cue uses F#/A/C#/E, avoiding semitone/tritone clashes in every
section. Background EngineRoom/OrbitRiff/CometAnswer/Kick/Snare/Hat select no cue.
Production BeatPulse, source ownership and gameplay remain unchanged.

`meteor_sizzle.evil` and11original patchJSONs are editable MIDI Evil sources.
**Explicitly running compose.py replaces manual project/patch edits.** The recipe
is composition data, not another exporter or synthesizer.

All nine actions and legacy identifiers remain: timpani47 is a dry triangular shot;
tuba58 is a low filtered saw impact/thrust; trombone57 a middle square impact;
trumpet56 a bright octave triangle impact/reward; hit55 a low crash chord. Large,
medium and small share the24-unit theme cursor. Sample voicing makes large one
octave lower and small one octave higher, preserving pitch classes and map contracts.
Thrust uses a short65tick note at velocity48, versus130tick theme impacts at118.
Shots use120tick positional units at64. Crash/wave chords use400ticks; life is a
short high240tick note. Menu remains a quieter counter-line. Drum is AtSongPosition
and tiles every tick to76800; theme/surge/crash/fanfare/life/jam remain Loop, all Off.
No shot cooldown/cap, thrust onset, source lifetime or event ordering changes.

Render recipe: seed11800 plus instrument index,32000Hz,velocity127,mastergain0.8,
720tick render gates. All44WAVs are finite exact-key samples; no offline resampling, loop points or truncation. Bank zones declare50ms release and
0.42s longest gate (actual maximum400ticks=0.3125s; backing maximum360ticks).
The sample gates last0.5625s plus50ms release:19600frames/0.6125s each. Five triggered
slots (<8),one layer/key (<6),1,732,738-byte bank (<4MiB),14,010-byte MIDI (<256KiB).
Measured peak backing is five voices. Every declared slot and exact key is validated.

## Rebuild and install

From games/asteroids with the usual sibling engine/MIDI Evil layout:

```sh
python3 music-source/compose.py
python3 music-source/build.py /tmp/asteroids-final-a
python3 music-source/build.py /tmp/asteroids-final-b
cmp /tmp/asteroids-final-a/ai_meatieroids.mid /tmp/asteroids-final-b/ai_meatieroids.mid
cmp /tmp/asteroids-final-a/ai_meatieroids.sf2 /tmp/asteroids-final-b/ai_meatieroids.sf2
cp /tmp/asteroids-final-a/ai_meatieroids.mid assets/ai/music/ai_meatieroids.mid
cp /tmp/asteroids-final-a/ai_meatieroids.sf2 assets/soundfonts/ai_meatieroids.sf2
cargo run --manifest-path ../../insiculous_2d/Cargo.toml -p editor --example remap_soundfont -- \
  assets ai/music/ai_meatieroids.mid soundfonts/ai_meatieroids.sf2 \
  'Original AI composition and MIDI Evil synthesis for Be Insiculous, 2026' CC0-1.0
```

Retain the existing sidecar through export. Native SongDocument remapping owns
map/fingerprint/provenance serialization and validates every declared preset/key
and loaded Conductor. Remove the retired bank only after closure validation.
`build.py` and `scripts/prepare_soundfont.sh` are thin shared-helper wrappers;
default output is ignored music-source/rendered/, never runtime assets. No third-party
bank fallback. New bank LICENSE records original CC0 synthesis and its SHA256.

Shared builder pinned to7c8fed0, actual engine HEAD
d6fe15caee2dd1ae9af1fa988d2db8944bfc9fad (test-cache followup). Engine packer/remap/
audition baseline3623e73c04906f6a4d601908c366c1bc5b6b6f5d. MIDI Evil exporter baseline
46ae908; approved baseline3d061afafc64c71266e643271cb25ee88f4e84e8; previous batch
fc2713a6c18986b68f53c96305d3ea455f8dbd4a; actual revision used here
2d7409276f8c4f558fde0a8489dcfb470503c426. Python3.12.3,Rust1.94.0,x86_64Linux,
existing Cargo.lock. Two final clean builds match all47outputs byte-for-byte;
no cross-platform floating-point identity claim. Runtime uses only committed assets.

Historical editor scores require the original bank and LICENSE together, not a
retarget to this synthesis. Pre-migration3f200b212bb59ce91f4ea90bd90dbbeb038b8ba0
retains that closure. Restore separately, for example:

```sh
git worktree add --detach /tmp/meatieroids-historical 3f200b212bb59ce91f4ea90bd90dbbeb038b8ba0
```

Use that worktree's assets in the editor/audition; building the detached game also
needs its usual relative engine layout. AI prefixes/quarantine remain in place.

## Synthetic auditions

```sh
python3 music-source/preview.py
```

The11authored TSVs (idle,sparse and nine actions) are editable read-only inputs.
Only ignored previews/dense.tsv is generated. Each demo requires nonempty finite
0..7.9s timestamps for its8second render and full100ms onset windows; all demos
validate before any render. Idle is intentionally empty and exempt. The script
checks identical action/idle stereoPCM16 formats and lengths before subtraction.

Real engine Conductor with production limiter and unity buses produces121second
idle/sparse/dense.wav and logs, crossing two complete loops. Dense stresses two-player
20Hz shot bursts with recovery gaps and repeated thrust onsets, plus every impact,
crash,reward and menu action. demo-idle and demo-ACTION/isolated-ACTION WAVs/logs
cover all nine actions at final gain. Mix-minus-identical-idle is linear only with
both limiters inactive; PCM16 subtraction adds at most one quantization step.
Metrics use actual TSV timestamps for first100ms cue/backing RMS. These are synthetic
event mixes, not captured gameplay or listening approval; no normalization.

Conservative dense voice estimates: two20Hz shot streams and93.75ms gates plus50ms
release need at most six shot voices. Heavy slot needs two thrust voices plus one
impact; middle one. Bright slot has one impact,three wave,one life andone menu
(six); crash two. All below24. Background voices use separate channels. Required
clipping,limiting,nonfinite,rejected,late,stale and silent counters stay zero.
The closure test isolates each action in a fresh Conductor with music0/sfx1 and
4096stereo frames, asserting finite unclipped PCM and energy>0.0001: a signal check
within~93ms, not proof of perceived audibility. Impact/reward versus shot/thrust
levels are reviewable in the isolated metrics and recordings.

Local bundle paths verified from src/web_entry.rs: game meatieroids v6 and editor
asteroids v5. No deployment, synchronization or version changes in this batch.

## Runtime SHA256

- `ai/music/ai_meatieroids.mid`: `8de80ef59aa1a98b0e7faaf23ef58e04169d39fca77d65b89c5b1e820a873699`
- `soundfonts/ai_meatieroids.sf2`: `fbf1ac84d6da60e92adeae52c6f0a683804a7a5fce177a5b478b156510786fba`
