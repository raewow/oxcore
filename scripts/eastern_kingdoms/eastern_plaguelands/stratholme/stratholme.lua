--[[
    @script_type: creature_ai
    @entry: 17814
    @name: stratholme

    Stratholme dungeon helper scripts.
    Ported from vmangos stratholme.cpp

    Contents:
    - mob_freed_soul             (entry 17814): random say on spawn
    - mob_restless_soul          (entry 11122): dies when hit by Egan's Blaster (17368)
    - mobs_spectral_ghostly_citizen (entries 17772, 17773):
          Haunting Phantom (16336) in combat; tagged by Egan's Blaster then dies;
          spawns restless souls on death; emote reactions (dance/rude/wave/bow/kiss)
          NOTE: ReceiveEmote callback not yet in Lua API — emote reactions are stubbed.
    - mobs_cristal_zuggurat      (entry 17312): dies when all nearby acolytes (10399) dead

    Skipped (requires GO/spell-script API):
    - go_gauntlet_gate          (GO interaction → TYPE_BARON_RUN = IN_PROGRESS)
    - go_stratholme_postbox     (GO interaction → spawns 3 undead postmen 11142)
    - go_supply_crate           (GO interaction → spawns plagued critters)
    - spell_haunting_phantoms   (AuraScript: 5% periodic proc to summon phantoms 16334/16335)
    - spell_eye_of_naxxramas    (SpellScript: summon gargoyle attacks target)
]]

------------------------------------------------------------------------
-- mob_freed_soul (entry 17814)
-- Says one of 5 random lines when it spawns.
------------------------------------------------------------------------
local FREED_SOUL_TEXTS = { 6451, 6452, 6453, 6454, 6455 }

local freed_soul = {}

function freed_soul:OnSpawn(input)
    local idx = math.random(1, #FREED_SOUL_TEXTS)
    return {
        { action = "SAY", text_id = FREED_SOUL_TEXTS[idx] },
    }
end

function freed_soul:JustRespawned(input)
    return self:OnSpawn(input)
end

RegisterCreatureAI(17814, freed_soul)

------------------------------------------------------------------------
-- mob_restless_soul (entry 11122)
-- Tagged when hit by Egan's Blaster (17368) while player has quest 5282.
-- After being tagged: kill self after 5s, spawn mob_freed_soul (17814) on death.
-- The freed soul casts SPELL_SOUL_FREED (17370) and follows the tagger.
-- NOTE: quest status check not available in OnSpellHit; tag anyone who hits with 17368.
------------------------------------------------------------------------
local SPELL_EGAN_BLASTER  = 17368
local SPELL_SOUL_FREED    = 17370
local NPC_FREED_SOUL      = 11136

local TIMER_DIE  = 1

local DATA_TAGGED  = "tagged"
local DATA_TAGGER  = "tagger"  -- stores low 32 bits of tagger GUID (i64)

local restless_soul = {}

function restless_soul:OnSpellHit(input, spell_id, caster_guid)
    if spell_id ~= SPELL_EGAN_BLASTER then return {} end
    if input:GetCustomData(DATA_TAGGED) == 1 then return {} end

    return {
        { action = "SET_CUSTOM_DATA", key = DATA_TAGGED, value = 1 },
        { action = "SET_TIMER", timer_id = TIMER_DIE, duration = 5000 },
    }
end

function restless_soul:OnUpdate(input)
    if input:GetCustomData(DATA_TAGGED) ~= 1 then return {} end
    if not input:IsTimerReady(TIMER_DIE) then return {} end

    -- Kill self; OnDeath will handle freed soul spawn
    return {
        { action = "KILL_SELF" },
    }
end

function restless_soul:OnDeath(input, killer_guid)
    if input:GetCustomData(DATA_TAGGED) ~= 1 then return {} end

    -- Spawn freed soul at our position
    return {
        { action = "SPAWN_CREATURE",
          entry = NPC_FREED_SOUL,
          x = input.position.x,
          y = input.position.y,
          z = input.position.z,
          o = input.position.o,
          summon_type = "TIMED_DESPAWN",
          duration = 300000 },
    }
end

RegisterCreatureAI(11122, restless_soul)

------------------------------------------------------------------------
-- mobs_spectral_ghostly_citizen (entries 17772, 17773)
-- In combat: casts Haunting Phantom (16336) every 20s.
-- Tagged by Egan's Blaster (17368): kill self after 5s, spawn 1-4 restless
--   souls (entry 11122) at random nearby positions (probability decreases).
-- Emote reactions (DANCE/RUDE/WAVE/BOW/KISS):
--   TODO(api): ReceiveEmote callback not yet available in Lua API.
--   DANCE in combat: evade once
--   RUDE: cast Slap (6754) if in melee range, else rude emote
--   WAVE: wave emote
--   BOW: bow emote
--   KISS: flex emote
------------------------------------------------------------------------
local SPELL_HAUNTING_PHANTOM = 16336
local SPELL_SLAP             = 6754
local NPC_RESTLESS_SOUL      = 11122

local TIMER_HAUNTING = 1
local TIMER_TAGGED_DIE = 2

local DATA_SGC_TAGGED = "tagged"

local spectral_citizen = {}

function spectral_citizen:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_HAUNTING, duration = 20000 },
    }
