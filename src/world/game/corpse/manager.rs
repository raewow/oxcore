//! CorpseManager - owns all player-corpse world objects.
//!
//! Corpses are treated as first-class world objects (like GameObjects). They:
//! - Have a unique GUID allocated from a counter with HighGuid::Corpse
//! - Live in `Map::corpses` + the spatial grid
//! - Are serialized to the update-packet pipeline via `build_create_msg`
//! - Use UPDATEFLAG_ALL | UPDATEFLAG_HAS_POSITION (0x50) like GameObjects —
//!   position-only (NO movement block, per project memory note about
//!   UPDATEFLAG_LIVING vs HAS_POSITION being a login-crash cause if wrong).

use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::shared::protocol::ObjectGuid;
use crate::world::game::player::death::corpse::Corpse;

/// CorpseManager holds all active corpse world objects across every map.
pub struct CorpseManager {
    /// All known corpses, keyed by corpse GUID (HighGuid::Corpse).
    corpses: DashMap<ObjectGuid, RwLock<Corpse>>,
    /// Monotonic counter for allocating new corpse GUIDs.
    next_counter: AtomicU32,
}

impl Default for CorpseManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CorpseManager {
    pub fn new() -> Self {
        Self {
            corpses: DashMap::new(),
            // Counter starts at 1; 0 is reserved as "no corpse".
            next_counter: AtomicU32::new(1),
        }
    }

    /// Allocate a fresh corpse GUID.
    pub fn alloc_corpse_guid(&self) -> ObjectGuid {
        let counter = self.next_counter.fetch_add(1, Ordering::Relaxed);
        ObjectGuid::new_corpse(counter)
    }

    /// Insert a corpse. Replaces any existing corpse with the same GUID.
    pub fn add(&self, corpse: Corpse) {
        self.corpses.insert(corpse.guid, RwLock::new(corpse));
    }

    /// Remove a corpse by GUID. Returns the removed corpse if present.
    pub fn remove(&self, guid: ObjectGuid) -> Option<Corpse> {
        self.corpses
            .remove(&guid)
            .map(|(_, lock)| lock.into_inner())
    }

    /// Get a read-only snapshot of a corpse (clone).
    pub fn get(&self, guid: ObjectGuid) -> Option<Corpse> {
        self.corpses
            .get(&guid)
            .map(|entry| entry.value().read().clone())
    }

    /// Iterate all known corpses as (guid, cloned Corpse). Used by the corpse
    /// expiration tick in Phase 4.
    pub fn all(&self) -> Vec<(ObjectGuid, Corpse)> {
        self.corpses
            .iter()
            .map(|entry| (*entry.key(), entry.value().read().clone()))
            .collect()
    }

