# Maps System — reference comparison and remediation plan

> **Status (2026-07-25).** The first remediation pass has landed.
>
> | # | Area | Status |
> |---|---|---|
> | 1 | VMap/MMap tile indices | **fixed** — `grid_to_terrain_tile`, verified against files on disk |
> | 2 | Visibility + activation distances | **fixed** — `MapConfig`, 100/170/533 + config keys, runtime ramp, corner-box activation |
> | 3 | Grid unload guards | **fixed** — `active_objects_near_grid`, unload locks, evade-on-idle, respawn relocation |
> | 4 | Grid membership write-once | **fixed** — rosters derived from `Grid::objects` |
> | 5 | Active cells / marked cells | open |
> | 6 | Active objects | **partly** — `Map::active_objects` + spawn-grid pin exist; still no DB `SPAWN_FLAG_ACTIVE` and no always-visible handling |
> | 7 | Per-cell spawn index | **fixed** — per-grid index (`build_grid_index`) |
> | 8–13 | Cell query geometry, `Map` identity, persistent state, zone derivation, update ordering, query helpers | open |
>
> Three further defects were found and fixed during the work, not in the original
> list: `unload_grid` never cleared the map registries, so despawned objects were
> returned by range queries forever; both Lua summon paths never reached the `Map`,
> so those creatures were invisible to every range query while still being ticked;
> and `get_objects_in_range` filtered on grid state, hiding objects during the load
> window. Tests covering each live in `crates/map/tests/`.

Compares `reference/core/src/game/Maps/*` (vmangos/Nostalrius lineage, ~20k LoC)
against our `crates/map` (~6.3k LoC) plus the map-adjacent parts of `crates/world`
(`game/grid_system.rs`, `game/map_update.rs`, `game/visibility`, `game/player/visibility`).

Our architecture is deliberately different (DashMap registries + a leaf `map` crate
with no `World` dependency, instead of intrusive linked lists + template visitors).
This document only calls out **behavioural** gaps — where the core logic differs in a
way a player or a designer would notice — not structural differences.

Ordered by severity.

---

## 1. BLOCKER — VMap/MMap tiles are loaded with the wrong tile indices

**What the reference does.** Two distinct coordinate spaces exist and mangos converts
between them explicitly:

- *NGrid space* (`MaNGOS::ComputeGridPair`, `GridDefines.h:153-168`):
  ```cpp
  int x_val = int(x_offset + CENTER_VAL + 0.5);   // from world x
  int y_val = int(y_offset + CENTER_VAL + 0.5);   // from world y
  return RET_TYPE(y_val, x_val);                  // NOTE: axes swapped on return
  ```
  so `GridPair.x_coord` is derived from world **y**, `GridPair.y_coord` from world **x**.

- *Terrain/file space* (`GridMap.cpp:1132-1136`):
  ```cpp
  int gx = (int)(32 - y / SIZE_OF_GRIDS);
  int gy = (int)(32 - x / SIZE_OF_GRIDS);
  ```

The bridge is in `Map::EnsureGridCreated` (`Map.cpp:329-335`) and `Map::UnloadGrid`
(`Map.cpp:1570-1579`):
```cpp
int gx = (MAX_NUMBER_OF_GRIDS - 1) - p.x_coord;   // 63 - ngrid_x
int gy = (MAX_NUMBER_OF_GRIDS - 1) - p.y_coord;   // 63 - ngrid_y
LoadMapAndVMap(gx, gy);   // -> maps/%03u%02u%02u.map  (mapId, gy, gx)
                          // -> VMapManager::loadMap(mapId, gx, gy)
                          // -> MMapManager::loadMap(mapId, gx, gy)
```

**What we do.** We have two coordinate helpers and they are *not* related to each other:

- `crates/map/src/grid_coords.rs:68` — `world_to_grid(x,y) = (floor((x+17066.67)/533.33), floor((y+17066.67)/533.33))`.
  This is mangos's magnitude formula but **transposed** (our gx ≡ mangos `y_coord`,
  our gy ≡ mangos `x_coord`) and **not mirrored**. Self-consistent for spatial use.
