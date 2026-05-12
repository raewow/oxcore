//! Trainer packet handlers

use anyhow::Result;
use tracing::{info, warn};

use crate::shared::messages::spells::{SmsgPlaySpellVisual, SmsgSpellGo, SmsgSpellStart};
use crate::world::game::broadcast_mgr::broadcast_around_creature;
use crate::shared::messages::trainer::{
    SmsgTrainerBuyFailed, SmsgTrainerBuySucceeded, SmsgTrainerList, TrainerBuyError,
    TrainerSpellData,
};
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::game::npc::trainer::types::TrainerSpellState;
use crate::world::World;

/// Handle CMSG_TRAINER_LIST (0x1B0)
pub async fn handle_trainer_list(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let trainer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read trainer GUID"))?;

    info!(
        "CMSG_TRAINER_LIST: player={:?}, trainer={:?}",
        player_guid, trainer_guid
    );

    send_trainer_list(player_guid, trainer_guid, world).await
}

/// Send SMSG_TRAINER_LIST to a player for a given trainer NPC.
/// Called from CMSG_TRAINER_LIST handler and direct gossip-hello auto-open.
pub async fn send_trainer_list(
    player_guid: ObjectGuid,
    trainer_guid: ObjectGuid,
    world: &World,
) -> Result<()> {
    // Get creature entry and trainer_type
    let creature_info = world
        .managers
        .creature_mgr
        .get_creature(trainer_guid)
        .and_then(|c| {
            world
                .managers
                .creature_mgr
                .get_template(c.entry)
                .map(|t| (c.entry, t.trainer_type))
        });

    let (entry, trainer_type) = match creature_info {
        Some(info) => info,
        None => {
            warn!("send_trainer_list: trainer {:?} not found", trainer_guid);
            return Ok(());
        }
    };

    // Get trainer spells
    let all_spells = world.systems.trainer_manager.get_trainer_spells(entry);
    info!("send_trainer_list: entry={} trainer_type={} spells_in_db={}", entry, trainer_type, all_spells.len());
    if all_spells.is_empty() {
        warn!("send_trainer_list: no spells for trainer entry {}", entry);
    }

    // Build per-spell data with state relative to this player
    let spell_data: Vec<TrainerSpellData> = {
        let player_mgr = world.systems.player.manager();

        all_spells
            .iter()
            .filter_map(|ts| {
                // Look up the teaching spell entry to get the trigger spell
                // npc_trainer.spell is a "teaching" spell with SPELL_EFFECT_LEARN_SPELL
                // EffectTriggerSpell[0] is the actual spell being learned
                let teach_entry = world.managers.spell_mgr.get(ts.spell_id)?;
                let trigger_spell_id = teach_entry.effect_trigger_spell[0];

                // Get trigger spell entry for level / chain data
                let trigger_entry = world.managers.spell_mgr.get(trigger_spell_id)?;

                // req_level: use DB value if set, otherwise use trigger spell's own level
                let req_level = if ts.req_level > 0 {
                    ts.req_level
                } else {
                    trigger_entry.spell_level as u8
                };

                // Skip spell if no valid level (same as VMaNGOS: else return;)
                if req_level == 0 {
                    return None;
                }

                // State check uses the TRIGGER spell (the one the player actually learns)
                let state = player_mgr.with_player(player_guid, |player| {
                    if player.spells.knows_spell(trigger_spell_id) {
                        return TrainerSpellState::Gray;
                    }
                    if player.level < req_level {
                        return TrainerSpellState::Red;
                    }
                    if ts.req_skill != 0 {
                        let skill_val = player
                            .skills
                            .skills
                            .get(&ts.req_skill)
                            .map(|s| s.current_value)
                            .unwrap_or(0);
                        if skill_val < ts.req_skill_value {
                            return TrainerSpellState::Red;
                        }
                    }
                    TrainerSpellState::Green
                })?;

                Some(TrainerSpellData {
                    // Send the teaching spell id (npc_trainer.spell) — same as VMaNGOS tSpell->spell
                    spell_id: ts.spell_id,
                    state: state as u8,
                    cost: ts.cost,
                    primary_prof_first_rank_available: 0,
                    primary_prof_first_rank: 0,
                    req_level,
                    req_skill: ts.req_skill as u32,
                    req_skill_value: ts.req_skill_value as u32,
                    req_spell_1: 0,
                    req_spell_2: 0,
                    unknown: 0,
                })
            })
            .collect()
    };

    info!("send_trainer_list: sending {} spells to player {:?}", spell_data.len(), player_guid);
    for sd in &spell_data {
        info!("  spell_id={} state={} cost={} req_level={} req_skill={} req_skill_value={}",
            sd.spell_id, sd.state, sd.cost, sd.req_level, sd.req_skill, sd.req_skill_value);
    }

    let msg = SmsgTrainerList {
        trainer_guid,
        trainer_type: trainer_type as u32,
        spells: spell_data,
        greeting: "Hello! I can train you.".to_string(),
    };

    // Log raw packet bytes for debugging
    let raw = crate::shared::messages::ToWorldPacket::to_world_packet(&msg);
    let raw_bytes = raw.data();
    let preview_len = raw_bytes.len().min(80);
    info!("send_trainer_list: SMSG_TRAINER_LIST size={} bytes preview: {:02X?}", raw_bytes.len(), &raw_bytes[..preview_len]);

    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, msg);

    Ok(())
}

