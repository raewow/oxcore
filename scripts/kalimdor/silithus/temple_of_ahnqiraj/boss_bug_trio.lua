--[[
    @script_type: creature_ai
    @entry: 15511, 15544, 15543
    @name: boss_bug_trio

    The Bug Trio (Kri, Vem, Yauj) - Temple of Ahn'Qiraj
    Verified against vmangos boss_bug_trio.cpp

    Kri (15511): Cleave, Thrash, Toxic Volley; spawns Poison Cloud on death
    Vem (15544): Charge, Knockback, Knockdown; casts Vengeance (enrages survivors) on death
    Yauj (15543): Fear (resets threat), Heal (self or lowest ally), Ravage; spawns 10 brood on death

    Shared: Devour mechanic (bugs heal to full on devouring a dead bug) is server-side
]]

-- Kri spells
local SPELL_THRASH          = 3391
local SPELL_CLEAVE          = 19983   -- was 26350 (wrong)
local SPELL_TOXIC_VOLLEY    = 25812
local SPELL_SUMMON_CLOUD    = 25786   -- death spawn (poison cloud)

-- Vem spells
local SPELL_CHARGE          = 26561
local SPELL_KNOCKBACK       = 18813   -- was 26027 (wrong)
local SPELL_KNOCKDOWN       = 19128   -- was missing
local SPELL_VENGEANCE       = 25790   -- enrages surviving bugs on Vem death

-- Yauj spells
local SPELL_RAVAGE          = 24213   -- was missing
local SPELL_HEAL            = 25807
local SPELL_FEAR            = 19408

-- Yauj death spawn
local NPC_YAUJ_BROOD        = 15621

-- Timer IDs for Kri
local TIMER_KRI_CLEAVE          = 1
local TIMER_KRI_TOXIC_VOLLEY    = 2
local TIMER_KRI_THRASH          = 3

-- Timer IDs for Vem
local TIMER_VEM_CHARGE          = 1
local TIMER_VEM_KNOCKBACK       = 2
local TIMER_VEM_KNOCKDOWN       = 3

-- Timer IDs for Yauj
local TIMER_YAUJ_HEAL           = 1
local TIMER_YAUJ_FEAR           = 2
local TIMER_YAUJ_RAVAGE         = 3

--------------------------------------------------------------------------------
-- Kri (15511)
--------------------------------------------------------------------------------
local kri = {}

function kri:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_KRI_CLEAVE,       duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_KRI_TOXIC_VOLLEY, duration = math.random(8000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_KRI_THRASH,       duration = math.random(4000, 7000) },
    }
end

function kri:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_KRI_CLEAVE,       duration = math.random(4000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_KRI_TOXIC_VOLLEY, duration = math.random(8000, 10000) },
        { action = "SET_TIMER", timer_id = TIMER_KRI_THRASH,       duration = math.random(4000, 7000) },
    }
end

function kri:OnDeath(input)
    return {
        { action = "CAST_SPELL", spell_id = SPELL_SUMMON_CLOUD, target = "self" },
    }
end

function kri:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Cleave
    if input:IsTimerReady(TIMER_KRI_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KRI_CLEAVE, duration = math.random(5000, 12000) })
    end

    -- Toxic Volley
    if input:IsTimerReady(TIMER_KRI_TOXIC_VOLLEY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TOXIC_VOLLEY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KRI_TOXIC_VOLLEY, duration = math.random(8000, 14000) })
    end

    -- Thrash (random proc approximated as low-freq timer)
    if input:IsTimerReady(TIMER_KRI_THRASH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THRASH, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KRI_THRASH, duration = math.random(2000, 8000) })
    end

    return actions
end

RegisterCreatureAI(15511, kri)

--------------------------------------------------------------------------------
-- Vem (15544)
--------------------------------------------------------------------------------
local vem = {}

function vem:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_VEM_CHARGE,    duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKBACK, duration = math.random(15000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKDOWN, duration = math.random(5000, 8000) },
    }
end

function vem:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_VEM_CHARGE,    duration = math.random(10000, 15000) },
        { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKBACK, duration = math.random(15000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKDOWN, duration = math.random(5000, 8000) },
    }
end

function vem:OnDeath(input)
    -- Enrage surviving bugs with Vengeance
    return {
        { action = "CAST_SPELL", spell_id = SPELL_VENGEANCE, target = "self" },
    }
end

function vem:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Charge random target not in melee range
    if input:IsTimerReady(TIMER_VEM_CHARGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CHARGE, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VEM_CHARGE, duration = math.random(15000, 20000) })
    end

    -- Knock Away current target (-80% threat on hit)
    if input:IsTimerReady(TIMER_VEM_KNOCKBACK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCKBACK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKBACK, duration = math.random(10000, 14000) })
    end

    -- Knockdown random melee target
    if input:IsTimerReady(TIMER_VEM_KNOCKDOWN) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCKDOWN, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_VEM_KNOCKDOWN, duration = math.random(15000, 20000) })
    end

    return actions
end

RegisterCreatureAI(15544, vem)

--------------------------------------------------------------------------------
-- Yauj (15543)
--------------------------------------------------------------------------------
local yauj = {}

function yauj:OnEnterCombat(input)
    return {
        { action = "SET_COMBAT_WITH_ZONE" },
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_HEAL,   duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_FEAR,   duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_RAVAGE, duration = math.random(4000, 9000) },
    }
end

function yauj:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_HEAL,   duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_FEAR,   duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_YAUJ_RAVAGE, duration = math.random(4000, 9000) },
    }
end

function yauj:OnDeath(input)
    -- Spawn 10 Yauj Brood near boss
    local actions = {}
    for _ = 1, 10 do
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_YAUJ_BROOD })
    end
    return actions
end

function yauj:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Fear (AoE) + reset threat; repeat 20s
    if input:IsTimerReady(TIMER_YAUJ_FEAR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FEAR, target = "self" })
        table.insert(actions, { action = "RESET_THREAT" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YAUJ_FEAR, duration = 20000 })
    end

    -- Heal self or lowest health ally; repeat 12s on success
    if input:IsTimerReady(TIMER_YAUJ_HEAL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEAL, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YAUJ_HEAL, duration = 12000 })
    end

    -- Ravage current target
    if input:IsTimerReady(TIMER_YAUJ_RAVAGE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAVAGE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_YAUJ_RAVAGE, duration = math.random(12000, 20000) })
    end

    return actions
end

RegisterCreatureAI(15543, yauj)
