--[[
    @script_type: creature_ai
    @entry: 10812
    @name: boss_dathrohan_balnazzar

    Dathrohan / Balnazzar - Stratholme (Living)
    Ported from vmangos boss_dathrohan_balnazzar.cpp

    Phase 1 (Dathrohan, entry 10812): Crusader abilities
    Phase 2 (Balnazzar, entry 10813, triggered at <40% HP): Demon abilities
    On death: spawns 32 skeleton adds in 8 groups of 4

    Note: Entry swap (UpdateEntry NPC_BALNAZZAR) on transform is not yet supported
    in the Lua creature AI API. The spell phase transition and add spawning on death
    are implemented below. The add spawn coordinates from the cpp are included.
]]

-- Phase 1: Dathrohan (Crusader) spells
local SPELL_CRUSADERS_HAMMER = 17286  -- AOE stun, self-cast
local SPELL_CRUSADER_STRIKE  = 17281  -- Melee damage on target
local SPELL_HOLY_STRIKE      = 17284  -- Weapon damage +3 on target

-- Phase 2: Balnazzar (Demon) spells
local SPELL_SHADOW_SHOCK     = 17399  -- Shadow damage on target
local SPELL_MIND_BLAST       = 17287  -- Shadow damage on target (both phases)
local SPELL_PSYCHIC_SCREAM   = 13704  -- AoE fear, self-cast
local SPELL_SLEEP            = 12098  -- Sleep a random target
local SPELL_MIND_CONTROL     = 17405  -- Mind control second-highest threat target

-- Transform spell
local SPELL_BALNAZZAR_TRANSFORM = 17288  -- Restore HP/mana, stun (triggers at <40%)

-- Skeleton add NPC entries
local NPC_SKEL_BERSERKER = 10391
local NPC_SKEL_GUARDIAN  = 10390

-- Timer IDs
local TIMER_CRUSADERS_HAMMER = 1
local TIMER_CRUSADER_STRIKE  = 2
local TIMER_HOLY_STRIKE      = 3
local TIMER_MIND_BLAST       = 4
local TIMER_SHADOW_SHOCK     = 5
local TIMER_PSYCHIC_SCREAM   = 6
local TIMER_SLEEP            = 7
local TIMER_MIND_CONTROL     = 8
local TIMER_CHECK_TRANSFORM  = 9

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SAY", text = "Ah, more adventurers come to meet their fate! Your deaths will serve the Scarlet Crusade well." },
        -- Phase 1 timers
        { action = "SET_TIMER", timer_id = TIMER_MIND_BLAST,       duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_CRUSADERS_HAMMER, duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_STRIKE,      duration = 18000 },
        -- Phase 2 timers (start counting so they're ready when we transform)
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHOCK,     duration = 3000 },
        { action = "SET_TIMER", timer_id = TIMER_PSYCHIC_SCREAM,   duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SLEEP,            duration = 9000 },
        { action = "SET_TIMER", timer_id = TIMER_MIND_CONTROL,     duration = 18000 },
        { action = "SET_TIMER", timer_id = TIMER_CHECK_TRANSFORM,  duration = 500 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Transform at <40% HP (phase 1 -> phase 2)
    if input:IsTimerReady(TIMER_CHECK_TRANSFORM) then
        if input.health_pct < 40 and not input.transformed then
            input.transformed = true
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BALNAZZAR_TRANSFORM, target = "self" })
            table.insert(actions, { action = "SAY", text = "Fools! Did you think you could destroy the great Balnazzar?!" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHECK_TRANSFORM, duration = 500 })
    end

    if not input.transformed then
        -- Phase 1: Dathrohan (Crusader)

        -- Mind Blast (both phases, current target, repeats every 15-20s)
        if input:IsTimerReady(TIMER_MIND_BLAST) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIND_BLAST, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIND_BLAST, duration = math.random(15000, 20000) })
        end

        -- Crusader's Hammer (AoE stun, self, repeats every 12s)
        if input:IsTimerReady(TIMER_CRUSADERS_HAMMER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CRUSADERS_HAMMER, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CRUSADERS_HAMMER, duration = 12000 })
        end

        -- Crusader Strike (current target, repeats every 15s)
        if input:IsTimerReady(TIMER_CRUSADER_STRIKE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CRUSADER_STRIKE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CRUSADER_STRIKE, duration = 15000 })
        end

        -- Holy Strike (current target, repeats every 15s)
        if input:IsTimerReady(TIMER_HOLY_STRIKE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_STRIKE, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_STRIKE, duration = 15000 })
        end
    else
        -- Phase 2: Balnazzar (Demon)

        -- Mind Blast (current target, repeats every 15-20s)
        if input:IsTimerReady(TIMER_MIND_BLAST) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIND_BLAST, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIND_BLAST, duration = math.random(15000, 20000) })
        end

        -- Shadow Shock (current target, repeats every 11s)
        if input:IsTimerReady(TIMER_SHADOW_SHOCK) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_SHOCK, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_SHOCK, duration = 11000 })
        end

        -- Psychic Scream (AoE fear, self, repeats every 20s)
        if input:IsTimerReady(TIMER_PSYCHIC_SCREAM) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PSYCHIC_SCREAM, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PSYCHIC_SCREAM, duration = 20000 })
        end

        -- Deep Sleep (random hostile, repeats every 15s)
        if input:IsTimerReady(TIMER_SLEEP) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SLEEP, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SLEEP, duration = 15000 })
        end

        -- Mind Control (second highest threat target, repeats every 25-30s)
        if input:IsTimerReady(TIMER_MIND_CONTROL) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIND_CONTROL, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIND_CONTROL, duration = math.random(25000, 30000) })
        end
    end

    return actions
end

function boss:OnDeath(input, killer_guid)
    local actions = {
        { action = "SAY", text = "Ugh... impossible..." },
    }

    -- Spawn 32 skeleton adds in 8 groups of 4 across the district
    local spawn_points = {
        {3444.156, -3090.626, 135.002, 2.240},
        {3449.123, -3087.009, 135.002, 2.240},
        {3446.246, -3093.466, 135.002, 2.240},
        {3451.160, -3089.904, 135.002, 2.240},
        {3457.995, -3080.916, 135.002, 3.784},
        {3454.302, -3076.330, 135.002, 3.784},
        {3460.975, -3078.901, 135.002, 3.784},
        {3457.338, -3073.979, 135.002, 3.784},
        {3479.995, -3062.916, 135.002, 3.784},
        {3476.302, -3058.330, 135.002, 3.784},
        {3482.975, -3060.901, 135.002, 3.784},
        {3479.338, -3055.979, 135.002, 3.784},
        {3501.995, -3074.916, 134.997, 3.784},
        {3498.302, -3070.330, 134.997, 3.784},
        {3504.975, -3072.901, 134.997, 3.784},
        {3501.338, -3067.979, 134.997, 3.784},
    }

    for _, pt in ipairs(spawn_points) do
        local entry = (math.random(0, 1) == 0) and NPC_SKEL_BERSERKER or NPC_SKEL_GUARDIAN
        table.insert(actions, {
            action = "SPAWN_CREATURE",
            entry = entry,
            x = pt[1], y = pt[2], z = pt[3], o = pt[4],
            summon_type = "dead_despawn",
            duration = 3600000,
        })
    end

    return actions
end

RegisterCreatureAI(10812, boss)
