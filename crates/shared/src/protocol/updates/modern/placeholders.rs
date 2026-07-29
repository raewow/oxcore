//! Fields 1.14 requires on a newly created object, which vanilla has no source for.
//!
//! Vanilla objects carry a much smaller field set. Several of the fields 1.14 added are *divisors*
//! or *scales*, so leaving them at their zero default is not a cosmetic gap -- the client renders a
//! unit at zero scale and divides by a zero haste, and does not survive it. That is why HermesProxy
//! has `ObjectUpdate.InitializePlaceholders`
//! (`World/Server/Packets/UpdatePackets.cs:96`), which this transcribes.
//!
//! These apply to **create** blocks only. A values update carries just what changed, and the client
//! already holds these from the create.

use super::field_map::*;
use super::fields::{ModernFieldsArray, ModernObjectType};
use crate::protocol::guid::ObjectGuid;

/// Highest level a 1.12 character can reach, for `ACTIVE_PLAYER_FIELD_MAX_LEVEL`.
const MAX_LEVEL: u32 = 60;

/// `UNIT_FIELD_FLAGS_2` default. Opaque, but HermesProxy sends it on every unit.
const UNIT_FLAGS_2_DEFAULT: u32 = 2048;

/// Honor needed for the next level. 1.14 divides by this, so zero is fatal.
const HONOR_NEXT_LEVEL: u32 = 5500;

/// "No PvP tier", which the client reads as a sentinel rather than a count.
const NO_PVP_TIER: u32 = u32::MAX;

/// Apply every placeholder the object's type calls for.
pub fn apply(
    fields: &mut ModernFieldsArray,
    object_type: ModernObjectType,
    guid: ObjectGuid,
    realm_id: u16,
) {
    if object_type.is_unit() {
        apply_unit(fields);
    }
    if matches!(
        object_type,
        ModernObjectType::Player | ModernObjectType::ActivePlayer
    ) {
        apply_player(fields, guid, realm_id);
    }
    if object_type == ModernObjectType::ActivePlayer {
        apply_active_player(fields);
    }
}

/// Applies to every unit, creatures included -- a creature with zero scale is just as broken as a
/// player with one.
fn apply_unit(fields: &mut ModernFieldsArray) {
    // Power regen multipliers. HermesProxy fills six of the seven slots; the seventh has no power
    // type behind it in 1.12.
    for slot in 0..6 {
        fields.set_modern_f32(MODERN_UNIT_FIELD_MOD_POWER_REGEN + slot, 1.0);
    }

    fields.set_modern(MODERN_UNIT_FIELD_FLAGS_2, UNIT_FLAGS_2_DEFAULT);

    // Scales. Zero here renders the unit invisible at best.
    fields.set_modern_f32(MODERN_UNIT_FIELD_DISPLAY_SCALE, 1.0);
    fields.set_modern_f32(MODERN_UNIT_FIELD_NATIVE_X_DISPLAY_SCALE, 1.0);

    // Haste multipliers. The client divides by these.
    fields.set_modern_f32(MODERN_UNIT_MOD_CAST_HASTE, 1.0);
    fields.set_modern_f32(MODERN_UNIT_FIELD_MOD_HASTE, 1.0);
    fields.set_modern_f32(MODERN_UNIT_FIELD_MOD_RANGED_HASTE, 1.0);
    fields.set_modern_f32(MODERN_UNIT_FIELD_MOD_HASTE_REGEN, 1.0);
    fields.set_modern_f32(MODERN_UNIT_FIELD_MOD_TIME_RATE, 1.0);

    fields.set_modern_f32(MODERN_UNIT_FIELD_HOVERHEIGHT, 1.0);
    fields.set_modern(MODERN_UNIT_FIELD_SCALE_DURATION, 100);
    // -1 means "no look-at controller"; 0 would name controller zero.
    fields.set_modern(MODERN_UNIT_FIELD_LOOK_AT_CONTROLLER_ID, (-1i32) as u32);
}

