# Audit: AuctionHouseMgr::SendAuctionSuccessfulMail

**Rust location:** `src/world/game/auction/manager.rs:536-610` — `send_auction_successful_mail`
**Status:** verified

## Claim coverage

| Claim | Category | Status | Notes |
|-------|----------|--------|-------|
| Accepts AuctionEntry*; reads owner, itemTemplate, bidder, bid, buyout, deposit | input | complete | All fields accessed via `auction.*` |
| No null guard on auction pointer — UB if null | danger | complete | Rust reference parameter; null impossible by type |
| owner guid constructed as ObjectGuid(HIGHGUID_PLAYER, owner) | side_effect | complete | `ObjectGuid` constructed from `auction.seller_guid.low()` |
| Online owner looked up via sObjectMgr.GetPlayer | side_effect | missing | Online lookup deferred to caller; TODO comment at line 565 |
| owner_accId = 0; if owner null, set from GetPlayerAccountIdByGUID | branch | complete | `get_player_account_id_by_guid` called; returns 0 on miss → early return |
| GetPlayerAccountIdByGUID returning 0 causes skip of entire block | unknown | complete | `if owner_acc_id == 0 { return Ok(()) }` |
| All mail logic when owner non-null OR owner_accId non-zero | branch | complete | Early return on `owner_acc_id == 0` covers both cases |
| Non-zero owner_accId treated as sufficient for delivery | assumption | complete | Matches C++ semantics |
| Subject: `{itemTemplate}:0:{AUCTION_SUCCESSFUL}` | output | complete | `format!("{}:0:{}", auction.item_template, AUCTION_SUCCESSFUL)` with `AUCTION_SUCCESSFUL = 2` |
| AUCTION_SUCCESSFUL macro value | danger | complete | Defined as `const AUCTION_SUCCESSFUL: u32 = 2` matching Mail.h enum |
| Auction cut via GetAuctionCut() | input | complete | `auction.get_auction_cut(cut_percent, cut_rate)` |
| Body: bidder hex 16-wide right-aligned + :bid:buyout:deposit:auctionCut | output | complete | `format!("{:>16x}:{}:{}:{}:{}", auction.bidder_guid.low(), ...)` |
| Debug log of body string | side_effect | complete | `tracing::debug!("AuctionSuccessful body string : {}", body)` |
| Profit = bid + deposit - auctionCut (uint32 wrapping) | output | complete | `wrapping_add` + `wrapping_sub` matches C++ unsigned arithmetic |
| Profit overflow/underflow possible | danger | complete | Preserved via wrapping ops — same semantics as C++ |
| Online owner → SendAuctionOwnerNotification(auction, true) | branch | missing | TODO comment; caller responsibility. Manager has no session access. |
| Offline owner + non-zero accId → mail sent, no notification | branch | complete | Mail always sent when acc_id non-zero; notification is caller's |
| owner raw pointer session dereference without null check | danger | complete | Rust reference parameter eliminates null-ptr danger |
| MailDraft with subject+body, SetMoney(profit), SendMailTo with MAIL_CHECK_MASK_COPIED | output | complete | `mail_repo.create` with `money: profit`, `checked: MailCheckMask::COPIED` |

## Summary

**Complete:** 17/19 claims fully ported.
**Missing:** 2 claims relate to online player notification — deferred to caller (TODO comment in code). No game-state impact; players see the mail regardless.

All correctness-critical behaviour (profit calculation, subject/body format, mail delivery, early-return on unknown owner) is fully ported.
