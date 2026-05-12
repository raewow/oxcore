//! Update field constants for Vanilla WoW 1.12.1
//! Based on UpdateFields_1_12_1.h (Build 5875)
//!
//! All field numbers are absolute (not offsets) for direct use in update packets

// Object Fields
pub const OBJECT_FIELD_GUID: u32 = 0x0;
pub const OBJECT_FIELD_TYPE: u32 = 0x2;
pub const OBJECT_FIELD_ENTRY: u32 = 0x3;
pub const OBJECT_FIELD_SCALE_X: u32 = 0x4;
pub const OBJECT_FIELD_PADDING: u32 = 0x5;
pub const OBJECT_END: u32 = 0x6;

// DynamicObject Fields
pub const DYNAMICOBJECT_CASTER: u32 = OBJECT_END + 0x0;
pub const DYNAMICOBJECT_BYTES: u32 = OBJECT_END + 0x2;
pub const DYNAMICOBJECT_SPELLID: u32 = OBJECT_END + 0x3;
pub const DYNAMICOBJECT_RADIUS: u32 = OBJECT_END + 0x4;
pub const DYNAMICOBJECT_POS_X: u32 = OBJECT_END + 0x5;
pub const DYNAMICOBJECT_POS_Y: u32 = OBJECT_END + 0x6;
pub const DYNAMICOBJECT_POS_Z: u32 = OBJECT_END + 0x7;
pub const DYNAMICOBJECT_FACING: u32 = OBJECT_END + 0x8;
pub const DYNAMICOBJECT_PAD: u32 = OBJECT_END + 0x9;
pub const DYNAMICOBJECT_END: u32 = OBJECT_END + 0xA;

// Item Fields
pub const ITEM_FIELD_OWNER: u32 = OBJECT_END + 0x0;
pub const ITEM_FIELD_CONTAINED: u32 = OBJECT_END + 0x2;
pub const ITEM_FIELD_CREATOR: u32 = OBJECT_END + 0x4;
pub const ITEM_FIELD_GIFTCREATOR: u32 = OBJECT_END + 0x6;
pub const ITEM_FIELD_STACK_COUNT: u32 = OBJECT_END + 0x8;
pub const ITEM_FIELD_DURATION: u32 = OBJECT_END + 0x9;
pub const ITEM_FIELD_SPELL_CHARGES: u32 = OBJECT_END + 0xA;
pub const ITEM_FIELD_FLAGS: u32 = OBJECT_END + 0xF;
pub const ITEM_FIELD_ENCHANTMENT: u32 = OBJECT_END + 0x10;
pub const ITEM_FIELD_PROPERTY_SEED: u32 = OBJECT_END + 0x25;
pub const ITEM_FIELD_RANDOM_PROPERTIES_ID: u32 = OBJECT_END + 0x26;
pub const ITEM_FIELD_ITEM_TEXT_ID: u32 = OBJECT_END + 0x27;
pub const ITEM_FIELD_DURABILITY: u32 = OBJECT_END + 0x28;
pub const ITEM_FIELD_MAXDURABILITY: u32 = OBJECT_END + 0x29;
pub const ITEM_END: u32 = OBJECT_END + 0x2A;

// Container Fields
pub const CONTAINER_FIELD_NUM_SLOTS: u32 = ITEM_END + 0x0;
pub const CONTAINER_ALIGN_PAD: u32 = ITEM_END + 0x1;
pub const CONTAINER_FIELD_SLOT_1: u32 = ITEM_END + 0x2;
pub const CONTAINER_END: u32 = ITEM_END + 0x4A;

// Corpse Fields
pub const CORPSE_FIELD_OWNER: u32 = OBJECT_END + 0x0;
pub const CORPSE_FIELD_PARTY: u32 = OBJECT_END + 0x2;
pub const CORPSE_FIELD_DISPLAY_ID: u32 = OBJECT_END + 0x4;
pub const CORPSE_FIELD_ITEM: u32 = OBJECT_END + 0x5;
pub const CORPSE_FIELD_BYTES_1: u32 = OBJECT_END + 0x18;
pub const CORPSE_FIELD_BYTES_2: u32 = OBJECT_END + 0x19;
pub const CORPSE_FIELD_FLAGS: u32 = OBJECT_END + 0x1A;
pub const CORPSE_FIELD_DYNAMIC_FLAGS: u32 = OBJECT_END + 0x1B;
pub const CORPSE_FIELD_DEATH_TIME: u32 = OBJECT_END + 0x1C;
pub const CORPSE_END: u32 = OBJECT_END + 0x1D;

