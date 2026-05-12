--[[
    @script_type: creature_ai
    @entry: 3975
    @name: boss_herod

    Herod - Scarlet Monastery (Armory)
    Ported from MaNGOS boss_herod.cpp

    Abilities:
    - Rushing Charge: Cast on aggro
    - Cleave: Periodic melee cleave
    - Whirlwind: Periodic with yell
    - Frenzy: Enrage at 30% HP (one-time)
    - On death: Spawns Scarlet Trainees
]]

local SPELL_RUSHING_CHARGE  = 8260
local SPELL_CLEAVE          = 15496
local SPELL_WHIRLWIND       = 8989
local SPELL_FRENZY          = 8269

local NPC_SCARLET_TRAINEE   = 6575

local TIMER_CLEAVE          = 1
local TIMER_WHIRLWIND       = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "Blades of Light!" },
        { action = "CAST_SPELL", spell_id = SPELL_RUSHING_CHARGE, target = "current_target" },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 45000 },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 45000 },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
    }
end

function boss:OnKill(input)
    return {
        { action = "SAY", text = "Ha! Is that all?" },
    }
end

function boss:OnDeath(input)
    return {
        { action = "SPAWN_CREATURE", entry = NPC_SCARLET_TRAINEE, x = 1939.18, y = -431.58, z = 17.09, count = 20 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 30% HP (one-time)
    local enraged = input:GetCustomData("enraged") or 0
    if enraged == 0 and input.health_pct <= 0.30 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "YELL", text = "Light, give me strength!" })
        table.insert(actions, { action = "EMOTE", text = "%s becomes enraged!" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "enraged", value = 1 })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = 12000 })
    end

    -- Whirlwind
    if input:IsTimerReady(TIMER_WHIRLWIND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_WHIRLWIND, target = "current_target" })
        table.insert(actions, { action = "YELL", text = "Whirlwind!" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WHIRLWIND, duration = 30000 })
    end

    return actions
end

RegisterCreatureAI(3975, boss)
