use anyhow::Result;
use bytes::{BufMut, BytesMut};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

use crate::auth::common::codes::AuthCmd;

/// Chunk structure for patch data transfer
#[repr(C, packed)]
pub struct Chunk {
    pub cmd: u8,
    pub data_size: u16,
    pub data: [u8; 4096],
}

/// Patch transfer handler
/// Handles the actual file transfer of patch files to clients
pub struct PatchHandler {
    stream: TcpStream,
    patch_file: File,
}

impl PatchHandler {
    /// Create a new patch handler
    pub fn new(stream: TcpStream, patch_file: File) -> Self {
        Self { stream, patch_file }
    }

    /// Start the patch transfer
    /// Reads the patch file in chunks and sends them to the client
    pub async fn transfer(mut self) -> Result<()> {
        info!("Starting patch transfer");

        // Do 1 second sleep, similar to the one in game/WorldSocket.cpp
        // Seems client have problems with too fast sends
        sleep(Duration::from_secs(1)).await;

        let mut buffer = vec![0u8; 4096]; // 4KB chunks (page size on most arch)
        let mut chunk_buf = BytesMut::with_capacity(4096 + 3);

        loop {
            // Read chunk from file
            let n = match self.patch_file.read(&mut buffer).await {
                Ok(0) => {
                    debug!("Patch transfer complete (EOF)");
                    break; // EOF
                }
                Ok(n) => n,
                Err(e) => {
                    error!("Error reading patch file: {}", e);
                    return Err(e.into());
                }
            };

            // Build chunk packet: cmd (1 byte) + data_size (2 bytes) + data
            chunk_buf.clear();
            chunk_buf.put_u8(AuthCmd::XferData as u8);
            chunk_buf.put_u16_le(n as u16);
            chunk_buf.extend_from_slice(&buffer[..n]);

            // Send chunk to client
            if let Err(e) = self.stream.write_all(&chunk_buf).await {
                error!("Error sending patch chunk: {}", e);
                return Err(e.into());
            }

            // Flush to ensure data is sent
            if let Err(e) = self.stream.flush().await {
                error!("Error flushing patch data: {}", e);
                return Err(e.into());
            }

            debug!("Sent patch chunk: {} bytes", n);
        }

        info!("Patch transfer completed successfully");
        Ok(())
    }
}
