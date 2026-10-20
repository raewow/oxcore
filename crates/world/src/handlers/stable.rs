//! Stable master handlers: a hunter keeps up to two tamed pets at the stable besides the
//! one travelling with them, and buys the stable slots to do it.

use anyhow::Result;
use tracing::debug;

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::npc::interaction::can_interact_with_npc;
use crate::game::pet::{pet_slot, HunterPet, MAX_STABLE_SLOTS};
use crate::World;
use oxcore_shared::messages::stable::{MsgListStabledPets, SmsgStableResult, StableResult, StabledPetInfo};
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

const NPC_FLAG_STABLEMASTER: u32 = 0x0000_2000;

fn player_of(session: &WorldSession) -> Result<ObjectGuid> {
    session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))
}

/// The stable master's GUID, and the pet number for unstabling and swapping. 1.14 puts the
/// number first.
fn read_request(
    session: &WorldSession,
    packet: &mut WorldPacket,
    with_number: bool,
) -> Result<(ObjectGuid, u32)> {
    if session.protocol() == Protocol::Modern && with_number {
        let mut reader = BitReader::new(packet.contents());
        let number = reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read pet number"))?;
        let (high, low) = reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow::anyhow!("Failed to read stable master GUID"))?;
        return Ok((ObjectGuid::from_guid128(high, low), number));
    }
    let master = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read stable master GUID"))?;
    let number = if with_number {
        packet.read_u32().unwrap_or(0)
    } else {
        0
    };
    Ok((master, number))
}

/// The stable window's pet list: the pet with the hunter first, then the stabled ones.
pub fn send_stable_list(world: &World, player: ObjectGuid, stable_master: ObjectGuid) {
    let pets = world.systems.pet.pets(player);
    let info = |pet: &HunterPet, slot: u8| StabledPetInfo {
        pet_number: pet.number,
        entry: pet.entry,
        display_id: pet.display_id,
        level: u32::from(pet.level),
        name: pet.name.clone(),
        loyalty: pet.loyalty,
        slot,
    };
    let mut list = Vec::new();
    // A dead pet is not offered for stabling.
    let with_player = world
        .systems
        .pet
        .out_pet(player)
        .filter(|_| world.systems.pet.active_pet_alive(player))
        .or_else(|| pets.iter().find(|p| p.travels_with_owner()).cloned());
    if let Some(pet) = &with_player {
        list.push(info(pet, 1));
    }
    for pet in pets.iter().filter(|p| p.is_stabled()) {
        list.push(info(pet, pet.slot as u8 + 1));
    }
    let stable_slots = world
        .managers
        .player_mgr
        .with_player(player, |p| p.stable_slots)
        .unwrap_or(0);
    world.managers.broadcast_mgr.send_msg_to_player(
        player,
        MsgListStabledPets {
            stable_master,
            stable_slots,
            pets: list,
        },
    );
}

fn reply(session: &WorldSession, result: StableResult) -> Result<()> {
    session.send_msg(SmsgStableResult { result })
}

/// Talking to a stable master breaks interact-cancelled auras and channels.
async fn at_stable_master(world: &World, player: ObjectGuid, master: ObjectGuid) -> Result<bool> {
    if !can_interact_with_npc(world, player, master, NPC_FLAG_STABLEMASTER) {
        debug!("{:?} cannot use stable master {:?}", player, master);
        return Ok(false);
    }
    crate::handlers::gossip_handler::begin_npc_interaction(world, player, master).await?;
    Ok(true)
}

/// Handle MSG_LIST_STABLED_PETS (1.14: CMSG_REQUEST_STABLED_PETS)
pub async fn handle_list_stabled_pets(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (master, _) = read_request(session, packet, false)?;
    if at_stable_master(world, player, master).await? {
        send_stable_list(world, player, master);
    }
    Ok(())
}

