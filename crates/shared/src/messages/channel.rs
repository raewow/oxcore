//! Channel system message structs
//!
//! This module contains type-safe message structures for channel-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgChannelNotify`] - Channel notification (join, leave, kick, ban, etc.)
//! - [`SmsgChannelList`] - List of channel members

use crate::game::chat::ChatNotify;
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::ObjectGuid;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// `region << 24 | site << 16 | realm id`, the same scheme the bnet realm list advertises.
const VIRTUAL_REALM_ADDRESS: u32 = 0x0101_0000 | DEFAULT_REALM_ID as u32;

/// 1.14's `ChannelFlags`, which is **not** the flag set 1.12 sends.
///
/// The two enums share a width and nothing else: 1.12's bits mean custom / trade / not-LFG /
/// general / city / LFG, and 1.14's mean auto-join / zone-based / read-only / allow-item-links /
/// only-in-cities / linked-channel. `0x01` is "custom" in one and "auto-join" in the other, so
/// passing the byte through would assert channel behaviour we never computed — a custom channel
/// would come back flagged auto-join and the client would rejoin it on every zone change.
///
/// There is no name-level correspondence to translate through either, so nothing is carried. Zero
/// means "an ordinary channel", which is what every channel this server serves actually is.
const CHANNEL_FLAGS_NONE: u32 = 0;

/// SMSG_CHANNEL_NOTIFY - Channel notification packet
///
/// Used for all channel notifications: join, leave, kick, ban, etc.
/// The packet format varies based on the notification type.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - notify_type (u8) - Notification type
/// - channel_name (cstring) - Channel name
/// - [type-specific fields] - Varies by notification type
#[derive(Debug, Clone)]
pub struct SmsgChannelNotify<'a> {
    /// Notification type
    pub notify_type: ChatNotify,
    /// Channel name
    pub channel_name: &'a str,
    /// Additional data based on notification type
    pub data: ChannelNotifyData,
}

/// Additional data for channel notifications
#[derive(Debug, Clone, Default)]
pub struct ChannelNotifyData {
    /// Primary GUID (player who joined/left/was kicked/etc.)
    pub guid: Option<ObjectGuid>,
    /// Secondary GUID (actor who kicked/banned)
    pub actor_guid: Option<ObjectGuid>,
    /// Player name (for PLAYER_NOT_FOUND, etc.)
    pub name: Option<String>,
    /// Channel flags (for YOU_JOINED)
    pub flags: Option<u32>,
    /// Old member flags (for MODE_CHANGE)
    pub old_flags: Option<u8>,
    /// New member flags (for MODE_CHANGE)
    pub new_flags: Option<u8>,
}

impl ChannelNotifyData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_guid(mut self, guid: ObjectGuid) -> Self {
        self.guid = Some(guid);
        self
    }

    pub fn with_actor(mut self, guid: ObjectGuid) -> Self {
        self.actor_guid = Some(guid);
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = Some(flags);
        self
    }

    pub fn with_mode_change(mut self, old_flags: u8, new_flags: u8) -> Self {
        self.old_flags = Some(old_flags);
        self.new_flags = Some(new_flags);
        self
    }
}

impl<'a> SmsgChannelNotify<'a> {
    /// Create a "you joined" notification
    pub fn you_joined(channel_name: &'a str, channel_flags: u32) -> Self {
        Self {
            notify_type: ChatNotify::YouJoinedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_flags(channel_flags),
        }
    }

    /// Create a "you left" notification
    pub fn you_left(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::YouLeftNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "player joined" notification
    pub fn player_joined(channel_name: &'a str, player_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::JoinedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(player_guid),
        }
    }