- `crates/map/src/terrain/defines.rs:137` — `terrain_grid_coords` correctly implements
  `(32 - y/533, 32 - x/533)`. Terrain height/liquid lookups are therefore **correct**.

But `crates/world/src/game/grid_system.rs:175-183` feeds the *spatial* indices to the
*file-space* loaders:
```rust
world.managers.vmap_mgr.load_map(map_id, grid_x as i32, grid_y as i32);
world.managers.mmap_mgr.load_map_tile(map_id, grid_x as i32, grid_y as i32);
```
This is the only caller of either loader.

**Worked example** — Northshire Abbey, `(x, y) = (-8949, -132)`:

| space | gx | gy | file requested |
|---|---|---|---|
| ours (spatial) | 15 | 31 | `vmaps/000_15_31.vmtile`, `mmaps/0003115.mmtile` |
| correct (terrain) | 32 | 48 | `vmaps/000_32_48.vmtile`, `mmaps/0004832.mmtile` |

We load geometry from a transposed/mirrored corner of the map, or (more often) no
file exists and we silently get no navmesh and no collision. Every LoS check,
vmap height query and navmesh path in the game is running against wrong or absent data.

**Fix.** In `grid_system.rs`, convert before calling:
```rust
// NGrid space -> terrain/file space, per Map::EnsureGridCreated.
let tile_x = 63 - grid_y as i32;   // our gy ≡ mangos GridPair.y_coord
let tile_y = 63 - grid_x as i32;   // our gx ≡ mangos GridPair.x_coord
world.managers.vmap_mgr.load_map(map_id, tile_x, tile_y);
world.managers.mmap_mgr.load_map_tile(map_id, tile_x, tile_y);
```
Same conversion in `unload_grid` (currently `mmap_mgr.unload_tile(map_id, grid_x, grid_y)`,
`grid_system.rs:336-338`; note vmap tiles are never unloaded at all — see §4).

Better: put the conversion in `crates/map/src/grid_coords.rs` as
`grid_to_terrain_tile(gx, gy) -> (i32, i32)` with a round-trip test against
`terrain_grid_coords`, so the two spaces can never drift again.

---

## 2. CRITICAL — Visibility and grid-activation distances

**Reference** (`ObjectDefines.h:29-32`, `Map.cpp:220-225`, `Map.cpp:2112`, `Map.cpp:2467`):

| constant | value | used by |
|---|---|---|
| `MAX_VISIBILITY_DISTANCE` | `SIZE_OF_GRIDS` = 533.33 | hard cap in `Cell::Visit` |
| `DEFAULT_VISIBILITY_DISTANCE` | **100.0** | continents (`Map::InitVisibilityDistance`) |
| `DEFAULT_VISIBILITY_INSTANCE` | **170.0** | `DungeonMap::InitVisibilityDistance` |
| `DEFAULT_VISIBILITY_BG` | **533.0** | `BattleGroundMap::InitVisibilityDistance` |

Two *separate* distances are tracked per map: `m_visibilityDistance` (what a player is
sent) and `m_gridActivationDistance` (which cells get ticked / which grids get loaded).
Both are adjusted **at runtime** based on map update cost (`Map.cpp:1053-1081`): if a
tick exceeds `MAPUPDATE_TICK_LOWER_*`, the distances decrement one yard per tick down to
a configured floor; when ticks are cheap they climb back to the configured max. This is
the server's main overload valve.

**Ours.** `crates/map/src/map.rs:11` — a single `DEFAULT_VISIBILITY_DISTANCE = 533.33333`
for every map, stored in `Map::visibility_distance` with no setter, no map-type
awareness, and no grid-activation distance at all.

Consequences:
- On continents we make everything within 533 yards visible instead of 100 — ~28× the
  area, so ~28× the create/destroy packets and update-object traffic per player.
