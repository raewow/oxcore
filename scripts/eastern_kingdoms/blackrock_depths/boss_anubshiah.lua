--[[
    @script_type: creature_ai
    @entry: 9031
    @name: boss_anubshiah

    Boss Anub'shiah - Blackrock Depths (Ring of Law)
    Ported from MaNGOS boss_anubshiah.cpp
]]

local SPELL_SHADOW_BOLT       = 15472
local SPELL_CURSE_OF_TONGUES  = 15470
local SPELL_CURSE_OF_WEAKNESS = 12493
local SPELL_DEMON_ARMOR       = 13787
local SPELL_ENVELOPING_WEB    = 15471

local TIMER_SHADOW_BOLT       = 1
local TIMER_CURSE_OF_TONGUES  = 2
local TIMER_CURSE_OF_WEAKNESS = 3
local TIMER_DEMON_ARMOR       = 4
local TIMER_ENVELOPING_WEB    = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_TONGUES, duration = 24000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_WEAKNESS, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_DEMON_ARMOR, duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_ENVELOPING_WEB, duration = 16000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_TONGUES, duration = 24000 },
        { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_WEAKNESS, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_DEMON_ARMOR, duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_ENVELOPING_WEB, duration = 16000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Bolt on current target
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = 7000 })
    end

    -- Curse of Tongues on random hostile
    if input:IsTimerReady(TIMER_CURSE_OF_TONGUES) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_TONGUES, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_TONGUES, duration = 18000 })
    end

    -- Curse of Weakness on current target
    if input:IsTimerReady(TIMER_CURSE_OF_WEAKNESS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_WEAKNESS, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_WEAKNESS, duration = 45000 })
    end

    -- Demon Armor (self-cast)
    if input:IsTimerReady(TIMER_DEMON_ARMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMON_ARMOR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEMON_ARMOR, duration = 300000 })
    end

    -- Enveloping Web on random hostile
    if input:IsTimerReady(TIMER_ENVELOPING_WEB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENVELOPING_WEB, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ENVELOPING_WEB, duration = 12000 })
    end

    return actions
end

RegisterCreatureAI(9031, boss)
