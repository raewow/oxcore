//! PostgreSQL character persistence.
//!
//! Production world and web callers use `Databases.character` for the complete character surface.

use anyhow::{Context, Result};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterRow {
    pub guid: i64,
    pub account: i64,
    pub name: String,
    pub race: i16,
    pub class: i16,
    pub gender: i16,
    pub skin: i16,
    pub face: i16,
    pub hair_style: i16,
    pub hair_color: i16,
    pub facial_hair: i16,
    pub level: i16,
    pub xp: i64,
    pub money: i64,
    pub character_flags: i64,
    pub zone: i64,
    pub map: i64,
    pub instance: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub transport_guid: i64,
    pub transport_x: f32,
    pub transport_y: f32,
    pub transport_z: f32,
    pub transport_o: f32,
    pub known_taxi_mask: Option<String>,
    pub current_taxi_path: Option<String>,
    pub online: bool,
    pub played_time_total: i64,
    pub played_time_level: i64,
    pub health: i64,
    pub power1: i64,
    pub power2: i64,
    pub power3: i64,
    pub power4: i64,
    pub power5: i64,
    pub create_time: chrono::DateTime<chrono::Utc>,
    pub logout_time: Option<chrono::DateTime<chrono::Utc>>,
    pub rest_bonus: f32,
    pub reset_talents_multiplier: i64,
    pub reset_talents_time: i64,
    pub death_expire_time: i64,
    pub stable_slots: i16,
    pub bank_bag_slots: i16,
    pub extra_flags: i64,
    pub honor_rank_points: f32,
    pub honor_highest_rank: i64,
    pub honor_standing: i64,
    pub honor_last_week_hk: i64,
    pub honor_last_week_cp: f32,
    pub honor_stored_hk: i32,
    pub honor_stored_dk: i32,
    pub watched_faction: i32,
    pub drunk: i32,
    pub explored_zones: Option<String>,
    pub equipment_cache: Option<String>,
    pub ammo_id: i64,
    pub action_bars: i16,
    pub deleted_account: Option<i64>,
    pub deleted_name: Option<String>,
    pub deleted_time: Option<i64>,
    pub world_phase_mask: Option<i32>,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterSkillRow {
    pub skill: i64,
    pub value: i64,
    pub max: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterActionRow {
    pub button: i16,
    pub action: i64,
    pub r#type: i16,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterSpellRow {
    pub spell: i64,
    pub active: i16,
    pub disabled: i16,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterHomebindRow {
    pub map: i64,
    pub zone: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
}

/// Signed PostgreSQL representation of a persisted aura holder.
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterAuraRow {
    pub guid: i64,
    pub caster_guid: i64,
    pub item_guid: i64,
    pub spell: i64,
    pub stacks: i64,
    pub charges: i64,
    pub base_points0: f32,
    pub base_points1: f32,
    pub base_points2: f32,
    pub periodic_time0: i64,
    pub periodic_time1: i64,
    pub periodic_time2: i64,
    pub max_duration: i32,
    pub duration: i32,
    pub effect_index_mask: i16,
}

/// Signed PostgreSQL representation of a persisted spell cooldown.
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterSpellCooldownRow {
    pub guid: i64,
    pub spell: i64,
    pub spell_expire_time: i64,
    pub category: i64,
    pub category_expire_time: i64,
    pub item_id: i64,
}

#[derive(Debug, Clone)]
pub struct PgCharacterCreate<'a> {
    pub guid: i64,
    pub account: i64,
    pub name: &'a str,
    pub race: i16,
    pub class: i16,
    pub gender: i16,
    pub skin: i16,
    pub face: i16,
    pub hair_style: i16,
    pub hair_color: i16,
    pub facial_hair: i16,
    pub map: i64,
    pub zone: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub health: i64,
    pub power1: i64,
    pub money: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgItemInstanceRow {
    pub guid: i64,
    pub item_id: i64,
    pub owner_guid: i64,
    pub creator_guid: i64,
    pub gift_creator_guid: i64,
    pub count: i64,
    pub duration: i32,
    pub charges: Option<String>,
    pub flags: i64,
    pub enchantments: String,
    pub random_property_id: i16,
    pub durability: i32,
    pub text: i64,
    pub generated_loot: bool,
}

/// Signed database representation of a row in `characters.item_loot`.
///
/// PostgreSQL integers are signed, so conversion to protocol-sized unsigned values happens only
/// at the world boundary after range validation.
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgItemLootRow {
    pub guid: i64,
    pub owner_guid: i64,
    pub item_id: i64,
    pub amount: i64,
    pub property: i32,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgInventoryRow {
    pub guid: i64,
    pub bag: i64,
    pub slot: i16,
    pub item_guid: i64,
    pub item_id: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgQuestStatusRow {
    pub guid: i64,
    pub quest: i64,
    pub status: i16,
    pub rewarded: bool,
    pub explored: bool,
    pub timer: i64,
    pub mob_count1: i64,
    pub mob_count2: i64,
    pub mob_count3: i64,
    pub mob_count4: i64,
    pub item_count1: i64,
    pub item_count2: i64,
    pub item_count3: i64,
    pub item_count4: i64,
    pub reward_choice: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgReputationRow {
    pub guid: i64,
    pub faction: i64,
    pub standing: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgMailRow {
    pub id: i64,
    pub message_type: i16,
    pub stationery: i16,
    pub mail_template_id: i64,
    pub sender_guid: i64,
    pub receiver_guid: i64,
    pub subject: Option<String>,
    pub item_text_id: i64,
    pub has_items: i16,
    pub expire_time: i64,
    pub deliver_time: i64,
    pub money: i64,
    pub cod: i64,
    pub checked: i16,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgMailItemRow {
    pub mail_id: i64,
    pub item_guid: i64,
    pub item_id: i64,
    pub receiver_guid: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgItemTextRow {
    pub id: i64,
    pub text: Option<String>,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgAuctionRow {
    pub id: i64,
    pub house_id: i64,
    pub item_guid: i64,
    pub item_id: i64,
    pub seller_guid: i64,
    pub buyout_price: i32,
    pub expire_time: i64,
    pub buyer_guid: i64,
    pub last_bid: i32,
    pub start_bid: i32,
    pub deposit: i32,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgSocialRow {
    pub guid: i64,
    pub friend: i64,
    pub flags: i16,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGroupRow {
    pub group_id: i64,
    pub leader_guid: i64,
    pub main_tank_guid: i64,
    pub main_assistant_guid: i64,
    pub loot_method: i16,
    pub loot_threshold: i16,
    pub looter_guid: i64,
    pub icon1: i64,
    pub icon2: i64,
    pub icon3: i64,
    pub icon4: i64,
    pub icon5: i64,
    pub icon6: i64,
    pub icon7: i64,
    pub icon8: i64,
    pub is_raid: i16,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGroupMemberRow {
    pub group_id: i64,
    pub member_guid: i64,
    pub assistant: i16,
    pub subgroup: i16,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGroupInstanceRow {
    pub leader_guid: i64,
    pub instance: i64,
    pub permanent: i16,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgAuctionWithAccountRow {
    #[sqlx(flatten)]
    pub auction: PgAuctionRow,
    pub account: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgAuctionItemLoadRow {
    pub creator_guid: i64,
    pub gift_creator_guid: i64,
    pub count: i64,
    pub duration: i32,
    pub charges: Option<String>,
    pub flags: i64,
    pub enchantments: String,
    pub random_property_id: i16,
    pub durability: i32,
    pub text: i64,
    pub item_guid: i64,
    pub item_id: i64,
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGroupMemberWithCharacterDataRow {
    pub member_guid: i64,
    pub assistant: i16,
    pub subgroup: i16,
    pub name: Option<String>,
    pub level: Option<i16>,
    pub class: Option<i16>,
    pub zone: Option<i64>,
    pub online: Option<bool>,
}

#[derive(Clone)]
pub struct PgCharacterRepository {
    pool: Arc<PgPool>,
}

impl PgCharacterRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, character: &PgCharacterCreate<'_>) -> Result<()> {
        sqlx::query("INSERT INTO characters.characters (guid, account, name, race, class, gender, skin, face, hair_style, hair_color, facial_hair, money, zone, map, position_x, position_y, position_z, orientation, health, power1) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)")
            .bind(character.guid).bind(character.account).bind(character.name).bind(character.race)
            .bind(character.class).bind(character.gender).bind(character.skin).bind(character.face)
            .bind(character.hair_style).bind(character.hair_color).bind(character.facial_hair)
            .bind(character.money).bind(character.zone).bind(character.map).bind(character.position_x)
            .bind(character.position_y).bind(character.position_z).bind(character.orientation)
            .bind(character.health).bind(character.power1).execute(&*self.pool).await
            .context("Failed to create PostgreSQL character")?;
        Ok(())
    }

    pub async fn find_by_guid(&self, guid: i64) -> Result<Option<PgCharacterRow>> {
        sqlx::query_as("SELECT * FROM characters.characters WHERE guid = $1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL character by GUID")
    }

    pub async fn find_by_account(&self, account: i64) -> Result<Vec<PgCharacterRow>> {
        sqlx::query_as("SELECT * FROM characters.characters WHERE account = $1 ORDER BY guid")
            .bind(account)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL characters by account")
    }

    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM characters.characters WHERE name = $1)",
        )
        .bind(name)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to check PostgreSQL character name")
    }

    pub async fn find_equipped_items_batch(&self, guids: &[i64]) -> Result<Vec<PgInventoryRow>> {
        sqlx::query_as("SELECT guid, bag, slot, item_guid, item_id FROM characters.character_inventory WHERE guid = ANY($1) AND bag IN (0, 255) AND slot BETWEEN 0 AND 18 ORDER BY guid, slot")
            .bind(guids).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL equipped items")
    }

    pub async fn find_homebind(&self, guid: i64) -> Result<Option<PgCharacterHomebindRow>> {
        sqlx::query_as("SELECT map, zone, position_x, position_y, position_z FROM characters.character_homebind WHERE guid = $1")
            .bind(guid).fetch_optional(&*self.pool).await.context("Failed to fetch PostgreSQL character homebind")
    }

    pub async fn save_homebind(
        &self,
        guid: i64,
        map: i64,
        zone: i64,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_homebind (guid,map,zone,position_x,position_y,position_z) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (guid) DO UPDATE SET map=EXCLUDED.map,zone=EXCLUDED.zone,position_x=EXCLUDED.position_x,position_y=EXCLUDED.position_y,position_z=EXCLUDED.position_z")
            .bind(guid).bind(map).bind(zone).bind(x).bind(y).bind(z).execute(&*self.pool).await
            .context("Failed to save PostgreSQL character homebind").map(|_| ())
    }

    pub async fn find_auras(&self, guid: i64) -> Result<Vec<PgCharacterAuraRow>> {
        sqlx::query_as("SELECT guid,caster_guid::BIGINT,item_guid,spell,stacks,charges,base_points0,base_points1,base_points2,periodic_time0,periodic_time1,periodic_time2,max_duration,duration,effect_index_mask FROM characters.character_aura WHERE guid=$1")
            .bind(guid).fetch_all(&*self.pool).await.context("Failed to load PostgreSQL character auras")
    }

    pub async fn replace_auras(&self, guid: i64, auras: &[PgCharacterAuraRow]) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL aura transaction")?;
        sqlx::query("DELETE FROM characters.character_aura WHERE guid=$1")
            .bind(guid)
            .execute(&mut *tx)
            .await?;
        for aura in auras {
            sqlx::query("INSERT INTO characters.character_aura (guid,caster_guid,item_guid,spell,stacks,charges,base_points0,base_points1,base_points2,periodic_time0,periodic_time1,periodic_time2,max_duration,duration,effect_index_mask) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
                .bind(aura.guid).bind(aura.caster_guid).bind(aura.item_guid).bind(aura.spell).bind(aura.stacks).bind(aura.charges).bind(aura.base_points0).bind(aura.base_points1).bind(aura.base_points2).bind(aura.periodic_time0).bind(aura.periodic_time1).bind(aura.periodic_time2).bind(aura.max_duration).bind(aura.duration).bind(aura.effect_index_mask).execute(&mut *tx).await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL aura transaction")
    }

    pub async fn find_spell_cooldowns(
        &self,
        guid: i64,
    ) -> Result<Vec<PgCharacterSpellCooldownRow>> {
        sqlx::query_as("SELECT guid,spell,spell_expire_time,category,category_expire_time,item_id FROM characters.character_spell_cooldown WHERE guid=$1")
            .bind(guid).fetch_all(&*self.pool).await.context("Failed to load PostgreSQL spell cooldowns")
    }

    pub async fn replace_spell_cooldowns(
        &self,
        guid: i64,
        cooldowns: &[PgCharacterSpellCooldownRow],
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL cooldown transaction")?;
        sqlx::query("DELETE FROM characters.character_spell_cooldown WHERE guid=$1")
            .bind(guid)
            .execute(&mut *tx)
            .await?;
        for cooldown in cooldowns {
            sqlx::query("INSERT INTO characters.character_spell_cooldown (guid,spell,spell_expire_time,category,category_expire_time,item_id) VALUES ($1,$2,$3,$4,$5,$6)")
                .bind(cooldown.guid).bind(cooldown.spell).bind(cooldown.spell_expire_time).bind(cooldown.category).bind(cooldown.category_expire_time).bind(cooldown.item_id).execute(&mut *tx).await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL cooldown transaction")
    }

    pub async fn find_skills(&self, guid: i64) -> Result<Vec<PgCharacterSkillRow>> {
        sqlx::query_as("SELECT skill, value, max FROM characters.character_skills WHERE guid = $1")
            .bind(guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL character skills")
    }

    pub async fn find_actions(&self, guid: i64) -> Result<Vec<PgCharacterActionRow>> {
        sqlx::query_as("SELECT button, action, type FROM characters.character_action WHERE guid = $1 ORDER BY button")
            .bind(guid).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL character actions")
    }

    pub async fn find_spells(&self, guid: i64) -> Result<Vec<PgCharacterSpellRow>> {
        sqlx::query_as(
            "SELECT spell, active, disabled FROM characters.character_spell WHERE guid = $1",
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch PostgreSQL character spells")
    }

    pub async fn find_tutorials(&self, account: i64) -> Result<Option<[i64; 8]>> {
        sqlx::query_as("SELECT tut0, tut1, tut2, tut3, tut4, tut5, tut6, tut7 FROM characters.character_tutorial WHERE account = $1")
            .bind(account).fetch_optional(&*self.pool).await.context("Failed to fetch PostgreSQL tutorials")
            .map(|row: Option<(i64, i64, i64, i64, i64, i64, i64, i64)>| row.map(|r| [r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7]))
    }

    pub async fn find_account_data(&self, account: i64) -> Result<Vec<(i64, i64, Vec<u8>)>> {
        sqlx::query_as(
            "SELECT type, time::BIGINT, data FROM characters.account_data WHERE account = $1",
        )
        .bind(account)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch PostgreSQL account data")
    }

    pub async fn find_character_account_data(&self, guid: i64) -> Result<Vec<(i64, i64, Vec<u8>)>> {
        sqlx::query_as("SELECT type, time::BIGINT, data FROM characters.character_account_data WHERE guid = $1")
            .bind(guid).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL character account data")
    }

    pub async fn update_online(&self, guid: i64, online: bool) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET online = $1 WHERE guid = $2")
            .bind(online)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL character online state")
            .map(|_| ())
    }

    pub async fn update_position(
        &self,
        guid: i64,
        map: i64,
        instance: i64,
        zone: i64,
        x: f32,
        y: f32,
        z: f32,
        o: f32,
    ) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET map=$1, instance=$2, zone=$3, position_x=$4, position_y=$5, position_z=$6, orientation=$7 WHERE guid=$8")
            .bind(map).bind(instance).bind(zone).bind(x).bind(y).bind(z).bind(o).bind(guid).execute(&*self.pool).await.context("Failed to update PostgreSQL character position").map(|_| ())
    }

    pub async fn update_experience(&self, guid: i64, xp: i64, level: i16) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET xp=$1, level=$2 WHERE guid=$3")
            .bind(xp)
            .bind(level)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL character experience")
            .map(|_| ())
    }

    pub async fn update_health_and_power(
        &self,
        guid: i64,
        health: i64,
        power: [i64; 5],
    ) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET health=$1, power1=$2, power2=$3, power3=$4, power4=$5, power5=$6 WHERE guid=$7")
            .bind(health).bind(power[0]).bind(power[1]).bind(power[2]).bind(power[3]).bind(power[4]).bind(guid).execute(&*self.pool).await.context("Failed to update PostgreSQL character health and power").map(|_| ())
    }

    pub async fn update_rest_data(
        &self,
        guid: i64,
        rest_bonus: f32,
        character_flags: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET rest_bonus=$1, logout_time=CURRENT_TIMESTAMP, character_flags=$2 WHERE guid=$3")
            .bind(rest_bonus).bind(character_flags).bind(guid).execute(&*self.pool).await.context("Failed to update PostgreSQL character rest state").map(|_| ())
    }

    pub async fn save_actions(&self, guid: i64, buttons: &[(i16, i64, i16)]) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL action transaction")?;
        sqlx::query("DELETE FROM characters.character_action WHERE guid = $1")
            .bind(guid)
            .execute(&mut *tx)
            .await?;
        for &(button, action, kind) in buttons {
            sqlx::query("INSERT INTO characters.character_action (guid, button, action, type) VALUES ($1,$2,$3,$4)").bind(guid).bind(button).bind(action).bind(kind).execute(&mut *tx).await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL action transaction")
    }

    pub async fn save_skills(&self, guid: i64, skills: &[(i64, i64, i64)]) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL skill transaction")?;
        sqlx::query("DELETE FROM characters.character_skills WHERE guid = $1")
            .bind(guid)
            .execute(&mut *tx)
            .await?;
        for &(skill, value, max) in skills {
            sqlx::query("INSERT INTO characters.character_skills (guid, skill, value, max) VALUES ($1,$2,$3,$4)").bind(guid).bind(skill).bind(value).bind(max).execute(&mut *tx).await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL skill transaction")
    }

    pub async fn save_tutorials(&self, account: i64, flags: [i64; 8]) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_tutorial (account,tut0,tut1,tut2,tut3,tut4,tut5,tut6,tut7) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (account) DO UPDATE SET tut0=EXCLUDED.tut0,tut1=EXCLUDED.tut1,tut2=EXCLUDED.tut2,tut3=EXCLUDED.tut3,tut4=EXCLUDED.tut4,tut5=EXCLUDED.tut5,tut6=EXCLUDED.tut6,tut7=EXCLUDED.tut7")
            .bind(account).bind(flags[0]).bind(flags[1]).bind(flags[2]).bind(flags[3]).bind(flags[4]).bind(flags[5]).bind(flags[6]).bind(flags[7]).execute(&*self.pool).await.context("Failed to save PostgreSQL tutorials").map(|_| ())
    }

    pub async fn upsert_account_data(
        &self,
        account: i64,
        kind: i64,
        time: i64,
        data: &[u8],
    ) -> Result<()> {
        sqlx::query("INSERT INTO characters.account_data (account,type,time,data) VALUES ($1,$2,$3,$4) ON CONFLICT (account,type) DO UPDATE SET time=EXCLUDED.time,data=EXCLUDED.data").bind(account).bind(kind).bind(time).bind(data).execute(&*self.pool).await.context("Failed to save PostgreSQL account data").map(|_| ())
    }

    pub async fn upsert_character_account_data(
        &self,
        guid: i64,
        kind: i64,
        time: i64,
        data: &[u8],
    ) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_account_data (guid,type,time,data) VALUES ($1,$2,$3,$4) ON CONFLICT (guid,type) DO UPDATE SET time=EXCLUDED.time,data=EXCLUDED.data").bind(guid).bind(kind).bind(time).bind(data).execute(&*self.pool).await.context("Failed to save PostgreSQL character account data").map(|_| ())
    }

    pub async fn rename(&self, guid: i64, name: &str, character_flags: i64) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET name=$1, character_flags=$2 WHERE guid=$3")
            .bind(name)
            .bind(character_flags)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to rename PostgreSQL character")
            .map(|_| ())
    }

    pub async fn delete(&self, guid: i64, soft: bool) -> Result<()> {
        if soft {
            sqlx::query("UPDATE characters.characters SET deleted_name=name, deleted_account=account, deleted_time=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT, name='', account=0 WHERE guid=$1")
                .bind(guid).execute(&*self.pool).await.context("Failed to soft-delete PostgreSQL character")?;
        } else {
            sqlx::query("DELETE FROM characters.characters WHERE guid=$1")
                .bind(guid)
                .execute(&*self.pool)
                .await
                .context("Failed to delete PostgreSQL character")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct PgInventoryRepository {
    pool: Arc<PgPool>,
}

impl PgInventoryRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn load(&self, guid: i64) -> Result<Vec<PgInventoryRow>> {
        sqlx::query_as("SELECT guid, bag, slot, item_guid, item_id FROM characters.character_inventory WHERE guid = $1 ORDER BY bag, slot")
            .bind(guid).fetch_all(&*self.pool).await.context("Failed to load PostgreSQL inventory")
    }

    pub async fn find_item(&self, guid: i64) -> Result<Option<PgItemInstanceRow>> {
        sqlx::query_as("SELECT * FROM characters.item_instance WHERE guid = $1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to load PostgreSQL item instance")
    }

    pub async fn create_item(&self, item: &PgItemInstanceRow, slot: &PgInventoryRow) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL item transaction")?;
        sqlx::query("INSERT INTO characters.item_instance (guid, item_id, owner_guid, creator_guid, gift_creator_guid, count, duration, charges, flags, enchantments, random_property_id, durability, text, generated_loot) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(item.guid).bind(item.item_id).bind(item.owner_guid).bind(item.creator_guid).bind(item.gift_creator_guid).bind(item.count).bind(item.duration).bind(&item.charges).bind(item.flags).bind(&item.enchantments).bind(item.random_property_id).bind(item.durability).bind(item.text).bind(item.generated_loot).execute(&mut *tx).await.context("Failed to create PostgreSQL item instance")?;
        sqlx::query("INSERT INTO characters.character_inventory (guid, bag, slot, item_guid, item_id) VALUES ($1,$2,$3,$4,$5)")
            .bind(slot.guid).bind(slot.bag).bind(slot.slot).bind(item.guid).bind(item.item_id).execute(&mut *tx).await.context("Failed to create PostgreSQL inventory slot")?;
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL item transaction")
    }

    pub async fn update_item_count(&self, guid: i64, count: i64) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET count = $1 WHERE guid = $2")
            .bind(count)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL item count")?;
        Ok(())
    }

    pub async fn find_items_by_owner(&self, owner_guid: i64) -> Result<Vec<PgItemInstanceRow>> {
        sqlx::query_as("SELECT * FROM characters.item_instance WHERE owner_guid = $1")
            .bind(owner_guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load PostgreSQL owner items")
    }

    pub async fn player_money(&self, guid: i64) -> Result<i64> {
        sqlx::query_scalar("SELECT money FROM characters.characters WHERE guid = $1")
            .bind(guid)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to load PostgreSQL player money")
    }

    pub async fn update_player_money(&self, guid: i64, money: i64) -> Result<()> {
        self.update_item_field(
            "UPDATE characters.characters SET money=$1 WHERE guid=$2",
            money,
            guid,
        )
        .await
    }

    pub async fn move_item(&self, guid: i64, item_guid: i64, bag: i64, slot: i16) -> Result<()> {
        sqlx::query("UPDATE characters.character_inventory SET bag=$1, slot=$2 WHERE guid=$3 AND item_guid=$4")
            .bind(bag).bind(slot).bind(guid).bind(item_guid).execute(&*self.pool).await.context("Failed to move PostgreSQL inventory item").map(|_| ())
    }

    pub async fn delete_item(&self, item_guid: i64, all_references: bool) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL item delete transaction")?;
        for table in if all_references {
            &[
                "character_inventory",
                "auction",
                "mail_items",
                "character_gifts",
                "item_loot",
                "item_instance",
            ][..]
        } else {
            &["character_inventory", "item_loot", "item_instance"][..]
        } {
            let column = if *table == "item_loot" || *table == "item_instance" {
                "guid"
            } else {
                "item_guid"
            };
            sqlx::query(&format!("DELETE FROM characters.{table} WHERE {column}=$1"))
                .bind(item_guid)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL item delete")
    }

    pub async fn remove_from_slot(&self, guid: i64, bag: i64, slot: i16) -> Result<()> {
        sqlx::query(
            "DELETE FROM characters.character_inventory WHERE guid=$1 AND bag=$2 AND slot=$3",
        )
        .bind(guid)
        .bind(bag)
        .bind(slot)
        .execute(&*self.pool)
        .await
        .context("Failed to remove PostgreSQL inventory slot")
        .map(|_| ())
    }

    pub async fn add_to_slot(&self, guid: i64, item_guid: i64, bag: i64, slot: i16) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_inventory (guid,bag,slot,item_guid,item_id) SELECT $1,$2,$3,guid,item_id FROM characters.item_instance WHERE guid=$4 ON CONFLICT (guid,bag,slot) DO UPDATE SET item_guid=EXCLUDED.item_guid,item_id=EXCLUDED.item_id")
            .bind(guid).bind(bag).bind(slot).bind(item_guid).execute(&*self.pool).await.context("Failed to add PostgreSQL inventory slot").map(|_| ())
    }

    async fn update_item_field(&self, query: &str, value: i64, guid: i64) -> Result<()> {
        sqlx::query(query)
            .bind(value)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL item field")
            .map(|_| ())
    }

    pub async fn update_item_owner(&self, guid: i64, owner: i64) -> Result<()> {
        self.update_item_field(
            "UPDATE characters.item_instance SET owner_guid=$1 WHERE guid=$2",
            owner,
            guid,
        )
        .await
    }
    pub async fn update_item_durability(&self, guid: i64, durability: i64) -> Result<()> {
        self.update_item_field(
            "UPDATE characters.item_instance SET durability=$1 WHERE guid=$2",
            durability,
            guid,
        )
        .await
    }
    pub async fn update_item_duration(&self, guid: i64, duration: i64) -> Result<()> {
        self.update_item_field(
            "UPDATE characters.item_instance SET duration=$1 WHERE guid=$2",
            duration,
            guid,
        )
        .await
    }
    pub async fn update_item_flags(&self, guid: i64, flags: i64) -> Result<()> {
        self.update_item_field(
            "UPDATE characters.item_instance SET flags=$1 WHERE guid=$2",
            flags,
            guid,
        )
        .await
    }
    pub async fn update_item_enchantments(&self, guid: i64, enchantments: &str) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET enchantments=$1 WHERE guid=$2")
            .bind(enchantments)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL item enchantments")
            .map(|_| ())
    }
    pub async fn update_item_charges(&self, guid: i64, charges: &str) -> Result<()> {
        sqlx::query("UPDATE characters.item_instance SET charges=$1 WHERE guid=$2")
            .bind(charges)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL item charges")
            .map(|_| ())
    }
    pub async fn next_item_guid(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(guid) FROM characters.item_instance")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL item GUID")
    }
}

#[derive(Clone)]
pub struct PgQuestRepository {
    pool: Arc<PgPool>,
}

impl PgQuestRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn load(&self, guid: i64) -> Result<Vec<PgQuestStatusRow>> {
        sqlx::query_as(
            "SELECT * FROM characters.character_queststatus WHERE guid = $1 ORDER BY quest",
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load PostgreSQL quest status")
    }

    pub async fn save(&self, status: &PgQuestStatusRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_queststatus (guid, quest, status, rewarded, explored, timer, mob_count1, mob_count2, mob_count3, mob_count4, item_count1, item_count2, item_count3, item_count4, reward_choice) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) ON CONFLICT (guid, quest) DO UPDATE SET status=EXCLUDED.status, rewarded=EXCLUDED.rewarded, explored=EXCLUDED.explored, timer=EXCLUDED.timer, mob_count1=EXCLUDED.mob_count1, mob_count2=EXCLUDED.mob_count2, mob_count3=EXCLUDED.mob_count3, mob_count4=EXCLUDED.mob_count4, item_count1=EXCLUDED.item_count1, item_count2=EXCLUDED.item_count2, item_count3=EXCLUDED.item_count3, item_count4=EXCLUDED.item_count4, reward_choice=EXCLUDED.reward_choice")
            .bind(status.guid).bind(status.quest).bind(status.status).bind(status.rewarded).bind(status.explored).bind(status.timer).bind(status.mob_count1).bind(status.mob_count2).bind(status.mob_count3).bind(status.mob_count4).bind(status.item_count1).bind(status.item_count2).bind(status.item_count3).bind(status.item_count4).bind(status.reward_choice).execute(&*self.pool).await.context("Failed to save PostgreSQL quest status")?;
        Ok(())
    }
    pub async fn find(&self, guid: i64, quest: i64) -> Result<Option<PgQuestStatusRow>> {
        sqlx::query_as("SELECT * FROM characters.character_queststatus WHERE guid=$1 AND quest=$2")
            .bind(guid)
            .bind(quest)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL quest status")
    }
    pub async fn find_rewarded(&self, guid: i64) -> Result<Vec<PgQuestStatusRow>> {
        sqlx::query_as(
            "SELECT * FROM characters.character_queststatus WHERE guid=$1 AND rewarded=true",
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch rewarded PostgreSQL quests")
    }
    pub async fn delete(&self, guid: i64, quest: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_queststatus WHERE guid=$1 AND quest=$2")
            .bind(guid)
            .bind(quest)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL quest status")
            .map(|_| ())
    }
    pub async fn delete_all(&self, guid: i64, rewarded_only: bool) -> Result<()> {
        let query = if rewarded_only {
            "DELETE FROM characters.character_queststatus WHERE guid=$1 AND rewarded=true"
        } else {
            "DELETE FROM characters.character_queststatus WHERE guid=$1"
        };
        sqlx::query(query)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL quest statuses")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgReputationRepository {
    pool: Arc<PgPool>,
}

impl PgReputationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn load(&self, guid: i64) -> Result<Vec<PgReputationRow>> {
        sqlx::query_as(
            "SELECT * FROM characters.character_reputation WHERE guid = $1 ORDER BY faction",
        )
        .bind(guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to load PostgreSQL reputations")
    }

    pub async fn save(&self, reputation: &PgReputationRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_reputation (guid, faction, standing, flags) VALUES ($1,$2,$3,$4) ON CONFLICT (guid, faction) DO UPDATE SET standing=EXCLUDED.standing, flags=EXCLUDED.flags")
            .bind(reputation.guid).bind(reputation.faction).bind(reputation.standing).bind(reputation.flags)
            .execute(&*self.pool).await.context("Failed to save PostgreSQL reputation")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PgMailRepository {
    pool: Arc<PgPool>,
}

impl PgMailRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT guid FROM characters.characters WHERE name = $1")
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find PostgreSQL player by name")
    }

    pub async fn find_by_id(&self, id: i64, receiver_guid: i64) -> Result<Option<PgMailRow>> {
        sqlx::query_as("SELECT * FROM characters.mail WHERE id = $1 AND receiver_guid = $2")
            .bind(id)
            .bind(receiver_guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL mail by ID")
    }

    pub async fn find_by_receiver(&self, receiver_guid: i64) -> Result<Vec<PgMailRow>> {
        sqlx::query_as("SELECT * FROM characters.mail WHERE receiver_guid = $1 AND checked < 2 ORDER BY id DESC")
            .bind(receiver_guid).fetch_all(&*self.pool).await
            .context("Failed to fetch PostgreSQL mail for receiver")
    }

    pub async fn find_mail_items(&self, mail_id: i64) -> Result<Vec<PgMailItemRow>> {
        sqlx::query_as("SELECT * FROM characters.mail_items WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL mail items")
    }

    pub async fn find_items_by_receiver(&self, receiver_guid: i64) -> Result<Vec<PgMailItemRow>> {
        sqlx::query_as("SELECT * FROM characters.mail_items WHERE receiver_guid = $1")
            .bind(receiver_guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL receiver mail items")
    }

    pub async fn count_by_receiver(&self, receiver_guid: i64) -> Result<i64> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM characters.mail WHERE receiver_guid = $1 AND checked < 2",
        )
        .bind(receiver_guid)
        .fetch_one(&*self.pool)
        .await
        .context("Failed to count PostgreSQL mail")
    }

    pub async fn find_item_text(&self, id: i64) -> Result<Option<PgItemTextRow>> {
        sqlx::query_as("SELECT * FROM characters.item_text WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL item text")
    }

    pub async fn create(&self, mail: &PgMailRow) -> Result<i64> {
        sqlx::query_scalar("INSERT INTO characters.mail (message_type, stationery, mail_template_id, sender_guid, receiver_guid, subject, item_text_id, has_items, expire_time, deliver_time, money, cod, checked) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) RETURNING id")
            .bind(mail.message_type).bind(mail.stationery).bind(mail.mail_template_id).bind(mail.sender_guid).bind(mail.receiver_guid).bind(&mail.subject).bind(mail.item_text_id).bind(mail.has_items).bind(mail.expire_time).bind(mail.deliver_time).bind(mail.money).bind(mail.cod).bind(mail.checked)
            .fetch_one(&*self.pool).await.context("Failed to create PostgreSQL mail")
    }

    pub async fn add_item(&self, item: &PgMailItemRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.mail_items (mail_id, item_guid, item_id, receiver_guid) VALUES ($1,$2,$3,$4)")
            .bind(item.mail_id).bind(item.item_guid).bind(item.item_id).bind(item.receiver_guid).execute(&*self.pool).await.context("Failed to add PostgreSQL mail item")?;
        Ok(())
    }

    pub async fn update_checked(&self, id: i64, receiver_guid: i64, checked: i16) -> Result<()> {
        self.update_mail(
            "UPDATE characters.mail SET checked = $1 WHERE id = $2 AND receiver_guid = $3",
            checked,
            id,
            receiver_guid,
        )
        .await
    }

    pub async fn clear_money(&self, id: i64, receiver_guid: i64) -> Result<()> {
        sqlx::query("UPDATE characters.mail SET money = 0, checked = 1 WHERE id = $1 AND receiver_guid = $2").bind(id).bind(receiver_guid).execute(&*self.pool).await.context("Failed to clear PostgreSQL mail money")?;
        Ok(())
    }

    pub async fn remove_item(&self, mail_id: i64, item_guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.mail_items WHERE mail_id = $1 AND item_guid = $2")
            .bind(mail_id)
            .bind(item_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to remove PostgreSQL mail item")?;
        Ok(())
    }

    pub async fn update_has_items(
        &self,
        id: i64,
        receiver_guid: i64,
        has_items: i16,
    ) -> Result<()> {
        self.update_mail(
            "UPDATE characters.mail SET has_items = $1 WHERE id = $2 AND receiver_guid = $3",
            has_items,
            id,
            receiver_guid,
        )
        .await
    }

    async fn update_mail(
        &self,
        query: &str,
        value: i16,
        id: i64,
        receiver_guid: i64,
    ) -> Result<()> {
        sqlx::query(query)
            .bind(value)
            .bind(id)
            .bind(receiver_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL mail")?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.mail WHERE id = $1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL mail")?;
        Ok(())
    }

    pub async fn return_to_sender(
        &self,
        id: i64,
        receiver_guid: i64,
        sender_guid: i64,
    ) -> Result<()> {
        sqlx::query("UPDATE characters.mail SET receiver_guid = $1, sender_guid = $2, checked = 0 WHERE id = $3 AND receiver_guid = $2")
            .bind(sender_guid).bind(receiver_guid).bind(id).execute(&*self.pool).await.context("Failed to return PostgreSQL mail")?;
        Ok(())
    }

    pub async fn create_item_text(&self, text: &str) -> Result<i64> {
        sqlx::query_scalar("INSERT INTO characters.item_text (text) VALUES ($1) RETURNING id")
            .bind(text)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to create PostgreSQL item text")
    }

    pub async fn delete_expired(&self, current_time: i64) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM characters.mail WHERE expire_time > 0 AND expire_time < $1")
                .bind(current_time)
                .execute(&*self.pool)
                .await
                .context("Failed to delete expired PostgreSQL mail")?
                .rows_affected(),
        )
    }
}

#[derive(Clone)]
pub struct PgAuctionRepository {
    pool: Arc<PgPool>,
}

impl PgAuctionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn get_max_auction_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(id) FROM characters.auction")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max auction ID")
    }
    pub async fn find_by_id(&self, id: i64) -> Result<Option<PgAuctionRow>> {
        sqlx::query_as("SELECT * FROM characters.auction WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL auction")
    }
    pub async fn find_by_house(&self, house_id: i64) -> Result<Vec<PgAuctionRow>> {
        self.find_many("house_id", house_id).await
    }
    pub async fn find_by_seller(&self, seller_guid: i64) -> Result<Vec<PgAuctionRow>> {
        self.find_many("seller_guid", seller_guid).await
    }
    pub async fn find_by_bidder(&self, buyer_guid: i64) -> Result<Vec<PgAuctionRow>> {
        self.find_many("buyer_guid", buyer_guid).await
    }
    async fn find_many(&self, column: &str, value: i64) -> Result<Vec<PgAuctionRow>> {
        sqlx::query_as(&format!(
            "SELECT * FROM characters.auction WHERE {column} = $1"
        ))
        .bind(value)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch PostgreSQL auctions")
    }
    pub async fn find_active_auctions(&self, current_time: i64) -> Result<Vec<PgAuctionRow>> {
        sqlx::query_as("SELECT * FROM characters.auction WHERE expire_time > $1")
            .bind(current_time)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch active PostgreSQL auctions")
    }
    pub async fn find_active_by_house_with_account(
        &self,
        house_id: i64,
        current_time: i64,
    ) -> Result<Vec<PgAuctionWithAccountRow>> {
        sqlx::query_as("SELECT a.id, a.house_id, a.item_guid, a.item_id, a.seller_guid, a.buyout_price, a.expire_time, a.buyer_guid, a.last_bid, a.start_bid, a.deposit, c.account FROM characters.auction a JOIN characters.characters c ON c.guid = a.seller_guid WHERE a.house_id = $1 AND a.expire_time > $2")
            .bind(house_id).bind(current_time).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL auctions with account")
    }
    pub async fn find_all_for_load(&self) -> Result<Vec<PgAuctionRow>> {
        sqlx::query_as("SELECT * FROM characters.auction")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load PostgreSQL auctions")
    }
    pub async fn find_all_items_for_load(&self) -> Result<Vec<PgAuctionItemLoadRow>> {
        sqlx::query_as("SELECT i.creator_guid, i.gift_creator_guid, i.count, i.duration, i.charges, i.flags, i.enchantments, i.random_property_id, i.durability, i.text, a.item_guid, i.item_id FROM characters.auction a JOIN characters.item_instance i ON a.item_guid = i.guid")
            .fetch_all(&*self.pool).await.context("Failed to load PostgreSQL auction items")
    }
    pub async fn create_auction(&self, auction: &PgAuctionRow) -> Result<()> {
        self.save_auction(auction, false).await
    }
    pub async fn update_auction(&self, auction: &PgAuctionRow) -> Result<()> {
        self.save_auction(auction, true).await
    }
    async fn save_auction(&self, auction: &PgAuctionRow, update: bool) -> Result<()> {
        let query = if update {
            "UPDATE characters.auction SET house_id=$1,item_guid=$2,item_id=$3,seller_guid=$4,buyout_price=$5,expire_time=$6,buyer_guid=$7,last_bid=$8,start_bid=$9,deposit=$10 WHERE id=$11"
        } else {
            "INSERT INTO characters.auction (house_id,item_guid,item_id,seller_guid,buyout_price,expire_time,buyer_guid,last_bid,start_bid,deposit,id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"
        };
        sqlx::query(query)
            .bind(auction.house_id)
            .bind(auction.item_guid)
            .bind(auction.item_id)
            .bind(auction.seller_guid)
            .bind(auction.buyout_price)
            .bind(auction.expire_time)
            .bind(auction.buyer_guid)
            .bind(auction.last_bid)
            .bind(auction.start_bid)
            .bind(auction.deposit)
            .bind(auction.id)
            .execute(&*self.pool)
            .await
            .context("Failed to save PostgreSQL auction")?;
        Ok(())
    }
    pub async fn update_bid(&self, id: i64, buyer_guid: i64, last_bid: i32) -> Result<()> {
        sqlx::query("UPDATE characters.auction SET buyer_guid = $1, last_bid = $2 WHERE id = $3")
            .bind(buyer_guid)
            .bind(last_bid)
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL auction bid")?;
        Ok(())
    }
    pub async fn delete_auction(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.auction WHERE id = $1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL auction")?;
        Ok(())
    }
    pub async fn delete_expired_auctions(&self, current_time: i64) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM characters.auction WHERE expire_time <= $1")
                .bind(current_time)
                .execute(&*self.pool)
                .await
                .context("Failed to delete expired PostgreSQL auctions")?
                .rows_affected(),
        )
    }
}

#[derive(Clone)]
pub struct PgSocialRepository {
    pool: Arc<PgPool>,
}

impl PgSocialRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn find_by_guid(&self, guid: i64) -> Result<Vec<PgSocialRow>> {
        sqlx::query_as("SELECT * FROM characters.character_social WHERE guid = $1 LIMIT 255")
            .bind(guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL social entries")
    }
    pub async fn exists(&self, guid: i64, friend: i64) -> Result<bool> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM characters.character_social WHERE guid = $1 AND friend = $2)").bind(guid).bind(friend).fetch_one(&*self.pool).await.context("Failed to check PostgreSQL social entry")
    }
    pub async fn find_player_guid_by_name(&self, name: &str) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT guid FROM characters.characters WHERE name = $1 LIMIT 1")
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find PostgreSQL player by name")
    }
    pub async fn get_character_name(&self, guid: i64) -> Result<Option<String>> {
        sqlx::query_scalar("SELECT name FROM characters.characters WHERE guid = $1 LIMIT 1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find PostgreSQL character name")
    }
    pub async fn add_or_update(&self, guid: i64, friend: i64, flags: i16) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_social (guid, friend, flags) VALUES ($1,$2,$3) ON CONFLICT (guid, friend) DO UPDATE SET flags = characters.character_social.flags | EXCLUDED.flags").bind(guid).bind(friend).bind(flags).execute(&*self.pool).await.context("Failed to save PostgreSQL social entry")?;
        Ok(())
    }
    pub async fn update_flags(&self, guid: i64, friend: i64, flags: i16) -> Result<()> {
        self.set_flags("flags = $1", guid, friend, flags).await
    }
    pub async fn add_flags(&self, guid: i64, friend: i64, flags: i16) -> Result<()> {
        self.set_flags("flags = flags | $1", guid, friend, flags)
            .await
    }
    pub async fn remove_flags(&self, guid: i64, friend: i64, flags: i16) -> Result<()> {
        self.set_flags("flags = flags & ~$1", guid, friend, flags)
            .await
    }
    async fn set_flags(&self, change: &str, guid: i64, friend: i64, flags: i16) -> Result<()> {
        sqlx::query(&format!(
            "UPDATE characters.character_social SET {change} WHERE guid = $2 AND friend = $3"
        ))
        .bind(flags)
        .bind(guid)
        .bind(friend)
        .execute(&*self.pool)
        .await
        .context("Failed to update PostgreSQL social flags")?;
        Ok(())
    }
    pub async fn remove(&self, guid: i64, friend: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_social WHERE guid = $1 AND friend = $2")
            .bind(guid)
            .bind(friend)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL social entry")?;
        Ok(())
    }
    pub async fn delete_all_for_character(&self, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_social WHERE guid = $1 OR friend = $1")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL character social entries")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PgGroupRepository {
    pool: Arc<PgPool>,
}

