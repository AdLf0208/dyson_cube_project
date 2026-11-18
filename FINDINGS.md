# Dyson Cube Project — upstream findings

Ported from **Dyson Cube Project 1.0.5** (`InnovativeOnlineIndustries/Dyson-Cube-Project`),
the exact ATM10 jar `dysoncubeproject-1.0.5.jar` (759,076 B unpacked, 171 entries,
`ports/_shared/licence-cache/dysoncubeproject-1.0.5.jar`). Behaviour was read from
the upstream repository at commit **`13c01e0`** ("Removed log message"), cloned to
`ports/_shared/upstream/dysoncubeproject` (deleted after these findings were
recorded). Analysis only: this port is a clean-room Rust rewrite.

## Licence (read from the jar, not the catalogue)

- `META-INF/neoforge.mods.toml`: `license = "MIT"`, `modId = "dysoncubeproject"`,
  `version = "1.0.5"`, `authors = "Buuz135"`, `displayName = "Dyson Cube Project"`.
- `gradle.properties` upstream: `mod_license=MIT`; `LICENSE`: MIT, "Copyright (c)
  2025 Innovative Online Industries".
- Rank **0** (permissive). The port stays local-only until the maintainer approves
  publishing (`ARCHITECTURE.md` §8).

## What the jar actually contains (inventory, not the name)

| Family | Entries |
|---|---|
| Blocks (3) | `multiblock_structure`, `ray_receiver_controller`, `em_railejector_controller` |
| Items (6 with models) | `beam`, `beam_package`, `solar_sail`, `solar_sail_package` + the two controllers' block items |
| Blockstates (3) | one `""` variant each — no block state properties |
| Block models | **all three are 73-byte stubs** (`{"textures":{"particle":…}}`, no elements): every block is drawn by a block-entity renderer |
| Recipes (6) + recipe advancements (6) | `beam`, `beam_package`, `solar_sail`, `solar_sail_package`, `ray_receiver_controller`, `em_railejector_controller` |
| Loot (2) | the two controllers only (`survives_explosion`); `multiblock_structure` has `noLootTable()` |
| Advancements (7) | root + six sphere-percentage milestones on the ejector |
| Assets | 6 lang files, 6 block textures, 4 item textures, 2 sounds (`ray.ogg`, `railgun.ogg`), 5 shaders, `sounds.json` |
| Code | 3 blocks, 3 block entities, 2 renderers, 2 GUI addons, a shader/sky renderer, 2 network messages, 2 command trees |

The brief said "7 items, 12 recipes"; the artefact says **6 items with models**
(`multiblock_structure` has neither an item model nor a creative-tab entry — see
`DCPContent.Blocks.MULTIBLOCK_STRUCTURE`, registered with `null` as its tab) and
**6 recipes + 6 recipe advancements = 12 recipe JSONs**. The artefact wins; the
port declares `multiblock_structure` with `"item": false`.

## Behaviour read (files → what was extracted)

### `multiblock/MultiblockStructure.java`

- `new MultiblockStructure(sizeX, sizeY, sizeZ, shape)`; the ray receiver's is
  `MULTIBLOCK_STRUCTURE = new MultiblockStructure(3, 6, 3, SHAPE)`
  (`block/RayReceiverControllerBlock.java:41`).
- `validateSpace(level, anchor)`: `halfX = sizeX / 2` (integer, floor), `halfZ =
  sizeZ / 2`; `min = anchor.offset(-halfX, 0, -halfZ)`; **every** cell of
  `sizeX × sizeY × sizeZ` from `min` must be `isAir()` — including the anchor cell
  itself, which is where the controller is about to go.
- `getAABB` uses the full size in each direction (only used for rendering).

### `block/DefaultMultiblockControllerBlock.java`

- `getStateForPlacement` returns the block's default state only when
  `getMultiblockStructure().validateSpace(level, clickedPos)`; otherwise `null`,
  i.e. **the controller cannot be placed at all** when the space is not clear.

### `block/RayReceiverControllerBlock.java`

- `Properties.ofFullCopy(Blocks.IRON_BLOCK)` (hardness 5, resistance 6, iron-block
  sounds, needs the right tool), no state properties.
