# Rust Code Audit: Stats, Power, Combat Integration

## Executive Summary

The Rust implementation has a solid foundation with the 4-tier modifier system, core stat formulas, and power management. However, there are **critical gaps** that prevent full parity with the C++ reference:

1. **Aura modifier removal is broken** (TODO - doesn't actually remove modifiers)
2. **BaseModifierGroup is unused** (crit/dodge/block base mods not integrated)
3. **Defense skill scaling missing** (affects crit, dodge, parry, block)
4. **Weapon damage system is stubbed** (bare-hand defaults only)
5. **Health regen not implemented**
6. **Combat system integration incomplete** (rage decay, threat, durability)
7. **Mana regen aura bonuses incomplete** (missing percent modifier, interrupt calc)
8. **Druid shapeshift AP not implemented** (Predatory Strikes, cat form 1.7 patch)

---

## Detailed Rust Code Audit

### 1. StatsSystem Architecture (system.rs)

**Strengths:**
- Stateless design with clear lifecycle hooks (init, update, shutdown, login, logout)
- `OnceLock<BaseStatsData>` for lazy initialization
- `recalculate_all` is the single entry point for all stat updates
- Health/mana ratio preservation on max changes (lines 157-163, 194-199)
- `dirty` flag for broadcast tracking (line 305)

**Issues:**

#### Issue 1.1: `on_player_login` sets health/mana to max unconditionally
```rust
player.stats.health = player.stats.max_health;
player.stats.mana = player.stats.max_mana;
```
**Problem:** This ignores saved health/mana from the database. Should load from DB first.
**Severity:** Medium - affects character persistence.

#### Issue 1.2: `on_level_up` sets health/mana to max unconditionally
```rust
player.stats.health = player.stats.max_health;
player.stats.mana = player.stats.max_mana;
```
**Problem:** C++ preserves current health/mana ratio, then heals to full only on the level-up info packet. This Rust implementation resets to max regardless.
**Severity:** Low - cosmetic difference, but not matching C++.

#### Issue 1.3: `send_stat_update` reads `player.power.current[0]` into `player.stats.mana`
```rust
player.stats.mana = player.power.current[0];
```
**Problem:** This creates a circular dependency. The stats system shouldn't modify the power system. Power system should be the source of truth for mana.
**Severity:** Medium - architectural issue.

#### Issue 1.4: `send_stat_update` sets `UNIT_FIELD_POWER1` but not `UNIT_FIELD_MAXPOWER1`
Wait, it does set both (lines 488-489). OK.

#### Issue 1.5: `UNIT_FIELD_ATTACK_POWER_MODS` and `UNIT_FIELD_ATTACK_POWER_MULTIPLIER` not set
The Rust code only sets `UNIT_FIELD_ATTACK_POWER` and `UNIT_FIELD_RANGED_ATTACK_POWER` (lines 495-496). The C++ code also sets:
- `UNIT_FIELD_ATTACK_POWER_MODS` (positive/negative split)
- `UNIT_FIELD_ATTACK_POWER_MULTIPLIER` (AP_MOD_PCT - 1.0)
**Severity:** High - client displays will be wrong.

#### Issue 1.6: `recalculate_all` uses `class_base.base_health` but C++ uses `GetCreateHealth()`
```rust
let max_health = ((class_base.base_health as f32 + health_base_value) * health_base_pct ...)
```
**Problem:** `GetCreateHealth()` in C++ is the base health from level/class tables. Using `class_base.base_health` is correct for the first term, but the C++ code also adds `GetModifierValue(unitMod, BASE_VALUE)` as a separate term. In Rust, `health_base_value` is the `BaseValue` modifier. This is correct.
**Severity:** None - implementation is correct.

#### Issue 1.7: `get_level_up_gains` returns deltas but they are not used
```rust
let _str_gain = new_str.saturating_sub(old_str);
```
**Problem:** The gains are computed but not returned or stored. The C++ code sends these in `SMSG_LEVELUP_INFO`.
**Severity:** Low - SMSG_LEVELUP_INFO is not implemented yet.

---

### 2. UnitModifierGroup (modifiers.rs)

**Strengths:**
- Correct 4-tier formula implementation: `((base_value * base_pct) + total_value) * total_pct`
- Proper default values (BASE_PCT=1.0, TOTAL_PCT=1.0)
- Offhand damage defaults to 50% (0.5)
- `handle_stat_modifier` correctly handles flat vs percentage modifiers
- `get_modifier_value` returns 0 for TOTAL_PCT <= 0

**Issues:**

#### Issue 2.1: `BaseModifierGroup` is defined but never used in `StatsState`
Looking at `StatsState` (state.rs lines 28-29):
```rust
pub unit_mods: UnitModifierGroup,
pub base_mods: BaseModifierGroup,
```
The `base_mods` field exists but `recalculate_all` never uses it.
**Problem:** BaseModifierGroup was intended for crit/dodge/block base modifiers (like `GetMeleeCritFromAgility()`), but the Rust code computes these inline in `derived.rs` instead.
**Severity:** Medium - code duplication and architectural drift.

#### Issue 2.2: No `handle_stat_modifier` integration for armor, health, power, resistance
The `handle_stat_modifier` method exists but is only called by the aura system for 5 stats. The C++ code uses it for all unit mods (armor, health, mana, resistances, damage).
**Severity:** High - prevents proper aura-driven changes to health, armor, etc.

---

### 3. Derived Formulas (derived.rs)

**Strengths:**
- All core formulas are pure functions (testable)
- Health/mana bonus formulas match C++ exactly
- Class base crit values match C++
- Spirit regen formula has class-specific rates

**Issues:**

#### Issue 3.1: `calculate_melee_ap` for Druid is wrong
```rust
CLASS_DRUID => strength * 2.0 - 20.0,
```
**Problem:** C++ has complex shapeshift logic for Druid:
- Cat form (patch 1.7+): `level * mLevelMult + strength * 2.0 + agility - 20.0`
- Bear/Direbear: `level * mLevelMult + strength * 2.0 - 20.0`
- Default: `strength * 2.0 - 20.0`
The Rust code only implements the default case.
**Severity:** High - Druid attack power is completely wrong in forms.

#### Issue 3.2: `calculate_ranged_ap` for Druid is wrong
```rust
CLASS_HUNTER => level * 2.0 + agility * 2.0 - 10.0,
CLASS_ROGUE | CLASS_WARRIOR => level + agility - 10.0,
_ => agility - 10.0,
```
**Problem:** C++ Druid ranged AP:
- Cat/Bear/Direbear: `0.0` (no ranged AP in forms)
- Default: `agility - 10.0`
**Severity:** Medium - Druid ranged AP in forms should be 0.

#### Issue 3.3: Missing `dodge_from_defense_skill` and `crit_from_defense_skill`
The C++ code adds defense skill scaling:
- `crit += (weapon_skill - max_for_level) * 0.04`
- `dodge += (defense_skill - max_for_level) * 0.04`
- `parry += (defense_skill - max_for_level) * 0.04`
- `block += (defense_skill - max_for_level) * 0.04`
**Severity:** High - all defense-related percentages are wrong.

#### Issue 3.4: `mana_regen_from_spirit` formula is simplified
```rust
CLASS_MAGE | CLASS_PRIEST | CLASS_WARLOCK => (spirit / 4.0 + 12.5).max(0.0),
```
**Problem:** C++ uses `GetRegenMPPerSpirit()` which is more complex (class-specific interpolation like crit/dodge). The Rust formula is a rough approximation.
**Severity:** Medium - mana regen values may be slightly off.

#### Issue 3.5: Missing `health_regen_from_spirit`
No function for health regeneration from spirit.
**Severity:** Medium - health regen not implemented.

---

### 4. PowerSystem (power/system.rs)

**Strengths:**
- 2-second tick accumulator with proper carry
- 5-second rule for mana
- `consume_power` resets 5-second timer
- `on_damage_dealt` / `on_damage_taken` for rage generation
- `broadcast_power_value` for client updates

**Issues:**

#### Issue 4.1: `regen_tick` uses `get_time_ms()` which is wall-clock time
```rust
fn get_time_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
```
**Problem:** Should use the world tick time (from `diff`) for deterministic behavior. Wall-clock time can drift.
**Severity:** Medium - affects 5-second rule accuracy.

#### Issue 4.2: Rage decay uses `in_combat = false` hardcoded
```rust
let in_combat = false; // TODO: Get from CombatSystem
```
**Problem:** Rage never decays because combat state is always false.
**Severity:** High - warriors will keep rage forever.

#### Issue 4.3: `on_login` doesn't load saved power values from DB
```rust
// TODO: Load saved power values from DB
```
**Severity:** Medium - affects character persistence.

#### Issue 4.4: `broadcast_power_value` doesn't broadcast max power
Only broadcasts current power via `UNIT_FIELD_POWER1`. Max power changes (from gear/buffs) are not sent.
**Severity:** Medium - client won't see max power changes.

#### Issue 4.5: `PowerState` has `is_eating` and `is_drinking` but never used
These fields are set but never checked in the regen logic.
**Severity:** Low - drinking regen works via aura system instead.

#### Issue 4.6: `mp5_from_gear` is never populated
The field exists but no code sets it from equipment.
**Severity:** High - MP5 from gear is ignored.

#### Issue 4.7: `casting_regen_pct` is never populated
The field exists but no code sets it from talents/auras.
**Severity:** High - Meditation talent doesn't work.

---

### 5. AuraSystem Integration (auras/system.rs)

**Strengths:**
- `apply_modifier` correctly handles flat and pct values
- `get_total_aura_modifier` and `get_total_aura_modifier_by_misc` exist in container
- `absorb_damage` is well-implemented

**Issues:**

#### Issue 5.1: `apply_modifier` doesn't use `handle_stat_modifier`
```rust
player.stats.unit_mods.set_modifier_value(
    unit_mod,
    UnitModifierType::TotalValue,
    current + modifier.flat_value,
);
```
**Problem:** It uses `set_modifier_value` directly instead of `handle_stat_modifier`. This means:
- Percentage modifiers are added as flat values instead of being applied via `ApplyPercentModFloatVar`
- The `handle_stat_modifier` method exists but is never called
**Severity:** High - percentage stat auras are wrong.

#### Issue 5.2: `remove_modifier` is a TODO
```rust
// TODO: Track which modifiers came from which sources
// For now, we just recalc everything
```
**Problem:** When an aura is removed, the modifier stays in the unit_mods. Recalculating doesn't help because the old value is still there.
**Severity:** CRITICAL - aura removal is completely broken.

#### Issue 5.3: `apply_modifier` only handles 5 stats
```rust
let unit_mod = match modifier.stat {
    STAT_STRENGTH => UnitMods::StatStrength,
    ...
    _ => return,
};
```
**Problem:** No handling for armor, health, mana, resistances, damage, attack power modifiers.
**Severity:** High - many aura types don't work.

#### Issue 5.4: `get_total_aura_modifier_by_misc` uses equality instead of bitmask
```rust
.filter(|a| a.aura_type == aura_type && a.misc_value == misc_value)
```
**Problem:** For school masks (e.g., `misc_value = 0x7F` for all schools), this will not match.
**Severity:** Medium - only affects combined school auras.

---

### 6. CombatSystem Integration (combat/system.rs)

**Strengths:**
- `apply_damage` correctly reduces health with `saturating_sub`
- `enter_combat` is called on damage
- `execute_attack` handles hit table, damage calculation, and outcome

**Issues:**

#### Issue 6.1: `apply_damage` doesn't call `PowerSystem::on_damage_taken`
```rust
// Apply damage to health
let current_health = player.stats.health;
let new_health = current_health.saturating_sub(damage);
player.stats.health = new_health;

// Enter combat
player.combat.enter_combat(target);
```
**Problem:** No rage generation for the victim. No spell interrupt.
**Severity:** High - warriors don't generate rage from being hit.

#### Issue 6.2: `apply_damage` doesn't broadcast health update
```rust
// TODO: Broadcast health update
```
**Severity:** Medium - health bar doesn't update on damage.

#### Issue 6.3: `apply_damage` doesn't check for death
```rust
// TODO: Check for death
```
**Severity:** High - players can go below 0 health without dying.

#### Issue 6.4: `execute_attack` doesn't call `PowerSystem::on_damage_dealt`
```rust
if damage_result.damage > 0 {
    self.apply_damage(attack.target, damage_result.damage, player_mgr)
        .await?;
}
```
**Problem:** No rage generation for the attacker on damage dealt.
**Severity:** High - warriors don't generate rage from dealing damage.

---

## Action Plan

### Phase 1: Fix Critical Bugs (CRITICAL)

**1.1. Fix aura modifier removal**
- File: `src/world/game/player/auras/system.rs`
- Change `remove_modifier` to track and reverse modifiers
- Options:
  a) Store modifiers in `Aura` struct and reverse them on removal
  b) Store a list of `(StatModifier, old_value)` in `AuraState`
  c) Rebuild all unit_mods from scratch on each aura change (nuclear option)