- `Map::activate_grids_around_position` (`map.rs:216-254`) computes
  `grid_range = ceil(533.33/533.33) + 1 = 2`, so it activates a **5×5 = 25 grid** block
  per player. Reference activates the cells covering 100 yards — typically 1 grid,
  at most 4. We spawn roughly an order of magnitude more creatures than we should.
- No load-shedding path when a tick runs long.

**Fix.**
1. Add `map_type` to `Map` (from `MapEntry` in DBC — `crates/dbc/src/structures/world.rs:57`)
   and an `init_visibility_distance()` that picks 100 / 170 / 533.
2. Split `visibility_distance` and `grid_activation_distance` into `AtomicU32`
   (or f32 bits) so the tick loop can tune them.
3. Port the ramp from `Map::Update` tail: after measuring the map's tick cost, decrement
   towards the floor or increment towards the max.
4. Make `activate_grids_around_position` use `grid_activation_distance`.

---

## 3. CRITICAL — Grid state machine is missing its guards

**Reference.** Four states (`NGrid.h:70-77`), driven once per map tick from
`Map::Update` (`Map.cpp:1015-1025`) through `MapManager::UpdateGridState` into
`GridStates.cpp`:

- `ACTIVE` → checks only every `grid_expiry/10`. If `grid.ActiveObjectsInGrid() == 0`
  **and** `!map.ActiveObjectsNearGrid(x, y)`, it runs `ObjectGridStoper` and drops to
  `IDLE`. Otherwise it resets the expiry timer. (`GridStates.cpp:32-48`)
- `IDLE` → immediately resets expiry and moves to `REMOVAL`. (`GridStates.cpp:50-56`)
- `REMOVAL` → only if `!info.getUnloadLock()`, and only once the timer passes, calls
  `Map::UnloadGrid(x, y, false)`; on refusal the expiry is reset. (`GridStates.cpp:58-73`)

Supporting machinery we have no equivalent for:

| reference | purpose | our status |
|---|---|---|
| `GridInfo::i_unloadExplicitLock` | `LoadGrid(cell, no_unload=true)` pins grids holding active spawns | missing |
| `GridInfo::i_unloadActiveLockCount` | `Map::AddToActive` pins the *respawn* grid of every active creature so it can't clone on reload (`Map.cpp:1924-1948`) | missing |
| `Map::ActiveObjectsNearGrid` | expands the grid's cell box by `ceil(visDist/33.33)+1` cells and refuses unload if any player or active object falls inside (`Map.cpp:1886-1922`) | missing |
| `ObjectGridStoper` | on ACTIVE→IDLE: `AI()->EnterEvadeMode()`, `DeleteThreatList()`, `RemoveAllDynObjects()` for every creature, without unloading (`ObjectGridLoader.cpp:359-384`) | missing |
| `ObjectGridRespawnMover` / `CreatureRespawnRelocation` | before unload, teleports creatures whose respawn point is in a *different* grid back to it, so they don't vanish permanently (`ObjectGridLoader.cpp:36-81`, `Map.cpp:1510-1535`) | missing |
| `Map::ResetGridExpiry(grid, 0.1f)` on player entry/relocation | keeps a grid alive while a player is walking through it (`Map.cpp:353`, `Map.cpp:1419-1423`) | missing |
| `RemoveAllObjectsInRemoveList()` bracketing the unload | ensures pending deletes settle before/after unload (`Map.cpp:1551-1564`) | missing |

**Ours.** `crates/map/src/grid/grid.rs` has five states (adds `Loading`) but the only
unload gate is `Grid::should_unload` — `state == Idle && idle_since.elapsed() > 5 min`
(`grid.rs:19`, `grid.rs:218-228`), where `Idle` is entered the moment the last *player*
leaves the grid (`grid.rs:145-152`). Nothing checks for players in neighbouring grids,
active objects, or explicit locks.

Behavioural consequences:
- A player standing 10 yards over a grid boundary does not keep the neighbour grid
  loaded; after 5 minutes it unloads under their feet and every creature there despawns
  while still visible.
