# Dyson Cube Project — a Horizon port

**Dyson Cube Project** is a Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [InnovativeOnlineIndustries/Dyson-Cube-Project](https://github.com/InnovativeOnlineIndustries/Dyson-Cube-Project) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Dyson Cube Project ([source](https://github.com/InnovativeOnlineIndustries/Dyson-Cube-Project)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `dysoncubeproject.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The energy hand-off is one transactional capability insert.** 0.1.0 called the
- **Teardown triggers on a player's break**, not on any removal. `onRemove`'s typed
- **`blocks_motion` is a collision-shape query plus a fluid read** (`block_shape(…,
- **No log lines.** The SDK exports no logger and a native mod may not add a crate, so
- **No `mark_modified`.** A Part that changed `event.entity.nbt` answers the default
- **`tools/e2e.sh` now needs a launcher-built game** (`GAME=`), has no `--dev-mod`, and

## Not yet ported

- **EM Rail Ejector**: its multiblock, its 400,000 FE buffer, 40 FE per sent item,
- **The shared sphere**: `DysonSphereProgressSavedData`, subscribe/unsubscribe, one
- **The client half**: `RayReceiverRender`, `EMRailEjectorRender`, `HologramRender`,
- **The component items' payloads** (`dysoncubeproject:solar_sail` /
- **Advancements**: the root and the six sphere-percentage milestones.
- The ejector block/item/recipe/loot.
- **No 0.2.0 in-game run.** A native `.hmod` needs a game built by
- **A non-player removal of a placeholder no longer takes the controller** (see the

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `dysoncubeproject.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Dyson Cube Project was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/InnovativeOnlineIndustries/Dyson-Cube-Project

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run dysoncubeproject` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/dysoncubeproject/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
