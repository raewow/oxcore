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

- **Battleground raids** do not show the leader crown, and an invite sent from
  inside a battleground goes to the inviter's own group (as in the reference)
  but the invitee only sees it on leaving theirs.
- **Alterac Valley is the core battle only.** Missing: armour-scrap upgrades of
  defenders, the air, cavalry and world-boss assaults raised by turn-ins,
  landmines, quest completions on tower, graveyard and mine captures, the
  commanders' guards stopping on a commander's death, Snivvle, and the herald
  and captain yells (announcements go to battleground chat instead).
- **Buffs** are taken through each buff's area trigger. Gameobject traps never
  fire on their own anywhere (no proximity trigger exists), so a buff touched
  without crossing its trigger does nothing.
- **Announcement text** for Arathi Basin and Alterac Valley node events, and the
  queue announcer, is English text, not localised strings.
- **Insignia:** a taken insignia leaves the body as it is (the reference turns it
  into bones), and the side of an offline owner is unknown, so anyone may take it.

## World PvP

- **Eastern Plaguelands:** the towers, slider, map states, counts, Echoes of
  Lordaeron and each tower's reward work, but the buff is not given inside the
  Stratholme and Scholomance zones, the banners do not change art kit, the
  Eastwall squad stands where it is summoned instead of marching, the tower
  buffer NPCs are not spawned (their quest credit is given directly) and tower
  states do not survive a restart.
- **Silithus** covers zone 1377 only (not the Ahn'Qiraj zones' buff), does not
  show the dust bags, and does not keep its count over a restart.

## PvP flag and honour

- **Contested and free-for-all PvP** (guards attacking flagged players,
  arenas such as Gurubashi) are not modelled.
- **Duels** are not supported.

## Vendors

- **Reputation discount is a stub.** `calculate_reputation_discount` in
  `game/vendor/system.rs` always returns Neutral, so vendors never discount.

## Not verified

- The 31 database-backed tests (ignored without a database) and live 1.12 and
  1.14 client checks of teleports, flights, the 1.12 creature waypoint packing
  fix, battlegrounds (raids, insignia, buffs), world PvP (Eastern
  Plaguelands), the PvP flag and honour (inspect, 1.14 honour tab, city
  protectors). The battleground world tables have never been loaded by this
  code against a real database.
