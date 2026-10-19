--[[
    @script_type: zone
    @zone_id: 139
    @name: world_pvp_eastern_plaguelands

    Eastern Plaguelands world PvP: four towers, each taken by standing at it.

    Every living, non-GM player within a tower's capture radius pushes its
    slider towards their side, one point a second per player more than the
    other side has there. The slider runs from 1200 (Alliance) to -1200
    (Horde); past the neutral band (20% either side of the middle) the tower
    belongs to whoever is pushing it, and at either end it is fully held.

    Holding towers gives every player of that side in the zone Echoes of
    Lordaeron, one rank per tower. Each tower also gives its holder something:
    Crown Guard a graveyard, Eastwall an escort squad, Northpass a curing
    shrine and Plaguewood a flight master.
]]

local RADIUS = 80
local MAX_VALUE = 1200
local NEUTRAL_PCT = 20
local MIN_VALUE = MAX_VALUE * NEUTRAL_PCT / 100

-- Objective states, as the slider places a tower.
local NEUTRAL = 0
local ALLY_CONTESTED = 1
local HORDE_CONTESTED = 2
local ALLY_PROGRESSING = 3
local HORDE_PROGRESSING = 4
local ALLY = 5
local HORDE = 6

local WS_SLIDER_DISPLAY = 2426
local WS_SLIDER_POSITION = 2427
local WS_SLIDER_NEUTRAL = 2428
local WS_COUNT = { [TEAM_ALLIANCE] = 2327, [TEAM_HORDE] = 2328 }

local SOUND_CAPTURED = { [TEAM_ALLIANCE] = 8173, [TEAM_HORDE] = 8213 }
local SOUND_VICTORY = { [TEAM_ALLIANCE] = 8455, [TEAM_HORDE] = 8454 }
local SOUND_WARNING = { [TEAM_ALLIANCE] = 8332, [TEAM_HORDE] = 8333 }

local TEXT_ALL_TOWERS = { [TEAM_ALLIANCE] = 13638, [TEAM_HORDE] = 13637 }

-- Echoes of Lordaeron, ranks 1-4 by towers held.
local BUFFS = {
    [TEAM_ALLIANCE] = { 11413, 11414, 11415, 1386 },
    [TEAM_HORDE] = { 30880, 30683, 30682, 29520 },
}

local EP_GRAVEYARD = 927

-- World state of each tower state, in state order.
local function tower_states(neutral, a_cont, h_cont, a_prog, h_prog, a, h)
    return { [NEUTRAL] = neutral, [ALLY_CONTESTED] = a_cont, [HORDE_CONTESTED] = h_cont,
             [ALLY_PROGRESSING] = a_prog, [HORDE_PROGRESSING] = h_prog, [ALLY] = a, [HORDE] = h }
end

local EASTWALL, NORTHPASS, PLAGUEWOOD, CROWN_GUARD = 1, 2, 3, 4

