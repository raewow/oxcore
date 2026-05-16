//! Spell Effect Dispatcher
//!
//! Each spell has up to 3 effects (Effect1, Effect2, Effect3 in Spell.dbc).
//! The dispatcher reads the spell entry and calls the appropriate handler
//! for each effect.
//!
//! CRITICAL: This module must stay small. Each effect type gets its own file.
//! The old executor.rs was 335KB - we must never allow that to happen again.

pub mod aura;
pub mod combat;
pub mod damage;
pub mod dispel;
pub mod healing;
pub mod item;
pub mod misc;
pub mod movement;
pub mod object;
pub mod pet;
pub mod power;
pub mod profession;
pub mod pvp;
pub mod quest;
pub mod resurrect;
pub mod script;
pub mod teleport;

use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;

/// Input data for a single spell effect.
///
/// This is a snapshot of everything an effect handler needs.
/// No locks, no Arc, just plain data.
#[derive(Debug, Clone)]
pub struct EffectInput {
    pub caster_guid: ObjectGuid,
    pub target_guid: Option<ObjectGuid>,
    pub spell_id: u32,
    pub effect_index: u8,
    pub base_value: i32,
    pub misc_value: i32,
    pub misc_value_b: i32,
    pub is_triggered: bool,
    /// Die sides for random damage variance (from DBC effect_die_sides)
    pub die_sides: i32,
    /// Points gained per caster level (from DBC effect_real_points_per_level)
    pub points_per_level: f32,
    /// Spell power coefficient (from DBC effect_bonus_coefficient, or calculated from cast time)
    pub spell_coefficient: f32,
    /// Spell school (0=Physical, 1=Holy, 2=Fire, 3=Nature, 4=Frost, 5=Shadow, 6=Arcane)
    pub spell_school: u8,
    /// Casting time in milliseconds (from DBC, for fallback coefficient calc)
    pub casting_time_index: u32,
}

impl EffectInput {
    pub fn new(
        caster_guid: ObjectGuid,
        target_guid: Option<ObjectGuid>,
        spell_id: u32,
        effect_index: u8,
    ) -> Self {
        Self {
            caster_guid,
            target_guid,
            spell_id,
            effect_index,
            base_value: 0,
            misc_value: 0,
            misc_value_b: 0,
            is_triggered: false,
            die_sides: 0,
            points_per_level: 0.0,
            spell_coefficient: 0.0,
            spell_school: 0,
            casting_time_index: 0,
        }
    }

    /// Calculate base value with dice roll and level scaling.
    /// Matches old system's calculate_base_value().
    pub fn calculate_base_value(&self, caster_level: u8) -> i32 {
        let level_bonus = (caster_level as f32 * self.points_per_level) as i32;
        let base = self.base_value + level_bonus;

        if self.die_sides > 0 {
            // Random roll: 1..=die_sides
            use rand::Rng;
            let roll = rand::thread_rng().gen_range(1..=self.die_sides);
            base + roll
        } else {
            base
        }
    }

    /// Get the spell power coefficient for this effect.
    /// Uses DBC coefficient if available, otherwise calculates from cast time.
    pub fn get_spell_coefficient(&self) -> f32 {
        if self.spell_coefficient > 0.01 {
            return self.spell_coefficient;
        }

        // Fallback: calculate from cast time
        if self.casting_time_index == 0 {
            // Instant cast spells get reduced coefficient
            0.15
        } else {
            // Formula: cast_time_ms / 3500, capped at 1.0
            (self.casting_time_index as f32 / 3500.0).min(1.0)
        }
    }
}

/// Result from an effect handler.
#[derive(Debug, Clone)]
pub struct EffectResult {
    /// Damage dealt (to be applied by executor)
    pub damage: u32,
    /// Healing done (to be applied by executor)
    pub healing: u32,
    /// Whether the effect succeeded
    pub success: bool,
}

impl EffectResult {
    pub fn empty() -> Self {
        Self {
            damage: 0,
            healing: 0,
            success: true,
        }
    }

