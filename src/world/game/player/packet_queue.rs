//! Packet collapsing queue for reducing network overhead
//!
//! This module implements packet collapsing optimization where redundant movement
//! heartbeat packets are replaced by newer ones while preserving critical state
//! changes. This reduces network bandwidth and client CPU usage.

use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// Configuration for packet queue behavior
#[derive(Debug, Clone)]
pub struct PacketQueueConfig {
    pub max_skippable: usize,
    pub max_non_skippable: usize,
}

impl Default for PacketQueueConfig {
    fn default() -> Self {
        Self {
            max_skippable: 100,
            max_non_skippable: 200,
        }
    }
}

/// Packet category determines collapsing behavior
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketCategory {
    /// Can be replaced by newer packets (e.g., MSG_MOVE_HEARTBEAT)
    /// Uses key to identify replaceable packets
    Skippable { key: PacketKey },

    /// Must be sent but can be queued (e.g., MSG_MOVE_START_FORWARD)
    NonSkippable,

    /// Bypass queue, send immediately (e.g., combat, chat)
    Critical,
}

/// Key for identifying replaceable packets
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PacketKey {
    pub opcode: Opcode,
    pub source_guid: ObjectGuid,
}

impl PacketKey {
    pub fn new(opcode: Opcode, source_guid: ObjectGuid) -> Self {
        Self {
            opcode,
            source_guid,
        }
    }
}

/// Categorize an opcode for collapsing behavior
pub fn categorize_opcode(opcode: Opcode) -> PacketCategory {
    match opcode {
        // Critical: Movement packets - must be sent immediately for responsive feel
        // (matches old world system's send_direct_to_listeners approach)
        Opcode::MSG_MOVE_HEARTBEAT
        | Opcode::MSG_MOVE_START_FORWARD
        | Opcode::MSG_MOVE_START_BACKWARD
        | Opcode::MSG_MOVE_STOP
        | Opcode::MSG_MOVE_START_STRAFE_LEFT
        | Opcode::MSG_MOVE_START_STRAFE_RIGHT
        | Opcode::MSG_MOVE_STOP_STRAFE
        | Opcode::MSG_MOVE_JUMP
        | Opcode::MSG_MOVE_START_TURN_LEFT
        | Opcode::MSG_MOVE_START_TURN_RIGHT
        | Opcode::MSG_MOVE_STOP_TURN
        | Opcode::MSG_MOVE_SET_FACING
        | Opcode::MSG_MOVE_FALL_LAND => PacketCategory::Critical,

        // Critical: Combat, chat, spells (bypass queue)
        Opcode::SMSG_ATTACKSTART
        | Opcode::SMSG_ATTACKSTOP
        | Opcode::SMSG_ATTACKSWING_NOTINRANGE
        | Opcode::SMSG_ATTACKSWING_BADFACING
        | Opcode::SMSG_ATTACKSWING_DEADTARGET
        | Opcode::SMSG_ATTACKSWING_CANT_ATTACK
        | Opcode::SMSG_MESSAGECHAT
        | Opcode::SMSG_SPELL_START
        | Opcode::SMSG_SPELL_GO
        | Opcode::SMSG_SPELL_FAILURE => PacketCategory::Critical,

        // NonSkippable: Everything else (state changes, etc.)
        _ => PacketCategory::NonSkippable,
    }
}

/// Metrics for packet queue performance monitoring
#[derive(Debug, Clone, Default)]
pub struct PacketQueueMetrics {
    /// Number of packets that were replaced (collapsed)
    pub packets_collapsed: u64,

    /// Total number of packets queued
    pub packets_queued: u64,

    /// Total number of packets sent via flush
    pub packets_sent: u64,
}

/// Collapsing packet queue per player
///
/// This queue reduces network overhead by:
/// - Replacing skippable packets (heartbeats) with newer ones
/// - Queuing non-skippable packets in FIFO order
/// - Bypassing critical packets entirely
#[derive(Debug)]
pub struct PacketQueue {
    /// Latest skippable packet per key (HashMap replaces old with new)
    skippable: HashMap<PacketKey, WorldPacket>,

    /// Non-skippable packets (FIFO queue)
    non_skippable: VecDeque<WorldPacket>,

    /// Configuration limits
    config: PacketQueueConfig,

    /// Metrics for monitoring
    packets_collapsed: AtomicU64,
    packets_queued: AtomicU64,
    packets_sent: AtomicU64,
}

impl PacketQueue {
    /// Create a new packet queue with default configuration
    pub fn new() -> Self {
        Self::with_config(PacketQueueConfig::default())
    }

    /// Create a new packet queue with custom configuration
    pub fn with_config(config: PacketQueueConfig) -> Self {
        Self {
            skippable: HashMap::new(),
            non_skippable: VecDeque::new(),
            config,
            packets_collapsed: AtomicU64::new(0),
            packets_queued: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
        }
    }

