# Movement Effects Implementation Plan

## File: `movement.rs`

## Current Implementation Status

### Implemented Effects
- `SPELL_EFFECT_CHARGE` (78) - Stub (logs only)
- `SPELL_EFFECT_KNOCK_BACK` (98) - Stub (logs only)

### Current Limitations
- No actual movement implementation
- No pathfinding integration
- No physics/collision handling
- Missing leap and teleport effects

## Planned Implementation

### Phase 1: Charge Implementation (Priority: High)

#### 1.1 Basic Charge
```rust
/// SPELL_EFFECT_CHARGE (78)
///
/// Charge to the target (Warrior Charge, etc.)
pub async fn effect_charge(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Get target position
    let target_pos = world.systems.player.get_position(target_guid)?;
    
    // Calculate charge destination (stop at melee range)
    let melee_range = 5.0; // yards
    let caster_pos = world.systems.player.get_position(input.caster_guid)?;
    let direction = caster_pos.direction_to(&target_pos);
    let charge_pos = target_pos.offset(&direction, -melee_range);
    
    // Start charge movement
    world.systems.movement.start_charge(
        input.caster_guid,
        charge_pos,
        CHARGE_SPEED,
        Some(target_guid), // Stop at target
    ).await?;
    
    // Apply stun/root when reaching target (if spell has that effect)
    // This is handled by a separate effect or aura
    
    Ok(EffectResult::empty())
}
```

#### 1.2 Charge with Pathfinding
```rust
// For charges that need pathfinding (around obstacles)
fn calculate_charge_path(
    start: &Position,
    end: &Position,
    world: &World,
) -> Option<Vec<Position>> {
    // Use pathfinding system to find path
    // If no path found, charge in straight line (may fail if blocked)
    world.systems.pathfinding.find_path(start, end)
}
```

### Phase 2: Knockback Implementation (Priority: High)

#### 2.1 Basic Knockback
```rust
/// SPELL_EFFECT_KNOCK_BACK (98)
///
/// Knock the target back from the caster
pub async fn effect_knock_back(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    // Get positions
    let caster_pos = world.systems.player.get_position(input.caster_guid)?;
    let target_pos = world.systems.player.get_position(target_guid)?;
    
    // Calculate knockback direction (away from caster)
    let direction = caster_pos.direction_to(&target_pos);
    
    // Get knockback parameters from spell
    let distance = input.base_value.max(0) as f32; // yards
    let speed = input.misc_value.max(0) as f32;    // speed
    
    // Calculate end position
    let end_pos = target_pos.offset(&direction, distance);
    
    // Validate end position (collision check)
    let valid_end_pos = world.systems.collision.check_line_of_sight(
        target_pos,
        end_pos,
    )?;
    
    // Apply knockback movement
    world.systems.movement.start_knockback(
        target_guid,
        valid_end_pos,
        speed,
        input.caster_guid, // Source of knockback
    ).await?;
    
    // Interrupt casting
    world.systems.spells.interrupt_cast(target_guid, input.caster_guid).await?;
    
    Ok(EffectResult::empty())
}
```

#### 1.2 Knockback Physics
```rust
// Arc knockback (parabolic trajectory)
fn calculate_knockback_arc(
    start: &Position,
    end: &Position,
    height: f32,
) -> Vec<Position> {
    // Calculate parabolic arc points
    // Used for visual effect and collision detection
}

// Ground clamping
fn clamp_to_ground(pos: &mut Position, world: &World) {
    // Ensure knockback doesn't put target underground or in air
    pos.z = world.systems.terrain.get_height(pos.x, pos.y);
}
```

### Phase 3: Teleport Effects (Priority: Critical)

#### 3.1 Teleport Units
```rust
/// SPELL_EFFECT_TELEPORT_UNITS (5)
///
/// Teleport target to a location
pub async fn effect_teleport_units(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
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

// Destination types
fn get_teleport_destination(
    input: &EffectInput,
    world: &World,
) -> Result<Position> {
    // Target type determines destination:
    // - TARGET_LOCATION_CASTER_HOME_BIND -> Player's hearth location
    // - TARGET_LOCATION_DATABASE -> From spell_target_position table
    // - TARGET_LOCATION_CASTER_DEST -> Target location from cast
    // - etc.
    
    match input.target_type {
        TARGET_LOCATION_CASTER_HOME_BIND => {
            world.systems.player.get_home_bind(input.caster_guid)
        }
        TARGET_LOCATION_DATABASE => {
            world.db.get_spell_target_position(input.spell_id)
        }
        _ => {
            get_spell_target_location(input, world)
        }
    }
}
```

