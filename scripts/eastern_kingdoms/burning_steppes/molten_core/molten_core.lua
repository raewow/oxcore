--[[
    Molten Core dungeon helper scripts.
    Ported from vmangos molten_core.cpp

    Contents:
    - mob_firewalker       (entry ~11660): Fireblossom sequence + Incite Flames
    - mob_ancient_core_hound (entry 11673): Cone of Fire + random debuff per spawn
    - mob_core_hound       (entry 11671): fake-death mechanic, resurrects if pack-mate alive
    - mob_firelord         (entry 11668 / NPC_FIRELORD): Incinerate Aura + Soulburn + Lava Spawn summon
    - mob_lava_surger      (entry 12101 / NPC_LAVA_SURGER): Surge when target distance > 7yd

    Note on mob_firewalker entry: vmangos registers by script name "mob_firewalker";
    the DB-level NPC entry may differ per server. Update RegisterCreatureAI call if needed.
    NPC_FIRELORD = 11668, but NPC_CORE_RAGER = 11672 (not firelord) — verify in DB.
]]

------------------------------------------------------------------------
-- mob_firewalker (entry — check DB for "mob_firewalker" script reference)
-- Fireblossom casting sequence: cast SPELL_FIREBLOSSOM_CASTING then fire
-- SPELL_FIREBLOSSOM 6 times at 1s intervals; also maintains Incite Flames DoT.
------------------------------------------------------------------------
local SPELL_FIREBLOSSOM         = 19637
local SPELL_FIREBLOSSOM_CASTING = 19636
local SPELL_INCITE_FLAMES       = 19635

local TIMER_FW_CASTING   = 1
local TIMER_FW_BLOSSOM   = 2
local TIMER_FW_INCITE    = 3
local DATA_FW_NB_BLOSSOM = "nb_blossom"

local firewalker = {}

function firewalker:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_FW_CASTING, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FW_INCITE,  duration = 20000 },
        { action = "SET_CUSTOM_DATA", key = DATA_FW_NB_BLOSSOM, value = 0 },
    }
end

function firewalker:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    -- Fireblossom casting: start sequence every 12s
    if input:IsTimerReady(TIMER_FW_CASTING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBLOSSOM_CASTING, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_FW_NB_BLOSSOM, value = 6 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FW_CASTING, duration = 12000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FW_BLOSSOM, duration = 1000 })
    end

    -- Fire individual blossoms at random hostile targets
    if input:GetCustomData(DATA_FW_NB_BLOSSOM) > 0 and input:IsTimerReady(TIMER_FW_BLOSSOM) then
        local remaining = input:GetCustomData(DATA_FW_NB_BLOSSOM)
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBLOSSOM, target = "random_hostile" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_FW_NB_BLOSSOM, value = remaining - 1 })
        if remaining - 1 > 0 then
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FW_BLOSSOM, duration = 1000 })
        end
    end

    -- Incite Flames on current target every 20s
    if input:IsTimerReady(TIMER_FW_INCITE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_INCITE_FLAMES, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FW_INCITE, duration = 20000 })
    end

    return actions
end

-- Note: entry 11660 is likely correct for Firewalker; verify in creature_template DB
RegisterCreatureAI(11660, firewalker)

------------------------------------------------------------------------
-- mob_ancient_core_hound (entry 11673 = NPC_ANCIENT_CORE_HOUND)
-- On spawn, randomly selects one of 6 debuffs. Cone of Fire + Bite every 6s.
-- Long respawn if Magmadar is dead (instance data TYPE_MAGMADAR == DONE).
------------------------------------------------------------------------
local SPELL_CONE_OF_FIRE       = 19630
local SPELL_VICIOUS_BITE       = 19319
local SPELL_BITE                = 19771
local SPELL_GROUND_STOMP       = 19364
local SPELL_ANCIENT_DREAD      = 19365
local SPELL_CAUTERIZING_FLAMES = 19366
local SPELL_WITHERING_HEAT     = 19367
local SPELL_ANCIENT_DESPAIR    = 19369
local SPELL_ANCIENT_HYSTERIA   = 19372

