//! The transport game object (`GenericTransport`).
//!
//! A transport is a GameObject that moves along a path and carries units in its local frame.
//! This is the object layer that ties the ported motion maths together: it owns the
//! transport's world position, its passenger set and its path cursor, and binds the passenger
//! coordinate transforms ([`super::passenger`]) to its own position.
//!
//! The passenger-side of boarding - writing the transport offset and `MOVEFLAG_ONTRANSPORT`
//! onto the rider, relocating it through the Map - lives on the Unit and is not modelled
//! here; `Creature`/`Player` have no transport movement fields yet. This layer owns only what
//! the transport itself owns: membership and its own position.

use std::collections::BTreeSet;

use oxcore_shared::protocol::{ObjectGuid, Position};

use super::generic::time_since_creation;
use super::passenger::TransportFrame;

/// A moving platform units can ride: the shared base of ships, zeppelins, elevators and
/// trams (`GenericTransport`).
#[derive(Debug, Clone)]
pub struct Transport {
    /// This transport's object GUID.
    pub guid: ObjectGuid,
    /// Game-object template entry.
    pub entry: u32,
    /// Map the transport is on.
    pub map_id: u32,
    /// Current world position and facing.
    pub position: Position,

    /// Server clock (ms) when the transport was created, for `time_since_creation`.
    creation_time_ms: u32,
    /// Progress along the path; full time since start for MO transports, or time within the
    /// cycle for looping ones (`m_pathProgress`).
    path_progress: u32,
    /// The path progress the transport started at, added to `time_since_creation` each tick
    /// (`m_startProgress`).
    start_progress: u32,
    /// The keyframe the transport is currently on, carried across ticks (`m_currentFrame`).
    frame_cursor: usize,
    /// GUIDs of the units currently aboard (`m_passengers`), kept ordered like `std::set`.
    passengers: BTreeSet<ObjectGuid>,
}

