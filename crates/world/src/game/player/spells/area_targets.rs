//! Area target gathering for area-effect spells.
//!
//! Area target gathering for area-effect spells, driven by the
//! `SpellNotifierCreatureAndPlayer` policy. Given a radius, a
//! *push-type* (how the search region is anchored and shaped relative to the
//! caster / source point / destination point / explicit target) and a
//! *target-mask* (which relationship a candidate must have with the caster), it
//! gathers the matching units on the caster's map into a target list.
//!
//! The grid iteration is provided by the map's spatial index
//! ([`crate::World`] -> `Map::get_objects_in_range`); the visitation
//! body (center resolution, mask filter, per-push geometry gate, append) is
//! expressed as small pure helpers so it can be unit-tested against synthetic
//! candidate sets without a live world.

use crate::game::player::spells::state::SpellCastTargets;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::f32::consts::PI;

/// How the area search region is anchored and shaped relative to the caster.
///
/// Center-anchored variants gather every unit
/// within `radius` of a fixed point; [`SpellNotifyPushType::Cone`] instead keeps
/// units inside a frontal arc of the caster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellNotifyPushType {
    /// No supported anchor; nothing is gathered.
    None,
    /// Frontal cone centered on the caster's facing.
    Cone,
    /// Sphere centered on the caster itself.
    SelfCenter,
    /// Sphere centered on the spell's source point.
    SrcCenter,
    /// Sphere centered on the spell's destination point.
    DestCenter,
    /// Sphere centered on the explicit unit target.
    TargetCenter,
}

/// Which relationship a candidate must have with the caster to be included.
///
/// Mirrors `SpellTargets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellTargets {
    /// Everything in range (subject only to the alive/geometry gates).
    All,
    /// Only units hostile to the caster.
    Hostile,
    /// Everything except units friendly to the caster.
    NotFriendly,
    /// Everything except units hostile to the caster.
    NotHostile,
    /// Only units friendly to the caster.
    Friendly,
    /// AoE damage rule: player-controlled casters hit anything not friendly,
    /// while non-player casters hit only what is hostile to them.
    AoeDamage,
}

/// Default cone arc for [`SpellNotifyPushType::Cone`].
///
/// The notifier's cone case calls the in-front check with the default arc of
/// `M_PI` (a 180-degree frontal hemisphere).
pub const DEFAULT_CONE_ARC: f32 = PI;

/// An abstracted unit candidate, mirroring the fields the visitation
/// callback reads from each grid entry. Keeping this world-free lets the filter
/// policy be exercised directly in tests.
#[derive(Debug, Clone, Copy)]
pub struct AreaCandidate {
    pub guid: ObjectGuid,
    pub position: Position,
    pub is_alive: bool,
    pub is_hostile_to_caster: bool,
    pub is_friendly_to_caster: bool,
}

/// Resolved inputs for the pure gathering policy.
#[derive(Debug, Clone, Copy)]
pub struct AreaSearchParams {
    pub push_type: SpellNotifyPushType,
    pub targets: SpellTargets,
    pub radius: f32,
    /// Anchor point for the center-based push types (already resolved).
    pub center: Position,
    /// Caster position (with orientation in `o`), used by the cone geometry.
    pub caster_position: Position,
    /// Whether the effective caster is player-controlled (affects `AoeDamage`).
    pub caster_is_player_controlled: bool,
    /// Whether the spell permits dead candidates.
    pub allow_dead_target: bool,
}

/// 2D (XY) inclusive range test: is `pos` within `radius` of `center`?
pub fn is_within_dist_2d(center: Position, pos: Position, radius: f32) -> bool {
    let dx = pos.x - center.x;
    let dy = pos.y - center.y;
    dx * dx + dy * dy <= radius * radius
}

/// Normalize an angle into `[-PI, PI]`.
fn normalize_angle(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle < -PI {
        angle += 2.0 * PI;
    }
    angle
}

