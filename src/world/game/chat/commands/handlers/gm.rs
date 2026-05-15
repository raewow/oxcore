//! GM command handlers for world
//!
//! Commands for Game Masters to manage player state.

use anyhow::Result;
use sqlx::Row;

use crate::shared::common::AccountType;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::{Opcode, Position, WorldPacket};
use crate::world::game::chat::commands::context::{ChatCommandContext, ChatCommandInfo};

/// Base movement speeds (from MaNGOS/Unit.cpp)
const BASE_RUN_SPEED: f32 = 7.0;

/// Mod command - modifies player stats (speed, etc.)
pub async fn cmd_mod(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok("Usage: .mod <stat> <value>. Example: .mod speed 10".to_string());
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok("Usage: .mod <stat> <value>. Example: .mod speed 10".to_string());
    }

    let stat_name = parts[0].to_lowercase();
    let value = parts[1]
        .parse::<f32>()
        .map_err(|_| anyhow::anyhow!("Invalid value. Must be a number."))?;

    match stat_name.as_str() {
        "speed" => mod_speed(ctx, value).await,
        _ => Ok(format!(
            "Unknown stat '{}'. Supported stats: speed",
            stat_name
        )),
    }
}

/// Modify player run speed
async fn mod_speed(ctx: &ChatCommandContext<'_>, rate: f32) -> Result<String> {
    let rate = rate.max(0.0);
    let new_speed = rate * BASE_RUN_SPEED;

    // Build SMSG_FORCE_RUN_SPEED_CHANGE packet
    let mut packet = WorldPacket::new(Opcode::SMSG_FORCE_RUN_SPEED_CHANGE);

    // Write packed GUID
    let guid_raw = ctx.player_guid.raw();
    packet.write_packed_guid_raw(guid_raw);

    // Write movement counter (use 0 for GM commands)
    packet.write_u32(0);

    // Write new speed
    packet.write_f32(new_speed);

    // Send packet to player
    ctx.session.send_packet(packet)?;

    Ok(format!(
        "Set run speed to {}x ({:.2} yards/sec)",
        rate, new_speed
    ))
}

pub fn mod_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "mod",
        help: "Modifies player stats. Usage: .mod <stat> <value>. Example: .mod speed 10",
        min_security: AccountType::GameMaster,
    }
}

/// Speed command - standalone command to change speed
pub async fn cmd_speed(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok("Usage: .speed <multiplier>. Example: .speed 10".to_string());
    }

    let rate = match args.parse::<f32>() {
        Ok(r) if r >= 0.0 => r,
        _ => return Ok("Invalid speed value. Must be a positive number.".to_string()),
    };

    mod_speed(ctx, rate).await
}

pub fn speed_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "speed",
        help: "Set movement speed multiplier. Usage: .speed <multiplier>",
        min_security: AccountType::GameMaster,
    }
}

/// Kill command - instantly kills the current target
pub async fn cmd_kill(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    use crate::shared::protocol::ObjectGuid;

    let args = args.trim();
    let target_guid = if matches!(args.to_ascii_lowercase().as_str(), "self" | "me") {
        ctx.player_guid
    } else {
        match ctx.target {
            Some(guid) => guid,
            None => {
                return Ok(
                    "No target selected. Select a target first, or use .kill self.".to_string(),
                )
            }
        }
    };

    // 2. Validate target is a unit
    if !target_guid.is_unit() {
        return Ok("Target must be a unit (player or creature).".to_string());
    }

    // 3. Get target name before killing (for feedback)
    let target_name = get_target_name(ctx, target_guid);

    // 4. Execute kill based on target type
    if target_guid.is_player() {
        let is_self_kill = target_guid == ctx.player_guid;
        let killer_guid = if target_guid == ctx.player_guid {
            None
        } else {
            Some(ctx.player_guid)
        };

        // Kill player via DeathSystem
        ctx.world.systems.death.on_killed(
            target_guid,
            killer_guid,
            Some(5), // spell_id (5 = GM instant kill)
            ctx.world,
        )?;

        if is_self_kill {
            ctx.world
                .systems
                .death
                .handle_release_spirit(ctx.player_guid, ctx.world)?;
            return Ok("Killed yourself and released spirit.".to_string());
        }
    } else {
        // Kill creature via CreatureManager
        let death_info = ctx.world.managers.creature_mgr.handle_death(
            target_guid,
            Some(ctx.player_guid), // killer
        );

        // Stop creature movement on death (vmangos: StopMoving in SetDeathState)
        if let Some(ref info) = death_info {
            ctx.world.systems.creature_movement.send_stop_packet(
                info.guid,
                info.position,
                ctx.world,
            );
        }

        // Send death VALUES update so the client sees the creature die
        // (health=0, stand_state=7, clear flags)
        send_creature_killed_update(ctx.world, target_guid);
    }

    Ok(format!("Killed {}", target_name))
}

