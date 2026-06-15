# Plan: WorldSession::HandleQuestgiverHelloOpcode

## Required changes
1. Add a Rust `get_npc_if_can_interact_with` equivalent and use it in the handler.
2. Add player/unit APIs for feign death cleanup and interacting spell/aura interruption.
3. Add creature movement pause support with civilian and totem exclusions.
4. Confirm whether default gossip menu id is represented in `GossipSystem`; if not, pass it through instead of sending only quest entries.
5. Keep Lua `OnGossipHello` as the first chance to handle the interaction.

## Tests
- Invalid or non-interactable creature returns without gossip packet.
- Feign death/interacting cleanup methods are invoked on valid hello.
- Eligible creature movement is paused; civilian/totem creatures are not paused.
- Lua `OnGossipHello` actions suppress default gossip.
- Default gossip sends quest entries when script does not handle the interaction.
