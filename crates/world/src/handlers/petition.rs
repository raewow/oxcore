//! Guild charter and tabard design handlers.
//!
//! A charter is bought from a guild master, shown to and signed by nine players of the
//! owner's side, and turned in to found the guild. The tabard designer saves a guild's
//! emblem for its leader.

use anyhow::Result;
use tracing::debug;

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::guild::petition::{
    is_valid_charter_name, Petition, GUILD_CHARTER_COST, GUILD_CHARTER_ITEM,
};
use crate::game::inventory::AddItemResult;
use crate::game::npc::interaction::can_interact_with_npc;
use crate::World;
use oxcore_shared::game::guild::{GuildEmblem, ERR_ALREADY_IN_GUILD_S, ERR_GUILD_NAME_INVALID};
use oxcore_shared::game::petition::{PetitionResult, PetitionSignature};
use oxcore_shared::messages::guild::SmsgGuildCommandResult;
use oxcore_shared::messages::petition::{
    GuildEmblemResult, MsgPetitionDecline, MsgPetitionRename, MsgSaveGuildEmblem,
    SmsgPetitionQueryResponse, SmsgPetitionShowSignatures, SmsgPetitionSignResults,
    SmsgTurnInPetitionResults,
};
use oxcore_shared::messages::vendor::{BuyError, SmsgBuyFailed};
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

const NPC_FLAG_PETITIONER: u32 = 0x0000_0200;
const NPC_FLAG_TABARDDESIGNER: u32 = 0x0000_0400;

const GUILD_CREATE_S: u32 = 0;
const GUILD_INVITE_S: u32 = 1;
const ERR_ALREADY_INVITED_TO_GUILD_S: u32 = 0x05;
const ERR_GUILD_NAME_EXISTS_S: u32 = 0x07;
const ERR_GUILD_NOT_ALLIED: u32 = 0x0C;

/// Signatures a charter needs before it can be turned in, from the configuration.
fn required_signatures(world: &World) -> u32 {
    world
        .config
        .min_petition_signs
        .unwrap_or(oxcore_shared::game::petition::GUILD_CHARTER_SIGNATURES)
}

/// What saving a tabard design costs, in copper.
const GUILD_EMBLEM_COST: u32 = 100_000;

fn player_of(session: &WorldSession) -> Result<ObjectGuid> {
    session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))
}

fn read_guid(session: &WorldSession, packet: &mut WorldPacket) -> Result<ObjectGuid> {
    packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read GUID"))
}

fn modern_guid(reader: &mut BitReader) -> Result<ObjectGuid> {
    let (high, low) = reader
        .read_packed_guid_128()
        .ok_or_else(|| anyhow::anyhow!("Failed to read GUID"))?;
    Ok(ObjectGuid::from_guid128(high, low))
}

fn command_result(session: &WorldSession, command: u32, name: &str, error_code: u32) -> Result<()> {
    session.send_msg(SmsgGuildCommandResult {
        command,
        target_name: name,
        error_code,
    })
}

/// The charter `item` in `player`'s bags and the petition it carries.
fn charter_of(world: &World, player: ObjectGuid, item: ObjectGuid) -> Option<Petition> {
    let charter = world.systems.inventory.cache().get_item(player, item)?;
    let petition_id = {
        let charter = charter.read();
        if charter.entry != GUILD_CHARTER_ITEM {
            return None;
        }
        charter.enchantments.first().map(|(id, _, _)| *id)?
    };
    world.systems.guild.petitions.get(petition_id)
}

fn show_signatures(world: &World, to: ObjectGuid, petition: &Petition) {
    world.managers.broadcast_mgr.send_msg_to_player(
        to,
        SmsgPetitionShowSignatures {
            charter_guid: petition.charter,
            owner_guid: petition.owner,
            petition_guid: petition.id,
            signatures: &petition.signatures,
        },
    );
}

