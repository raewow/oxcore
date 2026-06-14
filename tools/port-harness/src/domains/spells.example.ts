/**
 * Optional spell-system domain config for the `pilot spells` example command.
 * Not used by default indexing — copy patterns into port-harness.config.ts for your domain.
 */

export const SPELL_PILOT_FILES = [
  "src/game/Spells/Spell.h",
  "src/game/Spells/Spell.cpp",
  "src/game/Spells/SpellAuras.h",
  "src/game/Spells/SpellAuras.cpp",
  "src/game/Spells/SpellEffects.cpp",
  "src/game/Spells/SpellMgr.h",
  "src/game/Spells/SpellMgr.cpp",
  "src/game/Spells/SpellEntry.h",
  "src/game/Spells/SpellEntry.cpp",
  "src/game/Spells/SpellCastTargetsInfo.h",
  "src/game/Spells/SpellCastTargetsInfo.cpp",
  "src/game/Spells/SpellModifier.h",
  "src/game/Spells/SpellModifier.cpp",
  "src/game/Spells/SpellModMgr.h",
  "src/game/Spells/SpellModMgr.cpp",
  "src/game/Server/Packets/Spell.h",
  "src/game/Server/Packets/Spell.cpp",
];

export const SPELL_FLOW_DEFS = [
  { name: "cast_pipeline", description: "Spell cast lifecycle", risk: "high" as const },
  { name: "validation", description: "Spell cast validation checks", risk: "high" as const },
  { name: "power_reagents", description: "Power and reagent handling", risk: "medium" as const },
  { name: "target_selection", description: "Spell target selection and mapping", risk: "high" as const },
  { name: "effect_application", description: "Per-target spell effect execution", risk: "high" as const },
  { name: "packet_wiring", description: "Spell packet send/receive handlers", risk: "medium" as const },
  { name: "spell_data", description: "DBC-backed spell metadata and managers", risk: "medium" as const },
];

export const SPELL_FLOW_MAP: Record<string, { flow: string; rustTarget: string }> = {
  "Spell::prepare": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::cast": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::finish": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::cancel": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::update": { flow: "cast_pipeline", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::Spell": { flow: "spell_data", rustTarget: "src/world/game/player/spells/state.rs" },
  "Spell::CheckCast": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckItems": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckRange": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckPower": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckCasterAuras": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckPetCast": { flow: "validation", rustTarget: "src/world/game/player/spells/validation.rs" },
  "Spell::CheckTarget": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::CheckTargetCreatureType": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::CheckScriptTargeting": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::FillTargetMap": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::SetTargetMap": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::AddUnitTarget": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::AddGOTarget": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::AddItemTarget": { flow: "target_selection", rustTarget: "src/world/game/player/spells/targets.rs" },
  "Spell::TakePower": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::TakeReagents": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::TakeCastItem": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::TakeAmmo": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::CalculatePowerCost": { flow: "power_reagents", rustTarget: "src/world/game/player/spells/modifiers.rs" },
  "Spell::DoAllEffectOnTarget": { flow: "effect_application", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::DoSpellHitOnUnit": { flow: "effect_application", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::HandleThreatSpells": { flow: "effect_application", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::handle_immediate": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::handle_delayed": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::_handle_immediate_phase": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::_handle_finish_phase": { flow: "immediate_delayed", rustTarget: "src/world/game/player/spells/system.rs" },
  "Spell::SendSpellStart": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendSpellGo": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendSpellCooldown": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendChannelStart": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendChannelUpdate": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendInterrupted": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
  "Spell::SendResurrectRequest": { flow: "packet_wiring", rustTarget: "src/world/handlers/spells.rs" },
};

export const SPELL_FLOW_CATEGORIES_HINT = `## Known Flow Categories (spell system)
- cast_pipeline: prepare, cast, finish, cancel, update
- validation: CheckCast, CheckItems, CheckRange, CheckPower, CheckCasterAuras
- target_selection: SelectTargets, FillTargetMap, CheckTarget
- power_reagents: TakePower, TakeReagents, CalculatePowerCost
- effect_application: DoAllEffectOnTarget, DoSpellHitOnUnit, HandleThreatSpells
- packet_wiring: SendSpellStart, SendSpellGo, SendChannelStart, SendResurrectRequest
- spell_data: Spell.cpp/Spell.h, SpellMgr, SpellEntry, target-info helpers
- immediate_delayed: handle_immediate, handle_delayed, _handle_immediate_phase`;
