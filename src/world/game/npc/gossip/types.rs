//! Gossip data structures
//!
//! Defines the core data types for gossip menus, menu items, NPC text, and broadcast text.
//! These structures mirror the database tables: gossip_menu, gossip_menu_option, npc_text, broadcast_text.

/// Gossip menu (menu ID -> text ID mapping)
#[derive(Debug, Clone)]
pub struct GossipMenu {
    /// Menu entry ID
    pub entry: u32,
    /// Text ID (from npc_text table)
    pub text_id: u32,
    /// Script ID for custom script execution
    pub script_id: u32,
    /// Condition ID for menu visibility
    pub condition_id: u32,
}

/// A single gossip menu option
#[derive(Debug, Clone)]
pub struct GossipMenuItem {
    /// Menu ID this option belongs to
    pub menu_id: u32,
    /// Option ID within the menu
    pub id: u32,
    /// Icon ID (0-15, see GossipIcon enum)
    pub option_icon: u8,
    /// Display text
    pub option_text: String,
    /// Broadcast text ID for localization
    pub option_broadcast_text: u32,
    /// Option type (GOSSIP_OPTION_* constant)
    pub option_id: u32,
    /// Required NPC flags to show this option
    pub npc_option_npcflag: u32,
    /// Next menu ID (-1 = none, 0 = keep current)
    pub action_menu_id: i32,
    /// Point of Interest ID to show
    pub action_poi_id: u32,
    /// Script ID to execute
    pub action_script_id: u32,
    /// Whether this option requires text input
    pub box_coded: bool,
    /// Money cost in copper (for input box)
    pub box_money: u32,
    /// Input box prompt text
    pub box_text: String,
    /// Input box prompt broadcast text ID
    pub box_broadcast_text: u32,
    /// Condition ID for option visibility
    pub condition_id: u32,
}

/// NPC text option (one of 8 possible variants)
#[derive(Debug, Clone)]
pub struct NpcTextOption {
    /// Probability weight for random selection (0.0-1.0)
    pub probability: f32,
    /// Broadcast text ID for localization
    pub broadcast_text_id: u32,
}

/// NPC text entry (greeting text with up to 8 variants)
#[derive(Debug, Clone)]
pub struct NpcText {
    /// Text ID
    pub id: u32,
    /// 8 text options with probabilities
    pub options: [NpcTextOption; 8],
}

impl NpcText {
    /// Create a new NPC text entry with default (zero) options
    pub fn new(id: u32) -> Self {
        Self {
            id,
            options: [
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
                NpcTextOption {
                    probability: 0.0,
                    broadcast_text_id: 0,
                },
            ],
        }
    }
}

/// Broadcast text entry (localized text strings)
#[derive(Debug, Clone)]
pub struct BroadcastText {
    /// Entry ID
    pub entry: u32,
    /// Male text
    pub male_text: String,
    /// Female text
    pub female_text: String,
    /// Chat type
    pub chat_type: u8,
    /// Language ID
    pub language_id: u32,
    /// Sound ID
    pub sound_id: u32,
    /// Emote IDs (up to 3)
    pub emote_ids: [u32; 3],
    /// Emote delays (up to 3)
    pub emote_delays: [u32; 3],
}

/// Gossip option icon types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GossipIcon {
    /// Default chat bubble
    Chat = 0,
    /// Vendor/bag icon
    Vendor = 1,
    /// Flight master/taxi
    Taxi = 2,
    /// Trainer
    Trainer = 3,
    /// Spirit healer
    SpiritHealer = 4,
    /// Innkeeper/binder
    Binder = 5,
    /// Banker
    Banker = 6,
    /// Petition
    Petition = 7,
    /// Tabard designer
    Tabard = 8,
    /// Battlemaster
    Battlemaster = 9,
    /// Auctioneer
    Auctioneer = 10,
    /// Talent master
    TalentMaster = 11,
    /// Stable master
    Stablemaster = 12,
    /// Guild banker
    Guild = 13,
    /// Unlearn talents
    UnlearnTalents = 14,
    /// Arena
    Arena = 15,
}

impl Default for GossipIcon {
    fn default() -> Self {
        GossipIcon::Chat
    }
}

/// Gossip option type constants
pub mod gossip_option {
    /// None/invalid
    pub const NONE: u32 = 0;
    /// Standard gossip
    pub const GOSSIP: u32 = 1;
    /// Quest giver
    pub const QUESTGIVER: u32 = 2;
    /// Vendor
    pub const VENDOR: u32 = 3;
    /// Taxi/flight master
    pub const TAXIVENDOR: u32 = 4;
    /// Trainer
    pub const TRAINER: u32 = 5;
    /// Spirit healer
    pub const SPIRITHEALER: u32 = 6;
    /// Spirit guide
    pub const SPIRITGUIDE: u32 = 7;
    /// Innkeeper
    pub const INNKEEPER: u32 = 8;
    /// Banker
    pub const BANKER: u32 = 9;
    /// Petitioner
    pub const PETITIONER: u32 = 10;
    /// Tabard designer
    pub const TABARDDESIGNER: u32 = 11;
    /// Battlefield/battlemaster
    pub const BATTLEFIELD: u32 = 12;
    /// Auctioneer
    pub const AUCTIONEER: u32 = 13;
    /// Stable pet
    pub const STABLEPET: u32 = 14;
    /// Armorer (repair)
    pub const ARMORER: u32 = 15;
    /// Unlearn talents
    pub const UNLEARNTALENTS: u32 = 16;
    /// Unlearn pet skills
    pub const UNLEARNPETSKILLS: u32 = 17;
}

/// Maximum number of gossip menu items
pub const GOSSIP_MAX_MENU_ITEMS: usize = 15;

/// Default gossip message text ID
pub const DEFAULT_GOSSIP_MESSAGE: u32 = 0x7FFFFFFF;
