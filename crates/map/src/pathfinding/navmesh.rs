//! Detour navigation meshes: tile parsing, cross-tile links and path queries.
//!
//! Tiles are kept separately, exactly as extracted, and addressed by
//! `(tile x, tile y, polygon)`. Polygons inside a tile are connected through
//! their own neighbour table; polygons on a tile border are connected to the
//! adjacent tile when both tiles are loaded, by matching collinear border
//! edges and keeping the overlapping part as the portal. Paths are found with
//! A* over polygons and then pulled tight through the portals (string pulling),
//! so the result follows the walkable surface and never cuts across holes,
//! cliffs or unwalkable polygons.
//!
//! Detour stores vertices in Recast space, `(world y, world z, world x)`; all
//! public functions take and return world positions.

use oxcore_shared::protocol::Position;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// `'DNAV'` as Detour writes it.
pub const DT_NAVMESH_MAGIC: i32 =
    ((b'D' as i32) << 24) | ((b'N' as i32) << 16) | ((b'A' as i32) << 8) | (b'V' as i32);
pub const DT_NAVMESH_VERSION: i32 = 7;

const VERTS_PER_POLY: usize = 6;
const DT_EXT_LINK: u16 = 0x8000;
const POLYTYPE_GROUND: u8 = 0;

// Serialized sizes (64-bit polygon references, 4-byte alignment).
const HEADER_SIZE: usize = 100;
const POLY_SIZE: usize = 32;
const LINK_SIZE: usize = 16;
const DETAIL_MESH_SIZE: usize = 12;
const BV_NODE_SIZE: usize = 16;
const OFFMESH_CON_SIZE: usize = 36;

/// Most polygons a single path search will expand.
const MAX_SEARCH_NODES: usize = 4096;
/// Longest polygon corridor and point path returned.
const MAX_PATH_LENGTH: usize = 256;

/// Terrain classes stored in each polygon's flags.
pub mod nav_terrain {
    pub const GROUND: u16 = 0x01;
    pub const MAGMA: u16 = 0x02;
    pub const SLIME: u16 = 0x04;
    pub const WATER: u16 = 0x08;
    pub const STEEP_SLOPES: u16 = 0x10;
    /// What a creature that can walk and swim may cross.
    pub const WALK_AND_SWIM: u16 = GROUND | MAGMA | SLIME | WATER;
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// Grid placement of a map's tiles (the `.mmap` header).
#[derive(Debug, Clone, Copy)]
pub struct NavMeshParams {
    /// Recast-space origin.
    pub origin: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
}

#[derive(Debug, Clone)]
pub struct TileHeader {
    pub x: i32,
    pub y: i32,
    pub layer: i32,
    pub poly_count: usize,
    pub vert_count: usize,
    pub walkable_height: f32,
    pub walkable_radius: f32,
    pub walkable_climb: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct NavPoly {
    pub verts: [u16; VERTS_PER_POLY],
    pub neis: [u16; VERTS_PER_POLY],
    pub flags: u16,
    pub vert_count: u8,
    pub area: u8,
    pub kind: u8,
}

#[derive(Debug, Clone, Copy)]
struct PolyDetail {
    vert_base: u32,
    tri_base: u32,
    vert_count: u8,
    tri_count: u8,
}

/// One parsed Detour tile.
#[derive(Debug, Clone)]
pub struct NavTile {
    pub header: TileHeader,
    verts: Vec<[f32; 3]>,
    polys: Vec<NavPoly>,
    detail_meshes: Vec<PolyDetail>,
    detail_verts: Vec<[f32; 3]>,
    detail_tris: Vec<[u8; 4]>,
    poly_bounds: Vec<([f32; 3], [f32; 3])>,
}

impl NavTile {
    /// Polygons in the tile, off-mesh connection polygons included.
    pub fn polygon_count(&self) -> usize {
        self.polys.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.verts.len()
    }

    /// A polygon's terrain flags (see [`nav_terrain`]), and whether it is a
    /// walkable ground polygon rather than an off-mesh connection.
    pub fn polygon_flags(&self, poly: usize) -> Option<(u16, bool)> {
        self.polys
            .get(poly)
            .map(|p| (p.flags, p.kind == POLYTYPE_GROUND))
    }

    /// A polygon's vertices in world space.
    pub fn polygon_world_vertices(&self, poly: usize) -> Vec<Position> {
        let Some(p) = self.polys.get(poly) else {
            return Vec::new();
        };
        (0..p.vert_count as usize)
            .map(|i| to_world(self.verts[p.verts[i] as usize]))
            .collect()
    }

    fn poly_vert(&self, poly: &NavPoly, i: usize) -> [f32; 3] {
        self.verts[poly.verts[i] as usize]
    }

    /// Height of the detail surface under `pos` (inside the polygon in 2D).
    fn poly_height(&self, poly_index: usize, pos: [f32; 3]) -> Option<f32> {
        let poly = &self.polys[poly_index];
        if let Some(detail) = self.detail_meshes.get(poly_index) {
            for t in 0..detail.tri_count as usize {
                let tri = self.detail_tris[detail.tri_base as usize + t];
                let v = |k: usize| -> [f32; 3] {
                    let idx = tri[k] as usize;
                    if idx < poly.vert_count as usize {
                        self.poly_vert(poly, idx)
                    } else {
                        self.detail_verts[detail.vert_base as usize + idx - poly.vert_count as usize]
                    }
                };
                if let Some(h) = closest_height_on_triangle(pos, v(0), v(1), v(2)) {
                    return Some(h);
                }
            }
        }
        // No detail mesh (or a point on a shared edge): use the polygon's fan.
        let a = self.poly_vert(poly, 0);
        for i in 1..poly.vert_count.saturating_sub(1) as usize {
            let b = self.poly_vert(poly, i);
            let c = self.poly_vert(poly, i + 1);
            if let Some(h) = closest_height_on_triangle(pos, a, b, c) {
                return Some(h);
            }
        }
        None
    }

