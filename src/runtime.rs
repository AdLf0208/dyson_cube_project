// Pure model of the Dyson Cube Project ray receiver (slice 1).
//
// This file is both the standalone Cargo library and the injected
// `crate::game::dysoncubeproject` module. Every number comes from the mod's own
// `Config.java`, and every rule from the class named beside it; `FINDINGS.md`
// records the file each one was read from.

/// The multiblock's footprint, from `RayReceiverControllerBlock.MULTIBLOCK_STRUCTURE`
/// (`new MultiblockStructure(3, 6, 3, SHAPE)`).
pub const SIZE_X: i32 = 3;
pub const SIZE_Y: i32 = 6;
pub const SIZE_Z: i32 = 3;

/// `Config.MAX_SOLAR_PANELS`: the most sails a sphere may hold.
pub const MAX_SOLAR_PANELS: i32 = 50_000_000;
/// `Config.BEAM_TO_SOLAR_PANEL_RATIO`: sails one beam supports.
pub const BEAM_TO_SOLAR_PANEL_RATIO: i32 = 6;
/// `Config.POWER_PER_SAIL`: FE one sail makes each tick.
pub const POWER_PER_SAIL: i32 = 20;
/// `Config.RAY_RECEIVER_EXTRACT_POWER`: FE a receiver pulls and pushes each tick.
pub const RAY_RECEIVER_EXTRACT_POWER: i32 = 50_000_000;
/// `Config.RAY_RECEIVER_POWER_BUFFER`: the receiver's own buffer.
pub const RAY_RECEIVER_POWER_BUFFER: i32 = 100_000_000;

/// Vanilla's `Level.isRaining` threshold on `getRainLevel`.
const RAIN_LEVEL_RAINING: f32 = 0.2;
/// A tick count in a day; vanilla's `Level.TICKS_PER_DAY`.
const TICKS_PER_DAY: i64 = 24_000;
/// The first tick of the night, where `Level.isDay` stops holding.
const NIGHT_START: i64 = 12_000;

/// A block box, as `MultiblockStructure.getAABB` bounds the structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volume {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl Volume {
    /// The box's size in blocks, one per axis.
    #[must_use]
    pub fn size(&self) -> [i32; 3] {
        [
            self.max[0] - self.min[0] + 1,
            self.max[1] - self.min[1] + 1,
            self.max[2] - self.min[2] + 1,
        ]
    }
}

/// A cell that stops the structure from forming.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocked {
    pub pos: [i32; 3],
}

impl std::fmt::Display for Blocked {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the block at {}, {}, {} is in the way",
            self.pos[0], self.pos[1], self.pos[2]
        )
    }
}

impl std::error::Error for Blocked {}

/// Vanilla's `BlockState.isAir`: the three air blocks.
#[must_use]
pub fn is_air(name: &str) -> bool {
    matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

/// The box a controller at `anchor` reserves: it sits at the bottom-centre of
/// the footprint (`MultiblockStructure.validateSpace`).
#[must_use]
pub fn volume(anchor: [i32; 3]) -> Volume {
    let half_x = SIZE_X / 2;
    let half_z = SIZE_Z / 2;
    Volume {
        min: [anchor[0] - half_x, anchor[1], anchor[2] - half_z],
        max: [
            anchor[0] - half_x + SIZE_X - 1,
            anchor[1] + SIZE_Y - 1,
            anchor[2] - half_z + SIZE_Z - 1,
        ],
    }
}

/// `MultiblockStructure.validateSpace`: every cell of the box must be air,
/// the controller's own cell included. `is_air` is asked per position.
///
/// # Errors
/// The first cell that is not air, bottom layer first.
pub fn validate_space(
    is_air: impl Fn([i32; 3]) -> bool,
    anchor: [i32; 3],
) -> Result<Volume, Blocked> {
    let volume = volume(anchor);
    for y in volume.min[1]..=volume.max[1] {
        for x in volume.min[0]..=volume.max[0] {
            for z in volume.min[2]..=volume.max[2] {
                let pos = [x, y, z];
                if !is_air(pos) {
                    return Err(Blocked { pos });
                }
            }
        }
    }
    Ok(volume)
}

/// The placeholder blocks `RayReceiverControllerBlock.setPlacedBy` writes, in
/// its order and without its one repeat: the four sides of the base, four
/// above the controller, and the 3×3 ring at y + 2.
#[must_use]
pub fn structure_positions(anchor: [i32; 3]) -> [[i32; 3]; 16] {
    let at = |dx: i32, dy: i32, dz: i32| [anchor[0] + dx, anchor[1] + dy, anchor[2] + dz];
    [
        at(-1, 0, 0),
        at(1, 0, 0),
        at(0, 0, -1),
        at(0, 0, 1),
        at(0, 1, 0),
        at(0, 2, 0),
        at(0, 3, 0),
        at(0, 4, 0),
        at(-1, 2, -1),
        at(-1, 2, 0),
        at(-1, 2, 1),
        at(0, 2, -1),
        at(0, 2, 1),
        at(1, 2, -1),
        at(1, 2, 0),
        at(1, 2, 1),
    ]
}

/// `world/DysonSphereStructure.java`: one sphere's progress and its reservoir.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sphere {
    pub beams: i32,
    pub solar_panels: i32,
    pub stored_power: i32,
    pub last_consumed_power: i32,
}

