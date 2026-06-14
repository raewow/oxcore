# WorldSession::HandleAuctionHelloOpcode

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Handles the Auction Hello client opcode: resolves the auctioneer NPC via the packet GUID and auctioneer NPC flag, returns early with a debug log if interaction is invalid, clears feign-death auras when the player is in feign death, then delegates to SendAuctionHello with the resolved creature.

## Source
```cpp
void WorldSession::HandleAuctionHelloOpcode(WorldPackets::AuctionHouse::AuctionHello const& packet)
{
    Creature* unit = GetPlayer()->GetNPCIfCanInteractWith(packet.auctioneerGuid, UNIT_NPC_FLAG_AUCTIONEER);
    if (!unit)
    {
        sLog.Out(LOG_BASIC, LOG_LVL_DEBUG, "WORLD: HandleAuctionHelloOpcode - %s not found or you can't interact with him.", packet.auctioneerGuid.GetString().c_str());
        return;
    }

    // remove fake death
    if (GetPlayer()->HasUnitState(UNIT_STATE_FEIGN_DEATH))
        GetPlayer()->RemoveSpellsCausingAura(SPELL_AURA_FEIGN_DEATH);

    SendAuctionHello(unit);
}
```
