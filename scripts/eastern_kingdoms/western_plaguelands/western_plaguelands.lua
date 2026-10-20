--[[
    Western Plaguelands zone scripts
    Reference: eastern_kingdoms/western_plaguelands/western_plaguelands.cpp

    npc_the_scourge_cauldron (entries 11075-11078)
        EnableMoveInLosEvent in Reset. MoveInLineOfSight fires for players.
        When a player with the correct quest (per area_id) approaches,
        summon the cauldron lord NPC and self-destruct.

        Area IDs → quest IDs → cauldron lord entries:
          199 (Felstone)  → quests 5216/5229 → entry 11075
          200 (Dalson)    → quests 5219/5231 → entry 11077
          201 (Gahrron)   → quests 5225/5235 → entry 11078
          202 (Writhing)  → quests 5222/5233 → entry 11076

    npc_andorhal_tower (entries 11489-11492)
        EnableMoveInLosEvent. MoveInLineOfSight: when a player approaches
        and a beacon torch GO (176093) is within 20yd, award kill credit.

    npc_highprotectorlorik (entry 1846)
        Combat AI: always has Retribution Aura (8990) active; casts
        Arcane Blast (10833) every 10-12s, Divine Shield (13874) at <15% HP
        every 45s, Holy Light (15493) at <60% HP every 2-6s, Shield Slam
        (15655) on a casting target every 9s.
]]

-- npc_the_scourge_cauldron (entry 11152) and npc_andorhal_tower (entries 10902-10905):
-- deferred. The cauldron needs the approaching player's quest log and the tower needs a
-- gameobject proximity check, and neither is available to Lua creature AI yet.

-- ============================================================
-- npc_highprotectorlorik  (entry 1846)
-- ============================================================
local NPC_HIGHPROTECTORLORIK = 1846

local SPELL_RETRIBUTION_AURA = 8990
local SPELL_ARCANE_BLAST     = 10833
local SPELL_DIVINE_SHIELD    = 13874
local SPELL_HOLY_LIGHT       = 15493
local SPELL_SHIELD_SLAM      = 15655

local TIMER_ARCANE_BLAST  = 1
local TIMER_DIVINE_SHIELD = 2
local TIMER_HOLY_LIGHT    = 3
local TIMER_SHIELD_SLAM   = 4

local npc_highprotectorlorik = {}

local function lorik_start()
    return {
        { action = "SET_TIMER", timer_id = TIMER_ARCANE_BLAST,  duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_DIVINE_SHIELD, duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_HOLY_LIGHT,    duration = 2000 },
        { action = "SET_TIMER", timer_id = TIMER_SHIELD_SLAM,   duration = 2000 },
    }
end

function npc_highprotectorlorik:OnSpawn(input)
    return lorik_start()
end

function npc_highprotectorlorik:OnReset(input)
    return lorik_start()
end

function npc_highprotectorlorik:OnUpdate(input)
    local actions = {}

    -- Always maintain Retribution Aura
    if not input:HasAura(SPELL_RETRIBUTION_AURA) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RETRIBUTION_AURA,
            target = "self", triggered = true })
    end

    if not input.is_in_combat or not input.current_target then
        return actions
    end

    -- Divine Shield at <15% HP; once ready it waits for the threshold
    if input:IsTimerReady(TIMER_DIVINE_SHIELD) and input.health_pct <= 15.0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DIVINE_SHIELD, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DIVINE_SHIELD, duration = 45000 })
    end

    if input:IsTimerReady(TIMER_ARCANE_BLAST) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ARCANE_BLAST, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ARCANE_BLAST,
            duration = 10000 + math.floor(math.random() * 2000) })
    end

    -- Holy Light (self-heal at <60% HP); once ready it waits for the threshold
    if input:IsTimerReady(TIMER_HOLY_LIGHT) and input.health_pct <= 60.0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HOLY_LIGHT, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HOLY_LIGHT,
            duration = 2000 + math.floor(math.random() * 4000) })
    end

    if input:IsTimerReady(TIMER_SHIELD_SLAM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHIELD_SLAM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SHIELD_SLAM, duration = 9000 })
    end

    return actions
end

RegisterCreatureAI(NPC_HIGHPROTECTORLORIK, npc_highprotectorlorik)
