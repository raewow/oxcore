--[[
    Eastern Plaguelands zone scripts
    Reference: eastern_kingdoms/eastern_plaguelands/eastern_plaguelands.cpp

    go_mark_of_detonation (GO entry 177668)
        OnEffectDummy: spell 19250 (Placing Smokey's Explosives) hits the mark.
        Award kill credit for nearby Trigger Scourge Structure (entry 12247).

    npc_eris_havenfire (quest 7622 — Balance of Light and Shadow)
        OnQuestAccept: starts the Havenfire event (complex AI-driven event
        with peasant spawns, waves, and timers; deferred — requires UpdateAI).

    npc_joseph_redpath
        OnGossipHello: sends gossip menu 3861 and triggers credit/event
        (complex AI-driven Darrowshire event; event trigger deferred).

    npc_the_scourge_cauldron, npc_andorhal_tower, npc_highprotectorlorik,
    npc_demetria, npc_guard_didier, npc_caravan_mule — UpdateAI-driven, deferred.
]]

-- go_mark_of_detonation
local GO_MARK_OF_DETONATION        = 177668
local SPELL_PLACING_SMOKIES_EXPLO  = 19250
local NPC_TRIGGER_SCOURGE_STRUCT   = 12247

local go_mark_of_detonation = {}

function go_mark_of_detonation:OnEffectDummy(caster_guid, spell_id, eff_index, go_guid)
    if spell_id == SPELL_PLACING_SMOKIES_EXPLO and eff_index == 0 then
        -- Award kill credit for the nearest Trigger: Scourge Structure.
        -- The C++ version finds the creature, awards KilledMonsterCredit,
        -- deals lethal damage, and fires a GO fire sequence from a GUID map.
        -- The kill-credit and kill-damage are the scriptable portion here.
        return {
            { action = "KILL_CREDIT_NEAREST_CREATURE",
              creature_entry = NPC_TRIGGER_SCOURGE_STRUCT,
              search_radius  = 8.0 },
        }
    end
    return {}
end

RegisterEffectDummyScript(GO_MARK_OF_DETONATION, go_mark_of_detonation)

-- npc_eris_havenfire (entry 14500 — Eris Havenfire)
-- Quest 7622 — Balance of Light and Shadow
local QUEST_BALANCE_OF_LIGHT = 7622
local NPC_ERIS_HAVENFIRE     = 14494

local npc_eris_havenfire = {}

function npc_eris_havenfire:OnQuestAccept(player, npc_guid, quest_id)
    if quest_id == QUEST_BALANCE_OF_LIGHT then
        -- The C++ version starts the full Havenfire escort/wave event via
        -- creature AI. That requires UpdateAI support. Stub fires.
        return {}
    end
    return {}
end

RegisterGossipScript(NPC_ERIS_HAVENFIRE, npc_eris_havenfire)

-- npc_joseph_redpath (entry 16384 — Joseph Redpath)
-- GossipHello: menu 3861; triggers Darrowshire event.
local NPC_JOSEPH_REDPATH     = 10936
local GOSSIP_MENU_REDPATH    = 3861

local npc_joseph_redpath = {}

function npc_joseph_redpath:OnGossipHello(player, npc_guid)
    -- The C++ version sends gossip menu 3861 and, if quest 5211 is incomplete,
    -- awards KilledMonsterCredit and calls BeginEvent on creature AI.
    -- BeginEvent is UpdateAI-driven; stub the gossip open here.
    return {
        { action = "SEND_GOSSIP_MENU", menu_id = GOSSIP_MENU_REDPATH },
    }
end

RegisterGossipScript(NPC_JOSEPH_REDPATH, npc_joseph_redpath)
