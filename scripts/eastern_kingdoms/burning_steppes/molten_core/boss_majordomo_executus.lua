--[[
    @script_type: creature_ai
    @entry: 12018
    @name: boss_majordomo_executus

    Majordomo Executus - Molten Core
    Ported from vmangos boss_majordomo_executus.cpp

    Note: Full add management (spawning/despawning 8 adds), defeat dialogue,
    Ragnaros summoning event, and gossip handling require instance script support.
    This covers the combat AI portion with correct spells and timers.
]]

-- Spells (exact from vmangos)
local SPELL_AEGIS_OF_RAGNAROS = 20620    -- Heal, cast at start and 50% HP
local SPELL_MAGIC_REFLECTION = 20619     -- Random choice with Damage Shield every 30s
local SPELL_DAMAGE_SHIELD = 21075        -- Random choice with Magic Reflection every 30s
local SPELL_TELEPORT_TARGET = 20534      -- Teleport current target
local SPELL_TELEPORT_RANDOM = 20618      -- Teleport random player
local SPELL_ENCOURAGEMENT = 21086        -- Cast on all adds when one dies
local SPELL_IMMUNITY = 21087             -- Cast when <= 4 adds alive
local SPELL_CHAMPION = 21090             -- Cast on last remaining add
local SPELL_SEPARATION_ANXIETY = 21094   -- Cast on all adds at combat start

-- NPC entries
local NPC_FLAMEWAKER_ELITE = 11664
local NPC_FLAMEWAKER_HEALER = 11663

-- Yell text IDs (from vmangos)
local SAY_AGGRO = 7612
local SAY_SLAY = 9425
local SAY_LAST_ADD = 8545

-- Timer IDs
local TIMER_REFLECTION = 1       -- Alternates Magic Reflection / Damage Shield
local TIMER_TELEPORT = 2
local TIMER_AEGIS = 3

local boss = {}
local aegis_50_triggered = false

function boss:OnEnterCombat(input)
    aegis_50_triggered = false
    return {
        { action = "YELL", text_id = SAY_AGGRO },
        { action = "CAST_SPELL", spell_id = SPELL_AEGIS_OF_RAGNAROS, target = "self" },
        { action = "SET_TIMER", timer_id = TIMER_REFLECTION, duration = 30000 },
        { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = 20000 },  -- 10s + rand(0-20s) ~= 20s
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Aegis of Ragnaros at 50% HP
    if not aegis_50_triggered and input.health_pct < 0.50 then
        aegis_50_triggered = true
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_AEGIS_OF_RAGNAROS, target = "self" })
    end

    -- Magic Reflection or Damage Shield (alternating, every 30s)
    if input:IsTimerReady(TIMER_REFLECTION) then
        -- Random choice between the two shields
        if math.random(2) == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MAGIC_REFLECTION, target = "self" })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DAMAGE_SHIELD, target = "self" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_REFLECTION, duration = 30000 })
    end

    -- Teleport (random choice between target and random, repeats ~25s)
    if input:IsTimerReady(TIMER_TELEPORT) then
        if math.random(2) == 1 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TELEPORT_RANDOM, target = "random_hostile" })
        else
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TELEPORT_TARGET, target = "current_target" })
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_TELEPORT, duration = 25000 })  -- 20s + rand(0-10s)
    end

    return actions
end

function boss:OnKill(input)
    return {
        { action = "YELL", text_id = SAY_SLAY },
    }
end

RegisterCreatureAI(12018, boss)
