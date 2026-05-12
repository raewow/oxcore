--[[
    @script_type: creature_ai
    @entry: 4275
    @name: boss_arugal

    Archmage Arugal - Shadowfang Keep (final boss)
    Ported from MaNGOS boss_arugal (shadowfang_keep.cpp)

    Abilities:
    - Void Bolt: Shadow bolt cast at current target
    - Arugal's Curse: Worgen transformation on a random target
    - Thundershock: AoE when players are in melee range
    - Shadow Port: Teleports between 3 positions in the room
    - Yells periodically during combat
]]

local SPELL_VOID_BOLT         = 7588
local SPELL_SHADOW_PORT_UPPER = 7587
local SPELL_SHADOW_PORT_SPAWN = 7586
local SPELL_SHADOW_PORT_STAIRS = 7136
local SPELL_ARUGALS_CURSE     = 7621
local SPELL_THUNDERSHOCK      = 7803

local TIMER_VOID_BOLT         = 1
local TIMER_CURSE             = 2
local TIMER_THUNDERSHOCK      = 3
local TIMER_TELEPORT          = 4
local TIMER_YELL              = 5

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "You, too, shall serve!" },
        { action = "CAST_SPELL", spell_id = SPELL_VOID_BOLT, target = "current_target" },
        { action = "SET_TIMER", timer_id = TIMER_VOID_BOLT, duration = math.random(2900, 4800) },
        { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_THUNDERSHOCK, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = math.random(22000, 26000) },
        { action = "SET_TIMER", timer_id = TIMER_YELL, duration = math.random(32000, 46000) },
        { action = "SET_CUSTOM_DATA", key = "position", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_VOID_BOLT, duration = math.random(2900, 4800) },
        { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_THUNDERSHOCK, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = math.random(22000, 26000) },
        { action = "SET_TIMER", timer_id = TIMER_YELL, duration = math.random(32000, 46000) },
        { action = "SET_CUSTOM_DATA", key = "position", value = 0 },
    }
end

function boss:OnKill(input)
    return {
        { action = "YELL", text = "Another falls!" },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Combat yell
    if input:IsTimerReady(TIMER_YELL) then
        table.insert(actions, { action = "YELL", text = "Release your rage!" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YELL, duration = math.random(34000, 68000) })
    end

    -- Arugal's Curse on random target
    if input:IsTimerReady(TIMER_CURSE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARUGALS_CURSE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE, duration = math.random(20000, 35000) })
    end

    -- Thundershock (melee range AoE)
    if input:IsTimerReady(TIMER_THUNDERSHOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERSHOCK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_THUNDERSHOCK, duration = math.random(30200, 38500) })
    end

    -- Void Bolt
    if input:IsTimerReady(TIMER_VOID_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VOID_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VOID_BOLT, duration = math.random(2900, 4800) })
    end

    -- Teleport between positions
    if input:IsTimerReady(TIMER_TELEPORT) then
        local current_pos = input:GetCustomData("position") or 0
        local new_pos
        -- Pick a different position (0=spawn ledge, 1=upper ledge, 2=stairs)
        if current_pos == 0 then
            new_pos = math.random(1, 2)
        else
            new_pos = math.random(0, 1)
            if new_pos == current_pos then
                new_pos = 2
            end
        end

        if new_pos == 0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_PORT_SPAWN, target = "self" })
        elseif new_pos == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_PORT_UPPER, target = "self" })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_PORT_STAIRS, target = "self" })
        end

        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "position", value = new_pos })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = math.random(48000, 55000) })
    end

    return actions
end

RegisterCreatureAI(4275, boss)
