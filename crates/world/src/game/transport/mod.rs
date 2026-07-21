//! Transport subsystem.
//!
//! Only the infra-free foundations are ported so far: passenger geometry and keyframe
//! animation lookup. The transport objects, template loading, movement and passenger
//! tracking that build on these are not yet ported.

pub mod animation;
pub mod passenger;
pub mod segment;

pub use animation::{TransportAnimation, TransportAnimationEntry};
pub use passenger::{normalize_orientation, TransportFrame};
pub use segment::{calculate_segment_pos, MotionProfile, SegmentFrame};
