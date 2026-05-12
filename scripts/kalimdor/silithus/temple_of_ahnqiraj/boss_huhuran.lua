--[[
    @script_type: creature_ai
    @entry: 15509
    @name: boss_huhuran

    Princess Huhuran - Temple of Ahn'Qiraj
    Ported from MaNGOS boss_huhuran.cpp

    Abilities:
    - Frenzy: Periodic frenzy with Poison Bolt, lasts 15 seconds
    - Wyvern Sting: Random target
    - Acid Spit: Current target
    - Noxious Poison: Current target
    - Berserk: Permanent frenzy at 30% HP with Poison Bolt
]]

local SPELL_FRENZY          = 26051
local SPELL_BERSERK         = 26068
local SPELL_POISONBOLT      = 26052
local SPELL_NOXIOUSPOISON   = 26053
local SPELL_WYVERNSTING     = 26180
local SPELL_ACIDSPIT        = 26050

local TIMER_FRENZY          = 1
local TIMER_WYVERN          = 2
local TIMER_SPIT            = 3
local TIMER_POISON_BOLT     = 4
local TIMER_NOXIOUS_POISON  = 5
local TIMER_FRENZY_END      = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = math.random(25000, 35000) },
        { action = "SET_TIMER", timer_id = TIMER_WYVERN, duration = math.random(18000, 28000) },
        { action = "SET_TIMER", timer_id = TIMER_SPIT, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_POISON_BOLT, duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_POISON, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_FRENZY_END, duration = 15000 },
        { action = "SET_CUSTOM_DATA", key = "frenzy", value = 0 },
        { action = "SET_CUSTOM_DATA", key = "berserk", value = 0 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    local frenzy = input:GetCustomData("frenzy") or 0
    local berserk = input:GetCustomData("berserk") or 0

    -- Frenzy (periodic, lasts 15s)
    if frenzy == 0 and input:IsTimerReady(TIMER_FRENZY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "frenzy", value = 1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_BOLT, duration = 3000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY_END, duration = 15000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FRENZY, duration = math.random(25000, 35000) })
    end

    -- Wyvern Sting
    if input:IsTimerReady(TIMER_WYVERN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WYVERNSTING, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WYVERN, duration = math.random(15000, 32000) })
    end

    -- Acid Spit
    if input:IsTimerReady(TIMER_SPIT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ACIDSPIT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPIT, duration = math.random(5000, 10000) })
    end

    -- Noxious Poison
    if input:IsTimerReady(TIMER_NOXIOUS_POISON) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_NOXIOUSPOISON, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_POISON, duration = math.random(12000, 24000) })
    end

    -- Poison Bolt (only during frenzy or berserk)
    if (frenzy == 1 or berserk == 1) and input:IsTimerReady(TIMER_POISON_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POISONBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POISON_BOLT, duration = 3000 })
    end

    -- Frenzy ends after 15s
    if frenzy == 1 and input:IsTimerReady(TIMER_FRENZY_END) then
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_FRENZY })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "frenzy", value = 0 })
    end

    -- Berserk at 30% HP (permanent)
    if berserk == 0 and input.health_pct <= 0.31 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERK, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s goes into a berserker rage!" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "berserk", value = 1 })
    end

    return actions
end

RegisterCreatureAI(15509, boss)
