//! Lua action types.
//!
//! Actions are returned by Lua scripts and executed by the server.
//! This provides a safe, pure-function interface between scripts and game state.

use super::snapshot::LuaGuid;
use super::super::common::ObjectGuid;

/// Target for spell casting.
#[derive(Debug, Clone)]
pub enum SpellTarget {
    /// Cast on self.
    Self_,
    /// Cast on current attack target.
    CurrentTarget,
    /// Random hostile from threat list.
    RandomHostile,
    /// Random hostile in spell range.
    RandomHostileInRange,
    /// Lowest health friendly unit.
    LowestHealthFriendly,
    /// Specific target by GUID.
    Guid(ObjectGuid),
}

/// Summon type for spawned creatures.
#[derive(Debug, Clone, Copy, Default)]
pub enum SummonType {
    /// Despawn after duration.
    TimedDespawn,
    /// Despawn after duration if out of combat.
    TimedDespawnOutOfCombat,
    /// Despawn when corpse disappears.
    #[default]
    CorpseDespawn,
    /// Leave corpse, despawn after duration.
    CorpseTimedDespawn,
    /// Despawn immediately on death.
    DeadDespawn,
    /// Only despawn via script.
    ManualDespawn,
}

/// React state for creatures.
#[derive(Debug, Clone, Copy)]
pub enum ReactState {
    Passive,
    Defensive,
    Aggressive,
}

/// Actions that can be returned by Lua scripts.
///
/// Scripts return tables of actions which are parsed into this enum.
/// The server then executes these actions in order.
#[derive(Debug, Clone)]
pub enum LuaAction {
    // ==================== Movement ====================
    /// Move to specific coordinates.
    MoveTo { x: f32, y: f32, z: f32, run: bool },

    /// Move toward a target.
    MoveToTarget {
        target: ObjectGuid,
        min_distance: f32,
    },

    /// Return to spawn position.
    ReturnToSpawn,

    /// Stop all movement.
    StopMovement,

    /// Flee from a target.
    FleeFrom {
        target: ObjectGuid,
        distance: f32,
        duration_ms: u32,
    },

    /// Turn to face a target.
    FaceTarget { target: ObjectGuid },

    /// Enable random idle movement.
    RandomMovement { radius: f32 },

    // ==================== Combat ====================
    /// Cast a spell.
    CastSpell {
        spell_id: u32,
        target: SpellTarget,
        triggered: bool,
    },

    /// Perform melee attack.
    MeleeAttack { target: ObjectGuid },

    /// Change attack target.
    SetAttackTarget { target: Option<ObjectGuid> },

    /// Force enter combat.
    EnterCombat { target: ObjectGuid },

    /// Reset and return to spawn.
    EnterEvadeMode,

    /// Stop combat without returning home (CombatStop + clear threat list).
    LeaveCombat,

    /// Interrupt current spell cast.
    InterruptSpell,

    // ==================== Threat ====================
    /// Add threat to a target.
    AddThreat { target: ObjectGuid, amount: f32 },

    /// Modify threat by percentage.
    ModifyThreatPercent { target: ObjectGuid, percent: f32 },

    /// Clear all threat.
    ClearThreatList,

    /// Reset all threat to equal values.
    ResetThreat,

    // ==================== State ====================
    /// Set or reset a timer.
    SetTimer { timer_id: u32, duration_ms: u32 },

    /// Set encounter phase.
    SetPhase { phase: u32 },

    /// Store custom data.
    SetCustomData { key: String, value: i64 },

    // ==================== Instance ====================
    /// Set instance encounter data.
    SetInstanceData { data_id: u32, value: u32 },

    /// Store a GUID in instance data.
    SetInstanceGuid { data_id: u32, guid: ObjectGuid },

    /// Open a door (by GUID).
    OpenDoor { guid: ObjectGuid },

    /// Open a door (by data ID storing the GUID).
    OpenDoorByData { data_id: u32 },

    /// Close a door (by GUID).
    CloseDoor { guid: ObjectGuid },

