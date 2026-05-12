//! Per-player settings state
//!
//! Embedded in the Player struct. Contains action buttons, macros,
//! tutorial flags, and account data blobs.

/// Action button type constants matching the 1.12 client protocol.
/// The type occupies the upper 8 bits of the packed u32 sent in SMSG_ACTION_BUTTONS.
pub const ACTION_BUTTON_SPELL: u8 = 0; // action = spell_id
pub const ACTION_BUTTON_MACRO: u8 = 64; // action = macro_index
pub const ACTION_BUTTON_ITEM: u8 = 128; // action = item_id

/// Maximum action bar slots the 1.12 client supports.
pub const MAX_ACTION_BUTTONS: usize = 120;

/// Maximum macros the 1.12 client allows per character.
pub const MAX_MACROS: usize = 18;

/// Number of tutorial flag u32 words (8 words * 32 bits = 256 flags).
pub const TUTORIAL_FLAG_COUNT: usize = 8;

/// Number of account data types (0-7).
pub const NUM_ACCOUNT_DATA_TYPES: usize = 8;

/// A single action bar button binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionButton {
    /// Spell ID, item ID, or macro index depending on `button_type`.
    pub action: u32,
    /// Discriminant: 0 = spell, 64 = macro, 128 = item.
    pub button_type: u8,
}

impl ActionButton {
    /// Pack into the wire format: action in lower 24 bits, type in upper 8 bits.
    pub fn pack(&self) -> u32 {
        (self.action & 0x00FFFFFF) | ((self.button_type as u32) << 24)
    }

    /// Unpack from wire format.
    pub fn unpack(packed: u32) -> Option<Self> {
        let action = packed & 0x00FFFFFF;
        let button_type = ((packed >> 24) & 0xFF) as u8;
        if action == 0 && button_type == 0 {
            None // Empty slot
        } else {
            Some(Self {
                action,
                button_type,
            })
        }
    }
}

/// A single macro entry.
#[derive(Debug, Clone)]
pub struct MacroEntry {
    /// Display name (max 16 characters in the 1.12 client).
    pub name: String,
    /// Icon index into the client's icon atlas.
    pub icon: u8,
    /// Macro body text (slash commands, max ~255 bytes).
    pub body: String,
}

/// Metadata for one account data blob.
#[derive(Debug, Clone)]
pub struct AccountDataEntry {
    /// Unix timestamp of last modification.
    pub time: u32,
    /// Raw data blob (compressed on the wire, stored decompressed).
    pub data: Vec<u8>,
}

/// Per-player settings state, embedded in the Player struct.
#[derive(Debug, Clone)]
pub struct SettingsState {
    /// 120 action bar buttons. Index = button slot (0-119).
    pub action_buttons: [Option<ActionButton>; MAX_ACTION_BUTTONS],

    /// Character macros. Vec length <= MAX_MACROS.
    pub macros: Vec<MacroEntry>,

    /// Tutorial completion bitflags (8 u32 words = 256 bits).
    pub tutorial_flags: [u32; TUTORIAL_FLAG_COUNT],

    /// Account data blobs, indexed by type (0-7).
    /// Types 0, 2, 4 are account-wide; types 1, 3, 5, 6, 7 are per-character.
    pub account_data: [Option<AccountDataEntry>; NUM_ACCOUNT_DATA_TYPES],

    /// Dirty flag: when true, the system writes state to DB on next save tick.
    pub need_save: bool,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            action_buttons: [None; MAX_ACTION_BUTTONS],
            macros: Vec::new(),
            tutorial_flags: [0u32; TUTORIAL_FLAG_COUNT],
            account_data: Default::default(),
            need_save: false,
        }
    }
}

impl SettingsState {
    /// Set a single action button.
    pub fn set_action_button(&mut self, slot: u8, action: u32, button_type: u8) {
        if (slot as usize) < MAX_ACTION_BUTTONS {
            self.action_buttons[slot as usize] = Some(ActionButton {
                action,
                button_type,
            });
            self.need_save = true;
        }
    }

    /// Clear a single action button.
    pub fn clear_action_button(&mut self, slot: u8) {
        if (slot as usize) < MAX_ACTION_BUTTONS {
            self.action_buttons[slot as usize] = None;
            self.need_save = true;
        }
    }

    /// Set a tutorial flag by bit index (0-255).
    pub fn set_tutorial_flag(&mut self, flag_index: u32) {
        let word = (flag_index / 32) as usize;
        let bit = flag_index % 32;
        if word < TUTORIAL_FLAG_COUNT {
            self.tutorial_flags[word] |= 1 << bit;
            self.need_save = true;
        }
    }

    /// Clear all tutorial flags (CMSG_TUTORIAL_CLEAR).
    pub fn clear_tutorial_flags(&mut self) {
        self.tutorial_flags = [0u32; TUTORIAL_FLAG_COUNT];
        self.need_save = true;
    }

    /// Reset tutorial flags to default (CMSG_TUTORIAL_RESET).
    /// Default = all zeros (all tutorials will re-show).
    pub fn reset_tutorial_flags(&mut self) {
        self.tutorial_flags = [0u32; TUTORIAL_FLAG_COUNT];
        self.need_save = true;
    }

    /// Mark all tutorials as completed (set all bits to 1).
    pub fn complete_all_tutorials(&mut self) {
        self.tutorial_flags = [0xFFFFFFFF; TUTORIAL_FLAG_COUNT];
        self.need_save = true;
    }
}
