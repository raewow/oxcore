//! Social System - friend lists, ignore lists, status broadcasting, WHO command

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use std::sync::Arc;

use crate::shared::database::characters::models::social::CharacterSocialRow;
use crate::shared::database::characters::repositories::SocialRepositoryTrait;
use crate::shared::game::social::{FriendInfo, FriendStatus, FriendsResult, SocialFlag, SOCIALMGR_FRIEND_LIMIT, SOCIALMGR_IGNORE_LIMIT};
use crate::shared::messages::social::{SmsgFriendList, SmsgFriendStatus, SmsgIgnoreList, SmsgWho, WhoPlayerInfo};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{HighGuid, ObjectGuid, Position};
use crate::world::game::player::PlayerManager;
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::BroadcastManager;

use super::types::{FriendEntry, IgnoreEntry, SocialState, WhoRequest, WhisperBlockReason, WhisperValidationResult};

/// Social System - manages friend lists, ignore lists, and social interactions
pub struct SocialSystem {
    /// Per-player social state (Arc for sharing across async tasks)
    state: Arc<DashMap<ObjectGuid, SocialState>>,
    repository: Arc<dyn SocialRepositoryTrait>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    player_mgr: Arc<PlayerManager>,
}

impl SocialSystem {
    /// Create a new social system
    pub fn new(
        repository: Arc<dyn SocialRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
    ) -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            repository,
            broadcast_mgr,
            player_mgr,
        }
    }

    // ========== INITIALIZATION ==========

    /// Load player social data from database into cache
    /// Called on player login
    pub async fn load_player_social(&self, player_guid: ObjectGuid) -> Result<()> {
        let player_guid_low = player_guid.low();

        // Load from database
        let rows = self
            .repository
            .find_by_guid(player_guid_low).await
            .context("Failed to load social data")?;

        // Build social state
        let mut social_state = SocialState::new();

        for row in rows {
            let friend_guid = ObjectGuid::new_without_entry(HighGuid::Player, row.friend);

            if row.flags & SocialFlag::Friend as u8 != 0 {
                social_state.friends.insert(
                    friend_guid,
                    FriendEntry::new(friend_guid, row.flags),
                );
            }

            if row.flags & SocialFlag::Ignored as u8 != 0 {
                social_state.ignores.insert(
                    friend_guid,
                    IgnoreEntry::new(friend_guid, row.flags),
                );
            }
        }

        // Store in state map
        self.state.insert(player_guid, social_state.clone());

        tracing::debug!(
            "Loaded social data for player {:?}: {} friends, {} ignores",
            player_guid,
            social_state.friend_count(),
            social_state.ignore_count()
        );

        Ok(())
    }

    /// Unload player social data from cache
    /// Called on player logout
    pub fn unload_player_social(&self, player_guid: ObjectGuid) {
        self.state.remove(&player_guid);
        tracing::debug!("Unloaded social data for player {:?}", player_guid);
    }

    // ========== FRIEND OPERATIONS ==========

    /// Add a friend by name (resolves name to GUID)
    pub async fn add_friend_by_name(
        &self,
        player_guid: ObjectGuid,
        friend_name: String,
    ) -> Result<()> {
        // Resolve friend name to GUID via database lookup
        let (friend_guid, is_friend_online) = match self
            .repository
            .find_player_guid_by_name(&friend_name)
            .await?
        {
            Some(guid_low) => {
                let guid = ObjectGuid::new_without_entry(HighGuid::Player, guid_low);
                // Check if friend is currently online
                let is_online = self.player_mgr.get_player(guid).is_some();
                (guid, is_online)
            }
            None => {
                // Friend not found
                let msg = SmsgFriendStatus {
                    result: FriendsResult::NotFound,
                    friend_guid: ObjectGuid::empty(),
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        };

        // Call internal add_friend with resolved GUID
        self.add_friend(player_guid, friend_guid, friend_name, is_friend_online).await

    }

    /// Add a friend to the player's friend list
    pub async fn add_friend(
        &self,
        player_guid: ObjectGuid,
        friend_guid: ObjectGuid,
        friend_name: String,
        is_friend_online: bool,
    ) -> Result<()> {
        // 1. Validate: cannot friend self
        if player_guid == friend_guid {
            let msg = SmsgFriendStatus {
                result: FriendsResult::Self_,
                friend_guid: ObjectGuid::empty(),
                friend_info: None,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // 2. Check if already friends
        if let Some(state) = self.state.get(&player_guid) {
            if state.has_friend(friend_guid) {
                let msg = SmsgFriendStatus {
                    result: FriendsResult::Already,
                    friend_guid,
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }

            // 3. Check friend list limit
            if state.friend_count() >= SOCIALMGR_FRIEND_LIMIT {
                let msg = SmsgFriendStatus {
                    result: FriendsResult::ListFull,
                    friend_guid,
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        }

        // 4. Add to database
        let flags = SocialFlag::Friend as u8;
        self.repository
            .add_or_update(player_guid.low(), friend_guid.low(), flags).await
            .context("Failed to add friend to database")?;

        // 5. Update state
        if let Some(mut state) = self.state.get_mut(&player_guid) {
            state.friends.insert(friend_guid, FriendEntry::new(friend_guid, flags));
        }

        // 6. Send response to player
        let result = if is_friend_online {
            FriendsResult::AddedOnline
        } else {
            FriendsResult::AddedOffline
        };

        let friend_info = if is_friend_online {
            // Resolve actual friend info from PlayerManager
            if let Some(friend_player) = self.player_mgr.get_player(friend_guid) {
                Some(FriendInfo {
                    status: FriendStatus::Online, // TODO: Check AFK/DND status from player flags
                    flags,
                    area: friend_player.zone_id,
                    level: friend_player.level as u32,
                    class: friend_player.class as u32,
                })
            } else {
                Some(FriendInfo::new(flags))
            }
        } else {
            None
        };

        let msg = SmsgFriendStatus {
            result,
            friend_guid,
            friend_info,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        tracing::debug!(
            "Player {:?} added friend {:?} ({})",
            player_guid,
            friend_guid,
            friend_name
        );

        Ok(())
    }

    /// Remove a friend from the player's friend list
    pub async fn remove_friend(
        &self,
        player_guid: ObjectGuid,
        friend_guid: ObjectGuid,
    ) -> Result<()> {
        // 1. Check if friend exists
        let has_friend = self.state.get(&player_guid)
            .map(|s| s.has_friend(friend_guid))
            .unwrap_or(false);

        if !has_friend {
            let msg = SmsgFriendStatus {
                result: FriendsResult::NotFound,
                friend_guid,
                friend_info: None,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // 2. Remove from database
        self.repository
            .remove(player_guid.low(), friend_guid.low()).await
            .context("Failed to remove friend from database")?;

        // 3. Update state
        if let Some(mut state) = self.state.get_mut(&player_guid) {
            state.friends.remove(&friend_guid);
        }

        // 4. Send response to player
        let msg = SmsgFriendStatus {
            result: FriendsResult::Removed,
            friend_guid,
            friend_info: None,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        tracing::debug!("Player {:?} removed friend {:?}", player_guid, friend_guid);

        Ok(())
    }

    /// Get all friend GUIDs for a player
    pub fn get_friend_list(&self, player_guid: ObjectGuid) -> Vec<ObjectGuid> {
        self.state
            .get(&player_guid)
            .map(|s| s.friends.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Send friend list to player
    ///
    /// Note: Friend names are NOT sent in this packet. The client uses its name cache
    /// (populated via SMSG_NAME_QUERY_RESPONSE) to display friend names.
    pub fn send_friend_list(&self, player_guid: ObjectGuid) {
        // Get friend GUIDs from state
        let friend_guids = self.get_friend_list(player_guid);
        let friend_guids_low: Vec<u32> = friend_guids.iter().map(|g| g.low()).collect();

        // Build friend info for each friend
        let mut friend_infos = Vec::new();

        for friend_guid in &friend_guids {
            // Get flags from state
            let flags = self
                .state
                .get(&player_guid)
                .and_then(|s| s.get_friend_entry(*friend_guid).map(|e| e.flags))
                .unwrap_or(SocialFlag::Friend as u8);

            // Check if friend is online
            if let Some(friend_player) = self.player_mgr.get_player(*friend_guid) {
                friend_infos.push(FriendInfo {
                    status: FriendStatus::Online, // TODO: Check AFK/DND
                    flags,
                    area: friend_player.zone_id,
                    level: friend_player.level as u32,
                    class: friend_player.class as u32,
                });
            } else {
                // Friend offline - only status is sent, no area/level/class
                friend_infos.push(FriendInfo {
                    status: FriendStatus::Offline,
                    flags,
                    area: 0,
                    level: 0,
                    class: 0,
                });
            }
        }

        let msg = SmsgFriendList {
            friend_guids: &friend_guids_low,
            friend_infos: &friend_infos,
        };
        let packet = msg.to_world_packet();
        self.broadcast_mgr.send_to_player(player_guid, packet);
    }

    // ========== IGNORE OPERATIONS ==========

    /// Add a player to ignore list by name
    pub async fn add_ignore_by_name(
        &self,
        player_guid: ObjectGuid,
        ignored_name: String,
    ) -> Result<()> {
        // Resolve ignored player name to GUID via database lookup
        let ignored_guid = match self
            .repository
            .find_player_guid_by_name(&ignored_name)
            .await?
        {
            Some(guid_low) => ObjectGuid::new_without_entry(HighGuid::Player, guid_low),
            None => {
                // Player not found
                let msg = SmsgFriendStatus {
                    result: FriendsResult::IgnoreNotFound,
                    friend_guid: ObjectGuid::empty(),
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        };

        // Call internal add_ignore with resolved GUID
        self.add_ignore(player_guid, ignored_guid, ignored_name).await
    }

    /// Add a player to the ignore list
    pub async fn add_ignore(
        &self,
        player_guid: ObjectGuid,
        ignored_guid: ObjectGuid,
        ignored_name: String,
    ) -> Result<()> {
        // 1. Validate: cannot ignore self
        if player_guid == ignored_guid {
            let msg = SmsgFriendStatus {
                result: FriendsResult::IgnoreSelf,
                friend_guid: ObjectGuid::empty(),
                friend_info: None,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // 2. Check if already ignored
        if let Some(state) = self.state.get(&player_guid) {
            if state.has_ignore(ignored_guid) {
                let msg = SmsgFriendStatus {
                    result: FriendsResult::IgnoreAlready,
                    friend_guid: ignored_guid,
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }

            // 3. Check ignore list limit
            if state.ignore_count() >= SOCIALMGR_IGNORE_LIMIT {
                let msg = SmsgFriendStatus {
                    result: FriendsResult::IgnoreFull,
                    friend_guid: ignored_guid,
                    friend_info: None,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        }

        // 4. Add to database
        let flags = SocialFlag::Ignored as u8;
        self.repository
            .add_or_update(player_guid.low(), ignored_guid.low(), flags).await
            .context("Failed to add ignore to database")?;

        // 5. Update state
        if let Some(mut state) = self.state.get_mut(&player_guid) {
            state.ignores.insert(ignored_guid, IgnoreEntry::new(ignored_guid, flags));
        }

        // 6. Send response to player
        let msg = SmsgFriendStatus {
            result: FriendsResult::IgnoreAdded,
            friend_guid: ignored_guid,
            friend_info: None,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        tracing::debug!(
            "Player {:?} added ignore {:?} ({})",
            player_guid,
            ignored_guid,
            ignored_name
        );

        Ok(())
    }

    /// Remove a player from the ignore list
    pub async fn remove_ignore(
        &self,
        player_guid: ObjectGuid,
        ignored_guid: ObjectGuid,
    ) -> Result<()> {
        // 1. Check if ignore exists
        let has_ignore = self.state.get(&player_guid)
            .map(|s| s.has_ignore(ignored_guid))
            .unwrap_or(false);

        if !has_ignore {
            let msg = SmsgFriendStatus {
                result: FriendsResult::IgnoreNotFound,
                friend_guid: ignored_guid,
                friend_info: None,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // 2. Remove from database
        self.repository
            .remove(player_guid.low(), ignored_guid.low()).await
            .context("Failed to remove ignore from database")?;

        // 3. Update state
        if let Some(mut state) = self.state.get_mut(&player_guid) {
            state.ignores.remove(&ignored_guid);
        }

        // 4. Send response to player
        let msg = SmsgFriendStatus {
            result: FriendsResult::IgnoreRemoved,
            friend_guid: ignored_guid,
            friend_info: None,
        };
        self.broadcast_mgr.send_msg_to_player(player_guid, msg);

        tracing::debug!("Player {:?} removed ignore {:?}", player_guid, ignored_guid);

        Ok(())
    }

    /// Get all ignored GUIDs for a player
    pub fn get_ignore_list(&self, player_guid: ObjectGuid) -> Vec<ObjectGuid> {
        self.state
            .get(&player_guid)
            .map(|s| s.ignores.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Send ignore list to player
    pub fn send_ignore_list(&self, player_guid: ObjectGuid) {
        let ignores = self.get_ignore_list(player_guid);
        let ignore_guids_low: Vec<u32> = ignores.iter().map(|g| g.low()).collect();

        let msg = SmsgIgnoreList {
            ignore_guids: &ignore_guids_low,
        };
        let packet = msg.to_world_packet();
        self.broadcast_mgr.send_to_player(player_guid, packet);
    }

    // ========== WHO COMMAND ==========

    /// Send WHO list to player
    pub fn send_who_list(&self, player_guid: ObjectGuid, request: WhoRequest) {
        // Get all online players
        let mut matching_players = Vec::new();
        let mut total_online = 0;

        // Iterate through all online players
        for entry in self.player_mgr.iter() {
            let player = entry.value();
            total_online += 1;

            // Apply filters
            // Level filter
            if player.level < request.min_level as u8 || player.level > request.max_level as u8 {
                continue;
            }

            // Race mask filter
            let race_bit = 1u32 << (player.race - 1);
            if request.race_mask != 0xFFFFFFFF && (request.race_mask & race_bit) == 0 {
                continue;
            }

            // Class mask filter
            let class_bit = 1u32 << (player.class - 1);
            if request.class_mask != 0xFFFFFFFF && (request.class_mask & class_bit) == 0 {
                continue;
            }

            // Zone filter
            if !request.zone_ids.is_empty() && !request.zone_ids.contains(&player.zone_id) {
                continue;
            }

            // Player name filter
            if !request.player_name.is_empty()
                && !player.name.to_lowercase().contains(&request.player_name.to_lowercase()) {
                continue;
            }

            // Search strings filter
            if !request.search_strings.is_empty() {
                let mut matches = false;
                for search_str in &request.search_strings {
                    if player.name.to_lowercase().contains(&search_str.to_lowercase()) {
                        matches = true;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            }

            // TODO: Guild name filter - requires guild system integration
            // For now, skip guild filter
            if !request.guild_name.is_empty() {
                continue; // Skip players when guild filter is requested but not implemented
            }

            // Add to results
            matching_players.push(WhoPlayerInfo {
                name: player.name.clone(),
                guild_name: String::new(), // TODO: Get from guild system
                level: player.level as u32,
                class: player.class as u32,
                race: player.race as u32,
                zone: player.zone_id,
            });

            // Limit results to 50 players
            if matching_players.len() >= 50 {
                break;
            }
        }

        let msg = SmsgWho {
            players: &matching_players,
            total_online,
        };
        let packet = msg.to_world_packet();
        self.broadcast_mgr.send_to_player(player_guid, packet);

        tracing::debug!(
            "WHO command from {:?}: {} matches, {} total online",
            player_guid,
            matching_players.len(),
            total_online
        );
    }

    // ========== STATUS BROADCASTING ==========

    /// Broadcast friend status change to all players who have this player friended
    pub fn broadcast_friend_status(
        &self,
        player_guid: ObjectGuid,
        result: FriendsResult,
        friend_info: Option<FriendInfo>,
    ) {
        // Find all players who have this player as a friend
        let mut listers = Vec::new();
        let state_count = self.state.len();

        for entry in self.state.iter() {
            if entry.value().has_friend(player_guid) {
                listers.push(*entry.key());
            }
        }

        tracing::info!(
            "[SOCIAL] broadcast_friend_status: player={:?} result={:?} online_players={} listers_found={}",
            player_guid,
            result,
            state_count,
            listers.len()
        );

        if listers.is_empty() {
            return;
        }

        let msg = SmsgFriendStatus {
            result,
            friend_guid: player_guid,
            friend_info,
        };

        for lister_guid in &listers {
            tracing::info!(
                "[SOCIAL] Sending SMSG_FRIEND_STATUS to {:?} about {:?} going {:?}",
                lister_guid,
                player_guid,
                result
            );
            self.broadcast_mgr
                .send_msg_to_player(*lister_guid, msg.clone())
                ;
        }
    }

    // ========== QUERY OPERATIONS ==========

    /// Check if a player has another player friended
    pub fn has_friend(&self, player_guid: ObjectGuid, friend_guid: ObjectGuid) -> bool {
        self.state
            .get(&player_guid)
            .map(|s| s.has_friend(friend_guid))
            .unwrap_or(false)
    }

    /// Check if a player has another player ignored
    pub fn has_ignore(&self, player_guid: ObjectGuid, ignored_guid: ObjectGuid) -> bool {
        self.state
            .get(&player_guid)
            .map(|s| s.has_ignore(ignored_guid))
            .unwrap_or(false)
    }

    /// Check if player_guid is ignored by by_player_guid
    pub fn is_ignored(&self, player_guid: ObjectGuid, by_player_guid: ObjectGuid) -> bool {
        self.has_ignore(by_player_guid, player_guid)
    }

    /// Get friend count for a player
    pub fn get_friend_count(&self, player_guid: ObjectGuid) -> usize {
        self.state
            .get(&player_guid)
            .map(|s| s.friend_count())
            .unwrap_or(0)
    }

    /// Get ignore count for a player
    pub fn get_ignore_count(&self, player_guid: ObjectGuid) -> usize {
        self.state
            .get(&player_guid)
            .map(|s| s.ignore_count())
            .unwrap_or(0)
    }

    // ========== WHISPER VALIDATION ==========

    /// Validate if a whisper can be sent from sender to target
    /// Returns Ok(()) if allowed, Err(reason) if blocked
    pub fn validate_whisper(
        &self,
        sender_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> WhisperValidationResult {
        // Check ignore list first
        if self.has_ignore(target_guid, sender_guid) {
            return Err(WhisperBlockReason::TargetIgnoresSender);
        }

        // TODO: Check cross-faction whispers based on config

        Ok(())
    }
}

// ========== SYSTEM TRAIT IMPLEMENTATION ==========

impl SocialSystem {
    pub async fn init(&self) -> Result<()> {

        Ok(())
    }

    pub fn update(&self, _diff: std::time::Duration) -> Result<()> {
        // No periodic updates needed
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.state.clear();

        Ok(())
    }

    pub fn on_player_login(&self, guid: ObjectGuid) -> Result<()> {
        // Load social data asynchronously and broadcast online status
        let system = Arc::new(self.clone_for_async());
        let player_mgr = Arc::clone(&self.player_mgr);

        tokio::spawn(async move {
            // 1. Load social data first
            if let Err(e) = system.load_player_social(guid).await {
                tracing::error!("Failed to load social data for player {:?}: {}", guid, e);
                return;
            }

            // 2. Build friend info for broadcast
            let friend_info = if let Some(player) = player_mgr.get_player(guid) {
                Some(FriendInfo {
                    status: FriendStatus::Online,
                    flags: SocialFlag::Friend as u8,
                    area: player.zone_id,
                    level: player.level as u32,
                    class: player.class as u32,
                })
            } else {
                None
            };

            // 3. Broadcast online status to all players who have this player friended
            tracing::info!("[SOCIAL] Player {:?} logged in, broadcasting online status", guid);
            system
                .broadcast_friend_status(guid, FriendsResult::Online, friend_info)
                ;
        });
        Ok(())
    }

    pub async fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        tracing::info!("[SOCIAL] Player {:?} logging out, broadcasting offline status", guid);

        // Broadcast offline status BEFORE unloading (in case we need any data)
        self.broadcast_friend_status(guid, FriendsResult::Offline, None)
            ;

        self.unload_player_social(guid);

        Ok(())
    }
}

impl SocialSystem {
    /// Helper to clone for async tasks
    fn clone_for_async(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            repository: Arc::clone(&self.repository),
            broadcast_mgr: Arc::clone(&self.broadcast_mgr),
            player_mgr: Arc::clone(&self.player_mgr),
        }
    }

    // ========== TEST HELPERS ==========

    #[cfg(test)]
    pub(crate) fn set_social_state(&self, player_guid: ObjectGuid, state: SocialState) {
        self.state.insert(player_guid, state);
    }
}
