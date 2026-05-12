//! Per-player movement buffer for map-level processing
//!
//! Socket tasks write movement packets into this buffer.
//! Map::update() reads and processes them during each tick,
//! with heartbeat coalescing to reduce redundant processing.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use parking_lot::Mutex;

use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::core::common::MovementInfo;

/// A single buffered movement packet, pre-parsed at push time
pub struct BufferedMovement {
    pub opcode: Opcode,
    pub movement_info: MovementInfo,
    pub received_at: Instant,
}

/// Classification for coalescing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    /// State changes (START_*, STOP_*, JUMP, SET_FACING, FALL_LAND)
    /// Must be preserved in FIFO order
    StateChange,
    /// Heartbeats (MSG_MOVE_HEARTBEAT)
    /// Only the latest per batch matters
    Heartbeat,
}

/// Per-player movement buffer (thread-safe)
///
/// Written to by socket tasks, read by map update loop.
/// Uses parking_lot::Mutex for brief lock holds.
pub struct MovementBuffer {
    inner: Mutex<MovementBufferInner>,
    player_guid: ObjectGuid,
    packets_received: AtomicU64,
    packets_coalesced: AtomicU64,
    packets_dropped_stale: AtomicU64,
}

struct MovementBufferInner {
    queue: VecDeque<BufferedMovement>,
    max_size: usize,
}

/// Maximum age before a movement packet is considered stale
const STALE_THRESHOLD: Duration = Duration::from_millis(500);

/// Maximum buffer size per player (safety valve)
const MAX_BUFFER_SIZE: usize = 128;

impl MovementBuffer {
    pub fn new(player_guid: ObjectGuid) -> Self {
        Self {
            inner: Mutex::new(MovementBufferInner {
                queue: VecDeque::with_capacity(32),
                max_size: MAX_BUFFER_SIZE,
            }),
            player_guid,
            packets_received: AtomicU64::new(0),
            packets_coalesced: AtomicU64::new(0),
            packets_dropped_stale: AtomicU64::new(0),
        }
    }

    /// Push a movement packet into the buffer.
    /// Pre-parses MovementInfo from the packet for efficient map-level processing.
    /// Called from the socket task.
    pub fn push(&self, opcode: Opcode, mut packet: WorldPacket) -> Result<()> {
        let mut movement_info = MovementInfo::read_from_packet(&mut packet)?;
        movement_info.mover_guid = self.player_guid;

        let mut inner = self.inner.lock();
        self.packets_received.fetch_add(1, Ordering::Relaxed);

        // Safety valve: drop oldest if buffer is full
        if inner.queue.len() >= inner.max_size {
            inner.queue.pop_front();
            self.packets_dropped_stale.fetch_add(1, Ordering::Relaxed);
        }

        inner.queue.push_back(BufferedMovement {
            opcode,
            movement_info,
            received_at: Instant::now(),
        });

        Ok(())
    }

    /// Drain and coalesce all buffered movements.
    /// Called from the map update loop each tick.
    ///
    /// Coalescing algorithm:
    /// - State changes are always preserved in FIFO order
    /// - Heartbeats: only the latest one per batch survives
    /// - Packets older than 500ms are dropped as stale
    pub fn drain_coalesced(&self) -> Vec<BufferedMovement> {
        let now = Instant::now();
        let mut inner = self.inner.lock();

        if inner.queue.is_empty() {
            return Vec::new();
        }

        let raw: VecDeque<BufferedMovement> = std::mem::take(&mut inner.queue);
        // Restore capacity for next tick
        inner.queue = VecDeque::with_capacity(32);
        drop(inner); // Release lock before processing

        let mut result: Vec<BufferedMovement> = Vec::with_capacity(raw.len());
        let mut last_heartbeat_idx: Option<usize> = None;

        for entry in raw {
            // Drop stale packets
            if now.duration_since(entry.received_at) > STALE_THRESHOLD {
                self.packets_dropped_stale.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            match classify_movement(entry.opcode) {
                MovementKind::StateChange => {
                    result.push(entry);
                }
                MovementKind::Heartbeat => {
                    if let Some(idx) = last_heartbeat_idx {
                        // Replace previous heartbeat with this newer one
                        result[idx] = entry;
                        self.packets_coalesced.fetch_add(1, Ordering::Relaxed);
                    } else {
                        last_heartbeat_idx = Some(result.len());
                        result.push(entry);
                    }
                }
            }
        }

        result
    }

    pub fn player_guid(&self) -> ObjectGuid {
        self.player_guid
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received.load(Ordering::Relaxed)
    }

    pub fn packets_coalesced(&self) -> u64 {
        self.packets_coalesced.load(Ordering::Relaxed)
    }

    pub fn packets_dropped_stale(&self) -> u64 {
        self.packets_dropped_stale.load(Ordering::Relaxed)
    }
}

/// Classify a movement opcode for coalescing purposes
pub fn classify_movement(opcode: Opcode) -> MovementKind {
    match opcode {
        Opcode::MSG_MOVE_HEARTBEAT => MovementKind::Heartbeat,
        _ => MovementKind::StateChange,
    }
}

/// Check if an opcode is a movement opcode that should go to the movement buffer
/// (bypassing the per-player handler task)
pub fn is_movement_opcode(opcode: Opcode) -> bool {
    matches!(
        opcode,
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
            | Opcode::MSG_MOVE_FALL_LAND
    )
}
