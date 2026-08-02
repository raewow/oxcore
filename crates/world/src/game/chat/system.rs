//! Chat System - channels, messages, flood protection

use anyhow::Result;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::game::player::PlayerManager;
use crate::game::BroadcastManager;
use oxcore_shared::game::chat::{ChatMsg, ChatTag, Language, Team};
use oxcore_shared::messages::channel::{ChannelMemberInfo, *};
use oxcore_shared::messages::chat::*;
use oxcore_shared::messages::update::DEFAULT_REALM_ID;
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::{ObjectGuid, Position, WorldPacket};

use super::commands::{self, CommandRegistry};
use super::log::{ChatLogEntry, ChatLogger};
use super::types::*;
use super::validation;

pub struct ChatSystem {
    channels: DashMap<Team, DashMap<String, ChannelData>>,
    player_channels: DashMap<ObjectGuid, HashSet<String>>,
    flood_tracker: DashMap<ObjectGuid, FloodState>,
    flood_config: FloodConfig,
    next_channel_id: AtomicU32,
    broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
    player_mgr: Arc<PlayerManager>,
    command_registry: CommandRegistry,
    chat_logger: Option<Arc<ChatLogger>>,
}

impl ChatSystem {
    pub fn new(
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
    ) -> Self {
        // Initialize command registry
        let mut command_registry = CommandRegistry::new();
        commands::handlers::register_all_commands(&mut command_registry);

        Self {
            channels: DashMap::new(),
            player_channels: DashMap::new(),
            flood_tracker: DashMap::new(),
            flood_config: FloodConfig::default(),
            next_channel_id: AtomicU32::new(100),
            broadcast_mgr,
            player_mgr,
            command_registry,
            chat_logger: None,
        }
    }

    /// Attach a persistent chat logger. Every successfully delivered message is
    /// queued for the `logs.chat_log` table from then on.
    pub fn with_chat_logger(mut self, logger: Arc<ChatLogger>) -> Self {
        self.chat_logger = Some(logger);
        self
    }

    pub fn with_flood_config(
        broadcast_mgr: Arc<dyn BroadcastManagerTrait>,
        player_mgr: Arc<PlayerManager>,
        flood_config: FloodConfig,
    ) -> Self {
        let mut system = Self::new(broadcast_mgr, player_mgr);
        system.flood_config = flood_config;
        system
    }

    // ========== INITIALIZATION ==========

    pub(crate) fn create_default_channels(&self, team: Team) {
        let team_channels = self.channels.entry(team).or_insert_with(DashMap::new);

        let defaults = [
            ("General", ChannelId::General as u32),
            ("Trade", ChannelId::Trade as u32),
            ("LocalDefense", ChannelId::LocalDefense as u32),
            ("WorldDefense", ChannelId::WorldDefense as u32),
            ("GuildRecruitment", ChannelId::GuildRecruitment as u32),
            ("LookingForGroup", ChannelId::LookingForGroup as u32),
        ];

        for (name, id) in defaults {
            let channel = CachedChannel::new(id, name.to_string(), team);
            team_channels.insert(name.to_string(), ChannelData::new(channel));
        }
    }

    // ========== FLOOD PROTECTION ==========

    pub(crate) fn check_flood_protection(&self, player_guid: ObjectGuid) -> Result<(), ChatError> {
        let now = Instant::now();
        let window = Duration::from_secs(self.flood_config.window_secs);

        let mut flood_state = self
            .flood_tracker
            .entry(player_guid)
            .or_insert_with(FloodState::new);

        if let Some(mute_end) = flood_state.mute_end_time {
            if now < mute_end {
                let remaining = mute_end.duration_since(now).as_secs();

                let msg = SmsgChatRestricted;
                self.broadcast_mgr.send_msg_to_player(player_guid, msg);

                return Err(ChatError::Muted {
                    remaining_secs: remaining,
                });
            } else {
                flood_state.mute_end_time = None;
            }
        }

        while let Some(&oldest) = flood_state.message_timestamps.front() {
            if now.duration_since(oldest) > window {
                flood_state.message_timestamps.pop_front();
            } else {
                break;
            }
        }

        if flood_state.message_timestamps.len() >= self.flood_config.max_messages as usize {
            let mute_duration = Duration::from_secs(self.flood_config.mute_duration_secs);
            flood_state.mute_end_time = Some(now + mute_duration);

            // Clear message timestamps when muting so player starts fresh after unmute
            flood_state.message_timestamps.clear();

            let msg = SmsgChatRestricted;
            self.broadcast_mgr.send_msg_to_player(player_guid, msg);

            tracing::warn!(
                "Player {:?} auto-muted for {} seconds (flood protection)",
                player_guid,
                self.flood_config.mute_duration_secs
            );

            return Err(ChatError::Muted {
                remaining_secs: self.flood_config.mute_duration_secs,
            });
        }

        flood_state.message_timestamps.push_back(now);

        Ok(())
    }

    pub(crate) fn clean_expired_mutes(&self) {
        let now = Instant::now();

        self.flood_tracker.retain(|_, flood_state| {
            if let Some(mute_end) = flood_state.mute_end_time {
                if now >= mute_end {
                    flood_state.mute_end_time = None;
                }
            }
            true
        });
    }

    // ========== CHANNEL OPERATIONS ==========

