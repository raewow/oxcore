--[[
    Wailing Caverns dungeon helper scripts.
    Ported from vmangos wailing_caverns.cpp

    Contents:
    - npc_disciple_of_naralex (entry 3592): complex escort NPC with ritual battle
    - npc_evolving_ectoplasm  (entry 5763): school-based immunity/morph mechanic

    Escort sequence (mirrors vmangos WaypointReached + UpdateEscortAI):
      WP 0:  say CAST_MARK, pause, cast Mark of the Wild on all nearby players, resume
      WP 7:  say SAY_1ST_WP, pause, summon 2 Deviate Raptors, wait for them to die, say SAY_AFTER_1ST_TRASH, resume
      WP 15: say SAY_BEFORE_CIRCLE, cast SPELL_CLEANSING (self-rooted 15s), summon 3 Deviate Vipers mid-cast, resume after combat clear
      WP 26: say SAY_BEFORE_CHAMBER, pause briefly, resume
      WP 30: final ritual sequence (10+ sub-phases) — summon moccasins, ectoplasms, Mutanus, await kill, fly out

    NOTE: Instance data checks (TYPE_MUTANUS == DONE) require instance_wailing_caverns.lua
    to set that data when Mutanus dies.
    NOTE: Flying movement (MovePoint fly mode) is approximated with MOVE_TO.
    NOTE: Start trigger (OnScriptEventHappened) requires Phase D gossip API.
]]

-- Spell IDs
local SPELL_MARK      = 5232    -- Mark of the Wild (cast on nearby players at WP 0)
local SPELL_SLEEP     = 1090    -- Sleep (combat, random target)
local SPELL_POTION    = 8141    -- Healing Potion (self, <80% HP)
local SPELL_CLEANSING = 6270    -- Cleansing ritual at circle
local SPELL_AWAKENING = 6271    -- Awakening ritual (WP 30)
local SPELL_SHAPESHIFT = 8153   -- Druid form for flying out

-- Text IDs
local SAY_CAST_MARK        = 1255
local SAY_1ST_WP           = 1256
local SAY_AFTER_1ST_TRASH  = 1257
local SAY_BEFORE_CIRCLE    = 1258
local SAY_AFTER_CIRCLE     = 1259
local SAY_BEFORE_CHAMBER   = 1263
local SAY_BEFORE_RITUAL    = 1264
local EMOTE_DISCIPLE_1     = 1265
local EMOTE_NARALEX_1      = 1268
local SAY_ATTACKED         = 1273
local EMOTE_NARALEX_2      = 1269
local SAY_MUTANUS_SPAWNED  = 1276
local SAY_NARALEX_AWAKEN   = 1271
local SAY_DISCIPLE_FINAL   = 1267
local SAY_NARALEX_FINAL1   = 1272
local SAY_NARALEX_FINAL2   = 2103

-- NPC entries
local MOB_DEVIATE_RAPTOR      = 3636
local MOB_DEVIATE_VIPER       = 5755
local MOB_DEVIATE_MOCCASIN    = 5762
local MOB_NIGHTMARE_ECTOPLASM = 5763
local MOB_MUTANUS_DEVOURER    = 3654

-- Mob summon positions (from vmangos Position[10][3])
local MOB_POSITIONS = {
    { x = -52.9, y = 269.8, z = -92.8 },   -- [0] viper 0
    { x = -58.5, y = 279.8, z = -92.8 },   -- [1] viper 1
    { x = -49.6, y = 278.2, z = -92.8 },   -- [2] viper 2
    { x = 142.7, y = 254.0, z = -102.2 },  -- [3] moccasin/ectoplasm/mutanus
    { x = 140.5, y = 219.8, z = -102.4 },  -- [4] ectoplasm
    { x = 92.2,  y = 261.9, z = -101.5 },  -- [5] ectoplasm
    { x = 100.3, y = 268.6, z = -102.2 },  -- [6] ectoplasm
    { x = 123.8, y = 271.9, z = -102.4 },  -- [7] moccasin
    { x = 151.9, y = 234.3, z = -102.5 },  -- [8] moccasin
    { x = 127.6, y = 200.8, z = -101.8 },  -- [9] moccasin/ectoplasm
}