- **Recommendation:** Option (a) - add `applied_modifiers: Vec<AppliedModifier>` to `Aura` struct

**1.2. Fix `apply_modifier` to use `handle_stat_modifier`**
- File: `src/world/game/player/auras/system.rs`
- Replace `set_modifier_value` with `handle_stat_modifier`
- For percentage auras, use `handle_stat_modifier` with `UnitModifierType::BasePct` or `TotalPct`

**1.3. Fix PowerSystem rage decay**
- File: `src/world/game/player/power/system.rs`
- Integrate `CombatSystem` to check `player.combat.in_combat`
- Or pass `in_combat` boolean to `regen_tick`

**1.4. Fix CombatSystem rage generation**
- File: `src/world/game/combat/system.rs`
- Call `world.systems.power.on_damage_dealt` and `on_damage_taken`

**1.5. Fix CombatSystem death check**
- File: `src/world/game/combat/system.rs`
- After `apply_damage`, check `health == 0` and call `DeathSystem::on_killed`

### Phase 2: Fill Core Gaps (HIGH)

**2.1. Add defense skill scaling**
- File: `src/world/game/player/stats/derived.rs`
- Add `defense_bonus_from_skill(defense_skill, level) -> f32`
- Modify `recalculate_all` to add defense skill bonus to crit, dodge, parry, block
- Need `SkillState` integration (defense skill from `player.skills`)

