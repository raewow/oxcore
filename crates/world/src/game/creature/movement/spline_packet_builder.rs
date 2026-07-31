//! Serialization of spline state into the movement packets the client expects.
//!
//! Operates on the faithful [`MoveSpline`] port. Like that type, this is not yet wired
//! into creature movement, which builds its packets through
//! `oxcore_shared::messages::movement::SmsgMonsterMove`.

use super::move_spline::{MoveSpline, SplineFacing};
use super::spline_base::{EvaluationMode, Vec3};
use oxcore_shared::protocol::WorldPacket;

/// Cap on intermediate points in a single MONSTER_MOVE packet.
pub const MAX_POINTS_PER_PACKET: usize = 20;

/// How the client should orient the unit while it follows the spline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MonsterMoveType {
    Normal = 0,
    Stop = 1,
    FacingSpot = 2,
    FacingTarget = 3,
    FacingAngle = 4,
}

/// Spline flags the client must not see in a MONSTER_MOVE packet.
const MASK_NO_MONSTER_MOVE: u32 = 0x0000_0001;

/// Fake flag telling the client to drop the first vertex after one cycle completes.
const FLAG_ENTER_CYCLE: u32 = 0x0000_0080;

/// Raw spline flag bits, as sent on the wire.
fn raw_flags(spline: &MoveSpline) -> u32 {
    let flags = spline.flags();
    let mut raw = 0u32;
    if flags.done {
        raw |= MASK_NO_MONSTER_MOVE;
    }
    if flags.falling {
        raw |= 0x0000_0800;
    }
    if flags.cyclic {
        raw |= 0x0000_0040;
    }
    if flags.catmullrom {
        raw |= 0x0000_0400;
    }
    if flags.walkmode {
        raw |= 0x0000_0100;
    }
    raw
}

/// Header shared by every MONSTER_MOVE variant: start point, id, facing and timing.
pub fn write_common_monster_move_part(spline: &MoveSpline, data: &mut WorldPacket) {
    let start = spline.start_point().unwrap_or_default();
    data.write_f32(start.x);
    data.write_f32(start.y);
    data.write_f32(start.z);
    data.write_u32(spline.id());

    match spline.facing() {
        SplineFacing::Target(target) => {
            data.write_u8(MonsterMoveType::FacingTarget as u8);
            data.write_u64(target);
        }
        SplineFacing::Angle(angle) => {
            data.write_u8(MonsterMoveType::FacingAngle as u8);
            data.write_f32(angle);
        }
        SplineFacing::Point(point) => {
            data.write_u8(MonsterMoveType::FacingSpot as u8);
            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z);
        }
        SplineFacing::None => {
            data.write_u8(MonsterMoveType::Normal as u8);
        }
    }

    // Cyclic paths carry the fake enter-cycle flag so the client discards the first
    // vertex once a lap completes.
    let mut flags = raw_flags(spline);
    if spline.is_cyclic() {
        flags |= FLAG_ENTER_CYCLE;
    }
    data.write_u32(flags & !MASK_NO_MONSTER_MOVE);
    data.write_u32(spline.duration() as u32);
}

/// Points of a linear path: the destination in full, then packed offsets.
///
/// Returns the index the write stopped at when the path had to be split across packets,
/// or `None` when the whole path fit.
fn write_linear_path(spline: &MoveSpline, data: &mut WorldPacket, start: usize) -> Option<usize> {
    let path = spline.real_path();
    if path.is_empty() {
        data.write_u32(0);
        return None;
    }

    // The curve carries virtual points the client never sees; the last real index is
    // three back from the end of the point array.
    let mut last_idx = spline.curve_points().len().saturating_sub(3);
    let mut split_at = None;
    if last_idx.saturating_sub(start) > MAX_POINTS_PER_PACKET {
        last_idx = start + MAX_POINTS_PER_PACKET;
        split_at = Some(last_idx);
    }

    let destination = path[last_idx];

    data.write_u32((last_idx - start + 1) as u32);
    data.write_f32(destination.x);
    data.write_f32(destination.y);
    data.write_f32(destination.z);

    if last_idx > 1 {
        for point in &path[start..last_idx] {
            let mut offset = Vec3::new(
                destination.x - point.x,
                destination.y - point.y,
                destination.z - point.z,
            );

            // A fully zero offset freezes the 1.12 client, so nudge Z off zero.
            if offset.x.abs() < 0.25 && offset.y.abs() < 0.25 && offset.z.abs() < 0.25 {
                offset.z += if offset.z < 0.0 { 0.51 } else { 0.26 };
            }

            data.write_pack_xyz(offset.x, offset.y, offset.z);
        }
    }

    split_at
}

