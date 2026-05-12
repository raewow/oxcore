//! Character handlers - enumeration, login, logout

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

use crate::shared::database::{CharacterRepository, Databases, ReputationRepository};
use crate::shared::messages::character::{
    SmsgLogoutCancelAck, SmsgLogoutComplete, SmsgLogoutResponse,
};
use crate::shared::messages::social::SmsgStandstateUpdate;
use crate::shared::messages::create::SmsgOutOfRange;
use crate::shared::messages::login::{
    CharacterEnumEntry, EquipmentSlot, SmsgBindPointUpdate, SmsgCharEnum, SmsgInitWorldStates,
    SmsgInitializeFactionsEmpty, SmsgLoginSetTimeSpeed,
    SmsgLoginVerifyWorld, SmsgSetRestStart,
};
use crate::shared::messages::movement::{SmsgForceMoveRoot, SmsgForceMoveUnroot};
use crate::shared::messages::update::{
    CreateObjectBlock, ObjectType, SmsgUpdateObject, UpdateBlockData,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, Position, WorldPacket};
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::core::common::position::Position as WorldPosition;
use crate::world::game::common::update_fields::{
    OBJECT_FIELD_GUID, OBJECT_FIELD_SCALE_X, OBJECT_FIELD_TYPE, PLAYER_BYTES, PLAYER_BYTES_2,
    PLAYER_FIELD_COINAGE, PLAYER_FIELD_INV_SLOT_HEAD, PLAYER_FIELD_PACK_SLOT_1,
    PLAYER_FIELD_WATCHED_FACTION_INDEX, PLAYER_FLAGS, PLAYER_NEXT_LEVEL_XP, PLAYER_QUEST_LOG_1_1,
    PLAYER_SKILL_INFO_1_1, PLAYER_XP, UNIT_FIELD_BYTES_0, UNIT_FIELD_BYTES_1,
    UNIT_FIELD_DISPLAYID, UNIT_FIELD_FACTIONTEMPLATE, UNIT_FIELD_FLAGS, UNIT_FIELD_HEALTH,
    UNIT_FIELD_LEVEL, UNIT_FIELD_MAXHEALTH, UNIT_FIELD_MAXPOWER1, UNIT_FIELD_NATIVEDISPLAYID,
    UNIT_FIELD_POWER1,
};
use crate::world::game::common::object_type::update_flags;
use crate::world::game::common::object_type::ObjectTypeId;
use crate::world::core::common::packet_compression::compress_update_packet_if_needed;
use crate::world::core::session::{SessionState, WorldSession};
use crate::world::game::player::reputation::FactionEntry;
use crate::world::game::player::{Player, PlayerBroadcaster};
use crate::world::World;

/// Handle CMSG_CHAR_ENUM - list characters for this account
pub async fn handle_char_enum(
    session: &WorldSession,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    let account_id = session.account_id();

    // Use CharacterRepository to fetch characters
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    let characters = char_repo.find_by_account(account_id).await?;

    // Query equipped items for all characters at once
    // Load equipped items for all characters in a single batch query
    let character_guids: Vec<u32> = characters.iter().map(|c| c.guid).collect();
    let equipped_items = char_repo
        .find_equipped_items_batch(&character_guids)
        .await
        .unwrap_or_default();

    // Convert to CharacterEnumEntry
    let char_entries: Vec<CharacterEnumEntry> = characters
        .iter()
        .map(|row| {
            // Convert player_flags to character_flags
            const PLAYER_FLAGS_GHOST: u32 = 0x00000010;
            const CHARACTER_FLAG_GHOST: u32 = 0x00002000;
            const PLAYER_FLAGS_HIDE_HELM: u32 = 0x00000400;
            const PLAYER_FLAGS_HIDE_CLOAK: u32 = 0x00000800;

            let mut char_flags: u32 = 0;
            if row.character_flags & PLAYER_FLAGS_GHOST != 0 {
                char_flags |= CHARACTER_FLAG_GHOST;
            }
            if row.character_flags & PLAYER_FLAGS_HIDE_HELM != 0 {
                char_flags |= PLAYER_FLAGS_HIDE_HELM;
            }
            if row.character_flags & PLAYER_FLAGS_HIDE_CLOAK != 0 {
                char_flags |= PLAYER_FLAGS_HIDE_CLOAK;
            }

            // Build equipment array from equipped items
            let mut equipment = [EquipmentSlot {
                display_id: 0,
                inventory_type: 0,
            }; 19];
            if let Some(equipped) = equipped_items.get(&row.guid) {
                for (slot, item_id) in equipped {
                    if *slot < 19 && *item_id != 0 {
                        if let Some(template) = world.managers.item_mgr.get_template(*item_id) {
                            equipment[*slot as usize] = EquipmentSlot {
                                display_id: template.display_id,
                                inventory_type: template.inventory_type,
                            };
                        }
                    }
                }
            }

            CharacterEnumEntry {
                guid: row.guid,
                name: row.name.clone(),
                race: row.race,
                class: row.class,
                gender: row.gender,
                skin: row.skin,
                face: row.face,
                hair_style: row.hair_style,
                hair_color: row.hair_color,
                facial_hair: row.facial_hair,
                level: row.level,
                zone: row.zone,
                map: row.map,
                position_x: row.position_x,
                position_y: row.position_y,
                position_z: row.position_z,
                guild_id: 0, // TODO: Implement guild lookup
                character_flags: char_flags,
                first_login: false, // TODO: Implement first login detection
                pet_info: None,     // TODO: Implement pet lookup
                equipment,
            }
        })
        .collect();

    // Build and send the packet using SmsgCharEnum
    let message = SmsgCharEnum {
        characters: &char_entries,
    };

    session.send_msg(message)?;

    Ok(())
}

/// Handle CMSG_PLAYER_LOGIN - enter world
pub async fn handle_player_login(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    let guid_raw = packet
        .read_u64()
        .ok_or_else(|| anyhow!("Failed to read character GUID"))?;
    handle_player_login_with_guid(session, guid_raw, databases, world).await
}

