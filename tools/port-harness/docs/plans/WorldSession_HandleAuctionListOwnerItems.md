# Plan: WorldSession::HandleAuctionListOwnerItems

## Rust Symbol
`handle_auction_list_owner_items` in `src/world/handlers/auction.rs`

## Target File
`src/world/handlers/auction.rs`

## Dispatcher
Add `Opcode::CMSG_AUCTION_LIST_OWNER_ITEMS => auction::handle_auction_list_owner_items(...)` in `src/world/handlers/mod.rs`.

## Implementation

```rust
/// Handle CMSG_AUCTION_LIST_OWNER_ITEMS (0x0259)
///
/// Packet format (vanilla 1.12.1):
/// - auctioneerGuid (packed u64)
/// - listfrom     (u32) -- paging offset
///
/// Mirrors C++ `WorldSession::HandleAuctionListOwnerItems`.
/// Rejects duplicate in-flight requests, validates the auction house,
/// clears feign-death auras, then enqueues an async owner-query task.
/// The async task system is not yet ported, so the actual enqueue is a TODO.
pub async fn handle_auction_list_owner_items(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Gate: only one AH list request at a time
    if session.received_ah_list_request() {
        return Ok(());
    }

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let auctioneer_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow::anyhow!("Failed to read auctioneer GUID"))?;
    let listfrom = packet.read_u32().unwrap_or(0);

    // Auctioneer validation
    let player = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Player not found"))?;

    let auction_house = get_checked_auction_house_for_auctioneer(
        &player,
        auctioneer_guid,
        &world.managers.auction_mgr,
        None, // NPC interaction not yet ported
    );

    if auction_house.is_none() {
        return Ok(());
    }

    // Remove feign-death auras if present
    let removed_feign_death = world
        .managers
        .player_mgr
        .with_player_mut(player_guid, |player| {
            let removed = player.auras.container.remove_spell_auras(AURA_FEIGN_DEATH);
            if !removed.is_empty() {
                player.auras.needs_client_update = true;
                player.auras.needs_stat_recalc = true;
            }
            removed.len()
        })
        .unwrap_or(0);

    if removed_feign_death > 0 {
        debug!(
            "CMSG_AUCTION_LIST_OWNER_ITEMS: cleared feign death aura(s) for player {:?}",
            player_guid
        );
    }

    // Mark request in-flight
    session.set_received_ah_list_request(true);

    // TODO: Build AuctionHouseClientQueryTask and enqueue via sWorld.AddAsyncTask
    // The async query system (AUCTION_QUERY_LIST_OWNER) is not yet ported.
    tracing::debug!(
        "CMSG_AUCTION_LIST_OWNER_ITEMS: async task enqueue TODO (account={} listfrom={})",
        session.account_id(),
        listfrom
    );

    Ok(())
}
```

## Test Plan

- **Duplicate request**: Call handler twice; second call returns immediately without sending packets.
- **Invalid auctioneer**: `get_checked_auction_house_for_auctioneer` returns `None`; handler returns `Ok(())` silently.
- **Feign-death removal**: Player with active feign-death aura has it removed and flags set.
- **Gate set**: On success path, `session.received_ah_list_request()` is `true` after handler completes.

## Dependencies

- `WorldSession::received_ah_list_request` / `set_received_ah_list_request` — already in Rust.
- `get_checked_auction_house_for_auctioneer` — already in Rust.
- `AURA_FEIGN_DEATH` — already in Rust.
- `AuctionHouseManager` — already in Rust.

## Blocking

- **Async task enqueue**: The `AuctionHouseClientQueryTask` and `sWorld.AddAsyncTask` infrastructure is not yet available. The handler will log a TODO and set the in-flight flag. This is acceptable because the async response path is a separate symbol/flow.
