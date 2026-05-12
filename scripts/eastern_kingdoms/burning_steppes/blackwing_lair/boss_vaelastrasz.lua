--[[
    @script_type: creature_ai
    @entry: 13020
    @name: boss_vaelastrasz

    Vaelastrasz the Corrupt - Blackwing Lair
    Verified against vmangos boss_vaelastrasz.cpp

    Notes:
    - Gossip / intro speech sequence (BeginSpeech, SAY_LINE_1/2/3) is handled
      by the zone script / OnGossipSelect; not replicated here.
    - Burning Adrenaline targeting (mana users vs tank) requires player power-type
      queries; using random_hostile as approximation until full API support.
    - Boss starts at 30% HP (set by the engine per instance data).
]]

local SPELL_ESSENCE_OF_THE_RED = 23513  -- infinite mana/rage/energy aura on raid; cast on aggro
local SPELL_FLAME_BREATH       = 23461  -- on current target; repeat every 5-10s
local SPELL_FIRE_NOVA          = 23462  -- AoE self; repeat every 2s
local SPELL_TAIL_SWEEP         = 15847  -- AoE self (behind); repeat every 4-6s
local SPELL_BURNING_ADRENALINE = 23620  -- self-cast by victim; caster every 15s, tank every 45s
local SPELL_CLEAVE             = 19983  -- was 20684 (wrong); on current target; repeat every 5-10s

local TIMER_CLEAVE                    = 1
local TIMER_FLAME_BREATH              = 2
local TIMER_FIRE_NOVA                 = 3
local TIMER_BURNING_ADRENALINE_CASTER = 4
local TIMER_BURNING_ADRENALINE_TANK   = 5
local TIMER_TAIL_SWEEP                = 6

local PHASE_NORMAL = 1
local PHASE_LOW_HP = 2  -- below 15% hp yell

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_ESSENCE_OF_THE_RED, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,                    duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH,              duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FIRE_NOVA,                 duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_CASTER, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_TANK,   duration = 45000 },
        { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP,                duration = 8000 },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_HEALTH_PERCENT", percent = 30 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,                    duration = 6000 },
        { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH,              duration = 8000 },
        { action = "SET_TIMER", timer_id = TIMER_FIRE_NOVA,                 duration = 4000 },
        { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_CASTER, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_TANK,   duration = 45000 },
        { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP,                duration = 8000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Yell at 15% HP
    if input.phase == PHASE_NORMAL and input.health_pct < 0.15 then
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = 9965 })  -- SAY_HALFLIFE
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_LOW_HP })
    end

    -- Cleave on current target; repeat every 5-10s
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(5000, 10000) })
    end

    -- Flame Breath on current target; repeat every 5-10s
    if input:IsTimerReady(TIMER_FLAME_BREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FLAME_BREATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FLAME_BREATH, duration = math.random(5000, 10000) })
    end

    -- Fire Nova (AoE self); repeat every 2s
    if input:IsTimerReady(TIMER_FIRE_NOVA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIRE_NOVA, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_FIRE_NOVA, duration = 2000 })
    end

    -- Burning Adrenaline on random mana user (approximated as random hostile); every 15s
    if input:IsTimerReady(TIMER_BURNING_ADRENALINE_CASTER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BURNING_ADRENALINE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_CASTER, duration = 15000 })
    end

    -- Burning Adrenaline on current tank; every 45s
    if input:IsTimerReady(TIMER_BURNING_ADRENALINE_TANK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BURNING_ADRENALINE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BURNING_ADRENALINE_TANK, duration = 45000 })
    end

    -- Tail Sweep (AoE self, knocks back behind); repeat every 4-6s
    if input:IsTimerReady(TIMER_TAIL_SWEEP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TAIL_SWEEP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAIL_SWEEP, duration = math.random(4000, 6000) })
    end

    return actions
end

RegisterCreatureAI(13020, boss)
