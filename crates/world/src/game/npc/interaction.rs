//! Whether a player is in a position to talk to an NPC or use a game object's menu.

use crate::game::common::creature_flags::CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS;
use crate::game::creature::ai::units::distance;
use crate::game::gameobject::GameObjectType;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// How close a player must stand to talk to an NPC.
pub const INTERACTION_DISTANCE: f32 = 5.0;

const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x0200_0000;

/// Whether `player` may talk to the creature `npc`. With a non-zero `npc_flag_mask` the
/// creature must also carry one of those NPC flags.
///
/// The creature must be alive, selectable, out of combat, close by and not
/// hostile; ghosts may only talk to creatures visible to ghosts, and the living may not.
/// A player at Unfriendly or worse with the creature's faction is refused.
pub fn can_interact_with_npc(
    world: &World,
    player: ObjectGuid,
    npc: ObjectGuid,
    npc_flag_mask: u32,
) -> bool {
    let Some(creature) = world.managers.creature_mgr.get_creature(npc) else {
        return false;
    };
    if !creature.is_alive()
        || creature.unit_flags & UNIT_FLAG_NOT_SELECTABLE != 0
        || (npc_flag_mask != 0 && creature.npc_flags & npc_flag_mask == 0)
    {
        return false;
    }
    let (faction, static_flags1, position, map_id, instance_id) = (
        creature.faction,
        creature.static_flags1,
        creature.position,
        creature.map_id,
        creature.instance_id,
    );
    drop(creature);
    if world.managers.creature_mgr.is_in_combat(npc) {
        return false;
    }

    let visible_to_ghosts = static_flags1 & CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS != 0;
    let reachable = world
        .managers
        .player_mgr
        .with_player(player, |p| {
            if p.taxi.is_flying() || p.map_id != map_id || p.instance_id != instance_id {
                return false;
            }
            if p.is_alive() == visible_to_ghosts {
                return false;
            }
            if crate::game::faction::creature_is_hostile_to_player(&world.dbc.read(), faction, p) {
                return false;
            }
            distance(&p.movement.position, &position) <= INTERACTION_DISTANCE
        })
        .unwrap_or(false);
    if !reachable {
        return false;
    }

    let reputation_faction = {
        let dbc = world.dbc.read();
        dbc.get_faction_template(faction)
            .map(|t| t.faction)
            .filter(|&f| dbc.get_faction(f).is_some_and(|f| f.reputation_list_id >= 0))
    };
    match reputation_faction {
        Some(faction_id) => world
            .systems
            .reputation
            .get_rank(player, faction_id, world)
            .is_none_or(|rank| rank > ReputationRank::Unfriendly),
        None => true,
    }
}

/// Whether a living player is close enough to a spawned game object to use its menu.
pub fn can_interact_with_gameobject(world: &World, player: ObjectGuid, go: ObjectGuid) -> bool {
    let Some((reach, map_id, instance_id, in_world)) =
        world.managers.gameobject_mgr.with_gameobject(go, |g| {
            (
                Reach {
                    position: g.position,
                    rotation: g.rotation,
                    scale: g.scale,
                    display_id: g.display_id,
                    radius: interaction_radius(g.go_type),
                },
                g.map_id,
                g.instance_id,
                g.in_world,
            )
        })
    else {
        return false;
    };
    let bounds = world.managers.gameobject_mgr.display_bounds(reach.display_id);
    in_world
        && world
            .managers
            .player_mgr
            .with_player(player, |p| {
                p.is_alive()
                    && !p.taxi.is_flying()
                    && p.map_id == map_id
                    && p.instance_id == instance_id
                    && reach.contains(bounds, &p.movement.position)
            })
            .unwrap_or(false)
}

/// How far from its model a game object of this type can be used.
fn interaction_radius(go_type: GameObjectType) -> f32 {
    match go_type {
        GameObjectType::QuestGiver
        | GameObjectType::Text
        | GameObjectType::FlagStand
        | GameObjectType::FlagDrop
        | GameObjectType::MiniGame => 5.55556,
        GameObjectType::Binder => 10.0,
        GameObjectType::Chair | GameObjectType::FishingNode => 100.0,
        GameObjectType::AreaDamage => 0.0,
        _ => INTERACTION_DISTANCE,
    }
}

/// A game object's placement, for measuring reach.
struct Reach {
    position: Position,
    rotation: [f32; 4],
    scale: f32,
    display_id: u32,
    radius: f32,
}

impl Reach {
    /// Whether `point` lies within `radius` of the object's model box (turned and scaled
    /// with the object), or of its centre when the model has no bounds.
    fn contains(&self, bounds: Option<[f32; 6]>, point: &Position) -> bool {
        let Some(bounds) = bounds else {
            return distance(&self.position, point) <= self.radius;
        };
        let offset = [
            point.x - self.position.x,
            point.y - self.position.y,
            point.z - self.position.z,
        ];
        let local = rotate(conjugate(self.local_rotation()), offset);
        (0..3).all(|axis| {
            let min = bounds[axis] * self.scale - self.radius;
            let max = bounds[axis + 3] * self.scale + self.radius;
            (min..=max).contains(&local[axis])
        })
    }

    /// The stored rotation, or a turn about the vertical axis by the facing when none is
    /// stored.
    fn local_rotation(&self) -> [f32; 4] {
        if self.rotation.iter().any(|v| *v != 0.0) {
            return self.rotation;
        }
        let half = self.position.o / 2.0;
        [0.0, 0.0, half.sin(), half.cos()]
    }
}

fn conjugate([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    [-x, -y, -z, w]
}

/// Rotates vector `v` by unit quaternion `q` (x, y, z, w).
fn rotate([qx, qy, qz, qw]: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    // v' = v + 2w(q × v) + 2 q × (q × v)
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let q = [qx, qy, qz];
    let t = cross(q, v).map(|c| 2.0 * c);
    let u = cross(q, t);
    [
        v[0] + qw * t[0] + u[0],
        v[1] + qw * t[1] + u[1],
        v[2] + qw * t[2] + u[2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reach(o: f32) -> Reach {
        Reach {
            position: Position::new(100.0, 100.0, 0.0, o),
            rotation: [0.0; 4],
            scale: 1.0,
            display_id: 1,
            radius: 5.0,
        }
    }

    #[test]
    fn without_bounds_reach_is_measured_from_the_centre() {
        let r = reach(0.0);
        assert!(r.contains(None, &Position::new(104.0, 100.0, 0.0, 0.0)));
        assert!(!r.contains(None, &Position::new(106.0, 100.0, 0.0, 0.0)));
    }

    #[test]
    fn bounds_extend_reach_along_a_long_object() {
        // A 20-yard-long object along its own x axis.
        let bounds = Some([-10.0, -1.0, 0.0, 10.0, 1.0, 2.0]);
        let r = reach(0.0);
        assert!(r.contains(bounds, &Position::new(114.0, 100.0, 0.0, 0.0)));
        assert!(!r.contains(bounds, &Position::new(100.0, 107.0, 0.0, 0.0)));
    }

    #[test]
    fn bounds_turn_with_the_object() {
        let bounds = Some([-10.0, -1.0, 0.0, 10.0, 1.0, 2.0]);
        let r = reach(std::f32::consts::FRAC_PI_2);
        assert!(r.contains(bounds, &Position::new(100.0, 114.0, 0.0, 0.0)));
        assert!(!r.contains(bounds, &Position::new(114.0, 100.0, 0.0, 0.0)));
    }
}