// GameObject Fields
pub const OBJECT_FIELD_CREATED_BY: u32 = OBJECT_END + 0x0;
pub const GAMEOBJECT_DISPLAYID: u32 = OBJECT_END + 0x2;
pub const GAMEOBJECT_FLAGS: u32 = OBJECT_END + 0x3;
pub const GAMEOBJECT_ROTATION: u32 = OBJECT_END + 0x4;
pub const GAMEOBJECT_STATE: u32 = OBJECT_END + 0x8;
pub const GAMEOBJECT_POS_X: u32 = OBJECT_END + 0x9;
pub const GAMEOBJECT_POS_Y: u32 = OBJECT_END + 0xA;
pub const GAMEOBJECT_POS_Z: u32 = OBJECT_END + 0xB;
pub const GAMEOBJECT_FACING: u32 = OBJECT_END + 0xC;
pub const GAMEOBJECT_DYN_FLAGS: u32 = OBJECT_END + 0xD;
pub const GAMEOBJECT_FACTION: u32 = OBJECT_END + 0xE;
pub const GAMEOBJECT_TYPE_ID: u32 = OBJECT_END + 0xF;
pub const GAMEOBJECT_LEVEL: u32 = OBJECT_END + 0x10;
pub const GAMEOBJECT_ARTKIT: u32 = OBJECT_END + 0x11;
pub const GAMEOBJECT_ANIMPROGRESS: u32 = OBJECT_END + 0x12;
pub const GAMEOBJECT_PADDING: u32 = OBJECT_END + 0x13;
pub const GAMEOBJECT_END: u32 = OBJECT_END + 0x14;

