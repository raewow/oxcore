//! The store of built transport paths, keyed by game-object entry (`TransportMgr`'s
//! `m_transportTemplates` and the loading around it).
//!
//! Each MO_TRANSPORT game object has one template: the keyframe path built by
//! [`generate_waypoints`](super::waypoints::generate_waypoints) plus the derived motion
//! constants. This module owns those templates and the pure logic that loads, looks up and
//! period-overrides them. Reading the game-object info map to find the transports, pulling
//! their taxi path and speed out of `moTransport`, and the `WorldDatabase` period query are
//! the caller's data sources - they are injected here, the same way the taxi nodes are.

use std::collections::{BTreeSet, HashMap};

use super::schedule::KeyFrame;
use super::waypoints::{generate_waypoints, TaxiPathNode, TransportPath};
use super::schedule::ScheduleProfile;

/// One transport's built path and motion constants (`TransportTemplate`).
#[derive(Debug, Clone)]
pub struct TransportTemplate {
    pub entry: u32,
    pub keyframes: Vec<KeyFrame>,
    pub accel_time: f32,
    pub accel_dist: f32,
    /// Total traversal time in milliseconds, possibly overridden from the DB.
    pub path_time: u32,
    pub maps_used: BTreeSet<u32>,
    /// Whether this transport runs on instanced maps (needs map data; caller may refine).
    pub in_instance: bool,
    /// Whether a continent instance of this transport has already been spawned.
    pub spawned: bool,
}

impl TransportTemplate {
    fn from_path(entry: u32, path: TransportPath) -> Self {
        Self {
            entry,
            keyframes: path.keyframes,
            accel_time: path.accel_time,
            accel_dist: path.accel_dist,
            path_time: path.path_time,
            maps_used: path.maps_used,
            in_instance: false,
            spawned: false,
        }
    }

    /// The keyframe the transport should start at on `map_id`: the first whose node is on that
    /// map (the frame search at the top of `TransportMgr::CreateTransport`).
    pub fn start_frame_on_map(&self, map_id: u32) -> Option<&KeyFrame> {
        self.keyframes.iter().find(|k| k.map_id == map_id)
    }

    /// Apply a database period override (`LoadTransportTemplates`' second pass).
    ///
    /// A non-zero period replaces the computed traversal time and pins the last keyframe's
    /// departure to it, since the generated timing is not exact. A zero period is ignored.
    pub fn override_period(&mut self, period: u32) {
        if period == 0 {
            return;
        }
        self.path_time = period;
        if let Some(last) = self.keyframes.last_mut() {
            last.departure_time = period;
        }
    }
}

/// Owns every transport's built template (`TransportMgr::m_transportTemplates`).
#[derive(Debug, Default, Clone)]
pub struct TransportTemplateStore {
    templates: HashMap<u32, TransportTemplate>,
}