- Creatures in idle grids keep running full AI/regen/movement for 5 minutes because
  there is no `ObjectGridStoper` equivalent — see §5.
- Nothing prevents unloading a grid that holds an active (`SPAWN_FLAG_ACTIVE`) spawn.

**Fix.** Implement, in order of value:
1. `Map::active_objects_near_grid(gx, gy)` — port `Map.cpp:1886-1922` verbatim using
   the cell box + `ceil(vis/CELL_SIZE)+1` expansion; gate `get_grids_to_unload` on it.
2. Unload locks on `Grid` (`explicit_lock: bool`, `active_lock: u16`) with
   `Grid::unload_locked()`; set the explicit lock from a `load_grid(no_unload)` path.
3. `ObjectGridStoper` equivalent at Active→Idle: evade + clear threat for every creature
   registered in the grid. This is the fix that makes idle grids cheap without unloading.
4. Respawn relocation before despawn in `GridSystem::unload_grid`.

---

## 4. CRITICAL — Grid membership tracking for creatures is write-once

`GridManager::register_creature` / `register_gameobject`
(`crates/map/src/grid/grid.rs:487-502`) push a GUID into the grid's `creatures` /
`gameobjects` SmallVec. They are called from exactly one place —
`grid_system.rs:215` / `:249`, at spawn time — and nothing ever moves an entry between
grids.

Meanwhile `Map::relocate_creature` (`map.rs:122-142`) *does* maintain the per-cell
`objects` lists correctly. So the two views diverge as soon as a creature walks:

- A creature that wanders from grid A into grid B is still despawned when **A** unloads,
  even if B is active and a player is watching it.
- A creature that wanders into A is never despawned when A unloads; it stays in
  `CreatureManager` forever, ticking AI in an unloaded grid.
- Summons never enter these lists at all (`Map::add_creature`, `map.rs:145-150`, does not
  register), so map-summoned creatures leak past every grid unload.

The reference avoids this entirely because grid membership *is* the storage:
`Map::CreatureCellRelocation` (`Map.cpp:1485-1508`) moves the object between
`GridRefManager`s and updates `Creature::SetCurrentCell`, and unload iterates the grid's
actual containers.

**Fix.** Either (a) make `Grid::creatures`/`gameobjects` derived — on unload, filter
`Map::creatures` by `world_to_grid(pos) == (gx, gy)` — or (b) update the registration
lists inside `relocate_creature` / `add_creature` / `remove_creature`. (a) is less code
and cannot desync; (b) is closer to the reference. Also port `Map::CheckGridIntegrity`
(`Map.cpp:1611-1628`) as a debug assertion.

Related: `CreatureCellRelocation` **refuses** to move a non-active creature into an
unloaded grid and falls back to `CreatureRespawnRelocation` (`Map.cpp:1461-1483`). We
happily relocate into unloaded grids, which is how creatures end up alive in grids that
were never loaded.

---

## 5. HIGH — No "active cells" concept: everything ticks, everywhere

**Reference.** Object updates are driven by *cells marked around players and active
objects*, not by a global registry (`Map.cpp:711-866`):

```
UpdateCells(diff)                       // throttled by MAPUPDATE_UPDATE_CELLS_DIFF
 └─ UpdateActiveCellsSynch/Asynch
     ├─ resetMarkedCells()              // bitset<1024*1024>
     ├─ for each player   -> MarkCellsAroundObject(gridActivationDistance)
     ├─ for each active non-player -> MarkCellsAroundObject(...)
     └─ for each marked cell -> Visit(ObjectUpdater)  // creatures only; players/corpses skipped
```
`Cell::CalculateCellArea` bounds the marked box, and cells are visited at most once per
tick thanks to `marked_cells`. Creatures outside every player's activation radius are
simply not updated.