    /// Closest point on a polygon to `pos`, and whether `pos` lies over it.
    fn closest_point_on_poly(&self, poly_index: usize, pos: [f32; 3]) -> ([f32; 3], bool) {
        let poly = &self.polys[poly_index];
        let nv = poly.vert_count as usize;
        let verts: Vec<[f32; 3]> = (0..nv).map(|i| self.poly_vert(poly, i)).collect();
        if point_in_polygon_2d(pos, &verts) {
            if let Some(h) = self.poly_height(poly_index, pos) {
                return ([pos[0], h, pos[2]], true);
            }
        }
        (closest_point_on_boundary(pos, &verts), false)
    }
}

/// Little-endian reader over a tile buffer; every read is bounds checked.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    fn bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let slice = self.data.get(self.pos..self.pos + N)?;
        self.pos += N;
        slice.try_into().ok()
    }

    fn i32(&mut self) -> Option<i32> {
        self.bytes::<4>().map(i32::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.bytes::<4>().map(u32::from_le_bytes)
    }

    fn u16(&mut self) -> Option<u16> {
        self.bytes::<2>().map(u16::from_le_bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        self.bytes::<1>().map(|b| b[0])
    }

    fn f32(&mut self) -> Option<f32> {
        self.bytes::<4>().map(f32::from_le_bytes)
    }

    fn vec3(&mut self) -> Option<[f32; 3]> {
        Some([self.f32()?, self.f32()?, self.f32()?])
    }
}

fn count(value: i32) -> Option<usize> {
    usize::try_from(value).ok()
}

/// Parse a Detour tile (the body of an `.mmtile`, after its 20-byte header).
///
/// Returns `None` for anything that is not a complete, internally consistent
/// tile of the supported format: wrong magic/version, truncated sections,
/// polygon vertices or neighbours out of range, or detail meshes pointing
/// outside their arrays.
pub fn parse_detour_tile(data: &[u8]) -> Option<NavTile> {
    let mut r = Reader::at(data, 0);
    if r.i32()? != DT_NAVMESH_MAGIC || r.i32()? != DT_NAVMESH_VERSION {
        return None;
    }
    let x = r.i32()?;
    let y = r.i32()?;
    let layer = r.i32()?;
    let _user_id = r.u32()?;
    let poly_count = count(r.i32()?)?;
    let vert_count = count(r.i32()?)?;
    let max_link_count = count(r.i32()?)?;
    let detail_mesh_count = count(r.i32()?)?;
    let detail_vert_count = count(r.i32()?)?;
    let detail_tri_count = count(r.i32()?)?;
    let bv_node_count = count(r.i32()?)?;
    let off_mesh_con_count = count(r.i32()?)?;
    let _off_mesh_base = r.i32()?;
    let walkable_height = r.f32()?;
    let walkable_radius = r.f32()?;
    let walkable_climb = r.f32()?;
    let bmin = r.vec3()?;
    let bmax = r.vec3()?;
    let _bv_quant_factor = r.f32()?;

    let verts_at = align4(HEADER_SIZE);
    let polys_at = verts_at + align4(vert_count * 12);
    let links_at = polys_at + align4(poly_count * POLY_SIZE);
    let detail_meshes_at = links_at + align4(max_link_count * LINK_SIZE);
    let detail_verts_at = detail_meshes_at + align4(detail_mesh_count * DETAIL_MESH_SIZE);
    let detail_tris_at = detail_verts_at + align4(detail_vert_count * 12);
    let bv_at = detail_tris_at + align4(detail_tri_count * 4);
    let off_mesh_at = bv_at + align4(bv_node_count * BV_NODE_SIZE);
    let end = off_mesh_at + align4(off_mesh_con_count * OFFMESH_CON_SIZE);
    if end > data.len() {
        return None;
    }

    let mut r = Reader::at(data, verts_at);
    let verts = (0..vert_count)
        .map(|_| r.vec3())
        .collect::<Option<Vec<_>>>()?;

    let mut r = Reader::at(data, polys_at);
    let mut polys = Vec::with_capacity(poly_count);
    for _ in 0..poly_count {
        let _first_link = r.u32()?;
        let mut pverts = [0u16; VERTS_PER_POLY];
        for v in &mut pverts {
            *v = r.u16()?;
        }
        let mut neis = [0u16; VERTS_PER_POLY];
        for n in &mut neis {
            *n = r.u16()?;
        }
        let flags = r.u16()?;
        let pvert_count = r.u8()?;
        let area_and_type = r.u8()?;
        let poly = NavPoly {
            verts: pverts,
            neis,
            flags,
            vert_count: pvert_count,
            area: area_and_type & 0x3f,
            kind: area_and_type >> 6,
        };
        let nv = poly.vert_count as usize;
        if nv > VERTS_PER_POLY || (poly.kind == POLYTYPE_GROUND && nv < 3) {
            return None;
        }
        if poly.verts[..nv].iter().any(|&v| v as usize >= vert_count) {
            return None;
        }
        for &nei in &poly.neis[..nv] {
            if nei != 0 && nei & DT_EXT_LINK == 0 && nei as usize > poly_count {
                return None;
            }
        }
        polys.push(poly);
    }

    let mut r = Reader::at(data, detail_meshes_at);
    let mut detail_meshes = Vec::with_capacity(detail_mesh_count);
    for _ in 0..detail_mesh_count {
        let mesh = PolyDetail {
            vert_base: r.u32()?,
            tri_base: r.u32()?,
            vert_count: r.u8()?,
            tri_count: r.u8()?,
        };
        let _pad = r.u16()?;
        if mesh.vert_base as usize + mesh.vert_count as usize > detail_vert_count
            || mesh.tri_base as usize + mesh.tri_count as usize > detail_tri_count
        {
            return None;
        }
        detail_meshes.push(mesh);
    }

    let mut r = Reader::at(data, detail_verts_at);
    let detail_verts = (0..detail_vert_count)
        .map(|_| r.vec3())
        .collect::<Option<Vec<_>>>()?;

    let mut r = Reader::at(data, detail_tris_at);
    let detail_tris = (0..detail_tri_count)
        .map(|_| r.bytes::<4>())
        .collect::<Option<Vec<_>>>()?;

    // Detail triangle indices must address either polygon or detail vertices.
    for (poly_index, mesh) in detail_meshes.iter().enumerate() {
        let Some(poly) = polys.get(poly_index) else {
            break;
        };
        let limit = poly.vert_count as usize + mesh.vert_count as usize;
        for t in 0..mesh.tri_count as usize {
            let tri = detail_tris[mesh.tri_base as usize + t];
            if tri[..3].iter().any(|&i| i as usize >= limit) {
                return None;
            }
        }
    }

    let mut tile = NavTile {
        header: TileHeader {
            x,
            y,
            layer,
            poly_count,
            vert_count,
            walkable_height,
            walkable_radius,
            walkable_climb,
            bmin,
            bmax,
        },
        verts,
        polys,
        detail_meshes,
        detail_verts,
        detail_tris,
        poly_bounds: Vec::new(),
    };
    tile.poly_bounds = (0..tile.polys.len()).map(|i| poly_bounds(&tile, i)).collect();
    Some(tile)
}

fn poly_bounds(tile: &NavTile, poly_index: usize) -> ([f32; 3], [f32; 3]) {
    let poly = &tile.polys[poly_index];
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    let mut grow = |v: [f32; 3]| {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    };
    for i in 0..poly.vert_count as usize {
        grow(tile.poly_vert(poly, i));
    }
    if let Some(detail) = tile.detail_meshes.get(poly_index) {
        for i in 0..detail.vert_count as usize {
            grow(tile.detail_verts[detail.vert_base as usize + i]);
        }
    }
    (lo, hi)
}

/// A polygon in a tiled navmesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PolyRef {
    pub tile_x: i32,
    pub tile_y: i32,
    pub poly: u16,
}

/// A border connection to a polygon in a neighbouring tile.
#[derive(Debug, Clone, Copy)]
struct ExtLink {
    to: PolyRef,
    /// Edge of the owning polygon that carries the portal.
    edge: u8,
    /// Portal limits along that edge, 0..1.
    tmin: f32,
    tmax: f32,
}

/// Outcome of a navmesh path query.
#[derive(Debug, Clone)]
pub enum NavPath {
    /// The start or end lies in a tile that is not loaded.
    NoTiles,
    /// No walkable polygon near the start or end (a hole in the mesh).
    NoPolygon,
    /// A path along the mesh. `complete` is false when it stops short.
    Path {
        points: Vec<Position>,
        complete: bool,
    },
}

/// A map's navmesh, assembled from its loaded tiles.
#[derive(Default)]
pub struct NavMesh {
    params: Option<NavMeshParams>,
    tiles: HashMap<(i32, i32), NavTile>,
    ext_links: HashMap<PolyRef, Vec<ExtLink>>,
}

impl NavMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_params(params: NavMeshParams) -> Self {
        Self {
            params: Some(params),
            ..Self::default()
        }
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Polygons across all loaded tiles.
    pub fn polygon_count(&self) -> usize {
        self.tiles.values().map(NavTile::polygon_count).sum()
    }

