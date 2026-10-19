--[[
    @script_type: instance
    @map_id: 489
    @name: battleground_warsong_gulch

    Warsong Gulch: capture the enemy flag three times.

    Each side's flag waits at its base. An enemy picks it up (carrying it as
    an aura), and scores by bringing it to their own base while their own
    flag is home. A dropped flag lies on the ground for ten seconds: the
    enemy can pick it up again, its own side returns it with a click, and
    otherwise it goes home by itself. After a capture both flags reappear
    23 seconds later.
]]

-- Spawn event slots.
local EVENT_FLAG_A = 0          -- the Alliance (Silverwing) flag at its base
local EVENT_FLAG_H = 1          -- the Horde (Warsong) flag at its base
local EVENT_SPIRIT_GUIDES = 2

-- Gameobjects of the dropped flags.
local GO_ALLIANCE_FLAG_GROUND = 179785
local GO_HORDE_FLAG_GROUND = 179786

-- Carried-flag auras: Alliance players carry the Warsong flag, Horde the Silverwing flag.
local SPELL_WARSONG_FLAG = 23333
local SPELL_SILVERWING_FLAG = 23335

-- World states.
local WS_FLAG_TAKEN_ALLIANCE = 1545   -- the Alliance flag is held (1) or dropped (-1)
local WS_FLAG_TAKEN_HORDE = 1546
local WS_CAPTURES_ALLIANCE = 1581
local WS_CAPTURES_HORDE = 1582
local WS_CAPTURES_MAX = 1601
local WS_FLAG_STATE_HORDE = 2338      -- 2 while the Horde carries a flag
local WS_FLAG_STATE_ALLIANCE = 2339

-- Flag states.
local ON_BASE = 0
local WAIT_RESPAWN = 1
local ON_PLAYER = 2
local ON_GROUND = 3

-- Script data.
local D_STATE_A = 1
local D_STATE_H = 2
local D_SCORE_A = 3
local D_SCORE_H = 4
local G_KEEPER_A = 1     -- the Horde player holding the Alliance flag
local G_KEEPER_H = 2     -- the Alliance player holding the Horde flag
local T_RESPAWN = 1      -- both flags come back after a capture
local T_DROP_A = 2
local T_DROP_H = 3

local MAX_SCORE = 3
local RESPAWN_MS = 23000
local DROP_MS = 10000

-- Area triggers.
local AT_ALLIANCE_BASE = 3646    -- the Alliance flag room: where the Alliance scores
local AT_HORDE_BASE = 3647
local AT_HORDE_EXIT = 3669
local AT_ALLIANCE_EXIT = 3671
local BUFF_TRIGGERS = { [3686] = true, [3687] = true, [3706] = true, [3707] = true, [3708] = true, [3709] = true }
local ALLIANCE_BASE = { x = 1539.89, y = 1481.36, z = 352.66 }
local HORDE_BASE = { x = 918.5, y = 1434.04, z = 346.05 }

-- Graveyards (WorldSafeLocs).
local GY_FLAGROOM_ALLIANCE = 769
local GY_FLAGROOM_HORDE = 770
local GY_MAIN_ALLIANCE = 771
local GY_MAIN_HORDE = 772

-- Broadcast texts.
local TEXT_ONE_MINUTE = 10015
local TEXT_HALF_MINUTE = 10016
local TEXT_BEGUN = 10014
local TEXT_CAPTURED_HF = 9801
local TEXT_CAPTURED_AF = 9802
local TEXT_FLAGS_PLACED = 9803
local TEXT_PICKEDUP_AF = 9804
local TEXT_DROPPED_AF = 9805
local TEXT_DROPPED_HF = 9806
local TEXT_PICKEDUP_HF = 9807
local TEXT_RETURNED_AF = 9808
local TEXT_RETURNED_HF = 9809
local TEXT_ALLIANCE_FLAG_RESPAWNED = 10022
local TEXT_HORDE_FLAG_RESPAWNED = 10023
local TEXT_HORDE_WINS = 9842
local TEXT_ALLIANCE_WINS = 9843

-- Sounds.
local SOUND_CAPTURED_ALLIANCE = 8173
local SOUND_CAPTURED_HORDE = 8213
local SOUND_FLAGS_RESPAWNED = 8232
local SOUND_RETURNED = 8192
local SOUND_HORDE_FLAG_PICKED_UP = 8212
local SOUND_ALLIANCE_FLAG_PICKED_UP = 8174

