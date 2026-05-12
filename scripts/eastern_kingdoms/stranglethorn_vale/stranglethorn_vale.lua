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

local mob_yenniku = {}

function mob_yenniku:OnSpellHit(self, npc_guid, spell_id, caster_guid)
    if spell_id == SPELL_YENNIKURS_RELEASE and not self.reset_pending then
        self.reset_pending = true
        self.reset_timer   = 60000
        return {
            { action = "PLAY_EMOTE",   emote_id = EMOTE_STATE_STUN },
            { action = "LEAVE_COMBAT" },
            { action = "CLEAR_THREAT_LIST" },
            { action = "SET_FACTION",  faction_id = 83 },
        }
    end
    return {}
end

function mob_yenniku:OnUpdate(self, npc_guid, diff_ms, input)
    if self.reset_pending then
        self.reset_timer = self.reset_timer - diff_ms
        if self.reset_timer <= 0 then
            self.reset_pending = false
            return {
                { action = "PLAY_EMOTE",   emote_id = EMOTE_STATE_NONE },
                { action = "SET_FACTION",  faction_id = 28 },  -- Bloodscalp
                { action = "ENTER_EVADE_MODE" },
            }
        end
    end

    if input.is_in_combat and input.current_target then
        return {}  -- DoMeleeAttackIfReady handled by default AI
    end

    return {}
end

RegisterCreatureAI(MOB_YENNIKU, mob_yenniku)

-- ============================================================
-- npc_pats_hellfire_guy  (entry 5082)
-- ============================================================
local NPC_PATS_HELLFIRE_GUY     = 5082
local SPELL_HELLFIRE_CAST_VISUAL = 24207

local npc_pats_hellfire_guy = {}

function npc_pats_hellfire_guy:OnSpawn(self, npc_guid)
    self.cast_delay = 2000
    self.cast_done  = false
    return {}
end

function npc_pats_hellfire_guy:OnReset(self, npc_guid)
    self.cast_delay = 2000
    self.cast_done  = false
    return {}
end

function npc_pats_hellfire_guy:OnUpdate(self, npc_guid, diff_ms, input)
    if not self.cast_done then
        self.cast_delay = (self.cast_delay or 2000) - diff_ms
        if self.cast_delay <= 0 then
            self.cast_done = true
            return { { action = "CAST_SPELL", spell_id = SPELL_HELLFIRE_CAST_VISUAL } }
        end
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

local mob_assistant_kryll = {}

function mob_assistant_kryll:OnSpawn(self, npc_guid)
    self.speech_timer = 360000
    return {}
end

function mob_assistant_kryll:OnReset(self, npc_guid)
    self.speech_timer = 360000
    return {}
end

function mob_assistant_kryll:OnUpdate(self, npc_guid, diff_ms, input)
    self.speech_timer = (self.speech_timer or 360000) - diff_ms
    if self.speech_timer <= 0 then
        local line = KRYLL_LINES[rand_range(1, #KRYLL_LINES)]
        self.speech_timer = rand_range(15, 40) * 60 * 1000
        return { { action = "SAY", text = line } }
    end
    return {}
end

RegisterCreatureAI(MOB_ASSISTANT_KRYLL, mob_assistant_kryll)

-- go_transpolyporter (entry 142052): OnUse item-check deferred (requires GO script OnUse support)
