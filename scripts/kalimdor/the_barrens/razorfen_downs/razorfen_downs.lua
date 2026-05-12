--[[
    @script_type: creature_ai
    @entry: 8025
    @name: razorfen_downs

    Razorfen Downs dungeon helper scripts.
    Ported from vmangos razorfen_downs.cpp

    Contents:
    - npc_belnistrasz (entry 8025): escort NPC for quest "Extinguishing the Idol" (3525).
      Walks to the idol room, pauses to perform a ~3-minute ritual, summons
      waves of quilboar mobs from 3 spawner positions. Casts Fireball and Frost
      Nova in combat during the ritual.

    Ritual phase schedule (mirrors vmangos UpdateEscortAI):
      Phase 0:  cast SPELL_IDOL_SHUTDOWN, wait 1s
      Phase 1:  summon wave, wait 39s
      Phase 2:  summon wave, wait 20s
      Phase 3:  say 3-min warning, wait 20s
      Phase 4:  summon wave, wait 40s
      Phase 5:  summon wave + say 2-min warning, wait 40s
      Phase 6:  summon wave, wait 20s
      Phase 7:  say 1-min warning, wait 40s
      Phase 8:  summon wave, wait 20s
      Phase 9:  say finish, wait 3s
      Phase 10: quest complete, clean up

    Waypoint 24 triggers the ritual pause (from WaypointReached).
    Waypoints are stored in the DB (creature_movement_template for this NPC).

    Skipped:
    - QuestAccept start trigger (requires Phase D gossip/quest API)
    - go_gong (GameObject interaction — requires GO API)
    - GO spawning at end (GO spawn API not yet available)
]]

-- Spell IDs
local SPELL_FIREBALL     = 9053
local SPELL_FROST_NOVA   = 11831
local SPELL_IDOL_SHUTDOWN = 12774

-- Text IDs
local SAY_BELNISTRASZ_READY     = 4493
local SAY_BELNISTRASZ_START_RIT = 4501
local SAY_BELNISTRASZ_AGGRO_1   = 9008
local SAY_BELNISTRASZ_AGGRO_2   = 9007
local SAY_BELNISTRASZ_3_MIN     = 4504
local SAY_BELNISTRASZ_2_MIN     = 4505
local SAY_BELNISTRASZ_1_MIN     = 4506
local SAY_BELNISTRASZ_FINISH    = 4507

-- NPC entries
local NPC_IDOL_ROOM_SPAWNER     = 8611
local NPC_WITHERED_BATTLE_BOAR  = 7333
local NPC_WITHERED_QUILGUARD    = 7329
local NPC_DEATHS_HEAD_GEOMANCER = 7335
local NPC_PLAGUEMAW_THE_ROTTING = 7356

-- Spawner positions (from vmangos m_fSpawnerCoord)
local SPAWNER_POSITIONS = {
    { x = 2582.79, y = 954.392, z = 52.4821, o = 3.78736 },
    { x = 2569.42, y = 956.380, z = 52.2732, o = 5.42797 },
    { x = 2570.62, y = 942.393, z = 53.7433, o = 0.71558 },
}

-- Timer IDs
local TIMER_RITUAL   = 1
local TIMER_FIREBALL  = 2
local TIMER_FROSTNOVA = 3

-- custom_data keys
local DATA_RITUAL_PHASE  = "ritual_phase"
local DATA_IN_RITUAL     = "in_ritual"
local DATA_AGGRO_SAID    = "aggro_said"

-- Waypoint that triggers the ritual pause
local WAYPOINT_RITUAL_START = 24

------------------------------------------------------------------------
-- Helper: pick a random spawner position
------------------------------------------------------------------------
local function random_spawner_pos()
    return SPAWNER_POSITIONS[math.random(1, #SPAWNER_POSITIONS)]
end

------------------------------------------------------------------------
-- Helper: build the "summon a wave" action
-- Spawns the idol room spawner NPC which then summons the actual wave.
-- The spawner NPC (8611) is a dummy that triggers JustSummoned on Belnistrasz.
------------------------------------------------------------------------
local function summon_wave_actions(input)
    local pos = random_spawner_pos()
    -- Spawn the idol room spawner. When it appears, JustSummoned fires
    -- and we spawn the actual wave mobs from that position.
    return {
        { action = "SPAWN_CREATURE",
          entry = NPC_IDOL_ROOM_SPAWNER,
          x = pos.x, y = pos.y, z = pos.z, o = pos.o,
          summon_type = "TIMED_DESPAWN",
          duration = 10000 },
    }
end

------------------------------------------------------------------------
-- npc_belnistrasz (entry 8025)
------------------------------------------------------------------------
local belnistrasz = {}

function belnistrasz:OnSpawn(input)
    return {
        { action = "SET_COMBAT_MOVEMENT", enabled = true },
        { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_IN_RITUAL, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_AGGRO_SAID, value = 0 },
    }
end

function belnistrasz:JustRespawned(input)
    return self:OnSpawn(input)
end

function belnistrasz:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_FIREBALL,  duration = 1000 },
        { action = "SET_TIMER", timer_id = TIMER_FROSTNOVA, duration = 6000 },
    }