#### 3.2 Portal Teleport
```rust
/// SPELL_EFFECT_PORTAL_TELEPORT (4)
///
/// Portal teleport (city portals, etc.)
pub async fn effect_portal_teleport(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    // Similar to teleport_units but with portal visual/gameplay
    
    // Create portal game object at caster location
    let portal_guid = world.systems.game_object.create(
        PORTAL_ENTRY_ID,
        caster_pos,
        duration_ms,
    ).await?;
    
    // Portal applies teleport aura to players who enter
    // This is handled by the game object
    
    Ok(EffectResult::empty())
}
```

### Phase 4: Additional Movement Effects (Priority: Medium)

#### 4.1 Leap
```rust
/// SPELL_EFFECT_LEAP (29)
///
/// Leap to target location (Heroic Leap, etc.)
pub async fn effect_leap(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_pos = get_spell_target_location(input, world)?;
    
    // Validate leap destination
    let caster_pos = world.systems.player.get_position(input.caster_guid)?;
    let distance = caster_pos.distance_to(&target_pos);
    let max_leap_distance = get_spell_range(&spell_entry);
    
    if distance > max_leap_distance {
        return Err(anyhow!("Target too far for leap"));
    }
    
    // Check line of sight
    if !world.systems.collision.has_line_of_sight(caster_pos, target_pos) {
        return Err(anyhow!("No line of sight to leap target"));
    }
    
    // Perform leap (parabolic arc)
    world.systems.movement.start_leap(
        input.caster_guid,
        target_pos,
        LEAP_SPEED,
        LEAP_HEIGHT,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

#### 4.2 Teleport to Caster Face
```rust
/// SPELL_EFFECT_TELEPORT_UNITS_FACE_CASTER (43)
///
/// Teleport target to caster and face them
pub async fn effect_teleport_units_face_caster(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };
    
    let caster_pos = world.systems.player.get_position(input.caster_guid)?;
    
    // Teleport target to caster's front
    let melee_range = 5.0;
    let target_pos = caster_pos.offset_forward(melee_range);
    
    // Face the caster
    let orientation = target_pos.angle_to(&caster_pos);
    
    world.systems.movement.teleport(
        target_guid,
        Position::new(target_pos.x, target_pos.y, target_pos.z, orientation),
        TeleportFlags::SPELL,
    ).await?;
    
    Ok(EffectResult::empty())
}
```

#### 4.3 Other Movement Effects

| Effect | ID | Description | Implementation |
|--------|-----|-------------|----------------|
| `SPELL_EFFECT_PULL` | 70 | Pull target to caster | Reverse of knockback |
| `SPELL_EFFECT_PLAYER_PULL` | 124 | Pull player | Special handling for players |
| `SPELL_EFFECT_STUCK` | 84 | Unstuck (teleport to graveyard) | Use graveyard system |
| `SPELL_EFFECT_TELEPORT_GRAVEYARD` | 120 | Teleport to graveyard | Graveyard system |

## Dependencies

### Required Systems
- [ ] MovementSystem - For charge, knockback, teleport
- [ ] PathfindingSystem - For charge path calculation
- [ ] CollisionSystem - For validation
- [ ] TerrainSystem - For ground height
- [ ] SpellSystem - For cast interruption

### Required DBC Data
- [ ] Spell.dbc - Range, speed values
- [ ] SpellRadius.dbc - For knockback distances

## Testing Checklist

- [ ] Charge moves caster to target
- [ ] Charge stops at melee range
- [ ] Knockback pushes target away
- [ ] Knockback interrupts casting
- [ ] Teleport moves to correct location
- [ ] Portal creates game object
- [ ] Leap follows arc trajectory
- [ ] Stuck teleports to graveyard
- [ ] Collision prevents invalid destinations

## References

- MaNGOS: `SpellEffects.cpp` - `EffectCharge()`, `EffectKnockBack()`, `EffectTeleportUnits()`
- MaNGOS: `Unit.cpp` - Movement and teleport functions
