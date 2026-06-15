# Audit: player_health_power_resurrection

## Scope
Audit of health/power restoration on creation, resurrection, and level-up against Rust implementation.

## Status: PARTIAL — Core resurrection paths present, missing character creation details, DB deletion, and some sickness edge cases.

---

### 1. Player::Create

**C++ Behaviour:**
- Creates Object with guidlow
- Validates race/class pair via ObjectMgr
- Sets position from PlayerInfo
- Sets power type from ChrClasses DBC
- Sets faction, race, class, gender, power type in UNIT_FIELD_BYTES_0
- Sets player flags (UNIT_FLAG_PLAYER_CONTROLLED)
- Sets appearance bytes (skin, face, hair, color, facial hair)
- Sets starting level from config
- Sets starting money
- Calls InitStatsForLevel, InitTaxiNodes, InitTalentForLevel, InitPrimaryProfessions
- Loads reputation
- Calls UpdateMaxHealth and SetHealth(max)
- For mana classes: UpdateMaxPower(MANA) and SetPower(MANA, max)
- Calls LearnDefaultSpells

**Rust Implementation:**
- Character creation handled in `handle_char_create` (character.rs, not fully read)
- Player struct is populated from DB on login (no explicit "Create" method in the Rust sense)
- StatsSystem::on_player_login (stats/system.rs lines 65-74):
  - Calls `recalculate_all(guid)` then sets `health = max_health`, `mana = max_mana`
- PowerSystem::on_login (power/system.rs lines 269-298):
  - Sets power type, max values, initializes current mana/energy to max, rage to 0
- **MISSING**: Explicit character creation method with validation
- **MISSING**: Appearance bytes setup
- **MISSING**: Starting money
- **MISSING**: InitTaxiNodes, InitTalentForLevel, InitPrimaryProfessions
- **MISSING**: LearnDefaultSpells
- **MISSING**: Reputation loading
- **PRESENT**: Full health/mana on login (stats + power system)

**Verdict: ⚠️ PARTIAL** — Health/mana initialization present, but character creation pipeline is not fully ported.

---

### 2. Player::BuildEnumData

**C++ Behaviour:**
- Reads from QueryResult (character row)
- Serializes: guid, name, race, class, gender, skin, face, hairStyle, hairColor, facialHair, level, zone, map, position, guildId, flags, first login flag, pet displayId, pet level, pet family, equipment display IDs/Enchant IDs for 19 visible slots, weapon enchant glow for main hand

**Rust Implementation:**
- handle_char_enum (character.rs lines 42-100):
  - Uses CharacterRepository to fetch characters
  - Builds `CharacterEnumEntry` with all fields
  - Equipment display IDs from item templates (lines 82-98)
  - **PRESENT**: guid, name, race, class, gender, level, zone, map, position, guildId, flags
  - **PRESENT**: Equipment display IDs and inventory types for 19 slots
  - **MISSING**: Pet displayId, pet level, pet family
  - **MISSING**: Weapon enchant glow for main hand
  - **MISSING**: First login flag
  - **MISSING**: Appearance bytes (skin, face, hairStyle, hairColor, facialHair) — these may be in the DB row but not explicitly shown in the code read

**Verdict: ⚠️ PARTIAL** — Core enum data present, missing pet info and weapon enchant glow.

---

### 3. Player::DeleteFromDB

**C++ Behaviour:**
- Deletes all related records: character_inventory, mail, mail_items, item_instance, character_social, character_spell, character_talent, character_achievement, character_reputation, character_queststatus, character_cooldowns, character_aura, character_glyphs, character_skills, character_stats, character_pet, guild members, arena teams, battleground data, and the character row
- Handles pet deletion, mail cleanup, and item deletion
- Returns true on success

**Rust Implementation:**
- **MISSING**: No explicit character deletion handler found
- Likely handled in the database layer or account management

**Verdict: ⚠️ PARTIAL** — Not found in the game code. May be in a separate service.

---

### 4. Player::ResurrectPlayer

**C++ Behaviour:**
- Interrupts resurrect spells
- Sets death state to ALIVE
- Removes ghost form
- Unroots
- If restore_percent > 0: restores health to max*percent, mana to max*percent, rage to 0, energy to max*percent
- Updates zone/area for alive state
- Resets death timer
- Updates visibility
- If applySickness is true and level >= death_sickness_level (default 11): casts resurrection sickness spell
- Duration = 1 minute per level above 10 for levels 11-19, 10 minutes for level 20+
- Uses race-specific sickness spell from ChrRaces DBC

