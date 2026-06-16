# Spell Effects Master Documentation

## Overview

This document provides a complete mapping of all MaNGOS spell effects (excluding unused ones) to their appropriate implementation files in world_v2.

**Total Effects Documented**: ~100 (excluding 30+ unused effects marked in MaNGOS as "unused from pre-1.2.1" or test-only)

## File Organization

| File | Category | Effect Count | Description |
|------|----------|--------------|-------------|
| `damage.rs` | Combat - Damage | 8 | Direct damage, weapon damage, environmental |
| `healing.rs` | Combat - Healing | 4 | Direct heals, full heals, mechanical heals |
| `power.rs` | Combat - Power | 3 | Power drain, energize, power burn |
| `aura.rs` | Buffs/Debuffs | 7 | Aura application and area auras |
| `summon.rs` | Summoning | 11 | Pets, guardians, totems, demons |
| `item.rs` | Items/Enchanting | 10 | Item creation, enchanting, disenchanting |
| `teleport.rs` | Teleportation | 7 | Teleports, binds, taxi paths |
| `combat.rs` | Combat Mechanics | 12 | Threat, combo points, interrupts, defense |
| `pet.rs` | Pet Management | 3 | Pet spells, dismissal, totem destruction |
| `profession.rs` | Professions | 6 | Skills, crafting, skinning |
| `quest.rs` | Quest/Reputation | 3 | Quest completion, honor, reputation |
| `script.rs` | Scripting | 5 | Dummy effects, triggers, events |
| `movement.rs` | Movement | 4 | Charge, knockback, leap, pull |
| `dispel.rs` | Dispel | 2 | Magic dispel, mechanic dispel |
| `pvp.rs` | PvP | 3 | Duel, sanctuary, player corpse |
| `object.rs` | Game Objects | 6 | Object spawning, activation, doors |
| `resurrect.rs` | Resurrection | 3 | Various resurrection types |
| `misc.rs` | Miscellaneous | 5 | Instakill, learn spell, language, etc. |

## Effect-to-File Mapping

### damage.rs (8 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 0 | `None` | No effect | Handled in dispatcher |
| 2 | `SchoolDamage` | Direct spell damage (Fireball, etc.) | ✓ Implemented |
| 7 | `EnvironmentalDamage` | Fire, lava, drowning, falling | Add to damage.rs |
| 9 | `HealthLeech` | Drain Life - damage + heal | ✓ Implemented |
| 17 | `WeaponDamageNoSchool` | Weapon damage without school | ✓ Implemented |
| 31 | `WeaponPercentDamage` | % of weapon damage | Add to damage.rs |
| 58 | `WeaponDamage` | Weapon damage with school | ✓ Implemented |
| 121 | `NormalizedWeaponDmg` | Normalized weapon damage | ✓ Implemented |

### healing.rs (4 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 10 | `Heal` | Direct heal | ✓ Implemented |
| 67 | `HealMaxHealth` | Heal to full (Lay on Hands) | ✓ Implemented |
| 75 | `HealMechanical` | Heal mechanical units | Add to healing.rs |
| 117 | `SpiritHeal` | Spirit healer resurrection heal | Add to healing.rs |

### power.rs (3 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 8 | `PowerDrain` | Drain mana/energy from target | ✓ Implemented |
| 30 | `Energize` | Restore mana/energy | ✓ Implemented |
| 62 | `PowerBurn` | Burn mana and deal damage | ✓ Implemented |

### aura.rs (7 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 6 | `ApplyAura` | Apply buff/debuff | ✓ Implemented |
| 27 | `PersistentAreaAura` | Ground effect (Consecration) | Add to aura.rs |
| 35 | `ApplyAreaAuraParty` | Party-wide aura | Add to aura.rs |
| 119 | `ApplyAreaAuraPet` | Pet aura | Add to aura.rs |
| 128 | `ApplyAreaAuraFriend` | Friendly area aura | Add to aura.rs |
| 129 | `ApplyAreaAuraEnemy` | Enemy area aura | Add to aura.rs |
| 132 | `ApplyAreaAuraRaid` | Raid-wide aura | Add to aura.rs |

