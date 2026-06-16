# Spell Healing Effects Implementation Plan

## File: `healing.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_HEAL` (10) - Basic implementation with direct healing
- `SPELL_EFFECT_HEAL_MAX_HEALTH` (67) - Heals to full health

### Current Limitations
- No healing power coefficient calculation
- No critical heal handling
- No healing reduction modifiers (Mortal Strike, etc.)
- No overheal tracking
- No combat system integration

## Planned Implementation

### Phase 1: Core Healing Pipeline (Priority: Critical)

#### 1.1 Healing Power Integration
```rust
// Calculate heal amount with coefficients
fn calculate_heal_amount(
    base_value: i32,
    healing_power: i32,
    coefficient: f32,
    modifiers: &[SpellMod],
) -> u32 {
    // coefficient = duration / 3.5 for direct heals (capped at 1.0)
    // coefficient = duration / 15.0 for HoT heals (no cap)
    let hp_bonus = (healing_power as f32 * coefficient) as i32;
    let total = base_value + hp_bonus;
    
    // Apply modifiers (flat then percent)
    let modified = apply_healing_modifiers(total, modifiers);
    
    modified.max(0) as u32
}
```

**Effects to Update:**
- `effect_heal` - Add healing power calculation
- `effect_heal_max_health` - Already full heal, no calc needed

#### 1.2 Critical Heal System
```rust
// Roll for critical heal
fn roll_heal_crit(
    base_crit_chance: f32,
    modifiers: &[SpellMod],
) -> bool {
    let crit_chance = base_crit_chance + get_crit_modifiers(modifiers);
    let roll = random_float(0.0, 100.0);
    roll <= crit_chance
}

// Apply crit multiplier (same as spells: 150%)
fn apply_heal_crit_multiplier(heal: u32, modifiers: &[SpellMod]) -> u32 {
    let base_multiplier = 1.5; // 150% for heals
    let bonus = get_crit_heal_bonus(modifiers);
    (heal as f32 * (base_multiplier + bonus)) as u32
}
```

**Effects to Update:**
- `effect_heal` - Add crit roll and multiplier

#### 1.3 Healing Reduction
```rust
// Apply healing reduction debuffs (Mortal Strike, Wound Poison)
fn apply_healing_reduction(
    heal_amount: u32,
    target_guid: ObjectGuid,
    world: &World,
) -> u32 {
    // Check for healing reduction auras on target
    // Common: Mortal Strike (-50%), Wound Poison (-10-50%)
    let reduction_pct = world.systems.auras.get_healing_reduction(target_guid);
    (heal_amount as f32 * (1.0 - reduction_pct)) as u32
}
```

**Effects to Update:**
- `effect_heal` - Add healing reduction check
- `effect_heal_max_health` - May also be affected by reduction

### Phase 2: Combat System Integration (Priority: Critical)

#### 2.1 Replace Direct Health Modification
```rust
// Current (placeholder):
player.stats.health = new_health;

// Target:
world.systems.combat.apply_heal(
    caster_guid,
    target_guid,
    heal_amount,
    spell_id,
    is_crit,
    overheal,
    world,
).await?;
```

**Effects to Update:**
- `effect_heal` - Use CombatSystem
- `effect_heal_max_health` - Use CombatSystem

#### 2.2 Overheal Tracking
```rust
// Calculate overheal for threat/resource mechanics
fn calculate_overheal(
    heal_amount: u32,
    target_current_health: u32,
    target_max_health: u32,
) -> (u32, u32) {
    let health_deficit = target_max_health - target_current_health;
    let actual_heal = heal_amount.min(health_deficit);
    let overheal = heal_amount - actual_heal;
    (actual_heal, overheal)
}
```

**Effects to Update:**
- `effect_heal` - Track overheal
- `effect_heal_max_health` - No overheal possible

#### 2.3 Threat Generation
```rust
// Healing generates threat (usually 0.5x heal amount)
fn generate_heal_threat(
    caster_guid: ObjectGuid,
    heal_amount: u32,
    overheal: u32,
    threat_modifiers: &[SpellMod],
) {
    let effective_heal = heal_amount - overheal;
    let base_threat = (effective_heal as f32 * 0.5) as u32;
    let modified_threat = apply_threat_modifiers(base_threat, threat_modifiers);
    
    // Healing threat is distributed among all enemies targeting the healed player
    world.systems.threat.add_heal_threat(caster_guid, modified_threat);
}
```

**Effects to Update:**
- `effect_heal` - Add threat generation
- `effect_heal_max_health` - Add threat generation

### Phase 3: Additional Healing Effects (Priority: Medium)

#### 3.1 Missing Effects to Add

| Effect | ID | Description | Implementation Notes |
|--------|-----|-------------|---------------------|
| `SPELL_EFFECT_HEAL_MECHANICAL` | 75 | Heal mechanical units | Check target creature type |
| `SPELL_EFFECT_HEALTH_FUNNEL` | 65 | Warlock health funnel | Drain caster, heal pet |
| `SPELL_EFFECT_SPIRIT_HEAL` | 117 | Spirit healer resurrection | Resurrect at spirit healer |

#### 3.2 Health Funnel Implementation
```rust
/// SPELL_EFFECT_HEALTH_FUNNEL (65)
pub async fn effect_health_funnel(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Drain health from caster
    let drain_amount = input.base_value.max(0) as u32;
    
    // Apply damage to caster
    world.systems.combat.apply_spell_damage(
        input.caster_guid,
        input.caster_guid,
        drain_amount,
        input.spell_id,
        SpellSchool::Shadow,
        false,
        0,
        world,
    ).await?;
    
    // Heal pet (target should be pet)
    if let Some(target_guid) = input.target_guid {
        world.systems.combat.apply_heal(
            input.caster_guid,
            target_guid,
            drain_amount,
            input.spell_id,
            false,
            0,
            world,
        ).await?;
    }
    
    Ok(EffectResult::with_healing(drain_amount))
}
```

### Phase 4: Healing Over Time (HoT) Support (Priority: High)

#### 4.1 Periodic Healing
HoT spells (Renew, Rejuvenation) use `SPELL_EFFECT_APPLY_AURA` with a periodic heal aura type. The actual healing happens in the aura system, not here.

**Integration Point:**
- Aura system needs periodic heal implementation
- This file focuses on direct heals only

## Dependencies

### Required Systems
- [ ] CombatSystem - For proper heal application
- [ ] StatSystem - For healing power
- [ ] AuraSystem - For healing modifiers and reduction
- [ ] ThreatSystem - For threat generation

### Required DBC Data
- [ ] Spell.dbc - Base values, coefficients
- [ ] SpellCastTimes.dbc - For coefficient calculation

## Testing Checklist

- [ ] Direct heals work (Flash Heal, Healing Touch)
- [ ] Critical heals work correctly
- [ ] Healing power affects heal amount
- [ ] Healing reduction debuffs work (Mortal Strike)
- [ ] Overheal is tracked correctly
- [ ] Threat is generated (0.5x effective heal)
- [ ] Full heal works (Lay on Hands)
- [ ] Combat log shows correct values

## References

- MaNGOS: `SpellEffects.cpp` - `EffectHeal()`
- MaNGOS: `SpellAuras.cpp` - Periodic heal handling
- MaNGOS: `Unit.cpp` - `CalculateHealAmount()`
