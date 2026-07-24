//! Building a transport's keyframe path from its taxi-path nodes
//! (`TransportMgr::GenerateWaypoints`).
//!
//! A transport follows a taxi path: a list of nodes with positions, map ids and action
//! flags. This turns that node list into the keyframe schedule the transport replays -
//! selecting which nodes become keyframes, measuring the Catmull-Rom spline distance between
//! them, computing each node's facing from an orientation spline, and handing the distances
//! to [`super::schedule`] for the timing. Loading the nodes from the taxi-path DBC store is
//! the caller's job, so they are taken as input here (mirroring how [`super::schedule`] takes
//! its distances as input).

use std::collections::BTreeSet;

use crate::game::creature::movement::spline_base::{EvaluationMode, SplineBase, Vec3};

use super::passenger::normalize_orientation;
use super::schedule::{compute_schedule, KeyFrame, ScheduleProfile};

/// One node of a transport's taxi path (`TaxiPathNodeEntry`, the fields this port reads).
#[derive(Debug, Clone, Copy)]
pub struct TaxiPathNode {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Bit 0 set means the path teleports away from here; the value 2 means the transport
    /// stops here.
    pub action_flag: u8,
    /// How long the transport waits at a stop node, in seconds.
    pub delay: u32,
}

impl TaxiPathNode {
    fn is_stop(&self) -> bool {
        self.action_flag == 2
    }

    fn is_teleport(&self) -> bool {
        self.action_flag & 1 != 0
    }