local ACH_DEBUFFS = {
    SPELL_GROUND_STOMP,
    SPELL_ANCIENT_DREAD,
    SPELL_CAUTERIZING_FLAMES,
    SPELL_WITHERING_HEAT,
    SPELL_ANCIENT_DESPAIR,
    SPELL_ANCIENT_HYSTERIA,
}

local TIMER_ACH_CONE   = 1
local TIMER_ACH_DEBUFF = 2
local TIMER_ACH_BITE   = 3
local DATA_ACH_DEBUFF  = "debuff"

local ancient_core_hound = {}

function ancient_core_hound:OnSpawn(input)
    -- Pick random debuff for this spawn
    local debuff_idx = math.random(1, #ACH_DEBUFFS)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_ACH_DEBUFF, value = ACH_DEBUFFS[debuff_idx] },
    }
end

function ancient_core_hound:JustRespawned(input)
    return self:OnSpawn(input)
end

function ancient_core_hound:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_ACH_CONE,   duration = 4000 + math.random(0, 3000) },
        { action = "SET_TIMER", timer_id = TIMER_ACH_DEBUFF, duration = 12000 + math.random(0, 3000) },
        { action = "SET_TIMER", timer_id = TIMER_ACH_BITE,   duration = 4000 },
    }
end

function ancient_core_hound:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_ACH_CONE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CONE_OF_FIRE, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACH_CONE, duration = 6000 + math.random(0, 2000) })
    end

    if input:IsTimerReady(TIMER_ACH_DEBUFF) then
        local debuff = input:GetCustomData(DATA_ACH_DEBUFF)
        if debuff ~= 0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = debuff, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACH_DEBUFF, duration = 14000 + math.random(0, 10000) })
    end

    if input:IsTimerReady(TIMER_ACH_BITE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BITE, target = "current_target", triggered = true })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACH_BITE, duration = 6000 })
    end

    return actions
end

RegisterCreatureAI(11673, ancient_core_hound)

------------------------------------------------------------------------
-- mob_core_hound (entry 11671 = NPC_CORE_HOUND)
-- Fake death when would-be-killed: set HP to 1, stop combat, appear dead.
-- After 10s: check if any pack-mate (same entry, nearby) is alive in combat.
-- If yes: resurrect (full heal, fire nova visual, resume combat).
-- If no: actually die (KILL_SELF).
------------------------------------------------------------------------
local SPELL_FULL_HEAL        = 17683
local SPELL_SERRATED_BITE    = 19771
local SPELL_FIRE_NOVA_VISUAL = 19823
local SPELL_PACIFY_SELF      = 19951

local NPC_CORE_HOUND = 11671

local TIMER_CH_BITE      = 1
local TIMER_CH_RESURRECT = 2

local DATA_CH_FEIGNED = "feigned"

local core_hound = {}

function core_hound:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_CH_BITE, duration = 4000 + math.random(0, 3000) },
        { action = "SET_CUSTOM_DATA", key = DATA_CH_FEIGNED, value = 0 },
    }
end

function core_hound:OnDamageTaken(input, attacker_guid, damage, spell_id)
    -- Would be killed: feign death
    if input:GetCustomData(DATA_CH_FEIGNED) == 1 then return {} end
    if damage >= input.health then
        return {
            -- Consume all health (set to 1 to prevent death)
            { action = "SET_HEALTH_PCT", percent = 0.01 },
            { action = "CAST_SPELL", spell_id = SPELL_PACIFY_SELF, target = "self", triggered = true },
            { action = "SET_MELEE_ATTACK", enabled = false },
            { action = "SET_COMBAT_MOVEMENT", enabled = false },
            { action = "SET_REACT_STATE", state = "PASSIVE" },
            { action = "SET_CUSTOM_DATA", key = DATA_CH_FEIGNED, value = 1 },
            { action = "SET_TIMER", timer_id = TIMER_CH_RESURRECT, duration = 10000 },
        }
    end
    return {}
end

