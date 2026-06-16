# Miscellaneous Spell Effects Implementation Plan

## File: `misc.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_INSTAKILL` (1) - Basic implementation
- `SPELL_EFFECT_DUMMY` (3) - Stub (logs only)
- `SPELL_EFFECT_LEARN_SPELL` (36) - Basic implementation
- `SPELL_EFFECT_TRIGGER_SPELL` (64) - Stub (logs only)
- `SPELL_EFFECT_INTERRUPT_CAST` (68) - Partial implementation (lockout only)

### Current Limitations
- Dummy effects need script system
- Trigger spell needs SpellSystem integration
- Interrupt needs full cast cancellation
- Many critical effects missing

## Planned Implementation

### Phase 1: Critical Missing Effects (Priority: Critical)

#### 1.1 Teleport Units
```rust
/// SPELL_EFFECT_TELEPORT_UNITS (5)
///
/// Teleport target to a location (Hearthstone, portals, etc.)
pub async fn effect_teleport_units(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    
    // Get destination from spell target
    let destination = match input.target_type {
        TARGET_LOCATION_CASTER_HOME_BIND => {
            // Hearthstone - use player's home bind
            world.systems.player.get_home_bind(target_guid)?
        }
        TARGET_LOCATION_DATABASE => {
            // From spell_target_position table
            world.db.get_spell_target_position(input.spell_id)?
        }
        _ => {
            // Use target location from cast
            get_spell_target_location(input, world)?
        }
    };
    
    // Perform teleport
    world.systems.movement.teleport(target_guid, destination, TeleportFlags::SPELL).await?;
    
    Ok(EffectResult::empty())
}
```

#### 1.2 Summon
```rust
/// SPELL_EFFECT_SUMMON (28)
///
/// Summon a creature (Warlock pets, elementals, etc.)
pub async fn effect_summon(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains creature entry ID
    let creature_entry = input.misc_value as u32;
    
    // Get summon location
    let summon_pos = get_summon_location(input, world)?;
    
    // Create creature
    let creature_guid = world.systems.creature.spawn(
        creature_entry,
        summon_pos,
        SummonType::Temporary,
        get_summon_duration(&spell_entry),
    ).await?;
    
    // Set owner/pet relationship
    world.systems.creature.set_owner(creature_guid, input.caster_guid)?;
    
    Ok(EffectResult::empty())
}
```

#### 1.3 Create Item
```rust
/// SPELL_EFFECT_CREATE_ITEM (24)
///
/// Create items (Conjure Food, Conjure Water, etc.)
pub async fn effect_create_item(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains item entry ID
    let item_entry = input.misc_value as u32;
    
    // base_value contains item count
    let item_count = input.base_value.max(1) as u32;
    
    // Create item in caster's inventory
    world.systems.inventory.create_item(
        input.caster_guid,
        item_entry,
        item_count,
    ).await?;
    
    // Send item creation message
    send_item_created_message(input.caster_guid, item_entry, item_count);
    
    Ok(EffectResult::empty())
}
```

#### 1.4 Resurrect
```rust
/// SPELL_EFFECT_RESURRECT (18)
///
/// Resurrect a dead player
pub async fn effect_resurrect(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Check if target is dead
    if !world.systems.player.is_dead(target_guid) {
        return Err(anyhow!("Target is not dead"));
    }
    
    // Calculate health/mana restore
    let health_pct = get_resurrect_health_percent(&spell_entry);
    let mana_pct = get_resurrect_mana_percent(&spell_entry);
    
    // Resurrect target
    world.systems.player.resurrect(
        target_guid,
        health_pct,
        mana_pct,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

#### 1.5 Open Lock
```rust
/// SPELL_EFFECT_OPEN_LOCK (33)
///
/// Open locked doors, chests, etc. (Pick Lock, keys)
pub async fn effect_open_lock(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Err(anyhow!("No target for open lock")),
    };
    
    // Get target game object
    let go = world.systems.game_object.get(target_guid)?;
    
    // Check if object is locked
    if !go.is_locked() {
        return Err(anyhow!("Target is not locked"));
    }
    
    // Check lock type vs spell lock type
    let lock_type = go.get_lock_type();
    let spell_lock_type = get_spell_lock_type(&spell_entry);
    
    if lock_type != spell_lock_type {
        return Err(anyhow!("Wrong key/lockpick for this lock"));
    }
    
    // Check skill requirement for lockpicking
    if lock_type == LOCK_TYPE_PICKLOCK {
        let required_skill = go.get_required_lockpicking_skill();
        let caster_skill = world.systems.player.get_skill_level(
            input.caster_guid,
            SKILL_LOCKPICKING,
        );
        
        if caster_skill < required_skill {
            return Err(anyhow!("Lockpicking skill too low"));
        }
        
        // Chance to fail based on skill difference
        if roll_lockpick_failure(caster_skill, required_skill) {
            return Err(anyhow!("Failed to pick lock"));
        }
    }
    
    // Unlock the object
    world.systems.game_object.unlock(target_guid)?;
    
    Ok(EffectResult::empty())
}
```

#### 1.6 Dual Wield
```rust
/// SPELL_EFFECT_DUAL_WIELD (40)
///
/// Enable dual wielding for the player
pub async fn effect_dual_wield(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Enable dual wield skill
    world.systems.player.enable_dual_wield(input.caster_guid)?;
    
    // Learn dual wield spell (674)
    world.systems.spells.learn_spell(input.caster_guid, 674)?;
    
    Ok(EffectResult::empty())
}
```

### Phase 2: Script Effects (Priority: High)

#### 2.1 Dummy Effect Scripts
```rust
/// SPELL_EFFECT_DUMMY (3)
///
/// Dummy effects require custom script handling
pub async fn effect_dummy(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Route to script system based on spell_id
    world.systems.spell_scripts.execute_dummy(
        input.spell_id,
        input.effect_index,
        input.caster_guid,
        input.target_guid,
        input.misc_value,
        input.base_value,
        world,
    ).await?;
    
    Ok(EffectResult::empty())
}