### summon.rs (11 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 28 | `Summon` | Summon creature | Add to summon.rs |
| 41 | `SummonWild` | Summon wild creature | Add to summon.rs |
| 42 | `SummonGuardian` | Summon guardian | Add to summon.rs |
| 55 | `TameCreature` | Tame beast | Add to summon.rs |
| 56 | `SummonPet` | Summon active pet | Add to summon.rs |
| 73 | `SummonPossessed` | Mind control | Add to summon.rs |
| 74 | `SummonTotem` | Summon totem | Add to summon.rs |
| 87-90 | `SummonTotemSlot1-4` | Totem in specific slot | Add to summon.rs |
| 97 | `SummonCritter` | Vanity pet | Add to summon.rs |
| 109 | `SummonDeadPet` | Revive and summon pet | Add to summon.rs |
| 112 | `SummonDemon` | Summon warlock demon | Add to summon.rs |

### item.rs (10 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 24 | `CreateItem` | Create items (conjure) | Add to item.rs |
| 34 | `SummonChangeItem` | Transform item | Add to item.rs |
| 53 | `EnchantItem` | Permanent enchant | Add to item.rs |
| 54 | `EnchantItemTemporary` | Temporary enchant | Add to item.rs |
| 59 | `OpenLockItem` | Open lock with key | Add to item.rs |
| 92 | `EnchantHeldItem` | Enchant main hand | Add to item.rs |
| 99 | `Disenchant` | Disenchant item | Add to item.rs |
| 101 | `FeedPet` | Feed pet with item | Add to item.rs |
| 111 | `DurabilityDamage` | Damage item durability | Add to item.rs |
| 115 | `DurabilityDamagePCT` | % durability damage | Add to item.rs |

### teleport.rs (7 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 5 | `TeleportUnits` | Teleport to location | Add to teleport.rs |
| 11 | `Bind` | Set hearthstone location | Add to teleport.rs |
| 43 | `TeleportUnitsFaceCaster` | Teleport and face | Add to teleport.rs |
| 84 | `Stuck` | Unstuck teleport | Add to teleport.rs |
| 85 | `SummonPlayer` | Summon player to caster | Add to teleport.rs |
| 123 | `SendTaxi` | Start flight path | Add to teleport.rs |
| 124 | `PlayerPull` | Pull player to caster | Add to teleport.rs |

### combat.rs (12 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 19 | `AddExtraAttacks` | Extra melee swings | Add to combat.rs |
| 20 | `Dodge` | Enable dodge | Add to combat.rs |
| 22 | `Parry` | Enable parry | Add to combat.rs |
| 23 | `Block` | Enable block | Add to combat.rs |
| 40 | `DualWield` | Enable dual wield | Add to combat.rs |
| 63 | `Threat` | Modify threat | Add to combat.rs |
| 68 | `InterruptCast` | Interrupt spell | ✓ Implemented (move to combat.rs) |
| 69 | `Distract` | Distract NPCs | Add to combat.rs |
| 79 | `Sanctuary` | PvP protection | Add to combat.rs |
| 80 | `AddComboPoints` | Add combo points | Add to combat.rs |
| 114 | `AttackMe` | Taunt | Add to combat.rs |
| 125 | `ModifyThreatPercent` | Threat % modifier | Add to combat.rs |

### pet.rs (3 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 57 | `LearnPetSpell` | Teach pet spell | Add to pet.rs |
| 102 | `DismissPet` | Dismiss pet | Add to pet.rs |
| 110 | `DestroyAllTotems` | Destroy totems | Add to pet.rs |

### profession.rs (6 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 44 | `SkillStep` | Increase skill | Add to profession.rs |
| 47 | `TradeSkill` | Crafting | Add to profession.rs |
| 60 | `Proficiency` | Weapon/armor prof | Add to profession.rs |
| 95 | `Skinning` | Skin creature | Add to profession.rs |
| 116 | `SkinPlayerCorpse` | Remove insignia | Add to profession.rs |
| 118 | `Skill` | Learn skill | Add to profession.rs |

### quest.rs (3 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 16 | `QuestComplete` | Complete quest | Add to quest.rs |
| 45 | `AddHonor` | Add honor points | Add to quest.rs |
| 103 | `Reputation` | Modify reputation | Add to quest.rs |

### script.rs (5 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 3 | `Dummy` | Script placeholder | ✓ Implemented |
| 32 | `TriggerMissile` | Create projectile | Add to script.rs |
| 61 | `SendEvent` | Trigger game event | Add to script.rs |
| 64 | `TriggerSpell` | Cast another spell | ✓ Implemented (move to script.rs) |
| 77 | `ScriptEffect` | Script handler | Add to script.rs |
| 131 | `Nostalrius` | Custom effect | Add to script.rs |

### movement.rs (4 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 29 | `Leap` | Heroic leap | Add to movement.rs |
| 70 | `Pull` | Pull target | Add to movement.rs |
| 96 | `Charge` | Warrior charge | ✓ Implemented (fix ID from 78) |
| 98 | `KnockBack` | Knockback | ✓ Implemented |

