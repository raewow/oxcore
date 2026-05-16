//! PlayerBroadcaster - manages listeners for packet broadcasting
//!
//! Each player has a broadcaster that maintains a list of listeners (other players)
//! who should receive packets sent through BroadcastManager. The visibility system
//! manages the listener list based on who can see whom.

use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::game::player::packet_queue::{PacketQueue, PacketQueueConfig};
use arc_swap::ArcSwap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// PlayerBroadcaster - manages listeners for a player
#[derive(Debug)]
pub struct PlayerBroadcaster {
    packet_tx: mpsc::UnboundedSender<WorldPacket>,
    self_guid: ObjectGuid,
    listeners: Arc<RwLock<HashMap<ObjectGuid, Arc<PlayerBroadcaster>>>>,
    listener_snapshot: Arc<ArcSwap<Vec<(ObjectGuid, Arc<PlayerBroadcaster>)>>>,
    packet_queue: RwLock<PacketQueue>,
}

impl PlayerBroadcaster {
    pub fn listeners(&self) -> &Arc<RwLock<HashMap<ObjectGuid, Arc<PlayerBroadcaster>>>> {
        &self.listeners
    }

    /// Get a lock-free snapshot of listeners for broadcasting
    pub fn listener_snapshot(&self) -> &Arc<ArcSwap<Vec<(ObjectGuid, Arc<PlayerBroadcaster>)>>> {
        &self.listener_snapshot
    }

    /// Rebuild the listener snapshot after modifications (called by add/remove/clear)
    fn rebuild_snapshot(&self) {
        let listeners = self.listeners.read();
        let snapshot: Vec<_> = listeners.iter().map(|(k, v)| (*k, Arc::clone(v))).collect();
        self.listener_snapshot.store(Arc::new(snapshot));
    }

    pub fn new(packet_tx: mpsc::UnboundedSender<WorldPacket>, self_guid: ObjectGuid) -> Self {
        Self::with_config(packet_tx, self_guid, PacketQueueConfig::default())
    }

    pub fn with_config(
        packet_tx: mpsc::UnboundedSender<WorldPacket>,
        self_guid: ObjectGuid,
        queue_config: PacketQueueConfig,
    ) -> Self {
        Self {
            packet_tx,
            self_guid,
            listeners: Arc::new(RwLock::new(HashMap::new())),
            listener_snapshot: Arc::new(ArcSwap::new(Arc::new(Vec::new()))),
            packet_queue: RwLock::new(PacketQueue::with_config(queue_config)),
        }
    }

    pub fn guid(&self) -> ObjectGuid {
        self.self_guid
    }

    pub fn add_listener(
        &self,
        listener_guid: ObjectGuid,
        listener_broadcaster: Arc<PlayerBroadcaster>,
    ) {
        if listener_guid == self.self_guid {
            return;
        }

        let was_present = {
            let mut listeners = self.listeners.write();
            let was_present = listeners.contains_key(&listener_guid);
            listeners.insert(listener_guid, listener_broadcaster);
            was_present
        };

        // Rebuild snapshot after modification
        self.rebuild_snapshot();

        if !was_present {
            debug!(
                "Player {:?} added listener {:?}",
                self.self_guid, listener_guid
            );
        }
    }

    pub fn remove_listener(&self, listener_guid: ObjectGuid) {
        let was_removed = {
            let mut listeners = self.listeners.write();
            listeners.remove(&listener_guid).is_some()
        };

        if was_removed {
            // Rebuild snapshot after modification
            self.rebuild_snapshot();
            debug!(
                "Player {:?} removed listener {:?}",
                self.self_guid, listener_guid
            );
        }
    }

    pub fn clear_listeners(&self) {
        let count = {
            let mut listeners = self.listeners.write();
            let count = listeners.len();
            listeners.clear();
            count
        };

        // Rebuild snapshot after clearing
        self.rebuild_snapshot();
        debug!(
            "[BROADCASTER] Player {:?} cleared all listeners (was: {})",
            self.self_guid, count
        );
    }

    pub fn has_listener(&self, guid: ObjectGuid) -> bool {
        self.listeners.read().contains_key(&guid)
    }

    pub fn listener_count(&self) -> usize {
        self.listeners.read().len()
    }

    /// Send a packet directly to this player (non-blocking, guaranteed delivery)
    /// Uses unbounded channel - packets are never dropped, only fails if player disconnected
    pub fn send_direct(&self, packet: WorldPacket) {
        tracing::trace!(
            "[PKT-DIRECT] opcode={:?} len={} to={:?}",
            packet.opcode(),
            packet.size(),
            self.self_guid
        );
        if let Err(_) = self.packet_tx.send(packet) {
            debug!("Skipping send to disconnected player {:?}", self.self_guid);
        }
    }

    /// Queue a packet for later flushing (packet collapsing optimization)
    pub fn queue_packet(&self, packet: WorldPacket) {
        let mut queue = self.packet_queue.write();
        queue.enqueue(packet);

        // Safety valve: force flush if queue is too full
        if queue.should_force_flush() {
            tracing::warn!(
                "[BROADCASTER] Force flushing queue for player {:?} (queue full)",
                self.self_guid
            );
            drop(queue);
            // Note: Can't async flush from sync context, caller should flush soon
        }
    }

    /// Flush queued packets and send them via the packet channel (non-blocking, guaranteed delivery)
    /// Uses unbounded channel - packets are never dropped
    pub fn flush_queue(&self) {
        let packets = {
            let mut queue = self.packet_queue.write();
            if queue.is_empty() {
                return;
            }
            queue.flush()
        };

        for packet in packets {
            if let Err(_) = self.packet_tx.send(packet) {
                // Channel closed = player disconnected
                tracing::debug!(
                    "[BROADCASTER] Skipping flush to disconnected player {:?}",
                    self.self_guid
                );
                break;
            }
        }
    }

    /// Get packet queue metrics for monitoring
    pub fn queue_metrics(&self) -> crate::world::game::player::packet_queue::PacketQueueMetrics {
        self.packet_queue.read().metrics()
    }

    pub fn free_at_logout(&self) {
        self.clear_listeners();
        self.packet_queue.write().clear();
    }
}