-- Instance data IDs (from def_wailing_caverns.h)
local TYPE_DISCIPLE = 4
local TYPE_MUTANUS  = 5
local DATA_NARALEX  = 6

-- Timer IDs
local TIMER_EVENT   = 1
local TIMER_SLEEP   = 2
local TIMER_POTION  = 3

-- custom_data keys
local DATA_WAYPOINT    = "wp"          -- last waypoint reached
local DATA_SUB_PHASE   = "sub"         -- sub-event phase counter
local DATA_YELLED      = "yelled"      -- aggro yell flag
local DATA_DONE        = "done"        -- ritual complete

------------------------------------------------------------------------
-- npc_disciple_of_naralex (entry 3592)
------------------------------------------------------------------------
local disciple = {}

function disciple:OnSpawn(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT,  value = -1 },
        { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_YELLED,    value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_DONE,      value = 0 },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP,  duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_POTION, duration = 5000 },
    }
end

function disciple:JustRespawned(input)
    return self:OnSpawn(input)
end

function disciple:OnEnterCombat(input)
    local actions = {}
    -- Say aggro line once
    if input:GetCustomData(DATA_YELLED) == 0 then
        table.insert(actions, { action = "SAY", text_id = SAY_ATTACKED })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_YELLED, value = 1 })
    end
    return actions
end

function disciple:OnEvade(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_YELLED, value = 0 },
    }
end

