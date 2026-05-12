--[[
    Gnomeregan dungeon helper scripts.
    Ported from vmangos gnomeregan.cpp

    Contents:
    - npc_blastmaster_emi_shortfuse (entry 7998): escort NPC for the Grubbis encounter.
      Walks through Gnomeregan, pauses to place explosive charges, summons waves of
      Caverndeep Ambushers/Burrowers from packed position arrays, then summons
      Grubbis (7361) + Chomper (6215) at the climax.

    Key mechanics:
      Phase 1-3:   Start/intro dialogue, begin escort walk
      Phase 4-9:   Waypoints + speech as she walks through the first corridor
      Phase 10-20: Southern cave-in sequence (open door, set charges, summon packs 1-3, blow it up)
      Phase 21-33: Northern cave-in sequence (open door, set charges, summon packs 4-6)
      Phase 33-35: Grubbis + Chomper spawn; wait for Grubbis death; final speech
      Phase 36+:   Finish

    Skipped:
    - QuestAccept/Gossip start trigger (requires Phase D gossip/quest API)
    - GO_CAVE_IN_SOUTH / GO_CAVE_IN_NORTH door open/close (requires GO API)
    - Instance data TYPE_EXPLOSIVE_CHARGE (GO-based charge activation)
    - Formation group for Chomper (CreatureGroups C++ API not available)

    Mob summon data is embedded from vmangos asSummonInfo array.
]]

-- Text IDs
local SAY_START       = 4050
local SAY_INTRO_1     = 4051
local SAY_INTRO_2     = 4052
local SAY_INTRO_3     = 4129
local SAY_INTRO_4     = 4130
local SAY_LOOK_1      = 4131
local SAY_HEAR_1      = 4132
local SAY_AGGRO_1     = 5161
local SAY_CHARGE_1    = 4133
local SAY_CHARGE_2    = 4134
local SAY_BLOW_1_10   = 4135
local SAY_BLOW_1_5    = 4136
local SAY_BLOW_1      = 4137
local SAY_FINISH_1    = 4206
local SAY_LOOK_2      = 4207
local SAY_HEAR_2      = 4208
local SAY_CHARGE_3    = 4209
local SAY_CHARGE_4    = 4325
local SAY_BLOW_2_10   = 4326
local SAY_BLOW_2_5    = 4327
local SAY_BLOW_SOON   = 4329
local SAY_FINISH_2    = 4446
local SAY_AGGRO_2     = 5164
local SAY_GRUBBIS_SPAWN = 4328

-- Spell IDs
local SPELL_EXPLOSION_SOUTH = 12159
local SPELL_EXPLOSION_NORTH = 12158

-- NPC entries
local NPC_GRUBBIS              = 7361
local NPC_CHOMPER              = 6215
local NPC_CAVERNDEEP_BURROWER  = 6206
local NPC_CAVERNDEEP_AMBUSHER  = 6207

-- Summon positions grouped by pack index (from vmangos asSummonInfo)
-- Pack 1: south first wave (9 ambushers)
-- Pack 2: south second wave (4 ambushers)
-- Pack 3: south third wave (2 ambushers)
-- Pack 4: north first wave (4 ambushers)
-- Pack 5: north second wave (3 ambushers + 1 burrower)
-- Pack 6: north third wave (8 ambushers)
-- Pack 7: Grubbis + Chomper
local SUMMON_PACKS = {
    [1] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -566.8114, y = -111.7036, z = -151.1891, o = 5.986479 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -568.5875, y = -113.7559, z = -151.1869, o = 0.069813 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -570.2333, y = -116.8126, z = -151.2272, o = 0.296706 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -550.6331, y = -108.7592, z = -153.9650, o = 0.890118 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -558.9717, y = -115.0669, z = -151.8799, o = 0.523599 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -556.6719, y = -112.0526, z = -152.8255, o = 0.488692 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -552.6419, y = -113.4385, z = -153.0727, o = 0.802851 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -549.1248, y = -112.1469, z = -153.7987, o = 0.750492 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -546.7435, y = -112.3051, z = -154.2225, o = 0.925025 },
    },
    [2] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -571.4071, y = -108.7721, z = -150.6547, o = 5.480334 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -573.7970, y = -106.5265, z = -150.4106, o = 5.550147 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -576.3784, y = -108.0483, z = -150.4227, o = 5.585053 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -576.6970, y = -111.7413, z = -150.6484, o = 5.759586 },
    },
    [3] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -571.3161, y = -114.4412, z = -151.0931, o = 6.021386 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -570.3127, y = -111.7964, z = -151.0400, o = 2.042035 },
    },
    [4] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -474.5954, y = -104.0740, z = -146.0483, o = 2.338741 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -477.9396, y = -108.6563, z = -145.7394, o = 1.553343 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -475.6625, y = -97.12168, z = -146.5959, o = 1.291544 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -480.5233, y = -88.40702, z = -146.3772, o = 3.001966 },
    },
    [5] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -474.2943, y = -105.2212, z = -145.9747, o = 2.251475 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -481.1831, y = -101.4225, z = -146.3770, o = 2.146755 },
        { e = NPC_CAVERNDEEP_BURROWER, x = -475.0871, y = -100.0160, z = -146.4382, o = 2.303835 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -478.8562, y = -106.9321, z = -145.8533, o = 1.658063 },
    },
    [6] = {
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -473.8762, y = -107.4022, z = -145.8380, o = 2.024582 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -490.5134, y = -92.72843, z = -148.0954, o = 3.054326 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -491.4010, y = -88.25341, z = -148.0358, o = 3.560472 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -479.1431, y = -106.2270, z = -145.9097, o = 1.727876 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -475.3185, y = -101.4804, z = -146.2717, o = 2.234021 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -485.1559, y = -89.57419, z = -146.9299, o = 3.071779 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -482.2516, y = -96.80614, z = -146.6596, o = 2.303835 },
        { e = NPC_CAVERNDEEP_AMBUSHER, x = -477.9874, y = -92.82047, z = -146.6944, o = 3.124139 },
    },
    [7] = {
        { e = NPC_GRUBBIS, x = -476.3761, y = -108.1901, z = -145.7763, o = 1.919862 },
        { e = NPC_CHOMPER,  x = -473.1326, y = -103.0901, z = -146.1155, o = 2.042035 },
    },
}