    /// Close a door (by data ID storing the GUID).
    CloseDoorByData { data_id: u32 },

    // ==================== Summoning ====================
    /// Spawn a creature.
    SpawnCreature {
        entry: u32,
        x: f32,
        y: f32,
        z: f32,
        o: f32,
        summon_type: SummonType,
        duration_ms: u32,
    },

    /// Despawn a creature by GUID.
    DespawnCreature { guid: ObjectGuid },

    /// Despawn all creatures with entry.
    DespawnCreaturesByEntry { entry: u32 },

    /// Spawn a game object.
    SpawnGameObject {
        entry: u32,
        x: f32,
        y: f32,
        z: f32,
        o: f32,
        duration_secs: u32,
    },

    /// Respawn a despawned game object.
    RespawnGameObject {
        guid: ObjectGuid,
        duration_secs: u32,
    },

    // ==================== Chat ====================
    /// Creature says text (nearby players).
    Say { text: String },

    /// Creature yells text (zone-wide).
    Yell { text: String },

    /// Creature performs emote.
    Emote { emote_id: u32 },

    /// Creature performs text emote.
    TextEmote { text: String },

    /// Play a sound.
    PlaySound { sound_id: u32, zone_wide: bool },

    // ==================== Zone ====================
    /// Show text to all players in zone.
    ZoneText { text: String },

    /// Yell to all players in zone.
    ZoneYell { text: String },

    /// Update world state.
    SendWorldState { state_id: u32, value: i32 },

    /// Play sound to all in zone.
    PlaySoundToZone { sound_id: u32 },

    // ==================== Player (Gossip) ====================
    /// Teleport a player.
    TeleportPlayer {
        player: ObjectGuid,
        map_id: u32,
        x: f32,
        y: f32,
        z: f32,
        o: f32,
    },

    /// Give item to player.
    GiveItem {
        player: ObjectGuid,
        item_id: u32,
        count: u32,
    },

    /// Remove item from player.
    TakeItem {
        player: ObjectGuid,
        item_id: u32,
        count: u32,
    },

    /// Give gold to player (in copper).
    GiveGold { player: ObjectGuid, amount: u32 },

    /// Take gold from player (in copper).
    TakeGold { player: ObjectGuid, amount: u32 },

    /// Add reputation.
    AddReputation {
        player: ObjectGuid,
        faction_id: u32,
        amount: i32,
    },

    /// Complete quest for player.
    CompleteQuest { player: ObjectGuid, quest_id: u32 },

    // ==================== Gossip ====================
    /// Set gossip menu text.
    GossipMenu { npc_text_id: u32 },

    /// Add gossip option.
    GossipOption {
        id: u32,
        icon: u8,
        text: String,
        coded: bool,
    },

    /// Add quest to gossip menu.
    GossipQuest { quest_id: u32 },

    /// Send gossip menu to player.
    GossipSend,

    /// Close gossip window.
    GossipClose,

    /// Open vendor window.
    SendVendor,

    /// Open trainer window.
    SendTrainer,

    /// Open bank.
    SendBanker,

    /// Open auction house.
    SendAuctioneer,

    /// Bind at inn.
    SendInnkeeper,

    /// Open flight master.
    SendTaxi,

    // ==================== Creature State ====================
    /// Change creature faction.
    SetFaction { faction_id: u32 },

    /// Set react state.
    SetReactState { state: ReactState },

    /// Enable/disable combat movement.
    SetCombatMovement { enabled: bool },

    /// Enable/disable melee attacks.
    SetMeleeAttack { enabled: bool },

    /// Set immunity flags.
    SetImmune { physical: bool, spell: bool },

    /// Root in place.
    SetRoot { rooted: bool },

    /// Set health percentage.
    SetHealthPercent { percent: f32 },

    /// Change display model.
    Morph { display_id: u32 },

    /// Reset display model.
    Demorph,

    /// Kill the creature.
    KillSelf,

    // ==================== Phase 5: New Actions ====================
    /// Remove an aura from the creature.
    RemoveAura { spell_id: u32 },

