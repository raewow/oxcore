# Plan: WorldSession::GetDialogStatus

## Required changes
1. Add config-aware low-level hidden quest behaviour so Rust can return `DialogStatus::Chat` when C++ would not show unavailable low-level quests.
2. Add focused tests for repeatable autocomplete involved quests returning `RewardRep`.
3. Add tests for active and rewarded start quests being skipped.
4. Keep the current GUID-based creature/gameobject relation lookup.

## Tests
- Low-level hidden quest returns `Chat` when the relevant config hides unavailable quests.
- Low-level visible quest returns `Unavailable` when hiding is disabled.
- Repeatable autocomplete involved quest returns `RewardRep`.
- Normal complete involved quest still returns `Reward2` with highest priority.
- Active start quest and non-repeatable rewarded start quest do not produce available status.
