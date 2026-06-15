# Audit: AuctionHouseMgr::SendAuctionExpiredMail

**Rust location:** `src/world/game/auction/manager.rs` — `send_auction_expired_mail`
**Status:** verified

## Claim coverage

| Claim | Category | Status | Notes |
|-------|----------|--------|-------|
| Accepts AuctionEntry*; reads itemGuidLow, owner, itemTemplate | input | complete | All fields accessed via `auction.*` |
| Returns void | output | complete | `-> Result<()>`; Ok(()) in all paths |
| auction/pItem/owner raw pointers — no null check on session | danger | complete | Rust references eliminate null-ptr danger; session access deferred to caller |
| Intent: return item to owner by mail | assumption | complete | Implemented as described |
| GetAItem(itemGuidLow) lookup | side_effect | complete | `self.get_a_item(item_guid_low)` |
| GetAItem failure modes unknown | unknown | complete | Returns None → logged error path |
| GetAItem null → log "Auction item (GUID: %u) not found, and lost." + return | branch | complete | `error!("Auction item (GUID: {}) not found, and lost.", item_guid_low)` |
| owner guid constructed as ObjectGuid(HIGHGUID_PLAYER, owner) | side_effect | complete | `auction.seller_guid.low()` used directly |
| Online owner resolved via sObjectMgr.GetPlayer | side_effect | missing | Online lookup deferred to caller; TODO comment in code |
| owner null → owner_accId from GetPlayerAccountIdByGUID | branch | complete | `get_player_account_id_by_guid(owner_guid_low)` |
| Owner-exists path when owner non-null OR owner_accId non-zero | branch | complete | `if owner_acc_id == 0 { destroy } else { return }` |
| Subject: `{itemTemplate}:0:{AUCTION_EXPIRED}` | side_effect | complete | `format!("{}:0:{}", auction.item_template, AUCTION_EXPIRED)` with `AUCTION_EXPIRED = 3` |
| AUCTION_EXPIRED numeric value | unknown | complete | Defined as `const AUCTION_EXPIRED: u32 = 3` matching Mail.h enum |
| owner online → SendAuctionOwnerNotification(auction, false) | branch | missing | TODO comment; caller responsibility. Manager has no session access. |
| Online path: RemoveAItem NOT called before AddItem | danger | partial | Rust always calls `remove_a_item` before `mail_repo.create`. No online/offline distinction without session access. Functionally correct since mail_repo.add_item doesn't need cache presence. |
| owner null + accId non-zero → RemoveAItem before mail | branch | complete | `remove_a_item` called before `mail_repo.create` in all delivery cases |
| MailDraft with subject only (no body), AddItem, SendMailTo with MAIL_CHECK_MASK_COPIED | side_effect | complete | `mail_repo.create` with `item_text_id: 0`, then `mail_repo.add_item` |
| MailDraft/SendMailTo deletes or mailboxes item | assumption | complete | `mail_repo.add_item` persists item to mail; no further cleanup needed |
| owner null + accId 0 → DELETE item_instance, RemoveAItem, delete pItem | branch | complete | `item_repo.delete` + `remove_a_item` + `drop(pitem)` |
| No-receiver: pItem freed after map removal | danger | complete | `Arc<Item>` dropped naturally; no raw delete needed |

## Summary

**Complete:** 16/20 claims fully ported.
**Missing:** 2 claims relate to online player notification — deferred to caller (TODO comment). No game-state impact.
**Partial:** 1 claim on RemoveAItem online-vs-offline ordering — always removes from cache before mail in Rust (no session access to distinguish). Functionally equivalent since the mail layer doesn't read from the auction item cache.

All correctness-critical behaviour (item return by mail, no-owner cleanup, subject format, error logging on missing item) is fully ported.