**Ours.** Every creature subsystem iterates the whole `CreatureManager` DashMap each
tick: `game/creature/ai/system.rs:38`, `combat_update.rs:21`, `regen.rs:32`,
`movement/system.rs:33`, `respawn/system.rs:31`, `ai/aggro_scan.rs:32`. The AI pass at
least pre-filters to in-combat/scripted creatures, but movement, regen and aggro scans
touch everything loaded. With 25 grids activated per player (§2) that is a lot of
creatures.

**Fix.** The cheapest faithful port that fits our architecture:
1. Add `Map::marked_cells: bitset` (1024×1024 bits = 128 KiB per map, same as reference)
   plus `mark_cells_around(pos, radius)` and `is_cell_marked(cell_id)`.
2. At the top of the tick, mark cells around every player and every active object.
3. Have the creature subsystems skip creatures whose cell is unmarked — a single
   `world_to_cell(pos)` + bit test per creature, still O(n) but a very cheap n.
4. Once §3.3 (`ObjectGridStoper`) exists, most of those creatures are also evaded and
   idle, so the remaining cost is negligible.

---

## 6. HIGH — Active objects (`SPAWN_FLAG_ACTIVE`) are not implemented

The reference keeps `Map::m_activeNonPlayers` (`Map.h:693-695`) and:
- loads their grids at startup regardless of players — `Map::SpawnActiveObjects` +
  `ActiveObjectsGridLoader` (`Map.cpp:181-218`);
- keeps those grids loaded via `incUnloadActiveLock` (`Map.cpp:1924-1948`);
- marks cells around them every tick so they update with no player nearby
  (`Map.cpp:797-798`);
- pushes them into **every** player's visible set regardless of distance —
  `Map::UpdateActiveObjectVisibility` (`Map.cpp:1650-1687`), called from
  `UpdateObjectVisibility` for players.

We have none of this. Practical impact: world bosses / event NPCs / ships that must exist
and move while nobody is nearby (Doomsday messengers, Kazzak/Azuregos patrol state,
transport-linked NPCs) will not tick, and long-range-visible objects will pop in at 100
yards instead of being always visible.

**Fix.** `Map::active_objects: DashSet<ObjectGuid>` + `add_to_active`/`remove_from_active`
called when a creature/GO with the active spawn flag enters/leaves the world; force-load
its grid at map creation; include the set unconditionally in visibility deltas.

---

## 7. HIGH — Per-cell spawn index missing (grid loading is O(all spawns))

**Reference.** `ObjectMgr` builds `CellObjectGuids` keyed by
`cell_id = cell_y * 1024 + cell_x` at startup, and per-instance overrides live in
`MapPersistentState::GetCellObjectGuids` (`MapPersistentStateMgr.h:110-121`).
`ObjectGridLoader::LoadN` walks the grid's 16×16 cells and pulls the exact guid set for
each (`ObjectGridLoader.cpp:216-260`, `:294-311`).

**Ours.** `CreatureManager::get_spawns_for_grid` (`game/creature/manager.rs:562-580`)
linearly scans every spawn on the map and recomputes `world_to_grid` for each, once per
grid load; same for gameobjects. On Eastern Kingdoms that is ~40k spawns scanned per grid
load × 25 grids per logging-in player.

**Fix.** Build `HashMap<(map_id, cell_id), CellObjectGuids>` at load time (or at minimum
`HashMap<(map_id, grid_x, grid_y), Vec<spawn_idx>>`). Keep the per-cell granularity if
you later want the reference's per-cell loading; per-grid is enough to kill the hot loop.

---

## 8. MEDIUM — Cell-level query geometry is unused

`crates/map/src/grid/cell.rs` exists and cells are populated, but every range query goes
through `GridManager::get_objects_in_range` (`grid.rs:409-429`), which returns **all
objects in every overlapping grid** — up to 9 grids ≈ 1600×1600 yards — and lets the
caller distance-filter (`game/player/visibility/system.rs:255+`).

The reference visits cells, not grids: `Cell::CalculateCellArea` (`CellImpl.h:39-52`)
computes the cell box, and for boxes larger than 4×4 cells `Cell::VisitCircle`
(`CellImpl.h:123-174`) fills a *circumscribed octagon* rather than the full square,
plus it always visits the standing cell first. There is also the `nocreate` flag
(`Cell::SetNoCreate`) which lets a query traverse cells **without** triggering grid
loading — we have no equivalent, so any range query can implicitly activate grids.