/// Handle CMSG_STABLE_PET: put the living pet that is out into a free stable slot.
pub async fn handle_stable_pet(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (master, _) = read_request(session, packet, false)?;
    let alive = world
        .managers
        .player_mgr
        .with_player(player, |p| p.is_alive())
        .unwrap_or(false);
    if !alive || !at_stable_master(world, player, master).await? {
        return reply(session, StableResult::Failed);
    }
    let pets = &world.systems.pet;
    if pets.out_pet(player).is_none() || !pets.active_pet_alive(player) {
        return reply(session, StableResult::Failed);
    }
    let used: Vec<u32> = pets
        .pets(player)
        .iter()
        .filter(|p| p.is_stabled())
        .map(|p| p.slot)
        .collect();
    let owned = world
        .managers
        .player_mgr
        .with_player(player, |p| u32::from(p.stable_slots))
        .unwrap_or(0);
    let free = (pet_slot::FIRST_STABLE..=pet_slot::LAST_STABLE).find(|slot| !used.contains(slot));
    match free {
        Some(slot) if slot <= owned && pets.dismiss_to_slot(player, slot, world) => {
            reply(session, StableResult::Stabled)
        }
        _ => reply(session, StableResult::Failed),
    }
}

/// Handle CMSG_UNSTABLE_PET: take a stabled pet out when no pet travels with the hunter.
pub async fn handle_unstable_pet(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (master, number) = read_request(session, packet, true)?;
    if !at_stable_master(world, player, master).await? {
        return reply(session, StableResult::Failed);
    }
    let pets = &world.systems.pet;
    let stabled = pets.record(player, number).filter(HunterPet::is_stabled);
    let pet_with_player = pets.out_pet(player).is_some()
        || pets.pets(player).iter().any(HunterPet::travels_with_owner);
    let Some(stabled) = stabled.filter(|_| !pet_with_player) else {
        return reply(session, StableResult::Failed);
    };
    match pets.bring_out(player, stabled.number, world) {
        Ok(_) => reply(session, StableResult::Unstabled),
        Err(_) => reply(session, StableResult::Failed),
    }
}

/// Handle CMSG_STABLE_SWAP_PET: trade the living pet that is out for a stabled one.
pub async fn handle_stable_swap_pet(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (master, number) = read_request(session, packet, true)?;
    if !at_stable_master(world, player, master).await? {
        return reply(session, StableResult::Failed);
    }
    let pets = &world.systems.pet;
    if pets.out_pet(player).is_none() || !pets.active_pet_alive(player) {
        return reply(session, StableResult::Failed);
    }
    let Some(stabled) = pets.record(player, number).filter(HunterPet::is_stabled) else {
        return reply(session, StableResult::Failed);
    };
    // The pet that was out takes the stabled pet's place.
    if !pets.dismiss_to_slot(player, stabled.slot, world) {
        return reply(session, StableResult::Failed);
    }
    match pets.bring_out(player, stabled.number, world) {
        Ok(_) => reply(session, StableResult::Unstabled),
        Err(_) => reply(session, StableResult::Failed),
    }
}

/// Handle CMSG_BUY_STABLE_SLOT
pub async fn handle_buy_stable_slot(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player = player_of(session)?;
    let (master, _) = read_request(session, packet, false)?;
    if !at_stable_master(world, player, master).await? {
        return reply(session, StableResult::Failed);
    }
    let Some((owned, money)) = world
        .managers
        .player_mgr
        .with_player(player, |p| (p.stable_slots, p.money))
    else {
        return Ok(());
    };
    if owned >= MAX_STABLE_SLOTS {
        return reply(session, StableResult::Failed);
    }
    let Some(price) = world.dbc.read().get_stable_slot_price(u32::from(owned) + 1) else {
        return reply(session, StableResult::Failed);
    };
    if money < price {
        return reply(session, StableResult::NotEnoughMoney);
    }
    world.systems.inventory.remove_gold(player, price);
    world
        .managers
        .player_mgr
        .with_player_mut(player, |p| p.stable_slots = owned + 1);
    save_stable_slots(world, player, owned + 1);
    reply(session, StableResult::BoughtSlot)
}