**Rust Implementation:**
- DeathSystem::resurrect_player (death/system.rs lines 812-961):
  - Handles CorpseRun, SpiritHealer, PlayerSpell, SelfResurrection, Battleground methods
  - CorpseRun: 50% health/mana (resurrect.rs lines 51-65)
  - SpiritHealer: 50% health/mana + sickness (resurrect.rs lines 71-85)
  - PlayerSpell: uses stored health/mana from resurrection data (resurrect.rs lines 92-105)
  - Sets health and mana (system.rs lines 866-867)
  - Removes ghost form (line 871)
  - Restores run speed (line 874)
  - Clears water walking (lines 876-878)
  - Clears unit flags (line 881)
  - Despawns corpse (lines 918-920)
  - Applies resurrection sickness for spirit healer (lines 938-950)
  - Sickness spell: AURA_MOD_TOTAL_STAT_PERCENTAGE (-75%) and AURA_MOD_DAMAGE_PERCENT_DONE (-75%) (lines 992-1000)
  - Teleports for corpse run/player spell (lines 933-935)
  - Broadcasts update (lines 955-958)
- **MISSING**: Interrupts resurrect spells
- **MISSING**: Rage reset to 0, energy restore
- **MISSING**: Zone/area update for alive state
- **MISSING**: Death timer reset
- **PRESENT**: Resurrection sickness with correct spell effects and duration calculation
- **PRESENT**: Ghost form removal, speed restoration, corpse despawn

**Verdict: ⚠️ PARTIAL** — Core resurrection paths present with correct sickness. Missing spell interrupt, rage/energy reset, and zone update.

---

### 5. Player::UpdateAllStats

**C++ Behaviour:**
- Iterates all 5 stats (STR/AGI/STA/INT/SPI), sets each via GetTotalStatValue + SetStat
- Calls: UpdateAttackPowerAndDamage(false), UpdateAttackPowerAndDamage(true), UpdateMaxHealth(), UpdateMaxPower(i) for all power types, UpdateAllCritPercentages(), UpdateAllSpellCritChances(), UpdateDefenseBonusesMod(), UpdateSpellDamageAndHealingBonus(), UpdateManaRegen(), UpdateResistances(i) for all schools
- Called on login, level-up, aura apply/remove, gear change
- Returns true unconditionally

**Rust Implementation:**
- StatsSystem::recalculate_all (stats/system.rs lines 83-307):
  - Computes all 5 stats with modifier formula (lines 102-127)
  - Updates max health with stamina bonus and ratio preservation (lines 130-163)
  - Updates max mana with intellect bonus and ratio preservation (lines 165-205)
  - Attack power (lines 208-212)
  - Armor and resistances (lines 214-230)
  - Spell power and healing power (lines 232-257)
  - Crit percentages (lines 260-277)
  - Dodge (lines 280-282)
  - Parry/Block base values (lines 285-286)
  - Damage ranges (lines 288-296)
  - Mana regen base (lines 298-303)
  - **MISSING**: UpdateAllSpellCritChances
  - **MISSING**: UpdateDefenseBonusesMod (block, parry, dodge defense skill scaling)
  - **MISSING**: UpdateManaRegen aura integration
  - **PRESENT**: All core stat calculations, health/mana ratio preservation

**Verdict: ⚠️ PARTIAL** — Core recalculation present. Missing spell crit, defense bonuses, and mana regen aura integration.

---

## Summary

| Symbol | Status | Gaps |
|--------|--------|------|
| Player::Create | ⚠️ Partial | Missing creation pipeline, validation, appearance, spells, reputation |
| Player::BuildEnumData | ⚠️ Partial | Missing pet info, weapon enchant glow, first login flag |
| Player::DeleteFromDB | ⚠️ Partial | Not found in game code |
| Player::ResurrectPlayer | ⚠️ Partial | Missing spell interrupt, rage/energy reset, zone update |
| Player::UpdateAllStats | ⚠️ Partial | Missing spell crit, defense bonuses, mana regen aura |

**Overall: 0/5 Complete, 5/5 Partial.**

**Critical gaps for full audit pass:**
1. Character creation pipeline with validation and initialization
2. Character deletion handler
3. Pet info in character enum
4. Weapon enchant glow in character enum
5. Resurrection spell interrupt
6. Rage/energy reset on resurrection
7. Zone update on resurrection
8. Spell crit calculation in UpdateAllStats
9. Defense bonuses (block/parry/dodge defense skill scaling)
10. Mana regen aura integration in UpdateAllStats

## Next Steps

1. Port character creation pipeline
2. Port character deletion handler
3. Add pet info to character enum
4. Add weapon enchant glow to character enum
5. Add resurrection spell interrupt
6. Add rage/energy reset on resurrection
7. Add zone update on resurrection
8. Add spell crit to recalculate_all
9. Add defense bonuses to recalculate_all
10. Re-audit after gaps are closed
