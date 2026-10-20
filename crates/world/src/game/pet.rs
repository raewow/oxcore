//! Runtime ownership and lifecycle for controlled player pets.
//!
//! Summoned pets (a warlock's demon) exist only while summoned. A hunter's tamed pets are
//! kept: one travels with the hunter (out, or dismissed until called back) and up to two
//! wait at the stable. Their records live in `characters.character_pet`, written back in
//! the background.

use crate::game::creature::CreatureManager;
use crate::game::player::PlayerManager;
use crate::World;
use anyhow::{bail, Result};
use dashmap::DashMap;
use oxcore_shared::protocol::ObjectGuid;
use std::f32::consts::FRAC_PI_2;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

const PET_FOLLOW_DISTANCE: f32 = 2.0;

/// Where a hunter's pet is kept (`character_pet.slot`).
pub mod pet_slot {
    /// With the hunter: out now, or brought back out at login.
    pub const CURRENT: u32 = 0;
    pub const FIRST_STABLE: u32 = 1;
    pub const LAST_STABLE: u32 = 2;
    /// Dismissed: travels with the hunter but stays away until called.
    pub const NOT_IN_SLOT: u32 = 100;
}

/// The most stable slots a hunter can buy.
pub const MAX_STABLE_SLOTS: u8 = 2;

const HUNTER_PET: i16 = 1;
const REACT_DEFENSIVE: i16 = 1;

/// One of a hunter's tamed pets.
#[derive(Debug, Clone, PartialEq)]
pub struct HunterPet {
    pub number: u32,
    pub entry: u32,
    pub display_id: u32,
    pub level: u8,
    pub name: String,
    pub loyalty: u32,
    pub slot: u32,
    pub health: u32,
    pub mana: u32,
    pub created_by_spell: u32,
}

impl HunterPet {
    /// With the hunter, whether out or dismissed.
    pub fn travels_with_owner(&self) -> bool {
        self.slot == pet_slot::CURRENT || self.slot > pet_slot::LAST_STABLE
    }

    pub fn is_stabled(&self) -> bool {
        (pet_slot::FIRST_STABLE..=pet_slot::LAST_STABLE).contains(&self.slot)
    }
}

pub struct PetSystem {
    player_mgr: Arc<PlayerManager>,
    creature_mgr: Arc<CreatureManager>,
    /// Each online hunter's tamed pets.
    hunter_pets: DashMap<ObjectGuid, Vec<HunterPet>>,
    /// Which tamed pet is out, by owner.
    out: DashMap<ObjectGuid, u32>,
    next_number: AtomicU32,
    pool: OnceLock<sqlx::PgPool>,
}

impl PetSystem {
    pub fn new(player_mgr: Arc<PlayerManager>, creature_mgr: Arc<CreatureManager>) -> Self {
        Self {
            player_mgr,
            creature_mgr,
            hunter_pets: DashMap::new(),
            out: DashMap::new(),
            next_number: AtomicU32::new(1),
            pool: OnceLock::new(),
        }
    }

    /// Pick up pet numbering from the character database and write records back to it.
    pub async fn load(&self, pool: &sqlx::PgPool) -> Result<()> {
        let highest: Option<i64> = sqlx::query_scalar("SELECT MAX(id) FROM characters.character_pet")
            .fetch_one(pool)
            .await?;
        self.next_number
            .store(u32::try_from(highest.unwrap_or(0))? + 1, Ordering::Relaxed);
        let _ = self.pool.set(pool.clone());
        Ok(())
    }

    /// Summon one transient pet and replace any existing active pet.
    pub fn summon(&self, owner_guid: ObjectGuid, entry: u32, world: &World) -> Result<ObjectGuid> {
        self.spawn(owner_guid, entry, None, world)
    }