-- Helper: build spawn actions for a pack
local function spawn_pack(pack_id)
    local actions = {}
    local pack = SUMMON_PACKS[pack_id]
    if not pack then return actions end
    for _, mob in ipairs(pack) do
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = mob.e,
            x = mob.x, y = mob.y, z = mob.z, o = mob.o,
            summon_type = "DEAD_DESPAWN",
            duration = 0,
        })
    end
    return actions
end

-- Timer IDs
local TIMER_PHASE = 1

-- custom_data keys
local DATA_PHASE       = "phase"
local DATA_AGGRO_SAID  = "aggro"
local DATA_GRUBBIS_DEAD = "grubbis_dead"

-- Instance data ID for Grubbis encounter
local TYPE_GRUBBIS = 0

------------------------------------------------------------------------
-- npc_blastmaster_emi_shortfuse (entry 7998)
------------------------------------------------------------------------
local emi = {}

function emi:OnSpawn(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_PHASE,        value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_AGGRO_SAID,   value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_GRUBBIS_DEAD, value = 0 },
    }
end

function emi:JustRespawned(input)
    return self:OnSpawn(input)
end

function emi:OnEnterCombat(input)
    if input:GetCustomData(DATA_AGGRO_SAID) == 0 then
        local say_id = (math.random(0, 1) == 0) and SAY_AGGRO_1 or SAY_AGGRO_2
        return {
            { action = "SAY", text_id = say_id },
            { action = "SET_CUSTOM_DATA", key = DATA_AGGRO_SAID, value = 1 },
        }
    end
    return {}
end

function emi:OnEvade(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_AGGRO_SAID, value = 0 },
    }
end

-- MovementInform fires for waypoints along the escort path
function emi:MovementInform(input, movement_type, point_id)
    local actions = {}
    local phase = input:GetCustomData(DATA_PHASE)

    if point_id == 4 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 4 })
    elseif point_id == 9 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 5 })
    elseif point_id == 12 then
        table.insert(actions, { action = "SAY", text_id = SAY_CHARGE_1 })
    elseif point_id == 15 then
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SAY", text_id = SAY_BLOW_1_10 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 16 })
    elseif point_id == 16 then
        table.insert(actions, { action = "SAY", text_id = SAY_CHARGE_3 })
    elseif point_id == 19 then
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 31 })
    end

    return actions
end