- `setPlacedBy` (server): creates/gets the placer's sphere in the saved data,
  stores its id in the block entity, then places the placeholder blocks:
  `(-1,0,0) (1,0,0) (0,0,-1) (0,0,1)`, `above() … above(4)` (y 1..4) and the 3×3
  ring at `y = 2` from `pos.offset(-1,2,-1)` — **16 unique blocks** (the ring's
  centre repeats `above(2)`).
- `onRemove`: sets those same 17 positions (16 unique) to air.
- `CONTROLLER_LOCAL_SHAPE = SHAPE ∩ box(0,0,0,16,15.9,16)` = the template's full
  cube for the controller's own cell.

### `block/MultiblockStructureBlock.java`

- `Properties.ofFullCopy(Blocks.IRON_BLOCK).noOcclusion().noLootTable()`.
- `createStructure`: `setBlockAndUpdate(at, multiblock_structure)` and stores the
  controller position in the placeholder's block entity.
- `onRemove`: a broken placeholder calls `world.destroyBlock(controllerPos, true)`
  — **breaking the shell drops the controller**.
- `getShape`/`getCollisionShape` = the controller's shape split into 16³ cells
  (the renderer draws the machine; the shape is what you collide with).
- `getCloneItemStack` returns the controller's item (middle-click picks the
  controller, not the placeholder).

### `block/tile/RayReceiverBlockEntity.java` — the energy loop

```java
serverTick:
  if (level.isDay() && !level.isRaining() && level.canSeeSky(pos.above())) {
      extractingAmount = min(RAY_RECEIVER_EXTRACT_POWER,
                            buffer.maxEnergyStored - buffer.energyStored);
      extracted = sphere.extractPower(extractingAmount);
      buffer.energyStored += extracted;
  }
  capability = level.getCapability(EnergyStorage.BLOCK, pos.below(), Direction.UP);
  if (capability != null && capability.canReceive()) {
      received = capability.receiveEnergy(min(RAY_RECEIVER_EXTRACT_POWER, buffer.energyStored), true);
      buffer.energyStored -= received;
      capability.receiveEnergy(received, false);
  }
  // pitch animation from level.getTimeOfDay(1f) (client visual)
```

- Buffer: `new EnergyStorageComponent<>(RAY_RECEIVER_POWER_BUFFER, 0, Integer.MAX_VALUE, …)`
  — **cannot receive** FE (`0`), extracts up to `MAX_VALUE`.
- The capability is registered for `Direction.DOWN` only
  (`DysonCubeProject.java:93-99`): the block below pulls through its UP face.
- `@Save dysonSphereId`, `energyStorageComponent`, `currentPitch` are the block
  entity's NBT.

### `world/DysonSphereStructure.java` — the sphere model

- `beams`, `solarPanels`, `storedPower`, `lastConsumedPower` (NBT keys of the same
  names).
- `getMaxSolarPanels() = min(MAX_SOLAR_PANELS, beams * BEAM_TO_SOLAR_PANEL_RATIO)`.
- `getProgress() = solarPanels / (double) MAX_SOLAR_PANELS`.
- `getMaxBeams() = (int) Math.ceil(MAX_SOLAR_PANELS / (double) BEAM_TO_SOLAR_PANEL_RATIO)`.
- `increaseBeams/increaseSolarPanels` add then clamp to the max above.
- `generatePower()`: `lastConsumedPower = 0; storedPower = min(sails * POWER_PER_SAIL,
  storedPower + sails * POWER_PER_SAIL)` — i.e. the reservoir refills to exactly one
  tick of output, never more.
- `extractPower(amount)`: `min(amount, storedPower)`, subtracts it and adds it to
  `lastConsumedPower`.

### `DysonCubeProject.java` — where generation runs

- `LevelTickEvent.Pre`, **Overworld only** (`dimensionTypeRegistration().getRegisteredName()
  == minecraft:overworld`), every tick: `data.getSpheres().values().forEach(DysonSphereStructure::generatePower)`.
  Every 4 ticks it syncs the spheres to clients; every 200 it publishes advancements.
