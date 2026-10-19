--[[
    @script_type: zone
    @zone_id: 1377
    @name: world_pvp_silithus

    Silithus world PvP: carry Silithyst to your faction's camp.

    Players pick up Silithyst at geysers and mounds and carry it as an aura.
    Bringing it to their camp's turn-in counts it for their faction; the
    first faction to deliver 200 wins Cenarion Favor for everyone of theirs
    in the zone, and the count starts over. Losing the aura anywhere else
    drops a mound the other side may pick up.

    Uses the same objective hooks as the battlegrounds: area triggers,
    gameobject use, a watched aura and zone world states.
]]

local SPELL_SILITHYST = 29519
local SPELL_TRACES_OF_SILITHYST = 29534
local SPELL_CENARION_FAVOR = 30754
local SPELL_HONOR_199 = 31420
local SPELL_CAP_REWARD = 31247

local GO_SILITHYST_MOUND = 181597
local GO_SILITHYST_GEYSER = 181598

local AT_ALLIANCE = 4162
local AT_HORDE = 4168
local TURN_IN = {
    [TEAM_ALLIANCE] = { trigger = AT_ALLIANCE, x = -7142.04, y = 1397.92, z = 4.33 },
    [TEAM_HORDE] = { trigger = AT_HORDE, x = -7588.48, y = 756.81, z = -16.42 },
}
local QUEST_CREDIT = { [TEAM_ALLIANCE] = 17090, [TEAM_HORDE] = 18199 }

local WS_GATHERED_A = 2313
local WS_GATHERED_H = 2314
local WS_MAX = 2317
local MAX_RESOURCES = 200

local D_GATHERED = { [TEAM_ALLIANCE] = 1, [TEAM_HORDE] = 2 }
local D_LAST_CONTROLLER = 3

-- Announcer lines at 25%, 50%, 75% and 100%.
local YELLS = {
    [TEAM_ALLIANCE] = { [50] = 13479, [100] = 13480, [150] = 13481, [200] = 13470 },
    [TEAM_HORDE] = { [50] = 13476, [100] = 13477, [150] = 13478, [200] = 13469 },
}

local silithus = {
    WatchedAuras = { SPELL_SILITHYST },
}

local function gathered(input, team)
    return input:GetData(D_GATHERED[team])
end

local function world_states(input, a, h)
    return {
        PVP.world_state(WS_GATHERED_A, a),
        PVP.world_state(WS_GATHERED_H, h),
        PVP.world_state(WS_MAX, MAX_RESOURCES),
    }
end

function silithus:OnPlayerEnter(input, player)
    local actions = world_states(input, gathered(input, TEAM_ALLIANCE), gathered(input, TEAM_HORDE))
    if PVP.team_of(input, player) == input:GetData(D_LAST_CONTROLLER) then
        table.insert(actions, { action = "PLAYER_CAST_SPELL", player = player, spell_id = SPELL_CENARION_FAVOR })
    end
    return actions
end

function silithus:OnPlayerLeave(input, player)
    return { { action = "PLAYER_REMOVE_AURA", player = player, spell_id = SPELL_CENARION_FAVOR } }
end

-- Geysers and dropped mounds hand out Silithyst.
function silithus:OnGameObjectUse(input, player, go, entry)
    if entry ~= GO_SILITHYST_MOUND and entry ~= GO_SILITHYST_GEYSER then
        return {}
    end
    local p = PVP.player(input, player)
    if p == nil or p.auras[SPELL_SILITHYST] then
        return true
    end
    local actions = { { action = "PLAYER_CAST_SPELL", player = player, spell_id = SPELL_SILITHYST } }
    if entry == GO_SILITHYST_MOUND then
        table.insert(actions, { action = "DESPAWN_GAMEOBJECT", guid = go })
    end
    return actions
end

function silithus:OnAreaTrigger(input, player, trigger)
    if trigger ~= AT_ALLIANCE and trigger ~= AT_HORDE then
        return false
    end
    local p = PVP.player(input, player)
    if p == nil or not p.auras[SPELL_SILITHYST] then
        return false
    end
    local team = p.team
    if TURN_IN[team] == nil or TURN_IN[team].trigger ~= trigger then
        -- The other side's camp keeps its Silithyst.
        return true
    end

    local count = gathered(input, team) + 1
    local actions = {
        { action = "KILL_CREDIT", player = player, entry = QUEST_CREDIT[team] },
        { action = "PLAYER_REMOVE_AURA", player = player, spell_id = SPELL_SILITHYST },
        { action = "PLAYER_CAST_SPELL", player = player, spell_id = SPELL_TRACES_OF_SILITHYST },
        { action = "PLAYER_CAST_SPELL", player = player, spell_id = SPELL_HONOR_199 },
        { action = "PLAYER_CAST_SPELL", player = player, spell_id = SPELL_CAP_REWARD },
    }
    if YELLS[team][count] then
        table.insert(actions, PVP.announce(YELLS[team][count], PVP.chat_of(team)))
    end
    local a, h = gathered(input, TEAM_ALLIANCE), gathered(input, TEAM_HORDE)
    if count >= MAX_RESOURCES then
        -- Cenarion Favor for the winners; both counts start over.
        table.insert(actions, { action = "CAST_SPELL_ON_TEAM", spell_id = SPELL_CENARION_FAVOR, team = team })
        table.insert(actions, PVP.data(D_LAST_CONTROLLER, team))
        table.insert(actions, PVP.announce_text(
            (team == TEAM_ALLIANCE and "The Alliance" or "The Horde")
                .. " has collected 200 silithyst!", "SYSTEM"))
        table.insert(actions, PVP.data(D_GATHERED[TEAM_ALLIANCE], 0))
        table.insert(actions, PVP.data(D_GATHERED[TEAM_HORDE], 0))
        a, h = 0, 0
    else
        table.insert(actions, PVP.data(D_GATHERED[team], count))
        if team == TEAM_ALLIANCE then a = count else h = count end
    end
    PVP.extend(actions, world_states(input, a, h))
    return actions
end

-- Silithyst lost away from a turn-in falls to the ground as a mound.
function silithus:OnAuraRemoved(input, player, spell)
    if spell ~= SPELL_SILITHYST then
        return {}
    end
    local p = PVP.player(input, player)
    if p == nil then
        return {}
    end
    local camp = TURN_IN[p.team]
    if camp and PVP.distance(p, camp.x, camp.y, camp.z) <= 5 then
        return {}
    end
    return {
        { action = "SPAWN_GAMEOBJECT", entry = GO_SILITHYST_MOUND, x = p.x, y = p.y, z = p.z, o = 0, duration = 0 },
    }
end

RegisterZoneScript(1377, silithus)
