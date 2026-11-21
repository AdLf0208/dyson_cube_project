# Dyson Cube Project → Horizon — slice 1: the ray receiver and its multiblock

Ported from **Dyson Cube Project 1.0.5** (`InnovativeOnlineIndustries/Dyson-Cube-Project`),
the exact ATM10 jar `dysoncubeproject-1.0.5.jar` (759,076 B unpacked, 171 entries,
`ports/_shared/licence-cache/dysoncubeproject-1.0.5.jar`). Behaviour was read from
the upstream repository at commit **`13c01e0`** (analysis only; this is a clean-room
Rust rewrite). What was read and every number extracted: [`FINDINGS.md`](FINDINGS.md).

**Licence: MIT → rank 0 (permissive, publishable).** Read from the jar's own
`META-INF/neoforge.mods.toml` (`license = "MIT"`), its `gradle.properties`
(`mod_license=MIT`) and its `LICENSE` ("Copyright (c) 2025 Innovative Online
Industries"). The campaign still keeps every port local-only until the maintainer
approves publishing (`ARCHITECTURE.md` §8). Ported now because it is a P0
`content`/`deep` **core** mod at licence rank 0 with no unported dependency.

**Class + tier:** `content` · **deep** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). Approach per the pin's
`docs/modpacks.md` ("Tiers (the API a port needs)"): `deep` normally means the
megastructure and a custom renderer; the honest slice is the **multiblock formation
and one controller's energy loop** with the jar's own numbers — machines-shaped code
(block entity, API 142 buffer, a server tick) plus the real space check, not the
whole sphere and not the client half.

## Slice 1 scope

| in | what |
|---|---|
| `dysoncubeproject:ray_receiver_controller` | the machine: a 3×6×3 box, a sphere, a 100,000,000 FE buffer, 50,000,000 FE/t in and out |
| `dysoncubeproject:multiblock_structure` | the sixteen placeholders the controller writes around itself (`"item": false`: the jar has no item model for it) |
| `dysoncubeproject:beam`, `beam_package`, `solar_sail`, `solar_sail_package` | the mod's components, with their own recipes |
| `/dysoncubeproject status\|set\|add` | upstream's four operator forms (level 4) plus the GUI's readout |

Everything else is a recorded gap (below): the EM Rail Ejector (the second
controller, and the only way to grow a sphere), the shared per-player sphere,
the client half, the advancements.

## Behaviour (upstream → this port)

| aspect | upstream | this port |
|---|---|---|
| placement | `DefaultMultiblockControllerBlock.getStateForPlacement` → `validateSpace`, which needs the whole 3×6×3 box air, the controller's own cell included | `state_for_placement` (API 76) refuses the placement: no block, no item spent |
| formation | `RayReceiverControllerBlock.setPlacedBy` writes 16 placeholders (the four sides, four above, the 3×3 ring at y+2; upstream writes 17 and repeats one) | the same 16 positions, from `structure_positions` in `runtime.rs`, written by `place_block` (API 14) |
| teardown | a broken placeholder calls `world.destroyBlock(controllerPos, true)`; a broken controller clears its placeholders | `destroy_block` (API 14): `native_machine_destroy_block` drops the controller; `clear_placeholders` empties the shell |
| sphere | `DysonSphereStructure`: `maxSolarPanels = min(50,000,000, beams*6)`, `maxBeams = ceil(50,000,000/6)`, `progress = sails/50,000,000`, `generatePower` refills the reservoir to one tick of `sails*20`, `extractPower(amount) = min(amount, stored)` | the same model in `runtime::Sphere`, tested against those formulas |
| receiver | `RayReceiverBlockEntity.serverTick`: when day, not raining and it can see the sky, pull `min(50,000,000, room)` from the sphere into the buffer; then offer `min(50,000,000, held)` to the block below (simulate, then commit) | `runtime::tick` + `Buffer::offer_down`/`take_down`; the world half is `native_energy_insert(below, UP, …, simulate)` (API 142) |
| capability | `Capabilities.EnergyStorage.BLOCK` on `Direction.DOWN` only, `receiveEnergy` 0, `extractEnergy` MAX | `native_block_entity_energy`: capacity 100,000,000, `max_insert: 0`, `max_extract: 50,000,000`, extract on the down face only |
| readout | `DysonProgressGuiAddon`: progress, power gen, power con, beams, sails, `Needs more beams` | `/dysoncubeproject status` prints those, plus the buffer and the shell's state (the GUI is a gap) |
| commands | `/dysoncubeproject set|add beams|panels <sphereId> <n>`, permission level 4, clamped to the sphere's maxima | the same four forms at level 4, addressed by the controller's position, clamped the same way |

## Hooks

| Hook (API) | Helper | What it does |
|---|---|---|
| `state_for_placement` (76) | `state_for_placement.rs` | Refuses the controller where its 3×6×3 box is not empty. |
| `place_block` (14) | `place_block.rs` | Writes the sixteen placeholders and their controller position. |
| `destroy_block` (14) | `destroy_block.rs` | A broken placeholder drops the controller; a broken controller clears the shell. |
| `block_entity` (15) | `block_entity.rs` | The controller and the placeholders get a block entity. |
| `block_entity_ticks` (15) | `ticks.rs` | Only the controller ticks. |
| `block_entity_energy` (142) | `energy.rs` | The receiver's 100,000,000 FE buffer, given only through its down face. |
| `block_entity_tick` (17) | `tick.rs` | Generate → pull → push down, then save the sphere and the buffer. |
| `register_commands` (8) | `register.rs` | Five usages at level 4. |
| `perform_command` (8) | `perform.rs` | The command body (status / set / add). |
| `content_root` (AE2 pattern) | `embedded_content.rs` | Writes the embedded `.hmod` and appends it to the dev list. |

