# Licence provenance

- Mod: `dysoncubeproject` — original: <https://github.com/InnovativeOnlineIndustries/Dyson-Cube-Project>
  (Dyson Cube Project).
- Original author: Buuz135 (Innovative Online Industries).
- Original jar this port was written against: `dysoncubeproject-1.0.5.jar`, whose
  `META-INF/neoforge.mods.toml` declares **`license = "MIT"`** (and whose
  `gradle.properties` says `mod_license=MIT`; its `LICENSE` is the MIT text,
  "Copyright (c) 2025 Innovative Online Industries").
- Copyright of the original: Copyright (c) 2025 Innovative Online Industries.
- This port is **MIT**, © 2026 the Horizon port authors.
- **Rank 0 (permissive).** The port stays local-only until the maintainer
  approves publishing (`ARCHITECTURE.md` §8).

The notice travels **inside the distributed archive**, not only in this source
repository: the native bundle carries `mixins/dysoncubeproject/LICENSE` and
`mixins/dysoncubeproject/LICENSE-NOTICE.md` (`tools/package.py` adds them), and
the embedded content mod carries them at `data/dysoncubeproject/license/`
(`tools/package.sh` writes them before building the `.hmod`).

What this port is: a clean-room Rust rewrite of the ray receiver's behaviour
(the multiblock formation, the sphere's power model and the FE buffer) against
the pinned Horizon APIs, plus the mod's own carried data — blockstates, models,
textures, lang, recipes and loot — under the original MIT licence above. No Java
source was copied; the upstream repository was read for behaviour only.