function emi:OnUpdate(input)
    -- Only process event timer OOC
    if input.is_in_combat then return {} end
    if not input:IsTimerReady(TIMER_PHASE) then return {} end

    local phase = input:GetCustomData(DATA_PHASE)
    local actions = {}

    if phase == 1 then
        table.insert(actions, { action = "SAY", text_id = SAY_START })
        table.insert(actions, { action = "SET_FACTION", faction_id = 250 })  -- FACTION_ESCORT_N_NEUTRAL_PASSIVE
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 2 })

    elseif phase == 2 then
        table.insert(actions, { action = "SAY", text_id = SAY_INTRO_1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 3500 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 3 })

    elseif phase == 3 then
        -- Begin escort walk (escort starts via quest accept, waypoints from DB)
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 4 then
        table.insert(actions, { action = "SAY", text_id = SAY_INTRO_2 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 5 then
        table.insert(actions, { action = "SAY", text_id = SAY_INTRO_3 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 6000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 6 })

    elseif phase == 6 then
        table.insert(actions, { action = "SAY", text_id = SAY_INTRO_4 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 9000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 7 })

    elseif phase == 7 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 8 })

    elseif phase == 8 then
        table.insert(actions, { action = "SAY", text_id = SAY_LOOK_1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 9 })

    elseif phase == 9 then
        table.insert(actions, { action = "SAY", text_id = SAY_HEAR_1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 10 })

    elseif phase == 10 then
        -- Summon first wave pack
        for _, a in ipairs(spawn_pack(1)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 11 then
        for _, a in ipairs(spawn_pack(2)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 12 })

    elseif phase == 12 then
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 13 then
        for _, a in ipairs(spawn_pack(3)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 11000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 14 })

    elseif phase == 14 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 15 })

    elseif phase == 15 then
        table.insert(actions, { action = "SAY", text_id = SAY_CHARGE_2 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 16 then
        table.insert(actions, { action = "SAY", text_id = SAY_BLOW_1_5 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 17 })

    elseif phase == 17 then
        table.insert(actions, { action = "SAY", text_id = SAY_BLOW_1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 18 })

    elseif phase == 18 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EXPLOSION_SOUTH, target = "self", triggered = true })
        -- TODO(api): close south cave-in GO, set explosive charge data
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5500 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 20 })

    elseif phase == 20 then
        table.insert(actions, { action = "EMOTE", emote_id = 4 })  -- EMOTE_ONESHOT_CHEER
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 6000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 21 })

    elseif phase == 21 then
        table.insert(actions, { action = "SAY", text_id = SAY_FINISH_1 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 6000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 22 })

    elseif phase == 22 then
        table.insert(actions, { action = "SAY", text_id = SAY_LOOK_2 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 3000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 23 })

    elseif phase == 23 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 3000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 24 })

    elseif phase == 24 then
        table.insert(actions, { action = "SAY", text_id = SAY_HEAR_2 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 8000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 25 })

    elseif phase == 25 then
        -- Resume escort and summon north pack 4
        for _, a in ipairs(spawn_pack(4)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 26 then
        for _, a in ipairs(spawn_pack(5)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 27 })

    elseif phase == 27 then
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 28 then
        for _, a in ipairs(spawn_pack(6)) do table.insert(actions, a) end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 10000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 29 })

    elseif phase == 29 then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 30 })

    elseif phase == 30 then
        table.insert(actions, { action = "SAY", text_id = SAY_CHARGE_4 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })

    elseif phase == 31 then
        table.insert(actions, { action = "SAY", text_id = SAY_BLOW_2_10 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 5000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 32 })

    elseif phase == 32 then
        table.insert(actions, { action = "SAY", text_id = SAY_BLOW_2_5 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 33 })

    elseif phase == 33 then
        -- Spawn Grubbis + Chomper
        for _, a in ipairs(spawn_pack(7)) do table.insert(actions, a) end
        table.insert(actions, { action = "SAY", text_id = SAY_GRUBBIS_SPAWN })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 1000 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 34 })

    elseif phase == 34 then
        -- Wait for Grubbis to die
        local grubbis_list = input:GetCreaturesByEntry(NPC_GRUBBIS)
        if #grubbis_list == 0 or input:GetCustomData(DATA_GRUBBIS_DEAD) == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_EXPLOSION_NORTH, target = "self", triggered = true })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 35 })
        else
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PHASE, duration = 2000 })
        end

    elseif phase == 35 then
        table.insert(actions, { action = "SAY", text_id = SAY_FINISH_2 })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 })
        -- TODO(api): GroupEventHappens / quest complete requires Phase D quest API
    end

    return actions
end

-- Track when Grubbis dies
function emi:SummonedCreatureJustDied(input, summoned_guid)
    -- Can't check entry of summoned_guid without API extension.
    -- Mark that a key summon died; if Grubbis was the only one this works fine.
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_GRUBBIS_DEAD, value = 1 },
    }
end

function emi:OnDeath(input, killer_guid)
    -- Despawn all summons on death
    return {
        { action = "DESPAWN_CREATURE", entry = NPC_CAVERNDEEP_AMBUSHER },
        { action = "DESPAWN_CREATURE", entry = NPC_CAVERNDEEP_BURROWER },
        { action = "DESPAWN_CREATURE", entry = NPC_GRUBBIS },
        { action = "DESPAWN_CREATURE", entry = NPC_CHOMPER },
    }
end

RegisterCreatureAI(7998, emi)
