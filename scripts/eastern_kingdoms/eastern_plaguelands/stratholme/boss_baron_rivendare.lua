--[[
    @script_type: creature_ai
    @entry: 10440
    @name: boss_baron_rivendare

    Baron Rivendare - Stratholme (Undead side)
    No vmangos C++ reference available; scripted from known 1.12 behavior.

    The final boss of Stratholme Undead. Rides atop his skeletal horse and
    commands undead forces. The famous Deathcharger mount drops from him.

    Mechanics:
    - Shadow Bolt: periodic ranged nuke
    - Unholy Aura: passive AoE aura buff to nearby undead (cast on combat start)
    - Raise Dead: summons skeletal adds periodically
    - Cleave: periodic frontal melee
    - Frenzy: enrage at 30% HP
]]

local SPELL_SHADOW_BOLT     = 17432
local SPELL_UNHOLY_AURA     = 17467
local SPELL_RAISE_DEAD      = 17462
local SPELL_CLEAVE          = 15496

local NPC_SKELETAL_SOLDIER  = 11709

local TIMER_SHADOW_BOLT     = 1
local TIMER_RAISE_DEAD      = 2
local TIMER_CLEAVE          = 3

local PHASE_NORMAL          = 0
local PHASE_ENRAGED         = 1

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
        { action = "YELL", text = "Your pathetic efforts against the Scourge have failed! Now face the consequences!" },
        { action = "CAST_SPELL", spell_id = SPELL_UNHOLY_AURA, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(4000, 8000)   },
        { action = "SET_TIMER", timer_id = TIMER_RAISE_DEAD,  duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_CLEAVE,      duration = math.random(6000, 10000)  },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_PHASE", phase = PHASE_NORMAL },
    }
end

function boss:OnKill(input)
    return {
        { action = "YELL", text = "There will be no more mercy!" },
    }
end

function boss:OnDeath(input)
    return {
        { action = "YELL", text = "Kel'Thuzad... I have... failed..." },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Frenzy at 30% HP (one-time)
    if input.phase == PHASE_NORMAL and input.health_pct < 0.30 then
        table.insert(actions, { action = "EMOTE", text = "%s goes into a frenzy!" })
        table.insert(actions, { action = "SET_PHASE", phase = PHASE_ENRAGED })
    end

    -- Shadow Bolt
    if input:IsTimerReady(TIMER_SHADOW_BOLT) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHADOW_BOLT, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHADOW_BOLT, duration = math.random(6000, 12000) })
    end

    -- Raise Dead (summon skeletal soldiers)
    if input:IsTimerReady(TIMER_RAISE_DEAD) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAISE_DEAD, target = "self" })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SKELETAL_SOLDIER, distance = 10.0 })
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_SKELETAL_SOLDIER, distance = 10.0 })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_RAISE_DEAD, duration = math.random(25000, 35000) })
    end

    -- Cleave
    if input:IsTimerReady(TIMER_CLEAVE) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_CLEAVE, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_CLEAVE, duration = math.random(8000, 12000) })
    end

    return actions
end

RegisterCreatureAI(10440, boss)
