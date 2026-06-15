# Audit: WorldSession::HandleQuestgiverRequestRewardOpcode

**Status:** partial  
**Passed:** false  
**Coverage:** 2/5 claims

## Summary
The Rust handler and `QuestSystem::handle_quest_reward_request` cover the final offer-reward packet dispatch, but the auto-complete step (`CanCompleteQuest` → `CompleteQuest`) and the alive/dead visibility gate are completely missing. The target validation is also entry-level rather than instance-level.

## Rust locations
- Packet handler: `src/world/handlers/quest_handler.rs:470-506`
- System method: `src/world/game/npc/quest/system.rs:1073-1103`

## Issues
- [error] Missing auto-complete step. C++ calls `CanCompleteQuest` → `CompleteQuest` before checking status. Rust only checks `active_quest_is_complete`.
- [error] Missing alive/dead check. C++ requires player alive, or if dead, creature must be `IsInvisibleForAlive()`.
- [warning] Missing direct object resolution. C++ uses `GetObjectByTypeMask` and `HasInvolvedQuest` on instance; Rust uses entry-level `quest_giver_can_start_or_finish`.

## Missing behaviours
- No `CanCompleteQuest` → `CompleteQuest` auto-complete before offering reward.
- No alive/dead visibility gate.
- No instance-level target validation.

## Planning notes
- Add `complete_quest` logic to `handle_quest_reward_request` that auto-completes objectives if `CanCompleteQuest` equivalent succeeds.
- Add alive/dead check with `is_invisible_for_alive` support.
- Add `handle_quest_reward_request` integration test: incomplete quest becomes complete via auto-complete, dead player rejection, invalid quest giver.

## Claims

### Claim 1: Object resolution & involved quest
- **Condition:** `GetObjectByTypeMask` returns null or `!HasInvolvedQuest`
- **C++ behaviour:** Return immediately.
- **Rust:** `quest_giver_can_start_or_finish` checks entry-level relations.
- **Status:** partial

### Claim 2: Alive/dead visibility
- **Condition:** `!IsAlive()` and creature `!IsInvisibleForAlive()`
- **C++ behaviour:** Return immediately.
- **Rust:** No alive check.
- **Status:** missing

### Claim 3: Auto-complete quest
- **Condition:** `CanCompleteQuest(packet.quest)` returns true
- **C++ behaviour:** `CompleteQuest(packet.quest)` before checking status.
- **Rust:** `active_quest_is_complete` checks current progress only; does not attempt to auto-complete.
- **Status:** missing

### Claim 4: Status gate
- **Condition:** `GetQuestStatus != QUEST_STATUS_COMPLETE`
- **C++ behaviour:** Return immediately.
- **Rust:** `!is_complete && !is_auto_complete` → return.
- **Status:** covered

### Claim 5: Offer reward
- **Condition:** Quest template exists and status is complete
- **C++ behaviour:** `SendQuestGiverOfferReward`.
- **Rust:** `send_offer_reward`.
- **Status:** covered
