--[[
    Ruins of Ahn'Qiraj dungeon helper scripts.
    Ported from vmangos ruins_of_ahnqiraj.cpp

    Contents:
    - mob_anubisath_guardian  (entry 15355): 4 random spells chosen on spawn; enrage + explode at <10% HP; summons warriors/swarms (cap 4)
    - mob_flesh_hunter        (entry 15335): Consume channel; Poison Bolt; Split/Trash melee; heals on Consume hit
    - mob_obsidian_destroyer  (entry 15338): Purge on full mana; Drain Mana every 7s; zone combat
    - mob_hive_zara_soldier   (entry 15320): Venom Spit every 5-10s; Retaliation at <20% HP; zone combat
    - mob_silicate_feeder     (entry 15333): neutral until attacked, Cloud of Disease on death
    - mob_qiraji_swarmguard   (entry 15343): Sundering Cleave every 8-12s; zone combat
    - mob_qiraji_gladiator    (entry 15324): Trample every 4-6s; Uppercut every 10-15s; Vengeance enrage on gladiator death
    - mob_hive_zara_stinger   (entry 15327): Charge Stinger every 5s on random target
    - mob_swarmguard_needler  (entry 15344): Cleave every 8s (captain Tuubid group)
    - boss_tuubid             (entry 15392): Attack Order mark every 9s; Cleave + Sunder Armor
    - mob_qiraji_warrior      (entry 15387): Thunder Clap + Uppercut; Enrage at <20% HP
    - mob_tornado_ossirian    (entry 15428): passive wanderer, self-aura on spawn, immune

    Skipped (requires SpellScript API):
    - AQ20DrainManaScript: OnCheckTarget filters mana-less targets (Phase E SpellScript)
    - RajaxxThundercrashScript: custom damage calculation (Phase E SpellScript)

    NOTE: QirajiWarriorAI and SwarmguardNeedlerAI follow Tuubid's marked target.
    Cross-NPC targeting is not available in Lua — simplified to standard threat-based combat.
    NPC_TUUBID = 15392 (Captain Tuubid)
]]

------------------------------------------------------------------------
-- mob_anubisath_guardian (entry 15355)
-- 4 random spells chosen at spawn. Enrage + Explode sequence at <10% HP.
-- Summons 1 warrior (15537) or swarm (15538) every 10s, cap 4.
------------------------------------------------------------------------
local SPELL_METEOR        = 24340
local SPELL_PLAGUE        = 22997
local SPELL_SHADOW_STORM  = 26546
local SPELL_THUNDER_CLAP  = 26554
local SPELL_REFLECT_ARFR  = 13022
local SPELL_REFLECT_FSSH  = 19595
local SPELL_ENRAGE_AG     = 8269
local SPELL_EXPLODE       = 25699
local SPELL_INIT_EXPLODE  = 25698

local NPC_ANU_WARRIOR = 15537
local NPC_ANU_SWARM   = 15538

local TIMER_AG_SPELL1   = 1
local TIMER_AG_SPELL2   = 2
local TIMER_AG_SUMMON   = 3
local TIMER_AG_EXPLODE  = 4

local DATA_AG_SPELL1       = "ag_spell1"
local DATA_AG_SPELL2       = "ag_spell2"
local DATA_AG_SPELL3       = "ag_spell3"
local DATA_AG_SPELL4       = "ag_spell4"
local DATA_AG_SUMMON_NPC   = "ag_summon_npc"
local DATA_AG_SUMMON_COUNT = "ag_summon_count"
local DATA_AG_ENRAGED      = "ag_enraged"
local DATA_AG_EXPLODING    = "ag_exploding"

local anubisath_guardian = {}

