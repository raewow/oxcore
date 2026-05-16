//! Broadcast Manager - packet broadcasting using PlayerBroadcaster listeners
//!
//! Provides a high-level API for sending packets to players and broadcasting
//! to nearby players. Uses the PlayerBroadcaster listener system managed by
//! the visibility system.

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::session::SessionManager;
use crate::world::game::player::packet_queue::{categorize_opcode, PacketCategory};
use crate::world::game::player::PlayerManager;
use std::sync::Arc;

/// Trait abstraction for broadcast manager operations
/// Enables mocking for tests via mockall
///
/// NOTE: These are intentionally synchronous (non-blocking) for performance.
/// Uses try_send() internally - packets are dropped if channel is full.
#[cfg_attr(test, mockall::automock)]
pub trait BroadcastManagerTrait: Send + Sync {
    fn send_to_player(&self, player_guid: ObjectGuid, packet: WorldPacket);
    fn broadcast_to_players(&self, player_guids: &[ObjectGuid], packet: &WorldPacket);
    fn broadcast_nearby(&self, sender_guid: ObjectGuid, packet: &WorldPacket, include_self: bool);
}

/// Extension trait for sending messages directly (struct-based API)
/// This is a separate trait to avoid issues with trait objects and generics
pub trait BroadcastManagerExt: BroadcastManagerTrait {
    fn send_msg_to_player<T: ToWorldPacket + Send>(&self, player_guid: ObjectGuid, msg: T) {
        self.send_to_player(player_guid, msg.to_world_packet())
    }
}

// Blanket implementation for all types implementing BroadcastManagerTrait
// Note: ?Sized allows this to work with trait objects (dyn BroadcastManagerTrait)
impl<T: ?Sized + BroadcastManagerTrait> BroadcastManagerExt for T {}

pub struct BroadcastManager {
    session_mgr: Arc<SessionManager>,
    player_mgr: Arc<PlayerManager>,
}

impl BroadcastManager {
    pub fn new(session_mgr: Arc<SessionManager>, player_mgr: Arc<PlayerManager>) -> Self {
        Self {
            session_mgr,
            player_mgr,
        }
    }

    /// Send message to specific player using struct-based API (preferred)
    /// Non-blocking - uses try_send() internally
    pub fn send_msg_to_player<T: ToWorldPacket + Send>(&self, player_guid: ObjectGuid, msg: T) {
        if let Some(broadcaster) = self.player_mgr.get_broadcaster(player_guid) {
            broadcaster.send_direct(msg.to_world_packet());
        }
    }

    /// Send packet to specific player (non-blocking)
    pub fn send_to_player(&self, player_guid: ObjectGuid, packet: WorldPacket) {
        if let Some(broadcaster) = self.player_mgr.get_broadcaster(player_guid) {
            broadcaster.send_direct(packet);
        } else {
            tracing::debug!("No broadcaster found for player {:?}", player_guid);
        }
    }

    /// Broadcast packet to a list of specific players (non-blocking)
    pub fn broadcast_to_players(&self, player_guids: &[ObjectGuid], packet: &WorldPacket) {
        for &guid in player_guids {
            self.send_to_player(guid, packet.clone());
        }
    }

    /// Broadcast packet to all online players (non-blocking)
    pub fn broadcast_to_all(&self, packet: &WorldPacket) {
        let all_guids = self.session_mgr.get_all_sessions();
        for guid in all_guids {
            self.send_to_player(guid, packet.clone());
        }
    }

