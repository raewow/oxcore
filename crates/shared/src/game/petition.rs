#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetitionType {
    Guild = 1,
    ArenaTeam = 2,
}

impl PetitionType {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetitionResult {
    Ok = 0,
    AlreadySigned = 1,
    NoSignature = 2,
    TooManySignatures = 3,
    NoSuchPetition = 4,
    NoSuchPetitionSignature = 5,
    AlreadyInGuild = 6,
    CannotSignSameGuild = 7,
    PlayerNotFound = 8,
    NotEligible = 9,
}

impl PetitionResult {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone)]
pub struct PetitionInfo {
    pub guid: crate::shared::protocol::ObjectGuid,
    pub petition_id: u32,
    pub petition_name: String,
    pub deadline: u32,
    pub creator_guid: crate::shared::protocol::ObjectGuid,
    pub signs: u8,
    pub min_signatures: u8,
    pub max_signatures: u8,
    pub petition_type: PetitionType,
    pub allow_modification: bool,
}

impl PetitionInfo {
    pub fn new(guid: crate::shared::protocol::ObjectGuid) -> Self {
        Self {
            guid,
            petition_id: 0,
            petition_name: String::new(),
            deadline: 0,
            creator_guid: crate::shared::protocol::ObjectGuid::empty(),
            signs: 0,
            min_signatures: 0,
            max_signatures: 0,
            petition_type: PetitionType::Guild,
            allow_modification: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PetitionSignature {
    pub player_guid: crate::shared::protocol::ObjectGuid,
    pub player_account: u32,
    pub name: String,
    pub offer_result: PetitionResult,
}

impl PetitionSignature {
    pub fn new(player_guid: crate::shared::protocol::ObjectGuid) -> Self {
        Self {
            player_guid,
            player_account: 0,
            name: String::new(),
            offer_result: PetitionResult::Ok,
        }
    }
}
