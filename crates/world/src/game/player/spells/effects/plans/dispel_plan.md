# Dispel Effects Implementation Plan

## File: `dispel.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_DISPEL` (38) - Stub (logs only)

### Current Limitations
- No actual dispel logic
- No dispel type filtering
- No dispel resistance
- Missing dispel mechanic effect

## Planned Implementation

### Phase 1: Basic Dispel (Priority: High)

#### 1.1 Dispel Magic Implementation
```rust
/// SPELL_EFFECT_DISPEL (38)
///
/// Dispels magic effects from the target.
/// Used by Dispel Magic, Purge, etc.
pub async fn effect_dispel(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };
    
    // Get dispel parameters from spell
    let dispel_type = get_dispel_type(input.misc_value);
    let dispel_count = input.base_value.max(1) as usize;
    
    // Get target's dispellable auras
    let dispellable_auras = world.systems.auras.get_dispellable_auras(
        target_guid,
        dispel_type,
    ).await?;
    
    if dispellable_auras.is_empty() {
        // No auras to dispel
        return Ok(EffectResult::empty());
    }
    
    // Randomly select auras to dispel
    let mut rng = thread_rng();
    let selected_auras: Vec<_> = dispellable_auras
        .choose_multiple(&mut rng, dispel_count)
        .cloned()
        .collect();
    
    // Dispel each selected aura
    for aura in selected_auras {
        // Check dispel resistance
        if roll_dispel_resistance(&aura, input.caster_guid, target_guid, world) {
            // Dispel resisted
            send_dispel_resist_message(input.caster_guid, target_guid, aura.spell_id);
            continue;
        }
        
        // Remove the aura
        world.systems.auras.remove_aura(
            target_guid,
            aura.spell_id,
            aura.caster_guid,
        ).await?;
        
        // Log dispel
        tracing::debug!(
            "Dispelled aura {} from {}",
            aura.spell_id, target_guid
        );
    }
    
    Ok(EffectResult::empty())
}

// Map misc_value to dispel type
fn get_dispel_type(misc_value: i32) -> DispelType {
    match misc_value {
        0 => DispelType::Magic,
        1 => DispelType::Curse,
        2 => DispelType::Disease,
        3 => DispelType::Poison,
        4 => DispelType::Stealth,
        5 => DispelType::Invisibility,
        6 => DispelType::All,
        _ => DispelType::Magic,
    }
}
```

#### 1.2 Dispel Resistance
```rust
// Roll for dispel resistance
fn roll_dispel_resistance(
    aura: &Aura,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    world: &World,
) -> bool {
    // Get caster's dispel chance
    let dispel_chance = calculate_dispel_chance(caster_guid, target_guid, aura, world);
    
    // Roll
    let roll = random_float(0.0, 100.0);
    roll > dispel_chance
}

// Calculate dispel chance
fn calculate_dispel_chance(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    aura: &Aura,
    world: &World,
) -> f32 {
    // Base chance based on level difference
    let caster_level = world.systems.player.get_level(caster_guid);
    let target_level = world.systems.player.get_level(target_guid);
    let level_diff = target_level as i32 - caster_level as i32;
    
    let base_chance = match level_diff {
        d if d < -3 => 100.0,  // Target much lower level
        -3 => 99.0,
        -2 => 98.0,
        -1 => 97.0,
        0 => 96.0,             // Same level
        1 => 95.0,
        2 => 94.0,
        3 => 93.0,
        d => (96.0 - (d as f32 * 3.0)).max(0.0), // Higher level targets harder to dispel
    };
    
    // Apply modifiers from talents/auras
    let modifiers = world.systems.auras.get_dispel_modifiers(caster_guid);
    
    base_chance + modifiers
}
```

### Phase 2: Dispel Mechanic (Priority: Medium)

#### 2.1 Dispel Mechanic Effect
```rust
/// SPELL_EFFECT_DISPEL_MECHANIC (108)
///
/// Dispels auras by mechanic type (stun, root, etc.)
pub async fn effect_dispel_mechanic(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => input.caster_guid,
    };
    
    // misc_value contains the mechanic to dispel
    let mechanic = Mechanics::from_id(input.misc_value as u32);
    
    // Get auras with this mechanic
    let auras_to_dispel = world.systems.auras.get_auras_by_mechanic(
        target_guid,
        mechanic,
    ).await?;
    
    // Dispel all auras with this mechanic
    for aura in auras_to_dispel {
        world.systems.auras.remove_aura(
            target_guid,
            aura.spell_id,
            aura.caster_guid,
        ).await?;
    }
    
    Ok(EffectResult::empty())
}

// Mechanics enum
pub enum Mechanics {
    None = 0,
    Charm = 1,
    Disoriented = 2,
    Disarm = 3,
    Distract = 4,
    Fear = 5,
    Fumble = 6,
    Root = 7,
    Pacify = 8,
    Silence = 9,
    Sleep = 10,
    Snare = 11,
    Stun = 12,
    Freeze = 13,
    Knockout = 14,
    Bleed = 15,
    Bandage = 16,
    Polymorph = 17,
    Banish = 18,
    Shield = 19,
    Shackle = 20,
    Mount = 21,
    Persuade = 22,
    Turn = 23,
    Horror = 24,
    Invulnerability = 25,
    Interrupt = 26,
    Daze = 27,
    Discovery = 28,
    ImmuneShield = 29,
    Sapped = 30,
}
```