    /// Broadcast to nearby players using PlayerBroadcaster listeners (non-blocking)
    ///
    /// Uses smart routing based on packet category:
    /// - Critical packets (combat, chat) → send_direct (immediate)
    /// - Skippable/NonSkippable packets → queue_packet (collapsing enabled)
    ///
    /// TODO: Implement proper distance/visibility checks:
    /// - Say: 25 yards
    /// - Yell: map-wide
    /// - Emote: 25 yards
    /// Currently broadcasts to all players on same map via visibility system
    pub fn broadcast_nearby(
        &self,
        sender_guid: ObjectGuid,
        packet: &WorldPacket,
        include_self: bool,
    ) {
        if !sender_guid.is_player() {
            tracing::warn!(
                "broadcast_nearby called with non-player GUID {:?} — use broadcast_around_creature() instead",
                sender_guid
            );
        }
        let broadcaster = self.player_mgr.get_broadcaster(sender_guid);

        if let Some(broadcaster) = broadcaster {
            // Lock-free snapshot read - eliminates RwLock contention during combat
            let listeners = broadcaster.listener_snapshot().load_full();

            // Categorize packet for smart routing
            let category = categorize_opcode(packet.opcode());
            let use_queue = !matches!(category, PacketCategory::Critical);

            // Send to self if requested
            if include_self {
                if use_queue {
                    broadcaster.queue_packet(packet.clone());
                } else {
                    broadcaster.send_direct(packet.clone());
                }
            }

            // Send to all listeners
            for (_listener_guid, listener_broadcaster) in listeners.iter() {
                if use_queue {
                    listener_broadcaster.queue_packet(packet.clone());
                } else {
                    listener_broadcaster.send_direct(packet.clone());
                }
            }
        } else {
            tracing::debug!(
                "No broadcaster found for player {:?}, cannot broadcast",
                sender_guid
            );
        }
    }

    /// Broadcast to nearby players excluding self (convenience method for movement)
    pub fn broadcast_nearby_exclude_self(&self, sender_guid: ObjectGuid, packet: &WorldPacket) {
        self.broadcast_nearby(sender_guid, packet, false);
    }

    /// Broadcast to players within a specific distance range (non-blocking)
    ///
    /// Filters listeners by 2D distance (X/Y coordinates only) from sender.
    /// Used for distance-limited chat (Say: 25 yards, Yell: 300 yards, etc.)
    ///
    /// Returns the list of player GUIDs that received the packet (for logging/debugging)
    pub fn broadcast_to_distance(
        &self,
        sender_guid: ObjectGuid,
        packet: &WorldPacket,
        max_distance: f32,
        include_self: bool,
    ) -> Vec<ObjectGuid> {
        let mut recipients = Vec::new();

        // Get sender position from player manager
        let sender_pos = match self.player_mgr.get_position(sender_guid) {
            Some(position) => position,
            None => {
                tracing::debug!(
                    "Cannot broadcast_to_distance: sender {:?} position not found",
                    sender_guid
                );
                return recipients;
            }
        };

        let broadcaster = self.player_mgr.get_broadcaster(sender_guid);

        if let Some(broadcaster) = broadcaster {
            // Lock-free snapshot read - eliminates RwLock contention during combat
            let listeners = broadcaster.listener_snapshot().load_full();

            // Send to self if requested
            if include_self {
                broadcaster.send_direct(packet.clone());
                recipients.push(sender_guid);
            }

            // Filter listeners by distance and send
            for (listener_guid, listener_broadcaster) in listeners.iter() {
                // Get listener position from player manager
                if let Some(listener_pos) = self.player_mgr.get_position(*listener_guid) {
                    // Calculate 2D distance (X/Y only, ignore Z)
                    let distance = sender_pos.distance_2d(&listener_pos);

                    if distance <= max_distance {
                        listener_broadcaster.send_direct(packet.clone());
                        recipients.push(*listener_guid);
                    }
                }
            }
        } else {
            tracing::debug!(
                "No broadcaster found for player {:?}, cannot broadcast",
                sender_guid
            );
        }

        recipients
    }

    /// Flush all player packet queues (packet collapsing optimization)
    ///
    /// This should be called from the world update loop after map updates
    /// to send accumulated packets with collapsing applied.
    pub fn flush_all_queues(&self) {
        let guids: Vec<ObjectGuid> = self.player_mgr.iter().map(|entry| *entry.key()).collect();

        for guid in guids {
            if let Some(broadcaster) = self.player_mgr.get_broadcaster(guid) {
                broadcaster.flush_queue();
            }
        }
    }
}

impl BroadcastManagerTrait for BroadcastManager {
    fn send_to_player(&self, player_guid: ObjectGuid, packet: WorldPacket) {
        if let Some(broadcaster) = self.player_mgr.get_broadcaster(player_guid) {
            broadcaster.send_direct(packet);
        } else {
            tracing::debug!("No broadcaster found for player {:?}", player_guid);
        }
    }

