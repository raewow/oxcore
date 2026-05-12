--[[
    @script_type: creature_ai
    @entry: 5709
    @name: boss_shade_of_eranikus

    Shade of Eranikus - Sunken Temple (Temple of Atal'Hakkar)
    No dedicated MaNGOS SD2 boss script exists; created from known Vanilla spell data.

    Mechanics:
    - War Stomp (AoE stun around self)
    - Acid Breath (frontal cone nature damage)
    - Noxious Breath (frontal cone nature damage + DoT)
    - Deep Slumber (sleep on random hostile)
]]

local SPELL_WAR_STOMP      = 11876
local SPELL_ACID_BREATH    = 12884
local SPELL_NOXIOUS_BREATH = 24818
local SPELL_DEEP_SLUMBER   = 12890

local TIMER_WAR_STOMP      = 1
local TIMER_ACID_BREATH    = 2
local TIMER_NOXIOUS_BREATH = 3
local TIMER_DEEP_SLUMBER   = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_ACID_BREATH, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_BREATH, duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_DEEP_SLUMBER, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- War Stomp
    if input:IsTimerReady(TIMER_WAR_STOMP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WAR_STOMP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WAR_STOMP, duration = 18000 })
    end

    -- Acid Breath
    if input:IsTimerReady(TIMER_ACID_BREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ACID_BREATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ACID_BREATH, duration = 12000 })
    end

    -- Noxious Breath
    if input:IsTimerReady(TIMER_NOXIOUS_BREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_NOXIOUS_BREATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_BREATH, duration = 20000 })
    end

    -- Deep Slumber
    if input:IsTimerReady(TIMER_DEEP_SLUMBER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEEP_SLUMBER, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEEP_SLUMBER, duration = 22000 })
    end

    return actions
end

RegisterCreatureAI(5709, boss)
