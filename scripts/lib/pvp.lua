--[[
    @script_type: library
    @name: lib_pvp
    @priority: 2

    Helpers shared by battleground and world-PvP objective scripts.

    Objective scripts receive an `input` table describing their scope:
      input.players       list of { guid, name, team, x, y, z, alive, level,
                          class, race, area_id, is_gm, auras = { [spell] = true } }
      input.world_states  { [state_id] = value } set by this scope
      input.events        { [event1] = event2 } active battleground spawn events
      input.battleground  { type, status, bracket, min_level, max_level,
                          elapsed_ms, weekend, winner, alliance_players,
                          horde_players } (battleground copies only)
      input.data / input:GetData(id), input:GetGuid(id), input.timers
]]

TEAM_ALLIANCE = 469
TEAM_HORDE = 67

-- Battleground spawn event slots shared by every battleground.
BG_EVENT_DOOR = 254
BG_EVENT_GHOST_GATE = 253

PVP = {}

-- The player row for a guid, or nil.
function PVP.player(input, guid)
    for _, p in ipairs(input.players or {}) do
        if p.guid == guid then
            return p
        end
    end
    return nil
end

function PVP.team_of(input, guid)
    local p = PVP.player(input, guid)
    return p and p.team or 0
end

function PVP.other_team(team)
    if team == TEAM_ALLIANCE then
        return TEAM_HORDE
    end
    return TEAM_ALLIANCE
end

-- "ALLIANCE"/"HORDE" announcement colour for a team.
function PVP.chat_of(team)
    if team == TEAM_ALLIANCE then
        return "ALLIANCE"
    end
    return "HORDE"
end

function PVP.status(input)
    return input.battleground and input.battleground.status or ""
end

function PVP.in_progress(input)
    return PVP.status(input) == "IN_PROGRESS"
end

-- 1-based bracket index for per-bracket reward tables.
function PVP.bracket(input)
    return (input.battleground and input.battleground.bracket or 0) + 1
end

function PVP.weekend(input)
    return input.battleground ~= nil and input.battleground.weekend == true
end

function PVP.distance2d(p, x, y)
    local dx, dy = p.x - x, p.y - y
    return math.sqrt(dx * dx + dy * dy)
end

function PVP.distance(p, x, y, z)
    local dx, dy, dz = p.x - x, p.y - y, p.z - z
    return math.sqrt(dx * dx + dy * dy + dz * dz)
end

-- Count living, non-GM players of each team within `radius` of a point.
function PVP.count_near(input, x, y, z, radius)
    local alliance, horde = 0, 0
    local inside = {}
    for _, p in ipairs(input.players or {}) do
        if p.alive and not p.is_gm and PVP.distance(p, x, y, z) <= radius then
            if p.team == TEAM_ALLIANCE then
                alliance = alliance + 1
            elseif p.team == TEAM_HORDE then
                horde = horde + 1
            end
            table.insert(inside, p)
        end
    end
    return alliance, horde, inside
end

-- An empty guid, to clear a stored guid with SET_GUID.
PVP.NO_GUID = 0

function PVP.guid_is_set(guid)
    return guid ~= nil and not guid:IsEmpty()
end

-- Action builders.
function PVP.world_state(id, value)
    return { action = "SET_WORLD_STATE", state_id = id, value = value }
end

function PVP.data(id, value)
    return { action = "SET_DATA", data_id = id, value = value }
end

function PVP.announce(text_id, chat, player, team)
    return { action = "ANNOUNCE", text_id = text_id, chat = chat or "NEUTRAL", player = player, team = team }
end

function PVP.announce_text(text, chat, player, team)
    return { action = "ANNOUNCE", text = text, chat = chat or "NEUTRAL", player = player, team = team }
end

function PVP.sound(sound_id, team)
    return { action = "PLAY_SOUND_TO_SCOPE", sound_id = sound_id, team = team }
end

function PVP.event(event1, event2, spawn)
    if spawn == nil then
        spawn = true
    end
    return { action = "SPAWN_EVENT", event1 = event1, event2 = event2 or 0, spawn = spawn }
end

function PVP.honor_team(team, amount)
    return { action = "REWARD_HONOR_TO_TEAM", team = team, amount = amount }
end

function PVP.reputation_team(team, faction, amount)
    return { action = "REWARD_REPUTATION_TO_TEAM", team = team, faction_id = faction, amount = amount }
end

function PVP.score(player, column, value)
    return { action = "UPDATE_SCORE", player = player, column = column, value = value or 1 }
end

-- Append every action of `more` to `actions`.
function PVP.extend(actions, more)
    for _, a in ipairs(more) do
        table.insert(actions, a)
    end
    return actions
end
