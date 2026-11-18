# Working on this Horizon mod package

This port is one flat SDK package (`api = "0.2"`), not a `.hnmod` bundle. The campaign
rules live one level up in [`../../AGENTS.md`](../../AGENTS.md); this file only says
what is specific to this directory.

- **Layout.** `horizon-mod.toml` is the only manifest and the only identity (`id = "dysoncubeproject"`
  is the resource namespace). `Cargo.toml` depends only on `horizon-mod-api` from the pinned
  SDK, which any `cargo horizon-mod` command fetches into `.horizon/sdk/` (git-ignored, never
  packed). `src/lib.rs` is the one `impl mod_api::Hooks` plus this port's Parts;
  `src/support.rs` is the game glue; `src/runtime.rs` is the pure model and its tests.
  `assets/` and `data/` are the carried content, at the package root.
- **`src/runtime.rs` is byte-identical to the retired 0.1.0 bundle's copy.** Do not format,
  edit or "improve" it; the campaign's rule is that a migration changes glue, manifest and
  packaging only. `cmp` it against
  `../_shared/evidence/dysoncubeproject/retired-hnmod-0.1.0/mixins/dysoncubeproject/runtime.rs`.
- **Nothing else may sit at the package root.** `cargo horizon-mod build` refuses any path
  that is not a package path, which is why `mixins/`, `content/`, `compatibility.json`,
  `rust-toolchain.toml`, `tools/horizon.py`, `tools/package.py`, `tools/package.sh` and
  `tools/audit/` live under `../_shared/evidence/dysoncubeproject/retired-hnmod-0.1.0/`.
- **Builds go through the campaign's slot registry** (`../tools/build-slot.sh run --job … --
  bash -c 'cd ports/dysoncubeproject && …'`), with `CARGO_INCREMENTAL=0`, no `--target-dir`
  and no private `BUILD_SLOTS_DIR`. A compilable mod is checked with `cargo test`, `cargo
  clippy --all-targets -- -D warnings` and `cargo fmt --check` (never `--all`: that formats
  the fetched SDK); the package itself with `cargo horizon-mod check <id>.hmod`.
- **No `Cargo.lock` inside the package.** `cargo test`/`clippy` write one; `rm -f Cargo.lock`
  right before `cargo horizon-mod build`, or the artifact changes.
- **Do not commit** the SDK, `target/`, exported sources or Minecraft assets.
- **Keep the claims honest.** A behaviour claim needs a game built by the launcher
  (`horizon-launcher native-build . --trust-native-code`), which is not installed here; the
  0.1.0 runs are history, not evidence for the 0.2.0 package (see `VALIDATION.md`).