// Example script implementations needed:
// - Warrior: Execute (damage based on rage)
// - Rogue: Combo point finishers
// - Druid: Form changes
// - Hunter: Pet commands
// - Warlock: Soul Shard mechanics
```

#### 2.2 Script Effect
```rust
/// SPELL_EFFECT_SCRIPT_EFFECT (77)
///
/// General script effect handler
pub async fn effect_script_effect(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Similar to dummy, but different routing
    world.systems.spell_scripts.execute_script(
        input.spell_id,
        input,
        world,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

#### 2.3 Send Event
```rust
/// SPELL_EFFECT_SEND_EVENT (61)
///
/// Triggers a game event
pub async fn effect_send_event(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains event ID
    let event_id = input.misc_value as u32;
    
    // Trigger event
    world.systems.events.trigger(
        event_id,
        input.caster_guid,
        input.target_guid,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

### Phase 3: Trigger & Chain Effects (Priority: High)

#### 3.1 Trigger Spell
```rust
/// SPELL_EFFECT_TRIGGER_SPELL (64)
///
/// Triggers another spell
pub async fn effect_trigger_spell(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains spell ID to trigger
    let triggered_spell_id = input.misc_value as u32;
    
    if triggered_spell_id == 0 {
        return Ok(EffectResult::empty());
    }
    
    // Trigger the spell
    world.systems.spells.cast_spell(
        input.caster_guid,
        triggered_spell_id,
        input.target_guid,
        true, // is_triggered
        world,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

#### 3.2 Trigger Missile
```rust
/// SPELL_EFFECT_TRIGGER_MISSILE (32)
///
/// Triggers a missile (projectile) spell
pub async fn effect_trigger_missile(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains destination spell ID
    let missile_spell_id = input.misc_value as u32;
    
    // Get target location
    let target_pos = get_spell_target_location(input, world)?;
    
    // Create missile
    world.systems.missile.create(
        input.caster_guid,
        missile_spell_id,
        input.caster_pos,
        target_pos,
        MISSILE_SPEED,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

### Phase 4: Interrupt & Control (Priority: High)

#### 4.1 Complete Interrupt Implementation
```rust
/// SPELL_EFFECT_INTERRUPT_CAST (68)
///
/// Interrupt target's spell cast
pub async fn effect_interrupt_cast(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Get interrupt parameters
    let lockout_school = input.misc_value as u8;
    let lockout_duration_ms = input.base_value.max(0) as u32;
    
    // Interrupt any active cast
    if let Some(active_cast) = world.systems.spells.get_active_cast(target_guid) {
        // Cancel the cast
        world.systems.spells.cancel_cast(target_guid, input.caster_guid).await?;
        
        // Get the interrupted spell's school
        let interrupted_school = get_spell_school(active_cast.spell_id);
        
        // Apply school lockout
        if lockout_duration_ms > 0 {
            world.systems.spells.apply_school_lockout(
                target_guid,
                interrupted_school,
                lockout_duration_ms,
            ).await?;
        }
        
        // Send interrupt packet
        send_cast_interrupted_message(target_guid, active_cast.spell_id);
    }
    
    // Also apply specified school lockout if different
    if lockout_school != 0 && lockout_duration_ms > 0 {
        world.systems.spells.apply_school_lockout(
            target_guid,
            lockout_school,
            lockout_duration_ms,
        ).await?;
    }
    
    Ok(EffectResult::empty())
}
```

#### 4.2 Distract
```rust
/// SPELL_EFFECT_DISTRACT (69)
///
/// Distract target ( Rogue Distract)
pub async fn effect_distract(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Get distract location
    let distract_pos = get_spell_target_location(input, world)?;
    
    // Find all NPCs in radius
    let radius = get_spell_radius(&spell_entry, input.effect_index);
    let npcs = world.systems.creature.get_in_radius(distract_pos, radius);
    
    for npc in npcs {
        // Make NPC face distract location
        world.systems.creature.face_position(npc.guid, distract_pos)?;
        
        // Interrupt current action
        world.systems.creature.interrupt_current_action(npc.guid)?;
        
        // Set distracted state
        world.systems.creature.set_distracted(
            npc.guid,
            distract_pos,
            DISTRACT_DURATION_MS,
        )?;
    }
    
    Ok(EffectResult::empty())
}
```

### Phase 5: Class-Specific Effects (Priority: Medium)

#### 5.1 Weapon Abilities
```rust
/// SPELL_EFFECT_WEAPON_PERCENT_DAMAGE (31)
pub async fn effect_weapon_percent_damage(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // base_value contains percent (100 = 100% weapon damage)
    let percent = input.base_value as f32 / 100.0;
    
    // Calculate weapon damage
    let weapon_damage = calculate_weapon_damage(input.caster_guid, world)?;
    
    // Apply percent
    let total_damage = (weapon_damage as f32 * percent) as u32;
    
    // Apply damage
    if let Some(target_guid) = input.target_guid {
        apply_damage(input.caster_guid, target_guid, total_damage, input.spell_id, world).await?;
    }
    
    Ok(EffectResult::with_damage(total_damage))
}

/// SPELL_EFFECT_ADD_EXTRA_ATTACKS (19)
pub async fn effect_add_extra_attacks(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // base_value contains number of extra attacks
    let extra_attacks = input.base_value.max(1) as u32;
    
    // Add extra attacks to caster
    world.systems.combat.add_extra_attacks(input.caster_guid, extra_attacks)?;
    
    Ok(EffectResult::empty())
}
```

#### 5.2 Combo Points
```rust
/// SPELL_EFFECT_ADD_COMBO_POINTS (80)
pub async fn effect_add_combo_points(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // base_value contains combo points to add
    let combo_points = input.base_value.max(0) as u8;
    
    if let Some(target_guid) = input.target_guid {
        world.systems.player.add_combo_points(
            input.caster_guid,
            target_guid,
            combo_points,
        )?;
    }
    
    Ok(EffectResult::empty())
}
```

#### 5.3 Defense Abilities
```rust
/// SPELL_EFFECT_DODGE (20)
pub async fn effect_dodge(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Apply dodge buff aura
    // Actual dodge chance handled by aura system
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PARRY (22)
pub async fn effect_parry(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Apply parry buff aura
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_BLOCK (23)
pub async fn effect_block(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // Apply block buff aura
    Ok(EffectResult::empty())
}
```

### Phase 6: Utility Effects (Priority: Low)

#### 6.1 Quest & Reputation
```rust
/// SPELL_EFFECT_QUEST_COMPLETE (16)
pub async fn effect_quest_complete(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains quest ID
    let quest_id = input.misc_value as u32;
    
    world.systems.quest.complete_quest(input.caster_guid, quest_id)?;
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_REPUTATION (103)
pub async fn effect_reputation(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains faction ID
    // base_value contains reputation amount
    let faction_id = input.misc_value as u32;
    let reputation = input.base_value;
    
    world.systems.reputation.modify_reputation(
        input.caster_guid,
        faction_id,
        reputation,
    )?;
    
    Ok(EffectResult::empty())
}
```

#### 6.2 Skill & Proficiency
```rust
/// SPELL_EFFECT_SKILL_STEP (44)
pub async fn effect_skill_step(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains skill ID
    // base_value contains skill increase
    let skill_id = input.misc_value as u16;
    let skill_increase = input.base_value as u16;
    
    world.systems.skills.increase_skill(
        input.caster_guid,
        skill_id,
        skill_increase,
    )?;
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_PROFICIENCY (60)
pub async fn effect_proficiency(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // misc_value contains proficiency type (weapon/armor)
    let proficiency = input.misc_value as u32;
    
    world.systems.player.learn_proficiency(input.caster_guid, proficiency)?;
    
    Ok(EffectResult::empty())
}
```

## Dependencies

### Required Systems
- [ ] SpellScriptSystem - For dummy/script effects
- [ ] CreatureSystem - For summons
- [ ] ItemSystem - For create_item
- [ ] QuestSystem - For quest_complete
- [ ] ReputationSystem - For reputation
- [ ] SkillSystem - For skill_step
- [ ] GameObjectSystem - For open_lock
- [ ] MovementSystem - For teleport

### Required DBC Data
- [ ] Spell.dbc - All effect parameters
- [ ] Item.dbc - For created items
- [ ] Creature.dbc - For summons

## Testing Checklist

- [ ] Instakill works
- [ ] Learn spell adds to spellbook
- [ ] Trigger spell casts correctly
- [ ] Interrupt cancels casting
- [ ] Teleport moves player
- [ ] Summon creates creature
- [ ] Create item adds to inventory
- [ ] Resurrect revives dead players
- [ ] Open lock unlocks objects
- [ ] Dual wield enables off-hand
- [ ] Dummy effects route to scripts
- [ ] Quest complete finishes quests

## References

- MaNGOS: `SpellEffects.cpp` - All Effect* functions
- MaNGOS: `Spell.cpp` - Trigger spell handling
