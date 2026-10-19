//! State an objective script keeps for its scope: the world states players see
//! and, in a battleground copy, which spawn events are active.
//!
//! A scope is either one copy of a map (a battleground match) or one zone of
//! the open world. Both keep world states the same way, so a capture point or
//! score counter looks identical to the client wherever it lives.

use std::collections::{BTreeMap, HashMap};

use dashmap::DashMap;
use oxcore_shared::messages::battleground::SmsgUpdateWorldState;
use oxcore_shared::protocol::ObjectGuid;

use crate::World;

/// Where an objective script's state lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// One copy of a map (battlegrounds, dungeons).
    Copy { map_id: u32, instance_id: u32 },
    /// One zone of the open world.
    Zone { zone_id: u32 },
}

/// "No event active" for an event slot.
pub const EVENT_NONE: u8 = 255;

/// World states and spawn events for every scope.
#[derive(Default)]
pub struct ScopeStates {
    world_states: DashMap<Scope, BTreeMap<u32, i32>>,
    /// Active `event2` for each `event1` slot, per copy.
    copy_events: DashMap<(u32, u32), HashMap<u8, u8>>,
    /// Watched auras each player had at the last look, per scope.
    watched: DashMap<(Scope, ObjectGuid), Vec<u32>>,
}

impl ScopeStates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a world state. Returns whether the value changed.
    pub fn store(&self, scope: Scope, state_id: u32, value: i32) -> bool {
        let mut states = self.world_states.entry(scope).or_default();
        states.insert(state_id, value) != Some(value)
    }

    pub fn world_state(&self, scope: Scope, state_id: u32) -> Option<i32> {
        self.world_states
            .get(&scope)
            .and_then(|states| states.get(&state_id).copied())
    }

    /// All world states of a scope.
    pub fn world_states(&self, scope: Scope) -> HashMap<u32, i32> {
        self.world_states
            .get(&scope)
            .map(|states| states.iter().map(|(&k, &v)| (k, v)).collect())
            .unwrap_or_default()
    }

    /// The states a player in this copy and zone should be shown on arrival:
    /// the zone's, then the copy's (the copy wins on a clash).
    pub fn states_for(&self, map_id: u32, instance_id: u32, zone_id: u32) -> Vec<(u32, i32)> {
        let mut merged: BTreeMap<u32, i32> = BTreeMap::new();
        if let Some(states) = self.world_states.get(&Scope::Zone { zone_id }) {
            merged.extend(states.iter().map(|(&k, &v)| (k, v)));
        }
        if let Some(states) = self.world_states.get(&Scope::Copy {
            map_id,
            instance_id,
        }) {
            merged.extend(states.iter().map(|(&k, &v)| (k, v)));
        }
        merged.into_iter().collect()
    }

    /// Remember a player's watched auras; returns the ones they lost since.
    pub fn swap_watched(&self, scope: Scope, player: ObjectGuid, now: Vec<u32>) -> Vec<u32> {
        let before = self.watched.insert((scope, player), now.clone()).unwrap_or_default();
        before.into_iter().filter(|spell| !now.contains(spell)).collect()
    }

    /// Forget a player's watched auras in a scope (they left it).
    pub fn forget_watched(&self, scope: Scope, player: ObjectGuid) -> Vec<u32> {
        self.watched.remove(&(scope, player)).map(|(_, v)| v).unwrap_or_default()
    }

    /// Players whose watched auras are remembered for a scope.
    pub fn watched_players(&self, scope: Scope) -> Vec<ObjectGuid> {
        self.watched
            .iter()
            .filter(|entry| entry.key().0 == scope)
            .map(|entry| entry.key().1)
            .collect()
    }

    /// Forget everything a scope held (a match closing).
    pub fn clear(&self, scope: Scope) {
        self.watched.retain(|key, _| key.0 != scope);
        self.world_states.remove(&scope);
        if let Scope::Copy {
            map_id,
            instance_id,
        } = scope
        {
            self.copy_events.remove(&(map_id, instance_id));
        }
    }

    /// The active `event2` of an event slot in a copy.
    pub fn active_event(&self, map_id: u32, instance_id: u32, event1: u8) -> u8 {
        self.copy_events
            .get(&(map_id, instance_id))
            .and_then(|events| events.get(&event1).copied())
            .unwrap_or(EVENT_NONE)
    }

    pub fn is_event_active(&self, map_id: u32, instance_id: u32, event1: u8, event2: u8) -> bool {
        event2 != EVENT_NONE && self.active_event(map_id, instance_id, event1) == event2
    }

    /// Every active event of a copy, as `event1 -> event2`.
    pub fn active_events(&self, map_id: u32, instance_id: u32) -> HashMap<u8, u8> {
        self.copy_events
            .get(&(map_id, instance_id))
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    /// Make `event2` the active event of its slot (or clear the slot).
    /// Returns the event that was active before.
    pub fn set_active_event(&self, map_id: u32, instance_id: u32, event1: u8, event2: u8) -> u8 {
        let mut events = self.copy_events.entry((map_id, instance_id)).or_default();
        let previous = events.get(&event1).copied().unwrap_or(EVENT_NONE);
        if event2 == EVENT_NONE {
            events.remove(&event1);
        } else {
            events.insert(event1, event2);
        }
        previous
    }
}

