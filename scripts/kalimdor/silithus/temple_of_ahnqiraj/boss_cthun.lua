--[[
    @script_type: creature_ai
    @entry: 15589, 15727, 15726, 15725, 15728, 15334, 15802
    @name: boss_cthun

    C'Thun - Temple of Ahn'Qiraj
    Verified against vmangos boss_cthun.cpp

    Multi-creature encounter:
    - Eye of C'Thun (15589): Phase 1 - Green Beam + Dark Glare rotation
    - C'Thun Body (15727): Phase 2 - Tentacles, stomach mechanic, weakened phase
    - Eye Tentacle (15726): Mind Flay random targets
    - Claw Tentacle (15725): Ground Rupture + Hamstring
    - Giant Claw Tentacle (15728): Ground Rupture + Thrash + Hamstring
    - Giant Eye Tentacle (15334): Green Beam
    - Flesh Tentacle (15802): Must be killed to weaken C'Thun
]]

-- Spells
local SPELL_GREEN_BEAM              = 26134
local SPELL_DARK_GLARE              = 26029
local SPELL_RED_COLORATION          = 22518
local SPELL_MIND_FLAY               = 26143
local SPELL_GROUND_RUPTURE          = 26139
local SPELL_HAMSTRING               = 26141
local SPELL_TRANSFORM               = 26232
local SPELL_MASSIVE_GROUND_RUPTURE  = 26100
local SPELL_THRASH                  = 3391
local SPELL_MOUTH_TENTACLE          = 26332
local SPELL_DIGESTIVE_ACID          = 26476
local SPELL_EXIT_STOMACH            = 25383

-- NPC Entries
local NPC_EYE_OF_CTHUN              = 15589
local NPC_CTHUN                     = 15727
local MOB_CLAW_TENTACLE             = 15725
local MOB_EYE_TENTACLE              = 15726
local MOB_GIANT_CLAW_TENTACLE       = 15728
local MOB_GIANT_EYE_TENTACLE        = 15334
local MOB_FLESH_TENTACLE            = 15802

-- Phases
local PHASE_EYE_NORMAL              = 0
local PHASE_EYE_DARK_GLARE          = 1
local PHASE_TRANSITION              = 2
local PHASE_CTHUN_BODY              = 3
local PHASE_CTHUN_WEAKENED          = 4

--------------------------------------------------------------------------------
-- Eye of C'Thun (15589) - Phase 1 Boss
--------------------------------------------------------------------------------
local eye = {}

-- Eye timers
local EYE_TIMER_BEAM                = 1
local EYE_TIMER_PHASE               = 2
local EYE_TIMER_CLAW_TENTACLE       = 3
local EYE_TIMER_EYE_TENTACLE        = 4
local EYE_TIMER_DARK_GLARE          = 5

function eye:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_EYE_NORMAL },
        { action = "SET_TIMER", timer_id = EYE_TIMER_BEAM, duration = 3000 },
        { action = "SET_TIMER", timer_id = EYE_TIMER_PHASE, duration = 45000 },
        { action = "SET_TIMER", timer_id = EYE_TIMER_CLAW_TENTACLE, duration = 5000 },
        { action = "SET_TIMER", timer_id = EYE_TIMER_EYE_TENTACLE, duration = 45000 },
        { action = "SET_TIMER", timer_id = EYE_TIMER_DARK_GLARE, duration = 1000 },
        { action = "SET_CUSTOM_DATA", key = "dark_glare_ticks", value = 0 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function eye:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input.phase == PHASE_EYE_NORMAL then
        -- Green Beam on random target
        if input:IsTimerReady(EYE_TIMER_BEAM) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GREEN_BEAM, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_BEAM, duration = 3000 })
        end

        -- Spawn Claw Tentacle on random target
        if input:IsTimerReady(EYE_TIMER_CLAW_TENTACLE) then
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_CLAW_TENTACLE, x = 0, y = 0, z = 0, o = 0, summon_type = 1, duration = 30000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_CLAW_TENTACLE, duration = 12500 })
        end

        -- Spawn 8 Eye Tentacles in a ring
        if input:IsTimerReady(EYE_TIMER_EYE_TENTACLE) then
            -- 8 positions around the Eye
            local offsets = {
                {0, 20}, {10, 10}, {20, 0}, {10, -10},
                {0, -20}, {-10, -10}, {-20, 0}, {-10, 10},
            }
            for _, off in ipairs(offsets) do
                table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_EYE_TENTACLE, x = off[1], y = off[2], z = 0, o = 0, summon_type = 1, duration = 45000 })
            end
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_EYE_TENTACLE, duration = 45000 })
        end

        -- Switch to Dark Glare phase
        if input:IsTimerReady(EYE_TIMER_PHASE) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_EYE_DARK_GLARE })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RED_COLORATION, target = "self" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "dark_glare_ticks", value = 0 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_DARK_GLARE, duration = 1000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_PHASE, duration = 38000 })
        end

    elseif input.phase == PHASE_EYE_DARK_GLARE then
        -- Dark Glare beam sweeps for 35 seconds (1 tick/second)
        local ticks = input:GetCustomData("dark_glare_ticks") or 0
        if ticks < 35 and input:IsTimerReady(EYE_TIMER_DARK_GLARE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DARK_GLARE, target = "self" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "dark_glare_ticks", value = ticks + 1 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_DARK_GLARE, duration = 1000 })
        end

        -- Switch back to Eye Beam phase
        if input:IsTimerReady(EYE_TIMER_PHASE) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_EYE_NORMAL })
            table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_RED_COLORATION })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_BEAM, duration = 3000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_CLAW_TENTACLE, duration = 5000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_EYE_TENTACLE, duration = 45000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = EYE_TIMER_PHASE, duration = 45000 })
        end
    end

    return actions