/// Handle player login with a pre-extracted GUID.
/// Used by the spawned login task (socket reads GUID before spawning).
pub async fn handle_player_login_with_guid(
    session: &WorldSession,
    guid_raw: u64,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    info!("[LOGIN] handle_player_login ENTER");
    let login_start = std::time::Instant::now();

    // 1. Read character GUID (u64, but only low 32 bits used for players)
    let guid = ObjectGuid::new_player(guid_raw as u32);
    info!("[LOGIN] GUID read: {:?}", guid);

    info!(
        "[LOGIN] Player login request for GUID {} from account {}",
        guid,
        session.account_id()
    );

    // 2. Load character from database using repository and validate ownership
    info!("[LOGIN] Loading character from database");
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    let character = char_repo
        .find_by_guid(guid.counter())
        .await?
        .ok_or_else(|| anyhow!("Character {} not found", guid))?;
    info!("[LOGIN TIMING] Character loaded: {}ms", login_start.elapsed().as_millis());
    info!("[LOGIN] Character loaded: {}", character.name);

    // DIAGNOSTIC: Log position from database
    info!(
        "[LOGIN] Character position from DB: map={}, zone={}, pos=({:.2}, {:.2}, {:.2}, {:.2})",
        character.map,
        character.zone,
        character.position_x,
        character.position_y,
        character.position_z,
        character.orientation
    );

    if character.account != session.account_id() {
        warn!(
            "Character {} does not belong to account {} (belongs to {})",
            guid,
            session.account_id(),
            character.account
        );
        return Err(anyhow!("Character does not belong to this account"));
    }

    info!(
        "[LOGIN] Loading character '{}' (level {}) at map {} zone {}",
        character.name, character.level, character.map, character.zone
    );

    // 3. Create slim Player object
    info!("[LOGIN] Creating Player object");
    let mut player = Player::new(
        guid,
        character.name.clone(),
        character.map,
        character.instance,
        character.zone,
        character.level,
        character.race,
        character.class,
        character.gender,
    );
    player.set_appearance(
        character.skin,
        character.face,
        character.hair_style,
        character.hair_color,
        character.facial_hair,
    );
    player.rest_bonus = character.rest_bonus;
    player.player_flags = character.character_flags;
    player.xp = character.xp;

    // Load homebind from database
    let homebind = char_repo.find_homebind(guid.counter()).await?;
    if let Some(hb) = &homebind {
        player.homebind_map = hb.map;
        player.homebind_zone = hb.zone;
        player.homebind_x = hb.position_x;
        player.homebind_y = hb.position_y;
        player.homebind_z = hb.position_z;
        info!(
            "[LOGIN] Homebind loaded: map={}, zone={}, pos=({:.1}, {:.1}, {:.1})",
            hb.map, hb.zone, hb.position_x, hb.position_y, hb.position_z
        );
    } else {
        // No homebind in DB — use current character position as fallback
        player.homebind_map = character.map;
        player.homebind_zone = character.zone;
        player.homebind_x = character.position_x;
        player.homebind_y = character.position_y;
        player.homebind_z = character.position_z;
        info!("[LOGIN] No homebind found, using current position as fallback");
    }
    player.next_level_xp =
        crate::world::game::player::experience::calculate_xp_for_level(player.level);

    // 4. Add to PlayerManager
    info!("[LOGIN] Adding player to PlayerManager");
    world
        .managers
        .player_mgr
        .add_player(player, session.account_id());

    // Block the map update loop from processing this player's visibility
    // until after the self-create packet is sent. VisibilityState::new() sets
    // dirty=true and force_immediate=true, which would cause the map loop to
    // send nearby object CREATE_OBJECT2 packets before the player's own
    // self-create arrives at the client, causing the camera to spin.
    world
        .managers
        .player_mgr
        .with_player_mut(guid, |player| {
            player.visibility.update_in_progress = true;
        });
    info!("[LOGIN] Player added to PlayerManager");

    // 4.5. Create and set PlayerBroadcaster
    info!("[LOGIN] Creating PlayerBroadcaster");
    if let Some(mut player_mut) = world.managers.player_mgr.get_player_mut(guid) {
        let packet_tx = session.packet_tx();
        let broadcaster = PlayerBroadcaster::new(packet_tx, guid);
        player_mut.set_broadcaster(std::sync::Arc::new(broadcaster));
        info!(
            "[LOGIN] PlayerBroadcaster created and set for player {:?}",
            guid
        );
    }

    // 5. Add player to map (for spatial organization and visibility)
    info!("[LOGIN] Adding player to map");
    if let Some(_player_ref) = world.managers.player_mgr.get_player(guid) {
        let map = world.managers.map_mgr.get_or_create_map(_player_ref.map_id, _player_ref.instance_id);
        map.add_player(
            guid,
            Position::new(
                character.position_x,
                character.position_y,
                character.position_z,
                character.orientation,
            ),
        );
        info!(
            "[LOGIN] Added player {:?} to map {} at ({:.1}, {:.1}, {:.1})",
            guid,
            _player_ref.map_id,
            character.position_x,
            character.position_y,
            character.position_z
        );
    }

    // 5.1 Force load grids around player - spawns creatures before visibility check
    // 5.1 Trigger async grid loading (returns immediately, loads in background)
    info!("[LOGIN] Triggering async grid loading for player");
    world.systems.grid.async_load_grids_for_player(
        guid,
        character.map,
        character.instance,
        Arc::new(world.clone()),
    );
    info!("[LOGIN TIMING] Grid loading triggered: {}ms", login_start.elapsed().as_millis());
    info!("[LOGIN] Async grid loading triggered (will complete in background)");

    // 6. Set player GUID in session (but don't set LoggedIn yet - that happens after visibility)
    info!("[LOGIN] Setting player GUID in session");
    session.set_player_guid(Some(guid));
    world.session_mgr.register_player(session.id(), guid);
    info!("[LOGIN] Player GUID set in session");

    // 7. Initialize systems for this player
    info!("[LOGIN] Initializing systems");
    let initial_position = Position::new(
        character.position_x,
        character.position_y,
        character.position_z,
        character.orientation,
    );
    world
        .systems
        .on_player_login(guid, Some(initial_position), world)
        .await?;
    info!("[LOGIN TIMING] Systems initialized: {}ms", login_start.elapsed().as_millis());
    info!(
        "[LOGIN] Systems initialized with initial position ({:.2}, {:.2}, {:.2})",
        character.position_x, character.position_y, character.position_z
    );

    // 7.0.0.5 Restore ghost/death state if the player logged out while dead.
    // The character row carries PLAYER_FLAGS_GHOST and `death_expire_time`
    // across sessions. We re-apply the ghost aura (8326), water-walking and
    // run-speed bonus so the client renders the ghost correctly.
    {
        const PLAYER_FLAGS_GHOST_RAW: u32 = 0x00000010;
        if character.character_flags & PLAYER_FLAGS_GHOST_RAW != 0 {
            use crate::world::game::player::death::state::DeathState;

            world.managers.player_mgr.with_player_mut(guid, |player| {
                player.death.death_state = DeathState::Dead;
                player.death.death_expire_time = character.death_expire_time;
                player.player_flags |= PLAYER_FLAGS_GHOST_RAW;
                player.movement.water_walking = true;
                player.movement.run_speed = 7.0 * 1.5; // ghost speed
                // Re-apply spell 8326 via the aura container so visibility
                // rules pick it up on the next tick.
                use crate::world::game::player::auras::aura::{Aura, AuraFlags};
                use crate::world::game::player::auras::effects;
                let flags = AuraFlags {
                    is_positive: true,
                    is_negative: false,
                    is_passive: true,
                    can_be_cancelled: false,
                    is_hidden: false,
                    is_permanent: true,
                };
                let aura = Aura::new(8326, guid, 0, effects::AURA_GHOST, 0, 0, None, 0, 1, 0, flags);
                player.auras.container.add_aura(aura);
                player.auras.needs_client_update = true;
            });

            // Link any persisted corpse (best-effort; a missing row is non-fatal).
            use crate::shared::database::characters::repositories::CorpseRepository;
            let char_db = Arc::new(databases.character.clone());
            let corpse_repo = CorpseRepository::new(char_db);
            if let Ok(Some(row)) = corpse_repo.find_for_player(guid.counter()).await {
                let corpse_guid = crate::shared::protocol::ObjectGuid::new_corpse(row.guid);
                let corpse_pos = crate::shared::protocol::Position::new(
                    row.position_x,
                    row.position_y,
                    row.position_z,
                    row.orientation,
                );
                world.managers.player_mgr.with_player_mut(guid, |player| {
                    player.death.corpse_guid = Some(corpse_guid);
                    player.death.corpse_position = Some(corpse_pos);
                    player.death.corpse_map_id = Some(row.map);
                });
            }
        } else {
            // Even for alive players, restore the reclaim-streak clock.
            world.managers.player_mgr.with_player_mut(guid, |player| {
                player.death.death_expire_time = character.death_expire_time;
            });
        }
    }

    // 7.0.0 Initialize rest state from saved data + offline accumulation
    {
        use crate::world::game::player::environment::{RestType, PLAYER_FLAGS_RESTING};

        // Determine saved rest type from character flags
        let saved_rest_type = if character.character_flags & PLAYER_FLAGS_RESTING != 0 {
            // Was resting when logged out — we don't know if tavern or city,
            // but InCity is safe since the zone check below will correct it
            RestType::InCity
        } else {
            RestType::No
        };

        // Restore rest bonus + calculate offline rest accumulation
        world.systems.environment.on_player_login(
            guid,
            character.rest_bonus,
            saved_rest_type,
            character.logout_time,
            &world.managers.player_mgr,
        )?;

        // Check if login zone is a capital city and set rest state
        const AREA_FLAG_CAPITAL: u32 = 0x100;
        let is_capital = {
            let dbc = world.dbc.read();
            if let Some(area) = dbc.get_area(character.zone) {
                if area.flags & AREA_FLAG_CAPITAL != 0 {
                    true
                } else if area.zone != 0 {
                    dbc.get_area(area.zone)
                        .map(|parent| parent.flags & AREA_FLAG_CAPITAL != 0)
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_capital {
            info!("[LOGIN] Player logging in to capital city zone={}, setting rest state", character.zone);
            world.systems.environment.set_rest_type(
                guid,
                RestType::InCity,
                0,
                &world.managers.player_mgr,
            )?;
        }
    }

    // 7.0.1 Load player data from database in parallel
    info!("[LOGIN] Loading player data (reputation, skills, actions) in parallel");

    let (reputation_rows, skill_rows, action_rows, active_quests, rewarded_quests) = {
        let char_guid = guid.counter();
        let char_db = Arc::new(databases.character.clone());

        tokio::try_join!(
            // Reputation from DB
            async {
                let rep_repo = ReputationRepository::new(char_db.clone());
                rep_repo.find_reputations(char_guid).await
            },

            // Skills from DB
            async {
                let char_repo = CharacterRepository::new(char_db.clone());
                char_repo.find_skills(char_guid).await
            },

            // Actions from DB
            async {
                let char_repo = CharacterRepository::new(char_db.clone());
                char_repo.find_actions(char_guid).await
            },

            // Quest statuses from DB
            async {
                use crate::shared::database::characters::repositories::{QuestRepository, QuestRepositoryTrait};
                let quest_repo = QuestRepository::new(char_db.clone());
                quest_repo.find_quest_statuses(char_guid).await
            },

            // Rewarded quests from DB
            async {
                use crate::shared::database::characters::repositories::{QuestRepository, QuestRepositoryTrait};
                let quest_repo = QuestRepository::new(char_db.clone());
                quest_repo.find_rewarded_quests(char_guid).await
            }
        )?
    };

    info!("[LOGIN TIMING] Parallel data loaded (rep/skills/actions/quests): {}ms", login_start.elapsed().as_millis());
    info!(
        "[LOGIN] Loaded {} reputation, {} skill, {} action, {} quest, {} rewarded entries in parallel",
        reputation_rows.len(),
        skill_rows.len(),
        action_rows.len(),
        active_quests.len(),
        rewarded_quests.len()
    );

    // Load quest state into player
    world.systems.quest.load_from_db(guid, active_quests, rewarded_quests);
    info!("[LOGIN] Quest state loaded from database");

    // Initialize reputation system with cached DBC data
    world.systems.reputation.initialize(guid, world)?;
    info!("[LOGIN] Reputation system initialized with cached DBC faction data");

    // Load saved reputation from database
    let rep_data: Vec<(u32, i32, i32)> = reputation_rows
        .into_iter()
        .map(|r| (r.faction, r.standing, r.flags))
        .collect();
    world
        .systems
        .reputation
        .load_from_db(guid, rep_data, world)?;
    info!("[LOGIN] Reputation data loaded from database");

    // 7.05 Initialize skills from loaded data
    if skill_rows.is_empty() {
        info!("[LOGIN] No skills found, initializing default skills for class/race");
        world.systems.player.manager().with_player_mut(guid, |player| {
            crate::world::game::player::skills::initialize_default_skills(
                &mut player.skills,
                player.class,
                player.race,
                player.level,
            );
        });
    } else {
        // Load existing skills from database
        world.systems.player.manager().with_player_mut(guid, |player| {
                for (position, row) in skill_rows.iter().enumerate() {
                    let skill_data = crate::world::game::player::skills::SkillData {
                        skill_id: row.skill as u16,
                        current_value: row.value as u16,
                        max_value: row.max as u16,
                        step: 0,
                        position,
                        state: crate::world::game::player::skills::SkillSaveState::Unchanged,
                    };
                    player.skills.skills.insert(row.skill as u16, skill_data);
                }
        });
    }
    info!("[LOGIN] Skills initialized");

    // 7.06 Initialize action buttons from loaded data
    if !action_rows.is_empty() {
        world
            .managers
            .player_mgr
            .with_player_mut(guid, |player| {
                for row in &action_rows {
                    info!("[LOGIN] Setting action button: slot={}, action={}, type={}", 
                          row.button, row.action, row.r#type);
                    player
                        .settings
                        .set_action_button(row.button, row.action, row.r#type);
                }
            });
        info!(
            "[LOGIN] Loaded {} action buttons from database",
            action_rows.len()
        );
    } else {
        // No saved action buttons - load defaults from playercreateinfo_action
        use crate::shared::database::world::repositories::PlayerCreateInfoRepository;
        let create_repo = PlayerCreateInfoRepository::new(Arc::new(databases.world.clone()));
        match create_repo
            .get_create_info_actions(character.race, character.class).await

        {
            Ok(default_actions) if !default_actions.is_empty() => {
                world
                    .managers
                    .player_mgr
                    .with_player_mut(guid, |player| {
                        for action in &default_actions {
                            player.settings.set_action_button(
                                action.button as u8,
                                action.action,
                                action.action_type as u8,
                            );
                        }
                    });
                info!(
                    "[LOGIN] Loaded {} default action buttons from playercreateinfo_action",
                    default_actions.len()
                );
            }
            Ok(_) => {
                info!("[LOGIN] No action buttons found in database or defaults");
            }
            Err(e) => {
                warn!("[LOGIN] Failed to load default action buttons: {}", e);
            }
        }
    }

    // 7.07 Load spells - default race/class spells + saved spells from database (in parallel)
    {
        use crate::shared::database::world::repositories::PlayerCreateInfoRepository;

        const ATTACK_SPELL_ID: u32 = 6603;

        // Load default and saved spells in parallel (non-fatal if tables missing)
        let (default_spells, saved_spells) = {
            let char_guid = guid.counter();
            let race = character.race;
            let class = character.class;
            let world_db = Arc::new(databases.world.clone());
            let char_db = Arc::new(databases.character.clone());

            tokio::join!(
                async {
                    let create_repo = PlayerCreateInfoRepository::new(world_db);
                    create_repo
                        .get_create_info_spells(race, class).await
                        .unwrap_or_else(|e| {
                            warn!("[LOGIN] Failed to load default spells: {}", e);
                            Vec::new()
                        })
                },
                async {
                    let char_repo = CharacterRepository::new(char_db);
                    char_repo
                        .find_spells(char_guid).await
                        .unwrap_or_else(|e| {
                            warn!("[LOGIN] Failed to load saved spells: {}", e);
                            Vec::new()
                        })
                }
            )
        };

        world
            .managers
            .player_mgr
            .with_player_mut(guid, |player| {
                // Always learn Attack (spell 6603)
                player.spells.learn_spell(ATTACK_SPELL_ID);

                // Learn default spells for this race/class
                for row in &default_spells {
                    player.spells.learn_spell(row.spell);
                }

                // Learn saved spells from database
                for row in &saved_spells {
                    if row.active != 0 && row.disabled == 0 {
                        player.spells.learn_spell(row.spell);
                    }
                }
            });

        info!(
            "[LOGIN] Loaded {} default + {} saved spells",
            default_spells.len(),
            saved_spells.len()
        );
    }

    // 7.08 Load quest progress (active and rewarded quests)
    info!("[LOGIN] Loading quest progress from database");
    world.systems.quest.load_player_quests(guid).await?;
    info!("[LOGIN] Quest progress loaded");

    // 7.1 Load inventory before sending any packets
    info!("[LOGIN] Loading inventory from database");
    world.systems.inventory.load_player_inventory(guid).await?;
    info!("[LOGIN] Inventory loaded");

    // 7.2 Send inventory to client
    info!("[LOGIN] Sending inventory to client");
    info!("[LOGIN] Inventory sent");

    // 7.2 Send money update
    if let Some(money) = world.systems.inventory.get_money(guid) {
        info!("[LOGIN] Sending money update: {} copper", money);
        world.systems.inventory.send_money_update(guid, money);
    }

    // 7.3 Visibility flags are already set by VisibilityState::new() (dirty=true, force_immediate=true).
    // update_in_progress=true was set in step 4 to prevent the map loop from processing
    // visibility before the self-create is sent. It will be cleared after the self-create below.

    // 7.4 Set session state to LoggedIn (before sending login packets)
    info!("[LOGIN] Setting session state to LoggedIn");
    session.set_state(crate::world::core::session::SessionState::LoggedIn);
    info!("[LOGIN] Session state updated to LoggedIn");

    // 7.5 Create player packet handler task
    use crate::world::core::network::player_handler::PlayerPacketHandler;
    // Get Arc<WorldSession> from session manager
    let session_arc = world.session_mgr.get_session(session.id())
        .ok_or_else(|| anyhow::anyhow!("Session not found in manager"))?;
    let handler = PlayerPacketHandler::spawn(
        guid,
        session_arc,
        Arc::clone(&world.databases),
        Arc::new(world.clone()),
    );
    world.player_handlers.insert(guid, handler);
    tracing::info!("[LOGIN] Created packet handler for player {}", guid);

    // 7.6 Create movement buffer for map-level movement processing
    world.create_movement_buffer(guid);
    tracing::info!("[LOGIN] Created movement buffer for player {}", guid);

    // 8. Send login packets using message structs
    // Order is CRITICAL - must match MaNGOS/old world sequence

    // 1/11. SMSG_LOGIN_VERIFY_WORLD - tells client where they are
    let verify_world = SmsgLoginVerifyWorld {
        map_id: character.map,
        position: Position::new(
            character.position_x,
            character.position_y,
            character.position_z,
            character.orientation,
        ),
    };
    session.send_msg(verify_world)?;
    info!(
        "[LOGIN] 1/11 SMSG_LOGIN_VERIFY_WORLD: map={}, pos=({:.1}, {:.1}, {:.1})",
        character.map, character.position_x, character.position_y, character.position_z
    );

    // 1.5/11. SMSG_NAME_QUERY_RESPONSE - send player's own name info so chat works
    // The client needs this BEFORE it can display chat messages with this GUID
    let name_response = crate::shared::messages::query::SmsgNameQueryResponse::new(
        guid,
        &character.name,
        character.race,
        character.gender,
        character.class,
    );
    session.send_msg(name_response)?;
    info!(
        "[LOGIN] 1.5/11 SMSG_NAME_QUERY_RESPONSE: sent player's own name info for {:?}",
        guid
    );

    // 2/11. SMSG_ACCOUNT_DATA_TIMES - account data timestamps
    // Send directly via session (not broadcast_mgr) to ensure delivery during login
    {
        use crate::shared::messages::settings::SmsgAccountDataTimes;
        use crate::world::game::player::settings::state::NUM_ACCOUNT_DATA_TYPES;

        let timestamps = world
            .managers
            .player_mgr
            .with_player(guid, |player| {
                let mut times = [0u32; NUM_ACCOUNT_DATA_TYPES];
                for (i, entry) in player.settings.account_data.iter().enumerate() {
                    if let Some(e) = entry {
                        times[i] = e.time;
                    }
                }
                times
            })
            .unwrap_or([0u32; NUM_ACCOUNT_DATA_TYPES]);

        let msg = SmsgAccountDataTimes::new(timestamps);
        session.send_msg(msg)?;
    }
    info!("[LOGIN] 2/11 SMSG_ACCOUNT_DATA_TIMES");

    // 3/11. SMSG_SET_REST_START - rest state time
    session.send_msg(SmsgSetRestStart { time: 0 })?;
    info!("[LOGIN] 3/11 SMSG_SET_REST_START: time=0");

    // 4/11. SMSG_BINDPOINTUPDATE - hearthstone location from database
    let (hb_map, hb_zone, hb_x, hb_y, hb_z) = world
        .managers
        .player_mgr
        .with_player(guid, |p| {
            (p.homebind_map, p.homebind_zone, p.homebind_x, p.homebind_y, p.homebind_z)
        })
        .unwrap_or((character.map, character.zone, character.position_x, character.position_y, character.position_z));
    let bind_point = SmsgBindPointUpdate {
        x: hb_x,
        y: hb_y,
        z: hb_z,
        map_id: hb_map,
        zone_id: hb_zone,
    };
    session.send_msg(bind_point)?;
    info!(
        "[LOGIN] 4/11 SMSG_BINDPOINTUPDATE: map={}, zone={}",
        hb_map, hb_zone
    );

    // 5/11. SMSG_TUTORIAL_FLAGS - tutorial state from player settings
    // Send directly via session (not broadcast_mgr) to ensure delivery during login
    {
        use crate::shared::messages::login::SmsgTutorialFlags;
        use crate::world::game::player::settings::state::TUTORIAL_FLAG_COUNT;

        let flags = world
            .managers
            .player_mgr
            .with_player(guid, |player| player.settings.tutorial_flags)
            .unwrap_or([0xFFFFFFFF; TUTORIAL_FLAG_COUNT]);

        let msg = SmsgTutorialFlags { flags };
        session.send_msg(msg)?;
    }
    info!("[LOGIN] 5/11 SMSG_TUTORIAL_FLAGS: sent from player settings");

    // 6/11. SMSG_INITIAL_SPELLS - send spellbook to client
    // Send directly via session (not broadcast_mgr) to ensure delivery during login
    {
        use crate::shared::messages::spells::{InitialSpellCooldown, SmsgInitialSpells};

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut spellbook: Vec<u32> = Vec::new();
        let mut cooldowns: Vec<InitialSpellCooldown> = Vec::new();

        world.systems.player.manager().with_player_mut(guid, |player| {
            spellbook = player.spells.spellbook.clone();

            for (&spell_id, &cd_end) in &player.spells.cooldowns {
                if cd_end > now {
                    cooldowns.push(InitialSpellCooldown {
                        spell_id,
                        item_id: 0,
                        category: 0,
                        spell_cooldown_ms: (cd_end - now) as u32,
                        category_cooldown_ms: 0,
                    });
                }
            }
        });

        let msg = SmsgInitialSpells {
            cast_count: 0,
            spells: spellbook,
            cooldowns,
        };
        session.send_packet(msg.to_world_packet())?;
    }
    info!("[LOGIN] 6/11 SMSG_INITIAL_SPELLS: sent spellbook");

    // 6.5/11. SMSG_SET_PROFICIENCY - weapon and armor proficiencies
    let proficiencies = world.systems.skills.on_player_login(guid, world)?;
    for msg in proficiencies {
        let packet = crate::shared::messages::errors::SmsgSetProficiency {
            item_class: msg.item_class,
            proficiency_mask: msg.sub_class_mask,
        };
        session.send_msg(packet)?;
    }
    info!("[LOGIN] 6.5/11 SMSG_SET_PROFICIENCY: sent weapon/armor proficiencies");

    // 7/11. SMSG_ACTION_BUTTONS - action buttons from player settings
    // Send directly via session (not broadcast_mgr) to ensure delivery during login
    {
        use crate::shared::messages::login::ActionButton as MsgActionButton;
        use crate::shared::messages::login::SmsgActionButtons;
        use crate::world::game::player::settings::state::MAX_ACTION_BUTTONS;

        let buttons = world
            .managers
            .player_mgr
            .with_player(guid, |player| {
                let mut result = [MsgActionButton::empty(); MAX_ACTION_BUTTONS];
                for (i, btn) in player.settings.action_buttons.iter().enumerate() {
                    if let Some(btn) = btn {
                        result[i] = MsgActionButton {
                            action: btn.action,
                            action_type: btn.button_type,
                        };
                    }
                }
                result
            })
            .unwrap_or([MsgActionButton::empty(); MAX_ACTION_BUTTONS]);

        let msg = SmsgActionButtons { buttons: &buttons };
        session.send_msg(msg)?;
    }
    info!("[LOGIN] 7/11 SMSG_ACTION_BUTTONS: sent from player settings");

    // 8/11. SMSG_INITIALIZE_FACTIONS - send reputation data
    // Send directly via session (not broadcast_mgr) to ensure delivery during login.
    // Note: ReputationSystem::send_initialize_factions has a bug where it calls
    // async send_msg_to_player without .await from a sync function (future dropped).
    {
        use crate::shared::game::reputation::MAX_REPUTATION_LIST_SLOTS;
        use crate::shared::messages::reputation::SmsgInitializeFactions;

        let faction_data = world
            .systems
            .player
            .manager()
            .with_player_mut(guid, |player| {
                let race = player.race;
                let class = player.class;
                let dbc_guard = world.dbc.read();

                let mut slots: Vec<(u8, u32)> = Vec::with_capacity(MAX_REPUTATION_LIST_SLOTS);

                for rep_list_id in 0..MAX_REPUTATION_LIST_SLOTS as u32 {
                    if let Some(standing) = player.reputation.get_standing(rep_list_id) {
                        let base_rep = dbc_guard
                            .get_faction(standing.faction_id)
                            .map(|e| e.get_base_reputation(race, class))
                            .unwrap_or(0);
                        let absolute = base_rep + standing.standing;
                        slots.push((standing.flags as u8, absolute as u32));
                    } else {
                        slots.push((0, 0));
                    }
                }

                slots
            })
            .unwrap_or_default();

        let msg = SmsgInitializeFactions {
            factions: faction_data
                .into_iter()
                .enumerate()
                .map(|(i, (flags, standing))| (i as u32, (flags, standing as i32)))
                .collect(),
        };
        session.send_msg(msg)?;
    }
    info!("[LOGIN] 8/11 SMSG_INITIALIZE_FACTIONS: sent reputation data");

    // 9/11. SMSG_LOGIN_SETTIMESPEED - game time and speed (CRITICAL)
    session.send_msg(SmsgLoginSetTimeSpeed::default())?;
    info!("[LOGIN] 9/11 SMSG_LOGIN_SETTIMESPEED");

    // Clear update_in_progress BEFORE sending self-create.
    // The self-create goes into the FIFO channel next, so even if the map loop
    // races in after this, its visibility packets queue AFTER our self-create.
    // This avoids DashMap contention that could block the handler and prevent
    // the socket write loop from flushing queued packets.
    world
        .managers
        .player_mgr
        .with_player_mut(guid, |player| {
            player.visibility.update_in_progress = false;
        });

    // 10/11. SMSG_UPDATE_OBJECT - send items first, then player (matches MaNGOS order)

    // Get player reference
    let player_ref = world
        .managers
        .player_mgr
        .get_player(guid)
        .ok_or_else(|| anyhow!("Player not found after adding"))?;

    // Build item create blocks first (like old world does)
    let mut item_blocks: Vec<CreateObjectBlock> = Vec::new();
    world
        .systems
        .inventory
        .build_item_create_blocks(guid, &mut item_blocks);

    // Build player create block
    let player_block = build_player_create_block_for_player(&player_ref, world)?;

    // Combine into single SMSG_UPDATE_OBJECT
    let mut update_object = SmsgUpdateObject::new();
    for block in &item_blocks {
        update_object = update_object.add_block(UpdateBlockData::CreateObject2(block.clone()));
    }
    update_object = update_object.add_block(UpdateBlockData::CreateObject2(player_block));

    // Compressing packet (matching /world behavior)
    // Note: SmsgUpdateObject internally creates a WorldPacket, so we just need to compress and send it
    let mut packet = update_object.to_world_packet();
    let compressed = compress_update_packet_if_needed(packet)?;
    session.send_packet(compressed.clone())?;
    info!(
        "[LOGIN] 10/11 SMSG_UPDATE_OBJECT: 1 player block (opcode={:?})",
        compressed.opcode()
    );
    // 11/11. SMSG_INIT_WORLD_STATES
    let init_world_states = SmsgInitWorldStates::new(character.map, character.zone);
    session.send_msg(init_world_states)?;
    info!(
        "[LOGIN] 11/11 SMSG_INIT_WORLD_STATES: map={}, zone={}",
        character.map, character.zone
    );

    // Clear update_in_progress (already moved to before self-create send).

    info!(
        "[LOGIN] Player '{}' ({:?}) ready to enter world at ({:.1}, {:.1}, {:.1}) map {}",
        character.name,
        guid,
        character.position_x,
        character.position_y,
        character.position_z,
        character.map
    );

    info!("[LOGIN TIMING] Login complete: {}ms total", login_start.elapsed().as_millis());
    info!("[LOGIN] Login sequence complete for player {:?}", guid);

    // Clear login guard so session can be reused if player logs out and back in
    session.clear_login_in_progress();

    Ok(())
}

// TODO: this definitely should go elsewhere
/// Build CREATE_OBJECT block for player creation
/// Returns just the CreateObjectBlock to be added to an SMSG_UPDATE_OBJECT
pub fn build_player_create_block_for_player(
    player: &Player,
    world: &World,
) -> Result<CreateObjectBlock> {
    // Convert world ObjectGuid to world::common ObjectGuid for the message struct
    let world_guid = WorldObjectGuid::from_low(player.guid.counter());

    tracing::info!(
        "[LOGIN-SELF-CREATE] Building self CREATE_OBJECT2 for player {:?}",
        player.guid
    );
    tracing::info!(
        "[LOGIN-SELF-CREATE] Input GUID: raw=0x{:016X}, counter={}",
        player.guid.raw(),
        player.guid.counter()
    );
    tracing::info!(
        "[LOGIN-SELF-CREATE] WorldObjectGuid (from_low): raw=0x{:016X}, counter={}",
        world_guid.raw(),
        world_guid.counter()
    );

    // Get stats from player - stats system should have calculated these

    // Get position from PlayerManager
    let position = world
        .managers
        .player_mgr
        .get_position(player.guid)
        .unwrap_or(Position::xyz(0.0, 0.0, 0.0));

    // DIAGNOSTIC: Log position retrieved from PlayerManager
    tracing::info!(
        "[LOGIN] build_player_create_block: position from PlayerManager for {:?}: ({:.2}, {:.2}, {:.2}, {:.2})",
        player.guid,
        position.x,
        position.y,
        position.z,
        position.o
    );

    // Build position - convert to WorldPosition for wire format
    let world_position = WorldPosition::new(position.x, position.y, position.z, position.o);

    // Calculate field values
    let faction = get_faction_template(player.race);
    let display_id = get_display_id(player.race, player.gender);

    // Collect money from inventory system
    let money = world.systems.inventory.get_money(player.guid).unwrap_or(0);

    // Collect visible item fields from inventory system
    let mut inventory_fields: Vec<(u32, u32)> = Vec::new();
    world
        .systems
        .inventory
        .populate_visible_items(player.guid, &mut inventory_fields);

    // Collect inventory slot GUID fields separately (need to use set_guid_field)
    let mut inventory_guid_fields: Vec<(u32, WorldObjectGuid)> = Vec::new();
    world
        .systems
        .inventory
        .populate_inventory_slots(player.guid, &mut inventory_guid_fields);

    // Sort fields by index to ensure proper update mask ordering
    inventory_fields.sort_by_key(|&(idx, _)| idx);

    tracing::info!(
        "[LOGIN-SELF-CREATE] Movement data: pos=({:.2}, {:.2}, {:.2}, o={:.4}), movement_flags=0 (hardcoded for self-create)",
        world_position.x,
        world_position.y,
        world_position.z,
        world_position.o
    );
    tracing::info!(
        "[LOGIN-SELF-CREATE] Using update_flags: UPDATEFLAG_SELF (0x{:02X}) + UPDATEFLAG_LIVING (added by with_movement)",
        update_flags::UPDATEFLAG_SELF
    );

    // Build the create block using shared message struct
    // UPDATEFLAG_SELF for player self-create, with_movement() adds UPDATEFLAG_LIVING
    // NOTE: UPDATEFLAG_ALL is for ITEMS only, not players!
    let mut create_block =
        CreateObjectBlock::new(world_guid, ObjectTypeId::Player, ObjectType::Player)
            .with_flags(update_flags::UPDATEFLAG_SELF)
            .with_movement(world_position, 0, None)
            .set_fields(inventory_fields);

    // Set inventory slot GUID fields using set_guid_field (ensures proper GUID handling)
    for (field_index, item_guid) in inventory_guid_fields {
        create_block = create_block.set_guid_field(field_index, item_guid);
    }

    // Get health values from player stats (calculated by stats system)
    let current_health = player.stats.health;
    let max_health = player.stats.max_health;
    
    tracing::info!(
        "[LOGIN] Player health: {}/{} (from stats system)",
        current_health,
        max_health
    );

    // Power values from power system
    let current_mana = player.power.current[0];
    let max_mana = player.power.max[0];

    let create_block = create_block
        // Object fields
        .set_guid_field(OBJECT_FIELD_GUID, world_guid)
        .set_field(OBJECT_FIELD_TYPE, 0x19) // TYPEMASK_OBJECT | TYPEMASK_UNIT | TYPEMASK_PLAYER
        .set_float_field(OBJECT_FIELD_SCALE_X, 1.0)
        // Unit fields
        .set_field(UNIT_FIELD_HEALTH, current_health)
        .set_field(UNIT_FIELD_MAXHEALTH, max_health)
        .set_field(UNIT_FIELD_POWER1, current_mana)
        .set_field(UNIT_FIELD_MAXPOWER1, max_mana)
        .set_field(UNIT_FIELD_LEVEL, player.level as u32)
        .set_field(UNIT_FIELD_FACTIONTEMPLATE, faction);

    // UNIT_FIELD_BYTES_0: race | class<<8 | gender<<16 | powerType<<24
    // Using u32 approach like creature code for consistency
    let power_type = get_power_type(player.class) as u32;
    let bytes_0_u32: u32 = (player.race as u32)
        | ((player.class as u32) << 8)
        | ((player.gender as u32) << 16)
        | (power_type << 24);

    tracing::info!(
        "[LOGIN] UNIT_FIELD_BYTES_0: race={}, class={}, gender={}, powerType={} => u32=0x{:08X}",
        player.race,
        player.class,
        player.gender,
        power_type,
        bytes_0_u32
    );

    let create_block = create_block
        // UNIT_FIELD_BYTES_0: race, class, gender, powerType as packed u32
        .set_field(UNIT_FIELD_BYTES_0, bytes_0_u32)
        // UNIT_FIELD_BYTES_1: stand state, pet loyalty, shapeshift form, vis flags
        .set_bytes_field(
            UNIT_FIELD_BYTES_1,
            [player.stand_state, 0, player.shapeshift_form, 0],
        )
        .set_field(UNIT_FIELD_FLAGS, 0x00000008_u32) // PLAYER_CONTROLLED
        .set_field(UNIT_FIELD_DISPLAYID, display_id)
        .set_field(UNIT_FIELD_NATIVEDISPLAYID, display_id)
        // Player fields
        // PLAYER_BYTES: skin, face, hair_style, hair_color
        .set_bytes_field(
            PLAYER_BYTES,
            [
                player.skin,
                player.face,
                player.hair_style,
                player.hair_color,
            ],
        )
        // PLAYER_BYTES_2: facial_style, unk1, bank_bag_slots, rest_state
        // Rest state: 0x01 = rested, 0x02 = normal
        .set_bytes_field(
            PLAYER_BYTES_2,
            [
                player.facial_hair,
                0xEE,
                0,
                if player.rest_bonus > 10.0 { 0x01 } else { 0x02 },
            ],
        )
        .set_field(PLAYER_XP, player.xp)
        .set_field(PLAYER_NEXT_LEVEL_XP, player.next_level_xp)
        .set_field(PLAYER_FLAGS, player.player_flags)
        .set_field(PLAYER_FIELD_COINAGE, money)
        // PLAYER_FIELD_WATCHED_FACTION_INDEX: -1 = no faction being tracked
        .set_field(PLAYER_FIELD_WATCHED_FACTION_INDEX, 0xFFFFFFFF_u32);

    // Add all skills from player's skill state to UPDATE_OBJECT
    // Each skill takes 3 consecutive u32 fields:
    //   Field 0: (skill_id << 16) | step     - skill_id in HIGH 16 bits, step in LOW 16 bits
    //   Field 1: (current << 16) | max       - current value in HIGH 16 bits, max in LOW 16 bits
    //   Field 2: bonus (usually 0, skipped)
    let mut create_block = create_block;
    for skill_data in player.skills.skills.values() {
        if skill_data.state == crate::world::game::player::skills::SkillSaveState::Deleted {
            continue;
        }
        
        let base_field = PLAYER_SKILL_INFO_1_1 + (skill_data.position as u32) * 3;
        let skill_index = ((skill_data.skill_id as u32) << 16) | (skill_data.step as u32);
        let skill_value = ((skill_data.current_value as u32) << 16) | (skill_data.max_value as u32);
        
        create_block = create_block
            .set_field(base_field, skill_index)
            .set_field(base_field + 1, skill_value);
        // Field 2 (bonus) is 0, skipped

        tracing::info!(
            "[LOGIN] Added skill {} at position {}: index=0x{:08X}, value=0x{:08X}",
            skill_data.skill_id, skill_data.position, skill_index, skill_value
        );
    }

    // Quest log fields
    const MAX_QUEST_OFFSET: u32 = 3;
    for (slot, quest) in player.active_quests.iter().enumerate() {
        let slot_u32 = slot as u32;
        let quest_id_field = PLAYER_QUEST_LOG_1_1 + slot_u32 * MAX_QUEST_OFFSET;
        let count_state_field = quest_id_field + 1;
        let timer_field = quest_id_field + 2;

        // Pack creature/go counts into 6-bit fields
        let mut packed: u32 = 0;
        for i in 0..4usize {
            let count = (quest.creature_or_go_count[i] as u32).min(63);
            packed |= count << (i as u32 * 6);
        }

        create_block = create_block
            .set_field(quest_id_field, quest.quest_id)
            .set_field(count_state_field, packed)
            .set_field(timer_field, quest.timer);

        tracing::info!(
            "[LOGIN] Added quest log slot {}: quest_id={}, packed_counts=0x{:08X}, timer={}",
            slot, quest.quest_id, packed, quest.timer
        );
    }

    // Log inventory slot fields for debugging
    for &(field_idx, value) in &create_block.fields {
        if field_idx >= PLAYER_FIELD_INV_SLOT_HEAD && field_idx < PLAYER_FIELD_INV_SLOT_HEAD + 46 {
            let slot = (field_idx - PLAYER_FIELD_INV_SLOT_HEAD) / 2;
            let is_high = (field_idx - PLAYER_FIELD_INV_SLOT_HEAD) % 2;
            tracing::info!(
                "[LOGIN]   Inventory slot field: field={} (0x{:X}), slot={}, is_high={}, value={} (0x{:X})",
                field_idx, field_idx, slot, is_high, value, value
            );
        } else if field_idx >= PLAYER_FIELD_PACK_SLOT_1 && field_idx < PLAYER_FIELD_PACK_SLOT_1 + 32
        {
            let slot_offset = (field_idx - PLAYER_FIELD_PACK_SLOT_1) / 2;
            let slot = slot_offset + 23;
            let is_high = (field_idx - PLAYER_FIELD_PACK_SLOT_1) % 2;
            tracing::info!(
                "[LOGIN]   Pack slot field: field={} (0x{:X}), slot={}, is_high={}, value={} (0x{:X})",
                field_idx, field_idx, slot, is_high, value, value
            );
        }
    }

    Ok(create_block)
}

/// Get faction template for race
fn get_faction_template(race: u8) -> u32 {
    match race {
        1 => 1,   // Human
        2 => 2,   // Orc
        3 => 3,   // Dwarf
        4 => 4,   // Night Elf
        5 => 5,   // Undead
        6 => 6,   // Tauren
        7 => 115, // Gnome
        8 => 116, // Troll
        _ => 1,   // Default to Human
    }
}

/// Get display ID for race/gender
fn get_display_id(race: u8, gender: u8) -> u32 {
    // These are the base model display IDs for each race/gender
    match (race, gender) {
        (1, 0) => 49,   // Human Male
        (1, 1) => 50,   // Human Female
        (2, 0) => 51,   // Orc Male
        (2, 1) => 52,   // Orc Female
        (3, 0) => 53,   // Dwarf Male
        (3, 1) => 54,   // Dwarf Female
        (4, 0) => 55,   // Night Elf Male
        (4, 1) => 56,   // Night Elf Female
        (5, 0) => 57,   // Undead Male
        (5, 1) => 58,   // Undead Female
        (6, 0) => 59,   // Tauren Male
        (6, 1) => 60,   // Tauren Female
        (7, 0) => 1563, // Gnome Male
        (7, 1) => 1564, // Gnome Female
        (8, 0) => 1478, // Troll Male
        (8, 1) => 1479, // Troll Female
        _ => 49,        // Default Human Male
    }
}

/// Get power type for class
fn get_power_type(class: u8) -> u32 {
    match class {
        1 => 1,  // Warrior - Rage
        2 => 0,  // Paladin - Mana
        3 => 0,  // Hunter - Mana
        4 => 3,  // Rogue - Energy
        5 => 0,  // Priest - Mana
        6 => 0,  // Death Knight - Runic Power (not in vanilla)
        7 => 0,  // Shaman - Mana
        8 => 0,  // Mage - Mana
        9 => 0,  // Warlock - Mana
        11 => 0, // Druid - Mana
        _ => 0,
    }
}

/// Get primary language skill ID for race
/// Returns the skill ID for the race's native language
fn get_language_skill_for_race(race: u8) -> u16 {
    match race {
        // Alliance races - Common (skill 98)
        1 | 3 | 4 | 7 => 98, // Human, Dwarf, Night Elf, Gnome
        // Horde races - Orcish (skill 109)
        2 | 5 | 6 | 8 => 109, // Orc, Undead, Tauren, Troll
        _ => 98,              // Default to Common
    }
}

/// Handle CMSG_LOGOUT_REQUEST - player requests to log out
pub async fn handle_logout_request(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    debug!("Logout requested for session {}", session.id());

    // Get player GUID
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("Logout requested but no player in session {}", session.id());
            return Ok(());
        }
    };

    // Check player exists (drop the DashMap guard immediately to avoid deadlock
    // with with_player_mut() later in this function)
    if world.managers.player_mgr.get_player(player_guid).is_none() {
        warn!("Logout requested but player {} not found", player_guid);
        return Ok(());
    }

    // Check if player is in combat
    let player_mgr = world.systems.player.manager();
    let in_combat = world.systems.combat.is_in_combat(player_guid, &player_mgr);

    // Determine if instant logout (config = 0 OR resting in city/tavern)
    let logout_timer_secs = world.config.logout_timer;

    // Check if player is in a rested area (inn or capital city)
    let in_rested_area = world.managers.player_mgr.with_player(player_guid, |player| {
        use crate::world::game::player::environment::RestType;
        matches!(player.environment.rest_type, RestType::InTavern | RestType::InCity)
    }).unwrap_or(false);

    let instant_logout = logout_timer_secs == 0 || in_rested_area;

    if in_rested_area {
        info!(
            "Player {} is in rested area, allowing instant logout",
            player_guid
        );
    }

    // Determine logout reason
    let reason = if in_combat {
        1u8 // In combat
    } else {
        0u8 // Can logout
    };

    // Send SMSG_LOGOUT_RESPONSE
    let response = SmsgLogoutResponse {
        reason,
        instant: instant_logout,
    };
    session.send_msg(response)?;

    if !in_combat {
        if instant_logout {
            // Instant logout - perform immediately
            perform_logout_cleanup(session, world).await?;
        } else {
            // Start timer
            session.start_logout_timer();
            info!(
                "[LOGOUT_TIMER] Started {}-second logout timer for session {} (account: {}, player: {})",
                logout_timer_secs,
                session.id(),
                session.account_name(),
                player_guid
            );

            // Root player to prevent movement during logout
            let world_guid = WorldObjectGuid::from_raw(player_guid.raw());
            let root_msg = SmsgForceMoveRoot { guid: world_guid };
            session.send_msg(root_msg)?;

            // Set server-side state: sit and root
            const STAND_STATE_SIT: u8 = 1;
            const UNIT_FLAG_DISABLE_MOVE: u32 = 0x00000004;

            world.managers.player_mgr.with_player_mut(player_guid, |player| {
                player.stand_state = STAND_STATE_SIT;
                player.unit_flags |= UNIT_FLAG_DISABLE_MOVE;
                info!(
                    "[LOGOUT] Set logout state for {}: stand_state={}, unit_flags=0x{:08X}",
                    player_guid, player.stand_state, player.unit_flags
                );
            });

            // Send SMSG_STANDSTATE_UPDATE to show sit animation
            session.send_msg(SmsgStandstateUpdate {
                stand_state: STAND_STATE_SIT,
            })?;

            info!("[LOGOUT] Player {} rooted and sitting during logout countdown", player_guid);

            // TODO: PLAYER STATE SYSTEM - Comprehensive player state tracking needed
            // =======================================================================
            // Currently we only send SMSG_FORCE_MOVE_ROOT packet which tells the CLIENT
            // not to allow movement. However, we don't track this state server-side.
            //
            // In old /world implementation (src/world/game/objects/player/player.rs):
            // - set_rooted(true) sets the DISABLE_MOVE unit flag (0x00000004)
            // - set_stand_state(StandState::Sit) sets visual sit animation
            // - Both are tracked in the Player object as fields
            //
            // PLAYER STATE TYPES (from old /world):
            // -------------------------------------
            // 1. StandState (u8 enum) - Visual animation state
            //    Values: Stand=0, Sit=1, SitChair=2, Sleep=3, SitLowChair=4,
            //            SitMediumChair=5, SitHighChair=6, Dead=7, Kneel=8, Submerged=9
            //
            // 2. SheathState (u8 enum) - Weapon display state
            //    Values: Unarmed=0 (sheathed), Melee=1 (drawn), Ranged=2
            //
            // 3. Unit Flags (u32 bitmask) - Gameplay state flags
            //    Key flags (from src/world/common/combat.rs):
            //    - DISABLE_MOVE      = 0x00000004  (rooted - what we need for logout)
            //    - STUNNED           = 0x00040000  (cannot act)
            //    - CONFUSED          = 0x00400000  (random movement)
            //    - IN_COMBAT         = 0x00080000  (in combat state)
            //    - SILENCED          = 0x00002000  (cannot cast spells)
            //    - PACIFIED          = 0x00020000  (cannot attack)
            //    - FLEEING           = 0x00800000  (fear effect)
            //    - PVP               = 0x00001000  (PvP flagged)
            //    - LOOTING           = 0x00000200  (looting corpse)
            //    - PLAYER_CONTROLLED = 0x00000008  (always set for players)
            //    Plus 20+ more flags for various states
            //
            // IMPLEMENTATION NEEDED FOR LOGOUT:
            // ---------------------------------
            // 1. Track unit_flags in a system (which system? see options below)
            // 2. Set DISABLE_MOVE flag when logout starts
            // 3. Set StandState::Sit for sit animation
            // 4. Clear both when logout is cancelled
            // 5. Check DISABLE_MOVE flag before processing movement packets
            //
            // ARCHITECTURE OPTIONS:
            // --------------------
            // Option A: Add to StatsSystem (easiest, but mixes concerns)
            //   - StatsSystem already tracks health/mana/stats
            //   - Could add: unit_flags, stand_state, sheath_state
            //   - Pro: Already exists, players already registered there
            //   - Con: Mixing stat calculation with state flags
            //
            // Option B: Create PlayerStateSystem (cleanest separation)
            //   - New system: DashMap<ObjectGuid, PlayerState>
            //   - PlayerState { unit_flags: u32, stand_state: u8, sheath_state: u8 }
            //   - Pro: Clean separation of concerns
            //   - Con: Another system to manage, another map to maintain
            //
            // Option C: Add to CombatSystem (unit flags are combat-related)
            //   - CombatSystem tracks combat state
            //   - Many unit flags are combat-related (STUNNED, IN_COMBAT, etc.)
            //   - Pro: Logical grouping with combat mechanics
            //   - Con: Stand state doesn't belong in combat system
            //
            // RECOMMENDED: Option B (PlayerStateSystem)
            // -----------------------------------------
            // Create: src/world/systems/player_state/mod.rs
            //
            // pub struct PlayerStateSystem {
            //     state: DashMap<ObjectGuid, PlayerState>,
            // }
            //
            // #[derive(Default)]
            // pub struct PlayerState {
            //     pub unit_flags: u32,      // Gameplay state flags (DISABLE_MOVE, STUNNED, etc.)
            //     pub stand_state: u8,      // Visual animation (Stand, Sit, etc.)
            //     pub sheath_state: u8,     // Weapon display (Unarmed, Melee, Ranged)
            // }
            //
            // impl PlayerStateSystem {
            //     pub fn set_unit_flag(&self, guid: ObjectGuid, flag: u32) { ... }
            //     pub fn remove_unit_flag(&self, guid: ObjectGuid, flag: u32) { ... }
            //     pub fn has_unit_flag(&self, guid: ObjectGuid, flag: u32) -> bool { ... }
            //     pub fn set_stand_state(&self, guid: ObjectGuid, state: u8) { ... }
            //     pub fn get_stand_state(&self, guid: ObjectGuid) -> u8 { ... }
            //     // ... etc
            // }
            //
            // PACKETS TO SEND:
            // ---------------
            // When setting sit state, also send SMSG_STANDSTATE_UPDATE:
            //   packet.write_u8(StandState::Sit as u8);
            //
            // MOVEMENT VALIDATION:
            // -------------------
            // In movement handler, before processing movement:
            //   if world.systems.player_state.has_unit_flag(guid, DISABLE_MOVE) {
            //       // Reject movement, player is rooted
            //       return Ok(());
            //   }
            //
            // FILES TO REFERENCE FROM OLD /WORLD:
            // -----------------------------------
            // - src/world/game/objects/player/player.rs:1420-1428 (set_rooted implementation)
            // - src/world/game/objects/player/player.rs:492-512 (stand_state methods)
            // - src/world/common/combat.rs:17-51 (unit_flags constants)
            // - src/world/game/objects/unit/misc.rs:92-147 (StandState enum)
            // - src/world/handlers/character_handlers.rs:2799-2810 (logout sit + root)
            // - src/world/handlers/character_handlers.rs:3120-3130 (logout cancel unroot + stand)
            //
            debug!("Player {} rooted during logout countdown", player_guid);
        }
    }

    Ok(())
}

/// Handle CMSG_LOGOUT_CANCEL - player cancels logout
pub async fn handle_logout_cancel(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Get player GUID
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("Logout cancel but no player in session {}", session.id());
            return Ok(());
        }
    };

    session.cancel_logout_timer();

    // Send ACK
    session.send_msg(SmsgLogoutCancelAck)?;

    // Unroot player to allow movement again
    let world_guid = WorldObjectGuid::from_low(player_guid.counter());
    let unroot_msg = SmsgForceMoveUnroot { guid: world_guid };
    session.send_msg(unroot_msg)?;

    // Clear server-side state: stand and unroot
    const STAND_STATE_STAND: u8 = 0;
    const UNIT_FLAG_DISABLE_MOVE: u32 = 0x00000004;

    world.managers.player_mgr.with_player_mut(player_guid, |player| {
        player.stand_state = STAND_STATE_STAND;
        player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;
    });

    // Send SMSG_STANDSTATE_UPDATE to show stand animation
    session.send_msg(SmsgStandstateUpdate {
        stand_state: STAND_STATE_STAND,
    })?;

    debug!("Player {} unrooted and standing after logout cancel", player_guid);

    Ok(())
}

/// Perform actual logout cleanup
/// Called either immediately (instant) or after timer expires
pub async fn perform_logout_cleanup(session: &WorldSession, world: &World) -> Result<()> {
    let session_id = session.id();

    info!(
        "[LOGOUT_TIMER] perform_logout_cleanup called for session {}",
        session_id
    );

    // Get player GUID before cleanup
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!(
                "[LOGOUT_TIMER] Logout cleanup called but no player GUID in session {}",
                session_id
            );
            return Ok(());
        }
    };

    info!(
        "[LOGOUT_TIMER] Starting logout cleanup for player {} (session {})",
        player_guid,
        session_id
    );

    // Clear logout state flags defensively (handles disconnect during logout)
    const STAND_STATE_STAND: u8 = 0;
    const UNIT_FLAG_DISABLE_MOVE: u32 = 0x00000004;

    world.managers.player_mgr.with_player_mut(player_guid, |player| {
        player.stand_state = STAND_STATE_STAND;
        player.unit_flags &= !UNIT_FLAG_DISABLE_MOVE;
    });

    // Remove player handler and movement buffer FIRST to stop processing new packets
    world.remove_player_handler(player_guid);
    world.remove_movement_buffer(player_guid);
    tracing::info!("[LOGOUT] Removed packet handler and movement buffer for player {}", player_guid);

    info!(
        "Performing logout cleanup for player {} (session {})",
        player_guid, session_id
    );

    // Send SMSG_LOGOUT_COMPLETE
    session.send_msg(SmsgLogoutComplete)?;

    // Save all player data to database (BEFORE removing from systems)
    // This saves: position, experience, health/power, rest state, spells, action bars, reputation, skills
    // This must happen before on_player_logout removes the player from PlayerManager
    world
        .managers
        .player_mgr
        .save_all_player_data(player_guid, &world.databases.character)
        .await?;
    info!("[LOGOUT] Saved all player data (position, XP, health, rest, spells, actions, reputation, skills) for {:?}", player_guid);

    // Save quest progress to database
    world
        .systems
        .quest
        .save_player_quests(player_guid)
        .await?;
    info!("[LOGOUT] Saved quest progress for {:?}", player_guid);

    // Notify systems of logout (BEFORE removing from PlayerManager)
    world.systems.on_player_logout(player_guid, world).await?;

    // Remove player from map (BEFORE removing from PlayerManager)
    if let Some(player) = world.managers.player_mgr.get_player(player_guid) {
        let map = world.managers.map_mgr.get_or_create_map(player.map_id, player.instance_id);
        map.remove_player(player_guid);
        info!(
            "[LOGOUT] Removed player {:?} from map {}",
            player_guid, player.map_id
        );
    }

    // Logout player (saves to DB, removes from manager)
    let account_id = session.account_id();
    world
        .managers
        .player_mgr
        .logout_player(player_guid, account_id, &world.databases.character, world)
        .await?;

    // Unregister player from session manager
    world.session_mgr.unregister_player(player_guid);
    session.set_player_guid(None);

    // Clear logout timer to prevent repeated cleanup attempts
    session.cancel_logout_timer();

    // Set session state back to Authenticated (character select)
    session.set_state(SessionState::Authenticated);

    info!("Logout cleanup complete for player {}", player_guid);

    Ok(())
}

