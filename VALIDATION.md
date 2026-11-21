# Validation — Dyson Cube Project slice 1

> **The 0.1.0 ladder, artifacts and e2e below are the retired `.hnmod`'s history.**
> The port is now one flat SDK `.hmod` (mod API 0.2, 0.2.0), whose own ladder is in
> [§ SDK 0.2.0](#sdk-020-the-flat-package-2026-11-18) at the end of this file, and
> whose in-game run does not exist yet (no `horizon-launcher` here). The
> **behaviour** evidence below still applies to `src/runtime.rs`, which the package
> carries byte-identically.

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every build, test and e2e
ran inside a `ports/tools/build-slot.sh` lease; no bare `cargo`, no `--target-dir`,
`CARGO_INCREMENTAL=0`.

## Ladder

| step | command (through the slot) | result |
|---|---|---|
| fmt | `cargo fmt --check` | clean |
| lint | `cargo clippy --all-targets --locked --offline -- -D warnings` | clean |
| unit | `cargo test --locked --offline` | **38 passed** |
| stage | `python3 tools/horizon.py stage --horizon ../_shared/pins/horizon-1d63e0a` | `stage_changed: true` |
| check | `python3 tools/horizon.py check …` | `cargo_exit 0`, **0 warnings** |
| test | `python3 tools/horizon.py test …` | `cargo_exit 0`, **42 passed** (`game::dysoncubeproject`) |
| build | `python3 tools/horizon.py build …` | `cargo_exit 0`, 0 warnings, `fast/horizon-client` |
| package | `tools/package.sh` | `.hmod` + `.hnmod`, hashes below |
| registry check | `cargo horizon-mod check dysoncubeproject.hnmod --kind native-mod` | accepted for 26.3 at the pin |

Test counts: 28 model tests + 5 command-parser tests + 5 sky-scan tests = 38 in
`runtime.rs`; the embedded module adds `support.rs`'s 5 NBT/formation tests → 42.

Red-first evidence: 28 failed on `todo!()`, then 5, then 4. The fourth red was a
**real defect in the port**, not in the tests: `open_to_sky`'s predicate was
`!shell.contains(&pos) && !blocks_motion(pos)`, which makes a shell cell fail `all`
instead of passing over it — so the receiver never pulled (measured in game: `gen
2000 FE/t, con 0 FE/t, buffer 0/100000000 FE`). The fixed predicate
(`shell.contains(&pos) || !blocks_motion(pos)`) is what the e2e now measures at
2,000.0 FE/t.

## Artifacts

| file | size | SHA-256 | entries |
|---|---:|---|---:|
| `content/dysoncubeproject.hmod` | 33,315 B | `44b7ec52095ead0b682393dcb6f479f1e192ada07a5dde1a8e17f4fd0896f1e0` | 52 |
| `dysoncubeproject.hnmod` | 45,951 B | `035fe01e82e1919efc5250fdba0110f03a5cd64f03696720d0c4cfbd2a60da93` | 17 |

- `tools/package.py` run twice gives the same SHA-256 (fixed dates/modes, sorted
  entries).
- The `.hnmod` carries `mixins/dysoncubeproject/data/dysoncubeproject-content.hmod`,
  byte-identical to `content/dysoncubeproject.hmod` (`unzip -p … | sha256sum`), plus
  `LICENSE` and `LICENSE-NOTICE.md` at the bundle root; the `.hmod` carries them at
  `data/dysoncubeproject/license/`.
- `tools/carry_content.sh --check` reports 13 carried entries byte-identical to the
  jar.

## E2E (three legs, `ports/_shared/evidence/dysoncubeproject/`)

The arena is the **Overworld** (the only level whose tick generates sphere power),
a stone platform at y=179 and the controller at (100,180,0), `time set 1000`,
`weather clear`. Every mod command goes through `ctl chat "/…"`.

`e2e-default.log` — PASS + EXIT:0:

| check | measured |
|---|---|
| A wrong shape | a stone at (100,183,0): the controller stayed `minecraft:air`, 0 placeholders |
| B formation | controller placed, **16** `multiblock_structure` placeholders, `formed, box 3x6x3, structure 16/16` |
| C fresh readout | `beams 0, sails 0/0, progress 0%, gen 0, con 0, buffer 0/100000000` |
| D ceilings (at night) | `8,333,334 beams → sails 0/50000000`; `+100,000,000 sails → 50,000,000/50,000,000`; `beams 1 → sails 6/6`, `progress 0%` |
| E rate | 100 sails → `gen 2000 FE/t`; **110 ticks: 17,360 → 237,360 FE = 2000.0 FE/t** |
| E2 sky control | a stone roof at y+5: 251,360 → 251,360; removed: 297,360 → 435,360 |
| F night control | 449,360 → 449,360 at t=18000; back at t=1000: 495,360 → 635,360 |
| G caps | 50,000,000 sails → `gen 1000000000 FE/t`, buffer stops at **100,000,000/100,000,000**, `down 0` |
| H shell break | breaking (101,180,0) left 0 placeholders and no controller |
| I controller break | 15 placeholders → break the controller → **0** left |

`e2e-selftest.log` — FAIL + EXIT:1: `SELFTEST=1` forces the rate to 1.0, and the
run stops at `FAIL: the receiver filled at 1.0 FE/t, not 100 sails * 20`, so the
scenario can fail.

`e2e-embedded.log` — PASS + EXIT:0: launched with **no `--dev-mod`**;
`.horizon-cache/dysoncubeproject-embedded-content.hmod` is 33,315 B and identical to
`content/dysoncubeproject.hmod`; the control leg runs the same binary with
`.horizon-cache` as a plain file and there is **no controller item** and the block
stays `minecraft:air`.

`python3 ports/tools/check-e2e-logs.py dysoncubeproject --strict` → 3/3, every log
ending in `EXIT:`.

## Runner facts that cost time here

- `build-slot.sh e2e --port .` resolves `e2e_abs` to the campaign root and then runs
  `python3 tools/horizon.py` from there. Pass `--port ports/dysoncubeproject` (from
  either cwd) for an e2e; `--port .` is fine for `run`.
- A forwarded e2e needs `/tmp/opencode` on the second machine; the script now makes
  it before `mktemp`.
- `--horizon` must be omitted for a forwarded run (the port's `tools/horizon.py`
  defaults to the pin checkout beside the port). A relative `--horizon` breaks
  because the forwarded runner `cd`s into the port.
- The shared `horizon-ctl` is not on the second machine; the e2e builds the port's
  own `horizon-ctl` when the shared one is missing.
- `expect_status` must print its informational line to stderr: the caller captures
  stdout, and a second copy of the fields makes every `*_of` helper answer twice
  (`"0\n0" != "0"`).
- The controller is walled in by its own shell; step I clears the placeholder that
  faces the player with `setblock` (not a player break, which would take the
  controller with it) so the controller itself can be mined.
- `tools/test_template.py` and `tools/test_stage.py` (the template's own fixture
  checks) build a catalogue-free fixture workspace, so the mixins CLI rightly refuses
  a named-hook plan with `E_HOOK_CATALOG`; the staged ladder (`horizon.py
  stage/check/test/build` against the real catalogue) is the check that applies, so
  those two template fixtures were removed with this note.
- A craft cannot be driven headlessly (the slot click needs a renderer), so the `c:`
  tag correction is not exercised by a craft in the e2e; recorded in `notes.md`.
- `cargo test --lib game::dysoncubeproject` matches both modules; `game::<id>::tests`
  would have matched only one and run 37.
- One `horizon.py test` run was SIGTERMed mid-compile (`cargo_exit: -15`) on the
  shared box during a 5-e2e wave; the retry was green. It was the queue, not the
  port.

## SDK 0.2.0: the flat package (2026-11-18)

Pins: SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` (`package_format` 6),
mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for 26.3. The SDK is cached at
`../_shared/sdk/0b14f8a/` and copied to `.horizon/sdk/` (git-ignored). Every step ran
inside a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` — the runner is the
slot guard; the ladder itself is `cargo`'s, because there is no staging step any more.
`CARGO_INCREMENTAL=0`, no `--target-dir`, no private `BUILD_SLOTS_DIR`.

| step | command (in the port, inside a lease) | result | evidence |
|---|---|---|---|
| runtime only, before any glue | `cargo test` | **38 passed** | quoted in `notes.md` |
| glue red first | `cargo test` with `todo!()` bodies | **6 failed / 40 passed** | `evidence/dysoncubeproject/sdk-test-red-todo.log` |
| unit | `cargo test --offline` | **48 passed**, 0 failed | `sdk-test.log` |
| lint | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `sdk-clippy.log` |
| fmt | `cargo fmt --check` (not `--all`) | exit 0 | `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/dysoncubeproject.hmod` | `sdk-build.log` |
| package again | same command | 63,551 B, same sha256 | `sdk-build-second.log`, `sdk-sha256.txt` |
| check | `cargo horizon-mod check dysoncubeproject.hmod` | exit 0 | `sdk-check.log` |
| fits | `cargo horizon-mod fits dysoncubeproject.hmod --build nightly` | `0b14f8a  made for`, exit 0 | `sdk-fits.log` |
| entries | `unzip -l` | 60 entries, no `Cargo.lock` | `sdk-entries.txt` |
| runtime untouched | `cmp` + `unzip -p … src/runtime.rs \| sha256sum` | `89598f0c…4b1a` three ways | `sdk-runtime-sha256.txt` |
| content carry | `tools/carry_content.sh --check` + sha256 diff | 13 jar entries; 51 files identical | `sdk-content-carry.log` |
| used surface | `unzip -p … uses.json` | 318 entries | `sdk-uses.json`, `sdk-uses-summary.txt` |
| retired move | — | the retired tree, before the first build | `sdk-move-retired.log` |

**Artifact:** `dysoncubeproject.hmod`, 63,551 B, sha256
`063195efbfe9cf23791c1147642aa47eab9c7c23a612146a803cfee90a6c07dc`, 60 entries
(root: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`,
`uses.json`; `src/` ×3; `assets/` ×29; `data/` ×22).

### Not run for 0.2.0: the in-game e2e

*(0.2.1 has one — see [§ 0.2.1](#sdk-021-the-shared-sphere-2026-11-21) below.)*

`tools/e2e.sh` now takes `GAME=<launcher-built horizon-client>` and an optional
`CONTROL_GAME=<a launcher-built game without this package>`; `SELFTEST=1` still flips
the rate assertion so the leg can fail. A native `.hmod` is compiled into the game by
`horizon-launcher native-build . --trust-native-code`, and **`horizon-launcher` is not
installed in this environment**, so no 0.2.0 game exists to run it against. No in-game
claim is made for the package; the 0.1.0 legs above are marked historical in
`../_shared/evidence/dysoncubeproject/HISTORICAL.md`.

## SDK 0.2.1: the shared sphere (2026-11-21)

Pin: SDK nightly `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (`package_format` 6), mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for 26.3. The `.horizon/sdk` copy from
0b14f8a was deleted first and the pin's SDK copied in from `../_shared/sdk/0a875da/`
(trap 17a: a repin does not refresh it). Every step ran inside a
`ports/tools/build-slot.sh` lease (`BUILD_LOCAL=1`, an explicit `cd ports/dysoncubeproject`),
`CARGO_INCREMENTAL=0`, no `--target-dir`, no private `BUILD_SLOTS_DIR`.

| step | command (in the port, inside a lease) | result | evidence |
|---|---|---|---|
| red first | `cargo test --offline` with `todo!()` bodies | **12 failed / 45 passed** | `evidence/dysoncubeproject/slice2-test-red.log` |
| unit | `cargo test --offline` | **57 passed**, 0 failed | `sdk-test.log` |
| lint | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `sdk-clippy.log` |
| fmt | `cargo fmt --check` (not `--all`) | exit 0 | `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | 68,932 B, `dbdc5aa7…` | `sdk-build.log` |
| package again | same command | same sha256 | `sdk-build-second.log`, `sdk-sha256.txt` |
| check | `cargo horizon-mod check dysoncubeproject.hmod` | exit 0, 60 entries | `sdk-check.log` |
| fits (the pin) | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-fits.log` |
| fits (another build) | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits`, exit 0 | `sdk-fits-other.log` |
| entries | `unzip -l` | 60 entries, no `Cargo.lock` | `sdk-entries.txt` |
| gate | `verify-sdk-package.py dysoncubeproject` | **1/1** | quoted in `notes.md` |

**Artifact:** `dysoncubeproject.hmod`, 68,932 B, sha256
`dbdc5aa7bb629c6caabc16b6e476493bf5170c830f13711d25e620f38c9e6b94`, 60 entries, declared
0.2.1.

### The in-game legs (0.2.1)

Built by `ports/tools/launcher-game.sh build dysoncubeproject --port ports/dysoncubeproject`
(`Compiling dysoncubeproject (1 of 1)` in the build log — the manifest had changed, so the
launcher did not reuse an older binary). The packaged `src/lib.rs`, `src/support.rs`,
`src/runtime.rs` and `horizon-mod.toml` are `cmp`-identical to the port's — the game runs the
delivered sources. The launch is `--offscreen` (the crafting phase opens a menu; a headless
session has no renderer), and every mod command goes through `ctl chat "/…"`.

| leg | file | verdict |
|---|---|---|
| the scenario, package present | `evidence/dysoncubeproject-e2e/e2e-launcher.log` | **PASS + EXIT:0** |
| `SELFTEST=1` (rate flipped) | `e2e-launcher-selftest.log` | **FAIL + EXIT:1** at `the receiver filled at 1.0 FE/t` |
| the same scenario, `GAME=$CONTROL_GAME` | `e2e-launcher-control.log` | **FAIL + EXIT:1** at `the beam recipe offered {"item":null,…}` |

`python3 ports/tools/check-e2e-logs.py dysoncubeproject-e2e --strict` → **3/3**, every log
ending in `EXIT:`.

Runner facts that cost time here:

- `build-slot.sh run` forwards `check`/`test`/`build` to the second machine unless
  `BUILD_LOCAL=1`, and a forwarded step runs in the campaign copy — a *local* `cargo fmt`
  only happens with `BUILD_LOCAL=1` **and** an explicit `cd ports/dysoncubeproject`.
- The e2e guard wraps the leg and writes its own log under `evidence/e2e-guard/`; the
  script's own output is the file to keep.
- A shared reservoir makes the *pair* of receivers the unit of a rate assertion (one
  receiver takes the tick's power), and a full buffer cannot show a night/day delta — both
  cost one failed run before the phases were reordered.
- `cargo horizon-mod fits --build 0b14f8a` false-failed on a local type named `Command`;
  the runtime's command enum is `SphereOp` now (see `notes.md`).
