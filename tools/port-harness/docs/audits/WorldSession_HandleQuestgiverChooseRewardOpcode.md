# Audit: WorldSession::HandleQuestgiverChooseRewardOpcode

**Status:** partial  
**Passed:** false  
**Coverage:** 5/7 claims

## Summary
The Rust handler and `QuestSystem::handle_quest_reward` cover most of the reward-granting logic, but two critical C++ validation gates are missing: alive/dead visibility checks, and direct object-level involved-quest validation. The packet-hacking log level is also downgraded from `error` to `warn`.

## Rust locations
- Packet handler: `src/world/handlers/quest_handler.rs:508-551`
- System method: `src/world/game/npc/quest/system.rs:1170-1565`

## Issues
- [error] Missing alive/dead check. C++ requires player alive, or if dead, the creature must be `IsVisibleForDead()` (e.g. quest 3912). Rust `handle_quest_reward` skips this entirely.
- [error] Missing direct object target resolution. C++ uses `GetObjectByTypeMask` and `HasInvolvedQuest` on the object instance. Rust checks `quest_giver_can_start_or_finish` which only checks creature/gameobject entry-level quest relations.
- [warning] Packet-hacking log for invalid reward choice is `warn!` in Rust; C++ uses `LOG_LVL_ERROR`.
- [warning] No separate `CanRewardQuest` pre-flight gate. Rust validates inline (complete, inventory, money) rather than delegating to a single `can_reward_quest` method.

## Missing behaviours
- No alive/dead gate before proceeding to reward quest.
- No instance-level `HasInvolvedQuest` validation (entry-level relations are checked instead).
- No `CanRewardQuest` with `reward` index validation as a single pre-flight check.

## Planning notes
- Add `is_alive` check to `handle_quest_reward`; if dead, check creature `is_visible_for_dead` (requires adding that flag to creature data).
- Add `can_reward_quest` method that consolidates completeness, inventory, money, and reward-choice validation.
- Add `handle_quest_reward` integration tests: invalid reward choice, dead player without visible-for-dead creature, invalid quest giver, successful reward with follow-up quest.

## Claims

### Claim 1: Packet-hacking gate
- **Condition:** `reward_choice >= QUEST_REWARD_CHOICES_COUNT`
- **C++ behaviour:** `sLog.Out(LOG_BASIC, LOG_LVL_ERROR, ...)` then return.
- **Rust:** `can_store_reward_items` returns `None` for invalid choice; logs `warn!`. Should be `error!`.
- **Status:** partial

### Claim 2: Object target resolution
- **Condition:** `GetObjectByTypeMask(guid, TYPEMASK_CREATURE_OR_GAMEOBJECT)` returns null
- **C++ behaviour:** Return immediately.
- **Rust:** `handle_quest_reward` does not resolve the object; it delegates to `quest_giver_can_start_or_finish` which checks relations by entry.
- **Status:** missing

### Claim 3: Involved quest check
- **Condition:** `!HasInvolvedQuest(packet.quest)`
- **C++ behaviour:** Return immediately.
- **Rust:** `quest_giver_can_start_or_finish` checks entry-level start/finish relations; not identical to instance-level `HasInvolvedQuest`.
- **Status:** partial

### Claim 4: Alive/dead visibility
- **Condition:** `!IsAlive()` and creature `!IsVisibleForDead()`
- **C++ behaviour:** Return immediately.
- **Rust:** No alive check.
- **Status:** missing

### Claim 5: Quest template existence
- **Condition:** `sObjectMgr.GetQuestTemplate` returns null
- **C++ behaviour:** No-op (if block skipped).
- **Rust:** `get_quest_template` returns `None` → early return.
- **Status:** covered

### Claim 6: Reward path
- **Condition:** `CanRewardQuest(pQuest, reward, true)` returns true
- **C++ behaviour:** `RewardQuest` + send next-quest details if follow-up exists.
- **Rust:** `handle_quest_reward` grants XP, money, items, rep, removes items, marks quest rewarded, persists to DB, sends completion packets, and sends follow-up quest details.
- **Status:** covered

### Claim 7: Re-offer reward
- **Condition:** `CanRewardQuest` returns false
- **C++ behaviour:** `SendQuestGiverOfferReward`.
- **Rust:** `handle_quest_reward` returns early with `Ok(())` without re-sending offer reward.
- **Status:** missing
