use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct CharacterBattlegroundDataRow {
    pub guid: u32,
    pub instance_id: u32,
    pub team: u8,
    pub join_x: f32,
    pub join_y: f32,
    pub join_z: f32,
    pub join_o: f32,
    pub join_map: u32,
}