    fn spawn(
        &self,
        owner_guid: ObjectGuid,
        entry: u32,
        level: Option<u8>,
        world: &World,
    ) -> Result<ObjectGuid> {
        let (position, map_id, instance_id, phase_mask, faction, owner_pvp) = self
            .player_mgr
            .with_player(owner_guid, |player| {
                (
                    player.movement.position,
                    player.map_id,
                    player.instance_id,
                    player.phase_mask,
                    player.faction_template(),
                    player.unit_flags & crate::game::common::unit_flags::PVP,
                )
            })
            .ok_or_else(|| anyhow::anyhow!("pet owner {owner_guid:?} is not online"))?;
        if self.creature_mgr.get_template(entry).is_none() {
            bail!("cannot summon pet with unknown creature entry {entry}");
        }

        self.dismiss(owner_guid, world);
        let pet_guid = self
            .creature_mgr
            .spawn_pet(
                entry,
                owner_guid,
                position,
                map_id,
                instance_id,
                phase_mask,
                faction,
                level,
            )
            .expect("template was checked before spawning pet");
        self.creature_mgr.with_creature_mut(pet_guid, |pet| {
            pet.motion_master.move_follow(
                owner_guid,
                PET_FOLLOW_DISTANCE,
                FRAC_PI_2,
                pet.guid,
                pet.position,
                pet.walk_speed(),
            );
            pet.following_target = Some(owner_guid);
            // A pet shares its owner's PvP flag.
            pet.unit_flags |= owner_pvp;
        });
        self.player_mgr
            .with_player_mut(owner_guid, |player| player.active_pet = Some(pet_guid));
        world
            .managers
            .map_mgr
            .get_or_create_map(map_id, instance_id)
            .add_creature(pet_guid, position);
        Ok(pet_guid)
    }

    /// Take the owner's pet out of the world. A tamed pet keeps its place and remembers
    /// its health. Safe to call repeatedly.
    pub fn dismiss(&self, owner_guid: ObjectGuid, world: &World) -> Option<ObjectGuid> {
        let pet_guid = self
            .player_mgr
            .with_player_mut(owner_guid, |player| player.active_pet.take())??;
        let (_, pet) = self.creature_mgr.remove_creature(pet_guid)?;
        if pet.owner_guid == Some(owner_guid) {
            if let Some(map) = world.managers.map_mgr.get_map(pet.map_id, pet.instance_id) {
                map.remove_creature(pet_guid, pet.position);
            }
        }
        if let Some((_, number)) = self.out.remove(&owner_guid) {
            let health = if pet.is_alive() { pet.current_health } else { 0 };
            if let Some(record) = self.update(owner_guid, number, |record| {
                record.health = health;
                record.mana = pet.current_mana;
            }) {
                self.save(owner_guid, record);
            }
        }
        Some(pet_guid)
    }

    /// Put the hunter's tamed pet away in `slot`: dismissed (`NOT_IN_SLOT`) or a stable slot.
    /// Returns false when no living tamed pet is out.
    pub fn dismiss_to_slot(&self, owner_guid: ObjectGuid, slot: u32, world: &World) -> bool {
        let Some(number) = self.out.get(&owner_guid).map(|n| *n) else {
            return false;
        };
        if !self.active_pet_alive(owner_guid) {
            return false;
        }
        self.dismiss(owner_guid, world);
        if let Some(record) = self.update(owner_guid, number, |record| record.slot = slot) {
            self.save(owner_guid, record);
        }
        true
    }

    /// Whether the owner's pet is out and alive.
    pub fn active_pet_alive(&self, owner_guid: ObjectGuid) -> bool {
        self.player_mgr
            .with_player(owner_guid, |p| p.active_pet)
            .flatten()
            .and_then(|pet| self.creature_mgr.with_creature(pet, |c| c.is_alive()))
            .unwrap_or(false)
    }

    /// The tamed pet that is out, if the owner's pet is one.
    pub fn out_pet(&self, owner_guid: ObjectGuid) -> Option<HunterPet> {
        let number = *self.out.get(&owner_guid)?;
        self.record(owner_guid, number)
    }