/// Handle CMSG_PETITION_BUY: buy a guild charter from a guild master.
pub async fn handle_petition_buy(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (npc, name) = if session.protocol() == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let length = reader.read_bits(7).unwrap_or(0) as usize;
        let npc = modern_guid(&mut reader)?;
        let _index = reader.read_u32();
        (npc, reader.read_string(length).unwrap_or_default())
    } else {
        let npc = read_guid(session, packet)?;
        let _ = packet.read_u32();
        let _ = packet.read_u64();
        (npc, packet.read_cstring().unwrap_or_default())
    };

    if !can_interact_with_npc(world, player, npc, NPC_FLAG_PETITIONER) {
        debug!("Charter purchase: {:?} cannot talk to {:?}", player, npc);
        return Ok(());
    }
    let designer = world
        .managers
        .creature_mgr
        .with_creature(npc, |c| c.npc_flags & NPC_FLAG_TABARDDESIGNER != 0)
        .unwrap_or(false);
    if !designer {
        return Ok(());
    }
    super::gossip_handler::remove_feign_death(world, player);

    let guild = &world.systems.guild;
    if guild.is_in_guild(player) {
        return Ok(());
    }
    // One charter at a time; a charter whose item is gone no longer counts.
    if let Some(existing) = guild.petitions.by_owner(player) {
        if charter_of(world, player, existing.charter).is_some() {
            return Ok(());
        }
        guild.petitions.remove(existing.id);
    }
    if guild.has_guild_name(&name) {
        return command_result(session, GUILD_CREATE_S, &name, ERR_GUILD_NAME_EXISTS_S);
    }
    if !is_valid_charter_name(&name) {
        return command_result(session, GUILD_CREATE_S, &name, ERR_GUILD_NAME_INVALID);
    }

    let buy_failed = |error| {
        session.send_msg(SmsgBuyFailed {
            vendor_guid: npc,
            item_id: GUILD_CHARTER_ITEM,
            error,
        })
    };
    if world.systems.item_mgr.get_template(GUILD_CHARTER_ITEM).is_none() {
        return buy_failed(BuyError::CantFind);
    }
    let Some((money, team)) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.money, p.get_team()))
    else {
        return Ok(());
    };
    if money < GUILD_CHARTER_COST {
        return buy_failed(BuyError::NotEnoughMoney);
    }

    let charter = match world
        .systems
        .inventory
        .add_item(player, GUILD_CHARTER_ITEM, 1)
        .await
    {
        AddItemResult::Success { items_created, .. } => match items_created.first() {
            Some(charter) => *charter,
            None => return Ok(()),
        },
        AddItemResult::InventoryFull => return buy_failed(BuyError::CantCarryMore),
        _ => return Ok(()),
    };
    world
        .systems
        .inventory
        .remove_gold(player, GUILD_CHARTER_COST);

    let petition_id = guild.petitions.create(player, charter, &name, team);
    world
        .systems
        .inventory
        .apply_enchantment(player, charter, petition_id, 0)
        .await;
    Ok(())
}

/// Handle CMSG_PETITION_SHOW_SIGNATURES: the owner looks at their charter.
pub async fn handle_petition_show_signatures(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let item = read_guid(session, packet)?;
    if world.systems.guild.is_in_guild(player) {
        return Ok(());
    }
    if let Some(petition) = charter_of(world, player, item) {
        show_signatures(world, player, &petition);
    }
    Ok(())
}

/// Handle CMSG_PETITION_QUERY: the details of a charter the client saw.
pub async fn handle_petition_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let petition_id = if session.protocol() == Protocol::Modern {
        BitReader::new(packet.contents()).read_u32()
    } else {
        packet.read_u32()
    }
    .ok_or_else(|| anyhow::anyhow!("Failed to read petition id"))?;
    let Some(petition) = world.systems.guild.petitions.get(petition_id) else {
        return Ok(());
    };
    session.send_msg(SmsgPetitionQueryResponse {
        petition_guid: petition.id,
        owner_guid: petition.owner,
        guild_name: &petition.name,
    })
}

/// Handle MSG_PETITION_RENAME: the owner renames the guild on their charter.
pub async fn handle_petition_rename(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (item, name) = if session.protocol() == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let item = modern_guid(&mut reader)?;
        reader.align();
        let length = reader.read_bits(7).unwrap_or(0) as usize;
        (item, reader.read_string(length).unwrap_or_default())
    } else {
        let item = read_guid(session, packet)?;
        (item, packet.read_cstring().unwrap_or_default())
    };

    let Some(petition) = charter_of(world, player, item) else {
        return Ok(());
    };
    if world.systems.guild.has_guild_name(&name) {
        return command_result(session, GUILD_CREATE_S, &name, ERR_GUILD_NAME_EXISTS_S);
    }
    if !is_valid_charter_name(&name) {
        return command_result(session, GUILD_CREATE_S, &name, ERR_GUILD_NAME_INVALID);
    }
    if world.systems.guild.petitions.rename(petition.id, &name) {
        session.send_msg(MsgPetitionRename {
            charter_guid: item,
            new_name: &name,
        })?;
    }
    Ok(())
}

