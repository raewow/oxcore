--[[
    @script_type: creature_ai
    @entry: 14517
    @name: boss_jeklik

    High Priestess Jeklik - Zul'Gurub
    Verified against vmangos boss_jeklik.cpp

    Phase 1 (Bat Form, >50% HP): Charge, Screech, SonicBurst, Swoop, PierceArmor, SpawnBats
    Phase 2 (Priest Form, <=50% HP): SWP, MindFlay, GreatHeal, CurseOfBlood, SpawnFlyingBats
]]

local SPELL_CHARGE         = 24408  -- was 22911 (wrong)
local SPELL_SCREECH        = 6605
local SPELL_SONICBURST     = 23918
local SPELL_SWOOP          = 23919
local SPELL_PIERCEARMOR    = 12097
local SPELL_SHADOW_WORD_PAIN = 23952
local SPELL_MIND_FLAY      = 23953
local SPELL_GREAT_HEAL     = 23954
local SPELL_CURSE_OF_BLOOD = 16098
local SPELL_BAT_FORM       = 23966

local NPC_BAT          = 11368  -- bats spawned every 40s in phase 1
local NPC_FLYING_BAT   = 14965  -- bats spawned every 10s in phase 2

local TIMER_SPAWN_BATS     = 1
local TIMER_CHARGE         = 2
local TIMER_SCREECH        = 3
local TIMER_SONICBURST     = 4
local TIMER_SWOOP          = 5
local TIMER_PIERCEARMOR    = 6
local TIMER_SHADOW_WORD_PAIN = 7
local TIMER_MIND_FLAY      = 8
local TIMER_GREAT_HEAL     = 9
local TIMER_CURSE_OF_BLOOD = 10
local TIMER_FLYING_BATS    = 11

local PHASE_BAT    = 1  -- >50% hp
local PHASE_PRIEST = 2  -- <=50% hp

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10027 },  -- SAY_AGGRO
        { action = "SET_PHASE", phase = PHASE_BAT },
        { action = "CAST_SPELL", spell_id = SPELL_BAT_FORM, target = "self" },
        -- Phase 1 timers
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_BATS,  duration = 40000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,      duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_SCREECH,     duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SONICBURST,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SWOOP,       duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_PIERCEARMOR, duration = 9000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_BAT },
        { action = "SET_TIMER", timer_id = TIMER_SPAWN_BATS,  duration = 40000 },
        { action = "SET_TIMER", timer_id = TIMER_CHARGE,      duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_SCREECH,     duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SONICBURST,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_SWOOP,       duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_PIERCEARMOR, duration = 9000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Phase transition at 50% HP
    if input.phase == PHASE_BAT and input.health_pct <= 0.50 then
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_PRIEST })
        table.insert(actions, { action = "REMOVE_AURA", spell_id = SPELL_BAT_FORM })
        -- Phase 2 timers
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = 9000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIND_FLAY,        duration = 2000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GREAT_HEAL,       duration = 20000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD,   duration = 26000 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLYING_BATS,      duration = 10000 })
    end

    -- PHASE 1 (Bat Form)
    if input.phase == PHASE_BAT then
        if input:IsTimerReady(TIMER_SPAWN_BATS) then
            for _ = 1, 6 do
                table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_BAT })
            end
            table.insert(actions, { action = "SCRIPT_TEXT", text_id = 10370 })  -- TEXT_SUMMON_BATS
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SPAWN_BATS, duration = 65000 })
        end

        if input:IsTimerReady(TIMER_CHARGE) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE, target = "random_hostile" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CHARGE, duration = math.random(15000, 30000) })
        end

        if input:IsTimerReady(TIMER_SCREECH) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SCREECH, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SCREECH, duration = 30000 })
        end

        if input:IsTimerReady(TIMER_SONICBURST) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SONICBURST, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SONICBURST, duration = math.random(20000, 24000) })
        end

        if input:IsTimerReady(TIMER_SWOOP) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SWOOP, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SWOOP, duration = math.random(12000, 15000) })
        end

        if input:IsTimerReady(TIMER_PIERCEARMOR) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PIERCEARMOR, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PIERCEARMOR, duration = math.random(16000, 18000) })
        end
    end

    -- PHASE 2 (Priest Form)
    if input.phase == PHASE_PRIEST then
        if input:IsTimerReady(TIMER_FLYING_BATS) then
            table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_FLYING_BAT })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLYING_BATS, duration = 10000 })
        end

        if input:IsTimerReady(TIMER_SHADOW_WORD_PAIN) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_WORD_PAIN, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_WORD_PAIN, duration = math.random(8000, 12000) })
        end

        if input:IsTimerReady(TIMER_GREAT_HEAL) then
            table.insert(actions, { action = "SCRIPT_TEXT", text_id = 10494 })  -- TEXT_GREAT_HEAL
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_GREAT_HEAL, target = "self" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_GREAT_HEAL, duration = math.random(20000, 25000) })
        end

        if input:IsTimerReady(TIMER_MIND_FLAY) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIND_FLAY, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_MIND_FLAY, duration = math.random(25000, 30000) })
        end

        if input:IsTimerReady(TIMER_CURSE_OF_BLOOD) then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CURSE_OF_BLOOD, target = "current_target" })
            table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CURSE_OF_BLOOD, duration = math.random(25000, 30000) })
        end
    end

    return actions
end

function boss:OnDeath(input)
    return { { action = "SCRIPT_TEXT", text_id = 10452 } }  -- SAY_DEATH
end

RegisterCreatureAI(14517, boss)
