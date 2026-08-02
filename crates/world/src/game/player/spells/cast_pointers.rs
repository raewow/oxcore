//! Cached caster/target pointer refresh.
//!
//! A spell cast caches target references as GUIDs; during a delayed cast they
//! may need re-resolving from the live world. The *decisions* of which lookup
//! branch to take are factored into pure helpers (no world access) so the full
//! branch matrix is unit-testable without a DB.

use crate::game::player::spells::state::{CurrentSpellType, SpellCastTargets, SpellState};
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};

// ─── Per-cast state (input/output) ───────────────────────────────────────────

/// Inputs the caller gathers to refresh a cast's cached pointer state.
#[derive(Debug, Clone, Copy)]
pub struct CastPointerInput {
    /// The caster's own GUID.
    pub caster_guid: ObjectGuid,
    /// The unit-level caster GUID. Identical to
    /// `caster_guid` for player / creature / pet casters; `None` for an
    /// unowned GameObject caster (the GO has no owning Unit).
    pub caster_unit_guid: Option<ObjectGuid>,
    /// The stored original-caster GUID to refresh.
    pub original_caster_guid: ObjectGuid,
}

impl CastPointerInput {
    /// Convenience for player / creature / pet casters, where the caster and
    /// caster-unit guids are the same.
    pub fn for_unit(caster_guid: ObjectGuid, original_caster_guid: ObjectGuid) -> Self {
        Self {
            caster_guid,
            caster_unit_guid: Some(caster_guid),
            original_caster_guid,
        }
    }
}

/// Refreshed cached pointer state for one cast. Each `Option<ObjectGuid>` is
/// a resolved target, plus two seam flags recording that both sub-steps
/// ran in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PointerRefresh {
    /// Resolved original caster.
    pub original_caster: Option<ObjectGuid>,
    /// Resolved unit target.
    pub unit_target: Option<ObjectGuid>,
    /// Resolved game object target.
    pub go_target: Option<ObjectGuid>,
    /// Resolved item target (only re-resolved when the caster is a Player).
    pub item_target: Option<ObjectGuid>,
    /// Set once the original-caster refresh step ran (seam flag for tests).
    pub original_caster_refreshed: bool,
    /// Set once the target re-resolution step ran after the original-caster
    /// refresh (seam flag for tests).
    pub targets_refreshed: bool,
}

// ─── Pure decision logic ────────────────────────────────────────────────────

/// Which resolution branch the original-caster refresh takes.
///
/// Carrying the guid in the `GameObjectLookup` / `UnitLookup` variants keeps
/// the branch selection pure: the world lookup is performed later, against
/// this decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalCasterResolution {
    /// `original_caster_guid == caster_guid` — the cached
    /// original-caster pointer is simply the caster unit.
    SameAsCaster,
    /// The original caster is a game object — look it up via the caster's
    /// map, then read its owner.
    GameObjectLookup(ObjectGuid),
    /// Otherwise — look up the unit and validate it is in the world.
    UnitLookup(ObjectGuid),
}

/// Identifies which lookup branch to take from the stored GUIDs alone, with
/// no world access.
pub fn resolve_original_caster_branch(
    original_caster_guid: ObjectGuid,
    caster_guid: ObjectGuid,
) -> OriginalCasterResolution {
    if original_caster_guid == caster_guid {
        OriginalCasterResolution::SameAsCaster
    } else if original_caster_guid.is_game_object() {
        OriginalCasterResolution::GameObjectLookup(original_caster_guid)
    } else {
        OriginalCasterResolution::UnitLookup(original_caster_guid)
    }
}

/// A world-free snapshot of the GUIDs that get re-resolved during a pointer
/// refresh. Held separately from `SpellCastTargets` so the pure helpers stay
/// world-independent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TargetGuidSnapshot {
    pub unit_target: Option<ObjectGuid>,
    pub go_target: Option<ObjectGuid>,
    pub item_target: Option<ObjectGuid>,
    pub corpse_target: Option<ObjectGuid>,
}

