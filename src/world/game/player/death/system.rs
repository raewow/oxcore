//! Death System - orchestrates death and resurrection logic
//!
//! The `DeathSystem` is called by the world update loop and by packet handlers
//! to manage the complete death and resurrection pipeline.

use super::corpse::Corpse;
use super::durability;
use super::flow::*;
use super::ghost;
use super::graveyard::{self, GraveyardManager};
use super::resurrect::{self, ResurrectionMethod};
use super::sickness;
use super::state::{DeathState, DeathSystemState};
use crate::shared::messages::death::{
    SmsgCorpseReclaimDelay, SmsgDeathReleaseLocation, SmsgPreResurrect, SmsgResurrectRequest,
};
use crate::shared::messages::inventory::SmsgDurabilityDamageDeath;
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Position;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::dbc::DbcManager;
use crate::world::game::broadcast_mgr::BroadcastManagerTrait;
use crate::world::World;
use anyhow::Result;
use sqlx::MySqlPool;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// SPELL_ATTR_EX3_NO_DURABILITY_LOSS — spell marked as not causing durability loss on killing blow.
const SPELL_ATTR_EX3_NO_DURABILITY_LOSS: u32 = 0x00000080;

/// Map IDs of vanilla battlegrounds. Until a real BG system lands,
/// `is_player_in_battleground` just checks that the player's current map
/// matches one of these. Update when new BG maps are added.
const BATTLEGROUND_MAP_IDS: &[u32] = &[
    30,  // Alterac Valley
    489, // Warsong Gulch
    529, // Arathi Basin
];

/// Return true iff the given player is currently on a battleground map.
pub fn is_player_in_battleground(world: &World, player_guid: ObjectGuid) -> bool {
    let map_id = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |p| p.map_id)
        .unwrap_or(0);
    BATTLEGROUND_MAP_IDS.contains(&map_id)
}

/// MovementFlags::WATERWALKING — matches `shared::protocol::movement::MoveFlags::WATERWALKING`.
const MOVEMENT_FLAG_WATERWALKING: u32 = 0x10000000;

/// Bridge: Corpse struct → CorpseRow DB model.
fn corpse_to_row(corpse: &Corpse) -> crate::shared::database::characters::models::corpse::CorpseRow {
    use crate::shared::database::characters::models::corpse::CorpseRow;
    CorpseRow {
        guid: corpse.guid.counter(),
        player_guid: corpse.owner_guid.counter(),
        position_x: corpse.position.x,
        position_y: corpse.position.y,
        position_z: corpse.position.z,
        orientation: corpse.position.o,
        map: corpse.map_id,
        time: corpse.created_time,
        corpse_type: corpse.corpse_type as u8,
        instance: corpse.instance_id,
    }
}

/// System that manages player death and resurrection
pub struct DeathSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    graveyard_mgr: RwLock<GraveyardManager>,
    /// Players queued for the next battleground spirit-healer resurrection
    /// wave. The real BG wave tick (Phase 8) will drain this into resurrections.
    spirit_healer_queue: RwLock<Vec<ObjectGuid>>,
}