/// Handle CMSG_PETITION_SIGN: a player signs a charter shown to them.
pub async fn handle_petition_sign(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let charter = read_guid(session, packet)?;
    let Some(petition) = world.systems.guild.petitions.by_charter(charter) else {
        debug!("No charter {:?} for {:?} to sign", charter, player);
        return Ok(());
    };
    // The client shows at most nine names.
    if petition.is_complete(required_signatures(world))
        || petition.signatures.len() as u32
            >= oxcore_shared::game::petition::GUILD_CHARTER_SIGNATURES
    {
        return Ok(());
    }

    let sign_result = |to: ObjectGuid, result| {
        world.managers.broadcast_mgr.send_msg_to_player(
            to,
            SmsgPetitionSignResults {
                charter_guid: charter,
                player_guid: player,
                result,
            },
        );
    };
    if petition.owner == player {
        sign_result(player, PetitionResult::CantSignOwn);
        return Ok(());
    }
    let Some((name, team, account)) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.name.clone(), p.get_team(), p.account_id))
    else {
        return Ok(());
    };
    if team != petition.team {
        return command_result(session, GUILD_CREATE_S, "", ERR_GUILD_NOT_ALLIED);
    }
    let guild = &world.systems.guild;
    if guild.is_in_guild(player) {
        return command_result(session, GUILD_INVITE_S, &name, ERR_ALREADY_IN_GUILD_S);
    }
    if guild.get_pending_invite(player).is_some() {
        return command_result(session, GUILD_INVITE_S, &name, ERR_ALREADY_INVITED_TO_GUILD_S);
    }
    if petition.signed_by(player, account) {
        sign_result(player, PetitionResult::AlreadySigned);
        world.managers.broadcast_mgr.send_msg_to_player(
            petition.owner,
            SmsgGuildCommandResult {
                command: GUILD_INVITE_S,
                target_name: &name,
                error_code: ERR_ALREADY_INVITED_TO_GUILD_S,
            },
        );
        return Ok(());
    }

    if guild.petitions.add_signature(
        petition.id,
        PetitionSignature {
            player_guid: player,
            player_account: account,
        },
    ) {
        sign_result(player, PetitionResult::Ok);
        sign_result(petition.owner, PetitionResult::Ok);
    }
    Ok(())
}

/// Handle MSG_PETITION_DECLINE: a player would not sign; tell the owner.
pub async fn handle_petition_decline(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let charter = read_guid(session, packet)?;
    let Some(petition) = world.systems.guild.petitions.by_charter(charter) else {
        return Ok(());
    };
    let name = world
        .managers
        .player_mgr
        .with_player(player, |p| p.name.clone())
        .unwrap_or_default();
    world.managers.broadcast_mgr.send_msg_to_player(
        petition.owner,
        MsgPetitionDecline {
            player_guid: player,
            player_name: &name,
        },
    );
    Ok(())
}

/// Handle CMSG_OFFER_PETITION: the owner offers their charter to another player.
pub async fn handle_offer_petition(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (item, target) = if session.protocol() == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let _ = reader.read_u32();
        (modern_guid(&mut reader)?, modern_guid(&mut reader)?)
    } else {
        (read_guid(session, packet)?, read_guid(session, packet)?)
    };

    let Some((target_name, target_team)) = world
        .managers
        .player_mgr
        .with_player(target, |p| (p.name.clone(), p.get_team()))
    else {
        return Ok(());
    };
    let own_team = world
        .managers
        .player_mgr
        .with_player(player, |p| p.get_team());
    if own_team != Some(target_team) {
        return command_result(session, GUILD_CREATE_S, "", ERR_GUILD_NOT_ALLIED);
    }
    let guild = &world.systems.guild;
    if guild.is_in_guild(target) {
        return command_result(session, GUILD_INVITE_S, &target_name, ERR_ALREADY_IN_GUILD_S);
    }
    if guild.get_pending_invite(target).is_some() {
        return command_result(
            session,
            GUILD_INVITE_S,
            &target_name,
            ERR_ALREADY_INVITED_TO_GUILD_S,
        );
    }
    if let Some(petition) = charter_of(world, player, item) {
        show_signatures(world, target, &petition);
    }
    Ok(())
}

