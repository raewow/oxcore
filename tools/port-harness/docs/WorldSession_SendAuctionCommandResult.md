# WorldSession::SendAuctionCommandResult

**File:** src/game/Handlers/AuctionHouseHandler.cpp

## Summary
Builds and sends SMSG_AUCTION_COMMAND_RESULT to the client: always writes auction id (0 if auc is null), action, and error code; appends extra uint32/ObjectGuid fields only for specific ErrorCode/Action combinations; then calls SendPacket.

## Source
```cpp
void WorldSession::SendAuctionCommandResult(AuctionEntry* auc, AuctionAction Action, AuctionError ErrorCode, InventoryResult invError)
{
    WorldPacket data(SMSG_AUCTION_COMMAND_RESULT, 16);
    data << uint32(auc ? auc->Id : 0);
    data << uint32(Action);
    data << uint32(ErrorCode);

    switch (ErrorCode)
    {
        case AUCTION_OK:
            if (Action == AUCTION_BID_PLACED)
                data << uint32(auc->GetAuctionOutBid());    // new AuctionOutBid?
            break;
        case AUCTION_ERR_INVENTORY:
            data << uint32(invError);
            break;
        case AUCTION_ERR_HIGHER_BID:
            data << ObjectGuid(HIGHGUID_PLAYER, auc->bidder); // new bidder guid
            data << uint32(auc->bid);                       // new bid
            data << uint32(auc->GetAuctionOutBid());        // new AuctionOutBid?
            break;
        default:
            break;
    }

    SendPacket(&data);
}
```
