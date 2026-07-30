//! Game object interaction handler
//!
//! Handles CMSG_GAMEOBJ_USE (0x00B1) — player right-clicks / uses a game object.
//! Routes to Lua OnGameObjectHello script if registered for the GO's entry.

use anyhow::Result;
use tracing::debug;

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::lua::{build_player_snapshot, execute_gossip_actions};
use crate::core::session::WorldSession;
use crate::game::gameobject::GameObjectType;
use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Protocol, WorldPacket};

fn read_gameobject_use_guid(protocol: Protocol, packet: &mut WorldPacket) -> Result<ObjectGuid> {
    packet
        .read_guid_for(protocol)
        .ok_or_else(|| anyhow::anyhow!("Failed to read GO GUID"))
}

/// Handle CMSG_GAMEOBJ_USE (0x00B1)
///
/// Sent when the player right-clicks / activates a game object.
/// Packet format: GUID (8 bytes, unpacked)
pub async fn handle_gameobj_use(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let go_guid = read_gameobject_use_guid(session.protocol(), packet)?;

    debug!(
        "CMSG_GAMEOBJ_USE: player={:?}, go={:?}",
        player_guid, go_guid
    );

    let Some(go) = world.managers.gameobject_mgr.get_gameobject(go_guid) else {
        debug!(
            "CMSG_GAMEOBJ_USE: GO {:?} not found or has no entry",
            go_guid
        );
        return Ok(());
    };
    let go_entry = go.entry;
    let go_type = go.go_type;
    // Looting re-enters the gameobject manager to validate and update the chest.
    // Release this DashMap read guard before dispatching the interaction.
    drop(go);

    let template = world.managers.gameobject_mgr.get_template(go_entry);
    let gossip_id = template
        .as_ref()
        .map(|template| match go_type {
            GameObjectType::QuestGiver => template.data[3].max(0) as u32,
            GameObjectType::Goober if template.data[7] <= 0 => template.data[19].max(0) as u32,
            _ => 0,
        })
        .unwrap_or(0);

    if go_type == GameObjectType::Chest {
        world
            .systems
            .loot
            .handle_loot_request(player_guid, go_guid, world)
            .await?;
        return Ok(());
    }

    if !matches!(go_type, GameObjectType::QuestGiver | GameObjectType::Goober) {
        return Ok(());
    }

    // Check for a Lua script handling OnGameObjectHello before default gossip.
    if let Some(script) = world.managers.lua_mgr.get_game_object_script(go_entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gameobject_hello(lua, &player_snap, go_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, go_guid, world).await?;
            return Ok(());
        }
    }

    let quest_items =
        world
            .systems
            .quest
            .prepare_gameobject_quest_menu(player_guid, go_entry, world);

    if gossip_id == 0 && !quest_items.is_empty() {
        world
            .systems
            .quest
            .send_gameobject_quest_list(player_guid, go_guid, go_entry, world)?;
        return Ok(());
    }

    if gossip_id > 0 || !quest_items.is_empty() {
        let quest_data = if quest_items.is_empty() {
            None
        } else {
            Some(
                quest_items
                    .into_iter()
                    .map(|q| oxcore_shared::messages::GossipQuestData {
                        quest_id: q.quest_id,
                        icon: q.icon,
                        level: q.level,
                        title: q.title,
                    })
                    .collect(),
            )
        };
        world
            .systems
            .gossip
            .send_gameobject_gossip_menu(player_guid, go_guid, gossip_id, quest_data)
            .await?;
    }

    Ok(())
}