/// Handle CMSG_CHAR_CREATE - create new character
pub async fn handle_char_create(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    use crate::shared::database::world::repositories::PlayerCreateInfoRepository;
    use crate::shared::messages::character::SmsgCharCreate;
    use crate::world::game::common::account_result::{char_create, char_name};
    use crate::world::config::get_config_mgr;
    use crate::world::game::player::name_validation::{
        normalize_character_name, validate_character_name, NameValidationResult,
    };
    use crate::shared::game::chat::Team;

    // 1. Parse packet
    let name = packet
        .read_cstring()
        .ok_or_else(|| anyhow!("Failed to read character name"))?;
    let race = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read race"))?;
    let class = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read class"))?;
    let gender = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read gender"))?;
    let skin = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read skin"))?;
    let face = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read face"))?;
    let hair_style = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read hair_style"))?;
    let hair_color = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read hair_color"))?;
    let facial_hair = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read facial_hair"))?;
    let _outfit_id = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read outfit_id"))?;

    debug!(
        "Character creation: name={}, race={}, class={}, account={}",
        name,
        race,
        class,
        session.account_id()
    );

    // 2. Normalize and validate name
    let normalized_name = normalize_character_name(&name);
    let validation = validate_character_name(&normalized_name);

    if validation != NameValidationResult::Success {
        let result_code = match validation {
            NameValidationResult::TooShort => char_name::TOO_SHORT,
            NameValidationResult::TooLong => char_name::TOO_LONG,
            NameValidationResult::Profane => char_name::PROFANE,
            NameValidationResult::Reserved => char_name::RESERVED,
            NameValidationResult::InvalidCharacters => char_name::ONLY_LETTERS,
            NameValidationResult::MixedLanguages => char_name::MIXED_LANGUAGES,
            _ => char_name::FAILURE,
        };
        session
            .send_msg(SmsgCharCreate {
                result: result_code,
            })?;
        return Ok(());
    }

    // 3. Check name availability
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    if char_repo.exists_by_name(&normalized_name).await? {
        session
            .send_msg(SmsgCharCreate {
                result: char_create::NAME_IN_USE,
            })?;
        return Ok(());
    }

    // 4. Get config values before async operations
    let (characters_per_realm, characters_creating_disabled, is_pvp_realm, allow_two_side_accounts) = {
        let config = get_config_mgr().get();
        (
            config.characters_per_realm,
            config.characters_creating_disabled,
            config.is_pvp_realm,
            config.allow_two_side_accounts,
        )
    };

    // 5. Check character limit
    let existing_chars = char_repo.find_by_account(session.account_id()).await?;
    if existing_chars.len() >= characters_per_realm as usize {
        session
            .send_msg(SmsgCharCreate {
                result: char_create::SERVER_LIMIT,
            })?;
        return Ok(());
    }

    // 6. Check faction restrictions
    let team = Team::from_race(race);

    // Check if creation is disabled for this faction
    let disabled = match team {
        Team::Alliance => (characters_creating_disabled & (1 << 0)) != 0,
        Team::Horde => (characters_creating_disabled & (1 << 1)) != 0,
        _ => false,
    };

    if disabled {
        session
            .send_msg(SmsgCharCreate {
                result: char_create::DISABLED,
            })?;
        return Ok(());
    }

    // 7. Check PvP realm cross-faction restriction
    if is_pvp_realm && !allow_two_side_accounts && !existing_chars.is_empty() {
        let first_char_team = Team::from_race(existing_chars[0].race);
        if team != first_char_team && team != Team::None && first_char_team != Team::None {
            session
                .send_msg(SmsgCharCreate {
                    result: char_create::PVP_TEAMS_VIOLATION,
                })?;
            return Ok(());
        }
    }

    // 8. Get starting position
    let create_repo = PlayerCreateInfoRepository::new(Arc::new(databases.world.clone()));
    let create_info = create_repo.get_create_info(race, class).await?;

    let (map_id, zone_id, position_x, position_y, position_z, orientation) = match create_info {
        Some(info) => {
            debug!(
                "Using starting position: race={}, class={}, map={}, zone={}",
                race, class, info.map, info.zone
            );
            (
                info.map,
                info.zone,
                info.position_x,
                info.position_y,
                info.position_z,
                info.orientation,
            )
        }
        None => {
            warn!(
                "No starting position for race={}, class={}, using default",
                race, class
            );
            // Default: Elwynn Forest (Human starting zone)
            (0, 12, -8949.95, -132.493, 83.5312, 0.0)
        }
    };

    // 9. Generate GUID using proper generator
    let player_guid = world.managers.player_mgr.generate_player_guid();
    let guid = player_guid.counter();

    // 10. Get base health and mana from player_classlevelstats (class/level 1)
    let class_stats: Option<(u16, u16)> = sqlx::query_as::<_, (u16, u16)>(
        "SELECT basehp, basemana FROM player_classlevelstats WHERE class = ? AND level = 1",
    )
    .bind(class)
    .fetch_optional(&databases.world)
    .await?;
    let (base_hp, base_mana) = class_stats.unwrap_or((20, 0));

    // 11. Get stamina and intellect from player_levelstats (race/class/level 1)
    let level_stats: Option<(u8, u8)> = sqlx::query_as::<_, (u8, u8)>(
        "SELECT sta, inte FROM player_levelstats WHERE race = ? AND class = ? AND level = 1",
    )
    .bind(race)
    .bind(class)
    .fetch_optional(&databases.world)
    .await?;
    let (stamina, intellect) = level_stats.unwrap_or((20, 20));

    // 12. Calculate health bonus from stamina (MaNGOS formula)
    // First 20 stamina: 1 HP each, above 20: 10 HP each
    let health_bonus = {
        let base_stam = (stamina as u32).min(20);
        let more_stam = (stamina as u32).saturating_sub(20);
        base_stam + (more_stam * 10)
    };

    // 13. Calculate mana bonus from intellect (MaNGOS formula)
    // First 20 intellect: 1 mana each, above 20: 15 mana each
    let mana_bonus = {
        let base_int = (intellect as u32).min(20);
        let more_int = (intellect as u32).saturating_sub(20);
        base_int + (more_int * 15)
    };

    let max_health = (base_hp as u32) + health_bonus;
    let max_mana = (base_mana as u32) + mana_bonus;

    // Get starting money from config
    let starting_money = world.config.start_player_money;

    // 14. Create character using simplified method
    char_repo
        .create_simple(
            guid,
            session.account_id(),
            &normalized_name,
            race,
            class,
            gender,
            skin,
            face,
            hair_style,
            hair_color,
            facial_hair,
            map_id,
            zone_id,
            position_x,
            position_y,
            position_z,
            orientation,
            max_health,
            max_mana,
            starting_money,
        )
        .await?;

    debug!(
        "Character created: {} (guid={}, health={}, mana={}, money={})",
        normalized_name, guid, max_health, max_mana, starting_money
    );

    // Create starting items
    let starting_items = create_repo.get_create_info_items(race, class).await?;
    if !starting_items.is_empty() {
        match super::character_create_items::give_starting_items(
            &databases.character,
            &world.managers.item_mgr,
            guid,
            &starting_items,
        ).await

        {
            Ok(created_count) => {
                debug!(
                    "Created {} starting items for character {} (expected {})",
                    created_count,
                    guid,
                    starting_items.len()
                );
                if created_count < starting_items.len() {
                    warn!(
                        "Only created {}/{} starting items for character {} - some items may have been skipped due to missing templates or insufficient inventory space",
                        created_count,
                        starting_items.len(),
                        guid
                    );
                }
            }
            Err(e) => {
                error!(
                    "Failed to create starting items for character {}: {}. Character was created but may be missing starting equipment.",
                    guid, e
                );
                // Continue - character is already created, don't fail the whole request
            }
        }
    }

    // Create starting action buttons
    info!("[CHAR_CREATE] Loading default action buttons for race={} class={}", race, class);
    let starting_actions = match create_repo.get_create_info_actions(race, class).await {
        Ok(actions) => {
            info!("[CHAR_CREATE] Found {} default action buttons for race {} class {}", actions.len(), race, class);
            actions
        }
        Err(e) => {
            warn!("[CHAR_CREATE] Failed to load default action buttons: {}", e);
            Vec::new()
        }
    };
    if !starting_actions.is_empty() {
        info!("[CHAR_CREATE] Inserting {} action buttons for character {}", starting_actions.len(), guid);
        match super::character_create_items::give_starting_actions(
            &databases.character,
            guid,
            &starting_actions,
        ).await

        {
            Ok(_) => {
                info!("[CHAR_CREATE] Successfully created {} action buttons for character {}", starting_actions.len(), guid);
            }
            Err(e) => {
                warn!(
                    "[CHAR_CREATE] Failed to create starting actions for character {}: {}",
                    guid, e
                );
            }
        }
    } else {
        info!("[CHAR_CREATE] No default action buttons found for race {} class {}", race, class);
    }

    session
        .send_msg(SmsgCharCreate {
            result: char_create::SUCCESS,
        })?;

    Ok(())
}