impl DeathSystem {
    /// Create a new DeathSystem
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self {
            broadcast_mgr,
            graveyard_mgr: RwLock::new(GraveyardManager::new()),
            spirit_healer_queue: RwLock::new(Vec::new()),
        }
    }

    /// Queue a ghost for the next BG spirit-healer resurrection wave.
    /// Called by `handle_area_spirit_healer_queue`. Wave tick consumes this.
    pub fn queue_for_spirit_healer(&self, player_guid: ObjectGuid) {
        let mut q = self.spirit_healer_queue.write().unwrap();
        if !q.contains(&player_guid) {
            q.push(player_guid);
        }
    }

    /// Initialize the death system
    pub async fn init(&self) -> Result<()> {

        Ok(())
    }

    /// Load graveyard data from database and DBC. Called after DBC is loaded.
    pub async fn load_graveyards(
        &self,
        world_pool: Arc<MySqlPool>,
        dbc_mgr: &DbcManager,
    ) -> Result<()> {
        let mut mgr = self.graveyard_mgr.write().unwrap();
        mgr.load(world_pool, dbc_mgr).await?;
        Ok(())
    }

    /// Shutdown the death system
    pub async fn shutdown(&self) -> Result<()> {

        Ok(())
    }

    /// Handle a player being killed.
    ///
    /// Called by the combat system or spell system when a player's health
    /// reaches 0. This is the entry point for the entire death flow.
    pub fn on_killed(
        &self,
        player_guid: ObjectGuid,
        killer_guid: Option<ObjectGuid>,
        _spell_id: Option<u32>,
        world: &World,
    ) -> Result<()> {
        let is_pvp_death = killer_guid.map_or(false, |g| g.is_player());

        // Get player data before modifications
        let player_data = world.systems.player.manager().with_player(player_guid, |player| {
            (
                player.race,
                player.map_id,
                player.movement.position,
                player.stats.max_health,
                player.power.max_mana(),
                player.zone_id,
            )
        });

        let (player_race, map_id, death_position, _max_health, _max_mana, zone_id) = match player_data {
            Some(data) => data,
            None => {
                warn!("Player {:?} not found for death handling", player_guid);
                return Ok(());
            }
        };

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Set death state and related fields
        world.systems.player.manager().with_player_mut(player_guid, |player| {
            // 1. Set death state
            player.death.death_state = DeathState::JustDied;
            player.death.death_timer_ms = CORPSE_REPOP_TIME_MS;
            player.death.is_pvp_death = is_pvp_death;

            // Advance the death-streak window. If the previous window is
            // still open we add another step to climb the ladder (up to the
            // 120s cap); otherwise we start a fresh window.
            let base = player.death.death_expire_time.max(now_secs);
            player.death.death_expire_time = base + DEATH_EXPIRE_STEP_SECS;

            // 2. Set health to 0
            player.stats.health = 0;

            // 3. Set unit flags
            player.unit_flags |= UNIT_FLAG_DISABLE_MOVE;

            // 4. Clear combat state
            player.combat.in_combat = false;
            player.combat.attack_target = None;
            player.combat.attackers.clear();
            player.combat.is_auto_attacking = false;

            // 5. Clear combo points
            player.combat.combo_points = 0;
            player.combat.combo_target = None;

            // 6. Clear any pending resurrection
            player.death.resurrection_data = None;

            // 7. Record death position for corpse
            player.death.corpse_position = Some(death_position);
            player.death.corpse_map_id = Some(map_id);

            // 8. Remove all non-passive auras (buffs, debuffs — not talents)
            let removed_auras = player.auras.container.remove_all_non_passive();
            if !removed_auras.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
                debug!("Removed {} auras on death for player {:?}", removed_auras.len(), player_guid);
            }
        });

        // 9. Apply durability loss (10% on equipment, not in BG, not from PvP,
        //    not from spells flagged SPELL_ATTR_EX3_NO_DURABILITY_LOSS).
        let in_bg = is_player_in_battleground(world, player_guid);
        let spell_prevents_loss = _spell_id
            .and_then(|sid| world.managers.spell_mgr.get(sid))
            .map(|entry| (entry.attributes_ex3 & SPELL_ATTR_EX3_NO_DURABILITY_LOSS) != 0)
            .unwrap_or(false);
        if durability::should_apply_durability_loss(in_bg, is_pvp_death, spell_prevents_loss) {
            self.apply_death_durability(player_guid, world);
        }

        // 10. Spawn the Corpse world object — see `CorpseManager::build_create_msg`.
        self.spawn_corpse(player_guid, is_pvp_death, world);

        // 11. Pick a self-resurrection spell (Reincarnation / Soulstone /
        //    Twisting Nether) and store it in PLAYER_SELF_RES_SPELL. The
        //    client lights up the self-res button on the death screen iff this
        //    field is non-zero. Consumed by `handle_self_res`.
        let self_res = self.select_resurrection_spell_id(player_guid, world);
        if self_res != 0 {
            world.systems.player.manager().with_player_mut(player_guid, |player| {
                player.self_res_spell = self_res;
            });
        }

        // 12. Send packets to the dead player
        self.send_death_packets(player_guid, is_pvp_death, death_position, map_id, zone_id, player_race, world)?;

        info!("Player {:?} killed by {:?}", player_guid, killer_guid);

        // 13. Award honor for PvP kill credit. Reward logic is async + may
        // touch the DB, so we spawn a task rather than blocking the tick.
        if is_pvp_death {
            let world_clone = world.clone();
            let victim = player_guid;
            let killer = killer_guid;
            tokio::spawn(async move {
                world_clone
                    .systems
                    .honor
                    .reward_honor_on_death(victim, killer, &world_clone)
                    .await;
            });
        }

        // 11. Advance transitional state
        world.systems.player.manager().with_player_mut(player_guid, |player| {
            player.death.death_state = DeathState::Corpse;
        });

        // 12. Broadcast the death update (health=0, flags) to nearby players
        self.broadcast_player_update(player_guid, world);

        Ok(())
    }

    /// Handle CMSG_REPOP_REQUEST - player clicks "Release Spirit".
    pub fn handle_release_spirit(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        // Validate state — accept both Corpse and JustDied (the latter can happen
        // if the client sends CMSG_REPOP_REQUEST before the world tick advances
        // JustDied -> Corpse, mirroring the VMaNGOS HandleRepopRequestOpcode guard).
        let death_state = world.systems.player.manager().with_player(player_guid, |player| {
            player.death.death_state
        }).unwrap_or(DeathState::Alive);

        match death_state {
            DeathState::Corpse => {} // normal path
            DeathState::JustDied => {
                // Advance to Corpse so the rest of the function works correctly
                world.systems.player.manager().with_player_mut(player_guid, |player| {
                    player.death.death_state = DeathState::Corpse;
                });
            }
            _ => {
                warn!("Player {:?} tried to release spirit but is not in Corpse/JustDied state ({:?})", player_guid, death_state);
                return Ok(());
            }
        }

        // Compute BG flag before we enter the write-locked closure (avoids
        // re-entering the player manager lock).
        let in_bg = is_player_in_battleground(world, player_guid);

        // Get player data for ghost form and set ghost state
        let player_data = world.systems.player.manager().with_player_mut(player_guid, |player| {
            let pending_spells = ghost::build_player_repop(
                player.race,
                &mut player.stats.health,
                &mut player.player_flags,
                &mut player.death.death_state,
            );

            // Clear the disable-move flag so the ghost can walk
            player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;

            // Apply ghost speed modifier (150% run speed in open world, 100% in BG).
            let speed_mult = ghost::get_ghost_speed_multiplier(in_bg);
            player.movement.run_speed = 7.0 * speed_mult;

            // Enable water walking (ghosts walk on water)
            player.movement.water_walking = true;
            player.movement.movement_flags |= MOVEMENT_FLAG_WATERWALKING;

            (
                player.race,
                player.map_id,
                player.movement.position,
                player.zone_id,
                pending_spells,
            )
        });

        let (_player_race, map_id, death_pos, zone_id, pending_spells) = match player_data {
            Some(data) => data,
            None => {
                warn!("Player {:?} not found for release spirit", player_guid);
                return Ok(());
            }
        };

        // Apply the queued ghost auras (8326 Ghost, optionally 20584 Wisp Spirit).
        // We use direct aura-container manipulation rather than SpellSystem::cast_spell
        // because this function is sync and the cast would otherwise be async. The
        // aura needs to exist server-side so visibility rules and the AURA_GHOST
        // check work — the client renders the ghost state from PLAYER_FLAGS_GHOST.
        for spell_id in &pending_spells {
            self.apply_ghost_aura(player_guid, *spell_id, world);
        }

        // Send SMSG_MOVE_WATER_WALK so the client lets the ghost walk on water.
        self.send_water_walk(player_guid, true, world);

        // Find graveyard and teleport
        self.teleport_to_graveyard(player_guid, death_pos, map_id, zone_id, _player_race, world)?;

        // Broadcast updated state (ghost flag, health=1) to nearby players
        self.broadcast_player_update(player_guid, world);

        info!("Player {:?} released spirit, now ghost at graveyard", player_guid);
        Ok(())
    }

    /// Handle CMSG_RECLAIM_CORPSE - player clicks "Resurrect" near corpse.
    pub fn handle_reclaim_corpse(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        // Validate state and proximity
        let can_reclaim = world.systems.player.manager().with_player(player_guid, |player| {
            if player.death.death_state != DeathState::Dead {
                return false;
            }
            // Check proximity to corpse
            if let Some(corpse_pos) = &player.death.corpse_position {
                let pos = player.movement.position;
                is_within_corpse_reclaim_range(
                    pos.x, pos.y, pos.z,
                    corpse_pos.x, corpse_pos.y, corpse_pos.z,
                )
            } else {
                false
            }
        }).unwrap_or(false);

        if !can_reclaim {
            warn!("Player {:?} cannot reclaim corpse (wrong state or too far)", player_guid);
            return Ok(());
        }

        // Resurrect at corpse location
        self.resurrect_player(player_guid, ResurrectionMethod::CorpseRun, world)?;

        info!("Player {:?} reclaimed corpse", player_guid);
        Ok(())
    }

    /// Handle CMSG_RESURRECT_RESPONSE - player accepts or declines a res spell.
    pub fn handle_resurrect_response(
        &self,
        player_guid: ObjectGuid,
        resurrector_guid: ObjectGuid,
        accept: bool,
        world: &World,
    ) -> Result<()> {
        if !accept {
            world.systems.player.manager().with_player_mut(player_guid, |player| {
                resurrect::decline_resurrection(&mut player.death);
            });
            debug!("Player {:?} declined resurrection from {:?}", player_guid, resurrector_guid);
            return Ok(());
        }

        // Validate the offer matches
        let valid = world.systems.player.manager().with_player(player_guid, |player| {
            resurrect::is_resurrection_requested_by(&player.death, resurrector_guid)
        }).unwrap_or(false);

        if !valid {
            warn!(
                "Player {:?} accepted resurrection from {:?} but no matching offer exists",
                player_guid, resurrector_guid
            );
            return Ok(());
        }

        // Execute resurrection from spell
        self.resurrect_player(player_guid, ResurrectionMethod::PlayerSpell, world)?;

        info!(
            "Player {:?} accepted resurrection from {:?}",
            player_guid, resurrector_guid
        );
        Ok(())
    }

    /// Handle spirit healer activation
    pub fn handle_spirit_healer(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        // Validate player is in ghost form
        let is_ghost = world.systems.player.manager().with_player(player_guid, |player| {
            player.death.death_state == DeathState::Dead
        }).unwrap_or(false);

        if !is_ghost {
            warn!("Player {:?} tried to use spirit healer but is not a ghost", player_guid);
            return Ok(());
        }

        // Apply additional 25% durability loss for spirit healer resurrection
        self.apply_spirit_healer_durability(player_guid, world);

        // Resurrect at spirit healer
        self.resurrect_player(player_guid, ResurrectionMethod::SpiritHealer, world)?;

        info!("Player {:?} resurrected at spirit healer", player_guid);
        Ok(())
    }

    /// Offer a resurrection to a dead player (called by spell system).
    pub fn offer_resurrection(
        &self,
        target_guid: ObjectGuid,
        caster_guid: ObjectGuid,
        caster_name: &str,
        location: Position,
        map_id: u32,
        instance_id: u32,
        health: u32,
        mana: u32,
        world: &World,
    ) -> Result<()> {
        world.systems.player.manager().with_player_mut(target_guid, |player| {
            resurrect::offer_resurrection(
                &mut player.death,
                caster_guid,
                location,
                map_id,
                instance_id,
                health,
                mana,
            );
        });

        // Send SMSG_RESURRECT_REQUEST to the dead player
        let packet = SmsgResurrectRequest {
            caster_guid,
            caster_name: caster_name.to_string(),
            is_pet: false,
        };
        self.broadcast_mgr.send_to_player(target_guid, packet.to_world_packet());

        debug!("Sent resurrection offer to player {:?} from {:?}", target_guid, caster_guid);
        Ok(())
    }

    /// Called every world tick. Updates death timers and handles auto-release.
    pub fn update(&self,
        diff: Duration,
        world: &World,
    ) -> Result<()> {
        let diff_ms = diff.as_millis() as u32;

        // Collect players needing auto-release
        let mut auto_release = Vec::new();

        world.systems.player.manager().for_each_player(|guid, player| {
            match player.death.death_state {
                DeathState::Corpse => {
                    // Tick the death timer (corpse expiry)
                    if tick_death_timer(&mut player.death, diff_ms) {
                        // Timer expired: auto-release spirit
                        auto_release.push(guid);
                    }
                }
                DeathState::JustAlived => {
                    // Transitional: advance to Alive on next tick
                    player.death.death_state = DeathState::Alive;
                }
                _ => {}
            }
        });

        // Process auto-releases (outside the player lock)
        for guid in auto_release {
            self.handle_release_spirit(guid, world)?;
        }

        // Age corpses: convert to bones after 6 min, remove bones after 3 days.
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expired = world.managers.corpse_mgr.find_expired(now_secs);
        for event in expired {
            use crate::world::game::corpse::manager::CorpseExpiration;
            match event {
                CorpseExpiration::ConvertToBones { guid, map_id, position, .. } => {
                    // Flip type to Bones in-memory.
                    world.managers.corpse_mgr.convert_to_bones(guid);
                    // Persist: update `corpse_type` to 0 (Bones).
                    let pool = Arc::new(world.databases.character.clone());
                    let counter = guid.counter();
                    tokio::spawn(async move {
                        use crate::shared::database::characters::repositories::CorpseRepository;
                        let repo = CorpseRepository::new(pool);
                        // Load + rewrite with corpse_type=0. We do it via save()
                        // with ON DUPLICATE KEY UPDATE so this is a single
                        // row upsert.
                        if let Ok(Some(mut row)) = repo.load_all().await.map(|mut v| {
                            v.retain(|r| r.guid == counter);
                            v.into_iter().next()
                        }) {
                            row.corpse_type = 0;
                            if let Err(e) = repo.save(&row).await {
                                warn!("Failed to persist bones conversion {}: {}", counter, e);
                            }
                        }
                    });
                    // Trigger a visibility refresh on nearby players (a partial
                    // SMSG_UPDATE_OBJECT would be ideal, but for now we rely
                    // on the next visibility tick to re-send the CREATE block
                    // with BONES flag set).
                    let _ = (map_id, position);
                }
                CorpseExpiration::Remove { guid, map_id, position } => {
                    self.despawn_corpse(guid, position, map_id, world);
                }
            }
        }

        Ok(())
    }

    /// Teleport player to nearest graveyard
    fn teleport_to_graveyard(
        &self,
        player_guid: ObjectGuid,
        death_pos: Position,
        map_id: u32,
        zone_id: u32,
        race: u8,
        world: &World,
    ) -> Result<()> {
        let team = graveyard::team_from_race(race);

        // Look up closest graveyard
        let gy = {
            let mgr = self.graveyard_mgr.read().unwrap();
            mgr.get_closest_graveyard(
                death_pos.x, death_pos.y, death_pos.z,
                map_id, zone_id, zone_id, // area_id = zone_id as fallback
                team,
            )
        };

        let gy = match gy {
            Some(gy) => gy,
            None => {
                warn!(
                    "No graveyard found for player {:?} in zone {} on map {} — using homebind",
                    player_guid, zone_id, map_id
                );
                // Fall back to homebind position
                let homebind = world.systems.player.manager().with_player(player_guid, |p| {
                    (p.homebind_map, Position {
                        x: p.homebind_x,
                        y: p.homebind_y,
                        z: p.homebind_z,
                        o: 0.0,
                    })
                });
                match homebind {
                    Some((hb_map, hb_pos)) => {
                        self.perform_ghost_teleport(player_guid, hb_map, hb_pos, world)?;
                        return Ok(());
                    }
                    None => return Ok(()),
                }
            }
        };

        let dest = Position {
            x: gy.position.x,
            y: gy.position.y,
            z: gy.position.z,
            o: 0.0,
        };

        info!(
            "Teleporting ghost {:?} to graveyard {} on map {} at ({:.1}, {:.1}, {:.1})",
            player_guid, gy.safe_loc_id, gy.map_id, dest.x, dest.y, dest.z
        );

        self.perform_ghost_teleport(player_guid, gy.map_id, dest, world)?;
        Ok(())
    }

    /// Execute the actual teleport for a ghost player.
    fn perform_ghost_teleport(
        &self,
        player_guid: ObjectGuid,
        dest_map: u32,
        dest_pos: Position,
        world: &World,
    ) -> Result<()> {
        // Get session for sending packets
        let session = match world.session_mgr.get_session_by_player(player_guid) {
            Some(s) => s,
            None => {
                warn!("No session found for player {:?} during ghost teleport", player_guid);
                return Ok(());
            }
        };

        // SMSG_TRANSFER_PENDING
        let mut transfer = WorldPacket::new(Opcode::SMSG_TRANSFER_PENDING);
        transfer.write_u32(dest_map);
        session.send_packet(transfer)?;

        // SMSG_NEW_WORLD
        let mut new_world = WorldPacket::new(Opcode::SMSG_NEW_WORLD);
        new_world.write_u32(dest_map);
        new_world.write_f32(dest_pos.x);
        new_world.write_f32(dest_pos.y);
        new_world.write_f32(dest_pos.z);
        new_world.write_f32(dest_pos.o);
        session.send_packet(new_world)?;

        // Store pending teleport so worldport_ack handler completes it
        session.set_pending_teleport(Some((dest_map, 0, dest_pos)));

        Ok(())
    }

    /// Send death-related packets to the player
    fn send_death_packets(
        &self,
        player_guid: ObjectGuid,
        is_pvp_death: bool,
        death_position: Position,
        map_id: u32,
        zone_id: u32,
        race: u8,
        world: &World,
    ) -> Result<()> {
        let team = graveyard::team_from_race(race);

        // Find closest graveyard for the release location packet
        let gy_pos = {
            let mgr = self.graveyard_mgr.read().unwrap();
            mgr.get_closest_graveyard(
                death_position.x, death_position.y, death_position.z,
                map_id, zone_id, zone_id,
                team,
            ).map(|gy| (gy.map_id, gy.position))
        };

        // SMSG_DEATH_RELEASE_LOC — tells client where graveyard is
        if let Some((gy_map, gy_pos)) = gy_pos {
            let release_loc = SmsgDeathReleaseLocation {
                map_id: gy_map,
                position: gy_pos,
            };
            self.broadcast_mgr.send_to_player(player_guid, release_loc.to_world_packet());
        } else {
            // No graveyard found — send death position as fallback
            let release_loc = SmsgDeathReleaseLocation {
                map_id,
                position: death_position,
            };
            self.broadcast_mgr.send_to_player(player_guid, release_loc.to_world_packet());
        }

        // SMSG_CORPSE_RECLAIM_DELAY — greys out "Resurrect" button.
        // Use the escalating-delay ladder based on the player's recent-death
        // window (vmangos behavior: 30s / 60s / 120s depending on how recently
        // the last death occurred).
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expire = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |p| p.death.death_expire_time)
            .unwrap_or(0);
        let delay_s = compute_reclaim_delay(expire, now_secs, is_pvp_death);
        let reclaim_delay = SmsgCorpseReclaimDelay {
            delay_ms: delay_s * 1000,
        };
        self.broadcast_mgr.send_to_player(player_guid, reclaim_delay.to_world_packet());

        debug!(
            "Sent death packets to player {:?}, reclaim delay: {}s",
            player_guid, delay_s
        );

        Ok(())
    }

    /// Resurrect a player using the specified method
    fn resurrect_player(
        &self,
        player_guid: ObjectGuid,
        method: ResurrectionMethod,
        world: &World,
    ) -> Result<()> {
        // Send SmsgPreResurrect to clear the death screen
        let pre_res = SmsgPreResurrect { player_guid };
        self.broadcast_mgr.send_to_player(player_guid, pre_res.to_world_packet());

        // Get resurrection data
        let res_data = world.systems.player.manager().with_player_mut(player_guid, |player| {
            let max_health = player.stats.max_health;
            let max_mana = player.power.max_mana();
            let level = player.level;
            let race = player.race;

            let (health, mana, teleport_pos, teleport_map) = match method {
                ResurrectionMethod::CorpseRun => {
                    let (h, m) = resurrect::resurrect_at_corpse(
                        &mut player.death,
                        max_health,
                        max_mana,
                    );
                    // Teleport to corpse position
                    let pos = player.death.corpse_position.unwrap_or(player.movement.position);
                    let map = player.death.corpse_map_id.unwrap_or(player.map_id);
                    (h, m, Some(pos), Some(map))
                }
                ResurrectionMethod::SpiritHealer => {
                    let (h, m) = resurrect::resurrect_at_spirit_healer(
                        &mut player.death,
                        max_health,
                        max_mana,
                    );
                    // Stay at current position (graveyard)
                    (h, m, None, None)
                }
                ResurrectionMethod::PlayerSpell => {
                    if let Some((h, m, pos, map)) = resurrect::resurrect_from_spell(
                        &mut player.death,
                    ) {
                        (h, m, Some(pos), Some(map))
                    } else {
                        (max_health / 2, max_mana / 2, None, None)
                    }
                }
                _ => {
                    // Other methods not yet implemented
                    (max_health / 2, max_mana / 2, None, None)
                }
            };

            // Set health and mana
            player.stats.health = health;
            player.power.set_mana(mana);

            // Remove ghost form; collect spell IDs whose auras we need to drop.
            let auras_to_remove = ghost::remove_ghost_form(player.race, &mut player.player_flags);

            // Restore normal run speed
            player.movement.run_speed = 7.0;

            // Clear water walking
            player.movement.water_walking = false;
            player.movement.movement_flags &= !MOVEMENT_FLAG_WATERWALKING;

            // Clear unit flags
            player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;

            // Capture and clear corpse data — we remove the world-object
            // *after* the lock is dropped to avoid re-entering the map's
            // grid lock under a player write lock.
            let corpse_info = match (player.death.corpse_guid, player.death.corpse_position, player.death.corpse_map_id) {
                (Some(guid), Some(pos), Some(map)) => Some((guid, pos, map)),
                _ => None,
            };
            player.death.corpse_position = None;
            player.death.corpse_map_id = None;
            player.death.corpse_guid = None;

            (teleport_pos, teleport_map, level, race, auras_to_remove, corpse_info)
        });

        let (teleport_pos, teleport_map, level, race, auras_to_remove, corpse_info) = match res_data {
            Some(data) => data,
            None => {
                warn!("Player {:?} not found for resurrection", player_guid);
                return Ok(());
            }
        };

        // Remove the corpse world object (visibility, map, manager).
        if let Some((corpse_guid, corpse_pos, corpse_map)) = corpse_info {
            self.despawn_corpse(corpse_guid, corpse_pos, corpse_map, world);
        }

        // Remove ghost auras (8326 Ghost, optionally 20584 Wisp Spirit) that we
        // applied during release-spirit. Must happen after the stat-recalc flag
        // is set so the client sees the cleared slot.
        for spell_id in &auras_to_remove {
            self.remove_ghost_aura(player_guid, *spell_id, world);
        }

        // Unset water walking on the client.
        self.send_water_walk(player_guid, false, world);

        // Teleport if needed (corpse run or player spell)
        if let (Some(pos), Some(map_id)) = (teleport_pos, teleport_map) {
            self.perform_ghost_teleport(player_guid, map_id, pos, world)?;
        }

        // Apply resurrection sickness if spirit healer and level >= threshold
        if method.applies_sickness() {
            let (spell_id, duration) = {
                let dbc = world.dbc.read();
                sickness::compute_resurrection_sickness(level, race, Some(&*dbc))
            };
            if spell_id != 0 && duration > 0 {
                let duration_ms = duration * 1000;
                debug!(
                    "Applying resurrection sickness (spell {}) for {}s to player {:?}",
                    spell_id, duration, player_guid
                );
                self.apply_resurrection_sickness(player_guid, spell_id, duration_ms, world);
            }
        }

        // Broadcast the resurrection update (health restored, ghost flag cleared)
        self.broadcast_player_update(player_guid, world);

        Ok(())
    }

    /// Apply resurrection sickness aura directly to the player's aura container.
    ///
    /// Spell 15007 has two effects:
    ///   Effect 0: AURA_MOD_TOTAL_STAT_PERCENTAGE (misc=-1 all stats, value=-75)
    ///   Effect 1: AURA_MOD_DAMAGE_PERCENT_DONE (misc=127 all schools, value=-75)
    fn apply_resurrection_sickness(
        &self,
        player_guid: ObjectGuid,
        spell_id: u32,
        duration_ms: u32,
        world: &World,
    ) {
        use crate::world::game::player::auras::aura::{Aura, AuraFlags};
        use crate::world::game::player::auras::effects;

        world.systems.player.manager().with_player_mut(player_guid, |player| {
            let negative_flags = AuraFlags {
                is_positive: false,
                is_negative: true,
                is_passive: false,
                can_be_cancelled: false,
                is_hidden: false,
                is_permanent: false,
            };

            // Effect 0: -75% all stats
            let stat_aura = Aura::new(
                spell_id,
                player_guid,     // self-applied
                0,               // effect_index
                effects::AURA_MOD_TOTAL_STAT_PERCENTAGE,
                -1,              // misc_value: -1 = all stats
                -75,             // base_value: -75%
                Some(duration_ms),
                0,               // no periodic
                1,               // max stacks
                0,               // no charges
                negative_flags,
            );

            // Effect 1: -75% damage done
            let damage_aura = Aura::new(
                spell_id,
                player_guid,
                1,               // effect_index
                effects::AURA_MOD_DAMAGE_PERCENT_DONE,
                127,             // misc_value: all school mask
                -75,             // base_value: -75%
                Some(duration_ms),
                0,
                1,
                0,
                negative_flags,
            );

            player.auras.container.add_aura(stat_aura);
            player.auras.container.add_aura(damage_aura);
            player.auras.needs_client_update = true;
            player.auras.needs_stat_recalc = true;
        });
    }

    /// Pick a self-resurrection spell id at death time, vmangos-style.
    ///
    /// Priority (matches `SelectResurrectionSpellId` at vmangos Player.cpp:19905):
    ///   1. Warlock Soulstone — a dummy aura on the player maps to a rez spell.
    ///      Each soulstone rank has its own aura→rez-spell pairing.
    ///   2. Shaman Reincarnation (spell 20608 learned, 21169 off-cooldown,
    ///      and shamanic-focus item 17030 in inventory).
    ///   3. Twisting Nether (warlock talent proc, spell 23701 aura → 23700).
    ///
    /// Returns 0 if the player has no valid self-res option.
    fn select_resurrection_spell_id(&self, player_guid: ObjectGuid, world: &World) -> u32 {
        // Soulstone aura → resurrection spell mapping (ranks 1-5).
        const SOULSTONE_MAP: &[(u32, u32)] = &[
            (20707, 3026),  // Soulstone rank 1
            (20762, 20758), // rank 2
            (20763, 20759), // rank 3
            (20764, 20760), // rank 4
            (20765, 20761), // rank 5
        ];

        // Check soulstone auras first.
        let soulstone = world.systems.player.manager().with_player(player_guid, |player| {
            for (aura_spell, rez_spell) in SOULSTONE_MAP {
                if player.auras.container.has_aura(*aura_spell) {
                    return *rez_spell;
                }
            }
            0u32
        }).unwrap_or(0);
        if soulstone != 0 {
            return soulstone;
        }

        // Twisting Nether (Warlock) — aura 23701 → rez 23700.
        let twisting = world.systems.player.manager().with_player(player_guid, |player| {
            if player.auras.container.has_aura(23701) { 23700 } else { 0 }
        }).unwrap_or(0);
        if twisting != 0 {
            return twisting;
        }

        // Reincarnation (Shaman): requires spell 20608 learned, 21169 not on
        // cooldown. Item check (17030 Ankh) is simplified for now — the real
        // vmangos path consumes one Ankh from inventory on activation.
        let has_reincarn = world.systems.player.manager().with_player(player_guid, |player| {
            player.spells.learned_spells.contains(&20608)
        }).unwrap_or(false);
        if has_reincarn {
            // 21169 is the rez spell id. If it's off cooldown, return it.
            if let Ok(false) = crate::world::game::player::spells::cooldowns::is_on_cooldown(
                player_guid, 21169, world,
            ) {
                return 21169;
            }
        }

        0
    }

    /// Spawn the Corpse world object at the player's death location. Creates
    /// the Corpse struct, registers it with `CorpseManager`, and inserts it
    /// into the map's grid so visibility picks it up.
    fn spawn_corpse(&self, player_guid: ObjectGuid, is_pvp_death: bool, world: &World) {
        use crate::world::game::player::death::corpse::create_corpse_from_player;

        // Pull appearance + equipment + position info.
        let snapshot = world.systems.player.manager().with_player(player_guid, |player| {
            (
                player.race,
                player.gender,
                player.skin,
                player.face,
                player.hair_style,
                player.hair_color,
                player.facial_hair,
                player.map_id,
                player.instance_id,
                player.movement.position,
            )
        });
        let (race, gender, skin, face, hair_style, hair_color, facial_hair, map_id, instance_id, pos) = match snapshot {
            Some(v) => v,
            None => return,
        };

        // Pull 19 equipment display IDs from the inventory cache.
        let mut equipment_display: [u32; 19] = [0; 19];
        let cache = world.systems.inventory.cache();
        for (slot, item_guid) in cache.get_equipment_slots(player_guid) {
            if (slot as usize) >= 19 {
                continue;
            }
            if let Some(item_lock) = cache.get_item(player_guid, item_guid) {
                let entry = item_lock.read().entry;
                if let Some(tpl) = world.managers.item_mgr.get_template(entry) {
                    equipment_display[slot as usize] = tpl.display_id;
                }
            }
        }

        // Allocate a corpse GUID and build the struct.
        let corpse_guid = world.managers.corpse_mgr.alloc_corpse_guid();
        let corpse = create_corpse_from_player(
            corpse_guid,
            player_guid,
            pos,
            map_id,
            instance_id,
            is_pvp_death,
            skin,
            face,
            hair_style,
            hair_color,
            facial_hair,
            gender,
            race,
            equipment_display,
        );

        // Register with manager + map.
        world.managers.corpse_mgr.add(corpse.clone());
        let map = world.managers.map_mgr.get_or_create_map(map_id, instance_id);
        map.add_corpse(corpse_guid, pos);

        // Persist to the `corpse` DB table so the corpse survives relog.
        // This is fire-and-forget — a failed write shouldn't block the death.
        let pool = Arc::new(world.databases.character.clone());
        let row = corpse_to_row(&corpse);
        tokio::spawn(async move {
            use crate::shared::database::characters::repositories::CorpseRepository;
            let repo = CorpseRepository::new(pool);
            if let Err(e) = repo.save(&row).await {
                warn!("Failed to persist corpse {:?}: {}", row.guid, e);
            }
        });

        // Wire the GUID back onto the player so reclaim/resurrect can find it.
        world.systems.player.manager().with_player_mut(player_guid, |player| {
            player.death.corpse_guid = Some(corpse_guid);
        });

        debug!(
            "Spawned corpse {:?} for player {:?} on map {} at ({:.1},{:.1},{:.1})",
            corpse_guid, player_guid, map_id, pos.x, pos.y, pos.z
        );
    }

    /// Remove a corpse world object. Mirrors `spawn_corpse`.
    fn despawn_corpse(
        &self,
        corpse_guid: ObjectGuid,
        corpse_pos: Position,
        corpse_map: u32,
        world: &World,
    ) {
        if let Some(map) = world.managers.map_mgr.get_map(corpse_map, 0) {
            map.remove_corpse(corpse_guid, corpse_pos);
        }
        world.managers.corpse_mgr.remove(corpse_guid);

        // Delete from DB.
        let pool = Arc::new(world.databases.character.clone());
        let counter = corpse_guid.counter();
        tokio::spawn(async move {
            use crate::shared::database::characters::repositories::CorpseRepository;
            let repo = CorpseRepository::new(pool);
            if let Err(e) = repo.delete(counter).await {
                warn!("Failed to delete persisted corpse {}: {}", counter, e);
            }
        });

        debug!("Despawned corpse {:?}", corpse_guid);
    }

    /// Load every corpse from the DB at startup and rehydrate CorpseManager +
    /// Map grids. Appearance/equipment are filled with defaults; a corpse
    /// loaded from disk is a placeholder body — enough for visibility + reclaim.
    pub async fn load_corpses(&self, world: &World) -> Result<()> {
        use crate::shared::database::characters::repositories::CorpseRepository;
        use crate::world::game::player::death::corpse::{Corpse, CorpseType};

        let pool = Arc::new(world.databases.character.clone());
        let repo = CorpseRepository::new(pool);
        let rows = repo.load_all().await?;

        let mut max_counter: u32 = 0;
        for row in rows {
            max_counter = max_counter.max(row.guid);

            let owner_guid = ObjectGuid::new_player(row.player_guid);
            let corpse_guid = ObjectGuid::new_corpse(row.guid);
            let pos = Position {
                x: row.position_x,
                y: row.position_y,
                z: row.position_z,
                o: row.orientation,
            };
            let corpse = Corpse {
                guid: corpse_guid,
                owner_guid,
                position: pos,
                map_id: row.map,
                instance_id: row.instance,
                corpse_type: CorpseType::from(row.corpse_type as u32),
                created_time: row.time,
                // Appearance/equipment are reconstructed lazily if/when the
                // owner logs in (Phase 9). Default to 0/empty for now.
                skin: 0,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_style: 0,
                gender: 0,
                race: 1, // fallback human
                equipment: [0; 19],
            };

            world.managers.corpse_mgr.add(corpse);
            let map = world.managers.map_mgr.get_or_create_map(row.map, row.instance);
            map.add_corpse(corpse_guid, pos);
        }

        // Advance the allocator past the highest stored counter so new
        // corpses don't collide with rehydrated ones.
        world.managers.corpse_mgr.bump_counter(max_counter);
        Ok(())
    }

    /// Apply a ghost-related aura (spell 8326 Ghost, spell 20584 Wisp Spirit)
    /// by directly inserting an AURA_GHOST entry into the player's aura
    /// container. We can't go through `SpellSystem::cast_spell` here because
    /// this code path is sync. MaNGOS-style vanilla clients don't stream
    /// per-aura packets (no SMSG_AURA_UPDATE in 1.12); the aura is read via
    /// UNIT_FIELD_AURA during the next visibility update.
    fn apply_ghost_aura(&self, player_guid: ObjectGuid, spell_id: u32, world: &World) {
        use crate::world::game::player::auras::aura::{Aura, AuraFlags};
        use crate::world::game::player::auras::effects;

        world.systems.player.manager().with_player_mut(player_guid, |player| {
            let flags = AuraFlags {
                is_positive: true,
                is_negative: false,
                is_passive: true,
                can_be_cancelled: false,
                is_hidden: false,
                is_permanent: true,
            };

            let aura = Aura::new(
                spell_id,
                player_guid, // self-applied
                0,           // effect_index 0
                effects::AURA_GHOST,
                0,  // misc_value: unused for AURA_GHOST
                0,  // base_value: unused
                None, // permanent until removed
                0,  // no periodic
                1,  // max stacks
                0,  // no charges
                flags,
            );

            player.auras.container.add_aura(aura);
            player.auras.needs_client_update = true;
        });
    }

    /// Remove a ghost aura from the player's container. Mirrors
    /// `apply_ghost_aura` — used during resurrection to clear the ghost state.
    fn remove_ghost_aura(&self, player_guid: ObjectGuid, spell_id: u32, world: &World) {
        world.systems.player.manager().with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_aura(spell_id, 0);
            if removed.is_some() {
                player.auras.needs_client_update = true;
            }
        });
    }

    /// Send SMSG_MOVE_WATER_WALK / SMSG_MOVE_LAND_WALK to let the client
    /// render water-walking. Uses packed GUID + counter 0.
    fn send_water_walk(&self, player_guid: ObjectGuid, enable: bool, _world: &World) {
        let opcode = if enable {
            Opcode::SMSG_MOVE_WATER_WALK
        } else {
            Opcode::SMSG_MOVE_LAND_WALK
        };
        let mut packet = WorldPacket::new(opcode);
        packet.write_packed_guid_raw(player_guid.raw());
        packet.write_u32(0); // counter
        self.broadcast_mgr.send_to_player(player_guid, packet);
    }

    /// Apply 10% durability loss on death to all equipped items.
    /// Reads equipment from inventory cache, applies loss, writes back.
    fn apply_death_durability(&self, player_guid: ObjectGuid, world: &World) {
        let cache = world.systems.inventory.cache();
        let equipment_slots = cache.get_equipment_slots(player_guid);

        // Build (current, max) slice from equipped items
        let mut items_with_durability: Vec<(ObjectGuid, u32, u32)> = Vec::new();
        for (_slot, item_guid) in &equipment_slots {
            if let Some(item_lock) = cache.get_item(player_guid, *item_guid) {
                let item = item_lock.read();
                if item.max_durability > 0 {
                    items_with_durability.push((item.guid, item.durability, item.max_durability));
                }
            }
        }

        // Apply 10% loss using existing function
        let mut durability_pairs: Vec<(u32, u32)> = items_with_durability
            .iter()
            .map(|(_, cur, max)| (*cur, *max))
            .collect();
        let affected = durability::apply_death_durability_loss(&mut durability_pairs);

        // Write back new durability values
        for (i, (guid, _, _)) in items_with_durability.iter().enumerate() {
            let new_dur = durability_pairs[i].0;
            if let Some(item_lock) = cache.get_item(player_guid, *guid) {
                let mut item = item_lock.write();
                item.durability = new_dur;
            }
        }

        if affected > 0 {
            // Send SMSG_DURABILITY_DAMAGE_DEATH to notify client
            self.broadcast_mgr
                .send_to_player(player_guid, SmsgDurabilityDamageDeath.to_world_packet());
            debug!(
                "Applied death durability loss to {} items for player {:?}",
                affected, player_guid
            );
        }
    }

    /// Apply 25% additional durability loss for spirit healer resurrection.
    fn apply_spirit_healer_durability(&self, player_guid: ObjectGuid, world: &World) {
        let cache = world.systems.inventory.cache();
        let equipment_slots = cache.get_equipment_slots(player_guid);

        let mut items_with_durability: Vec<(ObjectGuid, u32, u32)> = Vec::new();
        for (_slot, item_guid) in &equipment_slots {
            if let Some(item_lock) = cache.get_item(player_guid, *item_guid) {
                let item = item_lock.read();
                if item.max_durability > 0 {
                    items_with_durability.push((item.guid, item.durability, item.max_durability));
                }
            }
        }

        let mut durability_pairs: Vec<(u32, u32)> = items_with_durability
            .iter()
            .map(|(_, cur, max)| (*cur, *max))
            .collect();
        let affected = durability::apply_spirit_healer_durability_loss(&mut durability_pairs);

        for (i, (guid, _, _)) in items_with_durability.iter().enumerate() {
            let new_dur = durability_pairs[i].0;
            if let Some(item_lock) = cache.get_item(player_guid, *guid) {
                let mut item = item_lock.write();
                item.durability = new_dur;
            }
        }

        if affected > 0 {
            debug!(
                "Applied spirit healer durability loss to {} items for player {:?}",
                affected, player_guid
            );
        }
    }

    /// Trigger a visibility refresh for a player.
    ///
    /// This forces the player's updated fields (health, flags) to be sent
    /// to nearby players on the next visibility tick.
    fn broadcast_player_update(&self, player_guid: ObjectGuid, world: &World) {
        // Mark the player's visibility as dirty + force_immediate so the
        // visibility system re-evaluates who can see this player and sends
        // updated create/destroy packets on the next tick.
        world.systems.player.manager().with_player_mut(player_guid, |player| {
            player.visibility.dirty = true;
            player.visibility.force_immediate = true;
        });
    }
}
