# WorldSession::SendAuctionHello

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
WorldSession::SendAuctionHello resolves the auction house entry for a given Unit, constructs an AuctionHelloResponse packet populated with the unit's object GUID and the resolved houseId, and passes ownership of that packet to SendPacket for transmission.

## Source
```cpp
void WorldSession::SendAuctionHello(Unit* unit)
{
    // always return pointer
    AuctionHouseEntry const* ahEntry = AuctionHouseMgr::GetAuctionHouseEntry(unit);

    auto helloResponse = std::make_unique<WorldPackets::AuctionHouse::AuctionHelloResponse>();
    helloResponse->auctioneerGuid = unit->GetObjectGuid();
    helloResponse->houseId = ahEntry->houseId;
    SendPacket(std::move(helloResponse));
}
```
