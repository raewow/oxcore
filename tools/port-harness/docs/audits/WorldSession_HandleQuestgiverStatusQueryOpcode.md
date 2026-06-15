# Audit: WorldSession::HandleQuestgiverStatusQueryOpcode

**Status:** partial  
**Passed:** false  
**Coverage:** 3/6 claims

## Summary
Rust sends questgiver status for creature and gameobject GUIDs using relation-based computation, and returns without a packet for unresolved GUIDs. It is missing C++ script status override/fallback and hostile creature suppression.

## Rust locations
- Handler: `src/world/handlers/quest_handler.rs:20-44`
- Status send: `src/world/game/npc/quest/system.rs:882-914`
- GUID status resolver: `src/world/game/npc/quest/system.rs:96-111`

## Claims

### Claim 1: Packet GUID read
- **C++ behaviour:** Reads questgiver packed GUID from the packet.
- **Rust:** `packet.read_guid()` reads the target GUID.
- **Status:** covered

### Claim 2: Unknown target
- **C++ behaviour:** If `_player->GetObjectByTypeMask` cannot resolve creature or gameobject, log and return without sending status.
- **Rust:** `get_quest_giver_status_for_guid` returns `None`; `send_quest_giver_status` returns `false` without sending.
- **Status:** covered

### Claim 3: Hostile creature
- **C++ behaviour:** Hostile creature questgivers keep `DIALOG_STATUS_NONE`.
- **Rust:** No hostility check exists in status resolution.
- **Status:** missing

### Claim 4: Creature script status
- **C++ behaviour:** Calls `sScriptMgr.GetDialogStatus`; values `0..=7` are used directly, values above `7` fall back to relation status.
- **Rust:** No dialog-status script hook exists for status query.
- **Status:** missing

### Claim 5: Gameobject script status
- **C++ behaviour:** Calls `sScriptMgr.GetDialogStatus` for gameobjects with the same fallback rule.
- **Rust:** No gameobject dialog-status script hook exists.
- **Status:** missing

### Claim 6: Status packet
- **C++ behaviour:** Sends `PlayerTalkClass->SendQuestGiverStatus` with the computed status.
- **Rust:** `send_quest_giver_status` sends `SmsgQuestgiverStatus` after mapping local status to message status.
- **Status:** covered
