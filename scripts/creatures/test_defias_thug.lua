--[[
    @script_type: creature_ai
    @entry: 38
    @name: test_defias_thug

    Test script for Defias Thug (entry 38) in Elwynn Forest.
    This is a simple example to verify the Lua scripting system works.
]]

local thug = {}

-- Timer constants
local TIMER_TAUNT = 1

-- Called when creature enters combat
function thug:OnEnterCombat(input)
    print("[LUA] Defias Thug entered combat!")
    return {
        { action = "SAY", text = "You dare challenge the Defias Brotherhood?" },
        { action = "SET_TIMER", timer_id = TIMER_TAUNT, duration = 5000 },
    }
end

-- Called every update tick while in combat
function thug:OnUpdate(input)
    local actions = {}

    -- Check if taunt timer is ready
    if input:IsTimerReady(TIMER_TAUNT) and input.is_in_combat then
        -- Random chance to taunt
        if math.random(1, 100) <= 30 then
            table.insert(actions, { action = "SAY", text = "The Brotherhood will never fall!" })
        end
        -- Reset timer
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAUNT, duration = 8000 })
    end

    return actions
end

-- Called when creature dies
function thug:OnDeath(input, killer_guid)
    print("[LUA] Defias Thug died!")
    return {
        { action = "SAY", text = "You haven't seen... the last... of us..." },
    }
end

-- Called when creature evades (resets)
function thug:OnEvade(input)
    print("[LUA] Defias Thug evaded!")
    return {
        { action = "SAY", text = "Coward! Run while you can!" },
    }
end

-- Called when creature spawns
function thug:OnSpawn(input)
    print("[LUA] Defias Thug spawned at position: " .. input.position.x .. ", " .. input.position.y)
    return {}
end

-- Register the script for entry 38 (Defias Thug)
RegisterCreatureAI(38, thug)