/// Handle CMSG_TURN_IN_PETITION: found the guild named on a fully signed charter.
pub async fn handle_turn_in_petition(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let item = read_guid(session, packet)?;
    let Some(petition) = charter_of(world, player, item) else {
        return Ok(());
    };
    let guild = &world.systems.guild;
    let turn_in_result = |result| session.send_msg(SmsgTurnInPetitionResults { result });

    if guild.is_in_guild(player) {
        return turn_in_result(PetitionResult::AlreadyInGuild);
    }
    if petition.owner != player {
        return Ok(());
    }
    if !petition.is_complete(required_signatures(world)) {
        return turn_in_result(PetitionResult::NeedMoreSignatures);
    }
    if guild.has_guild_name(&petition.name) {
        return command_result(session, GUILD_CREATE_S, &petition.name, ERR_GUILD_NAME_EXISTS_S);
    }

    let leader_name = world
        .managers
        .player_mgr
        .with_player(player, |p| p.name.clone())
        .unwrap_or_default();
    guild
        .create_guild_from_petition(player, leader_name, petition.name.clone())
        .await?;
    let Some(guild_id) = guild.get_player_guild(player).and_then(|s| s.guild_id) else {
        return Ok(());
    };
    let lowest_rank = guild
        .get_guild(guild_id)
        .map(|g| g.ranks.len().saturating_sub(1) as u8)
        .unwrap_or(0);
    for signature in &petition.signatures {
        let name = signer_name(world, signature.player_guid).await;
        if let Err(e) = guild
            .add_member_directly(signature.player_guid, name, guild_id, lowest_rank)
            .await
        {
            debug!(
                "Charter signer {:?} could not join the new guild: {e:#}",
                signature.player_guid
            );
        }
    }

    guild.petitions.remove(petition.id);
    world.systems.inventory.remove_item(player, item, 1);
    turn_in_result(PetitionResult::Ok)
}