    /// Enqueue a packet, applying collapsing logic based on category
    pub fn enqueue(&mut self, packet: WorldPacket) {
        self.packets_queued.fetch_add(1, Ordering::Relaxed);

        let category = categorize_opcode(packet.opcode());

        match category {
            PacketCategory::Skippable { mut key } => {
                // Extract source GUID from packet data if possible
                // For now, use opcode-only key (all heartbeats from same entity collapse)
                key.opcode = packet.opcode();

                // If we already have a packet with this key, we're collapsing
                if self.skippable.contains_key(&key) {
                    self.packets_collapsed.fetch_add(1, Ordering::Relaxed);
                }

                // Insert or replace packet
                self.skippable.insert(key, packet);
            }

            PacketCategory::NonSkippable => {
                self.non_skippable.push_back(packet);
            }

            PacketCategory::Critical => {
                // Critical packets should bypass queue entirely
                // This case shouldn't be reached as caller should check
                tracing::warn!(
                    "Critical packet {:?} queued (should bypass queue)",
                    packet.opcode()
                );
                self.non_skippable.push_back(packet);
            }
        }
    }

    /// Check if queue should be force flushed (safety valve)
    pub fn should_force_flush(&self) -> bool {
        self.non_skippable.len() >= self.config.max_non_skippable
            || self.skippable.len() >= self.config.max_skippable
    }

    /// Flush all queued packets and return them for sending
    ///
    /// Skippable packets are sent first (oldest data), then non-skippable (FIFO)
    pub fn flush(&mut self) -> Vec<WorldPacket> {
        let mut packets = Vec::new();

        // First, send skippable packets (latest versions)
        for (_key, packet) in self.skippable.drain() {
            packets.push(packet);
        }

        // Then send non-skippable packets in FIFO order
        while let Some(packet) = self.non_skippable.pop_front() {
            packets.push(packet);
        }

        let count = packets.len() as u64;
        self.packets_sent.fetch_add(count, Ordering::Relaxed);

        packets
    }

    /// Clear all queued packets (e.g., on player disconnect)
    pub fn clear(&mut self) {
        self.skippable.clear();
        self.non_skippable.clear();
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.skippable.is_empty() && self.non_skippable.is_empty()
    }

    /// Get current queue sizes
    pub fn sizes(&self) -> (usize, usize) {
        (self.skippable.len(), self.non_skippable.len())
    }

    /// Get metrics for monitoring
    pub fn metrics(&self) -> PacketQueueMetrics {
        PacketQueueMetrics {
            packets_collapsed: self.packets_collapsed.load(Ordering::Relaxed),
            packets_queued: self.packets_queued.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
        }
    }
}

impl Default for PacketQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_packet(opcode: Opcode) -> WorldPacket {
        WorldPacket::new(opcode)
    }

    #[test]
    fn test_categorization() {
        // Critical: Movement packets (sent immediately for responsiveness)
        assert_eq!(
            categorize_opcode(Opcode::MSG_MOVE_HEARTBEAT),
            PacketCategory::Critical
        );
        assert_eq!(
            categorize_opcode(Opcode::MSG_MOVE_START_FORWARD),
            PacketCategory::Critical
        );

        // Critical: Combat, chat, spells
        assert_eq!(
            categorize_opcode(Opcode::SMSG_ATTACKSTART),
            PacketCategory::Critical
        );
        assert_eq!(
            categorize_opcode(Opcode::SMSG_MESSAGECHAT),
            PacketCategory::Critical
        );

        // NonSkippable: Everything else
        assert_eq!(
            categorize_opcode(Opcode::SMSG_UPDATE_OBJECT),
            PacketCategory::NonSkippable
        );
    }

    #[test]
    fn test_non_skippable_fifo() {
        let mut queue = PacketQueue::new();

        // Queue 3 non-skippable update packets
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));

        // Should have 3 non-skippable packets
        assert_eq!(queue.non_skippable.len(), 3);
        assert_eq!(queue.metrics().packets_collapsed, 0);

        // Flush should return all 3 in FIFO order
        let packets = queue.flush();
        assert_eq!(packets.len(), 3);
    }

    #[test]
    fn test_empty_flush() {
        let mut queue = PacketQueue::new();

        assert!(queue.is_empty());

        let packets = queue.flush();
        assert_eq!(packets.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut queue = PacketQueue::new();

        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));

        assert!(!queue.is_empty());

        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.skippable.len(), 0);
        assert_eq!(queue.non_skippable.len(), 0);
    }

    #[test]
    fn test_force_flush_check() {
        let config = PacketQueueConfig {
            max_skippable: 5,
            max_non_skippable: 3,
        };
        let mut queue = PacketQueue::with_config(config);

        // Add packets up to limit (use non-movement, non-critical opcodes)
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        assert!(!queue.should_force_flush());

        // One more should trigger force flush
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        assert!(queue.should_force_flush());
    }

    #[test]
    fn test_metrics() {
        let mut queue = PacketQueue::new();

        // Queue some non-skippable packets
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));
        queue.enqueue(create_test_packet(Opcode::SMSG_UPDATE_OBJECT));

        let metrics = queue.metrics();
        assert_eq!(metrics.packets_queued, 3);
        assert_eq!(metrics.packets_collapsed, 0);
        assert_eq!(metrics.packets_sent, 0);

        // Flush
        let packets = queue.flush();
        assert_eq!(packets.len(), 3);

        let metrics = queue.metrics();
        assert_eq!(metrics.packets_sent, 3);
    }
}
