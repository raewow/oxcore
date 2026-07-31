//! Transport ownership and the boarding orchestration.
//!
//! [`TransportManager`] owns the live [`Transport`] objects and drives the two-sided boarding
//! the transports perform: updating the
//! transport's passenger set *and* the rider's own transport state. Riders reach the manager
//! through the [`TransportPassenger`] trait, so a creature and a player board by the same
//! code path despite storing their movement state differently.

use std::collections::HashMap;

use oxcore_shared::protocol::{HighGuid, ObjectGuid, Position};

use crate::core::common::movement::MoveFlags;
use crate::core::common::position::is_valid_map_coord;
use crate::game::creature::creature::Creature;
use crate::game::player::player::Player;

use super::object::Transport;
use super::segment::MotionProfile;
use super::ship::ship_position;
use super::template::{TransportTemplate, TransportTemplateStore};

/// What happened when repositioning a passenger against its transport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepositionOutcome {
    /// The passenger was moved to this world position.
    Repositioned(Position),
    /// The passenger is on a different map than its transport (mid-teleport); skipped.
    DifferentMap,
    /// The computed world position is off the map; the passenger was left where it was.
    InvalidPosition,
    /// The passenger has no transport offset recorded, so there is nothing to place.
    NotAboard,
    /// No such transport is registered.
    UnknownTransport,
}

/// A unit that can ride a transport, seen uniformly regardless of how it stores its state.
///
/// This is the unified movement/transport view the boarding code works against: a creature
/// keeps it in its [`MovementInfo`](crate::core::common::movement::MovementInfo), a player in
/// its `MovementState`, and both satisfy this trait.
pub trait TransportPassenger {
    /// The rider's object GUID, its key in the transport's passenger set.
    fn passenger_guid(&self) -> ObjectGuid;

    /// The map the rider is on, to detect a rider that has not yet followed a teleporting
    /// transport across maps.
    fn map_id(&self) -> u32;

    /// The rider's current world position.
    fn world_position(&self) -> Position;

    /// Move the rider to a new world position (the `Relocate` in `UpdatePassengerPosition`).
    ///
    /// This sets the rider's stored coordinates; the grid-cell move and visibility broadcast
    /// that a full `Map` relocation performs are the map layer's job.
    fn set_world_position(&mut self, position: Position);

    /// The transport the rider is currently on, if any.
    fn current_transport(&self) -> Option<ObjectGuid>;

    /// The rider's stored transport-local offset, if it is on a transport.
    fn transport_offset(&self) -> Option<Position>;

    /// Record boarding: transport GUID, local offset and the `ONTRANSPORT` flag together.
    fn set_transport_ride(&mut self, transport: ObjectGuid, offset: Position);

    /// Clear the transport ride and the `ONTRANSPORT` flag.
    fn clear_transport_ride(&mut self);
}

impl TransportPassenger for Creature {
    fn passenger_guid(&self) -> ObjectGuid {
        self.guid
    }

    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn world_position(&self) -> Position {
        self.position
    }

    fn set_world_position(&mut self, position: Position) {
        self.position = position;
    }

    fn current_transport(&self) -> Option<ObjectGuid> {
        self.movement_info.transport_guid
    }

    fn transport_offset(&self) -> Option<Position> {
        self.movement_info.transport_position
    }

    fn set_transport_ride(&mut self, transport: ObjectGuid, offset: Position) {
        self.movement_info.set_transport_data(transport, offset);
        self.movement_info.flags.set_flag(MoveFlags::ONTRANSPORT);
    }

    fn clear_transport_ride(&mut self) {
        self.movement_info.clear_transport_data();
        self.movement_info.flags.remove_flag(MoveFlags::ONTRANSPORT);
    }
}

impl TransportPassenger for Player {
    fn passenger_guid(&self) -> ObjectGuid {
        self.guid
    }

    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn world_position(&self) -> Position {
        self.movement.position
    }

    fn set_world_position(&mut self, position: Position) {
        self.movement.position = position;
    }