/// Handle CMSG_TRAINER_BUY_SPELL (0x1B2)
pub async fn handle_trainer_buy_spell(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let trainer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read trainer GUID"))?;

    let spell_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read spell ID"))?;

    info!(
        "CMSG_TRAINER_BUY_SPELL: player={:?}, trainer={:?}, spell={}",
        player_guid, trainer_guid, spell_id
    );

    // Get trainer creature entry
    let entry = match world
        .managers
        .creature_mgr
        .get_creature(trainer_guid)
        .map(|c| c.entry)
    {
        Some(e) => e,
        None => {
            warn!(
                "CMSG_TRAINER_BUY_SPELL: trainer {:?} not found",
                trainer_guid
            );
            send_buy_failed(world, player_guid, trainer_guid, spell_id, TrainerBuyError::Unavailable);
            return Ok(());
        }
    };

    // Find the spell in trainer's list
    let all_spells = world.systems.trainer_manager.get_trainer_spells(entry);
    let trainer_spell = match all_spells.iter().find(|s| s.spell_id == spell_id) {
        Some(s) => s.clone(),
        None => {
            warn!(
                "CMSG_TRAINER_BUY_SPELL: spell {} not offered by trainer entry {}",
                spell_id, entry
            );
            send_buy_failed(world, player_guid, trainer_guid, spell_id, TrainerBuyError::Unavailable);
            return Ok(());
        }
    };

    // Resolve the actual spell to learn (teaching spell -> trigger spell)
    let trigger_spell_id = match world.managers.spell_mgr.get(spell_id) {
        Some(e) => e.effect_trigger_spell[0],
        None => {
            warn!("CMSG_TRAINER_BUY_SPELL: teaching spell {} not in DBC", spell_id);
            send_buy_failed(world, player_guid, trainer_guid, spell_id, TrainerBuyError::Unavailable);
            return Ok(());
        }
    };

    // Determine effective req_level (same logic as send_trainer_list)
    let effective_req_level = if trainer_spell.req_level > 0 {
        trainer_spell.req_level
    } else {
        world.managers.spell_mgr.get(trigger_spell_id)
            .map(|e| e.spell_level as u8)
            .unwrap_or(0)
    };

    // Validate requirements (read-only pass)
    let validation_err = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            if player.spells.knows_spell(trigger_spell_id) {
                return Some(TrainerBuyError::Unavailable);
            }
            if player.level < effective_req_level {
                return Some(TrainerBuyError::Unavailable);
            }
            if trainer_spell.req_skill != 0 {
                let skill_val = player
                    .skills
                    .skills
                    .get(&trainer_spell.req_skill)
                    .map(|s| s.current_value)
                    .unwrap_or(0);
                if skill_val < trainer_spell.req_skill_value {
                    return Some(TrainerBuyError::SkillNotMet);
                }
            }
            None
        });

    match validation_err {
        None => {
            warn!(
                "CMSG_TRAINER_BUY_SPELL: player {:?} not found",
                player_guid
            );
            send_buy_failed(world, player_guid, trainer_guid, spell_id, TrainerBuyError::Unavailable);
            return Ok(());
        }
        Some(Some(err)) => {
            send_buy_failed(world, player_guid, trainer_guid, spell_id, err);
            return Ok(());
        }
        Some(None) => {} // All checks passed
    }

    // Check money via inventory system (authoritative)
    let player_money = world.systems.inventory.get_money(player_guid).unwrap_or(0);
    if player_money < trainer_spell.cost {
        send_buy_failed(
            world,
            player_guid,
            trainer_guid,
            spell_id,
            TrainerBuyError::NotEnoughMoney,
        );
        return Ok(());
    }

    // Deduct money via inventory system (sends client update)
    world
        .systems
        .inventory
        .remove_gold(player_guid, trainer_spell.cost);

    // Learn the trigger spell (the actual spell, not the teaching wrapper)
    world
        .systems
        .spells
        .learn_spell(player_guid, trigger_spell_id, world)
        .await?;

    // Send SMSG_TRAINER_BUY_SUCCEEDED
    let msg = SmsgTrainerBuySucceeded {
        trainer_guid,
        spell_id,
    };
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, msg);

    // Send teaching spell cast animation (SMSG_SPELL_START + SMSG_SPELL_GO + SMSG_PLAY_SPELL_VISUAL).
    // VMaNGOS: SpellVisual == 222 means the player is the caster; otherwise the trainer NPC casts on
    // the player.  We look up spell_visual from the teaching spell's DBC entry.
    let spell_visual = world
        .managers
        .spell_mgr
        .get(spell_id)
        .map(|e| e.spell_visual)
        .unwrap_or(0);
    send_trainer_spell_animation(world, player_guid, trainer_guid, spell_id, spell_visual);

    info!(
        "Player {:?} learned spell {} from trainer entry {} for {} copper",
        player_guid, spell_id, entry, trainer_spell.cost
    );

    Ok(())
}

