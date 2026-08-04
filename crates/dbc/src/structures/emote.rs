use crate::file_loader::DbcRecord;
use crate::store::DbcEntry;
use anyhow::{Context, Result};

/// EmotesText.dbc entry
///
/// Maps the `EmoteID` a client sends in `CMSG_TEXT_EMOTE`/`CTextEmote` (the row index a text
/// command like `/wave` resolves to) to the actual `Emote` enum value that drives the physical
/// animation. The two ids live in unrelated spaces -- same relationship as a spell id to its
/// `SpellXSpellVisualID` -- so the raw client value can never be reused directly as the animation
/// id sent in `SMSG_EMOTE`; it must go through this table first.
///
/// Format: "nxixxxxxxxxxxxxxxxx" (19 fields) -- field 0 is the row id, field 1 is the name string
/// (skipped), field 2 is `textid`/`m_emoteID`, the `Emote` enum value. Fields 3-18 are the
/// per-locale `EmoteText` string refs, irrelevant here.
#[derive(Debug, Clone)]
pub struct EmotesTextEntry {
    pub id: u32,
    /// The `Emote` enum value (animation id) this text emote plays. `0` means no animation
    /// (`EMOTE_ONESHOT_NONE`) -- e.g. purely textual commands.
    pub emote_id: u32,
}

impl DbcEntry for EmotesTextEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read EmotesText ID")?;

        if id == 0 {
            return Ok(None);
        }

        let emote_id = record
            .get_u32(2)
            .context("Failed to read EmotesText textid")?;

        Ok(Some((id, Self { id, emote_id })))
    }
}
