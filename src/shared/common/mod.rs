pub mod account_types;

pub use account_types::AccountType;

use std::sync::OnceLock;
use std::time::Instant;

static SERVER_START: OnceLock<Instant> = OnceLock::new();

/// Returns server uptime in milliseconds. The WoW 1.12 client expects movement block
/// timestamps to be in the same time domain as its internal timer (GetTickCount-style),
/// NOT unix epoch milliseconds. This keeps CREATE_OBJECT timestamps in a range the
/// client can work with relative to the game_time it received at SMSG_LOGIN_SETTIMESPEED.
pub fn server_mstime() -> u32 {
    SERVER_START.get_or_init(Instant::now).elapsed().as_millis() as u32
}