    pub fn with_damage(damage: u32) -> Self {
        Self {
            damage,
            healing: 0,
            success: true,
        }
    }

    pub fn with_healing(healing: u32) -> Self {
        Self {
            damage: 0,
            healing,
            success: true,
        }
    }
}

/// Effect dispatcher - routes spell effects to their handlers.
pub struct EffectsDispatcher;

impl EffectsDispatcher {
    /// Create a new effects dispatcher
    pub fn new() -> Self {
        Self
    }

    /// Dispatch all effects for a spell.
    ///
    /// Uses the target resolution system to determine per-effect targets,
    /// then runs hit/miss rolls against each target before applying effects.
    ///
    /// Flow per effect:
    /// 1. Get resolved targets for this effect
    /// 2. For each target: roll hit/miss/resist
    /// 3. On hit: call effect handler
    /// 4. On miss/resist: record for SMSG_SPELL_GO miss list
    pub async fn dispatch(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        world: &World,
    ) -> Result<Vec<EffectResult>> {
        self.dispatch_with_targets(
            caster_guid,
            spell_id,
            target_guid,
            is_triggered,
            None,
            world,
        )
        .await
    }

    /// Dispatch with pre-resolved targets (used when target resolution already happened).
    pub async fn dispatch_with_targets(
        &self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        resolved: Option<&crate::world::game::player::spells::targets::ResolvedTargets>,
        world: &World,
    ) -> Result<Vec<EffectResult>> {
        use crate::world::game::player::spells::hit;

        let mut results = Vec::new();

        // Look up spell entry
        let spell_entry = match world.managers.spell_mgr.get(spell_id) {
            Some(entry) => entry,
            None => {
                tracing::warn!("Spell {} not found", spell_id);
                return Ok(results);
            }
        };

        // For each of the 3 effects:
        for effect_index in 0..3 {
            let effect_type = spell_entry.effect[effect_index];
            if effect_type == 0 {
                continue; // SPELL_EFFECT_NONE
            }

            // Convert effect type to enum
            let effect_type_enum = match SpellEffectType::from_u32(effect_type) {
                Some(et) => et,
                None => {
                    tracing::warn!("Unknown effect type {} for spell {}", effect_type, spell_id);
                    continue;
                }
            };

            // Get targets for this effect
            let effect_targets: Vec<ObjectGuid> = if let Some(res) = resolved {
                res.effect_targets[effect_index].clone()
            } else {
                // Legacy path: use explicit target_guid
                target_guid.into_iter().collect()
            };

            // Dispatch effect once per target
            for eff_target in &effect_targets {
                // Roll hit/miss (skip for self-targeting and triggered spells)
                if !is_triggered && *eff_target != caster_guid {
                    let outcome = hit::roll_spell_hit(caster_guid, *eff_target, spell_id, world);
                    match outcome {
                        hit::SpellHitOutcome::Miss => {
                            tracing::debug!(
                                "[SPELL-HIT] spell {} MISSED target {:?}",
                                spell_id,
                                eff_target
                            );
                            continue;
                        }
                        hit::SpellHitOutcome::Resist => {
                            tracing::debug!(
                                "[SPELL-HIT] spell {} RESISTED by {:?}",
                                spell_id,
                                eff_target
                            );
                            continue;
                        }
                        hit::SpellHitOutcome::Immune => {
                            tracing::debug!(
                                "[SPELL-HIT] spell {} target {:?} IMMUNE",
                                spell_id,
                                eff_target
                            );
                            continue;
                        }
                        hit::SpellHitOutcome::Reflect => {
                            tracing::debug!(
                                "[SPELL-HIT] spell {} REFLECTED by {:?}",
                                spell_id,
                                eff_target
                            );
                            // TODO: Apply effect to caster instead
                            continue;
                        }
                        hit::SpellHitOutcome::Hit | hit::SpellHitOutcome::PartialResist(_) => {
                            // Fire SpellHit AI event on the target creature (if it has a Lua AI script).
                            // Only fire on the first effect (index 0) to avoid duplicate callbacks.
                            if effect_index == 0 {
                                let is_creature = world
                                    .managers
                                    .creature_mgr
                                    .with_creature(*eff_target, |_| ())
                                    .is_some();
                                if is_creature {
                                    crate::world::game::creature::ai::queue_event(
                                        world,
                                        *eff_target,
                                        crate::world::game::creature::ai::AIEvent::SpellHit {
                                            caster_guid,
                                            spell_id,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }

                // Create EffectInput for this target
                let input = EffectInput {
                    caster_guid,
                    target_guid: Some(*eff_target),
                    spell_id,
                    effect_index: effect_index as u8,
                    base_value: spell_entry.effect_base_points[effect_index],
                    misc_value: spell_entry.effect_misc_value[effect_index],
                    misc_value_b: 0,
                    is_triggered,
                    die_sides: spell_entry.effect_die_sides[effect_index],
                    points_per_level: spell_entry.effect_real_points_per_level[effect_index],
                    spell_coefficient: spell_entry.effect_bonus_coefficient[effect_index],
                    spell_school: spell_entry.school as u8,
                    casting_time_index: spell_entry.casting_time_index,
                };

                match dispatch_effect(effect_type_enum, &input, world).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        tracing::error!(
                            "Effect {} failed for spell {} target {:?}: {}",
                            effect_index,
                            spell_id,
                            eff_target,
                            e
                        );
                        results.push(EffectResult::empty());
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Default for EffectsDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Spell effect types (from Spell.dbc Effect column)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffectType {
    None = 0,
    Instakill = 1,
    SchoolDamage = 2,
    Dummy = 3,
    ApplyAura = 6,
    EnvironmentalDamage = 7,
    PowerDrain = 8,
    HealthLeech = 9,
    Heal = 10,
    WeaponDamageNoSchool = 17,
    Energize = 30,
    WeaponPercentDamage = 31,
    ApplyAreaAuraParty = 35,
    LearnSpell = 36,
    Dispel = 38,
    WeaponDamage = 58,
    PowerBurn = 62,
    TriggerSpell = 64,
    HealMaxHealth = 67,
    InterruptCast = 68,
    PersistentAreaAura = 27,
    HealMechanical = 75,
    Charge = 78,
    KnockBack = 98,
    NormalizedWeaponDmg = 121,
    SpiritHeal = 117,
    ApplyAreaAuraPet = 119,
    ApplyAreaAuraFriend = 128,
    ApplyAreaAuraEnemy = 129,
    ApplyAreaAuraRaid = 132,
    // Item effects
    CreateItem = 24,
    SummonChangeItem = 34,
    EnchantItem = 53,
    EnchantItemTemporary = 54,
    OpenLockItem = 59,
    EnchantHeldItem = 92,
    Disenchant = 99,
    FeedPet = 101,
    DurabilityDamage = 111,
    DurabilityDamagePCT = 115,
    // Teleport effects
    TeleportUnits = 5,
    Bind = 11,
    TeleportUnitsFaceCaster = 43,
    Stuck = 84,
    SummonPlayer = 85,
    SendTaxi = 123,
    PlayerPull = 124,
    // Profession effects
    SkillStep = 44,
    TradeSkill = 47,
    Proficiency = 60,
    Skinning = 95,
    SkinPlayerCorpse = 116,
    Skill = 118,
    // Quest effects
    QuestComplete = 16,
    AddHonor = 45,
    Reputation = 103,
    // Combat effects
    AddExtraAttacks = 19,
    Dodge = 20,
    Parry = 22,
    Block = 23,
    DualWield = 40,
    Threat = 63,
    Distract = 69,
    Sanctuary = 79,
    AddComboPoints = 80,
    AttackMe = 114,
    ModifyThreatPercent = 125,
    // Pet effects
    LearnPetSpell = 57,
    DismissPet = 102,
    DestroyAllTotems = 110,
    // Script effects
    TriggerMissile = 32,
    SendEvent = 61,
    ScriptEffect = 77,
    Nostalrius = 131,
    // Dispel effects
    DispelMechanic = 108,
    // PvP effects
    Duel = 83,
    Inebriate = 100,
    // Object effects
    OpenLock = 33,
    TransDoor = 50,
    SummonObjectWild = 76,
    ActivateObject = 86,
    SummonObjectSlot1 = 104,
    SummonObjectSlot2 = 105,
    SummonObjectSlot3 = 106,
    SummonObjectSlot4 = 107,
    DespawnObject = 130,
    // Resurrection effects
    Resurrect = 18,
    SelfResurrect = 94,
    ResurrectNew = 113,
    // Misc effects
    Language = 39,
    Spawn = 46,
    CreateHouse = 81,
    // Movement effects
    Leap = 29,
    Pull = 70,
    // --- Missing effects added for MaNGOS parity ---
    // Summon effects
    SummonType = 28,    // Generic summon dispatcher (pets, totems, guardians)
    SummonPet = 56,     // Summon hunter/warlock pet
    SummonDeadPet = 55, // Revive Pet
    TameCreature = 167, // Hunter tame beast
    // Healing/Power
    HealPct = 136,     // Heal % of max health
    EnergizePct = 137, // Energize % of max power
    // Trigger
    TriggerSpellWithValue = 109, // Trigger spell, passing base_value
    // Combat
    Taunt = 87,   // Direct taunt effect
    Charge2 = 96, // Charge variant
    // Quest
    QuestStart = 134,         // Start a quest
    QuestFail = 135,          // Fail a quest
    KillCreditPersonal = 138, // Kill credit (personal)
    KillCreditGroup = 139,    // Kill credit (group)
    // Misc
    PickPocket = 140,          // Rogue pick pocket
    AddFarsight = 141,         // Far sight effect
    StealBeneficialBuff = 126, // Steal a buff (not used in vanilla, stub)
    ForceCast = 133,           // Force target to cast
    RemoveAura = 65,           // Remove an aura
    // Stealth
    Stealth = 48, // Apply stealth effect (not aura)
}

impl SpellEffectType {
    /// Convert from u32
    pub fn from_u32(value: u32) -> Option<Self> {
        use SpellEffectType::*;
        match value {
            0 => Some(SpellEffectType::None),
            1 => Some(Instakill),
            2 => Some(SchoolDamage),
            3 => Some(Dummy),
            6 => Some(ApplyAura),
            7 => Some(EnvironmentalDamage),
            8 => Some(PowerDrain),
            9 => Some(HealthLeech),
            10 => Some(Heal),
            17 => Some(WeaponDamageNoSchool),
            27 => Some(PersistentAreaAura),
            30 => Some(Energize),
            31 => Some(WeaponPercentDamage),
            35 => Some(ApplyAreaAuraParty),
            36 => Some(LearnSpell),
            38 => Some(Dispel),
            58 => Some(WeaponDamage),
            62 => Some(PowerBurn),
            64 => Some(TriggerSpell),
            67 => Some(HealMaxHealth),
            68 => Some(InterruptCast),
            75 => Some(HealMechanical),
            78 => Some(Charge),
            98 => Some(KnockBack),
            117 => Some(SpiritHeal),
            119 => Some(ApplyAreaAuraPet),
            121 => Some(NormalizedWeaponDmg),
            128 => Some(ApplyAreaAuraFriend),
            129 => Some(ApplyAreaAuraEnemy),
            132 => Some(ApplyAreaAuraRaid),
            // Item effects
            24 => Some(CreateItem),
            34 => Some(SummonChangeItem),
            53 => Some(EnchantItem),
            54 => Some(EnchantItemTemporary),
            59 => Some(OpenLockItem),
            92 => Some(EnchantHeldItem),
            99 => Some(Disenchant),
            101 => Some(FeedPet),
            111 => Some(DurabilityDamage),
            115 => Some(DurabilityDamagePCT),
            // Teleport effects
            5 => Some(TeleportUnits),
            11 => Some(Bind),
            43 => Some(TeleportUnitsFaceCaster),
            84 => Some(Stuck),
            85 => Some(SummonPlayer),
            123 => Some(SendTaxi),
            124 => Some(PlayerPull),
            // Profession effects
            44 => Some(SkillStep),
            47 => Some(TradeSkill),
            60 => Some(Proficiency),
            95 => Some(Skinning),
            116 => Some(SkinPlayerCorpse),
            118 => Some(Skill),
            // Quest effects
            16 => Some(QuestComplete),
            45 => Some(AddHonor),
            103 => Some(Reputation),
            // Combat effects
            19 => Some(AddExtraAttacks),
            20 => Some(Dodge),
            22 => Some(Parry),
            23 => Some(Block),
            40 => Some(DualWield),
            63 => Some(Threat),
            69 => Some(Distract),
            79 => Some(Sanctuary),
            80 => Some(AddComboPoints),
            114 => Some(AttackMe),
            125 => Some(ModifyThreatPercent),
            // Pet effects
            57 => Some(LearnPetSpell),
            102 => Some(DismissPet),
            110 => Some(DestroyAllTotems),
            // Script effects
            32 => Some(TriggerMissile),
            61 => Some(SendEvent),
            77 => Some(ScriptEffect),
            131 => Some(Nostalrius),
            // Dispel effects
            108 => Some(DispelMechanic),
            // PvP effects
            83 => Some(Duel),
            100 => Some(Inebriate),
            // Object effects
            33 => Some(OpenLock),
            50 => Some(TransDoor),
            76 => Some(SummonObjectWild),
            86 => Some(ActivateObject),
            104 => Some(SummonObjectSlot1),
            105 => Some(SummonObjectSlot2),
            106 => Some(SummonObjectSlot3),
            107 => Some(SummonObjectSlot4),
            130 => Some(DespawnObject),
            // Resurrection effects
            18 => Some(Resurrect),
            94 => Some(SelfResurrect),
            113 => Some(ResurrectNew),
            // Misc effects
            39 => Some(Language),
            46 => Some(Spawn),
            81 => Some(CreateHouse),
            // Movement effects
            29 => Some(Leap),
            70 => Some(Pull),
            // Missing effects for MaNGOS parity
            28 => Some(SummonType),
            48 => Some(Stealth),
            55 => Some(SummonDeadPet),
            56 => Some(SummonPet),
            65 => Some(RemoveAura),
            87 => Some(Taunt),
            96 => Some(Charge2),
            109 => Some(TriggerSpellWithValue),
            126 => Some(StealBeneficialBuff),
            133 => Some(ForceCast),
            134 => Some(QuestStart),
            135 => Some(QuestFail),
            136 => Some(HealPct),
            137 => Some(EnergizePct),
            138 => Some(KillCreditPersonal),
            139 => Some(KillCreditGroup),
            140 => Some(PickPocket),
            141 => Some(AddFarsight),
            167 => Some(TameCreature),
            _ => Option::None,
        }
    }
}

/// Dispatch a single effect to its handler.
///
/// This is the main routing function. Effect types are from Spell.dbc Effect field.
pub async fn dispatch_effect(
    effect_type: SpellEffectType,
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    use SpellEffectType::*;

    match effect_type {
        None => Ok(EffectResult::empty()),
        Instakill => misc::effect_insta_kill(input, world).await,
        SchoolDamage => damage::effect_school_damage(input, world).await,
        Dummy => script::effect_dummy(input, world).await,
        ApplyAura => aura::effect_apply_aura(input, world).await,
        EnvironmentalDamage => damage::effect_environmental_damage(input, world).await,
        PowerDrain => power::effect_power_drain(input, world).await,
        HealthLeech => damage::effect_health_leech(input, world).await,
        Heal => healing::effect_heal(input, world).await,
        WeaponDamageNoSchool => damage::effect_weapon_damage(input, world).await,
        PersistentAreaAura => aura::effect_persistent_area_aura(input, world).await,
        Energize => power::effect_energize(input, world).await,
        WeaponPercentDamage => damage::effect_weapon_percent_damage(input, world).await,
        ApplyAreaAuraParty => aura::effect_apply_area_aura_party(input, world).await,
        LearnSpell => misc::effect_learn_spell(input, world).await,
        Dispel => dispel::effect_dispel(input, world).await,
        WeaponDamage => damage::effect_weapon_damage(input, world).await,
        PowerBurn => power::effect_power_burn(input, world).await,
        TriggerSpell => misc::effect_trigger_spell(input, world).await,
        HealMaxHealth => healing::effect_heal_max_health(input, world).await,
        InterruptCast => misc::effect_interrupt_cast(input, world).await,
        HealMechanical => healing::effect_heal_mechanical(input, world).await,
        Charge => movement::effect_charge(input, world).await,
        KnockBack => movement::effect_knock_back(input, world).await,
        SpiritHeal => healing::effect_spirit_heal(input, world).await,
        NormalizedWeaponDmg => damage::effect_normalized_weapon_dmg(input, world).await,
        ApplyAreaAuraPet => aura::effect_apply_area_aura_pet(input, world).await,
        ApplyAreaAuraFriend => aura::effect_apply_area_aura_friend(input, world).await,
        ApplyAreaAuraEnemy => aura::effect_apply_area_aura_enemy(input, world).await,
        ApplyAreaAuraRaid => aura::effect_apply_area_aura_raid(input, world).await,
        // Item effects
        CreateItem => item::effect_create_item(input, world).await,
        SummonChangeItem => item::effect_summon_change_item(input, world).await,
        EnchantItem => item::effect_enchant_item_perm(input, world).await,
        EnchantItemTemporary => item::effect_enchant_item_tmp(input, world).await,
        OpenLockItem => item::effect_open_lock_item(input, world).await,
        EnchantHeldItem => item::effect_enchant_held_item(input, world).await,
        Disenchant => item::effect_disenchant(input, world).await,
        FeedPet => item::effect_feed_pet(input, world).await,
        DurabilityDamage => item::effect_durability_damage(input, world).await,
        DurabilityDamagePCT => item::effect_durability_damage_pct(input, world).await,
        // Teleport effects
        TeleportUnits => teleport::effect_teleport_units(input, world).await,
        Bind => teleport::effect_bind(input, world).await,
        TeleportUnitsFaceCaster => teleport::effect_teleport_units_face_caster(input, world).await,
        Stuck => teleport::effect_stuck(input, world).await,
        SummonPlayer => teleport::effect_summon_player(input, world).await,
        SendTaxi => teleport::effect_send_taxi(input, world).await,
        PlayerPull => teleport::effect_player_pull(input, world).await,
        // Profession effects
        SkillStep => profession::effect_skill_step(input, world).await,
        TradeSkill => profession::effect_trade_skill(input, world).await,
        Proficiency => profession::effect_proficiency(input, world).await,
        Skinning => profession::effect_skinning(input, world).await,
        SkinPlayerCorpse => profession::effect_skin_player_corpse(input, world).await,
        Skill => profession::effect_skill(input, world).await,
        // Quest effects
        QuestComplete => quest::effect_quest_complete(input, world).await,
        AddHonor => quest::effect_add_honor(input, world).await,
        Reputation => quest::effect_reputation(input, world).await,
        // Combat effects
        AddExtraAttacks => combat::effect_add_extra_attacks(input, world).await,
        Dodge => combat::effect_dodge(input, world).await,
        Parry => combat::effect_parry(input, world).await,
        Block => combat::effect_block(input, world).await,
        DualWield => combat::effect_dual_wield(input, world).await,
        Threat => combat::effect_threat(input, world).await,
        Distract => combat::effect_distract(input, world).await,
        Sanctuary => combat::effect_sanctuary(input, world).await,
        AddComboPoints => combat::effect_add_combo_points(input, world).await,
        AttackMe => combat::effect_attack_me(input, world).await,
        ModifyThreatPercent => combat::effect_modify_threat_percent(input, world).await,
        // Pet effects
        LearnPetSpell => pet::effect_learn_pet_spell(input, world).await,
        DismissPet => pet::effect_dismiss_pet(input, world).await,
        DestroyAllTotems => pet::effect_destroy_all_totems(input, world).await,
        // Script effects
        TriggerMissile => script::effect_trigger_missile(input, world).await,
        SendEvent => script::effect_send_event(input, world).await,
        ScriptEffect => script::effect_script_effect(input, world).await,
        Nostalrius => script::effect_nostalrius(input, world).await,
        // Dispel effects
        DispelMechanic => dispel::effect_dispel_mechanic(input, world).await,
        // PvP effects
        Duel => pvp::effect_duel(input, world).await,
        Inebriate => pvp::effect_inebriate(input, world).await,
        // Object effects
        OpenLock => object::effect_open_lock(input, world).await,
        TransDoor => object::effect_trans_door(input, world).await,
        SummonObjectWild => object::effect_summon_object_wild(input, world).await,
        ActivateObject => object::effect_activate_object(input, world).await,
        SummonObjectSlot1 => object::effect_summon_object_slot1(input, world).await,
        SummonObjectSlot2 => object::effect_summon_object_slot2(input, world).await,
        SummonObjectSlot3 => object::effect_summon_object_slot3(input, world).await,
        SummonObjectSlot4 => object::effect_summon_object_slot4(input, world).await,
        DespawnObject => object::effect_despawn_object(input, world).await,
        // Resurrection effects
        Resurrect => resurrect::effect_resurrect(input, world).await,
        SelfResurrect => resurrect::effect_self_resurrect(input, world).await,
        ResurrectNew => resurrect::effect_resurrect_new(input, world).await,
        // Misc effects
        Language => misc::effect_language(input, world).await,
        Spawn => misc::effect_spawn(input, world).await,
        CreateHouse => misc::effect_create_house(input, world).await,
        // Movement effects
        Leap => movement::effect_leap(input, world).await,
        Pull => movement::effect_pull(input, world).await,
        Charge2 => movement::effect_charge(input, world).await, // Same as Charge
        // --- Stub effects for MaNGOS parity ---
        SummonType | SummonPet | SummonDeadPet | TameCreature => {
            tracing::debug!(
                "[SPELL-EFFECT] Unimplemented summon effect {:?} for spell {}",
                effect_type,
                input.spell_id
            );
            Ok(EffectResult::empty())
        }
        HealPct => {
            tracing::debug!("[SPELL-EFFECT] HealPct stub for spell {}", input.spell_id);
            Ok(EffectResult::empty())
        }
        EnergizePct => {
            tracing::debug!(
                "[SPELL-EFFECT] EnergizePct stub for spell {}",
                input.spell_id
            );
            Ok(EffectResult::empty())
        }
        TriggerSpellWithValue => misc::effect_trigger_spell(input, world).await, // Same as TriggerSpell for now
        Taunt => combat::effect_attack_me(input, world).await, // Taunt uses same logic as AttackMe
        QuestStart | QuestFail | KillCreditPersonal | KillCreditGroup => {
            tracing::debug!(
                "[SPELL-EFFECT] Unimplemented quest effect {:?} for spell {}",
                effect_type,
                input.spell_id
            );
            Ok(EffectResult::empty())
        }
        PickPocket | AddFarsight | StealBeneficialBuff | ForceCast | Stealth | RemoveAura => {
            tracing::debug!(
                "[SPELL-EFFECT] Unimplemented effect {:?} for spell {}",
                effect_type,
                input.spell_id
            );
            Ok(EffectResult::empty())
        }
    }
}