**Fix.** Add `GridManager::get_objects_in_cells(center, radius)` implementing
`CalculateCellArea` + the octagon fill, and route visibility/AoE/aggro scans through it.
Add a `no_create: bool` to the query so scans don't cause grid activation.

---

## 9. MEDIUM — `Map` has no identity, no lifecycle, no subclasses

`crates/map/src/map.rs` is a bag of four DashMaps plus a `GridManager`. Absent versus
`Map.h`:

| reference member | what it does | our status |
|---|---|---|
| `MapEntry const* m_mapEntry` | map type, `Instanceable()`, `IsDungeon()`, `IsRaid()`, `IsBattleGround()`, `IsContinent()`, `maxPlayers`, `resetDelay`, `ghostEntrance*`, `linkedZone` | absent (DBC struct exists but `Map` never reads it) |
| `WorldMap` / `DungeonMap` / `BattleGroundMap` | per-type `Add`/`Remove`/`Update`/`CanEnter`/`UnloadAll`/`InitVisibilityDistance` | absent — one concrete `Map` |
| `m_unloadTimer` + `CanUnload` | empty instances unload after `INSTANCE_UNLOAD_DELAY` | absent; maps are created and never destroyed (`MapManager::get_or_create_map`) |
| `m_persistentState` | see §10 | absent |
| `m_data` (`InstanceData`) + `CreateInstanceData` | per-instance script state, `OnPlayerEnter`/`OnPlayerLeave`, `Update` | partially covered by Lua zone scripts, no map hook |
| `m_objectsStore` + `GetCreature/GetGameObject/GetUnit/GetWorldObject` | map-scoped object lookup | we use global managers keyed by GUID — acceptable, but nothing scopes a lookup to a map/instance |
| `GenerateLocalLowGuid` | per-map low-GUID counters for dynobjects/pets/transports | absent |
| `m_mCreatureSummonLimit/Count` + `SetSummonLimitForObject` | caps summons per owner | absent |
| `m_transports` + `UpdateSync` | transports tick **outside** grid activation | we have `game/transport/*`; verify it ticks independently of grids |
| `m_dynamicTree` + `Insert/RemoveGameObjectModel`, `Balance()` | per-map dynamic collision | we have one global `VMapManager` dynamic tree; instances of the same map will share/collide |
| `m_creatureLinkingHolder` | creature linking | `game/coordination/linking_manager.rs` exists |
| `m_objectsToRemove` + `RemoveAllObjectsInRemoveList` | deferred deletion so an object being iterated isn't freed | absent |
| `TeleportAllPlayersTo`, `SendToPlayers(team)`, `SendToPlayersInZone`, `SendDefenseMessage`, `PlayDirectSoundToMap`, `SendMonsterTextToMap` | map-wide broadcast helpers | partially in broadcast manager; no zone-filtered or team-filtered map send |
| `MarkAsCrashed` / `CrashUnload` | isolate a panicking map instead of taking the server down | absent |
| `ShouldUpdateMap(now, inactiveTimeLimit)` | stop ticking maps empty for > `EMPTY_MAPS_UPDATE_TIME`, unless corpses pending (`Map.cpp:3558-3581`) | absent — we tick every map created, forever |

The `dynamic tree per map` point is worth singling out: `Map::InsertGameObjectModel` is
per-map, ours (`gameobject_mgr.add_collision_model(guid, &vmap_mgr)`) is global. Two
instances of Deadmines will see each other's doors.

---

## 10. MEDIUM — `MapPersistentState` has no counterpart

`MapPersistentStateMgr.{h,cpp}` (1642 LoC) owns:

