# Spell Damage Effects Implementation Plan

## File: `damage.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_SCHOOL_DAMAGE` (2) - Basic implementation with direct damage
- `SPELL_EFFECT_HEALTH_LEECH` (9) - Basic implementation (damage + heal)
- `SPELL_EFFECT_WEAPON_DAMAGE` (58) - Stub (applies bonus damage only)
- `SPELL_EFFECT_WEAPON_DAMAGE_NOSCHOOL` (17) - Delegates to weapon_damage
- `SPELL_EFFECT_NORMALIZED_WEAPON_DMG` (121) - Delegates to weapon_damage

### Current Limitations
- No spell power coefficient calculation
- No critical hit handling
- No resistance/absorb mechanics
- No combat system integration
- Weapon damage uses placeholder values

## Planned Implementation

### Phase 1: Core Damage Pipeline (Priority: Critical)

#### 1.1 Spell Power Integration
```rust
// Calculate spell damage with coefficients
fn calculate_spell_damage(
    base_value: i32,
    spell_power: i32,
    coefficient: f32,
    modifiers: &[SpellMod],
) -> u32 {
    // coefficient = duration / 3.5 for direct damage (capped at 1.0)
    // coefficient = duration / 15.0 for DoT damage (no cap)
    let sp_bonus = (spell_power as f32 * coefficient) as i32;
    let total = base_value + sp_bonus;
    
    // Apply modifiers (flat then percent)
    let modified = apply_damage_modifiers(total, modifiers);
    
    modified.max(0) as u32
}
```

**Effects to Update:**
- `effect_school_damage` - Add spell power calculation
- `effect_health_leech` - Add spell power calculation

#### 1.2 Critical Hit System
```rust
// Roll for critical hit
fn roll_spell_crit(
    base_crit_chance: f32,
    modifiers: &[SpellMod],
    target_level: u8,
) -> bool {
    let crit_chance = base_crit_chance + get_crit_modifiers(modifiers);
    let roll = random_float(0.0, 100.0);
    roll <= crit_chance
}

// Apply crit multiplier
fn apply_crit_multiplier(damage: u32, modifiers: &[SpellMod]) -> u32 {
    let base_multiplier = 1.5; // 150% for spells
    let bonus = get_crit_damage_bonus(modifiers);
    (damage as f32 * (base_multiplier + bonus)) as u32
}
```

**Effects to Update:**
- `effect_school_damage` - Add crit roll and multiplier
- `effect_weapon_damage` - Add melee crit (200% multiplier)

#### 1.3 Resistance & Absorb
```rust
// Calculate partial resist
fn calculate_partial_resist(
    damage: u32,
    school: SpellSchool,
    target_resistance: i32,
    caster_level: u8,
    target_level: u8,
) -> (u32, ResistType) {
    // Based on resistance vs level difference
    // Returns (damage_after_resist, resist_type)
}

// Apply absorb shields
fn apply_absorb(
    damage: u32,
    target_guid: ObjectGuid,
    world: &World,
) -> (u32, u32) {
    // Returns (damage_after_absorb, absorbed_amount)
}
```

**Effects to Update:**
- `effect_school_damage` - Add resist and absorb
- `effect_weapon_damage` - Add armor reduction

### Phase 2: Weapon Damage System (Priority: High)

#### 2.1 Weapon Stats Integration
```rust
// Get weapon damage range
fn get_weapon_damage_range(
    player: &Player,
    hand: WeaponHand,
) -> (u32, u32) {
    // Read from equipped weapon item
    // Return (min_damage, max_damage)
}

// Calculate AP contribution
fn calculate_ap_contribution(
    attack_power: i32,
    weapon_speed: f32,
    is_normalized: bool,
) -> f32 {
    if is_normalized {
        // Normalized speeds: Dagger=1.7, 1H=2.4, 2H=3.3, Ranged=2.8
        let normalized_speed = get_normalized_speed(weapon_type);
        attack_power as f32 / 14.0 * normalized_speed
    } else {
        attack_power as f32 / 14.0 * weapon_speed
    }
}
```