/// Frontal cone test: `target` is inside the arc of width `arc` centered on the
/// caster's facing and within `radius` (2D).
pub fn is_in_cone(caster: Position, target: Position, radius: f32, arc: f32) -> bool {
    if !is_within_dist_2d(caster, target, radius) {
        return false;
    }
    // A zero-length direction (target on top of caster) is always in front.
    if caster.x == target.x && caster.y == target.y {
        return true;
    }
    let angle_to_target = caster.angle_to(&target);
    let diff = normalize_angle(angle_to_target - caster.o).abs();
    diff <= arc / 2.0
}

/// The target-mask branch of the visitation. Decides whether a candidate
/// with the given relationship flags is eligible for `targets`.
pub fn passes_target_mask(
    targets: SpellTargets,
    candidate: &AreaCandidate,
    caster_is_player_controlled: bool,
) -> bool {
    match targets {
        SpellTargets::All => true,
        SpellTargets::Hostile => candidate.is_hostile_to_caster,
        SpellTargets::NotFriendly => !candidate.is_friendly_to_caster,
        SpellTargets::NotHostile => !candidate.is_hostile_to_caster,
        SpellTargets::Friendly => candidate.is_friendly_to_caster,
        SpellTargets::AoeDamage => {
            if caster_is_player_controlled {
                !candidate.is_friendly_to_caster
            } else {
                candidate.is_hostile_to_caster
            }
        }
    }
}

/// The push-type geometry branch of the visitation. Decides whether a
/// candidate at `candidate_pos` lies inside the search region.
pub fn passes_push_geometry(params: &AreaSearchParams, candidate_pos: Position) -> bool {
    match params.push_type {
        SpellNotifyPushType::SelfCenter
        | SpellNotifyPushType::SrcCenter
        | SpellNotifyPushType::DestCenter
        | SpellNotifyPushType::TargetCenter => {
            is_within_dist_2d(params.center, candidate_pos, params.radius)
        }
        SpellNotifyPushType::Cone => is_in_cone(
            params.caster_position,
            candidate_pos,
            params.radius,
            DEFAULT_CONE_ARC,
        ),
        SpellNotifyPushType::None => false,
    }
}

/// Whether the alive gate admits this candidate.
///
/// The notifier skips dead units unless the spell allows dead targets; the
/// all-targets mask also lets dead (still-in-world) units through.
fn passes_alive_gate(params: &AreaSearchParams, candidate: &AreaCandidate) -> bool {
    candidate.is_alive || params.allow_dead_target || params.targets == SpellTargets::All
}

/// Run every candidate through the alive gate, the target-mask filter and
/// the push-type geometry gate, appending each survivor's GUID to `out`.
pub fn fill_area_targets_from_candidates(
    params: &AreaSearchParams,
    candidates: &[AreaCandidate],
    out: &mut Vec<ObjectGuid>,
) {
    for candidate in candidates {
        if !passes_alive_gate(params, candidate) {
            continue;
        }
        if !passes_target_mask(
            params.targets,
            candidate,
            params.caster_is_player_controlled,
        ) {
            continue;
        }
        if !passes_push_geometry(params, candidate.position) {
            continue;
        }
        out.push(candidate.guid);
    }
}

/// Resolve the search center for a center-based push type from the available
/// anchor points.
///
/// Returns `None` when the anchor a push type needs is unavailable (e.g. a
/// destination-centered search with no destination set), which leaves the target
/// list untouched.
pub fn resolve_center(
    push_type: SpellNotifyPushType,
    caster_position: Position,
    src_position: Option<Position>,
    dest_position: Option<Position>,
    target_position: Option<Position>,
) -> Option<Position> {
    match push_type {
        SpellNotifyPushType::Cone | SpellNotifyPushType::SelfCenter => Some(caster_position),
        SpellNotifyPushType::SrcCenter => src_position.or(Some(caster_position)),
        SpellNotifyPushType::DestCenter => dest_position,
        SpellNotifyPushType::TargetCenter => target_position,
        SpellNotifyPushType::None => None,
    }
}

