//! Exploring the world: walking into an area for the first time marks it on the player's
//! map and, below the level cap, earns experience scaled to the area's level.

use oxcore_shared::game::experience::XpSource;
use oxcore_shared::messages::environment::SmsgExplorationExperience;
use oxcore_shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use oxcore_shared::protocol::update_fields::PLAYER_EXPLORED_ZONES_1;
use oxcore_shared::protocol::{ObjectGuid, Position};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::World;

/// Words of explored-area bits a player carries; each area's explore flag is one bit.
pub const EXPLORED_ZONE_FIELDS: usize = 64;

/// The explored-area bits, one per AreaTable explore flag.
pub type ExploredZones = [u32; EXPLORED_ZONE_FIELDS];

fn base_xp_table() -> &'static RwLock<HashMap<u8, u32>> {
    static TABLE: OnceLock<RwLock<HashMap<u8, u32>>> = OnceLock::new();
    TABLE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Load `exploration_basexp`: the experience for discovering an area of each level.
pub async fn load_base_xp(pool: &sqlx::PgPool) -> anyhow::Result<usize> {
    let rows: Vec<(i16, i32)> =
        sqlx::query_as("SELECT level, basexp FROM world.exploration_basexp")
            .fetch_all(pool)
            .await?;
    let mut table = base_xp_table().write();
    for (level, xp) in rows {
        table.insert(u8::try_from(level)?, u32::try_from(xp)?);
    }
    Ok(table.len())
}

#[cfg(test)]
pub fn set_base_xp_for_test(level: u8, xp: u32) {
    base_xp_table().write().insert(level, xp);
}

fn base_xp(level: i32) -> u32 {
    u8::try_from(level)
        .ok()
        .and_then(|level| base_xp_table().read().get(&level).copied())
        .unwrap_or(0)
}

/// The explored-area words as saved (`explored_zones`, space-separated); missing or
/// unreadable words count as unexplored.
pub fn parse_saved(saved: Option<&str>) -> ExploredZones {
    let mut zones = [0; EXPLORED_ZONE_FIELDS];
    for (slot, word) in saved
        .unwrap_or_default()
        .split_whitespace()
        .take(EXPLORED_ZONE_FIELDS)
        .enumerate()
    {
        zones[slot] = word.parse().unwrap_or(0);
    }
    zones
}

/// The explored-area words in their saved form.
pub fn to_saved(zones: &ExploredZones) -> String {
    zones
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the area with this explore flag has been discovered.
pub fn is_explored(zones: &ExploredZones, explore_flag: u32) -> bool {
    let (word, bit) = ((explore_flag / 32) as usize, explore_flag % 32);
    word < EXPLORED_ZONE_FIELDS && zones[word] & (1 << bit) != 0
}

/// Experience for discovering an area of `area_level` at player level `level`: full value
/// up to five levels apart, the value of an area five levels above the player when it is
/// further above, and 5% less per level past five when it is further below.
pub fn discovery_xp(level: u8, area_level: i32, rate: f32) -> u32 {
    let diff = i32::from(level) - area_level;
    let xp = if diff < -5 {
        base_xp(i32::from(level) + 5) as f32
    } else if diff > 5 {
        let percent = (100 - (diff - 5) * 5).clamp(0, 100);
        (base_xp(area_level) * percent as u32 / 100) as f32
    } else {
        base_xp(area_level) as f32
    };
    (xp * rate) as u32
}

/// After a move: discover the area the player stands in, if it is new to them.
pub fn check_area_explore(world: &World, player_guid: ObjectGuid, map_id: u32, position: Position) {
    let flag = u32::from(
        world
            .managers
            .terrain_mgr
            .get(map_id)
            .get_area_flag(position.x, position.y),
    );
    // No terrain under the player.
    if flag == 0 || flag == 0xFFFF {
        return;
    }
    let (word, bit) = ((flag / 32) as usize, 1u32 << (flag % 32));
    if word >= EXPLORED_ZONE_FIELDS {
        tracing::warn!("Area flag {} at {:?} on map {} is out of range", flag, position, map_id);
        return;
    }

    let Some((value, level, saved)) = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |p| {
            if !p.is_alive() || p.taxi.is_flying() || p.explored_zones[word] & bit != 0 {
                return None;
            }
            p.explored_zones[word] |= bit;
            Some((p.explored_zones[word], p.level, to_saved(&p.explored_zones)))
        })
        .flatten()
    else {
        return;
    };

    world.managers.broadcast_mgr.send_msg_to_player(
        player_guid,
        SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(ObjectGuid::from_low(player_guid.counter()), ObjectType::Player)
                .set_field(PLAYER_EXPLORED_ZONES_1 + word as u32, value),
        )),
    );
    save(world, player_guid, saved);

    let Some((area_id, area_level)) = area_by_flag(world, flag, map_id) else {
        tracing::warn!(
            "Player {:?} discovered an unknown area (flag {}) on map {}",
            player_guid,
            flag,
            map_id
        );
        return;
    };
    let xp = if area_level > 0 && u32::from(level) < world.config.max_player_level {
        discovery_xp(level, area_level, world.config.rate_xp_explore)
    } else {
        0
    };
    // Sent even without experience, so the client names the new area.
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, SmsgExplorationExperience { area_id, xp });
    if xp > 0 {
        let world = world.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(e) = world
                    .systems
                    .experience
                    .add_xp(player_guid, xp, XpSource::Discovery, None, 1.0)
                    .await
                {
                    tracing::warn!("Exploration experience for {:?} failed: {e:#}", player_guid);
                }
            });
        }
    }
}

/// The area carrying `flag`, preferring one on `map_id` (some flags repeat across maps).
fn area_by_flag(world: &World, flag: u32, map_id: u32) -> Option<(u32, i32)> {
    let dbc = world.dbc.read();
    let mut fallback = None;
    for (_, area) in dbc.area_table.entries() {
        if area.explore_flag == flag {
            if area.map_id == map_id {
                return Some((area.id, area.area_level));
            }
            fallback.get_or_insert((area.id, area.area_level));
        }
    }
    fallback
}

fn save(world: &World, player_guid: ObjectGuid, saved: String) {
    let pool = world.databases.character.clone();
    let guid = i64::from(player_guid.counter());
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let repository = oxcore_db::database::characters::PgCharacterRepository::new(
                std::sync::Arc::new(pool),
            );
            if let Err(e) = repository.save_explored_zones(guid, &saved).await {
                tracing::warn!("Failed to save explored areas of character {}: {}", guid, e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_words_round_trip() {
        let mut zones = [0; EXPLORED_ZONE_FIELDS];
        zones[0] = 5;
        zones[63] = u32::MAX;
        assert_eq!(parse_saved(Some(&to_saved(&zones))), zones);
        assert_eq!(parse_saved(None), [0; EXPLORED_ZONE_FIELDS]);
        assert!(is_explored(&zones, 2));
        assert!(!is_explored(&zones, 1));
        assert!(is_explored(&zones, 63 * 32 + 31));
    }

    #[test]
    fn discovery_xp_scales_with_level_difference() {
        set_base_xp_for_test(10, 100);
        set_base_xp_for_test(20, 400);
        set_base_xp_for_test(25, 600);
        // Within five levels: the area's own value.
        assert_eq!(discovery_xp(12, 10, 1.0), 100);
        // Far above the player: the value five levels up from the player.
        assert_eq!(discovery_xp(20, 40, 1.0), 600);
        // Seven levels below: 10% less.
        assert_eq!(discovery_xp(27, 20, 1.0), 360);
        assert_eq!(discovery_xp(12, 10, 2.0), 200);
    }
}
