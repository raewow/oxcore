//! Liquid status production: reads terrain and VMap liquid data at a player's
//! position and feeds the result into their environment flags.
//!
//! This is what makes swimming, drowning, lava damage, and fishing work — the
//! mirror timers and environment flags downstream are all driven from the
//! `LiquidStatus` produced here.

use oxcore_map::terrain::{LiquidData, TerrainInfo, MAP_ALL_LIQUIDS};
use oxcore_map::VMapManager;
use oxcore_shared::protocol::{ObjectGuid, Position};

use crate::World;

use super::system::LiquidStatus;

/// Nudge the sample point up so a player resting exactly on the surface is
/// classified consistently.
const Z_EPSILON: f32 = 0.01;

/// Query the liquid at a position.
///
/// Checks WMO volumes (indoor pools, lava in instances) before the ADT liquid
/// layer (oceans, lakes, rivers), and reports where `pos` sits relative to the
/// surface along with the liquid's kind and heights.
pub fn query_liquid_status(
    terrain: &TerrainInfo,
    vmap: &VMapManager,
    pos: Position,
) -> LiquidStatus {
    let mut data = LiquidData::default();
    let status = terrain.get_liquid_status(
        pos.x,
        pos.y,
        pos.z + Z_EPSILON,
        MAP_ALL_LIQUIDS,
        Some(vmap),
        Some(&mut data),
    );

    if status.is_empty() {
        return LiquidStatus::none();
    }

    LiquidStatus { status, data }
}

