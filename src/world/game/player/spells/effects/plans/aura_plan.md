# Aura Effects Implementation Plan

## File: `aura.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_APPLY_AURA` (6) - Basic implementation, delegates to AuraSystem
- `SPELL_EFFECT_APPLY_AREA_AURA_PARTY` (35) - Not implemented
- `SPELL_EFFECT_PERSISTENT_AREA_AURA` (27) - Not implemented

### Current Limitations
- Aura type hardcoded/stubbed (needs DBC integration)
- Duration hardcoded to 30 seconds
- No periodic effect handling
- No stack/charge management
- No area aura propagation

## Planned Implementation

### Phase 1: DBC Integration (Priority: Critical)

#### 1.1 Aura Type from Spell DBC
```rust
// Read aura type from spell entry
fn get_aura_type(
    spell_entry: &SpellEntry,
    effect_index: u8,
) -> Option<AuraType> {
    let aura_type_id = spell_entry.effect_apply_aura_name[effect_index as usize];
    AuraType::from_id(aura_type_id)
}

// AuraType enum (100+ types)
pub enum AuraType {
    None = 0,
    BindSight = 1,
    ModPossess = 2,
    PeriodicDamage = 3,
    Dummy = 4,
    // ... 100+ more types
    PeriodicHeal = 8,
    ModAttackSpeed = 20,
    ModDamageDone = 79,
    ModResistance = 22,
    // etc.
}
```

**Effects to Update:**
- `effect_apply_aura` - Read actual aura type from DBC

#### 1.2 Duration from DBC
```rust
// Get duration from Duration.dbc
fn get_aura_duration(spell_entry: &SpellEntry) -> Option<u32> {
    let duration_index = spell_entry.duration_index;
    if duration_index == 0 {
        None // Permanent aura
    } else {
        let duration_entry = world.dbc.get_duration(duration_index)?;
        Some(duration_entry.duration as u32)
    }
}

// Duration can be modified by talents/auras
fn apply_duration_modifiers(
    base_duration: u32,
    modifiers: &[SpellMod],
) -> u32 {
    // Apply SPELLMOD_DURATION modifiers
}
```

**Effects to Update:**
- `effect_apply_aura` - Use actual duration from DBC

#### 1.3 Periodic Interval
```rust
// Get periodic interval (tick time)
fn get_periodic_interval(
    spell_entry: &SpellEntry,
    effect_index: u8,
) -> u32 {
    // amplitude field in DBC (in milliseconds)
    spell_entry.effect_amplitude[effect_index as usize]
}
```

**Effects to Update:**
- `effect_apply_aura` - Use actual periodic interval

### Phase 2: Aura System Integration (Priority: Critical)