    /// Why `owner_guid` cannot start taming `target`, if they cannot: only hunters tame,
    /// only without a pet of their own, and only unowned tameable beasts no higher in
    /// level than themselves.
    pub fn taming_failure(
        &self,
        owner_guid: ObjectGuid,
        target: Option<ObjectGuid>,
    ) -> Option<oxcore_shared::messages::stable::PetTameFailure> {
        use oxcore_shared::messages::stable::PetTameFailure;
        const CLASS_HUNTER: u8 = 3;
        let (class, level, has_pet, gm) = self.player_mgr.with_player(owner_guid, |p| {
            (p.class, p.level, p.active_pet.is_some(), p.gm_mode)
        })?;
        if class != CLASS_HUNTER && !gm {
            return Some(PetTameFailure::UnitsCantTame);
        }
        if has_pet || self.pets(owner_guid).iter().any(HunterPet::travels_with_owner) {
            return Some(PetTameFailure::AnotherSummonActive);
        }
        let Some((entry, target_level, owned)) = target.and_then(|t| {
            self.creature_mgr
                .with_creature(t, |c| (c.entry, c.level, c.owner_guid.is_some() || t.is_pet()))
        }) else {
            return Some(PetTameFailure::InvalidCreature);
        };
        if owned {
            return Some(PetTameFailure::CreatureAlreadyOwned);
        }
        if target_level > level && !gm {
            return Some(PetTameFailure::TooHighLevel);
        }
        if !self.creature_mgr.is_tameable(entry) {
            return Some(PetTameFailure::NotTameable);
        }
        None
    }

    /// A hunter tamed `target`: it becomes their pet at its own level, out and kept.
    pub fn tame(
        &self,
        owner_guid: ObjectGuid,
        target: ObjectGuid,
        spell_id: u32,
        world: &World,
    ) -> Result<ObjectGuid> {
        let (entry, level, display_id) = self
            .creature_mgr
            .with_creature(target, |c| (c.entry, c.level, c.display_id))
            .ok_or_else(|| anyhow::anyhow!("taming target {target:?} is gone"))?;
        let name = self
            .creature_mgr
            .get_template(entry)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        // Whatever was out before goes back to being dismissed.
        if self.out.contains_key(&owner_guid) {
            self.dismiss_to_slot(owner_guid, pet_slot::NOT_IN_SLOT, world);
        }
        let pet_guid = self.spawn(owner_guid, entry, Some(level), world)?;
        let health = self
            .creature_mgr
            .with_creature(pet_guid, |c| c.max_health)
            .unwrap_or(1);
        let record = HunterPet {
            number: self.next_number.fetch_add(1, Ordering::Relaxed),
            entry,
            display_id,
            level,
            name,
            loyalty: 1,
            slot: pet_slot::CURRENT,
            health,
            mana: 0,
            created_by_spell: spell_id,
        };
        self.out.insert(owner_guid, record.number);
        self.hunter_pets
            .entry(owner_guid)
            .or_default()
            .push(record.clone());
        self.save(owner_guid, record);
        Ok(pet_guid)
    }

    /// Call Pet: bring out the hunter's pet that travels with them.
    pub fn call_pet(
        &self,
        owner_guid: ObjectGuid,
        world: &World,
    ) -> std::result::Result<ObjectGuid, oxcore_shared::messages::stable::PetTameFailure> {
        use oxcore_shared::messages::stable::PetTameFailure;
        let pet = self
            .pets(owner_guid)
            .into_iter()
            .find(HunterPet::travels_with_owner)
            .ok_or(PetTameFailure::NoPetAvailable)?;
        if pet.health == 0 {
            return Err(PetTameFailure::Dead);
        }
        self.bring_out(owner_guid, pet.number, world)
            .map_err(|_| PetTameFailure::NoPetAvailable)
    }

    /// Bring out tamed pet `number`, making it the one with the hunter.
    pub fn bring_out(&self, owner_guid: ObjectGuid, number: u32, world: &World) -> Result<ObjectGuid> {
        let pet = self
            .record(owner_guid, number)
            .ok_or_else(|| anyhow::anyhow!("{owner_guid:?} has no pet {number}"))?;
        let pet_guid = self.spawn(owner_guid, pet.entry, Some(pet.level), world)?;
        self.creature_mgr.with_creature_mut(pet_guid, |c| {
            c.current_health = pet.health.clamp(1, c.max_health);
            c.current_mana = pet.mana.min(c.max_mana);
        });
        self.out.insert(owner_guid, number);
        if let Some(record) = self.update(owner_guid, number, |r| r.slot = pet_slot::CURRENT) {
            self.save(owner_guid, record);
        }
        Ok(pet_guid)
    }

