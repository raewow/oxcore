//! Guild system - guild creation/management/members/ranks (excluding bank)

use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::shared::database::characters::repositories::GuildRepositoryTrait;
use crate::shared::messages::guild::{
    smsg_guild_event_from_params, smsg_guild_query_response_from_cached,
    smsg_guild_roster_from_cached, SmsgGuildCommandResult, SmsgGuildDecline, SmsgGuildEvent,
    SmsgGuildInvite,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::world::game::player::PlayerManager;
use crate::world::game::ItemManager;

use super::types::*;
use super::utils::*;

pub struct GuildSystem {
    /// Guild data indexed by guild ID
    guilds: DashMap<u32, GuildData>,

    /// Player membership indexed by player GUID
    members: DashMap<ObjectGuid, PlayerGuildState>,

    /// Pending invites: invitee_guid -> (inviter_guid, guild_id, guild_name)
    pending_invites: DashMap<ObjectGuid, (ObjectGuid, u32, String)>,

    /// Next guild ID for auto-assignment
    next_guild_id: AtomicU32,

    repository: Arc<dyn GuildRepositoryTrait>,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    player_mgr: Arc<PlayerManager>,
    item_mgr: Arc<ItemManager>,
}

impl GuildSystem {
    pub fn new(
        repository: Arc<dyn GuildRepositoryTrait>,
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
        item_mgr: Arc<ItemManager>,
    ) -> Self {
        Self {
            guilds: DashMap::new(),
            members: DashMap::new(),
            pending_invites: DashMap::new(),
            next_guild_id: AtomicU32::new(1),
            repository,
            broadcast_mgr,
            player_mgr,
            item_mgr,
        }
    }

    /// Initialize system (load next guild ID from database)
    pub async fn initialize(&self) -> Result<()> {
        let max_id = self.repository.get_max_guild_id().await?;
        let next_id = max_id.unwrap_or(0) + 1;
        self.next_guild_id.store(next_id, Ordering::Relaxed);

        Ok(())
    }

    /// Get guild data by ID
    pub fn get_guild(&self, guild_id: u32) -> Option<GuildData> {
        self.guilds.get(&guild_id).map(|r| r.clone())
    }

    /// Get player's guild membership
    pub fn get_player_guild(&self, player_guid: ObjectGuid) -> Option<PlayerGuildState> {
        self.members.get(&player_guid).map(|r| r.clone())
    }

    /// Check if player is in a guild
    pub fn is_in_guild(&self, player_guid: ObjectGuid) -> bool {
        self.members
            .get(&player_guid)
            .map(|state| state.guild_id.is_some())
            .unwrap_or(false)
    }

    /// Get pending guild invite for a player
    pub fn get_pending_invite(&self, player_guid: ObjectGuid) -> Option<(ObjectGuid, u32, String)> {
        self.pending_invites
            .get(&player_guid)
            .map(|r| r.value().clone())
    }

    /// Remove and return pending guild invite for a player
    pub fn remove_pending_invite(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<(ObjectGuid, u32, String)> {
        self.pending_invites.remove(&player_guid).map(|(_, v)| v)
    }

    /// Clear all pending invites for a player
    pub fn clear_player_invites(&self, player_guid: ObjectGuid) {
        self.pending_invites.remove(&player_guid);
    }

    /// Create guild from petition (charter system)
    pub async fn create_guild_from_petition(
        &self,
        leader_guid: ObjectGuid,
        leader_name: String,
        guild_name: String,
    ) -> Result<()> {
        tracing::info!(
            "Guild create requested by {} for guild '{}'",
            leader_name,
            guild_name
        );

        // 1. Validate name length
        if guild_name.len() > GUILD_NAME_MAX_LENGTH {
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_CREATE_S
                target_name: "",
                error_code: ERR_GUILD_NAME_INVALID,
            };
            self.broadcast_mgr.send_msg_to_player(leader_guid, msg);
            return Ok(());
        }

        // 2. Check if player already in guild
        if self.is_in_guild(leader_guid) {
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_CREATE_S
                target_name: "",
                error_code: ERR_ALREADY_IN_GUILD_S,
            };
            self.broadcast_mgr.send_msg_to_player(leader_guid, msg);
            return Ok(());
        }

        // 3. Check if name exists
        if self.repository.exists_by_name(&guild_name).await? {
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_CREATE_S
                target_name: "",
                error_code: ERR_GUILD_NAME_EXISTS,
            };
            self.broadcast_mgr.send_msg_to_player(leader_guid, msg);
            return Ok(());
        }

        // 4. Allocate guild ID
        let guild_id = self.next_guild_id.fetch_add(1, Ordering::Relaxed);

        // 5. Build guild
        let guild = Guild {
            id: guild_id,
            name: guild_name.clone(),
            leader_guid,
            leader_name: leader_name.clone(),
            emblem: create_default_emblem(),
            info: String::new(),
            motd: String::new(),
            create_date: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };

        // 6. Build guild member for leader
        let leader_member = GuildMember {
            guid: leader_guid,
            name: leader_name.clone(),
            rank: 0,
            public_note: String::new(),
            officer_note: String::new(),
            level: 1,      // TODO: Load from character data
            class: 0,      // TODO: Load from character data
            zone: 0,       // TODO: Load from character data
            account_id: 0, // TODO: Load from character data
            logout_time: 0,
        };

        // 7. Create ranks
        let ranks = create_default_ranks();

        // 8. Build guild data
        let mut guild_data = GuildData {
            guild_id,
            info: guild.clone(),
            members: HashMap::new(),
            ranks: ranks.clone(),
        };
        guild_data
            .members
            .insert(leader_guid, leader_member.clone());

        // 9. Convert to database types
        use crate::shared::database::characters::models::guild::{
            GuildBankTabRow, GuildMemberRow, GuildRankRow, GuildRow,
        };

        let guild_row = GuildRow {
            guild_id,
            name: guild.name.clone(),
            leader_guid: leader_guid.low(),
            emblem_style: guild.emblem.style as i32,
            emblem_color: guild.emblem.color as i32,
            border_style: guild.emblem.border_style as i32,
            border_color: guild.emblem.border_color as i32,
            background_color: guild.emblem.background_color as i32,
            info: guild.info.clone(),
            motd: guild.motd.clone(),
            create_date: guild.create_date,
            bank_money: 0,
        };

        let rank_rows: Vec<GuildRankRow> = ranks
            .iter()
            .map(|r| GuildRankRow {
                guild_id,
                id: r.id as u32,
                name: r.name.clone(),
                rights: r.rights,
            })
            .collect();

        let member_row = GuildMemberRow {
            guild_id,
            guid: leader_guid.low(),
            rank: 0,
            player_note: String::new(),
            officer_note: String::new(),
        };

        // 10. Save to database (transactional)
        self.repository
            .create(
                &guild_row,
                &rank_rows,
                &member_row,
                &[], // No bank tabs for now
            )
            .await
            .context("Failed to create guild in database")?;

        // 11. Add to cache
        self.guilds.insert(guild_id, guild_data);

        // 12. Store player membership
        self.members.insert(
            leader_guid,
            PlayerGuildState {
                guild_id: Some(guild_id),
                rank_id: 0,
            },
        );

        // 13. Send success message to leader
        // Use empty string since target_name needs to be 'static for send_msg_to_player
        let msg = SmsgGuildCommandResult {
            command: 0, // TODO: GUILD_CREATE_S
            target_name: "",
            error_code: ERR_GUILD_SUCCESS,
        };
        self.broadcast_mgr.send_msg_to_player(leader_guid, msg);

        // 14. Send guild query response so client knows guild name/ranks/emblem
        self.query_guild(leader_guid, guild_id)?;

        // 15. Send guild roster to the creator
        self.send_guild_roster_to_player(leader_guid, guild_id)?;

        tracing::info!("Guild '{}' created with ID {}", guild_name, guild_id);

        Ok(())
    }

    /// Send guild invite to player
    pub async fn invite_player(
        &self,
        inviter_guid: ObjectGuid,
        invitee_name: String,
        guild_name: String,
    ) -> Result<()> {
        // Get inviter's guild membership
        let inviter_state = self
            .get_player_guild(inviter_guid)
            .ok_or_else(|| anyhow!("Inviter not in guild"))?;
        let guild_id = inviter_state
            .guild_id
            .ok_or_else(|| anyhow!("Inviter not in guild"))?;

        // Find invitee by name (online players only)
        let invitee_guid = self
            .player_mgr
            .find_player_by_name(&invitee_name)
            .ok_or_else(|| anyhow!("Player '{}' not found or offline", invitee_name))?;

        // Check invitee not already in guild
        if self.is_in_guild(invitee_guid) {
            return Err(anyhow!("Player already in guild"));
        }

        // Get inviter name for the invite packet
        let inviter_name = self
            .player_mgr
            .get_player_name(inviter_guid)
            .unwrap_or_else(|| "Unknown".to_string());

        // Store pending invite
        self.pending_invites
            .insert(invitee_guid, (inviter_guid, guild_id, guild_name.clone()));

        // Send invite packet to invitee
        let invite_packet = SmsgGuildInvite {
            inviter_name: &inviter_name,
            guild_name: &guild_name,
        };
        self.broadcast_mgr
            .send_msg_to_player(invitee_guid, invite_packet);

        tracing::info!(
            "Guild invite sent by {} to {} for guild '{}'",
            inviter_name,
            invitee_name,
            guild_name
        );

        Ok(())
    }

    /// Join a guild after accepting invite
    pub async fn join_guild(
        &self,
        guild_id: u32,
        player_guid: ObjectGuid,
        player_name: String,
    ) -> Result<()> {
        // Validate guild exists
        let guild = self
            .get_guild(guild_id)
            .ok_or_else(|| anyhow!("Guild not found"))?;

        // Add to database (rank 5 is default lowest rank)
        let member_row = crate::shared::database::characters::models::guild::GuildMemberRow {
            guild_id,
            guid: player_guid.counter(),
            rank: 5, // Default rank (lowest)
            player_note: String::new(),
            officer_note: String::new(),
        };
        self.repository.add_member(&member_row).await?;

        // Update cache
        let state = PlayerGuildState {
            guild_id: Some(guild_id),
            rank_id: 5,
        };
        self.members.insert(player_guid, state);

        // Send welcome event to guild (event type 3 = player joined)
        let event_packet = smsg_guild_event_from_params(
            3, // GE_JOINED
            &[&player_name],
        );

        // Broadcast to all guild members
        let guild_data = self.guilds.get(&guild_id);
        if let Some(ref guild_ref) = guild_data {
            for member_guid in guild_ref.members.keys() {
                self.broadcast_mgr
                    .send_msg_to_player(*member_guid, event_packet.clone());
            }
        }

        // Send roster to all members
        self.send_guild_roster(guild_id)?;

        tracing::info!("Player {} joined guild '{}'", player_name, guild.info.name);

        Ok(())
    }

    /// Remove a member from guild (by officer/leader)
    pub async fn remove_member(
        &self,
        remover_guid: ObjectGuid,
        target_guid: ObjectGuid,
        target_name: String,
    ) -> Result<()> {
        // Get remover's guild and rank
        let remover_state = self
            .get_player_guild(remover_guid)
            .ok_or_else(|| anyhow!("Remover not in guild"))?;
        let guild_id = remover_state
            .guild_id
            .ok_or_else(|| anyhow!("Remover not in guild"))?;

        // Check permissions (rank 2 or higher = officer+)
        if remover_state.rank_id > 2 {
            return Err(anyhow!("Insufficient permissions to remove members"));
        }

        // Get target's guild state
        let target_state = self
            .get_player_guild(target_guid)
            .ok_or_else(|| anyhow!("Target not in guild"))?;

        // Validate same guild
        if target_state.guild_id != Some(guild_id) {
            return Err(anyhow!("Target not in your guild"));
        }

        // Can't remove guild leader (rank 0)
        if target_state.rank_id == 0 {
            return Err(anyhow!("Cannot remove guild leader"));
        }

        // Remove from database
        self.repository
            .remove_member(guild_id, target_guid.counter())
            .await?;

        // Update cache
        self.members.remove(&target_guid);

        // Notify guild (event type 4 = player removed)
        let event = smsg_guild_event_from_params(
            4, // GE_REMOVED
            &[&target_name],
        );

        // Broadcast to all guild members
        let guild_data = self.guilds.get(&guild_id);
        if let Some(ref guild_ref) = guild_data {
            for member_guid in guild_ref.members.keys() {
                self.broadcast_mgr
                    .send_msg_to_player(*member_guid, event.clone());
            }
        }

        tracing::info!(
            "Player {} removed from guild by {:?}",
            target_name,
            remover_guid
        );

        Ok(())
    }

    /// Disband a guild (leader only)
    pub async fn disband_guild(&self, disbander_guid: ObjectGuid) -> Result<()> {
        // Get disbander's guild
        let state = self
            .get_player_guild(disbander_guid)
            .ok_or_else(|| anyhow!("Not in guild"))?;
        let guild_id = state.guild_id.ok_or_else(|| anyhow!("Not in guild"))?;

        // Verify is guild leader (rank 0)
        if state.rank_id != 0 {
            return Err(anyhow!("Only guild leader can disband guild"));
        }

        // Get all member GUIDs for notifications
        let member_guids: Vec<ObjectGuid> = self
            .members
            .iter()
            .filter(|entry| entry.value().guild_id == Some(guild_id))
            .map(|entry| *entry.key())
            .collect();

        // Delete from database
        self.repository.delete(guild_id).await?;

        // Remove from cache
        self.guilds.remove(&guild_id);
        for guid in &member_guids {
            self.members.remove(guid);
        }

        // Notify all members (event type 2 = disbanded)
        let disband_event = smsg_guild_event_from_params(
            2, // GE_DISBANDED
            &[""],
        );
        for guid in member_guids {
            self.broadcast_mgr
                .send_msg_to_player(guid, disband_event.clone());
        }

        tracing::info!("Guild {} disbanded by {:?}", guild_id, disbander_guid);

        Ok(())
    }

    /// Accept guild invite
    pub async fn accept_invite(
        &self,
        _invitee_guid: ObjectGuid,
        _invitee_name: String,
    ) -> Result<()> {
        // TODO: Implement invite tracking to know which guild to join
        // For now, this is not implemented
        tracing::warn!("Guild invite acceptance not implemented - missing invite tracking");
        Ok(())
    }

    /// Decline guild invite
    pub async fn decline_invite(
        &self,
        inviter_guid: ObjectGuid,
        invitee_name: String,
    ) -> Result<()> {
        // Send decline notification
        // Use empty string since player_name needs to be 'static for send_msg_to_player
        let msg = SmsgGuildDecline { player_name: "" };
        self.broadcast_mgr.send_msg_to_player(inviter_guid, msg);

        tracing::debug!(
            "Player {} declined invite from {:?}",
            invitee_name,
            inviter_guid
        );

        Ok(())
    }

    /// Build SMSG_GUILD_ROSTER packet from guild data
    fn build_guild_roster_packet(&self, guild_data: &GuildData) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_ROSTER);

        packet.write_u32(guild_data.members.len() as u32);
        packet.write_string(&guild_data.info.motd);
        packet.write_string(&guild_data.info.info);

        // Rank rights
        packet.write_u32(guild_data.ranks.len() as u32);
        for rank in &guild_data.ranks {
            packet.write_u32(rank.rights);
        }

        // Members
        for member in guild_data.members.values() {
            packet.write_guid_raw(member.guid.raw());

            // Online = member exists in player_mgr
            let is_online = self.player_mgr.get_player(member.guid).is_some();
            let status: u8 = if is_online { GRF_ONLINE } else { 0 };
            packet.write_u8(status);

            packet.write_string(&member.name);
            packet.write_u32(member.rank as u32);
            packet.write_u8(member.level);
            packet.write_u8(member.class);
            packet.write_u32(member.zone);

            if !is_online {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let days_since_logout = if member.logout_time > 0 {
                    (now - member.logout_time) as f32 / 86400.0
                } else {
                    0.0
                };
                packet.write_f32(days_since_logout);
            }

            packet.write_string(&member.public_note);
            packet.write_string(&member.officer_note);
        }

        packet
    }

    /// Send guild roster to a specific player
    pub fn send_guild_roster_to_player(
        &self,
        player_guid: ObjectGuid,
        guild_id: u32,
    ) -> Result<()> {
        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        let packet = self.build_guild_roster_packet(&guild_data);
        self.broadcast_mgr.send_to_player(player_guid, packet);

        Ok(())
    }

    /// Send guild roster to all members
    pub fn send_guild_roster(&self, guild_id: u32) -> Result<()> {
        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        let packet = self.build_guild_roster_packet(&guild_data);
        for member_guid in guild_data.members.keys() {
            self.broadcast_mgr
                .send_to_player(*member_guid, packet.clone());
        }

        Ok(())
    }

    /// Query guild info — sends SMSG_GUILD_QUERY_RESPONSE
    pub fn query_guild(&self, requester_guid: ObjectGuid, guild_id: u32) -> Result<()> {
        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        // Build SMSG_GUILD_QUERY_RESPONSE inline (1.12 format):
        // u32 guild_id, CString name, CString[10] rank_names, 5x u32 emblem
        let mut packet = WorldPacket::new(Opcode::SMSG_GUILD_QUERY_RESPONSE);
        packet.write_u32(guild_data.guild_id);
        packet.write_string(&guild_data.info.name);

        // Write 10 rank names (pad with empty strings)
        for i in 0..10 {
            if i < guild_data.ranks.len() {
                packet.write_string(&guild_data.ranks[i].name);
            } else {
                packet.write_string("");
            }
        }

        // Emblem fields
        packet.write_u32(guild_data.info.emblem.style as u32);
        packet.write_u32(guild_data.info.emblem.color as u32);
        packet.write_u32(guild_data.info.emblem.border_style as u32);
        packet.write_u32(guild_data.info.emblem.border_color as u32);
        packet.write_u32(guild_data.info.emblem.background_color as u32);

        self.broadcast_mgr.send_to_player(requester_guid, packet);

        Ok(())
    }

    /// Leave guild
    pub async fn leave_guild(&self, player_guid: ObjectGuid, _player_name: String) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };

        // TODO: implement get_player_name on PlayerManager
        let player_name = "Unknown".to_string();

        // Check if guild master
        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        if guild_data.info.leader_guid == player_guid {
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_QUIT_S
                target_name: "",
                error_code: ERR_GUILD_PERMISSIONS,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Remove member from database
        self.repository
            .remove_member(guild_id, player_guid.low())
            .await
            .context("Failed to remove guild member")?;

        // Update guild cache
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            guild.members.remove(&player_guid);
            drop(guild);
        }

        // Update player state
        self.members.insert(
            player_guid,
            PlayerGuildState {
                guild_id: None,
                rank_id: 0,
            },
        );

        // Send event
        self.log_guild_event(guild_id, 3, player_guid, &player_name)
            .await?;

        // Send roster to all remaining members
        if let Some(guild) = self.guilds.get(&guild_id) {
            self.send_guild_roster(guild_id)?;
        }

        tracing::info!(
            "Player {} left guild '{}'",
            player_name,
            guild_data.info.name
        );

        Ok(())
    }

    /// Promote guild member
    pub async fn promote_member(
        &self,
        promoter_guid: ObjectGuid,
        target_name: String,
    ) -> Result<()> {
        // 1. Find target player (TODO: implement player name lookup)
        // For now, skip this
        tracing::warn!("promote_member not fully implemented - player name lookup missing");
        return Ok(());

        // 2. Get promoter's guild
        // let Some(promoter_state) = self.get_player_guild(promoter_guid) else {
        //     tracing::warn!("Promoter {:?} not in a guild", promoter_guid);
        //     return Ok(());
        // };

        // let Some(guild_id) = promoter_state.guild_id else {
        //     return Ok(());
        // };

        // let Some(guild_data) = self.guilds.get(&guild_id) else {
        //     return Ok(());
        // };

        // Rest of the function is unreachable due to early return above
    }

    /// Demote guild member
    pub async fn demote_member(&self, demoter_guid: ObjectGuid, target_name: String) -> Result<()> {
        // TODO: implement player name lookup
        tracing::warn!("demote_member not fully implemented - player name lookup missing");
        Ok(())
    }

    /// Change guild leader
    pub async fn change_leader(
        &self,
        current_leader_guid: ObjectGuid,
        new_leader_guid: ObjectGuid,
    ) -> Result<()> {
        // 1. Get new leader name (TODO: implement get_player_name)
        let new_leader_name = "Unknown".to_string();

        // 2. Get current leader's guild
        let Some(current_state) = self.get_player_guild(current_leader_guid) else {
            tracing::warn!("Current leader {:?} not in a guild", current_leader_guid);
            return Ok(());
        };

        let Some(guild_id) = current_state.guild_id else {
            return Ok(());
        };

        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        // 3. Check if current leader is attempting to transfer leadership
        if guild_data.info.leader_guid != current_leader_guid {
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_LEADER_S
                target_name: "",
                error_code: ERR_GUILD_PERMISSIONS,
            };
            self.broadcast_mgr
                .send_msg_to_player(current_leader_guid, msg);
            return Ok(());
        }

        // 4. Update guild info in database
        self.repository
            .update_leader(guild_id, current_leader_guid.low(), new_leader_guid.low())
            .await
            .context("Failed to update guild leader")?;

        // 5. Update cache
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            guild.info.leader_guid = new_leader_guid;
            drop(guild);
        }

        // Update player state for old leader
        if let Some(mut old_leader_state) = self.members.get_mut(&current_leader_guid) {
            old_leader_state.rank_id = 1; // Demote to member
        }

        // Update player state for new leader
        self.members.insert(
            new_leader_guid,
            PlayerGuildState {
                guild_id: Some(guild_id),
                rank_id: 0,
            },
        );

        // Send event
        self.log_guild_event(guild_id, 6, new_leader_guid, &new_leader_name)
            .await?;

        // Send roster
        self.send_guild_roster(guild_id)?;

        tracing::info!(
            "Guild '{}' leader changed to '{}'",
            guild_data.info.name,
            new_leader_name
        );

        Ok(())
    }

    /// Set guild MOTD
    pub async fn set_motd(&self, player_guid: ObjectGuid, motd: String) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };

        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            guild.info.motd = motd.clone();
            drop(guild);
        }

        // Update database
        self.repository
            .update_motd(guild_id, &motd)
            .await
            .context("Failed to set guild MOTD")?;

        // Send MOTD to all members
        self.send_guild_roster(guild_id)?;

        Ok(())
    }

    /// Set guild info
    pub async fn set_info(&self, player_guid: ObjectGuid, info: String) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };

        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            guild.info.info = info.clone();
            drop(guild);
        }

        // Update database
        self.repository
            .update_info(guild_id, &info)
            .await
            .context("Failed to set guild info")?;

        // Send event to all members
        self.log_guild_event(guild_id, 0, player_guid, &info)
            .await?;

        // Send roster
        self.send_guild_roster(guild_id)?;

        Ok(())
    }

    /// Set guild rank
    pub async fn set_rank(
        &self,
        player_guid: ObjectGuid,
        rank_id: u8,
        rank_name: String,
    ) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };
        drop(state); // Release borrow

        if let Some(guild) = self.guilds.get(&guild_id) {
            // Check permissions - TODO
        }

        // Update rank in database (with default rights)
        self.repository
            .update_rank(guild_id, rank_id as u32, &rank_name, 0)
            .await
            .context("Failed to set guild rank name")?;

        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            if let Some(rank) = guild.ranks.get_mut(rank_id as usize) {
                rank.name = rank_name;
            }
        }

        // Send roster to all members
        self.send_guild_roster(guild_id)?;

        Ok(())
    }

    /// Add a new rank
    pub async fn create_rank(&self, player_guid: ObjectGuid, rank_name: String) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };

        // Check permissions - TODO
        let current_rank_count = if let Some(guild) = self.guilds.get(&guild_id) {
            guild.ranks.len()
        } else {
            10 // Use lowest available
        };

        if current_rank_count >= GUILD_RANKS_MAX_COUNT {
            // Use empty string since target_name needs to be 'static for send_msg_to_player
            let msg = SmsgGuildCommandResult {
                command: 0, // TODO: GUILD_ADD_RANK_S
                target_name: "",
                error_code: ERR_GUILD_PERMISSIONS,
            };
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);
            return Ok(());
        }

        // Add rank to database
        use crate::shared::database::characters::models::guild::GuildRankRow;
        let rank_row = GuildRankRow {
            guild_id,
            id: current_rank_count as u32,
            name: rank_name.clone(),
            rights: GRIGHT_OFFCHATLISTEN,
        };
        self.repository
            .create_rank(&rank_row)
            .await
            .context("Failed to create guild rank")?;

        // Add to cache
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            guild.ranks.push(GuildRank {
                id: current_rank_count as u8,
                name: rank_name.clone(),
                rights: GRIGHT_OFFCHATLISTEN,
            });
        }

        // Send roster
        self.send_guild_roster(guild_id)?;

        tracing::info!(
            "Guild {} added new rank '{}' ({}/{})",
            guild_id,
            rank_name,
            current_rank_count,
            GUILD_RANKS_MAX_COUNT
        );

        Ok(())
    }

    /// Delete a rank
    pub async fn delete_rank(&self, player_guid: ObjectGuid, rank_id: u8) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };

        if let Some(guild) = self.guilds.get(&guild_id) {
            // Check permissions - TODO
            let rank_count = guild.ranks.len();

            if rank_id as usize >= rank_count - 1 {
                // Don't allow deleting last rank
                let msg = SmsgGuildCommandResult {
                    command: 0, // TODO: GUILD_DEL_RANK_S
                    target_name: "",
                    error_code: ERR_GUILD_PERMISSIONS,
                };
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);
                return Ok(());
            }
        }

        // Delete rank from database
        self.repository
            .delete_rank(guild_id, rank_id as u32)
            .await
            .context("Failed to delete guild rank")?;

        // Remove rank from cache
        if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
            let mut ranks: Vec<GuildRank> = Vec::new();
            for rank in guild.ranks.iter() {
                if rank.id != rank_id {
                    let mut new_rank = rank.clone();
                    if rank.id > rank_id {
                        new_rank.id -= 1;
                    }
                    ranks.push(new_rank);
                }
            }

            guild.ranks = ranks;

            // Update player states (shift ranks down)
            let member_guids: Vec<ObjectGuid> = guild.members.keys().copied().collect();
            drop(guild);

            for member_guid in member_guids {
                if let Some(mut player_state) = self.members.get_mut(&member_guid) {
                    if player_state.guild_id == Some(guild_id) && player_state.rank_id >= rank_id {
                        player_state.rank_id = player_state.rank_id.saturating_sub(1);
                    }
                }
            }
        }

        // Send roster
        self.send_guild_roster(guild_id)?;

        tracing::info!("Guild {} deleted rank {}", guild_id, rank_id);

        Ok(())
    }

    /// Set public note for member
    pub async fn set_public_note(
        &self,
        player_guid: ObjectGuid,
        target_name: String,
        note: String,
    ) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let guild_id = state
            .guild_id
            .ok_or_else(|| anyhow!("Guild ID not found"))?;
        drop(state); // Release borrow

        // Find target GUID first
        let target_guid_opt = if let Some(guild) = self.guilds.get(&guild_id) {
            // Check permissions - TODO

            // Find target member by name
            guild
                .members
                .iter()
                .find(|(_, member)| member.name == target_name)
                .map(|(guid, _)| *guid)
        } else {
            None
        };

        if let Some(target_guid) = target_guid_opt {
            // Update cache
            if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
                if let Some(mut member) = guild.members.get_mut(&target_guid) {
                    member.public_note = note.clone();
                }
            }

            // Update database
            self.repository
                .update_member_public_note(guild_id, target_guid.low(), &note)
                .await
                .context("Failed to set public note")?;
        }

        // Send roster
        self.send_guild_roster(guild_id)?;

        Ok(())
    }

    /// Set officer note for member
    pub async fn set_officer_note(
        &self,
        player_guid: ObjectGuid,
        target_name: String,
        note: String,
    ) -> Result<()> {
        let Some(state) = self.members.get(&player_guid) else {
            tracing::warn!("Player {:?} not in a guild", player_guid);
            return Ok(());
        };

        let Some(guild_id) = state.guild_id else {
            return Ok(());
        };
        drop(state); // Release borrow

        // Find target GUID first
        let target_guid_opt = if let Some(guild) = self.guilds.get(&guild_id) {
            // Check permissions - TODO

            // Find target member by name
            guild
                .members
                .iter()
                .find(|(_, member)| member.name == target_name)
                .map(|(guid, _)| *guid)
        } else {
            None
        };

        if let Some(target_guid) = target_guid_opt {
            // Update cache
            if let Some(mut guild) = self.guilds.get_mut(&guild_id) {
                if let Some(mut member) = guild.members.get_mut(&target_guid) {
                    member.officer_note = note.clone();
                }
            }

            // Update database
            self.repository
                .update_member_officer_note(guild_id, target_guid.low(), &note)
                .await
                .context("Failed to set officer note")?;
        }

        // Send roster
        self.send_guild_roster(guild_id)?;

        Ok(())
    }

    /// Get guild by name (for GM commands)
    pub fn get_guild_by_name(&self, name: &str) -> Option<GuildData> {
        self.guilds
            .iter()
            .find(|entry| entry.info.name == name)
            .map(|entry| entry.clone())
    }

    /// Check if guild name exists (for GM commands)
    pub fn has_guild_name(&self, name: &str) -> bool {
        self.guilds.iter().any(|entry| entry.info.name == name)
    }

    /// Add member directly to guild (GM command - bypasses invite system)
    pub async fn add_member_directly(
        &self,
        player_guid: ObjectGuid,
        player_name: String,
        guild_id: u32,
        rank_id: u8,
    ) -> Result<()> {
        // Check if player already in guild
        if self.is_in_guild(player_guid) {
            return Err(anyhow!("Player already in a guild"));
        }

        // Validate guild exists
        let guild = self
            .get_guild(guild_id)
            .ok_or_else(|| anyhow!("Guild not found"))?;

        // Validate rank exists
        if rank_id as usize >= guild.ranks.len() {
            return Err(anyhow!("Invalid rank ID"));
        }

        // Add to database
        let member_row = crate::shared::database::characters::models::guild::GuildMemberRow {
            guild_id,
            guid: player_guid.counter(),
            rank: rank_id,
            player_note: String::new(),
            officer_note: String::new(),
        };
        self.repository.add_member(&member_row).await?;

        // Update cache
        let state = PlayerGuildState {
            guild_id: Some(guild_id),
            rank_id,
        };
        self.members.insert(player_guid, state);

        // Update guild member list in cache
        if let Some(mut guild_data) = self.guilds.get_mut(&guild_id) {
            guild_data.members.insert(
                player_guid,
                GuildMember {
                    guid: player_guid,
                    name: player_name.clone(),
                    rank: rank_id,
                    public_note: String::new(),
                    officer_note: String::new(),
                    level: 1,
                    class: 0,
                    zone: 0,
                    account_id: 0,
                    logout_time: 0,
                },
            );
        }

        // Send guild event (player joined)
        self.log_guild_event(guild_id, 3, player_guid, &player_name)
            .await?;

        // Send roster to all members
        self.send_guild_roster(guild_id)?;

        tracing::info!(
            "GM added {} to guild {} (rank {})",
            player_name,
            guild.info.name,
            rank_id
        );

        Ok(())
    }

    /// Set member rank directly (GM command - bypasses permission checks)
    pub async fn set_member_rank_directly(
        &self,
        player_guid: ObjectGuid,
        rank_id: u8,
    ) -> Result<()> {
        // Get player's guild
        let state = self
            .get_player_guild(player_guid)
            .ok_or_else(|| anyhow!("Player not in a guild"))?;
        let guild_id = state
            .guild_id
            .ok_or_else(|| anyhow!("Player not in a guild"))?;

        // Validate rank exists
        let guild = self.get_guild(guild_id).unwrap();
        if rank_id as usize >= guild.ranks.len() {
            return Err(anyhow!("Invalid rank ID"));
        }

        // Update database
        self.repository
            .update_member_rank(guild_id, player_guid.counter(), rank_id)
            .await?;

        // Update cache
        if let Some(mut guild_data) = self.guilds.get_mut(&guild_id) {
            if let Some(member) = guild_data.members.get_mut(&player_guid) {
                member.rank = rank_id;
            }
        }

        // Update player state
        self.members.insert(
            player_guid,
            PlayerGuildState {
                guild_id: Some(guild_id),
                rank_id,
            },
        );

        // Get player name for event
        let player_name = if let Some(member) = guild.members.get(&player_guid) {
            member.name.clone()
        } else {
            "Unknown".to_string()
        };

        // Send guild event (promotion)
        self.log_guild_event(guild_id, 0, player_guid, &player_name)
            .await?;

        // Send roster to all members
        self.send_guild_roster(guild_id)?;

        tracing::info!(
            "GM set player {} to rank {} in guild {}",
            player_name,
            rank_id,
            guild.info.name
        );

        Ok(())
    }

    /// Rename guild (GM command)
    pub async fn rename_guild(&self, guild_id: u32, new_name: String) -> Result<()> {
        // Validate new name length
        if new_name.len() > 24 {
            return Err(anyhow!("Guild name too long (max 24 characters)"));
        }

        // Check if new name already exists
        if self.has_guild_name(&new_name) {
            return Err(anyhow!("Guild name already exists"));
        }

        // Update database
        self.repository
            .update_guild_name(guild_id, &new_name)
            .await?;

        // Update cache
        if let Some(mut guild_data) = self.guilds.get_mut(&guild_id) {
            let old_name = guild_data.info.name.clone();
            guild_data.info.name = new_name.clone();

            // Send guild event (tabard change - indicates guild modification)
            self.log_guild_event(guild_id, 9, guild_data.info.leader_guid, &new_name)
                .await?;

            tracing::info!("GM renamed guild '{}' to '{}'", old_name, new_name);
        } else {
            return Err(anyhow!("Guild not found"));
        }

        Ok(())
    }

    /// Log guild event to all guild members
    async fn log_guild_event(
        &self,
        guild_id: u32,
        event_type: u8,
        player_guid: ObjectGuid,
        param: &str,
    ) -> Result<()> {
        let Some(guild_data) = self.guilds.get(&guild_id) else {
            return Ok(());
        };

        let event_packet = smsg_guild_event_from_params(event_type, &[param]);

        for member_guid in guild_data.members.keys() {
            self.broadcast_mgr
                .send_msg_to_player(*member_guid, event_packet.clone());
        }

        tracing::info!(
            "Guild event: {} (type={}, param={}, guid={:?})",
            param,
            event_type,
            param,
            player_guid
        );

        Ok(())
    }
}

impl GuildSystem {
    pub async fn init(&self) -> Result<()> {
        self.initialize().await?;
        Ok(())
    }

    pub fn update(&self, _diff: std::time::Duration) -> Result<()> {
        // No periodic updates needed
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.guilds.clear();
        self.members.clear();

        Ok(())
    }

    pub fn on_player_login(&self, _guid: ObjectGuid) -> Result<()> {
        Ok(())
    }

    pub fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        // Clear pending invites
        self.clear_player_invites(guid);

        // Remove from member list
        if let Some(mut member) = self.members.get_mut(&guid) {
            member.guild_id = None;
            member.rank_id = 0;
            drop(member);
        }

        Ok(())
    }
}
