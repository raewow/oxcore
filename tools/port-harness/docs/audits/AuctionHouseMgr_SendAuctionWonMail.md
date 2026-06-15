# Audit: AuctionHouseMgr::SendAuctionWonMail

**Rust location:** `src/world/game/auction/manager.rs` — `send_auction_won_mail`
**Status:** verified

## Claim coverage

| Claim | Category | Status | Notes |
|-------|----------|--------|-------|
| Takes AuctionEntry; reads itemGuidLow, bidder, owner, itemTemplate, bid, buyout | input | complete | All fields accessed via `auction.*` |
| GetAItem miss → return immediately | branch | complete | `let Some(pitem) = self.get_a_item(...) else { return Ok(()) }` |
| Constructs bidder_guid, resolves online Player* | side_effect | complete | Online lookup deferred to caller; account id resolved via `get_player_account_id_by_guid` |
| bidder_accId initialised to 0 | input | complete | Account lookup returns 0 on miss |
| GM_LOG_TRADE path: gather account/security/name | branch | missing | GM trade logging not ported; no config equivalent yet. Low risk — logging only. |
| bidder online during GM log: accId/security/name from session | branch | missing | Same as above |
| bidder offline during GM log: accId via DB, security via sAccountMgr | branch | missing | Same as above |
| bidder_security > SEC_PLAYER → name lookup with LANG_UNKNOWN fallback | branch | missing | Same as above |
| bidder_security > SEC_PLAYER → write GM trade log via sLog.Player | branch | missing | Same as above |
| GM_LOG_TRADE false + bidder null → bidder_accId via GetPlayerAccountIdByGUID | branch | complete | Always uses `get_player_account_id_by_guid` regardless of GM log config |
| Delivery path when bidder non-null OR bidder_accId non-zero | branch | complete | `if bidder_acc_id == 0 { destroy } else { deliver }` |
| Subject: `{itemTemplate}:0:{AUCTION_WON}` | output | complete | `format!("{}:0:{}", auction.item_template, AUCTION_WON)` with `AUCTION_WON = 1` |
| Body: owner hex 16-wide right-aligned + :{bid}:{buyout} | output | complete | `format!("{:>16x}:{}:{}", auction.seller_guid.low(), ...)` |
| Debug log of body string | side_effect | complete | `tracing::debug!("AuctionWon body string : {}", body)` |
| UPDATE item_instance SET owner_guid = bidder + CommitTransaction | side_effect | complete | `item_repo.update_owner(item_guid_low, bidder_guid_low)` |
| bidder online → SendAuctionBidderNotification(auction, true) | branch | missing | TODO comment; caller responsibility. Manager has no session access. |
| bidder offline → RemoveAItem before mail send | branch | partial | `remove_a_item` called after mail create for all paths (no online/offline distinction without session access). Functionally equivalent for the offline case. |
| Mail via MailDraft.AddItem.SendMailTo with MAIL_CHECK_MASK_COPIED | output | complete | `mail_repo.create` + `mail_repo.add_item` with `checked: MailCheckMask::COPIED` |
| No-receiver: DELETE item_instance, RemoveAItem, delete pItem | branch | complete | `item_repo.delete` + `remove_a_item` + `drop(pitem)` |

## Summary

**Complete:** 11/19 claims fully ported.
**Missing:** 5 claims relate to GM trade logging (`CONFIG_BOOL_GM_LOG_TRADE` path) — logging-only behaviour, no game-state impact. Deferred until GM log system is ported.
**Missing:** 1 claim is online bidder notification — deferred to caller (TODO comment in code).
**Partial:** 1 claim on RemoveAItem ordering — functionally correct but doesn't distinguish online vs offline path since manager has no session access.

All correctness-critical behaviour (item delivery, ownership transfer, mail creation, no-receiver cleanup) is fully ported.
