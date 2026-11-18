// Game-side glue for the Dyson Cube Project ray receiver (slice 1).
//
// The rules live in [`crate::runtime`] and are byte-identical to the retired
// .hnmod's `runtime.rs`; this module only reads the world, calls the runtime and
// writes the result back.
//
// Shape, as upstream has it: placing the controller reserves a 3×6×3 box of air
// (`MultiblockStructure.validateSpace`), the placement writes sixteen
// `multiblock_structure` placeholders around it, and a broken placeholder takes
// the controller with it. The controller's block entity keeps the sphere
// (`beams`, `solarPanels`, `storedPower`, `lastConsumedPower`) and the FE buffer
// as the int `energy` the declared energy capability reads. Each server tick the
// sphere generates, the receiver pulls what its buffer has room for when the sky
// allows (`Level.isDay && !isRaining && canSeeSky`), and it hands up to
// 50,000,000 FE to the block below.
//
// Read from Dyson Cube Project 1.0.5 (`MultiblockStructure.validateSpace`,
// `RayReceiverControllerBlock.setPlacedBy/onRemove`, `MultiblockStructureBlock`,
// `RayReceiverBlockEntity.serverTick`, `DysonSphereStructure`, `Config`,
// `DysonCubeProject`'s level tick and commands). See `../FINDINGS.md`.
use mod_api::{
    BlockShapeKind, BlockUpdateFlags, CommandArgument, CommandArgumentValue, Compound,
    DimensionKey, Direction, ENERGY, HostTransaction, LevelHost, LevelRef, ModEnergyBuffer,
    ServerHost, Tag, Text,
};

use crate::runtime::{
    Buffer, Command as SphereCommand, RAY_RECEIVER_EXTRACT_POWER, RAY_RECEIVER_POWER_BUFFER, Sky,
    Sphere, Stat, USAGE, is_air, open_to_sky, structure_positions, tick, validate_space, volume,
};

pub(crate) const CONTROLLER: &str = "dysoncubeproject:ray_receiver_controller";
pub(crate) const STRUCTURE: &str = "dysoncubeproject:multiblock_structure";
const AIR: &str = "minecraft:air";
/// The only level whose tick generates sphere power (`DysonCubeProject`'s
/// `LevelTickEvent.Pre` guard).
const OVERWORLD: DimensionKey = DimensionKey::OVERWORLD;
/// The sixteen placeholders upstream writes.
const PLACEHOLDERS: usize = 16;

/// Upstream's five usages, all at permission level 4.
pub(crate) const USAGES: [&str; 5] = [
    "dysoncubeproject status <pos:block_pos>",
    "dysoncubeproject set beams <pos:block_pos> <value:integer>",
    "dysoncubeproject set panels <pos:block_pos> <value:integer>",
    "dysoncubeproject add beams <pos:block_pos> <delta:integer>",
    "dysoncubeproject add panels <pos:block_pos> <delta:integer>",
];

/// `Level.setBlock`'s `UPDATE_ALL`: neighbours and clients are told.
fn update_all() -> BlockUpdateFlags {
    BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

/// The sphere as `DysonSphereStructure.serializeNBT` saves it.
pub(crate) fn sphere_of(nbt: &Compound) -> Sphere {
    let mut sphere = Sphere::new();
    sphere.beams = read_int(nbt, "beams").max(0);
    sphere.solar_panels = read_int(nbt, "solarPanels").max(0);
    sphere.stored_power = read_int(nbt, "storedPower").max(0);
    sphere.last_consumed_power = read_int(nbt, "lastConsumedPower").max(0);
    sphere
}

pub(crate) fn store_sphere(nbt: &mut Compound, sphere: &Sphere) {
    write_int(nbt, "beams", sphere.beams);
    write_int(nbt, "solarPanels", sphere.solar_panels);
    write_int(nbt, "storedPower", sphere.stored_power);
    write_int(nbt, "lastConsumedPower", sphere.last_consumed_power);
}

/// The controller a placeholder belongs to (`MultiblockStructureBlockEntity`'s
/// saved `controllerPos`).
pub(crate) fn controller_of(nbt: &Compound) -> [i32; 3] {
    [
        read_int(nbt, "controllerX"),
        read_int(nbt, "controllerY"),
        read_int(nbt, "controllerZ"),
    ]
}

pub(crate) fn store_controller(nbt: &mut Compound, controller: [i32; 3]) {
    write_int(nbt, "controllerX", controller[0]);
    write_int(nbt, "controllerY", controller[1]);
    write_int(nbt, "controllerZ", controller[2]);
}

/// The receiver's declared FE buffer: a 100,000,000 FE store that takes nothing
/// and gives 50,000,000 a call, reachable from below only — upstream registers
/// `Capabilities.EnergyStorage.BLOCK` on `Direction.DOWN`.
pub(crate) fn energy_buffer() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: RAY_RECEIVER_POWER_BUFFER,
        max_insert: 0,
        max_extract: RAY_RECEIVER_EXTRACT_POWER,
        insert_faces: [false; 6],
        // `Direction::ALL` is Down, Up, North, South, West, East.
        extract_faces: [true, false, false, false, false, false],
    }
}

