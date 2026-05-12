--[[
    Undercity zone scripts
    Reference: eastern_kingdoms/undercity/undercity.cpp

    npc_lady_sylvanas_windrunner (entry 10181)
        Combat AI: Summon Skeletons (20464) every 25s, Fade (20672) every 50s
        (then kite at 30yd for 5s), Black Arrow (20733) random target every 15s,
        Multi Shot (20735) random target every 10s, Shoot (20463) main target
        every 10s. On enter combat: plays sound 5886.
]]

local NPC_SYLVANAS      = 10181

local SPELL_SUMMON_SKEL = 20464
local SPELL_FADE        = 20672
local SPELL_BLACK_ARROW = 20733
local SPELL_MULTI_SHOT  = 20735
local SPELL_SHOOT       = 20463

local npc_sylvanas = {}

function npc_sylvanas:OnSpawn(self, npc_guid)
    self.summon_skel_timer = 25000
    self.fade_timer        = 50000
    self.faded_timer       = 0
    self.black_arrow_timer = 15000
    self.multi_shot_timer  = 10000
    self.shoot_timer       = 10000
    return {}
end

function npc_sylvanas:OnReset(self, npc_guid)
    self.summon_skel_timer = 25000
    self.fade_timer        = 50000
    self.faded_timer       = 0
    self.black_arrow_timer = 15000
    self.multi_shot_timer  = 10000
    self.shoot_timer       = 10000
    return {}
end

function npc_sylvanas:OnEnterCombat(self, npc_guid, target_guid)
    return { { action = "PLAY_SOUND", sound_id = 5886, zone_wide = false } }
end

function npc_sylvanas:OnUpdate(self, npc_guid, diff_ms, input)
    local actions = {}

    -- Faded kite phase (stay at range 30yd for 5s after Fade)
    if self.faded_timer and self.faded_timer > 0 then
        self.faded_timer = self.faded_timer - diff_ms
        if self.faded_timer <= 0 then
            self.faded_timer = 0
        end
        return {}  -- skip other actions while kiting
    end

    if not input.is_in_combat or not input.current_target then
        return {}
    end

    self.summon_skel_timer = self.summon_skel_timer - diff_ms
    if self.summon_skel_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SUMMON_SKEL })
        self.summon_skel_timer = 25000
    end

    self.fade_timer = self.fade_timer - diff_ms
    if self.fade_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FADE })
        self.fade_timer        = 50000
        self.faded_timer       = 5000
        self.black_arrow_timer = 0
        self.multi_shot_timer  = 0
        self.shoot_timer       = 0
    end

    self.black_arrow_timer = self.black_arrow_timer - diff_ms
    if self.black_arrow_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_BLACK_ARROW,
            target_guid = input.current_target })
        self.black_arrow_timer = 15000
    end

    self.multi_shot_timer = self.multi_shot_timer - diff_ms
    if self.multi_shot_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MULTI_SHOT,
            target_guid = input.current_target })
        self.multi_shot_timer = 10000
    end

    self.shoot_timer = self.shoot_timer - diff_ms
    if self.shoot_timer <= 0 then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_SHOOT,
            target_guid = input.current_target })
        self.shoot_timer = 10000
    end

    return actions
end

RegisterCreatureAI(NPC_SYLVANAS, npc_sylvanas)
