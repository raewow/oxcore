//! Transport ownership and the boarding orchestration (`TransportMgr` + `GenericTransport`
//! passenger handling).
//!
//! [`TransportManager`] owns the live [`Transport`] objects and drives the two-sided boarding
//! that C++ `GenericTransport::AddPassenger`/`RemovePassenger` perform: updating the
//! transport's passenger set *and* the rider's own transport state. Riders reach the manager
//! through the [`TransportPassenger`] trait, so a creature and a player board by the same
//! code path despite storing their movement state differently.

use std::collections::HashMap;

use oxcore_shared::protocol::{ObjectGuid, Position};

use crate::core::common::movement::MoveFlags;
use crate::game::creature::creature::Creature;
use crate::game::player::player::Player;

use super::object::Transport;

/// A unit that can ride a transport, seen uniformly regardless of how it stores its state.
///
/// This is the unified movement/transport view the boarding code works against: a creature
/// keeps it in its [`MovementInfo`](crate::core::common::movement::MovementInfo), a player in
/// its `MovementState`, and both satisfy this trait.
pub trait TransportPassenger {
    /// The rider's object GUID, its key in the transport's passenger set.
    fn passenger_guid(&self) -> ObjectGuid;

    /// The rider's current world position.
    fn world_position(&self) -> Position;

    /// The transport the rider is currently on, if any.
    fn current_transport(&self) -> Option<ObjectGuid>;

    /// The rider's stored transport-local offset, if it is on a transport.
    fn transport_offset(&self) -> Option<Position>;

    /// Record boarding: transport GUID, local offset and the `ONTRANSPORT` flag together
    /// (`SetTransportData` + `AddMovementFlag(MOVEFLAG_ONTRANSPORT)`).
    fn set_transport_ride(&mut self, transport: ObjectGuid, offset: Position);

    /// Clear the transport ride and the `ONTRANSPORT` flag (`ClearTransportData`).
    fn clear_transport_ride(&mut self);
}

impl TransportPassenger for Creature {
    fn passenger_guid(&self) -> ObjectGuid {
        self.guid
    }

    fn world_position(&self) -> Position {
        self.position
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

    fn world_position(&self) -> Position {
        self.movement.position
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

/// Owns the live transports and orchestrates boarding (`TransportMgr`).
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

    /// Board a passenger onto a transport (`GenericTransport::AddPassenger`).
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

    /// Remove a passenger from a transport (`GenericTransport::RemovePassenger`).
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal rider standing in for a creature or player, exercising the boarding
    /// orchestration without constructing a full entity.
    struct TestRider {
        guid: ObjectGuid,
        position: Position,
        transport_guid: Option<ObjectGuid>,
        transport_offset: Option<Position>,
        on_transport_flag: bool,
    }

    impl TestRider {
        fn at(guid: ObjectGuid, position: Position) -> Self {
            Self {
                guid,
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
        fn world_position(&self) -> Position {
            self.position
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
        TestRider::at(ObjectGuid::new_player(7), Position::new(105.0, 200.0, 50.0, 0.0))
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
        assert!((offset.x - 5.0).abs() < 1e-4 && offset.y.abs() < 1e-4, "got {offset:?}");
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