- **Respawn times** per (map, instance): `GetCreatureRespawnTime(loguid)` /
  `SaveCreatureRespawnTime`, likewise for GOs, persisted to the `creature_respawn` /
  `gameobject_respawn` tables and reloaded at startup
  (`LoadCreatureRespawnTimes` / `LoadGameobjectRespawnTimes`).
- **Per-instance spawn overrides** (`AddCreatureToGrid` / `RemoveCreatureFromGrid`).
- **Dungeon binding + reset**: `DungeonPersistentState` (reset time, `CanReset`,
  bound players/groups, `SaveToDB`/`DeleteFromDB`/`DeleteRespawnTimesAndData`) and
  `DungeonResetScheduler` (`LoadResetTimes`, `ScheduleAllDungeonResets`,
  `CalculateNextResetTime`, global raid reset).
- **Pool system** (`InitPools`, `IsSpawnedPoolObject`).

Ours: `CreatureManager::save_respawn_state` (`manager.rs:736`) writes an in-memory
`Instant`, dropped on restart. `game/instance/manager.rs` (971 LoC) covers bindings,
encounter tracking, reset scheduling and `can_enter_instance` — good coverage of the
*binding* half, but nothing for respawn persistence and no per-instance spawn data.

**Fix (scoped).** The high-value slice is respawn persistence: key respawn times by
`(map_id, instance_id, spawn_id)`, flush on grid unload and on shutdown, load at startup
and have `get_spawns_for_grid` skip spawns whose respawn time is in the future. Pools can
wait.

---

## 11. MEDIUM — Server-side area/zone derivation is missing

`TerrainInfo` exposes `GetAreaFlag(x,y,z,&isOutdoors)`, `GetAreaId`, `GetZoneId`,
`GetZoneAndAreaId`, `GetAreaInfo` (mogp flags / adtId / rootId / groupId) and
`IsOutdoors` (`GridMap.h:162-170`), with the flag→id resolution in
`TerrainManager::GetAreaIdByAreaFlag` / `GetZoneIdByAreaFlag` via
`AreaEntry::GetByAreaFlagAndMap` (`GridMap.cpp:1290-1314`) — note the 1.12 quirk that
areaflags are duplicated, so the lookup prefers the entry matching the map id.

We have `TerrainInfo::get_area_flag` (`terrain/manager.rs:75`) and it is **never called**.
`Player::zone_id` is taken from the client's `CMSG_ZONEUPDATE`
(`handlers/character.rs:2514-2536`).

Impact: zone is client-asserted (spoofable), and we cannot answer "is this player
indoors?" — which gates mounts, some spell casts, and the WMO-liquid path. Weather
(`game/weather/*`, currently untracked in git) also keys off zone.

**Fix.** Port `GetAreaFlag` → `AreaEntry` lookup on top of the existing DBC store, add
`is_outdoors` using the WMO group flags from the vmap area info, and derive zone
server-side on relocation, using the client packet only as a cross-check.

---

## 12. LOW/MEDIUM — Map update loop ordering and throttles

Reference `Map::Update` (`Map.cpp:952-1083`) runs a specific sequence, each stage with
its own throttle read from config:

```
m_dynamicTree.update(diff)
UpdateSessionsMovementAndSpellsIfNeeded()   // MAPUPDATE_UPDATE_PACKETS_DIFF
session updates
UpdatePlayers()                             // MAPUPDATE_UPDATE_PLAYERS_DIFF, skips
                                            // inactive players N ticks (INACTIVE_PLAYERS_SKIP_UPDATES)
UpdateCells(diff)                           // MAPUPDATE_UPDATE_CELLS_DIFF
SendObjectUpdates()
UpdateVisibilityForRelocations()            // only units in m_unitsRelocated, with a timeout
UpdateSessionsMovementAndSpellsIfNeeded(); UpdatePlayers()   // second pass
RemoveCorpses(); RemoveOldBones(diff)
grid state machine pass
UpdateScriptedEvents() (1s); ScriptsProcess(); InstanceData::Update
m_weatherSystem->UpdateWeathers(diff)
visibility/activation distance ramp
```

