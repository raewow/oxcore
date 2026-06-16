//! Equipment durability loss on death
//!
//! Durability loss is a gold sink that penalizes death. The amount depends
//! on how the player resurrects.

/// Durability loss percentages (fraction, not percent).
///
/// On death (any cause except BG or player-kill):
///   10% durability loss on ALL equipped items (slots 0-18)
///
/// On spirit healer resurrection (in addition to death loss):
///   Additional 25% durability loss (total 35% for that death)
///
/// Exemptions:
///   - Battleground deaths: NO durability loss at all
///   - Player-killed deaths (PvP): NO durability loss on death
///   - Spell with SPELL_ATTR_EX3_NO_DURABILITY_LOSS: NO durability loss
pub const DEATH_DURABILITY_LOSS: f32 = 0.10;
pub const SPIRIT_HEALER_DURABILITY_LOSS: f32 = 0.25;

/// Equipment slot range: head (0) through tabard (18).
pub const EQUIPMENT_SLOT_START: usize = 0;
pub const EQUIPMENT_SLOT_END: usize = 19;

/// Apply durability loss on death to all equipped items.
///
/// Called once during the JustDied transition. Only affects items in
/// equipment slots (0-18), not bags or bank.
///
/// Returns the number of items that lost durability (for logging).
pub fn apply_death_durability_loss(
    equipment_durability: &mut [(u32, u32)], // (current, max) per slot
) -> u32 {
    let mut items_affected = 0;

    for slot in EQUIPMENT_SLOT_START..EQUIPMENT_SLOT_END {
        if slot >= equipment_durability.len() {
            break;
        }

        let (ref mut current, max) = equipment_durability[slot];
        if *current == 0 || max == 0 {
            continue; // Empty slot or already broken
        }

        let loss = (max as f32 * DEATH_DURABILITY_LOSS).ceil() as u32;
        *current = current.saturating_sub(loss);
        items_affected += 1;
    }

    items_affected
}

/// Apply additional durability loss for spirit healer resurrection.
///
/// Called when the player chooses to resurrect at the spirit healer.
/// This is IN ADDITION to the death durability loss already applied.
///
/// Returns the number of items that lost durability.
pub fn apply_spirit_healer_durability_loss(equipment_durability: &mut [(u32, u32)]) -> u32 {
    let mut items_affected = 0;

    for slot in EQUIPMENT_SLOT_START..EQUIPMENT_SLOT_END {
        if slot >= equipment_durability.len() {
            break;
        }

        let (ref mut current, max) = equipment_durability[slot];
        if *current == 0 || max == 0 {
            continue;
        }

        let loss = (max as f32 * SPIRIT_HEALER_DURABILITY_LOSS).ceil() as u32;
        *current = current.saturating_sub(loss);
        items_affected += 1;
    }

    items_affected
}

/// Check whether durability loss should be applied for this death.
///
/// Returns false (no loss) if:
/// - Player is in a battleground
/// - Death was caused by another player (PvP kill)
/// - Killing spell has SPELL_ATTR_EX3_NO_DURABILITY_LOSS
pub fn should_apply_durability_loss(
    in_battleground: bool,
    killed_by_player: bool,
    spell_prevents_loss: bool,
) -> bool {
    !in_battleground && !killed_by_player && !spell_prevents_loss
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn death_durability_loss_basic() {
        // 3 items: sword (100/100), shield (50/80), empty slot (0/0)
        let mut equipment = vec![(100, 100), (50, 80), (0, 0)];
        let affected = apply_death_durability_loss(&mut equipment);

        assert_eq!(affected, 2);
        assert_eq!(equipment[0].0, 90); // 100 - ceil(100*0.10) = 90
        assert_eq!(equipment[1].0, 42); // 50 - ceil(80*0.10) = 42
        assert_eq!(equipment[2].0, 0); // empty, unchanged
    }

    #[test]
    fn spirit_healer_loss_stacks() {
        let mut equipment = vec![(90, 100)]; // already lost 10% from death
        apply_spirit_healer_durability_loss(&mut equipment);
        assert_eq!(equipment[0].0, 65); // 90 - ceil(100*0.25) = 65
                                        // Total from max: 100 -> 90 -> 65 = 35% total loss
    }

    #[test]
    fn saturating_sub_prevents_underflow() {
        let mut equipment = vec![(2, 100)];
        apply_death_durability_loss(&mut equipment);
        assert_eq!(equipment[0].0, 0); // 2 - 10 = 0 (not underflow)
    }
}