**2.2. Add weapon damage calculation**
- File: `src/world/game/player/stats/system.rs`
- Replace bare-hand defaults with `CalculateMinMaxDamage` equivalent
- Need `CombatSystem` weapon data (weapon_min, weapon_max, weapon_speed, ammo DPS)
- Add `WeaponSystem` or use `CombatSystem` weapon data

**2.3. Add mana regen aura bonuses**
- File: `src/world/game/player/power/system.rs`
- Add `SPELL_AURA_MOD_POWER_REGEN_PERCENT` (multiply spirit regen)
- Add `SPELL_AURA_MOD_MANA_REGEN_INTERRUPT` (interrupt calculation)
- Add `SPELL_AURA_MOD_REGEN` (health regen)
- Add `SPELL_AURA_MOD_HEALTH_REGEN_PERCENT` (health regen multiplier)
- Add `SPELL_AURA_MOD_REGEN_DURING_COMBAT` (in-combat regen)
- Add `SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT` (in-combat flat regen)

**2.4. Add druid shapeshift attack power**
- File: `src/world/game/player/stats/derived.rs`
- Add `calculate_melee_ap` with shapeshift form parameter
- Add Predatory Strikes aura detection
- Handle patch 1.7.0 cat form changes (1 AP per agility)

**2.5. Add `UNIT_FIELD_ATTACK_POWER_MODS` and `UNIT_FIELD_ATTACK_POWER_MULTIPLIER`**
- File: `src/world/game/player/stats/system.rs`
- Track positive/negative AP modifiers in `StatsState`
- Add AP multiplier (AP_MOD_PCT - 1.0)
- Add these to `send_stat_update`