impl Transport {
    /// A transport at `position` on `map_id`, created at server clock `creation_time_ms`.
    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        map_id: u32,
        position: Position,
        creation_time_ms: u32,
    ) -> Self {
        Self {
            guid,
            entry,
            map_id,
            position,
            creation_time_ms,
            path_progress: 0,
            start_progress: 0,
            frame_cursor: 0,
            passengers: BTreeSet::new(),
        }
    }

    /// Board a unit, returning whether it was newly added (`GenericTransport::AddPassenger`,
    /// the transport-owned `m_passengers.insert(...).second`).
    ///
    /// The passenger's own boarding state - its transport offset, `MOVEFLAG_ONTRANSPORT` and
    /// back-reference - is written on the Unit by the caller once units carry that state.
    pub fn add_passenger(&mut self, passenger: ObjectGuid) -> bool {
        self.passengers.insert(passenger)
    }

    /// Remove a unit, returning whether it was aboard (`GenericTransport::RemovePassenger`,
    /// the transport-owned erase; the C++ teleport-iterator dance guards against iterator
    /// invalidation, which a `BTreeSet` does not need).
    pub fn remove_passenger(&mut self, passenger: ObjectGuid) -> bool {
        self.passengers.remove(&passenger)
    }

    /// Whether `passenger` is aboard.
    pub fn has_passenger(&self, passenger: ObjectGuid) -> bool {
        self.passengers.contains(&passenger)
    }

    /// The units currently aboard, in GUID order (`GetPassengers`).
    pub fn passengers(&self) -> impl Iterator<Item = &ObjectGuid> {
        self.passengers.iter()
    }

    /// How many units are aboard.
    pub fn passenger_count(&self) -> usize {
        self.passengers.len()
    }

    /// Progress along the path (`GetPathProgress`).
    pub fn path_progress(&self) -> u32 {
        self.path_progress
    }

    /// Set the path progress (the transport's `Update` advances this).
    pub fn set_path_progress(&mut self, progress: u32) {
        self.path_progress = progress;
    }

    /// The path progress the transport started at (`m_startProgress`).
    pub fn start_progress(&self) -> u32 {
        self.start_progress
    }

    /// Set the starting path progress (fixed at creation).
    pub fn set_start_progress(&mut self, progress: u32) {
        self.start_progress = progress;
    }

    /// The keyframe cursor carried across ticks (`m_currentFrame`).
    pub fn frame_cursor(&self) -> usize {
        self.frame_cursor
    }

    /// Set the keyframe cursor (the tick advances this).
    pub fn set_frame_cursor(&mut self, cursor: usize) {
        self.frame_cursor = cursor;
    }

    /// Move the transport itself to a new position and facing (`GenericTransport::UpdatePosition`).
    ///
    /// Refreshing the collision model and repositioning passengers additionally need the Map
    /// and the Unit, so they are the caller's job for now.
    pub fn update_position(&mut self, x: f32, y: f32, z: f32, orientation: f32) {
        self.position = Position::new(x, y, z, orientation);
    }

    /// Alias for [`Self::update_position`] kept for callers that name the action `relocate`.
    pub fn relocate(&mut self, x: f32, y: f32, z: f32, orientation: f32) {
        self.update_position(x, y, z, orientation);
    }

    /// Milliseconds since the transport was created (`GetTimeSinceCreation`), given the
    /// current server clock.
    pub fn time_since_creation(&self, now_ms: u32) -> u32 {
        time_since_creation(self.creation_time_ms, now_ms)
    }

    /// This transport's frame, for transforming passenger coordinates against its position.
    fn frame(&self) -> TransportFrame {
        TransportFrame::new(
            self.position.x,
            self.position.y,
            self.position.z,
            self.position.o,
        )
    }

    /// World position of a passenger from its transport-local offset, against this
    /// transport's position (the inline `CalculatePassengerPosition`).
    pub fn calculate_passenger_position(&self, offset: Position) -> Position {
        self.frame().passenger_position(offset)
    }

    /// Transport-local offset of a passenger from its world position, against this
    /// transport's position (the inline `CalculatePassengerOffset`).
    pub fn calculate_passenger_offset(&self, world: Position) -> Position {
        self.frame().passenger_offset(world)
    }

    /// Recalculate the world position of every boarded creature (`GenericTransport::UpdatePassengerPositions`).
    ///
    /// Player passengers are not handled yet; the Rust `Player` does not expose transport offset
    /// state in the same way `Creature` does via `MovementInfo`.
    pub fn update_passenger_positions(&self, world: &crate::World) {
        let passengers: Vec<ObjectGuid> = self.passengers.iter().copied().collect();
        for passenger in passengers {
            let Some(offset) = world
                .managers
                .creature_mgr
                .with_creature(passenger, |creature| {
                    creature.movement_info.transport_position
                })
                .flatten()
            else {
                continue;
            };

            let new_position = self.calculate_passenger_position(offset);
            let _ = world
                .managers
                .creature_mgr
                .with_creature_mut(passenger, |creature| {
                    creature.position = new_position;
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transport() -> Transport {
        Transport::new(
            ObjectGuid::new_gameobject(176231, 1),
            176231,
            0,
            Position::new(100.0, 200.0, 50.0, 0.0),
            1_000,
        )
    }

    fn rider(counter: u32) -> ObjectGuid {
        ObjectGuid::new_creature(2000, counter)
    }

    #[test]
    fn boarding_reports_new_riders_and_ignores_repeats() {
        let mut t = transport();
        assert!(t.add_passenger(rider(1)));
        // Boarding the same unit twice does not double-count.
        assert!(!t.add_passenger(rider(1)));
        assert!(t.add_passenger(rider(2)));
        assert_eq!(t.passenger_count(), 2);
        assert!(t.has_passenger(rider(1)));
    }

    #[test]
    fn removing_reports_whether_the_unit_was_aboard() {
        let mut t = transport();
        t.add_passenger(rider(1));
        assert!(t.remove_passenger(rider(1)));
        // Removing again, or removing a unit never aboard, reports false.
        assert!(!t.remove_passenger(rider(1)));
        assert!(!t.remove_passenger(rider(9)));
        assert_eq!(t.passenger_count(), 0);
    }

    #[test]
    fn passengers_are_listed_in_guid_order() {
        let mut t = transport();
        t.add_passenger(rider(3));
        t.add_passenger(rider(1));
        t.add_passenger(rider(2));
        let order: Vec<ObjectGuid> = t.passengers().copied().collect();
        assert_eq!(order, vec![rider(1), rider(2), rider(3)]);
    }

    #[test]
    fn update_position_moves_the_frame_the_transforms_use() {
        let mut t = transport();
        // A passenger 5 yards ahead (local +x) sits at world (105, 200) with zero facing.
        let ahead = Position::new(5.0, 0.0, 0.0, 0.0);
        let before = t.calculate_passenger_position(ahead);
        assert!((before.x - 105.0).abs() < 1e-4 && (before.y - 200.0).abs() < 1e-4);

        // After a quarter turn the same local offset points along world +y instead.
        t.update_position(100.0, 200.0, 50.0, std::f32::consts::FRAC_PI_2);
        let after = t.calculate_passenger_position(ahead);
        assert!((after.x - 100.0).abs() < 1e-4 && (after.y - 205.0).abs() < 1e-4);
    }

    #[test]
    fn relocate_alias_calls_update_position() {
        let mut t = transport();
        t.relocate(1.0, 2.0, 3.0, 0.5);
        assert_eq!(t.position, Position::new(1.0, 2.0, 3.0, 0.5));
    }

    #[test]
    fn passenger_offset_recovers_the_local_position() {
        let t = transport();
        let offset = Position::new(3.0, -4.0, 1.5, 1.0);
        let world = t.calculate_passenger_position(offset);
        let recovered = t.calculate_passenger_offset(world);
        assert!(
            (recovered.x - offset.x).abs() < 1e-4
                && (recovered.y - offset.y).abs() < 1e-4
                && (recovered.z - offset.z).abs() < 1e-4
        );
    }

    #[test]
    fn path_progress_and_creation_time_are_tracked() {
        let mut t = transport();
        assert_eq!(t.path_progress(), 0);
        t.set_path_progress(4_200);
        assert_eq!(t.path_progress(), 4_200);
        // Created at clock 1000, now 5000 -> 4000 ms alive.
        assert_eq!(t.time_since_creation(5_000), 4_000);
    }

    #[tokio::test]
    async fn update_passenger_positions_moves_boarded_creatures() {
        use crate::config::Config;
        use crate::core::common::movement::MoveFlags;
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

        let world = test_world();
        let mut transport = Transport::new(
            ObjectGuid::new_gameobject(176231, 1),
            176231,
            0,
            Position::new(100.0, 200.0, 50.0, 0.0),
            1_000,
        );

        let passenger = add_test_creature(&world, 1, 1);
        world
            .managers
            .creature_mgr
            .with_creature_mut(passenger, |creature| {
                // Board the creature 5 yards ahead of the transport.
                creature
                    .movement_info
                    .set_transport_data(transport.guid, Position::new(5.0, 0.0, 0.0, 0.0));
                creature
                    .movement_info
                    .flags
                    .set_flag(MoveFlags::ONTRANSPORT);
                creature.position =
                    transport.calculate_passenger_position(Position::new(5.0, 0.0, 0.0, 0.0));
            });
        transport.add_passenger(passenger);

        // Move the transport and update passenger positions.
        transport.update_position(110.0, 220.0, 50.0, 0.0);
        transport.update_passenger_positions(&world);

        let new_position = world
            .managers
            .creature_mgr
            .with_creature(passenger, |creature| creature.position)
            .unwrap();
        assert!((new_position.x - 115.0).abs() < 1e-4);
        assert!((new_position.y - 220.0).abs() < 1e-4);
    }
}
