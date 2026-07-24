//! Follower reference helpers for linking a follower to its target.

use crate::World;
use oxcore_shared::protocol::ObjectGuid;

/// Helper for follower link lifecycle.
pub struct FollowerReference;

impl FollowerReference {
    /// Register a follower on the target creature.
    pub fn target_object_build_link(
        world: &World,
        target_guid: ObjectGuid,
        follower_guid: ObjectGuid,
    ) {
        let _ = world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |target| {
                target.add_follower(follower_guid);
            });

        let _ = world
            .managers
            .creature_mgr
            .with_creature_mut(follower_guid, |source| {
                source.following_target = Some(target_guid);
            });
    }

    /// Remove a follower from the target creature.
    pub fn target_object_destroy_link(
        world: &World,
        target_guid: ObjectGuid,
        follower_guid: ObjectGuid,
    ) {
        let _ = world
            .managers
            .creature_mgr
            .with_creature_mut(target_guid, |target| {
                target.remove_follower(follower_guid);
            });
    }

    /// Stop the source creature from following its current target.
    pub fn source_object_destroy_link(world: &World, source_guid: ObjectGuid) {
        let _ = world
            .managers
            .creature_mgr
            .with_creature_mut(source_guid, |source| {
                source.stop_following();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::creature::{Creature, CreatureTemplate};
    use crate::World;
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::{ObjectGuid, Position};
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_world() -> World {
        let pool = || {
            MySqlPoolOptions::new()
                .connect_lazy("mysql://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        let databases = Arc::new(Databases {
            world: pool(),
            character: pool(),
            auth: pool(),
            logs: pool(),
        });

        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn test_creature_template(entry: u32) -> CreatureTemplate {
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

    fn add_test_creature(world: &World, entry: u32, counter: u32) -> ObjectGuid {
        let guid = ObjectGuid::new_creature(entry, counter);
        let template = test_creature_template(entry);
        world
            .managers
            .creature_mgr
            .add_creature_for_test(Creature::new(
                guid,
                entry,
                1,
                Position::default(),
                0,
                0,
                &template,
                1,
                None,
            ));
        guid
    }

    #[tokio::test]
    async fn target_object_build_link_registers_bidirectional_follower_link() {
        let world = test_world();
        let target_guid = add_test_creature(&world, 1, 1);
        let follower_guid = add_test_creature(&world, 1, 2);

        FollowerReference::target_object_build_link(&world, target_guid, follower_guid);

        let target_has_follower = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |target| {
                target.followers.contains(&follower_guid)
            })
            .unwrap();
        assert!(target_has_follower);

        let follower_has_target = world
            .managers
            .creature_mgr
            .with_creature(follower_guid, |follower| {
                follower.following_target == Some(target_guid)
            })
            .unwrap();
        assert!(follower_has_target);
    }

    #[tokio::test]
    async fn target_object_destroy_link_removes_follower_from_target() {
        let world = test_world();
        let target_guid = add_test_creature(&world, 1, 1);
        let follower_guid = add_test_creature(&world, 1, 2);

        FollowerReference::target_object_build_link(&world, target_guid, follower_guid);
        FollowerReference::target_object_destroy_link(&world, target_guid, follower_guid);

        let target_no_longer_has_follower = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |target| {
                !target.followers.contains(&follower_guid)
            })
            .unwrap();
        assert!(target_no_longer_has_follower);
    }

    #[tokio::test]
    async fn source_object_destroy_link_clears_follower_target() {
        let world = test_world();
        let target_guid = add_test_creature(&world, 1, 1);
        let follower_guid = add_test_creature(&world, 1, 2);

        FollowerReference::target_object_build_link(&world, target_guid, follower_guid);
        FollowerReference::source_object_destroy_link(&world, follower_guid);

        let follower_no_longer_following = world
            .managers
            .creature_mgr
            .with_creature(follower_guid, |follower| {
                follower.following_target.is_none()
            })
            .unwrap();
        assert!(follower_no_longer_following);
    }
}