fn apply_player(fields: &mut ModernFieldsArray, guid: ObjectGuid, realm_id: u16) {
    // 1.14 expects every player to belong to a WoW account. Vanilla has no such GUID, so this
    // derives a stable one from the character, as HermesProxy does for other players.
    let account = ObjectGuid::from_raw(u64::from(guid.counter()));
    let (high, low) = account.to_guid128(realm_id);
    fields.set_modern(MODERN_PLAYER_WOW_ACCOUNT, low as u32);
    fields.set_modern(MODERN_PLAYER_WOW_ACCOUNT + 1, (low >> 32) as u32);
    fields.set_modern(MODERN_PLAYER_WOW_ACCOUNT + 2, high as u32);
    fields.set_modern(MODERN_PLAYER_WOW_ACCOUNT + 3, (high >> 32) as u32);

    fields.set_modern(
        MODERN_PLAYER_FIELD_VIRTUAL_PLAYER_REALM,
        virtual_realm_address(realm_id),
    );
    fields.set_modern(MODERN_PLAYER_FIELD_HONOR_LEVEL, 1);
    // Only index 3 of the six, matching the reference.
    fields.set_modern_f32(MODERN_PLAYER_FIELD_AVG_ITEM_LEVEL + 3, 1.0);
}

fn apply_active_player(fields: &mut ModernFieldsArray) {
    // RestInfo is {Threshold, StateID} pairs; the first needs a non-zero threshold.
    fields.set_modern(MODERN_ACTIVE_PLAYER_FIELD_REST_INFO, 1);
    fields.set_modern(MODERN_ACTIVE_PLAYER_FIELD_REST_INFO + 1, 0);

    for slot in 0..7 {
        fields.set_modern_f32(MODERN_ACTIVE_PLAYER_FIELD_MOD_DAMAGE_DONE_PCT + slot, 1.0);
    }
    fields.set_modern_f32(MODERN_ACTIVE_PLAYER_FIELD_MOD_HEALING_PCT, 1.0);
    fields.set_modern_f32(MODERN_ACTIVE_PLAYER_FIELD_MOD_HEALING_DONE_PCT, 1.0);
    fields.set_modern_f32(
        MODERN_ACTIVE_PLAYER_FIELD_MOD_PERIODIC_HEALING_DONE_PERCENT,
        1.0,
    );

    for slot in 0..3 {
        fields.set_modern_f32(
            MODERN_ACTIVE_PLAYER_FIELD_WEAPON_DMG_MULTIPLIERS + slot,
            1.0,
        );
        fields.set_modern_f32(
            MODERN_ACTIVE_PLAYER_FIELD_WEAPON_ATK_SPEED_MULTIPLIERS + slot,
            1.0,
        );
    }

    fields.set_modern_f32(MODERN_ACTIVE_PLAYER_FIELD_MOD_SPELL_POWER_PCT, 1.0);
    fields.set_modern(MODERN_ACTIVE_PLAYER_FIELD_MAX_LEVEL, MAX_LEVEL);
    fields.set_modern_f32(MODERN_ACTIVE_PLAYER_FIELD_MOD_PET_HASTE, 1.0);
    fields.set_modern(
        MODERN_ACTIVE_PLAYER_FIELD_HONOR_NEXT_LEVEL,
        HONOR_NEXT_LEVEL,
    );
    fields.set_modern(
        MODERN_ACTIVE_PLAYER_FIELD_PVP_TIER_MAX_FROM_WINS,
        NO_PVP_TIER,
    );
    fields.set_modern(
        MODERN_ACTIVE_PLAYER_FIELD_PVP_LAST_WEEKS_TIER_MAX_FROM_WINS,
        NO_PVP_TIER,
    );
}

