//! Navmesh checks against the extracted `data/mmaps` files.
//!
//! Skipped when the data is absent, so the suite still runs on a checkout
//! without extracted client data.

use oxcore_map::pathfinding::navmesh::{
    nav_terrain, parse_detour_tile, NavMesh, NavMeshParams, NavPath,
};
use oxcore_shared::protocol::Position;

fn mmaps_dir() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("data")
        .join("mmaps");
    dir.join("0004832.mmtile").is_file().then_some(dir)
}

fn le_i32(data: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

fn le_f32(data: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

fn load(dir: &std::path::Path) -> (NavMesh, oxcore_map::pathfinding::navmesh::NavTile) {
    let header = std::fs::read(dir.join("000.mmap")).unwrap();
    let params = NavMeshParams {
        origin: [le_f32(&header, 0), le_f32(&header, 4), le_f32(&header, 8)],
        tile_width: le_f32(&header, 12),
        tile_height: le_f32(&header, 16),
    };
    let file = std::fs::read(dir.join("0004832.mmtile")).unwrap();
    // 20-byte mmtile header: magic, Detour version, mmap version, size, liquids.
    let tile = parse_detour_tile(&file[20..]).expect("real tile parses");
    let mut mesh = NavMesh::with_params(params);
    mesh.add_tile(tile.clone());
    (mesh, tile)
}

#[test]
fn real_tile_counts_match_its_header() {
    let Some(dir) = mmaps_dir() else {
        eprintln!("skipping: no extracted data/mmaps");
        return;
    };
    let file = std::fs::read(dir.join("0004832.mmtile")).unwrap();
    let raw = &file[20..];
    let (_, tile) = load(&dir);
    assert_eq!(tile.polygon_count(), le_i32(raw, 24) as usize);
    assert_eq!(tile.vertex_count(), le_i32(raw, 28) as usize);
}

#[test]
fn paths_inside_a_real_tile_stay_on_the_mesh() {
    let Some(dir) = mmaps_dir() else {
        eprintln!("skipping: no extracted data/mmaps");
        return;
    };
    let (mesh, tile) = load(&dir);
    let centre = |poly: usize| -> Option<Position> {
        let (flags, ground) = tile.polygon_flags(poly)?;
        if !ground || flags & nav_terrain::GROUND == 0 {
            return None;
        }
        let verts = tile.polygon_world_vertices(poly);
        let n = verts.len() as f32;
        Some(Position::new(
            verts.iter().map(|v| v.x).sum::<f32>() / n,
            verts.iter().map(|v| v.y).sum::<f32>() / n,
            verts.iter().map(|v| v.z).sum::<f32>() / n + 0.5,
            0.0,
        ))
    };
    let walkable: Vec<Position> = (0..tile.polygon_count()).filter_map(centre).collect();
    assert!(walkable.len() > 100, "tile has ground polygons");

    let mut complete = 0;
    for pair in walkable.windows(2).step_by(walkable.len() / 40) {
        let (from, to) = (pair[0], *walkable.last().unwrap());
        let _ = pair;
        match mesh.find_path(from, to, nav_terrain::WALK_AND_SWIM) {
            NavPath::Path { points, complete: done } => {
                if done {
                    complete += 1;
                }
                assert!(points.len() >= 2);
                for p in &points {
                    assert!(
                        mesh.find_walk_poly(*p, nav_terrain::WALK_AND_SWIM).is_some(),
                        "path point {p:?} is off the mesh"
                    );
                }
            }
            NavPath::NoPolygon => {}
            NavPath::NoTiles => panic!("tile is loaded"),
        }
    }
    assert!(complete > 0, "at least one complete path inside the tile");
}
