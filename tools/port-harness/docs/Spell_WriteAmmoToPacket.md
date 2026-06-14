# Spell::WriteAmmoToPacket

Spell::WriteAmmoToPacket serializes projectile display information for spell-start and spell-go packets.

For players it inspects the ranged weapon first, then falls back to the ammo item prototype from `PLAYER_AMMO_ID`. For non-players it scans the virtual item slots and extracts the first ranged-capable item it finds.

Implementation location: `reference/core/src/game/Spells/Spell.cpp:4529-4595`.
