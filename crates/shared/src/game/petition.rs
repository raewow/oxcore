//! Guild charters.

/// Signatures a guild charter needs before it can be turned in.
pub const GUILD_CHARTER_SIGNATURES: u32 = 9;

/// The outcome of signing or turning in a charter, numbered as the 1.12 client reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetitionResult {
    Ok = 0,
    AlreadySigned = 1,
    AlreadyInGuild = 2,
    CantSignOwn = 3,
    NeedMoreSignatures = 4,
    NotServer = 5,
}

impl PetitionResult {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// One player's name on a charter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetitionSignature {
    pub player_guid: crate::protocol::ObjectGuid,
    pub player_account: u32,
}
