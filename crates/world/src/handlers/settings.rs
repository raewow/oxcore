//! Settings packet handlers for world
//!
//! These handlers are slim wrappers that parse packets and delegate
//! to the SettingsSystem. No business logic lives here.
//!
//! Handled packets:
//! - CMSG_SET_ACTION_BUTTON (0x0128)
//! - CMSG_UPDATE_ACCOUNT_DATA (0x20B)
//! - CMSG_REQUEST_ACCOUNT_DATA (0x20A)
//! - CMSG_TUTORIAL_FLAG (0x00FF)
//! - CMSG_TUTORIAL_CLEAR (0x0100)
//! - CMSG_TUTORIAL_RESET (0x0101)
//! - CMSG_COMPLETE_CINEMATIC (0x00FC)
//! - CMSG_NEXT_CINEMATIC_CAMERA (0x00FB)
//! - CMSG_SET_ACTION_BAR_TOGGLES (0x0568)

use crate::core::session::WorldSession;
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::{Opcode, Protocol, WorldPacket};
use tracing::{debug, warn};

/// Handle CMSG_COMPLETE_CINEMATIC (0x00FC)
///
/// Sent by the client when a cinematic finishes playing.
/// Cinematics play entirely client-side in 1.12; no server state to update.
pub fn handle_complete_cinematic(
    _session: &WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    debug!("CMSG_COMPLETE_CINEMATIC");
    Ok(())
}

/// Handle CMSG_NEXT_CINEMATIC_CAMERA (0x00FB)
///
/// Empty in reference server; client-side cinematic camera advance notification.
pub fn handle_next_cinematic_camera(
    _session: &WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    Ok(())
}

/// Handle CMSG_SET_ACTION_BAR_TOGGLES (0x0568)
///
/// Wire format:
///   u8  action_bar bitmask (bits 0-5 = extra bars 2-7 visible)
pub fn handle_set_action_bar_toggles(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let action_bar = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read action_bar byte"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            if action_bar != 0 {
                warn!(
                    "CMSG_SET_ACTION_BAR_TOGGLES: no player logged in, value={}",
                    action_bar
                );
            }
            return Ok(());
        }
    };

    debug!(
        "CMSG_SET_ACTION_BAR_TOGGLES: player={}, value={}",
        player_guid, action_bar
    );

    world
        .systems
        .settings
        .set_action_bar_toggles(player_guid, action_bar, world);

    Ok(())
}

/// Handle CMSG_SET_ACTION_BUTTON (0x0128)
///
/// Sent when the player drags a spell, item, or macro to an action bar slot.
/// Also sent with action=0 to clear a slot.
///
/// Wire format:
///   u8  slot (0-119)
///   u32 packed_action (action in low 24 bits, type in high 8 bits)
pub async fn handle_set_action_button(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // The two layouts disagree on order *and* width, so this is a branch, not a translation.
    // Vanilla packs the action and type into one u32 after the slot; 1.14 sends them as two u16s
    // ahead of it (JimsProxy `World/Server/Packets/CharacterPackets.cs:755-769`). Reading a modern
    // body the vanilla way puts the action in the slot and lands the slot inside the action.
    let (slot, action, button_type) = if session.protocol() == Protocol::Modern {
        let action = packet
            .read_u16()
            .ok_or_else(|| anyhow::anyhow!("Failed to read action"))?;
        let button_type = packet
            .read_u16()
            .ok_or_else(|| anyhow::anyhow!("Failed to read button type"))?;
        let slot = packet
            .read_u8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read slot"))?;
        (slot, u32::from(action), button_type as u8)
    } else {
        let slot = packet
            .read_u8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read slot"))?;
        let packed_action = packet
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read packed_action"))?;
        (
            slot,
            packed_action & 0x00FF_FFFF,
            ((packed_action >> 24) & 0xFF) as u8,
        )
    };

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_SET_ACTION_BUTTON received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_SET_ACTION_BUTTON: player={}, slot={}, action={}, type={}",
        player_guid, slot, action, button_type
    );

    if action == 0 && button_type == 0 {
        // Clear the action button
        world
            .systems
            .settings
            .clear_action_button(player_guid, slot, world)
            .await?;
    } else {
        // Set the action button
        world
            .systems
            .settings
            .set_action_button(player_guid, slot, action, button_type, world)
            .await?;
    }

    Ok(())
}

/// Handle CMSG_UPDATE_ACCOUNT_DATA (0x20B)
///
/// Sent when the client wants to save account data (key bindings, macros, etc.).
/// The data is compressed with zlib.
///
/// Wire format (1.12 client):
///   u32 data_type (0-7)
///   u32 decompressed_size
///   u8[] compressed_data (rest of packet)
pub async fn handle_update_account_data(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let data_type = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read data_type"))?;
    let decompressed_size = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read decompressed_size"))?;

    // Read remaining bytes as compressed data
    use bytes::Buf;
    let compressed_data = packet.data().chunk().to_vec();

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_UPDATE_ACCOUNT_DATA received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_UPDATE_ACCOUNT_DATA: player={}, type={}, decompressed_size={}",
        player_guid, data_type, decompressed_size
    );

    world
        .systems
        .settings
        .handle_account_data_update(
            player_guid,
            session.account_id(),
            data_type,
            decompressed_size,
            &compressed_data,
            world,
        )
        .await?;

    Ok(())
}

/// Handle CMSG_REQUEST_ACCOUNT_DATA (0x20A)
///
/// Sent when the client wants to request account data (usually during login
/// when it detects its cache is stale based on timestamps).
///
/// Wire format:
///   u32 data_type (0-7)
pub async fn handle_request_account_data(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let data_type = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read data_type"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_REQUEST_ACCOUNT_DATA received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_REQUEST_ACCOUNT_DATA: player={}, type={}",
        player_guid, data_type
    );

    world
        .systems
        .settings
        .handle_account_data_request(player_guid, data_type, world)
        .await?;

    Ok(())
}

/// Handle CMSG_TUTORIAL_FLAG (0x00FF)
///
/// Sent when the player completes a tutorial (e.g., first time opening a panel).
///
/// Wire format:
///   u32 flag_index (0-255)
pub async fn handle_tutorial_flag(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let flag_index = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read flag_index"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_TUTORIAL_FLAG received but player not logged in");
            return Ok(());
        }
    };

    debug!(
        "CMSG_TUTORIAL_FLAG: player={}, flag_index={}",
        player_guid, flag_index
    );

    world
        .systems
        .settings
        .handle_tutorial_flag(player_guid, flag_index, world)
        .await?;

    Ok(())
}

/// Handle CMSG_TUTORIAL_CLEAR (0x0100)
///
/// Sent when the player clicks "Clear All Tutorials" in the help panel.
/// Marks all tutorials as completed.
pub async fn handle_tutorial_clear(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_TUTORIAL_CLEAR received but player not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_TUTORIAL_CLEAR: player={}", player_guid);

    world
        .systems
        .settings
        .handle_tutorial_clear(player_guid, world)
        .await?;

    Ok(())
}

/// Handle CMSG_TUTORIAL_RESET (0x0101)
///
/// Sent when the player clicks "Reset Tutorials" in the help panel.
/// Clears all tutorial flags so they will show again.
pub async fn handle_tutorial_reset(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => {
            warn!("CMSG_TUTORIAL_RESET received but player not logged in");
            return Ok(());
        }
    };

    debug!("CMSG_TUTORIAL_RESET: player={}", player_guid);

    world
        .systems
        .settings
        .handle_tutorial_reset(player_guid, world)
        .await?;

    Ok(())
}