/// Handle CMSG_CHAR_DELETE - delete character
pub async fn handle_char_delete(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    _world: &World,
) -> Result<()> {
    use crate::shared::messages::character::SmsgCharDelete;
    use crate::world::game::common::account_result::char_delete;

    // 1. Parse GUID
    let guid_raw = packet
        .read_u64()
        .ok_or_else(|| anyhow!("Failed to read GUID"))?;
    let guid = guid_raw as u32;

    debug!(
        "Character delete request for guid={} from account {}",
        guid,
        session.account_id()
    );

    // 2. Load and verify character
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    let character = char_repo.find_by_guid(guid).await?;

    match character {
        None => {
            warn!("Character {} not found", guid);
            session
                .send_msg(SmsgCharDelete {
                    result: char_delete::FAILED,
                })?;
            return Ok(());
        }
        Some(char_data) if char_data.account != session.account_id() => {
            warn!(
                "Character {} does not belong to account {}",
                guid,
                session.account_id()
            );
            session
                .send_msg(SmsgCharDelete {
                    result: char_delete::FAILED,
                })?;
            return Ok(());
        }
        Some(_) => {}
    }

    // 3. Delete character (repository handles transaction and cascades)
    char_repo.delete(guid).await?;

    debug!("Character {} deleted successfully", guid);

    session
        .send_msg(SmsgCharDelete {
            result: char_delete::SUCCESS,
        })?;

    Ok(())
}