    pub async fn join_channel(
        &self,
        player_guid: ObjectGuid,
        channel_name: &str,
        password: Option<&str>,
        team: Team,
    ) -> Result<(), ChatError> {
        self.join_channel_with_id(player_guid, channel_name, password, team, None)
            .await
    }

    pub async fn join_channel_with_id(
        &self,
        player_guid: ObjectGuid,
        channel_name: &str,
        password: Option<&str>,
        team: Team,
        client_channel_id: Option<u32>,
    ) -> Result<(), ChatError> {
        validation::validate_channel_name(channel_name)?;

        if let Some(player_chans) = self.player_channels.get(&player_guid) {
            if player_chans.len() >= MAX_CHANNELS_PER_PLAYER {
                return Err(ChatError::MaxChannelsReached);
            }
        }

        let is_builtin = CachedChannel::is_builtin_channel(channel_name)
            || [
                "General - ",
                "Trade - ",
                "LocalDefense - ",
                "WorldDefense",
                "GuildRecruitment",
                "LookingForGroup",
            ]
            .iter()
            .any(|prefix| channel_name.starts_with(prefix));

        let team_channels = self.channels.entry(team).or_insert_with(DashMap::new);

        let mut channel_data = team_channels
            .entry(channel_name.to_string())
            .or_insert_with(|| {
                let id = if let Some(id) = client_channel_id.filter(|id| *id != 0) {
                    id
                } else if is_builtin {
                    match channel_name.to_lowercase().as_str() {
                        "general" => ChannelId::General as u32,
                        "trade" => ChannelId::Trade as u32,
                        "localdefense" => ChannelId::LocalDefense as u32,
                        "worlddefense" => ChannelId::WorldDefense as u32,
                        "guildrecruitment" => ChannelId::GuildRecruitment as u32,
                        "lookingforgroup" => ChannelId::LookingForGroup as u32,
                        _ => self.next_channel_id.fetch_add(1, Ordering::SeqCst),
                    }
                } else {
                    self.next_channel_id.fetch_add(1, Ordering::SeqCst)
                };

                let mut channel = CachedChannel::new(id, channel_name.to_string(), team);
                // Set password for custom channels if provided by the creator
                if !is_builtin {
                    if let Some(pwd) = password {
                        channel.password = pwd.to_string();
                    }
                }
                ChannelData::new(channel)
            });

        if channel_data.banned.contains(&player_guid) {
            return Err(ChatError::BannedFromChannel);
        }

        if !channel_data.channel.password.is_empty() {
            if password.is_none() || password.unwrap() != channel_data.channel.password {
                return Err(ChatError::WrongPassword);
            }
        }

        if channel_data.members.contains_key(&player_guid) {
            // Already in channel - silently succeed (idempotent for auto-join)
            return Ok(());
        }

        let member = CachedChannelMember::new(player_guid);
        channel_data.members.insert(player_guid, member);

        if !channel_data.channel.is_constant()
            && channel_data.channel.owner_guid.is_empty()
            && channel_data.members.len() == 1
        {
            channel_data.channel.owner_guid = player_guid;
            if let Some(m) = channel_data.members.get_mut(&player_guid) {
                m.flags.set_flag(ChannelMemberFlags::OWNER, true);
                m.flags.set_flag(ChannelMemberFlags::MODERATOR, true);
            }
        }

        let channel_id = channel_data.channel.id;
        let is_custom = !channel_data.channel.is_constant();

        // Send player_joined notification to other members for custom channels only
        if is_custom {
            let member_guids: Vec<ObjectGuid> = channel_data
                .members
                .keys()
                .copied()
                .filter(|&guid| guid != player_guid)
                .collect();

            if !member_guids.is_empty() {
                let notify = SmsgChannelNotify::player_joined(channel_name, player_guid);
                let packet = notify;
                self.broadcast_mgr
                    .broadcast_msg_to_players(&member_guids, &packet);
            }
        }

        drop(channel_data);

        self.player_channels
            .entry(player_guid)
            .or_insert_with(HashSet::new)
            .insert(channel_name.to_string());

        let (map_id, zone_id) = self
            .player_mgr
            .get_player(player_guid)
            .map(|player| (player.map_id, player.zone_id))
            .unwrap_or_default();
        // HighGuidType703::ChatChannel. The client retains this GUID and sends it back with every
        // channel message, so an empty GUID makes a joined channel read-only in its UI.
        let channel_guid_high = (26u64 << 58)
            | (u64::from(DEFAULT_REALM_ID & 0x1FFF) << 42)
            | (u64::from(map_id & 0x1FFF) << 29)
            | (u64::from(zone_id & 0x7F_FFFF) << 6);
        let modern_channel_flags = match channel_id {
            1 => 0x01 | 0x02,                      // General: auto-join, zone-based
            2 => 0x01 | 0x02 | 0x08 | 0x10 | 0x20, // Trade
            22 => 0x01 | 0x02 | 0x01_0000,         // LocalDefense
            23 => 0x04 | 0x01_0000,                // WorldDefense: read-only alerts
            24 => 0,                               // LookingForGroup
            25 => 0x02 | 0x10 | 0x02_0000,         // GuildRecruitment
            _ => 0,
        };
        let notify = SmsgChannelNotify::you_joined(channel_name, channel_id)
            .with_modern_channel_guid(channel_guid_high, u64::from(channel_id))
            .with_modern_channel_flags(modern_channel_flags);
        self.broadcast_mgr.send_msg_to_player(player_guid, notify);

        Ok(())
    }

