use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct InstanceRow {
    pub id: u32,
    pub map: u32,
    pub reset_time: i64,
    pub data: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct CharacterInstanceRow {
    pub guid: u32,
    pub instance: u32,
    pub permanent: u8,
    pub extend: u8,
}

#[derive(FromRow, Debug, Clone)]
pub struct GroupInstanceRow {
    pub leader_guid: u32,
    pub instance: u32,
    pub permanent: u8,
}