fn save_stable_slots(world: &World, player: ObjectGuid, slots: u8) {
    let pool = world.databases.character.clone();
    let guid = i64::from(player.counter());
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            if let Err(e) =
                sqlx::query("UPDATE characters.characters SET stable_slots = $1 WHERE guid = $2")
                    .bind(i16::from(slots))
                    .bind(guid)
                    .execute(&pool)
                    .await
            {
                tracing::warn!("Failed to save stable slots of character {}: {}", guid, e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::manager::CreatureTemplate;
    use crate::game::creature::Creature;
    use crate::game::player::broadcaster::PlayerBroadcaster;
    use crate::game::player::player::Player;
    use crate::Config;
    use oxcore_db::database::Databases;
    use oxcore_shared::protocol::{Opcode, Position};
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

    fn template(entry: u32, npc_flags: u32, creature_type: u8, static_flags1: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature {entry}"),
            subname: None,
            min_level: 8,
            max_level: 8,
            faction: 35,
            model_id_1: 903,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags,
            unit_flags: 0,
            static_flags1,
            flags_extra: 0,
            creature_type,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
            gold_min: 0,
            gold_max: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skinning_loot_id: 0,
            ai: Default::default(),
        }
    }

    fn add_creature(world: &World, template: CreatureTemplate, counter: u32) -> ObjectGuid {
        let entry = template.entry;
        world.managers.creature_mgr.add_template(template.clone());
        let guid = ObjectGuid::new_creature(entry, counter);
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            entry,
            counter,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
        guid
    }

    fn packet(opcode: Opcode, master: ObjectGuid, number: Option<u32>) -> WorldPacket {
        let mut packet = WorldPacket::new(opcode);
        packet.write_u64(master.raw());
        if let Some(number) = number {
            packet.write_u32(number);
        }
        packet
    }

    fn results(rx: &mut mpsc::UnboundedReceiver<WorldPacket>) -> Vec<u8> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter(|p| p.opcode() == Opcode::SMSG_STABLE_RESULT)
            .map(|p| p.data()[0])
            .collect()
    }

    #[tokio::test]
    async fn stabling_and_unstabling_a_pet() {
        let world = test_world();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(1, 1, "Hunter".to_string(), 0, tx));
        let hunter = ObjectGuid::new_player(1);
        session.set_player_guid(Some(hunter));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), hunter);
        let mut player = Player::new(hunter, "Hunter".to_string(), 0, 0, 1, 10, 2, 3, 0);
        player.stable_slots = 1;
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(session.packet_tx(), hunter)));
        world.managers.player_mgr.add_player(player, 1);

        let master = add_creature(&world, template(9984, NPC_FLAG_STABLEMASTER, 7, 0), 1);
        let wolf = add_creature(&world, template(299, 0, 1, 0x10), 2);
        world.managers.creature_mgr.set_pet_family_for_test(299, 1);
        world.systems.pet.tame(hunter, wolf, 1515, &world).unwrap();
        let number = world.systems.pet.out_pet(hunter).unwrap().number;

        handle_stable_pet(&session, &mut packet(Opcode::CMSG_STABLE_PET, master, None), &world)
            .await
            .unwrap();
        assert_eq!(results(&mut rx), vec![StableResult::Stabled as u8]);
        assert_eq!(world.systems.pet.record(hunter, number).unwrap().slot, 1);
        assert!(world.systems.pet.out_pet(hunter).is_none());

        // The one slot is full.
        handle_stable_pet(&session, &mut packet(Opcode::CMSG_STABLE_PET, master, None), &world)
            .await
            .unwrap();
        assert_eq!(results(&mut rx), vec![StableResult::Failed as u8]);

        handle_unstable_pet(
            &session,
            &mut packet(Opcode::CMSG_UNSTABLE_PET, master, Some(number)),
            &world,
        )
        .await
        .unwrap();
        assert_eq!(results(&mut rx), vec![StableResult::Unstabled as u8]);
        assert_eq!(world.systems.pet.out_pet(hunter).map(|p| p.number), Some(number));
    }
}