fn send_buy_failed(
    world: &World,
    player_guid: ObjectGuid,
    trainer_guid: ObjectGuid,
    spell_id: u32,
    error: TrainerBuyError,
) {
    let msg = SmsgTrainerBuyFailed {
        trainer_guid,
        spell_id,
        error,
    };
    world
        .managers
        .broadcast_mgr
        .send_msg_to_player(player_guid, msg);
}

/// The three packets emitted for a trainer spell animation plus whether the player
/// or the trainer NPC is the caster.
pub(crate) struct TrainerAnimPackets {
    pub caster_is_player: bool,
    pub spell_start: WorldPacket,
    pub spell_go: WorldPacket,
    pub spell_visual: WorldPacket,
}

/// Build the three animation packets for a trainer-teach cast.
/// VMaNGOS: SpellVisual == 222 → player is caster; otherwise the trainer NPC casts on the player.
pub(crate) fn build_trainer_anim_packets(
    player_guid: ObjectGuid,
    trainer_guid: ObjectGuid,
    spell_id: u32,
    spell_visual: u32,
) -> TrainerAnimPackets {
    const VISUAL_SELF_CAST: u32 = 222;

    let caster_is_player = spell_visual == VISUAL_SELF_CAST;
    let caster_guid = if caster_is_player { player_guid } else { trainer_guid };

    let spell_start = SmsgSpellStart {
        caster_guid,
        caster_guid_pack: caster_guid,
        spell_id,
        cast_flags: 0,
        cast_time_ms: 0,
        target_guid: Some(player_guid),
        cast_item_guid: None,
    }
    .to_world_packet();

    let spell_go = SmsgSpellGo {
        caster_guid,
        caster_guid_pack: caster_guid,
        spell_id,
        cast_flags: 0,
        hit_targets: vec![player_guid],
        miss_targets: vec![],
        target_guid: Some(player_guid),
        cast_item_guid: None,
    }
    .to_world_packet();

    let spell_visual = SmsgPlaySpellVisual {
        caster_guid,
        spell_visual_kit_id: spell_visual,
    }
    .to_world_packet();

    TrainerAnimPackets { caster_is_player, spell_start, spell_go, spell_visual }
}