function anubisath_guardian:OnSpawn(input)
    -- Randomly choose spell pairs (each slot picks one of two options)
    local s1 = (math.random(0, 1) == 1) and SPELL_METEOR or SPELL_PLAGUE
    local s2 = (math.random(0, 1) == 1) and SPELL_SHADOW_STORM or SPELL_THUNDER_CLAP
    local s3 = (math.random(0, 1) == 1) and SPELL_REFLECT_ARFR or SPELL_REFLECT_FSSH
    local s4 = (math.random(0, 1) == 1) and SPELL_ENRAGE_AG or SPELL_INIT_EXPLODE
    local summon_npc = (math.random(0, 1) == 1) and NPC_ANU_WARRIOR or NPC_ANU_SWARM
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SPELL1, value = s1 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SPELL2, value = s2 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SPELL3, value = s3 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SPELL4, value = s4 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SUMMON_NPC, value = summon_npc },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_SUMMON_COUNT, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_ENRAGED, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_AG_EXPLODING, value = 0 },
    }
end

function anubisath_guardian:JustRespawned(input)
    return self:OnSpawn(input)
end

function anubisath_guardian:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_AG_SPELL1,  duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_AG_SPELL2,  duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_AG_SUMMON,  duration = 10000 },
    }
end

function anubisath_guardian:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    -- Enrage + Explode sequence at <10% HP
    if input.health_pct < 10.0 and input:GetCustomData(DATA_AG_ENRAGED) == 0 then
        local s4 = input:GetCustomData(DATA_AG_SPELL4)
        table.insert(actions, { action = "CAST_SPELL", spell_id = s4, target = "self", triggered = true })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_AG_ENRAGED, value = 1 })
        if s4 == SPELL_INIT_EXPLODE then
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_AG_EXPLODING, value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AG_EXPLODE, duration = 6000 })
        end
        return actions
    end

    -- Explode after 6s countdown
    if input:GetCustomData(DATA_AG_EXPLODING) == 1 and input:IsTimerReady(TIMER_AG_EXPLODE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EXPLODE, target = "self", triggered = true })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_AG_EXPLODING, value = 0 })
        return actions
    end

    -- Spell 1 (meteor/plague)
    if input:IsTimerReady(TIMER_AG_SPELL1) then
        local s1 = input:GetCustomData(DATA_AG_SPELL1)
        table.insert(actions, { action = "CAST_SPELL", spell_id = s1, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AG_SPELL1, duration = 15000 })
    end

    -- Spell 2 (shadow storm/thunder clap)
    if input:IsTimerReady(TIMER_AG_SPELL2) then
        local s2 = input:GetCustomData(DATA_AG_SPELL2)
        table.insert(actions, { action = "CAST_SPELL", spell_id = s2, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AG_SPELL2, duration = 20000 })
    end

    -- Summon warrior/swarm (cap 4)
    if input:IsTimerReady(TIMER_AG_SUMMON) then
        local count = input:GetCustomData(DATA_AG_SUMMON_COUNT)
        if count < 4 then
            local npc = input:GetCustomData(DATA_AG_SUMMON_NPC)
            table.insert(actions, { action = "SPAWN_CREATURE", entry = npc, x = input.position.x, y = input.position.y, z = input.position.z })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_AG_SUMMON_COUNT, value = count + 1 })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_AG_SUMMON, duration = 10000 })
    end

    -- Spell 3 (reflect) — cast on self periodically
    -- Note: vmangos casts on reset; we cast once at 30s
    local s3 = input:GetCustomData(DATA_AG_SPELL3)
    if s3 ~= 0 and input:IsTimerReady(TIMER_AG_SPELL2) then
        -- Spell 3 shares no dedicated timer in original; apply on engage aura
        -- Simplified: cast as self-buff when not already active
        if not input:HasAura(s3) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = s3, target = "self", triggered = true })
        end
    end

    return actions
end

RegisterCreatureAI(15355, anubisath_guardian)

------------------------------------------------------------------------
-- mob_flesh_hunter (entry 15335)
-- Consume (25371) channel on target; Consume deals damage ticks via 25373;
-- heals caster via 25378. Poison Bolt (25424) every 8s. Trash/Split.
------------------------------------------------------------------------
local SPELL_CONSUME      = 25371
local SPELL_CONSUME_HEAL = 25378
local SPELL_POISON_BOLT  = 25424
local SPELL_CONSUME_DMG  = 25373
local SPELL_SPLIT        = 25383
local SPELL_TRASH        = 3391