**2.6. Add `BaseModifierGroup` usage**
- File: `src/world/game/player/stats/system.rs`
- Use `base_mods` for crit from agility
- Set `FLAT_MOD` from `melee_crit_from_agility`
- Set `PctMod` from aura `AURA_MOD_CRIT_PERCENT`
- Use `get_total_base_mod_value` for final crit calculation

### Phase 3: Integration & Polish (MEDIUM)

**3.1. Add health regen system**
- File: `src/world/game/player/power/system.rs` (or new `health_regen.rs`)
- Implement `RegenerateHealth` equivalent
- Handle polymorph, sitting, combat, aura bonuses

**3.2. Add MP5 from gear**
- File: `src/world/game/player/inventory/system.rs`
- Scan equipped items for mana regen stats
- Populate `power.mp5_from_gear`

**3.3. Add casting regen pct from talents**
- File: `src/world/game/player/talents/system.rs`
- After talent application, check for Meditation talents
- Set `power.casting_regen_pct`

**3.4. Add power type switching**
- File: `src/world/game/player/power/system.rs`
- Add `set_power_type` method
- Handle druid shapeshift (mana -> rage -> energy)
- Reset current values appropriately

**3.5. Add `CanModifyStats` optimization**
- File: `src/world/game/player/stats/system.rs`
- Add `can_modify_stats` flag to `StatsState`
- Skip `recalculate_all` if false
- Set false during batch aura/item changes

**3.6. Add death check after damage**
- File: `src/world/game/combat/system.rs`
- Call `world.systems.death.on_killed` when health reaches 0

**3.7. Add health update broadcast**
- File: `src/world/game/combat/system.rs`
- After `apply_damage`, broadcast health update

**3.8. Add threat generation**
- File: `src/world/game/combat/system.rs`
- Call creature threat manager when damage is dealt to creatures

**3.9. Add durability loss**
- File: `src/world/game/combat/system.rs`
- Apply durability loss chance on damage for both attacker and victim