// Unit Fields
pub const UNIT_FIELD_CHARM: u32 = OBJECT_END + 0x0;
pub const UNIT_FIELD_SUMMON: u32 = OBJECT_END + 0x2;
pub const UNIT_FIELD_CHARMEDBY: u32 = OBJECT_END + 0x4;
pub const UNIT_FIELD_SUMMONEDBY: u32 = OBJECT_END + 0x6;
pub const UNIT_FIELD_CREATEDBY: u32 = OBJECT_END + 0x8;
pub const UNIT_FIELD_TARGET: u32 = OBJECT_END + 0xA;
pub const UNIT_FIELD_PERSUADED: u32 = OBJECT_END + 0xC;
pub const UNIT_FIELD_CHANNEL_OBJECT: u32 = OBJECT_END + 0xE;
pub const UNIT_FIELD_HEALTH: u32 = OBJECT_END + 0x10;
pub const UNIT_FIELD_POWER1: u32 = OBJECT_END + 0x11;
pub const UNIT_FIELD_POWER2: u32 = OBJECT_END + 0x12;
pub const UNIT_FIELD_POWER3: u32 = OBJECT_END + 0x13;
pub const UNIT_FIELD_POWER4: u32 = OBJECT_END + 0x14;
pub const UNIT_FIELD_POWER5: u32 = OBJECT_END + 0x15;
pub const UNIT_FIELD_MAXHEALTH: u32 = OBJECT_END + 0x16;
pub const UNIT_FIELD_MAXPOWER1: u32 = OBJECT_END + 0x17;
pub const UNIT_FIELD_MAXPOWER2: u32 = OBJECT_END + 0x18;
pub const UNIT_FIELD_MAXPOWER3: u32 = OBJECT_END + 0x19;
pub const UNIT_FIELD_MAXPOWER4: u32 = OBJECT_END + 0x1A;
pub const UNIT_FIELD_MAXPOWER5: u32 = OBJECT_END + 0x1B;
pub const UNIT_FIELD_LEVEL: u32 = OBJECT_END + 0x1C;
pub const UNIT_FIELD_FACTIONTEMPLATE: u32 = OBJECT_END + 0x1D;
pub const UNIT_FIELD_BYTES_0: u32 = OBJECT_END + 0x1E;
pub const UNIT_VIRTUAL_ITEM_SLOT_DISPLAY: u32 = OBJECT_END + 0x1F;
pub const UNIT_VIRTUAL_ITEM_INFO: u32 = OBJECT_END + 0x22;
pub const UNIT_FIELD_FLAGS: u32 = OBJECT_END + 0x28;
pub const UNIT_FIELD_AURA: u32 = OBJECT_END + 0x29;
pub const UNIT_FIELD_AURAFLAGS: u32 = OBJECT_END + 0x59;
pub const UNIT_FIELD_AURALEVELS: u32 = OBJECT_END + 0x5F;
pub const UNIT_FIELD_AURAAPPLICATIONS: u32 = OBJECT_END + 0x6B;
pub const UNIT_FIELD_AURASTATE: u32 = OBJECT_END + 0x77;
pub const UNIT_FIELD_BASEATTACKTIME: u32 = OBJECT_END + 0x78;
pub const UNIT_FIELD_RANGEDATTACKTIME: u32 = OBJECT_END + 0x7A;
pub const UNIT_FIELD_BOUNDINGRADIUS: u32 = OBJECT_END + 0x7B;
pub const UNIT_FIELD_COMBATREACH: u32 = OBJECT_END + 0x7C;
pub const UNIT_FIELD_DISPLAYID: u32 = OBJECT_END + 0x7D;
pub const UNIT_FIELD_NATIVEDISPLAYID: u32 = OBJECT_END + 0x7E;
pub const UNIT_FIELD_MOUNTDISPLAYID: u32 = OBJECT_END + 0x7F;
pub const UNIT_FIELD_MINDAMAGE: u32 = OBJECT_END + 0x80;
pub const UNIT_FIELD_MAXDAMAGE: u32 = OBJECT_END + 0x81;
pub const UNIT_FIELD_MINOFFHANDDAMAGE: u32 = OBJECT_END + 0x82;
pub const UNIT_FIELD_MAXOFFHANDDAMAGE: u32 = OBJECT_END + 0x83;
pub const UNIT_FIELD_BYTES_1: u32 = OBJECT_END + 0x84;
pub const UNIT_FIELD_PETNUMBER: u32 = OBJECT_END + 0x85;
pub const UNIT_FIELD_PET_NAME_TIMESTAMP: u32 = OBJECT_END + 0x86;
pub const UNIT_FIELD_PETEXPERIENCE: u32 = OBJECT_END + 0x87;
pub const UNIT_FIELD_PETNEXTLEVELEXP: u32 = OBJECT_END + 0x88;
pub const UNIT_DYNAMIC_FLAGS: u32 = OBJECT_END + 0x89;
pub const UNIT_CHANNEL_SPELL: u32 = OBJECT_END + 0x8A;
pub const UNIT_MOD_CAST_SPEED: u32 = OBJECT_END + 0x8B;
pub const UNIT_CREATED_BY_SPELL: u32 = OBJECT_END + 0x8C;
pub const UNIT_NPC_FLAGS: u32 = OBJECT_END + 0x8D;
pub const UNIT_NPC_EMOTESTATE: u32 = OBJECT_END + 0x8E;
pub const UNIT_TRAINING_POINTS: u32 = OBJECT_END + 0x8F;
pub const UNIT_FIELD_STAT0: u32 = OBJECT_END + 0x90;
pub const UNIT_FIELD_STAT1: u32 = OBJECT_END + 0x91;
pub const UNIT_FIELD_STAT2: u32 = OBJECT_END + 0x92;
pub const UNIT_FIELD_STAT3: u32 = OBJECT_END + 0x93;
pub const UNIT_FIELD_STAT4: u32 = OBJECT_END + 0x94;
pub const UNIT_FIELD_RESISTANCES: u32 = OBJECT_END + 0x95;
pub const UNIT_FIELD_BASE_MANA: u32 = OBJECT_END + 0x9C;
pub const UNIT_FIELD_BASE_HEALTH: u32 = OBJECT_END + 0x9D;
pub const UNIT_FIELD_BYTES_2: u32 = OBJECT_END + 0x9E;
pub const UNIT_FIELD_ATTACK_POWER: u32 = OBJECT_END + 0x9F;
pub const UNIT_FIELD_ATTACK_POWER_MODS: u32 = OBJECT_END + 0xA0;
pub const UNIT_FIELD_ATTACK_POWER_MULTIPLIER: u32 = OBJECT_END + 0xA1;
pub const UNIT_FIELD_RANGED_ATTACK_POWER: u32 = OBJECT_END + 0xA2;
pub const UNIT_FIELD_RANGED_ATTACK_POWER_MODS: u32 = OBJECT_END + 0xA3;
pub const UNIT_FIELD_RANGED_ATTACK_POWER_MULTIPLIER: u32 = OBJECT_END + 0xA4;
pub const UNIT_FIELD_MINRANGEDDAMAGE: u32 = OBJECT_END + 0xA5;
pub const UNIT_FIELD_MAXRANGEDDAMAGE: u32 = OBJECT_END + 0xA6;
pub const UNIT_FIELD_POWER_COST_MODIFIER: u32 = OBJECT_END + 0xA7;
pub const UNIT_FIELD_POWER_COST_MULTIPLIER: u32 = OBJECT_END + 0xAE;
pub const UNIT_FIELD_PADDING: u32 = OBJECT_END + 0xB5;
pub const UNIT_END: u32 = OBJECT_END + 0xB6;