-- Reputation factions.
local FACTION_SILVERWING = 890
local FACTION_WARSONG = 889

-- Honour per level bracket.
local HONOR_CAPTURE = { 48, 82, 136, 226, 378, 396 }
local HONOR_WIN = { 24, 41, 68, 113, 189, 198 }
local HONOR_WIN_WEEKEND = { 48, 82, 136, 226, 378, 396 }
local HONOR_COMPLETE_WEEKEND = { 72, 123, 204, 339, 567, 594 }

-- Scoreboard columns.
local COLUMN_CAPTURES = 0
local COLUMN_RETURNS = 1

local wsg = {
    ScoreColumns = 2,
    WatchedAuras = { SPELL_WARSONG_FLAG, SPELL_SILVERWING_FLAG },
}

-- Everything about "the flag of `team`".
local function flag_of(team)
    if team == TEAM_ALLIANCE then
        return {
            state = D_STATE_A, keeper = G_KEEPER_A, event = EVENT_FLAG_A, drop_timer = T_DROP_A,
            taken_state = WS_FLAG_TAKEN_ALLIANCE, carrier_state = WS_FLAG_STATE_HORDE,
            aura = SPELL_SILVERWING_FLAG, ground = GO_ALLIANCE_FLAG_GROUND,
            picked_text = TEXT_PICKEDUP_AF, dropped_text = TEXT_DROPPED_AF,
            returned_text = TEXT_RETURNED_AF, respawned_text = TEXT_ALLIANCE_FLAG_RESPAWNED,
            picked_sound = SOUND_ALLIANCE_FLAG_PICKED_UP,
        }
    end
    return {
        state = D_STATE_H, keeper = G_KEEPER_H, event = EVENT_FLAG_H, drop_timer = T_DROP_H,
        taken_state = WS_FLAG_TAKEN_HORDE, carrier_state = WS_FLAG_STATE_ALLIANCE,
        aura = SPELL_WARSONG_FLAG, ground = GO_HORDE_FLAG_GROUND,
        picked_text = TEXT_PICKEDUP_HF, dropped_text = TEXT_DROPPED_HF,
        returned_text = TEXT_RETURNED_HF, respawned_text = TEXT_HORDE_FLAG_RESPAWNED,
        picked_sound = SOUND_HORDE_FLAG_PICKED_UP,
    }
end

local function score_state(team)
    if team == TEAM_ALLIANCE then
        return D_SCORE_A, WS_CAPTURES_ALLIANCE
    end
    return D_SCORE_H, WS_CAPTURES_HORDE
end

-- The flag `team` owns goes home (`captured`: both flags come back together).
local function respawn_flag(actions, team, captured)
    local flag = flag_of(team)
    table.insert(actions, PVP.data(flag.state, ON_BASE))
    table.insert(actions, PVP.event(flag.event, 0, true))
    if captured then
        table.insert(actions, PVP.event(EVENT_FLAG_A, 0, true))
        table.insert(actions, PVP.event(EVENT_FLAG_H, 0, true))
        table.insert(actions, PVP.data(D_STATE_A, ON_BASE))
        table.insert(actions, PVP.data(D_STATE_H, ON_BASE))
        table.insert(actions, PVP.announce(TEXT_FLAGS_PLACED))
        table.insert(actions, PVP.sound(SOUND_FLAGS_RESPAWNED))
    end
end

-- `player` of `team` takes the enemy flag.
local function pick_up(actions, input, player, team)
    local enemy = PVP.other_team(team)
    local flag = flag_of(enemy)
    table.insert(actions, PVP.event(flag.event, 0, false))
    table.insert(actions, { action = "DESPAWN_GAMEOBJECT", entry = flag.ground })
    table.insert(actions, { action = "SET_GUID", data_id = flag.keeper, guid = player })
    table.insert(actions, PVP.data(flag.state, ON_PLAYER))
    table.insert(actions, PVP.world_state(flag.carrier_state, ON_PLAYER))
    table.insert(actions, PVP.world_state(flag.taken_state, 1))
    table.insert(actions, { action = "PLAYER_CAST_SPELL", player = player, spell_id = flag.aura })
    table.insert(actions, { action = "SET_FLAG_CARRIER", team = enemy, player = player })
    table.insert(actions, PVP.sound(flag.picked_sound))
    table.insert(actions, PVP.announce(flag.picked_text, PVP.chat_of(team), player))
end

