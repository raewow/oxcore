--[[
    @script_type: instance
    @map_id: 30
    @name: battleground_alterac_valley

    Alterac Valley: the battle ends when a side's general falls.

    Graveyards and towers change hands by banner: an assault takes five
    minutes to complete unless the owners click the banner back. A taken
    graveyard becomes the attackers'; a taken tower is burned, costing its
    side 75 reinforcements. Killing a captain costs 100 and stops the
    captain's buff; every death costs one. Mines fall to whoever kills their
    boss, and feed their owner a reinforcement every 45 seconds.

    Not modelled: the armour-scrap upgrades of defenders, the air, cavalry
    and world-boss assaults raised by quest turn-ins, and the landmines.
]]

local NODES = 15
local NODE_NAMES = {
    [0] = "Stormpike Aid Station", "Stormpike Graveyard", "Stonehearth Graveyard",
    "Snowfall Graveyard", "Iceblood Graveyard", "Frostwolf Graveyard", "Frostwolf Relief Hut",
    "Dun Baldar South Bunker", "Dun Baldar North Bunker", "Icewing Bunker", "Stonehearth Bunker",
    "Iceblood Tower", "Tower Point", "East Frostwolf Tower", "West Frostwolf Tower",
}
local SNOWFALL = 3
local STONEHEARTH_BUNKER = 10
local function is_tower(node) return node >= 7 end

-- Owners: index into world-state and event tables.
local A, H, N = 0, 1, 2
local ASSAULTED, CONTROLLED = 0, 1
local function team_index(team) return team == TEAM_HORDE and H or A end
local function index_team(idx) return idx == H and TEAM_HORDE or TEAM_ALLIANCE end

-- Spawn event slots.
local EV_MINE_BOSS = 46            -- + mine
local EV_CAPTAIN_A = 48
local EV_CAPTAIN_H = 49
local EV_MINE = 50                 -- + mine
local EV_HERALD = 60
local EV_BOSS_A = 61
local EV_BOSS_H = 62
local EV_CAPTAIN_DEAD_A = 63
local EV_CAPTAIN_DEAD_H = 64
local EV_SNIVVLE = 65
local EV_EXPLOSIVES_A = 66
local EV_EXPLOSIVES_H = 67
local EV_LIEUTENANT_A = 68
local EV_LIEUTENANT_H = 69
local EV_LANDMINES_H = 100
local EV_LANDMINES_A = 101
local COMMANDERS_A = { [52] = true, [53] = true, [54] = true, [55] = true }
local COMMANDERS_H = { [56] = true, [57] = true, [58] = true, [59] = true }

-- World states: per node, [owner * 2 + state].
local NODE_STATES = {
    [0] = { 1326, 1325, 1328, 1327 }, { 1335, 1333, 1336, 1334 }, { 1304, 1302, 1303, 1301 },
    { 1343, 1341, 1344, 1342 }, { 1348, 1346, 1349, 1347 }, { 1339, 1337, 1340, 1338 },
    { 1331, 1329, 1332, 1330 }, { 1375, 1361, 1378, 1370 }, { 1374, 1362, 1379, 1371 },
    { 1376, 1363, 1380, 1372 }, { 1377, 1364, 1381, 1373 }, { 1390, 1368, 1395, 1385 },
    { 1389, 1367, 1394, 1384 }, { 1388, 1366, 1393, 1383 }, { 1387, 1365, 1392, 1382 },
}
local WS_SNOWFALL_NEUTRAL = 1966
local WS_SCORE = { [A] = 3127, [H] = 3128 }
local WS_SHOW_SCORE = { [A] = 3134, [H] = 3133 }
local MINE_STATES = { [0] = { [A] = 1358, [H] = 1359, [N] = 1360 }, [1] = { [A] = 1355, [H] = 1356, [N] = 1357 } }
local MINE_NAMES = { [0] = "Irondeep", [1] = "Coldtooth" }

local INITIAL_POINTS = 600
local CAPTURE_MS = 300000
local MINE_TICK_MS = 45000
local MINE_RECLAIM_MS = 1200000
local RES_TOWER = 75
local RES_CAPTAIN = 100

