//! Taxi DBC checks against the extracted `data/dbc` files.
//!
//! Skipped when the data directory is absent.

use oxcore_dbc::structures::{ChrRacesEntry, TaxiPathEntry, TaxiPathNodeEntry};
use oxcore_dbc::{load_dbc_store, DbcManager, DbcStore};
use std::collections::HashMap;

fn dbc_dir() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("data/dbc");
    dir.join("TaxiPath.dbc").is_file().then_some(dir)
}

#[test]
fn taxi_paths_and_their_points_load() {
    let Some(dir) = dbc_dir() else {
        eprintln!("skipping: no extracted data/dbc");
        return;
    };
    let mut paths: DbcStore<TaxiPathEntry> = DbcStore::new("niii");
    load_dbc_store(&mut paths, dir.join("TaxiPath.dbc").to_str().unwrap()).unwrap();
    let mut points: DbcStore<TaxiPathNodeEntry> = DbcStore::new("iiiifffii");
    load_dbc_store(&mut points, dir.join("TaxiPathNode.dbc").to_str().unwrap()).unwrap();

    assert!(paths.len() > 100, "vanilla has hundreds of flight paths");
    let mut by_path: HashMap<u32, Vec<&TaxiPathNodeEntry>> = HashMap::new();
    for (_, point) in points.entries() {
        by_path.entry(point.path).or_default().push(point);
    }
    // Every flight path has points, numbered from 0 without gaps, on a real map.
    for (_, path) in paths.entries() {
        let Some(list) = by_path.get_mut(&path.id) else {
            continue;
        };
        list.sort_by_key(|point| point.index);
        assert_eq!(list[0].index, 0, "path {} starts at point 0", path.id);
        assert_eq!(
            list.last().unwrap().index as usize,
            list.len() - 1,
            "path {} has no gaps",
            path.id
        );
        assert!(list.iter().all(|point| point.map_id < 1000));
        assert!(list.iter().all(|point| point.x.is_finite() && point.x.abs() < 20000.0));
    }
    let with_points = paths
        .entries()
        .filter(|(_, path)| by_path.contains_key(&path.id))
        .count();
    assert!(with_points * 10 > paths.len() * 9, "most paths have points");
}

#[test]
fn races_start_knowing_their_capital_flight_point() {
    let Some(dir) = dbc_dir() else {
        eprintln!("skipping: no extracted data/dbc");
        return;
    };
    let format = DbcManager::new().chr_races.format().to_string();
    let mut races: DbcStore<ChrRacesEntry> = DbcStore::new(&format);
    load_dbc_store(&mut races, dir.join("ChrRaces.dbc").to_str().unwrap()).unwrap();
    // Node 2 is Stormwind (bit 1), node 23 Orgrimmar (bit 22).
    let human = races.lookup(1).unwrap().starting_taxi_mask;
    let orc = races.lookup(2).unwrap().starting_taxi_mask;
    assert_ne!(human & (1 << 1), 0, "human mask {human:#x}");
    assert_ne!(orc & (1 << 22), 0, "orc mask {orc:#x}");
}
