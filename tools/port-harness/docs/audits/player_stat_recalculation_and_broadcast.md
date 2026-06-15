# Audit: player_stat_recalculation_and_broadcast

## Scope
Audit of C++ `StatSystem.cpp` methods against Rust `StatsSystem::recalculate_all` and `StatsSystem::send_stat_update`.

## Status: PARTIAL — Core formulas ported, missing shapeshift/AP modifiers, weapon damage, and defense skill scaling.

---

### 1. Player::UpdateStats

**C++ Behaviour:**
- Calls `GetTotalStatValue(stat)` using 4-tier modifier formula: `((base * base_pct) + total_value) * total_pct`
- Sets stat value
- Side effects per stat:
  - AGI → UpdateArmor, UpdateAllCritPercentages, UpdateDodgePercentage
  - STA → UpdateMaxHealth
  - INT → UpdateMaxPower(POWER_MANA), UpdateAllSpellCritChances, UpdateArmor
- Always calls UpdateAttackPowerAndDamage(both), UpdateSpellDamageAndHealingBonus, UpdateManaRegen
- Returns false for stat > STAT_SPIRIT

**Rust Implementation:**
- `StatsSystem::recalculate_all` (lines 83-307) computes all stats in one pass
- Uses `unit_mods.calculate_total_value(UnitMods::StatX, base_value)` which implements the same 4-tier formula (modifiers.rs line 221)
- Side effects are handled inline rather than through separate method calls
- Does NOT check for stat > STAT_SPIRIT (not needed since it iterates over known stats)
- AGI side effects: armor (line 216), crit (line 260), dodge (line 280)
- STA side effect: max health (line 130)
- INT side effects: max mana (line 165), spell crit (line 270), armor (line 216)
- Attack power computed (line 208), spell damage (line 232), mana regen (line 298)

**Verdict: ✅ CORRECT** — All side effects present, formula matches.

---

### 2. Player::UpdateSpellDamageAndHealingBonus

**C++ Behaviour:**
- Client-side only display
- For each school (HOLY to MAX_SPELL_SCHOOL): sets `PLAYER_FIELD_MOD_DAMAGE_DONE_POS + school` to `SpellBaseDamageBonusDone(GetSchoolMask(school))`
- Actual damage calculation done at cast time in `Unit::SpellDamageBonusDone`

**Rust Implementation:**
- Lines 232-256: Resets spell power per school, accumulates from `AURA_MOD_DAMAGE_DONE` auras filtered by school mask
- Healing power from `AURA_MOD_HEALING_DONE` auras
- Sets `player.stats.spell_power[school]` and `player.stats.healing_power`

**Verdict: ✅ CORRECT** — Ported correctly, aura-based accumulation matches.

---

### 3. Player::UpdateResistances

**C++ Behaviour:**
- For school > NORMAL: value = 0 if HOLY, else `GetTotalResistanceValue(SpellSchools(school))`
- For school == NORMAL: delegates to `UpdateArmor()`

**Rust Implementation:**
- Lines 224-230: Iterates schools 1-6, uses `UnitMods::from_resistance(school)` and `calculate_total_value(unit_mod, 0.0)`
- School 0 (armor) handled separately at line 222

**Verdict: ✅ CORRECT** — Armor and resistance logic matches.

---

### 4. Player::UpdateArmor

**C++ Behaviour:**
- dynamic = `GetStat(STAT_AGILITY) * 2.0f`
- Adds intellect-derived aura bonus: `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` with `SPELL_SCHOOL_MASK_NORMAL`
- Temporarily adds dynamic to `m_auraModifiersGroup[UNIT_MOD_ARMOR][TOTAL_VALUE]`
- Calls `GetTotalResistanceValue(SPELL_SCHOOL_NORMAL)`
- Sets armor
- Subtracts dynamic back

**Rust Implementation:**
- Lines 214-221: `agi_armor = armor_from_agility(agility)` (2.0 multiplier)
- `armor_total = unit_mods.calculate_total_value(Armor, 0.0) + agi_armor`
- The intellect-derived aura bonus is NOT explicitly added — it would be part of the `calculate_total_value` if auras are applied to the modifier group
- No temporary add/subtract pattern — Rust calculates in one pass

**Verdict: ⚠️ PARTIAL** — The temporary add/subtract pattern is not needed in Rust since the calculation is stateless. However, the intellect-derived aura bonus via `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` needs verification that it's applied through the modifier system.

---

### 5. Player::UpdateMaxHealth

