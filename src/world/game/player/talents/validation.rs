use super::dbc::{TalentInfo, TalentStore};
use super::state::TalentState;

/// Result of attempting to learn a talent rank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalentLearnResult {
    /// Talent was successfully learned (or can be learned).
    Ok,

    /// Player has no free talent points to spend.
    NoFreePoints,

    /// Talent is already at maximum rank.
    AlreadyMaxRank,

    /// Not enough points spent in this talent's tree.
    /// Contains (required, actual) point counts.
    InsufficientPointsInTree { required: u32, actual: u32 },

    /// Prerequisite talent is not fully ranked.
    /// Contains the prerequisite talent_id and its required rank.
    PrerequisiteNotMet {
        prerequisite_id: u32,
        required_rank: u32,
        current_rank: u8,
    },

    /// Talent ID does not exist in the DBC data.
    InvalidTalent,

    /// Talent belongs to a different class.
    WrongClass,
}

/// Validate whether a player can learn the next rank of a talent.
///
/// Checks are performed in priority order:
/// 1. Talent exists in DBC
/// 2. Talent is for the player's class
/// 3. Player has at least 1 free talent point
/// 4. Current rank < max rank (not already maxed)
/// 5. Points spent in tree >= row requirement (row * 5)
/// 6. Prerequisite talent is at required rank (if any)
///
/// This is a pure function with no side effects. It does not modify
/// any state - the caller is responsible for applying the result.
///
/// Ported from MaNGOS Player::LearnTalent() validation logic
/// (Player.cpp:3250-3340).
///
/// # Arguments
/// * `state` - Player's current talent state (read-only)
/// * `talent_id` - The talent the player wants to learn
/// * `class_id` - Player's class (for tab ownership check)
/// * `store` - The global talent DBC store
///
/// # Returns
/// `TalentLearnResult::Ok` if valid, or a specific error variant
pub fn validate_learn_talent(
    state: &TalentState,
    talent_id: u32,
    class_id: u8,
    store: &TalentStore,
) -> TalentLearnResult {
    // 1. Check talent exists
    let talent_info = match store.get_talent(talent_id) {
        Some(info) => info,
        None => return TalentLearnResult::InvalidTalent,
    };

    // 2. Check talent is for this class
    let tab = match store.get_tab(talent_info.tab_id) {
        Some(tab) => tab,
        None => return TalentLearnResult::InvalidTalent,
    };
    if !tab.is_for_class(class_id) {
        return TalentLearnResult::WrongClass;
    }

    // 3. Check free points > 0
    if state.free_talent_points == 0 {
        return TalentLearnResult::NoFreePoints;
    }

    // 4. Check current rank < max rank
    let current_rank = state.talent_rank(talent_id);
    let max_rank = talent_info.max_rank();
    if current_rank >= max_rank {
        return TalentLearnResult::AlreadyMaxRank;
    }

    // 5. Check points spent in tree >= row requirement
    let required_in_tree = talent_info.required_points_in_tab();
    let actual_in_tree = state.points_in_tab(talent_info.tab_id, &store.talent_to_tab);
    if actual_in_tree < required_in_tree {
        return TalentLearnResult::InsufficientPointsInTree {
            required: required_in_tree,
            actual: actual_in_tree,
        };
    }

    // 6. Check prerequisite talent is at required rank
    if talent_info.has_prerequisite() {
        let prereq_current = state.talent_rank(talent_info.prerequisite_talent_id);
        if (prereq_current as u32) < talent_info.prerequisite_rank {
            return TalentLearnResult::PrerequisiteNotMet {
                prerequisite_id: talent_info.prerequisite_talent_id,
                required_rank: talent_info.prerequisite_rank,
                current_rank: prereq_current,
            };
        }
    }

    TalentLearnResult::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_store() -> TalentStore {
        use super::super::dbc::{TalentInfo, TalentTabInfo};

        // Create a Warrior Arms tab (class_mask bit 0 = Warrior)
        let tab = TalentTabInfo {
            id: 161, // Arms tab ID
            name: "Arms".to_string(),
            class_mask: 1, // Warrior = bit 0
            tab_page: 0,
        };

        // Create two talents: one in row 0, one in row 1 with prerequisite
        let deflection = TalentInfo {
            id: 100,
            tab_id: 161,
            row: 0,
            column: 1,
            rank_spell_ids: [16462, 16463, 16464, 16465, 16466],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };

        let tactical_mastery = TalentInfo {
            id: 101,
            tab_id: 161,
            row: 1,
            column: 0,
            rank_spell_ids: [12295, 12676, 12677, 0, 0],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };

        let mortal_strike = TalentInfo {
            id: 102,
            tab_id: 161,
            row: 6,
            column: 1,
            rank_spell_ids: [12294, 21551, 21552, 21553, 21554],
            prerequisite_talent_id: 100, // Requires Deflection maxed
            prerequisite_rank: 5,
        };

        // Filler talents to allow reaching 30 points in tab for row 6 tests
        let filler1 = TalentInfo {
            id: 110,
            tab_id: 161,
            row: 0,
            column: 2,
            rank_spell_ids: [1, 2, 3, 4, 5],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };
        let filler2 = TalentInfo {
            id: 111,
            tab_id: 161,
            row: 0,
            column: 3,
            rank_spell_ids: [6, 7, 8, 9, 10],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };
        let filler3 = TalentInfo {
            id: 112,
            tab_id: 161,
            row: 1,
            column: 1,
            rank_spell_ids: [11, 12, 13, 14, 15],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };
        let filler4 = TalentInfo {
            id: 113,
            tab_id: 161,
            row: 1,
            column: 2,
            rank_spell_ids: [16, 17, 18, 19, 20],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };
        let filler5 = TalentInfo {
            id: 114,
            tab_id: 161,
            row: 2,
            column: 0,
            rank_spell_ids: [21, 22, 23, 24, 25],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };
        let filler6 = TalentInfo {
            id: 115,
            tab_id: 161,
            row: 2,
            column: 1,
            rank_spell_ids: [26, 27, 28, 29, 30],
            prerequisite_talent_id: 0,
            prerequisite_rank: 0,
        };

        TalentStore::load(
            vec![
                deflection,
                tactical_mastery,
                mortal_strike,
                filler1,
                filler2,
                filler3,
                filler4,
                filler5,
                filler6,
            ],
            vec![tab],
        )
    }

    #[test]
    fn test_learn_first_talent() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;

        let result = validate_learn_talent(&state, 100, 1, &store);
        assert_eq!(result, TalentLearnResult::Ok);
    }

    #[test]
    fn test_no_free_points() {
        let store = make_store();
        let state = TalentState::default(); // 0 free points

        let result = validate_learn_talent(&state, 100, 1, &store);
        assert_eq!(result, TalentLearnResult::NoFreePoints);
    }

    #[test]
    fn test_already_max_rank() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;
        state.talents.insert(100, 5); // Already rank 5/5

        let result = validate_learn_talent(&state, 100, 1, &store);
        assert_eq!(result, TalentLearnResult::AlreadyMaxRank);
    }

    #[test]
    fn test_insufficient_points_in_tree() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;
        // Row 1 requires 5 points in tree, but we have 0

        let result = validate_learn_talent(&state, 101, 1, &store);
        assert_eq!(
            result,
            TalentLearnResult::InsufficientPointsInTree {
                required: 5,
                actual: 0,
            }
        );
    }

    #[test]
    fn test_prerequisite_not_met() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;
        // Mortal Strike requires talent 100 (Deflection) at rank 5
        // Row 6 requires 30 points in tree; fill with filler talents + partial deflection
        state.talents.insert(100, 3); // Only rank 3, need 5 (prerequisite fails here)
        state.talents.insert(110, 5); // +5
        state.talents.insert(111, 5); // +5
        state.talents.insert(112, 5); // +5
        state.talents.insert(113, 5); // +5
        state.talents.insert(114, 5); // +5
        state.talents.insert(115, 2); // +2 = total 30 points in tab

        let result = validate_learn_talent(&state, 102, 1, &store);
        assert_eq!(
            result,
            TalentLearnResult::PrerequisiteNotMet {
                prerequisite_id: 100,
                required_rank: 5,
                current_rank: 3,
            }
        );
    }

    #[test]
    fn test_wrong_class() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;
        // Class 2 = Paladin, tab is Warrior-only

        let result = validate_learn_talent(&state, 100, 2, &store);
        assert_eq!(result, TalentLearnResult::WrongClass);
    }

    #[test]
    fn test_invalid_talent() {
        let store = make_store();
        let mut state = TalentState::default();
        state.free_talent_points = 1;

        let result = validate_learn_talent(&state, 99999, 1, &store);
        assert_eq!(result, TalentLearnResult::InvalidTalent);
    }
}
