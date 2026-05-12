--[[
    @script_type: creature_ai
    @entry: 15510
    @name: boss_fankriss

    Fankriss the Unyielding - Temple of Ahn'Qiraj
    Verified against vmangos boss_fankriss.cpp

    Mechanics:
    - Mortal Wound: urand(4,8)s init, repeat urand(4,8)s
    - Summon Spawn of Fankriss (worms): urand(20,30)s init, 1 wave at a time
    - Entangle (3 variants): Root random players, spawn Vekniss Hatchlings near them
      Web 1: urand(2,18)s, Web 2: urand(15,28)s, Web 3: urand(25,45)s; rotation repeats at 45s

    Note: The complex hatchling logic (tracking alive count, per-web batches, 3-location spawning)
    is approximated. Spawn Fankriss enrages after 10s (patch 1.10+).
]]

local SPELL_MORTAL_WOUND    = 25646  -- was 28467 (wrong)
local SPELL_ENTANGLE_1      = 720
local SPELL_ENTANGLE_2      = 731
local SPELL_ENTANGLE_3      = 1121

local NPC_SPAWN_FANKRISS    = 15630
local NPC_VEKNISS_HATCHLING = 15962  -- was NPC_HATCHLING 15962 (correct)

local TIMER_MORTAL_WOUND    = 1
local TIMER_SPAWN_WORMS     = 2
local TIMER_WEB_1           = 3
local TIMER_WEB_2           = 4
local TIMER_WEB_3           = 5

-- Hatchling spawn locations (from C++)
local HATCH_LOC = {
    { x = -8043.01, y = 1254.20, z = -84.19 },
    { x = -8003.00, y = 1222.90, z = -82.10 },
    { x = -8022.68, y = 1150.08, z = -89.33 },
}

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_WOUND, duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_WORMS,  duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_WEB_1,        duration = math.random(10000, 26000) },  -- 8+urand(2,18)
        { action = "SET_TIMER", timer_id = TIMER_WEB_2,        duration = math.random(23000, 36000) },  -- 8+urand(15,28)
        { action = "SET_TIMER", timer_id = TIMER_WEB_3,        duration = math.random(33000, 53000) },  -- 8+urand(25,45)
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_MORTAL_WOUND, duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_WORMS,  duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_WEB_1,        duration = math.random(10000, 26000) },
        { action = "SET_TIMER", timer_id = TIMER_WEB_2,        duration = math.random(23000, 36000) },
        { action = "SET_TIMER", timer_id = TIMER_WEB_3,        duration = math.random(33000, 53000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Mortal Wound on current target
    if input:IsTimerReady(TIMER_MORTAL_WOUND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MORTAL_WOUND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MORTAL_WOUND, duration = math.random(4000, 8000) })
    end

    -- Summon worms (1 wave, random player aggro)
    if input:IsTimerReady(TIMER_SPAWN_WORMS) then
        local count = math.random(1, 3)
        for _ = 1, count do
            table.insert(actions, { action = "SPAWN_CREATURE_NEAR_RANDOM", entry = NPC_SPAWN_FANKRISS })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPAWN_WORMS, duration = math.random(20000, 30000) })
    end

    -- Entangle web 1: root random player, spawn hatchlings in all 3 locations
    if input:IsTimerReady(TIMER_WEB_1) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENTANGLE_1, target = "random_hostile" })
        local spawn_count = math.random(2, 4)
        for i = 1, 3 do
            for _ = 1, spawn_count do
                local loc = HATCH_LOC[i]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_VEKNISS_HATCHLING,
                    x = loc.x, y = loc.y, z = loc.z, o = 0, summon_type = 1, duration = 65000 })
            end
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WEB_1, duration = math.random(2000, 18000) })
    end

    -- Entangle web 2
    if input:IsTimerReady(TIMER_WEB_2) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENTANGLE_2, target = "random_hostile" })
        local spawn_count = math.random(2, 4)
        for i = 1, 3 do
            for _ = 1, spawn_count do
                local loc = HATCH_LOC[i]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_VEKNISS_HATCHLING,
                    x = loc.x, y = loc.y, z = loc.z, o = 0, summon_type = 1, duration = 65000 })
            end
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WEB_2, duration = math.random(15000, 28000) })
    end

    -- Entangle web 3
    if input:IsTimerReady(TIMER_WEB_3) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENTANGLE_3, target = "random_hostile" })
        local spawn_count = math.random(2, 4)
        for i = 1, 3 do
            for _ = 1, spawn_count do
                local loc = HATCH_LOC[i]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_VEKNISS_HATCHLING,
                    x = loc.x, y = loc.y, z = loc.z, o = 0, summon_type = 1, duration = 65000 })
            end
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_WEB_3, duration = math.random(25000, 45000) })
    end

    return actions
end

RegisterCreatureAI(15510, boss)
