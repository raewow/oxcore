--[[
    @script_type: creature_ai
    @entry: 15275, 15276
    @name: boss_twinemperors

    Twin Emperors (Vek'nilash & Vek'lor) - Temple of Ahn'Qiraj
    Verified against vmangos boss_twinemperors.cpp

    Vek'nilash (15275) - Melee Emperor:
    - Unbalancing Strike
    - Uppercut
    - Mutate Bug (on nearby bugs)
    - Immune to magic damage

    Vek'lor (15276) - Caster Emperor:
    - Shadow Bolt
    - Blizzard
    - Arcane Burst (when melee in range)
    - Explode Bug (on nearby bugs)
    - Immune to physical damage

    Shared mechanics:
    - Heal Brother when within 60 yards
    - Teleport and swap positions every 30 seconds
    - Shared health (damage to one mirrors to the other)
    - Berserk after 15 minutes
]]

local SPELL_HEAL_BROTHER        = 7393
local SPELL_TWIN_TELEPORT       = 800
local SPELL_TWIN_TELEPORT_VIS   = 26638
local SPELL_BERSERK             = 26662

-- Vek'nilash spells
local SPELL_UPPERCUT            = 26007
local SPELL_UNBALANCING_STRIKE  = 26613
local SPELL_MUTATE_BUG          = 802

-- Vek'lor spells
local SPELL_SHADOWBOLT          = 26006
local SPELL_BLIZZARD            = 26607
local SPELL_ARCANEBURST         = 568
local SPELL_EXPLODEBUG          = 804

-- Vek'nilash timers
local TIMER_VN_UPPERCUT         = 1
local TIMER_VN_UNSTRIKE         = 2
local TIMER_VN_HEAL             = 3
local TIMER_VN_TELEPORT         = 4
local TIMER_VN_ENRAGE           = 5
local TIMER_VN_BUG              = 6

-- Vek'lor timers
local TIMER_VL_SHADOWBOLT       = 1
local TIMER_VL_BLIZZARD         = 2
local TIMER_VL_ARCANEBURST      = 3
local TIMER_VL_HEAL             = 4
local TIMER_VL_TELEPORT         = 5
local TIMER_VL_ENRAGE           = 6
local TIMER_VL_BUG              = 7

--------------------------------------------------------------------------------
-- Vek'nilash (15275) - Melee Emperor
--------------------------------------------------------------------------------
local veknilash = {}

function veknilash:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_VN_UPPERCUT, duration = math.random(14000, 29000) },
        { action = "SET_TIMER", timer_id = TIMER_VN_UNSTRIKE, duration = math.random(8000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_VN_HEAL, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_VN_TELEPORT, duration = math.random(30000, 40000) },
        { action = "SET_TIMER", timer_id = TIMER_VN_ENRAGE, duration = 3600000 },
        { action = "SET_TIMER", timer_id = TIMER_VN_BUG, duration = math.random(10000, 15000) },
    }
end

function veknilash:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Unbalancing Strike
    if input:IsTimerReady(TIMER_VN_UNSTRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UNBALANCING_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_UNSTRIKE, duration = math.random(8000, 20000) })
    end

    -- Uppercut on random melee
    if input:IsTimerReady(TIMER_VN_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_UPPERCUT, duration = math.random(15000, 30000) })
    end

    -- Heal brother when close
    if input:IsTimerReady(TIMER_VN_HEAL) then
        local veklor_list = input:GetCreaturesByEntry(15276)
        if veklor_list then
            for _, creature in ipairs(veklor_list) do
                if creature.is_alive then
                    table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEAL_BROTHER, target = "self" })
                    break
                end
            end
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_HEAL, duration = 1000 })
    end

    -- Teleport (swap positions with brother)
    if input:IsTimerReady(TIMER_VN_TELEPORT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TWIN_TELEPORT_VIS, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TWIN_TELEPORT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_TELEPORT, duration = 30000 })
    end

    -- Mutate Bug
    if input:IsTimerReady(TIMER_VN_BUG) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MUTATE_BUG, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_BUG, duration = math.random(10000, 17000) })
    end

    -- Berserk
    if input:IsTimerReady(TIMER_VN_ENRAGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VN_ENRAGE, duration = 3600000 })
    end

    return actions
end

RegisterCreatureAI(15275, veknilash)

--------------------------------------------------------------------------------
-- Vek'lor (15276) - Caster Emperor
--------------------------------------------------------------------------------
local veklor = {}

function veklor:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_VL_SHADOWBOLT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_VL_BLIZZARD, duration = math.random(15000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_VL_ARCANEBURST, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_VL_HEAL, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_VL_TELEPORT, duration = math.random(30000, 40000) },
        { action = "SET_TIMER", timer_id = TIMER_VL_ENRAGE, duration = 3600000 },
        { action = "SET_TIMER", timer_id = TIMER_VL_BUG, duration = math.random(7000, 10000) },
    }
end

function veklor:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_VL_SHADOWBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_SHADOWBOLT, duration = 2000 })
    end

    -- Blizzard on random target
    if input:IsTimerReady(TIMER_VL_BLIZZARD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLIZZARD, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_BLIZZARD, duration = math.random(15000, 20000) })
    end

    -- Arcane Burst (when melee are close)
    if input:IsTimerReady(TIMER_VL_ARCANEBURST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANEBURST, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_ARCANEBURST, duration = math.random(5000, 10000) })
    end

    -- Teleport (swap positions with brother)
    if input:IsTimerReady(TIMER_VL_TELEPORT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TWIN_TELEPORT_VIS, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TWIN_TELEPORT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_TELEPORT, duration = math.random(30000, 40000) })
    end

    -- Explode Bug
    if input:IsTimerReady(TIMER_VL_BUG) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EXPLODEBUG, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_BUG, duration = math.random(7000, 10000) })
    end

    -- Berserk
    if input:IsTimerReady(TIMER_VL_ENRAGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VL_ENRAGE, duration = 3600000 })
    end

    return actions
end

RegisterCreatureAI(15276, veklor)