/// The level the server is ticking. A block-entity tick is not told its level
/// (`BlockEntityTickEvent` carries only the position), so the level in place is
/// the one that is ticking.
pub(crate) fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// The block's registry id at `pos`, or none where the chunk is not loaded.
fn block_id(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    let state = level.block_state(host, pos)?;
    host.block_state_string(state)
}

fn is_block(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3], id: &str) -> bool {
    block_id(host, level, pos).as_deref() == Some(id)
}

/// `MultiblockStructure.validateSpace` over the live level host: every cell of
/// the 3×6×3 box must be air, the controller's own cell included. A cell that
/// cannot be read is refused rather than assumed clear.
pub(crate) fn placement_allowed(level: &dyn LevelHost, anchor: [i32; 3]) -> bool {
    validate_space(
        |pos| {
            level
                .block_state(pos)
                .and_then(|state| level.state_block(state))
                .is_some_and(is_air)
        },
        anchor,
    )
    .is_ok()
}

/// `setPlacedBy`: the sixteen placeholders around a fresh controller, each
/// pointing back at it. Answers how many were written.
pub(crate) fn form_shell(host: &mut dyn ServerHost, level: LevelRef, anchor: [i32; 3]) -> usize {
    let Some(structure) = host
        .block_key(STRUCTURE)
        .and_then(|key| host.block_state_default(key))
    else {
        return 0;
    };
    let mut written = 0;
    for at in structure_positions(anchor) {
        if !level
            .set_block(host, at, structure, update_all())
            .unwrap_or(false)
        {
            continue;
        }
        if let Some(entity) = host.block_entity(level, at) {
            let mut data = Compound::new();
            store_controller(&mut data, anchor);
            let _ = entity.merge_data(host, data);
        }
        written += 1;
    }
    written
}