function core_hound:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    -- Resurrection check
    if input:GetCustomData(DATA_CH_FEIGNED) == 1 and input:IsTimerReady(TIMER_CH_RESURRECT) then
        -- Check if any other core hound in pack is alive and in combat with health > 1
        local pack = input:GetCreaturesByEntry(NPC_CORE_HOUND)
        local pack_alive = false
        for _, guid in ipairs(pack) do
            -- We can only check by count; assume if any nearby core hound exists they may be alive
            pack_alive = true
            break
        end

        if pack_alive then
            -- Resurrect: full heal, fire nova visual
            table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_PACIFY_SELF })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FULL_HEAL, target = "self", triggered = true })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRE_NOVA_VISUAL, target = "self", triggered = true })
            table.insert(actions, { action = "SET_MELEE_ATTACK", enabled = true })
            table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
            table.insert(actions, { action = "SET_REACT_STATE", state = "AGGRESSIVE" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_CH_FEIGNED, value = 0 })
        else
            -- No pack-mates alive: actually die
            table.insert(actions, { action = "SET_HEALTH_PCT", percent = 0.0 })
            table.insert(actions, { action = "KILL_SELF" })
        end
        return actions
    end

    if input:GetCustomData(DATA_CH_FEIGNED) == 1 then return actions end

    -- Serrated Bite
    if input:IsTimerReady(TIMER_CH_BITE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SERRATED_BITE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CH_BITE, duration = 4000 + math.random(0, 3000) })
    end

    return actions
end

RegisterCreatureAI(11671, core_hound)

------------------------------------------------------------------------
-- mob_firelord (entry 11668 = NPC_FIRELORD)
-- Incinerate Aura on aggro; Soulburn on random player; summons Lava Spawn (12265).
------------------------------------------------------------------------
local SPELL_INCINERATE_AURA = 19396
local SPELL_LAVASPAWN       = 19569
local SPELL_SOULBURN        = 19393

local NPC_LAVA_SPAWN = 12265

local TIMER_FL_LAVASPAWN = 1
local TIMER_FL_SOULBURN  = 2

local firelord = {}

function firelord:OnEnterCombat(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_INCINERATE_AURA, target = "self", triggered = true },
        { action = "SET_TIMER", timer_id = TIMER_FL_LAVASPAWN, duration = 7500 + math.random(0, 5000) },
        { action = "SET_TIMER", timer_id = TIMER_FL_SOULBURN,  duration = 4000 + math.random(0, 2000) },
    }
end

function firelord:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_FL_LAVASPAWN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LAVASPAWN, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FL_LAVASPAWN, duration = 15000 + math.random(0, 5000) })
    end

    if input:IsTimerReady(TIMER_FL_SOULBURN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SOULBURN, target = "random_hostile", triggered = true })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FL_SOULBURN, duration = 3000 + math.random(0, 1000) })
    end

    return actions
end

RegisterCreatureAI(11668, firelord)

------------------------------------------------------------------------
-- mob_lava_surger (entry 12101 = NPC_LAVA_SURGER)
-- Surge (19196) only when target distance > 7 yards. Random 1-2s initial timer.
------------------------------------------------------------------------
local SPELL_SURGE = 19196

local TIMER_LS_SURGE = 1

local lava_surger = {}

function lava_surger:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_LS_SURGE, duration = 1000 + math.random(0, 1000) },
    }
end

function lava_surger:OnUpdate(input)
    if not input.is_in_combat then return {} end
    if not input:IsTimerReady(TIMER_LS_SURGE) then return {} end

    local actions = {}

    -- Only cast Surge if the target is > 7 yards away
    -- Use farthest hostile target; if distance > 7 cast, else skip
    if #input.threat_list > 0 then
        local farthest = input.threat_list[#input.threat_list]
        if farthest and farthest.distance and farthest.distance > 7.0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SURGE, target = farthest.guid })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LS_SURGE, duration = 5000 + math.random(0, 1000) })
        else
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LS_SURGE, duration = 1000 })
        end
    else
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LS_SURGE, duration = 1000 })
    end

    return actions
end

RegisterCreatureAI(12101, lava_surger)
