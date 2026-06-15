# Audit: player_power_modification_and_regen

## Scope
Audit of power modification and regeneration primitives against Rust PowerSystem.

## Status: PARTIAL — Core power types and regen present, missing health regen, shapeshift power switching, and some aura integrations.

---

### 1. Player::RegenerateHealth

**C++ Behaviour:**
- Called every 2 seconds
- If at max health, returns
- Polymorphed: regen 10% max health per tick
- Normal regen: `GetRegenHPPerSpirit() * HealthIncreaseRate`
- Out of combat: multiplied by health regen percent auras (SPELL_AURA_MOD_HEALTH_REGEN_PERCENT), multiplied by 1.5 if sitting/lying down, plus flat food regen auras (SPELL_AURA_MOD_REGEN)
- In combat with SPELL_AURA_MOD_REGEN_DURING_COMBAT: multiplied by combat regen modifier
- Always adds SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT flat bonus
- Carries fractional regen to next tick (`m_carryHealthRegen`)
- Calls ModifyHealth with integer amount

**Rust Implementation:**
- **MISSING**: No dedicated health regen system
- PowerSystem::regen_tick (lines 101-176) has a placeholder comment: "Health regen (out of combat: spirit-based, in combat: 0 for non-druids) — TODO: Health regen formulas when combat system integration is available"
- regen.rs has `calculate_health_regen_per_tick` (lines 113-119) but it's a simplified formula: `spirit * 0.5` per 2 seconds
- **MISSING**: Polymorph check
- **MISSING**: Health regen percent aura (SPELL_AURA_MOD_HEALTH_REGEN_PERCENT)
- **MISSING**: Sitting/lying down 1.5x multiplier
- **MISSING**: Food regen aura (SPELL_AURA_MOD_REGEN)
- **MISSING**: In-combat regen aura (SPELL_AURA_MOD_REGEN_DURING_COMBAT)
- **MISSING**: SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT flat bonus
- **MISSING**: Fractional carry (`m_carryHealthRegen`)

**Verdict: ⚠️ PARTIAL** — Simplified formula exists but not integrated. All aura-based modifiers and polymorph handling missing.

---

### 2. Unit::SetPowerType

**C++ Behaviour:**
- Sets byte value UNIT_FIELD_BYTES_0 for power type
- For POWER_RAGE: sets max to create rage, current to 0
- For POWER_FOCUS: sets max and current to create focus
- For POWER_ENERGY: sets max to create energy, current to 0
- For POWER_HAPPINESS: sets max and current to create happiness
- POWER_MANA: no change
- Updates group update flags for power type

**Rust Implementation:**
- PowerSystem::on_login (lines 269-298):
  - `player.power.power_type = PowerType::for_class(player.class)`
  - Sets max values: Mana = stats.max_mana, Rage = MAX_RAGE (100), Energy = MAX_ENERGY (100)
  - Initializes current: Mana = max, Energy = max, Rage = 0 (implicitly via default)
  - **MISSING**: GROUP_UPDATE_FLAG_POWER handling
  - **MISSING**: UNIT_FIELD_BYTES_0 update
- **MISSING**: Dynamic power type switching (e.g., druid shapeshift mana→rage/energy)
- **MISSING**: Create power values (uses hardcoded maxes instead of create values)

**Verdict: ⚠️ PARTIAL** — Basic power type initialization on login present. Dynamic switching and group update flags missing.

---

### 3. Unit::ModifyPower

**C++ Behaviour:**
- Adds delta to current power
- If result <= 0: sets power to 0, returns -curPower
- If result < maxPower: sets power to result, returns delta
- If result >= maxPower and curPower != maxPower: sets power to maxPower, returns maxPower - curPower
- If already at max, returns 0
- No-op if delta == 0
- Identical to ModifyHealth logic