    /// Set a unit flag.
    SetUnitFlag { flag: u32 },

    /// Remove a unit flag.
    RemoveUnitFlag { flag: u32 },

    /// Set combat with entire zone (all players become threat targets).
    SetCombatWithZone,

    /// Script text (lookup from DB by text ID).
    ScriptText { text_id: i32 },

    /// Set the creature's stand/animation state (0=stand,1=sit,2=sitchair,3=sleep,4=kneel,7=dead).
    SetStandState { state: u8 },

    /// Set the creature's dynamic flags (e.g. UNIT_DYNFLAG_DEAD = 4).
    SetDynFlag { flag: u32 },

    /// Clear the creature's dynamic flags.
    RemoveDynFlag { flag: u32 },

    // ==================== Phase 6: Movement ====================
    /// Move along a path of waypoints.
    MoveAlongPath {
        waypoints: Vec<(f32, f32, f32)>,
        run: bool,
        repeating: bool,
    },

    // ==================== Quest ====================
    /// Award quest kill credit to the triggering player for the nearest
    /// creature with the given entry within search_radius yards.
    KillCreditNearestCreature {
        creature_entry: u32,
        search_radius: f32,
    },

    // ==================== Player-relative spawning ====================
    /// Spawn a creature at the triggering player's current position.
    /// Equivalent to pPlayer->SummonCreature(entry, x, y, z, ...) in MaNGOS.
    SpawnCreatureAtPlayer {
        entry: u32,
        summon_type: SummonType,
        duration_ms: u32,
    },

    // ==================== Nearest-creature spell ====================
    /// Find the nearest alive creature with the given entry within
    /// search_radius yards of the NPC/GO and cast a spell on it.
    /// Equivalent to finding a creature and calling CastSpell(target, spell_id).
    CastSpellOnNearestCreature {
        creature_entry: u32,
        spell_id: u32,
        search_radius: f32,
    },
}