/// The controller this placeholder belongs to, if the block there is one of ours.
fn shell_controller(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<[i32; 3]> {
    if !is_block(host, level, pos, STRUCTURE) {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let data = entity.data(host)?;
    Some(controller_of(&data))
}

/// How many of the sixteen placeholders this controller still owns.
pub(crate) fn placeholder_count(
    host: &dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
) -> usize {
    structure_positions(controller)
        .iter()
        .filter(|at| shell_controller(host, level, **at) == Some(controller))
        .count()
}

/// Sets this controller's placeholders back to air, except `keep` (the cell the
/// game is removing itself). Answers how many were cleared.
pub(crate) fn clear_shell(
    host: &mut dyn ServerHost,
    level: LevelRef,
    controller: [i32; 3],
    keep: Option<[i32; 3]>,
) -> usize {
    let Some(air) = host.block_state_id(AIR) else {
        return 0;
    };
    let mut cleared = 0;
    for at in structure_positions(controller) {
        if Some(at) == keep || shell_controller(host, level, at) != Some(controller) {
            continue;
        }
        if level
            .set_block(host, at, air, update_all())
            .unwrap_or(false)
        {
            cleared += 1;
        }
    }
    cleared
}

/// A broken placeholder drops its controller, as `MultiblockStructureBlock.
/// `onRemove` calls `world.destroyBlock(controllerPos, true)`.
pub(crate) fn break_placeholder(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    let Some(controller) = shell_controller(host, level, pos) else {
        return;
    };
    if !is_block(host, level, controller, CONTROLLER) {
        return;
    }
    clear_shell(host, level, controller, Some(pos));
    let _ = level.destroy_block(host, controller, true, None, false);
}

/// A broken controller clears its placeholders (`RayReceiverControllerBlock`'s
/// `onRemove`).
pub(crate) fn break_controller(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    clear_shell(host, level, pos, None);
}

/// Vanilla's `MOTION_BLOCKING` for one cell: a collision box, or a fluid.
fn blocks_motion(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    if level
        .fluid_state(host, pos)
        .is_some_and(|fluid| !fluid.is_empty())
    {
        return true;
    }
    let Some(state) = level.block_state(host, pos) else {
        return false;
    };
    level
        .block_shape(host, state, pos, BlockShapeKind::Collision)
        .is_some_and(|boxes| !boxes.is_empty())
}

/// The world facts `Level.isDay/isRaining/canSeeSky` read.
pub(crate) fn sky_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Sky {
    let (lowest, height) = level.height(host).unwrap_or((0, 0));
    Sky {
        overworld: level.dimension() == OVERWORLD,
        day_time: level.day_time(host).unwrap_or(0),
        rain: level.weather(host).map_or(0.0, |(rain, _)| rain),
        open_to_sky: open_to_sky(|at| blocks_motion(host, level, at), pos, lowest + height),
    }
}

/// One server tick of the controller (`RayReceiverBlockEntity.serverTick`):
/// generate, pull, then give what the buffer offers to the block below, whose
/// energy capability is asked through its up face as vanilla's
/// `getCapability(…, pos.below(), Direction.UP)` does.
pub(crate) fn receiver_tick(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
) {
    let sky = sky_at(host, level, pos);
    let mut sphere = sphere_of(nbt);
    let mut buffer = Buffer::new(read_int(nbt, "energy"));
    let _ = tick(&mut buffer, &mut sphere, sky);
    let offer = buffer.offer_down();
    let mut down = 0;
    if offer > 0 {
        let below = [pos[0], pos[1] - 1, pos[2]];
        // The block below's energy handler through its up face, as vanilla's
        // `getCapability(…, pos.below(), Direction.UP)` asks it.
        if let Some(handler) = level.capability(host, &ENERGY, below, Some(Direction::Up)) {
            let amount = u32::try_from(offer).unwrap_or(u32::MAX);
            // One transactional insert: it answers what went in, which is what the
            // buffer gives up (0.1.0 simulated and then committed, two calls). The
            // transaction cannot fail, so its `Infallible` error never exists.
            let accepted = host
                .transaction(|host, tx| {
                    Ok::<u32, core::convert::Infallible>(handler.insert(host, tx, amount))
                })
                .unwrap_or_else(|never| match never {});
            down = buffer.take_down(i32::try_from(accepted).unwrap_or(i32::MAX));
        }
    }
    store_sphere(nbt, &sphere);
    write_int(nbt, "energy", buffer.energy);
    write_int(nbt, "down", down);
}

/// The controller's readout: the six lines of upstream's `DysonProgressGuiAddon`
/// (`Dyson Information`, `Progress`, `Power Gen`, `Power Con`, `Beams`, `Sails`)
/// plus the buffer the GUI's energy bar draws.
pub(crate) fn status_text(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> String {
    let entity = host.block_entity(level, pos);
    let data = entity.as_ref().and_then(|entity| entity.data(host));
    if !is_block(host, level, pos, CONTROLLER) {
        return format!("No ray receiver at {}, {}, {}.", pos[0], pos[1], pos[2]);
    }
    let Some(data) = data else {
        return format!("No ray receiver at {}, {}, {}.", pos[0], pos[1], pos[2]);
    };
    let sphere = sphere_of(&data);
    let placeholders = placeholder_count(host, level, pos);
    let box_size = volume(pos).size();
    let formed = if placeholders == PLACEHOLDERS {
        "formed"
    } else {
        "not formed"
    };
    format!(
        "Ray Receiver at {},{},{}: {formed}, box {}x{}x{}, structure {placeholders}/{PLACEHOLDERS}, beams {}, sails {}/{}, progress {}%, gen {} FE/t, con {} FE/t, stored {}, buffer {}/{} FE, down {}, t={}",
        pos[0],
        pos[1],
        pos[2],
        box_size[0],
        box_size[1],
        box_size[2],
        sphere.beams,
        sphere.solar_panels,
        sphere.max_solar_panels(),
        (sphere.progress() * 100.0).round() as i64,
        sphere.generation_per_tick(),
        sphere.last_consumed_power,
        sphere.stored_power,
        read_int(&data, "energy").max(0),
        RAY_RECEIVER_POWER_BUFFER,
        read_int(&data, "down").max(0),
        level.game_time(host).unwrap_or(0)
    )
}

/// Upstream's `set`/`add` on the sphere this controller holds. The command
/// addresses the controller: the SDK has no level saved data to key a sphere by
/// its player (`../notes.md`, deviations).
pub(crate) fn change_sphere(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    stat: Stat,
    value: i32,
    add: bool,
) -> String {
    let changed = change_sphere_at(host, level, pos, stat, value, add);
    let name = match stat {
        Stat::Beams => "beams",
        Stat::Panels => "solar panels",
    };
    match changed {
        Some(now) if add => format!("{name} at {}, {}, {} is now {now}", pos[0], pos[1], pos[2]),
        Some(now) => format!("Set {name} at {}, {}, {} to {now}", pos[0], pos[1], pos[2]),
        None => format!("No ray receiver at {}, {}, {}.", pos[0], pos[1], pos[2]),
    }
}

fn change_sphere_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    stat: Stat,
    value: i32,
    add: bool,
) -> Option<i32> {
    let entity = host.block_entity(level, pos)?;
    let data = entity.data(host)?;
    let mut sphere = sphere_of(&data);
    let now = match (stat, add) {
        (Stat::Beams, false) => sphere.set_beams(value),
        (Stat::Beams, true) => sphere.add_beams(value),
        (Stat::Panels, false) => sphere.set_solar_panels(value),
        (Stat::Panels, true) => sphere.add_solar_panels(value),
    };
    let mut patch = Compound::new();
    store_sphere(&mut patch, &sphere);
    match entity.merge_data(host, patch) {
        Ok(_) => Some(now),
        Err(_) => None,
    }
}

/// One `/dysoncubeproject` line, after the dispatcher resolved its typed
/// arguments (0.1.0 split the raw words itself; the SDK hands over the parsed
/// form, so no string is re-parsed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Invocation {
    /// One of upstream's three forms.
    Command(SphereCommand),
    /// Ours, but not one of its forms.
    Usage,
    /// Another command: the mod leaves it alone.
    NotOurs,
}

pub(crate) fn invocation(literals: &[String], arguments: &[CommandArgument]) -> Invocation {
    let words = literals.iter().map(String::as_str).collect::<Vec<_>>();
    let ["dysoncubeproject", rest @ ..] = words.as_slice() else {
        return Invocation::NotOurs;
    };
    let position = || match argument(arguments, "pos") {
        Some(CommandArgumentValue::BlockPos(pos)) => Some(*pos),
        _ => None,
    };
    match *rest {
        ["status"] => match position() {
            Some(pos) => Invocation::Command(SphereCommand::Status { pos }),
            None => Invocation::Usage,
        },
        [change @ ("set" | "add"), stat] => {
            let Some(stat) = stat_of(stat) else {
                return Invocation::Usage;
            };
            let (Some(pos), Some(CommandArgumentValue::Integer(value))) = (
                position(),
                argument(arguments, "value").or_else(|| argument(arguments, "delta")),
            ) else {
                return Invocation::Usage;
            };
            Invocation::Command(if change == "set" {
                SphereCommand::Set {
                    stat,
                    pos,
                    value: *value,
                }
            } else {
                SphereCommand::Add {
                    stat,
                    pos,
                    value: *value,
                }
            })
        }
        _ => Invocation::Usage,
    }
}

fn stat_of(word: &str) -> Option<Stat> {
    match word {
        "beams" => Some(Stat::Beams),
        "panels" => Some(Stat::Panels),
        _ => None,
    }
}

/// The last argument registered under `name`.
fn argument<'a>(arguments: &'a [CommandArgument], name: &str) -> Option<&'a CommandArgumentValue> {
    arguments
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

