# Movement Effects Documentation

## File: `movement.rs`

## Overview

Handles movement-based spell effects including charge, knockback, leap, and pull effects.

## Effects (4 total)

### 29 - SPELL_EFFECT_LEAP
**Function**: `effect_leap()`

Leaps to the target location.

**Parameters**:
- Target location (ground-targeted)

**Usage**:
- Heroic Leap (Warrior)
- Disengage (Hunter - backward leap)

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5232-5245`):
```cpp
void Spell::EffectLeapForward(SpellEffectIndex effIdx)
{
    if (!unitTarget)
        return;

    if (unitTarget->IsTaxiFlying())
        return;

    float x, y, z;
    m_targets.getDestination(x, y, z);

    unitTarget->NearTeleportTo(x, y, z, unitTarget->GetOrientation(), TELE_TO_NOT_LEAVE_TRANSPORT | TELE_TO_NOT_LEAVE_COMBAT | TELE_TO_NOT_UNSUMMON_PET | (unitTarget == m_caster ? TELE_TO_SPELL : 0));
}
```

**Key Behaviors**:
- Only works if target is not taxi flying
- Gets destination coordinates from spell targets
- Uses `NearTeleportTo()` for instant movement
- Teleport flags:
  - `TELE_TO_NOT_LEAVE_TRANSPORT`: Stay on transport
  - `TELE_TO_NOT_LEAVE_COMBAT`: Stay in combat
  - `TELE_TO_NOT_UNSUMMON_PET`: Keep pet summoned
  - `TELE_TO_SPELL`: Mark as spell teleport
- Instant movement (no arc/trajectory in vanilla)
- Client may show leap animation
- Must have line of sight to destination
- Cannot leap through walls or obstacles
- Maximum distance enforced by spell range

---

### 70 - SPELL_EFFECT_PULL
**Function**: `effect_pull()`

Pulls the target toward the caster.

**Parameters**:
- Target: Unit to pull
- `base_value`: Pull distance

**Usage**:
- Boss mechanics
- Special abilities

**Implementation Details** (from MaNGOS `SpellEffects.cpp:2584-2588`):
```cpp
void Spell::EffectPull(SpellEffectIndex /*effIdx*/)
{
    // TODO: create a proper pull towards distract spell center for distract
    sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "WORLD: Spell Effect DUMMY");
}
```

**Key Behaviors**:
- **NOT IMPLEMENTED** in MaNGOS (placeholder only)
- Logs debug message
- Intended for pulling targets toward caster
- Would be used for:
  - Boss mechanics (positioning)
  - Special crowd control
  - Gap closing abilities
- Real implementation would:
  - Calculate position between caster and target
  - Move target toward caster by specified distance
  - Preserve target's facing
  - Check for obstacles
- Currently unused in vanilla WoW

---

### 96 - SPELL_EFFECT_CHARGE
**Function**: `effect_charge()`

Charges to the target (Warrior Charge).

**Parameters**:
- Target: Enemy to charge to

**Usage**:
- Warrior Charge
- Intercept

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5332-5338`):
```cpp
void Spell::EffectCharge(SpellEffectIndex /*effIdx*/)
{
    if (!unitTarget)
        return;

    // see Spell::OnSpellLaunch
}
```

**Key Behaviors**:
- Actual charge implementation is in `Spell::OnSpellLaunch()`
- Effect placeholder only
- Charge mechanics:
  - Moves caster to target location
  - Stops at melee range (5 yards)
  - Pathfinding to avoid obstacles
  - Generates rage for Warriors (15 rage)
  - Roots/stuns target on arrival (separate effect)
- Cannot charge if:
  - Target is out of range
  - Path is blocked
  - Target is immune
- Used by:
  - Warrior Charge
  - Intercept (with stun)
- Animation plays on client
- Position updated server-side

---

### 98 - SPELL_EFFECT_KNOCK_BACK
**Function**: `effect_knock_back()`

Knocks the target back from the caster.

**Parameters**:
- Target: Unit to knock back
- `base_value`: Vertical speed (height)
- `misc_value`: Horizontal speed

**Usage**:
- Thunderfury proc
- Boss knockback abilities
- Explosive effects

**Implementation Details** (from MaNGOS `SpellEffects.cpp:5414-5424`):
```cpp
void Spell::EffectKnockBack(SpellEffectIndex effIdx)
{
    if (!unitTarget || unitTarget->IsTaxiFlying())
        return;

    // remove Dream Fog Sleep aura to let target be launched
    // ugly and barely working solution until proper pending states handling implemented
    unitTarget->RemoveAurasDueToSpell(24778);

    unitTarget->KnockBackFrom(m_caster, float(m_spellInfo->EffectMiscValue[effIdx]) / 10, damage / 10);
}
```

**Key Behaviors**:
- Cannot knock back units that are taxi flying
- Removes Dream Fog Sleep aura (spell 24778) before knockback
- Calls `KnockBackFrom()` with:
  - Horizontal speed: `EffectMiscValue[effIdx] / 10`
  - Vertical speed: `damage / 10`
- Interrupts spell casting
- Parabolic arc trajectory
- Collision detection with terrain
- Can knock targets off cliffs/ledges
- Direction is away from caster
- Used for crowd control and positioning
- Some bosses immune to knockback

## Dependencies

Required systems:
- `MovementSystem` - For pathfinding and movement
- `CollisionSystem` - For terrain collision

## References

- MaNGOS: `SpellEffects.cpp` - Movement effects
- MaNGOS: `Unit.cpp` - Charge and knockback implementation
