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
local EMOTE_POISON = 9786  -- broadcast text
local DISPLAY_DOOMBRINGER = 2727

local TIMER_TRANSFORM       = 1
local TIMER_EMOTE           = 2
local TIMER_DEMONIC_FRENZY  = 1
local TIMER_DEMONIC_DOOM    = 2
local TIMER_DESPAWN         = 3
local DATA_TRANSFORMING     = "transforming"
local DATA_ROARED           = "roared"

local npc_artorius_amiable = {}

local function amiable_start()
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_TRANSFORMING, value = 0 },
        { action = "SET_CUSTOM_DATA", key = DATA_ROARED, value = 0 },
    }
end

function npc_artorius_amiable:OnSpawn(input)
    return amiable_start()
end

function npc_artorius_amiable:OnReset(input)
    return amiable_start()
end

-- Triggered by DB event / script event system
function npc_artorius_amiable:OnProcessEventId(input, event_id, source_guid, is_start)
    return {
        { action = "SET_CUSTOM_DATA", key = DATA_TRANSFORMING, value = 1 },
        { action = "SET_CUSTOM_DATA", key = DATA_ROARED, value = 0 },
        { action = "SET_TIMER", timer_id = TIMER_TRANSFORM, duration = 10000 },
        { action = "SET_TIMER", timer_id = TIMER_EMOTE,     duration = 5000 },
        { action = "STOP_MOVEMENT" },
    }
end

function npc_artorius_amiable:OnUpdate(input)
    if input:GetCustomData(DATA_TRANSFORMING) == 0 then return {} end

    local actions = {}

    if input:GetCustomData(DATA_ROARED) == 0 and input:IsTimerReady(TIMER_EMOTE) then
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_ROARED, value = 1 })
        table.insert(actions, { action = "EMOTE", emote_id = EMOTE_ROAR })
    end

    if input:IsTimerReady(TIMER_TRANSFORM) then
        table.insert(actions, { action = "SET_CUSTOM_DATA", key = DATA_TRANSFORMING, value = 0 })
        -- Full UpdateEntry (swapping NPC entry) is not yet a Lua action: take on the
        -- Doombringer's look without its stats.
        table.insert(actions, { action = "MORPH", display_id = DISPLAY_DOOMBRINGER })
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

local function doombringer_start()
    return {
        { action = "SET_TIMER", timer_id = TIMER_DEMONIC_FRENZY, duration = rand_range(5000, 8000) },
        { action = "SET_TIMER", timer_id = TIMER_DEMONIC_DOOM,   duration = 7500 },
        { action = "SET_TIMER", timer_id = TIMER_DESPAWN,        duration = 20 * 60 * 1000 },
    }
end

function npc_artorius_doombringer:OnSpawn(input)
    return doombringer_start()
end

function npc_artorius_doombringer:OnReset(input)
    return doombringer_start()
end

function npc_artorius_doombringer:OnSpellHit(input, spell_id, caster_guid)
    -- Serpent Sting → Stinging Trauma
    if spell_id == SPELL_SERPENT_STING_8 or spell_id == SPELL_SERPENT_STING_9 then
        return {
            { action = "CAST_SPELL", spell_id = SPELL_STINGING_TRAUMA, target = "self", triggered = true },
            { action = "SCRIPT_TEXT", text_id = EMOTE_POISON },
        }
    end
    return {}
end

function npc_artorius_doombringer:OnUpdate(input)
    -- Despawn after 20 minutes without a fight
    if not input.is_in_combat then
        if input:IsTimerReady(TIMER_DESPAWN) then
            return { { action = "KILL_SELF" } }
        end
        return {}
    end

    if not input.current_target then return {} end

    -- If more than one unit on threat list, despawn (solo hunter only)
    if input.threat_list and #input.threat_list > 1 then
        return {
            { action = "SPAWN_CREATURE",
              entry = NPC_THE_CLEANER,
              x = input.position.x, y = input.position.y, z = input.position.z,
              summon_type = "TIMED_DESPAWN",
              duration = 20 * 60 * 1000 },
            { action = "KILL_SELF" },
        }
    end

    local actions = {
        -- Fighting holds off the despawn: the 20 minutes start again once it ends.
        { action = "SET_TIMER", timer_id = TIMER_DESPAWN, duration = 20 * 60 * 1000 },
    }

    if input:IsTimerReady(TIMER_DEMONIC_FRENZY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMONIC_FRENZY, target = "self" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEMONIC_FRENZY,
            duration = rand_range(15000, 20000) })
    end

    if input:IsTimerReady(TIMER_DEMONIC_DOOM) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DEMONIC_DOOM, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_DEMONIC_DOOM, duration = 7500 })
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

function npc_umi_yeti:MoveInLineOfSight(input, unit_guid, is_hostile)
    -- C++ override is empty; suppress aggro
    return {}
end

function npc_umi_yeti:OnSpellHit(input, spell_id, caster_guid)
    if spell_id == SPELL_UNSUMMON_YETI then
        return {
            { action = "STOP_MOVEMENT" },
            { action = "KILL_SELF" },
        }
    end
    return {}
end

RegisterCreatureAI(NPC_UMI_YETI, npc_umi_yeti)
