//! Dyson Cube Project for Horizon — the ray receiver, its multiblock and the shared
//! per-player sphere (slices 1–2).
//!
//! Upstream's multiblock controller reserves a 3×6×3 box of air, writes sixteen
//! `multiblock_structure` placeholders around itself, and subscribes its placer to a
//! sphere. The spheres are the level's own store (`DysonSphereProgressSavedData` as an
//! attachment), so several receivers draw from the same reservoir: the level tick
//! generates for every sphere once, and each server tick a receiver pulls what its
//! buffer has room for when the sky allows and offers up to 50,000,000 FE a tick to
//! the block below through a down-only energy capability.
//!
//! A moment that belongs to one block or block entity is a Part here: the controller's
//! own placement refusal, its teardown, its tick, and the placement notice that
//! subscribes it; the placeholder's teardown. The global hooks are the level tick (the
//! sphere's generation) and the command tree and body.
//!
//! The rules themselves live in [`runtime`] and are tested by `cargo test` without the
//! game.
use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent,
    BlockPart, BlockState, Command, Compound, Flow, Hooks, Host, HostSide, ItemPart, LevelHost,
    LevelTickPreEvent, OnBlockPlacedByEvent, OnDestroyedByPlayerEvent, RegisterCommandsEvent,
    Registrar, RegistrationContext, RunModCommandEvent, ServerHost, StateForPlacementEvent,
};

pub mod runtime;
pub mod support;

use support::{CONTROLLER, STRUCTURE};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The two block entity types, the receiver's down-only FE buffer, the block Parts
    /// and the controller item's placement notice.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let controller = r.block_key(CONTROLLER);
        let structure = r.block_key(STRUCTURE);

        // The same ids 0.1.0's `block_entity` hook gave each block, so a world keeps its
        // saved entities across the format change.
        let receiver = r
            .block_entity_type("ray_receiver_controller")
            .blocks(&[CONTROLLER])
            .build();
        // The type was declared on the line above; a refusal here is a programming error.
        r.block_entity_part(&receiver, ReceiverEntity)
            .expect("the receiver's block entity type is declared just above");
        // The placeholders store one thing — their controller's position — so a declared
        // type with no Part is enough: it starts with an empty compound.
        let _ = r
            .block_entity_type("multiblock_structure")
            .blocks(&[STRUCTURE])
            .build();

        r.block_energy(&controller, support::energy_buffer());
        r.block_part(controller, ReceiverBlock);
        r.block_part(structure, ShellBlock);

        let item = r.item_key(CONTROLLER);
        r.item_part(item, ControllerItem);
    }

    /// `/dysoncubeproject status|set|add`, upstream's five usages at permission level 4.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        for usage in support::USAGES {
            event.commands.push(Command::op(usage, 4));
        }
    }

    /// `DysonCubeProject`'s `LevelTickEvent.Pre`: every sphere of the Overworld refills
    /// its reservoir once, before any receiver pulls from it.
    fn level_tick_pre(&self, host: HostSide<'_>, event: &mut LevelTickPreEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        if event.level.dimension() != mod_api::DimensionKey::OVERWORLD {
            return;
        }
        support::generate_spheres(host);
    }

    /// The command body: typed arguments in, the same readout text out.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        if event.cancelled {
            return Flow::Continue;
        }
        let Some(dimension) = event.command.dimension.id(host) else {
            return Flow::Continue;
        };
        let Some(level) = host.level(&dimension) else {
            return Flow::Continue;
        };
        let handled = support::run_command(
            host,
            level,
            &event.command.literals,
            &event.command.arguments,
        );
        if handled {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

/// The controller's block: its placement refusal and its teardown.
struct ReceiverBlock;

#[mod_api::horizon_part]
impl BlockPart for ReceiverBlock {
    /// `DefaultMultiblockControllerBlock.getStateForPlacement`: the controller may only
    /// be placed where its 3×6×3 box is empty. `Some(None)` refuses the placement and
    /// keeps the stack, as vanilla's `null` does.
    fn state_for_placement(
        &self,
        level: &dyn LevelHost,
        event: &mut StateForPlacementEvent<'_>,
    ) -> Option<Option<BlockState>> {
        let anchor = [
            event.context.pos.x,
            event.context.pos.y,
            event.context.pos.z,
        ];
        (!support::placement_allowed(level, anchor)).then_some(None)
    }

    /// `RayReceiverControllerBlock.onRemove`: a broken controller clears its placeholders.
    fn on_destroyed_by_player(
        &self,
        host: &mut dyn ServerHost,
        event: &mut OnDestroyedByPlayerEvent<'_>,
    ) -> Option<bool> {
        let level = support::level_in_place(host)?;
        support::break_controller(host, level, event.pos);
        // The break itself is unchanged.
        None
    }
}

/// A placeholder block: its teardown takes the controller with it.
struct ShellBlock;

#[mod_api::horizon_part]
impl BlockPart for ShellBlock {
    /// `MultiblockStructureBlock.onRemove`: a broken placeholder destroys its controller,
    /// with its drops. This runs before the placeholder leaves the world, so the saved
    /// controller position is still readable — which is why it is this hook and not
    /// `affect_neighbors_after_removal` (see `../notes.md`, deviations).
    fn on_destroyed_by_player(
        &self,
        host: &mut dyn ServerHost,
        event: &mut OnDestroyedByPlayerEvent<'_>,
    ) -> Option<bool> {
        let level = support::level_in_place(host)?;
        support::break_placeholder(host, level, event.pos);
        None
    }
}

/// The controller's block entity: its saved data, its ticker, and one server tick.
struct ReceiverEntity;

// A block entity Part must also be a block Part; it answers no block moment of its own.
#[mod_api::horizon_part]
impl BlockPart for ReceiverEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for ReceiverEntity {
    /// `EntityBlock.newBlockEntity`: the sphere and the buffer start empty.
    fn block_entity(
        &self,
        _level: mod_api::LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// `EntityBlock.getTicker`: only the controller ticks (`RayReceiverBlockEntity` is an
    /// `ITickableBlockEntity`; the placeholders are not).
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick of the receiver: generate, pull, push down.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::level_in_place(host) else {
            return;
        };
        support::receiver_tick(host, level, event.pos, &mut event.entity.nbt);
    }
}

/// The controller item: placing it subscribes its placer's sphere and forms the
/// multiblock (`setPlacedBy`).
struct ControllerItem;

#[mod_api::horizon_part]
impl ItemPart for ControllerItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        let Some(level) = support::level_in_place(host) else {
            return;
        };
        // `setPlacedBy` subscribes the placer before it writes the shell.
        if let Some(placer) = event.placer {
            let _ = support::subscribe_receiver(host, level, event.pos, placer.uuid());
        }
        support::form_shell(host, level, event.pos);
    }
}

#[cfg(test)]
mod tests {
    /// The manifest's id is the resource namespace every block and item id is built on;
    /// a rename would silently orphan the carried content.
    #[test]
    fn the_package_id_matches_the_carried_namespace() {
        let manifest = include_str!("../horizon-mod.toml");
        assert!(manifest.contains("id = \"dysoncubeproject\""));
        assert_eq!(
            super::CONTROLLER,
            "dysoncubeproject:ray_receiver_controller"
        );
        assert_eq!(super::STRUCTURE, "dysoncubeproject:multiblock_structure");
        // The sphere store's attachment key is the mod's own namespace, and the id it
        // writes is the one `status` prints, so a rename would orphan a saved world.
        assert!(manifest.contains("version = \"0.2.1\""));
    }
}