### dispel.rs (2 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 38 | `Dispel` | Dispel magic | ✓ Implemented |
| 108 | `DispelMechanic` | Dispel by mechanic | Add to dispel.rs |

### pvp.rs (3 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 83 | `Duel` | Start duel | Add to pvp.rs |
| 100 | `Inebriate` | Drunk effect | Add to pvp.rs |
| 116 | `SkinPlayerCorpse` | BG corpse looting | Add to pvp.rs |

### object.rs (6 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 33 | `OpenLock` | Open door/chest | Add to object.rs |
| 50 | `TransDoor` | Activate door/portal | Add to object.rs |
| 76 | `SummonObjectWild` | Spawn game object | Add to object.rs |
| 86 | `ActivateObject` | Activate game object | Add to object.rs |
| 104-107 | `SummonObjectSlot1-4` | Object in slot | Add to object.rs |
| 130 | `DespawnObject` | Despawn object | Add to object.rs |

### resurrect.rs (3 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 18 | `Resurrect` | Resurrect player | Add to resurrect.rs |
| 94 | `SelfResurrect` | Self-resurrection | Add to resurrect.rs |
| 113 | `ResurrectNew` | New resurrection | Add to resurrect.rs |

### misc.rs (5 effects)

| ID | Effect Name | Description | Implementation Notes |
|----|-------------|-------------|---------------------|
| 1 | `Instakill` | Instant kill | ✓ Implemented |
| 36 | `LearnSpell` | Learn spell | ✓ Implemented |
| 39 | `Language` | Learn language | Add to misc.rs |
| 46 | `Spawn` | Spawn animation | Add to misc.rs |
| 81 | `CreateHouse` | Guild housing (TEST) | Add to misc.rs |

## Unused Effects (Excluded from Implementation)

These effects are marked as unused in MaNGOS or are test-only:

| ID | Effect Name | Reason |
|----|-------------|--------|
| 4 | `PortalTeleport` | unused from pre-1.2.1 |
| 12 | `Portal` | unused from pre-1.2.1 |
| 13 | `RitualBase` | unused from pre-1.2.1 |
| 14 | `RitualSpecialize` | unused from pre-1.2.1 |
| 15 | `RitualActivatePortal` | unused from pre-1.2.1 |
| 25 | `Weapon` | handled by skill system |
| 26 | `Defense` | one spell only |
| 37 | `SpellDefense` | one spell only (DND) |
| 48 | `Stealth` | handled by aura |
| 49 | `Detect` | handled by aura |
| 51 | `ForceCriticalHit` | unused from pre-1.2.1 |
| 52 | `GuaranteeHit` | unused from pre-1.2.1 |
| 65 | `HealthFunnel` | unused |
| 66 | `PowerFunnel` | unused from pre-1.2.1 |
| 78 | `Attack` | unused |
| 91 | `ThreatAll` | one test spell only |
| 93 | `SummonPhantasm` | unused |
| 120 | `TeleportGraveyard` | test only |
| 122 | `_122` | unused |
| 126 | `_126` | future spell steal? |
| 127 | `_127` | future prospecting? |
| 133 | `ApplyAreaAuraOwner` | handled differently |

## Implementation Priority

### Phase 1: Core Combat (Critical)
- damage.rs: EnvironmentalDamage, WeaponPercentDamage
- healing.rs: HealMechanical, SpiritHeal
- combat.rs: All 12 effects
- dispel.rs: DispelMechanic

### Phase 2: Player Systems (High)
- item.rs: All 10 effects
- teleport.rs: All 7 effects
- profession.rs: All 6 effects
- quest.rs: All 3 effects

### Phase 3: Pets/Summons (High)
- summon.rs: All 11 effects
- pet.rs: All 3 effects

### Phase 4: Advanced Features (Medium)
- aura.rs: Area auras
- script.rs: Script effects
- object.rs: Game objects
- resurrect.rs: Resurrection
- pvp.rs: PvP effects

### Phase 5: Polish (Low)
- misc.rs: Remaining effects
- movement.rs: Leap, Pull

## Notes

1. **Effect ID 96**: MaNGOS uses 96 for CHARGE, not 78. Update mod.rs accordingly.
2. **Area Auras**: Effects 128, 129, 132, 133 are BC+ backports, may not be needed for 1.12
3. **Custom Effects**: Effect 131 (Nostalrius) is server-specific
4. **Test Spells**: Several effects (81, 116, 120) are test-only and may not need full implementation