**C++ Behaviour:**
- `value = (BASE_VALUE + create_health) * BASE_PCT + TOTAL_VALUE + stamina_bonus) * TOTAL_PCT`
- `stamina_bonus = GetHealthBonusFromStamina(GetStat(STAT_STAMINA))`
- First 20 stamina = 1 HP per point, additional = 10 HP per point
- Clamped to minimum 1

**Rust Implementation:**
- Lines 130-163: Same formula
- `stamina_bonus = derived::health_bonus_from_stamina(stamina)` (line 130)
- `max_health = ((base_health + health_base_value) * health_base_pct + health_total_value + stamina_bonus) * health_total_pct`
- Clamped to `max(1.0, max_health)` (line 154)
- **Preserves health ratio** when max changes (lines 157-163)

**Verdict: ✅ CORRECT** — Formula matches, ratio preservation present.

---

### 6. Player::UpdateMaxPower

**C++ Behaviour:**
- For mana: `((base_value + create_power) * base_pct + total_value + intellect_bonus) * total_pct`
- `intellect_bonus = GetManaBonusFromIntellect(GetStat(STAT_INTELLECT))` only for POWER_MANA when create_power > 0
- First 20 intellect = 1 mana per point, additional = 15 mana per point
- Clamped to 0 if negative

**Rust Implementation:**
- Lines 165-205: For mana classes (power_type == 0):
  - `int_bonus = derived::mana_bonus_from_intellect(intellect)` (line 169)
  - Same formula with base_value, base_pct, total_value, total_pct
  - Clamped to `max(0.0, max_mana)` (line 192)
  - **Preserves mana ratio** (lines 194-199)
- For non-mana classes: `max_mana = derived::base_max_power(power_type)` (line 203)

**Verdict: ✅ CORRECT** — Formula matches, ratio preservation present.

---

### 7. Player::UpdateAttackPowerAndDamage

**C++ Behaviour:**
- `baseAttackPower = GetAttackPowerFromStrengthAndAgility(ranged, GetStat(STAT_STRENGTH), GetStat(STAT_AGILITY))`
- Class-specific formulas:
  - Ranged: Hunter = level*2 + agi*2 - 10, Rogue/Warrior = level + agi - 10, Druid cat/bear = 0, else = agi - 10
  - Melee: Warrior/Paladin = level*3 + str*2 - 20, Rogue/Hunter = level*2 + str + agi - 20, Shaman = level*2 + str*2 - 20, Druid = str*2 - 20 (with shapeshift variants), Mage/Priest/Warlock = str - 10
- Adds AP_MOD_POSITIVE_FLAT and AP_MOD_NEGATIVE_FLAT modifiers
- Sets UNIT_FIELD_ATTACK_POWER / UNIT_FIELD_RANGED_ATTACK_POWER
- Sets UNIT_FIELD_ATTACK_POWER_MODS (positive in low byte, negative in high byte)
- Sets multiplier field (AP_MOD_PCT - 1.0f)
- Automatically calls UpdateDamagePhysical for the attack type
- For melee: also calls UpdateDamagePhysical for offhand if CanDualWield() and HaveOffhandWeapon()

**Rust Implementation:**
- Lines 208-212: `melee_ap = derived::calculate_melee_ap(class, level, strength, agility)`
- Lines 211-212: `ranged_ap = derived::calculate_ranged_ap(class, level, agility)`
- **MISSING**: AP_MOD_POSITIVE_FLAT/NEGATIVE_FLAT modifiers (not applied)
- **MISSING**: AP multiplier field (UNIT_FIELD_ATTACK_POWER_MULTIPLIER)
- **MISSING**: Offhand damage update condition (CanDualWield + HaveOffhandWeapon)
- **MISSING**: Predatory Strikes aura handling for druid shapeshift
- **MISSING**: Cat Form 1.7.0 patch change (1 AP per agility in cat)
- **MISSING**: Unit field updates for attack power mods (positive/negative split)

**Verdict: ⚠️ PARTIAL** — Base formulas match, but AP modifier system, offhand conditions, and druid shapeshift bonuses are not ported.

---

### 8. Player::UpdateDamagePhysical

**C++ Behaviour:**
- Calls `CalculateMinMaxDamage(attType, false, mindamage, maxdamage)`
- Formula: `((base_value + weapon_damage) * base_pct + total_value + total_phys) * total_pct`
- base_value includes AP/14 * att_speed
- Handles shapeshift (druid form base damage), ammo DPS for ranged, weapon enchants, offhand penalty
- Sets UNIT_FIELD_MINDAMAGE/MAXDAMAGE, UNIT_FIELD_MINOFFHANDDAMAGE/MAXOFFHANDDAMAGE, UNIT_FIELD_MINRANGEDDAMAGE/MAXRANGEDDAMAGE

