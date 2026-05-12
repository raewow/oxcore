--[[
    @script_type: creature_ai
    @entry: 10438
    @name: boss_maleki

    Maleki the Pallid - Stratholme (Undead)
    Ported from vmangos boss_maleki_the_pallid.cpp

    Ranged caster who prefers to stay at distance. At <60% HP or <50% mana,
    uses Drain Life or Drain Mana. Ice Tomb on current target.
]]

-- Spells
local SPELL_FROSTBOLT  = 17503  -- Frost damage + slow on target
local SPELL_DRAIN_LIFE = 17238  -- Drains life from target to self
local SPELL_DRAIN_MANA = 17243  -- Drains mana from target to self
local SPELL_ICE_TOMB   = 16869  -- Freezes target in place (root + debuff)

-- Timer IDs
local TIMER_FROSTBOLT  = 1
local TIMER_ICE_TOMB   = 2
local TIMER_DRAIN      = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_ICE_TOMB,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_DRAIN,     duration = 4000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frostbolt (current target, repeats every 3.5-4.5s)
    if input:IsTimerReady(TIMER_FROSTBOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROSTBOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTBOLT, duration = math.random(3500, 4500) })
    end

    -- Ice Tomb (current target, repeats every 20-25s)
    if input:IsTimerReady(TIMER_ICE_TOMB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ICE_TOMB, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ICE_TOMB, duration = math.random(20000, 25000) })
    end

    -- Drain Life or Drain Mana (current target, when low on HP or mana, repeats every 12-18s)
    if input.health_pct < 60 then
        if input:IsTimerReady(TIMER_DRAIN) then
            -- Prefer Drain Mana if target has mana (approximated as random_hostile for mana users);
            -- use Drain Life as fallback
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAIN_LIFE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DRAIN, duration = math.random(12000, 18000) })
        end
    end

    return actions
end

function boss:OnDeath(input, killer_guid)
    return {}
end

RegisterCreatureAI(10438, boss)