impl Sphere {
    /// An empty sphere.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `getMaxSolarPanels`: the sails the sphere's beams support.
    #[must_use]
    pub fn max_solar_panels(&self) -> i32 {
        MAX_SOLAR_PANELS.min(self.beams.saturating_mul(BEAM_TO_SOLAR_PANEL_RATIO))
    }

    /// `getMaxBeams`: the beams a full sphere needs, rounded up as upstream does.
    #[must_use]
    pub fn max_beams(&self) -> i32 {
        // `Math.ceil(MAX / ratio)`, as upstream rounds up.
        (MAX_SOLAR_PANELS + BEAM_TO_SOLAR_PANEL_RATIO - 1) / BEAM_TO_SOLAR_PANEL_RATIO
    }

    /// `getProgress`: the share of `MAX_SOLAR_PANELS` the sphere holds, 0 to 1.
    #[must_use]
    pub fn progress(&self) -> f64 {
        f64::from(self.solar_panels) / f64::from(MAX_SOLAR_PANELS)
    }

    /// `setBeams`, clamped to `0..=max_beams`. Answers what it holds now.
    pub fn set_beams(&mut self, value: i32) -> i32 {
        self.beams = value.clamp(0, self.max_beams());
        self.beams
    }

    /// `setSolarPanels`, clamped to `0..=max_solar_panels`.
    pub fn set_solar_panels(&mut self, value: i32) -> i32 {
        self.solar_panels = value.clamp(0, self.max_solar_panels());
        self.solar_panels
    }

    /// `increaseBeams`, clamped the same way.
    pub fn add_beams(&mut self, delta: i32) -> i32 {
        self.set_beams(self.beams.saturating_add(delta))
    }

    /// `increaseSolarPanels`, clamped the same way.
    pub fn add_solar_panels(&mut self, delta: i32) -> i32 {
        self.set_solar_panels(self.solar_panels.saturating_add(delta))
    }

    /// What one tick of the sphere makes: `sails * POWER_PER_SAIL`.
    #[must_use]
    pub fn generation_per_tick(&self) -> i32 {
        self.solar_panels.saturating_mul(POWER_PER_SAIL)
    }

    /// `generatePower`: the reservoir refills to one tick of output, never more.
    pub fn generate_power(&mut self) {
        self.last_consumed_power = 0;
        let generation = self.generation_per_tick();
        self.stored_power = generation.min(self.stored_power.saturating_add(generation));
    }

    /// `extractPower`: what the sphere gives of `amount`, and what it keeps.
    pub fn extract_power(&mut self, amount: i32) -> i32 {
        let extracted = amount.max(0).min(self.stored_power.max(0));
        self.stored_power -= extracted;
        self.last_consumed_power = self.last_consumed_power.saturating_add(extracted);
        extracted
    }
}

