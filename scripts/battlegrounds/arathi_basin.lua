--[[
    @script_type: instance
    @map_id: 529
    @name: battleground_arathi_basin

    Arathi Basin: hold bases to gather 2000 resources.

    Clicking a banner assaults a base; if the defenders do not click it back
    within a minute, it is taken. Each side gains resources faster the more
    bases it holds (10 every 12, 9, 6 and 3 seconds for one to four bases,
    30 a second for all five). Resources also bring honour and reputation.
]]

local NODES = 5
local NODE_NAMES = { "Stables", "Blacksmith", "Farm", "Lumber Mill", "Gold Mine" }

-- Node states (also the event2 of each node's banner event; event1 is the node).
local NEUTRAL = 0
local ALLY_CONTESTED = 1
local HORDE_CONTESTED = 2
local ALLY_OCCUPIED = 3
local HORDE_OCCUPIED = 4

-- World states.
local WS_NODE_STATES = { 1767, 1782, 1772, 1792, 1787 }
local WS_NODE_ICONS = { 1842, 1846, 1845, 1844, 1843 }
local STATE_OFFSET = { [ALLY_CONTESTED] = 2, [HORDE_CONTESTED] = 3, [ALLY_OCCUPIED] = 0, [HORDE_OCCUPIED] = 1 }
local WS_BASES_HORDE = 1778
local WS_BASES_ALLY = 1779
local WS_RESOURCES_ALLY = 1776
local WS_RESOURCES_HORDE = 1777
local WS_RESOURCES_MAX = 1780
local WS_RESOURCES_WARNING = 1955
local WS_UNKNOWN = 1861

local MAX_SCORE = 2000
local NEAR_VICTORY = 1800
local CAPTURE_MS = 60000
local TICK_INTERVALS = { [1] = 12000, [2] = 9000, [3] = 6000, [4] = 3000, [5] = 1000 }
local TICK_POINTS = { [1] = 10, [2] = 10, [3] = 10, [4] = 10, [5] = 30 }

-- Script data: node states 10..14, previous states 20..24.
local function D_NODE(node) return 10 + node end
local function D_PREV(node) return 20 + node end
local function T_NODE(node) return 10 + node end
local D_SCORE = { [TEAM_ALLIANCE] = 1, [TEAM_HORDE] = 2 }
local D_LAST_TICK = { [TEAM_ALLIANCE] = 3, [TEAM_HORDE] = 4 }
local D_HONOR_TICS = { [TEAM_ALLIANCE] = 5, [TEAM_HORDE] = 6 }
local D_REP_TICS = { [TEAM_ALLIANCE] = 7, [TEAM_HORDE] = 8 }
local D_NEAR_VICTORY = 9

-- Rewards.
local HONOR_PER_TICK = { 41, 68, 113, 189, 198 }
local HONOR_WIN = { 41, 68, 113, 189, 198 }
local HONOR_TICS = 330
local HONOR_TICS_WEEKEND = 200
local REP_TICS = 200
local REP_TICS_WEEKEND = 150
local FACTION = { [TEAM_ALLIANCE] = 509, [TEAM_HORDE] = 510 }
local SPELL_REWARD_4_BASES = 24061
local SPELL_REWARD_5_BASES = 24064
local KILL_CREDITS = { 15001, 15002, 15003, 15004, 15005 }

-- Area triggers.
local AT_ALLIANCE_EXIT = 3948
local AT_HORDE_EXIT = 3949

-- Graveyards (WorldSafeLocs) and where they are.
local GY_NODES = {
    { id = 895, x = 1201.87, y = 1163.13 },
    { id = 894, x = 1016.59, y = 955.18 },
    { id = 893, x = 834.73, y = 784.98 },
    { id = 897, x = 772.76, y = 1213.11 },
    { id = 896, x = 1211.52, y = 781.56 },
}
local GY_START = { [TEAM_ALLIANCE] = 898, [TEAM_HORDE] = 899 }
local GY_ENTRANCE = { [TEAM_ALLIANCE] = 890, [TEAM_HORDE] = 889 }

-- Buffs at each base.
local BUFF_ENTRIES = { 179871, 179904, 179905 }
local BUFF_POSITIONS = {
    { 1185.566, 1184.629, -56.36329, 2.303831 },
    { 989.939026, 1008.75, -42.60327, 0.8203033 },
    { 818.0089, 842.3543, -56.54062, 3.176533 },
    { 808.8463, 1185.417, 11.92161, 5.619962 },
    { 1147.091, 816.8362, -98.39896, 6.056293 },
}

-- Broadcast texts and sounds.
local TEXT_ONE_MINUTE = 10477
local TEXT_HALF_MINUTE = 10478
local TEXT_BEGUN = 10479
local TEXT_ALLIANCE_NEAR_VICTORY = 10598
local TEXT_HORDE_NEAR_VICTORY = 10599
local TEXT_ALLIANCE_WINS = 10633
local TEXT_HORDE_WINS = 10634
local SOUND_CLAIMED = 8192
local SOUND_CAPTURED = { [TEAM_ALLIANCE] = 8173, [TEAM_HORDE] = 8213 }
local SOUND_ASSAULTED = { [TEAM_ALLIANCE] = 8212, [TEAM_HORDE] = 8174 }
local SOUND_NEAR_VICTORY = { [TEAM_ALLIANCE] = 8457, [TEAM_HORDE] = 8456 }

local COLUMN_ASSAULTED = 0
local COLUMN_DEFENDED = 1

local ab = {
    ScoreColumns = 2,
}

local function team_name(team)
    return team == TEAM_ALLIANCE and "Alliance" or "Horde"
end

local function contested(team)
    return team == TEAM_ALLIANCE and ALLY_CONTESTED or HORDE_CONTESTED
end

local function occupied(team)
    return team == TEAM_ALLIANCE and ALLY_OCCUPIED or HORDE_OCCUPIED
end

-- The team a (non-neutral) state belongs to.
local function owner(state)
    if state == ALLY_CONTESTED or state == ALLY_OCCUPIED then
        return TEAM_ALLIANCE
    elseif state == HORDE_CONTESTED or state == HORDE_OCCUPIED then
        return TEAM_HORDE
    end
    return 0
end

-- Node states after pending changes (`changes[node] = new state`).
local function states(input, changes)
    local s = {}
    for node = 0, NODES - 1 do
        s[node] = changes[node] or input:GetData(D_NODE(node))
    end
    return s
end

-- Move a node to a new state: banner, map icons, base counts.
local function set_node(actions, input, changes, node, new_state)
    local old_state = changes[node] or input:GetData(D_NODE(node))
    changes[node] = new_state
    table.insert(actions, PVP.data(D_PREV(node), old_state))
    table.insert(actions, PVP.data(D_NODE(node), new_state))
    table.insert(actions, PVP.event(node, new_state, true))
    local base = WS_NODE_STATES[node + 1]
    if old_state == NEUTRAL then
        table.insert(actions, PVP.world_state(WS_NODE_ICONS[node + 1], 0))
    else
        table.insert(actions, PVP.world_state(base + STATE_OFFSET[old_state], 0))
    end
    table.insert(actions, PVP.world_state(base + STATE_OFFSET[new_state], 1))
    local now = states(input, changes)
    local ally, horde = 0, 0
    for n = 0, NODES - 1 do
        if now[n] == ALLY_OCCUPIED then
            ally = ally + 1
        elseif now[n] == HORDE_OCCUPIED then
            horde = horde + 1
        end
    end
    table.insert(actions, PVP.world_state(WS_BASES_ALLY, ally))
    table.insert(actions, PVP.world_state(WS_BASES_HORDE, horde))
end

-- A base became `team`'s: 4 and 5 held bases complete the quests.
local function occupied_rewards(actions, input, changes, team)
    local now = states(input, changes)
    local held = 0
    for n = 0, NODES - 1 do
        if now[n] == occupied(team) and (changes[n] ~= nil or input:IsTimerReady(T_NODE(n))) then
            held = held + 1
        end
    end
    if held >= 5 then
        table.insert(actions, { action = "CAST_SPELL_ON_TEAM", spell_id = SPELL_REWARD_5_BASES, team = team })
    end
    if held >= 4 then
        table.insert(actions, { action = "CAST_SPELL_ON_TEAM", spell_id = SPELL_REWARD_4_BASES, team = team })
    end
end

function ab:OnLoad(input)
    local actions = {
        PVP.event(BG_EVENT_DOOR, 0, true),
        PVP.event(BG_EVENT_GHOST_GATE, 0, true),
        PVP.world_state(WS_RESOURCES_MAX, MAX_SCORE),
        PVP.world_state(WS_RESOURCES_WARNING, NEAR_VICTORY),
        PVP.world_state(WS_RESOURCES_ALLY, 0),
        PVP.world_state(WS_RESOURCES_HORDE, 0),
        PVP.world_state(WS_BASES_ALLY, 0),
        PVP.world_state(WS_BASES_HORDE, 0),
        PVP.world_state(WS_UNKNOWN, 2),
    }
    for node = 0, NODES - 1 do
        table.insert(actions, PVP.event(node, NEUTRAL, true))
        table.insert(actions, PVP.world_state(WS_NODE_ICONS[node + 1], 1))
        for _, offset in pairs(STATE_OFFSET) do
            table.insert(actions, PVP.world_state(WS_NODE_STATES[node + 1] + offset, 0))
        end
    end
    return actions
end

function ab:OnMatchEvent(input, event)
    if event == "ONE_MINUTE" then
        return { PVP.announce(TEXT_ONE_MINUTE) }
    elseif event == "HALF_MINUTE" then
        return { PVP.announce(TEXT_HALF_MINUTE) }
    elseif event == "BEGIN" then
        local actions = {
            { action = "DOOR_EVENT", event1 = BG_EVENT_DOOR, event2 = 0, open = true },
            PVP.event(BG_EVENT_GHOST_GATE, 0, false),
            PVP.announce(TEXT_BEGUN),
        }
        for i = 1, NODES do
            local p = BUFF_POSITIONS[i]
            table.insert(actions, {
                action = "SPAWN_GAMEOBJECT", entry = BUFF_ENTRIES[math.random(1, 3)],
                x = p[1], y = p[2], z = p[3], o = p[4], duration = 0,
            })
        end
        return actions
    elseif event == "DOORS_DESPAWN" then
        return { PVP.event(BG_EVENT_DOOR, 0, false) }
    end
    return {}
end

-- Clicking a base's banner.
function ab:OnGameObjectUse(input, player, go, entry, event1, event2)
    if not PVP.in_progress(input) or event1 >= NODES then
        return {}
    end
    local node = event1
    local team = PVP.team_of(input, player)
    local state = input:GetData(D_NODE(node))
    -- Only a neutral base or one the other side holds or contests may be clicked.
    if state ~= NEUTRAL and owner(state) == team then
        return {}
    end
    local name = NODE_NAMES[node + 1]
    local chat = PVP.chat_of(team)
    local actions = { { action = "KILL_CREDIT", player = player, entry = KILL_CREDITS[node + 1] } }
    local changes = {}
    local prev = input:GetData(D_PREV(node))
    local sound
    if state == NEUTRAL then
        table.insert(actions, PVP.score(player, COLUMN_ASSAULTED))
        set_node(actions, input, changes, node, contested(team))
        table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(node), duration = CAPTURE_MS })
        table.insert(actions, PVP.announce_text(
            "$n claims the " .. name .. "! If left unchallenged, the " .. team_name(team)
                .. " will control it in 1 minute!", chat, player))
        sound = SOUND_CLAIMED
    elseif (state == ALLY_CONTESTED or state == HORDE_CONTESTED) and prev < ALLY_OCCUPIED then
        -- Contested from neutral: the other side takes over the claim.
        table.insert(actions, PVP.score(player, COLUMN_ASSAULTED))
        set_node(actions, input, changes, node, contested(team))
        table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(node), duration = CAPTURE_MS })
        table.insert(actions, PVP.announce_text("$n has assaulted the " .. name .. "!", chat, player))
        sound = SOUND_ASSAULTED[team]
    elseif state == ALLY_CONTESTED or state == HORDE_CONTESTED then
        -- Contested from our own occupation: defended, ours again at once.
        table.insert(actions, PVP.score(player, COLUMN_DEFENDED))
        set_node(actions, input, changes, node, occupied(team))
        table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(node), duration = 0 })
        occupied_rewards(actions, input, changes, team)
        table.insert(actions, PVP.announce_text("$n has defended the " .. name .. "!", chat, player))
        table.insert(actions, PVP.announce_text(
            "The " .. team_name(team) .. " has taken the " .. name .. "!", chat))
        sound = SOUND_ASSAULTED[team]
    else
        -- Held by the other side: assault it.
        table.insert(actions, PVP.score(player, COLUMN_ASSAULTED))
        set_node(actions, input, changes, node, contested(team))
        table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(node), duration = CAPTURE_MS })
        table.insert(actions, PVP.announce_text("$n has assaulted the " .. name .. "!", chat, player))
        sound = SOUND_ASSAULTED[team]
    end
    table.insert(actions, PVP.sound(sound))
    -- Ghosts waiting at a graveyard that changed hands look for another.
    if owner(state) ~= 0 and state >= ALLY_OCCUPIED then
        table.insert(actions, { action = "REPOP_TEAM", team = owner(state) })
    end
    return actions
