use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct HonorCPRow {
    pub guid: u32,
    pub victim_type: u8,
    pub victim_id: u32,
    pub cp: f32,
    pub date: u32,
    pub r#type: u8,
}

#[derive(FromRow, Debug, Clone)]
pub struct HonorStoredRow {
    pub guid: u32,
    pub honor_rank_points: f32,
    pub honor_standing: u32,
    pub honor_highest_rank: u8,
    pub honor_last_week_hk: u32,
    pub honor_last_week_cp: f32,
    pub honor_stored_hk: u32,
    pub honor_stored_dk: u32,
}