-- Script data.
local function D_OWNER(n) return 100 + n end
local function D_STATE(n) return 120 + n end
local function D_TOTAL(n) return 140 + n end
local function D_PREV_OWNER(n) return 160 + n end
local function D_PREV_STATE(n) return 180 + n end
local function T_NODE(n) return 1 + n end
local D_SCORE = { [A] = 1, [H] = 2 }
local function D_MINE_OWNER(m) return 10 + m end
local function D_MINE_PREV(m) return 12 + m end
local function T_MINE_TICK(m) return 20 + m end
local function T_MINE_RECLAIM(m) return 22 + m end
local T_BUFF = { [A] = 30, [H] = 31 }
local D_CAPTAIN_DEAD = { [A] = 30, [H] = 31 }

-- Rewards.
local FACTION = { [A] = 730, [H] = 729 }
local KILL_BOSS, KILL_CAPTAIN, KILL_COMMANDER, KILL_TOWER = 6, 3, 1, 2
local KILL_SURVIVING_TOWER, KILL_SURVIVING_CAPTAIN, KILL_SURVIVING_GRAVE, KILL_SURVIVING_MINE = 3, 3, 1, 1
local REP = {
    boss = { 350, 525 }, captain = { 125, 185 }, commander = { 12, 18 }, tower = { 12, 18 },
    surviving_tower = { 12, 18 }, surviving_captain = { 125, 175 }, grave = { 12, 18 }, mine = { 24, 36 },
}
local SPELL_BOSS_KILLED = 23658
local SPELL_CAPTAIN_BUFF = { [A] = 23693, [H] = 22751 }

-- Graveyards (WorldSafeLocs) for nodes 0..6 and each side's cave.
local GRAVEYARDS = {
    [0] = { id = 751, x = 643.0, y = 44.0 }, { id = 689, x = 676.0, y = -374.0 },
    { id = 729, x = 73.42, y = -496.43 }, { id = 169, x = -157.41, y = 31.21 },
    { id = 749, x = -531.22, y = -405.23 }, { id = 690, x = -1090.48, y = -253.31 },
    { id = 750, x = -1496.07, y = -333.34 },
}
local CAVE = { [A] = { id = 611, x = 873.0, y = -491.28 }, [H] = { id = 610, x = -818.56, y = -619.25 } }

-- Area triggers.
local AT_ALLIANCE_EXIT = 2608
local AT_HORDE_EXIT = 2606

-- Broadcast texts and sounds.
local TEXT_ONE_MINUTE = 10638
local TEXT_HALF_MINUTE = 10639
local TEXT_BEGUN = 10640
local TEXT_ALLIANCE_WINS = 7335
local TEXT_HORDE_WINS = 7336
local SOUND_ASSAULTS = { [A] = 8212, [H] = 8174 }
local SOUND_GOOD = { [A] = 8173, [H] = 8213 }
local SOUND_TOWER_DEFENDED = 8192

-- Scoreboard columns.
local COL_GRAVES_ASSAULTED, COL_GRAVES_DEFENDED = 0, 1
local COL_TOWERS_ASSAULTED, COL_TOWERS_DEFENDED = 2, 3

local av = {
    ScoreColumns = 7,
}

local function team_name(idx) return idx == A and "Alliance" or "Horde" end

local function rep(input, kind)
    return REP[kind][PVP.weekend(input) and 2 or 1]
end

-- Bonus honour: as many honourable kills of a rank 1 enemy at the bracket's top level.
local function bonus_honor(input, kills)
    local level = input.battleground and input.battleground.max_level or 60
    local coeff = 0.1212
    if level >= 60 then coeff = 1.0
    elseif level >= 50 then coeff = 0.9545
    elseif level >= 40 then coeff = 0.5707
    elseif level >= 30 then coeff = 0.3434
    elseif level >= 20 then coeff = 0.2070 end
    return math.floor(kills * coeff * 188.3 * math.exp(0.05331))
end

-- Matches ending in under an hour give less of the objective honour.
local function honor_modifier(input)
    local hours = (input.battleground and input.battleground.elapsed_ms or 0) / 3600000
    if hours < 1 then
        return 60 ^ (hours - 1)
    end
    return 1
