--[[
    The Barrens zone scripts
    Reference: kalimdor/the_barrens/the_barrens.cpp

    at_twiggy_flathead (trigger ID 522)
        OnAreaTrigger: if player has quest 1719 (The Affray) incomplete and
        Twiggy Flathead (entry 6248) is nearby, the event can start.
        The C++ checks NPC AI state; we fire the trigger and let the area
        trigger quest system handle progression.

    event_the_principle_source (event ID 5246)
        OnProcessEventId: spawn 3x Burning Blade Toxicologist (entry 12319)
        at fixed coords and make them attack the player.

    npc_gilthares, npc_wizzlecranks_shredder — escort AI (deferred).
    event_the_conterattack — commented out in reference (not registered).
]]

-- at_twiggy_flathead
local AT_TWIGGY_FLATHEAD = 522
local QUEST_THE_AFFRAY   = 1719
local NPC_TWIGGY         = 6248

local at_twiggy_flathead = {}

function at_twiggy_flathead:OnAreaTrigger(player)
    -- The C++ checks quest incomplete and NPC AI CanStartEvent().
    -- We fire the trigger; the quest system handles further progression.
    return {}
end

RegisterAreaTriggerScript(AT_TWIGGY_FLATHEAD, at_twiggy_flathead)

-- event_the_principle_source
local EVENT_THE_PRINCIPLE_SOURCE       = 5246
local NPC_BURNING_BLADE_TOXICOLOGIST   = 12319

local event_principle_source = {}

function event_principle_source:OnProcessEventId(player, event_id, source_guid, is_start)
    if event_id ~= EVENT_THE_PRINCIPLE_SOURCE then
        return {}
    end
    -- Spawn 3 Burning Blade Toxicologists and attack the player.
    return {
        { action = "SPAWN_CREATURE",
          entry = NPC_BURNING_BLADE_TOXICOLOGIST,
          x = 331.52, y = -2270.94, z = 242.21, o = 5.15,
          duration_ms = 120000, attack_player = true },
        { action = "SPAWN_CREATURE",
          entry = NPC_BURNING_BLADE_TOXICOLOGIST,
          x = 332.09, y = -2291.26, z = 241.86, o = 1.05,
          duration_ms = 120000, attack_player = true },
        { action = "SPAWN_CREATURE",
          entry = NPC_BURNING_BLADE_TOXICOLOGIST,
          x = 345.97, y = -2282.66, z = 241.77, o = 3.16,
          duration_ms = 120000, attack_player = true },
    }
end

RegisterProcessEventScript(EVENT_THE_PRINCIPLE_SOURCE, event_principle_source)
