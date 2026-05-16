//! Executor for LuaActions returned by gossip/zone scripts.
//!
//! Unlike the creature-AI bridge (which translates to AIAction), gossip actions
//! operate directly on player-facing systems: gossip menus, vendor windows,
//! quest completion, inventory, etc.

use anyhow::Result;
use tracing::debug;

use super::actions::LuaAction;
use super::snapshot::PlayerSnapshot;
use crate::shared::messages::gossip::{
    GossipOptionData, GossipQuestData, SmsgGossipComplete, SmsgGossipMessage,
};
use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::World;

/// Build a `PlayerSnapshot` from world state for gossip/quest callbacks.
pub fn build_player_snapshot(player_guid: ObjectGuid, world: &World) -> PlayerSnapshot {
    world
        .managers
        .player_mgr
        .get_player(player_guid)
        .map(|p| PlayerSnapshot {
            guid: p.guid,
            name: p.name.clone(),
            level: p.level,
            class: p.class,
            race: p.race,
            faction: faction_from_race(p.race),
            gold: p.money,
            health: p.stats.health,
            max_health: p.stats.max_health,
            map_id: p.map_id,
            zone_id: p.zone_id,
        })
        .unwrap_or_default()
}

/// Derive the player faction (Horde=67, Alliance=469) from race.
fn faction_from_race(race: u8) -> u32 {
    match race {
        1 | 3 | 4 | 7 => 469, // Human, Dwarf, NightElf, Gnome -> Alliance
        2 | 5 | 6 | 8 => 67,  // Orc, Undead, Tauren, Troll -> Horde
        _ => 0,
    }
}

