//! DB2 hotfix query handler (modern only)
//!
//! The 1.14 client has no item-query opcode. It asks for item data, and for the dialogue lines that
//! gossip and quest text reference by id, as DB2 rows: one `CMSG_DB_QUERY_BULK` naming a table and a
//! batch of record ids, answered with one `SMSG_DB_REPLY` per id.
//!
//! **Every id gets a reply**, including ones we cannot serve. The frame that triggered the query —
//! a vendor list, a tooltip, a gossip window — waits on the reply, so an unanswered id leaves the
//! client showing "Retrieving item information" indefinitely rather than falling back.

use anyhow::Result;
use tracing::debug;

use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::messages::hotfix::{
    write_broadcast_text_record, write_item_record, write_item_sparse_record, BroadcastTextRecord,
    Db2Hash, HotfixStatus, SmsgDbReply,
};
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::WorldPacket;

/// The client will not ask for more than this in one batch; the count is a 13-bit field, so a
/// corrupt or hostile packet could otherwise claim 8191 records and make us allocate for all of them.
const MAX_QUERIES_PER_BATCH: usize = 8192;

/// Handle CMSG_DB_QUERY_BULK.
///
/// Body: `u32` table hash, then a 13-bit count, then that many `u32` record ids. The count is bit
/// packed but the ids that follow are byte-aligned, because the count's own reader flushes.
pub async fn handle_db_query_bulk(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let mut reader = BitReader::new(packet.contents());
    let Some(raw_hash) = reader.read_u32() else {
        return Ok(());
    };
    let Some(count) = reader.read_bits(13) else {
        return Ok(());
    };

    let count = (count as usize).min(MAX_QUERIES_PER_BATCH);
    let mut record_ids = Vec::with_capacity(count);
    for _ in 0..count {
        match reader.read_u32() {
            Some(id) => record_ids.push(id),
            None => break,
        }
    }

    let table = Db2Hash::from_raw(raw_hash);
    debug!(
        "CMSG_DB_QUERY_BULK: table={:?} (0x{:08X}), {} record(s)",
        table,
        raw_hash,
        record_ids.len()
    );

    // A table we do not serve still needs an answer per id, and the reply echoes the hash back. With
    // no `Db2Hash` to name it there is nothing well-formed to send, so the batch is dropped whole —
    // this is the case worth seeing in a log if a frame ever hangs.
    let Some(table) = table else {
        debug!(
            "Unserved DB2 table 0x{:08X}; {} id(s) unanswered",
            raw_hash,
            record_ids.len()
        );
        return Ok(());
    };

    let timestamp = world.hotfix_timestamp();

    for record_id in record_ids {
        let data = match table {
            Db2Hash::Item | Db2Hash::ItemSparse => world
                .managers
                .item_mgr
                .get_template(record_id)
                .map(|template| {
                    let record = world.managers.hotfix_store.item_record(&template);
                    match table {
                        Db2Hash::Item => write_item_record(&record),
                        _ => write_item_sparse_record(&record),
                    }
                }),
            // `npc_text` already names a `broadcast_text_id` per option and the gossip manager loads
            // the matching rows, so this table needs no separate source.
            Db2Hash::BroadcastText => world
                .systems
                .gossip
                .manager()
                .get_broadcast_text(record_id)
                .map(|text| {
                    write_broadcast_text_record(&BroadcastTextRecord {
                        entry: text.entry,
                        male_text: text.male_text,
                        female_text: text.female_text,
                        language: text.language_id,
                        // The client's columns are 16-bit where ours are 32; vanilla emote ids and
                        // delays both fit, and saturating keeps a bad row from wrapping to a
                        // plausible-looking different emote.
                        emotes: text.emote_ids.map(|id| id.min(u32::from(u16::MAX)) as u16),
                        emote_delays: text
                            .emote_delays
                            .map(|delay| delay.min(u32::from(u16::MAX)) as u16),
                    })
                }),
        };

        let reply = match data {
            Some(data) => SmsgDbReply {
                table_hash: table,
                record_id,
                timestamp,
                status: HotfixStatus::Valid,
                data,
            },
            None => SmsgDbReply::unknown(table, record_id, timestamp),
        };
        session.send_msg(reply)?;
    }

    Ok(())
}
