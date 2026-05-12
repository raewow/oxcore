--[[
    Blasted Lands zone scripts
    Reference: eastern_kingdoms/blasted_lands/blasted_lands.cpp

    go_stone_of_binding (GO entries 141812 / 141857 / 141858 / 141859)
        OnGameObjectHello: find nearest servant NPC (7668-7671) within 30 yd
        and cast spell 12938 (Binding ritual) on it.

        141812 -> Servant of Razelikh  (7668)
        141857 -> Servant of Grol      (7669)
        141858 -> Servant of Allistarj (7670)
        141859 -> Servant of Sevine    (7671)
]]

local SPELL_BINDING   = 12938
local SEARCH_RADIUS   = 30.0

local STONE_SERVANT_MAP = {
    [141812] = 7668,  -- Servant of Razelikh
    [141857] = 7669,  -- Servant of Grol
    [141858] = 7670,  -- Servant of Allistarj
    [141859] = 7671,  -- Servant of Sevine
}

local function make_stone_script(go_entry)
    local servant_entry = STONE_SERVANT_MAP[go_entry]
    local script = {}

    function script:OnGameObjectHello(player, go_guid)
        -- C++: find nearest servant NPC, cast SPELL_BINDING on it.
        return {
            { action = "CAST_SPELL_ON_NEAREST_CREATURE",
              creature_entry = servant_entry,
              spell_id       = SPELL_BINDING,
              search_radius  = SEARCH_RADIUS },
        }
    end

    return script
end

for go_entry, _ in pairs(STONE_SERVANT_MAP) do
    RegisterGameObjectScript(go_entry, make_stone_script(go_entry))
end
