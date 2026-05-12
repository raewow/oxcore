--[[
    @script_type: creature_ai
    @entry: 11583
    @name: boss_nefarian

    Nefarian (Dragon Form) - Blackwing Lair
    Verified against vmangos boss_nefarian.cpp

    Notes:
    - The flying intro/landing transition sequence uses movement APIs not
      available in the Lua action system; combat starts after landing.
    - Class call per-class effects (Warrior beserk aura, Mage polymorph loop,
      Warlock infernal summon, Rogue teleport, etc.) require per-player targeting
      not expressible as a single action; cast spell on self as best approximation.
    - Phase 3 skeleton raise (SPELL_RAISE_DRAKONID 23362) marks below 20% HP.
    - SPELL_BONE_CONSTRUCT spawning (23363) requires SpawnCreature API.
]]

local SPELL_SHADOWFLAME_INITIAL = 22992  -- initial AoE on aggro
local SPELL_SHADOWFLAME         = 22539  -- periodic AoE; repeat every 18-25s
local SPELL_BELLOWING_ROAR      = 22686  -- AoE fear self; repeat every 25-30s
local SPELL_VEIL_OF_SHADOW      = 22687  -- was 7068 (wrong); reduces healing on target; repeat every 10-15s
local SPELL_CLEAVE              = 20691
local SPELL_TAIL_LASH           = 23364  -- AoE knockback behind; repeat every 4-8s
local SPELL_RAISE_DRAKONID      = 23362  -- summons drakonid skeletons at 20% HP

-- Class call spells (affects all players of that class)
local SPELL_MAGE    = 23410  -- wild magic
local SPELL_WARRIOR = 23397  -- berserk
local SPELL_DRUID   = 23398  -- cat form
local SPELL_PRIEST  = 23401  -- corrupted healing
local SPELL_PALADIN = 23418  -- syphon blessing
local SPELL_SHAMAN  = 23425  -- corrupted totems
local SPELL_WARLOCK = 23427  -- infernals
local SPELL_HUNTER  = 23436  -- bow broke
local SPELL_ROGUE   = 23414  -- paralise

local TIMER_SHADOWFLAME    = 1
local TIMER_BELLOWING_ROAR = 2
local TIMER_VEIL_OF_SHADOW = 3
local TIMER_CLEAVE         = 4
local TIMER_TAIL_LASH      = 5
local TIMER_CLASS_CALL     = 6

local PHASE_NORMAL = 1
local PHASE_THREE  = 2  -- below 20%: raise skeletons

local CLASS_CALL_SPELLS = {
    SPELL_WARRIOR, SPELL_PALADIN, SPELL_HUNTER, SPELL_ROGUE,
    SPELL_PRIEST, SPELL_SHAMAN, SPELL_MAGE, SPELL_WARLOCK, SPELL_DRUID
}
-- SAY IDs from vmangos: 9855 Warrior, 9853 Paladin, 9849 Hunter, 9856 Rogue,
--                       9848 Priest, 9854 Shaman, 9850 Mage, 9852 Warlock, 9851 Druid
local CLASS_CALL_SAY_IDS = { 9855, 9853, 9849, 9856, 9848, 9854, 9850, 9852, 9851 }

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 9973 },  -- SAY_AGGRO
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "CAST_SPELL", spell_id = SPELL_SHADOWFLAME_INITIAL, target = "self" },
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWFLAME,    duration = math.random(18000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(25000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_VEIL_OF_SHADOW, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,         duration = math.random(7000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_TAIL_LASH,      duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_CLASS_CALL,     duration = math.random(25000, 35000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "SET_TIMER", timer_id = TIMER_SHADOWFLAME,    duration = math.random(18000, 25000) },
        { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(25000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_VEIL_OF_SHADOW, duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,         duration = math.random(7000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_TAIL_LASH,      duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_CLASS_CALL,     duration = math.random(25000, 35000) },
    }
end

function boss:OnKilledUnit(input)
    if math.random(0, 4) == 0 then
        return {
            { action = "SCRIPT_TEXT", text_id = 9972 },  -- SAY_SLAY
        }
    end
    return {}
end

function boss:OnDeath(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 9971 },  -- SAY_DEATH
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Shadow Flame (AoE self); repeat every 18-25s
    if input:IsTimerReady(TIMER_SHADOWFLAME) then
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = 9974 })  -- SAY_SHADOWFLAME
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOWFLAME, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOWFLAME, duration = math.random(18000, 25000) })
    end

    -- Bellowing Roar (AoE fear self); repeat every 25-30s
    if input:IsTimerReady(TIMER_BELLOWING_ROAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BELLOWING_ROAR, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BELLOWING_ROAR, duration = math.random(25000, 30000) })
    end

    -- Veil of Shadow on current target; repeat every 10-15s
    if input:IsTimerReady(TIMER_VEIL_OF_SHADOW) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_VEIL_OF_SHADOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VEIL_OF_SHADOW, duration = math.random(10000, 15000) })
    end

    -- Cleave on current target; repeat every 7-10s
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(7000, 10000) })
    end

    -- Tail Lash (AoE self); repeat every 4-8s
    if input:IsTimerReady(TIMER_TAIL_LASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TAIL_LASH, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TAIL_LASH, duration = math.random(4000, 8000) })
    end

    -- Class Call: pick random available class, yell, cast debuff; repeat every 25-35s
    if input:IsTimerReady(TIMER_CLASS_CALL) then
        local idx = math.random(1, #CLASS_CALL_SPELLS)
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = CLASS_CALL_SAY_IDS[idx] })
        table.insert(actions, { action = "CAST_SPELL", spell_id = CLASS_CALL_SPELLS[idx], target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLASS_CALL, duration = math.random(25000, 35000) })
    end

    -- Phase 3: below 20% HP — raise drakonid skeletons (once)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.20 then
        table.insert(actions, { action = "SCRIPT_TEXT", text_id = 9883 })  -- SAY_RAISE_SKELETONS
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAISE_DRAKONID, target = "self" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_THREE })
    end

    return actions
end

RegisterCreatureAI(11583, boss)