**Rust Implementation:**
- Lines 288-296: Bare-hand default damage (2.0s speed, AP-based)
  - `min_damage = 1.0 + ap_dmg`
  - `max_damage = 2.0 + ap_dmg`
- **MISSING**: Weapon damage ranges (mindamage/maxdamage from weapon)
- **MISSING**: Shapeshift damage formulas
- **MISSING**: Ammo DPS
- **MISSING**: Offhand 50% penalty
- **MISSING**: Total physical aura modifier
- **MISSING**: Weapon enchant/elemental damage
- Sets all damage fields but with bare-hand defaults only

**Verdict: ⚠️ PARTIAL** — Bare-hand damage present, weapon/shapeshift/ammo not implemented.

---

### 9. Player::UpdateBlockPercentage

**C++ Behaviour:**
- Block = 5.0% base + defense skill bonus + aura bonus
- Only if CanBlock() is true
- Defense skill bonus = `(defense_skill - max_for_level) * 0.04%`
- Aura bonus = `GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT)`
- Clamped to >= 0

**Rust Implementation:**
- Line 286: `player.stats.block_pct = 5.0`
- **MISSING**: Defense skill scaling
- **MISSING**: CanBlock() check
- **MISSING**: SPELL_AURA_MOD_BLOCK_PERCENT aura

**Verdict: ⚠️ PARTIAL** — Base value present, defense skill and aura modifiers missing.

---

### 10. Player::UpdateCritPercentage

**C++ Behaviour:**
- For main hand (BASE_ATTACK) or ranged (RANGED_ATTACK)
- Base = `GetTotalPercentageModValue(CRIT_PERCENTAGE/RANGED_CRIT_PERCENTAGE)` (from agility)
- Adds class base crit: Druid 0.9%, Mage 3.2%, Paladin 0.7%, Priest 3.0%, Shaman 1.7%, Warlock 2.0%
- Adds weapon skill vs max defense skill difference * 0.04%
- Clamped to >= 0

**Rust Implementation:**
- Lines 260-268: `agi_crit = derived::melee_crit_from_agility(class, level, agility)`
- `base_crit = derived::class_base_crit(class)`
- `aura_melee_crit = player.auras.container.get_total_aura_modifier(AURA_MOD_CRIT_PERCENT)`
- `player.stats.melee_crit_pct = base_crit + agi_crit + aura_melee_crit`
- **MISSING**: Weapon skill difference bonus (0.04% per skill point)
- **MISSING**: GetTotalPercentageModValue (base mod group system)
- Ranged crit: `ranged_agi_crit = derived::ranged_crit_from_agility(class, level, agility)` (lines 267-268)

**Verdict: ⚠️ PARTIAL** — Agility and class base crit ported, weapon skill bonus missing.

---

### 11. Player::UpdateAllCritPercentages

**C++ Behaviour:**
- `value = GetMeleeCritFromAgility()`
- Sets `SetBaseModValue(CRIT_PERCENTAGE, PCT_MOD, value)`
- Sets `SetBaseModValue(RANGED_CRIT_PERCENTAGE, PCT_MOD, value)`
- Calls `UpdateCritPercentage(BASE_ATTACK)` and `UpdateCritPercentage(RANGED_ATTACK)`

**Rust Implementation:**
- Not a separate method — handled inline in `recalculate_all` (lines 260-268)
- Both melee and ranged crit computed from agility
- Uses `BaseModifierGroup` (modifiers.rs lines 267-352) for the base mod system
- **MISSING**: The separate base mod group update step

**Verdict: ⚠️ PARTIAL** — Inline calculation is equivalent, but the explicit base mod group update is not separated.

---

### 12. Player::UpdateParryPercentage

**C++ Behaviour:**
- Parry = 5.0% base + defense skill bonus + weapon-based aura bonus
- Only if CanParry() is true
- Defense skill bonus = `(defense_skill - max_for_level) * 0.04%`
- Weapon-based aura bonus = `GetWeaponBasedAuraModifier(BASE_ATTACK, SPELL_AURA_MOD_PARRY_PERCENT)`
- Clamped to >= 0

**Rust Implementation:**
- Line 285: `player.stats.parry_pct = 5.0`
- **MISSING**: Defense skill scaling
- **MISSING**: CanParry() check
- **MISSING**: Weapon-based aura modifier (SPELL_AURA_MOD_PARRY_PERCENT)

**Verdict: ⚠️ PARTIAL** — Base value present, defense skill and aura modifiers missing.

---

### 13. Player::UpdateDodgePercentage