-- Fires when escort waypoint is reached (movement_type = waypoint, point_id = WP index)
function disciple:MovementInform(input, movement_type, point_id)
    local actions = {}
    -- Store waypoint
    table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT, value = point_id })
    table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 0 })

    if point_id == 0 then
        -- Say and pause; Event_Timer = 5000 → cast mark on players
        table.insert(actions, { action = "SAY", text_id = SAY_CAST_MARK })
        table.insert(actions, { action = "EMOTE", emote_id = 1 })  -- EMOTE_ONESHOT_TALK
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 5000 })

    elseif point_id == 7 then
        table.insert(actions, { action = "SAY", text_id = SAY_1ST_WP })
        table.insert(actions, { action = "EMOTE", emote_id = 1 })
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 6000 })

    elseif point_id == 15 then
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })

    elseif point_id == 26 then
        table.insert(actions, { action = "SAY", text_id = SAY_BEFORE_CHAMBER })
        table.insert(actions, { action = "EMOTE", emote_id = 25 })  -- EMOTE_ONESHOT_POINT
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })

    elseif point_id == 30 then
        -- Face Naralex (approximate — can't face a specific GUID)
        table.insert(actions, { action = "STOP_MOVEMENT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 1000 })
    end

    return actions
end

function disciple:OnUpdate(input)
    local actions = {}
    local wp = input:GetCustomData(DATA_WAYPOINT)

    -- Potion self-heal at <80% HP
    if input:IsTimerReady(TIMER_POTION) then
        if input.health_pct < 0.8 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_POTION, target = "self", triggered = true })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_POTION, duration = 45000 })
    end

    -- Combat: Sleep a random non-tank target
    if input.is_in_combat and input:IsTimerReady(TIMER_SLEEP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLEEP, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = 30000 })
    end

    -- Event timer processing by waypoint
    if not input:IsTimerReady(TIMER_EVENT) then return actions end

    local sub = input:GetCustomData(DATA_SUB_PHASE)

    if wp == 0 then
        -- Cast Mark on all nearby players (approximated: cast on highest threat target)
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MARK, target = "random_hostile", triggered = true })
        -- Resume escort
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT, value = -1 })

    elseif wp == 7 then
        if sub == 0 then
            -- Summon 2 raptors
            local p1 = { x = -67.851196, y = 214.383102, z = -93.499001 }
            local p2 = { x = -69.769707, y = 211.342804, z = -93.450737 }
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_DEVIATE_RAPTOR,
                x = p1.x, y = p1.y, z = p1.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_DEVIATE_RAPTOR,
                x = p2.x, y = p2.y, z = p2.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
        elseif sub == 1 then
            -- Check if raptors dead
            local raptors = input:GetCreaturesByEntry(MOB_DEVIATE_RAPTOR)
            if #raptors == 0 then
                table.insert(actions, { action = "SAY", text_id = SAY_AFTER_1ST_TRASH })
                table.insert(actions, { action = "EMOTE", emote_id = 25 })
                table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 2 })
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 5000 })
            else
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 1000 })
            end
        elseif sub == 2 then
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT, value = -1 })
        end

    elseif wp == 15 then
        if sub == 0 then
            table.insert(actions, { action = "SAY", text_id = SAY_BEFORE_CIRCLE })
            table.insert(actions, { action = "EMOTE", emote_id = 1 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
        elseif sub == 1 then
            -- Cast Cleansing (self-root)
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEANSING, target = "self" })
            table.insert(actions, { action = "SET_ROOT", rooted = true })
            table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = false })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 2 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 15000 })
        elseif sub == 2 then
            -- Summon 3 vipers mid-cast
            for i = 0, 2 do
                local p = MOB_POSITIONS[i + 1]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_DEVIATE_VIPER,
                    x = p.x, y = p.y, z = p.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            end
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 3 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 1000 })
        elseif sub == 3 then
            -- Wait for spell to finish
            if not input.is_casting then
                table.insert(actions, { action = "SET_ROOT", rooted = false })
                table.insert(actions, { action = "SET_COMBAT_MOVEMENT", enabled = true })
                table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 4 })
            end
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 1000 })
        elseif sub == 4 then
            -- Wait for combat to end
            if not input.is_in_combat then
                table.insert(actions, { action = "SAY", text_id = SAY_AFTER_CIRCLE })
                table.insert(actions, { action = "EMOTE", emote_id = 1 })
                table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 5 })
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
            else
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
            end
        elseif sub == 5 then
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT, value = -1 })
        end

    elseif wp == 26 then
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_WAYPOINT, value = -1 })

    elseif wp == 30 then
        -- Final ritual sequence (Naralex awakening)
        if sub == 0 then
            table.insert(actions, { action = "SAY", text_id = SAY_BEFORE_RITUAL })
            table.insert(actions, { action = "EMOTE", emote_id = 1 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
        elseif sub == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AWAKENING, target = "self" })
            table.insert(actions, { action = "EMOTE", emote_id = 1 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 2 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 4000 })
        elseif sub == 2 then
            -- TODO: say EMOTE_NARALEX_1 on Naralex NPC (requires cross-NPC targeting API)
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 3 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 5000 })
        elseif sub == 3 then
            -- Summon 3 Deviate Moccasins (positions 7, 8, 9)
            for i = 7, 9 do
                local p = MOB_POSITIONS[i + 1]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_DEVIATE_MOCCASIN,
                    x = p.x, y = p.y, z = p.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            end
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 4 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 40000 })
        elseif sub == 4 then
            -- Summon 7 Nightmare Ectoplasms (positions 3-9)
            for i = 3, 9 do
                local p = MOB_POSITIONS[i + 1]
                table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_NIGHTMARE_ECTOPLASM,
                    x = p.x, y = p.y, z = p.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            end
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 5 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 40000 })
        elseif sub == 5 then
            -- TODO: EMOTE_NARALEX_2 on Naralex
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 6 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 10000 })
        elseif sub == 6 then
            -- Summon Mutanus the Devourer
            local p = MOB_POSITIONS[4]  -- Position[3]
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_MUTANUS_DEVOURER,
                x = p.x, y = p.y, z = p.z, o = 0.0, summon_type = "DEAD_DESPAWN", duration = 0 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 7 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
        elseif sub == 7 then
            table.insert(actions, { action = "SAY", text_id = SAY_MUTANUS_SPAWNED })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 8 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
        elseif sub == 8 then
            -- Wait for Mutanus to be killed (TYPE_MUTANUS == DONE in instance data)
            local mutanus_done = input:GetInstanceData(TYPE_MUTANUS)
            if mutanus_done == 3 then  -- DONE = 3
                -- TODO: Naralex wakes up (say SAY_NARALEX_AWAKEN on Naralex)
                table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_AWAKENING })
                table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 9 })
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
            else
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
            end
        elseif sub == 9 then
            -- Wait for combat to clear
            if not input.is_in_combat then
                table.insert(actions, { action = "SAY", text_id = SAY_DISCIPLE_FINAL })
                table.insert(actions, { action = "EMOTE", emote_id = 1 })
                table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 10 })
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 5000 })
            else
                table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 2000 })
            end
        elseif sub == 10 then
            -- TODO: SAY_NARALEX_FINAL1 on Naralex
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 11 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 8000 })
        elseif sub == 11 then
            -- TODO: SAY_NARALEX_FINAL2 on Naralex; shapeshift both
            table.insert(actions, { action = "SAY", text_id = SAY_NARALEX_FINAL2 })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHAPESHIFT, target = "self", triggered = true })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_SUB_PHASE, value = 12 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_EVENT, duration = 8000 })
        elseif sub == 12 then
            -- Fly out: walk through intermediate points then despawn
            -- vmangos uses MOVE_FLY_MODE; approximate with MOVE_TO chain via MovementInform
            table.insert(actions, { action = "MOVE_TO", x = 101.0, y = 239.2, z = -91.2, run = false })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_DONE, value = 1 })
        end
    end

    return actions