end

RegisterCreatureAI(15589, eye)

--------------------------------------------------------------------------------
-- C'Thun Body (15727) - Phase 2 Boss
--------------------------------------------------------------------------------
local cthun = {}

local CTHUN_TIMER_PHASE             = 1
local CTHUN_TIMER_EYE_TENTACLE      = 2
local CTHUN_TIMER_GIANT_CLAW        = 3
local CTHUN_TIMER_GIANT_EYE         = 4
local CTHUN_TIMER_STOMACH_ACID      = 5
local CTHUN_TIMER_STOMACH_ENTER     = 6

function cthun:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_TRANSITION },
        { action = "SET_TIMER", timer_id = CTHUN_TIMER_PHASE, duration = 10000 },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_CUSTOM_DATA", key = "flesh_killed", value = 0 },
    }
end

function cthun:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    if input.phase == PHASE_TRANSITION then
        -- Emerge after 10 seconds
        if input:IsTimerReady(CTHUN_TIMER_PHASE) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_CTHUN_BODY })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRANSFORM, target = "self" })
            table.insert(actions, { action = "SET_HEALTH_PERCENT", percent = 100 })
            table.insert(actions, { action = "SET_COMBAT_WITH_ZONE" })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "flesh_killed", value = 0 })
            -- Spawn 2 flesh tentacles
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_FLESH_TENTACLE, x = -8571.0, y = 1990.0, z = -98.0, o = 1.22, summon_type = 1, duration = 0 })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_FLESH_TENTACLE, x = -8525.0, y = 1994.0, z = -98.0, o = 2.12, summon_type = 1, duration = 0 })
            -- Set body phase timers (P2_FIRST_* constants from vmangos)
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_EYE_TENTACLE, duration = 38000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_CLAW, duration = 8000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_EYE, duration = 38000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ACID, duration = 4000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ENTER, duration = 14750 })  -- 18000 - 3250 (STOMACH_GRAB_DURATION)
        end

    elseif input.phase == PHASE_CTHUN_BODY then
        -- Check if both flesh tentacles are dead => weaken
        local flesh_killed = input:GetCustomData("flesh_killed") or 0
        local flesh_list = input:GetCreaturesByEntry(15802)
        local alive_count = 0
        if flesh_list then
            for _, c in ipairs(flesh_list) do
                if c.is_alive then
                    alive_count = alive_count + 1
                end
            end
        end
        -- If no flesh tentacles alive, enter weakened state
        if alive_count == 0 and flesh_killed == 0 then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_CTHUN_WEAKENED })
            table.insert(actions, { action = "EMOTE", text = "C'Thun is weakened!" })
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RED_COLORATION, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_PHASE, duration = 45000 })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "flesh_killed", value = 1 })
            return actions
        end

        -- Stomach acid every 4s
        if input:IsTimerReady(CTHUN_TIMER_STOMACH_ACID) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DIGESTIVE_ACID, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ACID, duration = 4000 })
        end

        -- Stomach enter (swallow a player); repeat STOMACH_GRAB_COOLDOWN = 10000
        if input:IsTimerReady(CTHUN_TIMER_STOMACH_ENTER) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MOUTH_TENTACLE, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ENTER, duration = 10000 })
        end

        -- Giant Claw Tentacle every 60s
        if input:IsTimerReady(CTHUN_TIMER_GIANT_CLAW) then
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_GIANT_CLAW_TENTACLE, x = 0, y = 0, z = 0, o = 0, summon_type = 1, duration = 30000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_CLAW, duration = 60000 })
        end

        -- Giant Eye Tentacle every 60s
        if input:IsTimerReady(CTHUN_TIMER_GIANT_EYE) then
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_GIANT_EYE_TENTACLE, x = 0, y = 0, z = 0, o = 0, summon_type = 1, duration = 30000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_EYE, duration = 60000 })
        end

        -- Eye Tentacles ring every 30s
        if input:IsTimerReady(CTHUN_TIMER_EYE_TENTACLE) then
            local offsets = {
                {0, 25}, {12, 12}, {25, 0}, {12, -12},
                {0, -25}, {-12, -12}, {-25, 0}, {-12, 12},
            }
            for _, off in ipairs(offsets) do
                table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_EYE_TENTACLE, x = off[1], y = off[2], z = 0, o = 0, summon_type = 1, duration = 30000 })
            end
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_EYE_TENTACLE, duration = 30000 })
        end

    elseif input.phase == PHASE_CTHUN_WEAKENED then
        -- Weakened for 45 seconds, then back to body phase
        if input:IsTimerReady(CTHUN_TIMER_PHASE) then
            table.insert(actions, { action = "SET_PHASE", phase = PHASE_CTHUN_BODY })
            table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_RED_COLORATION })
            table.insert(actions, { action = "SET_CUSTOM_DATA", key = "flesh_killed", value = 0 })
            -- Respawn flesh tentacles
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_FLESH_TENTACLE, x = -8571.0, y = 1990.0, z = -98.0, o = 1.22, summon_type = 1, duration = 0 })
            table.insert(actions, { action = "SPAWN_CREATURE", entry = MOB_FLESH_TENTACLE, x = -8525.0, y = 1994.0, z = -98.0, o = 2.12, summon_type = 1, duration = 0 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_EYE_TENTACLE, duration = 38000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_CLAW, duration = 8000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_GIANT_EYE, duration = 38000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ACID, duration = 4000 })
            table.insert(actions, { action = "SET_TIMER", timer_id = CTHUN_TIMER_STOMACH_ENTER, duration = 14750 })
        end
    end

    return actions