    fn current_transport(&self) -> Option<ObjectGuid> {
        self.movement.transport_guid
    }

    fn transport_offset(&self) -> Option<Position> {
        self.movement.transport_position
    }

    fn set_transport_ride(&mut self, transport: ObjectGuid, offset: Position) {
        self.movement.transport_guid = Some(transport);
        self.movement.transport_position = Some(offset);
        self.movement.movement_flags |= MoveFlags::ONTRANSPORT.value();
    }

    fn clear_transport_ride(&mut self) {
        self.movement.transport_guid = None;
        self.movement.transport_position = None;
        self.movement.movement_flags &= !MoveFlags::ONTRANSPORT.value();
    }
}

/// Owns the live transports and orchestrates boarding.
#[derive(Debug, Default, Clone)]
pub struct TransportManager {
    transports: HashMap<ObjectGuid, Transport>,
}

impl TransportManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a transport.
    pub fn add_transport(&mut self, transport: Transport) {
        self.transports.insert(transport.guid, transport);
    }

    /// Unregister a transport, returning it if it was present.
    pub fn remove_transport(&mut self, guid: ObjectGuid) -> Option<Transport> {
        self.transports.remove(&guid)
    }

    pub fn get(&self, guid: ObjectGuid) -> Option<&Transport> {
        self.transports.get(&guid)
    }

    pub fn get_mut(&mut self, guid: ObjectGuid) -> Option<&mut Transport> {
        self.transports.get_mut(&guid)
    }

    pub fn count(&self) -> usize {
        self.transports.len()
    }

    /// The transports currently on `map_id`.
    pub fn transports_on_map(&self, map_id: u32) -> impl Iterator<Item = &Transport> {
        self.transports.values().filter(move |t| t.map_id == map_id)
    }

    /// Create a transport from its template and register it on `map_id`.
    ///
    /// The transport spawns at the first keyframe of its path that lies on this map, facing
    /// that keyframe's orientation, with its path cursor set to that keyframe's arrival so the
    /// tick picks up where the schedule places it. Its GUID is a mobile-transport GUID keyed
    /// by the template entry, as ships are single, shared objects. Returns `None` if the path
    /// never visits this map, or if the start position is off the map. `now_ms` is the
    /// current server clock, for the transport's creation time. Instance placement (the
    /// instanceable guard) is the map layer's concern; like the creature spawner this uses
    /// the continent path. The game-object template fields (scale, faction, flags, display)
    /// stay in the template, per the slim object model this codebase uses.
    pub fn create_transport(
        &mut self,
        template: &TransportTemplate,
        map_id: u32,
        now_ms: u32,
    ) -> Option<ObjectGuid> {
        let start = template.start_frame_on_map(map_id)?;
        let position = Position::new(
            start.node_x,
            start.node_y,
            start.node_z,
            start.initial_orientation,
        );
        if !is_valid_map_coord(position.x, position.y, position.z, position.o) {
            return None;
        }

        // A mobile transport's GUID uses the mobile-transport type with the template entry
        // as its low part.
        let guid = ObjectGuid::new_without_entry(HighGuid::MoTransport, template.entry);

        let mut transport = Transport::new(guid, template.entry, map_id, position, now_ms);
        transport.set_path_progress(start.arrive_time);
        transport.set_start_progress(start.arrive_time);
        self.transports.insert(guid, transport);
        Some(guid)
    }

    /// Advance a ship transport's position for the current time.
    ///
    /// Recomputes path progress from how long the transport has existed plus its start
    /// progress, wrapped over the path period, then places the transport at the spline point
    /// that progress maps to and carries the keyframe cursor forward. Returns the new position
    /// when the transport moved, or `None` when it is paused at a stop or the path has no
    /// period. The cross-map teleport branch (a teleport keyframe) and the passenger-reposition
    /// loop are handled separately; this is the on-map motion.
    pub fn tick_ship_movement(
        &mut self,
        transport_guid: ObjectGuid,
        template: &TransportTemplate,
        now_ms: u32,
    ) -> Option<Position> {
        if template.path_time == 0 {
            return None;
        }
        let transport = self.transports.get_mut(&transport_guid)?;

        let current = transport
            .time_since_creation(now_ms)
            .wrapping_add(transport.start_progress());
        transport.set_path_progress(current);
        let path_progress = current % template.path_time;

        let profile = MotionProfile {
            speed: template.speed,
            accel: template.accel,
            accel_time: template.accel_time,
            accel_dist: template.accel_dist,
        };
        let motion = ship_position(
            &template.keyframes,
            &template.segment_splines,
            &profile,
            transport.frame_cursor(),
            path_progress,
        )?;

        transport.set_frame_cursor(motion.cursor);
        transport.relocate(
            motion.position.x,
            motion.position.y,
            motion.position.z,
            motion.orientation,
        );
        Some(transport.position)
    }

    /// Spawn every template's transport that belongs on `map_id`.
    ///
    /// Skips continent transports already spawned elsewhere (they are shared across continent
    /// instances), creates the rest, and marks each created template spawned. Returns the
    /// GUIDs created.
    pub fn spawn_transports_on_map(
        &mut self,
        store: &mut TransportTemplateStore,
        map_id: u32,
        now_ms: u32,
    ) -> Vec<ObjectGuid> {
        let to_spawn: Vec<u32> = store
            .templates_on_map(map_id)
            .filter(|t| !(t.spawned && !t.in_instance))
            .map(|t| t.entry)
            .collect();

        let mut spawned = Vec::new();
        for entry in to_spawn {
            let created = match store.get(entry) {
                Some(template) => self.create_transport(template, map_id, now_ms),
                None => None,
            };
            if let Some(guid) = created {
                spawned.push(guid);
                if let Some(template) = store.get_mut(entry) {
                    template.spawned = true;
                }
            }
        }
        spawned
    }

    /// Board a passenger onto a transport.
    ///
    /// Adds the rider to the transport's passenger set and, if it was newly boarded, records
    /// the ride on the rider itself. When the rider is changing transports and `adjust_coords`
    /// is set, its local offset is recomputed from its world position against the transport;
    /// otherwise its existing offset is kept. Returns whether the rider newly boarded, or
    /// `false` if no such transport exists.
    pub fn board<P: TransportPassenger>(
        &mut self,
        transport_guid: ObjectGuid,
        passenger: &mut P,
        adjust_coords: bool,
    ) -> bool {
        let Some(transport) = self.transports.get_mut(&transport_guid) else {
            return false;
        };

        let boarded = transport.add_passenger(passenger.passenger_guid());
        if boarded {
            let changed_transports = passenger.current_transport() != Some(transport_guid);
            let offset = if changed_transports && adjust_coords {
                transport.calculate_passenger_offset(passenger.world_position())
            } else {
                passenger.transport_offset().unwrap_or_default()
            };
            passenger.set_transport_ride(transport_guid, offset);
        }
        boarded
    }

    /// Remove a passenger from a transport.
    ///
    /// Removes the rider from the passenger set and, if it was aboard, clears its transport
    /// state. Returns whether the rider was aboard, or `false` if no such transport exists.
    pub fn unboard<P: TransportPassenger>(
        &mut self,
        transport_guid: ObjectGuid,
        passenger: &mut P,
    ) -> bool {
        let Some(transport) = self.transports.get_mut(&transport_guid) else {
            return false;
        };

        let removed = transport.remove_passenger(passenger.passenger_guid());
        if removed {
            passenger.clear_transport_ride();
        }
        removed
    }

    /// Board a follower alongside the unit it is following.
    ///
    /// A follower (a pet, a summoned guardian) does not compute its own offset: it inherits
    /// the leader's transport offset and is teleported onto the leader's world position, so it
    /// rides in the same spot. Returns whether the transport exists.
    ///
    /// The reference splits the teleport by rider type; both paths set the follower's
    /// position, which is what is done here - the heartbeat/broadcast is the map/network
    /// layer's job.
    pub fn board_follower<L: TransportPassenger, F: TransportPassenger>(
        &mut self,
        transport_guid: ObjectGuid,
        leader: &L,
        follower: &mut F,
    ) -> bool {
        if !self.transports.contains_key(&transport_guid) {
            return false;
        }

        // AddPassenger(follower); the offset it would compute is overwritten below, matching
        // the reference order, so coordinate adjustment is skipped here.
        self.board(transport_guid, follower, false);

        // The follower rides in the leader's slot and stands on the leader's position; done
        // unconditionally, as the reference does after AddPassenger even for an already-aboard
        // unit.
        let leader_offset = leader.transport_offset().unwrap_or_default();
        follower.set_transport_ride(transport_guid, leader_offset);
        follower.set_world_position(leader.world_position());
        true
    }

    /// Remove a follower and return it to its leader.
    ///
    /// Takes the follower off the transport and teleports it onto the leader's world position.
    /// Returns whether the follower was aboard.
    pub fn unboard_follower<L: TransportPassenger, F: TransportPassenger>(
        &mut self,
        transport_guid: ObjectGuid,
        leader: &L,
        follower: &mut F,
    ) -> bool {
        let removed = self.unboard(transport_guid, follower);
        // The follower is teleported to the leader regardless of whether it was aboard,
        // matching the reference which relocates after RemovePassenger unconditionally.
        follower.set_world_position(leader.world_position());
        removed
    }

    /// Place one passenger in the world from its transport-local offset.
    ///
    /// Transforms the rider's stored offset into world coordinates against the transport,
    /// skipping a rider that is mid-teleport onto a different map and refusing a position that
    /// falls off the map (the rider stays put rather than being flung to bad coordinates).
    /// On success the rider's stored coordinates are updated; the grid-cell move and
    /// visibility broadcast a full relocation performs, and the `ctime` reset, belong to
    /// the map layer and are not done here.
    pub fn reposition_passenger<P: TransportPassenger>(
        &self,
        transport_guid: ObjectGuid,
        passenger: &mut P,
    ) -> RepositionOutcome {
        let Some(transport) = self.transports.get(&transport_guid) else {
            return RepositionOutcome::UnknownTransport;
        };
        if passenger.map_id() != transport.map_id {
            return RepositionOutcome::DifferentMap;
        }
        let Some(offset) = passenger.transport_offset() else {
            return RepositionOutcome::NotAboard;
        };

        let world = transport.calculate_passenger_position(offset);
        if !is_valid_map_coord(world.x, world.y, world.z, world.o) {
            return RepositionOutcome::InvalidPosition;
        }

        passenger.set_world_position(world);
        RepositionOutcome::Repositioned(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal rider standing in for a creature or player, exercising the boarding
    /// orchestration without constructing a full entity.
    struct TestRider {
        guid: ObjectGuid,
        map_id: u32,
        position: Position,
        transport_guid: Option<ObjectGuid>,
        transport_offset: Option<Position>,
        on_transport_flag: bool,
    }

    impl TestRider {
        fn at(guid: ObjectGuid, position: Position) -> Self {
            Self {
                guid,
                map_id: 0,
                position,
                transport_guid: None,
                transport_offset: None,
                on_transport_flag: false,
            }
        }
    }

    impl TransportPassenger for TestRider {
        fn passenger_guid(&self) -> ObjectGuid {
            self.guid
        }
        fn map_id(&self) -> u32 {
            self.map_id
        }
        fn world_position(&self) -> Position {
            self.position
        }
        fn set_world_position(&mut self, position: Position) {
            self.position = position;
        }
        fn current_transport(&self) -> Option<ObjectGuid> {
            self.transport_guid
        }
        fn transport_offset(&self) -> Option<Position> {
            self.transport_offset
        }
        fn set_transport_ride(&mut self, transport: ObjectGuid, offset: Position) {
            self.transport_guid = Some(transport);
            self.transport_offset = Some(offset);
            self.on_transport_flag = true;
        }
        fn clear_transport_ride(&mut self) {
            self.transport_guid = None;
            self.transport_offset = None;
            self.on_transport_flag = false;
        }
    }

    fn transport_guid() -> ObjectGuid {
        ObjectGuid::new_gameobject(176231, 1)
    }

    fn manager_with_transport() -> TransportManager {
        let mut mgr = TransportManager::new();
        mgr.add_transport(Transport::new(
            transport_guid(),
            176231,
            0,
            Position::new(100.0, 200.0, 50.0, 0.0),
            1_000,
        ));
        mgr
    }

    fn rider() -> TestRider {
        TestRider::at(
            ObjectGuid::new_player(7),
            Position::new(105.0, 200.0, 50.0, 0.0),
        )
    }

    /// A template store holding one straight-line transport whose path runs on `map_id`.
    fn store_with_template(entry: u32, map_id: u32) -> TransportTemplateStore {
        use super::super::schedule::ScheduleProfile;
        use super::super::waypoints::TaxiPathNode;

        let nodes: Vec<TaxiPathNode> = (0..5)
            .map(|i| TaxiPathNode {
                map_id,
                x: i as f32 * 10.0,
                y: 0.0,
                z: 0.0,
                action_flag: 0,
                delay: 0,
            })
            .collect();
        let mut store = TransportTemplateStore::new();
        store.load_template(
            entry,
            &nodes,
            &ScheduleProfile {
                speed: 10.0,
                accel: 5.0,
            },
        );
        store
    }

    #[test]
    fn creating_a_transport_places_it_at_its_start_frame() {
        let store = store_with_template(176231, 0);
        let template = store.get(176231).unwrap();
        let mut mgr = TransportManager::new();

        let guid = mgr
            .create_transport(template, 0, 5_000)
            .expect("created on its map");
        // A ship is a mobile-transport object keyed by its template entry.
        assert!(guid.is_mo_transport());
        assert_eq!(guid.counter(), 176231);
        let transport = mgr.get(guid).unwrap();
        assert_eq!(transport.entry, 176231);
        assert_eq!(transport.map_id, 0);
        // Positioned at a keyframe of the path (x is one of the interior node xs, 10/20/30).
        assert!([10.0, 20.0, 30.0]
            .iter()
            .any(|&x| (transport.position.x - x).abs() < 1e-3));
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn a_transport_is_not_created_on_a_map_its_path_avoids() {
        let store = store_with_template(176231, 0);
        let template = store.get(176231).unwrap();
        let mut mgr = TransportManager::new();
        // The path only runs on map 0.
        assert!(mgr.create_transport(template, 571, 5_000).is_none());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn spawning_a_map_creates_its_transports_once() {
        let mut store = store_with_template(176231, 0);
        let mut mgr = TransportManager::new();

        let spawned = mgr.spawn_transports_on_map(&mut store, 0, 5_000);
        assert_eq!(spawned.len(), 1);
        assert!(store.get(176231).unwrap().spawned);

        // Spawning the same continent map again does not re-create the already-spawned one.
        let again = mgr.spawn_transports_on_map(&mut store, 0, 6_000);
        assert!(again.is_empty());
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn a_spawned_transport_moves_along_its_path_over_time() {
        use super::super::schedule::ScheduleProfile;
        use super::super::waypoints::TaxiPathNode;

        // A longer straight path so there is clear travel to observe.
        let nodes: Vec<TaxiPathNode> = (0..7)
            .map(|i| TaxiPathNode {
                map_id: 0,
                x: i as f32 * 10.0,
                y: 0.0,
                z: 0.0,
                action_flag: 0,
                delay: 0,
            })
            .collect();
        let mut store = TransportTemplateStore::new();
        store.load_template(
            500,
            &nodes,
            &ScheduleProfile {
                speed: 10.0,
                accel: 5.0,
            },
        );
        let template = store.get(500).unwrap().clone();

        let mut mgr = TransportManager::new();
        // Create at server clock 0 so time_since_creation is just the tick clock.
        let guid = mgr.create_transport(&template, 0, 0).unwrap();

        // Tick partway and later along the run; the transport should advance and stay on the
        // line (y, z ~ 0).
        let quarter = template.path_time / 4;
        let half = template.path_time / 2;
        let early = mgr
            .tick_ship_movement(guid, &template, quarter)
            .expect("moving");
        let late = mgr
            .tick_ship_movement(guid, &template, half)
            .expect("moving");

        assert!(
            early.y.abs() < 1e-2 && early.z.abs() < 1e-2,
            "off the line: {early:?}"
        );
        assert!(
            late.x > early.x,
            "did not advance: {} -> {}",
            early.x,
            late.x
        );
        // The manager's stored transport reflects the latest position.
        assert_eq!(mgr.get(guid).unwrap().position, late);
    }

    #[test]
    fn spawning_skips_maps_the_transport_does_not_visit() {
        let mut store = store_with_template(176231, 0);
        let mut mgr = TransportManager::new();
        // No template's path runs on map 1.
        assert!(mgr.spawn_transports_on_map(&mut store, 1, 5_000).is_empty());
        assert_eq!(mgr.count(), 0);
        assert!(!store.get(176231).unwrap().spawned);
    }

    #[test]
    fn registration_tracks_transports_by_map() {
        let mut mgr = manager_with_transport();
        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.transports_on_map(0).count(), 1);
        assert_eq!(mgr.transports_on_map(1).count(), 0);
        assert!(mgr.remove_transport(transport_guid()).is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn boarding_updates_both_the_transport_and_the_rider() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();

        assert!(mgr.board(transport_guid(), &mut rider, true));

        // Transport side: the rider is in the passenger set.
        assert!(mgr.get(transport_guid()).unwrap().has_passenger(rider.guid));
        // Rider side: transport recorded, flag set, and the offset is its world position
        // relative to the transport - 5 yards ahead of it along +x.
        assert_eq!(rider.transport_guid, Some(transport_guid()));
        assert!(rider.on_transport_flag);
        let offset = rider.transport_offset.unwrap();
        assert!(
            (offset.x - 5.0).abs() < 1e-4 && offset.y.abs() < 1e-4,
            "got {offset:?}"
        );
    }

    #[test]
    fn boarding_an_unknown_transport_does_nothing() {
        let mut mgr = TransportManager::new();
        let mut rider = rider();
        assert!(!mgr.board(transport_guid(), &mut rider, true));
        assert_eq!(rider.transport_guid, None);
    }

    #[test]
    fn boarding_twice_reports_no_second_boarding() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        assert!(mgr.board(transport_guid(), &mut rider, true));
        // Already aboard: the second board is a no-op that reports false.
        assert!(!mgr.board(transport_guid(), &mut rider, true));
        assert_eq!(mgr.get(transport_guid()).unwrap().passenger_count(), 1);
    }

    #[test]
    fn a_follower_rides_in_its_leaders_slot() {
        let mut mgr = manager_with_transport();
        let mut leader = rider();
        mgr.board(transport_guid(), &mut leader, true);

        // A pet at some unrelated spot follows its owner aboard.
        let mut follower = TestRider::at(
            ObjectGuid::new_pet(2000, 9),
            Position::new(-500.0, -500.0, 0.0, 0.0),
        );
        assert!(mgr.board_follower(transport_guid(), &leader, &mut follower));

        // Follower shares the leader's offset and stands on the leader's world position.
        assert_eq!(follower.transport_offset, leader.transport_offset);
        assert_eq!(follower.transport_guid, Some(transport_guid()));
        assert!(follower.on_transport_flag);
        assert_eq!(follower.position, leader.position);
        // Both are aboard.
        assert_eq!(mgr.get(transport_guid()).unwrap().passenger_count(), 2);
    }

    #[test]
    fn removing_a_follower_returns_it_to_its_leader() {
        let mut mgr = manager_with_transport();
        let mut leader = rider();
        mgr.board(transport_guid(), &mut leader, true);
        let mut follower = TestRider::at(
            ObjectGuid::new_pet(2000, 9),
            Position::new(-500.0, -500.0, 0.0, 0.0),
        );
        mgr.board_follower(transport_guid(), &leader, &mut follower);

        assert!(mgr.unboard_follower(transport_guid(), &leader, &mut follower));
        // Off the transport, its ride cleared, and standing on the leader.
        assert_eq!(follower.transport_guid, None);
        assert!(!follower.on_transport_flag);
        assert_eq!(follower.position, leader.position);
        assert_eq!(mgr.get(transport_guid()).unwrap().passenger_count(), 1);
    }

    #[test]
    fn boarding_a_follower_onto_an_unknown_transport_does_nothing() {
        let mut mgr = TransportManager::new();
        let leader = rider();
        let mut follower = TestRider::at(
            ObjectGuid::new_pet(2000, 9),
            Position::new(-500.0, -500.0, 0.0, 0.0),
        );
        assert!(!mgr.board_follower(transport_guid(), &leader, &mut follower));
        assert_eq!(follower.transport_guid, None);
    }

    #[test]
    fn repositioning_moves_a_rider_with_its_transport() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        // Board to record the rider's offset (5 yards ahead of the transport).
        mgr.board(transport_guid(), &mut rider, true);

        // The transport turns 90 degrees in place; the rider must swing to its new side.
        mgr.get_mut(transport_guid()).unwrap().relocate(
            100.0,
            200.0,
            50.0,
            std::f32::consts::FRAC_PI_2,
        );
        let outcome = mgr.reposition_passenger(transport_guid(), &mut rider);

        match outcome {
            RepositionOutcome::Repositioned(world) => {
                // 5 yards along the transport's new +y facing: (100, 205).
                assert!(
                    (world.x - 100.0).abs() < 1e-3 && (world.y - 205.0).abs() < 1e-3,
                    "got {world:?}"
                );
                assert_eq!(rider.position, world);
            }
            other => panic!("expected Repositioned, got {other:?}"),
        }
    }

    #[test]
    fn repositioning_round_trips_through_the_offset() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        let original = rider.position;
        mgr.board(transport_guid(), &mut rider, true);

        // Without moving the transport, placing the rider from its offset recovers exactly
        // where it boarded.
        mgr.reposition_passenger(transport_guid(), &mut rider);
        assert!(
            (rider.position.x - original.x).abs() < 1e-3
                && (rider.position.y - original.y).abs() < 1e-3
                && (rider.position.z - original.z).abs() < 1e-3
        );
    }

    #[test]
    fn a_rider_on_another_map_is_left_alone() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        mgr.board(transport_guid(), &mut rider, true);
        let before = rider.position;

        // Rider has not yet followed the transport across a map teleport.
        rider.map_id = 1;
        assert_eq!(
            mgr.reposition_passenger(transport_guid(), &mut rider),
            RepositionOutcome::DifferentMap
        );
        assert_eq!(rider.position, before);
    }

    #[test]
    fn an_off_map_position_leaves_the_rider_put() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        mgr.board(transport_guid(), &mut rider, true);
        let before = rider.position;

        // Corrupt the stored offset so the computed world position lands off the map.
        rider.transport_offset = Some(Position::new(1.0e9, 0.0, 0.0, 0.0));
        assert_eq!(
            mgr.reposition_passenger(transport_guid(), &mut rider),
            RepositionOutcome::InvalidPosition
        );
        assert_eq!(rider.position, before);
    }

    #[test]
    fn unboarding_clears_both_sides() {
        let mut mgr = manager_with_transport();
        let mut rider = rider();
        mgr.board(transport_guid(), &mut rider, true);

        assert!(mgr.unboard(transport_guid(), &mut rider));
        assert_eq!(mgr.get(transport_guid()).unwrap().passenger_count(), 0);
        assert_eq!(rider.transport_guid, None);
        assert!(!rider.on_transport_flag);
        // Unboarding again reports the rider was not aboard.
        assert!(!mgr.unboard(transport_guid(), &mut rider));
    }
}