impl TargetGuidSnapshot {
    pub fn from_cast_targets(targets: &SpellCastTargets) -> Self {
        Self {
            unit_target: targets.unit_target_guid,
            go_target: targets.gameobject_target_guid,
            item_target: targets.item_target_guid,
            corpse_target: targets.corpse_target_guid,
        }
    }
}

/// Collects the stored GUIDs that need re-resolution (i.e. those present on
/// the snapshot). Stale (`None`) GUIDs are preserved as `None`.
pub fn target_guids_needing_resolution(snapshot: &TargetGuidSnapshot) -> Vec<ObjectGuid> {
    let mut out = Vec::new();
    if let Some(g) = snapshot.unit_target {
        out.push(g);
    }
    if let Some(g) = snapshot.go_target {
        out.push(g);
    }
    if let Some(g) = snapshot.item_target {
        out.push(g);
    }
    if let Some(g) = snapshot.corpse_target {
        out.push(g);
    }
    out
}

// ─── World-coupled lookups ───────────────────────────────────────────────────

/// Resolve a unit guid against the world. Returns `Some(guid)` when the unit
/// is present in its manager; a guid not in its manager is treated as
/// out-of-world and yields `None`.
fn lookup_unit_by_guid(guid: ObjectGuid, world: &World) -> Option<ObjectGuid> {
    if guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(guid, |_| ())
            .map(|()| guid)
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |_| ())
            .map(|()| guid)
    } else {
        None
    }
}

/// Whether the caster is in-world, used by the GameObject branch.
/// Players and creatures count as in-world while present in their managers;
/// a gameobject caster additionally requires its `in_world` flag to be set.
fn is_caster_in_world(caster_guid: ObjectGuid, world: &World) -> bool {
    if caster_guid.is_player() {
        world
            .systems
            .player
            .manager()
            .with_player(caster_guid, |_| ())
            .is_some()
    } else if caster_guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |_| ())
            .is_some()
    } else if caster_guid.is_game_object() {
        world
            .managers
            .gameobject_mgr
            .with_gameobject(caster_guid, |go| go.in_world)
            .unwrap_or(false)
    } else {
        false
    }
}

/// Finds a game object through the caster's map. The global manager can retain
/// objects from other maps and objects no longer placed in the world.
fn lookup_gameobject_on_caster_map(
    caster_guid: ObjectGuid,
    gameobject_guid: ObjectGuid,
    world: &World,
) -> Option<ObjectGuid> {
    let caster_map_id = if caster_guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(caster_guid, |player| player.map_id)
    } else if caster_guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(caster_guid, |creature| creature.map_id)
    } else if caster_guid.is_game_object() {
        world
            .managers
            .gameobject_mgr
            .with_gameobject(caster_guid, |gameobject| {
                gameobject.in_world.then_some(gameobject.map_id)
            })
            .flatten()
    } else {
        None
    }?;

    world
        .managers
        .gameobject_mgr
        .with_gameobject(gameobject_guid, |gameobject| {
            (gameobject.in_world && gameobject.map_id == caster_map_id).then_some(gameobject_guid)
        })
        .flatten()
}

/// World-coupled resolution of the cached original-caster pointer.
///
/// Returns the resolved original-caster guid (`None` when the caster is not
/// in world, a GO is not present, or
/// an unowned / out-of-world owner; branch 3 with a missing unit).
pub fn update_original_caster_pointer(
    input: &CastPointerInput,
    world: &World,
) -> Option<ObjectGuid> {
    match resolve_original_caster_branch(input.original_caster_guid, input.caster_guid) {
        OriginalCasterResolution::SameAsCaster => input.caster_unit_guid,
        OriginalCasterResolution::GameObjectLookup(go_guid) => {
            if !is_caster_in_world(input.caster_guid, world) {
                return None;
            }
            let owner_guid = world
                .managers
                .gameobject_mgr
                .with_gameobject(go_guid, |go| go.created_by)
                .filter(|g| !g.is_empty() && g.is_unit())?;
            lookup_unit_by_guid(owner_guid, world)
        }
        OriginalCasterResolution::UnitLookup(unit_guid) => lookup_unit_by_guid(unit_guid, world),
    }
}