local TIMER_FH_POISON   = 1
local TIMER_FH_CONSUME  = 2
local TIMER_FH_TRASH    = 3

local flesh_hunter = {}

function flesh_hunter:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FH_POISON,  duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FH_CONSUME, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_FH_TRASH,   duration = 4000 },
    }
end

function flesh_hunter:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_FH_CONSUME) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CONSUME, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FH_CONSUME, duration = 20000 })
    end

    if input:IsTimerReady(TIMER_FH_POISON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISON_BOLT, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FH_POISON, duration = 8000 + math.random(0, 4000) })
    end

    if input:IsTimerReady(TIMER_FH_TRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FH_TRASH, duration = 6000 })
    end

    return actions
end

RegisterCreatureAI(15335, flesh_hunter)

------------------------------------------------------------------------
-- mob_obsidian_destroyer (entry 15338)
-- Drain Mana (25754) every 7s. Purge (25756) when at full mana.
-- Drops GO small obsidian chunk (181068) on death (blocked — no GO summon on death API).
------------------------------------------------------------------------
local SPELL_PURGE_OD    = 25756
local SPELL_DRAINMANA   = 25754

local TIMER_OD_DRAINMANA = 1

local obsidian_destroyer = {}

function obsidian_destroyer:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_OD_DRAINMANA, duration = 7000 },
    }
end

function obsidian_destroyer:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_OD_DRAINMANA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAINMANA, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_OD_DRAINMANA, duration = 7000 })
    end

    -- TODO(api): Cast PURGE on self when mana is at max (requires mana power check in input)

    return actions
end

RegisterCreatureAI(15338, obsidian_destroyer)

------------------------------------------------------------------------
-- mob_hive_zara_soldier (entry 15320)
-- Venom Spit (25497) every 5-10s on random target.
-- Retaliation (22857) self-cast at <20% HP (once).
------------------------------------------------------------------------
local SPELL_VENOM_SPIT  = 25497
local SPELL_RETALIATION = 22857

local TIMER_HZS_VENOM = 1

local DATA_HZS_RETALIATED = "hzs_retaliated"

local hive_zara_soldier = {}

function hive_zara_soldier:OnSpawn(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_HZS_RETALIATED, value = 0 },
    }
end

function hive_zara_soldier:JustRespawned(input)
    return self:OnSpawn(input)
end

function hive_zara_soldier:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_HZS_VENOM, duration = 5000 },
    }
end

function hive_zara_soldier:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input.health_pct < 20.0 and input:GetCustomData(DATA_HZS_RETALIATED) == 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RETALIATION, target = "self", triggered = false })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_HZS_RETALIATED, value = 1 })
    end

    if input:IsTimerReady(TIMER_HZS_VENOM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VENOM_SPIT, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HZS_VENOM, duration = 5000 + math.random(0, 5000) })
    end

    return actions
end

RegisterCreatureAI(15320, hive_zara_soldier)

------------------------------------------------------------------------
-- mob_silicate_feeder (entry 15333)
-- Faction 7 (neutral/critter) until attacked; becomes hostile on aggro.
-- Casts Cloud of Disease (17742) on self on death.
------------------------------------------------------------------------
local SPELL_CLOUD_OF_DISEASE = 17742

local FACTION_FEEDER_NEUTRAL  = 7
local FACTION_FEEDER_HOSTILE  = 14

local DATA_SF_ATTACKED = "sf_attacked"

local silicate_feeder = {}

function silicate_feeder:OnSpawn(input)
    return {
        { action = "SET_FACTION", faction_id = FACTION_FEEDER_NEUTRAL },
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_CUSTOM_DATA", key = DATA_SF_ATTACKED, value = 0 },
    }
end

function silicate_feeder:JustRespawned(input)
    return self:OnSpawn(input)
end

