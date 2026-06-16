//! Update type enums and constants from UpdateData.h
//!
//! This module contains the core enums and constants used for update packets,
//! ported from the C++ reference implementation.

/// Update type constants for different packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectUpdateType {
    /// UPDATETYPE_VALUES - Update field values only
    Values = 0,
    /// UPDATETYPE_MOVEMENT - Movement update
    Movement = 1,
    /// UPDATETYPE_CREATE_OBJECT - Create object (1.12.1+)
    CreateObject = 2,
    /// UPDATETYPE_CREATE_OBJECT2 - Create object with extended data (1.8.4+)
    CreateObject2 = 3,
    /// UPDATETYPE_OUT_OF_RANGE_OBJECTS - Objects that moved out of range
    OutOfRangeObjects = 4,
    /// UPDATETYPE_NEAR_OBJECTS - Objects that moved into range
    NearObjects = 5,
}

impl ObjectUpdateType {
    /// Convert to u8 for packet writing
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Update flags for object creation/updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ObjectUpdateFlags {
    /// No flags
    None = 0x0000,
    /// This update is for the current player (self)
    SelfFlag = 0x0001,
    /// Object is on a transport
    Transport = 0x0002,
    /// Object is melee attacking (1.8.4+)
    MeleeAttacking = 0x0004,
    /// High GUID is included (1.8.4+)
    HighGuid = 0x0008,
    /// All objects should be updated (1.8.4+)
    All = 0x0010,
    /// Object is alive/has position (1.8.4+)
    Living = 0x0020,
    /// Object has position data (1.8.4+)
    HasPosition = 0x0040,
}

impl ObjectUpdateFlags {
    /// Convert to u16 for packet writing
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Check if flag is set
    pub fn has_flag(&self, flag: ObjectUpdateFlags) -> bool {
        (*self as u16) & (flag as u16) != 0
    }

    /// Check if this is a self update
    pub fn is_self(&self) -> bool {
        self.has_flag(ObjectUpdateFlags::SelfFlag)
    }

    /// Check if this is a living object update
    pub fn is_living(&self) -> bool {
        self.has_flag(ObjectUpdateFlags::Living)
    }

    /// Check if this has position data
    pub fn has_position(&self) -> bool {
        self.has_flag(ObjectUpdateFlags::HasPosition)
    }

    /// Check if this is on a transport
    pub fn is_on_transport(&self) -> bool {
        self.has_flag(ObjectUpdateFlags::Transport)
    }
}

/// Object type IDs used in update packets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectTypeId {
    /// TYPEID_OBJECT
    Object = 0,
    /// TYPEID_ITEM
    Item = 1,
    /// TYPEID_CONTAINER
    Container = 2,
    /// TYPEID_UNIT
    Unit = 3,
    /// TYPEID_PLAYER
    Player = 4,
    /// TYPEID_GAMEOBJECT
    GameObject = 5,
    /// TYPEID_DYNAMICOBJECT
    DynamicObject = 6,
    /// TYPEID_CORPSE
    Corpse = 7,
}

impl ObjectTypeId {
    /// Convert to u8 for packet writing
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Packet opcodes for update packets
pub mod opcodes {
    use crate::shared::protocol::opcodes::Opcode;

    /// SMSG_UPDATE_OBJECT - Standard update object packet
    pub const SMSG_UPDATE_OBJECT: Opcode = Opcode::SMSG_UPDATE_OBJECT;
    /// SMSG_COMPRESSED_UPDATE_OBJECT - Compressed update object packet
    pub const SMSG_COMPRESSED_UPDATE_OBJECT: Opcode = Opcode::SMSG_COMPRESSED_UPDATE_OBJECT;
    /// SMSG_DESTROY_OBJECT - Destroy object packet
    pub const SMSG_DESTROY_OBJECT: Opcode = Opcode::SMSG_DESTROY_OBJECT;
    /// SMSG_COMPRESSED_MOVES - Compressed movement updates (1.7.1+)
    pub const SMSG_COMPRESSED_MOVES: Opcode = Opcode::SMSG_COMPRESSED_MOVES;
}

/// Movement update flags for movement blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MovementFlags {
    /// No movement flags
    None = 0x00000000,
    /// Forward movement
    Forward = 0x00000001,
    /// Backward movement
    Backward = 0x00000002,
    /// Strafe left
    StrafeLeft = 0x00000004,
    /// Strafe right
    StrafeRight = 0x00000008,
    /// Left turn
    TurnLeft = 0x00000010,
    /// Right turn
    TurnRight = 0x00000020,
    /// Pitch up
    PitchUp = 0x00000040,
    /// Pitch down
    PitchDown = 0x00000080,
    /// Walking
    WalkMode = 0x00000100,
    /// On ground
    OnTransport = 0x00000200,
    /// Levitating (hover)
    Levitating = 0x00000400,
    /// Fixed z (no terrain collision)
    FixedZ = 0x00000800,
    /// Rooted (can't move)
    Root = 0x00001000,
    /// Falling
    Falling = 0x00002000,
    /// Falling far (after jump)
    FallingFar = 0x00004000,
    /// Swimming
    Swimming = 0x00200000,
    /// Ascending (flying up)
    Ascending = 0x00400000,
    /// Descending (flying down)
    Descending = 0x00800000,
    /// Can fly
    CanFly = 0x01000000,
    /// Currently flying
    Flying = 0x02000000,
}

impl MovementFlags {
    /// Convert to u32 for packet writing
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Check if flag is set
    pub fn has_flag(&self, flag: MovementFlags) -> bool {
        (*self as u32) & (flag as u32) != 0
    }

    /// Check if object is moving forward
    pub fn is_moving_forward(&self) -> bool {
        self.has_flag(MovementFlags::Forward)
    }

    /// Check if object is swimming
    pub fn is_swimming(&self) -> bool {
        self.has_flag(MovementFlags::Swimming)
    }

    /// Check if object is flying
    pub fn is_flying(&self) -> bool {
        self.has_flag(MovementFlags::Flying)
    }

    /// Check if object is falling
    pub fn is_falling(&self) -> bool {
        self.has_flag(MovementFlags::Falling)
    }
}

/// High-level object type masks for determining which fields apply to which objects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectTypeMask {
    /// All objects
    Object = 0x01,
    /// Units (players, creatures, etc.)
    Unit = 0x02,
    /// Players only
    Player = 0x04,
    /// Items
    Item = 0x08,
    /// Containers (bags)
    Container = 0x10,
    /// GameObjects
    GameObject = 0x20,
    /// DynamicObjects (spells, etc.)
    DynamicObject = 0x40,
    /// Corpses
    Corpse = 0x80,
}

impl ObjectTypeMask {
    /// Convert to u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if this mask includes the given type
    pub fn includes(&self, other: ObjectTypeMask) -> bool {
        (self.as_u8() & other.as_u8()) != 0
    }

    /// Check if this is a unit (player or creature)
    pub fn is_unit(&self) -> bool {
        self.includes(ObjectTypeMask::Unit)
    }

    /// Check if this is a player
    pub fn is_player(&self) -> bool {
        self.includes(ObjectTypeMask::Player)
    }
}
