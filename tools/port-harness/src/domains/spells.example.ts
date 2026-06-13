/**
 * Optional spell-system domain config for the `pilot spells` example command.
 * Not used by default indexing — copy patterns into port-harness.config.ts for your domain.
 */

export const SPELL_PILOT_FILES = [
  "src/game/Spells/Spell.h",
  "src/game/Spells/Spell.cpp",
];

export const SPELL_FLOW_DEFS = [
  { name: "cast_pipeline", description: "Spell cast lifecycle", risk: "high" as const },
  { name: "validation", description: "Spell cast validation checks", risk: "high" as const },
  { name: "power_reagents", description: "Power and reagent handling", risk: "medium" as const },
  { name: "target_selection", description: "Spell target selection and mapping", risk: "high" as const },
];

export const SPELL_FLOW_MAP: Record<string, { flow: string; rustTarget: string }> = {
  "Spell::prepare": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::cast": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::finish": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::cancel": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::update": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::CheckCast": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckItems": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckRange": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckPower": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckCasterAuras": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckPetCast": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::TakePower": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::TakeReagents": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::CalculatePowerCost": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::handle_immediate": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::handle_delayed": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::_handle_immediate_phase": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::_handle_finish_phase": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::FillTargetMap": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::SetTargetMap": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
};

export const SPELL_FLOW_CATEGORIES_HINT = `## Known Flow Categories (spell system)
- cast_pipeline: prepare, cast, finish, cancel, update
- validation: CheckCast, CheckItems, CheckRange, CheckPower, CheckCasterAuras
- target_selection: SelectTargets, FillTargetMap, CheckTarget
- power_reagents: TakePower, TakeReagents, CalculatePowerCost
- immediate_delayed: handle_immediate, handle_delayed, _handle_immediate_phase`;