/// Resolve the unit target from its GUID. Uses
/// the caster directly when the stored unit-target GUID matches the caster
/// (no map lookup needed).
fn resolve_unit_target(
    unit_target_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    world: &World,
) -> Option<ObjectGuid> {
    if unit_target_guid == caster_guid {
        return Some(caster_guid);
    }
    lookup_unit_by_guid(unit_target_guid, world)
}

/// Resolve the game object target from its GUID through the caster's map.
fn resolve_go_target(
    go_target_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    world: &World,
) -> Option<ObjectGuid> {
    lookup_gameobject_on_caster_map(caster_guid, go_target_guid, world)
}

/// First refresh the cached original-caster pointer
/// via [`update_original_caster_pointer`], then re-resolve the stored unit /
/// GO / item target GUIDs.
pub fn update_pointers(
    input: &CastPointerInput,
    targets: &SpellCastTargets,
    world: &World,
) -> PointerRefresh {
    let original_caster = update_original_caster_pointer(input, world);
    let mut refresh = PointerRefresh {
        original_caster,
        original_caster_refreshed: true,
        ..Default::default()
    };

    refresh.targets_refreshed = true;
    let snapshot = TargetGuidSnapshot::from_cast_targets(targets);
    refresh.unit_target = snapshot
        .unit_target
        .and_then(|g| resolve_unit_target(g, input.caster_guid, world));
    refresh.go_target = snapshot
        .go_target
        .and_then(|g| resolve_go_target(g, input.caster_guid, world));
    if input.caster_guid.is_player() {
        // The item target is only re-resolved when the caster is a Player. The
        // cached item pointer is the stored GUID itself; full item resolution
        // (player inventory / trade-frame accessor) is not wired through the
        // manager `with_*` lookups yet.
        refresh.item_target = snapshot.item_target;
    }
    refresh
}

// ─── Spell slot classification ───────────────────────────────────────────────

/// Maps a cast to one of the four `CurrentSpellTypes` slots via the fixed
/// priority chain (melee → auto-repeat → channeled → generic).
///
/// * `is_next_melee_swing` — whether the spell is a next-melee-swing spell
/// * `is_auto_repeat` — whether auto-repeat is active
/// * `is_channeled` — whether the cast is a channeled spell
/// * `cast_time_ms` — the cast time in milliseconds
pub fn current_container(
    is_next_melee_swing: bool,
    is_auto_repeat: bool,
    is_channeled: bool,
    cast_time_ms: u32,
    is_triggered_spell: bool,
    spell_state: SpellState,
) -> CurrentSpellType {
    if is_next_melee_swing {
        CurrentSpellType::Melee
    } else if is_auto_repeat {
        CurrentSpellType::Autorepeat
    } else if is_channeled
        && (cast_time_ms == 0 || is_triggered_spell || spell_state == SpellState::Casting)
    {
        CurrentSpellType::Channeled
    } else {
        CurrentSpellType::Generic
    }
}

// ─── Effective-source accessors ──────────────────────────────────────────────

/// Resolves the effective source of the cast's *effects* (explicit caster,
/// DoT/HoT applier, GO owner, or wild GO).
///
/// * empty original-caster GUID → the formal caster;
/// * a game-object original caster while the caster is in-world → that GO looked
///   up on the caster's map;
/// * otherwise → the cached original-caster unit (re-derived via
///   [`update_original_caster_pointer`]).
///
/// Returns `None` when the resolved object cannot be found.
pub fn get_affective_caster_object(input: &CastPointerInput, world: &World) -> Option<ObjectGuid> {
    if input.original_caster_guid.is_empty() {
        return Some(input.caster_guid);
    }
    if input.original_caster_guid.is_game_object() && is_caster_in_world(input.caster_guid, world) {
        return lookup_gameobject_on_caster_map(
            input.caster_guid,
            input.original_caster_guid,
            world,
        );
    }
    update_original_caster_pointer(input, world)
}