#### 2.1 Aura Application
```rust
// Current implementation is already calling AuraSystem
// Need to ensure AuraSystem handles:

pub async fn effect_apply_aura(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    
    // Get spell entry from DBC
    let spell_entry = world.dbc.get_spell(input.spell_id)?;
    
    // Read aura parameters from DBC
    let aura_type = get_aura_type(&spell_entry, input.effect_index)
        .ok_or_else(|| anyhow!("Invalid aura type"))?;
    let duration_ms = get_aura_duration(&spell_entry);
    let periodic_interval_ms = get_periodic_interval(&spell_entry, input.effect_index);
    let max_stacks = spell_entry.stack_amount as u8;
    
    // Determine aura flags
    let flags = AuraFlags {
        is_positive: is_positive_aura(&spell_entry, aura_type),
        is_negative: !is_positive_aura(&spell_entry, aura_type),
        is_passive: spell_entry.has_attribute(SPELL_ATTR_PASSIVE),
        can_be_cancelled: !spell_entry.has_attribute(SPELL_ATTR_NO_AURA_CANCEL),
        is_hidden: spell_entry.has_attribute(SPELL_ATTR_DO_NOT_DISPLAY),
        is_permanent: duration_ms.is_none(),
    };
    
    // Apply via AuraSystem
    world.systems.auras.apply_aura(
        target_guid,
        input.caster_guid,
        input.spell_id,
        input.effect_index,
        aura_type as u32,
        input.misc_value,
        input.base_value,
        duration_ms,
        periodic_interval_ms,
        max_stacks,
        0, // max_charges (from proc_charges)
        flags,
        world,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

### Phase 3: Area Auras (Priority: High)

#### 3.1 Party Area Aura
```rust
/// SPELL_EFFECT_APPLY_AREA_AURA_PARTY (35)
///
/// Applies aura to all party members within range.
/// Used by paladin auras, totems, etc.
pub async fn effect_apply_area_aura_party(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let caster_guid = input.caster_guid;
    
    // Get caster's party
    let party_members = world.systems.group.get_party_members(caster_guid);
    
    // Get aura radius from spell entry
    let radius = get_spell_radius(&spell_entry, input.effect_index);
    
    // Get caster position
    let caster_pos = world.systems.player.get_position(caster_guid)?;
    
    // Apply aura to all party members in range
    for member_guid in party_members {
        if member_guid == caster_guid {
            continue;
        }
        
        let member_pos = world.systems.player.get_position(member_guid)?;
        let distance = caster_pos.distance_to(&member_pos);
        
        if distance <= radius {
            // Apply aura to this member
            apply_aura_to_target(input, member_guid, world).await?;
        }
    }
    
    Ok(EffectResult::empty())
}
```

#### 3.2 Persistent Area Aura (Ground Effects)
```rust
/// SPELL_EFFECT_PERSISTENT_AREA_AURA (27)
///
/// Creates a persistent area effect (Consecration, Blizzard, etc.)
pub async fn effect_persistent_area_aura(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    // Get target location
    let target_location = get_spell_target_location(input, world)?;
    
    // Get aura parameters
    let radius = get_spell_radius(&spell_entry, input.effect_index);
    let duration_ms = get_aura_duration(&spell_entry);
    
    // Create dynamic object at location
    let dyn_object_guid = world.systems.dyn_object.create(
        input.spell_id,
        target_location,
        radius,
        duration_ms,
        caster_guid,
    ).await?;
    
    // The dynamic object will periodically apply the aura to targets in range
    // This is handled by the DynObject system
    
    Ok(EffectResult::empty())
}
```

#### 3.3 Other Area Aura Types

| Effect | ID | Description | Implementation |
|--------|-----|-------------|----------------|
| `APPLY_AREA_AURA_PET` | 119 | Aura affects pet | Check caster's pet |
| `APPLY_AREA_AURA_FRIEND` | 128 | Aura affects friendly targets | Filter by faction |
| `APPLY_AREA_AURA_ENEMY` | 129 | Aura affects enemy targets | Filter by hostile |
| `APPLY_AREA_AURA_RAID` | 132 | Aura affects raid | Use raid group |
| `APPLY_AREA_AURA_OWNER` | 133 | Aura affects owner | For pets/minions |

### Phase 4: Aura Modifiers (Priority: Medium)

#### 4.1 Stack Management
```rust
// Handle aura stacking rules
fn handle_aura_stacking(
    new_aura: &Aura,
    existing_auras: &mut Vec<Aura>,
) -> StackResult {
    // Check if aura can stack
    if !can_aura_stack(new_aura.aura_type) {
        // Replace existing aura if new one is more powerful
        return replace_if_stronger(new_aura, existing_auras);
    }
    
    // Check stack limit
    let same_aura_count = existing_auras.iter()
        .filter(|a| a.spell_id == new_aura.spell_id)
        .count();
    
    if same_aura_count >= new_aura.max_stacks as usize {
        // Refresh duration of existing stack
        return refresh_oldest_stack(existing_auras);
    }
    
    StackResult::AddNew
}
```

#### 4.2 Charge Management
```rust
// Handle aura charges (for procs)
fn consume_aura_charge(
    aura: &mut Aura,
) -> bool {
    if aura.charges > 0 {
        aura.charges -= 1;
        if aura.charges == 0 {
            return true; // Aura should be removed
        }
    }
    false
}

// Replenish charges on aura refresh
fn replenish_charges(aura: &mut Aura) {
    aura.charges = aura.max_charges;
}
```

## Dependencies

### Required Systems
- [ ] AuraSystem - Core aura management (partially exists)
- [ ] DBC System - For spell data
- [ ] GroupSystem - For party/raid auras
- [ ] DynObjectSystem - For persistent area auras
- [ ] CombatSystem - For periodic damage/heal

### Required DBC Data
- [ ] Spell.dbc - Aura types, durations, intervals
- [ ] Duration.dbc - Duration values
- [ ] SpellRadius.dbc - Area aura radii

## Testing Checklist

- [ ] Basic buffs apply correctly
- [ ] Debuffs apply correctly
- [ ] Duration is correct from DBC
- [ ] Periodic damage works (DoTs)
- [ ] Periodic healing works (HoTs)
- [ ] Passive auras work
- [ ] Aura stacking works
- [ ] Area auras affect party/raid
- [ ] Persistent area auras (ground effects)
- [ ] Charges are consumed correctly
- [ ] Auras expire correctly

## References

- MaNGOS: `SpellEffects.cpp` - `EffectApplyAura()`
- MaNGOS: `SpellAuras.cpp` - Aura application and management
- MaNGOS: `SpellAuras.h` - AuraType enum
