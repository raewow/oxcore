/// Account security levels (GM levels)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AccountType {
    /// Regular player (no special permissions)
    Player = 0,
    /// Moderator (basic GM permissions)
    Moderator = 1,
    /// Ticket master (handles GM tickets)
    TicketMaster = 2,
    /// Game Master (full GM permissions)
    GameMaster = 3,
    /// Basic Administrator
    BasicAdmin = 4,
    /// Developer
    Developer = 5,
    /// Administrator (full server access)
    Administrator = 6,
    /// Console (highest level, must be last)
    Console = 7,
}

impl AccountType {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => AccountType::Player,
            1 => AccountType::Moderator,
            2 => AccountType::TicketMaster,
            3 => AccountType::GameMaster,
            4 => AccountType::BasicAdmin,
            5 => AccountType::Developer,
            6 => AccountType::Administrator,
            7 => AccountType::Console,
            _ => {
                if value > 7 {
                    AccountType::Administrator
                } else {
                    AccountType::Player
                }
            }
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn is_gm(self) -> bool {
        self > AccountType::Player
    }

    pub fn can_accept_tickets(self) -> bool {
        self >= AccountType::GameMaster
    }

    pub fn can_show_gm_chat(self) -> bool {
        self >= AccountType::Moderator
    }
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Player
    }
}
