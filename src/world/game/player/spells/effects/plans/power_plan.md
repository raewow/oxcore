# Power Effects Implementation Plan

## File: `power.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_POWER_DRAIN` (8) - Basic implementation (drains mana, gives to caster)
- `SPELL_EFFECT_ENERGIZE` (30) - Basic implementation (restores power)
- `SPELL_EFFECT_POWER_BURN` (62) - Basic implementation (burns mana, deals damage)

### Current Limitations
- Power type hardcoded/stubbed (needs DBC integration)
- No power cost modifiers
- No power regeneration interaction
- No combat system integration for damage portion
- Missing power funnel effect

## Planned Implementation

### Phase 1: Power Type System (Priority: Critical)

#### 1.1 Power Type from DBC
```rust
// Map misc_value to PowerType
fn get_power_type_from_misc(misc_value: i32) -> PowerType {
    match misc_value {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        2 => PowerType::Focus,  // Pets
        3 => PowerType::Energy,
        4 => PowerType::Happiness, // Pets
        _ => PowerType::Mana, // Default
    }
}

// Get power type from spell entry
fn get_spell_power_type(spell_entry: &SpellEntry) -> PowerType {
    // SpellEntry.power_type field
    match spell_entry.power_type {
        0 => PowerType::Mana,
        1 => PowerType::Rage,
        3 => PowerType::Energy,
        _ => PowerType::Mana,
    }
}
```

**Effects to Update:**
- `effect_power_drain` - Use correct power type from misc_value
- `effect_energize` - Use correct power type from misc_value
- `effect_power_burn` - Already has basic power type logic

#### 1.2 Power Validation
```rust
// Check if target has the power type
fn target_has_power_type(
    target_guid: ObjectGuid,
    power_type: PowerType,
    world: &World,
) -> bool {
    world.systems.player.manager().with_player(target_guid, |player| {
        player.power.has_power_type(power_type)
    }).unwrap_or(false)
}

// Get current power amount
fn get_target_power(
    target_guid: ObjectGuid,
    power_type: PowerType,
    world: &World,
) -> u32 {
    world.systems.player.manager().with_player(target_guid, |player| {
        player.power.get_current(power_type)
    }).unwrap_or(0)
}
```

**Effects to Update:**
- All power effects - Add validation

### Phase 2: Power Cost Modifiers (Priority: Medium)

#### 2.1 Cost Reduction/Augmentation
```rust
// Apply power cost modifiers from talents/auras
fn apply_power_cost_modifiers(
    base_cost: u32,
    spell_id: u32,
    power_type: PowerType,
    caster_modifiers: &[SpellMod],
) -> u32 {
    // Flat modifiers first
    let flat_mod = get_flat_cost_modifiers(caster_modifiers, spell_id, power_type);
    let after_flat = (base_cost as i32 + flat_mod).max(0) as u32;
    
    // Percent modifiers second
    let pct_mod = get_pct_cost_modifiers(caster_modifiers, spell_id, power_type);
    (after_flat as f32 * (1.0 + pct_mod)) as u32
}

// Examples:
// - Mage: Clearcasting (reduces next spell to 0)
// - Warlock: Cataclysm (reduces Destruction spell costs)
// - Druid: Moonglow (reduces Healing Touch cost)
```

**Note:** This applies to spell casting costs, not power effects directly, but power drain/burn may be affected.

### Phase 3: Combat System Integration (Priority: High)

#### 3.1 Power Burn Damage
```rust
// Current (placeholder):
player.stats.health = new_health;

// Target:
world.systems.combat.apply_spell_damage(
    caster_guid,
    target_guid,
    damage,
    spell_id,
    SpellSchool::Shadow, // Power burn is usually Shadow school
    false,
    0,
    world,
).await?;
```

**Effects to Update:**
- `effect_power_burn` - Use CombatSystem for damage