10 injections, all stamped at the pin (`native-hooks.json`); no pending fragment is
named. Two pin facts shaped the design: API 142's amount is the int `energy` in the
block entity's saved compound, and `native_block` lives on `Game`, not on the
`GameLevel` the placement hook receives (`state_for_placement` reads blocks through
`level.world` + `level.facts`).

## Content

`content/` is carried from the jar: 2 blocks + 4 items declared in
`data/dysoncubeproject/horizon/`, their blockstates/models/items JSON, the jar's
own textures (3 block, 4 item) and all six lang files (byte-identical:
`tools/carry_content.sh --check`), 5 recipes, 1 loot table (singular `condition`),
the pickaxe tag, a creative tab, and the six `c:` convention tags the recipes need.
`cargo-horizon-mod build` reports `no code: content only`; 52 files, 33,315 B.

**The recipes' convention tags.** The jar's recipes key on NeoForge's `c:`
convention tags (`#c:storage_blocks/iron`, `#c:nuggets/iron`, `#c:storage_blocks/copper`,
`#c:ingots/copper`, `#c:glass_panes/colorless`, `#c:gems/lapis`). No pack at 26.3
declares them — Horizon ships no NeoForge data — so every recipe those keys name can
never match (an unknown tag is an empty ingredient, `docs/mods.md`). The port declares
the six tags in `data/c/tags/item/…`, mapping each to the vanilla item NeoForge's tag
contains, so the five recipes can match. **Limitation:** a craft cannot be driven in
this campaign's headless e2e (the slot-click path needs a renderer), so the fix is
proven by the declared data and `cargo horizon-mod check`, not by a craft in game.
*(Corrected in slice 2: a craft **can** be driven — `--offscreen` gives the session
a renderer, and all five recipes are crafted in the 0.2.1 e2e. See "Slice 2" below.)*

Two content decisions, both recorded:
- the jar's **block models are empty** (`{"textures":{"particle":…}}`, no elements):
  every block is drawn by a block-entity renderer, which this slice does not port.
  The port gives the controller and the placeholder a plain cube with the jar's own
  texture so the machine is visible and targetable.
- `multiblock_structure` has **no item model** in the jar and is registered with a
  `null` creative tab, so it is declared `"item": false` (the brief said "7 items";
  the artefact has 6 items with models).

## Upstream defects fixed

**The receiver's sky check could never open.** Upstream asks
`level.canSeeSky(pos.above())` (`block/tile/RayReceiverBlockEntity.java:77`) while
`setPlacedBy` writes a `multiblock_structure` placeholder into exactly that cell
(`block/RayReceiverControllerBlock.java:91`, `pos.above()`), and those placeholders
collide as the controller's own shape split per cell
(`MultiblockStructureBlock.getCollisionShape` → the `RayReceiverControllerBlock.SHAPE`
intersected with the cell, a full cube at `dy = 1`). Vanilla's
`Level.canSeeSky(pos)` is `getHeight(MOTION_BLOCKING) <= pos.getY()`, so a cell that
holds a motion-blocking block is never sky-visible: with its own shell standing in
that cell, the machine is blind for ever.

- **Evidence (measurement, before the fix):** the first in-game run of this port
  printed, on every read, `beams 17, sails 100/102, gen 2000 FE/t, con 0 FE/t,
  stored 2000, buffer 0/100000000 FE` — the sphere generated, the receiver never
  pulled. The pin's own `can_see_sky` idiom (`game/stats.rs:459`,
  `game/advancements.rs:196`) agrees: it scans strictly above the cell, and the
  shell occupies `y+2..y+4` in the same column.
- **Fix (minimal):** the column above the machine is scanned with the machine's
  **own shell skipped** — `runtime::open_to_sky(blocks_motion, anchor, top)`; a
  block that is not the shell still shadows the machine. The rule is unit-tested
  (`the_machines_own_shell_does_not_shadow_it`, `a_roof_above_the_shell_stops_the_sky`,
  `the_literal_upstream_check_could_never_open`) and proven in game twice: the
  machine fills at 2,000 FE/t under open sky, and a **stone roof at `y+5`** stops it
  until it is removed.
- **What a user notices:** the ray receiver produces power at all. Without the fix
  the block is a decorative, permanently idle machine.
- **Upstream report:** worth filing (one line in `serverTick`: check the sky above
  the *shell*, or make the placeholders non-blocking for the check). Not filed —
  the campaign files reports only when the orchestrator asks.

## Deviations (deliberate, not defects)

- The sphere lives in the controller's block entity, not in level saved data keyed
  by the player's UUID: the pin has **no level saved-data API**. One sphere per
  controller; the commands address the controller's position, not a sphere id.
- `Level.isDay()` is `day_time % 24000 < 12000` and the Overworld only (upstream's
  own level-tick guard); `isRaining()` is `native_weather().0 > 0.2` (vanilla's
  threshold). Both are the closest reads at API 97.
- `generatePower` runs inside the controller's tick rather than at level-tick-pre.
  With one sphere per controller the two orders are identical.
- The two blocks are drawn as plain cubes with the jar's textures (no renderer).
- The placeholders are `occludes: false` (the jar's `noOcclusion()`), a full cube
  for collision like the jar's split shape.

## Gaps (slice 2 and later)

- **EM Rail Ejector**: its multiblock, its 400,000 FE buffer, 40 FE per sent item,
  and the beam/sail ejection that is the only way to grow a sphere.
- **The shared sphere**: `DysonSphereProgressSavedData`, subscribe/unsubscribe, one
  sphere for several receivers, the `DysonSphereSyncMessage` packet.
- **The client half**: `RayReceiverRender`, `EMRailEjectorRender`, `HologramRender`,
  `SkyRender`, the two GUI addons, the five shaders, the two sounds.
- **The component items' payloads** (`dysoncubeproject:solar_sail` /
  `dysoncubeproject:beam` data components) — only the ejector reads them, and no
  component-registration API is used by this slice.
