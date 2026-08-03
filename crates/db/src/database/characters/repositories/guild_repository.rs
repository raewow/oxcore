use super::super::models::guild::*;
use super::super::{
    PgGuildBankTabRow, PgGuildEventLogRow, PgGuildMemberRow, PgGuildRankRow, PgGuildRepository,
    PgGuildRow,
};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait GuildRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, guild_id: u32) -> Result<Option<GuildRow>>;
    async fn find_by_name(&self, name: &str) -> Result<Option<GuildRow>>;
    async fn exists_by_name(&self, name: &str) -> Result<bool>;
    async fn get_max_guild_id(&self) -> Result<Option<u32>>;
    async fn find_members(&self, guild_id: u32) -> Result<Vec<GuildMemberRow>>;
    async fn find_members_with_character_data(
        &self,
        guild_id: u32,
    ) -> Result<Vec<GuildMemberWithCharacterDataRow>>;
    async fn find_ranks(&self, guild_id: u32) -> Result<Vec<GuildRankRow>>;
    async fn find_bank_tabs(&self, guild_id: u32) -> Result<Vec<GuildBankTabRow>>;
    async fn find_event_logs(&self, guild_id: u32, limit: u32) -> Result<Vec<GuildEventLogRow>>;
    async fn find_character_data(&self, guid: u32) -> Result<Option<(u8, u8, u32, u32, i64)>>;
    async fn create(
        &self,
        guild: &GuildRow,
        ranks: &[GuildRankRow],
        leader_member: &GuildMemberRow,
        bank_tabs: &[GuildBankTabRow],
    ) -> Result<()>;
    async fn update(&self, guild: &GuildRow) -> Result<()>;
    async fn update_motd(&self, guild_id: u32, motd: &str) -> Result<()>;
    async fn update_info(&self, guild_id: u32, info: &str) -> Result<()>;
    async fn update_guild_name(&self, guild_id: u32, name: &str) -> Result<()>;
    async fn update_emblem(
        &self,
        guild_id: u32,
        emblem_style: i32,
        emblem_color: i32,
        border_style: i32,
        border_color: i32,
        background_color: i32,
    ) -> Result<()>;
    async fn update_bank_money(&self, guild_id: u32, amount: u32) -> Result<()>;
    async fn update_leader(
        &self,
        guild_id: u32,
        old_leader_guid: u32,
        new_leader_guid: u32,
    ) -> Result<()>;
    async fn add_member(&self, member: &GuildMemberRow) -> Result<()>;
    async fn remove_member(&self, guild_id: u32, guid: u32) -> Result<()>;
    async fn update_member_rank(&self, guild_id: u32, guid: u32, rank: u8) -> Result<()>;
    async fn update_member_public_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()>;
    async fn update_member_officer_note(&self, guild_id: u32, guid: u32, note: &str) -> Result<()>;
    async fn create_rank(&self, rank: &GuildRankRow) -> Result<()>;
    async fn update_rank(&self, guild_id: u32, rank_id: u32, name: &str, rights: u32)
        -> Result<()>;
    async fn delete_rank(&self, guild_id: u32, rank_id: u32) -> Result<()>;
    async fn update_bank_tab(
        &self,
        guild_id: u32,
        tab_id: u8,
        name: &str,
        icon: &str,
        view_rank: u8,
        withdraw_rank: u8,
        deposit_rank: u8,
    ) -> Result<()>;
    async fn insert_event_log(&self, log: &GuildEventLogRow) -> Result<()>;
    async fn get_max_event_log_guid(&self, guild_id: u32) -> Result<Option<i32>>;
    async fn delete_old_event_logs(&self, guild_id: u32, keep_count: u32) -> Result<()>;
    async fn delete(&self, guild_id: u32) -> Result<()>;
}
pub struct GuildRepository {
    pool: Arc<PgPool>,
}
impl GuildRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgGuildRepository {
        PgGuildRepository::new(Arc::clone(&self.pool))
    }
    fn guild(row: super::super::PgGuildRow) -> Result<GuildRow> {
        Ok(GuildRow {
            guild_id: row.guild_id.try_into()?,
            name: row.name,
            leader_guid: row.leader_guid.try_into()?,
            emblem_style: row.emblem_style,
            emblem_color: row.emblem_color,
            border_style: row.border_style,
            border_color: row.border_color,
            background_color: row.background_color,
            info: row.info,
            motd: row.motd,
            create_date: row.create_date,
            bank_money: row.bank_money.try_into()?,
        })
    }
    fn guild_dto(row: &GuildRow) -> PgGuildRow {
        PgGuildRow {
            guild_id: row.guild_id.into(),
            name: row.name.clone(),
            leader_guid: row.leader_guid.into(),
            emblem_style: row.emblem_style,
            emblem_color: row.emblem_color,
            border_style: row.border_style,
            border_color: row.border_color,
            background_color: row.background_color,
            info: row.info.clone(),
            motd: row.motd.clone(),
            create_date: row.create_date,
            bank_money: row.bank_money.into(),
        }
    }
    fn member(row: super::super::PgGuildMemberRow) -> Result<GuildMemberRow> {
        Ok(GuildMemberRow {
            guild_id: row.guild_id.try_into()?,
            guid: row.guid.try_into()?,
            rank: row.rank.try_into()?,
            player_note: row.player_note,
            officer_note: row.officer_note,
        })
    }
    fn member_dto(row: &GuildMemberRow) -> PgGuildMemberRow {
        PgGuildMemberRow {
            guild_id: row.guild_id.into(),
            guid: row.guid.into(),
            rank: row.rank.into(),
            player_note: row.player_note.clone(),
            officer_note: row.officer_note.clone(),
        }
    }
    fn rank(row: super::super::PgGuildRankRow) -> Result<GuildRankRow> {
        Ok(GuildRankRow {
            guild_id: row.guild_id.try_into()?,
            id: row.id.try_into()?,
            name: row.name,
            rights: row.rights.try_into()?,
        })
    }
    fn rank_dto(row: &GuildRankRow) -> PgGuildRankRow {
        PgGuildRankRow {
            guild_id: row.guild_id.into(),
            id: row.id.into(),
            name: row.name.clone(),
            rights: row.rights.into(),
        }
    }
    fn tab(row: super::super::PgGuildBankTabRow) -> Result<GuildBankTabRow> {
        Ok(GuildBankTabRow {
            guild_id: row.guild_id.try_into()?,
            tab_id: row.tab_id.try_into()?,
            name: row.name,
            icon: row.icon,
            view_rank: row.view_rank.try_into()?,
            withdraw_rank: row.withdraw_rank.try_into()?,
            deposit_rank: row.deposit_rank.try_into()?,
        })
    }
    fn tab_dto(row: &GuildBankTabRow) -> PgGuildBankTabRow {
        PgGuildBankTabRow {
            guild_id: row.guild_id.into(),
            tab_id: row.tab_id.into(),
            name: row.name.clone(),
            icon: row.icon.clone(),
            view_rank: row.view_rank.into(),
            withdraw_rank: row.withdraw_rank.into(),
            deposit_rank: row.deposit_rank.into(),
        }
    }
}
#[async_trait]
impl GuildRepositoryTrait for GuildRepository {
    async fn find_by_id(&self, id: u32) -> Result<Option<GuildRow>> {
        self.pg()
            .find_by_id(id.into())
            .await?
            .map(Self::guild)
            .transpose()
    }
    async fn find_by_name(&self, n: &str) -> Result<Option<GuildRow>> {
        self.pg()
            .find_by_name(n)
            .await?
            .map(Self::guild)
            .transpose()
    }
    async fn exists_by_name(&self, n: &str) -> Result<bool> {
        self.pg().exists_by_name(n).await
    }
    async fn get_max_guild_id(&self) -> Result<Option<u32>> {
        self.pg()
            .get_max_guild_id()
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    async fn find_members(&self, id: u32) -> Result<Vec<GuildMemberRow>> {
        self.pg()
            .find_members(id.into())
            .await?
            .into_iter()
            .map(Self::member)
            .collect()
    }
    async fn find_members_with_character_data(
        &self,
        id: u32,
    ) -> Result<Vec<GuildMemberWithCharacterDataRow>> {
        self.pg()
            .find_members_with_character_data(id.into())
            .await?
            .into_iter()
            .map(|r| {
                Ok(GuildMemberWithCharacterDataRow {
                    guid: r.guid.try_into()?,
                    rank: r.rank.try_into()?,
                    player_note: r.player_note,
                    officer_note: r.officer_note,
                    name: r.name,
                    level: r.level.map(TryInto::try_into).transpose()?,
                    class: r.class.map(TryInto::try_into).transpose()?,
                    zone: r.zone.map(TryInto::try_into).transpose()?,
                    account: r.account.map(TryInto::try_into).transpose()?,
                    logout_time: r
                        .logout_time
                        .map(|v| v.timestamp())
                        .map(TryInto::try_into)
                        .transpose()?,
                })
            })
            .collect()
    }
    async fn find_ranks(&self, id: u32) -> Result<Vec<GuildRankRow>> {
        self.pg()
            .find_ranks(id.into())
            .await?
            .into_iter()
            .map(Self::rank)
            .collect()
    }
    async fn find_bank_tabs(&self, id: u32) -> Result<Vec<GuildBankTabRow>> {
        self.pg()
            .find_bank_tabs(id.into())
            .await?
            .into_iter()
            .map(Self::tab)
            .collect()
    }
    async fn find_event_logs(&self, id: u32, limit: u32) -> Result<Vec<GuildEventLogRow>> {
        self.pg()
            .find_event_logs(id.into(), limit.into())
            .await?
            .into_iter()
            .map(|r| {
                Ok(GuildEventLogRow {
                    guild_id: r.guild_id.try_into()?,
                    log_guid: r.log_guid.try_into()?,
                    event_type: r.event_type.try_into()?,
                    player_guid1: r.player_guid1.try_into()?,
                    player_guid2: r.player_guid2.try_into()?,
                    new_rank: r.new_rank.try_into()?,
                    timestamp: r.timestamp,
                })
            })
            .collect()
    }
    async fn find_character_data(&self, g: u32) -> Result<Option<(u8, u8, u32, u32, i64)>> {
        self.pg()
            .find_character_data(g.into())
            .await?
            .map(|(level, class, zone, account, logout_time)| {
                Ok((
                    level.try_into()?,
                    class.try_into()?,
                    zone.try_into()?,
                    account.try_into()?,
                    logout_time,
                ))
            })
            .transpose()
    }
    async fn create(
        &self,
        g: &GuildRow,
        r: &[GuildRankRow],
        m: &GuildMemberRow,
        t: &[GuildBankTabRow],
    ) -> Result<()> {
        self.pg()
            .create(
                &Self::guild_dto(g),
                &r.iter().map(Self::rank_dto).collect::<Vec<_>>(),
                &Self::member_dto(m),
                &t.iter().map(Self::tab_dto).collect::<Vec<_>>(),
            )
            .await
    }
    async fn update(&self, g: &GuildRow) -> Result<()> {
        self.pg().update(&Self::guild_dto(g)).await
    }
    async fn update_motd(&self, id: u32, v: &str) -> Result<()> {
        self.pg().update_motd(id.into(), v).await
    }
    async fn update_info(&self, id: u32, v: &str) -> Result<()> {
        self.pg().update_info(id.into(), v).await
    }
    async fn update_guild_name(&self, id: u32, v: &str) -> Result<()> {
        self.pg().update_guild_name(id.into(), v).await
    }
    async fn update_emblem(&self, id: u32, a: i32, b: i32, c: i32, d: i32, e: i32) -> Result<()> {
        self.pg().update_emblem(id.into(), a, b, c, d, e).await
    }
    async fn update_bank_money(&self, id: u32, v: u32) -> Result<()> {
        self.pg().update_bank_money(id.into(), v.into()).await
    }
    async fn update_leader(&self, id: u32, o: u32, n: u32) -> Result<()> {
        self.pg().update_leader(id.into(), o.into(), n.into()).await
    }
    async fn add_member(&self, r: &GuildMemberRow) -> Result<()> {
        self.pg().add_member(&Self::member_dto(r)).await
    }
    async fn remove_member(&self, id: u32, g: u32) -> Result<()> {
        self.pg().remove_member(id.into(), g.into()).await
    }
    async fn update_member_rank(&self, id: u32, g: u32, r: u8) -> Result<()> {
        self.pg()
            .update_member_rank(id.into(), g.into(), r.into())
            .await
    }
    async fn update_member_public_note(&self, id: u32, g: u32, n: &str) -> Result<()> {
        self.pg()
            .update_member_public_note(id.into(), g.into(), n)
            .await
    }
    async fn update_member_officer_note(&self, id: u32, g: u32, n: &str) -> Result<()> {
        self.pg()
            .update_member_officer_note(id.into(), g.into(), n)
            .await
    }
    async fn create_rank(&self, r: &GuildRankRow) -> Result<()> {
        self.pg().save_rank(&Self::rank_dto(r)).await
    }
    async fn update_rank(&self, id: u32, r: u32, n: &str, x: u32) -> Result<()> {
        self.pg()
            .save_rank(&PgGuildRankRow {
                guild_id: id.into(),
                id: r.into(),
                name: n.into(),
                rights: x.into(),
            })
            .await
    }
    async fn delete_rank(&self, id: u32, r: u32) -> Result<()> {
        self.pg().delete_rank(id.into(), r.into()).await
    }
    async fn update_bank_tab(
        &self,
        id: u32,
        t: u8,
        n: &str,
        i: &str,
        v: u8,
        w: u8,
        d: u8,
    ) -> Result<()> {
        self.pg()
            .save_bank_tab(&PgGuildBankTabRow {
                guild_id: id.into(),
                tab_id: t.into(),
                name: n.into(),
                icon: i.into(),
                view_rank: v.into(),
                withdraw_rank: w.into(),
                deposit_rank: d.into(),
            })
            .await
    }
    async fn insert_event_log(&self, r: &GuildEventLogRow) -> Result<()> {
        self.pg()
            .insert_event_log(&PgGuildEventLogRow {
                guild_id: r.guild_id.into(),
                log_guid: r.log_guid.into(),
                event_type: r.event_type.into(),
                player_guid1: r.player_guid1.into(),
                player_guid2: r.player_guid2.into(),
                new_rank: r.new_rank.into(),
                timestamp: r.timestamp,
            })
            .await
    }
    async fn get_max_event_log_guid(&self, id: u32) -> Result<Option<i32>> {
        self.pg()
            .get_max_event_log_guid(id.into())
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    async fn delete_old_event_logs(&self, id: u32, k: u32) -> Result<()> {
        self.pg().delete_old_event_logs(id.into(), k.into()).await
    }
    async fn delete(&self, id: u32) -> Result<()> {
        self.pg().delete(id.into()).await
    }
}