impl TransportTemplateStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up a transport's template (`TransportMgr::GetTransportTemplate`).
    pub fn get(&self, entry: u32) -> Option<&TransportTemplate> {
        self.templates.get(&entry)
    }

    pub fn get_mut(&mut self, entry: u32) -> Option<&mut TransportTemplate> {
        self.templates.get_mut(&entry)
    }

    pub fn count(&self) -> usize {
        self.templates.len()
    }

    /// Build and store one transport's template from its taxi path (the body of the
    /// `LoadTransportTemplates` loop).
    ///
    /// Returns whether a template was produced; a path that fails to generate is skipped,
    /// mirroring the C++ `m_transportTemplates.erase(entry)`.
    pub fn load_template(&mut self, entry: u32, nodes: &[TaxiPathNode], profile: &ScheduleProfile) -> bool {
        match generate_waypoints(nodes, profile) {
            Some(path) => {
                self.templates.insert(entry, TransportTemplate::from_path(entry, path));
                true
            }
            None => false,
        }
    }

    /// Load every supplied transport (`TransportMgr::LoadTransportTemplates`' first pass).
    ///
    /// The caller supplies the MO_TRANSPORT game objects it found - each with the taxi nodes
    /// and speed/accel profile read out of `moTransport` - and this generates and stores each,
    /// dropping any whose path fails to build. Returns how many templates were stored.
    pub fn load_templates<'a, I>(&mut self, transports: I) -> usize
    where
        I: IntoIterator<Item = (u32, &'a [TaxiPathNode], ScheduleProfile)>,
    {
        let mut loaded = 0;
        for (entry, nodes, profile) in transports {
            if self.load_template(entry, nodes, &profile) {
                loaded += 1;
            }
        }
        loaded
    }

    /// The templates whose path visits `map_id`, for spawning transports onto a map
    /// (the `mapsUsed` filter in `TransportMgr::SpawnTransportsOnMap`).
    pub fn templates_on_map(&self, map_id: u32) -> impl Iterator<Item = &TransportTemplate> {
        self.templates
            .values()
            .filter(move |t| t.maps_used.contains(&map_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ScheduleProfile {
        ScheduleProfile { speed: 10.0, accel: 5.0 }
    }

    fn node(map_id: u32, x: f32) -> TaxiPathNode {
        TaxiPathNode { map_id, x, y: 0.0, z: 0.0, action_flag: 0, delay: 0 }
    }

    /// A five-node straight path on the given map.
    fn path_on_map(map_id: u32) -> Vec<TaxiPathNode> {
        (0..5).map(|i| node(map_id, i as f32 * 10.0)).collect()
    }

    #[test]
    fn a_loaded_template_is_retrievable_by_entry() {
        let mut store = TransportTemplateStore::new();
        assert!(store.load_template(176231, &path_on_map(0), &profile()));

        let template = store.get(176231).expect("template stored");
        assert_eq!(template.entry, 176231);
        assert!(!template.keyframes.is_empty());
        assert!(template.maps_used.contains(&0));
        // An unknown entry has no template.
        assert!(store.get(1).is_none());
    }

    #[test]
    fn a_path_that_fails_to_build_is_not_stored() {
        let mut store = TransportTemplateStore::new();
        // Two nodes are too few to form an interior keyframe.
        assert!(!store.load_template(999, &[node(0, 0.0), node(0, 10.0)], &profile()));
        assert!(store.get(999).is_none());
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn loading_many_counts_only_the_ones_that_built() {
        let good = path_on_map(0);
        let bad = vec![node(0, 0.0), node(0, 10.0)];
        let mut store = TransportTemplateStore::new();

        let loaded = store.load_templates([
            (1u32, good.as_slice(), profile()),
            (2u32, bad.as_slice(), profile()),
        ]);
        assert_eq!(loaded, 1);
        assert!(store.get(1).is_some());
        assert!(store.get(2).is_none());
    }

    #[test]
    fn a_period_override_replaces_the_computed_time() {
        let mut store = TransportTemplateStore::new();
        store.load_template(176231, &path_on_map(0), &profile());
        let computed = store.get(176231).unwrap().path_time;

        store.get_mut(176231).unwrap().override_period(120_000);
        let template = store.get(176231).unwrap();
        assert_ne!(computed, 120_000);
        assert_eq!(template.path_time, 120_000);
        assert_eq!(template.keyframes.last().unwrap().departure_time, 120_000);

        // A zero override leaves the timing alone.
        store.get_mut(176231).unwrap().override_period(0);
        assert_eq!(store.get(176231).unwrap().path_time, 120_000);
    }

    #[test]
    fn start_frame_is_the_first_keyframe_on_the_map() {
        let mut store = TransportTemplateStore::new();
        store.load_template(176231, &path_on_map(0), &profile());
        let template = store.get(176231).unwrap();

        assert!(template.start_frame_on_map(0).is_some());
        // No keyframe is on a map the path never visits.
        assert!(template.start_frame_on_map(571).is_none());
    }

    #[test]
    fn only_templates_visiting_a_map_are_offered_for_spawn() {
        let mut store = TransportTemplateStore::new();
        store.load_template(1, &path_on_map(0), &profile());
        store.load_template(2, &path_on_map(1), &profile());

        assert_eq!(store.templates_on_map(0).count(), 1);
        assert_eq!(store.templates_on_map(1).count(), 1);
        assert_eq!(store.templates_on_map(999).count(), 0);
    }
}