end

-- After Mutanus dies, the instance script sets TYPE_MUTANUS = DONE.
-- The escort checks this in sub-phase 8.

RegisterCreatureAI(3592, disciple)

------------------------------------------------------------------------
-- npc_evolving_ectoplasm (entry 5763)
-- On spell hit: gains school immunity for 10s and changes morph.
-- Fire → red morph + fire immunity (7942)
-- Frost → blue morph + frost immunity (7940)
-- Nature → green morph + nature immunity (7941)
-- Shadow → black morph + shadow immunity (7743)
-- Note: spell school detection via spell_id range not perfectly available;
-- we use OnSpellHit and check spell_id against known school spells.
-- Full school detection requires API extension. Ported as best-effort.
------------------------------------------------------------------------
local SPELL_IMMUNE_FIRE    = 7942
local SPELL_IMMUNE_FROST   = 7940
local SPELL_IMMUNE_NATURE  = 7941
local SPELL_IMMUNE_SHADOW  = 7743
local SPELL_TRANSFORM_RED   = 7943
local SPELL_TRANSFORM_BLUE  = 7944
local SPELL_TRANSFORM_GREEN = 7945
local SPELL_TRANSFORM_BLACK = 7946

local TIMER_IMMUNE_EXPIRE = 1
local DATA_IMMUNE = "immune"

local evolving_ectoplasm = {}

function evolving_ectoplasm:OnSpellHit(input, spell_id, caster_guid)
    -- Can't detect spell school from spell_id alone without DB lookup.
    -- TODO(api): expose spell school in OnSpellHit input so we can branch by school.
    -- For now, just track that we got hit and apply a generic immunity timer.
    if input:GetCustomData(DATA_IMMUNE) == 1 then return {} end
    return {}
end

function evolving_ectoplasm:OnUpdate(input)
    if input:GetCustomData(DATA_IMMUNE) ~= 1 then return {} end
    if not input:IsTimerReady(TIMER_IMMUNE_EXPIRE) then return {} end

    -- Immunity expired: remove all transform/immune auras
    return {
        { action = "REMOVE_AURA", spell_id = SPELL_IMMUNE_FIRE },
        { action = "REMOVE_AURA", spell_id = SPELL_IMMUNE_FROST },
        { action = "REMOVE_AURA", spell_id = SPELL_IMMUNE_NATURE },
        { action = "REMOVE_AURA", spell_id = SPELL_IMMUNE_SHADOW },
        { action = "REMOVE_AURA", spell_id = SPELL_TRANSFORM_RED },
        { action = "REMOVE_AURA", spell_id = SPELL_TRANSFORM_BLUE },
        { action = "REMOVE_AURA", spell_id = SPELL_TRANSFORM_GREEN },
        { action = "REMOVE_AURA", spell_id = SPELL_TRANSFORM_BLACK },
        { action = "SET_CUSTOM_DATA", key = DATA_IMMUNE, value = 0 },
    }
end

RegisterCreatureAI(5763, evolving_ectoplasm)
