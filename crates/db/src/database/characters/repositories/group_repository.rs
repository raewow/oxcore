use super::super::models::group::*;
use super::group_repository_trait::GroupRepositoryTrait;
use crate::database::characters::{PgGroupMemberRow, PgGroupRepository, PgGroupRow};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

pub struct GroupRepository {
    pool: Arc<PgPool>,
}

impl GroupRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    fn pg(&self) -> PgGroupRepository {
        PgGroupRepository::new(Arc::clone(&self.pool))
    }

    fn group(row: PgGroupRow) -> Result<GroupRow> {
        Ok(GroupRow {
            group_id: row.group_id.try_into()?,
            leader_guid: row.leader_guid.try_into()?,
            main_tank_guid: row.main_tank_guid.try_into()?,
            main_assistant_guid: row.main_assistant_guid.try_into()?,
            loot_method: row.loot_method.try_into()?,
            loot_threshold: row.loot_threshold.try_into()?,
            looter_guid: row.looter_guid.try_into()?,
            icon1: row.icon1.try_into()?,
            icon2: row.icon2.try_into()?,
            icon3: row.icon3.try_into()?,
            icon4: row.icon4.try_into()?,
            icon5: row.icon5.try_into()?,
            icon6: row.icon6.try_into()?,
            icon7: row.icon7.try_into()?,
            icon8: row.icon8.try_into()?,
            is_raid: row.is_raid.try_into()?,
        })
    }

    fn member(row: PgGroupMemberRow) -> Result<GroupMemberRow> {
        Ok(GroupMemberRow {
            group_id: row.group_id.try_into()?,
            member_guid: row.member_guid.try_into()?,
            assistant: row.assistant.try_into()?,
            subgroup: row.subgroup.try_into()?,
        })
    }
}

#[async_trait]
impl GroupRepositoryTrait for GroupRepository {
    async fn get_max_group_id(&self) -> Result<Option<u32>> {
        self.pg()
            .get_max_group_id()
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    async fn find_by_id(&self, id: u32) -> Result<Option<GroupRow>> {
        self.pg()
            .find_by_id(id.into())
            .await?
            .map(Self::group)
            .transpose()
    }
    async fn find_all(&self) -> Result<Vec<GroupRow>> {
        self.pg()
            .find_all()
            .await?
            .into_iter()
            .map(Self::group)
            .collect()
    }
    async fn find_members(&self, id: u32) -> Result<Vec<GroupMemberRow>> {
        self.pg()
            .find_members(id.into())
            .await?
            .into_iter()
            .map(Self::member)
            .collect()
    }
    async fn find_group_for_member(&self, guid: u32) -> Result<Option<u32>> {
        self.pg()
            .find_group_for_member(guid.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    async fn find_members_with_character_data(
        &self,
        id: u32,
    ) -> Result<Vec<GroupMemberWithCharacterDataRow>> {
        self.pg()
            .find_members_with_character_data(id.into())
            .await?
            .into_iter()
            .map(|row| {
                Ok(GroupMemberWithCharacterDataRow {
                    member_guid: row.member_guid.try_into()?,
                    assistant: row.assistant.try_into()?,
                    subgroup: row.subgroup.try_into()?,
                    name: row.name,
                    level: row.level.map(TryInto::try_into).transpose()?,
                    class: row.class.map(TryInto::try_into).transpose()?,
                    zone: row.zone.map(TryInto::try_into).transpose()?,
                    online: row.online.map(|value| u8::from(value)).into(),
                })
            })
            .collect()
    }
    async fn save_group(&self, row: &GroupRow) -> Result<()> {
        self.pg()
            .save_group(&PgGroupRow {
                group_id: row.group_id.into(),
                leader_guid: row.leader_guid.into(),
                main_tank_guid: row.main_tank_guid.into(),
                main_assistant_guid: row.main_assistant_guid.into(),
                loot_method: row.loot_method.into(),
                loot_threshold: row.loot_threshold.into(),
                looter_guid: row.looter_guid.into(),
                icon1: row.icon1.into(),
                icon2: row.icon2.into(),
                icon3: row.icon3.into(),
                icon4: row.icon4.into(),
                icon5: row.icon5.into(),
                icon6: row.icon6.into(),
                icon7: row.icon7.into(),
                icon8: row.icon8.into(),
                is_raid: row.is_raid.into(),
            })
            .await
    }
    async fn add_member(&self, group_id: u32, member_guid: u32, subgroup: u16) -> Result<()> {
        self.pg()
            .add_member(group_id.into(), member_guid.into(), subgroup.try_into()?)
            .await
    }
    async fn update_member(
        &self,
        group_id: u32,
        member_guid: u32,
        assistant: bool,
        subgroup: u16,
    ) -> Result<()> {
        self.pg()
            .update_member(
                group_id.into(),
                member_guid.into(),
                i16::from(assistant),
                subgroup.try_into()?,
            )
            .await
    }
    async fn remove_member(&self, group_id: u32, member_guid: u32) -> Result<()> {
        self.pg()
            .remove_member(group_id.into(), member_guid.into())
            .await
    }
    async fn delete_group(&self, group_id: u32) -> Result<()> {
        self.pg().delete_group(group_id.into()).await
    }
}
