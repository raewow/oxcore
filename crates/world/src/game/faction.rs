//! Faction reactions between units.
//!
//! Reactions come from FactionTemplate.dbc (team masks plus explicit enemy and
//! friend lists). A creature whose faction carries reputation reacts to a player
//! through that player's standing instead, capped at neutral while the player is
//! at war with the faction.

use crate::dbc::structures::FactionTemplateDbcEntry;
use crate::dbc::DbcManager;
use crate::game::player::Player;
use oxcore_shared::game::reputation::ReputationRank;

/// Creatures of this faction answer a call for help from their own faction.
pub const FACTION_TEMPLATE_RESPOND_TO_CALL_FOR_HELP: u32 = 0x0000_0001;
/// Creatures of this faction run away instead of answering a call for help.
pub const FACTION_TEMPLATE_FLEE_FROM_CALL_FOR_HELP: u32 = 0x0000_0400;
/// Guards that attack players flagged for contested PvP.
pub const FACTION_TEMPLATE_FLAG_ATTACK_PVP_ACTIVE_PLAYERS: u32 = 0x0000_1000;

const FACTION_MASK_PLAYER: u32 = 1;

/// Friendship through the explicit friend list or the team masks.
pub fn template_is_friendly(a: &FactionTemplateDbcEntry, b: &FactionTemplateDbcEntry) -> bool {
    if b.faction != 0 {
        if a.enemy_factions.contains(&b.faction) {
            return false;
        }
        if a.friend_factions.contains(&b.faction) {
            return true;
        }
    }
    (a.friendly_mask & b.our_mask) != 0 || (a.our_mask & b.friendly_mask) != 0
}

/// Hostility through the explicit enemy list or the team masks.
pub fn template_is_hostile(a: &FactionTemplateDbcEntry, b: &FactionTemplateDbcEntry) -> bool {
    if b.faction != 0 {
        if a.enemy_factions.contains(&b.faction) {
            return true;
        }
        if a.friend_factions.contains(&b.faction) {
            return false;
        }
    }
    (a.hostile_mask & b.our_mask) != 0
}

/// Reaction of faction template `a` towards template `b`, neutral when either is unknown.
pub fn template_reaction(dbc: &DbcManager, a: u32, b: u32) -> ReputationRank {
    let (Some(a), Some(b)) = (dbc.get_faction_template(a), dbc.get_faction_template(b)) else {
        return ReputationRank::Neutral;
    };
    if template_is_hostile(a, b) {
        ReputationRank::Hostile
    } else if template_is_friendly(a, b) || template_is_friendly(b, a) {
        ReputationRank::Friendly
    } else {
        ReputationRank::Neutral
    }
}

/// Whether the faction behind this template has a reputation bar.
fn has_reputation(dbc: &DbcManager, template: &FactionTemplateDbcEntry) -> bool {
    dbc.get_faction(template.faction)
        .is_some_and(|faction| faction.reputation_list_id >= 0)
}

/// How a creature of faction template `creature_ft` feels about `player`.
pub fn creature_reaction_to_player(
    dbc: &DbcManager,
    creature_ft: u32,
    player: &Player,
) -> ReputationRank {
    if player.gm_mode {
        return ReputationRank::Neutral;
    }
    let Some(template) = dbc.get_faction_template(creature_ft) else {
        return ReputationRank::Neutral;
    };
    if let Some(faction) = dbc.get_faction(template.faction) {
        if faction.reputation_list_id >= 0 {
            let base = faction.get_base_reputation(player.race, player.class);
            let (standing, at_war) = player
                .reputation
                .get_standing_by_faction_id(template.faction)
                .map(|s| (s.standing, s.is_at_war()))
                .unwrap_or((0, false));
            let rank = ReputationRank::from_value(base + standing);
            return if at_war {
                rank.min(ReputationRank::Neutral)
            } else {
                rank
            };
        }
    }
    template_reaction(dbc, creature_ft, player.faction_template())
}

/// Whether a creature of faction template `creature_ft` is hostile to `player`.
pub fn creature_is_hostile_to_player(dbc: &DbcManager, creature_ft: u32, player: &Player) -> bool {
    creature_reaction_to_player(dbc, creature_ft, player) <= ReputationRank::Hostile
}

