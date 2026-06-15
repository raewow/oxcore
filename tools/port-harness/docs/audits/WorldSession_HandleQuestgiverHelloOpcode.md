# Audit: WorldSession::HandleQuestgiverHelloOpcode

**Status:** partial  
**Passed:** false  
**Coverage:** 2/7 claims

## Summary
Rust resolves creature questgiver hello, runs Lua `OnGossipHello`, and sends default gossip with quest entries when the script does not handle it. It does not yet implement the full C++ interaction validation, cleanup side effects, movement pause, or gameobject hello handling.

## Rust locations
- Handler: `src/world/handlers/quest_handler.rs:111-180`
- Quest menu preparation: `src/world/game/npc/quest/system.rs:916-925`
- Gossip send: `src/world/game/npc/gossip/system.rs`

## Claims

### Claim 1: Interaction validation
- **C++ behaviour:** Uses `GetNPCIfCanInteractWith` with `UNIT_NPC_FLAG_NONE`; invalid targets log and return.
- **Rust:** Only checks `creature_mgr.get_creature(guid)`.
- **Status:** partial

### Claim 2: Feign death cleanup
- **C++ behaviour:** Removes feign death aura when the player has stunned unit state.
- **Rust:** No feign death/unit-state cleanup.
- **Status:** missing

### Claim 3: Movement pause
- **C++ behaviour:** Pauses out-of-combat movement for non-civilian, non-totem creatures.
- **Rust:** No movement pause.
- **Status:** missing

### Claim 4: Interacting spell interruption
- **C++ behaviour:** Interrupts interacting channels and removes auras with interacting interrupt flags.
- **Rust:** No spell/channel/aura cleanup.
- **Status:** missing

### Claim 5: Script gossip hello
- **C++ behaviour:** If `sScriptMgr.OnGossipHello` handles the target, return without default gossip.
- **Rust:** Lua `OnGossipHello` actions execute first and return when actions are produced.
- **Status:** covered

### Claim 6: Default gossip
- **C++ behaviour:** Prepares gossip menu with the creature default menu id and sends prepared gossip.
- **Rust:** Sends gossip through `GossipSystem` with prepared quest entries; default menu id parity is not confirmed.
- **Status:** partial

### Claim 7: Gameobject targets
- **C++ behaviour:** This handler is NPC-specific through `GetNPCIfCanInteractWith`; gameobject questgiver interaction is expected through gameobject gossip paths, not this creature hello path.
- **Rust:** Rejects non-creature GUIDs.
- **Status:** covered for this handler, but flow-level gameobject gossip entry remains unverified.
