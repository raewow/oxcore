//! HonorSystem - awards honor on PvP kills.
//!
//! Hooks in two places:
//!   1. Combat damage application (`record_damage`) — tracks attackers.
//!   2. Player death (`reward_honor_on_death`) — distributes credit.

use std::sync::Arc;

use crate::shared::database::characters::models::honor::HonorCPRow;
use crate::shared::database::characters::repositories::HonorRepository;
use crate::shared::protocol::ObjectGuid;
use crate::world::World;

/// Stateless honor orchestrator. All per-player state lives on `Player`.
pub struct HonorSystem;

impl HonorSystem {
    pub fn new() -> Self {
        Self
    }

    /// Record a damage event. Called from the combat damage pipeline.
    /// No-op if either side isn't a player or they're on the same team.
    pub fn record_damage(
        &self,
        attacker: ObjectGuid,
        victim: ObjectGuid,
        damage: u32,
        world: &World,
    ) {
        if damage == 0 {
            return;
        }
        if !attacker.is_player() || !victim.is_player() {
            return;
        }
        if attacker == victim {
            return;
        }

        let now = now_secs();

        // Skip if attacker and victim share the same faction team.
        let (a_team, v_team) = {
            let mgr = world.systems.player.manager();
            let a = mgr
                .with_player(attacker, |p| team_from_race(p.race))
                .unwrap_or(0);
            let v = mgr
                .with_player(victim, |p| team_from_race(p.race))
                .unwrap_or(0);
            (a, v)
        };
        if a_team == 0 || v_team == 0 || a_team == v_team {
            return;
        }

        // Append to victim's tracker.
        world
            .systems
            .player
            .manager()
            .with_player_mut(victim, |victim_p| {
                victim_p.combat.honor.record_damage(attacker, now, damage);
            });
    }

    /// Distribute honor to recent contributors on a PvP death.
    ///
    /// If the contributor tracker is empty (the CombatSystem doesn't wire
    /// `record_damage` for melee yet), we fall back to attributing the
    /// killing blow to the single `killer` if one was supplied.
    ///
    /// Honor formula (simplified, matches vmangos' scale-down branch):
    ///   base_honor = victim_level * 0.5 / contributor_count
    ///   per-attacker = base_honor * (attacker_damage / total_damage)
    ///
    /// The resulting honor is logged in `character_honor_cp` and accumulated
    /// into `characters.honor_last_week_cp` + `.honor_last_week_hk`.
    pub async fn reward_honor_on_death(
        &self,
        victim: ObjectGuid,
        killer: Option<ObjectGuid>,
        world: &World,
    ) {
        if !victim.is_player() {
            return;
        }

        let now = now_secs();

        // Extract contributors + victim context atomically.
        let (mut contributors, victim_level, victim_team) = {
            let mgr = world.systems.player.manager();
            let Some(ctx) = mgr.with_player_mut(victim, |p| {
                (
                    p.combat.honor.take_contributors(now),
                    p.level,
                    team_from_race(p.race),
                )
            }) else {
                return;
            };
            ctx
        };

        // Fallback: if no damage tracker entries but we have a killer,
        // attribute full credit to the killer.
        if contributors.is_empty() {
            if let Some(k) = killer {
                if k.is_player() && k != victim {
                    contributors.push(crate::world::game::player::honor::Contributor {
                        guid: k,
                        last_damage_time: now,
                        total_damage: 1,
                    });
                }
            }
        }

        if contributors.is_empty() {
            return;
        }

        // Sum damage for pro-rating and count hostile contributors.
        let total_damage: u64 = contributors.iter().map(|c| c.total_damage).sum();
        if total_damage == 0 {
            return;
        }

        // Per-kill base honor: scale with victim level and contributor count.
        // This is a simplified vmangos-compatible curve.
        let base_honor = victim_base_honor(victim_level, contributors.len());

        let pool = Arc::new(world.databases.character.clone());

        for c in &contributors {
            // Filter hostile faction only.
            let att_team = world
                .systems
                .player
                .manager()
                .with_player(c.guid, |p| team_from_race(p.race))
                .unwrap_or(0);
            if att_team == 0 || att_team == victim_team {
                continue;
            }

            let share = (c.total_damage as f32 / total_damage as f32).clamp(0.0, 1.0);
            let honor = base_honor * share;
            if honor <= 0.0 {
                continue;
            }

            // Update in-memory honor stats.
            world.systems.player.manager().with_player_mut(c.guid, |p| {
                p.combat.honor_last_week_cp += honor;
                p.combat.honor_last_week_hk += 1;
            });

            // Log to character_honor_cp (fire and forget).
            let row = HonorCPRow {
                guid: c.guid.counter(),
                // victim_type: 4 = Player (matches vmangos ObjectTypeIDs).
                victim_type: 4,
                victim_id: victim.counter(),
                cp: honor,
                date: now as u32,
                // type: 1 = Honorable Kill. We don't yet detect dishonorable kills here.
                r#type: 1,
            };
            let pool_clone = Arc::clone(&pool);
            tokio::spawn(async move {
                let repo = HonorRepository::new(pool_clone);
                if let Err(e) = repo.save_honor_cp(&row).await {
                    tracing::warn!("Failed to log honor_cp: {}", e);
                }
            });
        }
    }
}

/// Faction team id: 1 = Alliance, 2 = Horde, 0 = unknown.
fn team_from_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 => 1, // Human, Dwarf, NightElf, Gnome
        2 | 5 | 6 | 8 => 2, // Orc, Undead, Tauren, Troll
        _ => 0,
    }
}

/// Compute the honor base for a kill given victim level and the group size of
/// contributors. This is a simplified stand-in for the full vmangos formula.
fn victim_base_honor(victim_level: u8, contributor_count: usize) -> f32 {
    // Scales with victim level, divided across the party.
    let level_factor = (victim_level as f32).max(1.0) * 0.5;
    let group_rate = 1.0 / (contributor_count as f32).max(1.0);
    level_factor * group_rate
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
