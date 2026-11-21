# Dyson Cube Project for Horizon — the ray receiver and its shared sphere

One flat Horizon mod package (`.hmod`, mod API `0.2`): the Rust in `src/`, the mod's
own blocks, items, models, textures, lang, recipes, loot and tags in `assets/` and
`data/`. **One file** installs the blocks, the items and the behaviour.

Ported from **Dyson Cube Project 1.0.5** (`InnovativeOnlineIndustries/Dyson-Cube-Project`),
the exact ATM10 jar `dysoncubeproject-1.0.5.jar`, which declares `license = "MIT"`.
This is a clean-room Rust rewrite from the upstream behaviour and the mod's own
data files; see [`FINDINGS.md`](FINDINGS.md) for what was read and
[`notes.md`](notes.md) for the slice boundary, the deviations and the gaps.

## What it does

- **The multiblock.** `dysoncubeproject:ray_receiver_controller` may only be
  placed where its **3×6×3** box is empty (`MultiblockStructure.validateSpace`).
  A blocked box refuses the placement outright — no block, no item spent. A clear
  box forms: the sixteen `dysoncubeproject:multiblock_structure` placeholders are
  written around the controller, a broken placeholder takes the controller with
  it, and a broken controller clears the placeholders.
- **The shared sphere.** The level keeps every sphere and who subscribed to which
  (`DysonSphereProgressSavedData`, here one level attachment saved as
  `data/horizon/attachments.dat` inside the dimension). A controller subscribes
  its placer on placement and uses that player's sphere — their own UUID until
  they subscribe to another. Beams support six sails each, a sphere holds at most
  50,000,000 sails, and one sail makes **20 FE** each tick — all from the mod's
  own `Config`.
- **The receiver.** The level tick generates for every sphere once, so several
  receivers on one sphere share it. While it is day, it is not raining and it can
  see the sky, a receiver pulls up to **50,000,000 FE** a tick from that sphere
  into its **100,000,000 FE** buffer, then offers the same amount to the block
  below. The buffer is the declared `horizon:energy` capability, drainable
  through the machine's bottom face only, as upstream registers it.
- **The readout and the commands.** `/dysoncubeproject status <x> <y> <z>` prints
  what upstream's `DysonProgressGuiAddon` shows (the sphere it reads, progress,
  power gen, power con, beams, sails) plus the buffer; upstream's own
  `/dysoncubeproject set|add beams|panels <sphere> <n>` change a sphere by id at
  permission level 4.

## What it does not do (yet)

The **EM Rail Ejector** — the second controller, and the only way to put beams
and sails into a sphere — is not ported. Neither is the **client half** (both
block-entity renderers, the hologram, the sky replacement, the GUI screens with
the subscribe/unsubscribe buttons, the shaders, the sounds), the advancements, or
the beam/sail data components. [`notes.md`](notes.md) lists them all, and why each
is a gap rather than a shortcut.

## Build and check

The package is built by the launcher's `cargo-horizon-mod`, which fetches the
pinned SDK into `.horizon/sdk/` (git-ignored) on first use:

```sh
cd ports/dysoncubeproject
cargo test --offline                        # the pure rules, no game
cargo clippy --all-targets -- -D warnings
cargo fmt --check
rm -f Cargo.lock && cargo horizon-mod build # -> target/horizon-mod/dysoncubeproject.hmod
cargo horizon-mod check dysoncubeproject.hmod
cargo horizon-mod fits dysoncubeproject.hmod --build nightly
```

`build` packs, it does not compile: the native compile is the launcher's
`horizon-launcher native-build . --trust-native-code`. `tools/e2e.sh` runs the
in-game scenario against such a game; the 0.1.0 runs (and their gaps) are
recorded in [`VALIDATION.md`](VALIDATION.md).

## Licence

MIT, © 2025 Innovative Online Industries (the original) and © 2026 the Horizon
port authors. [`LICENSE`](LICENSE) travels inside the `.hmod`;
[`LICENSE-NOTICE.md`](LICENSE-NOTICE.md) records where the licence was read from.
Local-only until the maintainer approves publishing (`ARCHITECTURE.md` §8).