    /// Border links currently resolved between loaded tiles.
    pub fn external_link_count(&self) -> usize {
        self.ext_links.values().map(Vec::len).sum()
    }

    /// Add (or replace) a tile and connect it to its loaded neighbours.
    pub fn add_tile(&mut self, tile: NavTile) {
        let key = (tile.header.x, tile.header.y);
        self.remove_tile(key.0, key.1);
        self.tiles.insert(key, tile);
        for side in [0u8, 2, 4, 6] {
            let neighbour = neighbour_tile(key, side);
            if self.tiles.contains_key(&neighbour) {
                self.connect(key, neighbour, side);
                self.connect(neighbour, key, opposite_side(side));
            }
        }
    }

    /// Remove a tile and every link into or out of it.
    pub fn remove_tile(&mut self, x: i32, y: i32) -> bool {
        if self.tiles.remove(&(x, y)).is_none() {
            return false;
        }
        self.ext_links
            .retain(|from, _| (from.tile_x, from.tile_y) != (x, y));
        for links in self.ext_links.values_mut() {
            links.retain(|link| (link.to.tile_x, link.to.tile_y) != (x, y));
        }
        self.ext_links.retain(|_, links| !links.is_empty());
        true
    }

    /// Link every border edge of `from` facing `side` to the matching edges of `to`.
    fn connect(&mut self, from: (i32, i32), to: (i32, i32), side: u8) {
        let (Some(from_tile), Some(to_tile)) = (self.tiles.get(&from), self.tiles.get(&to)) else {
            return;
        };
        let opposite = DT_EXT_LINK | u16::from(opposite_side(side));
        let mut new_links: Vec<(PolyRef, ExtLink)> = Vec::new();

        for (pi, poly) in from_tile.polys.iter().enumerate() {
            let nv = poly.vert_count as usize;
            for j in 0..nv {
                let nei = poly.neis[j];
                if nei & DT_EXT_LINK == 0 || (nei & 0xff) as u8 != side {
                    continue;
                }
                let va = from_tile.poly_vert(poly, j);
                let vb = from_tile.poly_vert(poly, (j + 1) % nv);
                let (amin, amax) = slab_end_points(va, vb, side);
                let apos = slab_coord(va, side);

                for (qi, other) in to_tile.polys.iter().enumerate() {
                    let onv = other.vert_count as usize;
                    for k in 0..onv {
                        if other.neis[k] != opposite {
                            continue;
                        }
                        let vc = to_tile.poly_vert(other, k);
                        let vd = to_tile.poly_vert(other, (k + 1) % onv);
                        if (apos - slab_coord(vc, side)).abs() > 0.01 {
                            continue;
                        }
                        let (bmin, bmax) = slab_end_points(vc, vd, side);
                        if !overlap_slabs(amin, amax, bmin, bmax, 0.01, to_tile.header.walkable_climb)
                        {
                            continue;
                        }
                        let lo = amin[0].max(bmin[0]);
                        let hi = amax[0].min(bmax[0]);
                        let axis = if side == 0 || side == 4 { 2 } else { 0 };
                        let span = vb[axis] - va[axis];
                        let (mut tmin, mut tmax) = if span.abs() > f32::EPSILON {
                            ((lo - va[axis]) / span, (hi - va[axis]) / span)
                        } else {
                            (0.0, 1.0)
                        };
                        if tmin > tmax {
                            std::mem::swap(&mut tmin, &mut tmax);
                        }
                        new_links.push((
                            PolyRef {
                                tile_x: from.0,
                                tile_y: from.1,
                                poly: pi as u16,
                            },
                            ExtLink {
                                to: PolyRef {
                                    tile_x: to.0,
                                    tile_y: to.1,
                                    poly: qi as u16,
                                },
                                edge: j as u8,
                                tmin: tmin.clamp(0.0, 1.0),
                                tmax: tmax.clamp(0.0, 1.0),
                            },
                        ));
                    }
                }
            }
        }

        for (from_ref, link) in new_links {
            self.ext_links.entry(from_ref).or_default().push(link);
        }
    }

