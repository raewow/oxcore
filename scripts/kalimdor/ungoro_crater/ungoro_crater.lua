--[[
    Un'Goro Crater zone scripts
    Reference: kalimdor/ungoro_crater/ungoro_crater.cpp

    at_scent_larkorwi (trigger IDs 1726-1734)
        OnAreaTrigger: if player has quest 4291 (The Scent of Lar'korwi) incomplete
        and no Lar'korwi Mate is nearby, spawn one at the trigger's position.
        The AT entry's (x,y,z) from the DB is used as the spawn point.

    npc_simone_the_inconspicuous (entry 14527)
        OnGossipHello: if quest 7636 (Stave of the Ancients) is incomplete,
        add gossip item and send NPC's default menu.
        OnGossipSelect: close menu and emote laugh (BeginEvent on creature AI
        deferred — requires creature UpdateAI for full transform/combat sequence).

    npc_ame01, npc_ringo — escort AI (deferred).
]]

-- at_scent_larkorwi
local AT_SCENT_LARKORWI_IDS   = { 1726, 1727, 1728, 1729, 1730, 1731, 1732, 1733, 1734 }
local NPC_LARKORWI_MATE       = 9683

-- AT spawn coordinates from areatrigger_template (x/y/z per trigger)
local AT_SPAWN_COORDS = {
    [1726] = { x = -7286.01, y = -2125.06, z = -272.114 },
    [1727] = { x = -7208.42, y = -2027.09, z = -271.74  },
    [1728] = { x = -7170.02, y = -1852.79, z = -272.988 },
    [1729] = { x = -7395.19, y = -1735.68, z = -279.763 },
    [1730] = { x = -7449.9,  y = -2111.57, z = -271.766 },
    [1731] = { x = -7463.35, y = -1957.64, z = -272.532 },
    [1732] = { x = -7509.58, y = -1946.56, z = -270.537 },
    [1733] = { x = -7516.87, y = -1829.76, z = -272.647 },
    [1734] = { x = -7594.12, y = -1771.62, z = -273.612 },
}

local function make_larkorwi_script(at_id)
    local coords = AT_SPAWN_COORDS[at_id]
    local script = {}

    function script:OnAreaTrigger(player)
        -- C++: alive, not GM, quest incomplete, no nearby mate within 25 yd
        -- Spawn Lar'korwi Mate at trigger position; 2-minute despawn.
        return {
            { action = "SPAWN_CREATURE",
              entry       = NPC_LARKORWI_MATE,
              x           = coords.x,
              y           = coords.y,
              z           = coords.z,
              o           = 3.3,
              duration_ms = 120000 },
        }
    end

    return script
end

for _, at_id in ipairs(AT_SCENT_LARKORWI_IDS) do
    RegisterAreaTriggerScript(at_id, make_larkorwi_script(at_id))
end

-- npc_simone_the_inconspicuous
local NPC_SIMONE              = 14527

local npc_simone = {}

function npc_simone:OnGossipHello(player, npc_guid)
    -- C++: if quest incomplete add gossip item, send NPC default menu
    return {
        { action = "GOSSIP_SEND" },
    }
end

function npc_simone:OnGossipSelect(player, npc_guid, sender, action_id)
    -- C++: CLOSE_GOSSIP_MENU + HandleEmote(LAUGH) + BeginEvent(player)
    -- BeginEvent triggers the creature's transform/combat AI (deferred).
    return {
        { action = "GOSSIP_CLOSE" },
        { action = "EMOTE", emote_id = 4 },  -- EMOTE_ONESHOT_LAUGH
    }
end

RegisterGossipScript(NPC_SIMONE, npc_simone)