end

function belnistrasz:MovementInform(input, movement_type, point_id)
    -- When Belnistrasz reaches waypoint 24, begin the ritual
    if point_id ~= WAYPOINT_RITUAL_START then return {} end

    return {
        { action = "SAY", text_id = SAY_BELNISTRASZ_START_RIT },
        { action = "STOP_MOVEMENT" },
        { action = "SET_COMBAT_MOVEMENT", enabled = false },
        { action = "SET_CUSTOM_DATA", key = DATA_IN_RITUAL, value = 1 },
        { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 0 },
        { action = "CAST_SPELL", spell_id = SPELL_IDOL_SHUTDOWN, target = "self", triggered = true },
        { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 1000 },
    }
end

function belnistrasz:OnUpdate(input)
    local actions = {}

    -- Combat spells (during and outside ritual)
    if input.is_in_combat then
        if input:IsTimerReady(TIMER_FIREBALL) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBALL, target = "current_target" })
            local t = 2000 + math.random(0, 1000)
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIREBALL, duration = t })
        end

        if input:IsTimerReady(TIMER_FROSTNOVA) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FROST_NOVA, target = "current_target" })
            local t = 10000 + math.random(0, 5000)
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FROSTNOVA, duration = t })
        end
    end

    -- Ritual phase progression
    if input:GetCustomData(DATA_IN_RITUAL) ~= 1 then return actions end
    if not input:IsTimerReady(TIMER_RITUAL) then return actions end

    local phase = input:GetCustomData(DATA_RITUAL_PHASE)

    if phase == 0 then
        -- Already cast IDOL_SHUTDOWN in MovementInform; advance
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 39000 })
        -- Spawn first wave
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end

    elseif phase == 1 then
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 2 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 20000 })

    elseif phase == 2 then
        table.insert(actions, { action = "SAY", text_id = SAY_BELNISTRASZ_3_MIN })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 3 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 20000 })

    elseif phase == 3 then
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 4 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 40000 })

    elseif phase == 4 then
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end
        table.insert(actions, { action = "SAY", text_id = SAY_BELNISTRASZ_2_MIN })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 5 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 40000 })

    elseif phase == 5 then
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 6 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 20000 })

    elseif phase == 6 then
        table.insert(actions, { action = "SAY", text_id = SAY_BELNISTRASZ_1_MIN })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 7 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 40000 })

    elseif phase == 7 then
        for _, a in ipairs(summon_wave_actions(input)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 8 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 20000 })

    elseif phase == 8 then
        table.insert(actions, { action = "SAY", text_id = SAY_BELNISTRASZ_FINISH })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 9 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RITUAL, duration = 3000 })

    elseif phase == 9 then
        -- Ritual complete: remove shutdown aura, re-enable movement, clear ritual flag
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_IDOL_SHUTDOWN })
        table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_IN_RITUAL, value = 0 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_RITUAL_PHASE, value = 10 })
        -- TODO(api): GroupEventHappens / quest completion requires Phase D quest API
    end

    return actions
end

------------------------------------------------------------------------
-- npc_idol_room_spawner (entry 8611)
-- Dummy NPC that spawns a wave of mobs when summoned. Despawns after
-- a short delay. The parent (Belnistrasz) receives JustSummoned.
-- We implement the wave spawning here directly on the spawner.
------------------------------------------------------------------------
local idol_room_spawner = {}

function idol_room_spawner:OnSpawn(input)
    -- Spawn 4 mobs: 2x Withered Battle Boar, 1x Quilguard, 1x Geomancer
    -- Phase 8+ (> 7 in vmangos): spawn Plaguemaw instead
    -- We use custom_data from parent which we can't read here, so always
    -- spawn the standard wave. Plaguemaw is the final wave at phase 8.
    -- TODO: pass phase info via instance data if needed.
    local actions = {}
    local entries = {
        NPC_WITHERED_BATTLE_BOAR,
        NPC_WITHERED_BATTLE_BOAR,
        NPC_WITHERED_QUILGUARD,
        NPC_DEATHS_HEAD_GEOMANCER,
    }
    for _, entry in ipairs(entries) do
        local rx = input.position.x + (math.random() - 0.5) * 4
        local ry = input.position.y + (math.random() - 0.5) * 4
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = entry,
            x = rx, y = ry, z = input.position.z, o = input.position.o,
            summon_type = "TIMED_DESPAWN",
            duration = 60000,
        })
    end
    return actions
end

RegisterCreatureAI(8025, belnistrasz)
RegisterCreatureAI(8611, idol_room_spawner)
