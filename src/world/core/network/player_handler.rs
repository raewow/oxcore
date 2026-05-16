use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

use crate::shared::database::Databases;
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::session::WorldSession;
use crate::world::handlers::dispatch_packet;
use crate::world::world::World;

/// Per-player packet handler with ordering guarantees
pub struct PlayerPacketHandler {
    /// Channel for sending packets to handler task
    packet_tx: mpsc::Sender<PlayerPacket>,
    /// Handler task handle
    _handle: JoinHandle<()>,
    /// Performance metrics
    metrics: Arc<HandlerMetrics>,
}

struct PlayerPacket {
    packet: WorldPacket,
    received_at: Instant,
}

struct HandlerMetrics {
    packets_processed: AtomicU64,
    queue_depth: AtomicUsize,
    max_queue_time_ms: AtomicU64,
}

impl PlayerPacketHandler {
    /// Spawn a new handler task for a player
    pub fn spawn(
        player_guid: ObjectGuid,
        session: Arc<WorldSession>,
        databases: Arc<Databases>,
        world: Arc<World>,
    ) -> Self {
        let (packet_tx, mut packet_rx) = mpsc::channel::<PlayerPacket>(100);
        let metrics = Arc::new(HandlerMetrics {
            packets_processed: AtomicU64::new(0),
            queue_depth: AtomicUsize::new(0),
            max_queue_time_ms: AtomicU64::new(0),
        });

        let metrics_clone = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            debug!(
                "[HANDLER] Started packet handler for player {}",
                player_guid
            );

            // Process packets in FIFO order
            while let Some(player_packet) = packet_rx.recv().await {
                let start = Instant::now();
                metrics_clone.queue_depth.fetch_sub(1, Ordering::SeqCst);

                // Track queue latency
                let queue_time = start.duration_since(player_packet.received_at);
                if queue_time.as_millis() > 3 {
                    // Lowered from 10ms to 3ms to catch sub-threshold delays
                    warn!(
                        "[HANDLER] Packet {:?} queued for {}ms for player {}",
                        player_packet.packet.opcode(),
                        queue_time.as_millis(),
                        player_guid
                    );
                }

                // Update max queue time metric
                let queue_ms = queue_time.as_millis() as u64;
                metrics_clone
                    .max_queue_time_ms
                    .fetch_max(queue_ms, Ordering::SeqCst);

                // Dispatch packet to handler
                let mut packet = player_packet.packet;
                let opcode = packet.opcode();
                let handler_start = Instant::now();
                if let Err(e) = dispatch_packet(&session, &mut packet, &databases, &world).await {
                    error!(
                        "[HANDLER] Error handling packet {:?} for player {}: {}",
                        opcode, player_guid, e
                    );
                }
                let handler_ms = handler_start.elapsed().as_millis();
                if handler_ms > 2 {
                    // Lowered from 5ms to 2ms to catch sub-threshold delays
                    warn!(
                        "[PERF] Handler {:?} took {}ms for player {}",
                        opcode, handler_ms, player_guid
                    );
                }

                // Monitor queue depth
                let current_depth = metrics_clone.queue_depth.load(Ordering::Relaxed);
                if current_depth > 20 {
                    warn!(
                        "[HANDLER] Player {} has {} packets queued (high queue depth)",
                        player_guid, current_depth
                    );
                }

                metrics_clone
                    .packets_processed
                    .fetch_add(1, Ordering::SeqCst);
            }

            debug!(
                "[HANDLER] Stopped packet handler for player {}",
                player_guid
            );
        });

        Self {
            packet_tx,
            _handle: handle,
            metrics,
        }
    }

    /// Send a packet to this handler (non-blocking)
    pub async fn send(&self, packet: WorldPacket) -> Result<()> {
        self.metrics.queue_depth.fetch_add(1, Ordering::SeqCst);

        self.packet_tx
            .send(PlayerPacket {
                packet,
                received_at: Instant::now(),
            })
            .await
            .map_err(|_| anyhow!("Handler task closed"))
    }
}
