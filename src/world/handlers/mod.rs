//! Packet handlers - parse packets, delegate to systems
//!
//! Handler flow:
//! 1. Receive packet
//! 2. Parse packet data
//! 3. Validate request
//! 4. Call appropriate system/manager
//! 5. Handler sends response via session

pub mod area_trigger;
pub mod auth;
pub mod character;
pub mod game_object_handler;
pub mod character_create_items;
pub mod chat;
pub mod creature_combat;
pub mod death;
pub mod gossip_handler;
pub mod group;
pub mod guild;
pub mod item;
pub mod loot;
pub mod mail;
pub mod movement;
pub mod query;
pub mod quest_handler;
pub mod reputation;
pub mod settings;
pub mod social;
pub mod spells;
pub mod talent;
pub mod ticket;
pub mod trade;
pub mod trainer_handler;
pub mod player_handler;
pub mod vendor_handler;

use anyhow::Result;
use tracing::{debug, info, warn};

use crate::shared::database::Databases;
use crate::shared::messages::SmsgPong;
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::{SessionState, WorldSession};
use crate::world::World;

/// Dispatch incoming packet to appropriate handler
///
/// Handlers send responses directly via the session
pub async fn dispatch_packet(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    let opcode = packet.opcode();
    let state = session.state();

    debug!("Dispatching packet {:?} in state {:?}", opcode, state);

    match state {
        SessionState::Authenticated => {
            // Character selection screen
            debug!("Processing packet {:?} in Authenticated state", opcode);
            match opcode {
                Opcode::CMSG_CHAR_ENUM => {
                    debug!("Dispatching to handle_char_enum");
                    character::handle_char_enum(session, databases, world).await?;
                }
                Opcode::CMSG_PLAYER_LOGIN => {
                    character::handle_player_login(session, packet, databases, world).await?;
                }
                Opcode::CMSG_CHAR_CREATE => {
                    character::handle_char_create(session, packet, databases, world).await?;
                }
                Opcode::CMSG_CHAR_DELETE => {
                    character::handle_char_delete(session, packet, databases, world).await?;
                }
                Opcode::CMSG_CHAR_RENAME => {
                    character::handle_char_rename(session, packet, databases, world).await?;
                }
                Opcode::CMSG_PING => {
                    let sequence = packet.read_u32().unwrap_or(0);
                    let _latency = packet.read_u32().unwrap_or(0);
                    debug!("Ping received: seq={}", sequence);

                    let pong = SmsgPong { sequence };
                    session.send_msg(pong)?;
                }
                _ => {
                    debug!("Unhandled opcode {:?} in Authenticated state", opcode);
                }
            }
        }
        SessionState::LoggedIn => {
            // In-world packets - Phase 3+
            match opcode {
                Opcode::CMSG_PING => {
                    let sequence = packet.read_u32().unwrap_or(0);
                    let _latency = packet.read_u32().unwrap_or(0);
                    debug!("Ping received: seq={}", sequence);

                    let pong = SmsgPong { sequence };
                    session.send_msg(pong)?;
                }

                // Social handlers
                Opcode::CMSG_ADD_FRIEND => {
                    social::handle_add_friend(session, packet, world).await?;
                }
                Opcode::CMSG_DEL_FRIEND => {
                    social::handle_del_friend(session, packet, world).await?;
                }
                Opcode::CMSG_FRIEND_LIST => {
                    social::handle_friend_list(session, world).await?;
                }
                Opcode::CMSG_ADD_IGNORE => {
                    social::handle_add_ignore(session, packet, world).await?;
                }
                Opcode::CMSG_DEL_IGNORE => {
                    social::handle_del_ignore(session, packet, world).await?;
                }
                Opcode::CMSG_WHO => {
                    social::handle_who(session, packet, world).await?;
                }
                // Note: Ignore list is sent automatically with friend list
                // There is no CMSG_IGNORE_LIST opcode in vanilla WoW

                // Guild handlers
                Opcode::CMSG_GUILD_QUERY => {
                    guild::handle_guild_query(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_CREATE => {
                    guild::handle_guild_create(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_INVITE => {
                    guild::handle_guild_invite(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_ACCEPT => {
                    guild::handle_guild_accept(session, world).await?;
                }
                Opcode::CMSG_GUILD_DECLINE => {
                    guild::handle_guild_decline(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_ROSTER => {
                    guild::handle_guild_roster(session, world).await?;
                }
                Opcode::CMSG_GUILD_LEAVE => {
                    guild::handle_guild_leave(session, world).await?;
                }
                Opcode::CMSG_GUILD_REMOVE => {
                    guild::handle_guild_remove(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_PROMOTE => {
                    guild::handle_guild_promote(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_DEMOTE => {
                    guild::handle_guild_demote(session, packet, world).await?;
                }
                Opcode::CMSG_GUILD_DISBAND => {
                    guild::handle_guild_disband(session, world).await?;
                }
                Opcode::CMSG_GUILD_INFO => {
                    guild::handle_guild_info(session, world).await?;
                }

                // Chat handlers
                Opcode::CMSG_MESSAGECHAT => {
                    chat::handle_messagechat(session, packet, world).await?;
                }
                Opcode::CMSG_JOIN_CHANNEL => {
                    chat::handle_join_channel(session, packet, world).await?;
                }
                Opcode::CMSG_LEAVE_CHANNEL => {
                    chat::handle_leave_channel(session, packet, world).await?;
                }
                Opcode::CMSG_CHANNEL_LIST => {
                    chat::handle_channel_list(session, packet, world).await?;
                }
                Opcode::CMSG_EMOTE => {
                    chat::handle_emote(session, packet, world).await?;
                }
                Opcode::CMSG_TEXT_EMOTE => {
                    chat::handle_text_emote(session, packet, world).await?;
                }

                // Logout handlers
                Opcode::CMSG_LOGOUT_REQUEST => {
                    character::handle_logout_request(session, packet, world).await?;
                }
                Opcode::CMSG_LOGOUT_CANCEL => {
                    character::handle_logout_cancel(session, packet, world).await?;
                }

                // Zone update
                Opcode::CMSG_ZONEUPDATE => {
                    character::handle_zoneupdate(session, packet, world).await?;
                }

                // Area trigger handler
                Opcode::CMSG_AREATRIGGER => {
                    area_trigger::handle_area_trigger(session, packet, world).await?;
                }

                // Worldport acknowledgment - client ready after map transfer
                Opcode::MSG_MOVE_WORLDPORT_ACK => {
                    info!("========================================");
                    info!("[DISPATCHER] MSG_MOVE_WORLDPORT_ACK opcode received, routing to handler...");
                    info!("========================================");
                    movement::handle_worldport_ack(session, packet, world).await?;
                    info!("[DISPATCHER] handle_worldport_ack completed successfully");
                }

                // Movement packets - processed inline for immediate responsiveness
                Opcode::MSG_MOVE_HEARTBEAT
                | Opcode::MSG_MOVE_START_FORWARD
                | Opcode::MSG_MOVE_START_BACKWARD
                | Opcode::MSG_MOVE_STOP
                | Opcode::MSG_MOVE_START_STRAFE_LEFT
                | Opcode::MSG_MOVE_START_STRAFE_RIGHT
                | Opcode::MSG_MOVE_STOP_STRAFE
                | Opcode::MSG_MOVE_JUMP
                | Opcode::MSG_MOVE_START_TURN_LEFT
                | Opcode::MSG_MOVE_START_TURN_RIGHT
                | Opcode::MSG_MOVE_STOP_TURN
                | Opcode::MSG_MOVE_SET_FACING
                | Opcode::MSG_MOVE_FALL_LAND => {
                    movement::handle_movement(session, opcode, packet, world).await?;
                }

                // Query handlers
                Opcode::CMSG_QUERY_TIME => {
                    query::handle_query_time(session).await?;
                }
                Opcode::CMSG_NAME_QUERY => {
                    query::handle_name_query(session, packet, databases, world).await?;
                }
                Opcode::CMSG_CREATURE_QUERY => {
                    query::handle_creature_query(session, packet, world).await?;
                }
                Opcode::CMSG_ITEM_QUERY_SINGLE => {
                    query::handle_item_query(session, packet, world).await?;
                }
                Opcode::CMSG_GAMEOBJECT_QUERY => {
                    query::handle_gameobject_query(session, packet, world).await?;
                }
                Opcode::CMSG_GAMEOBJ_USE => {
                    game_object_handler::handle_gameobj_use(session, packet, world).await?;
                }

                // Item handlers
                Opcode::CMSG_USE_ITEM => {
                    item::handle_use_item(session, packet, world).await?;
                }
                Opcode::CMSG_OPEN_ITEM => {
                    item::handle_open_item(session, packet, world).await?;
                }
                Opcode::CMSG_READ_ITEM => {
                    item::handle_read_item(session, packet, world).await?;
                }
                Opcode::CMSG_SWAP_ITEM => {
                    item::handle_swap_item(session, packet, world).await?;
                }
                Opcode::CMSG_SWAP_INV_ITEM => {
                    item::handle_swap_inv_item(session, packet, world).await?;
                }
                Opcode::CMSG_SPLIT_ITEM => {
                    item::handle_split_item(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOEQUIP_ITEM_SLOT => {
                    item::handle_autoequip_item_slot(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOEQUIP_ITEM => {
                    item::handle_autoequip_item(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOEQUIP_GROUND_ITEM => {
                    item::handle_autoequip_ground_item(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOSTORE_GROUND_ITEM => {
                    item::handle_autostore_ground_item(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOSTORE_BAG_ITEM => {
                    item::handle_autostore_bag_item(session, packet, world).await?;
                }
                Opcode::CMSG_DROP_ITEM => {
                    item::handle_drop_item(session, packet, world).await?;
                }
                Opcode::CMSG_DESTROYITEM => {
                    item::handle_destroy_item(session, packet, world).await?;
                }
                Opcode::CMSG_SET_AMMO => {
                    item::handle_set_ammo(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOBANK_ITEM => {
                    item::handle_autobank_item(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOSTORE_BANK_ITEM => {
                    item::handle_autostore_bank_item(session, packet, world).await?;
                }
                Opcode::CMSG_BUY_BANK_SLOT => {
                    item::handle_buy_bank_slot(session, packet, world).await?;
                }
                Opcode::CMSG_BUYBACK_ITEM => {
                    item::handle_buyback_item(session, packet, world).await?;
                }

                // Trade handlers
                Opcode::CMSG_INITIATE_TRADE => {
                    trade::handle_initiate_trade(session, packet, world).await?;
                }
                Opcode::CMSG_BEGIN_TRADE => {
                    trade::handle_begin_trade(session, world).await?;
                }
                Opcode::CMSG_SET_TRADE_ITEM => {
                    trade::handle_set_trade_item(session, packet, world).await?;
                }
                Opcode::CMSG_CLEAR_TRADE_ITEM => {
                    trade::handle_clear_trade_item(session, packet, world).await?;
                }
                Opcode::CMSG_SET_TRADE_GOLD => {
                    trade::handle_set_trade_gold(session, packet, world).await?;
                }
                Opcode::CMSG_ACCEPT_TRADE => {
                    trade::handle_accept_trade(session, packet, world).await?;
                }
                Opcode::CMSG_UNACCEPT_TRADE => {
                    trade::handle_unaccept_trade(session, world).await?;
                }
                Opcode::CMSG_CANCEL_TRADE => {
                    trade::handle_cancel_trade(session, world).await?;
                }
                Opcode::CMSG_BUSY_TRADE => {
                    trade::handle_busy_trade(session, world).await?;
                }
                Opcode::CMSG_IGNORE_TRADE => {
                    trade::handle_ignore_trade(session, world).await?;
                }

                // Group handlers
                Opcode::CMSG_GROUP_INVITE => {
                    group::handle_group_invite(session, packet, world).await?;
                }
                Opcode::CMSG_GROUP_ACCEPT => {
                    group::handle_group_accept(session, world).await?;
                }
                Opcode::CMSG_GROUP_DECLINE => {
                    group::handle_group_decline(session, world).await?;
                }
                Opcode::CMSG_GROUP_UNINVITE => {
                    group::handle_group_uninvite(session, packet, world).await?;
                }
                Opcode::MSG_PARTY_LEAVE => {
                    group::handle_party_leave(session, world).await?;
                }
                Opcode::CMSG_GROUP_SET_LEADER => {
                    group::handle_group_set_leader(session, packet, world).await?;
                }
                Opcode::CMSG_GROUP_DISBAND => {
                    group::handle_group_disband(session, world).await?;
                }
                Opcode::CMSG_GROUP_RAID_CONVERT => {
                    group::handle_group_raid_convert(session, world).await?;
                }
                Opcode::CMSG_GROUP_CHANGE_SUB_GROUP => {
                    group::handle_group_change_sub_group(session, packet, world).await?;
                }
                Opcode::CMSG_GROUP_SWAP_SUB_GROUP => {
                    group::handle_group_swap_sub_group(session, packet, world).await?;
                }
                Opcode::CMSG_GROUP_ASSISTANT_LEADER => {
                    group::handle_group_assistant_leader(session, packet, world).await?;
                }
                Opcode::CMSG_LOOT_METHOD => {
                    group::handle_set_loot_method(session, packet, world).await?;
                }
                Opcode::MSG_RAID_READY_CHECK => {
                    group::handle_raid_ready_check(session, packet, world).await?;
                }
                Opcode::MSG_RAID_TARGET_UPDATE => {
                    group::handle_raid_target_update(session, packet, world).await?;
                }
                Opcode::CMSG_REQUEST_RAID_INFO => {
                    group::handle_request_raid_info(session, world).await?;
                }
                Opcode::CMSG_REQUEST_PARTY_MEMBER_STATS => {
                    group::handle_request_party_member_stats(session, packet, world).await?;
                }

                // GM Ticket handlers
                Opcode::CMSG_GMTICKET_GETTICKET => {
                    ticket::handle_gmticket_getticket(session, packet, world).await?;
                }
                Opcode::CMSG_GMTICKET_CREATE => {
                    ticket::handle_gmticket_create(session, packet, world).await?;
                }
                Opcode::CMSG_GMTICKET_UPDATETEXT => {
                    ticket::handle_gmticket_updatetext(session, packet, world).await?;
                }
                Opcode::CMSG_GMTICKET_DELETETICKET => {
                    ticket::handle_gmticket_deleteticket(session, packet, world).await?;
                }
                Opcode::CMSG_GMTICKET_SYSTEMSTATUS => {
                    ticket::handle_gmticket_systemstatus(session, packet, world).await?;
                }

                // Gossip handlers
                Opcode::CMSG_GOSSIP_HELLO => {
                    gossip_handler::handle_gossip_hello(session, packet, world).await?;
                }
                Opcode::CMSG_GOSSIP_SELECT_OPTION => {
                    gossip_handler::handle_gossip_select_option(session, packet, world).await?;
                }
                Opcode::CMSG_NPC_TEXT_QUERY => {
                    gossip_handler::handle_npc_text_query(session, packet, world).await?;
                }

                // Player handlers
                Opcode::CMSG_SET_SELECTION => {
                    player_handler::handle_set_selection(session, packet, world).await?;
                }

                // Trainer handlers
                Opcode::CMSG_TRAINER_LIST => {
                    trainer_handler::handle_trainer_list(session, packet, world).await?;
                }
                Opcode::CMSG_TRAINER_BUY_SPELL => {
                    trainer_handler::handle_trainer_buy_spell(session, packet, world).await?;
                }

                // Vendor handlers
                Opcode::CMSG_LIST_INVENTORY => {
                    vendor_handler::handle_list_inventory(session, packet, world).await?;
                }
                Opcode::CMSG_BUY_ITEM => {
                    vendor_handler::handle_buy_item(session, packet, world).await?;
                }
                Opcode::CMSG_SELL_ITEM => {
                    vendor_handler::handle_sell_item(session, packet, world).await?;
                }

                // Quest handlers
                Opcode::CMSG_QUESTGIVER_STATUS_QUERY => {
                    quest_handler::handle_questgiver_status_query(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_HELLO => {
                    quest_handler::handle_questgiver_hello(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_QUERY_QUEST => {
                    quest_handler::handle_questgiver_query_quest(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_ACCEPT_QUEST => {
                    quest_handler::handle_questgiver_accept_quest(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_COMPLETE_QUEST => {
                    quest_handler::handle_questgiver_complete_quest(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_REQUEST_REWARD => {
                    quest_handler::handle_questgiver_request_reward(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_CHOOSE_REWARD => {
                    quest_handler::handle_questgiver_choose_reward(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTGIVER_CANCEL => {
                    quest_handler::handle_questgiver_cancel(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTLOG_REMOVE_QUEST => {
                    quest_handler::handle_questlog_remove_quest(session, packet, world).await?;
                }
                Opcode::CMSG_QUESTLOG_SWAP_QUEST => {
                    quest_handler::handle_questlog_swap_quest(session, packet, world).await?;
                }
                Opcode::CMSG_QUEST_CONFIRM_ACCEPT => {
                    quest_handler::handle_quest_confirm_accept(session, packet, world).await?;
                }

                // Reputation handlers
                Opcode::CMSG_SET_FACTION_ATWAR => {
                    reputation::handle_set_faction_atwar(session, packet, world).await?;
                }
                Opcode::CMSG_SET_FACTION_INACTIVE => {
                    reputation::handle_set_faction_inactive(session, packet, world).await?;
                }

                // Settings handlers
                Opcode::CMSG_SET_ACTION_BUTTON => {
                    settings::handle_set_action_button(session, packet, world).await?;
                }
                Opcode::CMSG_UPDATE_ACCOUNT_DATA => {
                    settings::handle_update_account_data(session, packet, world).await?;
                }
                Opcode::CMSG_REQUEST_ACCOUNT_DATA => {
                    settings::handle_request_account_data(session, packet, world).await?;
                }
                Opcode::CMSG_TUTORIAL_FLAG => {
                    settings::handle_tutorial_flag(session, packet, world).await?;
                }
                Opcode::CMSG_TUTORIAL_CLEAR => {
                    settings::handle_tutorial_clear(session, packet, world).await?;
                }
                Opcode::CMSG_TUTORIAL_RESET => {
                    settings::handle_tutorial_reset(session, packet, world).await?;
                }

                // Spell handlers
                Opcode::CMSG_CAST_SPELL => {
                    spells::handle_cast_spell(session, packet, world).await?;
                }
                Opcode::CMSG_CANCEL_CAST => {
                    spells::handle_cancel_cast(session, packet, world).await?;
                }
                Opcode::CMSG_CANCEL_CHANNELLING => {
                    spells::handle_cancel_channelling(session, packet, world).await?;
                }
                Opcode::CMSG_CANCEL_AURA => {
                    spells::handle_cancel_aura(session, packet, world).await?;
                }

                // Talent handlers
                Opcode::CMSG_LEARN_TALENT => {
                    talent::handle_learn_talent(session, packet, world).await?;
                }
                Opcode::CMSG_UNLEARN_TALENTS => {
                    talent::handle_unlearn_talents(session, packet, world).await?;
                }

                // Death handlers
                Opcode::CMSG_REPOP_REQUEST => {
                    death::handle_repop_request(session, packet, world).await?;
                }
                Opcode::CMSG_RECLAIM_CORPSE => {
                    death::handle_reclaim_corpse(session, packet, world).await?;
                }
                Opcode::CMSG_RESURRECT_RESPONSE => {
                    death::handle_resurrect_response(session, packet, world).await?;
                }
                Opcode::CMSG_SPIRIT_HEALER_ACTIVATE => {
                    death::handle_spirit_healer_activate(session, packet, world).await?;
                }
                Opcode::CMSG_SELF_RES => {
                    death::handle_self_res(session, packet, world).await?;
                }
                Opcode::CMSG_AREA_SPIRIT_HEALER_QUERY => {
                    death::handle_area_spirit_healer_query(session, packet, world).await?;
                }
                Opcode::CMSG_AREA_SPIRIT_HEALER_QUEUE => {
                    death::handle_area_spirit_healer_queue(session, packet, world).await?;
                }
                Opcode::CMSG_SETDEATHBINDPOINT => {
                    death::handle_setdeathbindpoint(session, packet, databases, world).await?;
                }
                Opcode::CMSG_GETDEATHBINDZONE => {
                    death::handle_getdeathbindzone(session, packet, world).await?;
                }

                // Creature combat handlers (Phase 2)
                Opcode::CMSG_ATTACKSWING => {
                    if let Some(target_guid) = packet.read_guid() {
                        if let Some(attacker_guid) = session.player_guid() {
                            creature_combat::handle_attack_swing(world, attacker_guid, target_guid).await?;
                        }
                    }
                }
                Opcode::CMSG_ATTACKSTOP => {
                    if let Some(attacker_guid) = session.player_guid() {
                        creature_combat::handle_attack_stop(world, attacker_guid).await?;
                    }
                }
                Opcode::CMSG_SETSHEATHED => {
                    // Client draws/sheathes weapon on combat enter/exit - acknowledge silently
                }

                // Loot handlers (Phase 7)
                Opcode::CMSG_LOOT => {
                    info!("[LOOT] CMSG_LOOT received at dispatch");
                    loot::handle_loot(session, packet, world).await?;
                }
                Opcode::CMSG_LOOT_MONEY => {
                    loot::handle_loot_money(session, packet, world).await?;
                }
                Opcode::CMSG_AUTOSTORE_LOOT_ITEM => {
                    loot::handle_loot_item(session, packet, world).await?;
                }
                Opcode::CMSG_LOOT_RELEASE => {
                    loot::handle_loot_release(session, packet, world).await?;
                }

                // Mail handlers
                Opcode::MSG_QUERY_NEXT_MAIL_TIME => {
                    mail::handle_query_next_mail_time(session, packet, world).await?;
                }
                Opcode::CMSG_GET_MAIL_LIST => {
                    mail::handle_get_mail_list(session, packet, world).await?;
                }
                Opcode::CMSG_ITEM_TEXT_QUERY => {
                    mail::handle_item_text_query(session, packet, world).await?;
                }

                // Movement acknowledgments (read and discard)
                Opcode::CMSG_SET_ACTIVE_MOVER => {
                    let _mover_guid = packet.read_guid();
                    debug!("CMSG_SET_ACTIVE_MOVER acknowledged");
                }
                Opcode::CMSG_MOVE_TIME_SKIPPED => {
                    let _mover_guid = packet.read_guid();
                    let _time_skipped = packet.read_u32();
                    debug!("CMSG_MOVE_TIME_SKIPPED acknowledged");
                }
                Opcode::CMSG_FORCE_MOVE_ROOT_ACK => {
                    // Client acknowledges being rooted (e.g. during logout)
                }

                // Stubs for systems not yet in v2
                Opcode::CMSG_TAXINODE_STATUS_QUERY => {
                    // Taxi system not wired in v2 yet
                    debug!("CMSG_TAXINODE_STATUS_QUERY stub");
                }
                Opcode::CMSG_BATTLEFIELD_STATUS => {
                    // No battleground system - send empty status
                    let mut response = WorldPacket::new(Opcode::SMSG_BATTLEFIELD_STATUS);
                    response.write_u32(0); // queue slot
                    response.write_u32(0); // map (0 = no BG)
                    session.send_packet(response)?;
                }
                Opcode::CMSG_MEETINGSTONE_INFO => {
                    // No LFG system - send "not in queue"
                    let mut response = WorldPacket::new(Opcode::SMSG_MEETINGSTONE_SETQUEUE);
                    response.write_u32(0); // area_id
                    response.write_u8(0);  // status = None
                    session.send_packet(response)?;
                }

                _ => {
                    if let Some(player_guid) = session.player_guid() {
                        if let Some(player) = world.managers.player_mgr.get_player(player_guid) {
                            info!(
                                "Unhandled opcode {:?} from player '{}' (GUID: {:?})",
                                opcode, player.name, player_guid
                            );
                        } else {
                            info!("Unhandled opcode {:?} from GUID {:?}", opcode, player_guid);
                        }
                    } else {
                        debug!("Unhandled opcode {:?}", opcode);
                    }
                }
            }
        }
        _ => {
            warn!(
                "Packet {:?} received in unexpected state {:?}",
                opcode, state
            );
        }
    }

    Ok(())
}
