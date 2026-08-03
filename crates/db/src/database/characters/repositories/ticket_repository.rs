use super::super::models::ticket::*;
use super::super::{PgSurveyRow, PgTicketRepository, PgTicketRow};
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;

pub struct TicketRepository {
    pool: Arc<PgPool>,
}
impl TicketRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    fn pg(&self) -> PgTicketRepository {
        PgTicketRepository::new(Arc::clone(&self.pool))
    }
    fn row(row: PgTicketRow) -> Result<GmTicketRow> {
        Ok(GmTicketRow {
            ticket_id: row.ticket_id.try_into()?,
            guid: row.guid.try_into()?,
            name: row.name,
            message: row.message,
            create_time: row.create_time.try_into()?,
            map: row.map.try_into()?,
            position_x: row.position_x,
            position_y: row.position_y,
            position_z: row.position_z,
            last_modified_time: row.last_modified_time.try_into()?,
            closed_by: row.closed_by.try_into()?,
            assigned_to: row.assigned_to.try_into()?,
            comment: row.comment,
            response: row.response,
            completed: row.completed,
            escalated: row.escalated.try_into()?,
            viewed: row.viewed,
            have_ticket: row.have_ticket,
            ticket_type: row.ticket_type.try_into()?,
            security_needed: row.security_needed.try_into()?,
        })
    }
    fn dto(row: &GmTicketRow) -> Result<PgTicketRow> {
        Ok(PgTicketRow {
            ticket_id: row.ticket_id.into(),
            guid: row.guid.into(),
            name: row.name.clone(),
            message: row.message.clone(),
            create_time: row.create_time.try_into()?,
            map: row.map.into(),
            position_x: row.position_x,
            position_y: row.position_y,
            position_z: row.position_z,
            last_modified_time: row.last_modified_time.try_into()?,
            closed_by: row.closed_by.into(),
            assigned_to: row.assigned_to.into(),
            comment: row.comment.clone(),
            response: row.response.clone(),
            completed: row.completed,
            escalated: row.escalated.into(),
            viewed: row.viewed,
            have_ticket: row.have_ticket,
            ticket_type: row.ticket_type.into(),
            security_needed: row.security_needed.into(),
        })
    }
    pub async fn get_max_ticket_id(&self) -> Result<Option<u32>> {
        self.pg()
            .get_max_ticket_id()
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    pub async fn get_max_survey_id(&self) -> Result<Option<u32>> {
        self.pg()
            .get_max_survey_id()
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
    }
    pub async fn find_by_id(&self, id: u32) -> Result<Option<GmTicketRow>> {
        self.pg()
            .find_by_id(id.into())
            .await?
            .map(Self::row)
            .transpose()
    }
    pub async fn find_open_tickets(&self) -> Result<Vec<GmTicketRow>> {
        self.pg()
            .find_open_tickets()
            .await?
            .into_iter()
            .map(Self::row)
            .collect()
    }
    pub async fn create_ticket(&self, row: &GmTicketRow) -> Result<()> {
        self.pg().save_ticket(&Self::dto(row)?).await
    }
    pub async fn update_ticket(&self, row: &GmTicketRow) -> Result<()> {
        self.pg().save_ticket(&Self::dto(row)?).await
    }
    pub async fn close_ticket(&self, id: u32, closed_by: &str) -> Result<()> {
        self.pg()
            .close_ticket(
                id.into(),
                closed_by
                    .parse()
                    .context("ticket closer must be a numeric GUID")?,
            )
            .await
    }
    pub async fn delete_ticket(&self, id: u32) -> Result<()> {
        self.pg().delete_ticket(id.into()).await
    }
    pub async fn create_survey(&self, row: &GmTicketSurveyRow) -> Result<()> {
        self.pg()
            .create_survey(&PgSurveyRow {
                survey_id: row.survey_id.into(),
                ticket_id: row.ticket_id.into(),
                main_survey: row.main_survey.into(),
                overall_comment: row.overall_comment.clone(),
                response_time: row.response_time.into(),
            })
            .await
    }
}