/// `region << 24 | site << 16 | realm id`, matching what the bnet realm list advertises.
fn virtual_realm_address(realm_id: u16) -> u32 {
    0x0101_0000 | u32::from(realm_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_at(fields: &ModernFieldsArray, slot: u16) -> Option<u32> {
        fields.modern_value(slot)
    }

    /// The scales and haste divisors must be 1.0, not 0.0. The client divides by the hastes and
    /// multiplies by the scales, so a zero is a divide-by-zero or an invisible unit -- this is the
    /// difference between a rendered world and a crash on entering it.
    #[test]
    fn units_get_non_zero_scales_and_haste_divisors() {
        let mut fields = ModernFieldsArray::new(ModernObjectType::Unit, 1);
        apply(
            &mut fields,
            ModernObjectType::Unit,
            ObjectGuid::from_raw(1),
            1,
        );

        for (slot, name) in [
            (MODERN_UNIT_FIELD_DISPLAY_SCALE, "DisplayScale"),
            (
                MODERN_UNIT_FIELD_NATIVE_X_DISPLAY_SCALE,
                "NativeXDisplayScale",
            ),
            (MODERN_UNIT_MOD_CAST_HASTE, "ModCastHaste"),
            (MODERN_UNIT_FIELD_MOD_HASTE, "ModHaste"),
            (MODERN_UNIT_FIELD_MOD_RANGED_HASTE, "ModRangedHaste"),
            (MODERN_UNIT_FIELD_MOD_HASTE_REGEN, "ModHasteRegen"),
            (MODERN_UNIT_FIELD_MOD_TIME_RATE, "ModTimeRate"),
        ] {
            let bits = value_at(&fields, slot).unwrap_or_else(|| panic!("{name} was not set"));
            assert_eq!(f32::from_bits(bits), 1.0, "{name} must be 1.0, never 0.0");
        }

        assert_eq!(
            value_at(&fields, MODERN_UNIT_FIELD_LOOK_AT_CONTROLLER_ID),
            Some((-1i32) as u32),
            "0 would name controller zero rather than 'none'"
        );
    }

    /// Creatures need them too — a creature at zero scale is as broken as a player at zero scale.
    #[test]
    fn placeholders_apply_to_creatures_not_just_players() {
        let mut fields = ModernFieldsArray::new(ModernObjectType::Unit, 1);
        apply(
            &mut fields,
            ModernObjectType::Unit,
            ObjectGuid::from_raw(1),
            1,
        );
        assert!(value_at(&fields, MODERN_UNIT_FIELD_DISPLAY_SCALE).is_some());
    }

    /// Self-only multipliers exist only under ActivePlayer, so a plain Player must not claim them.
    #[test]
    fn active_player_extras_are_scoped_to_active_player() {
        let mut player = ModernFieldsArray::new(ModernObjectType::Player, 1);
        apply(
            &mut player,
            ModernObjectType::Player,
            ObjectGuid::new_player(4),
            1,
        );
        assert_eq!(
            value_at(&player, MODERN_ACTIVE_PLAYER_FIELD_MOD_HEALING_PCT),
            None,
            "a plain Player has no ActivePlayer fields"
        );

        let mut active = ModernFieldsArray::new(ModernObjectType::ActivePlayer, 1);
        apply(
            &mut active,
            ModernObjectType::ActivePlayer,
            ObjectGuid::new_player(4),
            1,
        );
        let bits = value_at(&active, MODERN_ACTIVE_PLAYER_FIELD_MOD_HEALING_PCT)
            .expect("ActivePlayer gets the healing multiplier");
        assert_eq!(f32::from_bits(bits), 1.0);
    }

    /// A values update must not carry them: the client already holds them from the create, and
    /// resending would bloat every health tick.
    #[test]
    fn values_updates_carry_no_placeholders() {
        use super::super::block::{ModernUpdateBlock, ModernUpdateType};

        let block = ModernUpdateBlock::new(
            ModernUpdateType::Values,
            ObjectGuid::new_player(4),
            ModernObjectType::ActivePlayer,
            1,
        );
        assert_eq!(
            value_at(&block.fields, MODERN_UNIT_FIELD_DISPLAY_SCALE),
            None
        );
    }
}
