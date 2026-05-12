--[[
    @script_type: creature_ai
    @entry: 10001
    @name: test_new_callbacks

    Test script demonstrating new MaNGOS-style callbacks:
    - JustRespawned
    - HealedBy
    - SpellHitTarget
    - MovementInform
    - JustReachedHome
    - MoveInLineOfSight
    - JustSummoned
    - SummonedCreatureJustDied
    - SummonedCreatureDespawn
]]

local testBoss = {}

-- Timer IDs
local TIMER_SUMMON_ADDS = 1
local TIMER_SPELL_CAST = 2
local TIMER_MOVEMENT = 3

-- Phases
local PHASE_NORMAL = 1
local PHASE_ENRAGED = 2

-- Custom data keys
local KEY_ADDS_SPAWNED = "adds_spawned"
local KEY_ADDS_ALIVE = "adds_alive"
local KEY_HEALS_RECEIVED = "heals_received"

-- NPC entry to summon
local SUMMON_ENTRY = 10002

-- ============================================================================
-- LIFECYCLE CALLBACKS
-- ============================================================================

function testBoss:JustRespawned(input)
    print("[LUA] Boss " .. input.entry .. " just respawned!")
    return {
        { action = "SAY", text = "I have been reborn!" },
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_SPAWNED, value = 0 },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = 0 },
        { action = "SET_CUSTOM_DATA", key = KEY_HEALS_RECEIVED, value = 0 },
    }
end

function testBoss:OnSpawn(input)
    print("[LUA] Boss spawned at position: " .. input.position.x .. ", " .. input.position.y)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_SPAWNED, value = 0 },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = 0 },
        { action = "SET_CUSTOM_DATA", key = KEY_HEALS_RECEIVED, value = 0 },
    }
end

-- ============================================================================
-- COMBAT CALLBACKS
-- ============================================================================

function testBoss:OnEnterCombat(input)
    return {
        { action = "YELL", text = "You dare challenge me?!" },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_ADDS, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_SPELL_CAST, duration = 5000 },
    }
end

function testBoss:OnUpdate(input)
    local actions = {}

    -- Phase transition at 50% health
    if input.phase == PHASE_NORMAL and input.health_pct < 0.5 then
        table.insert(actions, { action = "YELL", text = "You will regret this! ENRAGE!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
        -- Summon 2 adds immediately when enraged
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = SUMMON_ENTRY,
            x = input.position.x + 5,
            y = input.position.y,
            z = input.position.z,
            summon_type = "CORPSE_DESPAWN",
            duration = 60000
        })
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = SUMMON_ENTRY,
            x = input.position.x - 5,
            y = input.position.y,
            z = input.position.z,
            summon_type = "CORPSE_DESPAWN",
            duration = 60000
        })
    end

    -- Summon adds periodically
    if input:IsTimerReady(TIMER_SUMMON_ADDS) and input.is_in_combat then
        local adds_spawned = input:GetCustomData(KEY_ADDS_SPAWNED)
        if adds_spawned < 10 then -- Max 10 summons
            table.insert(actions, { action = "SAY", text = "Minions, aid me!" })
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = SUMMON_ENTRY,
                x = input.position.x + math.random(-10, 10),
                y = input.position.y + math.random(-10, 10),
                z = input.position.z,
                summon_type = "CORPSE_DESPAWN",
                duration = 60000
            })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_ADDS, duration = 20000 })
        end
    end

    -- Cast spell periodically
    if input:IsTimerReady(TIMER_SPELL_CAST) and input.is_in_combat then
        local target = input:GetHighestThreatTarget()
        if target then
            table.insert(actions, { action = "CAST_SPELL", spell_id = 172, target = target.guid }) -- Shadow Bolt
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPELL_CAST, duration = 8000 })
    end

    return actions
end

function testBoss:OnDeath(input, killer_guid)
    return {
        { action = "YELL", text = "I... have been... defeated..." },
        { action = "SAY", text = "Adds spawned: " .. input:GetCustomData(KEY_ADDS_SPAWNED) },
        { action = "SAY", text = "Heals received: " .. input:GetCustomData(KEY_HEALS_RECEIVED) },
    }
end

function testBoss:OnEvade(input)
    return {
        { action = "SAY", text = "Cowards! Run while you can!" },
    }
end