**Effects to Update:**
- `effect_weapon_damage` - Full weapon damage calculation
- `effect_normalized_weapon_dmg` - Use normalized AP calculation

#### 2.2 Armor Reduction
```rust
// Calculate damage reduction from armor
fn calculate_armor_reduction(
    damage: u32,
    attacker_level: u8,
    target_armor: u32,
) -> u32 {
    // Formula: reduction = armor / (armor + 400 + 85 * level)
    // Returns damage after reduction
}
```

**Effects to Update:**
- `effect_weapon_damage` - Add armor reduction
- `effect_normalized_weapon_dmg` - Add armor reduction

### Phase 3: Combat System Integration (Priority: Critical)

#### 3.1 Replace Direct Health Modification
```rust
// Current (placeholder):
player.stats.health = new_health;

// Target:
world.systems.combat.apply_spell_damage(
    caster_guid,
    target_guid,
    damage,
    spell_id,
    school,
    is_crit,
    absorbed,
    world,
).await?;
```

**Effects to Update:**
- `effect_school_damage` - Use CombatSystem
- `effect_weapon_damage` - Use CombatSystem
- `effect_health_leech` - Use CombatSystem for damage portion

#### 3.2 Threat Generation
```rust
// Generate threat from damage dealt
fn generate_threat(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    damage: u32,
    threat_modifiers: &[SpellMod],
) {
    let base_threat = damage;
    let modified_threat = apply_threat_modifiers(base_threat, threat_modifiers);
    world.systems.threat.add_threat(caster_guid, target_guid, modified_threat);
}
```

**Effects to Update:**
- All damage effects - Add threat generation

### Phase 4: Additional Damage Effects (Priority: Medium)

#### 4.1 Missing Effects to Add

| Effect | ID | Description | Implementation Notes |
|--------|-----|-------------|---------------------|
| `SPELL_EFFECT_ENVIRONMENTAL_DAMAGE` | 7 | Fire, lava, falling | Use existing damage pipeline |
| `SPELL_EFFECT_WEAPON_PERCENT_DAMAGE` | 31 | % of weapon damage | Multiply weapon damage by percent |
| `SPELL_EFFECT_FORCE_CRITICAL_HIT` | 51 | Force next crit | Set forced_crit flag |
| `SPELL_EFFECT_GUARANTEE_HIT` | 52 | Ignore miss chance | Set hit guarantee flag |
| `SPELL_EFFECT_DURABILITY_DAMAGE` | 111 | Damage item durability | Target equipped items |
| `SPELL_EFFECT_DURABILITY_DAMAGE_PCT` | 115 | % durability damage | Percentage-based |

## Dependencies

### Required Systems
- [ ] CombatSystem - For proper damage application
- [ ] ItemSystem - For weapon stats
- [ ] StatSystem - For spell power, AP, armor
- [ ] AuraSystem - For damage modifiers
- [ ] ThreatSystem - For threat generation

### Required DBC Data
- [ ] Spell.dbc - Base values, coefficients
- [ ] SpellCastTimes.dbc - For coefficient calculation
- [ ] Item.dbc - Weapon damage ranges

## Testing Checklist

- [ ] Direct damage spells (Fireball, Shadow Bolt)
- [ ] Weapon abilities (Sinister Strike, Mortal Strike)
- [ ] Critical hits work correctly
- [ ] Spell power affects damage
- [ ] Resistance reduces damage
- [ ] Armor reduces physical damage
- [ ] Health leech heals caster
- [ ] Threat is generated
- [ ] Combat log shows correct values

## References

- MaNGOS: `SpellEffects.cpp` - `EffectSchoolDMG()`, `EffectWeaponDamage()`
- MaNGOS: `Spell.cpp` - Damage calculation pipeline
- MaNGOS: `Unit.cpp` - `CalculateSpellDamage()`, `CalcArmorReducedDamage()`
