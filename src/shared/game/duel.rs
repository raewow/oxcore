#[derive(Debug, Clone)]
pub struct DuelInfo {
    pub initiator_guid: crate::shared::protocol::ObjectGuid,
    pub opponent_guid: crate::shared::protocol::ObjectGuid,
    pub start_time: u64,
    pub duration: u32,
    pub initiator_health: f32,
    pub initiator_mana: f32,
    pub initiator_rage: f32,
    pub initiator_energy: f32,
    pub opponent_health: f32,
    pub opponent_mana: f32,
    pub opponent_rage: f32,
    pub opponent_energy: f32,
    pub winner: i8, // -1 = pending, 0 = initiator, 1 = opponent, 2 = both
    pub contested: bool,
    pub out_of_bounds: bool,
}

impl DuelInfo {
    pub fn new(
        initiator_guid: crate::shared::protocol::ObjectGuid,
        opponent_guid: crate::shared::protocol::ObjectGuid,
    ) -> Self {
        Self {
            initiator_guid,
            opponent_guid,
            start_time: 0,
            duration: 0,
            initiator_health: 0.0,
            initiator_mana: 0.0,
            initiator_rage: 0.0,
            initiator_energy: 0.0,
            opponent_health: 0.0,
            opponent_mana: 0.0,
            opponent_rage: 0.0,
            opponent_energy: 0.0,
            winner: -1,
            contested: false,
            out_of_bounds: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DuelRequest {
    pub initiator_guid: crate::shared::protocol::ObjectGuid,
    pub target_guid: crate::shared::protocol::ObjectGuid,
    pub request_id: u32,
    pub timeout: u64,
}
