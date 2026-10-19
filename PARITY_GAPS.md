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
  entrances is missing because there is no battleground system (see below).
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

## Battlegrounds

There is no battleground system; only scaffolding exists.

- **Wrong 1.12 opcodes.** `SMSG_BATTLEFIELD_JOINED` 0x2E1 (really
  `CMSG_LEAVE_BATTLEFIELD`), `CMSG_LEAVE_BATTLEFIELD` 0x2E5 (really
  `CMSG_GM_UNTEACH`), `CMSG_BATTLEFIELD_UN_QUEUE` 0x23F (really
  `SMSG_BATTLEFIELD_WIN`). Ten battleground opcodes are missing.
- **Wrong 1.12 layouts.** `SmsgBattlefieldStatus` has no queue slot and the
  wrong field order; `SmsgBattlefieldList` has no battlemaster guid or bracket.
- **No world-state updates** on either protocol (`SMSG_UPDATE_WORLD_STATE`).
- **No queue, invitations, per-match map copies, event-gated spawns, match
  lifecycle, entry point, leaving/Deserter, raid groups, scoreboard or
  spirit-guide waves.** The world tables for all of this are populated but not
  loaded. A character saved on a battleground map logs into one shared copy.
- **No Warsong Gulch, Arathi Basin or Alterac Valley logic.**
- **No PvP flag.** `CMSG_TOGGLE_PVP` is unhandled and nothing sets the flag.
- **Honour is a stand-in.** Simplified formula, no dishonourable kills, weekly
  honour kept in memory but never saved, no honour rank or weekly ranking.

## Vendors

- **Reputation discount is a stub.** `calculate_reputation_discount` in
  `game/vendor/system.rs` always returns Neutral, so vendors never discount.

## Not verified

- The 31 database-backed tests (ignored without a database) and live 1.12 and
  1.14 client checks of teleports, flights and the 1.12 creature waypoint
  packing fix.
