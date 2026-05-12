--[[
    @script_type: creature_ai
    @entry: 11380
    @name: boss_jindo

    Jin'do the Hexxer - Zul'Gurub
    Verified against vmangos boss_jindo.cpp

    Notes:
    - Brain Wash Totem (entry 15112) is summoned and roots to cast mind control
    - Powerful Healing Ward (entry 14987) is summoned as a healer totem
    - Hex reduces victim threat 100% and switches target
    - Delusions of Jindo casts on random player (DoT + damage)
    - Shade of Jindo (entry 14986) is summoned near the deluded player
    - Banish teleports a random player to a random location in the room
]]

local SPELL_BRAIN_WASH_TOTEM     = 24262
local SPELL_POWERFULL_HEALING_WARD = 24309
local SPELL_HEX                  = 17172  -- was 24053 (wrong); reduces victim threat 100%
local SPELL_DELUSIONS_OF_JINDO   = 24306
local SPELL_SHADE_OF_JINDO       = 24308
local SPELL_BANISH               = 24466  -- teleports random player

local NPC_SHADE              = 14986
local NPC_BRAINWASH_TOTEM    = 15112
local NPC_HEALING_WARD       = 14987

local TIMER_BRAINWASH_TOTEM  = 1
local TIMER_HEALING_WARD     = 2
local TIMER_HEX              = 3
local TIMER_DELUSIONS        = 4
local TIMER_SUMMON_SHADE     = 5
local TIMER_BANISH           = 6

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SCRIPT_TEXT", text_id = 10449 },  -- SAY_AGGRO
        { action = "SET_TIMER", timer_id = TIMER_BRAINWASH_TOTEM, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD,    duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_HEX,             duration = math.random(20000, 50000) },
        { action = "SET_TIMER", timer_id = TIMER_DELUSIONS,       duration = math.random(3000, 6000) },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_SHADE,    duration = math.random(6000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_BANISH,          duration = math.random(15000, 30000) },
    }
end

function boss:OnReset(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_BRAINWASH_TOTEM, duration = math.random(10000, 20000) },
        { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD,    duration = math.random(20000, 30000) },
        { action = "SET_TIMER", timer_id = TIMER_HEX,             duration = math.random(20000, 50000) },
        { action = "SET_TIMER", timer_id = TIMER_DELUSIONS,       duration = math.random(3000, 6000) },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_SHADE,    duration = math.random(6000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_BANISH,          duration = math.random(15000, 30000) },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Brain Wash Totem: summon a rooted totem that mind-controls players; repeat 10-30s
    if input:IsTimerReady(TIMER_BRAINWASH_TOTEM) then
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_BRAINWASH_TOTEM })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BRAINWASH_TOTEM, duration = math.random(10000, 30000) })
    end

    -- Powerful Healing Ward: summon a healing ward totem; repeat 20-30s
    if input:IsTimerReady(TIMER_HEALING_WARD) then
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_SELF", entry = NPC_HEALING_WARD })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEALING_WARD, duration = math.random(20000, 30000) })
    end

    -- Hex: on current victim; reduces their threat 100%; repeat 20-60s
    if input:IsTimerReady(TIMER_HEX) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEX, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_HEX, duration = math.random(20000, 60000) })
    end

    -- Delusions: on random player; DoT + damage; repeat 3-9s
    if input:IsTimerReady(TIMER_DELUSIONS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DELUSIONS_OF_JINDO, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DELUSIONS, duration = math.random(3000, 9000) })
    end

    -- Summon Shade of Jindo near random player; repeat 7-8s
    if input:IsTimerReady(TIMER_SUMMON_SHADE) then
        table.insert(actions, { action = "SPAWN_CREATURE_NEAR_RANDOM", entry = NPC_SHADE })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_SHADE, duration = math.random(7000, 8000) })
    end

    -- Banish: teleport random player; repeat 15-35s
    if input:IsTimerReady(TIMER_BANISH) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BANISH, target = "random_hostile" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_BANISH, duration = math.random(15000, 35000) })
    end

    return actions
end

RegisterCreatureAI(11380, boss)
