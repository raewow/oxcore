--[[
    Durotar zone scripts
    Reference: kalimdor/durotar/durotar.cpp

    LazyPeons (entry 10556)
        OnEffectDummy: spell SPELL_AWAKEN_PEON (19938) hits a sleeping peon (entry 10556).
        If the peon has the sleep aura (17743), award quest kill credit to the caster.
        Returns true = spell effect handled.
]]

local SPELL_AWAKEN_PEON  = 19938
local SPELL_BUFF_SLEEP   = 17743
local LAZY_PEON_ENTRY    = 10556

local lazy_peon = {}

function lazy_peon:OnEffectDummy(caster_guid, spell_id, eff_index, target_entry, target_guid)
    if spell_id == SPELL_AWAKEN_PEON and target_entry == LAZY_PEON_ENTRY then
        -- Award quest kill credit for waking the peon.
        -- Searches for the nearest lazy peon within 5 yards of the player and credits it.
        return {
            { action = "KILL_CREDIT_NEAREST_CREATURE",
              creature_entry = LAZY_PEON_ENTRY,
              search_radius  = 5.0 },
        }
    end
    return false
end

RegisterEffectDummyScript(LAZY_PEON_ENTRY, lazy_peon)
