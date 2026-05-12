--[[
    @script_type: creature_ai
    @entry: 7271
    @name: boss_zumrah

    Witch Doctor Zum'rah - Zul'Farrak
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    A troll witch doctor boss in Zul'Farrak. Summons zombie adds and casts
    shadow magic.

    Mechanics:
    - Shadow Bolt: periodic shadow nuke
    - Hex of Zum'rah: periodic curse on a random target (turns them into a zombie)
    - Summon Zombies: periodically raises nearby dead to fight for him
    - Shadow Ward: damage absorption shield on self
]]

local SPELL_SHADOW_BOLT         = 9613
local SPELL_HEX_OF_ZUMRAH       = 11920
local SPELL_SUMMON_ZOMBIE_ZULFARRAK = 11920  -- summon via same hex mechanic
local SPELL_SHADOW_WARD         = 9841

local NPC_ZULFARRAK_ZOMBIE      = 7286

local TIMER_SHADOW_BOLT         = 1
local TIMER_HEX                 = 2
local TIMER_SUMMON              = 3
local TIMER_SHADOW_WARD         = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT,  duration = math.random(3000, 7000)   },
        { action = "SET_TIMER", timer_id = TIMER_HEX,          duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON,       duration = math.random(12000, 18000) },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_WARD,  duration = math.random(15000, 25000) },
    }
end

function boss:OnReset(input) return {} end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(5000, 10000) })
    end

    -- Hex of Zum'rah (polymorph/curse random target)
    if input:IsTimerReady(TIMER_HEX) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEX_OF_ZUMRAH, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEX, duration = math.random(12000, 20000) })
    end

    -- Summon Zul'Farrak Zombies
    if input:IsTimerReady(TIMER_SUMMON) then
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_ZULFARRAK_ZOMBIE, distance = 8.0 })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_ZULFARRAK_ZOMBIE, distance = 8.0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON, duration = math.random(20000, 30000) })
    end

    -- Shadow Ward (absorb shield)
    if input:IsTimerReady(TIMER_SHADOW_WARD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_WARD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_WARD, duration = math.random(20000, 30000) })
    end

    return actions
end

RegisterCreatureAI(7271, boss)