local TOWERS = {
    [EASTWALL] = {
        point = { entry = 182097, x = 2574.51, y = -4794.89, z = 144.704, o = -1.45003 },
        states = tower_states(2361, 2359, 2360, 2357, 2358, 2354, 2356),
        taken_text = { [TEAM_ALLIANCE] = 13631, [TEAM_HORDE] = 13636 },
        flare = {
            [TEAM_ALLIANCE] = { entry = 181852, x = 2563.26, y = -4795.15, z = 145.852, o = 1.81514 },
            [TEAM_HORDE] = { entry = 181853, x = 2565.27, y = -4797.59, z = 147.846, o = 3.05433 },
        },
        credit = 17690,
    },
    [NORTHPASS] = {
        point = { entry = 181899, x = 3181.08, y = -4379.36, z = 174.123, o = -2.03472 },
        states = tower_states(2352, 2362, 2363, 2364, 2365, 2372, 2373),
        taken_text = { [TEAM_ALLIANCE] = 13630, [TEAM_HORDE] = 13635 },
        flare = {
            [TEAM_ALLIANCE] = { entry = 181852, x = 3171.86, y = -4377.2, z = 174.898, o = 0.174532 },
            [TEAM_HORDE] = { entry = 181853, x = 3169.76, y = -4375.13, z = 175.458, o = 2.70526 },
        },
        credit = 17696,
    },
    [PLAGUEWOOD] = {
        point = { entry = 182098, x = 2962.71, y = -3042.31, z = 154.789, o = 2.08426 },
        states = tower_states(2353, 2366, 2367, 2368, 2369, 2370, 2371),
        taken_text = { [TEAM_ALLIANCE] = 13629, [TEAM_HORDE] = 13634 },
        flare = {
            [TEAM_ALLIANCE] = { entry = 181852, x = 2971.41, y = -3038.36, z = 157.492, o = 5.35816 },
            [TEAM_HORDE] = { entry = 181853, x = 2973.17, y = -3037.19, z = 156.443, o = 1.97222 },
        },
        credit = 17698,
    },
    [CROWN_GUARD] = {
        point = { entry = 182096, x = 1860.85, y = -3731.23, z = 196.716, o = -2.53214 },
        states = tower_states(2355, 2374, 2375, 2376, 2377, 2378, 2379),
        taken_text = { [TEAM_ALLIANCE] = 13632, [TEAM_HORDE] = 13633 },
        flare = {
            [TEAM_ALLIANCE] = { entry = 181852, x = 1855.66, y = -3725.0, z = 197.044, o = 1.53589 },
            [TEAM_HORDE] = { entry = 181853, x = 1853.12, y = -3722.62, z = 197.406, o = 0.628317 },
        },
        credit = 17689,
    },
}

-- Eastwall's squad.
local SQUAD = {
    [TEAM_ALLIANCE] = { 17635, 17647, 17647, 17647, 17647 },
    [TEAM_HORDE] = { 17995, 17996, 17996, 17996, 17996 },
}
local SQUAD_POSITIONS = {
    { 2532.85, -4764.92, 103.617, 2.35619 },
    { 2533.33, -4769.31, 104.396, 2.37365 },
    { 2537.34, -4773.92, 105.941, 2.21657 },
    { 2537.77, -4765.94, 104.432, 2.3911 },
    { 2542.57, -4770.22, 106.145, 2.42601 },
}

-- Northpass's shrine.
local SHRINE = {
    [TEAM_ALLIANCE] = { { 181682, 3167.72, -4355.91, 138.785 }, { 180100, 3167.72, -4355.91, 138.785 } },
    [TEAM_HORDE] = { { 181955, 3167.5, -4356.25, 138.821 }, { 180101, 3167.5, -4356.25, 138.821 } },
}
local SHRINE_O = 1.69297

-- Crown Guard's graveyard banner and spirit.
local GRAVEYARD_AURA = { [TEAM_ALLIANCE] = 180421, [TEAM_HORDE] = 180422 }
local GRAVEYARD_AURA_POS = { 1985.47, -3653.88, 120.172, 1.46608 }
local SPIRIT_OF_VICTORY = 18039
local SPIRIT_POS = { 1856.58, -3714.72, 194.637, 0.762214 }

-- Plaguewood's flight master.
local FLIGHT_MASTER = 17209
local FLIGHT_MASTER_POS = { 2987.5, -3049.11, 120.126, 5.75959 }
local FLIGHT_MASTER_FACTION = { [TEAM_ALLIANCE] = 774, [TEAM_HORDE] = 775 }

-- Data ids.
local D_SPAWNED = 1
local function D_VALUE(t) return 10 + t end   -- slider + MAX_VALUE, so never negative
local function D_STATE(t) return 20 + t end
local function D_OWNER(t) return 30 + t end    -- team the tower counts for, or 0

