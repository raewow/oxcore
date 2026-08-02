//! Respawn System - handles creature respawn lifecycle

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use oxcore_shared::messages::update::SmsgUpdateObject;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::sync::Arc;

/// RespawnSystem - handles all respawn business logic and packet sending
pub struct RespawnSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl RespawnSystem {
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { broadcast_mgr }
    }

    /// Process respawns for all dead creatures
    pub async fn process_respawns(&self, world: &World) -> anyhow::Result<()> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Get creatures ready to respawn
        let ready_to_respawn: Vec<ObjectGuid> = world
            .managers
            .creature_mgr
            .iter_creatures()
            .filter(|e| e.should_respawn(current_time))
            .map(|e| *e.key())
            .collect();

        for guid in ready_to_respawn {
            self.respawn_creature(guid, world).await?;
        }

        Ok(())
    }

    /// Respawn a single creature
    async fn respawn_creature(&self, guid: ObjectGuid, world: &World) -> anyhow::Result<()> {
        // Pooled creatures re-roll their pool instead of respawning blindly:
        // the pool may replace this creature with another of its members
        // (PoolManager::UpdatePool on respawn).
        if !world.systems.pool.on_creature_respawn(guid, world) {
            return Ok(());
        }

        // Get respawn info before modifying
        let respawn_info = world
            .managers
            .creature_mgr
            .with_creature_mut(guid, |creature| {
                let map_id = creature.map_id;
                let instance_id = creature.instance_id;
                let position = creature.home_position;
                let entry = creature.entry;

                // Reset creature state
                creature.respawn();

                (map_id, instance_id, position, entry)
            });

        let Some((map_id, instance_id, position, entry)) = respawn_info else {
            return Ok(());
        };

        // Death cleared the motion master (`MotionMaster::Clear` in
        // `SetDeathState`), so the spawn's movement type has to be applied again —
        // otherwise every respawned wanderer and patroller stands on its spawn
        // point for the rest of the map's life.
        if let Some(spawn) = world.managers.creature_mgr.get_spawn_data(guid) {
            world.managers.creature_mgr.initialize_creature_movement(
                guid,
                &spawn,
                Some(&world.managers.waypoint_mgr),
            );
        }

        tracing::info!(
            "[RESPAWN] Creature {:?} (entry {}) respawning at ({:.1}, {:.1}, {:.1})",
            guid,
            entry,
            position.x,
            position.y,
            position.z
        );

        // Re-register with grid
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        map.add_creature(guid, position);

        // Send CREATE_OBJECT to nearby players
        self.send_creature_spawn(guid, map_id, instance_id, position, world);

        Ok(())
    }

    /// Send CREATE_OBJECT to players near the spawn point
    fn send_creature_spawn(
        &self,
        creature_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        position: Position,
        world: &World,
    ) {
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        let visibility_distance = map.visibility_distance();

        // Get nearby players
        let nearby_players: Vec<ObjectGuid> = map
            .get_objects_in_range(position, visibility_distance)
            .into_iter()
            .filter(|g| g.is_player())
            .collect();

        if nearby_players.is_empty() {
            return;
        }

        // Build create message using CreatureManager
        let Some(create_msg) = world
            .managers
            .creature_mgr
            .build_create_msg(creature_guid, world)
        else {
            return;
        };

        // Send to each nearby player via broadcast_mgr
        for player_guid in nearby_players {
            self.broadcast_mgr
                .send_msg_to_player(player_guid, create_msg.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::death::{process_corpse_decay, process_deaths, DeathState};
    use crate::game::creature::{CreatureSpawnData, CreatureTemplate};
    use oxcore_db::database::Databases;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn template(entry: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 35,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 7,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Kill -> corpse -> decay -> respawn, driven through the same calls the world
    /// tick makes. The timer half of this used to overflow into the next century,
    /// so nothing ever came back.
    #[tokio::test]
    async fn a_killed_creature_comes_back_after_its_spawn_delay() {
        let world = test_world();
        let spawn_pos = Position {
            x: 100.0,
            y: 200.0,
            z: 0.0,
            o: 0.0,
        };

        world.managers.creature_mgr.add_template(template(70));
        world
            .managers
            .creature_mgr
            .add_spawns_for_test(vec![CreatureSpawnData::new(1, 70, 0, spawn_pos, 300)]);

        let map = world.managers.map_mgr.get_or_create_map(0, 0);
        map.add_player(ObjectGuid::new_player(1), spawn_pos);
        world
            .systems
            .grid
            .process_map_grids(0, 0, &world)
            .await
            .expect("grid processing");

        let guid = *world
            .managers
            .creature_mgr
            .iter_creatures()
            .next()
            .expect("creature spawned")
            .key();

        // Kill it, then run the tick's death phase.
        world
            .managers
            .creature_mgr
            .with_creature_mut(guid, |c| c.kill(None));
        process_deaths(&world).await.expect("deaths");

        let (state, respawn_time) = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.death_state, c.respawn_time))
            .expect("creature");
        assert_eq!(state, DeathState::Corpse);
        // 300s base, +/-10% from the random respawn flag — not a timestamp.
        let delay_ms = respawn_time - now_ms();
        assert!(
            (269_000..=331_000).contains(&delay_ms),
            "respawn is {delay_ms}ms out, expected ~300s"
        );

        // Let the corpse decay away.
        process_corpse_decay(&world, 60_000).await.expect("decay");
        assert_eq!(
            world
                .managers
                .creature_mgr
                .with_creature(guid, |c| c.death_state),
            Some(DeathState::Dead)
        );
        assert!(!map.get_objects_in_range(spawn_pos, 10.0).contains(&guid));

        // Fast-forward to the respawn point and run the respawn phase.
        world
            .managers
            .creature_mgr
            .with_creature_mut(guid, |c| c.respawn_time = 0);
        world
            .systems
            .creature_respawn
            .process_respawns(&world)
            .await
            .expect("respawns");

        let creature = world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.death_state, c.in_world, c.current_health))
            .expect("creature");
        assert_eq!(creature.0, DeathState::Alive);
        assert!(creature.1, "respawned creature must be in world");
        assert!(creature.2 > 0);
        assert!(
            map.get_objects_in_range(spawn_pos, 10.0).contains(&guid),
            "respawned creature must be back on the map"
        );
    }
}