/// Points of a Catmull-Rom path, which are sent in full rather than packed.
fn write_catmull_rom_path(spline: &MoveSpline, data: &mut WorldPacket) {
    let points = spline.curve_points();
    let count = points.len().saturating_sub(3);

    if spline.is_cyclic() {
        data.write_u32((count + 1) as u32);
        // Duplicate leading point; the client erases it after the first lap.
        if let Some(first) = points.get(1) {
            data.write_f32(first.x);
            data.write_f32(first.y);
            data.write_f32(first.z);
        }
        for point in points.iter().skip(1).take(count) {
            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z);
        }
    } else {
        data.write_u32(count as u32);
        for point in points.iter().skip(2).take(count) {
            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z);
        }
    }
}

/// Write a full SMSG_MONSTER_MOVE body.
///
/// Returns the index a split path stopped at, or `None` when the path was complete.
pub fn write_monster_move(
    spline: &MoveSpline,
    data: &mut WorldPacket,
    first_point: usize,
) -> Option<usize> {
    write_common_monster_move_part(spline, data);

    if spline.flags().catmullrom {
        write_catmull_rom_path(spline, data);
        return None;
    }

    write_linear_path(spline, data, first_point)
    // Rewriting the duration field afterwards for a split path needs a positioned write the
    // packet API does not expose, so a split path currently carries the whole-spline duration.
}

/// Write the spline block used when creating the unit for a client mid-movement.
pub fn write_create(spline: &MoveSpline, data: &mut WorldPacket) {
    if !spline.initialized() {
        return;
    }

    data.write_u32(raw_flags(spline));

    match spline.facing() {
        SplineFacing::Angle(angle) => data.write_f32(angle),
        SplineFacing::Target(target) => data.write_u64(target),
        SplineFacing::Point(point) => {
            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z);
        }
        SplineFacing::None => {}
    }

    data.write_u32(spline.time_passed() as u32);
    data.write_u32(spline.duration() as u32);
    data.write_u32(spline.id());

    let path = spline.curve_points();
    data.write_u32(path.len() as u32);

    if spline.is_cyclic() {
        for point in path {
            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z);
        }
    } else {
        for (index, point) in path.iter().enumerate() {
            // The client asserts on a zero-length step, so nudge duplicates apart.
            let offset = match index {
                0 => 0.0,
                _ if path[index - 1] == *point => {
                    if index % 2 == 1 {
                        0.01
                    } else {
                        0.02
                    }
                }
                _ => 0.0,
            };

            data.write_f32(point.x);
            data.write_f32(point.y);
            data.write_f32(point.z + offset);
        }
    }

    let tail = if spline.is_cyclic() {
        Vec3::default()
    } else {
        spline.final_destination()
    };
    data.write_f32(tail.x);
    data.write_f32(tail.y);
    data.write_f32(tail.z);
}