end

function ab:Update(input)
    if not PVP.in_progress(input) then
        return {}
    end
    local actions = {}
    local changes = {}
    local diff = input.diff_ms or 0

    -- A minute after a claim the base is taken.
    for node = 0, NODES - 1 do
        local state = input:GetData(D_NODE(node))
        if (state == ALLY_CONTESTED or state == HORDE_CONTESTED) and input.timers[T_NODE(node)] == 0 then
            local team = owner(state)
            set_node(actions, input, changes, node, occupied(team))
            table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(node), duration = 0 })
            occupied_rewards(actions, input, changes, team)
            table.insert(actions, PVP.announce_text(
                "The " .. team_name(team) .. " has taken the " .. NODE_NAMES[node + 1] .. "!", PVP.chat_of(team)))
            table.insert(actions, PVP.sound(SOUND_CAPTURED[team]))
        end
    end

    -- Resources from held bases.
    local now = states(input, changes)
    local winner = nil
    for _, team in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
        local held = 0
        for n = 0, NODES - 1 do
            if now[n] == occupied(team) then
                held = held + 1
            end
        end
        if held > 0 then
            local last = input:GetData(D_LAST_TICK[team]) + diff
            if last > TICK_INTERVALS[held] then
                last = last - TICK_INTERVALS[held]
                local points = TICK_POINTS[held]
                local score = math.min(input:GetData(D_SCORE[team]) + points, MAX_SCORE)
                local honor_tics = input:GetData(D_HONOR_TICS[team]) + points
                local rep_tics = input:GetData(D_REP_TICS[team]) + points
                local rep_step = PVP.weekend(input) and REP_TICS_WEEKEND or REP_TICS
                local honor_step = PVP.weekend(input) and HONOR_TICS_WEEKEND or HONOR_TICS
                if rep_tics >= rep_step then
                    table.insert(actions, PVP.reputation_team(team, FACTION[team], 10))
                    rep_tics = rep_tics - rep_step
                end
                if honor_tics >= honor_step then
                    table.insert(actions, PVP.honor_team(team, HONOR_PER_TICK[math.min(PVP.bracket(input), 5)]))
                    honor_tics = honor_tics - honor_step
                end
                if input:GetData(D_NEAR_VICTORY) == 0 and score > NEAR_VICTORY then
                    table.insert(actions, PVP.data(D_NEAR_VICTORY, 1))
                    table.insert(actions, PVP.announce(
                        team == TEAM_ALLIANCE and TEXT_ALLIANCE_NEAR_VICTORY or TEXT_HORDE_NEAR_VICTORY))
                    table.insert(actions, PVP.sound(SOUND_NEAR_VICTORY[team]))
                end
                table.insert(actions, PVP.data(D_SCORE[team], score))
                table.insert(actions, PVP.data(D_HONOR_TICS[team], honor_tics))
                table.insert(actions, PVP.data(D_REP_TICS[team], rep_tics))
                table.insert(actions, PVP.world_state(
                    team == TEAM_ALLIANCE and WS_RESOURCES_ALLY or WS_RESOURCES_HORDE, score))
                if score >= MAX_SCORE and winner == nil then
                    winner = team
                end
            end
            table.insert(actions, PVP.data(D_LAST_TICK[team], last))
        end
    end

    if winner then
        local honor = HONOR_WIN[math.min(PVP.bracket(input), 5)]
        if PVP.weekend(input) then
            table.insert(actions, PVP.honor_team(winner, honor))
        end
        table.insert(actions, PVP.honor_team(winner, honor))
        table.insert(actions, PVP.announce(winner == TEAM_ALLIANCE and TEXT_ALLIANCE_WINS or TEXT_HORDE_WINS))
        table.insert(actions, { action = "END_MATCH", winner = winner })
    end
    return actions
end

function ab:OnAreaTrigger(input, player, trigger)
    local team = PVP.team_of(input, player)
    if (trigger == AT_ALLIANCE_EXIT and team == TEAM_ALLIANCE) or (trigger == AT_HORDE_EXIT and team == TEAM_HORDE) then
        return { { action = "LEAVE_BATTLEGROUND", player = player } }
    end
    return false
end

-- Before the gates open, the dead return to the entrance; afterwards to the
-- nearest base their side holds, else their start.
function ab:SelectGraveyard(input, player)
    local p = PVP.player(input, player)
    local team = p and p.team or TEAM_ALLIANCE
    if not PVP.in_progress(input) and not (p and p.is_gm) then
        return GY_ENTRANCE[team]
    end
    local best, best_distance = nil, nil
    for node = 0, NODES - 1 do
        if input:GetData(D_NODE(node)) == occupied(team) and p then
            local gy = GY_NODES[node + 1]
            local d = PVP.distance2d(p, gy.x, gy.y)
            if best_distance == nil or d < best_distance then
                best, best_distance = gy.id, d
            end
        end
    end
    return best or GY_START[team]
end

RegisterInstanceScript(529, ab)
