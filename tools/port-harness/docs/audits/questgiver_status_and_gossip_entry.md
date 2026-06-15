# Audit: questgiver_status_and_gossip_entry

**Status:** partial  
**Passed:** false  
**Coverage:** 8/15 claims

## Summary
The Rust port covers the basic questgiver status packet, relation-based status computation for creatures and gameobjects, and the default creature gossip entry path. It does not yet preserve all C++ behaviour for script dialog-status overrides, hostile creature suppression, exact NPC interaction validation, interaction aura cleanup, spell interruption, out-of-combat movement pause, or gameobject hello targets.

## Rust locations
- Status query handler: `src/world/handlers/quest_handler.rs:20-44`
- Hello handler: `src/world/handlers/quest_handler.rs:111-180`
- Dialog status computation: `src/world/game/npc/quest/system.rs:83-194`
- GUID relation lookup: `src/world/game/npc/quest/system.rs:196-224`
- Status packet send: `src/world/game/npc/quest/system.rs:882-914`
- Existing tests: `src/world/game/npc/quest/system.rs:3010-3127`

## Issues
- [error] Missing script `GetDialogStatus` override/fallback. C++ asks scripts first for creature and gameobject status, using relation status only when the script returns a value greater than `DIALOG_STATUS_REWARD2`.
- [error] Missing hostile creature suppression. C++ forces `DIALOG_STATUS_NONE` for hostile creature questgivers before script/relation status is sent.
- [error] Missing exact `GetNPCIfCanInteractWith` hello validation. Rust only resolves a creature by GUID; it does not enforce the full C++ interaction gate.
- [error] Missing hello cleanup side effects. Rust does not clear feign death, interrupt interacting channels, or remove interacting interrupt auras.
- [warning] Missing movement pause. C++ pauses out-of-combat movement for non-civilian, non-totem creatures before gossip opens.
- [warning] Missing gameobject hello support. Status handles gameobjects, but hello currently rejects non-creature questgiver GUIDs.
- [warning] Dialog status does not model the C++ `Chat` branch for low-level hidden quests. Rust currently maps non-takeable visible quests to `Unavailable`.

## Covered behaviours
- Unknown status-query target returns without sending status.
- Creature and gameobject questgiver relations are resolved by concrete GUID.
- Involved complete quests produce `Reward2`.
- Involved incomplete quests can produce `Incomplete`.
- Available start quests produce `Available`.
- Level-locked visible start quests produce `Unavailable`.
- Default creature hello can run Lua `OnGossipHello` first and return when actions are produced.
- Default creature hello sends a prepared gossip menu with quest entries when script does not handle it.

## Missing behaviours
- Script-provided dialog status and fallback threshold.
- Hostile creature status suppression.
- Full NPC interaction validation for hello.
- Feign death aura removal.
- Interacting channel interrupt and aura removal.
- Out-of-combat movement pause for eligible creatures.
- Gameobject questgiver hello path.
- Low-level quest `Chat` status when configured to hide unavailable low-level quests.
