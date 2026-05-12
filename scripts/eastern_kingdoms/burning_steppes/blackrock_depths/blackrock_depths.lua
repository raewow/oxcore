--[[
    Blackrock Depths dungeon helper scripts.
    Ported from vmangos blackrock_depths.cpp

    Contents:
    - mob_phalanx (entry 9502): Thunder Clap every 10s; Fireball Volley every 15s when
      HP < 51%; Mighty Blow every 10s.

    Skipped (requires GO scripting API):
    - go_shadowforge_brazier: GOHello toggles TYPE_LYCEUM instance data (Phase E GO API).

    Skipped (requires escort AI + GO gate control + area trigger):
    - npc_grimstone (entry 10096): Ring of Law arena master.
      Extremely complex: npc_escortAI with 15 event phases, 6 types of ring mob summons
      (entries 8925-8933), 6 ring bosses (entries 9027-9032), Theldren challenge group
      (entries 16049-16058), 4 arena GO gates (DATA_ARENA1-4), GO state toggling,
      area trigger entry/zone detection, wipe detection across all players within 80yd.
      Requires: GO open/close API, area trigger callbacks, escort AI system — all Phase D/E.

    TODO(api): When area triggers + GO scripting are available, port npc_grimstone:
    - RingMob entries: 8925 (Dredge Worm), 8926 (Deep Stinger), 8927 (Dark Screecher),
      8928 (Burrowing Thundersnout), 8933 (Cave Creeper), 8932 (Borer Beetle)
    - RingBoss entries: 9027 (Gorosh), 9028 (Grizzle), 9029 (Eviscerator),
      9030 (Ok'thor), 9031 (Anub'shiah), 9032 (Hedrum)
    - Challenge group: Theldren + random healer (16053/16055) + 3 DPS (16049-16054/16058)
    - Arena gates: DATA_ARENA1-4 (GO state toggles via OPEN_DOOR/CLOSE_DOOR)
    - Teleport spell on phase transitions: SPELL_GRIMSTONE_TELEPORT = 6422
    - Yell text IDs: SAY_GRIMSTONE_ARENA1-6 = 5441-5446
]]

-- Spell IDs
local SPELL_THUNDERCLAP     = 15588
local SPELL_FIREBALLVOLLEY  = 15285
local SPELL_MIGHTYBLOW      = 14099

-- Timer IDs
local TIMER_PX_THUNDER  = 1
local TIMER_PX_FIREBALL = 2
local TIMER_PX_MIGHTY   = 3

------------------------------------------------------------------------
-- mob_phalanx (entry 9502)
-- Thunder Clap (15588) every 10s.
-- Fireball Volley (15285) every 15s, only when HP < 51%.
-- Mighty Blow (14099) every 10s.
-- NOTE: vmangos activates Phalanx via the Ring of Law event (npc_grimstone calling
-- Activate()) which sets faction hostile and moves him to the arena entrance.
-- This simplified script handles combat spells only; activation is managed externally
-- by the instance script when TYPE_RING_OF_LAW progresses.
------------------------------------------------------------------------
local phalanx = {}

function phalanx:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_PX_THUNDER,  duration = 12000 },
        { action = "SET_TIMER", timer_id = TIMER_PX_FIREBALL, duration = 0 },
        { action = "SET_TIMER", timer_id = TIMER_PX_MIGHTY,   duration = 15000 },
    }
end

function phalanx:OnUpdate(input)
    if not input.is_in_combat then return {} end
    local actions = {}

    if input:IsTimerReady(TIMER_PX_THUNDER) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_THUNDERCLAP, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PX_THUNDER, duration = 10000 })
    end

    -- Fireball Volley only below 51% HP
    if input.health_pct < 51.0 and input:IsTimerReady(TIMER_PX_FIREBALL) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_FIREBALLVOLLEY, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PX_FIREBALL, duration = 15000 })
    end

    if input:IsTimerReady(TIMER_PX_MIGHTY) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_MIGHTYBLOW, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PX_MIGHTY, duration = 10000 })
    end

    return actions
end

RegisterCreatureAI(9502, phalanx)
