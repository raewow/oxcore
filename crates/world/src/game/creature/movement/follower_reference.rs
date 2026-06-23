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
