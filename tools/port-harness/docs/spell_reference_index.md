# Spell Reference Index

This is a navigation aid for the spell subsystem in `reference/core`.

It complements the generated method docs in this folder and the migration dashboard in
`spell_migration.md`.

## High-Value Reference Files

Start discovery with these files:

| File | Why it matters |
|------|----------------|
| `src/game/Spells/Spell.cpp` | Cast pipeline, target selection, execution, packets, helpers |
| `src/game/Spells/Spell.h` | Core spell state and method declarations |
| `src/game/Spells/SpellAuras.cpp` | Aura application, refresh, stacking, and removal behavior |
| `src/game/Spells/SpellEffects.cpp` | Per-effect execution and side effects |
| `src/game/Spells/SpellMgr.cpp` | Spell DB loading, lookup, and metadata assembly |
| `src/game/Spells/SpellEntry.cpp` | Spell template parsing and structure handling |
| `src/game/Spells/SpellCastTargetsInfo.cpp` | Cast-target serialization and target bookkeeping |
| `src/game/Spells/SpellModMgr.cpp` | Spell modifier registration and lookup |
| `src/game/Spells/SpellModifier.cpp` | Spell modifier data helpers |
| `src/game/Server/Packets/Spell.cpp` | Spell opcode handling and packet wiring |

## Documentation Clusters

The generated docs in this folder are easiest to read in these clusters:

### Cast Pipeline

- `Spell_Spell.md`
- `Spell_prepare.md`
- `Spell_cast.md`
- `Spell_handle_immediate.md`
- `Spell_handle_delayed.md`
- `Spell__handle_immediate_phase.md`
- `Spell__handle_finish_phase.md`
- `Spell_update.md`
- `Spell_cancel.md`
- `Spell_Delete.md`

### Target Selection

- `Spell_FillTargetMap.md`
- `Spell_SetTargetMap`
- `Spell_CheckScriptTargeting.md`
- `Spell_CheckTarget.md`
- `Spell_CheckTargetCreatureType.md`
- `Spell_ValidateExplicitTargetMask.md`
- `Spell_AddUnitTarget.md`
- `Spell_AddGOTarget.md`
- `Spell_AddItemTarget.md`
- `Spell_CleanupTargetList.md`

### Validation

- `Spell_CheckCast.md`
- `Spell_CheckItems.md`
- `Spell_CheckPower.md`
- `Spell_CheckRange.md`
- `Spell_CheckCasterAuras.md`
- `Spell_CheckPetCast.md`
- `Spell_CheckTamingSpell.md`
- `Spell_CanOpenLock.md`
- `Spell_CanAutoCast.md`

### Resource and Cost Handling

- `Spell_CalculatePowerCost.md`
- `Spell_TakePower.md`
- `Spell_TakeReagents.md`
- `Spell_TakeCastItem.md`
- `Spell_TakeAmmo.md`
- `Spell_IgnoreItemRequirements.md`

### Effect Execution

- `Spell_DoAllEffectOnTarget.md`
- `Spell_DoSpellHitOnUnit.md`
- `Spell_HandleThreatSpells.md`
- `Spell_ResetEffectDamageAndHeal.md`
- `Spell_HaveTargetsForEffect.md`
- `Spell_GetSpellBatchingEffectDelay.md`

### Client and Network Helpers

- `Spell_SendSpellStart.md`
- `Spell_SendSpellGo.md`
- `Spell_SendSpellCooldown.md`
- `Spell_SendChannelStart.md`
- `Spell_SendChannelUpdate.md`
- `Spell_SendInterrupted.md`
- `Spell_SendAllTargetsMiss.md`
- `Spell_SendResurrectRequest.md`
- `Spell_WriteSpellGoTargets.md`
- `Spell_WriteAmmoToPacket.md`

### Runtime and State Helpers

- `Spell_UpdatePointers.md`
- `Spell_UpdateOriginalCasterPointer.md`
- `Spell_GetCurrentContainer.md`
- `Spell_GetCastingObject.md`
- `Spell_GetAffectiveObject.md`
- `Spell_GetAffectiveCasterObject.md`
- `Spell_ClearCastItem.md`
- `Spell_IsNeedSendToClient.md`
- `Spell_IsTriggeredByProc.md`
- `Spell_IsTriggeredSpellWithRedundentData.md`
- `Spell_ShouldRemoveStealthAuras.md`

## Discovery Workflow

1. Index the full spell subsystem, not just `Spell.cpp`.
2. Pull in the packet layer so opcodes and cast packets stay visible.
3. Read the docs in the cluster that matches the symbol you are porting.
4. Update `spell_migration.md` when a new symbol is documented or a new target file appears.

## Suggested Commands

```bash
npm run dev -- pilot spells
npm run dev -- feature discover spells --sync --assign --queue
npm run dev -- feature queue spells --stage pipeline
```

## Current Dashboard Snapshot

`spell_migration.md` is still anchored on `Spell.cpp` only, so the biggest near-term
gain is widening discovery to `SpellAuras`, `SpellEffects`, `SpellMgr`, `SpellEntry`,
and the packet handler layer.