/// Execute `LuaAction`s returned by a gossip/zone script callback.
///
/// Actions that build up a gossip menu are accumulated and sent together
/// when `GossipSend` is encountered. Actions that affect the world
/// (faction change, say, spawn, items, quests) are applied inline.
///
/// `player_guid` — the player the interactions target
/// `npc_guid` — the NPC involved in the interaction
pub async fn execute_gossip_actions(
    actions: Vec<LuaAction>,
    player_guid: ObjectGuid,
    npc_guid: ObjectGuid,
    world: &World,
) -> Result<()> {
    // Accumulate gossip menu state until GossipSend
    let mut npc_text_id: u32 = 0;
    let mut gossip_options: Vec<GossipOptionData> = Vec::new();
    let mut gossip_quests: Vec<GossipQuestData> = Vec::new();
    let mut option_index: u32 = 0;

    for action in actions {
        match action {
            // ==================== Gossip menu building ====================
            LuaAction::GossipMenu { npc_text_id: id } => {
                npc_text_id = id;
            }
            LuaAction::GossipOption {
                id: _,
                icon,
                text,
                coded,
            } => {
                gossip_options.push(GossipOptionData {
                    index: option_index,
                    icon,
                    coded,
                    money: 0,
                    text,
                });
                option_index += 1;
            }
            LuaAction::GossipQuest { quest_id } => {
                // Look up quest title/level for the gossip entry
                if let Some(quest) = world.systems.quest.manager.get_quest_template(quest_id) {
                    gossip_quests.push(GossipQuestData {
                        quest_id,
                        icon: 2, // default available icon
                        level: quest.quest_level as u32,
                        title: quest.title.clone(),
                    });
                }
            }
            LuaAction::GossipSend => {
                let msg = SmsgGossipMessage {
                    source_guid: npc_guid,
                    menu_id: 0,
                    text_id: npc_text_id,
                    options: std::mem::take(&mut gossip_options),
                    quests: std::mem::take(&mut gossip_quests),
                };
                world
                    .managers
                    .broadcast_mgr
                    .send_msg_to_player(player_guid, msg);
                npc_text_id = 0;
                option_index = 0;
            }
            LuaAction::GossipClose => {
                world
                    .managers
                    .broadcast_mgr
                    .send_msg_to_player(player_guid, SmsgGossipComplete);
            }

            // ==================== NPC windows ====================
            LuaAction::SendVendor => {
                world
                    .systems
                    .vendor
                    .send_vendor_list(player_guid, npc_guid)
                    .await?;
            }
            LuaAction::SendTrainer
            | LuaAction::SendBanker
            | LuaAction::SendAuctioneer
            | LuaAction::SendInnkeeper
            | LuaAction::SendTaxi => {
                debug!(
                    "Gossip executor: NPC window action not yet implemented: {:?}",
                    action
                );
            }

            // ==================== Creature state ====================
            LuaAction::SetFaction { faction_id } => {
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(npc_guid, |c| {
                        c.faction = faction_id;
                    });
            }
            LuaAction::Say { text } => {
                send_creature_chat(world, npc_guid, 0x0B, &text); // CHAT_MSG_MONSTER_SAY
            }
            LuaAction::Yell { text } => {
                send_creature_chat(world, npc_guid, 0x0C, &text); // CHAT_MSG_MONSTER_YELL
            }
            LuaAction::ScriptText { text_id } => {
                // TODO: look up text from script_texts DB table
                debug!(
                    "Gossip executor: ScriptText {} not yet implemented",
                    text_id
                );
            }
            LuaAction::Emote { emote_id } => {
                send_creature_emote(world, npc_guid, emote_id);
            }

            // ==================== Player rewards ====================
            LuaAction::GiveItem {
                player,
                item_id,
                count,
            } => {
                world
                    .systems
                    .inventory
                    .add_item(player, item_id, count)
                    .await;
            }
            LuaAction::GiveGold { player, amount } => {
                world.managers.player_mgr.with_player_mut(player, |p| {
                    p.money = p.money.saturating_add(amount);
                });
            }
            LuaAction::TakeGold { player, amount } => {
                world.managers.player_mgr.with_player_mut(player, |p| {
                    p.money = p.money.saturating_sub(amount);
                });
            }
            LuaAction::AddReputation {
                player,
                faction_id,
                amount,
            } => {
                if let Err(e) = world
                    .systems
                    .reputation
                    .modify_reputation(player, faction_id, amount, world)
                {
                    debug!(
                        "Gossip executor: AddReputation faction={} amount={} failed: {}",
                        faction_id, amount, e
                    );
                }
            }
            LuaAction::CompleteQuest { player, quest_id } => {
                // If no player guid was specified in the script, default to the triggering player.
                let target = if player.is_empty() {
                    player_guid
                } else {
                    player
                };
                world
                    .systems
                    .quest
                    .handle_area_event_complete(target, quest_id);
                debug!(
                    "Gossip executor: CompleteQuest player={:?} quest={}",
                    target, quest_id
                );
            }

            // ==================== Spawning ====================
            LuaAction::SpawnCreature {
                entry, x, y, z, o, ..
            } => {
                // Dynamically summon a creature at the given coordinates.
                // Uses a transient spawn_id (0) since this is a script-created summon.
                let map_id = world
                    .managers
                    .player_mgr
                    .get_player(player_guid)
                    .map(|p| p.map_id)
                    .unwrap_or(0);
                let instance_id = world
                    .managers
                    .player_mgr
                    .get_player(player_guid)
                    .map(|p| p.instance_id)
                    .unwrap_or(0);
                let spawn = crate::world::game::creature::spawn::CreatureSpawnData::new(
                    0,
                    entry,
                    map_id,
                    Position { x, y, z, o },
                    0,
                );
                if world
                    .managers
                    .creature_mgr
                    .spawn_creature(&spawn, instance_id)
                    .is_none()
                {
                    // Spawn failed (likely no template) — log and continue
                    debug!(
                        "Gossip executor: SpawnCreature entry={} failed (template not found?)",
                        entry
                    );
                }
            }

            LuaAction::SpawnCreatureAtPlayer {
                entry,
                summon_type: _,
                duration_ms: _,
            } => {
                // Spawn a creature at the player's current position.
                // TODO: honour summon_type and duration_ms for timed/combat despawn.
                if let Some(p) = world.managers.player_mgr.get_player(player_guid) {
                    let pos = p.movement.position;
                    let map_id = p.map_id;
                    let instance_id = p.instance_id;
                    drop(p);
                    let spawn = crate::world::game::creature::spawn::CreatureSpawnData::new(
                        0, entry, map_id, pos, 0,
                    );
                    if world
                        .managers
                        .creature_mgr
                        .spawn_creature(&spawn, instance_id)
                        .is_none()
                    {
                        debug!(
                            "Gossip executor: SpawnCreatureAtPlayer entry={} failed",
                            entry
                        );
                    }
                }
            }

            // ==================== Quest credit ====================
            LuaAction::KillCreditNearestCreature {
                creature_entry,
                search_radius,
            } => {
                // Find the nearest alive creature with the given entry within search_radius yards
                // of the player, then award kill credit for it.
                let player_info = world
                    .managers
                    .player_mgr
                    .get_player(player_guid)
                    .map(|p| (p.movement.position, p.map_id));

                if let Some((pos, player_map)) = player_info {
                    let radius_sq = search_radius * search_radius;

                    // Collect candidates into a vec to avoid holding DashMap refs
                    let candidates: Vec<(ObjectGuid, u32, f32)> = world
                        .managers
                        .creature_mgr
                        .iter_creatures()
                        .filter_map(|e| {
                            let c = e.value();
                            if c.entry != creature_entry || c.map_id != player_map || !c.is_alive()
                            {
                                return None;
                            }
                            let dx = c.position.x - pos.x;
                            let dy = c.position.y - pos.y;
                            let dz = c.position.z - pos.z;
                            let dist_sq = dx * dx + dy * dy + dz * dz;
                            if dist_sq <= radius_sq {
                                Some((*e.key(), c.entry, dist_sq))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if let Some((cg, ce, _)) = candidates
                        .into_iter()
                        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        world.systems.quest.handle_kill_credit(player_guid, ce, cg);
                    } else {
                        debug!(
                            "Gossip executor: KillCreditNearestCreature entry={} not found within {}yd",
                            creature_entry, search_radius
                        );
                    }
                }
            }

            LuaAction::SetUnitFlag { flag } => {
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(npc_guid, |c| {
                        c.unit_flags |= flag;
                    });
            }
            LuaAction::RemoveUnitFlag { flag } => {
                world
                    .managers
                    .creature_mgr
                    .with_creature_mut(npc_guid, |c| {
                        c.unit_flags &= !flag;
                    });
            }

            LuaAction::CastSpellOnNearestCreature {
                creature_entry,
                spell_id,
                search_radius,
            } => {
                // Find the nearest alive creature with the given entry within search_radius
                // yards of the NPC/GO, then have the NPC cast the spell on it.
                // TODO: the "caster" here is npc_guid; if npc_guid is empty (e.g. area trigger
                // context), the cast will silently fail inside execute_creature_spell_cast.
                let npc_pos = world
                    .managers
                    .creature_mgr
                    .with_creature(npc_guid, |c| c.position)
                    .or_else(|| {
                        // Fall back to player position for GO/area-trigger contexts.
                        world
                            .managers
                            .player_mgr
                            .get_player(player_guid)
                            .map(|p| p.movement.position)
                    });

                if let Some(origin) = npc_pos {
                    let radius_sq = search_radius * search_radius;
                    let candidates: Vec<(ObjectGuid, f32)> = world
                        .managers
                        .creature_mgr
                        .iter_creatures()
                        .filter_map(|e| {
                            let c = e.value();
                            if c.entry != creature_entry || !c.is_alive() {
                                return None;
                            }
                            let dx = c.position.x - origin.x;
                            let dy = c.position.y - origin.y;
                            let dz = c.position.z - origin.z;
                            let dist_sq = dx * dx + dy * dy + dz * dz;
                            if dist_sq <= radius_sq {
                                Some((*e.key(), dist_sq))
                            } else {
                                None
                            }
                        })
                        .collect();

                    if let Some((target_guid, _)) = candidates
                        .into_iter()
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    {
                        crate::world::game::creature::ai::executor::execute_creature_spell_cast(
                            world,
                            npc_guid,
                            spell_id,
                            Some(target_guid),
                            true,
                        );
                    } else {
                        debug!(
                            "Gossip executor: CastSpellOnNearestCreature entry={} not found within {}yd",
                            creature_entry, search_radius
                        );
                    }
                }
            }

            // ==================== Silently ignore actions not relevant to gossip ====================
            _ => {
                debug!(
                    "Gossip executor: skipping action not applicable to gossip context: {:?}",
                    action
                );
            }
        }
    }

    Ok(())
}

/// Send SMSG_MESSAGECHAT from an NPC to nearby players.
fn send_creature_chat(world: &World, creature_guid: ObjectGuid, chat_type: u8, text: &str) {
    use crate::shared::game::chat::{ChatMsg, ChatTag, Language};
    use crate::shared::messages::chat::SmsgMessageChat;
    use crate::shared::messages::ToWorldPacket;
    use crate::world::game::broadcast_mgr::broadcast_around_creature;

    let name = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| c.name.clone())
        .unwrap_or_default();

    let msgtype = match chat_type {
        0x0B => ChatMsg::MonsterSay,
        0x0C => ChatMsg::MonsterYell,
        0x0D => ChatMsg::MonsterEmote,
        _ => ChatMsg::MonsterSay,
    };

    let packet = SmsgMessageChat {
        msgtype,
        language: Language::Universal,
        sender_guid: creature_guid,
        sender_name: Some(&name),
        target_guid: None,
        channel_name: None,
        player_rank: None,
        message: text,
        chat_tag: ChatTag::None,
    }
    .to_world_packet();

    broadcast_around_creature(world, creature_guid, &packet);
}

/// Send SMSG_EMOTE from an NPC.
fn send_creature_emote(world: &World, creature_guid: ObjectGuid, emote_id: u32) {
    let mut packet = WorldPacket::new(Opcode::SMSG_EMOTE);
    packet.write_u32(emote_id);
    packet.write_u64(creature_guid.raw());

    use crate::world::game::broadcast_mgr::broadcast_around_creature;
    broadcast_around_creature(world, creature_guid, &packet);
}