    pub async fn leave_channel(
        &self,
        player_guid: ObjectGuid,
        channel_name: &str,
        team: Team,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        if !channel_data.members.contains_key(&player_guid) {
            return Err(ChatError::NotInChannel);
        }

        let was_owner = channel_data.channel.owner_guid == player_guid;
        let is_custom = !channel_data.channel.is_constant();

        channel_data.members.remove(&player_guid);

        let new_owner_guid = if was_owner && is_custom && !channel_data.members.is_empty() {
            // Get the new owner guid first
            if let Some(&guid) = channel_data.members.keys().next() {
                channel_data.channel.owner_guid = guid;
                // Now mutably borrow the member to set flags
                if let Some(new_owner) = channel_data.members.get_mut(&guid) {
                    new_owner.flags.set_flag(ChannelMemberFlags::OWNER, true);
                }
                Some(guid)
            } else {
                None
            }
        } else {
            None
        };

        // Send player_left notification to other members for custom channels only
        if is_custom {
            let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();

            if !member_guids.is_empty() {
                let notify = SmsgChannelNotify::player_left(channel_name, player_guid);
                let packet = notify;
                self.broadcast_mgr
                    .broadcast_msg_to_players(&member_guids, &packet);
            }
        }

        // Notify all members of ownership change
        if let Some(owner_guid) = new_owner_guid {
            let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();

            if !member_guids.is_empty() {
                let notify = SmsgChannelNotify::owner_changed(channel_name, owner_guid);
                let packet = notify;
                self.broadcast_mgr
                    .broadcast_msg_to_players(&member_guids, &packet);
            }
        }

        let is_empty = channel_data.members.is_empty();
        drop(channel_data);

        if let Some(mut player_chans) = self.player_channels.get_mut(&player_guid) {
            player_chans.remove(channel_name);
        }

        if is_empty && is_custom {
            drop(team_channels);
            if let Some(mut team_chans) = self.channels.get_mut(&team) {
                team_chans.remove(channel_name);
            }
        }

        let notify = SmsgChannelNotify::you_left(channel_name);
        self.broadcast_mgr.send_msg_to_player(player_guid, notify);

        Ok(())
    }

    pub fn leave_all_channels(&self, player_guid: ObjectGuid) {
        if let Some(player_chans) = self.player_channels.remove(&player_guid) {
            let channel_names: Vec<String> = player_chans.1.into_iter().collect();

            for team_channels in self.channels.iter() {
                for channel_name in &channel_names {
                    if let Some(mut channel_data) = team_channels.get_mut(channel_name) {
                        channel_data.members.remove(&player_guid);
                    }
                }
            }
        }
    }

    // ========== CHANNEL MODERATION ==========

    pub fn set_moderator(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        is_moderator: bool,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        if let Some(member) = channel_data.members.get_mut(&player_guid) {
            let old_flags = member.flags.as_u8();
            if member.flags.has_flag(ChannelMemberFlags::MODERATOR) == is_moderator {
                return Ok(());
            }
            member
                .flags
                .set_flag(ChannelMemberFlags::MODERATOR, is_moderator);

            let notify = SmsgChannelNotify::mode_change(
                channel_name,
                player_guid,
                old_flags,
                member.flags.as_u8(),
            );
            let packet = notify;
            let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
            self.broadcast_mgr
                .broadcast_msg_to_players(&member_guids, &packet);

            Ok(())
        } else {
            Err(ChatError::NotInChannel)
        }
    }

    pub fn set_muted(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        is_muted: bool,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        if let Some(member) = channel_data.members.get_mut(&player_guid) {
            let old_flags = member.flags.as_u8();
            if member.flags.has_flag(ChannelMemberFlags::MUTED) == is_muted {
                return Ok(());
            }
            member.flags.set_flag(ChannelMemberFlags::MUTED, is_muted);

            let notify = SmsgChannelNotify::mode_change(
                channel_name,
                player_guid,
                old_flags,
                member.flags.as_u8(),
            );
            let packet = notify;
            let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
            self.broadcast_mgr
                .broadcast_msg_to_players(&member_guids, &packet);

            Ok(())
        } else {
            Err(ChatError::NotInChannel)
        }
    }

    pub fn ban_from_channel(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        channel_data.members.remove(&player_guid);
        channel_data.banned.insert(player_guid);

        Ok(())
    }

