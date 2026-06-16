pub mod response {
    pub const SUCCESS: u8 = 0x00;
    pub const FAILURE: u8 = 0x01;
    pub const CANCELLED: u8 = 0x02;
    pub const DISCONNECTED: u8 = 0x03;
    pub const FAILED_TO_CONNECT: u8 = 0x04;
    pub const CONNECTED: u8 = 0x05;
    pub const VERSION_MISMATCH: u8 = 0x06;
}

pub mod cstatus {
    pub const CONNECTING: u8 = 0x07;
    pub const NEGOTIATING_SECURITY: u8 = 0x08;
    pub const NEGOTIATION_COMPLETE: u8 = 0x09;
    pub const NEGOTIATION_FAILED: u8 = 0x0A;
    pub const AUTHENTICATING: u8 = 0x0B;
}

pub mod auth {
    pub const OK: u8 = 0x0C;
    pub const FAILED: u8 = 0x0D;
    pub const REJECT: u8 = 0x0E;
    pub const BAD_SERVER_PROOF: u8 = 0x0F;
    pub const UNAVAILABLE: u8 = 0x10;
    pub const SYSTEM_ERROR: u8 = 0x11;
    pub const BILLING_ERROR: u8 = 0x12;
    pub const BILLING_EXPIRED: u8 = 0x13;
    pub const VERSION_MISMATCH: u8 = 0x14;
    pub const UNKNOWN_ACCOUNT: u8 = 0x15;
    pub const INCORRECT_PASSWORD: u8 = 0x16;
    pub const SESSION_EXPIRED: u8 = 0x17;
    pub const SERVER_SHUTTING_DOWN: u8 = 0x18;
    pub const ALREADY_LOGGING_IN: u8 = 0x19;
    pub const LOGIN_SERVER_NOT_FOUND: u8 = 0x1A;
    pub const WAIT_QUEUE: u8 = 0x1B;
    pub const BANNED: u8 = 0x1C;
    pub const ALREADY_ONLINE: u8 = 0x1D;
    pub const NO_TIME: u8 = 0x1E;
    pub const DB_BUSY: u8 = 0x1F;
    pub const SUSPENDED: u8 = 0x20;
    pub const PARENTAL_CONTROL: u8 = 0x21;
}

pub mod realm_list {
    pub const IN_PROGRESS: u8 = 0x22;
    pub const SUCCESS: u8 = 0x23;
    pub const FAILED: u8 = 0x24;
    pub const INVALID: u8 = 0x25;
    pub const REALM_NOT_FOUND: u8 = 0x26;
}

pub mod account_create {
    pub const IN_PROGRESS: u8 = 0x27;
    pub const SUCCESS: u8 = 0x28;
    pub const FAILED: u8 = 0x29;
}

pub mod char_list {
    pub const RETRIEVING: u8 = 0x2A;
    pub const RETRIEVED: u8 = 0x2B;
    pub const FAILED: u8 = 0x2C;
}

pub mod char_create {
    pub const IN_PROGRESS: u8 = 0x2D;
    pub const SUCCESS: u8 = 0x2E;
    pub const ERROR: u8 = 0x2F;
    pub const FAILED: u8 = 0x30;
    pub const NAME_IN_USE: u8 = 0x31;
    pub const DISABLED: u8 = 0x32;
    pub const PVP_TEAMS_VIOLATION: u8 = 0x33;
    pub const SERVER_LIMIT: u8 = 0x34;
    pub const ACCOUNT_LIMIT: u8 = 0x35;
    pub const SERVER_QUEUE: u8 = 0x36;
    pub const ONLY_EXISTING: u8 = 0x37;
}

pub mod char_delete {
    pub const IN_PROGRESS: u8 = 0x38;
    pub const SUCCESS: u8 = 0x39;
    pub const FAILED: u8 = 0x3A;
    pub const FAILED_LOCKED_FOR_TRANSFER: u8 = 0x3B;
}

pub mod char_login {
    pub const IN_PROGRESS: u8 = 0x3C;
    pub const SUCCESS: u8 = 0x3D;
    pub const NO_WORLD: u8 = 0x3E;
    pub const DUPLICATE_CHARACTER: u8 = 0x3F;
    pub const NO_INSTANCES: u8 = 0x40;
    pub const FAILED: u8 = 0x41;
    pub const DISABLED: u8 = 0x42;
    pub const NO_CHARACTER: u8 = 0x43;
    pub const LOCKED_FOR_TRANSFER: u8 = 0x44;
}

pub mod char_name {
    pub const NO_NAME: u8 = 0x45;
    pub const TOO_SHORT: u8 = 0x46;
    pub const TOO_LONG: u8 = 0x47;
    pub const ONLY_LETTERS: u8 = 0x48;
    pub const MIXED_LANGUAGES: u8 = 0x49;
    pub const PROFANE: u8 = 0x4A;
    pub const RESERVED: u8 = 0x4B;
    pub const INVALID_APOSTROPHE: u8 = 0x4C;
    pub const MULTIPLE_APOSTROPHES: u8 = 0x4D;
    pub const THREE_CONSECUTIVE: u8 = 0x4E;
    pub const INVALID_SPACE: u8 = 0x4F;
    pub const SUCCESS: u8 = 0x50;
    pub const FAILURE: u8 = 0x51;
}