- Commands, permission level **4**:
  `/dysoncubeproject set beams <sphereId> <value>`, `set panels <sphereId> <value>`,
  `add beams <sphereId> <delta>`, `add panels <sphereId> <delta>`. `set` clamps to
  the sphere's max; `add` clamps `current + delta` to `0..max`.
- `Capabilities.EnergyStorage.BLOCK` for the ray receiver: only through `DOWN`.

### `Config.java` — every number the slice uses

| Constant | Value | Meaning |
|---|---:|---|
| `MAX_SOLAR_PANELS` | 50,000,000 | the sphere's cap |
| `BEAM_TO_SOLAR_PANEL_RATIO` | 6 | sails each beam supports |
| `POWER_PER_SAIL` | 20 | FE per sail per tick |
| `RAY_RECEIVER_EXTRACT_POWER` | 50,000,000 | FE/t the receiver pulls and pushes |
| `RAY_RECEIVER_POWER_BUFFER` | 100,000,000 | the receiver's buffer |
| `RAIL_EJECTOR_POWER_BUFFER` | 400,000 | (ejector — a gap) |
| `RAIL_EJECTOR_CONSUME` | 40 | (ejector — a gap) |

### `client/gui/DysonProgressGuiAddon.java` — the readout the slice reproduces

`Dyson Information`, `Progress: n%`, `Power Gen: sails*20 FE`, `Power Con:
lastConsumedPower FE`, `Beams: n`, `Sails: n/max`, and `Needs more beams` when
`sails >= max && sails < MAX_SOLAR_PANELS`.

### `client/*`, `network/*`, `datagen/*`

Renderers (`RayReceiverRender`, `EMRailEjectorRender`, `HologramRender`, `SkyRender`),
shaders, the two GUI addons and the sync packets are the client half — a recorded
gap (the pin has no per-block-entity renderer hook for a custom shape at this
slice's cost).

## Differences from the rewrite (deliberate)

| Upstream | This port | Why |
|---|---|---|
| spheres live in level saved data (`DysonSphereProgressSavedData`, keyed by the player's UUID, shared by every receiver) | the sphere lives in the controller's own block entity NBT | the pin has **no level saved-data API**; a controller's NBT persists with its chunk. One sphere per controller, and the command addresses the controller instead of a sphere id |
| `Level.isDay()` (sky darken < 4, fixed-time dimensions never day) | `day_time % 24000 < 12000` and the Overworld only | `native_day_time` (API 97) is the closest read; fixed-time dimensions are exactly the non-Overworld ones here |
| `level.isRaining()` | `native_weather().0 > 0.2` (vanilla's own threshold) | API 97 |
| `canSeeSky(pos.above())` | `native_light(pos.above()).1 == 15` | API 97's sky light is the closest read; a glass roof counts as sky in both |
| `generatePower` at level-tick-pre for every sphere | inside the controller's own tick, before its extraction | one sphere per controller makes the two orders identical |
| buffer exposed through `Capabilities.EnergyStorage.BLOCK` on `DOWN` | `native_block_entity_energy` with `extract_faces` on the down face only, `max_insert: 0` | API 142 |
| blocks drawn by block-entity renderers (empty block models) | a plain cube per block with the jar's own texture | the client half is a gap; an invisible machine would be unplayable, and the deviation is recorded |
| `/dysoncubeproject set|add beams|panels <sphereId> …` | the same four forms, addressed by the controller's position | no global sphere store (above) |
| ejector, beams/sails injection, advancements, sounds, shaders, GUIs | not carried | slice 1 is the receiver + its multiblock |

## Gaps left for later slices

- **EM Rail Ejector controller**: its multiblock, its 400,000 FE buffer, its 40 FE
  per sent item, the beam/sail ejection that is the only way to grow a sphere.
- **The sphere's shared, per-player model**: saved data, subscribe/unsubscribe,
  multiple receivers on one sphere, the sync packet.
- **The client half**: both renderers, the hologram, the sky replacement, the two
  GUI screens, the shaders, the sounds.
- **The component items' payloads** (`dysoncubeproject:solar_sail` /
  `dysoncubeproject:beam` data components) — no component-registration API is used
  by this slice, and only the ejector reads them.
- Advancements (root + six milestones), the ejector block/recipe/loot.
