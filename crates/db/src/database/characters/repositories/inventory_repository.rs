use super::super::models::item::*;
use super::super::{PgInventoryRepository, PgInventoryRow, PgItemInstanceRow};
use super::inventory_repository_trait::{InventoryRepositoryTrait, InventorySlotRow};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct InventoryRepository {
    pool: Arc<PgPool>,
}
impl InventoryRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgInventoryRepository {
        PgInventoryRepository::new(Arc::clone(&self.pool))
    }
    fn item(row: PgItemInstanceRow) -> Result<ItemInstanceRow> {
        Ok(ItemInstanceRow {
            guid: row.guid.try_into()?,
            item_id: row.item_id.try_into()?,
            owner_guid: row.owner_guid.try_into()?,
            creator_guid: row.creator_guid.try_into()?,
            gift_creator_guid: row.gift_creator_guid.try_into()?,
            count: row.count.try_into()?,
            duration: row.duration,
            charges: row.charges,
            flags: row.flags.try_into()?,
            enchantments: row.enchantments,
            random_property_id: row.random_property_id,
            durability: row.durability.try_into()?,
            text: row.text.try_into()?,
            generated_loot: row.generated_loot.then_some(1),
        })
    }
    fn item_dto(row: &ItemInstanceRow) -> PgItemInstanceRow {
        PgItemInstanceRow {
            guid: row.guid.into(),
            item_id: row.item_id.into(),
            owner_guid: row.owner_guid.into(),
            creator_guid: row.creator_guid.into(),
            gift_creator_guid: row.gift_creator_guid.into(),
            count: row.count.into(),
            duration: row.duration,
            charges: row.charges.clone(),
            flags: row.flags.into(),
            enchantments: row.enchantments.clone(),
            random_property_id: row.random_property_id,
            durability: row.durability.into(),
            text: row.text.into(),
            generated_loot: row.generated_loot.unwrap_or_default() != 0,
        }
    }
}
#[async_trait]
impl InventoryRepositoryTrait for InventoryRepository {
    async fn load_player_inventory(&self, guid: u32) -> Result<Vec<InventorySlotRow>> {
        self.pg()
            .load(guid.into())
            .await?
            .into_iter()
            .map(|r| {
                Ok(InventorySlotRow {
                    guid: r.guid.try_into()?,
                    bag: r.bag.try_into()?,
                    slot: r.slot.try_into()?,
                    item_guid: r.item_guid.try_into()?,
                })
            })
            .collect()
    }
    async fn get_player_money(&self, g: u32) -> Result<u32> {
        Ok(self.pg().player_money(g.into()).await?.try_into()?)
    }
    async fn find_item(&self, g: u32) -> Result<Option<ItemInstanceRow>> {
        self.pg()
            .find_item(g.into())
            .await?
            .map(Self::item)
            .transpose()
    }
    async fn find_items_by_owner(&self, g: u32) -> Result<Vec<ItemInstanceRow>> {
        self.pg()
            .find_items_by_owner(g.into())
            .await?
            .into_iter()
            .map(Self::item)
            .collect()
    }
    async fn create_item(&self, item: &ItemInstanceRow, slot: &InventorySlotRow) -> Result<()> {
        self.pg()
            .create_item(
                &Self::item_dto(item),
                &PgInventoryRow {
                    guid: slot.guid.into(),
                    bag: slot.bag.into(),
                    slot: slot.slot.into(),
                    item_guid: slot.item_guid.into(),
                    item_id: item.item_id.into(),
                },
            )
            .await
    }
    async fn update_item_count(&self, g: u32, c: u32) -> Result<()> {
        self.pg().update_item_count(g.into(), c.into()).await
    }
    async fn move_item(&self, g: u32, i: u32, b: u8, s: u8) -> Result<()> {
        self.pg()
            .move_item(g.into(), i.into(), b.into(), s.into())
            .await
    }
    async fn swap_items(
        &self,
        g: u32,
        i1: u32,
        b1: u8,
        s1: u8,
        i2: Option<u32>,
        b2: u8,
        s2: u8,
    ) -> Result<()> {
        self.pg()
            .move_item(g.into(), i1.into(), b2.into(), s2.into())
            .await?;
        if let Some(i2) = i2 {
            self.pg()
                .move_item(g.into(), i2.into(), b1.into(), s1.into())
                .await?
        }
        Ok(())
    }
    async fn delete_item(&self, g: u32) -> Result<()> {
        self.pg().delete_item(g.into(), false).await
    }
    async fn delete_item_all(&self, g: u32) -> Result<()> {
        self.pg().delete_item(g.into(), true).await
    }
    async fn remove_from_slot(&self, g: u32, b: u8, s: u8) -> Result<()> {
        self.pg()
            .remove_from_slot(g.into(), b.into(), s.into())
            .await
    }
    async fn add_to_slot(&self, g: u32, i: u32, b: u8, s: u8) -> Result<()> {
        self.pg()
            .add_to_slot(g.into(), i.into(), b.into(), s.into())
            .await
    }
    async fn update_player_money(&self, g: u32, m: u32) -> Result<()> {
        self.pg().update_player_money(g.into(), m.into()).await
    }
    async fn update_item_owner(&self, g: u32, o: u32) -> Result<()> {
        self.pg().update_item_owner(g.into(), o.into()).await
    }
    async fn update_item_durability(&self, g: u32, d: u16) -> Result<()> {
        self.pg().update_item_durability(g.into(), d.into()).await
    }
    async fn update_item_enchantments(&self, g: u32, v: &str) -> Result<()> {
        self.pg().update_item_enchantments(g.into(), v).await
    }
    async fn update_item_flags(&self, g: u32, v: u32) -> Result<()> {
        self.pg().update_item_flags(g.into(), v.into()).await
    }
    async fn update_item_charges(&self, g: u32, v: &str) -> Result<()> {
        self.pg().update_item_charges(g.into(), v).await
    }
    async fn batch_move_items(&self, g: u32, v: &[(u32, u8, u8)]) -> Result<()> {
        for &(i, b, s) in v {
            self.pg()
                .move_item(g.into(), i.into(), b.into(), s.into())
                .await?
        }
        Ok(())
    }
    async fn batch_update_counts(&self, v: &[(u32, u32)]) -> Result<()> {
        for &(g, c) in v {
            self.pg().update_item_count(g.into(), c.into()).await?
        }
        Ok(())
    }
    async fn update_item_duration(&self, g: u32, d: u32) -> Result<()> {
        self.pg().update_item_duration(g.into(), d.into()).await
    }
    async fn batch_update_durability(&self, v: &[(u32, u16)]) -> Result<()> {
        for &(g, d) in v {
            self.pg().update_item_durability(g.into(), d.into()).await?
        }
        Ok(())
    }
    async fn get_next_item_guid(&self) -> Result<u32> {
        Ok(self
            .pg()
            .next_item_guid()
            .await?
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("item GUID overflow"))?
            .try_into()?)
    }
    async fn reserve_item_guids(&self, _: u32) -> Result<u32> {
        self.get_next_item_guid().await
    }
}