**C++ Behaviour:**
- Dodge = class base + agility bonus + defense skill bonus + aura bonus
- Class base: Druid 0.9%, Mage 3.2%, Paladin 0.7%, Priest 3.0%, Shaman 1.7%, Warlock 2.0%
- Agility bonus = `GetDodgeFromAgility()`
- Defense skill bonus = `(defense_skill - max_for_level) * 0.04%`
- Aura bonus = `GetTotalAuraModifier(SPELL_AURA_MOD_DODGE_PERCENT)`
- Clamped to >= 0

**Rust Implementation:**
- Lines 280-282: `agi_dodge = derived::dodge_from_agility(class, level, agility)`
- `base_dodge = derived::class_base_dodge(class)`
- `player.stats.dodge_pct = base_dodge + agi_dodge`
- **MISSING**: Defense skill scaling
- **MISSING**: SPELL_AURA_MOD_DODGE_PERCENT aura

**Verdict: ⚠️ PARTIAL** — Class base and agility dodge ported, defense skill and aura missing.

---

### 14. Player::UpdateManaRegen

**C++ Behaviour:**
- `power_regen = GetRegenMPPerSpirit()` (spirit-based regen)
- Apply PCT bonus from `SPELL_AURA_MOD_POWER_REGEN_PERCENT` aura
- `power_regen_mp5 = GetTotalAuraModifierByMiscValue(SPELL_AURA_MOD_POWER_REGEN, POWER_MANA) / 5.0f`
- `modManaRegenInterrupt = GetTotalAuraModifier(SPELL_AURA_MOD_MANA_REGEN_INTERRUPT)`
- `m_modManaRegenInterrupt = power_regen_mp5 + power_regen * modManaRegenInterrupt / 100.0f`
- `m_modManaRegen = power_regen_mp5 + power_regen`

**Rust Implementation:**
- Lines 298-303: `player.stats.mana_regen_base = derived::mana_regen_from_spirit(class, spirit)`
- `aura_mana_regen_interrupt = player.auras.container.get_total_aura_modifier(AURA_MOD_MANA_REGEN_INTERRUPT)`
- `player.stats.mana_regen_interrupt = aura_mana_regen_interrupt`
- **MISSING**: `SPELL_AURA_MOD_POWER_REGEN_PERCENT` PCT bonus
- **MISSING**: `SPELL_AURA_MOD_POWER_REGEN` flat MP5 bonus
- **MISSING**: Interrupt regen calculation (power_regen_mp5 + power_regen * interrupt / 100)
- **MISSING**: `m_modManaRegen` (total regen while not casting)

**Verdict: ⚠️ PARTIAL** — Spirit base regen present, aura-based regen and interrupt mechanics incomplete.

---

## Summary

| Symbol | Status | Gaps |
|--------|--------|------|
| UpdateStats | ✅ Complete | None |
| UpdateSpellDamageAndHealingBonus | ✅ Complete | None |
| UpdateResistances | ✅ Complete | None |
| UpdateArmor | ⚠️ Partial | Intellect-derived aura via modifier system needs verification |
| UpdateMaxHealth | ✅ Complete | None |
| UpdateMaxPower | ✅ Complete | None |
| UpdateAttackPowerAndDamage | ⚠️ Partial | AP modifiers, offhand condition, druid shapeshift |
| UpdateDamagePhysical | ⚠️ Partial | Weapon damage, shapeshift, ammo, offhand penalty |
| UpdateBlockPercentage | ⚠️ Partial | Defense skill, CanBlock, aura |
| UpdateCritPercentage | ⚠️ Partial | Weapon skill bonus |
| UpdateAllCritPercentages | ⚠️ Partial | Base mod group separation |
| UpdateParryPercentage | ⚠️ Partial | Defense skill, CanParry, weapon aura |
| UpdateDodgePercentage | ⚠️ Partial | Defense skill, aura |
| UpdateManaRegen | ⚠️ Partial | Regen percent aura, flat MP5, interrupt calc |

**Overall: 4/14 Complete, 10/14 Partial.**

**Critical gaps for full audit pass:**
1. Defense skill scaling for block, parry, dodge, crit
2. Weapon-based aura modifiers (parry)
3. Aura-based modifiers for block, dodge, crit
4. AP modifier system (positive/negative flat, percentage)
5. Druid shapeshift attack power formulas (Predatory Strikes)
6. Weapon damage calculation (min/max with weapon DPS, ammo, shapeshift)
7. Mana regen aura bonuses and interrupt calculation

## Next Steps

1. Port defense skill system integration
2. Port weapon-based aura modifiers
3. Port AP modifier system with positive/negative split
4. Port weapon damage calculation with shapeshift/ammo
5. Port mana regen aura system
6. Re-audit after gaps are closed