impl LuaAction {
    /// Parse a Lua action table into a LuaAction.
    ///
    /// Returns None if the action type is unknown or required fields are missing.
    pub fn from_lua_table(table: &mlua::Table) -> Option<Self> {
        let action_type: String = table.get("action").ok()?;

        match action_type.as_str() {
            // Movement
            "MOVE_TO" => Some(LuaAction::MoveTo {
                x: table.get("x").ok()?,
                y: table.get("y").ok()?,
                z: table.get("z").ok()?,
                run: table.get("run").unwrap_or(true),
            }),
            "MOVE_TO_TARGET" => Some(LuaAction::MoveToTarget {
                target: parse_guid(table, "target")?,
                min_distance: table.get("min_distance").unwrap_or(0.0),
            }),
            "RETURN_TO_SPAWN" => Some(LuaAction::ReturnToSpawn),
            "STOP_MOVEMENT" => Some(LuaAction::StopMovement),
            "FLEE_FROM" => Some(LuaAction::FleeFrom {
                target: parse_guid(table, "target")?,
                distance: table.get("distance").unwrap_or(20.0),
                duration_ms: table.get("duration").unwrap_or(5000),
            }),
            "FACE_TARGET" => Some(LuaAction::FaceTarget {
                target: parse_guid(table, "target")?,
            }),
            "RANDOM_MOVEMENT" => Some(LuaAction::RandomMovement {
                radius: table.get("radius").unwrap_or(10.0),
            }),

            // Combat
            "CAST_SPELL" => Some(LuaAction::CastSpell {
                spell_id: table.get("spell_id").ok()?,
                target: parse_spell_target(table)?,
                triggered: table.get("triggered").unwrap_or(false),
            }),
            "MELEE_ATTACK" => Some(LuaAction::MeleeAttack {
                target: parse_guid(table, "target")?,
            }),
            "SET_ATTACK_TARGET" => Some(LuaAction::SetAttackTarget {
                target: parse_guid_optional(table, "target"),
            }),
            "ENTER_COMBAT" => Some(LuaAction::EnterCombat {
                target: parse_guid(table, "target")?,
            }),
            "ENTER_EVADE_MODE" => Some(LuaAction::EnterEvadeMode),
            "LEAVE_COMBAT" => Some(LuaAction::LeaveCombat),
            "INTERRUPT_SPELL" => Some(LuaAction::InterruptSpell),

            // Threat
            "ADD_THREAT" => Some(LuaAction::AddThreat {
                target: parse_guid(table, "target")?,
                amount: table.get("amount").ok()?,
            }),
            "MODIFY_THREAT" => Some(LuaAction::ModifyThreatPercent {
                target: parse_guid(table, "target")?,
                percent: table.get("percent").ok()?,
            }),
            "CLEAR_THREAT_LIST" => Some(LuaAction::ClearThreatList),
            "RESET_THREAT" => Some(LuaAction::ResetThreat),

            // State
            "SET_TIMER" => Some(LuaAction::SetTimer {
                timer_id: table.get("timer_id").ok()?,
                duration_ms: table.get("duration").ok()?,
            }),
            "SET_PHASE" => Some(LuaAction::SetPhase {
                phase: table.get("phase").ok()?,
            }),
            "SET_CUSTOM_DATA" => Some(LuaAction::SetCustomData {
                key: table.get("key").ok()?,
                value: table.get("value").ok()?,
            }),

            // Instance
            "SET_DATA" => Some(LuaAction::SetInstanceData {
                data_id: table.get("data_id").ok()?,
                value: table.get("value").ok()?,
            }),
            "SET_GUID" => Some(LuaAction::SetInstanceGuid {
                data_id: table.get("data_id").ok()?,
                guid: parse_guid(table, "guid")?,
            }),
            "OPEN_DOOR" => {
                if let Some(guid) = parse_guid_optional(table, "guid") {
                    Some(LuaAction::OpenDoor { guid })
                } else if let Ok(data_id) = table.get::<u32>("data_id") {
                    Some(LuaAction::OpenDoorByData { data_id })
                } else {
                    None
                }
            }
            "CLOSE_DOOR" => {
                if let Some(guid) = parse_guid_optional(table, "guid") {
                    Some(LuaAction::CloseDoor { guid })
                } else if let Ok(data_id) = table.get::<u32>("data_id") {
                    Some(LuaAction::CloseDoorByData { data_id })
                } else {
                    None
                }
            }

            // Summoning
            "SPAWN_CREATURE" => Some(LuaAction::SpawnCreature {
                entry: table.get("entry").ok()?,
                x: table.get("x").ok()?,
                y: table.get("y").ok()?,
                z: table.get("z").ok()?,
                o: table.get("o").unwrap_or(0.0),
                summon_type: parse_summon_type(table),
                duration_ms: table.get("duration").unwrap_or(0),
            }),
            "DESPAWN_CREATURE" => {
                if let Some(guid) = parse_guid_optional(table, "guid") {
                    Some(LuaAction::DespawnCreature { guid })
                } else if let Ok(entry) = table.get::<u32>("entry") {
                    Some(LuaAction::DespawnCreaturesByEntry { entry })
                } else {
                    None
                }
            }
            "SPAWN_GAMEOBJECT" => Some(LuaAction::SpawnGameObject {
                entry: table.get("entry").ok()?,
                x: table.get("x").ok()?,
                y: table.get("y").ok()?,
                z: table.get("z").ok()?,
                o: table.get("o").unwrap_or(0.0),
                duration_secs: table.get("duration").unwrap_or(0),
            }),
            "RESPAWN_GAMEOBJECT" => Some(LuaAction::RespawnGameObject {
                guid: parse_guid(table, "guid")?,
                duration_secs: table.get("duration").unwrap_or(3600),
            }),

            // Chat
            "SAY" => Some(LuaAction::Say {
                text: table.get("text").ok()?,
            }),
            "YELL" => Some(LuaAction::Yell {
                text: table.get("text").ok()?,
            }),
            "EMOTE" => {
                if let Ok(emote_id) = table.get::<u32>("emote_id") {
                    Some(LuaAction::Emote { emote_id })
                } else if let Ok(text) = table.get::<String>("text") {
                    Some(LuaAction::TextEmote { text })
                } else {
                    None
                }
            }
            "PLAY_SOUND" => Some(LuaAction::PlaySound {
                sound_id: table.get("sound_id").ok()?,
                zone_wide: table.get("zone_wide").unwrap_or(false),
            }),

            // Zone
            "ZONE_TEXT" => Some(LuaAction::ZoneText {
                text: table.get("text").ok()?,
            }),
            "ZONE_YELL" => Some(LuaAction::ZoneYell {
                text: table.get("text").ok()?,
            }),
            "SEND_WORLD_STATE" => Some(LuaAction::SendWorldState {
                state_id: table.get("state_id").ok()?,
                value: table.get("value").ok()?,
            }),
            "PLAY_SOUND_TO_ZONE" => Some(LuaAction::PlaySoundToZone {
                sound_id: table.get("sound_id").ok()?,
            }),

            // Player (Gossip)
            "TELEPORT_PLAYER" => Some(LuaAction::TeleportPlayer {
                player: parse_guid(table, "player")?,
                map_id: table.get("map").ok()?,
                x: table.get("x").ok()?,
                y: table.get("y").ok()?,
                z: table.get("z").ok()?,
                o: table.get("o").unwrap_or(0.0),
            }),
            "GIVE_ITEM" => Some(LuaAction::GiveItem {
                player: parse_guid(table, "player")?,
                item_id: table.get("item_id").ok()?,
                count: table.get("count").unwrap_or(1),
            }),
            "TAKE_ITEM" => Some(LuaAction::TakeItem {
                player: parse_guid(table, "player")?,
                item_id: table.get("item_id").ok()?,
                count: table.get("count").unwrap_or(1),
            }),
            "GIVE_GOLD" => Some(LuaAction::GiveGold {
                player: parse_guid(table, "player")?,
                amount: table.get("amount").ok()?,
            }),
            "TAKE_GOLD" => Some(LuaAction::TakeGold {
                player: parse_guid(table, "player")?,
                amount: table.get("amount").ok()?,
            }),
            "ADD_REPUTATION" => Some(LuaAction::AddReputation {
                player: parse_guid(table, "player")?,
                faction_id: table.get("faction_id").ok()?,
                amount: table.get("amount").ok()?,
            }),
            "COMPLETE_QUEST" => Some(LuaAction::CompleteQuest {
                // `player` is optional — when omitted the executor substitutes the
                // triggering player (player_guid passed to execute_gossip_actions).
                player: parse_guid(table, "player")
                    .unwrap_or_else(crate::shared::protocol::ObjectGuid::empty),
                quest_id: table.get("quest_id").ok()?,
            }),

            // Gossip
            "GOSSIP_MENU" => Some(LuaAction::GossipMenu {
                // Accept both "npc_text" (legacy) and "npc_text_id" (preferred).
                npc_text_id: table.get("npc_text_id")
                    .or_else(|_| table.get("npc_text"))
                    .ok()?,
            }),
            "GOSSIP_OPTION" => Some(LuaAction::GossipOption {
                id: table.get("id").ok()?,
                icon: table.get("icon").unwrap_or(0),
                text: table.get("text").ok()?,
                coded: table.get("coded").unwrap_or(false),
            }),
            "GOSSIP_QUEST" => Some(LuaAction::GossipQuest {
                quest_id: table.get("quest_id").ok()?,
            }),
            "GOSSIP_SEND" => Some(LuaAction::GossipSend),
            "GOSSIP_CLOSE" => Some(LuaAction::GossipClose),
            "SEND_VENDOR" => Some(LuaAction::SendVendor),
            "SEND_TRAINER" => Some(LuaAction::SendTrainer),
            "SEND_BANKER" => Some(LuaAction::SendBanker),
            "SEND_AUCTIONEER" => Some(LuaAction::SendAuctioneer),
            "SEND_INNKEEPER" => Some(LuaAction::SendInnkeeper),
            "SEND_TAXI" => Some(LuaAction::SendTaxi),

            // Creature State
            "SET_FACTION" => Some(LuaAction::SetFaction {
                faction_id: table.get("faction_id").ok()?,
            }),
            "SET_REACT_STATE" => Some(LuaAction::SetReactState {
                state: parse_react_state(table)?,
            }),
            "SET_COMBAT_MOVEMENT" => Some(LuaAction::SetCombatMovement {
                enabled: table.get("enabled").ok()?,
            }),
            "SET_MELEE_ATTACK" => Some(LuaAction::SetMeleeAttack {
                enabled: table.get("enabled").ok()?,
            }),
            "SET_IMMUNE" => Some(LuaAction::SetImmune {
                physical: table.get("physical").unwrap_or(false),
                spell: table.get("spell").unwrap_or(false),
            }),
            "SET_ROOT" => Some(LuaAction::SetRoot {
                rooted: table.get("rooted").ok()?,
            }),
            "SET_HEALTH_PCT" => Some(LuaAction::SetHealthPercent {
                percent: table.get("percent").ok()?,
            }),
            "MORPH" => Some(LuaAction::Morph {
                display_id: table.get("display_id").ok()?,
            }),
            "DEMORPH" => Some(LuaAction::Demorph),
            "KILL_SELF" => Some(LuaAction::KillSelf),

            // Phase 5 actions
            "REMOVE_AURA" => Some(LuaAction::RemoveAura {
                spell_id: table.get("spell_id").ok()?,
            }),
            "SET_UNIT_FLAG" => Some(LuaAction::SetUnitFlag {
                flag: table.get("flag").ok()?,
            }),
            "REMOVE_UNIT_FLAG" => Some(LuaAction::RemoveUnitFlag {
                flag: table.get("flag").ok()?,
            }),
            "SET_COMBAT_WITH_ZONE" => Some(LuaAction::SetCombatWithZone),
            "SET_STAND_STATE" => {
                let state: u8 = match table.get::<String>("state").as_deref() {
                    Ok("STAND")   => 0,
                    Ok("SIT")     => 1,
                    Ok("SLEEP")   => 3,
                    Ok("KNEEL")   => 4,
                    Ok("DEAD")    => 7,
                    Ok("SUBMERGED") => 5,
                    _ => table.get::<u8>("state").unwrap_or(0),
                };
                Some(LuaAction::SetStandState { state })
            }
            "SET_DYNFLAG" => Some(LuaAction::SetDynFlag {
                flag: table.get("flag").ok()?,
            }),
            "REMOVE_DYNFLAG" => Some(LuaAction::RemoveDynFlag {
                flag: table.get("flag").ok()?,
            }),
            "SCRIPT_TEXT" => Some(LuaAction::ScriptText {
                text_id: table.get("text_id").ok()?,
            }),

            // Phase 6
            "MOVE_ALONG_PATH" => {
                let waypoints_table: mlua::Table = table.get("waypoints").ok()?;
                let mut waypoints = Vec::new();
                for pair in waypoints_table.pairs::<i32, mlua::Table>() {
                    if let Ok((_, wp)) = pair {
                        let x: f32 = wp.get("x").or_else(|_| wp.get(1)).ok()?;
                        let y: f32 = wp.get("y").or_else(|_| wp.get(2)).ok()?;
                        let z: f32 = wp.get("z").or_else(|_| wp.get(3)).ok()?;
                        waypoints.push((x, y, z));
                    }
                }
                Some(LuaAction::MoveAlongPath {
                    waypoints,
                    run: table.get("run").unwrap_or(true),
                    repeating: table.get("repeating").unwrap_or(false),
                })
            }

            // Quest
            "KILL_CREDIT_NEAREST_CREATURE" => Some(LuaAction::KillCreditNearestCreature {
                creature_entry: table.get("creature_entry").ok()?,
                search_radius: table.get("search_radius").unwrap_or(10.0),
            }),
            "SPAWN_CREATURE_AT_PLAYER" => Some(LuaAction::SpawnCreatureAtPlayer {
                entry: table.get("entry").ok()?,
                summon_type: parse_summon_type(table),
                duration_ms: table.get("duration_ms").unwrap_or(0),
            }),
            "CAST_SPELL_ON_NEAREST_CREATURE" => Some(LuaAction::CastSpellOnNearestCreature {
                creature_entry: table.get("creature_entry").ok()?,
                spell_id: table.get("spell_id").ok()?,
                search_radius: table.get("search_radius").unwrap_or(10.0),
            }),

            _ => {
                tracing::warn!("Unknown Lua action type: {}", action_type);
                None
            }
        }
    }
}