// Player Fields
pub const PLAYER_DUEL_ARBITER: u32 = UNIT_END + 0x0;
pub const PLAYER_FLAGS: u32 = UNIT_END + 0x2;
pub const PLAYER_GUILDID: u32 = UNIT_END + 0x3;
pub const PLAYER_GUILDRANK: u32 = UNIT_END + 0x4;
pub const PLAYER_BYTES: u32 = UNIT_END + 0x5;
pub const PLAYER_BYTES_2: u32 = UNIT_END + 0x6;
pub const PLAYER_BYTES_3: u32 = UNIT_END + 0x7;
pub const PLAYER_DUEL_TEAM: u32 = UNIT_END + 0x8;
pub const PLAYER_GUILD_TIMESTAMP: u32 = UNIT_END + 0x9;
pub const PLAYER_QUEST_LOG_1_1: u32 = UNIT_END + 0xA;
pub const PLAYER_QUEST_LOG_1_2: u32 = UNIT_END + 0xB;
pub const PLAYER_XP: u32 = UNIT_END + 0x210;
pub const PLAYER_NEXT_LEVEL_XP: u32 = UNIT_END + 0x211;
pub const PLAYER_SKILL_INFO_1_1: u32 = UNIT_END + 0x212;
pub const PLAYER_CHARACTER_POINTS1: u32 = UNIT_END + 0x392;
pub const PLAYER_CHARACTER_POINTS2: u32 = UNIT_END + 0x393;
pub const PLAYER_TRACK_CREATURES: u32 = UNIT_END + 0x394;
pub const PLAYER_TRACK_RESOURCES: u32 = UNIT_END + 0x395;
pub const PLAYER_BLOCK_PERCENTAGE: u32 = UNIT_END + 0x396;
pub const PLAYER_DODGE_PERCENTAGE: u32 = UNIT_END + 0x397;
pub const PLAYER_PARRY_PERCENTAGE: u32 = UNIT_END + 0x398;
pub const PLAYER_CRIT_PERCENTAGE: u32 = UNIT_END + 0x399;
pub const PLAYER_RANGED_CRIT_PERCENTAGE: u32 = UNIT_END + 0x39A;
pub const PLAYER_EXPLORED_ZONES_1: u32 = UNIT_END + 0x39B;
pub const PLAYER_REST_STATE_EXPERIENCE: u32 = UNIT_END + 0x3DB;
pub const PLAYER_FIELD_COINAGE: u32 = UNIT_END + 0x3DC;
pub const PLAYER_FIELD_POSSTAT0: u32 = UNIT_END + 0x3DD;
pub const PLAYER_FIELD_POSSTAT1: u32 = UNIT_END + 0x3DE;
pub const PLAYER_FIELD_POSSTAT2: u32 = UNIT_END + 0x3DF;
pub const PLAYER_FIELD_POSSTAT3: u32 = UNIT_END + 0x3E0;
pub const PLAYER_FIELD_POSSTAT4: u32 = UNIT_END + 0x3E1;
pub const PLAYER_FIELD_NEGSTAT0: u32 = UNIT_END + 0x3E2;
pub const PLAYER_FIELD_NEGSTAT1: u32 = UNIT_END + 0x3E3;
pub const PLAYER_FIELD_NEGSTAT2: u32 = UNIT_END + 0x3E4;
pub const PLAYER_FIELD_NEGSTAT3: u32 = UNIT_END + 0x3E5;
pub const PLAYER_FIELD_NEGSTAT4: u32 = UNIT_END + 0x3E6;
pub const PLAYER_FIELD_RESISTANCEBUFFMODSPOSITIVE: u32 = UNIT_END + 0x3E7;
pub const PLAYER_FIELD_RESISTANCEBUFFMODSNEGATIVE: u32 = UNIT_END + 0x3EE;
pub const PLAYER_FIELD_MOD_DAMAGE_DONE_POS: u32 = UNIT_END + 0x3F5;
pub const PLAYER_FIELD_MOD_DAMAGE_DONE_NEG: u32 = UNIT_END + 0x3FC;
pub const PLAYER_FIELD_MOD_DAMAGE_DONE_PCT: u32 = UNIT_END + 0x403;
pub const PLAYER_FIELD_BYTES: u32 = UNIT_END + 0x40A;
pub const PLAYER_AMMO_ID: u32 = UNIT_END + 0x40B;
pub const PLAYER_SELF_RES_SPELL: u32 = UNIT_END + 0x40C;
pub const PLAYER_FIELD_PVP_MEDALS: u32 = UNIT_END + 0x40D;
pub const PLAYER_FIELD_BUYBACK_PRICE_1: u32 = UNIT_END + 0x40E;
pub const PLAYER_FIELD_BUYBACK_TIMESTAMP_1: u32 = UNIT_END + 0x41A;
pub const PLAYER_FIELD_SESSION_KILLS: u32 = UNIT_END + 0x426;
pub const PLAYER_FIELD_YESTERDAY_KILLS: u32 = UNIT_END + 0x427;
pub const PLAYER_FIELD_LAST_WEEK_KILLS: u32 = UNIT_END + 0x428;
pub const PLAYER_FIELD_THIS_WEEK_KILLS: u32 = UNIT_END + 0x429;
pub const PLAYER_FIELD_THIS_WEEK_CONTRIBUTION: u32 = UNIT_END + 0x42A;
pub const PLAYER_FIELD_LIFETIME_HONORBALE_KILLS: u32 = UNIT_END + 0x42B;
pub const PLAYER_FIELD_LIFETIME_DISHONORBALE_KILLS: u32 = UNIT_END + 0x42C;
pub const PLAYER_FIELD_YESTERDAY_CONTRIBUTION: u32 = UNIT_END + 0x42D;
pub const PLAYER_FIELD_LAST_WEEK_CONTRIBUTION: u32 = UNIT_END + 0x42E;
pub const PLAYER_FIELD_LAST_WEEK_RANK: u32 = UNIT_END + 0x42F;
pub const PLAYER_FIELD_BYTES2: u32 = UNIT_END + 0x430;
pub const PLAYER_FIELD_WATCHED_FACTION_INDEX: u32 = UNIT_END + 0x431;
pub const PLAYER_FIELD_COMBAT_RATING_1: u32 = UNIT_END + 0x432;
pub const PLAYER_END: u32 = UNIT_END + 0x446;

