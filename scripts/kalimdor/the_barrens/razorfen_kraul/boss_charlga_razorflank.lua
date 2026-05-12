--[[
    @script_type: creature_ai
    @entry: 4421
    @name: boss_charlga_razorflank

    Charlga Razorflank - Razorfen Kraul
    Based on known abilities (no dedicated SD2 boss script exists)

    Charlga is a caster boss and leader of the Razorfen Kraul quillboar.

    Abilities:
    - Chain Bolt: Nature damage chain lightning (8292)
    - Renew: Heals herself periodically (8362)
    - Healing Ward: Summons a healing ward totem (6278)
    - An Improved version of Heal in later phases
]]

local SPELL_CHAIN_BOLT       = 8292
local SPELL_RENEW            = 8362
local SPELL_HEALING_WARD     = 6278

local TIMER_CHAIN_BOLT       = 1
local TIMER_RENEW            = 2
local TIMER_HEALING_WARD     = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CHAIN_BOLT, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_RENEW, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD, duration = 20000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CHAIN_BOLT, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_RENEW, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD, duration = 20000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Chain Bolt
    if input:IsTimerReady(TIMER_CHAIN_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHAIN_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHAIN_BOLT, duration = math.random(9000, 14000) })
    end

    -- Renew on self when hurt
    if input:IsTimerReady(TIMER_RENEW) then
        if input.health_pct < 0.85 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RENEW, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RENEW, duration = 15000 })
    end

    -- Healing Ward
    if input:IsTimerReady(TIMER_HEALING_WARD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEALING_WARD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD, duration = math.random(25000, 35000) })
    end

    return actions
end

RegisterCreatureAI(4421, boss)