/// Handle CMSG_CHAR_RENAME - rename character
pub async fn handle_char_rename(
    session: &WorldSession,
    packet: &mut WorldPacket,
    databases: &Databases,
    world: &World,
) -> Result<()> {
    use crate::shared::messages::character::SmsgCharRename;
    use crate::world::game::common::account_result::{char_name, response};
    use crate::world::game::player::name_validation::{
        normalize_character_name, validate_character_name, NameValidationResult,
    };

    // Character flags
    const CHARACTER_FLAG_RENAME: u32 = 0x00004000;
    const CHARACTER_FLAG_RENAME_NEEDS_GM_REVIEW: u32 = 0x00008000;

    // 1. Parse packet
    let guid_u64 = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read GUID"))?;
    let guid = ObjectGuid::from(guid_u64);
    let new_name = packet
        .read_cstring()
        .ok_or_else(|| anyhow!("Failed to read new name"))?;

    debug!(
        "Character rename: guid={}, new_name={}, account={}",
        guid,
        new_name,
        session.account_id()
    );

    // 2. Normalize and validate name
    let normalized_name = normalize_character_name(&new_name);
    let validation = validate_character_name(&normalized_name);

    if validation != NameValidationResult::Success {
        let result_code = match validation {
            NameValidationResult::TooShort => char_name::TOO_SHORT,
            NameValidationResult::TooLong => char_name::TOO_LONG,
            NameValidationResult::Profane => char_name::PROFANE,
            NameValidationResult::Reserved => char_name::RESERVED,
            NameValidationResult::InvalidCharacters => char_name::ONLY_LETTERS,
            NameValidationResult::MixedLanguages => char_name::MIXED_LANGUAGES,
            _ => char_name::FAILURE,
        };
        session
            .send_msg(SmsgCharRename {
                result: result_code,
                guid: None,
                new_name: None,
            })?;
        return Ok(());
    }

    // 3. Check name availability
    let char_repo = CharacterRepository::new(Arc::new(databases.character.clone()));
    if char_repo.exists_by_name(&normalized_name).await? {
        session
            .send_msg(SmsgCharRename {
                result: char_name::FAILURE,
                guid: None,
                new_name: None,
            })?;
        return Ok(());
    }

    // 4. Load and verify character
    let character = char_repo.find_by_guid(guid.counter()).await?;

    let char_data = match character {
        None => {
            warn!("Character {} not found", guid);
            session
                .send_msg(SmsgCharRename {
                    result: response::FAILURE,
                    guid: None,
                    new_name: None,
                })?;
            return Ok(());
        }
        Some(char_data) if char_data.account != session.account_id() => {
            warn!(
                "Character {} does not belong to account {}",
                guid,
                session.account_id()
            );
            session
                .send_msg(SmsgCharRename {
                    result: response::FAILURE,
                    guid: None,
                    new_name: None,
                })?;
            return Ok(());
        }
        Some(char_data) => char_data,
    };

    // 5. Check rename flag
    if char_data.character_flags & CHARACTER_FLAG_RENAME == 0 {
        warn!("Character {} does not have RENAME flag", guid);
        session
            .send_msg(SmsgCharRename {
                result: response::FAILURE,
                guid: None,
                new_name: None,
            })?;
        return Ok(());
    }

    // 6. Check if character is online
    let is_online = world.managers.player_mgr.get_player(guid).is_some();
    if is_online {
        warn!("Cannot rename character {} while logged in", guid);
        session
            .send_msg(SmsgCharRename {
                result: response::FAILURE,
                guid: None,
                new_name: None,
            })?;
        return Ok(());
    }

    // 7. Update name and flags
    let new_flags = (char_data.character_flags & !CHARACTER_FLAG_RENAME)
        | CHARACTER_FLAG_RENAME_NEEDS_GM_REVIEW;
    char_repo
        .update_name_and_flags(guid.counter(), &normalized_name, new_flags)
        .await?;

    debug!("Character {} renamed to {}", guid, normalized_name);

    session
        .send_msg(SmsgCharRename {
            result: response::SUCCESS,
            guid: Some(guid.into()),
            new_name: Some(normalized_name),
        })?;

    Ok(())
}

