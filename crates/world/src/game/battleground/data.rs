//! Battleground definitions from the world database: templates, which NPCs
//! are battlemasters for which battleground, and the entrance portals.

use std::collections::HashMap;

use anyhow::Result;
use oxcore_shared::game::battleground::BattleGroundTypeId;
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::Position;
use sqlx::PgPool;

/// Level brackets are this many levels wide.
const BRACKET_WIDTH: u32 = 10;
/// The highest bracket id.
pub const BRACKET_LAST: u8 = 5;

/// One battleground's rules.
#[derive(Debug, Clone)]
pub struct BattlegroundTemplate {
    pub bg_type: BattleGroundTypeId,
    pub map_id: u32,
    pub min_players_per_team: u32,
    pub max_players_per_team: u32,
    pub min_level: u32,
    pub max_level: u32,
    pub alliance_win_spell: u32,
    pub alliance_lose_spell: u32,
    pub horde_win_spell: u32,
    pub horde_lose_spell: u32,
    pub alliance_start: Position,
    pub horde_start: Position,
    /// Reference loot a taken insignia drops, besides coins.
    pub player_loot_id: u32,
    /// Display name, for messages.
    pub name: String,
}

impl BattlegroundTemplate {
    pub fn start_location(&self, team: Team) -> Position {
        if team == Team::Horde {
            self.horde_start
        } else {
            self.alliance_start
        }
    }

    pub fn win_spell(&self, team: Team) -> u32 {
        if team == Team::Horde {
            self.horde_win_spell
        } else {
            self.alliance_win_spell
        }
    }

    pub fn lose_spell(&self, team: Team) -> u32 {
        if team == Team::Horde {
            self.horde_lose_spell
        } else {
            self.alliance_lose_spell
        }
    }

    /// The bracket a player of `level` queues in, or `None` below the minimum.
    pub fn bracket_for_level(&self, level: u32) -> Option<u8> {
        if level < self.min_level {
            return None;
        }
        Some(((level - self.min_level) / BRACKET_WIDTH).min(u32::from(BRACKET_LAST)) as u8)
    }

    /// Lowest level of a bracket. Alterac Valley has a single 51-60 bracket;
    /// Arathi Basin's brackets are shifted one up from its minimum level.
    pub fn bracket_min_level(&self, bracket: u8) -> u32 {
        let mut bracket = u32::from(bracket);
        match self.bg_type {
            BattleGroundTypeId::AlteracValley => return 51,
            BattleGroundTypeId::ArathiBasin => bracket += 1,
            _ => {}
        }
        let bracket = bracket.min(u32::from(BRACKET_LAST));
        if self.bg_type == BattleGroundTypeId::ArathiBasin {
            BRACKET_WIDTH * bracket + self.min_level - BRACKET_WIDTH
        } else {
            BRACKET_WIDTH * bracket + self.min_level
        }
    }

    /// One past the highest level of a bracket.
    pub fn bracket_max_level(&self, bracket: u8) -> u32 {
        if self.bg_type == BattleGroundTypeId::AlteracValley {
            return 61;
        }
        let last = bracket >= BRACKET_LAST
            || (self.bg_type == BattleGroundTypeId::ArathiBasin && bracket == BRACKET_LAST - 1);
        if last {
            self.bracket_min_level(BRACKET_LAST) + 1
        } else {
            self.bracket_min_level(bracket) + BRACKET_WIDTH
        }
    }

    /// The inclusive level range of a bracket.
    pub fn bracket_levels(&self, bracket: u8) -> (u8, u8) {
        let min = self.bracket_min_level(bracket).min(255) as u8;
        let max = self.bracket_max_level(bracket).saturating_sub(1).min(255) as u8;
        (min, max)
    }
}

/// An area trigger at a battleground's entrance portal.
#[derive(Debug, Clone)]
pub struct EntranceTrigger {
    pub team: Team,
    pub bg_type: BattleGroundTypeId,
    pub exit_map: u32,
    pub exit: Position,
}

/// Everything loaded from the database.
#[derive(Debug, Default)]
pub struct BattlegroundData {
    pub templates: HashMap<BattleGroundTypeId, BattlegroundTemplate>,
    /// NPC entry -> the battleground it queues for.
    pub battlemasters: HashMap<u32, BattleGroundTypeId>,
    /// Area trigger id -> entrance.
    pub entrances: HashMap<u32, EntranceTrigger>,
}

