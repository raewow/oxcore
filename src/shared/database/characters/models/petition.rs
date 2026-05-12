use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct PetitionRow {
    pub owner_guid: u32,
    pub petition_guid: u32,
    pub charter_guid: u32,
    pub name: String,
}

#[derive(FromRow, Debug, Clone)]
pub struct PetitionSignatureRow {
    pub owner_guid: u32,
    pub petition_guid: u32,
    pub player_guid: u32,
    pub player_account: u32,
}
