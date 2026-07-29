use anyhow::Result;

use super::{ObjectGuid, Position, Protocol, WorldPacket};

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

    /// Parse a 1.14 client movement body.
    ///
    /// Mirrors `ObjectUpdateBuilder`'s writer and HermesProxy's `ReadMovementInfoModern`
    /// (`World/Objects/MovementInfo.cs:276`). Structurally different from vanilla, not just
    /// renumbered:
    ///
    /// * the mover GUID leads the body, where vanilla sends none;
    /// * flags come *before* the timestamp as three plain u32s (from 1.14.1 on; earlier builds
    ///   bit-packed them after the position, so this is specific to our target build);
    /// * pitch and spline elevation are unconditional, where vanilla gates them on flags;
    /// * transport, fall and jump data are selected by a run of presence bits rather than by
    ///   movement flags.
    ///
    /// The flag word is translated back to vanilla so everything downstream — anticheat, collision,
    /// the movement state machine — keeps working on one representation.
    pub fn read_modern(packet: &mut WorldPacket) -> Result<Self> {
        use crate::protocol::bitbuf::BitReader;
        use crate::protocol::updates::modern::block::from_modern_movement_flags;

        let mut reader = BitReader::new(packet.contents());
        macro_rules! need {
            ($value:expr, $what:literal) => {
                $value.ok_or_else(|| anyhow::anyhow!(concat!("truncated movement: ", $what)))?
            };
        }

        let mut info = Self::new();

        // MoverGUID. The counter is all we keep: the caller overrides this with the session's
        // player anyway, and trusting a client-supplied mover would let one player move another.
        let (_high, low) = need!(reader.read_packed_guid_128(), "mover guid");
        info.mover_guid = ObjectGuid::new_player(low as u32);

        info.flags = MoveFlags::from(from_modern_movement_flags(need!(
            reader.read_u32(),
            "flags"
        )));
        let _flags_extra = need!(reader.read_u32(), "flags extra");
        let _flags_extra2 = need!(reader.read_u32(), "flags extra 2");

        info.time = need!(reader.read_u32(), "move time");
        let x = need!(reader.read_f32(), "position x");
        let y = need!(reader.read_f32(), "position y");
        let z = need!(reader.read_f32(), "position z");
        let o = need!(reader.read_f32(), "orientation");
        info.position = Position::new(x, y, z, o);

        let _pitch = need!(reader.read_f32(), "pitch");
        info.spline_elevation = Some(need!(reader.read_f32(), "spline elevation"));

        let remove_forces = need!(reader.read_u32(), "remove forces count");
        let _move_index = need!(reader.read_u32(), "move index");
        for _ in 0..remove_forces {
            need!(reader.read_packed_guid_128(), "removed force guid");
        }

        let has_transport = need!(reader.read_bit(), "has transport");
        let has_fall = need!(reader.read_bit(), "has fall");
        let _has_spline = need!(reader.read_bit(), "has spline");
        let _height_change_failed = need!(reader.read_bit(), "height change failed");
        let _remote_time_valid = need!(reader.read_bit(), "remote time valid");
        let has_inertia = need!(reader.read_bit(), "has inertia");

        if has_transport {
            let (_high, low) = need!(reader.read_packed_guid_128(), "transport guid");
            info.transport_guid = Some(ObjectGuid::new_player(low as u32));
            let tx = need!(reader.read_f32(), "transport x");
            let ty = need!(reader.read_f32(), "transport y");
            let tz = need!(reader.read_f32(), "transport z");
            let to = need!(reader.read_f32(), "transport o");
            info.transport_position = Some(Position::new(tx, ty, tz, to));
            let _seat = need!(reader.read_u8(), "transport seat");
            info.transport_time = Some(need!(reader.read_u32(), "transport time"));

            let has_prev_time = need!(reader.read_bit(), "has prev time");
            let has_vehicle_id = need!(reader.read_bit(), "has vehicle id");
            if has_prev_time {
                need!(reader.read_u32(), "prev move time");
            }
            if has_vehicle_id {
                need!(reader.read_u32(), "vehicle id");
            }
        }

        // Inertia has no vanilla equivalent, but it still has to be consumed or everything after
        // it is misread.
        if has_inertia {
            need!(reader.read_packed_guid_128(), "inertia guid");
            need!(reader.read_f32(), "inertia force x");
            need!(reader.read_f32(), "inertia force y");
            need!(reader.read_f32(), "inertia force z");
            need!(reader.read_u32(), "inertia lifetime");
        }

        if has_fall {
            info.fall_time = Some(need!(reader.read_u32(), "fall time"));
            info.jump_velocity = Some(need!(reader.read_f32(), "jump velocity"));

            if need!(reader.read_bit(), "has fall direction") {
                info.jump_sin_angle = Some(need!(reader.read_f32(), "jump sin"));
                info.jump_cos_angle = Some(need!(reader.read_f32(), "jump cos"));
                info.jump_xy_speed = Some(need!(reader.read_f32(), "jump speed"));
            }
        }

        // The bit reader worked off a borrowed slice, so move the packet's own cursor to match --
        // otherwise anything the caller reads afterwards re-reads the movement block.
        let consumed = reader.consumed();
        packet.advance(consumed);

        Ok(info)
    }

    /// Write this movement block in the 1.14 layout.
    ///
    /// The exact inverse of [`Self::read_modern`], kept next to it so the two stay in step. Also
    /// used by the create-object movement block, which embeds the same structure.
    pub fn write_modern(
        &self,
        writer: &mut crate::protocol::bitbuf::BitWriter,
        mover: ObjectGuid,
        realm_id: u16,
    ) {
        use crate::protocol::updates::modern::block::to_modern_movement_flags;

        let (high, low) = mover.to_guid128(realm_id);
        writer.write_packed_guid_128(high, low);

        writer.write_u32(to_modern_movement_flags(self.flags.value()));
        writer.write_u32(0); // FlagsExtra
        writer.write_u32(0); // FlagsExtra2

        writer.write_u32(self.time);
        writer.write_f32(self.position.x);
        writer.write_f32(self.position.y);
        writer.write_f32(self.position.z);
        writer.write_f32(self.position.o);

        writer.write_f32(0.0); // Pitch
        writer.write_f32(self.spline_elevation.unwrap_or(0.0));

        writer.write_u32(0); // RemoveForcesIDs count
        writer.write_u32(0); // MoveIndex

        // 1.14 selects the optional blocks with presence bits rather than movement flags.
        let has_transport = self.transport_guid.is_some();
        let has_fall =
            self.fall_time.is_some_and(|time| time != 0) || self.flags.has_flag(MoveFlags::JUMPING);

        writer.write_bit(has_transport);
        writer.write_bit(has_fall);
        writer.write_bit(false); // HasSpline
        writer.write_bit(false); // HeightChangeFailed
        writer.write_bit(false); // RemoteTimeValid
        writer.write_bit(false); // HasInertia
        writer.flush_bits();

        if has_transport {
            let (high, low) = self.transport_guid.unwrap_or_default().to_guid128(realm_id);
            writer.write_packed_guid_128(high, low);
            let offset = self.transport_position.unwrap_or_default();
            writer.write_f32(offset.x);
            writer.write_f32(offset.y);
            writer.write_f32(offset.z);
            writer.write_f32(offset.o);
            writer.write_u8(0); // VehicleSeatIndex
            writer.write_u32(self.transport_time.unwrap_or(0));
            writer.write_bit(false); // HasPrevMoveTime
            writer.write_bit(false); // HasVehicleRecID
            writer.flush_bits();
        }

        if has_fall {
            writer.write_u32(self.fall_time.unwrap_or(0));
            writer.write_f32(self.jump_velocity.unwrap_or(0.0));

            let has_direction = self.jump_xy_speed.is_some();
            writer.write_bit(has_direction);
            writer.flush_bits();

            if has_direction {
                writer.write_f32(self.jump_sin_angle.unwrap_or(0.0));
                writer.write_f32(self.jump_cos_angle.unwrap_or(0.0));
                writer.write_f32(self.jump_xy_speed.unwrap_or(0.0));
            }
        }
    }

    /// Parse a movement body in whichever layout the client speaks.
    ///
    /// The two layouts share no structure, so this is a branch rather than a translation; see
    /// [`Self::read_modern`].
    pub fn read_for(protocol: Protocol, packet: &mut WorldPacket) -> Result<Self> {
        match protocol {
            Protocol::Vanilla => Self::read_from_packet(packet),
            Protocol::Modern => Self::read_modern(packet),
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

#[cfg(test)]
mod modern_tests {
    use super::*;
    use crate::protocol::bitbuf::BitWriter;
    use crate::protocol::updates::modern::block::{
        from_modern_movement_flags, to_modern_movement_flags,
    };
    use crate::protocol::Opcode;

    /// Build a 1.14 client movement body. Mirrors what the client sends, so the reader is tested
    /// against the layout rather than against itself.
    fn modern_body(flags: u32, position: Position, fall: Option<(u32, f32)>) -> WorldPacket {
        let mut w = BitWriter::new();
        w.write_packed_guid_128(0x0800_0400_0000_0000, 42); // MoverGUID
        w.write_u32(flags);
        w.write_u32(0); // FlagsExtra
        w.write_u32(0); // FlagsExtra2
        w.write_u32(1234); // MoveTime
        w.write_f32(position.x);
        w.write_f32(position.y);
        w.write_f32(position.z);
        w.write_f32(position.o);
        w.write_f32(0.5); // Pitch
        w.write_f32(1.5); // StepUpStartElevation
        w.write_u32(0); // RemoveForcesIDs count
        w.write_u32(0); // MoveIndex

        w.write_bit(false); // HasTransport
        w.write_bit(fall.is_some()); // HasFall
        w.write_bit(false); // HasSpline
        w.write_bit(false); // HeightChangeFailed
        w.write_bit(false); // RemoteTimeValid
        w.write_bit(false); // HasInertia
        w.flush_bits();

        if let Some((time, velocity)) = fall {
            w.write_u32(time);
            w.write_f32(velocity);
            w.write_bit(false); // no fall direction
            w.flush_bits();
        }

        w.finish(Opcode::MSG_MOVE_HEARTBEAT)
    }

    #[test]
    fn reads_position_time_and_flags() {
        let position = Position::new(100.0, 200.0, 300.0, 1.5);
        let mut packet = modern_body(
            to_modern_movement_flags(MoveFlags::FORWARD.value()),
            position,
            None,
        );

        let info = MovementInfo::read_modern(&mut packet).expect("parsed");

        assert_eq!(info.position.x, 100.0);
        assert_eq!(info.position.y, 200.0);
        assert_eq!(info.position.z, 300.0);
        assert_eq!(info.position.o, 1.5);
        assert_eq!(info.time, 1234);
        assert!(info.flags.has_flag(MoveFlags::FORWARD));
        assert_eq!(info.spline_elevation, Some(1.5));
    }

    /// Fall data sits behind a presence bit, not behind a movement flag as it does in vanilla.
    #[test]
    fn reads_fall_data_from_its_presence_bit() {
        let mut packet = modern_body(0, Position::default(), Some((900, -7.5)));

        let info = MovementInfo::read_modern(&mut packet).expect("parsed");

        assert_eq!(info.fall_time, Some(900));
        assert_eq!(info.jump_velocity, Some(-7.5));
        assert_eq!(info.jump_sin_angle, None, "no fall direction was sent");
    }

    /// The parse must leave the packet cursor where it finished, or a caller reading afterwards
    /// re-reads the movement block.
    #[test]
    fn consumes_the_bytes_it_parsed() {
        let mut packet = modern_body(0, Position::default(), None);
        let before = packet.contents().len();

        MovementInfo::read_modern(&mut packet).expect("parsed");

        assert!(packet.contents().len() < before);
        assert_eq!(packet.contents().len(), 0, "the body is entirely movement");
    }

    /// A truncated body must be an error, not a panic or a half-populated position -- this is
    /// attacker-controlled input.
    #[test]
    fn a_truncated_body_is_rejected() {
        let full = modern_body(0, Position::default(), None);
        for length in 0..full.contents().len() {
            let mut packet = WorldPacket::new(Opcode::MSG_MOVE_HEARTBEAT);
            packet.write_bytes(&full.contents()[..length]);
            assert!(
                MovementInfo::read_modern(&mut packet).is_err(),
                "a {length}-byte body should not parse"
            );
        }
    }

    /// Flags round-trip, so a client's word survives the trip out and back.
    #[test]
    fn flag_translation_round_trips() {
        for flag in [
            MoveFlags::FORWARD,
            MoveFlags::BACKWARD,
            MoveFlags::WALK_MODE,
            MoveFlags::ROOT,
            MoveFlags::JUMPING,
            MoveFlags::SWIMMING,
            MoveFlags::FLYING,
            MoveFlags::WATERWALKING,
            MoveFlags::HOVER,
        ] {
            let value = flag.value();
            assert_eq!(
                from_modern_movement_flags(to_modern_movement_flags(value)),
                value,
                "flag 0x{value:08X} did not survive the round trip"
            );
        }
    }

    /// The writer and reader must agree exactly: the writer feeds observers' `SMSG_MOVE_UPDATE`
    /// and the create-object movement block, the reader consumes client input. A drift between
    /// them desynchronises the stream rather than failing cleanly, so round-trip them.
    #[test]
    fn write_modern_round_trips_through_read_modern() {
        let mut original = MovementInfo::new();
        original.flags = MoveFlags::from(
            MoveFlags::FORWARD.value() | MoveFlags::JUMPING.value() | MoveFlags::SWIMMING.value(),
        );
        original.position = Position::new(-8900.5, 700.25, 96.75, 2.25);
        original.time = 987_654;
        original.spline_elevation = Some(3.5);
        original.fall_time = Some(450);
        original.jump_velocity = Some(-9.5);
        original.jump_sin_angle = Some(0.5);
        original.jump_cos_angle = Some(0.75);
        original.jump_xy_speed = Some(6.25);

        let mover = ObjectGuid::new_player(42);
        let mut writer = BitWriter::new();
        original.write_modern(&mut writer, mover, 1);
        let mut packet = writer.finish(Opcode::SMSG_MOVE_UPDATE);

        let parsed = MovementInfo::read_modern(&mut packet).expect("parsed");

        assert_eq!(parsed.flags.value(), original.flags.value());
        assert_eq!(parsed.position.x, original.position.x);
        assert_eq!(parsed.position.y, original.position.y);
        assert_eq!(parsed.position.z, original.position.z);
        assert_eq!(parsed.position.o, original.position.o);
        assert_eq!(parsed.time, original.time);
        assert_eq!(parsed.spline_elevation, original.spline_elevation);
        assert_eq!(parsed.fall_time, original.fall_time);
        assert_eq!(parsed.jump_velocity, original.jump_velocity);
        assert_eq!(parsed.jump_sin_angle, original.jump_sin_angle);
        assert_eq!(parsed.jump_cos_angle, original.jump_cos_angle);
        assert_eq!(parsed.jump_xy_speed, original.jump_xy_speed);
        assert_eq!(packet.contents().len(), 0, "the whole body was consumed");
    }

    /// A move with no fall data must not emit the fall block, or the reader reads the next
    /// message's bytes as a fall time.
    #[test]
    fn round_trips_without_optional_blocks() {
        let mut original = MovementInfo::new();
        original.flags = MoveFlags::FORWARD;
        original.position = Position::new(1.0, 2.0, 3.0, 0.5);
        original.time = 7;

        let mut writer = BitWriter::new();
        original.write_modern(&mut writer, ObjectGuid::new_player(1), 1);
        let mut packet = writer.finish(Opcode::SMSG_MOVE_UPDATE);

        let parsed = MovementInfo::read_modern(&mut packet).expect("parsed");

        assert_eq!(parsed.fall_time, None);
        assert_eq!(parsed.transport_guid, None);
        assert_eq!(packet.contents().len(), 0);
    }

    /// Vanilla `Root` is 0x1000, which 1.14 reads as `FallingFar`. A pass-through would make every
    /// rooted player look like they were falling.
    #[test]
    fn root_is_not_confused_with_falling_far() {
        assert_eq!(
            to_modern_movement_flags(MoveFlags::ROOT.value()),
            0x0000_0400
        );
        assert_eq!(
            from_modern_movement_flags(0x0000_1000),
            MoveFlags::FALLINGFAR.value()
        );
    }
}