/// Players currently inside a scope.
pub fn players_in_scope(world: &World, scope: Scope) -> Vec<ObjectGuid> {
    match scope {
        Scope::Copy {
            map_id,
            instance_id,
        } => world
            .managers
            .map_mgr
            .get_map(map_id, instance_id)
            .map(|map| map.player_guids())
            .unwrap_or_default(),
        Scope::Zone { zone_id } => world.managers.lua_mgr.get_zone_state(zone_id).players,
    }
}

/// Set a world state and show the change to everyone in the scope.
pub fn set_world_state(world: &World, scope: Scope, state_id: u32, value: i32) {
    if !world.managers.scope_states.store(scope, state_id, value) {
        return;
    }
    let players = players_in_scope(world, scope);
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(&players, &SmsgUpdateWorldState { state_id, value });
}

/// Show one player a single world state (without storing it).
pub fn send_world_state(world: &World, player: ObjectGuid, state_id: u32, value: i32) {
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player, SmsgUpdateWorldState { state_id, value });
}

/// The `SMSG_INIT_WORLD_STATES` for a player arriving in a copy and zone.
pub fn init_world_states(
    world: &World,
    map_id: u32,
    instance_id: u32,
    zone_id: u32,
) -> oxcore_shared::messages::login::SmsgInitWorldStates {
    let mut message = oxcore_shared::messages::login::SmsgInitWorldStates::new(map_id, zone_id);
    for (state_id, value) in world
        .managers
        .scope_states
        .states_for(map_id, instance_id, zone_id)
    {
        message = message.with_state(state_id, value as u32);
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storing_reports_only_changes() {
        let states = ScopeStates::new();
        let scope = Scope::Zone { zone_id: 139 };
        assert!(states.store(scope, 2426, 1));
        assert!(!states.store(scope, 2426, 1));
        assert!(states.store(scope, 2426, 0));
        assert_eq!(states.world_state(scope, 2426), Some(0));
    }

    #[test]
    fn arrival_states_merge_zone_and_copy() {
        let states = ScopeStates::new();
        states.store(Scope::Zone { zone_id: 3277 }, 1, 10);
        states.store(Scope::Zone { zone_id: 3277 }, 2, 20);
        states.store(
            Scope::Copy {
                map_id: 489,
                instance_id: 7,
            },
            2,
            99,
        );
        assert_eq!(states.states_for(489, 7, 3277), vec![(1, 10), (2, 99)]);
        assert_eq!(states.states_for(489, 8, 3277), vec![(1, 10), (2, 20)]);
    }

    #[test]
    fn one_event_per_slot() {
        let states = ScopeStates::new();
        assert_eq!(states.active_event(529, 1, 0), EVENT_NONE);
        assert_eq!(states.set_active_event(529, 1, 0, 2), EVENT_NONE);
        assert!(states.is_event_active(529, 1, 0, 2));
        assert_eq!(states.set_active_event(529, 1, 0, 3), 2);
        assert!(!states.is_event_active(529, 1, 0, 2));
        assert!(!states.is_event_active(529, 2, 0, 3));
        states.clear(Scope::Copy {
            map_id: 529,
            instance_id: 1,
        });
        assert_eq!(states.active_event(529, 1, 0), EVENT_NONE);
    }
}
