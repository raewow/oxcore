# Audit: WorldSession::GetDialogStatus

**Status:** partial  
**Passed:** false  
**Coverage:** 3/6 claims

## Summary
Rust computes dialog status from creature and gameobject start/involved quest relations and has tests for complete, available, unavailable, gameobject, and unknown GUID cases. It lacks the C++ low-level hidden quest `Chat` path and needs closer parity around auto-complete/repeatable reward priority.

## Rust locations
- Entry and relation status: `src/world/game/npc/quest/system.rs:83-194`
- GUID relation lookup: `src/world/game/npc/quest/system.rs:196-224`
- Tests: `src/world/game/npc/quest/system.rs:3010-3127`

## Claims

### Claim 1: Questgiver type relations
- **C++ behaviour:** Uses creature relations for units, gameobject relations for gameobjects, otherwise returns none.
- **Rust:** `quest_giver_relations` handles creature and gameobject GUIDs and returns `None` otherwise.
- **Status:** covered

### Claim 2: Involved complete quest
- **C++ behaviour:** Complete unrewarded involved quests produce reward status, with repeatable autocomplete using `RewardRep`.
- **Rust:** Complete involved quests return `Reward2`; repeatable autocomplete maps to `RewardRep`.
- **Status:** covered

### Claim 3: Involved incomplete quest
- **C++ behaviour:** Incomplete involved quests can produce `Incomplete`.
- **Rust:** Incomplete active involved quests set `DialogStatus::Incomplete`.
- **Status:** covered

### Claim 4: Start quest visibility and level
- **C++ behaviour:** Visible and level-satisfied start quests produce `Available`; low-level hidden quests may produce `Chat`; visible but unavailable quests produce `Unavailable`.
- **Rust:** `can_take_quest` produces `Available`, otherwise `Unavailable`; no explicit `Chat` branch is present.
- **Status:** partial

### Claim 5: Existing active/rewarded start quests
- **C++ behaviour:** Already active or non-repeatable rewarded start quests do not produce a new start status.
- **Rust:** Active and non-repeatable rewarded quests are skipped.
- **Status:** covered

### Claim 6: Priority handling
- **C++ behaviour:** Keeps the highest-priority status, with immediate `Reward2` return for normal completed turn-ins.
- **Rust:** Uses enum ordering and immediate `Reward2` return.
- **Status:** covered, but repeatable/autocomplete edge cases need tests