/// How `player` feels about a creature of faction template `creature_ft`.
pub fn player_reaction_to_creature(
    dbc: &DbcManager,
    player: &Player,
    creature_ft: u32,
) -> ReputationRank {
    if player.gm_mode {
        return ReputationRank::Neutral;
    }
    if let Some(template) = dbc.get_faction_template(creature_ft) {
        if has_reputation(dbc, template) {
            let at_war = player
                .reputation
                .get_standing_by_faction_id(template.faction)
                .is_some_and(|s| s.is_at_war());
            return if at_war {
                ReputationRank::Hostile
            } else {
                ReputationRank::Friendly
            };
        }
    }
    template_reaction(dbc, player.faction_template(), creature_ft)
}

/// A faction template with no enemies and no friends, whose faction has no reputation.
pub fn is_neutral_to_all(dbc: &DbcManager, ft: u32) -> bool {
    let Some(template) = dbc.get_faction_template(ft) else {
        return true;
    };
    if template.faction == 0 {
        return true;
    }
    if has_reputation(dbc, template) {
        return false;
    }
    template.enemy_factions.iter().all(|&f| f == 0)
        && template.hostile_mask == 0
        && template.friendly_mask == 0
}

/// Hostile to every player regardless of team, and not a reputation faction.
pub fn is_hostile_to_players(dbc: &DbcManager, ft: u32) -> bool {
    let Some(template) = dbc.get_faction_template(ft) else {
        return false;
    };
    if template.faction == 0 || has_reputation(dbc, template) {
        return false;
    }
    template.hostile_mask & FACTION_MASK_PLAYER != 0
}

/// The template's behavior flags (call for help, flee, contested guard).
pub fn template_flags(dbc: &DbcManager, ft: u32) -> u32 {
    dbc.get_faction_template(ft)
        .map(|t| t.faction_flags)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(id: u32, faction: u32, our: u32, friendly: u32, hostile: u32) -> FactionTemplateDbcEntry {
        FactionTemplateDbcEntry {
            id,
            faction,
            faction_flags: 0,
            our_mask: our,
            friendly_mask: friendly,
            hostile_mask: hostile,
            enemy_factions: [0; 4],
            friend_factions: [0; 4],
        }
    }

    fn dbc_with(templates: &[FactionTemplateDbcEntry]) -> DbcManager {
        let mut dbc = DbcManager::new();
        for t in templates {
            dbc.faction_template.insert(t.id, t.clone());
        }
        dbc
    }

    #[test]
    fn monster_mask_is_hostile_to_both_teams() {
        // Alliance player (ours=3), Horde player (ours=5), monster hostile to players.
        let dbc = dbc_with(&[
            template(1, 1, 3, 2, 12),
            template(2, 2, 5, 4, 10),
            template(54, 54, 8, 0, 1),
        ]);
        assert_eq!(template_reaction(&dbc, 54, 1), ReputationRank::Hostile);
        assert_eq!(template_reaction(&dbc, 54, 2), ReputationRank::Hostile);
    }

    #[test]
    fn team_guard_is_hostile_only_to_the_other_team() {
        let dbc = dbc_with(&[
            template(1, 1, 3, 2, 12),
            template(2, 2, 5, 4, 10),
            template(11, 72, 2, 2, 12),
        ]);
        assert_eq!(template_reaction(&dbc, 11, 2), ReputationRank::Hostile);
        assert_eq!(template_reaction(&dbc, 11, 1), ReputationRank::Friendly);
    }

    #[test]
    fn enemy_list_overrides_masks() {
        let mut a = template(100, 100, 8, 0, 0);
        a.enemy_factions[0] = 200;
        let b = template(200, 200, 8, 8, 0);
        let dbc = dbc_with(&[a, b]);
        assert_eq!(template_reaction(&dbc, 100, 200), ReputationRank::Hostile);
    }

    #[test]
    fn unknown_templates_are_neutral() {
        let dbc = dbc_with(&[]);
        assert_eq!(template_reaction(&dbc, 1, 2), ReputationRank::Neutral);
        assert!(is_neutral_to_all(&dbc, 7));
    }
}