/// Gather units in an area on the caster's map into `out`, wiring the pure
/// policy above to the live map spatial index.
///
/// `original_caster`, when present, supplies the position and relationship
/// reference used in place of `caster_guid` (matching the effective-caster
/// substitution).
///
/// Relationship resolution here reuses the coarse creature/player heuristic that
/// the rest of the spell target code relies on ([`caster_relation`]); it is a
/// placeholder pending faction-template support and is intentionally the same
/// approximation used by the target-resolution pipeline.
#[allow(clippy::too_many_arguments)]
pub fn fill_area_targets(
    world: &World,
    caster_guid: ObjectGuid,
    cast_targets: &SpellCastTargets,
    radius: f32,
    push_type: SpellNotifyPushType,
    targets: SpellTargets,
    original_caster: Option<ObjectGuid>,
    allow_dead_target: bool,
    out: &mut Vec<ObjectGuid>,
) {
    let effective_caster = original_caster.unwrap_or(caster_guid);
    let caster_position = unit_position(effective_caster, world);

    let src_position = cast_targets
        .src_position
        .map(|(x, y, z)| Position { x, y, z, o: 0.0 });
    let dest_position = cast_targets
        .dst_position
        .map(|(x, y, z)| Position { x, y, z, o: 0.0 });
    let target_position = cast_targets
        .unit_target()
        .map(|guid| unit_position(guid, world));

    let center = match resolve_center(
        push_type,
        caster_position,
        src_position,
        dest_position,
        target_position,
    ) {
        Some(center) => center,
        None => return,
    };

    let params = AreaSearchParams {
        push_type,
        targets,
        radius,
        center,
        caster_position,
        caster_is_player_controlled: effective_caster.is_player(),
        allow_dead_target,
    };

    // Grid visitation: gather the player and creature entries around the center,
    // then materialize the unit candidates the visitation would have seen.
    let map = match unit_map_location(effective_caster, world) {
        Some((map_id, instance_id)) => world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id),
        None => return,
    };

    let mut nearby = map.get_players_in_range(center, radius);
    map.get_creatures_in_range(center, radius * radius, &mut nearby);
    let mut candidates = Vec::with_capacity(nearby.len());
    for guid in nearby {
        let (hostile, friendly) = caster_relation(effective_caster, guid);
        candidates.push(AreaCandidate {
            guid,
            position: unit_position(guid, world),
            is_alive: unit_is_alive(guid, world),
            is_hostile_to_caster: hostile,
            is_friendly_to_caster: friendly,
        });
    }

    fill_area_targets_from_candidates(&params, &candidates, out);
}

/// Coarse caster/candidate relationship heuristic, returning
/// `(is_hostile, is_friendly)`. Matches the placeholder used elsewhere in the
/// spell target code: cross-type (player vs creature) is hostile, same-type is
/// friendly, pending real faction-template evaluation.
fn caster_relation(caster: ObjectGuid, target: ObjectGuid) -> (bool, bool) {
    if caster == target {
        return (false, true);
    }
    let caster_is_player = caster.is_player();
    let target_is_player = target.is_player();
    if caster_is_player == target_is_player {
        (false, true)
    } else {
        (true, false)
    }
}

/// Fetch a unit's position (player or creature), defaulting to the origin when
/// the GUID is not a live tracked unit.
fn unit_position(guid: ObjectGuid, world: &World) -> Position {
    if guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(guid, |p| p.movement.position)
            .unwrap_or_default()
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| Position {
                x: c.position.x,
                y: c.position.y,
                z: c.position.z,
                o: c.position.o,
            })
            .unwrap_or_default()
    } else {
        Position::default()
    }
}

/// Whether a tracked unit is currently alive.
fn unit_is_alive(guid: ObjectGuid, world: &World) -> bool {
    if guid.is_player() {
        world.managers.player_mgr.is_player_alive(guid)
    } else if guid.is_creature_or_pet() {
        world.managers.creature_mgr.is_alive(guid)
    } else {
        false
    }
}