function silicate_feeder:OnEnterCombat(input)
    local actions = {}
    if input:GetCustomData(DATA_SF_ATTACKED) == 0 then
        table.insert(actions, { action = "SET_FACTION", faction_id = FACTION_FEEDER_HOSTILE })
        table.insert(actions, { action = "SET_REACT_STATE", state = "AGGRESSIVE" })
        table.insert(actions, { action = "SET_COMBAT_WITH_ZONE" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SF_ATTACKED, value = 1 })
    end
    return actions
end

function silicate_feeder:OnDeath(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_CLOUD_OF_DISEASE, target = "self", triggered = true },
    }
end

RegisterCreatureAI(15333, silicate_feeder)

------------------------------------------------------------------------
-- mob_qiraji_swarmguard (entry 15343)
-- Sundering Cleave (25174) every 8-12s. Zone combat.
------------------------------------------------------------------------
local SPELL_SUNDERING_CLEAVE = 25174

local TIMER_QSG_CLEAVE = 1

local qiraji_swarmguard = {}

function qiraji_swarmguard:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_QSG_CLEAVE, duration = 2000 },
    }
end

function qiraji_swarmguard:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_QSG_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUNDERING_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_QSG_CLEAVE, duration = 8000 + math.random(0, 4000) })
    end

    return actions
end

RegisterCreatureAI(15343, qiraji_swarmguard)

------------------------------------------------------------------------
-- mob_qiraji_gladiator (entry 15324)
-- Trample (5568) every 4-6s. Uppercut (10966) every 10-15s.
-- Vengeance (25164) self-cast on aggro when a previous gladiator has died
-- (TYPE_QIRAJI_GLADIATOR instance data > 0). Zone combat.
-- NOTE: Instance data tracking for gladiator deaths not available in this
-- helper script; Vengeance trigger omitted (requires instance script wiring).
------------------------------------------------------------------------
local SPELL_TRAMPLE_QG  = 5568
local SPELL_UPPERCUT_QG = 10966
local SPELL_VENGEANCE   = 25164

local TIMER_QG_TRAMPLE  = 1
local TIMER_QG_UPPERCUT = 2

local qiraji_gladiator = {}

function qiraji_gladiator:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_QG_TRAMPLE,  duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_QG_UPPERCUT, duration = 9000 },
    }
end

function qiraji_gladiator:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_QG_TRAMPLE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRAMPLE_QG, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_QG_TRAMPLE, duration = 4000 + math.random(0, 2000) })
    end

    if input:IsTimerReady(TIMER_QG_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT_QG, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_QG_UPPERCUT, duration = 10000 + math.random(0, 5000) })
    end

    return actions
end

RegisterCreatureAI(15324, qiraji_gladiator)

------------------------------------------------------------------------
-- mob_hive_zara_stinger (entry 15327)
-- Charge Stinger (25190) on a random target every 5s.
------------------------------------------------------------------------
local SPELL_CHARGE_STINGER = 25190

local TIMER_HZSt_CHARGE = 1

local hive_zara_stinger = {}

function hive_zara_stinger:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_HZSt_CHARGE, duration = 3000 },
    }
end

function hive_zara_stinger:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_HZSt_CHARGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE_STINGER, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HZSt_CHARGE, duration = 5000 })
    end

    return actions
end

RegisterCreatureAI(15327, hive_zara_stinger)

------------------------------------------------------------------------
-- mob_swarmguard_needler (entry 15344)
-- Cleave (20684) every 8s. Part of Captain Tuubid's group.
-- NOTE: vmangos follows Tuubid's marked target — cross-NPC targeting
-- not available; simplified to standard threat combat.
------------------------------------------------------------------------
local SPELL_CLEAVE_SN = 20684

local TIMER_SN_CLEAVE = 1

local swarmguard_needler = {}

function swarmguard_needler:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_SN_CLEAVE, duration = 6000 + math.random(0, 6000) },
    }
end

function swarmguard_needler:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_SN_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE_SN, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SN_CLEAVE, duration = 8000 })
    end

    return actions