end

RegisterCreatureAI(15727, cthun)

--------------------------------------------------------------------------------
-- Eye Tentacle (15726) - Mind Flay adds
--------------------------------------------------------------------------------
local eye_tentacle = {}

local ET_TIMER_MINDFLAY     = 1
local ET_TIMER_KILLSELF     = 2

function eye_tentacle:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = ET_TIMER_MINDFLAY, duration = 500 },
        { action = "SET_TIMER", timer_id = ET_TIMER_KILLSELF, duration = 35000 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function eye_tentacle:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Kill self after 35s to prevent overlap
    if input:IsTimerReady(ET_TIMER_KILLSELF) then
        table.insert(actions, { action = "KILL_SELF" })
        return actions
    end

    -- Mind Flay random target
    if input:IsTimerReady(ET_TIMER_MINDFLAY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIND_FLAY, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = ET_TIMER_MINDFLAY, duration = 10100 })
    end

    return actions
end

RegisterCreatureAI(15726, eye_tentacle)

--------------------------------------------------------------------------------
-- Claw Tentacle (15725) - Melee adds
--------------------------------------------------------------------------------
local claw_tentacle = {}

local CT_TIMER_RUPTURE      = 1
local CT_TIMER_HAMSTRING    = 2

function claw_tentacle:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = CT_TIMER_RUPTURE, duration = 500 },
        { action = "SET_TIMER", timer_id = CT_TIMER_HAMSTRING, duration = 2000 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function claw_tentacle:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Ground Rupture
    if input:IsTimerReady(CT_TIMER_RUPTURE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GROUND_RUPTURE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = CT_TIMER_RUPTURE, duration = 30000 })
    end

    -- Hamstring
    if input:IsTimerReady(CT_TIMER_HAMSTRING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HAMSTRING, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = CT_TIMER_HAMSTRING, duration = 5000 })
    end

    return actions
end

RegisterCreatureAI(15725, claw_tentacle)

--------------------------------------------------------------------------------
-- Giant Claw Tentacle (15728)
--------------------------------------------------------------------------------
local giant_claw = {}

local GCT_TIMER_RUPTURE     = 1
local GCT_TIMER_THRASH      = 2
local GCT_TIMER_HAMSTRING   = 3

function giant_claw:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = GCT_TIMER_RUPTURE, duration = 500 },
        { action = "SET_TIMER", timer_id = GCT_TIMER_THRASH, duration = 5000 },
        { action = "SET_TIMER", timer_id = GCT_TIMER_HAMSTRING, duration = 2000 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function giant_claw:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Massive Ground Rupture
    if input:IsTimerReady(GCT_TIMER_RUPTURE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MASSIVE_GROUND_RUPTURE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = GCT_TIMER_RUPTURE, duration = 30000 })
    end

    -- Thrash
    if input:IsTimerReady(GCT_TIMER_THRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THRASH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = GCT_TIMER_THRASH, duration = 10000 })
    end

    -- Hamstring
    if input:IsTimerReady(GCT_TIMER_HAMSTRING) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HAMSTRING, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = GCT_TIMER_HAMSTRING, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(15728, giant_claw)

--------------------------------------------------------------------------------
-- Giant Eye Tentacle (15334) - Green Beam caster
--------------------------------------------------------------------------------
local giant_eye = {}

local GET_TIMER_BEAM        = 1

function giant_eye:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = GET_TIMER_BEAM, duration = 500 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function giant_eye:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Green Beam on random target
    if input:IsTimerReady(GET_TIMER_BEAM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GREEN_BEAM, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = GET_TIMER_BEAM, duration = 2100 })
    end

    return actions
end

RegisterCreatureAI(15334, giant_eye)

--------------------------------------------------------------------------------
-- Flesh Tentacle (15802) - Must be killed to weaken C'Thun
-- (No special AI beyond melee attack, just needs to exist as a target)
--------------------------------------------------------------------------------
local flesh = {}

function flesh:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
    }
end

function flesh:OnUpdate(input)
    return {}
end

RegisterCreatureAI(15802, flesh)
