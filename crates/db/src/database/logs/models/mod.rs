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

#[derive(Debug, Clone)]
pub struct ChatOutboxInsert {
    pub sender_account: u32,
    pub sender_guid: u32,
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub target_name: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ChatLogRow {
    pub id: u64,
    pub time: i64,
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub sender_guid: Option<u32>,
    pub sender_name: Option<String>,
    pub sender_account: Option<u32>,
    pub target_guid: Option<u32>,
    pub target_name: Option<String>,
    pub message: String,
    pub map: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ChatChannelSummaryRow {
    pub channel_type: String,
    pub channel_name: Option<String>,
    pub message_count: i64,
    pub participants: i64,
    pub last_message_at: i64,
}

#[derive(Debug, Clone)]
pub struct ChatParticipantRow {
    pub guid: Option<u32>,
    pub name: String,
    pub account: Option<u32>,
    pub message_count: i64,
    pub last_seen: i64,
}