    /// Create a "player left" notification
    pub fn player_left(channel_name: &'a str, player_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::LeftNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(player_guid),
        }
    }

    /// Create a "wrong password" notification
    pub fn wrong_password(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::WrongPasswordNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "not member" notification
    pub fn not_member(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::NotMemberNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "not moderator" notification
    pub fn not_moderator(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::NotModeratorNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "not owner" notification
    pub fn not_owner(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::NotOwnerNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "password changed" notification
    pub fn password_changed(channel_name: &'a str, changer_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::PasswordChangedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(changer_guid),
        }
    }

    /// Create an "owner changed" notification
    pub fn owner_changed(channel_name: &'a str, new_owner_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::OwnerChangedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(new_owner_guid),
        }
    }

    /// Create a "player not found" notification
    pub fn player_not_found(channel_name: &'a str, player_name: &str) -> Self {
        Self {
            notify_type: ChatNotify::PlayerNotFoundNotice,
            channel_name,
            data: ChannelNotifyData::new().with_name(player_name),
        }
    }

    /// Create a "channel owner" notification (who owns the channel)
    pub fn channel_owner(channel_name: &'a str, owner_name: &str) -> Self {
        Self {
            notify_type: ChatNotify::ChannelOwnerNotice,
            channel_name,
            data: ChannelNotifyData::new().with_name(owner_name),
        }
    }

    /// Create a "mode change" notification
    pub fn mode_change(
        channel_name: &'a str,
        player_guid: ObjectGuid,
        old_flags: u8,
        new_flags: u8,
    ) -> Self {
        Self {
            notify_type: ChatNotify::ModeChangeNotice,
            channel_name,
            data: ChannelNotifyData::new()
                .with_guid(player_guid)
                .with_mode_change(old_flags, new_flags),
        }
    }

    /// Create an "announcements on" notification
    pub fn announcements_on(channel_name: &'a str, setter_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::AnnouncementsOnNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(setter_guid),
        }
    }

    /// Create an "announcements off" notification
    pub fn announcements_off(channel_name: &'a str, setter_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::AnnouncementsOffNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(setter_guid),
        }
    }

    /// Create a "moderation on" notification
    pub fn moderation_on(channel_name: &'a str, setter_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::ModerationOnNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(setter_guid),
        }
    }

    /// Create a "moderation off" notification
    pub fn moderation_off(channel_name: &'a str, setter_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::ModerationOffNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(setter_guid),
        }
    }

    /// Create a "muted" notification
    pub fn muted(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::MutedNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "player kicked" notification
    pub fn player_kicked(
        channel_name: &'a str,
        kicked_guid: ObjectGuid,
        kicker_guid: ObjectGuid,
    ) -> Self {
        Self {
            notify_type: ChatNotify::PlayerKickedNotice,
            channel_name,
            data: ChannelNotifyData::new()
                .with_guid(kicked_guid)
                .with_actor(kicker_guid),
        }
    }

    /// Create a "banned" notification (you are banned)
    pub fn banned(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::BannedNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "player banned" notification
    pub fn player_banned(
        channel_name: &'a str,
        banned_guid: ObjectGuid,
        banner_guid: ObjectGuid,
    ) -> Self {
        Self {
            notify_type: ChatNotify::PlayerBannedNotice,
            channel_name,
            data: ChannelNotifyData::new()
                .with_guid(banned_guid)
                .with_actor(banner_guid),
        }
    }

    /// Create a "player unbanned" notification
    pub fn player_unbanned(
        channel_name: &'a str,
        unbanned_guid: ObjectGuid,
        unbanner_guid: ObjectGuid,
    ) -> Self {
        Self {
            notify_type: ChatNotify::PlayerUnbannedNotice,
            channel_name,
            data: ChannelNotifyData::new()
                .with_guid(unbanned_guid)
                .with_actor(unbanner_guid),
        }
    }

    /// Create a "player not banned" notification
    pub fn player_not_banned(channel_name: &'a str, player_name: &str) -> Self {
        Self {
            notify_type: ChatNotify::PlayerNotBannedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_name(player_name),
        }
    }

    /// Create a "player already member" notification
    pub fn player_already_member(channel_name: &'a str, player_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::PlayerAlreadyMemberNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(player_guid),
        }
    }

    /// Create an "invite" notification
    pub fn invite(channel_name: &'a str, inviter_guid: ObjectGuid) -> Self {
        Self {
            notify_type: ChatNotify::InviteNotice,
            channel_name,
            data: ChannelNotifyData::new().with_guid(inviter_guid),
        }
    }

    /// Create an "invite wrong faction" notification
    pub fn invite_wrong_faction(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::InviteWrongFactionNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "wrong faction" notification
    pub fn wrong_faction(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::WrongFactionNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create an "invalid name" notification
    pub fn invalid_name(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::InvalidNameNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "not moderated" notification
    pub fn not_moderated(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::NotModeratedNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }

    /// Create a "player invited" notification
    pub fn player_invited(channel_name: &'a str, invited_name: &str) -> Self {
        Self {
            notify_type: ChatNotify::PlayerInvitedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_name(invited_name),
        }
    }

    /// Create a "player invite banned" notification
    pub fn player_invite_banned(channel_name: &'a str, banned_name: &str) -> Self {
        Self {
            notify_type: ChatNotify::PlayerInviteBannedNotice,
            channel_name,
            data: ChannelNotifyData::new().with_name(banned_name),
        }
    }

    /// Create a "throttled" notification
    pub fn throttled(channel_name: &'a str) -> Self {
        Self {
            notify_type: ChatNotify::ThrottledNotice,
            channel_name,
            data: ChannelNotifyData::new(),
        }
    }
}

impl ToWorldPacket for SmsgChannelNotify<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHANNEL_NOTIFY);

        // Write notify type
        packet.write_u8(self.notify_type as u8);

        // Write channel name
        packet.write_cstring(self.channel_name);

        // Write additional data based on notify type
        match self.notify_type {
            // Notifications with single GUID
            ChatNotify::JoinedNotice
            | ChatNotify::LeftNotice
            | ChatNotify::PasswordChangedNotice
            | ChatNotify::OwnerChangedNotice
            | ChatNotify::AnnouncementsOnNotice
            | ChatNotify::AnnouncementsOffNotice
            | ChatNotify::ModerationOnNotice
            | ChatNotify::ModerationOffNotice
            | ChatNotify::PlayerAlreadyMemberNotice
            | ChatNotify::InviteNotice => {
                if let Some(guid) = self.data.guid {
                    packet.write_guid_raw(guid.raw());
                }
            }

            // YOU_JOINED: flags (u32) + 0 (u32)
            ChatNotify::YouJoinedNotice => {
                packet.write_u32(self.data.flags.unwrap_or(0));
                packet.write_u32(0); // Always 0 in vanilla
            }

            // Notifications with string
            ChatNotify::PlayerNotFoundNotice
            | ChatNotify::ChannelOwnerNotice
            | ChatNotify::PlayerNotBannedNotice
            | ChatNotify::PlayerInvitedNotice
            | ChatNotify::PlayerInviteBannedNotice => {
                if let Some(ref name) = self.data.name {
                    packet.write_cstring(name);
                } else {
                    packet.write_u8(0); // Empty string
                }
            }

            // MODE_CHANGE: guid + old_flags (u8) + new_flags (u8)
            ChatNotify::ModeChangeNotice => {
                if let Some(guid) = self.data.guid {
                    packet.write_guid_raw(guid.raw());
                }
                packet.write_u8(self.data.old_flags.unwrap_or(0));
                packet.write_u8(self.data.new_flags.unwrap_or(0));
            }

            // Notifications with two GUIDs (target + actor)
            ChatNotify::PlayerKickedNotice
            | ChatNotify::PlayerBannedNotice
            | ChatNotify::PlayerUnbannedNotice => {
                if let Some(guid) = self.data.guid {
                    packet.write_guid_raw(guid.raw());
                }
                if let Some(actor) = self.data.actor_guid {
                    packet.write_guid_raw(actor.raw());
                }
            }

            // Notifications with no extra data
            ChatNotify::YouLeftNotice
            | ChatNotify::WrongPasswordNotice
            | ChatNotify::NotMemberNotice
            | ChatNotify::NotModeratorNotice
            | ChatNotify::NotOwnerNotice
            | ChatNotify::MutedNotice
            | ChatNotify::BannedNotice
            | ChatNotify::InviteWrongFactionNotice
            | ChatNotify::WrongFactionNotice
            | ChatNotify::InvalidNameNotice
            | ChatNotify::NotModeratedNotice
            | ChatNotify::ThrottledNotice => {
                // No additional data
            }
        }

        packet
    }

    /// `ChannelNotifyJoined::Write` and `ChannelNotifyLeft::Write`.
    ///
    /// **1.14 has no general-purpose channel notify.** Vanilla funnels thirty-two different
    /// notifications through one opcode with a type byte and a per-type tail; 1.14 lifted the two
    /// that change channel membership — "you joined" and "you left" — into their own opcodes and
    /// dropped the rest. The client renders the remainder (someone was kicked, the owner changed,
    /// you lack permission) from the chat stream instead, so they are not missing information, just
    /// carried elsewhere.
    ///
    /// Everything except the two membership notices therefore returns `None`. That is a real
    /// difference in the protocol, not an unfinished port: there is no 1.14 body for them to go in.
    ///
    /// Deliberately exhaustive with no catch-all, so a new [`ChatNotify`] variant has to be
    /// classified here rather than silently joining the dropped set.
    fn to_modern(&self) -> Option<WorldPacket> {
        let name = self.channel_name.as_bytes();
        let mut writer = BitWriter::new();

        match self.notify_type {
            ChatNotify::YouJoinedNotice => {
                writer.write_bits(name.len() as u32, 7);
                // Welcome message length. Vanilla has no channel MOTD, so this stays zero and no
                // string follows it -- but the field is read either way, and both lengths are part
                // of one bit run that the u32 below flushes.
                writer.write_bits(0, 11);

                writer.write_u32(CHANNEL_FLAGS_NONE);

                // 1.14 splits vanilla's single trailing word into a flags word and a channel id.
                // `data.flags` is what the join path fills with the channel's id -- see
                // `SmsgChannelNotify::you_joined`'s caller -- and the id is the field that matters:
                // the client binds a built-in channel to its slot by id. If that field ever starts
                // carrying actual flags, this line has to move to `CHANNEL_FLAGS_NONE` above, and
                // the two would need translating rather than copying.
                writer.write_i32(self.data.flags.unwrap_or(0) as i32);

                writer.write_u64(0); // InstanceID -- channels are not instanced in Classic Era
                // ChannelGUID: 1.14 names the channel object with a GUID built from the map, zone
                // and channel id. Vanilla has no channel object and this message carries neither
                // map nor zone, so it is sent empty rather than assembled from values we would be
                // making up.
                writer.write_packed_guid_128(0, 0);

                writer.write_bytes(name);
                // Welcome message omitted: its length was written as zero above.

                Some(writer.finish(Opcode::SMSG_CHANNEL_NOTIFY_JOINED))
            }

            ChatNotify::YouLeftNotice => {
                writer.write_bits(name.len() as u32, 7);
                // Suspended distinguishes a kick/ban from an ordinary leave; vanilla's "you left"
                // is only ever the ordinary one.
                writer.write_bit(false);
                writer.write_i32(self.data.flags.unwrap_or(0) as i32); // ChatChannelID
                writer.write_bytes(name);

                Some(writer.finish(Opcode::SMSG_CHANNEL_NOTIFY_LEFT))
            }

            // No 1.14 counterpart -- see above.
            ChatNotify::JoinedNotice
            | ChatNotify::LeftNotice
            | ChatNotify::WrongPasswordNotice
            | ChatNotify::NotMemberNotice
            | ChatNotify::NotModeratorNotice
            | ChatNotify::PasswordChangedNotice
            | ChatNotify::OwnerChangedNotice
            | ChatNotify::PlayerNotFoundNotice
            | ChatNotify::NotOwnerNotice
            | ChatNotify::ChannelOwnerNotice
            | ChatNotify::ModeChangeNotice
            | ChatNotify::AnnouncementsOnNotice
            | ChatNotify::AnnouncementsOffNotice
            | ChatNotify::ModerationOnNotice
            | ChatNotify::ModerationOffNotice
            | ChatNotify::MutedNotice
            | ChatNotify::PlayerKickedNotice
            | ChatNotify::BannedNotice
            | ChatNotify::PlayerBannedNotice
            | ChatNotify::PlayerUnbannedNotice
            | ChatNotify::PlayerNotBannedNotice
            | ChatNotify::PlayerAlreadyMemberNotice
            | ChatNotify::InviteNotice
            | ChatNotify::InviteWrongFactionNotice
            | ChatNotify::WrongFactionNotice
            | ChatNotify::InvalidNameNotice
            | ChatNotify::NotModeratedNotice
            | ChatNotify::PlayerInvitedNotice
            | ChatNotify::PlayerInviteBannedNotice
            | ChatNotify::ThrottledNotice => None,
        }
    }
}