    fn tile_coords(&self, p: [f32; 3]) -> Option<(i32, i32)> {
        let params = self.params?;
        Some((
            ((p[0] - params.origin[0]) / params.tile_width).floor() as i32,
            ((p[2] - params.origin[2]) / params.tile_height).floor() as i32,
        ))
    }

    /// Whether the tile under a world position is loaded.
    pub fn has_tile_at(&self, pos: Position) -> bool {
        match self.tile_coords(to_recast(pos)) {
            Some(key) => self.tiles.contains_key(&key),
            None => !self.tiles.is_empty(),
        }
    }

    fn poly(&self, r: PolyRef) -> Option<(&NavTile, &NavPoly)> {
        let tile = self.tiles.get(&(r.tile_x, r.tile_y))?;
        let poly = tile.polys.get(r.poly as usize)?;
        Some((tile, poly))
    }

    fn passes(poly: &NavPoly, include: u16) -> bool {
        poly.kind == POLYTYPE_GROUND && poly.flags & include != 0
    }

    /// Nearest polygon to `center` within `half_extents` (Recast space),
    /// returning it with the closest point on it.
    fn nearest_poly(
        &self,
        center: [f32; 3],
        half_extents: [f32; 3],
        include: u16,
    ) -> Option<(PolyRef, [f32; 3])> {
        let qmin = [
            center[0] - half_extents[0],
            center[1] - half_extents[1],
            center[2] - half_extents[2],
        ];
        let qmax = [
            center[0] + half_extents[0],
            center[1] + half_extents[1],
            center[2] + half_extents[2],
        ];
        let candidate_tiles: Vec<(i32, i32)> = match (self.tile_coords(qmin), self.tile_coords(qmax)) {
            (Some((x0, y0)), Some((x1, y1))) => (x0..=x1)
                .flat_map(|x| (y0..=y1).map(move |y| (x, y)))
                .filter(|key| self.tiles.contains_key(key))
                .collect(),
            _ => self.tiles.keys().copied().collect(),
        };

        let mut best: Option<(f32, PolyRef, [f32; 3])> = None;
        for key in candidate_tiles {
            let tile = &self.tiles[&key];
            for (pi, poly) in tile.polys.iter().enumerate() {
                if !Self::passes(poly, include) {
                    continue;
                }
                let (lo, hi) = tile.poly_bounds[pi];
                if (0..3).any(|k| hi[k] < qmin[k] || lo[k] > qmax[k]) {
                    continue;
                }
                let (closest, over) = tile.closest_point_on_poly(pi, center);
                let d = if over {
                    // Directly over the polygon within climb height counts as on it.
                    let dy = (center[1] - closest[1]).abs() - tile.header.walkable_climb;
                    if dy > 0.0 {
                        dy * dy
                    } else {
                        0.0
                    }
                } else {
                    dist_sq(center, closest)
                };
                if best.as_ref().map_or(true, |(bd, _, _)| d < *bd) {
                    best = Some((
                        d,
                        PolyRef {
                            tile_x: key.0,
                            tile_y: key.1,
                            poly: pi as u16,
                        },
                        closest,
                    ));
                }
            }
        }
        best.map(|(_, r, p)| (r, p))
    }