// Visible Equipment (for character model rendering)

/// PLAYER_VISIBLE_ITEM_1_0 = UNIT_END + 0x48 = 0x104 (260)
/// Each visible slot has MAX_VISIBLE_ITEM_OFFSET (12) fields:
/// - Field 0: Item Entry
/// - Field 1-2: Enchantments (PERM_ENCHANTMENT_SLOT, TEMP_ENCHANTMENT_SLOT)
/// - Field 3-6: Property enchantments
/// - Plus creator GUID and properties fields
/// Slots 1-19 cover all equipment slots (head to tabard)
/// Reference: UpdateFields_1_12_1.h line 169, UpdateFields_1_12_1.cpp line 162
pub const PLAYER_VISIBLE_ITEM_1_0: u32 = UNIT_END + 0x48;

/// MAX_VISIBLE_ITEM_OFFSET = 12 (for client builds > 1.5.1)
/// Number of fields per visible item slot
pub const MAX_VISIBLE_ITEM_OFFSET: u32 = 12;

// Equipment and Inventory Slots

/// Equipment slots 0-18: fields 486-523 (each slot is 2 fields: low and high GUID)
pub const PLAYER_FIELD_INV_SLOT_HEAD: u32 = UNIT_END + 0x12A;
/// Inventory slots 23-38: fields 532-563 (each slot is 2 fields: low and high GUID)
pub const PLAYER_FIELD_PACK_SLOT_1: u32 = UNIT_END + 0x158;
pub const PLAYER_FIELD_BANK_SLOT_1: u32 = UNIT_END + 0x178;
pub const PLAYER_FIELD_BANKBAG_SLOT_1: u32 = UNIT_END + 0x1A8;
pub const PLAYER_FIELD_VENDORBUYBACK_SLOT_1: u32 = UNIT_END + 0x1B4;
pub const PLAYER_FIELD_KEYRING_SLOT_1: u32 = UNIT_END + 0x1CC;
pub const PLAYER_FARSIGHT: u32 = UNIT_END + 0x20C;
pub const PLAYER_FIELD_COMBO_TARGET: u32 = UNIT_END + 0x20E;

