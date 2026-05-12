--[[
    @script_type: creature_ai
    @entry: 13282
    @name: boss_noxxion

    Boss Noxxion - Maraudon
    Ported from MaNGOS boss_noxxion.cpp

    Noxxion periodically goes invisible and spawns 5 adds.
    While invisible he is untargetable (faction change + not selectable flag).
    We approximate this with morph to invisible model + faction swap.
]]

local SPELL_TOXIC_VOLLEY = 21687
local SPELL_UPPERCUT     = 22916

local NPC_NOXXION_ADD    = 13456
local DISPLAY_NOXXION    = 11172
local DISPLAY_INVISIBLE  = 11686

local TIMER_TOXIC_VOLLEY = 1
local TIMER_UPPERCUT     = 2
local TIMER_ADDS         = 3
local TIMER_REAPPEAR     = 4

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_TOXIC_VOLLEY, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_ADDS, duration = 19000 },
        { action = "SET_CUSTOM_DATA", key = "invisible", value = 0 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_TOXIC_VOLLEY, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 16000 },
        { action = "SET_TIMER", timer_id = TIMER_ADDS, duration = 19000 },
        { action = "SET_CUSTOM_DATA", key = "invisible", value = 0 },
        { action = "MORPH", display_id = DISPLAY_NOXXION },
        { action = "SET_FACTION", faction_id = 16 },
    }
end

function boss:OnDeath(input, killer_guid)
    return {
        { action = "DEMORPH" },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    local invisible = input:GetCustomData("invisible") or 0

    -- Handle reappear from invisible phase
    if invisible == 1 then
        if input:IsTimerReady(TIMER_REAPPEAR) then
            table.insert(actions, { action = "SET_FACTION", faction_id = 14 })
            table.insert(actions, { action = "MORPH", display_id = DISPLAY_NOXXION })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "invisible", value = 0 })
        end
        return actions
    end

    if not input.is_in_combat then return actions end

    -- Toxic Volley
    if input:IsTimerReady(TIMER_TOXIC_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TOXIC_VOLLEY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TOXIC_VOLLEY, duration = 9000 })
    end

    -- Uppercut
    if input:IsTimerReady(TIMER_UPPERCUT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_UPPERCUT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_UPPERCUT, duration = 12000 })
    end

    -- Go invisible and spawn adds
    if input:IsTimerReady(TIMER_ADDS) then
        table.insert(actions, { action = "INTERRUPT_SPELL" })
        table.insert(actions, { action = "SET_FACTION", faction_id = 35 })
        table.insert(actions, { action = "MORPH", display_id = DISPLAY_INVISIBLE })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "invisible", value = 1 })

        -- Spawn 5 adds around the boss
        local px, py, pz = input.position.x, input.position.y, input.position.z
        for i = 1, 5 do
            local angle = (i / 5) * 6.283
            local ox = px + 8.0 * math.cos(angle)
            local oy = py + 8.0 * math.sin(angle)
            table.insert(actions, { action = "SPAWN_CREATURE", entry = NPC_NOXXION_ADD, x = ox, y = oy, z = pz, o = 0, summon_type = "TIMED_DESPAWN_OUT_OF_COMBAT", duration = 90000 })
        end

        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REAPPEAR, duration = 15000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ADDS, duration = 40000 })
    end

    return actions
end

RegisterCreatureAI(13282, boss)
