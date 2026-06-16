use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct QuestStatusRow {
    pub guid: u32,
    pub quest: u32,
    pub status: u8,
    pub rewarded: bool,
    pub explored: bool,
    pub timer: u32,
    pub mob_count1: u32,
    pub mob_count2: u32,
    pub mob_count3: u32,
    pub mob_count4: u32,
    pub item_count1: u32,
    pub item_count2: u32,
    pub item_count3: u32,
    pub item_count4: u32,
    pub reward_choice: u32,
}

#[derive(FromRow, Debug, Clone)]
pub struct QuestStatusRewardedRow {
    pub guid: u32,
    pub quest: u32,
    pub reward_choice: u32,
}
