//! What a player carries about battlegrounds: their queue slots, the match
//! they belong to, and where to send them when they leave.

use oxcore_shared::game::battleground::{BattleGroundTypeId, MAX_QUEUE_SLOTS};
use oxcore_shared::game::chat::Team;
use oxcore_shared::protocol::Position;

/// Where a player entered a battleground from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryPoint {
    pub map_id: u32,
    pub position: Position,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerBattleground {
    /// The battleground each queue slot is for.
    pub queue_slots: [Option<BattleGroundTypeId>; MAX_QUEUE_SLOTS],
    /// The match (copy instance id) each slot is invited to, or 0.
    pub invited_to: [u32; MAX_QUEUE_SLOTS],
    /// Unix time each slot was queued (shown by 1.14 on its ticket).
    pub ticket_times: [i64; MAX_QUEUE_SLOTS],
    /// The match the player belongs to (set before the teleport in), or 0.
    pub instance_id: u32,
    pub bg_type: Option<BattleGroundTypeId>,
    /// The side the player fights for in that match.
    pub team: Option<Team>,
    /// Where leaving sends them.
    pub entry_point: Option<EntryPoint>,
}

impl PlayerBattleground {
    /// The slot holding a battleground's queue.
    pub fn slot_of(&self, bg_type: BattleGroundTypeId) -> Option<usize> {
        self.queue_slots.iter().position(|slot| *slot == Some(bg_type))
    }

    pub fn has_free_slot(&self) -> bool {
        self.queue_slots.iter().any(Option::is_none)
    }

    /// Take a free slot for a queue. Returns it, or the existing slot.
    pub fn add_queue(&mut self, bg_type: BattleGroundTypeId) -> Option<usize> {
        if let Some(slot) = self.slot_of(bg_type) {
            return Some(slot);
        }
        let slot = self.queue_slots.iter().position(Option::is_none)?;
        self.queue_slots[slot] = Some(bg_type);
        self.invited_to[slot] = 0;
        Some(slot)
    }

    pub fn remove_queue(&mut self, bg_type: BattleGroundTypeId) -> Option<usize> {
        let slot = self.slot_of(bg_type)?;
        self.queue_slots[slot] = None;
        self.invited_to[slot] = 0;
        Some(slot)
    }

    pub fn in_any_queue(&self) -> bool {
        self.queue_slots.iter().any(Option::is_some)
    }

    pub fn in_battleground(&self) -> bool {
        self.instance_id != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_queue_slots() {
        let mut bg = PlayerBattleground::default();
        assert_eq!(bg.add_queue(BattleGroundTypeId::WarsongGulch), Some(0));
        assert_eq!(bg.add_queue(BattleGroundTypeId::ArathiBasin), Some(1));
        assert_eq!(bg.add_queue(BattleGroundTypeId::WarsongGulch), Some(0));
        assert_eq!(bg.add_queue(BattleGroundTypeId::AlteracValley), Some(2));
        assert!(!bg.has_free_slot());
        assert_eq!(bg.remove_queue(BattleGroundTypeId::ArathiBasin), Some(1));
        assert_eq!(bg.slot_of(BattleGroundTypeId::ArathiBasin), None);
        assert!(bg.has_free_slot());
    }
}
