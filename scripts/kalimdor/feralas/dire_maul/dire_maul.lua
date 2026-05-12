--[[
    Dire Maul dungeon helper scripts.
    Ported from vmangos instance_dire_maul.cpp + dreadsteed_ritual.cpp

    Contents:
    - npc_gordok_brute (entry 11441): Bruising Blow above 30% HP; Pummel; Uppercut;
      Enrage + Back Hand below 30% HP.

    Skipped (requires GO scripting API):
    - go_broken_trap / go_fixed_trap (GO levers): GOHello interactions (Phase E GO API)
    - Dreadsteed ritual (dreadsteed_ritual.cpp): Entirely GO-driven ceremony with
      five ritual crystals (GOs 177257-179505), Force Field (179503), Magic Vortex (179506).
      Requires GO activate + spell channel orchestration — blocked on Phase E.

    Skipped (requires gossip API):
    - npc_mizzle_the_crafty (entry 14353): Spawned after King Gordok dies, offers
      gossip menu that buffs players. Requires OnGossipHello/Select (Phase D).
    - npc_knot_thimblejack (entry 14338): Gossip/quest reward NPC.
      Requires gossip callbacks (Phase D).

    NOTE: Guard AI (Moldar 14326, Fengus 14321, Slipkik 14323, Kromcrush 14325) is
    handled by the instance script (instance_dire_maul.lua); not re-scripted here.
]]

-- Spell IDs
local SPELL_BACK_HAND     = 6253
local SPELL_ENRAGE_GB     = 15716
local SPELL_BRUISING_BLOW = 22572
local SPELL_PUMMEL        = 15615
local SPELL_UPPERCUT_GB   = 18072

-- Timer IDs
local TIMER_GB_BACKHAND    = 1
local TIMER_GB_BRUISING    = 2
local TIMER_GB_PUMMEL      = 3
local TIMER_GB_UPPERCUT    = 4

-- custom_data keys
local DATA_GB_ENRAGED = "gb_enraged"

------------------------------------------------------------------------
-- npc_gordok_brute (entry 11441)
-- Above 30% HP: Bruising Blow every 3-8s, Pummel when target casting (8-10s CD).
-- Below 30% HP: Enrage (once) + Back Hand every 5-9s.
-- Uppercut (18072) every 6-10s regardless of HP threshold.
------------------------------------------------------------------------
local gordok_brute = {}

function gordok_brute:OnSpawn(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_GB_ENRAGED, value = 0 },
    }
end

function gordok_brute:JustRespawned(input)
    return self:OnSpawn(input)
end

function gordok_brute:OnEnterCombat(input)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_GB_ENRAGED, value = 0 },
        { action = "SET_TIMER", timer_id = TIMER_GB_BACKHAND, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_GB_BRUISING, duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_GB_PUMMEL,   duration = 5000 },
        { action = "SET_TIMER", timer_id = TIMER_GB_UPPERCUT, duration = 0 },
    }
end

function gordok_brute:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    -- Enrage + switch to Back Hand at <30% HP
    if input.health_pct < 30.0 and input:GetCustomData(DATA_GB_ENRAGED) == 0 then
        table.insert(actions, { action = "EMOTE", text_id = 9413 })  -- EMOTE_ENRAGE
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE_GB, target = "self", triggered = true })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_GB_ENRAGED, value = 1 })
    end

    -- Back Hand (only below 30%)
    if input.health_pct < 30.0 and input:IsTimerReady(TIMER_GB_BACKHAND) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BACK_HAND, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GB_BACKHAND, duration = 5000 + math.random(0, 4000) })
    end

    -- Bruising Blow (only above 30%)
    if input.health_pct > 30.0 and input:IsTimerReady(TIMER_GB_BRUISING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BRUISING_BLOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GB_BRUISING, duration = 3000 + math.random(0, 5000) })
    end

    -- Pummel (only above 30%)
    -- NOTE: vmangos checks if target is casting; we cast it on timer regardless (no casting check available)
    if input.health_pct > 30.0 and input:IsTimerReady(TIMER_GB_PUMMEL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PUMMEL, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GB_PUMMEL, duration = 8000 + math.random(0, 2000) })
    end

    -- Uppercut (always)
    if input:IsTimerReady(TIMER_GB_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT_GB, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GB_UPPERCUT, duration = 6000 + math.random(0, 4000) })
    end

    return actions
end

RegisterCreatureAI(11441, gordok_brute)