**Rust Implementation:**
- PowerSystem::consume_power (lines 180-208):
  - `player.power.current[idx] -= amount` (no return of actual consumption)
  - Returns boolean (success/fail)
  - **MISSING**: Return of actual power change
- PowerSystem::restore_power (lines 212-228):
  - `player.power.current[idx] = (current + amount).min(max)`
  - **MISSING**: Return of actual power gain
- PowerSystem::regen_tick (lines 101-176):
  - Mana: `power.current[idx] = (current + whole).min(max)` (lines 140)
  - Rage: `power.current[idx].saturating_sub(RAGE_DECAY_PER_TICK)` (lines 152-153)
  - Energy: `power.current[idx] = (current + ENERGY_REGEN_PER_TICK).min(max)` (lines 159-160)
- **MISSING**: Unified ModifyPower primitive with proper clamping and return semantics
- **MISSING**: Power change from auras (energize/drain)

**Verdict: ⚠️ PARTIAL** — Power changes are scattered inline. No unified primitive with return semantics.

---

### 4. Unit::SetPower

**C++ Behaviour:**
- Sets current power value for a given power type
- Updates UNIT_FIELD_POWER1 + power index
- If player with group: sets GROUP_UPDATE_FLAG_POWER
- If controlled pet and owner has group: sets GROUP_UPDATE_FLAG_PET_POWER

**Rust Implementation:**
- PowerSystem::broadcast_power_value (lines 324-338):
  - Creates `ValuesUpdateBlock` with `UNIT_FIELD_POWER1 + power_type`
  - Sends `SmsgUpdateObject` packet
  - **MISSING**: Direct value setting (always goes through broadcast)
- **MISSING**: GROUP_UPDATE_FLAG_POWER
- **MISSING**: GROUP_UPDATE_FLAG_PET_POWER
- **MISSING**: Low-level setter that doesn't broadcast

**Verdict: ⚠️ PARTIAL** — Broadcast mechanism present, but no direct setter with group update flags.

---

### 5. Unit::SetMaxPower

**C++ Behaviour:**
- Sets max power value for a given power type
- Updates UNIT_FIELD_MAXPOWER1 + power index
- If player with group: sets GROUP_UPDATE_FLAG_MAX_POWER
- If controlled pet and owner has group: sets GROUP_UPDATE_FLAG_PET_MAX_POWER

**Rust Implementation:**
- PowerSystem::on_login (lines 276-284):
  - Sets max values for mana, rage, energy
  - **MISSING**: UNIT_FIELD_MAXPOWER1 update
  - **MISSING**: GROUP_UPDATE_FLAG_MAX_POWER
  - **MISSING**: GROUP_UPDATE_FLAG_PET_MAX_POWER
- StatsSystem::recalculate_all (stats/system.rs lines 165-205):
  - Updates `player.stats.max_mana` (which is synced to `player.power.max[0]` in regen_tick line 113)

**Verdict: ⚠️ PARTIAL** — Max values tracked internally, but not broadcast to client or group.

---

### 6. Mana Regen (from Player::RegeneratePower / UpdateManaRegen)

**C++ Behaviour:**
- 5-second rule: no spirit regen for 5s after mana use
- Base regen from spirit + MP5 from gear + aura flat regen
- SPELL_AURA_MOD_POWER_REGEN_PERCENT: PCT bonus on spirit base regen
- SPELL_AURA_MOD_POWER_REGEN: flat MP5 bonus
- SPELL_AURA_MOD_MANA_REGEN_INTERRUPT: interrupt regen percentage
- `m_modManaRegenInterrupt = power_regen_mp5 + power_regen * interrupt / 100`
- `m_modManaRegen = power_regen_mp5 + power_regen`