/// Channel member info for SMSG_CHANNEL_LIST
#[derive(Debug, Clone)]
pub struct ChannelMemberInfo {
    /// Member's GUID
    pub guid: ObjectGuid,
    /// Member flags (owner, moderator, muted, etc.)
    pub flags: u8,
}

/// SMSG_CHANNEL_LIST - List of channel members
///
/// Sent in response to /who channel or channel member list requests.
///
/// ## Packet Format (Vanilla 1.12.1)
/// - channel_name (cstring)
/// - channel_flags (u8)
/// - member_count (u32)
/// - members[member_count]:
///   - guid (u64)
///   - member_flags (u8)
#[derive(Debug, Clone)]
pub struct SmsgChannelList<'a> {
    /// Channel name
    pub channel_name: &'a str,
    /// Channel flags
    pub channel_flags: u8,
    /// List of members
    pub members: &'a [ChannelMemberInfo],
}

impl ToWorldPacket for SmsgChannelList<'_> {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CHANNEL_LIST);

        // Write channel name
        packet.write_cstring(self.channel_name);

        // Write channel flags
        packet.write_u8(self.channel_flags);

        // Write member count
        packet.write_u32(self.members.len() as u32);

        // Write each member
        for member in self.members {
            packet.write_guid_raw(member.guid.raw());
            packet.write_u8(member.flags);
        }

        packet
    }

    /// `ChannelListResponse::Write`.
    ///
    /// The channel name **moves to the end of the header**: 1.14 writes its 7-bit length up front
    /// alongside a display bit, then the flags and the member count, and only then the name bytes,
    /// with the member list after that. Vanilla leads with the name as a C string. Writing the name
    /// where vanilla puts it makes the client read the first few characters as its flags and count.
    ///
    /// Each member gains a realm address between the GUID and the flags byte.
    ///
    /// `Display` controls whether the client prints the roster or just refreshes it silently.
    /// Vanilla has no such bit and only ever sends this list in answer to an explicit request, so
    /// `true` is what the request asked for.
    fn to_modern(&self) -> Option<WorldPacket> {
        let name = self.channel_name.as_bytes();

        let mut writer = BitWriter::new();
        writer.write_bit(true); // Display -- see above
        writer.write_bits(name.len() as u32, 7);

        // See `CHANNEL_FLAGS_NONE`: the 1.12 and 1.14 flag sets are unrelated enums, so
        // `self.channel_flags` is deliberately not forwarded.
        writer.write_u32(CHANNEL_FLAGS_NONE);
        writer.write_i32(self.members.len() as i32);
        writer.write_bytes(name);

        for member in self.members {
            let (high, low) = member.guid.to_guid128(DEFAULT_REALM_ID);
            writer.write_packed_guid_128(high, low);
            writer.write_u32(VIRTUAL_REALM_ADDRESS);
            // Member flags (owner / moderator / muted) keep vanilla's bit values, so this byte
            // carries over unchanged -- unlike the channel flags above.
            writer.write_u8(member.flags);
        }

        Some(writer.finish(Opcode::SMSG_CHANNEL_LIST))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smsg_channel_notify_you_joined() {
        let msg = SmsgChannelNotify::you_joined("General", 0x10);
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_NOTIFY);
    }

    #[test]
    fn test_smsg_channel_notify_you_left() {
        let msg = SmsgChannelNotify::you_left("General");
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_NOTIFY);
    }

    #[test]
    fn test_smsg_channel_notify_player_joined() {
        let msg = SmsgChannelNotify::player_joined("General", ObjectGuid::from_low(1));
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_NOTIFY);
    }

    #[test]
    fn test_smsg_channel_notify_player_kicked() {
        let kicked = ObjectGuid::from_low(2);
        let kicker = ObjectGuid::from_low(1);
        let msg = SmsgChannelNotify::player_kicked("MyChannel", kicked, kicker);
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_NOTIFY);
    }

    #[test]
    fn test_smsg_channel_notify_mode_change() {
        let msg = SmsgChannelNotify::mode_change("MyChannel", ObjectGuid::from_low(1), 0x00, 0x04);
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_NOTIFY);
    }

    #[test]
    fn test_smsg_channel_list() {
        let members = vec![
            ChannelMemberInfo {
                guid: ObjectGuid::from_low(1),
                flags: 0x01, // Owner
            },
            ChannelMemberInfo {
                guid: ObjectGuid::from_low(2),
                flags: 0x00,
            },
        ];
        let msg = SmsgChannelList {
            channel_name: "General",
            channel_flags: 0x10,
            members: &members,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_CHANNEL_LIST);
    }
}
