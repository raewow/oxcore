//! Transport subsystem.
//!
//! The motion maths are ported and tested: passenger geometry ([`passenger`]), keyframe
//! animation lookup and loading ([`animation`]), the ship path schedule ([`schedule`]) and
//! segment position ([`segment`]), elevator interpolation ([`elevator`]) and shared timing
//! ([`generic`]). What remains is the GameObject-backed transport object that drives these -
//! create/update/teleport, passenger tracking and map relocation - which needs the Object,
//! Map, DBC and spline subsystems.

pub mod animation;
pub mod elevator;
pub mod generic;
pub mod manager;
pub mod object;
pub mod passenger;
pub mod schedule;
pub mod segment;
pub mod ship;
pub mod waypoints;

pub use animation::{TransportAnimation, TransportAnimationEntry, TransportAnimationManager};
pub use elevator::{interpolate_local_position, path_progress};
pub use generic::time_since_creation;
pub use manager::{TransportManager, TransportPassenger};
pub use object::Transport;
pub use passenger::{normalize_orientation, TransportFrame};
pub use ship::{advance_to_current_frame, classify_frame, FramePhase, ShipFrameState};
pub use waypoints::{generate_waypoints, TaxiPathNode, TransportPath};
pub use schedule::{compute_schedule, KeyFrame, ScheduleProfile};
pub use segment::{calculate_segment_pos, MotionProfile, SegmentFrame};