/// Refresh a player's liquid-derived environment flags from their position.
///
/// Called from the movement system after the player's position is updated, and
/// on login, mirroring where the reference triggers its area check.
pub fn update_player_liquid_status(
    player_guid: ObjectGuid,
    world: &World,
    map_id: u32,
    pos: Position,
) {
    let terrain = world.managers.terrain_mgr.get(map_id);
    let liquid_status = query_liquid_status(&terrain, &world.managers.vmap_mgr, pos);

    world.systems.environment.update_environment_flags(
        player_guid,
        &liquid_status,
        pos.z,
        world,
        &world.managers.player_mgr,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::environment::state::{EnvironmentFlags, EnvironmentState};
    use crate::game::player::environment::system::{
        update_environment_flags_internal, DEFAULT_COLLISION_HEIGHT,
    };
    use oxcore_map::pathfinding::vmap::VMapConfig;
    use oxcore_map::terrain::{
        LiquidStatusFlags, MAP_LIQUID_TYPE_DEEP_WATER, MAP_LIQUID_TYPE_MAGMA,
        MAP_LIQUID_TYPE_OCEAN, MAP_LIQUID_TYPE_SLIME, MAP_LIQUID_TYPE_WATER,
    };

    /// Build a status as if the player were `depth` yards below the surface.
    fn submerged(type_flags: u32, depth: f32, player_z: f32) -> LiquidStatus {
        let level = player_z + depth;
        LiquidStatus {
            status: if depth > 2.0 {
                LiquidStatusFlags::UNDER_WATER
            } else if depth > 0.0 {
                LiquidStatusFlags::IN_WATER
            } else {
                LiquidStatusFlags::WATER_WALK
            },
            data: LiquidData {
                type_flags,
                entry: 0,
                level,
                depth_level: player_z - 10.0,
            },
        }
    }

    fn apply(status: &LiquidStatus, player_z: f32) -> EnvironmentState {
        let mut env = EnvironmentState::default();
        update_environment_flags_internal(
            &mut env,
            status,
            player_z,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        env
    }

    #[test]
    fn no_liquid_clears_every_liquid_flag() {
        let mut env = EnvironmentState::default();
        env.env_flags = EnvironmentFlags::MASK_LIQUID_FLAGS;

        update_environment_flags_internal(
            &mut env,
            &LiquidStatus::none(),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );

        assert!(!env
            .env_flags
            .intersects(EnvironmentFlags::MASK_LIQUID_FLAGS));
    }

    #[test]
    fn deep_water_marks_swimming_and_submerged() {
        // 5 yards under the surface: past both the swim depth and head height.
        let env = apply(&submerged(MAP_LIQUID_TYPE_WATER, 5.0, 10.0), 10.0);

        assert!(env.env_flags.contains(EnvironmentFlags::LIQUID));
        assert!(env.env_flags.contains(EnvironmentFlags::IN_WATER));
        assert!(env.env_flags.contains(EnvironmentFlags::UNDERWATER));
        assert!(env.env_flags.contains(EnvironmentFlags::HIGH_LIQUID));
    }

    #[test]
    fn shallow_water_is_not_swimming_or_submerged() {
        // 1 yard deep: in the water, but not deep enough to swim.
        let env = apply(&submerged(MAP_LIQUID_TYPE_WATER, 1.0, 10.0), 10.0);

        assert!(env.env_flags.contains(EnvironmentFlags::LIQUID));
        assert!(env.env_flags.contains(EnvironmentFlags::IN_WATER));
        assert!(!env.env_flags.contains(EnvironmentFlags::UNDERWATER));
        assert!(!env.env_flags.contains(EnvironmentFlags::HIGH_LIQUID));
    }

    #[test]
    fn submersion_requires_surface_above_head_height() {
        // Status says under water, but the surface is below head height, so the
        // player is not actually submerged and must not start drowning.
        let mut status = submerged(MAP_LIQUID_TYPE_WATER, 5.0, 10.0);
        status.data.level = 10.0 + DEFAULT_COLLISION_HEIGHT - 0.1;

        let env = apply(&status, 10.0);
        assert!(!env.env_flags.contains(EnvironmentFlags::UNDERWATER));
    }

    #[test]
    fn deep_sea_sets_high_sea_for_fatigue() {
        let env = apply(
            &submerged(
                MAP_LIQUID_TYPE_OCEAN | MAP_LIQUID_TYPE_DEEP_WATER,
                5.0,
                10.0,
            ),
            10.0,
        );

        assert!(env.env_flags.contains(EnvironmentFlags::HIGH_SEA));
        assert!(env.env_flags.contains(EnvironmentFlags::IN_WATER));
    }

    #[test]
    fn shallow_ocean_without_deep_flag_has_no_fatigue() {
        let env = apply(&submerged(MAP_LIQUID_TYPE_OCEAN, 5.0, 10.0), 10.0);
        assert!(!env.env_flags.contains(EnvironmentFlags::HIGH_SEA));
    }

    #[test]
    fn magma_burns_when_merely_standing_on_the_surface() {
        // WATER_WALK: standing at the surface. Water would not count as "in",
        // but lava burns anyway.
        let env = apply(&submerged(MAP_LIQUID_TYPE_MAGMA, 0.0, 10.0), 10.0);

        assert!(env.env_flags.contains(EnvironmentFlags::IN_MAGMA));
        assert!(env
            .env_flags
            .intersects(EnvironmentFlags::MASK_LIQUID_HAZARD));
        assert!(!env.env_flags.contains(EnvironmentFlags::IN_WATER));
    }

    #[test]
    fn water_at_the_surface_is_not_in_water() {
        let env = apply(&submerged(MAP_LIQUID_TYPE_WATER, 0.0, 10.0), 10.0);

        assert!(env.env_flags.contains(EnvironmentFlags::LIQUID));
        assert!(!env.env_flags.contains(EnvironmentFlags::IN_WATER));
    }

    #[test]
    fn slime_sets_its_own_hazard_flag() {
        let env = apply(&submerged(MAP_LIQUID_TYPE_SLIME, 1.0, 10.0), 10.0);

        assert!(env.env_flags.contains(EnvironmentFlags::IN_SLIME));
        assert!(!env.env_flags.contains(EnvironmentFlags::IN_MAGMA));
    }

    #[test]
    fn submerging_drains_breath_and_surfacing_recovers_it() {
        let mut env = EnvironmentState::default();

        // Dive: the breath timer starts counting down.
        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_WATER, 5.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(env.env_flags.contains(EnvironmentFlags::UNDERWATER));
        assert!(
            env.breath_timer.scale < 0,
            "breath should drain while submerged, scale was {}",
            env.breath_timer.scale
        );

        // Surface but stay in the water: breath refills instead of resetting.
        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_WATER, 1.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(!env.env_flags.contains(EnvironmentFlags::UNDERWATER));
        assert!(
            env.breath_timer.scale > 0,
            "breath should recover at the surface, scale was {}",
            env.breath_timer.scale
        );
    }

    #[test]
    fn water_breathing_aura_stops_breath_from_draining() {
        let mut env = EnvironmentState::default();

        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_WATER, 5.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            true, // has water breathing
        );

        assert!(env.env_flags.contains(EnvironmentFlags::UNDERWATER));
        assert!(
            env.breath_timer.scale > 0,
            "water breathing must keep the breath timer topped up"
        );
    }

    #[test]
    fn entering_deep_sea_drains_fatigue_and_leaving_recovers() {
        let mut env = EnvironmentState::default();

        update_environment_flags_internal(
            &mut env,
            &submerged(
                MAP_LIQUID_TYPE_OCEAN | MAP_LIQUID_TYPE_DEEP_WATER,
                5.0,
                10.0,
            ),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(env.fatigue_timer.scale < 0);

        update_environment_flags_internal(
            &mut env,
            &LiquidStatus::none(),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(env.fatigue_timer.scale > 0);
    }

    #[test]
    fn hazard_timer_keeps_draining_when_moving_from_lava_to_slime() {
        let mut env = EnvironmentState::default();

        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_MAGMA, 1.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(env.environmental_timer.scale < 0);

        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_SLIME, 1.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(
            env.environmental_timer.scale < 0,
            "still standing in a hazard, so damage must keep ticking"
        );

        update_environment_flags_internal(
            &mut env,
            &submerged(MAP_LIQUID_TYPE_WATER, 1.0, 10.0),
            10.0,
            DEFAULT_COLLISION_HEIGHT,
            false,
        );
        assert!(env.environmental_timer.scale > 0);
    }

    #[test]
    fn query_returns_no_liquid_without_terrain_or_vmap_data() {
        let terrain = TerrainInfo::new(0, std::path::PathBuf::from("/nonexistent/maps"));
        let vmap = VMapManager::new("/nonexistent", VMapConfig::default());

        let status = query_liquid_status(&terrain, &vmap, Position::new(0.0, 0.0, 0.0, 0.0));
        assert!(!status.has_liquid());
    }
}