/// The world facts the receiver's tick reads: `Level.isDay`, `Level.isRaining`,
/// `Level.canSeeSky(pos.above())` and the dimension generation runs in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// The Overworld: the only level whose tick generates sphere power.
    pub overworld: bool,
    /// `native_day_time`.
    pub day_time: i64,
    /// `native_weather().0`, the rain level.
    pub rain: f32,
    /// Nothing solid above the machine but its own shell (`open_to_sky`).
    pub open_to_sky: bool,
}

impl Sky {
    /// `Level.isDay`: the first half of a day, and never in a fixed-time level.
    #[must_use]
    pub fn is_day(&self) -> bool {
        self.overworld && self.day_time.rem_euclid(TICKS_PER_DAY) < NIGHT_START
    }

    /// `Level.isRaining`.
    #[must_use]
    pub fn is_raining(&self) -> bool {
        self.rain > RAIN_LEVEL_RAINING
    }

    /// `Level.canSeeSky(pos.above())`, as `open_to_sky` answers it.
    #[must_use]
    pub fn sees_sky(&self) -> bool {
        self.open_to_sky
    }

    /// Whether this tick's conditions let the receiver pull from its sphere.
    #[must_use]
    pub fn harvests(&self) -> bool {
        self.overworld && self.is_day() && !self.is_raining() && self.sees_sky()
    }
}

/// Whether the machine sees the sky: the column above it, from `anchor[1] + 1`
/// up to `top`, is clear — with the machine's own shell skipped. `blocks_motion`
/// is vanilla's `MOTION_BLOCKING` for a cell (a collision box, or a fluid).
pub fn open_to_sky(blocks_motion: impl Fn([i32; 3]) -> bool, anchor: [i32; 3], top: i32) -> bool {
    let shell = structure_positions(anchor);
    (anchor[1] + 1..top).all(|y| {
        let pos = [anchor[0], y, anchor[2]];
        // The machine's own shell is transparent to its own receiver; anything
        // else that blocks movement (or holds a fluid) shadows it.
        shell.contains(&pos) || !blocks_motion(pos)
    })
}

/// The receiver's own buffer, the int `energy` in its saved compound (API 142).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buffer {
    pub energy: i32,
}

impl Buffer {
    /// A buffer holding `energy`, clamped to the receiver's capacity.
    #[must_use]
    pub fn new(energy: i32) -> Self {
        Self {
            energy: energy.clamp(0, RAY_RECEIVER_POWER_BUFFER),
        }
    }

    /// The room left before the buffer is full.
    #[must_use]
    pub fn room(&self) -> i32 {
        RAY_RECEIVER_POWER_BUFFER.saturating_sub(self.energy)
    }

    /// Fills up to `amount`, answering what went in.
    pub fn insert(&mut self, amount: i32) -> i32 {
        let taken = amount.max(0).min(self.room());
        self.energy += taken;
        taken
    }

    /// The most this tick may hand to the block below, `min(extract rate, held)`.
    #[must_use]
    pub fn offer_down(&self) -> i32 {
        self.energy.min(RAY_RECEIVER_EXTRACT_POWER)
    }

    /// Takes what the block below accepted, never more than the buffer holds.
    pub fn take_down(&mut self, accepted: i32) -> i32 {
        let taken = accepted.max(0).min(self.energy);
        self.energy -= taken;
        taken
    }
}

/// What one tick of a receiver did, for the readout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Harvest {
    /// What the sphere made this tick (`sails * POWER_PER_SAIL`).
    pub generated: i32,
    /// What moved from the sphere into the buffer.
    pub extracted: i32,
}

/// `RayReceiverBlockEntity.serverTick` without the world: the sphere generates,
/// then the receiver pulls what its buffer has room for when the sky allows.
pub fn tick(buffer: &mut Buffer, sphere: &mut Sphere, sky: Sky) -> Harvest {
    if !sky.overworld {
        return Harvest::default();
    }
    sphere.generate_power();
    let generated = sphere.generation_per_tick();
    let mut extracted = 0;
    if sky.harvests() {
        extracted = sphere.extract_power(RAY_RECEIVER_EXTRACT_POWER.min(buffer.room()));
        buffer.insert(extracted);
    }
    Harvest {
        generated,
        extracted,
    }
}