/// Send SMSG_SPELL_START + SMSG_SPELL_GO + SMSG_PLAY_SPELL_VISUAL for the trainer teaching
/// animation.  Mirrors VMaNGOS HandleTrainerBuySpellOpcode: if spell_visual == 222 the player
/// is the caster (self-cast), otherwise the trainer NPC casts on the player.
fn send_trainer_spell_animation(
    world: &World,
    player_guid: ObjectGuid,
    trainer_guid: ObjectGuid,
    spell_id: u32,
    spell_visual: u32,
) {
    let pkts = build_trainer_anim_packets(player_guid, trainer_guid, spell_id, spell_visual);

    if pkts.caster_is_player {
        // Player is caster — broadcast from player (include self so the player sees it too)
        world.managers.broadcast_mgr.broadcast_nearby(player_guid, &pkts.spell_start, true);
        world.managers.broadcast_mgr.broadcast_nearby(player_guid, &pkts.spell_go, true);
        world.managers.broadcast_mgr.broadcast_nearby(player_guid, &pkts.spell_visual, true);
    } else {
        // Trainer NPC is caster — broadcast from the creature's position
        broadcast_around_creature(world, trainer_guid, &pkts.spell_start);
        broadcast_around_creature(world, trainer_guid, &pkts.spell_go);
        broadcast_around_creature(world, trainer_guid, &pkts.spell_visual);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{HighGuid, Opcode};

    fn player_guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, 1)
    }

    fn trainer_guid() -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Unit, 2)
    }

    fn read_u32_le(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }

    fn read_u64_le(data: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
    }

    // Decode a packed GUID, returning (raw_guid, bytes_consumed).
    fn decode_packed_guid(data: &[u8]) -> (u64, usize) {
        let mask = data[0] as u64;
        let mut result = 0u64;
        let mut pos = 1usize;
        for bit in 0..8u64 {
            if mask & (1 << bit) != 0 {
                result |= (data[pos] as u64) << (bit * 8);
                pos += 1;
            }
        }
        (result, pos)
    }

    // ── caster selection ──────────────────────────────────────────────────────

    /// spell_visual == 222: player is the caster, broadcast path uses player broadcaster.
    #[test]
    fn visual_222_selects_player_as_caster() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 9999, 222);
        assert!(pkts.caster_is_player, "spell_visual 222 must set caster_is_player");
    }

    /// Any other spell_visual: trainer NPC is the caster.
    #[test]
    fn non_222_selects_trainer_as_caster() {
        for visual in [0u32, 1, 221, 223, 300, 9999] {
            let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 1, visual);
            assert!(!pkts.caster_is_player, "visual {visual} must use trainer as caster");
        }
    }

    // ── SMSG_PLAY_SPELL_VISUAL packet layout ──────────────────────────────────

    /// Opcode must be SMSG_PLAY_SPELL_VISUAL (0x1F3 = 499).
    #[test]
    fn play_spell_visual_opcode() {
        use crate::shared::messages::spells::SmsgPlaySpellVisual;
        let pkt = SmsgPlaySpellVisual {
            caster_guid: player_guid(),
            spell_visual_kit_id: 42,
        }
        .to_world_packet();
        assert_eq!(pkt.opcode(), Opcode::SMSG_PLAY_SPELL_VISUAL);
    }

    /// Packet body: full 8-byte GUID then 4-byte kit ID (little-endian).
    #[test]
    fn play_spell_visual_body_layout() {
        use crate::shared::messages::spells::SmsgPlaySpellVisual;
        let guid = player_guid();
        let kit_id: u32 = 0xBEEF;
        let pkt = SmsgPlaySpellVisual { caster_guid: guid, spell_visual_kit_id: kit_id }
            .to_world_packet();
        let data = pkt.data();

        assert_eq!(data.len(), 12, "SMSG_PLAY_SPELL_VISUAL must be exactly 12 bytes (8 GUID + 4 kit)");
        assert_eq!(read_u64_le(data, 0), guid.raw(), "first 8 bytes must be raw GUID");
        assert_eq!(read_u32_le(data, 8), kit_id, "bytes 8-11 must be SpellVisualKit ID");
    }

    // ── spell_visual packet caster fields ─────────────────────────────────────

    /// When spell_visual == 222 the SMSG_PLAY_SPELL_VISUAL caster GUID is the player's GUID.
    #[test]
    fn visual_pkt_caster_is_player_for_222() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 5, 222);
        let data = pkts.spell_visual.data();
        assert_eq!(read_u64_le(data, 0), player_guid().raw(),
            "SMSG_PLAY_SPELL_VISUAL caster must be player when spell_visual == 222");
    }

    /// When spell_visual != 222 the SMSG_PLAY_SPELL_VISUAL caster GUID is the trainer's GUID.
    #[test]
    fn visual_pkt_caster_is_trainer_for_other_visuals() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 5, 300);
        let data = pkts.spell_visual.data();
        assert_eq!(read_u64_le(data, 0), trainer_guid().raw(),
            "SMSG_PLAY_SPELL_VISUAL caster must be trainer when spell_visual != 222");
    }

    // ── SMSG_SPELL_START / SMSG_SPELL_GO caster fields ───────────────────────

    /// For spell_visual == 222 (self-cast) SMSG_SPELL_START opens with the player's packed GUID.
    #[test]
    fn spell_start_self_cast_caster_is_player() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 100, 222);
        let data = pkts.spell_start.data();
        let (first_guid, _) = decode_packed_guid(data);
        assert_eq!(first_guid, player_guid().raw(),
            "SMSG_SPELL_START first packed GUID must be player when self-cast");
    }

    /// For spell_visual != 222 SMSG_SPELL_START opens with the trainer's packed GUID.
    #[test]
    fn spell_start_trainer_cast_caster_is_trainer() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 100, 0);
        let data = pkts.spell_start.data();
        let (first_guid, _) = decode_packed_guid(data);
        assert_eq!(first_guid, trainer_guid().raw(),
            "SMSG_SPELL_START first packed GUID must be trainer when NPC casts");
    }

    /// SMSG_SPELL_GO hit list must contain exactly the player GUID.
    #[test]
    fn spell_go_hit_list_contains_player() {
        let pkts = build_trainer_anim_packets(player_guid(), trainer_guid(), 100, 0);
        let data = pkts.spell_go.data();

        // Skip: packed_guid(caster) + packed_guid(pack) + spell_id(4) + cast_flags(2)
        let (_, n1) = decode_packed_guid(data);
        let (_, n2) = decode_packed_guid(&data[n1..]);
        let base = n1 + n2 + 4 + 2; // after spell_id and cast_flags

        let hit_count = data[base];
        assert_eq!(hit_count, 1, "SMSG_SPELL_GO hit count must be 1 (the player)");

        let hit_guid = read_u64_le(data, base + 1);
        assert_eq!(hit_guid, player_guid().raw(), "hit list must contain the player's GUID");
    }
}
