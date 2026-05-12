--[[
    @script_type: creature_ai
    @entry: 4857
    @name: uldaman

    Uldaman dungeon helper scripts.
    Ported from vmangos uldaman.cpp

    Contents:
    - mob_stone_keeper   (entry 4857):  Trample cast on timer
    - mob_jadespine_basilisk (entry 7097): Crystalline Slumber + threat clear + target swap
    - mob_annora         (entry 6172):  Hidden NPC; becomes visible + moves when all nearby scorpions (7078) dead

    Skipped (requires GO/event API not yet available):
    - go_keystone_chamber (GameObject interaction → instance data IRONAYA_DOOR = DONE)
    - ProcessEventId_event_awaken_stone_keeper (scripted event trigger)
    - spell_uldaman_awaken_vault_warder (SpellScript, limits targets to 2)
]]

------------------------------------------------------------------------
-- mob_stone_keeper (entry 4857)
-- Trample (5568) on 4-10s timer. Evade is suppressed if hostile nearby.
------------------------------------------------------------------------
local SPELL_TRAMPLE = 5568

local NPC_STONE_KEEPER_ENTRY = 4857

local TIMER_TRAMPLE = 1

local stone_keeper = {}

function stone_keeper:OnEnterCombat(input)
    local timer = 4000 + math.random(0, 5000)
    return {
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = timer },
    }
end

function stone_keeper:OnUpdate(input)
    if not input.is_in_combat then return {} end
    if not input:IsTimerReady(TIMER_TRAMPLE) then return {} end

    local timer = 4000 + math.random(0, 6000)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_TRAMPLE, target = "current_target" },
        { action = "SET_TIMER", timer_id = TIMER_TRAMPLE, duration = timer },
    }
end

RegisterCreatureAI(4857, stone_keeper)

------------------------------------------------------------------------
-- mob_jadespine_basilisk (entry 7097)
-- Crystalline Slumber (3636) every ~28s: casts on current target,
-- wipes their threat, then switches to next top-aggro target.
------------------------------------------------------------------------
local SPELL_CRYSTALLINE_SLUMBER = 3636

local TIMER_SLUMBER = 1

local jadespine_basilisk = {}

function jadespine_basilisk:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_SLUMBER, duration = 2000 },
    }
end

function jadespine_basilisk:OnUpdate(input)
    if not input.is_in_combat then return {} end
    if not input:IsTimerReady(TIMER_SLUMBER) then return {} end

    local actions = {}

    -- Cast Crystalline Slumber on current target, wipe their threat
    table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CRYSTALLINE_SLUMBER, target = "current_target" })
    -- Drop the sleeping target's threat so we switch to someone else
    if input.current_target then
        table.insert(actions, { action = "MODIFY_THREAT", target = input.current_target, percent = -100 })
    end
    -- Reset timer
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLUMBER, duration = 28000 })

    return actions
end

RegisterCreatureAI(7097, jadespine_basilisk)

------------------------------------------------------------------------
-- mob_annora (entry 6172)
-- Spawns invisible and immune. Watches for all nearby Sandfury Scorpions
-- (entry 7078) within 30 yards to die, then becomes visible and moves
-- to her scripted position.
------------------------------------------------------------------------
local NPC_SCORPION = 7078
-- Position from vmangos: -164.3657, 210.7687, -49.572
local ANNORA_DEST_X = -164.3657
local ANNORA_DEST_Y =  210.7687
local ANNORA_DEST_Z =  -49.572

local TIMER_CHECK_SCORPIONS = 1
local CHECK_INTERVAL = 2000  -- poll every 2s

-- custom_data keys
local DATA_SPAWNED = "spawned"

local annora = {}

function annora:OnSpawn(input)
    return {
        { action = "SET_REACT_STATE", state = "PASSIVE" },
        { action = "SET_IMMUNE", physical = true, spell = true },
        { action = "SET_TIMER", timer_id = TIMER_CHECK_SCORPIONS, duration = CHECK_INTERVAL },
        { action = "SET_CUSTOM_DATA", key = DATA_SPAWNED, value = 0 },
    }
end

function annora:JustRespawned(input)
    return self:OnSpawn(input)
end

function annora:OnUpdate(input)
    -- Once revealed, stop checking
    if input:GetCustomData(DATA_SPAWNED) == 1 then return {} end
    if not input:IsTimerReady(TIMER_CHECK_SCORPIONS) then return {} end

    -- Count alive scorpions nearby
    local scorpions = input:GetCreaturesByEntry(NPC_SCORPION)
    local alive_count = #scorpions

    if alive_count == 0 then
        -- All dead: reveal Annora and move to position
        return {
            { action = "SET_IMMUNE", physical = false, spell = false },
            { action = "SET_REACT_STATE", state = "AGGRESSIVE" },
            { action = "MOVE_TO", x = ANNORA_DEST_X, y = ANNORA_DEST_Y, z = ANNORA_DEST_Z, run = false },
            { action = "SET_CUSTOM_DATA", key = DATA_SPAWNED, value = 1 },
        }
    else
        -- Still alive, keep polling
        return {
            { action = "SET_TIMER", timer_id = TIMER_CHECK_SCORPIONS, duration = CHECK_INTERVAL },
        }
    end
end

RegisterCreatureAI(6172, annora)