local ep = {
    -- Players inside each tower's radius last tick, by guid string.
    inside = { {}, {}, {}, {} },
    -- Each tower's state as of the change being handled.
    states = { NEUTRAL, NEUTRAL, NEUTRAL, NEUTRAL },
}

local function owner_of(state)
    if state == ALLY or state == ALLY_PROGRESSING then
        return TEAM_ALLIANCE
    elseif state == HORDE or state == HORDE_PROGRESSING then
        return TEAM_HORDE
    end
    return 0
end

local function is_grey(state)
    return state == NEUTRAL or state == ALLY_CONTESTED or state == HORDE_CONTESTED
end

local function towers_held(input, team, owners)
    local n = 0
    for t = 1, 4 do
        local owner = owners[t]
        if owner == nil then
            owner = input:GetData(D_OWNER(t))
        end
        if owner == team then
            n = n + 1
        end
    end
    return n
end

local function gameobject(entry, x, y, z, o)
    return { action = "SPAWN_GAMEOBJECT", entry = entry, x = x, y = y, z = z, o = o, duration = 0 }
end

local function creature(entry, pos, faction)
    return { action = "SPAWN_CREATURE", entry = entry, x = pos[1], y = pos[2], z = pos[3], o = pos[4],
             summon_type = "MANUAL_DESPAWN", faction = faction }
end

local function despawn_go(entry)
    return { action = "DESPAWN_GAMEOBJECT", entry = entry }
end

local function despawn_creature(entry)
    return { action = "DESPAWN_CREATURE", entry = entry }
end

-- Where the slider puts a tower, given who pushed it this tick.
local function state_for(value, challenger)
    if value <= -MIN_VALUE then
        return value <= -MAX_VALUE and HORDE or HORDE_PROGRESSING
    elseif value >= MIN_VALUE then
        return value >= MAX_VALUE and ALLY or ALLY_PROGRESSING
    elseif challenger == TEAM_ALLIANCE then
        return ALLY_CONTESTED
    elseif challenger == TEAM_HORDE then
        return HORDE_CONTESTED
    end
    return NEUTRAL
end

-- What a tower gives its holder, put in place or taken away for a state.
local function tower_rewards(actions, t, old, new, team, inside)
    local tower = TOWERS[t]
    local held = new == ALLY or new == HORDE
    local holding = owner_of(new) ~= 0
    local from_grey = is_grey(old) and holding

    -- The flare burns only over a fully held tower.
    table.insert(actions, despawn_go(181852))
    table.insert(actions, despawn_go(181853))
    for other = 1, 4 do
        local s = ep.states[other]
        if s == ALLY or s == HORDE then
            local f = TOWERS[other].flare[owner_of(s)]
            table.insert(actions, gameobject(f.entry, f.x, f.y, f.z, f.o))
        end
    end

    if t == EASTWALL then
        if is_grey(new) then
            for _, side in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
                for _, entry in ipairs(SQUAD[side]) do
                    table.insert(actions, despawn_creature(entry))
                end
            end
        elseif from_grey then
            for i, entry in ipairs(SQUAD[team]) do
                table.insert(actions, creature(entry, SQUAD_POSITIONS[i]))
            end
        end
    elseif t == NORTHPASS then
        for _, side in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
            for _, obj in ipairs(SHRINE[side]) do
                table.insert(actions, despawn_go(obj[1]))
            end
        end
        if holding then
            for _, obj in ipairs(SHRINE[owner_of(new)]) do
                table.insert(actions, gameobject(obj[1], obj[2], obj[3], obj[4], SHRINE_O))
            end
        end
    elseif t == CROWN_GUARD then
        table.insert(actions, despawn_go(GRAVEYARD_AURA[TEAM_ALLIANCE]))
        table.insert(actions, despawn_go(GRAVEYARD_AURA[TEAM_HORDE]))
        if holding then
            local p = GRAVEYARD_AURA_POS
            table.insert(actions, gameobject(GRAVEYARD_AURA[owner_of(new)], p[1], p[2], p[3], p[4]))
        end
        if from_grey then
            table.insert(actions, despawn_creature(SPIRIT_OF_VICTORY))
            table.insert(actions, creature(SPIRIT_OF_VICTORY, SPIRIT_POS))
        end
    elseif t == PLAGUEWOOD then
        table.insert(actions, despawn_creature(FLIGHT_MASTER))
        if holding then
            table.insert(actions, creature(FLIGHT_MASTER, FLIGHT_MASTER_POS, FLIGHT_MASTER_FACTION[owner_of(new)]))
        end
    end

    -- Taking a tower from grey: the takers there get their quest credit.
    if from_grey then
        for _, p in ipairs(inside) do
            if p.team == team then
                table.insert(actions, { action = "KILL_CREDIT", player = p.guid, entry = tower.credit })
            end
        end
    end
