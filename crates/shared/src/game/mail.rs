#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MailMessageType {
    Normal = 0,
    Auction = 2,
    Creature = 3,
    GameObject = 4,
    Item = 5,
}

impl From<u8> for MailMessageType {
    fn from(value: u8) -> Self {
        match value {
            0 => MailMessageType::Normal,
            2 => MailMessageType::Auction,
            3 => MailMessageType::Creature,
            4 => MailMessageType::GameObject,
            5 => MailMessageType::Item,
            _ => MailMessageType::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailState {
    Unchanged = 1,
    Changed = 2,
    Deleted = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailCheckMask(u8);

impl MailCheckMask {
    pub const NONE: u8 = 0x00;
    pub const READ: u8 = 0x01;
    pub const RETURNED: u8 = 0x02;
    pub const COPIED: u8 = 0x04;
    pub const COD_PAYMENT: u8 = 0x08;
    pub const HAS_BODY: u8 = 0x10;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn has_read(&self) -> bool {
        self.0 & Self::READ != 0
    }

    pub fn has_returned(&self) -> bool {
        self.0 & Self::RETURNED != 0
    }

    pub fn has_copied(&self) -> bool {
        self.0 & Self::COPIED != 0
    }

    pub fn has_cod_payment(&self) -> bool {
        self.0 & Self::COD_PAYMENT != 0
    }

    pub fn has_body(&self) -> bool {
        self.0 & Self::HAS_BODY != 0
    }

    pub fn set_read(&mut self) {
        self.0 |= Self::READ;
    }

    pub fn set_copied(&mut self) {
        self.0 |= Self::COPIED;
    }

    pub fn set_cod_payment(&mut self) {
        self.0 |= Self::COD_PAYMENT;
    }

    pub fn set_has_body(&mut self) {
        self.0 |= Self::HAS_BODY;
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<u8> for MailCheckMask {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MailStationery {
    Unknown = 1,
    Default = 41,
    Gm = 61,
    Auction = 62,
    Val = 64,
}

impl From<u8> for MailStationery {
    fn from(value: u8) -> Self {
        match value {
            1 => MailStationery::Unknown,
            41 => MailStationery::Default,
            61 => MailStationery::Gm,
            62 => MailStationery::Auction,
            64 => MailStationery::Val,
            _ => MailStationery::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailResponseType {
    Send = 0,
    MoneyTaken = 1,
    ItemTaken = 2,
    ReturnedToSender = 3,
    Deleted = 4,
    MadePermanent = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailResponseResult {
    Ok = 0,
    EquipError = 1,
    CannotSendToSelf = 2,
    NotEnoughMoney = 3,
    RecipientNotFound = 4,
    NotYourTeam = 5,
    InternalError = 6,
    DisabledForTrialAcc = 14,
    RecipientCapReached = 15,
}

#[derive(Debug, Clone)]
pub struct MailItem {
    pub item_guid: u32,
    pub item_id: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct MailSender {
    pub message_type: MailMessageType,
    pub sender_id: u32,
    pub stationery: MailStationery,
}

impl MailSender {
    pub fn new_player(sender_guid: u32) -> Self {
        Self {
            message_type: MailMessageType::Normal,
            sender_id: sender_guid,
            stationery: MailStationery::Default,
        }
    }

    pub fn new_auction(auction_id: u32) -> Self {
        Self {
            message_type: MailMessageType::Auction,
            sender_id: auction_id,
            stationery: MailStationery::Auction,
        }
    }

    pub fn new_creature(creature_entry: u32) -> Self {
        Self {
            message_type: MailMessageType::Creature,
            sender_id: creature_entry,
            stationery: MailStationery::Default,
        }
    }

    pub fn new_gameobject(gameobject_entry: u32) -> Self {
        Self {
            message_type: MailMessageType::GameObject,
            sender_id: gameobject_entry,
            stationery: MailStationery::Default,
        }
    }

    pub fn new_system() -> Self {
        Self {
            message_type: MailMessageType::Normal,
            sender_id: 0,
            stationery: MailStationery::Default,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MailDraft {
    pub mail_template_id: u16,
    pub mail_template_items_need: bool,
    pub subject: String,
    pub body_id: u32,
    pub items: Vec<MailItem>,
    pub money: u32,
    pub cod: u32,
}

impl MailDraft {
    pub fn new() -> Self {
        Self {
            mail_template_id: 0,
            mail_template_items_need: false,
            subject: String::new(),
            body_id: 0,
            items: Vec::new(),
            money: 0,
            cod: 0,
        }
    }

    pub fn set_subject(&mut self, subject: String) {
        self.subject = subject;
    }

    pub fn set_body_id(&mut self, body_id: u32) {
        self.body_id = body_id;
    }

    pub fn set_money(&mut self, money: u32) {
        self.money = money;
    }

    pub fn set_cod(&mut self, cod: u32) {
        self.cod = cod;
    }

    pub fn add_item(&mut self, item_guid: u32, item_id: u32) {
        self.items.push(MailItem { item_guid, item_id });
    }
}

#[derive(Debug, Clone)]
pub struct Mail {
    pub id: u32,
    pub message_type: MailMessageType,
    pub stationery: i8,
    pub mail_template_id: u16,
    pub sender_guid: u32,
    pub receiver_guid: u32,
    pub subject: String,
    pub item_text_id: u32,
    pub has_items: bool,
    pub items: Vec<MailItem>,
    pub expire_time: i64,
    pub deliver_time: i64,
    pub money: u32,
    pub cod: u32,
    pub checked: u8,
    pub state: MailState,
    pub check_mask: MailCheckMask,
}

impl Mail {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            message_type: MailMessageType::Normal,
            stationery: 41,
            mail_template_id: 0,
            sender_guid: 0,
            receiver_guid: 0,
            subject: String::new(),
            item_text_id: 0,
            has_items: false,
            items: Vec::new(),
            expire_time: 0,
            deliver_time: 0,
            money: 0,
            cod: 0,
            checked: 0,
            state: MailState::Unchanged,
            check_mask: MailCheckMask::new(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.expire_time > 0 && now > self.expire_time
    }

    pub fn is_read(&self) -> bool {
        self.check_mask.has_read()
    }

    pub fn is_delivered(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.deliver_time > 0 && now >= self.deliver_time
    }

    pub fn can_delete(&self) -> bool {
        !self.check_mask.has_cod_payment()
    }
}
