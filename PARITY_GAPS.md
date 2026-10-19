# Parity gaps

Known behaviour differences from the reference core that are not fixed yet.
Remove an entry when it is fixed; add one when a fix is deliberately left
partial.

## Flight paths

- **Honour-rank discount.** The reference takes money off a flight for honour
  rank as well as reputation; honour rank is not stored on the player, so only
  the Honored reputation discount applies.
- **Spell-started flights do not interrupt other casts.** The reference
  interrupts the player's other spells when a spell sends them on a flight.
- **No early landing.** The 1.14 client has a button to land at the next node;
  the server does not handle it (1.12 has no such option).
- **Taking off does not cancel an open trade.**
- **Flights in progress are not shown to players who come into range.** They
  see the rider at their current spot instead of moving along the path.

## Area triggers and teleporting

- **Battleground entrance triggers.** The level/team refusal for battleground
  entrances is missing because there is no battleground system.
- **Bind area ignores WMO areas.** Binding at an inn reads the area from
  terrain only, so inns inside buildings may record the zone instead of the
  sub-area.

## Maps and instances

- **Dungeon player cap has no source.** Vanilla `Map.dbc` has no player-limit
  column; the reference reads `map_template.player_limit`.
- **Hourly dungeon limit** counts per character, not per account, and game
  masters are not exempt.
- **Encounter in progress** is approximated: a raid-locking creature in combat
  counts as an engaged encounter, since instance scripts expose no generic
  in-progress state.

## Vendors

- **Reputation discount is a stub.** `calculate_reputation_discount` in
  `game/vendor/system.rs` always returns Neutral, so vendors never discount.

## Not verified

- The 31 database-backed tests (ignored without a database) and live 1.12 and
  1.14 client checks of teleports, flights and the 1.12 creature waypoint
  packing fix.