- **Advancements**: the root and the six sphere-percentage milestones.
- The ejector block/item/recipe/loot.

## Upstream files read (analysis only)

`block/DefaultMultiblockControllerBlock.java`, `block/RayReceiverControllerBlock.java`,
`block/MultiblockStructureBlock.java`, `block/tile/RayReceiverBlockEntity.java`,
`block/tile/MultiblockStructureBlockEntity.java`, `block/EMRailEjectorControllerBlock.java`,
`block/tile/EMRailEjectorBlockEntity.java`, `multiblock/MultiblockStructure.java`,
`world/DysonSphereStructure.java`, `world/DysonSphereProgressSavedData.java`,
`Config.java`, `DysonCubeProject.java`, `DCPContent.java`, `DCPAttachments.java`,
`item/DysonComponentItem.java`, `util/NumberUtils.java`,
`client/gui/DysonProgressGuiAddon.java`, `client/gui/SubscribeDysonGuiAddon.java`,
`client/gui/UnsubscribeDysonGuiAddon.java`, `client/tile/RayReceiverRender.java`,
`client/render/HologramRender.java`, `client/render/SkyRender.java`. Repo clone
deleted after the findings were recorded (the jar stays as evidence).

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

Ladder at the pin (`1d63e0a5`, mixins `5a64af93`), all green: `cargo fmt --check`
clean; `cargo clippy --all-targets -- -D warnings` clean; **38 unit tests**
(standalone crate); `stage`/`check`/`test`/`build` `cargo_exit 0`, **0 warnings**,
**42 embedded tests** (37 `game::dysoncubeproject::` + 5 `game::dysoncubeproject_state::`).
Red first, three times: 28 model tests failed on `todo!()`, then the 5 command
tests, then the 4 sky tests — and the sky red caught a real inverted predicate
(a shell cell failed `all` instead of being skipped), which is why the in-game rate
was 0 until it was fixed.

Artifacts in the port: `content/dysoncubeproject.hmod` **33,315 B**
`44b7ec52…f1e0`, 52 files, `no code`; `dysoncubeproject.hnmod` **45,951 B**
`035fe01e…da93`, 17 entries, reproducible on a second `tools/package.py` run, with
the built `.hmod` byte-identical inside it and `LICENSE`/`LICENSE-NOTICE.md` at the
bundle root. `cargo horizon-mod check dysoncubeproject.hnmod --kind native-mod`
accepts it for 26.3 at the pin.

In game (Overworld sky arena at y=180, headless), three legs under
`ports/_shared/evidence/dysoncubeproject/`: `e2e-default.log` **PASS + EXIT:0** — a
blocked box refused the placement and left no placeholder, the clear box formed with
**16** placeholders, the readout showed an empty sphere, `set beams 8333334` gave
50,000,000 sails and `add panels 100000000` clamped to that ceiling, one beam
supported six sails, 100 sails filled the buffer at **2,000.0 FE/t over 110 ticks**,
a **stone roof at y+5** stopped the harvest and removing it resumed it, **night**
stopped it and day resumed it, 50,000,000 sails generated 1,000,000,000 FE/t into a
buffer that stopped at **100,000,000 FE** with nothing leaving (no FE sink), a broken
placeholder took the controller with it, and a broken controller left **0**
placeholders; `e2e-selftest.log` **FAIL + EXIT:1** (the rate assertion flipped);
`e2e-embedded.log` **PASS + EXIT:0** with **no `--dev-mod`**, the bundle writing its
content to `.horizon-cache/` byte-identically, and the same binary with
`.horizon-cache` unusable has no controller item and the block stays
`minecraft:air`. `check-e2e-logs.py --strict` 3/3; `check-skills.py --notes` PASS.
Details and the runner traps: [`VALIDATION.md`](VALIDATION.md).

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) **retired `.hnmod`**: one flat
`.hmod` now carries a mod's code *and* its content. **0.2.0 is this port's first
publication** — there is no closed `.hnmod` PR of ours to supersede — and it is the
same slice as 0.1.0, republished as `api = "0.2"`. Only the glue, the manifest and
the packaging were rewritten: **`src/runtime.rs` is byte-identical** to the retired
bundle's copy (`89598f0c…4b1a`, 30,071 B, proved in three places: retired file,
`src/runtime.rs` and `unzip -p dysoncubeproject.hmod src/runtime.rs`).

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, `package_format` 6, for **26.3**.
The SDK is cached in-tree at `../_shared/sdk/0b14f8a/` and copied into
`.horizon/sdk/` (git-ignored, never packed) for offline builds.

**A discrepancy in the record.** A campaign note claimed this port had already been
migrated; the disk said otherwise — no `horizon-mod.toml`, and `mixins/`, the
`.hnmod`, `content/`, `compatibility.json` and `rust-toolchain.toml` all present. The
artefact wins: this section is the migration. The retired tree is preserved, not
deleted, at `../_shared/evidence/dysoncubeproject/retired-hnmod-0.1.0/`.

### What moved, and where the retired tree is

