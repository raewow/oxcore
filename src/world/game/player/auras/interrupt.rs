//! Aura interrupt flags from Spell.dbc (SpellAuraInterruptFlags column)
//!
//! These flags determine when auras should be automatically removed.
//! Matches MaNGOS AURA_INTERRUPT_FLAG_* values for vanilla 1.12.1.

/// Aura interrupt flags from Spell.dbc (SpellAuraInterruptFlags column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuraInterruptFlags(pub u32);

impl AuraInterruptFlags {
    pub const NONE: Self = Self(0);
    pub const HITBYSPELL: Self = Self(0x00000001); // Bit 0: Hit by any spell
    pub const DAMAGE: Self = Self(0x00000002); // Bit 1: Taking damage
    pub const MOVING: Self = Self(0x00000008); // Bit 3: Moving
    pub const TURNING: Self = Self(0x00000010); // Bit 4: Turning
    pub const ENTER_COMBAT: Self = Self(0x00000020); // Bit 5: Entering combat
    pub const NOT_MOUNTED: Self = Self(0x00000040); // Bit 6: Not mounted
    pub const NOT_ABOVEWATER: Self = Self(0x00000080); // Bit 7: Not above water
    pub const NOT_UNDERWATER: Self = Self(0x00000100); // Bit 8: Not underwater
    pub const NOT_SHEATHED: Self = Self(0x00000200); // Bit 9: Not sheathed
    pub const TALK: Self = Self(0x00000400); // Bit 10: Talking to NPC
    pub const USE: Self = Self(0x00000800); // Bit 11: Using item/object
    pub const MELEE_ATTACK: Self = Self(0x00001000); // Bit 12: Melee attack
    pub const SPELL_ATTACK: Self = Self(0x00002000); // Bit 13: Spell attack
    pub const UNUSED14: Self = Self(0x00004000);
    pub const TRANSFORM: Self = Self(0x00008000); // Bit 15: Transform
    pub const UNUSED16: Self = Self(0x00010000);
    pub const MOUNT: Self = Self(0x00020000); // Bit 17: Mounting
    pub const STANDING_CANCELS: Self = Self(0x00040000); // Bit 18: Stand up (food/drink/sleep)
    pub const LEAVE_AREA: Self = Self(0x00080000); // Bit 19: Leaving area
    pub const INVULNERABILITY_BUFF: Self = Self(0x00100000); // Bit 20: Invulnerability
    pub const STEALTH: Self = Self(0x00200000); // Bit 21: Stealth
    pub const CAST: Self = Self(0x00400000); // Bit 22: Casting a spell
    pub const LANDING: Self = Self(0x00800000); // Bit 23: Landing from flight

    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}