end

-- Reinforcement changes. `changes.score` tracks pending values within one callback.
local function score_of(input, changes, idx)
    return changes.score[idx] or input:GetData(D_SCORE[idx])
end

local function add_score(actions, input, changes, idx, points)
    local score = math.max(score_of(input, changes, idx) + points, 0)
    changes.score[idx] = score
    table.insert(actions, PVP.data(D_SCORE[idx], score))
    table.insert(actions, PVP.world_state(WS_SCORE[idx], score))
end

local function new_changes()
    return { score = {}, node = {} }
end

-- A node's state, pending changes first.
local function node(input, changes, n)
    local c = changes.node[n]
    if c then
        return c
    end
    return {
        owner = input:GetData(D_OWNER(n)),
        state = input:GetData(D_STATE(n)),
        total = input:GetData(D_TOTAL(n)),
        prev_owner = input:GetData(D_PREV_OWNER(n)),
        prev_state = input:GetData(D_PREV_STATE(n)),
    }
end

local function store(actions, changes, n, s)
    changes.node[n] = s
    table.insert(actions, PVP.data(D_OWNER(n), s.owner))
    table.insert(actions, PVP.data(D_STATE(n), s.state))
    table.insert(actions, PVP.data(D_TOTAL(n), s.total))
    table.insert(actions, PVP.data(D_PREV_OWNER(n), s.prev_owner))
    table.insert(actions, PVP.data(D_PREV_STATE(n), s.prev_state))
end

local function node_world_state(actions, n, s)
    table.insert(actions, PVP.world_state(NODE_STATES[n][s.owner * 2 + s.state + 1], 1))
    if s.prev_owner == N then
        table.insert(actions, PVP.world_state(WS_SNOWFALL_NEUTRAL, 0))
    else
        table.insert(actions, PVP.world_state(NODE_STATES[n][s.prev_owner * 2 + s.prev_state + 1], 0))
    end
end

-- Banner and defenders of a node's new state.
local function populate(actions, n, s)
    if s.state == CONTROLLED and s.owner ~= N then
        if is_tower(n) then
            table.insert(actions, PVP.event(NODES + n, s.owner * 2 + 1, true))
            table.insert(actions, PVP.event(23 + n, s.owner * 4, true))
        else
            table.insert(actions, PVP.event(NODES + n, s.owner * 4, true))
        end
    end
    table.insert(actions, PVP.event(n, s.owner * 2 + s.state, true))
end

local function announce(actions, text, idx)
    table.insert(actions, PVP.announce_text(text, idx == A and "ALLIANCE" or (idx == H and "HORDE" or "NEUTRAL")))
end

local function assault(actions, input, changes, n, player, idx)
    local s = node(input, changes, n)
    if s.owner == idx or s.total == idx then
        return
    end
    local new = {
        owner = idx, state = ASSAULTED, total = s.total, prev_owner = s.owner, prev_state = s.state,
    }
    store(actions, changes, n, new)
    table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(n), duration = CAPTURE_MS })
    node_world_state(actions, n, new)
    populate(actions, n, new)
    if is_tower(n) then
        announce(actions, NODE_NAMES[n] .. " is under attack! If left unchecked, the " .. team_name(idx)
            .. " will destroy it!", idx)
        table.insert(actions, PVP.score(player, COL_TOWERS_ASSAULTED))
    else
        announce(actions, "The " .. NODE_NAMES[n] .. " is under attack! If left unchecked, the "
            .. team_name(idx) .. " will capture it!", idx)
        table.insert(actions, PVP.score(player, COL_GRAVES_ASSAULTED))
    end
    table.insert(actions, PVP.sound(SOUND_ASSAULTS[idx]))
end