    /// Make the highest-known counter at least `counter + 1`. Used when
    /// rehydrating corpses from the DB on startup so new allocations don't
    /// collide with restored GUIDs.
    pub fn bump_counter(&self, counter: u32) {
        // fetch_update: ensure the stored value exceeds `counter`.
        let next = counter.saturating_add(1);
        let _ = self
            .next_counter
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                if current < next {
                    Some(next)
                } else {
                    None
                }
            });
    }

    /// Return corpses whose type should change or be removed given the
    /// current unix-seconds timestamp. This is a pure observation — callers
    /// apply the changes via `convert_to_bones` / `remove`. We split it this
    /// way so the DeathSystem owns the map.remove_corpse / DB.delete side
    /// effects without this module depending on the map.
    pub fn find_expired(&self, now_secs: u64) -> Vec<CorpseExpiration> {
        use crate::world::game::player::death::corpse::CorpseType;
        use crate::world::game::player::death::flow::{BONES_EXPIRE_SECS, CORPSE_REPOP_TIME_MS};

        // Corpse → bones after CORPSE_REPOP_TIME (6 min).
        let corpse_lifetime_secs = (CORPSE_REPOP_TIME_MS / 1000) as u64;

        let mut out = Vec::new();
        for entry in self.corpses.iter() {
            let corpse = entry.value().read();
            let age = now_secs.saturating_sub(corpse.created_time);
            match corpse.corpse_type {
                CorpseType::ResurrectablePve | CorpseType::ResurrectablePvp => {
                    if age >= corpse_lifetime_secs {
                        out.push(CorpseExpiration::ConvertToBones {
                            guid: *entry.key(),
                            owner_guid: corpse.owner_guid,
                            map_id: corpse.map_id,
                            position: corpse.position,
                        });
                    }
                }
                CorpseType::Bones => {
                    if age >= BONES_EXPIRE_SECS {
                        out.push(CorpseExpiration::Remove {
                            guid: *entry.key(),
                            map_id: corpse.map_id,
                            position: corpse.position,
                        });
                    }
                }
            }
        }
        out
    }

    /// Flip an existing corpse to Bones. Returns the new corpse state or
    /// None if the GUID is unknown.
    pub fn convert_to_bones(
        &self,
        guid: ObjectGuid,
    ) -> Option<crate::world::game::player::death::corpse::Corpse> {
        use crate::world::game::player::death::corpse::CorpseType;
        let entry = self.corpses.get(&guid)?;
        let mut c = entry.value().write();
        c.corpse_type = CorpseType::Bones;
        // Scrub reclaim info — bones are not reclaimable.
        Some(c.clone())
    }
}

/// Result produced by `CorpseManager::find_expired`. Consumed by the
/// DeathSystem update tick in Phase 4.
#[derive(Debug, Clone)]
pub enum CorpseExpiration {
    ConvertToBones {
        guid: ObjectGuid,
        owner_guid: ObjectGuid,
        map_id: u32,
        position: crate::shared::protocol::Position,
    },
    Remove {
        guid: ObjectGuid,
        map_id: u32,
        position: crate::shared::protocol::Position,
    },
}

