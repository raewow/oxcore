# Parity gaps

Known behaviour differences from the reference core that are not fixed yet.
Remove an entry when it is fixed; add one when a fix is deliberately left
partial.

## Flight paths

- **Spell-started flights do not interrupt other casts.** The reference
  interrupts the player's other spells when a spell sends them on a flight.
- **No early landing.** The 1.14 client has a button to land at the next node;
  the server does not handle it (1.12 has no such option).
- **Taking off does not cancel an open trade.**
- **Flights in progress are not shown to players who come into range.** They
  see the rider at their current spot instead of moving along the path.

## Area triggers and teleporting

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

Queues, matches and Warsong Gulch, Arathi Basin and Alterac Valley exist; the
field logic is Lua (`scripts/battlegrounds/`).

- **No battleground raid groups.** Players keep their own groups; the
  reference puts each side in a temporary raid for the match.
- **Alterac Valley is the core battle only.** Missing: armour-scrap upgrades of
  defenders, the air, cavalry and world-boss assaults raised by turn-ins,
  landmines, quest completions on tower, graveyard and mine captures, the
  commanders' guards stopping on a commander's death, Snivvle, and the herald
  and captain yells (announcements go to battleground chat instead).
- **Buffs.** Warsong Gulch relies on the static buff spawns; Arathi Basin
  spawns one random buff per base at the start and does not rotate or respawn
  them.
- **Announcement text** for Arathi Basin and Alterac Valley node events is
  English text in the scripts, not localised strings.
- **Not implemented:** looting insignia from dead players, queue announcer
  messages, the group-size queue limit, Alterac Valley's minimum queue size and
  initial player cap, 1.14's `SMSG_BATTLEGROUND_INIT`, and the "only in
  battlegrounds" / "battleground ended" spell cast checks.
- **Event spawns and respawns.** A battleground creature that respawns on its
  own timer ignores whether its event is still active.

## World PvP

- **Eastern Plaguelands towers are not implemented.**
- **Silithus** covers zone 1377 only (not the Ahn'Qiraj zones' buff), does not
  show the dust bags, and does not keep its count over a restart.

## PvP flag and honour

- **Pets are not flagged** with their owner.
- **Helping a flagged player** (heals, buffs) does not flag the helper; the
  hook exists but no spell path calls it.
- **Contested and free-for-all PvP** (guards attacking flagged players,
  arenas such as Gurubashi) are not modelled.
- **Melee between players** is new: off-hand swings at players use main-hand
  damage, and duels are not supported.
- **Honour:** city protector titles are not set by the weekly ranking,
  `MSG_INSPECT_HONOR_STATS` is not handled, and the 1.14 honour tab is empty
  (its update fields have no 1.14 mapping).

## Vendors

- **Reputation discount is a stub.** `calculate_reputation_discount` in
  `game/vendor/system.rs` always returns Neutral, so vendors never discount.

## Not verified

- The 31 database-backed tests (ignored without a database) and live 1.12 and
  1.14 client checks of teleports, flights, the 1.12 creature waypoint packing
  fix, battlegrounds, world PvP, the PvP flag and honour. The battleground
  world tables have never been loaded by this code against a real database.
