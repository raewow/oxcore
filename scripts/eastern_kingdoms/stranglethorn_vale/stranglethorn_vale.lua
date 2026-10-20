--[[
    Stranglethorn Vale zone scripts
    Reference: eastern_kingdoms/stranglethorn_vale/stranglethorn_vale.cpp

    mob_yenniku (entry 2519)
        SpellHit by spell 3607 (Yenniku's Release, quest 592):
        - Set emote state to STUN
        - Leave combat, clear threat list
        - Change faction to 83 (Horde generic)
        - After 60s timer: evade + revert faction to 28 (Bloodscalp troll)

    npc_witch_doctor_unbagwa (entry 1449)
        OnQuestRewarded quest 349: The C++ starts a gorilla/konda/mokk wave
        summon sequence driven by creature AI. UpdateAI wave logic is now
        supported; implement as timed summon waves.

    npc_pats_hellfire_guy (entry 5082)
        On spawn: cast spell 24207 (Hellfire Cast Visual) once after 2s delay.

    mob_assistant_kryll (entry 5083)
        Every 15-40 minutes: say one of 3 random Kryll recruitment lines.

    go_transpolyporter (entry 142052)
        OnUse: deny use if player doesn't have item 9173.
        Not implementable as pure Lua callback (requires item check in OnUse).
        Deferred — GO OnUse item-check requires Phase E GO script support.
]]

-- ============================================================
-- mob_yenniku  (entry 2519)
-- ============================================================
local MOB_YENNIKU           = 2519
local SPELL_YENNIKURS_RELEASE = 3607
local QUEST_HEADHUNTERS_HARVEST = 592
local EMOTE_STATE_STUN      = 64   -- EMOTE_STATE_STUN
local EMOTE_STATE_NONE      = 0

local TIMER_RESET       = 1
local DATA_RELEASED     = "released"

local mob_yenniku = {}

function mob_yenniku:OnSpellHit(input, spell_id, caster_guid)
    if spell_id ~= SPELL_YENNIKURS_RELEASE or input:GetCustomData(DATA_RELEASED) == 1 then
        return {}
    end
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_RELEASED, value = 1 },
        { action = "SET_TIMER", timer_id = TIMER_RESET, duration = 60000 },
        { action = "EMOTE",        emote_id = EMOTE_STATE_STUN },
        { action = "LEAVE_COMBAT" },
        { action = "CLEAR_THREAT_LIST" },
        { action = "SET_FACTION",  faction_id = 83 },
    }
end

function mob_yenniku:OnUpdate(input)
    if input:GetCustomData(DATA_RELEASED) == 1 and input:IsTimerReady(TIMER_RESET) then
        return {
            { action = "SET_CUSTOM_DATA", key = DATA_RELEASED, value = 0 },
            { action = "EMOTE",        emote_id = EMOTE_STATE_NONE },
            { action = "SET_FACTION",  faction_id = 28 },  -- Bloodscalp
            { action = "ENTER_EVADE_MODE" },
        }
    end
    return {}
end

RegisterCreatureAI(MOB_YENNIKU, mob_yenniku)

-- ============================================================
-- npc_pats_hellfire_guy  (entry 5082)
-- ============================================================
local NPC_PATS_HELLFIRE_GUY     = 5082
local SPELL_HELLFIRE_CAST_VISUAL = 24207

local TIMER_CAST     = 1
local DATA_CAST_DONE = "cast_done"

local npc_pats_hellfire_guy = {}

local function hellfire_start()
    return {
        { action = "SET_TIMER", timer_id = TIMER_CAST, duration = 2000 },
        { action = "SET_CUSTOM_DATA", key = DATA_CAST_DONE, value = 0 },
    }
end

function npc_pats_hellfire_guy:OnSpawn(input)
    return hellfire_start()
end

function npc_pats_hellfire_guy:OnReset(input)
    return hellfire_start()
end

function npc_pats_hellfire_guy:OnUpdate(input)
    if input:GetCustomData(DATA_CAST_DONE) == 0 and input:IsTimerReady(TIMER_CAST) then
        return {
            { action = "SET_CUSTOM_DATA", key = DATA_CAST_DONE, value = 1 },
            { action = "CAST_SPELL", spell_id = SPELL_HELLFIRE_CAST_VISUAL, target = "self" },
        }
    end
    return {}
end

RegisterCreatureAI(NPC_PATS_HELLFIRE_GUY, npc_pats_hellfire_guy)

-- ============================================================
-- mob_assistant_kryll  (entry 5083)
-- ============================================================
local MOB_ASSISTANT_KRYLL = 5083

local KRYLL_LINES = {
    "Psst... go to Booty Bay, Kryll needs hands...",
    "Kryll needs your help in Booty Bay!",
    "Kryll's invention may drastically change your life... Help him in Booty Bay!",
}

local function rand_range(lo, hi)
    return lo + math.floor(math.random() * (hi - lo + 1))
end

local TIMER_SPEECH = 1

local mob_assistant_kryll = {}

function mob_assistant_kryll:OnSpawn(input)
    return { { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 360000 } }
end

function mob_assistant_kryll:OnReset(input)
    return { { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = 360000 } }
end

function mob_assistant_kryll:OnUpdate(input)
    if not input:IsTimerReady(TIMER_SPEECH) then
        return {}
    end
    -- These lines have no broadcast text and there is no free-text say, so they are yelled.
    local line = KRYLL_LINES[rand_range(1, #KRYLL_LINES)]
    return {
        { action = "YELL", text = line },
        { action = "SET_TIMER", timer_id = TIMER_SPEECH, duration = rand_range(15, 40) * 60 * 1000 },
    }
end

RegisterCreatureAI(MOB_ASSISTANT_KRYLL, mob_assistant_kryll)

-- go_transpolyporter (entry 142052): OnUse item-check deferred (requires GO script OnUse support)