// Extra impl block re-opens for consistency with the previous block closing.
impl CorpseManager {
    /// Build UPDATETYPE_CREATE_OBJECT block for a corpse. Returns None if the
    /// GUID is not known. Matches the packet layout GameObjects use.
    ///
    /// Update fields written:
    ///   OBJECT_FIELD_GUID  (low/high u32 pair)
    ///   OBJECT_FIELD_TYPE  (bitmask: TYPEMASK_OBJECT | TYPEMASK_CORPSE = 0x81)
    ///   OBJECT_FIELD_ENTRY (0 for corpses — they have no template entry)
    ///   OBJECT_FIELD_SCALE_X (1.0)
    ///   CORPSE_FIELD_OWNER (player GUID)
    ///   CORPSE_FIELD_DISPLAY_ID (race+gender → corpse model)
    ///   CORPSE_FIELD_ITEM[0..18] (equipment display IDs)
    ///   CORPSE_FIELD_BYTES_1 (byte 0=skin, 1=face, 2=hair_style, 3=hair_color)
    ///   CORPSE_FIELD_BYTES_2 (byte 0=race, 1=0, 2=gender, 3=facial_style)
    ///   CORPSE_FIELD_FLAGS
    ///   CORPSE_FIELD_DYNAMIC_FLAGS (0 — nothing lootable yet)
    pub fn build_create_msg(
        &self,
        guid: ObjectGuid,
        _world: &crate::world::World,
    ) -> Option<crate::shared::messages::update::SmsgUpdateObject> {
        use crate::shared::messages::update::*;
        use crate::shared::protocol::update_fields::*;
        use crate::shared::protocol::updates::update_block_builder::update_flags;
        use crate::shared::protocol::updates::update_types::ObjectTypeId;

        let corpse = self.get(guid)?;

        // TYPEMASK_OBJECT (0x01) | TYPEMASK_CORPSE (0x80) = 0x81
        const TYPEMASK_OBJECT_CORPSE: u32 = 0x81;

        let display_id = corpse_display_id_for(corpse.race, corpse.gender);

        let bytes_1 = ((corpse.race as u32) << 8)
            | ((corpse.gender as u32) << 16)
            | ((corpse.skin as u32) << 24);
        let bytes_2 = (corpse.face as u32)
            | ((corpse.hair_style as u32) << 8)
            | ((corpse.hair_color as u32) << 16)
            | ((corpse.facial_style as u32) << 24);

        let corpse_flags = corpse_flag_bits(&corpse);

        let mut block =
            CreateObjectBlock::new(corpse.guid, ObjectTypeId::Corpse, ObjectType::Corpse)
                .with_position(corpse.position)
                .add_flags(update_flags::UPDATEFLAG_ALL | update_flags::UPDATEFLAG_HAS_POSITION)
                .set_guid_field(OBJECT_FIELD_GUID, corpse.guid)
                .set_field(OBJECT_FIELD_TYPE, TYPEMASK_OBJECT_CORPSE)
                .set_field(OBJECT_FIELD_ENTRY, 0)
                .set_float_field(OBJECT_FIELD_SCALE_X, 1.0)
                .set_guid_field(CORPSE_FIELD_OWNER, corpse.owner_guid)
                .set_float_field(CORPSE_FIELD_FACING, corpse.position.o)
                .set_float_field(CORPSE_FIELD_POS_X, corpse.position.x)
                .set_float_field(CORPSE_FIELD_POS_Y, corpse.position.y)
                .set_float_field(CORPSE_FIELD_POS_Z, corpse.position.z)
                .set_field(CORPSE_FIELD_DISPLAY_ID, display_id)
                .set_field(CORPSE_FIELD_BYTES_1, bytes_1)
                .set_field(CORPSE_FIELD_BYTES_2, bytes_2)
                .set_required(CORPSE_FIELD_GUILD, 0)
                .set_field(CORPSE_FIELD_FLAGS, corpse_flags)
                .set_field(CORPSE_FIELD_DYNAMIC_FLAGS, 0)
                .set_required(CORPSE_FIELD_PAD, 0);

        // Equipment display IDs in 19 slots starting at CORPSE_FIELD_ITEM.
        for (i, display) in corpse.equipment.iter().enumerate() {
            if *display != 0 {
                block = block.set_field(CORPSE_FIELD_ITEM + i as u32, *display);
            }
        }

        Some(SmsgUpdateObject::new().add_block(UpdateBlockData::CreateObject(block)))
    }
}

/// Look up the display id (creature model) for a player corpse.
///
/// Vanilla WoW has dedicated "corpse" models per race+gender. These values
/// come from CreatureDisplayInfo.dbc and match vmangos.
fn corpse_display_id_for(race: u8, gender: u8) -> u32 {
    // Gender: 0 = male, 1 = female.
    let male = gender == 0;
    match race {
        1 => {
            if male {
                49
            } else {
                50
            }
        } // Human
        2 => {
            if male {
                51
            } else {
                52
            }
        } // Orc
        3 => {
            if male {
                53
            } else {
                54
            }
        } // Dwarf
        4 => {
            if male {
                55
            } else {
                56
            }
        } // Night Elf
        5 => {
            if male {
                57
            } else {
                58
            }
        } // Undead
        6 => {
            if male {
                59
            } else {
                60
            }
        } // Tauren
        7 => {
            if male {
                1563
            } else {
                1564
            }
        } // Gnome
        8 => {
            if male {
                1478
            } else {
                1479
            }
        } // Troll
        _ => 49, // Fallback: Human male corpse
    }
}

/// Compute the CORPSE_FIELD_FLAGS value for a given corpse.
fn corpse_flag_bits(corpse: &Corpse) -> u32 {
    use crate::world::game::player::death::corpse::{corpse_flags, CorpseType};

    let mut flags = corpse_flags::UNK2; // vmangos always sets this
    if corpse.corpse_type == CorpseType::Bones {
        flags |= corpse_flags::BONES;
    }
    flags
}