| 0.1.0 path | Now | Kept at |
|---|---|---|
| `mixins/dysoncubeproject/runtime.rs` | `src/runtime.rs`, `cp` then `cmp`-checked | `retired-hnmod-0.1.0/mixins/dysoncubeproject/runtime.rs` |
| `mixins/dysoncubeproject/support.rs` + 10 hook helpers | `src/support.rs` (glue) + `src/lib.rs` (the one `Hooks`, the Parts) | `retired-hnmod-0.1.0/mixins/` |
| `mixins/dysoncubeproject/horizon-mixin.toml` (10 `[[injection]]`, 2 `[[module]]`) | no plan: typed `Hooks`/Part methods and `Registrar` declarations | `retired-hnmod-0.1.0/mixins/` |
| `mixins/…/data/dysoncubeproject-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: the package *is* the content, so there is nothing to embed | `retired-hnmod-0.1.0/mixins/` |
| `content/` (`horizon-mod.toml`, `assets/`, `data/`, built `.hmod`) | the package root: `horizon-mod.toml`, `assets/`, `data/` | `retired-hnmod-0.1.0/content/` |
| `dysoncubeproject.hnmod` (45,951 B) | `dysoncubeproject.hmod` (63,551 B, native + content) | `retired-hnmod-0.1.0/` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the pinned commit decides the toolchain | `retired-hnmod-0.1.0/` |
| `tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/audit/` | gone: staging and `.hnmod` packaging belong to the retired format | `retired-hnmod-0.1.0/tools/` |
| `Cargo.lock` | deleted, never packed (it changes the artifact, trap 2) | not kept |

**Kept and updated:** `tools/e2e.sh` (now `GAME=` for a launcher-built game, optional
`CONTROL_GAME=`, the same assertions, `SELFTEST=1` still flips the rate check),
`tools/carry_content.sh` (writes `assets/` at the package root now), `AGENTS.md`
(rewritten for the SDK layout — a stale copy is packed into every package),
`README.md`, `FINDINGS.md`, `VALIDATION.md`, `LICENSE`, `LICENSE-NOTICE.md`.

**The content half is the same bytes.** `assets/` + `data/` were lifted out of
`content/` by `cp` and every one of the **51 files** has the same sha256 in both
places (`evidence/dysoncubeproject/sdk-content-carry.log`); the 13 files taken from
the jar still match it (`tools/carry_content.sh --check`). The old
`content/horizon-mod.toml` merged into the root manifest.

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id = "dysoncubeproject"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a`/`e59da90` for 26.3 |
| `Cargo.toml` | one crate, `edition = "2024"`; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (`register`, `register_commands`, `run_mod_command`) and the four Parts |
| `src/support.rs` | the glue: the compound codecs, the shell's read/write/clear, the sky facts, the tick's energy hand-off, the readout and the typed command mapping. 9 unit tests |
| `src/runtime.rs` | the pure model (sphere, multiblock, buffer, sky, commands), **byte-identical**, 38 tests |
| `assets/`, `data/` | the carried content: 2 blocks + 6 items, models, blockstates, items, textures, 6 lang files, 5 recipes, 1 loot table, the pickaxe tag, a creative tab, the six `c:` tags, 2 licence copies |
| `LICENSE`, `LICENSE-NOTICE.md`, `README.md` | inside the package |
| `tools/e2e.sh`, `tools/carry_content.sh` | the scenario and the carry check |
| `dysoncubeproject.hmod` | the deliverable: 60 entries, 63,551 B, sha256 `063195ef…c07dc` |

### Helper → hook table (each old injection and module, and where it lives now)

| 0.1.0 (`hook`, API) | 0.2.0 | Notes |
|---|---|---|
| `embedded-content` (`content_root`, HEAD, cancellable) | **nothing** | one `.hmod` carries code and content; nothing is written to `.horizon-cache/` |
| `state-for-placement` (`state_for_placement`, 76) | `BlockPart::state_for_placement` on `ReceiverBlock` | `Some(None)` refuses and keeps the stack; `&dyn LevelHost::state_block` answers the cell's name, so no `GameLevel` internals |
| `place-block` (`place_block`, 14) | `ItemPart::on_block_placed_by` on `ControllerItem` | the item side: the old hook fired for a placed stack, and this is its typed form |
| `destroy-block` (`destroy_block`, 14) | `BlockPart::on_destroyed_by_player` on `ReceiverBlock` **and** `ShellBlock` | two Parts, so neither reads a block id to know which one broke (see the deviation below) |
| `block-entity` (`block_entity`, 15) | `BlockEntityPart::block_entity` on `ReceiverEntity` + the shell type declared with **no** Part | a declared type without a Part starts with an empty compound, which is exactly what 0.1.0 returned for both ids; the type ids are unchanged (`dysoncubeproject:ray_receiver_controller`, `…:multiblock_structure`), so saved entities keep their id |
| `block-entity-ticks` (15) | `BlockEntityPart::block_entity_ticks` → `event.ticks = true` | attached only to the controller's type, so the placeholders never tick |
| `block-entity-energy` (142) | `Registrar::block_energy(&controller, ModEnergyBuffer{…})` | a declaration, not a hook: 100,000,000 capacity, `max_insert: 0`, `max_extract: 50,000,000`, extract face Down only |
| `block-entity-tick` (17) | `BlockEntityPart::block_entity_tick` | `event.entity.nbt` replaces the old `ModBlockEntity`; the level is found with `level_in_place` |
| `register-commands` (8) | `Hooks::register_commands` | the same five usages, all `Command::op(…, 4)` |
| `perform-command` (8) | `Hooks::run_mod_command` | `CommandInvocation`'s typed arguments, no string re-parse; the readout text is byte-identical to 0.1.0 |
| `[[module]] runtime` | `src/runtime.rs` (`pub mod runtime;`) | byte-identical |
| `[[module]] state/support` | `src/support.rs` + `src/lib.rs` | no `[[module]]` section: an ordinary Rust module |

Eight hook methods are implemented (`state_for_placement`, `on_destroyed_by_player` ×2,
`block_entity`, `block_entity_ticks`, `block_entity_tick`, `on_block_placed_by`,
`register_commands`, `run_mod_command`) plus `register`. Nothing is left without a home
and nothing is silently deleted.

### The package

`rm -f Cargo.lock && cargo horizon-mod build` → **63,551 B**, sha256
`063195efbfe9cf23791c1147642aa47eab9c7c23a612146a803cfee90a6c07dc`, **60 entries**:
`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`,
`src/{lib,runtime,support}.rs`, `uses.json`, 29 `assets/…`, 22 `data/…`. **No
`Cargo.lock`.** A second build is byte-identical (`sdk-build-second.log`,
`sdk-sha256.txt`).