-- ============================================================================
-- HEALING CALLBACK
-- ============================================================================

function testBoss:HealedBy(input, healer_guid, amount, spell_id)
    print("[LUA] Boss healed by " .. tostring(healer_guid) .. " for " .. amount .. " (spell: " .. tostring(spell_id) .. ")")

    local heals_count = input:GetCustomData(KEY_HEALS_RECEIVED) + 1

    local actions = {
        { action = "SET_CUSTOM_DATA", key = KEY_HEALS_RECEIVED, value = heals_count }
    }

    -- React to being healed
    if amount > 500 then
        table.insert(actions, { action = "SAY", text = "Your healing will not save you!" })
        -- Cast interrupt spell at healer if possible
        table.insert(actions, { action = "CAST_SPELL", spell_id = 116, target = healer_guid }) -- Frostbolt (as interrupt example)
    end

    return actions
end

-- ============================================================================
-- SPELL TARGET CALLBACK
-- ============================================================================

function testBoss:SpellHitTarget(input, target_guid, spell_id)
    print("[LUA] Our spell " .. spell_id .. " hit target " .. tostring(target_guid))

    -- If we hit with Shadow Bolt (172), taunt
    if spell_id == 172 then
        return {
            { action = "SAY", text = "Feel the power of shadow!" }
        }
    end

    return {}
end

-- ============================================================================
-- MOVEMENT CALLBACKS
-- ============================================================================

function testBoss:MovementInform(input, movement_type, point_id)
    print("[LUA] Movement complete - type: " .. movement_type .. ", point: " .. point_id)

    return {
        { action = "SAY", text = "I have reached waypoint " .. point_id }
    }
end

function testBoss:JustReachedHome(input)
    print("[LUA] Boss reached home position after evade")

    return {
        { action = "SAY", text = "Back to my lair..." },
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = 0 },
    }
end

function testBoss:MoveInLineOfSight(input, unit_guid, is_hostile)
    print("[LUA] Unit " .. tostring(unit_guid) .. " in line of sight (hostile: " .. tostring(is_hostile) .. ")")

    -- Aggro hostile units that come into line of sight
    if is_hostile and not input.is_in_combat then
        return {
            { action = "SAY", text = "I see you, intruder!" },
            { action = "ENTER_COMBAT", target = unit_guid }
        }
    end

    return {}
end

-- ============================================================================
-- SUMMONING CALLBACKS
-- ============================================================================

function testBoss:JustSummoned(input, summoned_guid, entry)
    print("[LUA] Summoned creature " .. entry .. " with GUID " .. tostring(summoned_guid))

    local adds_spawned = input:GetCustomData(KEY_ADDS_SPAWNED) + 1
    local adds_alive = input:GetCustomData(KEY_ADDS_ALIVE) + 1

    local actions = {
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_SPAWNED, value = adds_spawned },
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = adds_alive },
    }

    -- Taunt on every 3rd summon
    if adds_spawned % 3 == 0 then
        table.insert(actions, { action = "SAY", text = "More minions join the fray! (" .. adds_alive .. " alive)" })
    end

    return actions
end

function testBoss:SummonedCreatureJustDied(input, summoned_guid)
    print("[LUA] Summoned creature died: " .. tostring(summoned_guid))

    local adds_alive = input:GetCustomData(KEY_ADDS_ALIVE) - 1

    local actions = {
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = adds_alive }
    }

    -- React when all adds are dead during enrage phase
    if adds_alive == 0 and input.phase == PHASE_ENRAGED then
        table.insert(actions, { action = "YELL", text = "No! All my minions have fallen!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = 172, target = "self" }) -- Cast spell on self in frustration
    else
        table.insert(actions, { action = "SAY", text = "One of my minions has fallen! (" .. adds_alive .. " remain)" })
    end

    return actions
end

function testBoss:SummonedCreatureDespawn(input, summoned_guid)
    print("[LUA] Summoned creature despawned: " .. tostring(summoned_guid))

    local adds_alive = input:GetCustomData(KEY_ADDS_ALIVE) - 1

    return {
        { action = "SET_CUSTOM_DATA", key = KEY_ADDS_ALIVE, value = adds_alive }
    }
end

-- ============================================================================
-- REGISTER SCRIPT
-- ============================================================================

RegisterCreatureAI(10001, testBoss)

