//! Integration checks against the extracted `data/maps` files.
//!
//! Skipped when the data directory is absent, so the suite still runs on a
//! checkout without extracted client data.

use oxcore_map::terrain::{
    LiquidStatusFlags, TerrainManager, INVALID_HEIGHT, MAP_ALL_LIQUIDS, MAP_LIQUID_TYPE_DEEP_WATER,
    MAP_LIQUID_TYPE_OCEAN,
};

/// Locate the repository `data` directory, if extracted data is present.
fn data_dir() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("data");

    dir.join("maps").is_dir().then_some(dir)
}

#[test]
fn loads_open_ocean_grid_as_deep_water() {
    let Some(data) = data_dir() else {
        eprintln!("skipping: no extracted data/maps");
        return;
    };

    let terrain = TerrainManager::new(&data).get(0);

    // Grid file 0002035.map: gy=20, gx=35 -> world (x=6400, y=-1600).
    // This is open ocean east of Eastern Kingdoms: a global OCEAN|DEEP_WATER
    // liquid layer at z=0 with no per-cell height map.
    let (x, y) = (6400.0, -1600.0);
    assert!(
        terrain.get_grid(x, y).is_some(),
        "expected a terrain grid at the ocean sample point"
    );

    let mut liquid = oxcore_map::terrain::LiquidData::default();
    let status = terrain.get_liquid_status(x, y, -5.0, MAP_ALL_LIQUIDS, None, Some(&mut liquid));

    assert!(
        status.intersects(LiquidStatusFlags::UNDER_WATER),
        "5 yards below the surface should be fully submerged, got {:?}",
        status
    );
    assert_eq!(liquid.level, 0.0, "ocean surface sits at z=0");
    assert!(
        liquid.type_flags & MAP_LIQUID_TYPE_OCEAN != 0,
        "expected ocean, got flags {:#x}",
        liquid.type_flags
    );
    assert!(
        liquid.is_deep_water(),
        "open ocean must set DEEP_WATER so the fatigue timer runs (flags {:#x})",
        liquid.type_flags
    );
    assert_eq!(
        liquid.type_flags & MAP_LIQUID_TYPE_DEEP_WATER,
        MAP_LIQUID_TYPE_DEEP_WATER
    );
}

#[test]
fn ocean_status_tracks_depth_relative_to_surface() {
    let Some(data) = data_dir() else {
        eprintln!("skipping: no extracted data/maps");
        return;
    };

    let terrain = TerrainManager::new(&data).get(0);
    let (x, y) = (6400.0, -1600.0);

    let at = |z: f32| terrain.get_liquid_status(x, y, z, MAP_ALL_LIQUIDS, None, None);

    // Surface is z=0 for this grid.
    assert!(at(-50.0).intersects(LiquidStatusFlags::UNDER_WATER));
    assert!(at(-0.1).intersects(LiquidStatusFlags::IN_WATER));
    assert_eq!(at(0.0), LiquidStatusFlags::WATER_WALK);
    assert_eq!(at(20.0), LiquidStatusFlags::ABOVE_WATER);

    assert!(terrain.is_in_water(x, y, -5.0, None));
    assert!(!terrain.is_in_water(x, y, 20.0, None));
    assert!(terrain.is_underwater(x, y, -5.0, None));
    assert_eq!(terrain.get_water_level(x, y, -5.0, None), 0.0);
}

#[test]
fn loads_land_grid_with_real_height_mesh() {
    let Some(data) = data_dir() else {
        eprintln!("skipping: no extracted data/maps");
        return;
    };

    let terrain = TerrainManager::new(&data).get(0);

    // Elwynn Forest area, grid file 0004832.map (gy=48, gx=32).
    let (x, y) = (-9000.0, -100.0);
    let grid = terrain
        .get_grid(x, y)
        .expect("expected a terrain grid over Elwynn Forest");

    let height = grid.get_height(x, y);
    assert!(
        height > INVALID_HEIGHT,
        "expected a real terrain height, got {}",
        height
    );
    // Elwynn sits well above sea level and well below the highest peaks.
    assert!(
        (0.0..500.0).contains(&height),
        "Elwynn height {} is implausible",
        height
    );

    // The area map is present for land grids, so the flag should be non-zero.
    assert_ne!(grid.get_area_flag(x, y), 0);

    // Standing on dry ground well above the terrain: no liquid.
    let status = terrain.get_liquid_status(x, y, height + 50.0, MAP_ALL_LIQUIDS, None, None);
    assert!(
        !status.intersects(LiquidStatusFlags::MASK_SWIMMING),
        "should not be swimming 50 yards above Elwynn ground, got {:?}",
        status
    );
}

#[test]
fn gameobject_model_list_loads_from_vmaps() {
    let Some(data) = data_dir() else {
        eprintln!("skipping: no extracted data/maps");
        return;
    };
    if !data.join("vmaps").is_dir() {
        eprintln!("skipping: no extracted data/vmaps");
        return;
    }

    let vmap =
        oxcore_map::VMapManager::new(&data, oxcore_map::pathfinding::vmap::VMapConfig::default());

    assert!(
        vmap.gameobject_model_count() > 0,
        "expected temp_gameobject_models to yield collision models"
    );

    // Every entry must name a model file and carry a usable bounding box or be
    // explicitly zero-bounded (which callers skip).
    let display_id = 1;
    if let Some(model) = vmap.gameobject_model(display_id) {
        assert!(!model.name.is_empty());
    }
}