impl PgGroupRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn get_max_group_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(group_id) FROM characters.groups")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max group ID")
    }
    pub async fn find_by_id(&self, group_id: i64) -> Result<Option<PgGroupRow>> {
        sqlx::query_as("SELECT * FROM characters.groups WHERE group_id = $1")
            .bind(group_id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL group")
    }
    pub async fn find_all(&self) -> Result<Vec<PgGroupRow>> {
        sqlx::query_as("SELECT * FROM characters.groups")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL groups")
    }
    pub async fn find_members(&self, group_id: i64) -> Result<Vec<PgGroupMemberRow>> {
        sqlx::query_as("SELECT * FROM characters.group_member WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL group members")
    }
    pub async fn find_all_members(&self) -> Result<Vec<PgGroupMemberRow>> {
        sqlx::query_as("SELECT * FROM characters.group_member")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL group members")
    }
    pub async fn find_group_for_member(&self, member_guid: i64) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT group_id FROM characters.group_member WHERE member_guid = $1")
            .bind(member_guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to find PostgreSQL group for member")
    }
    pub async fn find_members_with_character_data(
        &self,
        group_id: i64,
    ) -> Result<Vec<PgGroupMemberWithCharacterDataRow>> {
        sqlx::query_as("SELECT gm.member_guid, gm.assistant, gm.subgroup, c.name, c.level, c.class, c.zone, c.online FROM characters.group_member gm LEFT JOIN characters.characters c ON c.guid = gm.member_guid WHERE gm.group_id = $1")
            .bind(group_id).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL group character data")
    }
    pub async fn save_group(&self, group: &PgGroupRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.groups (group_id,leader_guid,main_tank_guid,main_assistant_guid,loot_method,loot_threshold,looter_guid,icon1,icon2,icon3,icon4,icon5,icon6,icon7,icon8,is_raid) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) ON CONFLICT (group_id) DO UPDATE SET leader_guid=EXCLUDED.leader_guid,main_tank_guid=EXCLUDED.main_tank_guid,main_assistant_guid=EXCLUDED.main_assistant_guid,loot_method=EXCLUDED.loot_method,loot_threshold=EXCLUDED.loot_threshold,looter_guid=EXCLUDED.looter_guid,icon1=EXCLUDED.icon1,icon2=EXCLUDED.icon2,icon3=EXCLUDED.icon3,icon4=EXCLUDED.icon4,icon5=EXCLUDED.icon5,icon6=EXCLUDED.icon6,icon7=EXCLUDED.icon7,icon8=EXCLUDED.icon8,is_raid=EXCLUDED.is_raid")
            .bind(group.group_id).bind(group.leader_guid).bind(group.main_tank_guid).bind(group.main_assistant_guid).bind(group.loot_method).bind(group.loot_threshold).bind(group.looter_guid).bind(group.icon1).bind(group.icon2).bind(group.icon3).bind(group.icon4).bind(group.icon5).bind(group.icon6).bind(group.icon7).bind(group.icon8).bind(group.is_raid).execute(&*self.pool).await.context("Failed to save PostgreSQL group")?;
        Ok(())
    }
    pub async fn add_member(&self, group_id: i64, member_guid: i64, subgroup: i16) -> Result<()> {
        sqlx::query("INSERT INTO characters.group_member (group_id, member_guid, assistant, subgroup) VALUES ($1,$2,0,$3)").bind(group_id).bind(member_guid).bind(subgroup).execute(&*self.pool).await.context("Failed to add PostgreSQL group member")?;
        Ok(())
    }
    pub async fn update_member(
        &self,
        group_id: i64,
        member_guid: i64,
        assistant: i16,
        subgroup: i16,
    ) -> Result<()> {
        sqlx::query("UPDATE characters.group_member SET assistant = $1, subgroup = $2 WHERE group_id = $3 AND member_guid = $4").bind(assistant).bind(subgroup).bind(group_id).bind(member_guid).execute(&*self.pool).await.context("Failed to update PostgreSQL group member")?;
        Ok(())
    }
    pub async fn remove_member(&self, group_id: i64, member_guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.group_member WHERE group_id = $1 AND member_guid = $2")
            .bind(group_id)
            .bind(member_guid)
            .execute(&*self.pool)
            .await
            .context("Failed to remove PostgreSQL group member")?;
        Ok(())
    }
    pub async fn save_instance(&self, instance: &PgGroupInstanceRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1,$2,$3) ON CONFLICT (leader_guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent").bind(instance.leader_guid).bind(instance.instance).bind(instance.permanent).execute(&*self.pool).await.context("Failed to save PostgreSQL group instance")?;
        Ok(())
    }
    pub async fn find_instances(&self, leader_guid: i64) -> Result<Vec<PgGroupInstanceRow>> {
        sqlx::query_as(
            "SELECT * FROM characters.group_instance WHERE leader_guid = $1 ORDER BY instance",
        )
        .bind(leader_guid)
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch PostgreSQL group instances")
    }
    pub async fn delete_group(&self, group_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.groups WHERE group_id = $1")
            .bind(group_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL group")?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildRow {
    pub guild_id: i64,
    pub name: String,
    pub leader_guid: i64,
    pub emblem_style: i32,
    pub emblem_color: i32,
    pub border_style: i32,
    pub border_color: i32,
    pub background_color: i32,
    pub info: String,
    pub motd: String,
    pub create_date: i64,
    pub bank_money: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildMemberRow {
    pub guild_id: i64,
    pub guid: i64,
    pub rank: i16,
    pub player_note: String,
    pub officer_note: String,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildRankRow {
    pub guild_id: i64,
    pub id: i64,
    pub name: String,
    pub rights: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildBankTabRow {
    pub guild_id: i64,
    pub tab_id: i16,
    pub name: String,
    pub icon: String,
    pub view_rank: i16,
    pub withdraw_rank: i16,
    pub deposit_rank: i16,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildEventLogRow {
    pub guild_id: i64,
    pub log_guid: i64,
    pub event_type: i16,
    pub player_guid1: i64,
    pub player_guid2: i64,
    pub new_rank: i16,
    pub timestamp: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgGuildMemberWithCharacterDataRow {
    pub guid: i64,
    pub rank: i16,
    pub player_note: String,
    pub officer_note: String,
    pub name: Option<String>,
    pub level: Option<i16>,
    pub class: Option<i16>,
    pub zone: Option<i64>,
    pub account: Option<i64>,
    pub logout_time: Option<chrono::DateTime<chrono::Utc>>,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCorpseRow {
    pub guid: i64,
    pub player_guid: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub map: i64,
    pub time: i64,
    pub corpse_type: i16,
    pub instance: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgBattlegroundDataRow {
    pub guid: i64,
    pub instance_id: i64,
    pub team: i16,
    pub join_x: f32,
    pub join_y: f32,
    pub join_z: f32,
    pub join_o: f32,
    pub join_map: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgTicketRow {
    pub ticket_id: i64,
    pub guid: i64,
    pub name: String,
    pub message: String,
    pub create_time: i64,
    pub map: i64,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub last_modified_time: i64,
    pub closed_by: i64,
    pub assigned_to: i64,
    pub comment: String,
    pub response: String,
    pub completed: bool,
    pub escalated: i16,
    pub viewed: bool,
    pub have_ticket: bool,
    pub ticket_type: i16,
    pub security_needed: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgSurveyRow {
    pub survey_id: i64,
    pub ticket_id: i64,
    pub main_survey: i16,
    pub overall_comment: String,
    pub response_time: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgPetitionRow {
    pub owner_guid: i64,
    pub petition_guid: i64,
    pub charter_guid: Option<i64>,
    pub name: String,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgPetitionSignatureRow {
    pub owner_guid: i64,
    pub petition_guid: i64,
    pub player_guid: i64,
    pub player_account: i64,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgHonorCpRow {
    pub guid: i64,
    pub victim_type: i16,
    pub victim_id: i64,
    pub cp: f32,
    pub date: i64,
    pub r#type: i16,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgHonorStoredRow {
    pub guid: i64,
    pub honor_rank_points: f32,
    pub honor_standing: i64,
    pub honor_highest_rank: i64,
    pub honor_last_week_hk: i64,
    pub honor_last_week_cp: f32,
    pub honor_stored_hk: i32,
    pub honor_stored_dk: i32,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgInstanceRow {
    pub id: i64,
    pub map: i64,
    pub reset_time: i64,
    pub data: Option<String>,
}
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct PgCharacterInstanceRow {
    pub guid: i64,
    pub instance: i64,
    pub permanent: i16,
    pub extend: i16,
}

#[derive(Clone)]
pub struct PgGuildRepository {
    pool: Arc<PgPool>,
}
impl PgGuildRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn find_by_id(&self, id: i64) -> Result<Option<PgGuildRow>> {
        sqlx::query_as("SELECT * FROM characters.guild WHERE guild_id=$1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL guild")
    }
    pub async fn find_by_name(&self, name: &str) -> Result<Option<PgGuildRow>> {
        sqlx::query_as("SELECT * FROM characters.guild WHERE name=$1")
            .bind(name)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL guild")
    }
    pub async fn exists_by_name(&self, name: &str) -> Result<bool> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM characters.guild WHERE name=$1)")
            .bind(name)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to check PostgreSQL guild name")
    }
    pub async fn get_max_guild_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(guild_id) FROM characters.guild")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max guild ID")
    }
    pub async fn find_members(&self, id: i64) -> Result<Vec<PgGuildMemberRow>> {
        sqlx::query_as("SELECT * FROM characters.guild_member WHERE guild_id=$1 ORDER BY guid")
            .bind(id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL guild members")
    }
    pub async fn find_character_data(
        &self,
        guid: i64,
    ) -> Result<Option<(i16, i16, i64, i64, i64)>> {
        sqlx::query_as("SELECT level, class, zone, account, EXTRACT(EPOCH FROM logout_time)::BIGINT FROM characters.characters WHERE guid=$1")
            .bind(guid).fetch_optional(&*self.pool).await.context("Failed to fetch PostgreSQL guild character data")
    }
    pub async fn find_members_with_character_data(
        &self,
        id: i64,
    ) -> Result<Vec<PgGuildMemberWithCharacterDataRow>> {
        sqlx::query_as("SELECT gm.guid,gm.rank,gm.player_note,gm.officer_note,c.name,c.level,c.class,c.zone,c.account,c.logout_time FROM characters.guild_member gm LEFT JOIN characters.characters c ON c.guid=gm.guid WHERE gm.guild_id=$1 ORDER BY gm.guid")
            .bind(id).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL guild members with character data")
    }
    pub async fn find_ranks(&self, id: i64) -> Result<Vec<PgGuildRankRow>> {
        sqlx::query_as("SELECT * FROM characters.guild_rank WHERE guild_id=$1 ORDER BY id")
            .bind(id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL guild ranks")
    }
    pub async fn find_bank_tabs(&self, id: i64) -> Result<Vec<PgGuildBankTabRow>> {
        sqlx::query_as("SELECT * FROM characters.guild_bank_tab WHERE guild_id=$1 ORDER BY tab_id")
            .bind(id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL guild bank tabs")
    }
    pub async fn find_event_logs(&self, id: i64, limit: i64) -> Result<Vec<PgGuildEventLogRow>> {
        sqlx::query_as("SELECT * FROM characters.guild_eventlog WHERE guild_id=$1 ORDER BY log_guid DESC LIMIT $2").bind(id).bind(limit).fetch_all(&*self.pool).await.context("Failed to fetch PostgreSQL guild event logs")
    }
    pub async fn create(
        &self,
        guild: &PgGuildRow,
        ranks: &[PgGuildRankRow],
        leader: &PgGuildMemberRow,
        tabs: &[PgGuildBankTabRow],
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL guild transaction")?;
        sqlx::query("INSERT INTO characters.guild VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
            .bind(guild.guild_id)
            .bind(&guild.name)
            .bind(guild.leader_guid)
            .bind(guild.emblem_style)
            .bind(guild.emblem_color)
            .bind(guild.border_style)
            .bind(guild.border_color)
            .bind(guild.background_color)
            .bind(&guild.info)
            .bind(&guild.motd)
            .bind(guild.create_date)
            .bind(guild.bank_money)
            .execute(&mut *tx)
            .await
            .context("Failed to create PostgreSQL guild")?;
        for rank in ranks {
            sqlx::query("INSERT INTO characters.guild_rank VALUES ($1,$2,$3,$4)")
                .bind(rank.guild_id)
                .bind(rank.id)
                .bind(&rank.name)
                .bind(rank.rights)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT INTO characters.guild_member VALUES ($1,$2,$3,$4,$5)")
            .bind(leader.guild_id)
            .bind(leader.guid)
            .bind(leader.rank)
            .bind(&leader.player_note)
            .bind(&leader.officer_note)
            .execute(&mut *tx)
            .await?;
        for tab in tabs {
            sqlx::query("INSERT INTO characters.guild_bank_tab VALUES ($1,$2,$3,$4,$5,$6,$7)")
                .bind(tab.guild_id)
                .bind(tab.tab_id)
                .bind(&tab.name)
                .bind(&tab.icon)
                .bind(tab.view_rank)
                .bind(tab.withdraw_rank)
                .bind(tab.deposit_rank)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL guild creation")
    }
    pub async fn update(&self, row: &PgGuildRow) -> Result<()> {
        sqlx::query("UPDATE characters.guild SET name=$1,leader_guid=$2,emblem_style=$3,emblem_color=$4,border_style=$5,border_color=$6,background_color=$7,info=$8,motd=$9,bank_money=$10 WHERE guild_id=$11").bind(&row.name).bind(row.leader_guid).bind(row.emblem_style).bind(row.emblem_color).bind(row.border_style).bind(row.border_color).bind(row.background_color).bind(&row.info).bind(&row.motd).bind(row.bank_money).bind(row.guild_id).execute(&*self.pool).await.context("Failed to update PostgreSQL guild").map(|_| ())
    }
    pub async fn update_motd(&self, id: i64, value: &str) -> Result<()> {
        self.update_text("motd", id, value).await
    }
    pub async fn update_info(&self, id: i64, value: &str) -> Result<()> {
        self.update_text("info", id, value).await
    }
    pub async fn update_guild_name(&self, id: i64, value: &str) -> Result<()> {
        self.update_text("name", id, value).await
    }
    async fn update_text(&self, column: &str, id: i64, value: &str) -> Result<()> {
        sqlx::query(&format!(
            "UPDATE characters.guild SET {column}=$1 WHERE guild_id=$2"
        ))
        .bind(value)
        .bind(id)
        .execute(&*self.pool)
        .await
        .context("Failed to update PostgreSQL guild text")
        .map(|_| ())
    }
    pub async fn update_bank_money(&self, id: i64, amount: i64) -> Result<()> {
        sqlx::query("UPDATE characters.guild SET bank_money=$1 WHERE guild_id=$2")
            .bind(amount)
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL guild bank money")
            .map(|_| ())
    }
    pub async fn update_emblem(
        &self,
        id: i64,
        emblem_style: i32,
        emblem_color: i32,
        border_style: i32,
        border_color: i32,
        background_color: i32,
    ) -> Result<()> {
        sqlx::query("UPDATE characters.guild SET emblem_style=$1,emblem_color=$2,border_style=$3,border_color=$4,background_color=$5 WHERE guild_id=$6").bind(emblem_style).bind(emblem_color).bind(border_style).bind(border_color).bind(background_color).bind(id).execute(&*self.pool).await.context("Failed to update PostgreSQL guild emblem").map(|_| ())
    }
    pub async fn update_leader(&self, id: i64, old_leader: i64, new_leader: i64) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL guild leader transaction")?;
        sqlx::query("UPDATE characters.guild SET leader_guid=$1 WHERE guild_id=$2")
            .bind(new_leader)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE characters.guild_member SET rank=0 WHERE guild_id=$1 AND guid=$2")
            .bind(id)
            .bind(new_leader)
            .execute(&mut *tx)
            .await?;
        if old_leader != new_leader {
            sqlx::query("UPDATE characters.guild_member SET rank=1 WHERE guild_id=$1 AND guid=$2")
                .bind(id)
                .bind(old_leader)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL guild leader change")
    }
    pub async fn add_member(&self, row: &PgGuildMemberRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.guild_member VALUES ($1,$2,$3,$4,$5)")
            .bind(row.guild_id)
            .bind(row.guid)
            .bind(row.rank)
            .bind(&row.player_note)
            .bind(&row.officer_note)
            .execute(&*self.pool)
            .await
            .context("Failed to add PostgreSQL guild member")
            .map(|_| ())
    }
    pub async fn update_member_rank(&self, id: i64, guid: i64, rank: i16) -> Result<()> {
        sqlx::query("UPDATE characters.guild_member SET rank=$1 WHERE guild_id=$2 AND guid=$3")
            .bind(rank)
            .bind(id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL guild member rank")
            .map(|_| ())
    }
    pub async fn update_member_public_note(&self, id: i64, guid: i64, note: &str) -> Result<()> {
        self.update_member_note("player_note", id, guid, note).await
    }
    pub async fn update_member_officer_note(&self, id: i64, guid: i64, note: &str) -> Result<()> {
        self.update_member_note("officer_note", id, guid, note)
            .await
    }
    async fn update_member_note(&self, column: &str, id: i64, guid: i64, note: &str) -> Result<()> {
        sqlx::query(&format!(
            "UPDATE characters.guild_member SET {column}=$1 WHERE guild_id=$2 AND guid=$3"
        ))
        .bind(note)
        .bind(id)
        .bind(guid)
        .execute(&*self.pool)
        .await
        .context("Failed to update PostgreSQL guild member note")
        .map(|_| ())
    }
    pub async fn remove_member(&self, id: i64, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.guild_member WHERE guild_id=$1 AND guid=$2")
            .bind(id)
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to remove PostgreSQL guild member")
            .map(|_| ())
    }
    pub async fn save_rank(&self, row: &PgGuildRankRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.guild_rank VALUES ($1,$2,$3,$4) ON CONFLICT (guild_id,id) DO UPDATE SET name=EXCLUDED.name,rights=EXCLUDED.rights").bind(row.guild_id).bind(row.id).bind(&row.name).bind(row.rights).execute(&*self.pool).await.context("Failed to save PostgreSQL guild rank").map(|_| ())
    }
    pub async fn delete_rank(&self, id: i64, rank: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.guild_rank WHERE guild_id=$1 AND id=$2")
            .bind(id)
            .bind(rank)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL guild rank")
            .map(|_| ())
    }
    pub async fn save_bank_tab(&self, row: &PgGuildBankTabRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.guild_bank_tab VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (guild_id,tab_id) DO UPDATE SET name=EXCLUDED.name,icon=EXCLUDED.icon,view_rank=EXCLUDED.view_rank,withdraw_rank=EXCLUDED.withdraw_rank,deposit_rank=EXCLUDED.deposit_rank").bind(row.guild_id).bind(row.tab_id).bind(&row.name).bind(&row.icon).bind(row.view_rank).bind(row.withdraw_rank).bind(row.deposit_rank).execute(&*self.pool).await.context("Failed to save PostgreSQL guild bank tab").map(|_| ())
    }
    pub async fn insert_event_log(&self, row: &PgGuildEventLogRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.guild_eventlog VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(row.guild_id)
            .bind(row.log_guid)
            .bind(row.event_type)
            .bind(row.player_guid1)
            .bind(row.player_guid2)
            .bind(row.new_rank)
            .bind(row.timestamp)
            .execute(&*self.pool)
            .await
            .context("Failed to add PostgreSQL guild event log")
            .map(|_| ())
    }
    pub async fn get_max_event_log_guid(&self, id: i64) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(log_guid) FROM characters.guild_eventlog WHERE guild_id=$1")
            .bind(id)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max guild event log ID")
    }
    pub async fn delete_old_event_logs(&self, id: i64, keep: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.guild_eventlog WHERE guild_id=$1 AND log_guid IN (SELECT log_guid FROM characters.guild_eventlog WHERE guild_id=$1 ORDER BY log_guid DESC OFFSET $2)").bind(id).bind(keep).execute(&*self.pool).await.context("Failed to delete old PostgreSQL guild event logs").map(|_| ())
    }
    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.guild WHERE guild_id=$1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL guild")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgCorpseRepository {
    pool: Arc<PgPool>,
}
impl PgCorpseRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn load_all(&self) -> Result<Vec<PgCorpseRow>> {
        sqlx::query_as("SELECT * FROM characters.corpse")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to load PostgreSQL corpses")
    }
    pub async fn find_for_player(&self, guid: i64) -> Result<Option<PgCorpseRow>> {
        sqlx::query_as("SELECT * FROM characters.corpse WHERE player_guid=$1 LIMIT 1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL corpse")
    }
    pub async fn save(&self, row: &PgCorpseRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.corpse VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (guid) DO UPDATE SET player_guid=EXCLUDED.player_guid,position_x=EXCLUDED.position_x,position_y=EXCLUDED.position_y,position_z=EXCLUDED.position_z,orientation=EXCLUDED.orientation,map=EXCLUDED.map,time=EXCLUDED.time,corpse_type=EXCLUDED.corpse_type,instance=EXCLUDED.instance").bind(row.guid).bind(row.player_guid).bind(row.position_x).bind(row.position_y).bind(row.position_z).bind(row.orientation).bind(row.map).bind(row.time).bind(row.corpse_type).bind(row.instance).execute(&*self.pool).await.context("Failed to save PostgreSQL corpse").map(|_| ())
    }
    pub async fn delete(&self, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.corpse WHERE guid=$1")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL corpse")
            .map(|_| ())
    }
    pub async fn delete_for_player(&self, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.corpse WHERE player_guid=$1")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL player corpses")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgBattlegroundRepository {
    pool: Arc<PgPool>,
}
impl PgBattlegroundRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn get_max_instance_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(instance_id) FROM characters.character_battleground_data")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max battleground instance")
    }
    pub async fn find_by_guid(&self, guid: i64) -> Result<Option<PgBattlegroundDataRow>> {
        sqlx::query_as("SELECT * FROM characters.character_battleground_data WHERE guid=$1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL battleground data")
    }
    pub async fn find_by_instance_id(&self, id: i64) -> Result<Vec<PgBattlegroundDataRow>> {
        sqlx::query_as("SELECT * FROM characters.character_battleground_data WHERE instance_id=$1")
            .bind(id)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL battleground data")
    }
    pub async fn save_player_data(&self, row: &PgBattlegroundDataRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_battleground_data VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (guid) DO UPDATE SET instance_id=EXCLUDED.instance_id,team=EXCLUDED.team,join_x=EXCLUDED.join_x,join_y=EXCLUDED.join_y,join_z=EXCLUDED.join_z,join_o=EXCLUDED.join_o,join_map=EXCLUDED.join_map").bind(row.guid).bind(row.instance_id).bind(row.team).bind(row.join_x).bind(row.join_y).bind(row.join_z).bind(row.join_o).bind(row.join_map).execute(&*self.pool).await.context("Failed to save PostgreSQL battleground data").map(|_| ())
    }
    pub async fn delete_player_data(&self, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_battleground_data WHERE guid=$1")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL battleground data")
            .map(|_| ())
    }
    pub async fn delete_instance_data(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_battleground_data WHERE instance_id=$1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL battleground instance data")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgTicketRepository {
    pool: Arc<PgPool>,
}
impl PgTicketRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn get_max_ticket_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(ticket_id) FROM characters.gm_tickets")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max ticket ID")
    }
    pub async fn get_max_survey_id(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(survey_id) FROM characters.gm_surveys")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max survey ID")
    }
    pub async fn find_by_id(&self, id: i64) -> Result<Option<PgTicketRow>> {
        sqlx::query_as("SELECT * FROM characters.gm_tickets WHERE ticket_id=$1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL ticket")
    }
    pub async fn find_open_tickets(&self) -> Result<Vec<PgTicketRow>> {
        sqlx::query_as("SELECT * FROM characters.gm_tickets WHERE closed_by=0")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch open PostgreSQL tickets")
    }
    pub async fn save_ticket(&self, row: &PgTicketRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.gm_tickets VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20) ON CONFLICT (ticket_id) DO UPDATE SET guid=EXCLUDED.guid,name=EXCLUDED.name,message=EXCLUDED.message,create_time=EXCLUDED.create_time,map=EXCLUDED.map,position_x=EXCLUDED.position_x,position_y=EXCLUDED.position_y,position_z=EXCLUDED.position_z,last_modified_time=EXCLUDED.last_modified_time,closed_by=EXCLUDED.closed_by,assigned_to=EXCLUDED.assigned_to,comment=EXCLUDED.comment,response=EXCLUDED.response,completed=EXCLUDED.completed,escalated=EXCLUDED.escalated,viewed=EXCLUDED.viewed,have_ticket=EXCLUDED.have_ticket,ticket_type=EXCLUDED.ticket_type,security_needed=EXCLUDED.security_needed").bind(row.ticket_id).bind(row.guid).bind(&row.name).bind(&row.message).bind(row.create_time).bind(row.map).bind(row.position_x).bind(row.position_y).bind(row.position_z).bind(row.last_modified_time).bind(row.closed_by).bind(row.assigned_to).bind(&row.comment).bind(&row.response).bind(row.completed).bind(row.escalated).bind(row.viewed).bind(row.have_ticket).bind(row.ticket_type).bind(row.security_needed).execute(&*self.pool).await.context("Failed to save PostgreSQL ticket").map(|_| ())
    }
    pub async fn close_ticket(&self, id: i64, closed_by: i64) -> Result<()> {
        sqlx::query("UPDATE characters.gm_tickets SET closed_by=$1 WHERE ticket_id=$2")
            .bind(closed_by)
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to close PostgreSQL ticket")
            .map(|_| ())
    }
    pub async fn delete_ticket(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.gm_tickets WHERE ticket_id=$1")
            .bind(id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL ticket")
            .map(|_| ())
    }
    pub async fn create_survey(&self, row: &PgSurveyRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.gm_surveys VALUES ($1,$2,$3,$4,$5)")
            .bind(row.survey_id)
            .bind(row.ticket_id)
            .bind(row.main_survey)
            .bind(&row.overall_comment)
            .bind(row.response_time)
            .execute(&*self.pool)
            .await
            .context("Failed to create PostgreSQL survey")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgPetitionRepository {
    pool: Arc<PgPool>,
}
impl PgPetitionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn find_by_charter_guid(&self, guid: i64) -> Result<Option<PgPetitionRow>> {
        sqlx::query_as("SELECT * FROM characters.petition WHERE charter_guid=$1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL petition")
    }
    pub async fn get_max_petition_guid(&self) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(petition_guid) FROM characters.petition")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max petition GUID")
    }
    pub async fn find_by_owner_guid(&self, guid: i64) -> Result<Option<PgPetitionRow>> {
        sqlx::query_as("SELECT * FROM characters.petition WHERE owner_guid=$1")
            .bind(guid)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL petition")
    }
    pub async fn find_signatures(&self, guid: i64) -> Result<Vec<PgPetitionSignatureRow>> {
        sqlx::query_as("SELECT * FROM characters.petition_sign WHERE petition_guid=$1")
            .bind(guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL petition signatures")
    }
    pub async fn save_petition(&self, row: &PgPetitionRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.petition VALUES ($1,$2,$3,$4) ON CONFLICT (owner_guid) DO UPDATE SET petition_guid=EXCLUDED.petition_guid,charter_guid=EXCLUDED.charter_guid,name=EXCLUDED.name").bind(row.owner_guid).bind(row.petition_guid).bind(row.charter_guid).bind(&row.name).execute(&*self.pool).await.context("Failed to save PostgreSQL petition").map(|_| ())
    }
    pub async fn add_signature(&self, row: &PgPetitionSignatureRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.petition_sign VALUES ($1,$2,$3,$4)")
            .bind(row.owner_guid)
            .bind(row.petition_guid)
            .bind(row.player_guid)
            .bind(row.player_account)
            .execute(&*self.pool)
            .await
            .context("Failed to add PostgreSQL petition signature")
            .map(|_| ())
    }
    pub async fn delete_petition(&self, charter: i64) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL petition transaction")?;
        if let Some(petition_guid) = sqlx::query_scalar::<_, i64>(
            "SELECT petition_guid FROM characters.petition WHERE charter_guid=$1",
        )
        .bind(charter)
        .fetch_optional(&mut *tx)
        .await?
        {
            sqlx::query("DELETE FROM characters.petition_sign WHERE petition_guid=$1")
                .bind(petition_guid)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM characters.petition WHERE charter_guid=$1")
            .bind(charter)
            .execute(&mut *tx)
            .await?;
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL petition deletion")
    }
    pub async fn update_petition_name(&self, charter: i64, name: &str) -> Result<()> {
        sqlx::query("UPDATE characters.petition SET name=$1 WHERE charter_guid=$2")
            .bind(name)
            .bind(charter)
            .execute(&*self.pool)
            .await
            .context("Failed to update PostgreSQL petition")
            .map(|_| ())
    }
    pub async fn delete_player_petitions(&self, owner: i64) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start PostgreSQL petition transaction")?;
        sqlx::query("DELETE FROM characters.petition_sign WHERE owner_guid=$1")
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM characters.petition WHERE owner_guid=$1")
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        tx.commit()
            .await
            .context("Failed to commit PostgreSQL player petition deletion")
    }
    pub async fn delete_player_signatures(&self, player: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.petition_sign WHERE player_guid=$1")
            .bind(player)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL player petition signatures")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgHonorRepository {
    pool: Arc<PgPool>,
}
impl PgHonorRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn find_honor_cp(&self, guid: i64) -> Result<Vec<PgHonorCpRow>> {
        sqlx::query_as("SELECT * FROM characters.character_honor_cp WHERE guid=$1")
            .bind(guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL honor CP")
    }
    pub async fn find_stored_data(&self, guid: i64) -> Result<Option<PgHonorStoredRow>> {
        sqlx::query_as("SELECT guid,honor_rank_points,honor_standing,honor_highest_rank,honor_last_week_hk,honor_last_week_cp,honor_stored_hk,honor_stored_dk FROM characters.characters WHERE guid=$1").bind(guid).fetch_optional(&*self.pool).await.context("Failed to fetch PostgreSQL honor state")
    }
    pub async fn save_honor_cp(&self, row: &PgHonorCpRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_honor_cp VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(row.guid)
            .bind(row.victim_type)
            .bind(row.victim_id)
            .bind(row.cp)
            .bind(row.date)
            .bind(row.r#type)
            .execute(&*self.pool)
            .await
            .context("Failed to save PostgreSQL honor CP")
            .map(|_| ())
    }
    pub async fn save_stored_data(&self, row: &PgHonorStoredRow) -> Result<()> {
        sqlx::query("UPDATE characters.characters SET honor_rank_points=$1,honor_standing=$2,honor_highest_rank=$3,honor_last_week_hk=$4,honor_last_week_cp=$5,honor_stored_hk=$6,honor_stored_dk=$7 WHERE guid=$8").bind(row.honor_rank_points).bind(row.honor_standing).bind(row.honor_highest_rank).bind(row.honor_last_week_hk).bind(row.honor_last_week_cp).bind(row.honor_stored_hk).bind(row.honor_stored_dk).bind(row.guid).execute(&*self.pool).await.context("Failed to save PostgreSQL honor state").map(|_| ())
    }
    pub async fn delete_honor_cp(&self, guid: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_honor_cp WHERE guid=$1")
            .bind(guid)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL honor CP")
            .map(|_| ())
    }
}

#[derive(Clone)]
pub struct PgInstanceRepository {
    pool: Arc<PgPool>,
}
impl PgInstanceRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub async fn find_by_id_and_map(&self, id: i64, map: i64) -> Result<Option<PgInstanceRow>> {
        sqlx::query_as("SELECT * FROM characters.instance WHERE id=$1 AND map=$2")
            .bind(id)
            .bind(map)
            .fetch_optional(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL instance")
    }
    pub async fn get_max_instance_id_by_map(&self, map: i64) -> Result<Option<i64>> {
        sqlx::query_scalar("SELECT MAX(id) FROM characters.instance WHERE map=$1")
            .bind(map)
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query PostgreSQL max instance ID")
    }
    pub async fn find_all(&self) -> Result<Vec<PgInstanceRow>> {
        sqlx::query_as("SELECT * FROM characters.instance")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL instances")
    }
    pub async fn find_character_instances(&self, guid: i64) -> Result<Vec<PgCharacterInstanceRow>> {
        sqlx::query_as("SELECT * FROM characters.character_instance WHERE guid=$1")
            .bind(guid)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch PostgreSQL character instances")
    }
    pub async fn save_instance(&self, row: &PgInstanceRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.instance VALUES ($1,$2,$3,$4) ON CONFLICT (id) DO UPDATE SET map=EXCLUDED.map,reset_time=EXCLUDED.reset_time,data=EXCLUDED.data").bind(row.id).bind(row.map).bind(row.reset_time).bind(&row.data).execute(&*self.pool).await.context("Failed to save PostgreSQL instance").map(|_| ())
    }
    pub async fn save_character_instance(&self, row: &PgCharacterInstanceRow) -> Result<()> {
        sqlx::query("INSERT INTO characters.character_instance VALUES ($1,$2,$3,$4) ON CONFLICT (guid,instance) DO UPDATE SET permanent=EXCLUDED.permanent,extend=EXCLUDED.extend").bind(row.guid).bind(row.instance).bind(row.permanent).bind(row.extend).execute(&*self.pool).await.context("Failed to save PostgreSQL character instance").map(|_| ())
    }
    pub async fn delete_instance(&self, id: i64, map: i64) -> Result<()> {
        sqlx::query("DELETE FROM characters.instance WHERE id=$1 AND map=$2")
            .bind(id)
            .bind(map)
            .execute(&*self.pool)
            .await
            .context("Failed to delete PostgreSQL instance")
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn repositories_accept_a_postgres_pool_without_connecting() {
        let pool = Arc::new(PgPool::connect_lazy("postgres://localhost/characters_test").unwrap());
        let _ = PgCharacterRepository::new(Arc::clone(&pool));
        let _ = PgInventoryRepository::new(Arc::clone(&pool));
        let _ = PgQuestRepository::new(Arc::clone(&pool));
        let _ = PgReputationRepository::new(Arc::clone(&pool));
        let _ = PgMailRepository::new(Arc::clone(&pool));
        let _ = PgAuctionRepository::new(Arc::clone(&pool));
        let _ = PgSocialRepository::new(Arc::clone(&pool));
        let _ = PgGroupRepository::new(pool);
    }

    #[tokio::test]
    async fn round_trips_core_state_when_postgres_is_configured() -> Result<()> {
        let Ok(url) = std::env::var("OXCORE_PG_TEST_URL") else {
            return Ok(());
        };
        let pool = Arc::new(PgPool::connect(&url).await?);
        let character = PgCharacterRepository::new(Arc::clone(&pool));
        let inventory = PgInventoryRepository::new(Arc::clone(&pool));
        let quests = PgQuestRepository::new(Arc::clone(&pool));
        let reputations = PgReputationRepository::new(Arc::clone(&pool));
        let guid = 8_000_000_001i64;
        let item_guid = 8_000_000_002i64;

        sqlx::query("DELETE FROM characters.item_instance WHERE guid = $1")
            .bind(item_guid)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.characters WHERE guid = $1")
            .bind(guid)
            .execute(&*pool)
            .await?;

        character
            .create(&PgCharacterCreate {
                guid,
                account: 1,
                name: "PgSlice",
                race: 1,
                class: 1,
                gender: 0,
                skin: 0,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_hair: 0,
                map: 0,
                zone: 1,
                position_x: 1.0,
                position_y: 2.0,
                position_z: 3.0,
                orientation: 4.0,
                health: 100,
                power1: 50,
                money: 25,
            })
            .await?;
        assert!(character.exists_by_name("PgSlice").await?);
        assert_eq!(
            character
                .find_by_account(1)
                .await?
                .iter()
                .filter(|row| row.guid == guid)
                .count(),
            1
        );

        inventory
            .create_item(
                &PgItemInstanceRow {
                    guid: item_guid,
                    item_id: 6948,
                    owner_guid: guid,
                    creator_guid: guid,
                    gift_creator_guid: 0,
                    count: 1,
                    duration: 0,
                    charges: None,
                    flags: 0,
                    enchantments: String::new(),
                    random_property_id: 0,
                    durability: 0,
                    text: 0,
                    generated_loot: false,
                },
                &PgInventoryRow {
                    guid,
                    bag: 0,
                    slot: 23,
                    item_guid,
                    item_id: 6948,
                },
            )
            .await?;
        inventory.update_item_count(item_guid, 2).await?;
        assert_eq!(inventory.load(guid).await?.len(), 1);
        assert_eq!(inventory.find_item(item_guid).await?.unwrap().count, 2);

        let quest = PgQuestStatusRow {
            guid,
            quest: 1,
            status: 1,
            rewarded: false,
            explored: true,
            timer: 0,
            mob_count1: 1,
            mob_count2: 0,
            mob_count3: 0,
            mob_count4: 0,
            item_count1: 0,
            item_count2: 0,
            item_count3: 0,
            item_count4: 0,
            reward_choice: 0,
        };
        quests.save(&quest).await?;
        assert_eq!(quests.load(guid).await?, vec![quest]);

        let reputation = PgReputationRow {
            guid,
            faction: 1,
            standing: 10,
            flags: 1,
        };
        reputations.save(&reputation).await?;
        assert_eq!(reputations.load(guid).await?, vec![reputation]);

        let mail = PgMailRepository::new(Arc::clone(&pool));
        let auctions = PgAuctionRepository::new(Arc::clone(&pool));
        let social = PgSocialRepository::new(Arc::clone(&pool));
        let groups = PgGroupRepository::new(Arc::clone(&pool));

        let text_id = mail.create_item_text("PostgreSQL mail body").await?;
        assert_eq!(
            mail.find_item_text(text_id).await?.unwrap().text.as_deref(),
            Some("PostgreSQL mail body")
        );
        let mail_id = mail
            .create(&PgMailRow {
                id: 0,
                message_type: 0,
                stationery: 41,
                mail_template_id: 0,
                sender_guid: guid,
                receiver_guid: guid,
                subject: Some("PostgreSQL mail".to_owned()),
                item_text_id: text_id,
                has_items: 0,
                expire_time: 9_000_000_000,
                deliver_time: 0,
                money: 25,
                cod: 0,
                checked: 0,
            })
            .await?;
        mail.add_item(&PgMailItemRow {
            mail_id,
            item_guid,
            item_id: 6948,
            receiver_guid: guid,
        })
        .await?;
        mail.update_has_items(mail_id, guid, 1).await?;
        assert_eq!(mail.find_mail_items(mail_id).await?.len(), 1);
        mail.clear_money(mail_id, guid).await?;
        assert_eq!(mail.find_by_id(mail_id, guid).await?.unwrap().money, 0);

        let auction = PgAuctionRow {
            id: 8_000_000_003,
            house_id: 1,
            item_guid,
            item_id: 6948,
            seller_guid: guid,
            buyout_price: 100,
            expire_time: 9_000_000_000,
            buyer_guid: 0,
            last_bid: 0,
            start_bid: 10,
            deposit: 1,
        };
        auctions.create_auction(&auction).await?;
        auctions.update_bid(auction.id, guid, 15).await?;
        assert_eq!(auctions.find_by_id(auction.id).await?.unwrap().last_bid, 15);
        assert_eq!(
            auctions
                .find_active_by_house_with_account(1, 0)
                .await?
                .len(),
            1
        );

        social.add_or_update(guid, guid, 1).await?;
        social.add_flags(guid, guid, 2).await?;
        social.remove_flags(guid, guid, 1).await?;
        assert_eq!(social.find_by_guid(guid).await?.pop().unwrap().flags, 2);

        let group = PgGroupRow {
            group_id: 8_000_000_004,
            leader_guid: guid,
            main_tank_guid: 0,
            main_assistant_guid: 0,
            loot_method: 0,
            loot_threshold: 0,
            looter_guid: 0,
            icon1: 0,
            icon2: 0,
            icon3: 0,
            icon4: 0,
            icon5: 0,
            icon6: 0,
            icon7: 0,
            icon8: 0,
            is_raid: 0,
        };
        groups.save_group(&group).await?;
        groups.add_member(group.group_id, guid, 0).await?;
        groups.update_member(group.group_id, guid, 1, 1).await?;
        // group_instance references a persisted instance in the PostgreSQL schema.
        sqlx::query("INSERT INTO characters.instance (id, map, reset_time) VALUES ($1, $2, $3)")
            .bind(1i64)
            .bind(0i64)
            .bind(0i64)
            .execute(&*pool)
            .await?;
        groups
            .save_instance(&PgGroupInstanceRow {
                leader_guid: guid,
                instance: 1,
                permanent: 1,
            })
            .await?;
        assert_eq!(
            groups
                .find_members(group.group_id)
                .await?
                .pop()
                .unwrap()
                .assistant,
            1
        );
        assert_eq!(groups.find_instances(guid).await?.len(), 1);

        groups.delete_group(group.group_id).await?;
        sqlx::query("DELETE FROM characters.group_instance WHERE leader_guid = $1")
            .bind(guid)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.instance WHERE id = $1")
            .bind(1i64)
            .execute(&*pool)
            .await?;
        social.delete_all_for_character(guid).await?;
        auctions.delete_auction(auction.id).await?;
        mail.delete(mail_id).await?;
        sqlx::query("DELETE FROM characters.item_text WHERE id = $1")
            .bind(text_id)
            .execute(&*pool)
            .await?;

        sqlx::query("DELETE FROM characters.characters WHERE guid = $1")
            .bind(guid)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.item_instance WHERE guid = $1")
            .bind(item_guid)
            .execute(&*pool)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn round_trips_remaining_character_persistence_when_postgres_is_configured() -> Result<()>
    {
        let Ok(url) = std::env::var("OXCORE_PG_TEST_URL") else {
            return Ok(());
        };
        let pool = Arc::new(PgPool::connect(&url).await?);
        let guid = 8_000_000_101i64;
        let guild_id = 8_000_000_102i64;
        let instance_id = 8_000_000_103i64;
        let ticket_id = 8_000_000_104i64;
        let petition_guid = 8_000_000_105i64;
        let charter_guid = 8_000_000_106i64;

        sqlx::query("DELETE FROM characters.gm_surveys WHERE survey_id=$1")
            .bind(ticket_id)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.gm_tickets WHERE ticket_id=$1")
            .bind(ticket_id)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.characters WHERE guid=$1")
            .bind(guid)
            .execute(&*pool)
            .await?;
        PgCharacterRepository::new(Arc::clone(&pool))
            .create(&PgCharacterCreate {
                guid,
                account: 1,
                name: "PgRemaining",
                race: 1,
                class: 1,
                gender: 0,
                skin: 0,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_hair: 0,
                map: 0,
                zone: 1,
                position_x: 0.0,
                position_y: 0.0,
                position_z: 0.0,
                orientation: 0.0,
                health: 100,
                power1: 100,
                money: 0,
            })
            .await?;

        let guilds = PgGuildRepository::new(Arc::clone(&pool));
        let guild = PgGuildRow {
            guild_id,
            name: "PgGuild".into(),
            leader_guid: guid,
            emblem_style: -1,
            emblem_color: 2,
            border_style: 3,
            border_color: 4,
            background_color: 5,
            info: "info".into(),
            motd: "motd".into(),
            create_date: 1,
            bank_money: 2,
        };
        let rank = PgGuildRankRow {
            guild_id,
            id: 0,
            name: "Guild Master".into(),
            rights: 1,
        };
        let member = PgGuildMemberRow {
            guild_id,
            guid,
            rank: 0,
            player_note: "public".into(),
            officer_note: "officer".into(),
        };
        let tab = PgGuildBankTabRow {
            guild_id,
            tab_id: 0,
            name: "Bank".into(),
            icon: "INV_Misc_Bag_10".into(),
            view_rank: 0,
            withdraw_rank: 0,
            deposit_rank: 0,
        };
        guilds
            .create(&guild, &[rank.clone()], &member, &[tab.clone()])
            .await?;
        guilds
            .insert_event_log(&PgGuildEventLogRow {
                guild_id,
                log_guid: 1,
                event_type: -1,
                player_guid1: guid,
                player_guid2: 0,
                new_rank: 0,
                timestamp: 1,
            })
            .await?;
        assert_eq!(guilds.find_ranks(guild_id).await?, vec![rank]);
        assert_eq!(guilds.find_bank_tabs(guild_id).await?, vec![tab]);
        assert_eq!(guilds.find_event_logs(guild_id, 1).await?.len(), 1);

        let corpses = PgCorpseRepository::new(Arc::clone(&pool));
        let corpse = PgCorpseRow {
            guid: guild_id,
            player_guid: guid,
            position_x: 1.0,
            position_y: 2.0,
            position_z: 3.0,
            orientation: 4.0,
            map: 1,
            time: 1,
            corpse_type: 1,
            instance: 0,
        };
        corpses.save(&corpse).await?;
        assert_eq!(corpses.find_for_player(guid).await?, Some(corpse));

        let battlegrounds = PgBattlegroundRepository::new(Arc::clone(&pool));
        let battleground = PgBattlegroundDataRow {
            guid,
            instance_id,
            team: 1,
            join_x: 1.0,
            join_y: 2.0,
            join_z: 3.0,
            join_o: 4.0,
            join_map: 1,
        };
        battlegrounds.save_player_data(&battleground).await?;
        assert_eq!(battlegrounds.find_by_guid(guid).await?, Some(battleground));

        let tickets = PgTicketRepository::new(Arc::clone(&pool));
        let ticket = PgTicketRow {
            ticket_id,
            guid,
            name: "PgRemaining".into(),
            message: "help".into(),
            create_time: 1,
            map: 1,
            position_x: 1.0,
            position_y: 2.0,
            position_z: 3.0,
            last_modified_time: 2,
            closed_by: 0,
            assigned_to: 0,
            comment: String::new(),
            response: String::new(),
            completed: false,
            escalated: 0,
            viewed: false,
            have_ticket: true,
            ticket_type: 1,
            security_needed: 0,
        };
        tickets.save_ticket(&ticket).await?;
        tickets
            .create_survey(&PgSurveyRow {
                survey_id: ticket_id,
                ticket_id,
                main_survey: 1,
                overall_comment: "fine".into(),
                response_time: 3,
            })
            .await?;
        assert_eq!(tickets.find_by_id(ticket_id).await?, Some(ticket));

        let petitions = PgPetitionRepository::new(Arc::clone(&pool));
        let petition = PgPetitionRow {
            owner_guid: guid,
            petition_guid,
            charter_guid: Some(charter_guid),
            name: "Pg Petition".into(),
        };
        let signature = PgPetitionSignatureRow {
            owner_guid: guid,
            petition_guid,
            player_guid: guid,
            player_account: 1,
        };
        petitions.save_petition(&petition).await?;
        petitions.add_signature(&signature).await?;
        assert_eq!(
            petitions.find_by_charter_guid(charter_guid).await?,
            Some(petition)
        );
        assert_eq!(
            petitions.find_signatures(petition_guid).await?,
            vec![signature]
        );

        let honor = PgHonorRepository::new(Arc::clone(&pool));
        let cp = PgHonorCpRow {
            guid,
            victim_type: 4,
            victim_id: 1,
            cp: 1.0,
            date: 1,
            r#type: 0,
        };
        let state = PgHonorStoredRow {
            guid,
            honor_rank_points: 1.0,
            honor_standing: 2,
            honor_highest_rank: 3,
            honor_last_week_hk: 4,
            honor_last_week_cp: 5.0,
            honor_stored_hk: 6,
            honor_stored_dk: 7,
        };
        honor.save_honor_cp(&cp).await?;
        honor.save_stored_data(&state).await?;
        assert_eq!(honor.find_honor_cp(guid).await?, vec![cp]);
        assert_eq!(honor.find_stored_data(guid).await?, Some(state));

        let instances = PgInstanceRepository::new(Arc::clone(&pool));
        let instance = PgInstanceRow {
            id: instance_id,
            map: 1,
            reset_time: 2,
            data: Some("state".into()),
        };
        let binding = PgCharacterInstanceRow {
            guid,
            instance: instance_id,
            permanent: 1,
            extend: 0,
        };
        instances.save_instance(&instance).await?;
        instances.save_character_instance(&binding).await?;
        assert_eq!(
            instances.find_by_id_and_map(instance_id, 1).await?,
            Some(instance)
        );
        assert_eq!(
            instances.find_character_instances(guid).await?,
            vec![binding]
        );

        guilds.delete(guild_id).await?;
        instances.delete_instance(instance_id, 1).await?;
        sqlx::query("DELETE FROM characters.gm_surveys WHERE survey_id=$1")
            .bind(ticket_id)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.gm_tickets WHERE ticket_id=$1")
            .bind(ticket_id)
            .execute(&*pool)
            .await?;
        sqlx::query("DELETE FROM characters.characters WHERE guid=$1")
            .bind(guid)
            .execute(&*pool)
            .await?;
        Ok(())
    }
}