-- The match is won by `team`.
local function finish(actions, input, team)
    local bracket = PVP.bracket(input)
    if PVP.weekend(input) then
        table.insert(actions, PVP.honor_team(TEAM_ALLIANCE, HONOR_COMPLETE_WEEKEND[bracket]))
        table.insert(actions, PVP.honor_team(TEAM_HORDE, HONOR_COMPLETE_WEEKEND[bracket]))
        table.insert(actions, PVP.honor_team(team, HONOR_WIN_WEEKEND[bracket]))
    end
    table.insert(actions, PVP.honor_team(team, HONOR_WIN[bracket]))
    table.insert(actions, PVP.world_state(WS_FLAG_STATE_ALLIANCE, 1))
    table.insert(actions, PVP.world_state(WS_FLAG_STATE_HORDE, 1))
    table.insert(actions, PVP.announce(team == TEAM_ALLIANCE and TEXT_ALLIANCE_WINS or TEXT_HORDE_WINS))
    table.insert(actions, { action = "END_MATCH", winner = team })
end

-- `player` of `team` brings the enemy flag home.
local function capture(actions, input, player, team)
    local enemy = PVP.other_team(team)
    local flag = flag_of(enemy)
    table.insert(actions, { action = "SET_GUID", data_id = flag.keeper, guid = PVP.NO_GUID })
    table.insert(actions, PVP.data(flag.state, WAIT_RESPAWN))
    table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = player, spell_id = flag.aura })
    table.insert(actions, { action = "SET_FLAG_CARRIER", team = enemy })

    local score_id, score_state_id = score_state(team)
    local score = math.min(input:GetData(score_id) + 1, MAX_SCORE)
    table.insert(actions, PVP.data(score_id, score))
    table.insert(actions, PVP.world_state(score_state_id, score))
    table.insert(actions, PVP.world_state(flag.taken_state, 0))
    table.insert(actions, PVP.world_state(flag.carrier_state, 1))

    local reputation = PVP.weekend(input) and 45 or 35
    if team == TEAM_ALLIANCE then
        table.insert(actions, PVP.sound(SOUND_CAPTURED_ALLIANCE))
        table.insert(actions, PVP.reputation_team(TEAM_ALLIANCE, FACTION_SILVERWING, reputation))
        table.insert(actions, PVP.announce(TEXT_CAPTURED_HF, "ALLIANCE", player))
    else
        table.insert(actions, PVP.sound(SOUND_CAPTURED_HORDE))
        table.insert(actions, PVP.reputation_team(TEAM_HORDE, FACTION_WARSONG, reputation))
        table.insert(actions, PVP.announce(TEXT_CAPTURED_AF, "HORDE", player))
    end
    table.insert(actions, PVP.honor_team(team, HONOR_CAPTURE[PVP.bracket(input)]))
    table.insert(actions, PVP.event(EVENT_FLAG_A, 0, false))
    table.insert(actions, PVP.event(EVENT_FLAG_H, 0, false))
    table.insert(actions, PVP.score(player, COLUMN_CAPTURES))

    if score >= MAX_SCORE then
        finish(actions, input, team)
    else
        table.insert(actions, { action = "SET_TIMER", timer_id = T_RESPAWN, duration = RESPAWN_MS })
    end
end

-- The carrier of the flag `team` owns loses it.
local function drop(actions, input, player, enemy_team)
    local flag = flag_of(enemy_team)
    local carrier_team = PVP.other_team(enemy_team)
    table.insert(actions, { action = "SET_GUID", data_id = flag.keeper, guid = PVP.NO_GUID })
    table.insert(actions, { action = "SET_FLAG_CARRIER", team = enemy_team })
    if not PVP.in_progress(input) then
        table.insert(actions, PVP.data(flag.state, ON_BASE))
        return
    end
    table.insert(actions, PVP.data(flag.state, ON_GROUND))
    local p = PVP.player(input, player)
    if p then
        table.insert(actions, {
            action = "SPAWN_GAMEOBJECT", entry = flag.ground, x = p.x, y = p.y, z = p.z, o = 0, duration = 0,
        })
    end
    table.insert(actions, PVP.world_state(flag.carrier_state, 1))
    table.insert(actions, PVP.world_state(flag.taken_state, -1))
    table.insert(actions, PVP.announce(flag.dropped_text, PVP.chat_of(carrier_team), player))
    table.insert(actions, { action = "SET_TIMER", timer_id = flag.drop_timer, duration = DROP_MS })
end