/// Parse a GUID from a Lua table field.
fn parse_guid(table: &mlua::Table, field: &str) -> Option<ObjectGuid> {
    // Try getting as LuaGuid userdata first
    if let Ok(guid) = table.get::<LuaGuid>(field) {
        return Some(guid.0);
    }
    // Try getting as raw u64/i64
    if let Ok(raw) = table.get::<u64>(field) {
        return Some(ObjectGuid::from_raw(raw));
    }
    if let Ok(raw) = table.get::<i64>(field) {
        return Some(ObjectGuid::from_raw(raw as u64));
    }
    None
}

/// Parse an optional GUID from a Lua table field.
fn parse_guid_optional(table: &mlua::Table, field: &str) -> Option<ObjectGuid> {
    parse_guid(table, field)
}

/// Parse spell target from Lua table.
fn parse_spell_target(table: &mlua::Table) -> Option<SpellTarget> {
    // Try string target first
    if let Ok(target_str) = table.get::<String>("target") {
        return match target_str.to_lowercase().as_str() {
            "self" => Some(SpellTarget::Self_),
            "current_target" => Some(SpellTarget::CurrentTarget),
            "random_hostile" => Some(SpellTarget::RandomHostile),
            "random_hostile_in_range" => Some(SpellTarget::RandomHostileInRange),
            "lowest_health_friendly" => Some(SpellTarget::LowestHealthFriendly),
            _ => None,
        };
    }

    // Try GUID target
    if let Some(guid) = parse_guid(table, "target") {
        return Some(SpellTarget::Guid(guid));
    }

    None
}