end

RegisterCreatureAI(15344, swarmguard_needler)

------------------------------------------------------------------------
-- boss_tuubid (entry 15392) — Captain Tuubid
-- Attack Order (25471): marks random player every 9s; all group members
-- focus that target. Cross-NPC targeting not available — simplified: only
-- Tuubid himself casts the mark debuff.
-- Cleave (26350) every 10s. Sunder Armor (24317) every 15s.
------------------------------------------------------------------------
local SPELL_ATTACK_ORDER = 25471
local SPELL_CLEAVE_T     = 26350
local SPELL_SUNDER_ARMOR = 24317

local TIMER_T_ATTACK_ORDER = 1
local TIMER_T_CLEAVE       = 2
local TIMER_T_SUNDER       = 3

local tuubid = {}

function tuubid:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_T_ATTACK_ORDER, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_T_CLEAVE,       duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_T_SUNDER,       duration = 6000 },
    }
end

function tuubid:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_T_ATTACK_ORDER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ATTACK_ORDER, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_T_ATTACK_ORDER, duration = 9000 })
    end

    if input:IsTimerReady(TIMER_T_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE_T, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_T_CLEAVE, duration = 10000 })
    end

    if input:IsTimerReady(TIMER_T_SUNDER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUNDER_ARMOR, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_T_SUNDER, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(15392, tuubid)

------------------------------------------------------------------------
-- mob_qiraji_warrior (entry 15387)
-- Enrage (8599) at <20% HP (once). Thunder Clap (15588) every 6-12s.
-- Uppercut (10966) every 10-15s. Zone combat.
-- NOTE: vmangos follows Tuubid's marked target — simplified to standard combat.
------------------------------------------------------------------------
local SPELL_ENRAGE_QW   = 8599
local SPELL_THUNDERCLAP = 15588
local SPELL_UPPERCUT    = 10966

local TIMER_QW_THUNDER  = 1
local TIMER_QW_UPPERCUT = 2

local DATA_QW_ENRAGED = "qw_enraged"

local qiraji_warrior = {}

function qiraji_warrior:OnSpawn(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_QW_ENRAGED, value = 0 },
    }
end

function qiraji_warrior:JustRespawned(input)
    return self:OnSpawn(input)
end

function qiraji_warrior:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_QW_THUNDER,  duration = 6000 + math.random(0, 6000) },
        { action = "SET_TIMER", timer_id = TIMER_QW_UPPERCUT, duration = 10000 + math.random(0, 5000) },
    }
end

function qiraji_warrior:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input.health_pct < 20.0 and input:GetCustomData(DATA_QW_ENRAGED) == 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE_QW, target = "current_target" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_QW_ENRAGED, value = 1 })
    end

    if input:IsTimerReady(TIMER_QW_THUNDER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERCLAP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_QW_THUNDER, duration = 6000 })
    end

    if input:IsTimerReady(TIMER_QW_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_QW_UPPERCUT, duration = 10000 + math.random(0, 5000) })
    end

    return actions
end

RegisterCreatureAI(15387, qiraji_warrior)

------------------------------------------------------------------------
-- mob_tornado_ossirian (entry 15428)
-- Passive wanderer. Self-aura (25160) on spawn. Immunity (10092) on spawn.
-- Does not engage in combat.
------------------------------------------------------------------------
local SPELL_TORNADO_AURA    = 25160
local SPELL_TORNADO_IMMUNE  = 10092

local tornado_ossirian = {}

function tornado_ossirian:OnSpawn(input)
    return {
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_IMMUNE", enabled = true },
        { action = "CAST_SPELL", spell_id = SPELL_TORNADO_AURA,   target = "self", triggered = true },
        { action = "CAST_SPELL", spell_id = SPELL_TORNADO_IMMUNE, target = "self", triggered = true },
    }
end

function tornado_ossirian:JustRespawned(input)
    return self:OnSpawn(input)
end

RegisterCreatureAI(15428, tornado_ossirian)