/// Handle CMSG_ZONEUPDATE - client notifies server of zone change
pub async fn handle_zoneupdate(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Read zone ID from packet
    let zone_id = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read zone_id from CMSG_ZONEUPDATE"))?;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow!("Not logged in"))?;

    debug!("CMSG_ZONEUPDATE: player={:?}, zone_id={}", player_guid, zone_id);

    // Update zone and read current rest state (single lock scope)
    let (changed, current_rest_type) = {
        match world.managers.player_mgr.get_player_mut(player_guid) {
            Some(mut player) => {
                let old_zone = player.zone_id;
                if old_zone == zone_id {
                    debug!("CMSG_ZONEUPDATE: Zone unchanged (zone_id={})", zone_id);
                    return Ok(());
                }
                player.zone_id = zone_id;
                debug!("CMSG_ZONEUPDATE: Updated zone from {} to {}", old_zone, zone_id);
                (true, player.environment.rest_type)
            }
            None => return Ok(()),
        }
    };
    // Player lock released here

    if !changed {
        return Ok(());
    }

    // --- Update rest state based on zone flags (MaNGOS Player.cpp:6737-6741) ---
    // AREA_FLAG_CAPITAL (0x100): capital cities grant rest XP
    const AREA_FLAG_CAPITAL: u32 = 0x100;

    let is_capital = {
        let dbc = world.dbc.read();
        if let Some(area) = dbc.get_area(zone_id) {
            info!("ZONEUPDATE: area_id={}, flags=0x{:X}, parent_zone={}", zone_id, area.flags, area.zone);
            if area.flags & AREA_FLAG_CAPITAL != 0 {
                info!("ZONEUPDATE: is_capital=true (direct)");
                true
            } else if area.zone != 0 {
                let parent_capital = dbc.get_area(area.zone)
                    .map(|parent| {
                        info!("ZONEUPDATE: parent area_id={}, parent_flags=0x{:X}", area.zone, parent.flags);
                        parent.flags & AREA_FLAG_CAPITAL != 0
                    })
                    .unwrap_or(false);
                info!("ZONEUPDATE: is_capital={} (via parent)", parent_capital);
                parent_capital
            } else {
                info!("ZONEUPDATE: is_capital=false (top-level, no capital flag)");
                false
            }
        } else {
            info!("ZONEUPDATE: area_id={} NOT FOUND in DBC!", zone_id);
            false
        }
    };

    let player_mgr = &world.managers.player_mgr;

    if is_capital {
        // Entered a capital city — set city rest
        world.systems.environment.set_rest_type(
            player_guid,
            crate::world::game::player::environment::RestType::InCity,
            0,
            player_mgr,
        )?;
    } else if current_rest_type == crate::world::game::player::environment::RestType::InCity {
        // Left a capital city and was resting in city — clear rest
        world.systems.environment.set_rest_type(
            player_guid,
            crate::world::game::player::environment::RestType::No,
            0,
            player_mgr,
        )?;
    }

    // Send updated PLAYER_FLAGS to client
    if is_capital || current_rest_type == crate::world::game::player::environment::RestType::InCity {
        if let Some(new_flags) = player_mgr.with_player(player_guid, |p| p.player_flags) {
            use crate::shared::messages::update::{
                ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
            };
            use crate::shared::messages::ToWorldPacket;
            use crate::world::game::common::update_fields::PLAYER_FLAGS;

            let world_guid =
                crate::world::core::common::guid::ObjectGuid::from_low(player_guid.counter());
            let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
                ValuesUpdateBlock::new(world_guid, ObjectType::Player)
                    .set_field(PLAYER_FLAGS, new_flags),
            ));
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(player_guid, update);
        }
    }

    // TODO: Handle other zone-specific effects:
    // - Check for exploration quest objectives
    // - Update PvP flags based on zone type
    // - Trigger zone-specific scripts
    // - Send zone-specific packets (weather, music)

    Ok(())
}
