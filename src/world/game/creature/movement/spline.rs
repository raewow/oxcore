//! MoveSpline - smooth path interpolation for creature movement
//!
//! Interpolates creature position along a sequence of waypoints over time.
//! Uses per-segment timing proportional to segment length (matching MaNGOS)
//! so creatures move at constant speed regardless of waypoint spacing.

use crate::shared::protocol::Position;

/// Movement spline for smooth path interpolation
#[derive(Debug, Clone)]
pub struct MoveSpline {
    /// Waypoints in the spline (including start position)
    waypoints: Vec<Position>,
    /// Cumulative timestamp at each waypoint (ms). timestamps[0] = 0.
    segment_timestamps: Vec<u32>,
    /// Total spline duration (ms)
    total_duration: u32,
    /// Current time in spline (ms)
    current_time: u32,
    /// Spline ID for packets
    spline_id: u32,
    /// Is spline active
    active: bool,
}

impl MoveSpline {
    /// Create a new spline with velocity-based per-segment timing.
    /// `waypoints` must include the start position as the first element.
    /// Duration for each segment is proportional to its length.
    pub fn new(waypoints: Vec<Position>, velocity: f32) -> Self {
        if waypoints.len() < 2 || velocity <= 0.0 {
            return Self {
                waypoints,
                segment_timestamps: vec![0],
                total_duration: 0,
                current_time: 0,
                spline_id: rand::random(),
                active: false,
            };
        }

        let velocity_inv = 1000.0 / velocity;
        let mut timestamps = Vec::with_capacity(waypoints.len());
        let mut time: u32 = 1; // MaNGOS minimal_duration = 1

        timestamps.push(0);
        for i in 1..waypoints.len() {
            let dx = waypoints[i].x - waypoints[i - 1].x;
            let dy = waypoints[i].y - waypoints[i - 1].y;
            let dz = waypoints[i].z - waypoints[i - 1].z;
            let seg_len = (dx * dx + dy * dy + dz * dz).sqrt();
            time += (seg_len * velocity_inv) as u32;
            timestamps.push(time);
        }

        let total_duration = time;

        Self {
            waypoints,
            segment_timestamps: timestamps,
            total_duration,
            current_time: 0,
            spline_id: rand::random(),
            active: true,
        }
    }

    /// Update spline progress. Returns true if still active.
    pub fn update(&mut self, diff_ms: u32) -> bool {
        if !self.active {
            return false;
        }

        self.current_time += diff_ms;

        if self.current_time >= self.total_duration {
            self.active = false;
            return false;
        }

        true
    }

    /// Get current interpolated position using per-segment timestamps
    pub fn get_position(&self) -> Position {
        if self.waypoints.is_empty() {
            return Position::default();
        }

        if self.waypoints.len() == 1 || !self.active {
            return *self.waypoints.last().unwrap();
        }

        // Find current segment by scanning timestamps
        let mut seg_idx = 0;
        for i in 1..self.segment_timestamps.len() {
            if self.current_time < self.segment_timestamps[i] {
                seg_idx = i - 1;
                break;
            }
            seg_idx = i - 1;
        }

        // Clamp to valid range
        let seg_idx = seg_idx.min(self.waypoints.len() - 2);

        let seg_start_time = self.segment_timestamps[seg_idx];
        let seg_end_time = self.segment_timestamps[seg_idx + 1];
        let seg_duration = seg_end_time - seg_start_time;

        let u = if seg_duration > 0 {
            (self.current_time - seg_start_time) as f32 / seg_duration as f32
        } else {
            1.0
        };

        let start = &self.waypoints[seg_idx];
        let end = &self.waypoints[seg_idx + 1];

        Position {
            x: start.x + (end.x - start.x) * u,
            y: start.y + (end.y - start.y) * u,
            z: start.z + (end.z - start.z) * u,
            o: (end.y - start.y).atan2(end.x - start.x),
        }
    }

    /// Check if spline is finished
    pub fn is_finished(&self) -> bool {
        !self.active
    }

    /// Check if spline is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get final position
    pub fn final_position(&self) -> Position {
        self.waypoints.last().copied().unwrap_or_default()
    }

    /// Get spline ID
    pub fn id(&self) -> u32 {
        self.spline_id
    }

    /// Get remaining time
    pub fn time_remaining(&self) -> u32 {
        self.total_duration.saturating_sub(self.current_time)
    }

    /// Get total duration
    pub fn total_duration(&self) -> u32 {
        self.total_duration
    }

    /// Get waypoints
    pub fn waypoints(&self) -> &[Position] {
        &self.waypoints
    }

    /// Stop the spline early
    pub fn stop(&mut self) {
        self.active = false;
    }
}

impl Default for MoveSpline {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            segment_timestamps: Vec::new(),
            total_duration: 0,
            current_time: 0,
            spline_id: 0,
            active: false,
        }
    }
}
