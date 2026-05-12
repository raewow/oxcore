--[[
    Azshara zone scripts
    Reference: kalimdor/azshara/azshara.cpp

    mob_maws (entry 15341)
        Out-of-combat: patrols a looping path around the Bay of Storms via
        MovementInform (MovePoint chain). Handled by DB waypoints.
        In-combat: casts Rampage (25744), Frenzy (19812), and at <20% HP also
        Dark Water (25743). On death broadcasts a server-wide emote.

    go_bay_of_storms (entry 180670)
        GameObjectAI that plays a cycling custom anim every 3-8s.
        GO AI is not yet supported in Lua; deferred.
]]

local MOB_MAWS                  = 15341

local SPELL_DARK_WATER          = 25743
local SPELL_FRENZY              = 19812
local SPELL_RAMPAGE             = 25744

local RAMPAGE_TIMER_INIT_MIN    = 20000
local RAMPAGE_TIMER_INIT_MAX    = 120000
local RAMPAGE_TIMER_MAX_P2      = 12000
local FRENZY_TIMER              = 25000
local FRENZY_TIMER_P2           = 15000
local DARK_WATER_TIMER          = 15000

local mob_maws = {}

local function rand_between(lo, hi)
    return lo + math.floor(math.random() * (hi - lo + 1))
end

function mob_maws:OnSpawn(self, npc_guid)
    self.rampage_timer   = rand_between(RAMPAGE_TIMER_INIT_MIN, RAMPAGE_TIMER_INIT_MAX)
    self.rampage_max     = RAMPAGE_TIMER_INIT_MAX
    self.frenzy_timer    = FRENZY_TIMER
    self.frenzy_max      = FRENZY_TIMER
    self.dark_water_timer = DARK_WATER_TIMER
    self.phase_two       = false
    return {}
end

function mob_maws:OnReset(self, npc_guid)
    self.rampage_timer   = rand_between(RAMPAGE_TIMER_INIT_MIN, RAMPAGE_TIMER_INIT_MAX)
    self.rampage_max     = RAMPAGE_TIMER_INIT_MAX
    self.frenzy_timer    = FRENZY_TIMER
    self.frenzy_max      = FRENZY_TIMER
    self.dark_water_timer = DARK_WATER_TIMER
    self.phase_two       = false
    return {}
end

function mob_maws:OnUpdate(self, npc_guid, diff_ms, input)
    if not input.is_in_combat or not input.current_target then
        return {}
    end

    local actions = {}

    -- Phase 2 transition at <20% HP
    if not self.phase_two and input.health_pct < 20.0 then
        self.phase_two   = true
        self.rampage_max = RAMPAGE_TIMER_MAX_P2
        self.frenzy_max  = FRENZY_TIMER_P2
        if self.rampage_timer > RAMPAGE_TIMER_MAX_P2 then
            self.rampage_timer = RAMPAGE_TIMER_MAX_P2
        end
        if self.frenzy_timer > FRENZY_TIMER_P2 then
            self.frenzy_timer = FRENZY_TIMER_P2
        end
    end

    -- Rampage
    self.rampage_timer = self.rampage_timer - diff_ms
    if self.rampage_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_RAMPAGE,
            target_guid = input.current_target })
        if not self.phase_two then
            self.rampage_max   = rand_between(20000, 120000)
        end
        self.rampage_timer = self.rampage_max
    end

    -- Frenzy (self-cast)
    self.frenzy_timer = self.frenzy_timer - diff_ms
    if self.frenzy_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FRENZY })
        self.frenzy_timer = self.frenzy_max
    end

    -- Dark Water (phase 2 only, self-cast)
    if self.phase_two then
        self.dark_water_timer = self.dark_water_timer - diff_ms
        if self.dark_water_timer <= 0 then
            table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_DARK_WATER })
            self.dark_water_timer = DARK_WATER_TIMER
        end
    end

    return actions
end

function mob_maws:OnDeath(self, npc_guid, killer_guid)
    -- C++: sWorld.SendBroadcastTextToWorld(EMOTE_THE_BEAST_RETURNS)
    -- Broadcast text 11160; no Lua zone-wide say action yet, so yell as approximation
    return { { action = "YELL", text = "The beast has been slain!" } }
end

RegisterCreatureAI(MOB_MAWS, mob_maws)