/// Get the visible item field for an equipment slot (0-18)
/// Returns the field number for the visible item entry (item entry ID, not display_id)
pub fn visible_item_entry_field(slot: u8) -> u32 {
    assert!(slot < 19, "Equipment slot must be 0-18");
    // Each visible slot has MAX_VISIBLE_ITEM_OFFSET (12) fields
    // Field 0 is the item entry, fields 1+ are enchantments
    PLAYER_VISIBLE_ITEM_1_0 + (slot as u32 * MAX_VISIBLE_ITEM_OFFSET)
}

/// Get the field numbers for an equipment slot (0-18)
/// Returns (field_low, field_high) for the slot's GUID
pub fn equipment_slot_fields(slot: u8) -> (u32, u32) {
    assert!(slot < 19, "Equipment slot must be 0-18");
    let slot_offset = slot as u32;
    let field_low = PLAYER_FIELD_INV_SLOT_HEAD + (slot_offset * 2);
    let field_high = field_low + 1;
    (field_low, field_high)
}

/// Get the field numbers for an inventory slot (23-38)
/// Returns (field_low, field_high) for the slot's GUID
pub fn inventory_slot_fields(slot: u8) -> (u32, u32) {
    assert!(slot >= 23 && slot < 39, "Inventory slot must be 23-38");
    let slot_offset = (slot - 23) as u32;
    let field_low = PLAYER_FIELD_PACK_SLOT_1 + (slot_offset * 2);
    let field_high = field_low + 1;
    (field_low, field_high)
}

/// Get the field numbers for any player slot (equipment or inventory)
/// Returns (field_low, field_high) for the slot's GUID, or None if invalid slot
pub fn player_slot_fields(slot: u8) -> Option<(u32, u32)> {
    if slot < 19 {
        Some(equipment_slot_fields(slot))
    } else if slot >= 23 && slot < 39 {
        Some(inventory_slot_fields(slot))
    } else {
        None
    }
}
