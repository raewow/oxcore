--[[
    Moonglade zone scripts
    Reference: kalimdor/moonglade/moonglade.cpp

    npc_keeper_remulos (entry varies)
        OnQuestAccept + OnEffectDummy (spell 25813 — Conjure Rift, summons Eranikus).
        The main AI drives a complex multi-phase event (Nightmare Manifests,
        Waking Legends) with summoning, healing, failed-quest checks, etc.
        All behaviour is deeply integrated into UpdateAI; deferred.

    boss_omen — from boss_omen.cpp, a world boss AI (deferred).
]]