    pub fn unban_from_channel(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
    ) -> Result<bool, ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        Ok(channel_data.banned.remove(&player_guid))
    }

    pub fn set_password(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        password: &str,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() {
            return Err(ChatError::NotOwner);
        }

        channel_data.channel.password = password.to_string();

        let notify = SmsgChannelNotify::password_changed(channel_name, player_guid);
        let packet = notify;

        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        Ok(())
    }

    pub fn set_owner_by_name(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        target_name: &str,
    ) -> Result<(), ChatError> {
        let target_guid = self
            .find_player_by_name(target_name)
            .ok_or(ChatError::TargetNotFound)?;

        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() {
            return Err(ChatError::NotOwner);
        }

        if !channel_data.members.contains_key(&target_guid) {
            return Err(ChatError::TargetNotFound);
        }

        // Clear old owner flag
        let old_owner_guid = channel_data.channel.owner_guid;
        if let Some(old_owner) = channel_data.members.get_mut(&old_owner_guid) {
            old_owner.flags.set_flag(ChannelMemberFlags::OWNER, false);
        }

        // Set new owner
        channel_data.channel.owner_guid = target_guid;
        if let Some(new_owner) = channel_data.members.get_mut(&target_guid) {
            new_owner.flags.set_flag(ChannelMemberFlags::OWNER, true);
            new_owner
                .flags
                .set_flag(ChannelMemberFlags::MODERATOR, true);
        }

        let notify = SmsgChannelNotify::owner_changed(channel_name, target_guid);
        let packet = notify;

        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        Ok(())
    }

    pub fn send_owner_info(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let channel_data = team_channels
            .get(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let owner_name = self
            .player_mgr
            .get_player_name(channel_data.channel.owner_guid)
            .unwrap_or_default();

        let notify = SmsgChannelNotify::channel_owner(channel_name, &owner_name);
        self.broadcast_mgr.send_msg_to_player(player_guid, notify);

        Ok(())
    }

    pub fn kick_player_by_name(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        target_name: &str,
    ) -> Result<(), ChatError> {
        let target_guid = self
            .find_player_by_name(target_name)
            .ok_or(ChatError::TargetNotFound)?;

        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() && !member.is_moderator() {
            return Err(ChatError::NotModerator);
        }

        if !channel_data.members.contains_key(&target_guid) {
            return Err(ChatError::TargetNotFound);
        }

        // Don't allow kicking the owner
        if target_guid == channel_data.channel.owner_guid {
            return Err(ChatError::NotOwner);
        }

        channel_data.members.remove(&target_guid);

        // Notify kicked player
        let kick_notify = SmsgChannelNotify::player_kicked(channel_name, target_guid, player_guid);
        self.broadcast_mgr
            .send_msg_to_player(target_guid, kick_notify);

        // Notify remaining members
        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
        let left_notify = SmsgChannelNotify::player_left(channel_name, target_guid);
        let packet = left_notify;
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        Ok(())
    }

    pub fn invite_player_by_name(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
        target_name: &str,
    ) -> Result<(), ChatError> {
        let target_guid = self
            .find_player_by_name(target_name)
            .ok_or(ChatError::TargetNotFound)?;

        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let channel_data = team_channels
            .get(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() && !member.is_moderator() {
            return Err(ChatError::NotModerator);
        }

        if channel_data.members.contains_key(&target_guid) {
            let notify = SmsgChannelNotify::player_already_member(channel_name, target_guid);
            self.broadcast_mgr.send_msg_to_player(player_guid, notify);
            return Ok(());
        }

        if channel_data.banned.contains(&target_guid) {
            let notify = SmsgChannelNotify::player_invite_banned(channel_name, target_name);
            self.broadcast_mgr.send_msg_to_player(player_guid, notify);
            return Ok(());
        }

        // Send invite notification to target
        let notify = SmsgChannelNotify::invite(channel_name, player_guid);
        self.broadcast_mgr.send_msg_to_player(target_guid, notify);

        // Send confirmation to inviter
        let invite_notify = SmsgChannelNotify::player_invited(channel_name, target_name);
        self.broadcast_mgr
            .send_msg_to_player(player_guid, invite_notify);

        Ok(())
    }

    pub fn toggle_announcements(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() {
            return Err(ChatError::NotOwner);
        }

        channel_data.channel.announce = !channel_data.channel.announce;

        let notify = if channel_data.channel.announce {
            SmsgChannelNotify::announcements_on(channel_name, player_guid)
        } else {
            SmsgChannelNotify::announcements_off(channel_name, player_guid)
        };

        let packet = notify;
        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        Ok(())
    }

    pub fn toggle_moderate(
        &self,
        team: Team,
        channel_name: &str,
        player_guid: ObjectGuid,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let mut channel_data = team_channels
            .get_mut(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let member = channel_data
            .members
            .get(&player_guid)
            .ok_or(ChatError::NotInChannel)?;

        if !member.is_owner() {
            return Err(ChatError::NotOwner);
        }

        channel_data.channel.moderate = !channel_data.channel.moderate;

        let notify = if channel_data.channel.moderate {
            SmsgChannelNotify::moderation_on(channel_name, player_guid)
        } else {
            SmsgChannelNotify::moderation_off(channel_name, player_guid)
        };

        let packet = notify;
        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        Ok(())
    }

    // ========== MESSAGE SENDING ==========

    pub async fn send_channel_message(
        &self,
        sender_guid: ObjectGuid,
        channel_name: &str,
        message: &str,
        team: Team,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection - if it fails, error packet already sent
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(()); // Silent fail, error already sent to client
        }

        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let channel_data = team_channels
            .get(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        let sender_member = channel_data
            .members
            .get(&sender_guid)
            .ok_or(ChatError::NotInChannel)?;

        if sender_member.is_muted() {
            return Err(ChatError::MutedInChannel);
        }

        if channel_data.channel.moderate {
            if !sender_member.is_owner()
                && !sender_member.is_moderator()
                && !sender_member.flags.has_flag(ChannelMemberFlags::VOICED)
            {
                return Err(ChatError::NoPermission);
            }
        }

        // Get all members (including sender for echo)
        let member_guids: Vec<ObjectGuid> = channel_data.members.keys().copied().collect();

        drop(channel_data);

        let sender_name = self.get_player_name(sender_guid);

        // Channels always use Universal language
        let packet = SmsgMessageChat {
            msgtype: ChatMsg::Channel,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: Some(channel_name),
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Broadcast to all channel members (including sender)
        self.broadcast_mgr
            .broadcast_msg_to_players(&member_guids, &packet);

        self.log_message(
            "Channel",
            Some(channel_name.to_string()),
            sender_guid,
            None,
            None,
            &clean_message,
        );

        Ok(())
    }

    pub async fn send_channel_list(
        &self,
        player_guid: ObjectGuid,
        team: Team,
        channel_name: &str,
    ) -> Result<(), ChatError> {
        let team_channels = self.channels.get(&team).ok_or(ChatError::ChannelNotFound)?;

        let channel_data = team_channels
            .get(channel_name)
            .ok_or(ChatError::ChannelNotFound)?;

        // Check if player is in the channel
        if !channel_data.members.contains_key(&player_guid) {
            return Err(ChatError::NotInChannel);
        }

        // Build member list
        let members: Vec<ChannelMemberInfo> = channel_data
            .members
            .iter()
            .map(|(guid, member)| ChannelMemberInfo {
                guid: *guid,
                flags: member.flags.as_u8(),
            })
            .collect();

        let channel_flags = if channel_data.channel.is_constant() {
            1u8 // Constant channel flag
        } else {
            0u8
        };

        drop(channel_data);

        // Send SMSG_CHANNEL_LIST
        let packet = SmsgChannelList {
            channel_name,
            channel_flags,
            members: &members,
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, packet);

        Ok(())
    }

    pub async fn send_whisper(
        &self,
        sender_guid: ObjectGuid,
        target_name: &str,
        message: &str,
        social_system: &crate::game::social::SocialSystem,
    ) -> Result<(), ChatError> {
        self.send_whisper_impl(
            sender_guid,
            target_name,
            message,
            social_system,
            false,
            true,
        )
        .await
    }

    /// Whisper sent on behalf of a GM (from the admin panel). Bypasses the sender's
    /// flood protection and skips the "player not found" client notification — there
    /// is no client to notify and the caller records delivery state itself.
    pub async fn send_whisper_as_gm(
        &self,
        sender_guid: ObjectGuid,
        target_name: &str,
        message: &str,
        social_system: &crate::game::social::SocialSystem,
    ) -> Result<(), ChatError> {
        self.send_whisper_impl(
            sender_guid,
            target_name,
            message,
            social_system,
            true,
            false,
        )
        .await
    }

    async fn send_whisper_impl(
        &self,
        sender_guid: ObjectGuid,
        target_name: &str,
        message: &str,
        social_system: &crate::game::social::SocialSystem,
        bypass_flood_protection: bool,
        notify_target_not_found: bool,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection - if it fails, error packet already sent
        if !bypass_flood_protection {
            if let Err(_) = self.check_flood_protection(sender_guid) {
                return Ok(()); // Silent fail, error already sent to client
            }
        }

        // Find target player
        let target_guid = match self.find_player_by_name(target_name) {
            Some(guid) => guid,
            None => {
                if notify_target_not_found {
                    // Target not found - send error notification to sender
                    let not_found_msg = SmsgChatPlayerNotFound { name: target_name };
                    self.broadcast_mgr
                        .send_msg_to_player(sender_guid, not_found_msg);
                }
                return Err(ChatError::TargetNotFound);
            }
        };

        // Check if sender is ignored by target
        if social_system.is_ignored(sender_guid, target_guid) {
            // Silently fail - don't notify sender that they're ignored
            return Ok(());
        }

        let sender_name = self.get_player_name(sender_guid);
        let target_name_actual = self.get_player_name(target_guid);

        // Send whisper to target
        let whisper_packet = SmsgMessageChat {
            msgtype: ChatMsg::Whisper,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: Some(target_guid),
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        self.broadcast_mgr
            .send_msg_to_player(target_guid, whisper_packet);

        // Send echo/confirmation to sender
        let inform_packet = SmsgMessageChat {
            msgtype: ChatMsg::WhisperInform,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&target_name_actual),
            target_guid: Some(target_guid),
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        self.broadcast_mgr
            .send_msg_to_player(sender_guid, inform_packet);

        self.log_message(
            "Whisper",
            None,
            sender_guid,
            Some(target_guid),
            Some(target_name_actual),
            &clean_message,
        );

        Ok(())
    }

    pub async fn send_say(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        sender_team: Team,
        allow_cross_faction: bool,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection - if it fails, error packet already sent
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(()); // Silent fail, error already sent to client
        }

        // Get player name
        let sender_name = self.get_player_name(sender_guid);

        // Build same-faction packet (Universal language - everyone understands)
        let same_faction_packet = SmsgMessageChat {
            msgtype: ChatMsg::Say,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Build cross-faction packet if cross-faction chat is disabled (racial language - gibberish)
        let cross_faction_packet = if !allow_cross_faction {
            let racial_lang = Language::for_faction(sender_team);
            Some(SmsgMessageChat {
                msgtype: ChatMsg::Say,
                language: racial_lang,
                sender_guid,
                sender_name: Some(&sender_name),
                target_guid: None,
                channel_name: None,
                player_rank: None,
                message: &clean_message,
                chat_tag: ChatTag::None,
            })
        } else {
            None
        };

        // Send to sender first (always same-faction)
        self.broadcast_mgr
            .send_msg_to_player(sender_guid, same_faction_packet.clone());

        tracing::info!(
            player = %sender_guid,
            message_bytes = clean_message.len(),
            "queued /say self echo"
        );

        // Broadcast to nearby players with distance filtering (25 yards) and faction-aware packets
        let broadcaster = self.player_mgr.get_broadcaster(sender_guid);
        let mut delivered_nearby = 0usize;
        if let Some(broadcaster) = broadcaster {
            let listeners: Vec<ObjectGuid> = {
                let lock = broadcaster.listeners().read();
                lock.keys().copied().collect()
            };

            // Get sender position for distance checks
            if let Some(sender_pos) = self.player_mgr.get_position(sender_guid) {
                for listener_guid in listeners {
                    if listener_guid == sender_guid {
                        continue; // Already sent to self
                    }

                    // Get listener position
                    if let Some(listener_pos) = self.player_mgr.get_position(listener_guid) {
                        // Distance check (25 yards for Say)
                        let distance = sender_pos.distance_2d(&listener_pos);
                        if distance > super::types::SAY_RANGE {
                            continue; // Too far
                        }

                        // Faction check - determine which packet to send
                        if let Some(listener) = self.player_mgr.get_player(listener_guid) {
                            let listener_team = Team::from_race(listener.race);
                            let packet = if listener_team == sender_team || allow_cross_faction {
                                // Same faction OR cross-faction enabled: send Universal language
                                &same_faction_packet
                            } else if let Some(ref cross_packet) = cross_faction_packet {
                                // Cross-faction disabled: send racial language (gibberish)
                                cross_packet
                            } else {
                                continue; // Shouldn't happen, but skip if no packet available
                            };

                            self.broadcast_mgr
                                .send_msg_to_player(listener_guid, packet.clone());
                            delivered_nearby += 1;
                        }
                    }
                }
            }
        }

        tracing::info!(
            player = %sender_guid,
            nearby_recipients = delivered_nearby,
            "completed /say recipient routing"
        );

        self.log_message("Say", None, sender_guid, None, None, &clean_message);

        Ok(())
    }

    pub async fn send_yell(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        sender_team: Team,
        allow_cross_faction: bool,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection - if it fails, error packet already sent
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(()); // Silent fail, error already sent to client
        }

        // Get player name
        let sender_name = self.get_player_name(sender_guid);

        // Build same-faction packet (Universal language - everyone understands)
        let same_faction_packet = SmsgMessageChat {
            msgtype: ChatMsg::Yell,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Build cross-faction packet if cross-faction chat is disabled (racial language - gibberish)
        let cross_faction_packet = if !allow_cross_faction {
            let racial_lang = Language::for_faction(sender_team);
            Some(SmsgMessageChat {
                msgtype: ChatMsg::Yell,
                language: racial_lang,
                sender_guid,
                sender_name: Some(&sender_name),
                target_guid: None,
                channel_name: None,
                player_rank: None,
                message: &clean_message,
                chat_tag: ChatTag::None,
            })
        } else {
            None
        };

        // Send to sender first (always same-faction)
        self.broadcast_mgr
            .send_msg_to_player(sender_guid, same_faction_packet.clone());

        // Broadcast to nearby players with distance filtering (300 yards) and faction-aware packets
        let broadcaster = self.player_mgr.get_broadcaster(sender_guid);
        if let Some(broadcaster) = broadcaster {
            let listeners: Vec<ObjectGuid> = {
                let lock = broadcaster.listeners().read();
                lock.keys().copied().collect()
            };

            // Get sender position for distance checks
            if let Some(sender_pos) = self.player_mgr.get_position(sender_guid) {
                for listener_guid in listeners {
                    if listener_guid == sender_guid {
                        continue; // Already sent to self
                    }

                    // Get listener position
                    if let Some(listener_pos) = self.player_mgr.get_position(listener_guid) {
                        // Distance check (300 yards for Yell)
                        let distance = sender_pos.distance_2d(&listener_pos);
                        if distance > super::types::YELL_RANGE {
                            continue; // Too far
                        }

                        // Faction check - determine which packet to send
                        if let Some(listener) = self.player_mgr.get_player(listener_guid) {
                            let listener_team = Team::from_race(listener.race);
                            let packet = if listener_team == sender_team || allow_cross_faction {
                                // Same faction OR cross-faction enabled: send Universal language
                                &same_faction_packet
                            } else if let Some(ref cross_packet) = cross_faction_packet {
                                // Cross-faction disabled: send racial language (gibberish)
                                cross_packet
                            } else {
                                continue; // Shouldn't happen, but skip if no packet available
                            };

                            self.broadcast_mgr
                                .send_msg_to_player(listener_guid, packet.clone());
                        }
                    }
                }
            }
        }

        self.log_message("Yell", None, sender_guid, None, None, &clean_message);

        Ok(())
    }

    pub async fn send_emote(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection - if it fails, error packet already sent
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(()); // Silent fail, error already sent to client
        }

        // Get player name
        let sender_name = self.get_player_name(sender_guid);

        // Build SMSG_MESSAGECHAT packet
        let packet = SmsgMessageChat {
            msgtype: ChatMsg::Emote,
            language: Language::Universal, // Emotes are always universal
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Send to sender first
        self.broadcast_mgr
            .send_msg_to_player(sender_guid, packet.clone());

        // Broadcast to nearby players (exclude sender)
        self.broadcast_mgr
            .broadcast_msg_nearby(sender_guid, &packet, false);

        self.log_message("Emote", None, sender_guid, None, None, &clean_message);

        Ok(())
    }

    /// Send party chat message to all group members
    pub async fn send_party(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        group_system: &super::super::group::GroupSystem,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(());
        }

        // Get sender's group
        let group = group_system
            .get_player_group(sender_guid)
            .ok_or(ChatError::NotInGroup)?;

        // Get player name
        let sender_name = self.get_player_name(sender_guid);

        // Build packet
        let packet = SmsgMessageChat {
            msgtype: ChatMsg::Party,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Send to all online group members
        for member in &group.members {
            if member.status.is_online() {
                self.broadcast_mgr
                    .send_msg_to_player(member.guid, packet.clone());
            }
        }

        self.log_message(
            "Party",
            Some(format!("group:{}", group.id)),
            sender_guid,
            None,
            None,
            &clean_message,
        );

        Ok(())
    }

    /// Send raid chat message to all raid members
    pub async fn send_raid(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        msg_type: ChatMsg,
        group_system: &super::super::group::GroupSystem,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        // Check flood protection
        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(());
        }

        // Get sender's group
        let group = group_system
            .get_player_group(sender_guid)
            .ok_or(ChatError::NotInGroup)?;

        // For RaidLeader and RaidWarning, verify sender is leader or assistant
        if msg_type == ChatMsg::RaidLeader || msg_type == ChatMsg::RaidWarning {
            if !group.is_leader(sender_guid) && !group.is_assistant(sender_guid) {
                return Err(ChatError::NoPermission);
            }
        }

        // Get player name
        let sender_name = self.get_player_name(sender_guid);

        // Build packet
        let packet = SmsgMessageChat {
            msgtype: msg_type,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Send to all online group members
        for member in &group.members {
            if member.status.is_online() {
                self.broadcast_mgr
                    .send_msg_to_player(member.guid, packet.clone());
            }
        }

        let raid_label = match msg_type {
            ChatMsg::RaidLeader => "RaidLeader",
            ChatMsg::RaidWarning => "RaidWarning",
            _ => "Raid",
        };
        self.log_message(
            raid_label,
            Some(format!("group:{}", group.id)),
            sender_guid,
            None,
            None,
            &clean_message,
        );

        Ok(())
    }

    /// Send guild chat message to all guild members
    pub fn send_guild(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        guild_system: &super::super::guild::GuildSystem,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(());
        }

        // Get sender's guild
        let guild_state = guild_system
            .get_player_guild(sender_guid)
            .ok_or(ChatError::NotInGuild)?;
        let guild_id = guild_state.guild_id.ok_or(ChatError::NotInGuild)?;

        // Get guild data for member list
        let guild_data = guild_system
            .get_guild(guild_id)
            .ok_or(ChatError::NotInGuild)?;

        let sender_name = self.get_player_name(sender_guid);

        let packet = SmsgMessageChat {
            msgtype: ChatMsg::Guild,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Send to all guild members (broadcast_mgr silently drops for offline players)
        for member_guid in guild_data.members.keys() {
            self.broadcast_mgr
                .send_msg_to_player(*member_guid, packet.clone());
        }

        self.log_message(
            "Guild",
            Some(guild_data.info.name.clone()),
            sender_guid,
            None,
            None,
            &clean_message,
        );

        Ok(())
    }

    /// Send officer chat message to guild officers
    pub fn send_officer(
        &self,
        sender_guid: ObjectGuid,
        message: &str,
        guild_system: &super::super::guild::GuildSystem,
    ) -> Result<(), ChatError> {
        let clean_message = validation::strip_invisible_chars(message);
        validation::validate_message(&clean_message)?;

        if let Err(_) = self.check_flood_protection(sender_guid) {
            return Ok(());
        }

        // Get sender's guild
        let guild_state = guild_system
            .get_player_guild(sender_guid)
            .ok_or(ChatError::NotInGuild)?;
        let guild_id = guild_state.guild_id.ok_or(ChatError::NotInGuild)?;

        // Officers are rank <= 1 (Guild Master = 0, Officer = 1)
        if guild_state.rank_id > 1 {
            return Err(ChatError::NoPermission);
        }

        let guild_data = guild_system
            .get_guild(guild_id)
            .ok_or(ChatError::NotInGuild)?;

        let sender_name = self.get_player_name(sender_guid);

        let packet = SmsgMessageChat {
            msgtype: ChatMsg::Officer,
            language: Language::Universal,
            sender_guid,
            sender_name: Some(&sender_name),
            target_guid: None,
            channel_name: None,
            player_rank: None,
            message: &clean_message,
            chat_tag: ChatTag::None,
        };

        // Send to all guild members (officers can see officer chat, but so can GM)
        // In vanilla, officer chat is visible to anyone with GRIGHT_OFFCHATLISTEN
        // For simplicity, send to all members — client filters based on rank
        for member_guid in guild_data.members.keys() {
            self.broadcast_mgr
                .send_msg_to_player(*member_guid, packet.clone());
        }

        self.log_message(
            "Officer",
            Some(guild_data.info.name.clone()),
            sender_guid,
            None,
            None,
            &clean_message,
        );

        Ok(())
    }

    // ========== HELPERS ==========

    fn get_player_name(&self, guid: ObjectGuid) -> String {
        self.player_mgr
            .get_player(guid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn find_player_by_name(&self, name: &str) -> Option<ObjectGuid> {
        let lower_name = name.to_lowercase();
        for player_ref in self.player_mgr.iter() {
            let player = player_ref.value();
            if player.name.to_lowercase() == lower_name {
                return Some(player.guid);
            }
        }
        None
    }

    // ========== CHAT LOGGING ==========

    /// Resolve the (account, map, position) context of a sender for the chat log.
    fn player_log_context(&self, guid: ObjectGuid) -> (u32, u32, Position) {
        let (account, map, position) = match self.player_mgr.get_player(guid) {
            Some(player) => (player.account_id, player.map_id, player.movement.position),
            None => (0, 0, Position::default()),
        };
        (account, map, position)
    }

    /// Queue a structured chat-log row for the given message (no-op when logging is
    /// disabled or no sender context is available).
    fn log_message(
        &self,
        channel_type: &'static str,
        channel_name: Option<String>,
        sender_guid: ObjectGuid,
        target_guid: Option<ObjectGuid>,
        target_name: Option<String>,
        message: &str,
    ) {
        let Some(logger) = &self.chat_logger else {
            return;
        };
        let (sender_account, map, position) = self.player_log_context(sender_guid);
        logger.log(ChatLogEntry {
            channel_type,
            channel_name,
            sender_guid: sender_guid.low(),
            sender_name: self.get_player_name(sender_guid),
            sender_account,
            target_guid: target_guid.map(|g| g.low()),
            target_name,
            message: message.to_string(),
            map,
            pos_x: position.x,
            pos_y: position.y,
            pos_z: position.z,
        });
    }

    pub fn get_channel_members(&self, team: Team, channel_name: &str) -> Vec<ObjectGuid> {
        self.channels
            .get(&team)
            .and_then(|tc| {
                tc.get(channel_name)
                    .map(|channel_data| channel_data.members.keys().copied().collect())
            })
            .unwrap_or_default()
    }

    pub fn is_in_channel(&self, player_guid: ObjectGuid, channel_name: &str, team: Team) -> bool {
        self.channels
            .get(&team)
            .and_then(|tc| {
                tc.get(channel_name)
                    .map(|channel_data| channel_data.members.contains_key(&player_guid))
            })
            .unwrap_or(false)
    }

    pub fn get_player_channels(&self, player_guid: ObjectGuid) -> Vec<String> {
        self.player_channels
            .get(&player_guid)
            .map(|chans| chans.iter().cloned().collect())
            .unwrap_or_default()
    }

    // ========== COMMAND SYSTEM ==========

    /// Check if a command exists in the registry
    pub fn command_exists(&self, command_name: &str) -> bool {
        self.command_registry.exists(command_name)
    }

    /// Execute a chat command and return the result message
    pub async fn execute_command<'a>(
        &self,
        command_str: &str,
        ctx: &commands::ChatCommandContext<'a>,
    ) -> Result<String> {
        self.command_registry.execute(command_str, ctx).await
    }

    /// Get help text for commands
    pub fn get_command_help(
        &self,
        command: Option<&str>,
        security: oxcore_shared::common::AccountType,
    ) -> String {
        self.command_registry.get_help(command, security)
    }

    // ========== TEST HELPERS ==========

    #[cfg(test)]
    pub(crate) fn test_channels_empty(&self) -> bool {
        self.channels.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn test_player_channels_empty(&self) -> bool {
        self.player_channels.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn test_flood_tracker_empty(&self) -> bool {
        self.flood_tracker.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn test_flood_tracker_contains(&self, guid: ObjectGuid) -> bool {
        self.flood_tracker.contains_key(&guid)
    }

    #[cfg(test)]
    pub(crate) fn test_get_channels(&self, team: Team) -> Option<Vec<String>> {
        self.channels.get(&team).map(|team_channels| {
            team_channels
                .iter()
                .map(|entry| entry.key().clone())
                .collect()
        })
    }

    // ========== LIFECYCLE METHODS ==========

    pub async fn init(&self) -> Result<()> {
        self.create_default_channels(Team::Alliance);
        self.create_default_channels(Team::Horde);

        Ok(())
    }

    pub fn update(&self, _diff: Duration) -> Result<()> {
        self.clean_expired_mutes();
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.channels.clear();
        self.player_channels.clear();
        self.flood_tracker.clear();

        Ok(())
    }

    pub fn on_player_login(&self, guid: ObjectGuid) -> Result<()> {
        self.flood_tracker.insert(guid, FloodState::new());
        Ok(())
    }

    pub fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        self.leave_all_channels(guid);
        self.flood_tracker.remove(&guid);
        Ok(())
    }
}