    /// Walkable polygon under (or near) a world position, with the point on it.
    /// Points more than three yards above the position are rejected.
    pub fn find_walk_poly(&self, pos: Position, include: u16) -> Option<(PolyRef, Position)> {
        let p = to_recast(pos);
        let (r, closest) = self.nearest_poly(p, [5.0, 10.0, 5.0], include)?;
        if closest[1] > p[1] + 3.0 {
            return None;
        }
        Some((r, to_world(closest)))
    }

    fn neighbours(&self, r: PolyRef, include: u16) -> Vec<(PolyRef, u8, f32, f32)> {
        let Some((_, poly)) = self.poly(r) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for j in 0..poly.vert_count as usize {
            let nei = poly.neis[j];
            if nei != 0 && nei & DT_EXT_LINK == 0 {
                let to = PolyRef {
                    poly: nei - 1,
                    ..r
                };
                if self.poly(to).is_some_and(|(_, p)| Self::passes(p, include)) {
                    out.push((to, j as u8, 0.0, 1.0));
                }
            }
        }
        if let Some(links) = self.ext_links.get(&r) {
            for link in links {
                if self.poly(link.to).is_some_and(|(_, p)| Self::passes(p, include)) {
                    out.push((link.to, link.edge, link.tmin, link.tmax));
                }
            }
        }
        out
    }

    /// Portal between two adjacent polygons, as (left, right) in Recast space.
    fn portal(&self, from: PolyRef, to: PolyRef, include: u16) -> Option<([f32; 3], [f32; 3])> {
        let (tile, poly) = self.poly(from)?;
        let (_, edge, tmin, tmax) = self
            .neighbours(from, include)
            .into_iter()
            .find(|(n, ..)| *n == to)?;
        let nv = poly.vert_count as usize;
        let v0 = tile.poly_vert(poly, edge as usize);
        let v1 = tile.poly_vert(poly, (edge as usize + 1) % nv);
        if tmin > 0.0 || tmax < 1.0 {
            Some((lerp(v0, v1, tmin), lerp(v0, v1, tmax)))
        } else {
            Some((v0, v1))
        }
    }

    fn closest_point_on(&self, r: PolyRef, p: [f32; 3]) -> [f32; 3] {
        match self.tiles.get(&(r.tile_x, r.tile_y)) {
            Some(tile) => tile.closest_point_on_poly(r.poly as usize, p).0,
            None => p,
        }
    }

    fn closest_point_on_boundary(&self, r: PolyRef, p: [f32; 3]) -> [f32; 3] {
        let Some((tile, poly)) = self.poly(r) else {
            return p;
        };
        let verts: Vec<[f32; 3]> = (0..poly.vert_count as usize)
            .map(|i| tile.poly_vert(poly, i))
            .collect();
        if point_in_polygon_2d(p, &verts) {
            return self.closest_point_on(r, p);
        }
        closest_point_on_boundary(p, &verts)
    }