/// Parse summon type from Lua table.
fn parse_summon_type(table: &mlua::Table) -> SummonType {
    if let Ok(summon_str) = table.get::<String>("summon_type") {
        match summon_str.to_uppercase().as_str() {
            "TIMED_DESPAWN" => SummonType::TimedDespawn,
            "TIMED_DESPAWN_OUT_OF_COMBAT" => SummonType::TimedDespawnOutOfCombat,
            "CORPSE_DESPAWN" => SummonType::CorpseDespawn,
            "CORPSE_TIMED_DESPAWN" => SummonType::CorpseTimedDespawn,
            "DEAD_DESPAWN" => SummonType::DeadDespawn,
            "MANUAL_DESPAWN" => SummonType::ManualDespawn,
            _ => SummonType::default(),
        }
    } else {
        SummonType::default()
    }
}

/// Parse react state from Lua table.
fn parse_react_state(table: &mlua::Table) -> Option<ReactState> {
    let state_str: String = table.get("state").ok()?;
    match state_str.to_uppercase().as_str() {
        "PASSIVE" => Some(ReactState::Passive),
        "DEFENSIVE" => Some(ReactState::Defensive),
        "AGGRESSIVE" => Some(ReactState::Aggressive),
        _ => None,
    }
}

/// Parse a table of actions from Lua.
pub fn parse_actions(lua_result: mlua::Value) -> Vec<LuaAction> {
    let mut actions = Vec::new();

    if let mlua::Value::Table(table) = lua_result {
        // Iterate over the array of action tables
        for pair in table.pairs::<i32, mlua::Table>() {
            if let Ok((_, action_table)) = pair {
                if let Some(action) = LuaAction::from_lua_table(&action_table) {
                    actions.push(action);
                }
            }
        }
    }

    actions
}
