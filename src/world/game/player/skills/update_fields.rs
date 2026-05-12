/// Pack skill_id and step into the first update field u32.
///
/// Layout: [skill_id: u16 (high)] [step: u16 (low)]
///
/// The step value represents the training tier for tiered skills:
/// - 0 = not tiered (weapon/defense/armor/language skills)
/// - 1 = Apprentice (professions: max 75)
/// - 2 = Journeyman (professions: max 150)
/// - 3 = Expert (professions: max 225)
/// - 4 = Artisan (professions: max 300)
pub fn make_skill_index(skill_id: u16, step: u16) -> u32 {
    ((skill_id as u32) << 16) | (step as u32)
}

/// Pack current skill value and max value into the second update field u32.
///
/// Layout: [current_value: u16 (high)] [max_value: u16 (low)]
pub fn make_skill_value(current: u16, max: u16) -> u32 {
    ((current as u32) << 16) | (max as u32)
}

/// Pack temporary bonus and permanent bonus into the third update field u32.
///
/// Layout: [temp_bonus: u16 (high)] [perm_bonus: u16 (low)]
///
/// Temporary bonuses come from buffs/auras (e.g. +5 Sword Skill from a buff).
/// Permanent bonuses come from racial passives or enchants.
///
/// The client displays:
/// - Green text: current_value + perm_bonus + temp_bonus
/// - Parenthesized base: current_value
pub fn make_skill_bonus(temp_bonus: u16, perm_bonus: u16) -> u32 {
    ((temp_bonus as u32) << 16) | (perm_bonus as u32)
}

/// Extract skill_id from the packed index field.
pub fn get_skill_id_from_index(index: u32) -> u16 {
    ((index >> 16) & 0xFFFF) as u16
}

/// Extract step from the packed index field.
pub fn get_step_from_index(index: u32) -> u16 {
    (index & 0xFFFF) as u16
}

/// Extract current value from the packed value field.
pub fn get_current_from_value(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

/// Extract max value from the packed value field.
pub fn get_max_from_value(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

/// Write all dirty skills into a player's update fields for UPDATE_OBJECT.
///
/// Only writes skills whose SkillSaveState is New or Changed, avoiding
/// a full re-send on every tick. After writing, marks skills as Unchanged.
///
/// # Arguments
/// * `skills` - The player's skill map
/// * `update_fields` - Mutable reference to the player's update fields array
/// * `base_offset` - The PLAYER_SKILL_INFO_1_1 offset in the update fields
pub fn write_skill_update_fields(
    skills: &mut std::collections::HashMap<u16, super::state::SkillData>,
    update_fields: &mut [u32],
    base_offset: usize,
) {
    use super::state::SkillSaveState;

    for skill_data in skills.values_mut() {
        let pos = skill_data.position;
        if pos >= super::state::PLAYER_MAX_SKILLS {
            continue;
        }

        let field_base = base_offset + pos * 3;

        match skill_data.state {
            SkillSaveState::New | SkillSaveState::Changed => {
                update_fields[field_base] = make_skill_index(skill_data.skill_id, skill_data.step);
                update_fields[field_base + 1] =
                    make_skill_value(skill_data.current_value, skill_data.max_value);
                update_fields[field_base + 2] = make_skill_bonus(0, 0); // TODO: bonus tracking
                skill_data.state = SkillSaveState::Unchanged;
            }
            SkillSaveState::Deleted => {
                update_fields[field_base] = 0;
                update_fields[field_base + 1] = 0;
                update_fields[field_base + 2] = 0;
            }
            SkillSaveState::Unchanged => {
                // No update needed
            }
        }
    }
}