/// Evaluation mode a spline was built with, for callers inspecting a built packet.
pub fn spline_mode(spline: &MoveSpline) -> EvaluationMode {
    if spline.flags().catmullrom {
        EvaluationMode::CatmullRom
    } else {
        EvaluationMode::Linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::movement::move_spline::{MoveSplineFlags, MoveSplineInitArgs};
    use oxcore_shared::protocol::Opcode;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn linear_spline() -> MoveSpline {
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            path: vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(30.0, 0.0, 0.0)],
            velocity: 10.0,
            spline_id: 42,
            ..MoveSplineInitArgs::default()
        });
        spline
    }

    fn packet() -> WorldPacket {
        WorldPacket::new(Opcode::SMSG_MONSTER_MOVE)
    }

    fn read_f32(bytes: &[u8], at: usize) -> f32 {
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    fn read_u32(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    #[test]
    fn common_part_writes_start_id_facing_and_duration() {
        let spline = linear_spline();
        let mut data = packet();
        write_common_monster_move_part(&spline, &mut data);
        let bytes = data.data().to_vec();

        // start xyz, id, move type byte, flags, duration
        assert_eq!(read_f32(&bytes, 0), 0.0);
        assert_eq!(read_f32(&bytes, 4), 0.0);
        assert_eq!(read_f32(&bytes, 8), 0.0);
        assert_eq!(read_u32(&bytes, 12), 42);
        assert_eq!(bytes[16], MonsterMoveType::Normal as u8);
        assert_eq!(read_u32(&bytes, 21), spline.duration() as u32);
    }

    #[test]
    fn facing_variants_pick_their_move_type_and_payload() {
        let mut args = MoveSplineInitArgs {
            path: vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0)],
            velocity: 10.0,
            ..MoveSplineInitArgs::default()
        };

        args.facing = SplineFacing::Angle(1.5);
        let mut spline = MoveSpline::new();
        spline.initialize(&args);
        let mut data = packet();
        write_common_monster_move_part(&spline, &mut data);
        let bytes = data.data().to_vec();
        assert_eq!(bytes[16], MonsterMoveType::FacingAngle as u8);
        assert_eq!(read_f32(&bytes, 17), 1.5);

        args.facing = SplineFacing::Target(0x1234);
        let mut spline = MoveSpline::new();
        spline.initialize(&args);
        let mut data = packet();
        write_common_monster_move_part(&spline, &mut data);
        assert_eq!(data.data()[16], MonsterMoveType::FacingTarget as u8);

        args.facing = SplineFacing::Point(v(5.0, 6.0, 7.0));
        let mut spline = MoveSpline::new();
        spline.initialize(&args);
        let mut data = packet();
        write_common_monster_move_part(&spline, &mut data);
        let bytes = data.data().to_vec();
        assert_eq!(bytes[16], MonsterMoveType::FacingSpot as u8);
        assert_eq!(read_f32(&bytes, 17), 5.0);
        assert_eq!(read_f32(&bytes, 21), 6.0);
        assert_eq!(read_f32(&bytes, 25), 7.0);
    }

    /// Header size for a normal facing: 12 start + 4 id + 1 type + 4 flags + 4 duration.
    const HEADER_LEN: usize = 25;

    #[test]
    fn a_three_point_path_sends_only_the_destination() {
        let spline = linear_spline();
        let mut data = packet();
        assert!(write_monster_move(&spline, &mut data, 0).is_none());
        let bytes = data.data().to_vec();

        assert_eq!(read_u32(&bytes, HEADER_LEN), 2);
        assert_eq!(read_f32(&bytes, HEADER_LEN + 4), 30.0);
        // The offset loop is guarded by `last_idx > 1`, which is false here, so the
        // middle point is not sent at all.
        assert_eq!(bytes.len(), HEADER_LEN + 4 + 12);
    }

    #[test]
    fn longer_linear_paths_pack_their_intermediate_offsets() {
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            path: vec![
                v(0.0, 0.0, 0.0),
                v(10.0, 0.0, 0.0),
                v(20.0, 0.0, 0.0),
                v(30.0, 0.0, 0.0),
            ],
            velocity: 10.0,
            ..MoveSplineInitArgs::default()
        });

        let mut data = packet();
        assert!(write_monster_move(&spline, &mut data, 0).is_none());
        let bytes = data.data().to_vec();

        assert_eq!(read_u32(&bytes, HEADER_LEN), 3);
        assert_eq!(read_f32(&bytes, HEADER_LEN + 4), 30.0);
        // Two intermediate points, each a packed u32 after the full destination.
        assert_eq!(bytes.len(), HEADER_LEN + 4 + 12 + 8);
    }

    #[test]
    fn cyclic_flag_is_added_for_looping_paths() {
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            path: vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(30.0, 0.0, 0.0)],
            velocity: 10.0,
            flags: MoveSplineFlags {
                cyclic: true,
                ..MoveSplineFlags::default()
            },
            ..MoveSplineInitArgs::default()
        });

        let mut data = packet();
        write_common_monster_move_part(&spline, &mut data);
        let flags = read_u32(&data.data().to_vec(), 17);

        assert!(flags & FLAG_ENTER_CYCLE != 0, "enter-cycle flag missing");
    }

    #[test]
    fn catmull_rom_paths_send_full_points_and_never_split() {
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            path: vec![
                v(0.0, 0.0, 0.0),
                v(10.0, 0.0, 0.0),
                v(20.0, 10.0, 0.0),
                v(30.0, 10.0, 0.0),
            ],
            velocity: 10.0,
            flags: MoveSplineFlags {
                catmullrom: true,
                ..MoveSplineFlags::default()
            },
            ..MoveSplineInitArgs::default()
        });

        let mut data = packet();
        assert!(write_monster_move(&spline, &mut data, 0).is_none());

        let bytes = data.data().to_vec();
        let count = read_u32(&bytes, 25);
        // Points minus the three virtual ones, each written as a full xyz triple.
        assert_eq!(bytes.len(), 25 + 4 + count as usize * 12);
    }

    #[test]
    fn write_create_skips_an_uninitialized_spline() {
        let spline = MoveSpline::new();
        let mut data = packet();
        write_create(&spline, &mut data);

        assert!(data.data().is_empty());
    }

    #[test]
    fn write_create_emits_timing_id_and_every_path_point() {
        let spline = linear_spline();
        let mut data = packet();
        write_create(&spline, &mut data);
        let bytes = data.data().to_vec();

        // flags, time passed, duration, id, node count
        assert_eq!(read_u32(&bytes, 4), 0); // no time passed yet
        assert_eq!(read_u32(&bytes, 8), spline.duration() as u32);
        assert_eq!(read_u32(&bytes, 12), 42);
        let nodes = read_u32(&bytes, 16);
        assert_eq!(nodes as usize, spline.curve_points().len());
        // node triples plus the trailing destination triple
        assert_eq!(bytes.len(), 20 + nodes as usize * 12 + 12);
    }

    #[test]
    fn a_long_path_is_split_and_reports_where_it_stopped() {
        let path: Vec<Vec3> = (0..40).map(|i| v(i as f32 * 10.0, 0.0, 0.0)).collect();
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            path,
            velocity: 10.0,
            ..MoveSplineInitArgs::default()
        });

        let mut data = packet();
        let split = write_monster_move(&spline, &mut data, 0);

        assert_eq!(split, Some(MAX_POINTS_PER_PACKET));
    }
}
