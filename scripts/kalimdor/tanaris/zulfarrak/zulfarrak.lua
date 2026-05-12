--[[
    Zul'Farrak dungeon helper scripts.
    Ported from vmangos zulfarrak.cpp

    Contents:
    - npc_sergeant_bly    (entry 7604): quest event leader; turns hostile after gossip trigger
    - npc_weegli_blastfuse (entry 7607): goblin combat AI + door-destruction sequence

    Skipped (requires GO/gossip API):
    - go_troll_cage:    GOHello → releases Bly and crew, moves them to stair positions
    - OnGossipHello/Select for npc_sergeant_bly: gossip menu triggers turn-hostile sequence
    - OnGossipHello/Select for npc_weegli_blastfuse: gossip menu triggers door-blow sequence
    - go_shallow_grave: GOHello → summon zombie/dead hero with chance rolls
    - The Weegli door-destruction sequence (requires GO summon + GO activate APIs)

    NOTE: Weegli's companion-assist (ensuring Oro/Murta/Bly attack the same target)
    requires cross-NPC targeting which isn't available. Simplified to individual combat AI.
]]

-- NPC entries
local ENTRY_BLY   = 7604
local ENTRY_RAVEN = 7605
local ENTRY_ORO   = 7606
local ENTRY_WEEGLI = 7607
local ENTRY_MURTA = 7608

-- Faction IDs
local FACTION_HOSTILE  = 14
local FACTION_FRIENDLY = 35
local FACTION_FREED    = 250

-- Text IDs
local SAY_BLY_1      = 3882
local SAY_BLY_2      = 3884
local SAY_WEEGLI_OHNO    = 3744
local SAY_WEEGLI_OK_I_GO = 3785

-- Spell IDs
local SPELL_SHIELD_BASH = 11972
local SPELL_REVENGE     = 12170
local SPELL_BOMB        = 8858
local SPELL_SHOOT       = 6660

-- Timer IDs
local TIMER_TEXT        = 1
local TIMER_SHIELD_BASH = 2
local TIMER_REVENGE     = 3
local TIMER_BOMB        = 4

-- custom_data keys
local DATA_GOSSIP_STEP = "gossip_step"  -- 0=idle, 1=say1, 2=say2, 3=turn hostile

-- Instance data IDs (from zulfarrak.h)
local EVENT_PYRAMID = 1
local PYRAMID_KILLED_ALL_TROLLS = 8

------------------------------------------------------------------------
-- npc_sergeant_bly (entry 7604)
-- Normally friendly. After gossip option (phase D blocked), begins a
-- delayed turn-hostile sequence: say SAY_1, wait 5s, say SAY_2, wait 5s,
-- set faction hostile and attack player.
-- Since gossip triggers require Phase D, the sequence can be initiated
-- via instance data EVENT_PYRAMID == PYRAMID_KILLED_ALL_TROLLS.
-- The hostile turn is started externally by the cage-release event.
------------------------------------------------------------------------
local bly = {}

function bly:OnSpawn(input)
    return {
        { action = "SET_FACTION", faction_id = FACTION_FRIENDLY },
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_CUSTOM_DATA", key = DATA_GOSSIP_STEP, value = 0 },
    }
end

function bly:JustRespawned(input)
    return self:OnSpawn(input)
end

function bly:OnEnterCombat(input)
    -- Once hostile, standard combat timers
    return {
        { action = "SET_TIMER", timer_id = TIMER_SHIELD_BASH, duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_REVENGE,     duration = 8000 },
    }
end

function bly:OnUpdate(input)
    local actions = {}
    local step = input:GetCustomData(DATA_GOSSIP_STEP)

    -- Post-gossip turn-hostile sequence (initiated externally)
    if step >= 1 and step <= 3 and input:IsTimerReady(TIMER_TEXT) then
        if step == 1 then
            table.insert(actions, { action = "SAY", text_id = SAY_BLY_1 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_GOSSIP_STEP, value = 2 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TEXT, duration = 5000 })
        elseif step == 2 then
            table.insert(actions, { action = "SAY", text_id = SAY_BLY_2 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_GOSSIP_STEP, value = 3 })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TEXT, duration = 5000 })
        elseif step == 3 then
            -- Turn hostile
            table.insert(actions, { action = "SET_FACTION", faction_id = FACTION_HOSTILE })
            table.insert(actions, { action = "SET_REACT_STATE", state = "AGGRESSIVE" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_GOSSIP_STEP, value = 0 })
            -- Also turn Raven, Oro, Murta hostile (by entry, affects all nearby)
            -- NOTE: SET_FACTION by entry is not available; relies on individual scripts or instance
        end
        return actions
    end

    -- Combat abilities
    if not input.is_in_combat then return actions end

    if input:IsTimerReady(TIMER_SHIELD_BASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHIELD_BASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHIELD_BASH, duration = 15000 })
    end

    if input:IsTimerReady(TIMER_REVENGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_REVENGE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REVENGE, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(7604, bly)

------------------------------------------------------------------------
-- npc_weegli_blastfuse (entry 7607)
-- Combat AI: Bomb every 10s, Shoot when target out of melee range.
-- After all trolls killed (EVENT_PYRAMID == PYRAMID_KILLED_ALL_TROLLS):
--   heals Oro, Murta, Bly, Raven to full.
-- Door destruction sequence (via gossip — blocked on Phase D):
--   Moves to door position, sets explosive, runs away.
-- Simplified: port combat AI only; door sequence stubbed.
------------------------------------------------------------------------
local weegli = {}

function weegli:OnSpawn(input)
    return {
        { action = "SET_FACTION", faction_id = FACTION_FRIENDLY },
        { action = "SET_REACT_STATE", state = "DEFENSIVE" },
    }
end

function weegli:JustRespawned(input)
    return self:OnSpawn(input)
end

function weegli:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BOMB,  duration = 10000 },
    }
end

function weegli:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_BOMB) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BOMB, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BOMB, duration = 10000 })
    end

    -- Shoot if target is not in melee range (use farthest threat target as proxy)
    if #input.threat_list > 0 then
        local target = input.threat_list[1]
        if target and target.distance and target.distance > 5.0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOOT, target = target.guid })
        end
    end

    return actions
end

-- TODO(api): Gossip trigger for DestroyDoor sequence requires Phase D gossip API.
-- When triggered: move to (1858.57, 1146.35, 14.745), say SAY_WEEGLI_OK_I_GO,
-- then move to (1863.77, 1176.99, 9.993), blow door, run to (1827.1, 1184.0, 8.993) + despawn.

RegisterCreatureAI(7607, weegli)