    /// Bring back a pet put away for travel: the tamed pet itself when it was one.
    pub fn resummon(&self, owner_guid: ObjectGuid, entry: u32, world: &World) -> Result<ObjectGuid> {
        let current = self
            .pets(owner_guid)
            .into_iter()
            .find(|p| p.slot == pet_slot::CURRENT && p.entry == entry && p.health > 0);
        match current {
            Some(pet) => self.bring_out(owner_guid, pet.number, world),
            None => self.summon(owner_guid, entry, world),
        }
    }

    /// The hunter gives up the tamed pet that is out; it is gone for good.
    pub fn abandon(&self, owner_guid: ObjectGuid, world: &World) {
        let number = self.out.get(&owner_guid).map(|n| *n);
        self.dismiss(owner_guid, world);
        let Some(number) = number else {
            return;
        };
        if let Some(mut pets) = self.hunter_pets.get_mut(&owner_guid) {
            pets.retain(|p| p.number != number);
        }
        if let Some(pool) = self.pool.get().cloned() {
            spawn_write(async move {
                sqlx::query("DELETE FROM characters.character_pet WHERE id = $1")
                    .bind(i64::from(number))
                    .execute(&pool)
                    .await
                    .map(|_| ())
            });
        }
    }

    /// The owner's tamed pets.
    pub fn pets(&self, owner_guid: ObjectGuid) -> Vec<HunterPet> {
        self.hunter_pets
            .get(&owner_guid)
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    pub fn record(&self, owner_guid: ObjectGuid, number: u32) -> Option<HunterPet> {
        self.hunter_pets
            .get(&owner_guid)?
            .iter()
            .find(|p| p.number == number)
            .cloned()
    }

    /// Change one of the owner's records; returns it as changed.
    fn update(
        &self,
        owner_guid: ObjectGuid,
        number: u32,
        change: impl FnOnce(&mut HunterPet),
    ) -> Option<HunterPet> {
        let mut pets = self.hunter_pets.get_mut(&owner_guid)?;
        let record = pets.iter_mut().find(|p| p.number == number)?;
        change(record);
        Some(record.clone())
    }

    /// Move a stabled pet to `slot` (used when swapping with the one that was out).
    pub fn set_slot(&self, owner_guid: ObjectGuid, number: u32, slot: u32) {
        if let Some(record) = self.update(owner_guid, number, |r| r.slot = slot) {
            self.save(owner_guid, record);
        }
    }

    /// Load the hunter's tamed pets when they log in, and bring out the one they had out.
    pub async fn on_player_login(&self, owner_guid: ObjectGuid, world: &World) {
        let Some(pool) = self.pool.get() else {
            return;
        };
        let rows: Vec<(i64, i64, Option<i64>, i64, Option<String>, i64, i64, i64, i64, i64)> =
            match sqlx::query_as(
                "SELECT id, entry, display_id, level, name, loyalty, slot, current_health, \
                 current_mana, created_by_spell FROM characters.character_pet \
                 WHERE owner_guid = $1 AND pet_type = $2",
            )
            .bind(i64::from(owner_guid.counter()))
            .bind(HUNTER_PET)
            .fetch_all(pool)
            .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!("Failed to load pets of {:?}: {e:#}", owner_guid);
                    return;
                }
            };
        let pets: Vec<HunterPet> = rows
            .into_iter()
            .filter_map(
                |(id, entry, display, level, name, loyalty, slot, health, mana, spell)| {
                    Some(HunterPet {
                        number: u32::try_from(id).ok()?,
                        entry: u32::try_from(entry).ok()?,
                        display_id: display.and_then(|d| u32::try_from(d).ok()).unwrap_or(0),
                        level: u8::try_from(level).ok()?,
                        name: name.unwrap_or_default(),
                        loyalty: u32::try_from(loyalty).ok()?,
                        slot: u32::try_from(slot).ok()?,
                        health: u32::try_from(health).ok()?,
                        mana: u32::try_from(mana).ok()?,
                        created_by_spell: u32::try_from(spell).ok()?,
                    })
                },
            )
            .collect();
        let out = pets
            .iter()
            .find(|p| p.slot == pet_slot::CURRENT && p.health > 0)
            .map(|p| p.number);
        self.hunter_pets.insert(owner_guid, pets);
        if let Some(number) = out {
            if let Err(e) = self.bring_out(owner_guid, number, world) {
                tracing::warn!("Could not bring out pet {} for {:?}: {e:#}", number, owner_guid);
            }
        }
    }

    pub fn on_player_logout(&self, owner_guid: ObjectGuid, world: &World) {
        self.dismiss(owner_guid, world);
        self.hunter_pets.remove(&owner_guid);
    }

    fn save(&self, owner_guid: ObjectGuid, pet: HunterPet) {
        let Some(pool) = self.pool.get().cloned() else {
            return;
        };
        let owner = i64::from(owner_guid.counter());
        spawn_write(async move {
            sqlx::query(
                "INSERT INTO characters.character_pet (id, entry, owner_guid, display_id, \
                 created_by_spell, pet_type, level, react_state, loyalty, name, slot, \
                 current_health, current_mana, save_time, action_bar_data, teach_spell_data) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, \
                 EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::NUMERIC(20,0), '', '') \
                 ON CONFLICT (id) DO UPDATE SET level = EXCLUDED.level, name = EXCLUDED.name, \
                 slot = EXCLUDED.slot, current_health = EXCLUDED.current_health, \
                 current_mana = EXCLUDED.current_mana, save_time = EXCLUDED.save_time",
            )
            .bind(i64::from(pet.number))
            .bind(i64::from(pet.entry))
            .bind(owner)
            .bind(i64::from(pet.display_id))
            .bind(i64::from(pet.created_by_spell))
            .bind(HUNTER_PET)
            .bind(i64::from(pet.level))
            .bind(REACT_DEFENSIVE)
            .bind(i64::from(pet.loyalty))
            .bind(&pet.name)
            .bind(i64::from(pet.slot))
            .bind(i64::from(pet.health))
            .bind(i64::from(pet.mana))
            .execute(&pool)
            .await
            .map(|_| ())
        });
    }

    #[cfg(test)]
    pub fn add_pet_for_test(&self, owner_guid: ObjectGuid, pet: HunterPet) {
        self.hunter_pets.entry(owner_guid).or_default().push(pet);
    }
}

