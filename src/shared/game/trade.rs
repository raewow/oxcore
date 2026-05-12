pub const TRADE_DISTANCE_YARDS: f32 = 11.11;
pub const TRADE_DISTANCE_METERS: f32 = TRADE_DISTANCE_YARDS * 0.9144;

pub const TRADE_SLOT_COUNT: usize = 7;
pub const TRADE_SLOT_TRADED_COUNT: usize = 6;
pub const TRADE_SLOT_NONTRADED: usize = 6;
pub const TRADE_SLOT_INVALID: u8 = 0xFF;

pub const TRADE_SCAM_PREVENTION_DELAY_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TradeStatus {
    Busy = 0,
    BeginTrade = 1,
    OpenWindow = 2,
    TradeCanceled = 3,
    TradeAccept = 4,
    Busy2 = 5,
    NoTarget = 6,
    BackToTrade = 7,
    TradeComplete = 8,
    TradeRejected = 9,
    TargetTooFar = 10,
    WrongFaction = 11,
    CloseWindow = 12,
    Unknown13 = 13,
    IgnoreYou = 14,
    YouStunned = 15,
    TargetStunned = 16,
    YouDead = 17,
    TargetDead = 18,
    YouLogout = 19,
    TargetLogout = 20,
    TrialAccount = 21,
    OnlyConjured = 22,
    NotOnTaplist = 23,
}
