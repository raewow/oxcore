use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct ReputationRow {
    pub guid: u32,
    pub faction: u32,
    pub standing: i32,
    pub flags: i32,
}