local function defend(actions, input, changes, n, player, idx)
    local s = node(input, changes, n)
    if s.owner == idx or s.state ~= ASSAULTED then
        return
    end
    -- Snowfall has no owner to defend it: the first click is an assault.
    if s.total == N then
        assault(actions, input, changes, n, player, idx)
        return
    end
    if s.prev_owner ~= idx then
        return
    end
    local new = {
        owner = idx, state = CONTROLLED, total = s.total, prev_owner = s.owner, prev_state = s.state,
    }
    store(actions, changes, n, new)
    table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(n), duration = 0 })
    populate(actions, n, new)
    node_world_state(actions, n, new)
    if is_tower(n) then
        announce(actions, NODE_NAMES[n] .. " was defended by the " .. team_name(idx) .. "!", idx)
        table.insert(actions, PVP.score(player, COL_TOWERS_DEFENDED))
        table.insert(actions, PVP.sound(SOUND_TOWER_DEFENDED))
    else
        announce(actions, "The " .. NODE_NAMES[n] .. " was defended by the " .. team_name(idx) .. "!", idx)
        table.insert(actions, PVP.score(player, COL_GRAVES_DEFENDED))
        table.insert(actions, PVP.sound(SOUND_GOOD[idx]))
    end
end

-- An assault ran its course: a graveyard changes hands, a tower burns.
local function destroyed(actions, input, changes, n)
    local s = node(input, changes, n)
    local idx = s.owner
    local new = {
        owner = idx, state = CONTROLLED, total = idx, prev_owner = idx, prev_state = s.state,
    }
    store(actions, changes, n, new)
    populate(actions, n, new)
    node_world_state(actions, n, new)
    if n == STONEHEARTH_BUNKER then
        table.insert(actions, PVP.event(25, 1, true))
    end
    if is_tower(n) then
        add_score(actions, input, changes, 1 - idx, -RES_TOWER)
        table.insert(actions, PVP.reputation_team(index_team(idx), FACTION[idx], rep(input, "tower")))
        table.insert(actions, PVP.honor_team(index_team(idx), bonus_honor(input, KILL_TOWER)))
        announce(actions, NODE_NAMES[n] .. " was destroyed by the " .. team_name(idx) .. "!", idx)
    else
        announce(actions, "The " .. NODE_NAMES[n] .. " was taken by the " .. team_name(idx) .. "!", idx)
        -- Ghosts at the lost graveyard look for another.
        table.insert(actions, { action = "REPOP_TEAM", team = index_team(1 - idx) })
    end
    table.insert(actions, PVP.sound(SOUND_GOOD[idx]))
end

local function mine_owner(input, changes, m)
    return changes["mine" .. m] or input:GetData(D_MINE_OWNER(m))
end

local function change_mine_owner(actions, input, changes, m, idx)
    table.insert(actions, { action = "SET_TIMER", timer_id = T_MINE_TICK(m), duration = MINE_TICK_MS })
    local current = mine_owner(input, changes, m)
    if current == idx then
        return
    end
    changes["mine" .. m] = idx
    table.insert(actions, PVP.data(D_MINE_PREV(m), current))
    table.insert(actions, PVP.data(D_MINE_OWNER(m), idx))
    table.insert(actions, PVP.world_state(MINE_STATES[m][idx], 1))
    table.insert(actions, PVP.world_state(MINE_STATES[m][current], 0))
    table.insert(actions, PVP.event(EV_MINE + m, idx, true))
    table.insert(actions, PVP.event(EV_MINE_BOSS + m, idx, true))
    if idx ~= N then
        table.insert(actions, PVP.sound(SOUND_GOOD[idx]))
        table.insert(actions, { action = "SET_TIMER", timer_id = T_MINE_RECLAIM(m), duration = MINE_RECLAIM_MS })
        announce(actions, "The " .. team_name(idx) .. " has taken the " .. MINE_NAMES[m] .. " Mine!", idx)
    end
end