**3.10. Add spell interrupt on damage**
- File: `src/world/game/combat/system.rs`
- Check if victim has active spell cast
- Interrupt if damage threshold met

### Phase 4: Test & Verify (ALL)

**4.1. Write unit tests for all derived formulas**
- Test health/mana bonuses at boundary conditions (20 points)
- Test AP formulas for all classes
- Test crit/dodge formulas for all classes
- Test defense skill scaling

**4.2. Write integration tests for aura modifiers**
- Apply aura -> verify stat change
- Remove aura -> verify stat reverts
- Apply percentage aura -> verify multiplicative behavior

**4.3. Write integration tests for power system**
- Test 5-second rule
- Test mana regen with spirit
- Test rage generation/depletion
- Test energy tick

**4.4. Write integration tests for damage system**
- Test auto-attack damage calculation
- Test absorb shield reduction
- Test death handling
- Test threat generation

**4.5. Run full test suite**
- `cargo test --test player_stats`
- `cargo test --test power_system`
- `cargo test --test combat_system`

---

## Priority Matrix

| Priority | Task | Effort | Impact |
|----------|------|--------|--------|
| CRITICAL | Fix aura modifier removal | 4h | CRITICAL - all auras broken |
| CRITICAL | Fix CombatSystem death check | 2h | HIGH - players can't die |
| CRITICAL | Fix CombatSystem rage generation | 3h | HIGH - warriors broken |
| HIGH | Fix `apply_modifier` to use `handle_stat_modifier` | 3h | HIGH - % auras wrong |
| HIGH | Fix PowerSystem rage decay | 1h | HIGH - rage never decays |
| HIGH | Add defense skill scaling | 4h | HIGH - all % stats wrong |
| HIGH | Add weapon damage calculation | 6h | HIGH - damage is stubbed |
| HIGH | Add druid shapeshift AP | 3h | MEDIUM - druid broken |
| MEDIUM | Add health regen system | 4h | MEDIUM - health regen missing |
| MEDIUM | Add mana regen aura bonuses | 4h | MEDIUM - regen incomplete |
| MEDIUM | Add MP5 from gear | 3h | MEDIUM - gear stats ignored |
| MEDIUM | Add `CanModifyStats` optimization | 2h | LOW - performance |
| LOW | Add `BaseModifierGroup` usage | 3h | LOW - code quality |
| LOW | Fix `get_total_aura_modifier_by_misc` | 1h | LOW - edge case |
| LOW | Add power type switching | 2h | LOW - druid forms |
| LOW | Add health/mana persistence on login | 2h | LOW - persistence |

---

## Recommended Next Steps

1. **Start with Phase 1 (Critical Bugs)** - These are the most impactful and easiest to fix
2. **Focus on aura modifier system first** - It's the foundation for all stat changes
3. **Then defense skill scaling** - It affects the most visible stats (crit, dodge, parry, block)
4. **Then weapon damage** - It's the most visible gameplay gap
5. **Finally, combat integration** - Threat, durability, spell interrupts are polish

## Files to Modify

1. `src/world/game/player/auras/system.rs` - Fix aura modifier apply/remove
2. `src/world/game/player/auras/container.rs` - Fix `get_total_aura_modifier_by_misc`
3. `src/world/game/player/auras/aura.rs` - Add `applied_modifiers` tracking
4. `src/world/game/player/stats/system.rs` - Add defense skill, weapon damage, AP mods, BaseModifierGroup
5. `src/world/game/player/stats/derived.rs` - Add defense skill, shapeshift AP
6. `src/world/game/player/stats/state.rs` - Add AP mods fields
7. `src/world/game/player/power/system.rs` - Fix rage decay, add health regen, mana regen auras
8. `src/world/game/player/power/regen.rs` - Add health regen formula
9. `src/world/game/combat/system.rs` - Add death check, rage generation, threat, durability
10. `src/world/game/player/inventory/system.rs` - Add MP5 from gear
11. `src/world/game/player/talents/system.rs` - Add casting regen pct

---

## Notes

- The `Player` struct is well-designed with separate state for each system
- The `PlayerManager` pattern with `with_player`/`with_player_mut` is clean but may cause lock contention
- The `StatsState` is comprehensive but some fields are not populated (AP mods, weapon damage)
- The `UnitModifierGroup` is correct but underutilized
- The `PowerSystem` has the right structure but needs combat integration
- The `AuraSystem` is the biggest gap - modifier tracking is broken
- The `CombatSystem` is the second biggest gap - missing combat outcomes and integrations
