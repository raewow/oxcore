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
pub mod passenger;
pub mod schedule;
pub mod segment;

pub use animation::{TransportAnimation, TransportAnimationEntry, TransportAnimationManager};
pub use elevator::{interpolate_local_position, path_progress};
pub use generic::time_since_creation;
pub use passenger::{normalize_orientation, TransportFrame};
pub use schedule::{compute_schedule, KeyFrame, ScheduleProfile};
pub use segment::{calculate_segment_pos, MotionProfile, SegmentFrame};