function av:OnLoad(input)
    local actions = {}
    local changes = new_changes()
    for _, ev in ipairs({ BG_EVENT_DOOR, BG_EVENT_GHOST_GATE, EV_CAPTAIN_A, EV_CAPTAIN_H, EV_HERALD,
        EV_SNIVVLE, EV_BOSS_A, EV_BOSS_H, EV_LANDMINES_A, EV_LANDMINES_H, EV_EXPLOSIVES_A,
        EV_EXPLOSIVES_H, EV_LIEUTENANT_A, EV_LIEUTENANT_H }) do
        table.insert(actions, PVP.event(ev, 0, true))
    end
    for ev = 52, 59 do
        table.insert(actions, PVP.event(ev, 0, true))
    end
    for idx = A, H do
        table.insert(actions, PVP.data(D_SCORE[idx], INITIAL_POINTS))
        table.insert(actions, PVP.world_state(WS_SCORE[idx], INITIAL_POINTS))
        table.insert(actions, PVP.world_state(WS_SHOW_SCORE[idx], 0))
    end
    for m = 0, 1 do
        table.insert(actions, PVP.data(D_MINE_OWNER(m), N))
        table.insert(actions, PVP.data(D_MINE_PREV(m), N))
        table.insert(actions, PVP.event(EV_MINE + m, N, true))
        table.insert(actions, PVP.event(EV_MINE_BOSS + m, N, true))
        for idx = A, N do
            table.insert(actions, PVP.world_state(MINE_STATES[m][idx], idx == N and 1 or 0))
        end
    end
    for n = 0, NODES - 1 do
        local owner = N
        if n <= 2 or (n >= 7 and n <= 10) then
            owner = A
        elseif n ~= SNOWFALL then
            owner = H
        end
        local s = { owner = owner, state = CONTROLLED, total = owner, prev_owner = owner, prev_state = CONTROLLED }
        store(actions, changes, n, s)
        populate(actions, n, s)
        if owner == N then
            table.insert(actions, PVP.event(NODES + n, N * 4, true))
        end
        for i = 1, 4 do
            local on = owner ~= N and i == owner * 2 + CONTROLLED + 1
            table.insert(actions, PVP.world_state(NODE_STATES[n][i], on and 1 or 0))
        end
    end
    table.insert(actions, PVP.world_state(WS_SNOWFALL_NEUTRAL, 1))
    return actions
end

function av:OnMatchEvent(input, event)
    if event == "ONE_MINUTE" then
        return { PVP.announce(TEXT_ONE_MINUTE) }
    elseif event == "HALF_MINUTE" then
        return { PVP.announce(TEXT_HALF_MINUTE) }
    elseif event == "BEGIN" then
        return {
            { action = "DOOR_EVENT", event1 = BG_EVENT_DOOR, event2 = 0, open = true },
            PVP.event(BG_EVENT_GHOST_GATE, 0, false),
            PVP.world_state(WS_SHOW_SCORE[A], 1),
            PVP.world_state(WS_SHOW_SCORE[H], 1),
            { action = "SET_TIMER", timer_id = T_BUFF[A], duration = 120000 + math.random(0, 4) * 60000 },
            { action = "SET_TIMER", timer_id = T_BUFF[H], duration = 120000 + math.random(0, 4) * 60000 },
            PVP.announce(TEXT_BEGUN),
        }
    elseif event == "DOORS_DESPAWN" then
        return { PVP.event(BG_EVENT_DOOR, 0, false) }
    end
    return {}
end

-- Banners: a controlled banner is assaulted, an assaulted one defended.
function av:OnGameObjectUse(input, player, go, entry, event1, event2)
    if not PVP.in_progress(input) or event1 >= NODES then
        return {}
    end
    local idx = team_index(PVP.team_of(input, player))
    local actions = {}
    local changes = new_changes()
    if event2 % 2 == CONTROLLED then
        assault(actions, input, changes, event1, player, idx)
    else
        defend(actions, input, changes, event1, player, idx)
    end
    return actions
end