`check`:

```
dysoncubeproject 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  63551 bytes, SHA-256 063195efbfe9cf23791c1147642aa47eab9c7c23a612146a803cfee90a6c07dc, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits`:

```
Dyson Cube Project:
0b14f8a  made for
```

`uses.json` in the package lists **318 entries** (28 hook ids, 39 methods, 77 handles,
141 types, 24 traits, 2 capabilities — `horizon:energy` and `registries` — 2 macros, 4
Parts). The list is the API surface the compiled crate reaches, the same breadth
`framedblocks`' 324 entries show, not one entry per call — recorded so the next reader
does not read it as 318 calls.

### The ladder (inside build slots, `CARGO_INCREMENTAL=0`, no `--target-dir`)

| Step | Command | Result |
|---|---|---|
| runtime only, before any glue | `cargo test` | **38 passed** (the byte-identical `runtime.rs`) |
| glue written with `todo!()` | `cargo test` | **red: 6 failed, 40 passed** — the 5 codec tests and the command-mapping test panicked on `todo!()` (`sdk-test-red-todo.log`) |
| glue filled in | `cargo test --offline` | **48 passed**, 0 failed (38 `runtime::`, 9 `support::`, 1 `tests::`) (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` (never `--all`) | exit 0 (`sdk-fmt.log`); `runtime.rs` still `89598f0c…4b1a` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `SDK 0b14f8a … in ./.horizon/sdk` / `Built ./target/horizon-mod/dysoncubeproject.hmod` (`sdk-build.log`) |
| registry check | `cargo horizon-mod check dysoncubeproject.hmod` | the block above, **EXIT 0** (`sdk-check.log`) |
| fits the pinned build | `cargo horizon-mod fits … --build nightly` | `0b14f8a  made for`, **EXIT 0** (`sdk-fits.log`) |
| same package twice | build again | 63,551 B, same sha256 (`sdk-build-second.log`, `sdk-sha256.txt`) |
| `runtime.rs` untouched | `cmp` + `unzip -p … src/runtime.rs \| sha256sum` | `89598f0c…4b1a` in all three places (`sdk-runtime-sha256.txt`) |
| content unchanged | `carry_content.sh --check` + sha256 diff | 13 jar entries match; 51 carried files identical to the retired copy (`sdk-content-carry.log`) |

### Behaviour that changed with the format (deliberate, and why)

- **The energy hand-off is one transactional capability insert.** 0.1.0 called the
  game's `native_energy_insert` twice — simulate, then commit. The SDK's route is
  `LevelRef::capability(host, &ENERGY, below, Some(Direction::Up))` and
  `EnergyHandler::insert` inside `host.transaction`, which answers what went in; the
  buffer gives up exactly that. Same rule (ask the block below through its up face,
  never move more than it takes), one call, atomic.
- **Teardown triggers on a player's break**, not on any removal. `onRemove`'s typed
  equivalent that fires before the block leaves the world is
  `BlockPart::on_destroyed_by_player`, and the shell *must* be read before removal
  because its saved `controllerX/Y/Z` is the only link to the controller.
  `affect_neighbors_after_removal` fires for every removal but only after the block
  entity is gone, so it cannot find the controller. **Recorded deviation:** an
  explosion or a piston removing a placeholder leaves the controller standing (0.1.0's
  `destroy_block` hook took it). The player path — which is how a shell is dismantled —
  behaves as before, and the clearing skips the cell the game itself is removing.
- **`blocks_motion` is a collision-shape query plus a fluid read** (`block_shape(…,
  Collision)` and `fluid_state().is_empty()`), the same rule as 0.1.0's
  `full_collision || fluid != None`, through the SDK's own tables.
- **No log lines.** The SDK exports no logger and a native mod may not add a crate, so
  the "multiblock_structure is not registered" warning is gone; a failed shell write is
  simply skipped, as before.
- **No `mark_modified`.** A Part that changed `event.entity.nbt` answers the default
  `Changed`, which dirties the chunk; the old explicit chunk-dirty call is gone.
- **`tools/e2e.sh` now needs a launcher-built game** (`GAME=`), has no `--dev-mod`, and
  takes an optional `CONTROL_GAME=` for the leg that proves the ids come from this
  package. `SELFTEST=1` still flips the rate assertion.

### Traps this migration hit

1. **Energy is not on `ServerHost`.** `energy_insert`/`energy_extract`/`energy_amount`
   are the *backend* trait; a mod asks
   `level.capability(host, &ENERGY, pos, Some(side))` and calls
   `EnergyHandler::{insert,extract,amount}` with a `Transaction` from
   `host.transaction`. Cost: one compile error (E0599).
2. **A `BlockEntityPart` must also be a `BlockPart`** (`BlockEntityPart: BlockPart`), so
   `impl BlockPart for ReceiverEntity {}` exists even though it answers no block moment.
3. **A `#[non_exhaustive]` argument enum needs a wildcard**, and typed arguments are read
   by `name` (`pos`, `value`, `delta`), not by position: the literal path already holds
   `[dysoncubeproject, set, beams]`, so the runtime's `parse_command` is never fed raw
   words.
4. **`let … else { return None }` in an `Option`-returning hook is a clippy error**
   (`question_mark`): the two teardown hooks use `let level = …?;`.
5. **A transaction whose error type is `Infallible` makes `if let Ok(..)` irrefutable**:
   `unwrap_or_else(|never| match never {})` is the warning-free form.
6. **`BlockUpdateFlags` has no const `|`** (the impl is not `const`): a `fn update_all()`
   builds `NEIGHBORS | CLIENTS` instead of a `const`.
7. **The block-entity tick is not told its level** (`BlockEntityTickEvent` carries only
   `pos` and the entity): `level_in_place` walks `host.levels().list_levels()` for
   `in_place`.
8. **`sky_at` must finish its immutable reads first.** `open_to_sky`'s closure borrows
   the host for the whole column scan, so the facts are computed into a `Sky` value
   before the same hook takes `&mut host` for the transaction.
9. **`Compound` keys are `JavaString`, which is not exported**: write with
   `nbt.insert(key.into(), Tag::Int(v))` and read with `nbt.get(&key.into())`.
10. **`LevelRef::dimension_id()` is `#[doc(hidden)]`** ("mods ask the host"): compare
    `level.dimension()` with `DimensionKey::OVERWORLD` instead.
11. **`LevelWriteError`/`BlockEntityError` are typed, not panics**: `set_block` answers
    `Ok(false)` for a refused write and `Err` for a stale level, both "nothing
    happened"; the glue skips that cell.
12. **`uses.json` is broad.** 318 entries for eight hook methods: it lists the surface
    the crate reaches, not the calls made (framedblocks: 324). Do not treat it as a
    call count.
13. **The old `AGENTS.md` would have been packed.** It told the reader to open
    `compatibility.json` and run `tools/horizon.py`; rewritten for the SDK layout rather
    than retired, because root documents travel inside the package.

### Gaps

Everything in the slice gaps above still stands (the EM Rail Ejector, the shared
per-player sphere, the client half, the components' payloads, the advancements). Added
by this format:

1. **No 0.2.0 in-game run.** A native `.hmod` needs a game built by
   `horizon-launcher native-build`, and `horizon-launcher` is not installed here, so the
   package's behaviour in game is **unverified**. The 0.1.0 runs in
   `../_shared/evidence/dysoncubeproject/` are historical evidence for the
   byte-identical `runtime.rs`, not for the package (`HISTORICAL.md` there); what changed
   is the transport of the same calls.
2. **A non-player removal of a placeholder no longer takes the controller** (see the
   deviation above): an explosion clears one shell cell and leaves the machine. A later
   slice can add `affect_neighbors_after_removal` plus a level attachment for the link.
3. **No diagnostics**: no logger, so a missing `multiblock_structure` or a refused shell
   write is silent.

### Changelog (0.2.0) — first publication

> **0.2.0 — first publication, as a native SDK `.hmod`.** Dyson Cube Project slice 1 for
> Horizon: the ray receiver controller and its 3×6×3 multiblock, the sphere's power
> model, and `/dysoncubeproject status|set|add`. Horizon's mod-API merge (launcher ≥
> 0.2.70) replaced the `.hnmod` container with one flat package, so what was a code
> bundle plus an embedded content `.hmod` is now a single `dysoncubeproject.hmod`:
> `horizon-mod.toml` with `api = "0.2"` and `[native]` pinned to nightly
> `0b14f8acfd97fa2c750e42795249dcc981178ca4` / mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` for 26.3, `src/lib.rs` with one
> `impl mod_api::Hooks` and four Parts, `src/support.rs` for the glue, and the two
> blocks, six items, models, textures, lang, recipes, loot, tags and creative tab in
> `assets/` and `data/`. The ten old injections became eight typed hook methods
> (`state_for_placement`, `on_destroyed_by_player`, `block_entity`,
> `block_entity_ticks`, `block_entity_tick`, `on_block_placed_by`, `register_commands`,
> `run_mod_command`), the 142 energy hook became a `block_energy` declaration
> (100,000,000 FE, extract through the down face only), and the content-injection hook
> is gone because the package carries its own content. `src/runtime.rs` and its 38 tests
> are byte-identical to 0.1.0. Package: 60 entries, **63,551 B**, sha256
> `063195efbfe9cf23791c1147642aa47eab9c7c23a612146a803cfee90a6c07dc`; `check` and
> `fits` exit 0; a second build is byte-identical. Known gaps: the EM Rail Ejector, the
> shared per-player sphere, the client renderers/GUIs, the advancements — and no
> 0.2.0 in-game run, because the launcher is not installed in this environment.

### Migration verification

| Check | Command | Result |
|---|---|---|
| Runtime tests before any glue | `cargo test` | **38 passed** |
| Glue red first | `cargo test` with `todo!()` bodies | **6 failed / 40 passed** (`sdk-test-red-todo.log`) |
| Deliverable tests | `cargo test --offline` | **48 passed**, 0 failed (`sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0 (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | exit 0 (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build` | 63,551 B (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check` | EXIT 0 (`sdk-check.log`) |
| Fits | `cargo horizon-mod fits --build nightly` | EXIT 0 (`sdk-fits.log`) |
| `runtime.rs` untouched | `cmp` + pack hash | `89598f0c…4b1a` three ways (`sdk-runtime-sha256.txt`) |
| Content unchanged | sha256 diff | 51 files identical (`sdk-content-carry.log`) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Slice 2 — the shared per-player sphere (0.2.1, 2026-11-21)

Pins: SDK nightly **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, `package_format` 6, for **26.3**. The
SDK is cached in-tree at `../_shared/sdk/0a875da/` and copied into `.horizon/sdk/`
(git-ignored, never packed); the stale 0b14f8a copy was deleted first (trap 17a).

**The gap line it comes from** (`notes.md` § Gaps, item 2): *"**The shared sphere**:
`DysonSphereProgressSavedData`, subscribe/unsubscribe, one sphere for several receivers,
the `DysonSphereSyncMessage` packet."*

**Why this before the EM Rail Ejector** (the first gap line): the ejector's own line calls
its ejection *"the only way to grow a sphere"*, and slice 1's sphere belonged to **one
controller** — a deviation forced by the 0.1.0 pin's missing level saved data. An ejector
on that model could only grow its own private copy, and a "nearest receiver" link would be
an invented rule. At `0a875da` the SDK has **level attachments** (`AttachmentType` +
`Holder::Level`, saved per dimension by `SaveDir::load_attachments`/`save_attachments` into
`data/horizon/attachments.dat`), which is upstream's `DysonSphereProgressSavedData`. So the
prerequisite became port-able and goes first; the ejector is now a clean next slice. The
sync packet stays part of the client gap (there is no client half yet).

### What changed

| aspect | slice 1 (0.2.0) | slice 2 (0.2.1), as upstream has it |
|---|---|---|
| the sphere | in the controller's block entity, one per controller | the level's `SPHERES` attachment: `spheres` by id and `subscribedPlayers` by UUID (`DysonSphereProgressSavedData.save/load`), saved with the dimension |
| who owns a sphere | the controller | the placer: `getSubscribedFor(uuid)` is the subscribed sphere, their own UUID otherwise; a placement creates it (`computeIfAbsent`) and saves the id in the controller (`dysonSphereId`) |
| generation | inside the receiver's tick | `Hooks::level_tick_pre`, Overworld only — once per sphere per level tick, as upstream's `LevelTickEvent.Pre`; a receiver only pulls (`runtime::harvest`) |
| receivers per sphere | one | any number: they share the tick's reservoir, and each is gated by its own sky |
| commands | `set|add … <x> <y> <z> <n>` | upstream's own `set|add beams|panels <sphere> <n>`, with upstream's messages; `status <x> <y> <z>` stays ours (the GUI addon as a line) and names the sphere |

### Hooks

| Hook | Helper | What it does |
|---|---|---|
| `level_tick_pre` | `support::generate_spheres` | Every sphere of the Overworld refills its reservoir once. |
| `on_block_placed_by` | `support::subscribe_receiver` + `form_shell` | Subscribes the placer's sphere, saves `dysonSphereId`, writes the shell (as before). |
| `block_entity_tick` | `support::receiver_tick` | Pulls from the sphere named by `dysonSphereId` (no generation), then pushes down. |
| `status` / `set|add` | `support::{status_text,change_sphere}` | Readout and the four upstream forms, by sphere id. |

`level_tick_pre` is a new global hook (stamped at the pin); the other five hook methods are
the existing ones. **A local type must not be named after an SDK type:** the runtime's
command enum was `Command`, and `cargo horizon-mod fits --build 0b14f8a` read the API's
`Command` instead and reported *"the type `Command` can no longer be built with a literal,
and `src/support.rs:540` builds it"* (exit 1). Renaming it to `SphereOp`/`LineError` (and
the `Invocation::Command` variant to `Sphere`) makes the same check answer `0b14f8a fits`.
This is trap 3 of the 2026-11-21 hand-over, met in person.

### Red → green (TDD)

The family's rules were written as failing tests first, with `todo!()` bodies for the two
new runtime functions and the store codec:

```
test result: FAILED. 45 passed; 12 failed; 0 ignored; 0 measured; 0 filtered out
thread 'runtime::tests::a_uuid_reads_as_java_prints_it' panicked at src/runtime.rs:309:5:
not yet implemented: slice 2: the UUID a sphere id is named by
thread 'runtime::tests::two_receivers_draw_from_one_reservoir' panicked at src/runtime.rs:412:5:
not yet implemented: slice 2: a receiver pulls from the shared sphere without generating
thread 'support::tests::the_store_codec_round_trips_upstreams_saved_form' panicked at src/support.rs:110:5:
not yet implemented: slice 2: the sphere store's saved form
```

(`../_shared/evidence/dysoncubeproject/slice2-test-red.log`, the whole run.) Then the
implementation, and **57 tests pass** — 46 runtime rules (the store, the UUID text, the clamped set/add by id,
generate-once, the shared reservoir, `harvest` without generation, the command grammar) and
10 glue tests (the saved form round trip, the saved sphere id, the typed command mapping)
plus the package-id test.

### In game (three legs, `../_shared/evidence/dysoncubeproject-e2e/`)

A launcher build (`launcher-game.sh build dysoncubeproject`; the log says
`Compiling dysoncubeproject (1 of 1)`, and the packaged `src/*.rs` + manifest are
`cmp`-identical to the port's), launched `--offscreen` because the crafting phase opens a
menu.

`e2e-launcher.log` — **PASS + EXIT:0**:

| phase | measured |
|---|---|
| C0 recipes | all five carried recipes craft in a crafting table the way they declare: beam ×2, solar_sail ×1, beam_package ×1, solar_sail_package ×1, ray_receiver_controller ×1, each result asserted by id and count and each grid empty afterwards |
| A wrong shape | a stone in the 3×6×3 box: the controller stayed air, 0 placeholders |
| B formation | the **crafted** controller placed and formed 16 placeholders |
| C readout | `sphere '83e67318-…'` — the placer's UUID — beams 0, sails 0/0, buffer 0 |
| D ceilings | `set beams 8333334` → 50,000,000 supported; `set panels 100` → 100/50000000; `add panels 100000000` → 50,000,000; `add beams -8333333` → 1 beam / 6 sails |
| E rate | one receiver: 100 sails, **2,000.0 FE/t** over 104 ticks |
| F shared | a second receiver reads the **same** id; the pair's buffers grow **2,000.0 FE/t together** (two private spheres would give 4,000) |
| F2 control | a stone roof over A: A flat, B alone at **2,000.0 FE/t**; the roof removed, A takes again |
| G independent | sphere `keepalive-…` set to 10 beams / 60 sails leaves the receivers at 17 / 100 |
| H night | the pair flat at t=18000, and **2,000.0 FE/t** again by day |
| I caps | 50,000,000 sails → 1,000,000,000 FE/t, buffer stops at **100,000,000** and `down 0` |
| J persistence | `data/horizon/attachments.dat` exists and holds the sphere id; after `save` + quit + relaunch the same id, beams and sails are back and the buffer did not regress |
| K teardown | a broken placeholder took the controller; a broken controller left 0 placeholders |

`e2e-launcher-selftest.log` — **FAIL + EXIT:1**: `SELFTEST=1` forces the rate to 1.0 and the
run stops at `FAIL: the receiver filled at 1.0 FE/t, not 100 sails * 20`.
`e2e-launcher-control.log` — **FAIL + EXIT:1**: the same scenario on the launcher build
**without** the package stops at `FAIL: the beam recipe offered {"item":null,"slot":200},
expected 2 x dysoncubeproject:beam` — the ids are unknown there, so no positive is vacuous.
`check-e2e-logs.py dysoncubeproject-e2e --strict` → 3/3.

### Coverage (parity rule)

| family in the jar | state |
|---|---|
| 5 carried recipes | **tested** — crafted in a table (C0), output id/count and consumption asserted |
| the ray receiver multiblock | **tested** — refused/short (A), formed (B), teardown both ways (K/K2) |
| the receiver's energy loop | **tested** — 2,000 FE/t (E), sky gate (F2), night (H), caps (I) |
| the shared sphere | **tested** — same id (F), one reservoir (F), independent id (G), saved across a relaunch (J) |
| `/dysoncubeproject` commands | **tested** — all four forms by sphere id (D, E, G, I) and the readout (C) |
| blocks and items | **tested** — the crafted controller places and reads back; the four components are given and used as ingredients |
| the EM Rail Ejector | `not ported` — the next slice; it grows the sphere that now exists |
| the client half (renderers, hologram, sky, GUI screens, sounds, sync packet) | `not ported` — no client half in this campaign's scope yet |
| the subscribe/unsubscribe GUI buttons | `not ported` — the command surface sets a sphere; the buttons send the same messages |
| the beam/sail data components | `not ported` — only the ejector reads them, and their values are 1/8 (sail/package) and 1/4 (beam/package) |
| the 7 advancements | `not ported` — the grant loop is upstream's `publishAdvancements` |

### Deviations now (and one removed)

- **Removed:** "one sphere per controller". The sphere is the level's, keyed by the player's
  UUID, as upstream's saved data has it.
- **The store is per dimension.** `Holder::Level` is the level in place, and upstream keys
  its saved data on the Overworld from any level. Only the Overworld generates and only the
  Overworld's store is read there, so a receiver in the Nether reads that level's (empty)
  store instead of the Overworld's. A receiver in the Nether could not pull anyway
  (`Level.isDay` is false in a fixed-time level), so nothing a player can observe differs.
- **No subscribe/unsubscribe buttons** (client half): a player gets their own sphere, and an
  operator can change any sphere by id. The level store keeps the `subscribedPlayers` map, so
  the buttons can be added without a save-format change.
- **`status <pos>` is ours**, not upstream's: upstream has the GUI addon. It names the
  sphere so the command's own id is discoverable in game.
- The two blocks are still drawn as plain cubes (no renderers), and `blocks_motion`,
  `isDay`, `isRaining` and the sky scan are unchanged from slice 1.

### Traps this slice hit

1. **A full buffer cannot show a night/day delta.** The first in-game run put the night
   control after the caps phase and it failed at `FAIL: the receiver did not resume by day`
   — the buffer was at 100,000,000 FE from the caps, so neither phase could move it. The
   night control now runs before the caps.
2. **With a shared reservoir, the receiver pair is the unit.** The day phase also failed
   while measuring only B: A takes the tick's reservoir, so B is legitimately flat. Both the
   sharing assertion and the night/day control now measure the **sum** of the two buffers.
3. **A craft *can* be driven in this campaign's e2e.** `--offscreen` gives the session a
   renderer, `ctl screen` lists the grid (100–108) and the result (200), and right-clicking
   drops one item per cell. Slice 1's "needs a renderer" note was wrong; the five recipes are
   now proven.
4. **`build-slot.sh run --step check` forwards by default** and then runs in the campaign
   copy, so `BUILD_LOCAL=1` plus an explicit `cd ports/dysoncubeproject` is the only
   deterministic form for a local ladder; a remote `cargo fmt` writes nothing in this tree.
5. **The attachment file is per dimension**: `saves/<world>/dimensions/minecraft/overworld/
   data/horizon/attachments.dat` (gzipped NBT), so the e2e asserts the file and greps the
   decompressed bytes for the sphere id before relaunching.

### The package (0.2.1)

`rm -f Cargo.lock && cargo horizon-mod build` → **68,932 B**, sha256
`dbdc5aa7bb629c6caabc16b6e476493bf5170c830f13711d25e620f38c9e6b94`, **60 entries**, no
`Cargo.lock`; a second build is byte-identical (`sdk-shas` in
`../_shared/evidence/dysoncubeproject/sdk-{build,build-second,sha256}.*`).

```
dysoncubeproject 0.2.1 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  68932 bytes, SHA-256 dbdc5aa7bb629c6caabc16b6e476493bf5170c830f13711d25e620f38c9e6b94, unsigned
```

```
Dyson Cube Project:
0a875da  made for
```
```
Dyson Cube Project:
0b14f8a  fits
```

`verify-sdk-package.py dysoncubeproject` → **1/1** (package, runtime byte-identical in tree
and pack, 57 tests, 0 clippy warnings, check, fits, `Skills:` line).

### Changelog (0.2.1)

> **0.2.1 — the shared per-player sphere.** The sphere is no longer one per controller: the
> level keeps every sphere and its subscriptions (the same shape as upstream's
> `DysonSphereProgressSavedData`), saved in the dimension's `data/horizon/attachments.dat`,
> and each receiver draws from the sphere its placer subscribed to — several receivers share
> one reservoir, and the tick generates for every sphere once (`level_tick_pre`, Overworld).
> `/dysoncubeproject set|add beams|panels` take a sphere id again, as upstream's commands do,
> and `status` names the sphere it reads. `SELFTEST=1` and the control run are the
> fail-capable legs; all five carried recipes are now crafted in game. Known gaps unchanged
> in kind: the EM Rail Ejector (next), the client half and its sync packet, the growth
> advancements, the component payloads.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