Notable pieces we lack:
- `UpdatePlayers` inactive-player skipping (players not in combat, no recent spell
  packets, no scheduled events accumulate `skippedUpdateTime` and are updated in batch).
- `m_unitsRelocated` — visibility is recomputed only for units that actually moved, with
  a per-tick time budget (`MAP_VISIBILITYUPDATE_TIMEOUT`) and leftover work carried to
  the next tick. Ours re-runs on a 4-tick throttle for *every* player
  (`player/visibility/system.rs:22`), with a dirty flag but no time budget.
- `RemoveCorpses` / `RemoveOldBones` are **map-owned** and bone creation is gated on
  `!IsRemovalGrid(corpse pos)` (`Map.cpp:3605-3715`). We have `game/corpse` but no
  grid-state gate — bones can be created in a grid that is about to unload.
- Second `UpdatePlayers()` pass after visibility, which is what makes movement feel
  responsive at high population.

Our `update_all_maps` (`game/map_update.rs`) is two loops (visibility, then async player
update) and everything else lives in the global `World::update` (`world.rs:466-560`),
so per-map isolation (and therefore per-map load shedding, crash isolation, and the
"skip empty maps" rule) is not expressible.

---

## 13. LOW — Assorted map query helpers not ported

From `Map.h:559-577` / `Map.cpp:3135-3470`, used widely by AI and spell code:

| helper | purpose |
|---|---|
| `GetLosHitPosition` | first collision point along a ray — knockback/charge destination clamping |
| `GetWalkHitPosition` | navmesh raycast with `moveAllowedFlags`, z-search and steep-slope handling |
| `GetWalkRandomPosition` | random reachable point within radius (random movement generator) |
| `GetSwimRandomPosition` | as above for liquid volumes |
| `FindCollisionModel` / `FindDynamicObjectCollisionModel` | which model blocked LoS (door vs terrain) |
| `TerrainInfo::GetWaterOrGroundLevel(pos, &ground, swim)` | the "where do I stand/float" primitive (`GridMap.cpp:1100-1130`) |
| `TerrainInfo::IsSwimmable(x,y,z,radius)` | radius-tolerant swim check |
| `Map::isInLineOfSight(..., checkDynLos, ignoreM2Model)` | note the two flags — dynamic (GO) LoS and M2 handling are separate switches |

We have `is_in_water` / `is_underwater` / `get_water_level` / `get_height_static` and a
`GamePathfinder`; the rest are absent. `GetWaterOrGroundLevel` and `GetWalkRandomPosition`
are the two that block faithful creature movement.

---

## 14. Not applicable / deliberately different

- `ScriptCommands.{h,cpp}` (3756 LoC, 93 DB script commands) — we use Lua. Worth keeping
  a mapping table if DB scripts from the reference dataset are ever imported.
- `ZoneScript` / `ZoneScriptMgr` — covered by `core/lua/zone_executor.rs`.
- Thread pools (`m_objectThreads`, `m_motionThreads`, `m_visibilityThreads`,
  `m_cellThreads`, `MapManager::m_continentThreads`) — our tokio model differs; the
  *logic* inside those callbacks (§5, §12) still needs porting, the threading does not.
- `MapReference` / `MapRefManager` / `GridReference` — intrusive lists replaced by
  DashMap registries.
- `MapManager::GetContinentInstanceId` (instanced continents / "Elysium sharding") —
  not needed unless we shard continents.

---

## Suggested ordering

1. §1 vmap/mmap tile indices — one-line-ish fix, unblocks all collision and pathing.
2. §2 visibility + grid activation distances — restores correct spawn/packet volume.
3. §4 grid membership on relocation — stops creature leaks and phantom despawns.
4. §3 unload guards (`active_objects_near_grid`, unload locks, grid stoper).
5. §7 spawn index — removes the O(all spawns) grid load.
6. §5 marked cells — cuts the per-tick creature cost.
7. §6 active objects, §11 server-side zone, §10 respawn persistence.
8. §8, §9, §12, §13 as follow-ups.
