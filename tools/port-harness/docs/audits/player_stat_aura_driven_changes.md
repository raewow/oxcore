# Audit: player_stat_aura_driven_changes

## Scope
Audit of aura-driven stat modifications against Rust AuraSystem and UnitModifierGroup.

## Status: PARTIAL — Core aura modifier accumulation present, missing item modifier system, CanModifyStats optimization, and per-source modifier tracking.

---

### 1. Player::_ApplyAllStatBonuses

**C++ Behaviour:**
- Calls `_ApplyAllAuraMods()` then `_ApplyAllItemMods()`
- Sets `CanModifyStats(false)` before and after to avoid intermediate recalculations
- Calls `UpdateAllStats()` to recalculate everything

**Rust Implementation:**
- **MISSING**: No explicit `_ApplyAllStatBonuses` method
- Aura application is handled per-aura in `AuraSystem::apply_modifier` (system.rs lines 835-889)
- Item modifiers not yet implemented
- Stats are recalculated via `StatsSystem::recalculate_all` after each aura change
- **MISSING**: `CanModifyStats` optimization (no intermediate recalculation suppression)

**Verdict: ⚠️ PARTIAL** — Aura application is per-aura, not batched. Missing item mods and CanModifyStats.

---

### 2. Player::_RemoveAllStatBonuses

**C++ Behaviour:**
- Calls `_RemoveAllItemMods()` then `_RemoveAllAuraMods()`
- Sets `CanModifyStats(false)` before and after
- Calls `UpdateAllStats()` to recalculate everything

**Rust Implementation:**
- **MISSING**: No explicit `_RemoveAllStatBonuses` method
- `AuraSystem::remove_modifier` (system.rs lines 893-912):
  - Has TODO: "Track which modifiers came from which sources"
  - Currently just triggers recalculation without actually removing modifiers
  - **BUG**: Modifiers are NOT removed when aura is removed

**Verdict: ⚠️ PARTIAL** — Modifier removal is broken (TODO only). Missing item mods and CanModifyStats.

---

### 3. Player::_ApplyAllItemMods

**C++ Behaviour:**
- Iterates all equipment slots (0 to INVENTORY_SLOT_BAG_END)
- For each non-broken item: applies weapon-dependent aura mods, item bonuses (stats/resistances), ammo bonuses for ranged slot
- Second pass: applies item set bonuses (independent of broken state), applies equip spells and enchantments (if not broken)

**Rust Implementation:**
- **MISSING**: No item modifier system found
- Inventory system exists (inventory/system.rs) but no item stat bonus application

**Verdict: ⚠️ PARTIAL** — Item system exists but stat modifiers from items not implemented.

---

### 4. Player::_RemoveAllItemMods

**C++ Behaviour:**
- First pass: removes item set bonuses (independent of broken state), removes equip spells and enchantments (if not broken)
- Second pass: removes weapon-dependent aura mods, removes item bonuses, removes ammo bonuses

**Rust Implementation:**
- **MISSING**: No item modifier removal system

**Verdict: ⚠️ PARTIAL** — Not implemented.

---

### 5. Unit::_ApplyAllAuraMods

**C++ Behaviour:**
- Iterates all spell aura holders (`m_spellAuraHolders`)
- Calls `ApplyAuraModifiers(true)` on each
- Applies all aura modifiers to unit's modifier groups

**Rust Implementation:**
- Aura application is per-aura when aura is added
- `AuraSystem::apply_modifier` (system.rs lines 835-889):
  - Applies flat value to `TotalValue` (lines 861-871)
  - Applies percent value to `TotalPct` (lines 873-883)
  - Triggers `recalculate_all` (line 887)
  - **MISSING**: Does not use `handle_stat_modifier` (which handles BASE_VALUE/BASE_PCT/TOTAL_VALUE/TOTAL_PCT correctly)
  - **MISSING**: Only handles stats (STR/AGI/STA/INT/SPI), not armor, health, power, resistances, damage

**Verdict: ⚠️ PARTIAL** — Basic stat modifier application present, but not using the full 4-tier system and only handles 5 stats.

---

### 6. Unit::HandleStatModifier

**C++ Behaviour:**
- Adds/removes value from `m_auraModifiersGroup[unitMod][modifierType]`
- `BASE_VALUE`/`TOTAL_VALUE`: `+= amount` (or `-=` if removing)
- `BASE_PCT`/`TOTAL_PCT`: `ApplyPercentModFloatVar` (multiply/divide by 1+amount/100)
- If `CanModifyStats()` is true, triggers appropriate Update* method

**Rust Implementation:**
- `UnitModifierGroup::handle_stat_modifier` (modifiers.rs lines 175-217):
  - `BaseValue`/`TotalValue`: `+= amount` or `-=` (lines 191-195)
  - `BasePct`/`TotalPct`: multiply by `(100+amount)/100` or reverse (lines 199-212)
  - **MATCHES** C++ ApplyPercentModFloatVar logic
  - **MISSING**: `CanModifyStats()` check
  - **MISSING**: Automatic trigger of Update* methods
  - **MISSING**: Only used in aura system, not directly called for item mods