    /// A* over polygons. Returns the corridor to `end`, or to the polygon that
    /// got closest to it when `end` is unreachable.
    fn find_poly_path(
        &self,
        start: PolyRef,
        end: PolyRef,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        include: u16,
    ) -> Vec<PolyRef> {
        struct Node {
            pos: [f32; 3],
            g: f32,
            parent: Option<PolyRef>,
            closed: bool,
        }
        #[derive(PartialEq)]
        struct Open(f32, PolyRef);
        impl Eq for Open {}
        impl PartialOrd for Open {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for Open {
            fn cmp(&self, other: &Self) -> Ordering {
                other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
            }
        }

        if start == end {
            return vec![start];
        }
        let mut nodes: HashMap<PolyRef, Node> = HashMap::new();
        let mut open = BinaryHeap::new();
        nodes.insert(
            start,
            Node {
                pos: start_pos,
                g: 0.0,
                parent: None,
                closed: false,
            },
        );
        open.push(Open(dist(start_pos, end_pos) * 0.999, start));
        let mut best = start;
        let mut best_h = dist(start_pos, end_pos);

        while let Some(Open(_, current)) = open.pop() {
            let (cur_pos, cur_g) = {
                let node = nodes.get_mut(&current).expect("opened node exists");
                if node.closed {
                    continue;
                }
                node.closed = true;
                (node.pos, node.g)
            };
            if current == end {
                best = end;
                break;
            }
            if nodes.len() >= MAX_SEARCH_NODES {
                break;
            }
            for (next, ..) in self.neighbours(current, include) {
                if nodes.get(&next).is_some_and(|n| n.closed) {
                    continue;
                }
                let Some((left, right)) = self.portal(current, next, include) else {
                    continue;
                };
                let mid = lerp(left, right, 0.5);
                let mut g = cur_g + dist(cur_pos, mid);
                if next == end {
                    g += dist(mid, end_pos);
                }
                let h = dist(mid, end_pos);
                let better = nodes.get(&next).map_or(true, |n| g < n.g);
                if !better {
                    continue;
                }
                nodes.insert(
                    next,
                    Node {
                        pos: mid,
                        g,
                        parent: Some(current),
                        closed: false,
                    },
                );
                if h < best_h {
                    best_h = h;
                    best = next;
                }
                open.push(Open(g + h * 0.999, next));
            }
        }

        let mut path = Vec::new();
        let mut cursor = Some(best);
        while let Some(r) = cursor {
            path.push(r);
            cursor = nodes.get(&r).and_then(|n| n.parent);
        }
        path.reverse();
        path.truncate(MAX_PATH_LENGTH);
        path
    }

    /// Pull a polygon corridor tight between `start` and `end` (Recast space).
    fn straight_path(
        &self,
        start: [f32; 3],
        end: [f32; 3],
        path: &[PolyRef],
        include: u16,
    ) -> Vec<[f32; 3]> {
        let mut points = vec![start];
        if path.len() > 1 {
            let mut apex = start;
            let mut left = start;
            let mut right = start;
            let (mut apex_index, mut left_index, mut right_index) = (0usize, 0usize, 0usize);
            let mut i = 0usize;
            while i < path.len() {
                let (pl, pr) = if i + 1 < path.len() {
                    match self.portal(path[i], path[i + 1], include) {
                        Some(portal) => portal,
                        None => break,
                    }
                } else {
                    (end, end)
                };

                // Skip a first portal the start point already stands on.
                if i == 0 && dist_pt_seg_sq_2d(apex, pl, pr) < 0.001 * 0.001 {
                    i += 1;
                    continue;
                }

                // Right side of the funnel.
                if tri_area_2d(apex, right, pr) <= 0.0 {
                    if vequal(apex, right) || tri_area_2d(apex, left, pr) > 0.0 {
                        right = pr;
                        right_index = i;
                    } else {
                        apex = left;
                        apex_index = left_index;
                        push_point(&mut points, apex);
                        left = apex;
                        right = apex;
                        left_index = apex_index;
                        right_index = apex_index;
                        i = apex_index + 1;
                        continue;
                    }
                }

                // Left side of the funnel.
                if tri_area_2d(apex, left, pl) >= 0.0 {
                    if vequal(apex, left) || tri_area_2d(apex, right, pl) < 0.0 {
                        left = pl;
                        left_index = i;
                    } else {
                        apex = right;
                        apex_index = right_index;
                        push_point(&mut points, apex);
                        left = apex;
                        right = apex;
                        left_index = apex_index;
                        right_index = apex_index;
                        i = apex_index + 1;
                        continue;
                    }
                }
                i += 1;
            }
        }
        push_point(&mut points, end);
        points.truncate(MAX_PATH_LENGTH);
        points
    }

    /// Find a path between two world positions over polygons passing `include`.
    pub fn find_path(&self, start: Position, end: Position, include: u16) -> NavPath {
        if !self.has_tile_at(start) || !self.has_tile_at(end) {
            return NavPath::NoTiles;
        }
        let (Some((start_ref, start_pt)), Some((end_ref, end_pt))) = (
            self.find_walk_poly(start, include),
            self.find_walk_poly(end, include),
        ) else {
            return NavPath::NoPolygon;
        };

        let start_r = to_recast(start_pt);
        let mut end_r = to_recast(end_pt);
        let mut complete = true;

        // Far from the mesh: aim at the edge of the end polygon instead.
        if distance(start, start_pt) > 7.0 || distance(end, end_pt) > 7.0 {
            end_r = self.closest_point_on_boundary(end_ref, to_recast(end));
            complete = false;
        }

        let corridor = self.find_poly_path(start_ref, end_ref, start_r, end_r, include);
        if corridor.is_empty() {
            return NavPath::NoPolygon;
        }
        let last = *corridor.last().expect("corridor is not empty");
        if last != end_ref {
            end_r = self.closest_point_on(last, end_r);
            complete = false;
        }

        let points = self
            .straight_path(start_r, end_r, &corridor, include)
            .into_iter()
            .map(to_world)
            .collect();
        NavPath::Path { points, complete }
    }
}

fn neighbour_tile((x, y): (i32, i32), side: u8) -> (i32, i32) {
    match side {
        0 => (x + 1, y),
        1 => (x + 1, y + 1),
        2 => (x, y + 1),
        3 => (x - 1, y + 1),
        4 => (x - 1, y),
        5 => (x - 1, y - 1),
        6 => (x, y - 1),
        _ => (x + 1, y - 1),
    }
}

fn opposite_side(side: u8) -> u8 {
    (side + 4) & 7
}

fn slab_coord(v: [f32; 3], side: u8) -> f32 {
    if side == 0 || side == 4 {
        v[0]
    } else {
        v[2]
    }
}

/// Edge end points projected onto the border it lies on: (position along the
/// border, height), ordered by position.
fn slab_end_points(va: [f32; 3], vb: [f32; 3], side: u8) -> ([f32; 2], [f32; 2]) {
    let axis = if side == 0 || side == 4 { 2 } else { 0 };
    if va[axis] < vb[axis] {
        ([va[axis], va[1]], [vb[axis], vb[1]])
    } else {
        ([vb[axis], vb[1]], [va[axis], va[1]])
    }
}

/// Whether two border edges overlap along the border and meet in height
/// (within `py`). Edges that only touch at end points do not connect.
fn overlap_slabs(amin: [f32; 2], amax: [f32; 2], bmin: [f32; 2], bmax: [f32; 2], px: f32, py: f32) -> bool {
    let minx = (amin[0] + px).max(bmin[0] + px);
    let maxx = (amax[0] - px).min(bmax[0] - px);
    if minx > maxx {
        return false;
    }
    let slope = |lo: [f32; 2], hi: [f32; 2]| {
        let run = hi[0] - lo[0];
        let d = if run.abs() > f32::EPSILON {
            (hi[1] - lo[1]) / run
        } else {
            0.0
        };
        (d, lo[1] - d * lo[0])
    };
    let (ad, ak) = slope(amin, amax);
    let (bd, bk) = slope(bmin, bmax);
    let dmin = (bd * minx + bk) - (ad * minx + ak);
    let dmax = (bd * maxx + bk) - (ad * maxx + ak);
    if dmin * dmax < 0.0 {
        return true;
    }
    let thr = (py * 2.0) * (py * 2.0);
    dmin * dmin <= thr || dmax * dmax <= thr
}

/// World position to Recast space.
pub fn to_recast(p: Position) -> [f32; 3] {
    [p.y, p.z, p.x]
}

/// Recast space to world position.
pub fn to_world(v: [f32; 3]) -> Position {
    Position::new(v[2], v[0], v[1], 0.0)
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    dist_sq(a, b).sqrt()
}

fn distance(a: Position, b: Position) -> f32 {
    dist(to_recast(a), to_recast(b))
}

fn vequal(a: [f32; 3], b: [f32; 3]) -> bool {
    const THR: f32 = 1.0 / 16384.0;
    dist_sq(a, b) < THR * THR
}

fn push_point(points: &mut Vec<[f32; 3]>, p: [f32; 3]) {
    if points.last().map_or(true, |&last| !vequal(last, p)) {
        points.push(p);
    }
}

/// Signed double area of the 2D (x/z) triangle.
fn tri_area_2d(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let abx = b[0] - a[0];
    let abz = b[2] - a[2];
    let acx = c[0] - a[0];
    let acz = c[2] - a[2];
    acx * abz - abx * acz
}

fn dist_pt_seg_sq_2d(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let (pqx, pqz) = (b[0] - a[0], b[2] - a[2]);
    let (dx, dz) = (p[0] - a[0], p[2] - a[2]);
    let d = pqx * pqx + pqz * pqz;
    let mut t = pqx * dx + pqz * dz;
    if d > 0.0 {
        t /= d;
    }
    let t = t.clamp(0.0, 1.0);
    let ex = a[0] + t * pqx - p[0];
    let ez = a[2] + t * pqz - p[2];
    ex * ex + ez * ez
}

/// Height of triangle `abc` at `p` (x/z), if `p` lies inside it.
fn closest_height_on_triangle(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<f32> {
    const EPS: f32 = 1e-6;
    let v0 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let v1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v2 = [p[0] - a[0], p[2] - a[2]];
    let mut denom = v0[0] * v1[2] - v0[2] * v1[0];
    if denom.abs() < EPS {
        return None;
    }
    let mut u = v1[2] * v2[0] - v1[0] * v2[1];
    let mut v = v0[0] * v2[1] - v0[2] * v2[0];
    if denom < 0.0 {
        denom = -denom;
        u = -u;
        v = -v;
    }
    let tol = denom * 1e-4;
    if u >= -tol && v >= -tol && u + v <= denom + tol {
        Some(a[1] + (v0[1] * u + v1[1] * v) / denom)
    } else {
        None
    }
}

/// Point-in-polygon test on the x/z plane.
fn point_in_polygon_2d(p: [f32; 3], verts: &[[f32; 3]]) -> bool {
    let mut inside = false;
    let n = verts.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let vi = verts[i];
        let vj = verts[j];
        if ((vi[2] > p[2]) != (vj[2] > p[2]))
            && (p[0] < (vj[0] - vi[0]) * (p[2] - vi[2]) / (vj[2] - vi[2]) + vi[0])
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Closest point to `p` on the polygon's edges (x/z distance, height interpolated).
fn closest_point_on_boundary(p: [f32; 3], verts: &[[f32; 3]]) -> [f32; 3] {
    let mut best = verts.first().copied().unwrap_or(p);
    let mut best_d = f32::MAX;
    for i in 0..verts.len() {
        let a = verts[i];
        let b = verts[(i + 1) % verts.len()];
        let (abx, abz) = (b[0] - a[0], b[2] - a[2]);
        let len = abx * abx + abz * abz;
        let t = if len > 0.0 {
            (((p[0] - a[0]) * abx + (p[2] - a[2]) * abz) / len).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = lerp(a, b, t);
        let d = (q[0] - p[0]).powi(2) + (q[2] - p[2]).powi(2);
        if d < best_d {
            best_d = d;
            best = q;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tile from polygons in Recast space for tests.
    fn tile(x: i32, y: i32, verts: Vec<[f32; 3]>, polys: Vec<(Vec<u16>, Vec<u16>)>) -> NavTile {
        let polys: Vec<NavPoly> = polys
            .into_iter()
            .map(|(pv, pn)| {
                let mut v = [0u16; VERTS_PER_POLY];
                let mut n = [0u16; VERTS_PER_POLY];
                v[..pv.len()].copy_from_slice(&pv);
                n[..pn.len()].copy_from_slice(&pn);
                NavPoly {
                    verts: v,
                    neis: n,
                    flags: nav_terrain::GROUND,
                    vert_count: pv.len() as u8,
                    area: 1,
                    kind: POLYTYPE_GROUND,
                }
            })
            .collect();
        let mut t = NavTile {
            header: TileHeader {
                x,
                y,
                layer: 0,
                poly_count: polys.len(),
                vert_count: verts.len(),
                walkable_height: 2.0,
                walkable_radius: 0.5,
                walkable_climb: 1.0,
                bmin: [0.0; 3],
                bmax: [0.0; 3],
            },
            verts,
            polys,
            detail_meshes: Vec::new(),
            detail_verts: Vec::new(),
            detail_tris: Vec::new(),
            poly_bounds: Vec::new(),
        };
        t.poly_bounds = (0..t.polys.len()).map(|i| poly_bounds(&t, i)).collect();
        t
    }

    fn params() -> NavMeshParams {
        NavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 10.0,
            tile_height: 10.0,
        }
    }

    /// Two 10x10 square tiles side by side along Recast x, joined by a border
    /// portal on x = 10. Each square is one polygon.
    fn two_tiles() -> NavMesh {
        let mut mesh = NavMesh::with_params(params());
        // Recast (x, y, z); edge 2 of the left square faces +x (side 0).
        mesh.add_tile(tile(
            0,
            0,
            vec![[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [10.0, 0.0, 10.0], [10.0, 0.0, 0.0]],
            vec![(vec![0, 1, 2, 3], vec![0, 0, DT_EXT_LINK, 0])],
        ));
        mesh.add_tile(tile(
            1,
            0,
            vec![[10.0, 0.0, 0.0], [10.0, 0.0, 10.0], [20.0, 0.0, 10.0], [20.0, 0.0, 0.0]],
            vec![(vec![0, 1, 2, 3], vec![DT_EXT_LINK | 4, 0, 0, 0])],
        ));
        mesh
    }

    fn world(rx: f32, rz: f32) -> Position {
        to_world([rx, 0.0, rz])
    }

    #[test]
    fn border_portals_connect_adjacent_tiles_in_both_directions() {
        let mesh = two_tiles();
        assert_eq!(mesh.external_link_count(), 2);
        match mesh.find_path(world(2.0, 5.0), world(18.0, 5.0), nav_terrain::GROUND) {
            NavPath::Path { points, complete } => {
                assert!(complete);
                assert_eq!(points.len(), 2, "straight across the portal: {points:?}");
            }
            other => panic!("expected a path, got {other:?}"),
        }
    }

    #[test]
    fn unloading_a_tile_removes_links_into_it() {
        let mut mesh = two_tiles();
        assert!(mesh.remove_tile(1, 0));
        assert_eq!(mesh.external_link_count(), 0);
        assert!(matches!(
            mesh.find_path(world(2.0, 5.0), world(18.0, 5.0), nav_terrain::GROUND),
            NavPath::NoTiles
        ));
    }

    #[test]
    fn filtered_polygons_are_not_walked() {
        let mut mesh = two_tiles();
        if let Some(t) = mesh.tiles.get_mut(&(1, 0)) {
            t.polys[0].flags = nav_terrain::WATER;
        }
        assert!(matches!(
            mesh.find_path(world(2.0, 5.0), world(18.0, 5.0), nav_terrain::GROUND),
            NavPath::NoPolygon
        ));
    }

    #[test]
    fn corridor_bends_around_a_missing_polygon() {
        // An L-shaped mesh in one tile: the straight line from the bottom-right
        // square to the top-left square crosses the missing top-right corner.
        let verts = vec![
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 4.0],
            [4.0, 0.0, 4.0],
            [4.0, 0.0, 0.0],
            [8.0, 0.0, 4.0],
            [8.0, 0.0, 0.0],
            [0.0, 0.0, 8.0],
            [4.0, 0.0, 8.0],
        ];
        // poly 0: (0,0)-(4,4); poly 1: right of it (4..8, 0..4); poly 2: above it (0..4, 4..8)
        let polys = vec![
            (vec![0, 1, 2, 3], vec![0, 3, 2, 0]),
            (vec![3, 2, 4, 5], vec![1, 0, 0, 0]),
            (vec![1, 6, 7, 2], vec![0, 0, 0, 1]),
        ];
        let mut mesh = NavMesh::with_params(NavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 100.0,
            tile_height: 100.0,
        });
        mesh.add_tile(tile(0, 0, verts, polys));
        match mesh.find_path(world(7.0, 3.0), world(3.0, 7.0), nav_terrain::GROUND) {
            NavPath::Path { points, complete } => {
                assert!(complete);
                assert!(points.len() >= 3, "must turn at the inner corner: {points:?}");
                let corner = to_world([4.0, 0.0, 4.0]);
                assert!(points
                    .iter()
                    .any(|p| (p.x - corner.x).abs() < 0.01 && (p.y - corner.y).abs() < 0.01));
            }
            other => panic!("expected a path, got {other:?}"),
        }
    }

    #[test]
    fn truncated_or_foreign_data_is_rejected() {
        assert!(parse_detour_tile(&[0u8; 50]).is_none());
        let mut data = vec![0u8; HEADER_SIZE];
        data[..4].copy_from_slice(&DT_NAVMESH_MAGIC.to_le_bytes());
        data[4..8].copy_from_slice(&DT_NAVMESH_VERSION.to_le_bytes());
        // One polygon declared but no room for it.
        data[24..28].copy_from_slice(&1i32.to_le_bytes());
        assert!(parse_detour_tile(&data).is_none());
    }

    #[test]
    fn coordinates_round_trip_between_world_and_recast_space() {
        let p = Position::new(1.0, 2.0, 3.0, 0.0);
        assert_eq!(to_recast(p), [2.0, 3.0, 1.0]);
        let back = to_world(to_recast(p));
        assert_eq!((back.x, back.y, back.z), (1.0, 2.0, 3.0));
    }
}
