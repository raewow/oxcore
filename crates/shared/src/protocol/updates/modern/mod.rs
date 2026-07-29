//! Modern (1.14.x) object-update encoding.
//!
//! Both protocols model object state identically -- an array of u32 slots plus a bitmask of which
//! ones are present -- so this module reuses the vanilla field writes the game systems already
//! emit and translates them. What differs is the framing:
//!
//! * slot numbers moved, so every index goes through [`field_map`];
//! * GUIDs widened from 64 to 128 bits, taking four slots instead of two;
//! * the mask is sent at the object type's full width rather than trimmed to the highest set bit;
//! * create blocks carry a bit-packed header and a movement block instead of vanilla's flag byte;
//! * out-of-range and destroyed objects moved out of the block list into the packet header.
//!
//! Reference: HermesProxy `World/Objects/Version/V1_14_1_40688/ObjectUpdateBuilder.cs` and
//! `World/Server/Packets/UpdatePackets.cs`. 40688 is the update-field table for build 42597.

pub mod block;
pub mod field_map;
pub mod fields;
pub mod placeholders;
pub mod repack;

pub use block::{ModernCreateData, ModernUpdateBlock, ModernUpdateType};
pub use fields::{ModernFieldsArray, ModernObjectType, ModernUpdateMask};