/// Resolve the `(map_id, instance_id)` the caster occupies.
fn unit_map_location(guid: ObjectGuid, world: &World) -> Option<(u32, u32)> {
    if guid.is_player() {
        world
            .managers
            .player_mgr
            .with_player(guid, |p| (p.map_id, p.instance_id))
    } else if guid.is_creature_or_pet() {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| (c.map_id, c.instance_id))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(n: u64) -> ObjectGuid {
        ObjectGuid::from_raw(n)
    }

    fn candidate(n: u64, x: f32, y: f32) -> AreaCandidate {
        AreaCandidate {
            guid: guid(n),
            position: Position {
                x,
                y,
                z: 0.0,
                o: 0.0,
            },
            is_alive: true,
            is_hostile_to_caster: false,
            is_friendly_to_caster: false,
        }
    }

    fn center_params(
        push: SpellNotifyPushType,
        targets: SpellTargets,
        radius: f32,
    ) -> AreaSearchParams {
        AreaSearchParams {
            push_type: push,
            targets,
            radius,
            center: Position::default(),
            caster_position: Position::default(),
            caster_is_player_controlled: false,
            allow_dead_target: false,
        }
    }

    // --- geometry: 2d range ---

    #[test]
    fn within_dist_2d_ignores_z() {
        let center = Position::xyz(0.0, 0.0, 0.0);
        let near = Position::xyz(3.0, 4.0, 1000.0); // dist 5 in XY
        assert!(is_within_dist_2d(center, near, 5.0));
        assert!(!is_within_dist_2d(center, near, 4.9));
    }

    // --- geometry: cone ---

    #[test]
    fn cone_includes_units_in_front() {
        // Caster at origin facing +X (o = 0).
        let caster = Position::new(0.0, 0.0, 0.0, 0.0);
        let front = Position::xyz(5.0, 0.0, 0.0);
        let behind = Position::xyz(-5.0, 0.0, 0.0);
        assert!(is_in_cone(caster, front, 10.0, DEFAULT_CONE_ARC));
        assert!(!is_in_cone(caster, behind, 10.0, DEFAULT_CONE_ARC));
    }

    #[test]
    fn cone_respects_radius() {
        let caster = Position::new(0.0, 0.0, 0.0, 0.0);
        let far_front = Position::xyz(20.0, 0.0, 0.0);
        assert!(!is_in_cone(caster, far_front, 10.0, DEFAULT_CONE_ARC));
    }

    #[test]
    fn cone_narrow_arc_excludes_side() {
        let caster = Position::new(0.0, 0.0, 0.0, 0.0);
        let side = Position::xyz(0.0, 5.0, 0.0); // 90deg to the left
                                                 // 180deg hemisphere includes the boundary; a narrow 60deg arc excludes it.
        assert!(is_in_cone(caster, side, 10.0, PI));
        assert!(!is_in_cone(caster, side, 10.0, PI / 3.0));
    }

    // --- target mask ---

    #[test]
    fn mask_hostile_and_friendly() {
        let mut hostile = candidate(1, 0.0, 0.0);
        hostile.is_hostile_to_caster = true;
        let mut friendly = candidate(2, 0.0, 0.0);
        friendly.is_friendly_to_caster = true;

        assert!(passes_target_mask(SpellTargets::Hostile, &hostile, false));
        assert!(!passes_target_mask(SpellTargets::Hostile, &friendly, false));

        assert!(passes_target_mask(SpellTargets::Friendly, &friendly, false));
        assert!(!passes_target_mask(SpellTargets::Friendly, &hostile, false));

        assert!(passes_target_mask(
            SpellTargets::NotFriendly,
            &hostile,
            false
        ));
        assert!(!passes_target_mask(
            SpellTargets::NotFriendly,
            &friendly,
            false
        ));

        assert!(passes_target_mask(SpellTargets::All, &friendly, false));
    }

    #[test]
    fn mask_aoe_damage_depends_on_caster_control() {
        let mut neutral = candidate(3, 0.0, 0.0); // neither hostile nor friendly
        neutral.is_hostile_to_caster = false;
        neutral.is_friendly_to_caster = false;

        // Player-controlled: hits anything not friendly -> neutral included.
        assert!(passes_target_mask(SpellTargets::AoeDamage, &neutral, true));
        // Non-player: only hostile -> neutral excluded.
        assert!(!passes_target_mask(
            SpellTargets::AoeDamage,
            &neutral,
            false
        ));
    }

    // --- alive gate ---

    #[test]
    fn dead_units_gated_unless_allowed_or_all() {
        let mut dead = candidate(4, 0.0, 0.0);
        dead.is_alive = false;

        let strict = center_params(SpellNotifyPushType::SelfCenter, SpellTargets::Hostile, 10.0);
        assert!(!passes_alive_gate(&strict, &dead));

        let mut allow = strict;
        allow.allow_dead_target = true;
        assert!(passes_alive_gate(&allow, &dead));

        let all = center_params(SpellNotifyPushType::SelfCenter, SpellTargets::All, 10.0);
        assert!(passes_alive_gate(&all, &dead));
    }

    // --- center resolution ---

    #[test]
    fn resolve_center_picks_correct_anchor() {
        let caster = Position::xyz(1.0, 1.0, 0.0);
        let src = Position::xyz(2.0, 2.0, 0.0);
        let dest = Position::xyz(3.0, 3.0, 0.0);
        let target = Position::xyz(4.0, 4.0, 0.0);

        assert_eq!(
            resolve_center(
                SpellNotifyPushType::SelfCenter,
                caster,
                Some(src),
                Some(dest),
                Some(target)
            ),
            Some(caster)
        );
        assert_eq!(
            resolve_center(
                SpellNotifyPushType::DestCenter,
                caster,
                Some(src),
                Some(dest),
                Some(target)
            ),
            Some(dest)
        );
        assert_eq!(
            resolve_center(
                SpellNotifyPushType::TargetCenter,
                caster,
                Some(src),
                Some(dest),
                Some(target)
            ),
            Some(target)
        );
        // Dest-centered with no destination -> no search.
        assert_eq!(
            resolve_center(SpellNotifyPushType::DestCenter, caster, None, None, None),
            None
        );
        // Src-centered falls back to the caster when no source is set.
        assert_eq!(
            resolve_center(SpellNotifyPushType::SrcCenter, caster, None, None, None),
            Some(caster)
        );
    }

    // --- end-to-end pure gather ---

    #[test]
    fn gather_center_hostiles_within_radius() {
        let mut params =
            center_params(SpellNotifyPushType::DestCenter, SpellTargets::Hostile, 10.0);
        params.center = Position::xyz(0.0, 0.0, 0.0);

        let mut in_range = candidate(1, 5.0, 0.0);
        in_range.is_hostile_to_caster = true;
        let mut out_of_range = candidate(2, 50.0, 0.0);
        out_of_range.is_hostile_to_caster = true;
        let mut friendly_in_range = candidate(3, 5.0, 0.0);
        friendly_in_range.is_friendly_to_caster = true;

        let candidates = [in_range, out_of_range, friendly_in_range];
        let mut out = Vec::new();
        fill_area_targets_from_candidates(&params, &candidates, &mut out);

        assert_eq!(out, vec![guid(1)]);
    }

    #[test]
    fn gather_cone_preserves_push_back_order() {
        let mut params = center_params(SpellNotifyPushType::Cone, SpellTargets::All, 20.0);
        params.caster_position = Position::new(0.0, 0.0, 0.0, 0.0); // facing +X

        let a = candidate(10, 5.0, 0.0); // in front
        let b = candidate(11, -5.0, 0.0); // behind, excluded
        let c = candidate(12, 8.0, 1.0); // in front

        let candidates = [a, b, c];
        let mut out = Vec::new();
        fill_area_targets_from_candidates(&params, &candidates, &mut out);

        assert_eq!(out, vec![guid(10), guid(12)]);
    }
}