#### 3.2 Threat Generation
```rust
// Power drain/burn generates threat
fn generate_power_threat(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    power_amount: u32,
    power_type: PowerType,
) {
    // Power manipulation generates threat (varies by power type)
    let threat_multiplier = match power_type {
        PowerType::Mana => 0.5,
        PowerType::Rage => 1.0, // Rage denial is valuable
        PowerType::Energy => 1.0,
        _ => 0.5,
    };
    
    let threat = (power_amount as f32 * threat_multiplier) as u32;
    world.systems.threat.add_threat(caster_guid, target_guid, threat);
}
```

**Effects to Update:**
- `effect_power_drain` - Add threat
- `effect_power_burn` - Add threat

### Phase 4: Additional Power Effects (Priority: Medium)

#### 4.1 Missing Effects to Add

| Effect | ID | Description | Implementation Notes |
|--------|-----|-------------|---------------------|
| `SPELL_EFFECT_POWER_FUNNEL` | 66 | Drain power from caster to target | Reverse of power_drain |

#### 4.2 Power Funnel Implementation
```rust
/// SPELL_EFFECT_POWER_FUNNEL (66)
///
/// Drains power from caster and gives to target.
/// Used by some pet abilities.
pub async fn effect_power_funnel(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    let funnel_amount = input.base_value.max(0) as u32;
    let power_type = get_power_type_from_misc(input.misc_value);
    
    // Drain from caster
    let actual_drain = world.systems.player.manager().with_player_mut(
        input.caster_guid,
        |player| {
            let idx = power_type as usize;
            let current = player.power.current[idx];
            let drain = funnel_amount.min(current);
            player.power.current[idx] = current - drain;
            drain
        }
    ).unwrap_or(0);
    
    // Give to target
    if actual_drain > 0 {
        world.systems.power.restore_power(
            target_guid,
            power_type,
            actual_drain,
            world,
        )?;
    }
    
    Ok(EffectResult::empty())
}
```

### Phase 5: Rage Generation (Priority: High)

#### 5.1 Rage from Damage Dealt
```rust
// Generate rage when dealing damage
fn generate_rage_from_damage(
    damage_dealt: u32,
    weapon_speed: f32,
    is_crit: bool,
) -> u32 {
    // Formula: damage / (rage_conversion * weapon_speed) + crit_bonus
    // Rage conversion varies by level
    let rage_conversion = get_rage_conversion(level);
    let base_rage = (damage_dealt as f32 / (rage_conversion * weapon_speed)) as u32;
    let crit_bonus = if is_crit { 1 } else { 0 };
    
    (base_rage + crit_bonus).min(100) // Cap at 100 rage
}
```

**Integration Point:**
- This belongs in CombatSystem, but power effects may trigger it

#### 5.2 Rage from Damage Taken
```rust
// Generate rage when taking damage
fn generate_rage_from_damage_taken(
    damage_taken: u32,
    level: u8,
) -> u32 {
    // Formula: damage / (rage_conversion * 2)
    let rage_conversion = get_rage_conversion(level);
    ((damage_taken as f32) / (rage_conversion * 2.0)) as u32
}
```

## Dependencies

### Required Systems
- [ ] CombatSystem - For power burn damage
- [ ] PowerSystem - Already exists, needs expansion
- [ ] StatSystem - For power costs
- [ ] ThreatSystem - For threat generation
- [ ] AuraSystem - For power cost modifiers

### Required DBC Data
- [ ] Spell.dbc - Power type, costs
- [ ] SpellCastTimes.dbc - For rage generation calculations

## Testing Checklist

- [ ] Power drain works (Mana Drain, Drain Mana)
- [ ] Power energize works (Mana potions, Innervate)
- [ ] Power burn deals damage (Mana Burn)
- [ ] Correct power type is used
- [ ] Power drain transfers to caster
- [ ] Rage generation from damage dealt
- [ ] Rage generation from damage taken
- [ ] Energy regeneration works
- [ ] Threat is generated

## References

- MaNGOS: `SpellEffects.cpp` - `EffectPowerDrain()`, `EffectEnergize()`, `EffectPowerBurn()`
- MaNGOS: `Spell.cpp` - Power cost handling
- MaNGOS: `Player.cpp` - Rage generation formulas