**Rust Implementation:**
- PowerSystem::regen_tick (lines 119-141):
  - 5-second rule: `power.spirit_regen_active = now >= last_mana_use_time + 5000ms` (line 121)
  - `calculate_mana_regen_per_tick(spirit_regen, mp5, spirit_regen_active, casting_regen_pct)` (line 124)
  - Drink regen from aura type 85 (AURA_MOD_POWER_REGEN) with misc_value 0 (line 104-110)
  - Accumulator for fractional regen (lines 135-139)
  - **MISSING**: SPELL_AURA_MOD_POWER_REGEN_PERCENT (PCT on spirit)
  - **MISSING**: SPELL_AURA_MOD_MANA_REGEN_INTERRUPT calculation
  - **MISSING**: `m_modManaRegen` and `m_modManaRegenInterrupt` tracking

**Verdict: ⚠️ PARTIAL** — 5-second rule and base regen present. Missing percent aura and interrupt regen.

---

### 7. Rage Generation/Decay

**C++ Behaviour:**
- From damage dealt: `damage * 7.5 / level` (capped)
- From damage taken: `damage * 2.5 / level` (capped)
- Decay out of combat: 2 per second (4 per 2s tick)

**Rust Implementation:**
- regen.rs lines 55-70: `rage_from_damage_dealt` and `rage_from_damage_taken` with exact formulas
- PowerSystem::on_damage_dealt (lines 231-246): adds rage on damage dealt
- PowerSystem::on_damage_taken (lines 250-265): adds rage on damage taken
- PowerSystem::regen_tick (lines 144-154): rage decay out of combat
- **MISSING**: Combat state check for decay (hardcoded `in_combat = false`)

**Verdict: ✅ CORRECT** — Formulas match. Combat state integration is a TODO.

---

### 8. Energy Regeneration

**C++ Behaviour:**
- Restore 20 per 2-second tick
- Always regenerates (in combat and out)
- Capped at 100

**Rust Implementation:**
- PowerSystem::regen_tick (lines 157-160):
  - `power.current[idx] = (current + ENERGY_REGEN_PER_TICK).min(max)`
  - `ENERGY_REGEN_PER_TICK = 20` (regen.rs line 85)
  - `MAX_ENERGY = 100` (regen.rs line 86)

**Verdict: ✅ CORRECT** — Exact match.

---

## Summary

| Symbol | Status | Gaps |
|--------|--------|------|
| Player::RegenerateHealth | ⚠️ Partial | Not integrated, missing all aura modifiers, polymorph, sitting bonus |
| Unit::SetPowerType | ⚠️ Partial | Dynamic switching, group flags, create values missing |
| Unit::ModifyPower | ⚠️ Partial | Scattered inline, no unified primitive with return |
| Unit::SetPower | ⚠️ Partial | No direct setter, missing group flags |
| Unit::SetMaxPower | ⚠️ Partial | Not broadcast, missing group flags |

**Additional findings:**
- Mana regen: ⚠️ Partial (5-second rule present, percent aura and interrupt regen missing)
- Rage generation: ✅ Complete (formulas match, combat state TODO)
- Energy regen: ✅ Complete (exact match)

**Overall: 2/5 Complete, 3/5 Partial.**

**Critical gaps for full audit pass:**
1. Health regen system integration with all aura modifiers
2. Unified `ModifyPower` primitive with return semantics
3. Power type dynamic switching (druid shapeshift)
4. `SetPower`/`SetMaxPower` with group update flags
5. `SPELL_AURA_MOD_POWER_REGEN_PERCENT` aura
6. `SPELL_AURA_MOD_MANA_REGEN_INTERRUPT` calculation
7. Polymorph health regen (10% max)
8. Combat state integration for rage decay

## Next Steps

1. Implement health regen system with aura modifiers
2. Create unified `ModifyPower` primitive
3. Port power type switching for druid forms
4. Add group update flags for power changes
5. Port `SPELL_AURA_MOD_POWER_REGEN_PERCENT` aura
6. Port `SPELL_AURA_MOD_MANA_REGEN_INTERRUPT` calculation
7. Integrate combat state for rage decay
8. Re-audit after gaps are closed
