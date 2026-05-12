--[[
    @script_type: creature_ai
    @entry: 15264
    @name: mob_anubisath_sentinel

    Anubisath Sentinel - Temple of Ahn'Qiraj
    Verified against vmangos mob_anubisath_sentinel.cpp

    Mechanics:
    - On aggro, gains one random ability from a pool of 9 (buff applied as aura)
    - Enrages at 30% HP
    - On death, transfers its ability to nearby alive sentinels and heals them to full
    - Pulls nearby sentinels into combat together (SET_COMBAT_WITH_ZONE)
    - Knock Away ability: SPELL_KNOCK_BUFF aura + SPELL_KNOCK cast every 13s
]]

-- Ability buff auras (one chosen randomly on aggro, permanent aura)
local SPELL_PERIODIC_MANA_BURN      = 812
local SPELL_MENDING                 = 2147
local SPELL_PERIODIC_SHADOW_STORM   = 2148
local SPELL_PERIODIC_THUNDERCLAP    = 2834
local SPELL_MORTAL_STRIKE           = 9347
local SPELL_FIRE_ARCANE_REFLECT     = 13022
local SPELL_SHADOW_FROST_REFLECT    = 19595
local SPELL_PERIODIC_KNOCK_AWAY     = 21737   -- buff aura that indicates this sentinel uses Knock Away
local SPELL_THORNS                  = 25777

-- Actual Knock Away spell (used only when ability = SPELL_PERIODIC_KNOCK_AWAY)
local SPELL_KNOCK                   = 23382

-- Death transfer spells (cast on nearby alive sentinel buddies)
local SPELL_TRANSFER                = 2400    -- transfers this sentinel's ability to buddy
local SPELL_HEAL_BRETHREN           = 26565   -- heals buddy to full

local SPELL_ENRAGE                  = 24318   -- was 8599 (wrong)

local TIMER_ENRAGE_CHECK            = 1
local TIMER_KNOCK                   = 2

local ABILITIES = {
    SPELL_MENDING,
    SPELL_MORTAL_STRIKE,
    SPELL_PERIODIC_SHADOW_STORM,
    SPELL_FIRE_ARCANE_REFLECT,
    SPELL_SHADOW_FROST_REFLECT,
    SPELL_THORNS,
    SPELL_PERIODIC_THUNDERCLAP,
    SPELL_PERIODIC_KNOCK_AWAY,
    SPELL_PERIODIC_MANA_BURN,
}

local sentinel = {}

function sentinel:OnEnterCombat(input)
    -- Pick a random ability and cast it on self as buff
    local ability = ABILITIES[math.random(1, #ABILITIES)]
    local actions = {
        { action = "CAST_SPELL", spell_id = ability, target = "self" },
        { action = "SET_CUSTOM_DATA", key = "my_ability", value = ability },
        { action = "SET_CUSTOM_DATA", key = "enraged", value = 0 },
        { action = "SET_COMBAT_WITH_ZONE" },
    }
    -- If Knock Away ability, set 13s timer for active knock spell
    if ability == SPELL_PERIODIC_KNOCK_AWAY then
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK, duration = 13000 })
    end
    return actions
end

function sentinel:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Knock Away active spell every 13s (only when this sentinel has that ability)
    local my_ability = input:GetCustomData("my_ability") or 0
    if my_ability == SPELL_PERIODIC_KNOCK_AWAY and input:IsTimerReady(TIMER_KNOCK) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_KNOCK, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_KNOCK, duration = 13000 })
    end

    -- Enrage at 30% HP
    local enraged = input:GetCustomData("enraged") or 0
    if enraged == 0 and input.health_pct and input.health_pct < 0.30 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENRAGE, target = "self" })
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = "enraged", value = 1 })
    end

    return actions
end

function sentinel:OnDeath(input)
    -- Transfer this sentinel's ability and heal brethren
    local actions = {}
    local sentinels = input:GetCreaturesByEntry(15264)
    if sentinels then
        for _, creature in ipairs(sentinels) do
            if creature.is_alive then
                table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_TRANSFER, target = "self" })
                table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_HEAL_BRETHREN, target = "self" })
                break
            end
        end
    end
    return actions
end

RegisterCreatureAI(15264, sentinel)