**Verdict: ⚠️ PARTIAL** — Formula logic matches. Missing CanModifyStats and auto-trigger.

---

### 7. Unit::GetModifierValue

**C++ Behaviour:**
- Reads raw modifier value from `m_auraModifiersGroup[unitMod][modifierType]`
- Special case: if `TOTAL_PCT <= 0`, returns 0
- Used by UpdateMaxHealth, UpdateMaxPower, and all stat calculation formulas

**Rust Implementation:**
- `UnitModifierGroup::get_modifier_value` (modifiers.rs lines 141-157):
  - Returns raw value from modifiers array
  - Special case: `TOTAL_PCT <= 0` returns 0 (lines 152-154)
  - **MATCHES** C++ behaviour exactly

**Verdict: ✅ CORRECT** — Exact match.

---

### 8. Unit::GetTotalAuraModifier

**C++ Behaviour:**
- Sums all aura modifiers of a given `SPELL_AURA_*` type across all active auras
- Used for flat bonuses like `SPELL_AURA_MOD_BLOCK_PERCENT`, `SPELL_AURA_MOD_DODGE_PERCENT`, etc.
- Returns total as `int32`

**Rust Implementation:**
- `AuraContainer::get_total_aura_modifier` (container.rs lines 298-304):
  - `self.auras.values().filter(|a| a.aura_type == aura_type).map(|a| a.current_value()).sum()`
  - **MATCHES** C++ behaviour
  - Used in stats system for crit auras (stats/system.rs line 262)
  - Used for healing power (stats/system.rs line 252)

**Verdict: ✅ CORRECT** — Exact match.

---

### 9. Unit::GetTotalAuraModifierByMiscMask

**C++ Behaviour:**
- Sums all aura modifiers of a given type where aura's `misc_value` bitmask overlaps with provided mask
- Used for school-specific modifiers like `SPELL_AURA_MOD_DAMAGE_DONE` where `misc_value` is school mask
- Returns total as `int32`

**Rust Implementation:**
- `AuraContainer::get_total_aura_modifier_by_misc` (container.rs lines 308-314):
  - `self.auras.values().filter(|a| a.aura_type == aura_type && a.misc_value == misc_value).map(|a| a.current_value()).sum()`
  - **DIFFERENT**: Uses equality (`==`) instead of bitmask overlap (`&`)
  - **BUG**: For school masks where multiple schools are combined, this will fail to match
  - Used in stats system for spell power per school (stats/system.rs lines 240-248)
  - The code there manually checks `(a.misc_value & school_mask) != 0` (line 246), which is correct
  - But `get_total_aura_modifier_by_misc` itself uses equality, which is incorrect for bitmask values

**Verdict: ⚠️ PARTIAL** — The manual filtering in stats system is correct, but `get_total_aura_modifier_by_misc` has a bug (uses equality instead of bitmask overlap).

---

## Summary

| Symbol | Status | Gaps |
|--------|--------|------|
| Player::_ApplyAllStatBonuses | ⚠️ Partial | Missing batching, item mods, CanModifyStats |
| Player::_RemoveAllStatBonuses | ⚠️ Partial | Modifier removal broken (TODO), missing item mods |
| Player::_ApplyAllItemMods | ⚠️ Partial | Not implemented |
| Player::_RemoveAllItemMods | ⚠️ Partial | Not implemented |
| Unit::_ApplyAllAuraMods | ⚠️ Partial | Only handles 5 stats, not full 4-tier system |
| Unit::HandleStatModifier | ⚠️ Partial | Missing CanModifyStats, auto-trigger |
| Unit::GetModifierValue | ✅ Complete | Exact match |
| Unit::GetTotalAuraModifier | ✅ Complete | Exact match |
| Unit::GetTotalAuraModifierByMiscMask | ⚠️ Partial | Bug: uses equality instead of bitmask overlap |

**Overall: 2/9 Complete, 7/9 Partial.**

**Critical gaps for full audit pass:**
1. Item modifier system (apply/remove item stat bonuses)
2. `CanModifyStats` optimization to avoid intermediate recalculations
3. Per-source modifier tracking (to remove modifiers when aura is removed)
4. Full 4-tier modifier application for all unit mods (not just 5 stats)
5. Fix `get_total_aura_modifier_by_misc` to use bitmask overlap
6. Auto-trigger stat recalculation on modifier change

## Next Steps

1. Implement per-source modifier tracking in aura system
2. Implement item modifier system
3. Add `CanModifyStats` optimization
4. Extend aura modifier application to all unit mods (armor, health, power, resistance, damage)
5. Fix `get_total_aura_modifier_by_misc` to use bitmask overlap
6. Add auto-trigger for stat recalculation on modifier change
7. Re-audit after gaps are closed
