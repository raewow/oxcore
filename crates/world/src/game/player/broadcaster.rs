//! PlayerBroadcaster - manages listeners for packet broadcasting
//!
//! Each player has a broadcaster that maintains a list of listeners (other players)
//! who should receive packets sent through BroadcastManager. The visibility system
//! manages the listener list based on who can see whom.

use crate::core::common::compress_update_packet_if_needed;
use crate::game::player::packet_queue::{PacketQueue, PacketQueueConfig};
use anyhow::Result;
use arc_swap::ArcSwap;
use oxcore_shared::messages::update::SmsgUpdateObject;
use oxcore_shared::messages::{Recipient, ToWorldPacket};
use oxcore_shared::protocol::{ObjectGuid, Opcode, Protocol, WorldPacket};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// PlayerBroadcaster - manages listeners for a player
#[derive(Debug)]
pub struct PlayerBroadcaster {
    packet_tx: mpsc::UnboundedSender<WorldPacket>,
    self_guid: ObjectGuid,
    /// Which protocol this player's client speaks. Held here because the broadcaster is the last
    /// point that still knows *who* a packet is for — by the time bytes reach the send queue the
    /// recipient is gone, so this is where a message must be turned into the right encoding.
    protocol: Protocol,
    /// Opcodes already reported as unported on this connection.
    ///
    /// An unported per-tick message (movement, object updates) would otherwise fill the log at
    /// tick rate and bury everything else, which is exactly what happened the first time a 1.14
    /// client reached the world.
    unported_seen: RwLock<HashSet<Opcode>>,
    /// Map this player is on, needed by modern object-update headers.
    ///
    /// Atomic because it changes on teleport, long after the broadcaster is behind an `Arc`.
    map_id: AtomicU16,
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