/// Delegates to [`get_affective_caster_object`] (the notifier uses this
/// when no explicit original caster is supplied).
pub fn get_affective_object(input: &CastPointerInput, world: &World) -> Option<ObjectGuid> {
    get_affective_caster_object(input, world)
}

/// Resolves the cast's *visual / casting* object.
///
/// When the original caster is a game object it returns that GO looked up on
/// the caster's map (only while the caster is in-world, else `None`); for every
/// other GUID it returns the formal caster unconditionally.
pub fn get_casting_object(input: &CastPointerInput, world: &World) -> Option<ObjectGuid> {
    if input.original_caster_guid.is_game_object() {
        if is_caster_in_world(input.caster_guid, world) {
            lookup_gameobject_on_caster_map(input.caster_guid, input.original_caster_guid, world)
        } else {
            None
        }
    } else {
        Some(input.caster_guid)
    }
}

/// Records the caster position at cast start.
///
/// When the caster is on a transport (`transport_position` is `Some`), uses
/// transport-relative offsets; otherwise uses the world `position`.
pub fn update_cast_start_position(
    position: Position,
    transport_position: Option<Position>,
) -> Position {
    transport_position.unwrap_or(position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::gameobject::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::player::player::Player;
    use crate::World;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::Position;
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: oxcore_db::database::lazy_logs_pool(),
        });
        World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn add_test_player(world: &World, guid: ObjectGuid, map_id: u32, instance_id: u32) {
        let player = Player::new(
            guid,
            format!("P{}", guid.counter()),
            map_id,
            instance_id,
            0,
            60,
            1,
            1,
            0,
        );
        world.managers.player_mgr.add_player(player, guid.counter());
    }

    fn make_go(guid: ObjectGuid, map_id: u32, created_by: ObjectGuid) -> GameObject {
        let template = GameObjectTemplate {
            entry: 1,
            go_type: 0,
            display_id: 0,
            name: "test".to_string(),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data: [0; 24],
        };
        let mut go = GameObject::new(
            guid,
            1,
            1,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
            },
            map_id,
            &template,
            [0.0; 4],
            0,
            0,
        );
        go.in_world = true;
        go.created_by = created_by;
        go
    }

    // ── update_cast_start_position ────────────────────────────────────────

    #[test]
    fn cast_start_position_without_transport_uses_world_position() {
        let world_pos = Position::new(10.0, 20.0, 30.0, 1.5);
        let result = update_cast_start_position(world_pos, None);
        assert_eq!(result, world_pos);
    }

    #[test]
    fn cast_start_position_with_transport_uses_transport_offset() {
        let world_pos = Position::new(10.0, 20.0, 30.0, 1.5);
        let transport_pos = Position::new(1.0, 2.0, 3.0, 0.5);
        let result = update_cast_start_position(world_pos, Some(transport_pos));
        assert_eq!(result, transport_pos);
    }

    // ── Pure branch selection ──────────────────────────────────────────────

    #[test]
    fn branch_when_original_equals_caster() {
        let caster = ObjectGuid::new_player(1);
        assert_eq!(
            resolve_original_caster_branch(caster, caster),
            OriginalCasterResolution::SameAsCaster
        );
    }

    #[test]
    fn branch_when_original_is_game_object_guid() {
        let caster = ObjectGuid::new_player(1);
        let go = ObjectGuid::new_gameobject(42, 7);
        assert_eq!(
            resolve_original_caster_branch(go, caster),
            OriginalCasterResolution::GameObjectLookup(go)
        );
        // Transport guids count as game objects too (they share the GO lookup).
        let transport =
            ObjectGuid::new_without_entry(oxcore_shared::protocol::HighGuid::Transport, 9);
        assert_eq!(
            resolve_original_caster_branch(transport, caster),
            OriginalCasterResolution::GameObjectLookup(transport)
        );
    }

    #[test]
    fn branch_when_original_is_creature_guid() {
        let caster = ObjectGuid::new_player(1);
        let creature = ObjectGuid::new_creature(33, 4);
        assert_eq!(
            resolve_original_caster_branch(creature, caster),
            OriginalCasterResolution::UnitLookup(creature)
        );
    }

    #[test]
    fn branch_when_original_is_player_guid() {
        let caster = ObjectGuid::new_creature(1, 1);
        let other = ObjectGuid::new_player(2);
        assert_eq!(
            resolve_original_caster_branch(other, caster),
            OriginalCasterResolution::UnitLookup(other)
        );
    }

    #[test]
    fn branch_when_original_is_pet_guid() {
        let caster = ObjectGuid::new_player(1);
        let pet = ObjectGuid::new_pet(17, 5);
        assert_eq!(
            resolve_original_caster_branch(pet, caster),
            OriginalCasterResolution::UnitLookup(pet)
        );
    }

    #[test]
    fn target_resolution_collects_present_guids_only() {
        let snap = TargetGuidSnapshot {
            unit_target: Some(ObjectGuid::new_player(1)),
            go_target: None,
            item_target: Some(ObjectGuid::new_item(2)),
            corpse_target: Some(ObjectGuid::new_corpse(3)),
        };
        let need = target_guids_needing_resolution(&snap);
        assert_eq!(need.len(), 3);
        assert!(need.contains(&ObjectGuid::new_player(1)));
        assert!(need.contains(&ObjectGuid::new_item(2)));
        assert!(need.contains(&ObjectGuid::new_corpse(3)));
    }

    // ── World-coupled original-caster refresh ──────────────────────────────

    #[tokio::test]
    async fn original_caster_same_as_caster_returns_caster_unit() {
        let world = test_world();
        let caster = ObjectGuid::new_player(7);
        let input = CastPointerInput::for_unit(caster, caster);
        assert_eq!(update_original_caster_pointer(&input, &world), Some(caster));
    }

    #[tokio::test]
    async fn original_caster_unit_lookup_returns_some_when_registered() {
        let world = test_world();
        let caster = ObjectGuid::new_creature(1, 1);
        let original = ObjectGuid::new_player(99);
        add_test_player(&world, original, 0, 0);
        let input = CastPointerInput::for_unit(caster, original);
        assert_eq!(
            update_original_caster_pointer(&input, &world),
            Some(original)
        );
    }

    #[tokio::test]
    async fn original_caster_unit_lookup_returns_none_when_not_in_world() {
        let world = test_world();
        let caster = ObjectGuid::new_creature(1, 1);
        let original = ObjectGuid::new_player(99);
        // Not registered → not in world.
        let input = CastPointerInput::for_unit(caster, original);
        assert_eq!(update_original_caster_pointer(&input, &world), None);
    }

    #[tokio::test]
    async fn original_caster_game_object_branch_resolves_owner() {
        let world = test_world();
        let caster = ObjectGuid::new_player(10);
        add_test_player(&world, caster, 0, 0);
        let owner = ObjectGuid::new_player(11);
        add_test_player(&world, owner, 0, 0);
        let go_guid = ObjectGuid::new_gameobject(5, 5);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 0, owner));
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(update_original_caster_pointer(&input, &world), Some(owner));
    }

    #[tokio::test]
    async fn original_caster_game_object_branch_null_when_caster_not_in_world() {
        let world = test_world();
        let caster = ObjectGuid::new_player(10); // not registered
        let go_guid = ObjectGuid::new_gameobject(5, 5);
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(update_original_caster_pointer(&input, &world), None);
    }

    #[tokio::test]
    async fn original_caster_game_object_branch_null_when_go_not_found() {
        let world = test_world();
        let caster = ObjectGuid::new_player(10);
        add_test_player(&world, caster, 0, 0);
        let go_guid = ObjectGuid::new_gameobject(5, 5); // not registered
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(update_original_caster_pointer(&input, &world), None);
    }

    #[tokio::test]
    async fn original_caster_game_object_branch_null_when_owner_not_in_world() {
        let world = test_world();
        let caster = ObjectGuid::new_player(10);
        add_test_player(&world, caster, 0, 0);
        let owner = ObjectGuid::new_player(11); // not registered
        let go_guid = ObjectGuid::new_gameobject(5, 5);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 0, owner));
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(update_original_caster_pointer(&input, &world), None);
    }

    // ── `update_pointers` (both steps, in order) ─────────────────────

    #[tokio::test]
    async fn update_pointers_runs_original_caster_then_targets() {
        let world = test_world();
        let caster = ObjectGuid::new_player(20);
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        // Explicit unit target equal to the caster exercises the pCaster
        // shortcut; no manager lookup should be needed there.
        targets.unit_target_guid = Some(caster);

        let refresh = update_pointers(&input, &targets, &world);

        assert!(
            refresh.original_caster_refreshed,
            "original-caster step ran"
        );
        assert!(
            refresh.targets_refreshed,
            "targets step ran after original caster"
        );
        assert_eq!(refresh.original_caster, Some(caster));
        assert_eq!(refresh.unit_target, Some(caster));
        assert!(refresh.go_target.is_none());
        assert!(refresh.item_target.is_none());
    }

    #[tokio::test]
    async fn update_pointers_resolves_registered_go_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(30);
        add_test_player(&world, caster, 0, 0);
        let go_guid = ObjectGuid::new_gameobject(7, 7);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 0, ObjectGuid::empty()));
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        targets.gameobject_target_guid = Some(go_guid);

        let refresh = update_pointers(&input, &targets, &world);
        assert_eq!(refresh.go_target, Some(go_guid));
        assert!(refresh.targets_refreshed);
    }

    #[tokio::test]
    async fn update_pointers_does_not_resolve_go_target_outside_caster_map() {
        let world = test_world();
        let caster = ObjectGuid::new_player(31);
        add_test_player(&world, caster, 1, 0);
        let go_guid = ObjectGuid::new_gameobject(7, 7);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 2, ObjectGuid::empty()));
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        targets.gameobject_target_guid = Some(go_guid);

        assert_eq!(update_pointers(&input, &targets, &world).go_target, None);
    }

    #[tokio::test]
    async fn update_pointers_keeps_item_guid_for_player_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_player(40);
        let item_guid = ObjectGuid::new_item(1);
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        targets.item_target_guid = Some(item_guid);

        let refresh = update_pointers(&input, &targets, &world);
        assert_eq!(refresh.item_target, Some(item_guid));
    }

    #[tokio::test]
    async fn update_pointers_skips_item_target_for_non_player_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_creature(1, 50);
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        targets.item_target_guid = Some(ObjectGuid::new_item(1));

        let refresh = update_pointers(&input, &targets, &world);
        assert!(
            refresh.item_target.is_none(),
            "non-player casters do not re-resolve item targets"
        );
    }

    #[tokio::test]
    async fn update_pointers_unit_target_pcaster_shortcut_avoids_lookup() {
        let world = test_world();
        // Caster is a player that is NOT registered; only the pCaster
        // shortcut should make the unit target resolve to the caster guid.
        let caster = ObjectGuid::new_player(60);
        let input = CastPointerInput::for_unit(caster, caster);
        let mut targets = SpellCastTargets::default();
        targets.unit_target_guid = Some(caster);

        let refresh = update_pointers(&input, &targets, &world);
        assert_eq!(refresh.unit_target, Some(caster));
    }

    // ── `current_container` (pure priority chain) ─────────────────

    #[test]
    fn container_melee_wins_over_everything() {
        // Even if every other predicate is set, melee has top priority.
        assert_eq!(
            current_container(true, true, true, 0, true, SpellState::Casting),
            CurrentSpellType::Melee
        );
    }

    #[test]
    fn container_auto_repeat_when_not_melee() {
        assert_eq!(
            current_container(false, true, true, 0, false, SpellState::Preparing),
            CurrentSpellType::Autorepeat
        );
    }

    #[test]
    fn container_channeled_requires_compound_condition() {
        // Channeled + instant (no cast time) → channeled.
        assert_eq!(
            current_container(false, false, true, 0, false, SpellState::Preparing),
            CurrentSpellType::Channeled
        );
        // Channeled + triggered → channeled even with a cast time.
        assert_eq!(
            current_container(false, false, true, 1500, true, SpellState::Preparing),
            CurrentSpellType::Channeled
        );
        // Channeled + already in the casting state → channeled.
        assert_eq!(
            current_container(false, false, true, 1500, false, SpellState::Casting),
            CurrentSpellType::Channeled
        );
    }

    #[test]
    fn container_channeled_with_cast_time_falls_through_to_generic() {
        // Channeled but non-instant, not triggered, not yet casting → generic.
        assert_eq!(
            current_container(false, false, true, 1500, false, SpellState::Preparing),
            CurrentSpellType::Generic
        );
    }

    #[test]
    fn container_generic_default() {
        assert_eq!(
            current_container(false, false, false, 1500, false, SpellState::Preparing),
            CurrentSpellType::Generic
        );
    }

    // ── Effective-source accessors ─────────────────────────────────────────

    #[tokio::test]
    async fn affective_caster_empty_original_returns_caster() {
        let world = test_world();
        let caster = ObjectGuid::new_player(70);
        let input = CastPointerInput {
            caster_guid: caster,
            caster_unit_guid: Some(caster),
            original_caster_guid: ObjectGuid::empty(),
        };
        assert_eq!(get_affective_caster_object(&input, &world), Some(caster));
        // GetAffectiveObject delegates to the same resolution.
        assert_eq!(get_affective_object(&input, &world), Some(caster));
    }

    #[tokio::test]
    async fn affective_caster_go_original_in_world_returns_go() {
        let world = test_world();
        let caster = ObjectGuid::new_player(71);
        add_test_player(&world, caster, 0, 0);
        let go_guid = ObjectGuid::new_gameobject(8, 8);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 0, ObjectGuid::empty()));
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(get_affective_caster_object(&input, &world), Some(go_guid));
    }

    #[tokio::test]
    async fn affective_caster_go_original_caster_not_in_world_is_none() {
        let world = test_world();
        // Caster not registered → not in world → falls through to original_caster,
        // which for a GO guid with an out-of-world caster is null.
        let caster = ObjectGuid::new_player(72);
        let go_guid = ObjectGuid::new_gameobject(8, 8);
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(get_affective_caster_object(&input, &world), None);
    }

    #[tokio::test]
    async fn affective_caster_unit_original_returns_cached_unit() {
        let world = test_world();
        let caster = ObjectGuid::new_creature(1, 5);
        let original = ObjectGuid::new_player(73);
        add_test_player(&world, original, 0, 0);
        let input = CastPointerInput::for_unit(caster, original);
        assert_eq!(get_affective_caster_object(&input, &world), Some(original));
    }

    #[tokio::test]
    async fn casting_object_non_go_returns_caster_without_world_check() {
        let world = test_world();
        // Player caster not registered; non-GO original guid → returns caster
        // unconditionally (no in-world guard on this path).
        let caster = ObjectGuid::new_player(74);
        let input = CastPointerInput::for_unit(caster, ObjectGuid::new_player(75));
        assert_eq!(get_casting_object(&input, &world), Some(caster));
    }

    #[tokio::test]
    async fn casting_object_go_in_world_returns_go_else_none() {
        let world = test_world();
        let caster = ObjectGuid::new_player(76);
        add_test_player(&world, caster, 0, 0);
        let go_guid = ObjectGuid::new_gameobject(9, 9);
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(make_go(go_guid, 0, ObjectGuid::empty()));
        let input = CastPointerInput::for_unit(caster, go_guid);
        assert_eq!(get_casting_object(&input, &world), Some(go_guid));

        // Same GO original but caster not in world → None.
        let offline = ObjectGuid::new_player(77);
        let input2 = CastPointerInput::for_unit(offline, go_guid);
        assert_eq!(get_casting_object(&input2, &world), None);
    }
}
