//! Settings System - handles action buttons, macros, account data, and tutorials
//!
//! Stateless system that operates on SettingsState embedded in Player.

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

use super::state::{
    AccountDataEntry, SettingsState, ACTION_BUTTON_ITEM, ACTION_BUTTON_MACRO, ACTION_BUTTON_SPELL,
    MAX_ACTION_BUTTONS, NUM_ACCOUNT_DATA_TYPES,
};
use oxcore_shared::game::account_data::{decompress_account_data, AccountDataType};

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

        if button_type != ACTION_BUTTON_SPELL
            && button_type != ACTION_BUTTON_MACRO
            && button_type != ACTION_BUTTON_ITEM
        {
            tracing::warn!(
                "set_action_button: invalid button type {} for player {}",
                button_type,
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

    /// Handle CMSG_SET_ACTION_BAR_TOGGLES: store which extra action bars are shown.
    pub fn set_action_bar_toggles(&self, player_guid: ObjectGuid, value: u8, world: &World) {
        world
            .managers
            .player_mgr
            .with_player_mut(player_guid, |player| {
                player.settings.set_action_bar_toggles(value);
            });

        tracing::debug!(
            "Action bar toggles set: player={}, value={}",
            player_guid,
            value
        );
    }

    /// Send action buttons to client (called during login).
    pub fn send_action_buttons(&self, player_guid: ObjectGuid, world: &World) {
        use oxcore_shared::messages::login::ActionButton as MsgActionButton;
        use oxcore_shared::messages::login::SmsgActionButtons;

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
    /// We decompress, store in memory, persist to DB immediately, and echo confirmation back.
    ///
    /// `client_time` is the modification time a 1.14 client attaches to its save; a 1.12 client
    /// sends none, so the server mints its own. Using the client's own time keeps the timestamp
    /// stable across the round trip, so the client's next `SMSG_ACCOUNT_DATA_TIMES` comparison sees
    /// an equal time and does not re-download its own blob.
    pub async fn handle_account_data_update(
        &self,
        player_guid: ObjectGuid,
        account_id: u32,
        data_type: u32,
        decompressed_size: u32,
        compressed_data: &[u8],
        client_time: Option<u32>,
        world: &World,
    ) -> Result<()> {
        use oxcore_db::database::CharacterRepository;

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

        // Prefer the client's own modification time when it sent one; otherwise mint one. A zero
        // time means "never cached", so it is treated the same as an absent client time.
        let server_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        let timestamp = client_time.filter(|&t| t != 0).unwrap_or(server_now);

        if matches!(
            ad_type,
            AccountDataType::GlobalBindings | AccountDataType::PerCharBindings
        ) {
            tracing::info!(
                player = %player_guid,
                account_id,
                data_type,
                compressed_bytes = compressed_data.len(),
                binding_bytes = decompressed.len(),
                timestamp,
                "received keybinding account-data update"
            );
        }

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

        // Persist to DB immediately
        let char_repo = CharacterRepository::new(Arc::new(world.databases.character.clone()));
        let time_u64 = timestamp as u64;
        if ad_type.is_global() {
            char_repo
                .upsert_account_data(account_id, data_type, time_u64, &decompressed)
                .await?;
        } else {
            char_repo
                .upsert_character_account_data(
                    player_guid.counter(),
                    data_type,
                    time_u64,
                    &decompressed,
                )
                .await?;
        }

        if matches!(
            ad_type,
            AccountDataType::GlobalBindings | AccountDataType::PerCharBindings
        ) {
            tracing::info!(
                player = %player_guid,
                account_id,
                data_type,
                binding_bytes = decompressed.len(),
                timestamp,
                "persisted keybinding account data"
            );
        }

        // Echo back SMSG_UPDATE_ACCOUNT_DATA to confirm receipt
        use oxcore_shared::messages::settings::SmsgUpdateAccountData;
        let response = SmsgUpdateAccountData {
            data_type,
            data: decompressed,
            player_guid,
            realm_id: world.get_realm_id() as u16,
            time: timestamp,
        };
        self.send_account_data_response(player_guid, account_id, response, world);

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
        account_id: u32,
        data_type: u32,
        world: &World,
    ) -> Result<()> {
        const MODERN_ACCOUNT_DATA_TYPES: usize = 13;
        if data_type as usize >= MODERN_ACCOUNT_DATA_TYPES {
            tracing::warn!(
                "Invalid account data request type {} from player {}",
                data_type,
                player_guid
            );
            return Ok(());
        }

        let (data, time) = if data_type as usize >= NUM_ACCOUNT_DATA_TYPES {
            // These slots were added after the vanilla account-data schema. A modern client still
            // expects an answer for them, but there is no legacy data to restore.
            (Vec::new(), 0)
        } else {
            world
                .managers
                .player_mgr
                .with_player(player_guid, |player| {
                    player.settings.account_data[data_type as usize]
                        .as_ref()
                        .map(|entry| (entry.data.clone(), entry.time))
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        };

        if matches!(
            AccountDataType::from_u32(data_type),
            Some(AccountDataType::GlobalBindings | AccountDataType::PerCharBindings)
        ) {
            tracing::info!(
                player = %player_guid,
                account_id,
                data_type,
                binding_bytes = data.len(),
                timestamp = time,
                "client requested keybinding account data"
            );
        }

        use oxcore_shared::messages::settings::SmsgUpdateAccountData;
        let response = SmsgUpdateAccountData {
            data_type,
            data,
            player_guid,
            realm_id: world.get_realm_id() as u16,
            time,
        };
        self.send_account_data_response(player_guid, account_id, response, world);

        Ok(())
    }

    fn send_account_data_response(
        &self,
        player_guid: ObjectGuid,
        account_id: u32,
        response: oxcore_shared::messages::settings::SmsgUpdateAccountData,
        world: &World,
    ) {
        if let Some(realm_session) = world.session_mgr.get_realm_session_by_account(account_id) {
            if matches!(response.data_type, 2 | 3) {
                tracing::info!(
                    player = %player_guid,
                    account_id,
                    data_type = response.data_type,
                    binding_bytes = response.data.len(),
                    timestamp = response.time,
                    "sending keybinding account data on realm socket"
                );
            }
            if let Err(error) = realm_session.send_msg(response) {
                tracing::warn!(%error, "failed to return account data on modern realm socket");
            }
        } else {
            if matches!(response.data_type, 2 | 3) {
                tracing::info!(
                    player = %player_guid,
                    account_id,
                    data_type = response.data_type,
                    binding_bytes = response.data.len(),
                    timestamp = response.time,
                    "sending keybinding account data on instance socket"
                );
            }
            self.broadcast_mgr.send_msg_to_player(player_guid, response);
        }
    }

    /// Send account data times during login (SMSG_ACCOUNT_DATA_TIMES).
    pub fn send_account_data_times(&self, player_guid: ObjectGuid, world: &World) {
        use oxcore_shared::messages::settings::SmsgAccountDataTimes;

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

        // Realm 1: this system has no realm id in scope, so hardcode 1 here.
        // It only qualifies the GUID-128, so it must agree with whatever `SmsgCharEnum` used —
        // both are 1 today. Thread a real realm id through `SystemManager` before running a
        // second realm.
        let msg = SmsgAccountDataTimes::new(timestamps, player_guid, 1);
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
        if word >= crate::game::player::settings::state::TUTORIAL_FLAG_COUNT {
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
        use oxcore_shared::messages::login::SmsgTutorialFlags;

        let flags = world
            .managers
            .player_mgr
            .with_player(player_guid, |player| player.settings.tutorial_flags)
            .unwrap_or([0xFFFFFFFF; super::state::TUTORIAL_FLAG_COUNT]);

        let msg = SmsgTutorialFlags { flags };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }
}