/// A signer's name, from the world when they are online and the database otherwise.
async fn signer_name(world: &World, player: ObjectGuid) -> String {
    if let Some(name) = world.managers.player_mgr.get_player_name(player) {
        return name;
    }
    sqlx::query_scalar::<_, String>("SELECT name FROM characters.characters WHERE guid = $1")
        .bind(i64::from(player.counter()))
        .fetch_optional(&world.databases.character)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Handle MSG_SAVE_GUILD_EMBLEM (1.14: CMSG_SAVE_GUILD_EMBLEM): a guild leader saves a tabard.
pub async fn handle_save_guild_emblem(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (designer, values) = if session.protocol() == Protocol::Modern {
        let mut reader = BitReader::new(packet.contents());
        let designer = modern_guid(&mut reader)?;
        let mut values = [0u32; 5];
        for value in &mut values {
            *value = reader.read_u32().unwrap_or(0);
        }
        (designer, values)
    } else {
        let designer = read_guid(session, packet)?;
        let mut values = [0u32; 5];
        for value in &mut values {
            *value = packet.read_u32().unwrap_or(0);
        }
        (designer, values)
    };
    // Both clients send style, colour, border style, border colour, then background.
    let [style, color, border_style, border_color, background_color] = values;
    let reply = |result| session.send_msg(MsgSaveGuildEmblem { result });

    if !can_interact_with_npc(world, player, designer, NPC_FLAG_TABARDDESIGNER) {
        return reply(GuildEmblemResult::FailNoMessage);
    }
    super::gossip_handler::remove_feign_death(world, player);

    let guild_id = match world.systems.guild.emblem_guild(player) {
        Ok(guild_id) => guild_id,
        Err(result) => return reply(result),
    };
    let money = world
        .managers
        .player_mgr
        .with_player(player, |p| p.money)
        .unwrap_or(0);
    if money < GUILD_EMBLEM_COST {
        return reply(GuildEmblemResult::NotEnoughMoney);
    }
    world.systems.inventory.remove_gold(player, GUILD_EMBLEM_COST);

    let narrow = |v: u32| u8::try_from(v).unwrap_or(0);
    world
        .systems
        .guild
        .set_emblem(
            guild_id,
            GuildEmblem {
                style: narrow(style),
                color: narrow(color),
                border_style: narrow(border_style),
                border_color: narrow(border_color),
                background_color: narrow(background_color),
            },
        )
        .await?;
    reply(GuildEmblemResult::Success)?;
    world.systems.guild.query_guild(player, guild_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::broadcaster::PlayerBroadcaster;
    use crate::game::player::player::Player;
    use crate::Config;
    use oxcore_db::database::Databases;
    use oxcore_shared::game::chat::Team;
    use oxcore_shared::protocol::Opcode;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn test_world() -> World {
        let pool = || {
            PgPoolOptions::new()
                .connect_lazy("postgres://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        World::new(
            Arc::new(Databases {
                world: pool(),
                character: pool(),
                auth: pool(),
                logs: oxcore_db::database::lazy_logs_pool(),
            }),
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    struct Online {
        guid: ObjectGuid,
        session: Arc<WorldSession>,
        rx: mpsc::UnboundedReceiver<WorldPacket>,
    }

    fn add_player(world: &World, counter: u32, race: u8) -> Online {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(counter, counter, format!("P{counter}"), 0, tx));
        let guid = ObjectGuid::new_player(counter);
        session.set_player_guid(Some(guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), guid);
        let mut player = Player::new(guid, format!("P{counter}"), 0, 0, 1, 20, race, 1, 0);
        player.account_id = counter;
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(session.packet_tx(), guid)));
        world.managers.player_mgr.add_player(player, counter);
        Online { guid, session, rx }
    }

    fn sent(rx: &mut mpsc::UnboundedReceiver<WorldPacket>) -> Vec<WorldPacket> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    fn sign(charter: ObjectGuid) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::CMSG_PETITION_SIGN);
        packet.write_u64(charter.raw());
        packet.write_u8(1);
        packet
    }

    fn sign_result(packets: &[WorldPacket]) -> Option<u32> {
        packets
            .iter()
            .find(|p| p.opcode() == Opcode::SMSG_PETITION_SIGN_RESULTS)
            .map(|p| u32::from_le_bytes(p.data()[16..20].try_into().unwrap()))
    }

    #[tokio::test]
    async fn signing_a_charter() {
        let world = test_world();
        let mut owner = add_player(&world, 1, 1);
        let mut signer = add_player(&world, 2, 3);
        let charter = ObjectGuid::new_item(50);
        let id = world
            .systems
            .guild
            .petitions
            .create(owner.guid, charter, "Dawn", Team::Alliance);

        handle_petition_sign(&owner.session, &mut sign(charter), &world)
            .await
            .unwrap();
        assert_eq!(sign_result(&sent(&mut owner.rx)), Some(PetitionResult::CantSignOwn as u32));

        handle_petition_sign(&signer.session, &mut sign(charter), &world)
            .await
            .unwrap();
        assert_eq!(sign_result(&sent(&mut signer.rx)), Some(PetitionResult::Ok as u32));
        assert_eq!(sign_result(&sent(&mut owner.rx)), Some(PetitionResult::Ok as u32));
        assert_eq!(world.systems.guild.petitions.get(id).unwrap().signatures.len(), 1);

        handle_petition_sign(&signer.session, &mut sign(charter), &world)
            .await
            .unwrap();
        assert_eq!(
            sign_result(&sent(&mut signer.rx)),
            Some(PetitionResult::AlreadySigned as u32)
        );
        assert_eq!(world.systems.guild.petitions.get(id).unwrap().signatures.len(), 1);
    }

    #[tokio::test]
    async fn the_other_side_cannot_sign() {
        let world = test_world();
        let owner = add_player(&world, 1, 1);
        let mut orc = add_player(&world, 2, 2);
        let charter = ObjectGuid::new_item(51);
        let id = world
            .systems
            .guild
            .petitions
            .create(owner.guid, charter, "Dawn", Team::Alliance);

        handle_petition_sign(&orc.session, &mut sign(charter), &world)
            .await
            .unwrap();

        let packets = sent(&mut orc.rx);
        assert_eq!(sign_result(&packets), None);
        assert!(packets
            .iter()
            .any(|p| p.opcode() == Opcode::SMSG_GUILD_COMMAND_RESULT));
        assert!(world.systems.guild.petitions.get(id).unwrap().signatures.is_empty());
    }

    #[tokio::test]
    async fn querying_a_charter_names_its_guild() {
        let world = test_world();
        let mut owner = add_player(&world, 1, 1);
        let id = world.systems.guild.petitions.create(
            owner.guid,
            ObjectGuid::new_item(52),
            "Dawn",
            Team::Alliance,
        );
        let mut packet = WorldPacket::new(Opcode::CMSG_PETITION_QUERY);
        packet.write_u32(id);
        packet.write_u64(ObjectGuid::new_item(52).raw());

        handle_petition_query(&owner.session, &mut packet, &world)
            .await
            .unwrap();

        let packets = sent(&mut owner.rx);
        let response = packets
            .iter()
            .find(|p| p.opcode() == Opcode::SMSG_PETITION_QUERY_RESPONSE)
            .expect("query answered");
        assert_eq!(&response.data()[0..4], &id.to_le_bytes());
        assert_eq!(&response.data()[12..17], b"Dawn\0");
    }
}