    fn broadcast_to_players(&self, player_guids: &[ObjectGuid], packet: &WorldPacket) {
        for &guid in player_guids {
            self.send_to_player(guid, packet.clone());
        }
    }

    fn broadcast_nearby(&self, sender_guid: ObjectGuid, packet: &WorldPacket, include_self: bool) {
        // Delegate to the concrete implementation
        BroadcastManager::broadcast_nearby(self, sender_guid, packet, include_self)
    }
}

/// Broadcast a packet to all players near a creature (non-blocking)
///
/// `broadcast_nearby()` only works for player senders (creatures don't have
/// PlayerBroadcasters). This function looks up the creature's position and
/// uses the map's spatial query to find nearby players instead.
pub fn broadcast_around_creature(
    world: &crate::world::World,
    creature_guid: ObjectGuid,
    packet: &WorldPacket,
) {
    tracing::warn!(
        "[BROADCAST-TRACE] broadcast_around_creature START for {:?}, opcode {:?}",
        creature_guid,
        packet.opcode()
    );
    let t_creature_lookup = std::time::Instant::now();
    let creature_info = world
        .managers
        .creature_mgr
        .with_creature(creature_guid, |c| (c.position, c.map_id, c.instance_id));
    tracing::warn!(
        "[BROADCAST-TRACE] creature lookup took {}µs",
        t_creature_lookup.elapsed().as_micros()
    );

    if let Some((position, map_id, instance_id)) = creature_info {
        tracing::warn!(
            "[BROADCAST-TRACE] getting map for map_id {} instance_id {}",
            map_id,
            instance_id
        );
        let t_get_map = std::time::Instant::now();
        let map = world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id);
        tracing::warn!(
            "[BROADCAST-TRACE] get_or_create_map took {}µs",
            t_get_map.elapsed().as_micros()
        );

        // Use optimized early-exit query - limit to 50 players max to avoid full map scan
        // In combat scenarios, typically only 5-20 players are nearby, so this prevents
        // scanning 100+ players on populated maps
        tracing::warn!("[BROADCAST-TRACE] calling get_players_in_range_limit");
        let query_start = std::time::Instant::now();
        let nearby_players =
            map.get_players_in_range_limit(position, map.visibility_distance(), 50);
        let query_elapsed = query_start.elapsed();
        let nearby_count = nearby_players.len();
        tracing::warn!(
            "[BROADCAST-TRACE] get_players_in_range_limit found {} players in {}µs",
            nearby_count,
            query_elapsed.as_micros()
        );

        // Log performance warning if spatial query is slow (should be <100µs)
        if query_elapsed.as_micros() > 100 {
            tracing::warn!(
                "[PERF] broadcast_around_creature spatial query took {}µs for {} players",
                query_elapsed.as_micros(),
                nearby_count
            );
        }

        // Send to ALL nearby players rather than filtering by has_object_created.
        // The 1.12.1 client safely ignores VALUES updates for unknown GUIDs.
        // Previously this filter silently dropped combat/health updates when the
        // visibility system hadn't yet registered the creature (race condition).
        tracing::debug!(
            "[BROADCAST] creature {:?} at ({:.1}, {:.1}) map {} -> {} nearby in {}µs, opcode {:?}",
            creature_guid,
            position.x,
            position.y,
            map_id,
            nearby_count,
            query_elapsed.as_micros(),
            packet.opcode(),
        );
        tracing::warn!(
            "[BROADCAST-TRACE] calling broadcast_to_players with {} players",
            nearby_count
        );
        let t_broadcast = std::time::Instant::now();
        world
            .managers
            .broadcast_mgr
            .broadcast_to_players(&nearby_players, packet);
        tracing::warn!(
            "[BROADCAST-TRACE] broadcast_to_players took {}µs",
            t_broadcast.elapsed().as_micros()
        );
    } else {
        tracing::warn!(
            "[BROADCAST] creature {:?} not found in creature_mgr, skipping broadcast",
            creature_guid,
        );
    }
    tracing::warn!("[BROADCAST-TRACE] broadcast_around_creature COMPLETE");
}