/// What `/dysoncubeproject set|add` changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Beams,
    Panels,
}

/// One `/dysoncubeproject` command, parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Status {
        pos: [i32; 3],
    },
    Set {
        stat: Stat,
        pos: [i32; 3],
        value: i32,
    },
    Add {
        stat: Stat,
        pos: [i32; 3],
        value: i32,
    },
}

/// Why a line is not a command of this mod's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// Another command: the mod leaves it alone.
    NotOurs,
    /// Ours, but not one of its forms.
    BadUsage,
}

/// What a refused command answers with (upstream has no usage line).
pub const USAGE: &str =
    "Usage: /dysoncubeproject status <x> <y> <z>, set|add beams|panels <x> <y> <z> <value>";

pub fn parse_command(words: &[&str]) -> Result<Command, CommandError> {
    let Some((head, rest)) = words.split_first() else {
        return Err(CommandError::NotOurs);
    };
    if *head != "dysoncubeproject" {
        return Err(CommandError::NotOurs);
    }
    match rest {
        ["status", x, y, z] => Ok(Command::Status {
            pos: parse_pos([*x, *y, *z])?,
        }),
        ["set", stat, x, y, z, value] => change(Change::Set, stat, [x, y, z], value),
        ["add", stat, x, y, z, value] => change(Change::Add, stat, [x, y, z], value),
        _ => Err(CommandError::BadUsage),
    }
}

/// Which of upstream's two mutating forms a line is.
#[derive(Clone, Copy)]
enum Change {
    Set,
    Add,
}

fn parse_pos([x, y, z]: [&str; 3]) -> Result<[i32; 3], CommandError> {
    Ok([
        x.parse().map_err(|_| CommandError::BadUsage)?,
        y.parse().map_err(|_| CommandError::BadUsage)?,
        z.parse().map_err(|_| CommandError::BadUsage)?,
    ])
}