-- The end: objective bonuses for both sides, then the win.
local function finish(actions, input, changes, winner_idx)
    local towers, graves, mines = { [A] = 0, [H] = 0 }, { [A] = 0, [H] = 0 }, { [A] = 0, [H] = 0 }
    for n = 0, NODES - 1 do
        local s = node(input, changes, n)
        if s.state == CONTROLLED then
            if is_tower(n) and s.total ~= N and ((n <= 10 and s.total == A) or (n >= 11 and s.total == H)) then
                towers[s.total] = towers[s.total] + 1
            elseif not is_tower(n) and s.owner ~= N and n ~= 0 and n ~= 6 then
                graves[s.owner] = graves[s.owner] + 1
            end
        end
    end
    for m = 0, 1 do
        local owner = mine_owner(input, changes, m)
        if owner ~= N then
            mines[owner] = mines[owner] + 1
        end
    end
    local modifier = honor_modifier(input)
    for idx = A, H do
        local team = index_team(idx)
        if towers[idx] > 0 then
            table.insert(actions, PVP.reputation_team(team, FACTION[idx], towers[idx] * rep(input, "surviving_tower")))
            table.insert(actions, PVP.honor_team(team, math.floor(bonus_honor(input, towers[idx] * KILL_SURVIVING_TOWER) * modifier)))
        end
        if graves[idx] > 0 then
            table.insert(actions, PVP.reputation_team(team, FACTION[idx], graves[idx] * rep(input, "grave")))
            table.insert(actions, PVP.honor_team(team, math.floor(bonus_honor(input, graves[idx] * KILL_SURVIVING_GRAVE) * modifier)))
        end
        if mines[idx] > 0 then
            table.insert(actions, PVP.reputation_team(team, FACTION[idx], mines[idx] * rep(input, "mine")))
            table.insert(actions, PVP.honor_team(team, math.floor(bonus_honor(input, mines[idx] * KILL_SURVIVING_MINE) * modifier)))
        end
        if input:GetData(D_CAPTAIN_DEAD[idx]) == 0 then
            table.insert(actions, PVP.reputation_team(team, FACTION[idx], rep(input, "surviving_captain")))
            table.insert(actions, PVP.honor_team(team, bonus_honor(input, KILL_SURVIVING_CAPTAIN)))
        end
    end
    if PVP.weekend(input) then
        table.insert(actions, PVP.honor_team(TEAM_ALLIANCE, 1584))
        table.insert(actions, PVP.honor_team(TEAM_HORDE, 1584))
        table.insert(actions, PVP.honor_team(index_team(winner_idx), 396))
    end
    table.insert(actions, PVP.announce(winner_idx == A and TEXT_ALLIANCE_WINS or TEXT_HORDE_WINS))
    table.insert(actions, { action = "END_MATCH", winner = index_team(winner_idx) })
end

function av:OnCreatureDeath(input, entry, guid, killer, event1, event2)
    if not PVP.in_progress(input) or event1 == nil or event1 == 255 then
        return {}
    end
    local actions = {}
    local changes = new_changes()
    if event1 == EV_BOSS_A or event1 == EV_BOSS_H then
        -- A general falls: the other side wins.
        local winner = event1 == EV_BOSS_A and H or A
        local team = index_team(winner)
        table.insert(actions, { action = "CAST_SPELL_ON_TEAM", spell_id = SPELL_BOSS_KILLED, team = team })
        table.insert(actions, PVP.reputation_team(team, FACTION[winner], rep(input, "boss")))
        table.insert(actions, PVP.honor_team(team, math.floor(bonus_honor(input, KILL_BOSS) * honor_modifier(input))))
        announce(actions, (winner == H and "Vanndar Stormpike" or "Drek'Thar") .. " is dead!", winner)
        finish(actions, input, changes, winner)
    elseif event1 == EV_CAPTAIN_A or event1 == EV_CAPTAIN_H then
        local loser = event1 == EV_CAPTAIN_A and A or H
        if input:GetData(D_CAPTAIN_DEAD[loser]) ~= 0 then
            return {}
        end
        local winner = 1 - loser
        local team = index_team(winner)
        table.insert(actions, PVP.data(D_CAPTAIN_DEAD[loser], 1))
        table.insert(actions, PVP.reputation_team(team, FACTION[winner], rep(input, "captain")))
        table.insert(actions, PVP.honor_team(team, bonus_honor(input, KILL_CAPTAIN)))
        add_score(actions, input, changes, loser, -RES_CAPTAIN)
        table.insert(actions, PVP.event(loser == A and EV_CAPTAIN_DEAD_A or EV_CAPTAIN_DEAD_H, 0, true))
    elseif COMMANDERS_A[event1] or COMMANDERS_H[event1] or event1 == EV_LIEUTENANT_A or event1 == EV_LIEUTENANT_H then
        local winner = (COMMANDERS_A[event1] or event1 == EV_LIEUTENANT_A) and H or A
        table.insert(actions, PVP.reputation_team(index_team(winner), FACTION[winner], rep(input, "commander")))
        table.insert(actions, PVP.honor_team(index_team(winner), bonus_honor(input, KILL_COMMANDER)))
    elseif event1 == EV_MINE_BOSS or event1 == EV_MINE_BOSS + 1 then
        if killer then
            local idx = team_index(PVP.team_of(input, killer))
            if PVP.team_of(input, killer) ~= 0 then
                change_mine_owner(actions, input, changes, event1 - EV_MINE_BOSS, idx)
            end
        end
    end
    return actions
