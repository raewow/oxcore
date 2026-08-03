/// Mail table row
///
/// Maps to the `mail` table in the characters database.
/// Contains mail messages with metadata for delivery, expiration, and attachments.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct ItemTextRow {
    pub id: u32,
    /// LONGTEXT - text can be null
    pub text: Option<String>,
}

impl TryFrom<crate::database::characters::PgMailRow> for MailRow {
    type Error = anyhow::Error;

    fn try_from(row: crate::database::characters::PgMailRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.try_into()?,
            message_type: row.message_type.try_into()?,
            stationery: row.stationery.try_into()?,
            mail_template_id: row.mail_template_id.try_into()?,
            sender_guid: row.sender_guid.try_into()?,
            receiver_guid: row.receiver_guid.try_into()?,
            subject: row.subject,
            item_text_id: row.item_text_id.try_into()?,
            has_items: row.has_items.try_into()?,
            expire_time: row.expire_time,
            deliver_time: row.deliver_time,
            money: row.money.try_into()?,
            cod: row.cod.try_into()?,
            checked: row.checked.try_into()?,
        })
    }
}

impl From<&MailRow> for crate::database::characters::PgMailRow {
    fn from(row: &MailRow) -> Self {
        Self {
            id: row.id.into(),
            message_type: row.message_type.into(),
            stationery: row.stationery.into(),
            mail_template_id: row.mail_template_id.into(),
            sender_guid: row.sender_guid.into(),
            receiver_guid: row.receiver_guid.into(),
            subject: row.subject.clone(),
            item_text_id: row.item_text_id.into(),
            has_items: row.has_items.into(),
            expire_time: row.expire_time,
            deliver_time: row.deliver_time,
            money: row.money.into(),
            cod: row.cod.into(),
            checked: row.checked.into(),
        }
    }
}

impl TryFrom<crate::database::characters::PgMailItemRow> for MailItemRow {
    type Error = anyhow::Error;

    fn try_from(row: crate::database::characters::PgMailItemRow) -> Result<Self, Self::Error> {
        Ok(Self {
            mail_id: row.mail_id.try_into()?,
            item_guid: row.item_guid.try_into()?,
            item_id: row.item_id.try_into()?,
            receiver_guid: row.receiver_guid.try_into()?,
        })
    }
}

impl From<&MailItemRow> for crate::database::characters::PgMailItemRow {
    fn from(row: &MailItemRow) -> Self {
        Self {
            mail_id: row.mail_id.into(),
            item_guid: row.item_guid.into(),
            item_id: row.item_id.into(),
            receiver_guid: row.receiver_guid.into(),
        }
    }
}

impl TryFrom<crate::database::characters::PgItemTextRow> for ItemTextRow {
    type Error = anyhow::Error;

    fn try_from(row: crate::database::characters::PgItemTextRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id.try_into()?,
            text: row.text,
        })
    }
}
