--[[
    Winterspring zone scripts
    Reference: kalimdor/winterspring/winterspring.cpp

    npc_artorius (entries 14531 = Amiable, 14535 = Doombringer)
        Amiable form: waits 10s then plays roar emote + transforms into
        Doombringer entry via MORPH. BeginEvent triggered by OnScriptEvent
        (external event system, mapped to OnProcessEventId here).
        Doombringer: combat AI with Demonic Frenzy (23257), Demonic Doom (23298);
        despawns if threat list > 1 (non-solo hunter). Despawns after 20m if
        out of combat. SpellHit Serpent Sting → Stinging Trauma (23299).

    npc_umi_yeti (entry 10218)
        SpellHit by spell 17163 (Unsummon Yeti) → stop movement + despawn.
]]

-- ============================================================
-- npc_artorius_the_amiable  (entry 14531)
-- ============================================================
local NPC_ARTORIUS_AMIABLE      = 14531
local NPC_ARTORIUS_DOOMBRINGER  = 14535
local NPC_THE_CLEANER           = 14503

local SPELL_STINGING_TRAUMA     = 23299
local SPELL_DEMONIC_FRENZY      = 23257
local SPELL_DEMONIC_DOOM        = 23298
local SPELL_SERPENT_STING_8     = 13555
local SPELL_SERPENT_STING_9     = 25295

local EMOTE_ROAR = 11  -- EMOTE_ONESHOT_ROAR

local npc_artorius_amiable = {}

function npc_artorius_amiable:OnSpawn(self, npc_guid)
    self.transform        = false
    self.transform_timer  = 10000
    self.emote_timer      = 5000
    return {}
end

function npc_artorius_amiable:OnReset(self, npc_guid)
    self.transform        = false
    self.transform_timer  = 10000
    self.emote_timer      = 5000
    return {}
end

-- Triggered by DB event / script event system
function npc_artorius_amiable:OnProcessEventId(self, npc_guid, event_id, source_guid, is_start)
    self.transform       = true
    self.transform_timer = 10000
    self.emote_timer     = 5000
    return { { action = "STOP_MOVEMENT" } }
end

function npc_artorius_amiable:OnUpdate(self, npc_guid, diff_ms, input)
    if not self.transform then return {} end

    local actions = {}

    if self.emote_timer and self.emote_timer > 0 then
        self.emote_timer = self.emote_timer - diff_ms
        if self.emote_timer <= 0 then
            self.emote_timer = 0
            table.insert(actions, { action = "PLAY_EMOTE", emote_id = EMOTE_ROAR })
        end
    end

    self.transform_timer = self.transform_timer - diff_ms
    if self.transform_timer <= 0 then
        self.transform = false
        -- Transform to Doombringer
        table.insert(actions, { action = "MORPH", display_id = 0 }) -- server uses UpdateEntry; MORPH resets display
        -- Note: full UpdateEntry (swapping NPC entry) is not yet a Lua action.
        -- The transform visually approximates; full stat swap requires Phase E engine support.
    end

    return actions
end

RegisterCreatureAI(NPC_ARTORIUS_AMIABLE, npc_artorius_amiable)

-- ============================================================
-- npc_artorius_the_doombringer  (entry 14535)
-- ============================================================
local npc_artorius_doombringer = {}

local function rand_range(lo, hi)
    return lo + math.floor(math.random() * (hi - lo + 1))
end

function npc_artorius_doombringer:OnSpawn(self, npc_guid)
    self.demonic_frenzy_timer = rand_range(5000, 8000)
    self.demonic_doom_timer   = 7500
    self.despawn_timer        = 20 * 60 * 1000  -- 20 minutes
    return {}
end

function npc_artorius_doombringer:OnReset(self, npc_guid)
    self.demonic_frenzy_timer = rand_range(5000, 8000)
    self.demonic_doom_timer   = 7500
    self.despawn_timer        = 20 * 60 * 1000
    return {}
end

function npc_artorius_doombringer:OnSpellHit(self, npc_guid, spell_id, caster_guid)
    -- Serpent Sting → Stinging Trauma
    if spell_id == SPELL_SERPENT_STING_8 or spell_id == SPELL_SERPENT_STING_9 then
        return {
            { action = "CAST_SPELL", spell_id = SPELL_STINGING_TRAUMA, triggered = true },
            { action = "PLAY_EMOTE", emote_id = 9786 }, -- EMOTE_POISON
        }
    end
    return {}
end

function npc_artorius_doombringer:OnUpdate(self, npc_guid, diff_ms, input)
    local actions = {}

    -- Out-of-combat despawn timer
    if not input.is_in_combat then
        self.despawn_timer = (self.despawn_timer or 20*60*1000) - diff_ms
        if self.despawn_timer <= 0 then
            table.insert(actions, { action = "KILL_SELF" })
        end
        return actions
    end

    if not input.current_target then return {} end

    -- If more than one unit on threat list, despawn (solo hunter only)
    if input.threat_list and #input.threat_list > 1 then
        table.insert(actions, { action = "SPAWN_CREATURE",
            entry = NPC_THE_CLEANER,
            x = input.position.x, y = input.position.y, z = input.position.z,
            summon_type = "TIMED_DESPAWN",
            duration_ms = 20 * 60 * 1000 })
        table.insert(actions, { action = "KILL_SELF" })
        return actions
    end

    self.demonic_frenzy_timer = self.demonic_frenzy_timer - diff_ms
    if self.demonic_frenzy_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMONIC_FRENZY })
        self.demonic_frenzy_timer = rand_range(15000, 20000)
    end

    self.demonic_doom_timer = self.demonic_doom_timer - diff_ms
    if self.demonic_doom_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMONIC_DOOM,
            target_guid = input.current_target })
        self.demonic_doom_timer = 7500
    end

    return actions
end

RegisterCreatureAI(NPC_ARTORIUS_DOOMBRINGER, npc_artorius_doombringer)

-- ============================================================
-- npc_umi_yeti  (entry 10218)
-- ============================================================
local NPC_UMI_YETI          = 10218
local SPELL_UNSUMMON_YETI   = 17163

local npc_umi_yeti = {}

function npc_umi_yeti:OnMoveInLineOfSight(self, npc_guid, unit_guid, is_hostile)
    -- C++ override is empty; suppress aggro
    return {}
end

function npc_umi_yeti:OnSpellHit(self, npc_guid, spell_id, caster_guid)
    if spell_id == SPELL_UNSUMMON_YETI then
        return {
            { action = "STOP_MOVEMENT" },
            { action = "KILL_SELF" },
        }
    end
    return {}
end

function npc_umi_yeti:OnUpdate(self, npc_guid, diff_ms, input)
    return {}
end

RegisterCreatureAI(NPC_UMI_YETI, npc_umi_yeti)
