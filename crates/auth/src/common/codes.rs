/// Authentication command opcodes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthCmd {
    LogonChallenge = 0x00,
    LogonProof = 0x01,
    ReconnectChallenge = 0x02,
    ReconnectProof = 0x03,
    RealmList = 0x10,
    XferInitiate = 0x30,
    XferData = 0x31,
    XferAccept = 0x32,
    XferResume = 0x33,
    XferCancel = 0x34,
}

impl AuthCmd {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(AuthCmd::LogonChallenge),
            0x01 => Some(AuthCmd::LogonProof),
            0x02 => Some(AuthCmd::ReconnectChallenge),
            0x03 => Some(AuthCmd::ReconnectProof),
            0x10 => Some(AuthCmd::RealmList),
            0x30 => Some(AuthCmd::XferInitiate),
            0x31 => Some(AuthCmd::XferData),
            0x32 => Some(AuthCmd::XferAccept),
            0x33 => Some(AuthCmd::XferResume),
            0x34 => Some(AuthCmd::XferCancel),
            _ => None,
        }
    }
}

/// Connection status for authentication flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    Challenge,  // Waiting for initial challenge
    LogonProof, // Waiting for logon proof
    ReconProof, // Waiting for reconnect proof
    Patch,      // Patch transfer in progress
    Authed,     // Authenticated, can request realm list
    Closed,     // Connection closed
}

/// Authentication result codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    Success = 0x00,
    FailUnknownAccount = 0x04,
    FailIncorrectPassword = 0x05,
    FailAlreadyOnline = 0x06,
    FailNoTime = 0x07,
    FailDbBusy = 0x08,
    FailVersionInvalid = 0x09,
    FailVersionUpdate = 0x0A,
    FailInvalidServer = 0x0B,
    FailSuspended = 0x0C,
    FailFailNoaccess = 0x0D,
    SuccessSurvey = 0x0E,
    FailParentcontrol = 0x0F,
    FailLockedEnforced = 0x10,
    FailTrialEnded = 0x11,
    FailUseBattlenet = 0x12,
    FailAntiIndulgence = 0x13,
    FailExpired = 0x14,
    FailNoGameTime = 0x15,
    FailChargeback = 0x16,
    FailInternetGameRoomWithBnet = 0x17,
    FailGameAccountLocked = 0x18,
    FailUnlockableLock = 0x19,
    FailConversionRequired = 0x20,
    FailDisconnected = 0xFF,
}

/// Lock flags for account security
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockFlag {
    None = 0x00,
    IpLock = 0x01,
    FixedPin = 0x02,
    Totp = 0x04,
    AlwaysEnforce = 0x08,
    GeoCountry = 0x10,
    GeoCity = 0x20,
}

impl LockFlag {
    pub fn from_u32(value: u32) -> u32 {
        value // Return as-is for bitwise operations
    }

    pub fn has_flag(flags: u32, flag: LockFlag) -> bool {
        (flags & flag as u32) != 0
    }
}