end

function av:OnPlayerDeath(input, victim, killer)
    if not PVP.in_progress(input) then
        return {}
    end
    local actions = {}
    local idx = team_index(PVP.team_of(input, victim))
    add_score(actions, input, new_changes(), idx, -1)
    return actions
end

function av:Update(input)
    if not PVP.in_progress(input) then
        return {}
    end
    local actions = {}
    local changes = new_changes()
    -- Captains rally their side every few minutes while they live.
    for idx = A, H do
        if input.timers[T_BUFF[idx]] == 0 then
            if input:GetData(D_CAPTAIN_DEAD[idx]) == 0 then
                table.insert(actions, { action = "CAST_SPELL_ON_TEAM", spell_id = SPELL_CAPTAIN_BUFF[idx], team = index_team(idx) })
            end
            table.insert(actions, { action = "SET_TIMER", timer_id = T_BUFF[idx], duration = 120000 + math.random(0, 4) * 60000 })
        end
    end
    -- Mines feed their owner, and fall back to the Troggs or Kobolds after a while.
    for m = 0, 1 do
        local owner = input:GetData(D_MINE_OWNER(m))
        if owner ~= N then
            if input.timers[T_MINE_TICK(m)] == 0 then
                add_score(actions, input, changes, owner, 1)
                table.insert(actions, { action = "SET_TIMER", timer_id = T_MINE_TICK(m), duration = MINE_TICK_MS })
            end
            if input.timers[T_MINE_RECLAIM(m)] == 0 then
                change_mine_owner(actions, input, changes, m, N)
            end
        end
    end
    -- Assaults that ran their five minutes.
    for n = 0, NODES - 1 do
        if input:GetData(D_STATE(n)) == ASSAULTED and input.timers[T_NODE(n)] == 0 then
            destroyed(actions, input, changes, n)
            table.insert(actions, { action = "SET_TIMER", timer_id = T_NODE(n), duration = 0 })
        end
    end
    return actions
end

function av:OnAreaTrigger(input, player, trigger)
    local team = PVP.team_of(input, player)
    if (trigger == AT_ALLIANCE_EXIT and team == TEAM_ALLIANCE) or (trigger == AT_HORDE_EXIT and team == TEAM_HORDE) then
        return { { action = "LEAVE_BATTLEGROUND", player = player } }
    end
    if trigger >= 3326 and trigger <= 3331 then
        return true
    end
    return false
end

function av:SelectGraveyard(input, player)
    local p = PVP.player(input, player)
    local idx = team_index(p and p.team or TEAM_ALLIANCE)
    if not PVP.in_progress(input) and not (p and p.is_gm) then
        return CAVE[idx].id
    end
    local best = CAVE[idx]
    local best_distance = p and PVP.distance2d(p, best.x, best.y) or 0
    if p and PVP.in_progress(input) then
        for n = 0, 6 do
            if input:GetData(D_OWNER(n)) == idx and input:GetData(D_STATE(n)) == CONTROLLED then
                local gy = GRAVEYARDS[n]
                local d = PVP.distance2d(p, gy.x, gy.y)
                if d < best_distance then
                    best, best_distance = gy, d
                end
            end
        end
    end
    return best.id
end

RegisterInstanceScript(30, av)
