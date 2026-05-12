--[[
    @script_type: creature_ai
    @entry: 14889,14888,14890,14887
    @name: bosses_emerald_dragons

    Emerald Dragons - World Bosses (Duskwood, Hinterlands, Feralas, Ashenvale)
    Verified against vmangos dragons_of_nightmare/*.cpp

    Four green dragon world bosses sharing a base kit (Noxious Breath, Tail Sweep,
    Seeping Fog, Summon Player) with unique abilities triggered at 75%, 50%, 25% HP.

    Entries:
      Emeriss  = 14889
      Lethon   = 14888
      Taerar   = 14890
      Ysondre  = 14887
]]

------------------------------------------------------------------------
-- SHARED SPELL CONSTANTS
------------------------------------------------------------------------
local SPELL_NOXIOUS_BREATH       = 24818
local SPELL_TAILSWEEP            = 15847
local SPELL_SEEPING_FOG_R        = 24813
local SPELL_SEEPING_FOG_L        = 24814
local SPELL_MARK_OF_NATURE_AURA  = 25041
local SPELL_SUMMON_PLAYER        = 24776

------------------------------------------------------------------------
-- EMERISS SPELLS
------------------------------------------------------------------------
local SPELL_VOLATILE_INFECTION   = 24928
local SPELL_CORRUPTION_OF_EARTH  = 24910
local SPELL_PUTRID_MUSHROOM      = 24904

------------------------------------------------------------------------
-- LETHON SPELLS
------------------------------------------------------------------------
local SPELL_SHADOW_BOLT_WHIRL    = 24834
local SPELL_DRAW_SPIRIT          = 24811

------------------------------------------------------------------------
-- TAERAR SPELLS
------------------------------------------------------------------------
local SPELL_ARCANE_BLAST         = 24857
local SPELL_BELLOWING_ROAR       = 22686

-- Shade summon spells
local SPELL_SUMMON_SHADE_1       = 24841
local SPELL_SUMMON_SHADE_2       = 24842
local SPELL_SUMMON_SHADE_3       = 24843

local NPC_SHADE_OF_TAERAR        = 15302

------------------------------------------------------------------------
-- YSONDRE SPELLS
------------------------------------------------------------------------
local SPELL_LIGHTNING_WAVE       = 24819
local SPELL_SUMMON_DRUIDS        = 24795

------------------------------------------------------------------------
-- NPC ENTRIES
------------------------------------------------------------------------
local NPC_EMERISS                = 14889
local NPC_LETHON                 = 14888
local NPC_TAERAR                 = 14890
local NPC_YSONDRE                = 14887
local NPC_DREAM_FOG              = 15224

------------------------------------------------------------------------
-- SHARED TIMER IDS (base dragon kit)
------------------------------------------------------------------------
local TIMER_NOXIOUS_BREATH       = 1
local TIMER_TAILSWEEP            = 2
local TIMER_SEEPING_FOG          = 3
local TIMER_SUMMON_PLAYER        = 4

------------------------------------------------------------------------
-- PER-DRAGON TIMER IDS (start at 10 to avoid collisions)
------------------------------------------------------------------------
-- Emeriss
local TIMER_VOLATILE_INFECTION   = 10

-- Lethon
local TIMER_SHADOW_BOLT_WHIRL    = 10
local TIMER_DRAW_SPIRIT_CHECK    = 11

-- Taerar
local TIMER_ARCANE_BLAST         = 10
local TIMER_BELLOWING_ROAR       = 11

-- Ysondre
local TIMER_LIGHTNING_WAVE       = 10

------------------------------------------------------------------------
-- HELPER: shared initial timer setup
------------------------------------------------------------------------
local function base_combat_actions()
    return {
        { action = "CAST_SPELL", spell_id = SPELL_MARK_OF_NATURE_AURA, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_BREATH, duration = math.random(7000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_TAILSWEEP,     duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_SEEPING_FOG,   duration = 20000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_PLAYER, duration = 30000 },
    }
end

------------------------------------------------------------------------
-- HELPER: shared update logic (returns actions + whether we should
--         continue to dragon-specific logic)
------------------------------------------------------------------------
local function base_update_actions(input)
    local actions = {}

    -- Noxious Breath (urand(7,10)s repeat)
    if input:IsTimerReady(TIMER_NOXIOUS_BREATH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_NOXIOUS_BREATH, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_NOXIOUS_BREATH, duration = math.random(7000, 10000) })
    end

    -- Tail Sweep (urand(6,8)s repeat)
    if input:IsTimerReady(TIMER_TAILSWEEP) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TAILSWEEP, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAILSWEEP, duration = math.random(6000, 8000) })
    end

    -- Seeping Fog (summon Dream Fog NPCs on both sides)
    if input:IsTimerReady(TIMER_SEEPING_FOG) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SEEPING_FOG_R, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SEEPING_FOG_L, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SEEPING_FOG, duration = 135000 })
    end

    -- Summon Player (teleport random player to dragon)
    if input:IsTimerReady(TIMER_SUMMON_PLAYER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_PLAYER, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_PLAYER, duration = 30000 })
    end

    return actions
end

------------------------------------------------------------------------
-- Phase tracks how many 25% thresholds have fired (0..3)
-- Threshold fires when health_pct < (1.0 - phase * 0.25)
------------------------------------------------------------------------
local function check_threshold(input, actions)
    local threshold = 1.0 - input.phase * 0.25
    if input.health_pct < threshold then
        return true
    end
    return false
end

--======================================================================
-- EMERISS (14889)
--======================================================================
local emeriss = {}

function emeriss:OnEnterCombat(input)
    local actions = base_combat_actions()
    table.insert(actions, { action = "SET_PHASE", phase = 1 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VOLATILE_INFECTION, duration = math.random(11000, 13000) })
    table.insert(actions, { action = "YELL", text = "Hope is a DISEASE of the soul! This land shall wither and die!" })
    return actions
end

function emeriss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
    }
end

function emeriss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Base dragon abilities
    local base = base_update_actions(input)
    for _, a in ipairs(base) do table.insert(actions, a) end

    -- Special ability at 75%, 50%, 25%: Corruption of Earth
    if check_threshold(input, actions) then
        table.insert(actions, { action = "YELL", text = "The land itself rises against you!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CORRUPTION_OF_EARTH, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = input.phase + 1 })
    end

    -- Volatile Infection (random target DoT)
    if input:IsTimerReady(TIMER_VOLATILE_INFECTION) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VOLATILE_INFECTION, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VOLATILE_INFECTION, duration = math.random(10000, 16000) })
    end

    return actions
end

RegisterCreatureAI(NPC_EMERISS, emeriss)

--======================================================================
-- LETHON (14888)
--======================================================================
local lethon = {}

function lethon:OnEnterCombat(input)
    local actions = base_combat_actions()
    table.insert(actions, { action = "SET_PHASE", phase = 1 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_WHIRL, duration = 5000 })
    table.insert(actions, { action = "YELL", text = "I can sense the SHADOW on your hearts. There can be no rest for the wicked!" })
    return actions
end

function lethon:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
    }
end

function lethon:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Base dragon abilities
    local base = base_update_actions(input)
    for _, a in ipairs(base) do table.insert(actions, a) end

    -- Special ability at 75%, 50%, 25%: Draw Spirit
    if check_threshold(input, actions) then
        table.insert(actions, { action = "YELL", text = "Your spirit belongs to me now!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DRAW_SPIRIT, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = input.phase + 1 })
    end

    -- Shadow Bolt Whirl (periodic AoE shadow bolts)
    if input:IsTimerReady(TIMER_SHADOW_BOLT_WHIRL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT_WHIRL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT_WHIRL, duration = 15000 })
    end

    return actions
end

RegisterCreatureAI(NPC_LETHON, lethon)

--======================================================================
-- TAERAR (14890)
--======================================================================
local taerar = {}

function taerar:OnEnterCombat(input)
    local actions = base_combat_actions()
    table.insert(actions, { action = "SET_PHASE", phase = 1 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_BLAST,   duration = math.random(11000, 13000) })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(27000, 30000) })
    table.insert(actions, { action = "YELL", text = "Peace is but a fleeting dream! Let the NIGHTMARE reign!" })
    return actions
end

function taerar:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
    }
end

function taerar:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Base dragon abilities
    local base = base_update_actions(input)
    for _, a in ipairs(base) do table.insert(actions, a) end

    -- Special ability at 75%, 50%, 25%: Summon Shades of Taerar
    if check_threshold(input, actions) then
        table.insert(actions, { action = "YELL", text = "Children of Madness - come forth!" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SHADE_1, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SHADE_2, target = "self" })
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SHADE_3, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = input.phase + 1 })
    end

    -- Arcane Blast (random target)
    if input:IsTimerReady(TIMER_ARCANE_BLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_BLAST, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_BLAST, duration = math.random(10000, 16000) })
    end

    -- Bellowing Roar (urand(25,28)s repeat)
    if input:IsTimerReady(TIMER_BELLOWING_ROAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BELLOWING_ROAR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(25000, 28000) })
    end

    return actions
end

RegisterCreatureAI(NPC_TAERAR, taerar)

--======================================================================
-- YSONDRE (14887)
--======================================================================
local ysondre = {}

function ysondre:OnEnterCombat(input)
    local actions = base_combat_actions()
    table.insert(actions, { action = "SET_PHASE", phase = 1 })
    table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LIGHTNING_WAVE, duration = math.random(10000, 13000) })
    table.insert(actions, { action = "YELL", text = "The strands of LIFE have been severed! The Dreamers must be avenged!" })
    return actions
end

function ysondre:OnReset(input)
    return {
        { action = "SET_PHASE", phase = 1 },
    }
end

function ysondre:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Base dragon abilities
    local base = base_update_actions(input)
    for _, a in ipairs(base) do table.insert(actions, a) end

    -- Special ability at 75%, 50%, 25%: Summon Demented Druid Spirits
    if check_threshold(input, actions) then
        table.insert(actions, { action = "YELL", text = "Come forth, ye Dreamers! Defend this sacred place!" })
        -- Summon 10 druid spirits via spell
        for i = 1, 10 do
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_DRUIDS, target = "self" })
        end
        table.insert(actions, { action = "SET_PHASE", phase = input.phase + 1 })
    end

    -- Lightning Wave (chain lightning on random target)
    if input:IsTimerReady(TIMER_LIGHTNING_WAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_LIGHTNING_WAVE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_LIGHTNING_WAVE, duration = math.random(8000, 12000) })
    end

    return actions
end

RegisterCreatureAI(NPC_YSONDRE, ysondre)