fn change(
    change: Change,
    stat: &str,
    pos: [&str; 3],
    value: &str,
) -> Result<Command, CommandError> {
    let stat = match stat {
        "beams" => Stat::Beams,
        "panels" => Stat::Panels,
        _ => return Err(CommandError::BadUsage),
    };
    let value: i32 = value.parse().map_err(|_| CommandError::BadUsage)?;
    let pos = parse_pos(pos)?;
    Ok(match change {
        Change::Set => Command::Set { stat, pos, value },
        Change::Add => Command::Add { stat, pos, value },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn air_at(_pos: [i32; 3]) -> bool {
        true
    }

    fn noon() -> Sky {
        Sky {
            overworld: true,
            day_time: 6_000,
            rain: 0.0,
            open_to_sky: true,
        }
    }

    #[test]
    fn an_empty_volume_forms() {
        let volume = validate_space(air_at, [100, 140, 0]).expect("all air forms");
        assert_eq!(volume.min, [99, 140, -1]);
        assert_eq!(volume.max, [101, 145, 1]);
        assert_eq!(volume.size(), [3, 6, 3]);
    }

    #[test]
    fn a_block_in_the_volume_refuses() {
        let blocked = [100, 143, 0];
        let answer = validate_space(|pos| pos != blocked, [100, 140, 0]);
        assert_eq!(answer, Err(Blocked { pos: blocked }));
        assert_eq!(
            answer.unwrap_err().to_string(),
            "the block at 100, 143, 0 is in the way"
        );
    }

    #[test]
    fn the_controller_cell_itself_must_be_air() {
        let anchor = [100, 140, 0];
        assert_eq!(
            validate_space(|pos| pos != anchor, anchor),
            Err(Blocked { pos: anchor }),
            "upstream validates the cell the controller goes in"
        );
    }

    #[test]
    fn a_block_outside_the_volume_does_not_refuse() {
        let anchor = [100, 140, 0];
        let outside = [102, 140, 0];
        assert!(validate_space(|pos| pos != outside, anchor).is_ok());
        let above = [100, 146, 0];
        assert!(validate_space(|pos| pos != above, anchor).is_ok());
    }

    #[test]
    fn the_sixteen_structure_blocks() {
        let anchor = [100, 140, 0];
        let positions = structure_positions(anchor);
        let unique: std::collections::BTreeSet<_> = positions.iter().collect();
        assert_eq!(unique.len(), 16, "upstream writes 17 and repeats one");
        let volume = volume(anchor);
        for pos in positions {
            assert!(
                (volume.min[0]..=volume.max[0]).contains(&pos[0])
                    && (volume.min[1]..=volume.max[1]).contains(&pos[1])
                    && (volume.min[2]..=volume.max[2]).contains(&pos[2]),
                "{pos:?} is outside the 3x6x3 box"
            );
            assert_ne!(pos, anchor, "the controller keeps its own cell");
        }
        assert_eq!(positions[0], [99, 140, 0]);
        assert_eq!(positions[4], [100, 141, 0]);
        assert_eq!(positions[15], [101, 142, 1]);
    }

    #[test]
    fn beams_set_the_sail_ceiling() {
        let mut sphere = Sphere::new();
        assert_eq!(sphere.max_solar_panels(), 0, "no beams, no sails");
        sphere.set_beams(10);
        assert_eq!(sphere.max_solar_panels(), 60);
        sphere.set_beams(sphere.max_beams());
        assert_eq!(sphere.max_solar_panels(), MAX_SOLAR_PANELS);
    }

    #[test]
    fn a_full_sphere_needs_ceiled_beams() {
        let sphere = Sphere::new();
        assert_eq!(sphere.max_beams(), 8_333_334);
        assert!(sphere.max_beams() * BEAM_TO_SOLAR_PANEL_RATIO >= MAX_SOLAR_PANELS);
    }

    #[test]
    fn set_beams_clamps_both_ends() {
        let mut sphere = Sphere::new();
        assert_eq!(sphere.set_beams(-5), 0);
        assert_eq!(sphere.set_beams(12), 12);
        assert_eq!(sphere.set_beams(i32::MAX), 8_333_334);
    }

    #[test]
    fn sails_stop_at_what_the_beams_support() {
        let mut sphere = Sphere::new();
        sphere.set_beams(10);
        assert_eq!(sphere.set_solar_panels(100), 60);
        assert_eq!(sphere.set_solar_panels(-1), 0);
        sphere.set_beams(sphere.max_beams());
        assert_eq!(sphere.set_solar_panels(i32::MAX), MAX_SOLAR_PANELS);
    }

    #[test]
    fn adding_moves_within_the_same_ceiling() {
        let mut sphere = Sphere::new();
        assert_eq!(sphere.add_beams(1), 1);
        assert_eq!(sphere.add_solar_panels(6), 6);
        assert_eq!(sphere.add_solar_panels(100), 6);
        assert_eq!(sphere.add_solar_panels(-100), 0);
        assert_eq!(sphere.add_beams(i32::MAX), 8_333_334);
    }

    #[test]
    fn progress_is_sails_over_the_hard_cap() {
        let mut sphere = Sphere::new();
        assert!((sphere.progress() - 0.0).abs() < f64::EPSILON);
        sphere.set_beams(sphere.max_beams());
        sphere.set_solar_panels(MAX_SOLAR_PANELS / 2);
        assert!((sphere.progress() - 0.5).abs() < 1e-9);
        sphere.set_solar_panels(MAX_SOLAR_PANELS);
        assert!((sphere.progress() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn one_sail_makes_twenty_a_tick() {
        let mut sphere = Sphere::new();
        sphere.set_beams(1);
        sphere.set_solar_panels(6);
        assert_eq!(sphere.generation_per_tick(), 120);
    }

    #[test]
    fn generating_refills_to_exactly_one_tick() {
        let mut sphere = Sphere::new();
        sphere.set_beams(1);
        sphere.set_solar_panels(6);
        sphere.generate_power();
        assert_eq!(sphere.stored_power, 120);
        sphere.generate_power();
        assert_eq!(sphere.stored_power, 120, "never more than one tick");
        sphere.stored_power = 0;
        sphere.generate_power();
        assert_eq!(sphere.stored_power, 120);
    }

    #[test]
    fn generating_resets_the_consumed_meter() {
        let mut sphere = Sphere::new();
        sphere.set_beams(1);
        sphere.set_solar_panels(6);
        sphere.generate_power();
        sphere.extract_power(50);
        assert_eq!(sphere.last_consumed_power, 50);
        sphere.generate_power();
        assert_eq!(sphere.last_consumed_power, 0);
    }

    #[test]
    fn extracting_takes_what_the_sphere_holds() {
        let mut sphere = Sphere::new();
        sphere.set_beams(1);
        sphere.set_solar_panels(6);
        sphere.generate_power();
        assert_eq!(sphere.extract_power(RAY_RECEIVER_EXTRACT_POWER), 120);
        assert_eq!(sphere.stored_power, 0);
        assert_eq!(
            sphere.extract_power(120),
            0,
            "an empty sphere gives nothing"
        );
        assert_eq!(sphere.last_consumed_power, 120);
    }

    #[test]
    fn day_is_the_first_half_of_the_day() {
        let mut sky = noon();
        for time in [0, 1, 6_000, 11_999] {
            sky.day_time = time;
            assert!(sky.is_day(), "t={time} is day");
        }
        for time in [12_000, 18_000, 23_999] {
            sky.day_time = time;
            assert!(!sky.is_day(), "t={time} is night");
        }
        sky.day_time = 24_000;
        assert!(sky.is_day(), "the clock wraps");
        sky.day_time = -18_000;
        assert!(sky.is_day(), "a negative clock wraps to noon");
        sky.day_time = -6_000;
        assert!(!sky.is_day(), "a negative clock wraps to night");
    }

    #[test]
    fn only_the_overworld_harvests() {
        let mut sky = noon();
        assert!(sky.harvests());
        sky.overworld = false;
        assert!(!sky.is_day(), "a fixed-time level has no day");
        assert!(!sky.harvests());
    }

    #[test]
    fn rain_stops_the_harvest() {
        let mut sky = noon();
        sky.rain = 0.1;
        assert!(!sky.is_raining());
        assert!(sky.harvests());
        sky.rain = 0.2;
        assert!(!sky.is_raining(), "vanilla's threshold is above 0.2");
        sky.rain = 0.5;
        assert!(sky.is_raining());
        assert!(!sky.harvests());
    }

    #[test]
    fn a_roof_stops_the_harvest() {
        let mut sky = noon();
        sky.open_to_sky = false;
        assert!(!sky.sees_sky());
        assert!(!sky.harvests());
        sky.open_to_sky = true;
        assert!(sky.harvests());
    }

    #[test]
    fn the_machines_own_shell_does_not_shadow_it() {
        let anchor = [100, 180, 0];
        let shell = structure_positions(anchor);
        let open = open_to_sky(|pos| shell.contains(&pos), anchor, anchor[1] + 60);
        assert!(
            open,
            "the placeholders above the controller are its own shell"
        );
    }

    #[test]
    fn the_literal_upstream_check_could_never_open() {
        // `RayReceiverBlockEntity.serverTick` asks `canSeeSky(pos.above())`, and
        // `setPlacedBy` writes a placeholder into exactly that cell. A literal
        // column check is therefore false for ever: this is the defect the port
        // fixes by skipping the machine's own shell (`../../notes.md`).
        let anchor = [100, 180, 0];
        let shell = structure_positions(anchor);
        let literal =
            (anchor[1] + 1..anchor[1] + 60).all(|y| !shell.contains(&[anchor[0], y, anchor[2]]));
        assert!(
            !literal,
            "the cell above the controller is its own shell, so canSeeSky is false"
        );
    }

    #[test]
    fn a_stone_beside_the_column_does_not_stop_the_sky() {
        let anchor = [100, 180, 0];
        let shell = structure_positions(anchor);
        let stone = [101, 181, 0];
        assert!(!shell.contains(&stone));
        assert!(open_to_sky(
            |pos| pos == stone || shell.contains(&pos),
            anchor,
            anchor[1] + 60
        ));
    }

    #[test]
    fn a_roof_above_the_shell_stops_the_sky() {
        let anchor = [100, 180, 0];
        let shell = structure_positions(anchor);
        let roof = [100, 185, 0];
        assert!(!shell.contains(&roof));
        assert!(!open_to_sky(
            |pos| pos == roof || shell.contains(&pos),
            anchor,
            anchor[1] + 60
        ));
    }

    #[test]
    fn the_scan_starts_above_the_controller_and_stops_at_the_top() {
        let anchor = [100, 180, 0];
        assert!(open_to_sky(|pos| pos == anchor, anchor, anchor[1] + 60));
        assert!(open_to_sky(
            |pos| pos[1] >= anchor[1] + 40,
            anchor,
            anchor[1] + 40
        ));
    }

    #[test]
    fn a_hundred_sails_fill_the_buffer_at_two_thousand_a_tick() {
        let mut sphere = Sphere::new();
        sphere.set_beams(17);
        sphere.set_solar_panels(100);
        let mut buffer = Buffer::default();
        for _ in 0..100 {
            let harvest = tick(&mut buffer, &mut sphere, noon());
            assert_eq!(harvest.generated, 2_000);
            assert_eq!(harvest.extracted, 2_000);
        }
        assert_eq!(buffer.energy, 200_000);
    }

    #[test]
    fn the_extraction_rate_caps_at_fifty_million() {
        let mut sphere = Sphere::new();
        sphere.set_beams(sphere.max_beams());
        sphere.set_solar_panels(MAX_SOLAR_PANELS);
        let mut buffer = Buffer::default();
        let harvest = tick(&mut buffer, &mut sphere, noon());
        assert_eq!(harvest.generated, 1_000_000_000);
        assert_eq!(harvest.extracted, RAY_RECEIVER_EXTRACT_POWER);
        assert_eq!(buffer.energy, 50_000_000);
    }

    #[test]
    fn the_buffer_stops_at_its_capacity() {
        let mut sphere = Sphere::new();
        sphere.set_beams(sphere.max_beams());
        sphere.set_solar_panels(MAX_SOLAR_PANELS);
        let mut buffer = Buffer::default();
        for _ in 0..3 {
            tick(&mut buffer, &mut sphere, noon());
        }
        assert_eq!(buffer.energy, RAY_RECEIVER_POWER_BUFFER);
        let harvest = tick(&mut buffer, &mut sphere, noon());
        assert_eq!(harvest.extracted, 0, "a full buffer takes nothing");
        assert_eq!(buffer.energy, RAY_RECEIVER_POWER_BUFFER);
    }

    #[test]
    fn night_still_generates_but_fills_nothing() {
        let mut sphere = Sphere::new();
        sphere.set_beams(1);
        sphere.set_solar_panels(6);
        let mut buffer = Buffer::default();
        let mut sky = noon();
        sky.day_time = 18_000;
        let harvest = tick(&mut buffer, &mut sphere, sky);
        assert_eq!(harvest.generated, 120, "upstream generates at night too");
        assert_eq!(harvest.extracted, 0);
        assert_eq!(buffer.energy, 0);
        assert_eq!(sphere.stored_power, 120);
    }

    #[test]
    fn the_nether_and_the_end_generate_nothing() {
        let mut sphere = Sphere::new();
        sphere.set_beams(sphere.max_beams());
        sphere.set_solar_panels(MAX_SOLAR_PANELS);
        let mut buffer = Buffer::default();
        let mut sky = noon();
        sky.overworld = false;
        let harvest = tick(&mut buffer, &mut sphere, sky);
        assert_eq!(harvest, Harvest::default());
        assert_eq!(sphere.stored_power, 0);
        assert_eq!(buffer.energy, 0);
    }

    #[test]
    fn an_empty_sphere_makes_nothing() {
        let mut sphere = Sphere::new();
        let mut buffer = Buffer::default();
        let harvest = tick(&mut buffer, &mut sphere, noon());
        assert_eq!(harvest.generated, 0);
        assert_eq!(harvest.extracted, 0);
        assert_eq!(buffer.energy, 0);
    }

    #[test]
    fn the_offer_down_is_capped_and_conserved() {
        let mut buffer = Buffer::new(60_000_000);
        assert_eq!(buffer.offer_down(), RAY_RECEIVER_EXTRACT_POWER);
        assert_eq!(buffer.take_down(0), 0);
        assert_eq!(buffer.energy, 60_000_000, "nothing accepted, nothing lost");
        assert_eq!(buffer.take_down(10_000_000), 10_000_000);
        assert_eq!(buffer.energy, 50_000_000);
        assert_eq!(
            buffer.take_down(i32::MAX),
            50_000_000,
            "never more than held"
        );
        assert_eq!(buffer.energy, 0);
    }

    #[test]
    fn a_small_buffer_offers_only_what_it_holds() {
        let buffer = Buffer::new(1_234);
        assert_eq!(buffer.offer_down(), 1_234);
    }

    #[test]
    fn status_parses_with_signed_coordinates() {
        assert_eq!(
            parse_command(&["dysoncubeproject", "status", "-3", "140", "12"]),
            Ok(Command::Status { pos: [-3, 140, 12] })
        );
    }

    #[test]
    fn set_and_add_parse_each_stat() {
        let at = [100, 140, 0];
        assert_eq!(
            parse_command(&["dysoncubeproject", "set", "beams", "100", "140", "0", "17"]),
            Ok(Command::Set {
                stat: Stat::Beams,
                pos: at,
                value: 17
            })
        );
        assert_eq!(
            parse_command(&[
                "dysoncubeproject",
                "set",
                "panels",
                "100",
                "140",
                "0",
                "100"
            ]),
            Ok(Command::Set {
                stat: Stat::Panels,
                pos: at,
                value: 100
            })
        );
        assert_eq!(
            parse_command(&["dysoncubeproject", "add", "beams", "100", "140", "0", "-1"]),
            Ok(Command::Add {
                stat: Stat::Beams,
                pos: at,
                value: -1
            })
        );
        assert_eq!(
            parse_command(&["dysoncubeproject", "add", "panels", "100", "140", "0", "6"]),
            Ok(Command::Add {
                stat: Stat::Panels,
                pos: at,
                value: 6
            })
        );
    }

    #[test]
    fn another_command_is_left_alone() {
        assert_eq!(
            parse_command(&["bigreactors", "status", "0", "0", "0"]),
            Err(CommandError::NotOurs)
        );
        assert_eq!(parse_command(&[]), Err(CommandError::NotOurs));
    }

    #[test]
    fn a_short_or_unknown_form_is_a_usage_error() {
        for words in [
            vec!["dysoncubeproject"],
            vec!["dysoncubeproject", "status"],
            vec!["dysoncubeproject", "status", "1", "2"],
            vec!["dysoncubeproject", "set", "beams", "1", "2", "3"],
            vec!["dysoncubeproject", "set", "sails", "1", "2", "3", "4"],
            vec!["dysoncubeproject", "add", "beams", "1", "2", "3", "many"],
            vec!["dysoncubeproject", "status", "1", "2", "x"],
        ] {
            assert_eq!(
                parse_command(&words),
                Err(CommandError::BadUsage),
                "{words:?}"
            );
        }
    }

    #[test]
    fn a_trailing_word_is_a_usage_error() {
        assert_eq!(
            parse_command(&["dysoncubeproject", "status", "1", "2", "3", "4"]),
            Err(CommandError::BadUsage)
        );
    }

    #[test]
    fn the_buffer_clamps_what_it_is_given() {
        assert_eq!(Buffer::new(-5).energy, 0);
        assert_eq!(Buffer::new(i32::MAX).energy, RAY_RECEIVER_POWER_BUFFER);
        let mut buffer = Buffer::new(RAY_RECEIVER_POWER_BUFFER - 10);
        assert_eq!(buffer.insert(1_000), 10);
        assert_eq!(buffer.insert(1_000), 0);
    }
}
