#[derive(Debug, Clone)]
pub struct ChatLogInsert {
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub sender_guid: u32,
    pub sender_name: String,
    pub sender_account: u32,
    pub target_guid: Option<u32>,
    pub target_name: Option<String>,
    pub message: String,
    pub map: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

#[derive(Debug, Clone)]
pub struct ChatOutboxRow {
    pub id: u64,
    pub sender_account: u32,
    pub sender_guid: u32,
    pub channel_type: String,
    pub target_name: Option<String>,
    pub message: String,
}