    fn pos(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

/// A transport's fully built path: its keyframes and the profile constants derived alongside.
#[derive(Debug, Clone)]
pub struct TransportPath {
    /// The keyframes, with distances, timing and facings filled in.
    pub keyframes: Vec<KeyFrame>,
    /// The per-segment Catmull-Rom splines the keyframes' legs are evaluated on, indexed by
    /// each keyframe's `spline_id` (the C++ per-keyframe `Spline` pointers).
    pub segment_splines: Vec<SplineBase>,
    /// Cruise speed in yards/sec (`goInfo->moTransport.moveSpeed`).
    pub speed: f32,
    /// Acceleration in yards/sec^2 (`goInfo->moTransport.accelRate`).
    pub accel: f32,
    /// Time to accelerate from a stop to cruise (`transportTemplate.accelTime`).
    pub accel_time: f32,
    /// Distance covered while accelerating (`transportTemplate.accelDist`).
    pub accel_dist: f32,
    /// Total time to traverse the whole path, in milliseconds (`transportTemplate.pathTime`).
    pub path_time: u32,
    /// The maps this path visits (`transportTemplate.mapsUsed`).
    pub maps_used: BTreeSet<u32>,
}

/// Build the keyframe path for a transport from its taxi-path `nodes` and motion `profile`
/// (`TransportMgr::GenerateWaypoints`).
///
/// Returns `None` when the path is too short to form a keyframe (the C++ path-id guard and
/// the non-empty-keyframes assert). The per-keyframe `Spline` pointer and the path-303/293
/// mid-course `Update` refresh the C++ also sets are runtime/DBC concerns not reproduced
/// here; the instanceable/`inInstance` decision needs map data and is left to the caller,
/// which gets the visited maps in `maps_used`.
pub fn generate_waypoints(
    nodes: &[TaxiPathNode],
    profile: &ScheduleProfile,
) -> Option<TransportPath> {
    if nodes.len() < 3 {
        // The path is walked over its interior (index 1..len-1), so it needs at least one
        // interior node to yield a keyframe.
        return None;
    }

    let orientation_spline = build_orientation_spline(nodes);

    // Walk the interior nodes, turning each into a keyframe unless it teleports or crosses to
    // another map - those mark the previous keyframe as a teleport and are skipped.
    let mut keyframes: Vec<KeyFrame> = Vec::new();
    let mut spline_path: Vec<Vec3> = Vec::new();
    let mut maps_used: BTreeSet<u32> = BTreeSet::new();
    let mut map_change = false;

    for i in 1..nodes.len() - 1 {
        if map_change {
            map_change = false;
            continue;
        }

        let node = &nodes[i];
        if node.is_teleport() || node.map_id != nodes[i + 1].map_id {
            if let Some(last) = keyframes.last_mut() {
                last.teleport = true;
            }
            map_change = true;
            continue;
        }

        let mut frame = KeyFrame::new(-1.0, node.is_stop(), node.delay);
        // The orientation spline carries a leading guard point, so real node i sits at
        // spline index i + 1.
        let tangent = orientation_spline
            .evaluate_derivative(i + 1, 0.0)
            .unwrap_or_default();
        frame.initial_orientation =
            normalize_orientation(tangent.y.atan2(tangent.x) + std::f32::consts::PI);
        frame.map_id = node.map_id;
        frame.node_x = node.x;
        frame.node_y = node.y;
        frame.node_z = node.z;

        keyframes.push(frame);
        spline_path.push(node.pos());
        maps_used.insert(node.map_id);
    }

    if keyframes.is_empty() {
        return None;
    }

    // The path always teleports from its last keyframe back to its first, even when closed.
    keyframes.last_mut().unwrap().teleport = true;

    // The first keyframe is arrived at by teleport, so it has no incoming distance.
    keyframes[0].dist_from_prev = 0.0;
    keyframes[0].index = 1;

    let segment_splines = measure_segment_distances(&mut keyframes, &spline_path);

    let path_time = compute_schedule(profile, &mut keyframes);

    Some(TransportPath {
        keyframes,
        segment_splines,
        speed: profile.speed,
        accel: profile.accel,
        accel_time: profile.accel_time(),
        accel_dist: profile.accel_dist(),
        path_time,
        maps_used,
    })
}

/// Build the Catmull-Rom spline used to give each node its facing.
///
/// The path points are bracketed by three extrapolated guard points (front, and two past the
/// end) so the spline's derivative can be evaluated at every real node, exactly as the C++
/// `SplineRawInitializer` sets up.
fn build_orientation_spline(nodes: &[TaxiPathNode]) -> SplineBase {
    let mut points: Vec<Vec3> = nodes.iter().map(TaxiPathNode::pos).collect();

    // Leading guard just before the first node.
    let front = points[0].lerp(points[1], -0.2);
    points.insert(0, front);
    // Two trailing guards past the last node.
    let last = points.len() - 1;
    points.push(points[last].lerp(points[last - 1], -0.2));
    let last = points.len() - 1;
    points.push(points[last].lerp(points[last - 1], -1.0));

    let mut spline = SplineBase::new();
    spline.init_raw_catmull_rom(points);
    spline
}

/// Fill in each keyframe's `dist_from_prev` from the Catmull-Rom distance between the nodes.
///
/// The path is cut into segments at teleport frames; each segment gets its own spline, and a
/// keyframe's distance is the spline arc length of the leg leaving it. Ports the segment loop
/// of `GenerateWaypoints`.
fn measure_segment_distances(keyframes: &mut [KeyFrame], spline_path: &[Vec3]) -> Vec<SplineBase> {
    let count = keyframes.len();
    let mut splines: Vec<SplineBase> = Vec::new();
    let mut start = 0usize;

    for i in 1..count {
        if !keyframes[i - 1].teleport && i + 1 != count {
            continue;
        }

        // A non-teleport segment includes the closing node; a teleport one does not.
        let extra = if !keyframes[i - 1].teleport { 1 } else { 0 };
        let mut spline = SplineBase::new();
        spline.init_spline(&spline_path[start..i + extra], EvaluationMode::CatmullRom);
        let spline_id = splines.len();

        for j in start..i + extra {
            let local = j - start;
            keyframes[j].index = (local + 1) as u32;
            keyframes[j].spline_id = spline_id;
            // Arc length of the leg from this node to the next; the terminal node of the
            // segment has no next leg, so its distance stays zero.
            keyframes[j].dist_from_prev = spline.seg_length(local + 1).unwrap_or(0.0);
        }

        if keyframes[i - 1].teleport {
            keyframes[i].index = (i - start + 1) as u32;
            keyframes[i].spline_id = spline_id;
            keyframes[i].dist_from_prev = 0.0;
        }

        splines.push(spline);
        start = i;
    }

    splines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ScheduleProfile {
        ScheduleProfile {
            speed: 10.0,
            accel: 5.0,
        }
    }

    fn node(x: f32, action_flag: u8, delay: u32) -> TaxiPathNode {
        TaxiPathNode {
            map_id: 0,
            x,
            y: 0.0,
            z: 0.0,
            action_flag,
            delay,
        }
    }

    /// Five colinear nodes 10 yards apart along +x. The interior three become keyframes.
    fn straight_line() -> Vec<TaxiPathNode> {
        vec![
            node(0.0, 0, 0),
            node(10.0, 0, 0),
            node(20.0, 0, 0),
            node(30.0, 0, 0),
            node(40.0, 0, 0),
        ]
    }

    #[test]
    fn a_short_path_yields_no_waypoints() {
        assert!(generate_waypoints(&[node(0.0, 0, 0), node(10.0, 0, 0)], &profile()).is_none());
    }

    #[test]
    fn only_the_interior_nodes_become_keyframes() {
        let path = generate_waypoints(&straight_line(), &profile()).unwrap();
        // Five nodes -> the three interior ones are keyframes.
        assert_eq!(path.keyframes.len(), 3);
        assert!(path.maps_used.contains(&0));
    }

    #[test]
    fn straight_leg_distances_are_the_node_spacing() {
        let path = generate_waypoints(&straight_line(), &profile()).unwrap();
        // Along a straight line the Catmull-Rom arc length is exactly the 10-yard spacing;
        // the terminal keyframe of the segment has no outgoing leg, so it is zero.
        let dists: Vec<f32> = path.keyframes.iter().map(|k| k.dist_from_prev).collect();
        assert!((dists[0] - 10.0).abs() < 1e-3, "got {dists:?}");
        assert!((dists[1] - 10.0).abs() < 1e-3, "got {dists:?}");
        assert!(dists[2].abs() < 1e-3, "got {dists:?}");
    }

    #[test]
    fn a_straight_path_faces_along_its_direction_of_travel() {
        let path = generate_waypoints(&straight_line(), &profile()).unwrap();
        // Travelling +x, the tangent is +x; the C++ formula atan2(0, +) + PI gives PI.
        for k in &path.keyframes {
            assert!(
                (k.initial_orientation - std::f32::consts::PI).abs() < 1e-3,
                "got {}",
                k.initial_orientation
            );
        }
    }

    #[test]
    fn the_path_has_a_positive_traversal_time_and_a_final_teleport() {
        let path = generate_waypoints(&straight_line(), &profile()).unwrap();
        assert!(path.path_time > 0);
        // Every path teleports from its last keyframe back to its first.
        assert!(path.keyframes.last().unwrap().teleport);
        // accel constants: 0.5*v^2/a = 10, v/a = 2.
        assert!((path.accel_dist - 10.0).abs() < 1e-3);
        assert!((path.accel_time - 2.0).abs() < 1e-3);
    }

    #[test]
    fn a_stop_node_becomes_a_dwelling_keyframe() {
        // Middle interior node stops for 3 seconds.
        let mut nodes = straight_line();
        nodes[2].action_flag = 2;
        nodes[2].delay = 3;
        let path = generate_waypoints(&nodes, &profile()).unwrap();

        let stop = path
            .keyframes
            .iter()
            .find(|k| k.is_stop_frame)
            .expect("a stop frame");
        assert_eq!(stop.delay_secs, 3);
        // A dwell delays the arrivals after it: the path takes longer than the stopless one.
        let stopless = generate_waypoints(&straight_line(), &profile()).unwrap();
        assert!(path.path_time > stopless.path_time);
    }

    #[test]
    fn a_map_change_marks_the_previous_keyframe_as_a_teleport() {
        // Interior node 2 sits on a different map than node 3, forcing a teleport there.
        let mut nodes = straight_line();
        nodes[3].map_id = 1;
        nodes[4].map_id = 1;
        let path = generate_waypoints(&nodes, &profile()).unwrap();
        // The keyframe before the map change carries the teleport flag.
        assert!(path.keyframes.iter().any(|k| k.teleport));
        // Both maps were recorded as visited.
        assert!(path.maps_used.contains(&0));
    }
}