end

function spectral_citizen:OnSpellHit(input, spell_id, caster_guid)
    if spell_id ~= SPELL_EGAN_BLASTER then return {} end
    if input:GetCustomData(DATA_SGC_TAGGED) == 1 then return {} end

    return {
        { action = "SET_CUSTOM_DATA", key = DATA_SGC_TAGGED, value = 1 },
        { action = "SET_TIMER", timer_id = TIMER_TAGGED_DIE, duration = 5000 },
    }
end

function spectral_citizen:OnUpdate(input)
    local actions = {}

    -- Tagged die timer
    if input:GetCustomData(DATA_SGC_TAGGED) == 1 and input:IsTimerReady(TIMER_TAGGED_DIE) then
        table.insert(actions, { action = "KILL_SELF" })
        return actions
    end

    if not input.is_in_combat then return actions end

    -- Haunting Phantom
    if input:IsTimerReady(TIMER_HAUNTING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HAUNTING_PHANTOM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HAUNTING, duration = 20000 })
    end

    return actions
end

function spectral_citizen:OnDeath(input, killer_guid)
    if input:GetCustomData(DATA_SGC_TAGGED) ~= 1 then return {} end

    -- Spawn 1-4 restless souls with decreasing probability (100%, 50%, 33%, 25%)
    -- vmangos: for i=1..4, spawn if urand(1,i)==1
    local actions = {}
    for i = 1, 4 do
        if math.random(1, i) == 1 then
            local rx = input.position.x + (math.random() - 0.5) * 40
            local ry = input.position.y + (math.random() - 0.5) * 40
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = NPC_RESTLESS_SOUL,
                x = rx,
                y = ry,
                z = input.position.z,
                o = 0.0,
                summon_type = "CORPSE_DESPAWN",
                duration = 600000,
            })
        end
    end
    return actions
end

RegisterCreatureAI(17772, spectral_citizen)
RegisterCreatureAI(17773, spectral_citizen)

------------------------------------------------------------------------
-- mobs_cristal_zuggurat (entry 17312)
-- Watches for all nearby acolytes (entry 10399) within 50 yards.
-- When all acolytes are dead: kill self (yells and sets instance data
-- are handled by OnDeath in the instance script).
------------------------------------------------------------------------
local NPC_ACOLYTE = 10399
local TIMER_CHECK_ACOLYTES = 1
local ACOLYTE_CHECK_INTERVAL = 2000

-- custom_data: 0 = haven't seen acolytes yet / not init, 1 = watching, 2 = done
local DATA_PHASE = "phase"

local cristal_zuggurat = {}

function cristal_zuggurat:OnSpawn(input)
    return {
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_TIMER", timer_id = TIMER_CHECK_ACOLYTES, duration = ACOLYTE_CHECK_INTERVAL },
        { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 0 },
    }
end

function cristal_zuggurat:JustRespawned(input)
    return self:OnSpawn(input)
end

function cristal_zuggurat:OnUpdate(input)
    if input:GetCustomData(DATA_PHASE) == 2 then return {} end
    if not input:IsTimerReady(TIMER_CHECK_ACOLYTES) then return {} end

    local acolytes = input:GetCreaturesByEntry(NPC_ACOLYTE)

    -- Phase 0: wait until we see at least one acolyte spawned
    if input:GetCustomData(DATA_PHASE) == 0 then
        if #acolytes > 0 then
            return {
                { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 1 },
                { action = "SET_TIMER", timer_id = TIMER_CHECK_ACOLYTES, duration = ACOLYTE_CHECK_INTERVAL },
            }
        else
            return {
                { action = "SET_TIMER", timer_id = TIMER_CHECK_ACOLYTES, duration = ACOLYTE_CHECK_INTERVAL },
            }
        end
    end

    -- Phase 1: watching — kill self when all acolytes dead
    if #acolytes == 0 then
        return {
            { action = "SET_CUSTOM_DATA", key = DATA_PHASE, value = 2 },
            { action = "KILL_SELF" },
        }
    else
        return {
            { action = "SET_TIMER", timer_id = TIMER_CHECK_ACOLYTES, duration = ACOLYTE_CHECK_INTERVAL },
        }
    end
end

RegisterCreatureAI(17312, cristal_zuggurat)