/// Send death VALUES update to nearby players so the client sees the creature die.
/// Same as creature_combat.rs send_creature_killed_update but callable from GM commands.
fn send_creature_killed_update(world: &crate::world::World, creature_guid: ObjectGuid) {
    use crate::shared::messages::update::{
        ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
    };
    use crate::shared::messages::ToWorldPacket;
    use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
    use crate::world::game::broadcast_mgr::broadcast_around_creature;
    use crate::world::game::common::update_fields::*;

    let Some((max_health, unit_flags)) = world
        .managers
        .creature_mgr
        .with_creature_mut(creature_guid, |c| (c.max_health, c.unit_flags))
    else {
        return;
    };

    let cleared_flags = unit_flags & !crate::world::game::common::unit_flags::IN_COMBAT;
    let world_guid = WorldObjectGuid::new_creature(creature_guid.entry(), creature_guid.counter());
    let empty_guid = WorldObjectGuid::from_raw(0);

    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(world_guid, ObjectType::Unit)
            .set_guid_field(UNIT_FIELD_TARGET, empty_guid)
            .set_field(UNIT_FIELD_HEALTH, 0u32)
            .set_field(UNIT_FIELD_MAXHEALTH, max_health)
            .set_field(UNIT_FIELD_FLAGS, cleared_flags)
            .set_field(UNIT_DYNAMIC_FLAGS, 0u32)
            .set_field(UNIT_FIELD_BYTES_1, 7u32) // Stand state Dead
            .set_field(UNIT_NPC_FLAGS, 0u32),
    ));

    broadcast_around_creature(world, creature_guid, &update.to_world_packet());
}

/// Helper to get target name for feedback
fn get_target_name(ctx: &ChatCommandContext<'_>, guid: ObjectGuid) -> String {
    if guid.is_player() {
        ctx.world
            .managers
            .player_mgr
            .get_player(guid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Unknown Player".to_string())
    } else if guid.is_creature() {
        // Get creature template name
        ctx.world
            .managers
            .creature_mgr
            .get_creature(guid)
            .and_then(|creature| {
                ctx.world
                    .managers
                    .creature_mgr
                    .get_template(creature.entry)
                    .map(|template| template.name.clone())
            })
            .unwrap_or_else(|| "Unknown Creature".to_string())
    } else {
        "Target".to_string()
    }
}

pub fn kill_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "kill",
        help: "Instantly kills your current target. Usage: .kill [self]",
        min_security: AccountType::GameMaster,
    }
}

// ---------------------------------------------------------------------------
// Teleport command
// ---------------------------------------------------------------------------

/// Teleport command - lists teleports or teleports to a location
pub async fn cmd_teleport(ctx: &ChatCommandContext<'_>, args: &str) -> Result<String> {
    let args = args.trim();

    // No arguments → list first page
    if args.is_empty() {
        return list_teleports(ctx, 1).await;
    }

    // "list" with optional page number
    if args.to_lowercase().starts_with("list") {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let page = if parts.len() > 1 {
            parts[1].parse::<u32>().unwrap_or(1)
        } else {
            1
        };
        return list_teleports(ctx, page).await;
    }

    // Try numeric ID first
    if let Ok(teleport_id) = args.parse::<u32>() {
        return teleport_by_id(ctx, teleport_id).await;
    }

    // Otherwise search by name
    teleport_by_name(ctx, args).await
}

