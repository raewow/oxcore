# Audit: player_health_modification

## Scope
Audit of health modification primitives (damage, healing, environmental) against Rust implementation.

## Status: PARTIAL — Core primitives present, missing combat system integration, threat, rage, durability, and duel rules.

---

### 1. Unit::ModifyHealth

**C++ Behaviour:**
- Adds delta to current health
- If result <= 0: sets health to 0, returns -curHealth (actual loss)
- If result < maxHealth: sets health to result, returns delta
- If result >= maxHealth and curHealth != maxHealth: sets health to maxHealth, returns maxHealth - curHealth
- If already at max, returns 0
- No-op if delta == 0

**Rust Implementation:**
- Spell healing (healing.rs lines 66-86): `actual_heal = heal_amount.min(max_heal)` where `max_heal = max_health.saturating_sub(health)`
- Combat damage (combat/system.rs lines 210-214): `new_health = current_health.saturating_sub(damage)`
- Spell damage (damage.rs lines 558-561): Same pattern with `saturating_sub`
- No standalone `modify_health` method — health changes are done inline where needed
- **MISSING**: Unified primitive that returns actual gain/loss
- **MISSING**: Handling of negative delta (damage) with return value

**Verdict: ⚠️ PARTIAL** — Saturation logic present but scattered inline, no unified method with proper return semantics.

---

### 2. Unit::DealDamage

**C++ Behaviour:**
1. Removes stealth/feign death on non-DoT damage
2. Duel handling: leaves victim at 1 HP if killed by duel opponent
3. Enters combat via SetInCombatWithAggressor/SetInCombatWithVictim
4. Counts damage taken for creatures
5. Rage generation for warrior on outgoing direct damage (RewardRage)
6. Loot recipient assignment for creatures
7. Death path: if health <= damage and invincibility threshold is 0, calls Kill() and handles duel completion
8. Non-death path: reduces health, generates threat for creatures, generates rage for victim warrior, applies durability loss chance for both attacker and victim players, interrupts victim spells on damage, delays channeled spells
9. Returns actual damage dealt

**Rust Implementation:**
- CombatSystem::apply_damage (combat/system.rs lines 204-228):
  - `player.stats.health = current_health.saturating_sub(damage)`
  - `player.combat.enter_combat(target)`
  - **TODO**: Broadcast health update, check for death
- Spell damage pipeline (damage.rs lines 534-579):
  - Absorb shields via `auras.absorb_damage` (line 549)
  - Damage applied with `saturating_sub`
  - Death detection (`new_health == 0 && current_health > 0`)
  - SMSG_SPELLNONMELEEDAMAGELOG sent
- **MISSING**: Stealth/feign death removal
- **MISSING**: Duel handling (1 HP rule)
- **MISSING**: Rage generation (dealer and victim)
- **MISSING**: Threat generation for creatures
- **MISSING**: Loot recipient assignment
- **MISSING**: Durability loss on damage
- **MISSING**: Spell interrupt on damage
- **MISSING**: Invincibility threshold check
- **MISSING**: Returns actual damage (absorb not subtracted from return)

**Verdict: ⚠️ PARTIAL** — Basic damage application and absorb present, but combat integration (rage, threat, durability, duel, interrupts) is missing.

---

### 3. SpellCaster::DealHeal

**C++ Behaviour:**
- Calls AI::HealedBy on victim
- Calls ModifyHealth with addhealth
- If caster is player or victim is player: sends SMSG_SPELLHEALLOG
- Returns actual health gain (from ModifyHealth)
- Totem healers redirect to their owner for combat log

**Rust Implementation:**
- effect_heal (healing.rs lines 20-131):
  - Calculates base heal + healing power * coefficient
  - Rolls crit (1.5x)
  - Applies heal: `actual_heal = heal_amount.min(max_heal)`
  - Sends SMSG_SPELLHEALLOG (line 90-98)
  - Overheal tracking (line 89)
  - **MISSING**: AI::HealedBy hook
  - **MISSING**: Totem owner redirection for combat log
- **PRESENT**: SMSG_SPELLHEALLOG with proper packet format (lines 247-269)

**Verdict: ⚠️ PARTIAL** — Core heal application and combat log present, missing AI hooks and totem redirection.

---

### 4. SpellCaster::SendHealSpellLog

**C++ Behaviour:**
- Sends SMSG_SPELLHEALLOG packet
- Contains: victim pack GUID, caster pack GUID, spell ID, heal amount, critical flag (1 if crit)
- Sent to all nearby players via SendMessageToSet

