# Plan: WorldSession::HandleQuestgiverStatusQueryOpcode

## Required changes
1. Add a script dialog-status hook that can return a raw status value for creature and gameobject questgivers.
2. Apply the C++ fallback rule: use script statuses `0..=7`, otherwise compute relation status.
3. Add hostility suppression before sending creature status.
4. Keep unresolved GUID behaviour as no-send.

## Tests
- Unknown questgiver GUID sends no packet.
- Hostile creature with available quest sends `None`.
- Creature script returns `Available` and overrides relation status.
- Creature script returns greater than `Reward2` and falls back to relation status.
- Gameobject script returns `Available` and overrides relation status.