/// Load templates (newest patch at or below `patch`), battlemasters and entrances.
///
/// `safe_loc` resolves a WorldSafeLocs id to its position.
pub async fn load(
    pool: &PgPool,
    patch: u8,
    safe_loc: impl Fn(u32) -> Option<(u32, Position)>,
) -> Result<BattlegroundData> {
    let mut data = BattlegroundData::default();

    let facings: HashMap<i64, f32> = sqlx::query_as::<_, (i64, f32)>(
        "SELECT id::bigint, orientation FROM world.world_safe_locs_facing",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    type TemplateRow = (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
    let rows = sqlx::query_as::<_, TemplateRow>(
        "SELECT DISTINCT ON (id) id::bigint, min_players_per_team::bigint,
                max_players_per_team::bigint, min_level::bigint, max_level::bigint,
                alliance_win_spell::bigint, alliance_lose_spell::bigint,
                horde_win_spell::bigint, horde_lose_spell::bigint,
                alliance_start_location::bigint, horde_start_location::bigint,
                player_loot_id::bigint
         FROM world.battleground_template
         WHERE patch <= $1
         ORDER BY id, patch DESC",
    )
    .bind(i16::from(patch))
    .fetch_all(pool)
    .await?;
    for (id, min_pt, max_pt, min_lvl, max_lvl, a_win, a_lose, h_win, h_lose, a_start, h_start, loot) in
        rows
    {
        let bg_type = BattleGroundTypeId::from_u32(id as u32);
        if bg_type == BattleGroundTypeId::None {
            continue;
        }
        let start = |loc: i64| {
            safe_loc(loc as u32).map(|(_, mut pos)| {
                pos.o = facings.get(&loc).copied().unwrap_or(0.0);
                pos
            })
        };
        let (Some(alliance_start), Some(horde_start)) = (start(a_start), start(h_start)) else {
            tracing::warn!("Battleground {} has an unknown start location; skipped", id);
            continue;
        };
        data.templates.insert(
            bg_type,
            BattlegroundTemplate {
                bg_type,
                map_id: bg_type.map_id(),
                min_players_per_team: min_pt as u32,
                max_players_per_team: max_pt as u32,
                min_level: min_lvl as u32,
                max_level: max_lvl as u32,
                alliance_win_spell: a_win as u32,
                alliance_lose_spell: a_lose as u32,
                horde_win_spell: h_win as u32,
                horde_lose_spell: h_lose as u32,
                alliance_start,
                horde_start,
                player_loot_id: loot as u32,
                name: match bg_type {
                    BattleGroundTypeId::AlteracValley => "Alterac Valley",
                    BattleGroundTypeId::WarsongGulch => "Warsong Gulch",
                    BattleGroundTypeId::ArathiBasin => "Arathi Basin",
                    BattleGroundTypeId::None => "",
                }
                .to_string(),
            },
        );
    }

    for (entry, bg) in sqlx::query_as::<_, (i64, i64)>(
        "SELECT entry::bigint, bg_template::bigint FROM world.battlemaster_entry",
    )
    .fetch_all(pool)
    .await?
    {
        let bg_type = BattleGroundTypeId::from_u32(bg as u32);
        if bg_type != BattleGroundTypeId::None {
            data.battlemasters.insert(entry as u32, bg_type);
        }
    }

    for (id, team, bg, map, x, y, z, o) in sqlx::query_as::<_, (i64, i64, i64, i64, f32, f32, f32, f32)>(
        "SELECT id::bigint, team::bigint, bg_template::bigint, exit_map::bigint,
                exit_position_x, exit_position_y, exit_position_z, exit_orientation
         FROM world.areatrigger_bg_entrance",
    )
    .fetch_all(pool)
    .await?
    {
        let bg_type = BattleGroundTypeId::from_u32(bg as u32);
        let team = match team {
            469 => Team::Alliance,
            67 => Team::Horde,
            _ => continue,
        };
        if bg_type == BattleGroundTypeId::None {
            continue;
        }
        data.entrances.insert(
            id as u32,
            EntranceTrigger {
                team,
                bg_type,
                exit_map: map as u32,
                exit: Position::new(x, y, z, o),
            },
        );
    }

    Ok(data)
}

#[cfg(test)]
pub(crate) fn test_template(bg_type: BattleGroundTypeId, min_level: u32) -> BattlegroundTemplate {
    BattlegroundTemplate {
        bg_type,
        map_id: bg_type.map_id(),
        min_players_per_team: 1,
        max_players_per_team: 10,
        min_level,
        max_level: 60,
        alliance_win_spell: 0,
        alliance_lose_spell: 0,
        horde_win_spell: 0,
        horde_lose_spell: 0,
        alliance_start: Position::new(1.0, 2.0, 3.0, 0.0),
        horde_start: Position::new(4.0, 5.0, 6.0, 0.0),
        player_loot_id: 0,
        name: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warsong_brackets_are_ten_levels_from_the_minimum() {
        let wsg = test_template(BattleGroundTypeId::WarsongGulch, 10);
        assert_eq!(wsg.bracket_for_level(9), None);
        assert_eq!(wsg.bracket_for_level(10), Some(0));
        assert_eq!(wsg.bracket_for_level(19), Some(0));
        assert_eq!(wsg.bracket_for_level(60), Some(5));
        assert_eq!(wsg.bracket_levels(0), (10, 19));
        assert_eq!(wsg.bracket_levels(4), (50, 59));
        assert_eq!(wsg.bracket_levels(5), (60, 60));
    }

    #[test]
    fn arathi_brackets_start_at_twenty_and_end_at_sixty() {
        let ab = test_template(BattleGroundTypeId::ArathiBasin, 20);
        assert_eq!(ab.bracket_levels(0), (20, 29));
        assert_eq!(ab.bracket_levels(3), (50, 59));
        assert_eq!(ab.bracket_levels(4), (60, 60));
    }

    #[test]
    fn alterac_is_one_bracket() {
        let av = test_template(BattleGroundTypeId::AlteracValley, 51);
        assert_eq!(av.bracket_for_level(55), Some(0));
        assert_eq!(av.bracket_levels(0), (51, 60));
    }
}
