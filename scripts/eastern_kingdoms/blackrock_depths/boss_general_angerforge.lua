--[[
    @script_type: creature_ai
    @entry: 9033
    @name: boss_general_angerforge

    Boss General Angerforge - Blackrock Depths
    Ported from MaNGOS boss_general_angerforge.cpp
]]

local SPELL_SUNDER_ARMOR = 15572

local NPC_ANVILRAGE_MEDIC     = 8894
local NPC_ANVILRAGE_RESERVIST = 8901

local TIMER_SUNDER_ARMOR = 1
local TIMER_ALARM        = 2

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = math.random(5000, 10000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = math.random(5000, 10000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Sunder Armor
    if input:IsTimerReady(TIMER_SUNDER_ARMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUNDER_ARMOR, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUNDER_ARMOR, duration = math.random(5000, 15000) })
    end

    -- Summon reinforcements below 30% HP
    if input.health_pct < 0.30 and input:IsTimerReady(TIMER_ALARM) then
        table.insert(actions, { action = "EMOTE", emote_id = 5286 })
        -- Spawn 8 reservists and 2 medics
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 716.817, y = 23.035, z = -45.344, o = 3.159, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 719.820, y = 25.443, z = -45.329, o = 3.194, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 720.068, y = 22.938, z = -45.341, o = 3.159, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 719.930, y = 19.805, z = -45.359, o = 3.107, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 724.482, y = 25.275, z = -45.316, o = 3.194, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 724.496, y = 22.622, z = -45.328, o = 3.159, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 724.706, y = 19.891, z = -45.338, o = 3.124, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_RESERVIST, x = 728.701, y = 18.928, z = -46.002, o = 3.107, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_MEDIC, x = 728.546, y = 21.528, z = -45.893, o = 3.142, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_ANVILRAGE_MEDIC, x = 728.648, y = 24.581, z = -45.947, o = 3.177, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 30000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ALARM, duration = 180000 })
    end

    return actions
end

RegisterCreatureAI(9033, boss)
