--[[
    @script_type: creature_ai
    @entry: 14516
    @name: boss_death_knight_darkreaver

    Death Knight Darkreaver - Scholomance
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    A Death Knight boss in Scholomance associated with the paladin
    "Charger" epic mount quest (quest 7631). Players must obtain his
    Corrupted Soul Shard from this fight.

    Mechanics:
    - Shadow Bolt Volley: periodic AoE shadow bolts
    - Frostbolt: periodic frost nuke
    - Unholy Ground: periodic AoE shadow damage aura
    - Mortal Strike: heavy melee strike
    - Frenzy enrage at 20% HP
]]

local SPELL_SHADOW_BOLT_VOLLEY  = 17228
local SPELL_FROSTBOLT           = 8406
local SPELL_UNHOLY_GROUND       = 17197
local SPELL_MORTAL_STRIKE       = 16856
local SPELL_FRENZY              = 8269

local TIMER_SHADOW_BOLT_VOLLEY  = 1
local TIMER_FROSTBOLT           = 2
local TIMER_UNHOLY_GROUND       = 3
local TIMER_MORTAL_STRIKE       = 4

local PHASE_NORMAL              = 0
local PHASE_ENRAGED             = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_VOLLEY, duration = math.random(5000, 10000)  },
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT,          duration = math.random(4000, 8000)   },
        { action = "SET_TIMER", timer_id = TIMER_UNHOLY_GROUND,      duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE,      duration = math.random(8000, 12000)  },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 20% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.20 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY, target = "self" })
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Shadow Bolt Volley
    if input:IsTimerReady(TIMER_SHADOW_BOLT_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT_VOLLEY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_VOLLEY, duration = math.random(8000, 15000) })
    end

    -- Frostbolt
    if input:IsTimerReady(TIMER_FROSTBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = math.random(5000, 10000) })
    end

    -- Unholy Ground (AoE aura)
    if input:IsTimerReady(TIMER_UNHOLY_GROUND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UNHOLY_GROUND, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_UNHOLY_GROUND, duration = math.random(15000, 25000) })
    end

    -- Mortal Strike
    if input:IsTimerReady(TIMER_MORTAL_STRIKE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_STRIKE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_STRIKE, duration = math.random(10000, 15000) })
    end

    return actions
end

RegisterCreatureAI(14516, boss)
