//! Settings System - handles action buttons, macros, account data, and tutorials
//!
//! Stateless system that operates on SettingsState embedded in Player.

use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::World;
use anyhow::Result;
use std::sync::Arc;

use super::account_data::{decompress_account_data, AccountDataType};
use super::state::{AccountDataEntry, SettingsState, MAX_ACTION_BUTTONS, NUM_ACCOUNT_DATA_TYPES};

/// Stateless system that operates on SettingsState through PlayerManager.
/// All packets are sent via BroadcastManager, never directly on sessions.
pub struct SettingsSystem {
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
}

impl SettingsSystem {
    pub fn new(broadcast_mgr: Arc<dyn BroadcastManagerTrait>) -> Self {
        Self { broadcast_mgr }
    }

    // ---------------------------------------------------------------
    // Action Buttons
    // ---------------------------------------------------------------

    /// Set a single action button (called from CMSG_SET_ACTION_BUTTON handler).
    pub async fn set_action_button(
        &self,
        player_guid: ObjectGuid,
        slot: u8,
        action: u32,
        button_type: u8,
        world: &World,
    ) -> Result<()> {
        if slot as usize >= MAX_ACTION_BUTTONS {
            tracing::warn!(
                "set_action_button: slot {} out of range for player {}",
                slot,
                player_guid
            );
            return Ok(());
        }

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.set_action_button(slot, action, button_type);
            });

        tracing::debug!(
            "Action button set: player={}, slot={}, action={}, type={}",
            player_guid,
            slot,
            action,
            button_type
        );
        Ok(())
    }

    /// Clear a single action button.
    pub async fn clear_action_button(
        &self,
        player_guid: ObjectGuid,
        slot: u8,
        world: &World,
    ) -> Result<()> {
        if slot as usize >= MAX_ACTION_BUTTONS {
            return Ok(());
        }

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.clear_action_button(slot);
            });

        tracing::debug!(
            "Action button cleared: player={}, slot={}",
            player_guid,
            slot
        );
        Ok(())
    }

    /// Send action buttons to client (called during login).
    pub fn send_action_buttons(&self, player_guid: ObjectGuid, world: &World) {
        use crate::shared::messages::login::ActionButton as MsgActionButton;
        use crate::shared::messages::login::SmsgActionButtons;

        let buttons = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
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
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    // ---------------------------------------------------------------
    // Account Data
    // ---------------------------------------------------------------

    /// Handle incoming CMSG_UPDATE_ACCOUNT_DATA.
    ///
    /// The client sends compressed data for one of 8 account data types.
    /// We decompress, store, and echo confirmation back.
    pub async fn handle_account_data_update(
        &self,
        player_guid: ObjectGuid,
        data_type: u32,
        decompressed_size: u32,
        compressed_data: &[u8],
        world: &World,
    ) -> Result<()> {
        let ad_type = match AccountDataType::from_u32(data_type) {
            Some(t) => t,
            None => {
                tracing::warn!(
                    "Invalid account data type {} from player {}",
                    data_type,
                    player_guid
                );
                return Ok(());
            }
        };

        // Decompress the data blob (size validation happens inside)
        let decompressed = decompress_account_data(compressed_data, decompressed_size)?;

        // Verify decompressed size matches what client claimed
        if decompressed.len() != decompressed_size as usize {
            tracing::warn!(
                "Account data decompressed size mismatch: expected {}, got {} from player {}",
                decompressed_size,
                decompressed.len(),
                player_guid
            );
        }

        // Generate server-side timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        // Store in player state
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.account_data[data_type as usize] = Some(AccountDataEntry {
                    time: timestamp,
                    data: decompressed.clone(),
                });
                player.settings.need_save = true;
            });

        // Echo back SMSG_UPDATE_ACCOUNT_DATA to confirm receipt
        use crate::shared::messages::settings::SmsgUpdateAccountData;
        let response = SmsgUpdateAccountData {
            data_type,
            data: decompressed,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, response);

        tracing::debug!(
            "Account data updated: player={}, type={:?}, timestamp={}",
            player_guid,
            ad_type,
            timestamp
        );
        Ok(())
    }

    /// Handle CMSG_REQUEST_ACCOUNT_DATA: client wants a specific blob.
    pub async fn handle_account_data_request(
        &self,
        player_guid: ObjectGuid,
        data_type: u32,
        world: &World,
    ) -> Result<()> {
        if data_type as usize >= NUM_ACCOUNT_DATA_TYPES {
            tracing::warn!(
                "Invalid account data request type {} from player {}",
                data_type,
                player_guid
            );
            return Ok(());
        }

        let data = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
                player.settings.account_data[data_type as usize]
                    .as_ref()
                    .map(|entry| entry.data.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        use crate::shared::messages::settings::SmsgUpdateAccountData;
        let response = SmsgUpdateAccountData { data_type, data };
        self.broadcast_mgr.send_msg_to_player(player_guid, response);

        Ok(())
    }

    /// Send account data times during login (SMSG_ACCOUNT_DATA_TIMES).
    pub fn send_account_data_times(&self, player_guid: ObjectGuid, world: &World) {
        use crate::shared::messages::settings::SmsgAccountDataTimes;

        let timestamps = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| {
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
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    // ---------------------------------------------------------------
    // Tutorial Flags
    // ---------------------------------------------------------------

    /// Handle CMSG_TUTORIAL_FLAG: set a single tutorial bit.
    pub async fn handle_tutorial_flag(
        &self,
        player_guid: ObjectGuid,
        flag_index: u32,
        world: &World,
    ) -> Result<()> {
        let word = (flag_index / 32) as usize;
        if word >= crate::world::game::player::settings::state::TUTORIAL_FLAG_COUNT {
            tracing::debug!(
                "Tutorial flag index {} out of range for player {}",
                flag_index,
                player_guid
            );
            return Ok(());
        }

        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.set_tutorial_flag(flag_index);
            });

        Ok(())
    }

    /// Handle CMSG_TUTORIAL_CLEAR: mark all tutorials as seen.
    pub async fn handle_tutorial_clear(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.complete_all_tutorials();
            });
        Ok(())
    }

    /// Handle CMSG_TUTORIAL_RESET: reset all tutorials to unseen.
    pub async fn handle_tutorial_reset(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<()> {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.reset_tutorial_flags();
            });
        Ok(())
    }

    /// Send tutorial flags during login (SMSG_TUTORIAL_FLAGS).
    pub fn send_tutorial_flags(&self, player_guid: ObjectGuid, world: &World) {
        use crate::shared::messages::login::SmsgTutorialFlags;

        let flags = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| player.settings.tutorial_flags)
            .unwrap_or([0xFFFFFFFF; super::state::TUTORIAL_FLAG_COUNT]);

        let msg = SmsgTutorialFlags { flags };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }
}