**Rust Implementation:**
- send_spell_heal_log (healing.rs lines 247-269):
  - `WorldPacket::new(Opcode::SMSG_SPELLHEALLOG)`
  - Writes: packed target GUID, packed caster GUID, spell_id, heal_amount, overheal, crit flag
  - Broadcasts via `broadcast_nearby(caster_guid, &packet, true)`
  - **DIFFERENT**: Also sends overheal amount (not in C++ version)
  - **PRESENT**: All other fields match

**Verdict: ✅ CORRECT** — Packet format matches, with extra overheal field (which is client-compatible).

---

### 5. SpellCaster::DealDamageMods

**C++ Behaviour:**
- If victim is dead, taxi flying, or creature in evade mode, or caster is priest with Spirit of Redemption (aura 27827): sets damage to 0 and adds to absorb
- Otherwise calls AI::DamageDeal and AI::DamageTaken for script hooks
- Adjusts absorb value if damage was reduced by scripts
- Does not modify for normal players

**Rust Implementation:**
- **MISSING**: No direct equivalent
- Absorb logic is handled in `auras.absorb_damage` (damage.rs line 549)
- **MISSING**: Dead/taxi flying/evade checks before damage application
- **MISSING**: Spirit of Redemption check
- **MISSING**: AI script hooks (DamageDeal/DamageTaken)

**Verdict: ⚠️ PARTIAL** — Absorb is handled by aura system, but pre-damage validation checks and AI hooks are missing.

---

### 6. Player::EnvironmentalDamage

**C++ Behaviour:**
- Returns 0 if dead or GM
- Applies school-specific immunity/absorb/resist: FIRE for lava/fire, NATURE for slime
- Exhausted/drowning/fall use NORMAL school; absorb does not work for these in patch 1.7+
- After resist/absorb, calls DealDamageMods
- Sends environmental damage log (SMSG_ENVIRONMENTALDAMAGELOG)
- Self-damages via DealDamage(this, ..., SELF_DAMAGE)
- On death: applies 10% durability loss to all items and sends SMSG_DURABILITY_DAMAGE_DEATH

**Rust Implementation:**
- EnvironmentSystem::environmental_damage (environment/system.rs lines 138-208):
  - Checks if alive (returns 0 if dead)
  - **MISSING**: GM check
  - Clamps damage to current health: `applied = amount.min(health)`
  - Sends SMSG_ENVIRONMENTALDAMAGELOG (lines 176-184)
  - On death: calls `death.on_killed(player_guid, None, None, world)` (lines 187-204)
  - **MISSING**: School-specific resistance/absorb (fire, nature, slime)
  - **MISSING**: DealDamageMods call
  - **MISSING**: 10% durability loss on death
  - **MISSING**: SMSG_DURABILITY_DAMAGE_DEATH
  - **MISSING**: SELF_DAMAGE flag for self-damage
- Spell effect pipeline (damage.rs lines 439-472):
  - effect_environmental_damage: applies damage directly without resistance/absorb
  - **MISSING**: School-specific handling

**Verdict: ⚠️ PARTIAL** — Basic environmental damage and death handling present, missing resistance/absorb, durability loss, and GM checks.

---

## Summary

| Symbol | Status | Gaps |
|--------|--------|------|
| Unit::ModifyHealth | ⚠️ Partial | Scattered inline, no unified method with return semantics |
| Unit::DealDamage | ⚠️ Partial | Rage, threat, durability, duel, interrupts, stealth removal |
| SpellCaster::DealHeal | ⚠️ Partial | AI hooks, totem redirection |
| SpellCaster::SendHealSpellLog | ✅ Complete | Extra overheal field (compatible) |
| SpellCaster::DealDamageMods | ⚠️ Partial | Pre-damage checks, AI hooks, Spirit of Redemption |
| Player::EnvironmentalDamage | ⚠️ Partial | Resistance/absorb, durability loss, GM check |

**Overall: 1/6 Complete, 5/6 Partial.**

**Critical gaps for full audit pass:**
1. Unified `ModifyHealth` primitive with proper return semantics
2. Combat system integration: rage generation, threat generation, durability loss
3. Duel handling (1 HP rule)
4. Spell interrupt on damage
5. Stealth/feign death removal on damage
6. Environmental damage: school-specific resistance, absorb, durability loss
7. Pre-damage validation (dead, flying, evade, Spirit of Redemption)
8. AI hooks (DamageDeal, DamageTaken, HealedBy)

## Next Steps

1. Implement unified `ModifyHealth` primitive in player stats
2. Port rage generation system (`RewardRage`)
3. Port threat generation for creature damage
4. Port durability loss on damage
5. Port duel handling (1 HP rule)
6. Port spell interrupt on damage
7. Port stealth/feign death removal
8. Add environmental damage resistance/absorb
9. Add environmental death durability loss
10. Re-audit after gaps are closed
