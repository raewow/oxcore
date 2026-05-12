--[[
    @script_type: creature_ai
    @entry: 11058
    @name: boss_nerubenkan

    Nerub'enkan - Stratholme (Undead)
    Ported from vmangos boss_nerubenkan.cpp

    Periodically summons Crypt Scarabs (entry 10876) or Undead Scarab groups
    (entry 10577, 4-8 at a time). Uses Encasing Webs to CC current target.
]]

-- Spells
local SPELL_ENCASING_WEBS      = 4962   -- Roots current target in webs
local SPELL_PIERCE_ARMOR       = 6016   -- Reduces target armor
local SPELL_RAISE_UNDEAD_SCARAB = 17235 -- Summon spell (not used directly - we use SPAWN_CREATURE)

-- Scarab NPC entries
local NPC_UNDEAD_SCARAB = 10577  -- regular scarabs (4-8 spawned)
local NPC_CRYPT_SCARAB  = 10876  -- crypt scarab (1 spawned)

-- Timer IDs
local TIMER_ENCASING_WEBS = 1
local TIMER_PIERCE_ARMOR  = 2
local TIMER_SUMMON_SCARAB = 3

local boss = {}

function boss:OnEnterCombat(input)
    return {
        { action = "SET_TIMER", timer_id = TIMER_ENCASING_WEBS, duration = 7000 },
        { action = "SET_TIMER", timer_id = TIMER_PIERCE_ARMOR,  duration = 15000 },
        { action = "SET_TIMER", timer_id = TIMER_SUMMON_SCARAB, duration = 3000 },
    }
end

function boss:OnUpdate(input)
    local actions = {}
    if not input.is_in_combat then return actions end

    -- Encasing Webs (current target, repeats every 10-15s)
    if input:IsTimerReady(TIMER_ENCASING_WEBS) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_ENCASING_WEBS, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_ENCASING_WEBS, duration = math.random(10000, 15000) })
    end

    -- Pierce Armor (current target, repeats every 15-20s)
    if input:IsTimerReady(TIMER_PIERCE_ARMOR) then
        table.insert(actions, { action = "CAST_SPELL", spell_id = SPELL_PIERCE_ARMOR, target = "current_target" })
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_PIERCE_ARMOR, duration = math.random(15000, 20000) })
    end

    -- Summon Scarabs (randomly spawn Crypt Scarab or 4-8 Undead Scarabs, repeats every 6-10s)
    if input:IsTimerReady(TIMER_SUMMON_SCARAB) then
        if math.random(0, 1) == 0 then
            -- Spawn a single Crypt Scarab
            table.insert(actions, {
                action = "SPAWN_CREATURE",
                entry = NPC_CRYPT_SCARAB,
                x = 0, y = 0, z = 0, o = 0,
                summon_type = "timed_despawn_out_of_combat",
                duration = 10000,
            })
        else
            -- Spawn 4-8 Undead Scarabs
            local count = math.random(4, 8)
            for i = 1, count do
                table.insert(actions, {
                    action = "SPAWN_CREATURE",
                    entry = NPC_UNDEAD_SCARAB,
                    x = 0, y = 0, z = 0, o = 0,
                    summon_type = "timed_despawn_out_of_combat",
                    duration = 10000,
                })
            end
        end
        table.insert(actions, { action = "SET_TIMER", timer_id = TIMER_SUMMON_SCARAB, duration = math.random(6000, 10000) })
    end

    return actions
end

function boss:OnDeath(input, killer_guid)
    return {}
end

RegisterCreatureAI(11058, boss)
