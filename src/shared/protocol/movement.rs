use anyhow::Result;

use super::{ObjectGuid, Position, WorldPacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveFlags(u32);

impl MoveFlags {
    // Vanilla 1.12.1 movement flags (from MaNGOS-classic MovementInfo.h)
    pub const NONE: MoveFlags = MoveFlags(0x00000000);
    pub const FORWARD: MoveFlags = MoveFlags(0x00000001);
    pub const BACKWARD: MoveFlags = MoveFlags(0x00000002);
    pub const STRAFE_LEFT: MoveFlags = MoveFlags(0x00000004);
    pub const STRAFE_RIGHT: MoveFlags = MoveFlags(0x00000008);
    pub const TURN_LEFT: MoveFlags = MoveFlags(0x00000010);
    pub const TURN_RIGHT: MoveFlags = MoveFlags(0x00000020);
    pub const PITCH_UP: MoveFlags = MoveFlags(0x00000040);
    pub const PITCH_DOWN: MoveFlags = MoveFlags(0x00000080);
    pub const WALK_MODE: MoveFlags = MoveFlags(0x00000100);
    pub const LEVITATING: MoveFlags = MoveFlags(0x00000400);
    pub const FIXED_Z: MoveFlags = MoveFlags(0x00000800);
    pub const ROOT: MoveFlags = MoveFlags(0x00001000);
    pub const JUMPING: MoveFlags = MoveFlags(0x00002000);
    pub const FALLINGFAR: MoveFlags = MoveFlags(0x00004000);
    pub const PENDING_STOP: MoveFlags = MoveFlags(0x00008000);
    pub const PENDING_UNSTRAFE: MoveFlags = MoveFlags(0x00010000);
    pub const PENDING_FORWARD: MoveFlags = MoveFlags(0x00020000);
    pub const PENDING_BACKWARD: MoveFlags = MoveFlags(0x00040000);
    pub const PENDING_STR_LEFT: MoveFlags = MoveFlags(0x00080000);
    pub const PENDING_STR_RIGHT: MoveFlags = MoveFlags(0x00100000);
    pub const SWIMMING: MoveFlags = MoveFlags(0x00200000);
    pub const SPLINE_ENABLED: MoveFlags = MoveFlags(0x00400000);
    pub const MOVED: MoveFlags = MoveFlags(0x00800000);
    pub const FLYING: MoveFlags = MoveFlags(0x01000000);
    pub const ONTRANSPORT: MoveFlags = MoveFlags(0x02000000);
    pub const SPLINE_ELEVATION: MoveFlags = MoveFlags(0x04000000);
    pub const WATERWALKING: MoveFlags = MoveFlags(0x10000000);
    pub const SAFE_FALL: MoveFlags = MoveFlags(0x20000000);
    pub const HOVER: MoveFlags = MoveFlags(0x40000000);

    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn has_flag(&self, flag: MoveFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn set_flag(&mut self, flag: MoveFlags) {
        self.0 |= flag.0;
    }

    pub fn remove_flag(&mut self, flag: MoveFlags) {
        self.0 &= !flag.0;
    }
}

impl From<u32> for MoveFlags {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<MoveFlags> for u32 {
    fn from(flags: MoveFlags) -> Self {
        flags.0
    }
}

#[derive(Debug, Clone)]
pub struct MovementInfo {
    pub mover_guid: ObjectGuid,
    pub flags: MoveFlags,
    pub position: Position,
    pub transport_guid: Option<ObjectGuid>,
    pub transport_position: Option<Position>,
    pub transport_time: Option<u32>,
    pub fall_time: Option<u32>,
    pub jump_velocity: Option<f32>,
    pub jump_sin_angle: Option<f32>,
    pub jump_cos_angle: Option<f32>,
    pub jump_xy_speed: Option<f32>,
    pub spline_elevation: Option<f32>,
    pub time: u32,
}

impl MovementInfo {
    pub fn new() -> Self {
        Self {
            mover_guid: ObjectGuid::empty(),
            flags: MoveFlags::NONE,
            position: Position::default(),
            transport_guid: None,
            transport_position: None,
            transport_time: None,
            fall_time: None,
            jump_velocity: None,
            jump_sin_angle: None,
            jump_cos_angle: None,
            jump_xy_speed: None,
            spline_elevation: None,
            time: 0,
        }
    }

    pub fn read_from_packet(packet: &mut WorldPacket) -> Result<Self> {
        let mut info = Self::new();

        let flags_value = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read movement flags"))?;
        info.flags = MoveFlags::from(flags_value);

        info.time = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read movement time"))?;

        let x = packet
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read position x"))?;
        let y = packet
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read position y"))?;
        let z = packet
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read position z"))?;
        let o = packet
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read position o"))?;
        info.position = Position::new(x, y, z, o);

        // Transport data (conditional on ONTRANSPORT 0x02000000)
        if info.flags.has_flag(MoveFlags::ONTRANSPORT) {
            let transport_guid_raw = packet
                .read_guid_raw()
                .ok_or_else(|| anyhow::anyhow!("Failed to read transport GUID"))?;
            info.transport_guid = Some(ObjectGuid::new_player(
                (transport_guid_raw & 0xFFFFFFFF) as u32,
            ));

            let tx = packet
                .read_f32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read transport x"))?;
            let ty = packet
                .read_f32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read transport y"))?;
            let tz = packet
                .read_f32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read transport z"))?;
            let to = packet
                .read_f32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read transport o"))?;
            info.transport_position = Some(Position::new(tx, ty, tz, to));

            info.transport_time = packet.read_u32();
        }

        // Swimming pitch (conditional on SWIMMING 0x00200000)
        if info.flags.has_flag(MoveFlags::SWIMMING) {
            let _s_pitch = packet
                .read_f32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read swimming pitch"))?;
        }

        // Fall time is ALWAYS present (unconditional)
        info.fall_time = packet.read_u32();

        // Jump data (conditional on JUMPING 0x2000)
        if info.flags.has_flag(MoveFlags::JUMPING) {
            info.jump_velocity = packet.read_f32();
            info.jump_sin_angle = packet.read_f32();
            info.jump_cos_angle = packet.read_f32();
            info.jump_xy_speed = packet.read_f32();
        }

        // Spline elevation (conditional on SPLINE_ELEVATION 0x04000000)
        if info.flags.has_flag(MoveFlags::SPLINE_ELEVATION) {
            info.spline_elevation = packet.read_f32();
        }

        Ok(info)
    }

    pub fn write_to_packet(&self, packet: &mut WorldPacket) {
        packet.write_packed_guid_raw(self.mover_guid.counter() as u64);

        packet.write_u32(self.flags.value());
        packet.write_u32(self.time);

        packet.write_f32(self.position.x);
        packet.write_f32(self.position.y);
        packet.write_f32(self.position.z);
        packet.write_f32(self.position.o);

        // Transport data (conditional on ONTRANSPORT 0x02000000)
        if self.flags.has_flag(MoveFlags::ONTRANSPORT) {
            if let Some(transport_guid) = self.transport_guid {
                packet.write_guid_raw(transport_guid.counter() as u64);
            } else {
                packet.write_guid_raw(0);
            }

            if let Some(transport_pos) = self.transport_position {
                packet.write_f32(transport_pos.x);
                packet.write_f32(transport_pos.y);
                packet.write_f32(transport_pos.z);
                packet.write_f32(transport_pos.o);
            } else {
                packet.write_f32(0.0);
                packet.write_f32(0.0);
                packet.write_f32(0.0);
                packet.write_f32(0.0);
            }

            packet.write_u32(self.transport_time.unwrap_or(0));
        }

        // Swimming pitch (conditional on SWIMMING 0x00200000)
        if self.flags.has_flag(MoveFlags::SWIMMING) {
            packet.write_f32(0.0); // s_pitch
        }

        // Fall time is ALWAYS written (unconditional)
        packet.write_u32(self.fall_time.unwrap_or(0));

        // Jump data (conditional on JUMPING 0x2000)
        if self.flags.has_flag(MoveFlags::JUMPING) {
            packet.write_f32(self.jump_velocity.unwrap_or(0.0));
            packet.write_f32(self.jump_sin_angle.unwrap_or(0.0));
            packet.write_f32(self.jump_cos_angle.unwrap_or(0.0));
            packet.write_f32(self.jump_xy_speed.unwrap_or(0.0));
        }

        // Spline elevation (conditional on SPLINE_ELEVATION 0x04000000)
        if self.flags.has_flag(MoveFlags::SPLINE_ELEVATION) {
            packet.write_f32(self.spline_elevation.unwrap_or(0.0));
        }
    }
}

impl Default for MovementInfo {
    fn default() -> Self {
        Self::new()
    }
}
