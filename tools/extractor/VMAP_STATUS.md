# VMap Extraction — Current State

**Date:** 2026-04-16
**Test map:** 33 (Shadowfang Keep)
**Source client:** `C:\Users\krist\Desktop\WOW\RetroWoW\RetroWoW`
**Reference:** `C:\Users\krist\Projects\wow\rcore\data\vmaps\` (original MaNGOS C++ extractor output)
**Our output:** `C:\Users\krist\Projects\wow\rcore\tools\extractor\data_tmp\vmaps\`

## Pipeline Overview

Two-stage extraction matching MaNGOS `vmap_extractor` + `vmap_assembler`:

1. **Extract** (`vmaps -i <client> -o <out> [maps...]`)
   - Parse MPQs, extract WMO roots + groups, extract M2 models
   - Write raw `VMAPs05` chunked format to `<out>/vmaps/Buildings/<Name>.wmo` and `<Name>.m2`
   - Write `<out>/vmaps/Buildings/dir_bin` index of every spawn (map, tile, flags, position, rotation, scale, bounds, name)

2. **Assemble** (`vmaps --assemble-only -o <out>`)
   - Convert each raw `VMAPs05` model → final `VMAP_7.0` `.vmo` (WMOD/GMOD/VERT/TRIM/MBIH/LIQU/GBIH chunks)
   - Build per-map BIH tree → `<NNN>.vmtree`
   - Group spawns per ADT tile → `<NNN>_<X>_<Y>.vmtile`

## Results (Map 33, tile 27_30)

| Metric                  | Ours     | Reference | Gap       |
|-------------------------|----------|-----------|-----------|
| `033_27_30.vmtile` size | 20449 B  | 24553 B   | 4104 B    |
| Spawns in tile          | 232      | 280       | 48 spawns |
| Spawns with bounds      | 232      | 280       | 48        |
| Flags `0x04` (WMO)      | 3        | 3         | 0         |
| Flags `0x05` (M2+bound) | 229      | 277       | 48        |
| Flags `0x01` (M2 only)  | 0        | 0         | 0         |
| `033.vmtree` size       | 66449 B  | 81281 B   | 14832 B   |
| Total `.vmo` files      | 133      | ~300+     | missing doodads |

**First-spawn WMO byte-size match (sanity check):**
- `Shipwreck_A.wmo.vmo`: ours 231452 B = ref 231452 B ✓
- `Duskwoodabandoned_Barn.wmo.vmo`: ours 109708 B = ref 109708 B ✓

## What Works

- ✅ Raw `VMAPs05` extraction (WMO roots + groups combined into one `.wmo`, M2 bounding geometry)
- ✅ `.vmo` conversion (VMAP_7.0 chunk format with BIH per group + group-level GBIH)
- ✅ Coordinate transforms (`fixCoords` cyclic rotation)
- ✅ WMO collision-triangle filtering (MOPY render/collision/detail flags)
- ✅ `fix_name_case` — MaNGOS `FixNameCase` (title-case base + lowercase ext)
- ✅ `.mdx`/`.mdl` → `.m2` rename on disk and in dir_bin
- ✅ WMO scale hardcoded to 1.0 (matches `wmo.cpp:596`)
- ✅ M2 dir_bin filter: skip doodads whose extracted `.m2` file is missing or has `nVertices == 0` (matches `model.cpp:261-267`)
- ✅ Per-M2 bound computation in assembly (`TileAssembler::calculateTransformedBound` equivalent): rotate + scale + translate vertices, AABB result, set `MOD_HAS_BOUND`
- ✅ Server `MOD_HAS_BOUND` flag value fixed (`1 << 2` not `1 << 0`)

## Open Issues

### Issue A: WMO-internal doodads not extracted (biggest gap)

**Symptom:** ~48 M2 spawns missing per tile (`Innbarrel.m2`, `Candleoff01.m2`, `Torch.m2`, `Crate01.m2`, `Hammock03.m2`, `Coffin.m2`, etc.)

**Cause:** MaNGOS iterates each WMO's MODD (doodad defs) + MODS (doodad sets), transforms each spawn by WMO rotation/position, writes `MOD_M2` entries to dir_bin. We parse MODD/MODS into `WMORoot` struct but never emit them.

**Fix:** Port `reference/vmangos/contrib/vmap_extractor/vmapextract/model.cpp:232-331`:
- For every WMO placement (global + ADT), loop `DoodadReferences`, filter by doodad set
- Compute `worldPos = wmoPos + wmoRot * doodad.local_pos`
- Compose `rotation = doodad.quat.toMatrix() * wmoRot`, convert to Euler degrees
- Check extracted doodad `.m2` file has `nVertices > 0` (same filter as top-level M2s)
- Write `dir_bin` entry with `flags = MOD_M2`, `uniqueId = GenerateUniqueObjectId(wmo.UniqueId, ++doodadId)`

**Files to change:** `tools/extractor/src/vmaps/mod.rs` (extend WMO placement loop in `process_map`)

### Issue B: Name null-byte quirk from MaNGOS `.mdx → .m2` rename

**Symptom:** Reference `nameLen = 13` for `"Innbarrel.m2\x00"` (13 bytes incl. trailing null). Ours: `nameLen = 12` for `"Innbarrel.m2"` (no null).

**Cause:** MaNGOS `model.cpp:202,243,298` computes `nlen = strlen(ModelInstName)` BEFORE the `.mdx` → `.m2` replacement at line 251-252 (which writes `'2'` at `[nlen-2]` and `'\0'` at `[nlen-1]`). So when source was `.mdx`, nlen stays at 4-char-extension size, and the trailing null byte is written. For originally-`.m2` names, no null byte.

**Fix:** In `dir_bin::write_entry`, when the name was converted from `.mdx`/`.mdl`, write `nlen = original_len_before_rename` and include the trailing null. Alternatively: always append `\0` and add 1 to nlen when a conversion happened. Record whether a conversion happened on the `DirBinEntry` struct.

**Impact:** Byte-level diff with reference; minor. Our server and MaNGOS server both strip trailing nulls for `.vmo` lookup.

### Issue C: `uniqueId` numbering scheme

**Symptom:** Ours: sequential counter starting at 1. Ref: e.g. `256660` (high-bit composition of WMO unique id + doodad index).

**Cause:** MaNGOS `GenerateUniqueObjectId(id, 0)` = `(id << 16) | (0 & 0xFFFF)` for top-level M2s (`model.cpp:191`) and `GenerateUniqueObjectId(wmoUniqueId, doodadIndex)` for WMO-internal doodads. For M2s the `id` is the MDDF `uniqueId`.

**Fix:** Use actual MDDF `unique_id` field (already parsed as `M2Placement.unique_id`), write `(unique_id << 16) | doodadInnerIdx`. For top-level M2s `doodadInnerIdx = 0`.

**Impact:** Non-functional (server uses name → `.vmo`, not uniqueId), but affects byte-diff and may matter for any tool that deduplicates by uniqueId.

### Issue D: Spawn order within vmtile differs

**Symptom:** Our first spawn in tile is `Shipwreck_A.wmo`; ref is `Silverpinetree02.m2`.

**Cause:** MaNGOS extraction order is ADT iteration order (MMDX → MDDF → MODF), plus WMO-internal doodads are written inline after each WMO placement. We write global WMOs first, then per-tile WMOs, then per-tile M2s — different interleaving.

**Fix (optional):** Match MaNGOS traversal order if byte-parity required. Server doesn't depend on order.

### Issue E: BIH tree structure differences

**Symptom:** `033.vmtree` 66449 B (ours) vs 81281 B (ref). Our tree is 18% smaller even though per-spawn data is comparable.

**Cause:** Our BIH build uses a different split heuristic or leaf size threshold vs MaNGOS `BIH::build`. Ref tree likely has more nodes (shallower leaves).

**Fix:** Port MaNGOS `BIH::buildHierarchy()` (reference/vmangos/src/game/vmap/BIH.cpp) exactly — leaf threshold, split plane selection.

**Impact:** Server queries work regardless of tree shape (both are valid BIHs), but performance characteristics differ.

## Code Hotspots

- `tools/extractor/src/vmaps/mod.rs` — extraction orchestration, `process_map`, `extract_m2_file`, `extract_wmo_file`, `fix_name_case`
- `tools/extractor/src/vmaps/dir_bin.rs` — `DirBinEntry` (in-memory), `DirBinWriter`/`DirBinReader`
- `tools/extractor/src/vmaps/vmo_converter.rs` — `VMAPs05` → `VMAP_7.0` `.vmo` conversion, `read_raw_model`, `convert_raw_file`
- `tools/extractor/src/vmaps/tree/builder.rs` — `build_map_tree`, M2 bound computation via `read_raw_model_vertices`
- `tools/extractor/src/vmaps/tree/bih.rs` — BIH implementation
- `tools/extractor/src/vmaps/tree/output.rs` — vmtree/vmtile writers
- `tools/extractor/src/vmaps/wmo/parser.rs` — WMO MODD/MODS parsing (data ready, not emitted)
- `src/world/map/pathfinding/vmap/file_loader.rs` — server-side loader, `MOD_HAS_BOUND = 1 << 2` (fixed)

## Testing Workflow (fast iterate)

```bash
# 1. Clean stale data and re-extract (map 33 only)
rm -rf tools/extractor/data_tmp/vmaps
tools/extractor/target/debug/extractor.exe vmaps \
  -i "C:/Users/krist/Desktop/WOW/RetroWoW/RetroWoW" \
  -o ./tools/extractor/data_tmp 33

