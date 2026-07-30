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

    /// Add a keyframe, keeping the path sorted by time and `total_time` at the latest.
    ///
    /// The path is keyed by time, so a keyframe at a time already present
    /// replaces the existing one rather than being inserted alongside it.
    pub fn add_frame(&mut self, entry: TransportAnimationEntry) {
        if entry.time_seg > self.total_time {
            self.total_time = entry.time_seg;
        }
        match self
            .path
            .binary_search_by(|frame| frame.time_seg.cmp(&entry.time_seg))
        {
            Ok(existing) => self.path[existing] = entry,
            Err(insert_at) => self.path.insert(insert_at, entry),
        }
    }

    pub fn frames(&self) -> &[TransportAnimationEntry] {
        &self.path
    }

    /// Index of the first keyframe at or after `time`.
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

/// The loaded animation paths of every transport, keyed by transport entry
#[derive(Debug, Default, Clone)]
pub struct TransportAnimationManager {
    animations: std::collections::HashMap<u32, TransportAnimation>,
}

impl TransportAnimationManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one keyframe of a transport's animation, creating the transport's path on
    /// first sight (`TransportMgr::AddPathNodeToTransport`).
    ///
    /// The keyframe's own `time_seg` is its key, so re-adding a node at a time already
    /// present replaces it, and `total_time` tracks the latest keyframe added.
    pub fn add_path_node(&mut self, transport_entry: u32, node: TransportAnimationEntry) {
        self.animations
            .entry(transport_entry)
            .or_default()
            .add_frame(node);
    }

    /// Load all animation rows from the transport-animation store
    /// (`TransportMgr::LoadTransportAnimationAndRotation`).
    ///
    /// Database/DBC decoding remains outside this state-only manager; callers supply the
    /// validated `(transport entry, animation row)` pairs from their chosen data source.
    pub fn load_path_nodes(
        &mut self,
        nodes: impl IntoIterator<Item = (u32, TransportAnimationEntry)>,
    ) {
        for (transport_entry, node) in nodes {
            self.add_path_node(transport_entry, node);
        }
    }

    /// The animation path loaded for `transport_entry`, if any (`GetTransportAnimInfo`).
    pub fn animation(&self, transport_entry: u32) -> Option<&TransportAnimation> {
        self.animations.get(&transport_entry)
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
        anim
    }

    #[test]
    fn add_frame_tracks_the_latest_time_and_replaces_duplicates() {
        let mut anim = animation();
        // The latest keyframe added set total_time.
        assert_eq!(anim.total_time, 2000);

        // Re-adding at an existing time overwrites rather than duplicating.
        anim.add_frame(frame(1000, 99.0));
        assert_eq!(anim.frames().len(), 3);
        assert_eq!(anim.next_anim_node(1000).unwrap().x, 99.0);
    }

    #[test]
    fn manager_keys_animations_by_transport_entry() {
        let mut mgr = TransportAnimationManager::new();
        mgr.load_path_nodes([
            (176231, frame(0, 0.0)),
            (176231, frame(1000, 10.0)),
            (164871, frame(0, 5.0)),
        ]);

        // Each transport keeps its own path and total_time.
        let boat = mgr.animation(176231).unwrap();
        assert_eq!(boat.total_time, 1000);
        assert_eq!(boat.frames().len(), 2);
        assert_eq!(mgr.animation(164871).unwrap().frames().len(), 1);
        // An unknown transport has no animation loaded.
        assert!(mgr.animation(1).is_none());
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