    /// Which protocol this player's client speaks.
    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// Set the protocol for this player's connection.
    pub fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
    }

    /// The map this player is on. Only modern object updates read it.
    pub fn map_id(&self) -> u16 {
        self.map_id.load(Ordering::Relaxed)
    }

    /// Record a map change, so later object updates carry the right map in their header.
    pub fn set_map_id(&self, map_id: u16) {
        self.map_id.store(map_id, Ordering::Relaxed);
    }

    /// Encode `msg` for this player's protocol.
    ///
    /// `None` means the message has no encoding for that protocol yet — see
    /// [`ToWorldPacket::to_modern`]. Callers log and drop rather than treating it as an error.
    pub fn encode(&self, msg: &dyn ToWorldPacket) -> Option<WorldPacket> {
        match self.protocol {
            Protocol::Vanilla => Some(msg.to_vanilla()),
            // Encoding happens here, per recipient, precisely because this is the last point that
            // still knows who the packet is for — see [`ToWorldPacket::to_modern_for`].
            Protocol::Modern => msg.to_modern_for(Recipient {
                guid: self.self_guid,
                map_id: self.map_id(),
            }),
        }
    }

    /// Encode `msg` for this player and send it immediately.
    pub fn send_msg(&self, msg: &dyn ToWorldPacket) {
        match self.encode(msg) {
            Some(packet) => self.send_encoded(packet),
            None => self.warn_unported(msg),
        }
    }

    /// Encode `msg` for this player and queue it for the next flush.
    pub fn queue_msg(&self, msg: &dyn ToWorldPacket) {
        match self.encode(msg) {
            Some(packet) => self.enqueue_encoded(packet),
            None => self.warn_unported(msg),
        }
    }

    /// Send an object update, compressing it only for clients that understand compression.
    ///
    /// `SMSG_COMPRESSED_UPDATE_OBJECT` wraps a *vanilla* body in zlib, so it is inseparable from
    /// the vanilla encoding — there is no way to compress first and choose a protocol later. A
    /// modern client gets the uncompressed body, which is also what HermesProxy sends: it inflates
    /// the legacy packet and forwards a plain `SMSG_UPDATE_OBJECT`.
    ///
    /// This exists because the visibility system is the one place that must compress, and routing
    /// it through the ordinary message path would silently drop every object update for a modern
    /// player — which is exactly what left 1.14 clients staring at a loading screen.
    pub fn send_update_object(&self, msg: &SmsgUpdateObject) -> Result<()> {
        match self.protocol {
            Protocol::Vanilla => {
                let compressed = compress_update_packet_if_needed(msg.to_vanilla())?;
                self.send_encoded(compressed);
            }
            Protocol::Modern => self.send_msg(msg),
        }
        Ok(())
    }

    /// Report a dropped message, naming the opcode.
    ///
    /// Encodes the vanilla body purely to read the opcode off it — the trait has no other way to
    /// ask a message what it is. That is wasted work, but only on the drop path, and a log line
    /// that does not say *which* message went missing is no help at all when a client renders a
    /// half-populated world.
    fn warn_unported(&self, msg: &dyn ToWorldPacket) {
        let opcode = msg.to_vanilla().opcode();
        if self.unported_seen.write().insert(opcode) {
            tracing::warn!(
                "No {:?} encoding for {:?}; dropping it for player {:?}. \
                 Further drops of this opcode on this connection are silent.",
                self.protocol,
                opcode,
                self.self_guid
            );
        } else {
            tracing::trace!("Dropping unported {:?} for {:?}", opcode, self.self_guid);
        }
    }

    /// Whether a packet handed to us pre-encoded can be sent as-is.
    ///
    /// A `WorldPacket` built by a caller is always vanilla bytes — nothing in the type records
    /// which protocol produced it. Putting those on a modern connection would be silent corruption
    /// the client can only report as a disconnect, so refuse and say which opcode did it. The
    /// remaining callers are the update-object and compression paths, which are vanilla-only by
    /// design; see `docs/modern-opcode-plan.md`.
    fn accepts_prebuilt(&self, packet: &WorldPacket) -> bool {
        if self.protocol == Protocol::Vanilla {
            return true;
        }
        tracing::warn!(
            "Refusing pre-encoded {:?} for modern player {:?}: the body is vanilla-only. \
             Send the message type instead so it can be encoded per protocol.",
            packet.opcode(),
            self.self_guid
        );
        false
    }

    pub fn with_config(
        packet_tx: mpsc::UnboundedSender<WorldPacket>,
        self_guid: ObjectGuid,
        queue_config: PacketQueueConfig,
    ) -> Self {
        Self {
            packet_tx,
            self_guid,
            protocol: Protocol::Vanilla,
            map_id: AtomicU16::new(0),
            unported_seen: RwLock::new(HashSet::new()),
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
    /// Send an already-encoded packet.
    ///
    /// Only safe for a vanilla recipient — see [`Self::accepts_prebuilt`]. Prefer
    /// [`Self::send_msg`], which encodes for whichever protocol this player speaks.
    pub fn send_direct(&self, packet: WorldPacket) {
        if !self.accepts_prebuilt(&packet) {
            return;
        }
        self.send_encoded(packet);
    }

    /// Send a packet already encoded for this player's protocol, skipping the pre-built check.
    fn send_encoded(&self, packet: WorldPacket) {
        tracing::trace!(
            "[PKT-DIRECT] opcode={:?} len={} to={:?}",
            packet.opcode(),
            packet.size(),
            self.self_guid
        );
        if self.packet_tx.send(packet).is_err() {
            debug!("Skipping send to disconnected player {:?}", self.self_guid);
        }
    }

    /// Queue a packet for later flushing (packet collapsing optimization)
    pub fn queue_packet(&self, packet: WorldPacket) {
        if !self.accepts_prebuilt(&packet) {
            return;
        }
        self.enqueue_encoded(packet);
    }

    /// Queue a packet already encoded for this player's protocol.
    fn enqueue_encoded(&self, packet: WorldPacket) {
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
    pub fn queue_metrics(&self) -> crate::game::player::packet_queue::PacketQueueMetrics {
        self.packet_queue.read().metrics()
    }

    pub fn free_at_logout(&self) {
        self.clear_listeners();
        self.packet_queue.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::messages::update::{ObjectType, UpdateBlockData, ValuesUpdateBlock};
    use oxcore_shared::protocol::HighGuid;

    fn broadcaster(
        protocol: Protocol,
    ) -> (PlayerBroadcaster, mpsc::UnboundedReceiver<WorldPacket>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let guid = ObjectGuid::new_without_entry(HighGuid::Player, 1);
        let mut broadcaster = PlayerBroadcaster::new(tx, guid);
        broadcaster.set_protocol(protocol);
        (broadcaster, rx)
    }

    /// A big enough update compresses for vanilla. `SMSG_COMPRESSED_UPDATE_OBJECT` wraps a vanilla
    /// body, so it must never reach a modern client -- that is unparseable to them, and it is what
    /// left 1.14 clients stuck on the loading screen.
    #[test]
    fn object_updates_compress_for_vanilla_and_not_for_modern() {
        let target = ObjectGuid::new_without_entry(HighGuid::Unit, 9);
        let mut msg = SmsgUpdateObject::new();
        // Enough blocks to cross the compression threshold.
        for field in 0..64 {
            msg = msg.add_block(UpdateBlockData::Values(
                ValuesUpdateBlock::new(target, ObjectType::Unit).set_field(22 + field, 1),
            ));
        }

        let (vanilla, mut vanilla_rx) = broadcaster(Protocol::Vanilla);
        vanilla.send_update_object(&msg).expect("sent");
        let sent = vanilla_rx.try_recv().expect("a packet was sent");
        assert_eq!(sent.opcode(), Opcode::SMSG_COMPRESSED_UPDATE_OBJECT);

        let (modern, mut modern_rx) = broadcaster(Protocol::Modern);
        modern.send_update_object(&msg).expect("sent");
        let sent = modern_rx.try_recv().expect("a packet was sent");
        assert_eq!(
            sent.opcode(),
            Opcode::SMSG_UPDATE_OBJECT,
            "a modern client cannot parse a zlib-wrapped vanilla body"
        );
    }

    /// The modern body must actually be built, not silently dropped -- a dropped self create-object
    /// leaves the client with no player to attach its camera to.
    #[test]
    fn a_modern_object_update_is_not_dropped() {
        let (modern, mut rx) = broadcaster(Protocol::Modern);
        modern
            .send_update_object(&SmsgUpdateObject::new())
            .expect("sent");
        assert!(rx.try_recv().is_ok(), "the update must reach the queue");
    }
}