# 2. Assemble (fast; no MPQ access)
tools/extractor/target/debug/extractor.exe vmaps \
  -i "C:/Users/krist/Desktop/WOW/RetroWoW/RetroWoW" \
  -o ./tools/extractor/data_tmp --assemble-only 33

# 3. Compare
node tools/compare-vmtile.js
```

Build: user-driven (`cargo build --bin extractor` in debug profile). Release builds blocked by gcc/sqlite toolchain; debug works.

## Reference Files

- `reference/vmangos/contrib/vmap_extractor/vmapextract/model.cpp` — M2 + WMO-doodad extraction, dir_bin write
- `reference/vmangos/contrib/vmap_extractor/vmapextract/wmo.cpp` — WMO placement write (`scale = 1.0f`)
- `reference/vmangos/contrib/vmap_extractor/vmapextract/adtfile.cpp` — `FixNameCase`, `FixNameSpaces`
- `reference/vmangos/contrib/vmap_extractor/vmapextract/vmapexport.h` — flag definitions (`MOD_M2 = 1`, `MOD_WORLDSPAWN = 2`, `MOD_HAS_BOUND = 4`)
- `reference/vmangos/contrib/vmap_assembler/vmap_assembler.cpp` — assembly entry
- `reference/vmangos/src/game/vmap/TileAssembler.cpp` — `convertRawFile` (300-331), `calculateTransformedBound` (245-286), `WorldModel_Raw::Read` (500-530), `GroupModel_Raw::Read` (402-492)
- `reference/vmangos/src/game/vmap/WorldModel.cpp` — `writeFile` (558-590), `readFile` (592-626), `GroupModel::writeToFile` (269-310)
- `reference/vmangos/src/game/vmap/ModelInstance.h` — flag enums (38-41)
- `reference/vmangos/src/game/vmap/BIH.cpp` — BIH build algorithm

## Next Step

Issue A (WMO doodads) — biggest gap, most impact on collision completeness in dungeons. Pick this up before full-map extraction.