-- `team`'s flag just went home: is their carrier of the enemy flag already
-- standing in their flag room? (The trigger fired before; it will not again.)
local function forced_capture(actions, input, team)
    local enemy_flag = flag_of(PVP.other_team(team))
    local keeper = input:GetGuid(enemy_flag.keeper)
    if not PVP.guid_is_set(keeper) then
        return
    end
    local p = PVP.player(input, keeper)
    local base = team == TEAM_ALLIANCE and ALLIANCE_BASE or HORDE_BASE
    if p and PVP.distance(p, base.x, base.y, base.z) <= 7 then
        capture(actions, input, keeper, team)
    end
end

function wsg:OnLoad(input)
    return {
        PVP.event(BG_EVENT_DOOR, 0, true),
        PVP.event(BG_EVENT_GHOST_GATE, 0, true),
        PVP.world_state(WS_CAPTURES_ALLIANCE, 0),
        PVP.world_state(WS_CAPTURES_HORDE, 0),
        PVP.world_state(WS_CAPTURES_MAX, MAX_SCORE),
        PVP.world_state(WS_FLAG_TAKEN_ALLIANCE, 0),
        PVP.world_state(WS_FLAG_TAKEN_HORDE, 0),
        PVP.world_state(WS_FLAG_STATE_ALLIANCE, 1),
        PVP.world_state(WS_FLAG_STATE_HORDE, 1),
    }
end

function wsg:OnMatchEvent(input, event)
    if event == "ONE_MINUTE" then
        return { PVP.announce(TEXT_ONE_MINUTE) }
    elseif event == "HALF_MINUTE" then
        return { PVP.announce(TEXT_HALF_MINUTE) }
    elseif event == "BEGIN" then
        return {
            { action = "DOOR_EVENT", event1 = BG_EVENT_DOOR, event2 = 0, open = true },
            PVP.event(EVENT_SPIRIT_GUIDES, 0, true),
            PVP.event(EVENT_FLAG_A, 0, true),
            PVP.event(EVENT_FLAG_H, 0, true),
            PVP.event(BG_EVENT_GHOST_GATE, 0, false),
            PVP.announce(TEXT_BEGUN),
        }
    elseif event == "DOORS_DESPAWN" then
        return { PVP.event(BG_EVENT_DOOR, 0, false) }
    end
    return {}
end

function wsg:OnGameObjectUse(input, player, go, entry, event1, event2)
    if not PVP.in_progress(input) then
        return {}
    end
    local team = PVP.team_of(input, player)
    local actions = {}
    -- A flag at its base: only the enemy may take it.
    if event1 == EVENT_FLAG_A and team == TEAM_HORDE and input:GetData(D_STATE_A) == ON_BASE then
        pick_up(actions, input, player, TEAM_HORDE)
    elseif event1 == EVENT_FLAG_H and team == TEAM_ALLIANCE and input:GetData(D_STATE_H) == ON_BASE then
        pick_up(actions, input, player, TEAM_ALLIANCE)
    -- A dropped flag: its own side returns it, the enemy takes it again.
    elseif entry == GO_ALLIANCE_FLAG_GROUND or entry == GO_HORDE_FLAG_GROUND then
        local owner = entry == GO_ALLIANCE_FLAG_GROUND and TEAM_ALLIANCE or TEAM_HORDE
        local flag = flag_of(owner)
        if input:GetData(flag.state) ~= ON_GROUND then
            return {}
        end
        if team == owner then
            table.insert(actions, { action = "DESPAWN_GAMEOBJECT", entry = flag.ground })
            table.insert(actions, PVP.world_state(flag.carrier_state, WAIT_RESPAWN))
            table.insert(actions, PVP.world_state(flag.taken_state, 0))
            respawn_flag(actions, owner, false)
            table.insert(actions, PVP.sound(SOUND_RETURNED))
            table.insert(actions, PVP.score(player, COLUMN_RETURNS))
            table.insert(actions, PVP.announce(flag.returned_text, PVP.chat_of(team), player))
            forced_capture(actions, input, owner)
        else
            pick_up(actions, input, player, team)
        end
    end
    return actions
end