/// List available teleports from game_tele table with pagination
async fn list_teleports(ctx: &ChatCommandContext<'_>, page: u32) -> Result<String> {
    let pool = &ctx.world.databases.world;
    const ITEMS_PER_PAGE: u32 = 50;

    let total_count: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_tele")
        .fetch_one(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query teleports: {}", e))?;

    if total_count == 0 {
        return Ok("No teleports found in database.".to_string());
    }

    let total_pages = ((total_count as u32) + ITEMS_PER_PAGE - 1) / ITEMS_PER_PAGE;
    let page = page.max(1).min(total_pages.max(1));
    let offset = (page - 1) * ITEMS_PER_PAGE;

    let rows = sqlx::query(
        "SELECT id, name, map, position_x, position_y, position_z \
         FROM game_tele ORDER BY name LIMIT ? OFFSET ?",
    )
    .bind(ITEMS_PER_PAGE as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to query teleports: {}", e))?;

    let mut output = format!(
        "Available teleports ({} total) - Page {}/{}:\n",
        total_count, page, total_pages
    );

    for row in &rows {
        let id: u32 = row.get(0);
        let name: String = row.get(1);
        let map_id: u32 = row.get(2);
        let x: f32 = row.get(3);
        let y: f32 = row.get(4);
        let z: f32 = row.get(5);
        output.push_str(&format!(
            "  [{}] {} - Map: {}, ({:.1}, {:.1}, {:.1})\n",
            id, name, map_id, x, y, z
        ));
    }

    if total_pages > 1 {
        output.push_str(&format!(
            "\nPage {}/{} - Use .teleport list <page> to navigate. ",
            page, total_pages
        ));
        if page < total_pages {
            output.push_str(&format!("Next: .teleport list {}\n", page + 1));
        }
        if page > 1 {
            output.push_str(&format!("Previous: .teleport list {}\n", page - 1));
        }
    }

    output.push_str("Use .teleport <name> or .teleport <id> to teleport.\n");
    Ok(output)
}

/// Teleport by numeric ID
async fn teleport_by_id(ctx: &ChatCommandContext<'_>, teleport_id: u32) -> Result<String> {
    let pool = &ctx.world.databases.world;

    let row = sqlx::query(
        "SELECT id, name, map, position_x, position_y, position_z, orientation \
         FROM game_tele WHERE id = ?",
    )
    .bind(teleport_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to query teleport: {}", e))?;

    let row = match row {
        Some(r) => r,
        None => return Ok(format!("Teleport ID {} not found.", teleport_id)),
    };

    let name: String = row.get(1);
    let map_id: u32 = row.get(2);
    let x: f32 = row.get(3);
    let y: f32 = row.get(4);
    let z: f32 = row.get(5);
    let o: f32 = row.get(6);

    perform_teleport(ctx, map_id, Position { x, y, z, o }, &name)
}

/// Teleport by name (partial match)
async fn teleport_by_name(ctx: &ChatCommandContext<'_>, search_name: &str) -> Result<String> {
    let pool = &ctx.world.databases.world;
    let pattern = format!("%{}%", search_name);

    let row = sqlx::query(
        "SELECT id, name, map, position_x, position_y, position_z, orientation \
         FROM game_tele WHERE name LIKE ? ORDER BY name LIMIT 1",
    )
    .bind(&pattern)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("Failed to query teleport: {}", e))?;

    let row = match row {
        Some(r) => r,
        None => {
            return Ok(format!(
                "No teleport found matching '{}'. Use .teleport to list all.",
                search_name
            ))
        }
    };

    let name: String = row.get(1);
    let map_id: u32 = row.get(2);
    let x: f32 = row.get(3);
    let y: f32 = row.get(4);
    let z: f32 = row.get(5);
    let o: f32 = row.get(6);

    perform_teleport(ctx, map_id, Position { x, y, z, o }, &name)
}

/// Send the teleport packets and store pending teleport in the session.
///
/// Uses the same two-step flow as area triggers:
///   1. SMSG_TRANSFER_PENDING + SMSG_NEW_WORLD  →  client loads new map
///   2. Client sends MSG_MOVE_WORLDPORT_ACK      →  handle_worldport_ack completes it
fn perform_teleport(
    ctx: &ChatCommandContext<'_>,
    map_id: u32,
    dest: Position,
    location_name: &str,
) -> Result<String> {
    let player_map = ctx
        .world
        .managers
        .player_mgr
        .with_player(ctx.player_guid, |p| p.map_id)
        .unwrap_or(0);

    // SMSG_TRANSFER_PENDING (only when changing maps, but send always for GM
    // teleport to guarantee the client enters the loading screen)
    let mut transfer = WorldPacket::new(Opcode::SMSG_TRANSFER_PENDING);
    transfer.write_u32(map_id);
    ctx.session.send_packet(transfer)?;

    // SMSG_NEW_WORLD
    let mut new_world = WorldPacket::new(Opcode::SMSG_NEW_WORLD);
    new_world.write_u32(map_id);
    new_world.write_f32(dest.x);
    new_world.write_f32(dest.y);
    new_world.write_f32(dest.z);
    new_world.write_f32(dest.o);
    ctx.session.send_packet(new_world)?;

    // Instance id 0 for GM teleport (continents)
    ctx.session.set_pending_teleport(Some((map_id, 0, dest)));

    Ok(format!("Teleporting to: {}", location_name))
}

pub fn teleport_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "teleport",
        help: "Teleport to a location. Usage: .teleport [list [page]|name|id]",
        min_security: AccountType::GameMaster,
    }
}

pub fn tp_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "tp",
        help: "Teleport to a location. Usage: .tp [list [page]|name|id]",
        min_security: AccountType::GameMaster,
    }
}

pub fn tele_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "tele",
        help: "Teleport to a location. Usage: .tele [list [page]|name|id]",
        min_security: AccountType::GameMaster,
    }
}

pub fn port_info() -> ChatCommandInfo {
    ChatCommandInfo {
        name: "port",
        help: "Teleport to a location. Usage: .port [list [page]|name|id]",
        min_security: AccountType::GameMaster,
    }
}
