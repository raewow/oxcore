use sqlx::FromRow;

/// Mail table row
///
/// Maps to the `mail` table in the characters database.
/// Contains mail messages with metadata for delivery, expiration, and attachments.
#[derive(FromRow, Debug, Clone)]
pub struct MailRow {
    pub id: u32,
    pub message_type: u8,
    /// TINYINT (signed)
    pub stationery: i8,
    /// MEDIUMINT UNSIGNED
    pub mail_template_id: u32,
    pub sender_guid: u32,
    pub receiver_guid: u32,
    /// LONGTEXT - subject can be null
    pub subject: Option<String>,
    pub item_text_id: u32,
    pub has_items: u8,
    /// BIGINT (signed) - expire time
    pub expire_time: i64,
    /// BIGINT (signed) - deliver time
    pub deliver_time: i64,
    pub money: u32,
    pub cod: u32,
    pub checked: u8,
}

/// Mail items table row
///
/// Maps to the `mail_items` table in the characters database.
/// Contains items attached to mail messages.
#[derive(FromRow, Debug, Clone)]
pub struct MailItemRow {
    pub mail_id: u32,
    pub item_guid: u32,
    pub item_id: u32,
    pub receiver_guid: u32,
}

/// Item text table row
///
/// Maps to the `item_text` table in the characters database.
/// Contains mail body text for longer messages (shared with item text system).
#[derive(FromRow, Debug, Clone)]
pub struct ItemTextRow {
    pub id: u32,
    /// LONGTEXT - text can be null
    pub text: Option<String>,
}