end

-- Echoes of Lordaeron for everyone of a side in the zone, by towers held.
local function buff_side(actions, input, team, count)
    for _, p in ipairs(input.players or {}) do
        if p.team == team then
            for _, spell in ipairs(BUFFS[team]) do
                table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = p.guid, spell_id = spell })
            end
            if count > 0 then
                table.insert(actions, { action = "PLAYER_CAST_SPELL", player = p.guid, spell_id = BUFFS[team][count] })
            end
        end
    end
end

local function spawn_points(input)
    local actions = { PVP.data(D_SPAWNED, 1) }
    for t = 1, 4 do
        local p = TOWERS[t].point
        table.insert(actions, gameobject(p.entry, p.x, p.y, p.z, p.o))
        if input:GetData(D_VALUE(t)) == 0 then
            table.insert(actions, PVP.data(D_VALUE(t), MAX_VALUE))
        end
        table.insert(actions, PVP.world_state(TOWERS[t].states[input:GetData(D_STATE(t))], 1))
    end
    table.insert(actions, PVP.world_state(WS_COUNT[TEAM_ALLIANCE], 0))
    table.insert(actions, PVP.world_state(WS_COUNT[TEAM_HORDE], 0))
    return actions
end

function ep:OnPlayerEnter(input, player)
    local p = PVP.player(input, player)
    if p == nil or BUFFS[p.team] == nil then
        return {}
    end
    local count = towers_held(input, p.team, {})
    if count > 0 then
        return { { action = "PLAYER_CAST_SPELL", player = player, spell_id = BUFFS[p.team][count] } }
    end
    return {}
end

function ep:OnPlayerLeave(input, player)
    local actions = {
        { action = "SET_PLAYER_WORLD_STATE", player = player, state_id = WS_SLIDER_DISPLAY, value = 0 },
    }
    for _, side in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
        for _, spell in ipairs(BUFFS[side]) do
            table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = player, spell_id = spell })
        end
    end
    for t = 1, 4 do
        self.inside[t][tostring(player)] = nil
    end
    return actions
end