### Phase 3: Advanced Dispel Features (Priority: Medium)

#### 3.1 Dispel All
```rust
// Dispel all magic effects (Mass Dispel, etc.)
fn dispel_all_effects(
    target_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    world: &World,
) -> Result<u32> {
    let all_auras = world.systems.auras.get_all_dispellable_auras(target_guid);
    let mut dispelled_count = 0;
    
    for aura in all_auras {
        if !roll_dispel_resistance(&aura, caster_guid, target_guid, world) {
            world.systems.auras.remove_aura(target_guid, aura.spell_id, aura.caster_guid).await?;
            dispelled_count += 1;
        }
    }
    
    Ok(dispelled_count)
}
```

#### 3.2 Stealth/Invisibility Dispel
```rust
// Special handling for stealth/invisibility detection
fn dispel_stealth(
    target_guid: ObjectGuid,
    world: &World,
) -> Result<bool> {
    // Check for stealth auras
    let stealth_auras = world.systems.auras.get_auras_by_type(
        target_guid,
        AuraType::ModStealth,
    );
    
    // Check for invisibility auras
    let invis_auras = world.systems.auras.get_auras_by_type(
        target_guid,
        AuraType::ModInvisibility,
    );
    
    // Remove all stealth/invisibility auras
    for aura in stealth_auras.iter().chain(invis_auras.iter()) {
        world.systems.auras.remove_aura(target_guid, aura.spell_id, aura.caster_guid).await?;
    }
    
    Ok(!stealth_auras.is_empty() || !invis_auras.is_empty())
}
```

#### 3.3 Enrage Dispel
```rust
// Dispel enrage effects (Tranquilizing Shot)
fn dispel_enrage(
    target_guid: ObjectGuid,
    world: &World,
) -> Result<bool> {
    // Enrage is a dispel type in some versions
    let enrage_auras = world.systems.auras.get_auras_by_dispel_type(
        target_guid,
        DispelType::Enrage,
    );
    
    for aura in enrage_auras {
        world.systems.auras.remove_aura(target_guid, aura.spell_id, aura.caster_guid).await?;
    }
    
    Ok(!enrage_auras.is_empty())
}
```

### Phase 4: Dispel Feedback (Priority: Low)

#### 4.1 Combat Log Messages
```rust
// Send dispel message to combat log
fn send_dispel_message(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
    dispelled_spell_id: u32,
) {
    // SMSG_SPELLDISPELLOG or similar
    let packet = SpellDispelLog {
        caster_guid,
        target_guid,
        spell_id,
        dispelled_spell_id,
    };
    
    broadcast_to_nearby(packet);
}

// Send dispel resist message
fn send_dispel_resist_message(
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    spell_id: u32,
) {
    // "Your dispel failed to remove X"
    let packet = SpellDispelResist {
        caster_guid,
        target_guid,
        spell_id,
    };
    
    send_to(caster_guid, packet);
}
```

## Dependencies

### Required Systems
- [ ] AuraSystem - For aura removal and queries
- [ ] CombatSystem - For combat log messages
- [ ] PlayerSystem - For level/stats

### Required DBC Data
- [ ] Spell.dbc - Dispel type, mechanic

## Testing Checklist

- [ ] Dispel removes magic buffs/debuffs
- [ ] Dispel respects dispel type (Magic/Curse/Disease/Poison)
- [ ] Dispel count limits work
- [ ] Dispel resistance works
- [ ] Level difference affects dispel chance
- [ ] Dispel mechanic removes by mechanic type
- [ ] Stealth/invisibility can be dispelled
- [ ] Combat log shows dispels
- [ ] Dispel resist messages appear

## References

- MaNGOS: `SpellEffects.cpp` - `EffectDispel()`
- MaNGOS: `SpellAuras.cpp` - Aura dispel mechanics
- MaNGOS: `Unit.cpp` - Dispel resistance calculation
