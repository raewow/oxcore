--[[
    @script_type: creature_ai
    @entry: 8014
    @name: razorfen_kraul

    Razorfen Kraul dungeon helper scripts.
    Ported from vmangos razorfen_kraul.cpp

    Contents:
    - npc_willix_the_importer (entry 8014): escort NPC for quest 1144.
      Walks waypoints through RFK, says lines at specific waypoints, summons
      2 Raging Agamar boars (entry 4514) twice along the path.

    Skipped:
    - QuestAccept start trigger (requires Phase D gossip/quest API)
    - npc_snufflenose_gopher (entry 4781): follower that seeks GO blueleaf tubers.
      Requires GameObject detection API (not yet available).
      Ported stub only.
    - EffectDummyCreature_npc_snufflenose_gopher: spell dummy effect handler
      (requires SpellEffect hook, not yet available).
]]

-- Text IDs
local SAY_WILLIX_READY   = 1482
local SAY_WILLIX_1       = 1483
local SAY_WILLIX_2       = 1484
local SAY_WILLIX_3       = 1485
local SAY_WILLIX_4       = 1486
local SAY_WILLIX_5       = 1487
local SAY_WILLIX_6       = 1488
local SAY_WILLIX_7       = 1490
local SAY_WILLIX_END     = 1493

local SAY_WILLIX_AGGRO = { 1546, 1544, 1545, 1547 }

-- NPC entries
local NPC_RAGING_AGAMAR = 4514

-- Boar spawn positions (from vmangos aBoarSpawn)
local BOAR_SPAWN = {
    { x = 2151.420, y = 1733.18, z = 52.10 },
    { x = 2144.463, y = 1726.89, z = 51.93 },
    { x = 1956.433, y = 1597.97, z = 81.75 },
    { x = 1958.971, y = 1599.01, z = 81.44 },
}

------------------------------------------------------------------------
-- npc_willix_the_importer (entry 8014)
------------------------------------------------------------------------
local willix = {}

function willix:OnSpawn(input)
    return {
        { action = "SET_REACT_STATE", state = "DEFENSIVE" },
        { action = "SET_IMMUNE", physical = false, spell = false },
    }
end

function willix:OnEnterCombat(input)
    -- 4/7 chance to say something on aggro
    local roll = math.random(0, 6)
    if roll <= 3 then
        return {
            { action = "SAY", text_id = SAY_WILLIX_AGGRO[roll + 1] },
        }
    end
    return {}
end

function willix:MovementInform(input, movement_type, point_id)
    -- Fired when Willix reaches a waypoint along his escort path
    local actions = {}

    if point_id == 2 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_1 })

    elseif point_id == 6 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_2 })

    elseif point_id == 9 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_3 })

    elseif point_id == 14 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_4 })
        -- Summon 2 boars on the pathway
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = NPC_RAGING_AGAMAR,
            x = BOAR_SPAWN[1].x, y = BOAR_SPAWN[1].y, z = BOAR_SPAWN[1].z, o = 0.0,
            summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT",
            duration = 25000,
        })
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = NPC_RAGING_AGAMAR,
            x = BOAR_SPAWN[2].x, y = BOAR_SPAWN[2].y, z = BOAR_SPAWN[2].z, o = 0.0,
            summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT",
            duration = 25000,
        })

    elseif point_id == 25 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_5 })

    elseif point_id == 33 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_6 })

    elseif point_id == 44 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_7 })
        -- Summon 2 boars near the end
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = NPC_RAGING_AGAMAR,
            x = BOAR_SPAWN[3].x, y = BOAR_SPAWN[3].y, z = BOAR_SPAWN[3].z, o = 0.0,
            summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT",
            duration = 25000,
        })
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = NPC_RAGING_AGAMAR,
            x = BOAR_SPAWN[4].x, y = BOAR_SPAWN[4].y, z = BOAR_SPAWN[4].z, o = 0.0,
            summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT",
            duration = 25000,
        })

    elseif point_id == 45 then
        table.insert(actions, { action = "SAY", text_id = SAY_WILLIX_END })
        table.insert(actions, { action = "STOP_MOVEMENT" })
        -- Set questgiver flag (NPC_FLAG_QUESTGIVER = 0x2)
        table.insert(actions, { action = "SET_UNIT_FLAG", flag = 0x2 })
        -- TODO(api): GroupEventHappens(QUEST_WILLIX_THE_IMPORTER) requires Phase D quest API
    end

    return actions
end

-- Summoned boars should attack Willix automatically (aggressive faction).
-- JustSummoned: summons are boars that AttackStart willix; we can't redirect
-- their target here since we don't control their AI. They use DB faction.

RegisterCreatureAI(8014, willix)

------------------------------------------------------------------------
-- npc_snufflenose_gopher (entry 4781)
-- STUB: follows owner, but GO tuber detection is blocked on GO API.
-- Ported what's possible (say on spawn, passive faction).
-- TODO(api): GameObject detection + FollowerAI patrol require GO/follow API.
------------------------------------------------------------------------
local SAY_GOPHER_SPAWN = 1638

local snufflenose_gopher = {}

function snufflenose_gopher:OnSpawn(input)
    return {
        { action = "SET_FACTION", faction_id = 35 },
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SAY", text_id = SAY_GOPHER_SPAWN },
    }
end

RegisterCreatureAI(4781, snufflenose_gopher)