function ep:Update(input)
    if input:GetData(D_SPAWNED) == 0 then
        return spawn_points(input)
    end
    local actions = {}
    local seconds = (input.diff_ms or 0) / 1000
    local owners = {}
    local any_change = false
    for t = 1, 4 do
        ep.states[t] = input:GetData(D_STATE(t))
    end

    for t = 1, 4 do
        local tower = TOWERS[t]
        local point = tower.point
        local alliance, horde, inside = PVP.count_near(input, point.x, point.y, point.z, RADIUS)

        local value = input:GetData(D_VALUE(t)) - MAX_VALUE
        local old_state = input:GetData(D_STATE(t))
        local push = (alliance - horde) * seconds
        local challenger = 0
        if push > 0 then
            challenger = TEAM_ALLIANCE
        elseif push < 0 then
            challenger = TEAM_HORDE
        end
        local new_value = math.max(-MAX_VALUE, math.min(MAX_VALUE, value + push))
        local stored = math.floor(new_value + MAX_VALUE + 0.5)
        if stored ~= input:GetData(D_VALUE(t)) then
            table.insert(actions, PVP.data(D_VALUE(t), stored))
        end
        local old_pct = math.ceil((value + MAX_VALUE) / (2 * MAX_VALUE) * 100)
        local pct = math.ceil((new_value + MAX_VALUE) / (2 * MAX_VALUE) * 100)

        -- The slider shows to whoever stands at the tower, and goes away when they leave.
        local now_inside = {}
        for _, p in ipairs(inside) do
            local key = tostring(p.guid)
            now_inside[key] = p.guid
            if not self.inside[t][key] then
                PVP.extend(actions, {
                    { action = "SET_PLAYER_WORLD_STATE", player = p.guid, state_id = WS_SLIDER_DISPLAY, value = 1 },
                    { action = "SET_PLAYER_WORLD_STATE", player = p.guid, state_id = WS_SLIDER_NEUTRAL, value = NEUTRAL_PCT },
                    { action = "SET_PLAYER_WORLD_STATE", player = p.guid, state_id = WS_SLIDER_POSITION, value = pct },
                })
            elseif pct ~= old_pct then
                table.insert(actions, { action = "SET_PLAYER_WORLD_STATE", player = p.guid, state_id = WS_SLIDER_POSITION, value = pct })
            end
        end
        for key, guid in pairs(self.inside[t]) do
            if not now_inside[key] then
                table.insert(actions, { action = "SET_PLAYER_WORLD_STATE", player = guid, state_id = WS_SLIDER_DISPLAY, value = 0 })
            end
        end
        self.inside[t] = now_inside

        -- A slider already at its end does not move or change state.
        local stuck = (challenger == TEAM_ALLIANCE and value >= MAX_VALUE)
            or (challenger == TEAM_HORDE and value <= -MAX_VALUE)
        local new_state = stuck and old_state or state_for(new_value, challenger)
        if new_state ~= old_state then
            any_change = true
            ep.states[t] = new_state
            table.insert(actions, PVP.data(D_STATE(t), new_state))
            table.insert(actions, PVP.world_state(tower.states[old_state], 0))
            table.insert(actions, PVP.world_state(tower.states[new_state], 1))

            -- Losing a fully held tower stops it counting; taking one from grey starts it.
            local team = owner_of(new_state)
            if old_state == ALLY or old_state == HORDE then
                owners[t] = 0
            end
            if new_state == ALLY or new_state == HORDE then
                table.insert(actions, PVP.sound(SOUND_VICTORY[team]))
            elseif team ~= 0 and (old_state == ALLY or old_state == HORDE) then
                table.insert(actions, PVP.sound(SOUND_WARNING[team]))
            elseif team ~= 0 and is_grey(old_state) then
                owners[t] = team
                table.insert(actions, PVP.sound(SOUND_CAPTURED[team]))
                table.insert(actions, PVP.announce(tower.taken_text[team], "SYSTEM"))
            end
            if owners[t] ~= nil then
                table.insert(actions, PVP.data(D_OWNER(t), owners[t]))
            end
            tower_rewards(actions, t, old_state, new_state, team, inside)
        end
    end

    -- Tower counts and the zone's buffs follow any change.
    if any_change then
        for _, team in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
            local count = towers_held(input, team, owners)
            table.insert(actions, PVP.world_state(WS_COUNT[team], count))
            buff_side(actions, input, team, count)
            if count == 4 then
                table.insert(actions, PVP.announce(TEXT_ALL_TOWERS[team], "SYSTEM"))
            end
        end
    end
    return actions
end

-- Crown Guard's holder rises at the tower's graveyard.
function ep:SelectGraveyard(input, player)
    local team = PVP.team_of(input, player)
    if team ~= 0 and owner_of(input:GetData(D_STATE(CROWN_GUARD))) == team then
        return EP_GRAVEYARD
    end
    return nil
end

RegisterZoneScript(139, ep)