/// Handle game object open/loot interaction.
///
/// Called internally when a player opens a chest or door (not directly from a packet).
/// Routes to Lua OnGameObjectOpen script if registered.
pub async fn handle_gameobj_open(
    player_guid: oxcore_shared::protocol::ObjectGuid,
    go_guid: oxcore_shared::protocol::ObjectGuid,
    world: &World,
) -> Result<()> {
    let go_entry = world
        .managers
        .gameobject_mgr
        .get_gameobject(go_guid)
        .map(|go| go.entry)
        .unwrap_or(0);

    if go_entry == 0 {
        return Ok(());
    }

    if let Some(script) = world.managers.lua_mgr.get_game_object_script(go_entry) {
        let player_snap = build_player_snapshot(player_guid, world);
        let actions = world
            .managers
            .lua_mgr
            .with_lua(|lua| script.on_gameobject_open(lua, &player_snap, go_guid));
        if !actions.is_empty() {
            execute_gossip_actions(actions, player_guid, go_guid, world).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::session::WorldSession;
    use crate::game::broadcast_mgr::BroadcastManagerTrait;
    use crate::game::gameobject::{GameObject, GameObjectTemplate};
    use crate::game::npc::quest::system::QuestSystem;
    use crate::game::npc::quest::types::QuestTemplate;
    use crate::game::player::broadcaster::PlayerBroadcaster;
    use crate::game::player::Player;
    use oxcore_shared::database::characters::repositories::quest_repository::{
        MockQuestRepositoryTrait, QuestRepositoryTrait,
    };
    use oxcore_shared::database::Databases;
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::{HighGuid, Opcode, Position};
    use sqlx::mysql::MySqlPoolOptions;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    fn lazy_pool() -> sqlx::MySqlPool {
        MySqlPoolOptions::new()
            .connect_lazy("mysql://test:test@localhost/test")
            .expect("lazy pool should be constructible")
    }

    fn test_world() -> crate::World {
        let databases = Arc::new(Databases {
            world: lazy_pool(),
            character: lazy_pool(),
            auth: lazy_pool(),
            logs: lazy_pool(),
        });

        crate::World::new(
            databases,
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn install_mock_quest_system(world: &mut crate::World) {
        let mut mock_repo = MockQuestRepositoryTrait::new();
        mock_repo
            .expect_delete_quest_status()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let quest_repo: Arc<dyn QuestRepositoryTrait> = Arc::new(mock_repo);
        let broadcast_mgr: Arc<dyn BroadcastManagerTrait> = world.managers.broadcast_mgr.clone();
        let quest_system = Arc::new(QuestSystem::new(
            Arc::clone(&world.systems.quest.manager),
            quest_repo,
            broadcast_mgr,
            Arc::clone(&world.managers.player_mgr),
            Arc::clone(&world.managers.creature_mgr),
            Arc::clone(&world.managers.item_mgr),
            Arc::clone(&world.systems.inventory),
            Arc::clone(&world.systems.experience),
        ));

        Arc::get_mut(&mut world.systems)
            .expect("systems should be uniquely owned")
            .quest = quest_system;
    }

    fn add_player(
        world: &mut crate::World,
    ) -> (Arc<WorldSession>, mpsc::UnboundedReceiver<WorldPacket>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let session = Arc::new(WorldSession::new(1, 1, "Tester".to_string(), 0, tx));
        let player_guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        session.set_player_guid(Some(player_guid));
        world.session_mgr.add_session(Arc::clone(&session));
        world.session_mgr.register_player(session.id(), player_guid);

        let mut player = Player::new(player_guid, "Tester".to_string(), 0, 0, 0, 10, 1, 1, 0);
        player.set_broadcaster(Arc::new(PlayerBroadcaster::new(
            session.packet_tx(),
            player_guid,
        )));
        world.managers.player_mgr.add_player(player, 1);
        (session, rx)
    }

    fn add_gameobject(
        world: &crate::World,
        entry: u32,
        go_type: u32,
        data: [i32; 24],
    ) -> ObjectGuid {
        let guid = ObjectGuid::new_gameobject(entry, 1);
        let template = GameObjectTemplate {
            entry,
            go_type,
            display_id: 1,
            name: format!("GameObject {entry}"),
            icon_name: String::new(),
            cast_bar_caption: String::new(),
            faction: 0,
            flags: 0,
            size: 1.0,
            data,
        };
        world
            .managers
            .gameobject_mgr
            .add_template_for_test(template.clone());
        world
            .managers
            .gameobject_mgr
            .add_gameobject_for_test(GameObject::new(
                guid,
                entry,
                1,
                Position::default(),
                0,
                &template,
                [0.0; 4],
                1,
                100,
            ));
        guid
    }

    fn add_quest(world: &crate::World, quest_id: u32, go_entry: u32) {
        world
            .systems
            .quest
            .manager
            .add_quest_template(QuestTemplate {
                id: quest_id,
                title: format!("Quest {quest_id}"),
                quest_level: 10,
                ..QuestTemplate::default()
            });
        world
            .systems
            .quest
            .manager
            .add_go_quest_starter(go_entry, quest_id);
    }

    fn read_packet(rx: &mut mpsc::UnboundedReceiver<WorldPacket>) -> WorldPacket {
        rx.try_recv().expect("expected outbound packet")
    }

    #[test]
    fn modern_gameobject_use_reads_packed_guid128() {
        let expected = ObjectGuid::new_gameobject(151955, 8171);
        let (high, low) = expected.to_guid128(1);
        let mut writer = BitWriter::new();
        writer.write_packed_guid_128(high, low);
        let mut packet = WorldPacket::new(Opcode::CMSG_GAMEOBJ_USE);
        packet.write_bytes(&writer.into_bytes());

        assert_eq!(read_gameobject_use_guid(Protocol::Modern, &mut packet).unwrap(), expected);
    }

    #[tokio::test]
    async fn gameobject_with_gossip_id_sends_gossip_message() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let mut data = [0; 24];
        data[3] = 7;
        let go_guid = add_gameobject(&world, 200, GameObjectType::QuestGiver as u32, data);

        let mut packet = WorldPacket::new(Opcode::CMSG_GAMEOBJ_USE);
        packet.write_guid(go_guid);
        handle_gameobj_use(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_GOSSIP_MESSAGE);
        assert_eq!(&out.data()[0..8], &go_guid.raw().to_le_bytes());
    }

    #[tokio::test]
    async fn gameobject_without_gossip_id_sends_direct_quest_list() {
        let mut world = test_world();
        install_mock_quest_system(&mut world);
        let (session, mut rx) = add_player(&mut world);
        let go_guid = add_gameobject(&world, 201, GameObjectType::QuestGiver as u32, [0; 24]);
        add_quest(&world, 1, 201);

        let mut packet = WorldPacket::new(Opcode::CMSG_GAMEOBJ_USE);
        packet.write_guid(go_guid);
        handle_gameobj_use(&session, &mut packet, &world)
            .await
            .expect("handler should succeed");

        let out = read_packet(&mut rx);
        assert_eq!(out.opcode(), Opcode::SMSG_QUESTGIVER_QUEST_LIST);
    }
}
