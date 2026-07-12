//! Cached caster/target pointer refresh (MaNGOS `Spell::UpdateOriginalCasterPointer`
//! and `Spell::UpdatePointers`).
//!
//! A `Spell` caches raw `Unit*`/`GameObject*`/`Item*` target pointers that can
//! go stale during a time delay (`Spell::Delayed`); on the next update they are
//! re-resolved from the matching GUIDs against the live world. Rust has no raw
//! pointers, so the cached pointers are modelled here as `Option<ObjectGuid>`
//! and the re-resolution is performed on demand by the world-coupled entries
//! below. The *decisions* of which lookup branch to take are factored into pure
//! helpers (no world access) so the full branch matrix is unit-testable without
//! a DB.

use crate::game::player::spells::state::SpellCastTargets;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;

// ─── Per-cast state (input/output) ───────────────────────────────────────────

/// Inputs the caller gathers to refresh a cast's cached pointer state. Rust's
/// `ActiveCast` does not carry original-caster fields, so the caller threads
/// them through here instead.
#[derive(Debug, Clone, Copy)]
pub struct CastPointerInput {
    /// `m_caster->GetObjectGuid()` — the caster `WorldObject`'s own guid.
    pub caster_guid: ObjectGuid,
    /// `m_casterUnit` — the `Unit*` the caster exposes. Identical to
    /// `caster_guid` for player / creature / pet casters; `None` for an
    /// unowned GameObject caster (the GO has no owning Unit).
    pub caster_unit_guid: Option<ObjectGuid>,
    /// `m_originalCasterGUID` — the stored original-caster guid whose pointer
    /// we refresh.
    pub original_caster_guid: ObjectGuid,
}

impl CastPointerInput {
    /// Convenience for player / creature / pet casters, where `m_caster` and
    /// `m_casterUnit` share the same guid.
    pub fn for_unit(caster_guid: ObjectGuid, original_caster_guid: ObjectGuid) -> Self {
        Self {
            caster_guid,
            caster_unit_guid: Some(caster_guid),
            original_caster_guid,
        }
    }
}

/// Refreshed cached pointer state for one cast. Each `Option<ObjectGuid>` is
/// the Rust equivalent of the C++ cached pointer (`None` ↔ `nullptr`), plus
/// two seam flags recording that both sub-steps of `Spell::UpdatePointers`
/// ran in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PointerRefresh {
    /// Resolved `m_originalCaster`.
    pub original_caster: Option<ObjectGuid>,
    /// Resolved `m_unitTarget`.
    pub unit_target: Option<ObjectGuid>,
    /// Resolved `m_GOTarget`.
    pub go_target: Option<ObjectGuid>,
    /// Resolved `m_itemTarget` (only re-resolved when the caster is a Player).
    pub item_target: Option<ObjectGuid>,
    /// Set once the original-caster refresh step ran (seam flag for tests).
    pub original_caster_refreshed: bool,
    /// Set once the target re-resolution step ran after the original-caster
    /// refresh (seam flag for tests).
    pub targets_refreshed: bool,
}

// ─── Pure decision logic ────────────────────────────────────────────────────

/// Which resolution branch `Spell::UpdateOriginalCasterPointer` takes.
///
/// Carrying the guid in the `GameObjectLookup` / `UnitLookup` variants keeps
/// the branch selection pure: the world lookup is performed later, against
/// this decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalCasterResolution {
    /// `m_originalCasterGUID == m_caster->GetObjectGuid()` — the cached
    /// original-caster pointer is simply `m_casterUnit`.
    SameAsCaster,
    /// `m_originalCasterGUID.IsGameObject()` — look the GO up via the caster's
    /// map, then read its owner.
    GameObjectLookup(ObjectGuid),
    /// Otherwise — `ObjectAccessor::GetUnit(*m_caster, guid)` then validate
    /// `unit->IsInWorld()`.
    UnitLookup(ObjectGuid),
}

/// Pure branch selector for `Spell::UpdateOriginalCasterPointer`. Identifies
/// which lookup branch to take from the stored GUIDs alone, with no world
/// access.
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

/// A world-free snapshot of the GUIDs `m_targets.Update(m_caster)` re-resolves.
/// Held separately from `SpellCastTargets` so the pure helpers stay
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

/// Pure transform of `m_targets.Update(m_caster)`: collects the stored GUIDs
/// that need re-resolution (i.e. those present on the snapshot). Stale
/// (`None`) GUIDs are preserved as `None` — equivalent to the C++ cached
/// pointer remaining `nullptr`.
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

/// Resolve a unit guid against the world (`ObjectAccessor::GetUnit`
/// equivalent). Returns `Some(guid)` when the unit is present in its manager,
/// which doubles as the MaNGOS `IsInWorld()` check for units — a guid not in
/// its manager is treated as out-of-world and yields `None`.
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

/// Approximation of `m_caster->IsInWorld()` used by the GameObject branch.
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

/// `Spell::UpdateOriginalCasterPointer` — world-coupled resolution of the
/// cached original-caster pointer.
///
/// Returns the resolved original-caster guid (`None` when the C++ pointer
/// would be `null` — branch 2 with a caster not in world, a GO not present, or
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

/// `m_targets.Update` `m_unitTarget` resolution. MaNGOS uses `pCaster`
/// directly when the stored unit-target GUID matches the caster (no map
/// lookup needed), else `ObjectAccessor::GetUnit`.
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

/// `m_targets.Update` `m_GOTarget` resolution via the game-object manager
/// (the stand-in for `m_caster->GetMap()->GetGameObject(guid)`).
fn resolve_go_target(go_target_guid: ObjectGuid, world: &World) -> Option<ObjectGuid> {
    world
        .managers
        .gameobject_mgr
        .with_gameobject(go_target_guid, |_| ())
        .map(|()| go_target_guid)
}

/// `Spell::UpdatePointers` — first refresh the cached original-caster pointer
/// via [`update_original_caster_pointer`], then re-resolve the stored unit /
/// GO / item target GUIDs (the C++ `m_targets.Update(m_caster)` step).
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
        .and_then(|g| resolve_go_target(g, world));
    if input.caster_guid.is_player() {
        // `m_itemTarget` is only re-resolved when the caster is a Player. The
        // cached item pointer is the stored GUID itself; full item resolution
        // (player inventory / trade-frame accessor) is not wired through the
        // manager `with_*` lookups yet.
        refresh.item_target = snapshot.item_target;
    }
    refresh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::game::gameobject::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::player::player::Player;
    use crate::World;
    use oxcore_shared::database::Databases;
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
            logs: lazy_pool(),
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
        let transport = ObjectGuid::new_without_entry(
            oxcore_shared::protocol::HighGuid::Transport,
            9,
        );
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
        assert_eq!(
            update_original_caster_pointer(&input, &world),
            Some(owner)
        );
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

    // ── `Spell::UpdatePointers` (both steps, in order) ─────────────────────

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

        assert!(refresh.original_caster_refreshed, "original-caster step ran");
        assert!(refresh.targets_refreshed, "targets step ran after original caster");
        assert_eq!(refresh.original_caster, Some(caster));
        assert_eq!(refresh.unit_target, Some(caster));
        assert!(refresh.go_target.is_none());
        assert!(refresh.item_target.is_none());
    }

    #[tokio::test]
    async fn update_pointers_resolves_registered_go_target() {
        let world = test_world();
        let caster = ObjectGuid::new_player(30);
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
}