/// Answers one `/dysoncubeproject` line: `true` when it was ours.
pub(crate) fn run_command(
    host: &mut dyn ServerHost,
    level: LevelRef,
    literals: &[String],
    arguments: &[CommandArgument],
) -> bool {
    let message = match invocation(literals, arguments) {
        Invocation::NotOurs => return false,
        Invocation::Usage => USAGE.to_owned(),
        Invocation::Command(SphereCommand::Status { pos }) => status_text(host, level, pos),
        Invocation::Command(SphereCommand::Set { stat, pos, value }) => {
            change_sphere(host, level, pos, stat, value, false)
        }
        Invocation::Command(SphereCommand::Add { stat, pos, value }) => {
            change_sphere(host, level, pos, stat, value, true)
        }
    };
    // Routed to the command's source, as vanilla's own feedback is.
    let _ = host.commands().feedback(Text::literal(message));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_sphere_reads_back() {
        let mut nbt = Compound::new();
        let mut sphere = Sphere::new();
        sphere.beams = 17;
        sphere.solar_panels = 100;
        sphere.stored_power = 2_000;
        sphere.last_consumed_power = 1_500;
        store_sphere(&mut nbt, &sphere);
        assert_eq!(sphere_of(&nbt), sphere);
        assert_eq!(read_int(&nbt, "solarPanels"), 100);
    }

    #[test]
    fn an_empty_compound_is_an_empty_sphere() {
        assert_eq!(sphere_of(&Compound::new()), Sphere::new());
    }

    #[test]
    fn a_negative_saved_sphere_reads_as_zero() {
        let mut nbt = Compound::new();
        write_int(&mut nbt, "beams", -4);
        write_int(&mut nbt, "solarPanels", -9);
        assert_eq!(sphere_of(&nbt).beams, 0);
        assert_eq!(sphere_of(&nbt).solar_panels, 0);
    }

    #[test]
    fn the_controller_position_round_trips() {
        let mut nbt = Compound::new();
        store_controller(&mut nbt, [100, 140, -7]);
        assert_eq!(
            controller_of(&nbt),
            [100, 140, -7],
            "the placeholder's saved controller position"
        );
    }

    #[test]
    fn the_sixteen_placeholders_are_the_upstream_set() {
        assert_eq!(structure_positions([0, 0, 0]).len(), PLACEHOLDERS);
        assert_eq!(PLACEHOLDERS, 16);
    }

    #[test]
    fn the_buffer_is_upstreams_down_only_100m() {
        let buffer = energy_buffer();
        assert_eq!(buffer.capacity, RAY_RECEIVER_POWER_BUFFER);
        assert_eq!(buffer.max_insert, 0);
        assert_eq!(buffer.max_extract, RAY_RECEIVER_EXTRACT_POWER);
        assert_eq!(
            buffer.extract_faces,
            [true, false, false, false, false, false],
            "only the down face (Direction 0) gives"
        );
        assert_eq!(buffer.insert_faces, [false; 6]);
    }

    fn line(literals: &[&str], arguments: &[(&str, CommandArgumentValue)]) -> Invocation {
        let literals = literals
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>();
        let arguments = arguments
            .iter()
            .map(|(name, value)| CommandArgument::new((*name).to_owned(), value.clone()))
            .collect::<Vec<_>>();
        invocation(&literals, &arguments)
    }

    #[test]
    fn the_three_forms_map_onto_the_runtime() {
        assert_eq!(
            line(
                &["dysoncubeproject", "status"],
                &[("pos", CommandArgumentValue::BlockPos([100, 140, -7]))]
            ),
            Invocation::Command(SphereCommand::Status {
                pos: [100, 140, -7]
            })
        );
        assert_eq!(
            line(
                &["dysoncubeproject", "set", "beams"],
                &[
                    ("pos", CommandArgumentValue::BlockPos([1, 2, 3])),
                    ("value", CommandArgumentValue::Integer(17)),
                ]
            ),
            Invocation::Command(SphereCommand::Set {
                stat: Stat::Beams,
                pos: [1, 2, 3],
                value: 17,
            })
        );
        assert_eq!(
            line(
                &["dysoncubeproject", "add", "panels"],
                &[
                    ("pos", CommandArgumentValue::BlockPos([1, 2, 3])),
                    ("delta", CommandArgumentValue::Integer(-6)),
                ]
            ),
            Invocation::Command(SphereCommand::Add {
                stat: Stat::Panels,
                pos: [1, 2, 3],
                value: -6,
            })
        );
    }

    #[test]
    fn another_command_is_left_alone_and_a_short_one_is_usage() {
        assert_eq!(line(&["other"], &[]), Invocation::NotOurs);
        assert_eq!(line(&[], &[]), Invocation::NotOurs);
        assert_eq!(line(&["dysoncubeproject"], &[]), Invocation::Usage);
        assert_eq!(
            line(&["dysoncubeproject", "set", "sails"], &[]),
            Invocation::Usage
        );
        // A registered form with an argument of the wrong kind is a usage error,
        // not a panic and not another command's.
        assert_eq!(
            line(
                &["dysoncubeproject", "status"],
                &[("pos", CommandArgumentValue::Integer(1))]
            ),
            Invocation::Usage
        );
        assert_eq!(
            line(
                &["dysoncubeproject", "set", "beams"],
                &[("pos", CommandArgumentValue::BlockPos([1, 2, 3]))]
            ),
            Invocation::Usage
        );
    }

    #[test]
    fn the_usage_line_is_upstreams() {
        assert!(USAGE.starts_with("Usage: /dysoncubeproject status"));
    }
}
