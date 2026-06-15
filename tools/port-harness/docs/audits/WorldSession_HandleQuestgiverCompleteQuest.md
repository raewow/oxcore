# Audit: WorldSession::HandleQuestgiverCompleteQuest

**Status:** partial  
**Passed:** false  
**Coverage:** 1/3 claims

## Summary
The Rust handler and `QuestSystem::handle_quest_complete` send the request-items dialog, but the `completable` flag is derived from `is_complete || is_auto_complete` rather than the C++ branches using `CanCompleteRepeatableQuest` and `CanRewardQuest`. The repeatable vs non-repeatable distinction is lost.

## Rust locations
- Packet handler: `src/world/handlers/quest_handler.rs:269-303`
- System method: `src/world/game/npc/quest/system.rs:989-1026`

## Issues
- [error] Missing repeatable vs non-repeatable branch. C++ uses `CanCompleteRepeatableQuest` for repeatable quests and `CanRewardQuest` for non-repeatable. Rust uses the same `is_complete || is_auto_complete` for both.
- [warning] Missing `CanRewardQuest` and `CanCompleteRepeatableQuest` validation. These are Player methods that check item counts, money, prerequisite status, etc.

## Missing behaviours
- No `CanCompleteRepeatableQuest` equivalent for repeatable quests.
- No `CanRewardQuest` equivalent for non-repeatable quests.
- No distinction between `QUEST_STATUS_COMPLETE` already achieved vs. still incomplete.

## Planning notes
- Add `can_complete_repeatable_quest` method to `QuestSystem` that checks `CanTakeQuest` + required items + `CanRewardQuest`.
- Add `can_reward_quest` method (non-reward-index version) that checks completeness, item counts, money.
- Update `handle_quest_complete` to branch on repeatable vs non-repeatable and use the correct validator.
- Add integration tests: repeatable quest with items (completable=true), repeatable quest without items (completable=false), non-repeatable quest ready for turn-in.

## Claims

### Claim 1: Quest template existence
- **Condition:** `sObjectMgr.GetQuestTemplate` returns null
- **C++ behaviour:** No-op.
- **Rust:** `get_quest_template` returns `None` → early return.
- **Status:** covered

### Claim 2: Repeatable vs non-repeatable branch
- **Condition:** `GetQuestStatus != QUEST_STATUS_COMPLETE` and quest is repeatable
- **C++ behaviour:** `SendQuestGiverRequestItems` with `CanCompleteRepeatableQuest`.
- **Rust:** `send_request_items` with `is_complete || is_auto_complete`; no distinction.
- **Status:** missing

### Claim 3: Non-repeatable / already-complete branch
- **Condition:** `GetQuestStatus != QUEST_STATUS_COMPLETE` (non-repeatable) or already complete
- **C++ behaviour:** `SendQuestGiverRequestItems` with `CanRewardQuest(pQuest, false)`.
- **Rust:** `send_request_items` with `is_complete || is_auto_complete`; no `CanRewardQuest` check.
- **Status:** missing
