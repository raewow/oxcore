# Plan: questgiver_status_and_gossip_entry

## Goal
Bring the questgiver status and initial gossip entry flow from `rust_compiled` to verified parity with the mapped C++ branches in `WorldSession::HandleQuestgiverStatusQueryOpcode`, `WorldSession::HandleQuestgiverHelloOpcode`, and `WorldSession::GetDialogStatus`.

## Scope
- Keep the current GUID-aware relation lookup and quest menu preparation.
- Add the missing C++ gates and side effects behind small world/session APIs where primitives already exist.
- Stub only where the engine subsystem is not present yet, and mark those stubs explicitly in tests or task notes.

## Changes
1. Add script status override support.
- Add a questgiver dialog-status callback path in the Lua/script manager or a script hook abstraction.
- In `send_quest_giver_status` or a new status resolver, query the script status for creature and gameobject targets before relation status.
- If script status is `0..=7`, use it; if it is greater than `7`, fall back to relation status.
- Add tests for script `Available`, script fallback, and gameobject script status.

2. Add hostile creature suppression.
- Add a creature hostility query usable from quest status resolution.
- Before script/relation status for creature questgivers, return/send `DialogStatus::None` when the creature is hostile to the player.
- Add a test proving hostile questgivers send `None` even when they have available relations.

3. Match hello interaction validation.
- Replace the hello handler's raw `get_creature` check with a `get_npc_if_can_interact_with` equivalent.
- Include the existing constraints available in Rust now: target exists, target is a creature, NPC flags allow gossip/quest interaction, and the player can interact with the target.
- Add tests for unresolved GUID, non-NPC target, and missing NPC flags.

4. Preserve hello cleanup side effects.
- Add player/unit methods for clearing feign death and interacting state if the aura/spell subsystem supports them.
- On hello, remove feign death when present, interrupt channels with interacting flags, and remove interrupt-flag auras.
- If spell/aura primitives are not yet available, introduce minimal no-op backed APIs with tests around call points and keep the feature task below `verified` until real effects exist.

5. Add movement pause parity.
- Add creature state/extra-flag checks for civilian and totem exclusions.
- Call pause-out-of-combat movement before script/default gossip for eligible creatures.
- Add a test with an eligible creature and excluded creature flags.

6. Add gameobject hello support.
- Resolve gameobject questgivers in hello instead of returning for non-creature GUIDs.
- Send prepared gameobject quest menu through the same gossip response path when no script handles it.
- Add a gameobject questgiver hello test.

7. Review low-level hidden quest status.
- Compare `can_take_quest`, level checks, and the hide-low-level-quests config with C++ `DIALOG_STATUS_CHAT` behaviour.
- Add config-aware tests for `Chat` versus `Unavailable`.

## Verification
- Run focused quest tests with `cargo test -p world questgiver` or the closest available package filter.
- Run `cargo check` after implementation.
- Re-run `flow_details("questgiver_status_and_gossip_entry")` and update all three tasks from `rust_compiled` to `verified` only after every audit claim has a Rust behaviour and test/inspection evidence.
- After verification, advance the tasks to `reviewed` and call `update_flow` with the final parity summary so the harness stage can leave `needs_audit`.