function wsg:OnAreaTrigger(input, player, trigger)
    if BUFF_TRIGGERS[trigger] then
        return true
    end
    local team = PVP.team_of(input, player)
    if trigger == AT_HORDE_EXIT and team == TEAM_HORDE then
        return { { action = "LEAVE_BATTLEGROUND", player = player } }
    elseif trigger == AT_ALLIANCE_EXIT and team == TEAM_ALLIANCE then
        return { { action = "LEAVE_BATTLEGROUND", player = player } }
    end
    if not PVP.in_progress(input) then
        return false
    end
    local actions = {}
    if trigger == AT_ALLIANCE_BASE and team == TEAM_ALLIANCE then
        if input:GetData(D_STATE_H) ~= ON_BASE and input:GetData(D_STATE_A) == ON_BASE
            and input:GetGuid(G_KEEPER_H) == player then
            capture(actions, input, player, TEAM_ALLIANCE)
        end
    elseif trigger == AT_HORDE_BASE and team == TEAM_HORDE then
        if input:GetData(D_STATE_A) ~= ON_BASE and input:GetData(D_STATE_H) == ON_BASE
            and input:GetGuid(G_KEEPER_A) == player then
            capture(actions, input, player, TEAM_HORDE)
        end
    end
    actions.handled = trigger == AT_ALLIANCE_BASE or trigger == AT_HORDE_BASE
    return actions
end

function wsg:Update(input)
    if not PVP.in_progress(input) then
        return {}
    end
    local actions = {}
    -- Both flags come back after a capture.
    if (input:GetData(D_STATE_A) == WAIT_RESPAWN or input:GetData(D_STATE_H) == WAIT_RESPAWN)
        and input:IsTimerReady(T_RESPAWN) then
        respawn_flag(actions, TEAM_ALLIANCE, true)
    end
    -- A dropped flag lying too long goes home.
    for _, team in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
        local flag = flag_of(team)
        if input:GetData(flag.state) == ON_GROUND and input:IsTimerReady(flag.drop_timer) then
            respawn_flag(actions, team, false)
            table.insert(actions, PVP.world_state(flag.taken_state, 0))
            table.insert(actions, PVP.announce(flag.respawned_text))
            table.insert(actions, PVP.sound(SOUND_FLAGS_RESPAWNED))
            table.insert(actions, { action = "DESPAWN_GAMEOBJECT", entry = flag.ground })
            forced_capture(actions, input, team)
        end
    end
    return actions
end

-- Losing the carried-flag aura (clicked off, immunity, death) drops the flag.
function wsg:OnAuraRemoved(input, player, spell)
    local actions = {}
    if spell == SPELL_WARSONG_FLAG and input:GetGuid(G_KEEPER_H) == player then
        drop(actions, input, player, TEAM_HORDE)
    elseif spell == SPELL_SILVERWING_FLAG and input:GetGuid(G_KEEPER_A) == player then
        drop(actions, input, player, TEAM_ALLIANCE)
    end
    return actions
end

function wsg:OnPlayerDeath(input, victim, killer)
    local actions = {}
    if input:GetGuid(G_KEEPER_H) == victim then
        drop(actions, input, victim, TEAM_HORDE)
        table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = victim, spell_id = SPELL_WARSONG_FLAG })
    elseif input:GetGuid(G_KEEPER_A) == victim then
        drop(actions, input, victim, TEAM_ALLIANCE)
        table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = victim, spell_id = SPELL_SILVERWING_FLAG })
    end
    return actions
end

-- A carrier who left the battleground takes nothing with them: the flag goes home.
function wsg:OnPlayerLeave(input, player)
    local actions = {}
    for _, team in ipairs({ TEAM_ALLIANCE, TEAM_HORDE }) do
        local flag = flag_of(team)
        if input:GetGuid(flag.keeper) == player then
            table.insert(actions, { action = "SET_GUID", data_id = flag.keeper, guid = PVP.NO_GUID })
            table.insert(actions, { action = "SET_FLAG_CARRIER", team = team })
            table.insert(actions, { action = "PLAYER_REMOVE_AURA", player = player, spell_id = flag.aura })
            table.insert(actions, PVP.world_state(flag.taken_state, 0))
            table.insert(actions, PVP.world_state(flag.carrier_state, 1))
            if PVP.in_progress(input) then
                respawn_flag(actions, team, false)
            else
                table.insert(actions, PVP.data(flag.state, ON_BASE))
            end
        end
    end
    return actions
end

-- Before the gates open the dead return to their flag room; afterwards to the main graveyard.
function wsg:SelectGraveyard(input, player)
    local team = PVP.team_of(input, player)
    local started = PVP.in_progress(input)
    if team == TEAM_ALLIANCE then
        return started and GY_MAIN_ALLIANCE or GY_FLAGROOM_ALLIANCE
    end
    return started and GY_MAIN_HORDE or GY_FLAGROOM_HORDE
end

RegisterInstanceScript(489, wsg)