fn spawn_write<F>(write: F)
where
    F: std::future::Future<Output = std::result::Result<(), sqlx::Error>> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            if let Err(e) = write.await {
                tracing::error!("Failed to save hunter pet: {e:#}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::manager::CreatureTemplate;
    use crate::game::creature::Creature;
    use crate::game::player::player::Player;
    use crate::Config;
    use oxcore_db::database::Databases;
    use oxcore_shared::messages::stable::PetTameFailure;
    use oxcore_shared::protocol::Position;
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;

    const WOLF: u32 = 299;
    const CLASS_HUNTER: u8 = 3;

    fn test_world() -> World {
        let pool = || {
            PgPoolOptions::new()
                .connect_lazy("postgres://test:test@localhost/test")
                .expect("lazy pool should be constructible")
        };
        World::new(
            Arc::new(Databases {
                world: pool(),
                character: pool(),
                auth: pool(),
                logs: oxcore_db::database::lazy_logs_pool(),
            }),
            Arc::new(Config::default()),
            50,
            PathBuf::from("."),
        )
    }

    fn add_hunter(world: &World, level: u8) -> ObjectGuid {
        let guid = ObjectGuid::new_player(1);
        let player = Player::new(guid, "Hunter".to_string(), 0, 0, 1, level, 2, CLASS_HUNTER, 0);
        world.managers.player_mgr.add_player(player, 1);
        guid
    }

    /// A tameable wolf of `level` standing next to the hunter.
    fn add_wolf(world: &World, level: u8, counter: u32) -> ObjectGuid {
        let template = CreatureTemplate {
            entry: WOLF,
            name: "Mangy Wolf".to_string(),
            subname: None,
            min_level: level,
            max_level: level,
            faction: 32,
            model_id_1: 903,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0x10,
            flags_extra: 0,
            creature_type: 1,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
            gold_min: 0,
            gold_max: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skinning_loot_id: 0,
            ai: Default::default(),
        };
        world.managers.creature_mgr.add_template(template.clone());
        world.managers.creature_mgr.set_pet_family_for_test(WOLF, 1);
        let guid = ObjectGuid::new_creature(WOLF, counter);
        world.managers.creature_mgr.add_creature(Creature::new(
            guid,
            WOLF,
            counter,
            Position::default(),
            0,
            0,
            &template,
            1,
            None,
        ));
        guid
    }

    #[tokio::test]
    async fn tamed_pet_is_kept_through_dismiss_and_call() {
        let world = test_world();
        let hunter = add_hunter(&world, 10);
        let wolf = add_wolf(&world, 8, 1);
        let pets = &world.systems.pet;

        assert_eq!(pets.taming_failure(hunter, Some(wolf)), None);
        let pet_guid = pets.tame(hunter, wolf, 1515, &world).unwrap();
        let level = world.managers.creature_mgr.with_creature(pet_guid, |c| c.level);
        assert_eq!(level, Some(8));
        let record = pets.out_pet(hunter).expect("tamed pet is out");
        assert_eq!((record.slot, record.name.as_str()), (pet_slot::CURRENT, "Mangy Wolf"));

        assert!(pets.dismiss_to_slot(hunter, pet_slot::NOT_IN_SLOT, &world));
        assert!(pets.out_pet(hunter).is_none());
        assert_eq!(pets.record(hunter, record.number).unwrap().slot, pet_slot::NOT_IN_SLOT);

        pets.call_pet(hunter, &world).expect("pet comes back");
        assert_eq!(pets.out_pet(hunter).map(|p| p.number), Some(record.number));
        assert_eq!(pets.record(hunter, record.number).unwrap().slot, pet_slot::CURRENT);
    }

    #[tokio::test]
    async fn calling_without_a_living_pet_fails() {
        let world = test_world();
        let hunter = add_hunter(&world, 10);
        let pets = &world.systems.pet;
        assert_eq!(pets.call_pet(hunter, &world), Err(PetTameFailure::NoPetAvailable));

        pets.add_pet_for_test(
            hunter,
            HunterPet {
                number: 40,
                entry: WOLF,
                display_id: 903,
                level: 8,
                name: "Wolf".to_string(),
                loyalty: 1,
                slot: pet_slot::NOT_IN_SLOT,
                health: 0,
                mana: 0,
                created_by_spell: 1515,
            },
        );
        assert_eq!(pets.call_pet(hunter, &world), Err(PetTameFailure::Dead));
    }

    #[tokio::test]
    async fn taming_needs_a_free_hunter_and_a_fitting_beast() {
        let world = test_world();
        let hunter = add_hunter(&world, 10);
        let pets = &world.systems.pet;

        let big = add_wolf(&world, 12, 2);
        assert_eq!(pets.taming_failure(hunter, Some(big)), Some(PetTameFailure::TooHighLevel));

        let wolf = add_wolf(&world, 8, 3);
        pets.tame(hunter, wolf, 1515, &world).unwrap();
        let other = add_wolf(&world, 8, 4);
        assert_eq!(
            pets.taming_failure(hunter, Some(other)),
            Some(PetTameFailure::AnotherSummonActive)
        );

        world
            .managers
            .player_mgr
            .with_player_mut(hunter, |p| p.class = 1);
        assert_eq!(pets.taming_failure(hunter, Some(other)), Some(PetTameFailure::UnitsCantTame));
    }

    #[tokio::test]
    async fn abandoning_forgets_the_pet() {
        let world = test_world();
        let hunter = add_hunter(&world, 10);
        let wolf = add_wolf(&world, 8, 5);
        let pets = &world.systems.pet;
        pets.tame(hunter, wolf, 1515, &world).unwrap();

        pets.abandon(hunter, &world);

        assert!(pets.pets(hunter).is_empty());
        assert_eq!(pets.call_pet(hunter, &world), Err(PetTameFailure::NoPetAvailable));
    }
}
