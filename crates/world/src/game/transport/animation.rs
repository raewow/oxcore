//! Transport keyframe animation lookup.
//!
//! A transport's canned motion is a table of `(time, position)` keyframes. Interpolating
//! it needs the frames bracketing a given time; those two lookups are ported here. Loading
//! the DBC and driving the interpolation belong to the unported transport system.

/// One keyframe of a transport's path (`TransportAnimationEntry`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportAnimationEntry {
    pub time_seg: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// A transport's keyframe path, kept sorted by time.
#[derive(Debug, Default, Clone)]
pub struct TransportAnimation {
    /// Keyframes in ascending time order.
    path: Vec<TransportAnimationEntry>,
    pub total_time: u32,
}

impl TransportAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a keyframe, keeping the path sorted by time.
    pub fn add_frame(&mut self, entry: TransportAnimationEntry) {
        let insert_at = self
            .path
            .partition_point(|frame| frame.time_seg < entry.time_seg);
        self.path.insert(insert_at, entry);
    }

    pub fn frames(&self) -> &[TransportAnimationEntry] {
        &self.path
    }

    /// Index of the first keyframe at or after `time` (the C++ `lower_bound`).
    fn lower_bound(&self, time: u32) -> usize {
        self.path.partition_point(|frame| frame.time_seg < time)
    }

    /// Keyframe strictly before `time` (`GetPrevAnimNode`).
    ///
    /// `None` when `time` is at or before the first keyframe: there is nothing earlier to
    /// interpolate from.
    pub fn prev_anim_node(&self, time: u32) -> Option<TransportAnimationEntry> {
        let index = self.lower_bound(time);
        if index == 0 {
            None
        } else {
            Some(self.path[index - 1])
        }
    }

    /// First keyframe at or after `time` (`GetNextAnimNode`).
    ///
    /// `None` when `time` is past the last keyframe.
    pub fn next_anim_node(&self, time: u32) -> Option<TransportAnimationEntry> {
        self.path.get(self.lower_bound(time)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(time_seg: u32, x: f32) -> TransportAnimationEntry {
        TransportAnimationEntry {
            time_seg,
            x,
            y: 0.0,
            z: 0.0,
        }
    }

    fn animation() -> TransportAnimation {
        let mut anim = TransportAnimation::new();
        // Added out of order to exercise the sorted insert.
        anim.add_frame(frame(2000, 20.0));
        anim.add_frame(frame(0, 0.0));
        anim.add_frame(frame(1000, 10.0));
        anim.total_time = 2000;
        anim
    }

    #[test]
    fn frames_are_kept_in_time_order() {
        let anim = animation();
        let times: Vec<u32> = anim.frames().iter().map(|f| f.time_seg).collect();
        assert_eq!(times, vec![0, 1000, 2000]);
    }

    #[test]
    fn next_node_returns_the_frame_at_or_after_the_time() {
        let anim = animation();

        // Exactly on a keyframe returns that keyframe.
        assert_eq!(anim.next_anim_node(1000).unwrap().x, 10.0);
        // Between keyframes returns the later one.
        assert_eq!(anim.next_anim_node(1500).unwrap().x, 20.0);
        // Past the end there is nothing.
        assert!(anim.next_anim_node(2001).is_none());
    }

    #[test]
    fn prev_node_returns_the_frame_strictly_before_the_time() {
        let anim = animation();

        // On the second keyframe, the previous is the first.
        assert_eq!(anim.prev_anim_node(1000).unwrap().x, 0.0);
        // Between keyframes, the previous is the earlier one.
        assert_eq!(anim.prev_anim_node(1500).unwrap().x, 10.0);
        // At or before the first keyframe there is nothing earlier.
        assert!(anim.prev_anim_node(0).is_none());
    }

    #[test]
    fn bracketing_frames_straddle_the_query_time() {
        let anim = animation();

        // The two frames the interpolator would blend between at t=1500.
        let prev = anim.prev_anim_node(1500).unwrap();
        let next = anim.next_anim_node(1500).unwrap();
        assert!(prev.time_seg < 1500 && next.time_seg >= 1500);
        assert_eq!((prev.x, next.x), (10.0, 20.0));
    }
}
