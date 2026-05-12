--[[
    @script_type: creature_ai
    @entry: 10430
    @name: boss_the_beast

    Boss The Beast - Upper Blackrock Spire
    Ported from MaNGOS boss_the_beast.cpp
    Note: Finkle Einhorn skinning summon is omitted (SpellHit on skinning effect).
]]

local SPELL_FLAMEBREAK       = 16785
local SPELL_TERRIFYING_ROAR  = 14100
local SPELL_BERSERKER_CHARGE = 16636
local SPELL_FIREBALL         = 16788
local SPELL_FIRE_BLAST       = 14144
local AURA_IMMOLATE          = 15506

local TIMER_FLAMEBREAK       = 1
local TIMER_TERRIFYING_ROAR  = 2
local TIMER_BERSERKER_CHARGE = 3
local TIMER_FIREBALL         = 4
local TIMER_FIRE_BLAST       = 5
local TIMER_IMMOLATE_CHECK   = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBREAK, duration = math.random(8000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFYING_ROAR, duration = 13000 },
        { action = "SET_TIMER", timer_id = TIMER_BERSERKER_CHARGE, duration = 1 },
        { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_FIRE_BLAST, duration = math.random(8000, 11000) },
        { action = "SET_TIMER", timer_id = TIMER_IMMOLATE_CHECK, duration = 3000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FLAMEBREAK, duration = math.random(8000, 12000) },
        { action = "SET_TIMER", timer_id = TIMER_TERRIFYING_ROAR, duration = 13000 },
        { action = "SET_TIMER", timer_id = TIMER_BERSERKER_CHARGE, duration = 1 },
        { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_FIRE_BLAST, duration = math.random(8000, 11000) },
        { action = "SET_TIMER", timer_id = TIMER_IMMOLATE_CHECK, duration = 3000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}

    -- Keep Immolate aura up (check periodically)
    if input:IsTimerReady(TIMER_IMMOLATE_CHECK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = AURA_IMMOLATE, target = "self", triggered = true })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_IMMOLATE_CHECK, duration = 5000 })
    end

    if not input.is_in_combat then return actions end

    -- Flamebreak (self-cast AoE)
    if input:IsTimerReady(TIMER_FLAMEBREAK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAMEBREAK, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAMEBREAK, duration = math.random(14000, 20000) })
    end

    -- Terrifying Roar (self-cast AoE fear)
    if input:IsTimerReady(TIMER_TERRIFYING_ROAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TERRIFYING_ROAR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TERRIFYING_ROAR, duration = math.random(16000, 18000) })
    end

    -- Berserker Charge on random hostile
    if input:IsTimerReady(TIMER_BERSERKER_CHARGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BERSERKER_CHARGE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BERSERKER_CHARGE, duration = math.random(15000, 20000) })
    end

    -- Fireball on random hostile
    if input:IsTimerReady(TIMER_FIREBALL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBALL, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = math.random(10000, 12000) })
    end

    -- Fire Blast on current target
    if input:IsTimerReady(TIMER_FIRE_BLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRE_BLAST, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIRE_BLAST, duration = math.random(14000, 20000) })
    end

    return actions
end

RegisterCreatureAI(10430, boss)
