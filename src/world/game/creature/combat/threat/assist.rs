//! Assist Threat Helper
//!
/// Healing and buffing generate "assist threat" - threat added to the creature targeting the assisted player.
/// Rules:
/// 1. Healing/buffing generates threat for the healer
/// 2. Threat is added to all creatures attacking the healed target
/// 3. Threat is ZERO if the creature is under hard CC (fear, blind, stun, confuse)
/// 4. Totems don't generate or receive assist threat
use crate::shared::protocol::ObjectGuid;
use crate::world::game::creature::Creature;

/// Hard CC unit flags that prevent assist threat
pub const UNIT_FLAG_CONFUSED: u32 = 0x00000400;
pub const UNIT_FLAG_FLEEING: u32 = 0x00000800;
pub const UNIT_FLAG_STUNNED: u32 = 0x00040000;

/// Hard CC mask
pub const HARD_CC_FLAGS: u32 = UNIT_FLAG_CONFUSED | UNIT_FLAG_FLEEING | UNIT_FLAG_STUNNED;

/// Assist threat helper
pub struct AssistThreatHelper;

impl AssistThreatHelper {
    /// Check if creature is under hard CC that prevents assist threat
    pub fn is_under_hard_cc(creature: &Creature) -> bool {
        (creature.unit_flags & HARD_CC_FLAGS) != 0
    }

    /// Check if creature can receive assist threat
    ///
    /// Returns false if:
    /// - Creature is under hard CC
    /// - Creature is a totem
    pub fn can_receive_assist_threat(creature: &Creature) -> bool {
        // Check hard CC
        if Self::is_under_hard_cc(creature) {
            return false;
        }

        // TODO: Check if creature is a totem
        // This would require checking summon_type which isn't in Creature yet
        // For now, we assume all creatures can receive assist threat unless CCed

        true
    }

    /// Calculate assist threat amount
    ///
    /// Healing threat is 50% of healing done
    /// Buff threat depends on the spell
    pub fn calc_assist_threat(amount: u32, is_healing: bool) -> f32 {
        if is_healing {
            amount as f32 * 0.5
        } else {
            // Buff threat is typically lower
            amount as f32 * 0.25
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::protocol::{ObjectGuid, Position};

    fn create_test_creature() -> Creature {
        use crate::world::game::creature::{manager::CreatureTemplate, Creature};

        let template = CreatureTemplate {
            entry: 1,
            name: "Test".to_string(),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 1,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        };

        Creature::new(
            ObjectGuid::new_creature(1, 1),
            1,
            1,
            Position::new(0.0, 0.0, 0.0, 0.0),
            0,
            1,
            &template,
            1,
            None,
        )
    }

    #[test]
    fn test_no_hard_cc() {
        let creature = create_test_creature();
        assert!(!AssistThreatHelper::is_under_hard_cc(&creature));
        assert!(AssistThreatHelper::can_receive_assist_threat(&creature));
    }

    #[test]
    fn test_stunned_prevents_assist_threat() {
        let mut creature = create_test_creature();
        creature.unit_flags |= UNIT_FLAG_STUNNED;
        assert!(AssistThreatHelper::is_under_hard_cc(&creature));
        assert!(!AssistThreatHelper::can_receive_assist_threat(&creature));
    }

    #[test]
    fn test_fleeing_prevents_assist_threat() {
        let mut creature = create_test_creature();
        creature.unit_flags |= UNIT_FLAG_FLEEING;
        assert!(AssistThreatHelper::is_under_hard_cc(&creature));
        assert!(!AssistThreatHelper::can_receive_assist_threat(&creature));
    }

    #[test]
    fn test_confused_prevents_assist_threat() {
        let mut creature = create_test_creature();
        creature.unit_flags |= UNIT_FLAG_CONFUSED;
        assert!(AssistThreatHelper::is_under_hard_cc(&creature));
        assert!(!AssistThreatHelper::can_receive_assist_threat(&creature));
    }

    #[test]
    fn test_healing_threat_calculation() {
        assert_eq!(AssistThreatHelper::calc_assist_threat(100, true), 50.0);
        assert_eq!(AssistThreatHelper::calc_assist_threat(200, true), 100.0);
    }

    #[test]
    fn test_buff_threat_calculation() {
        assert_eq!(AssistThreatHelper::calc_assist_threat(100, false), 25.0);
        assert_eq!(AssistThreatHelper::calc_assist_threat(200, false), 50.0);
    }
}
