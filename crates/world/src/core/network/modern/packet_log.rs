//! Structured JSONL packet trace for modern connections.
//!
//! Transcribed from JimsProxy's structured logging (`Framework/Logging/Log.cs:59-250`): one JSON
//! object per line, `{timestamp_ms, eventType, payload}`, with dotted-lowercase event names, written
//! by a background thread so the hot path never blocks on the filesystem.
//!
//! Why a separate file rather than more `tracing` output: this is the tool for decoding a body the
//! 1.14 client rejected, which means every packet, with bytes. `world.log` already runs to tens of
//! megabytes per session from vmap and navmesh chatter, and burying a wire trace in it makes both
//! harder to read. Off unless `world.modern_packet_log` names a path.
//!
//! Events emitted:
//!
//! | event | meaning |
//! |---|---|
//! | `session.start` | a modern connection reached the packet loop |
//! | `packet.tx` | server → client, with the plaintext body |
//! | `packet.rx` | client → server |
//! | `packet.untranslated` | an inbound opcode with no entry in the shared table |
//! | `packet.ignored` | inbound client chatter we knowingly do not act on |

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Handle to the writer thread, plus how much of each body to record.
struct PacketLog {
    lines: Sender<String>,
    body_bytes: usize,
}

static LOG: OnceLock<Option<PacketLog>> = OnceLock::new();

/// Open the trace file and start its writer thread. A no-op when `path` is empty.
///
/// Called once at startup. Failing to open the file disables the trace rather than failing the
/// server: this is a diagnostic, and losing it should not cost anyone their realm.
pub fn init(path: &str, body_bytes: usize) {
    let _ = LOG.set(open(path, body_bytes));
}

fn open(path: &str, body_bytes: usize) -> Option<PacketLog> {
    if path.is_empty() {
        return None;
    }

    let file = match File::create(path) {
        Ok(file) => file,
        Err(e) => {
            tracing::warn!("could not open modern packet log at {path}: {e}; tracing disabled");
            return None;
        }
    };

    let (lines, rx) = mpsc::channel::<String>();
    // A dedicated OS thread, not a tokio task: the writes are blocking, and the send side is called
    // from inside the connection loop where blocking would stall packet delivery.
    std::thread::Builder::new()
        .name("modern-packet-log".into())
        .spawn(move || {
            let mut writer = BufWriter::new(file);
            // Buffered, flushed per line. A trace whose last few lines are missing is useless
            // precisely when it matters -- the packets just before a disconnect.
            for line in rx {
                if writer.write_all(line.as_bytes()).is_err() || writer.flush().is_err() {
                    break;
                }
            }
        })
        .ok()?;

    tracing::info!("modern packet trace writing to {path} ({body_bytes} body bytes per packet)");
    Some(PacketLog { lines, body_bytes })
}

/// Whether tracing is on, so callers can skip building a payload they would throw away.
pub fn enabled() -> bool {
    LOG.get().map(Option::is_some).unwrap_or(false)
}

/// Record one packet in either direction.
///
/// `opcode` is the logical name where the table knows it, so a trace can be read without a lookup
/// table; `wire` is the raw modern value, which is the only identifier an untranslated packet has.
pub fn packet(event: &str, connection: &str, opcode: &str, wire: u16, body: &[u8]) {
    let Some(Some(log)) = LOG.get() else {
        return;
    };

    let shown = body.len().min(log.body_bytes);
    let mut hex = String::with_capacity(shown * 2);
    for byte in &body[..shown] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }

    // Hand-rolled rather than pulling in serde_json for four fields. All values are either numbers
    // or hex/identifier strings, so none of them can need escaping.
    let line = format!(
        r#"{{"timestamp_ms":{},"eventType":"{}","payload":{{"connection":"{}","opcode":"{}","wire":"0x{:04X}","len":{},"truncated":{},"body":"{}"}}}}{}"#,
        now_ms(),
        event,
        connection,
        opcode,
        wire,
        body.len(),
        body.len() > shown,
        hex,
        '\n',
    );
    // A full channel or a dead writer thread must not take the connection down with it.
    let _ = log.lines.send(line);
}

/// Record a non-packet event, such as a connection starting.
pub fn event(event: &str, connection: &str, detail: &str) {
    let Some(Some(log)) = LOG.get() else {
        return;
    };
    let line = format!(
        r#"{{"timestamp_ms":{},"eventType":"{}","payload":{{"connection":"{}","detail":"{}"}}}}{}"#,
        now_ms(),
        event,
        connection,
        detail,
        '\n',
    );
    let _ = log.lines.send(line);
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
