#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BattleGroundTypeId {
    None = 0,
    AlteracValley = 1,
    WarsongGulch = 2,
    ArathiBasin = 3,
    EyeOfTheStorm = 4,
}

impl BattleGroundTypeId {
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn map_id(self) -> u32 {
        match self {
            BattleGroundTypeId::AlteracValley => 30,
            BattleGroundTypeId::WarsongGulch => 489,
            BattleGroundTypeId::ArathiBasin => 529,
            BattleGroundTypeId::EyeOfTheStorm => 566,
            BattleGroundTypeId::None => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BattleGroundStatus {
    None = 0,
    WaitJoin = 1,
    InProgress = 2,
    WaitLeave = 3,
}

impl BattleGroundStatus {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BattleGroundWinner {
    None = 0,
    Alliance = 1,
    Horde = 2,
}

impl BattleGroundWinner {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone)]
pub struct BattleGroundPlayer {
    pub guid: crate::shared::protocol::ObjectGuid,
    pub name: String,
    pub team: crate::shared::game::chat::Team,
    pub score: BattleGroundScore,
    pub is_afk: bool,
    pub is_disconnected: bool,
    pub death_count: u32,
    pub kill_count: u32,
    pub honor_gained: u32,
    pub contribution_points: u32,
}

impl BattleGroundPlayer {
    pub fn new(
        guid: crate::shared::protocol::ObjectGuid,
        team: crate::shared::game::chat::Team,
    ) -> Self {
        Self {
            guid,
            name: String::new(),
            team,
            score: BattleGroundScore::default(),
            is_afk: false,
            is_disconnected: false,
            death_count: 0,
            kill_count: 0,
            honor_gained: 0,
            contribution_points: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BattleGroundScore {
    pub killing_blows: u32,
    pub deaths: u32,
    pub honor_kills: u32,
    pub bonus_honor: u32,
    pub damage_done: u32,
    pub healing_done: u32,
    pub flags_captured: u32,
    pub flags_defended: u32,
    pub mines_captured: u32,
    pub towers_captured: u32,
    pub towers_defended: u32,
    pub bases_assaulted: u32,
    pub bases_defended: u32,
}

impl BattleGroundScore {
    pub fn reset(&mut self) {
        *self = BattleGroundScore::default();
    }
}
