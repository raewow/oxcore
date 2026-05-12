use super::super::models::ticket::*;
use anyhow::{Context, Result};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct TicketRepository {
    pool: Arc<MySqlPool>,
}

impl TicketRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    // ========== QUERY METHODS (Read Operations) ==========

    /// Get the maximum ticket ID from the database (for generating next ID).
    pub async fn get_max_ticket_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(ticket_id) FROM gm_tickets")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max ticket_id")
    }

    /// Get the maximum survey ID from the database (for generating next ID).
    pub async fn get_max_survey_id(&self) -> Result<Option<u32>> {
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(survey_id) FROM gm_surveys")
            .fetch_one(&*self.pool)
            .await
            .context("Failed to query max survey_id")
    }

    /// Find a ticket by ID.
    pub async fn find_by_id(&self, ticket_id: u32) -> Result<Option<GmTicketRow>> {
        sqlx::query_as::<_, GmTicketRow>(
            r#"SELECT ticket_id, guid, name, message, create_time, map, position_x, position_y, position_z, 
                      last_modified_time, closed_by, assigned_to, comment, response, completed, escalated, viewed, 
                      have_ticket, ticket_type, security_needed 
               FROM gm_tickets WHERE ticket_id = ?"#,
        )
        .bind(ticket_id)
        .fetch_optional(&*self.pool)
        .await
        .context("Failed to fetch ticket by ID")
    }

    /// Find all open tickets.
    pub async fn find_open_tickets(&self) -> Result<Vec<GmTicketRow>> {
        sqlx::query_as::<_, GmTicketRow>(
            r#"SELECT ticket_id, guid, name, message, create_time, map, position_x, position_y, position_z, 
                      last_modified_time, closed_by, assigned_to, comment, response, completed, escalated, viewed, 
                      have_ticket, ticket_type, security_needed 
               FROM gm_tickets WHERE closed_by IS NULL OR closed_by = ''"#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("Failed to fetch open tickets")
    }

    // ========== COMMAND METHODS (Write Operations) ==========

    /// Create a new ticket.
    pub async fn create_ticket(&self, ticket: &GmTicketRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO gm_tickets (ticket_id, guid, name, message, create_time, map, position_x, position_y, position_z, 
                                       last_modified_time, closed_by, assigned_to, comment, response, completed, escalated, viewed, 
                                       have_ticket, ticket_type, security_needed) 
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(ticket.ticket_id)
        .bind(ticket.guid)
        .bind(&ticket.name)
        .bind(&ticket.message)
        .bind(ticket.create_time)
        .bind(ticket.map)
        .bind(ticket.position_x)
        .bind(ticket.position_y)
        .bind(ticket.position_z)
        .bind(ticket.last_modified_time)
        .bind(&ticket.closed_by)
        .bind(ticket.assigned_to)
        .bind(&ticket.comment)
        .bind(&ticket.response)
        .bind(ticket.completed)
        .bind(ticket.escalated)
        .bind(ticket.viewed)
        .bind(ticket.have_ticket)
        .bind(ticket.ticket_type)
        .bind(ticket.security_needed)
        .execute(&*self.pool)
        .await
        .context("Failed to create ticket")?;

        Ok(())
    }

    /// Update a ticket.
    pub async fn update_ticket(&self, ticket: &GmTicketRow) -> Result<()> {
        sqlx::query(
            r#"UPDATE gm_tickets SET guid = ?, name = ?, message = ?, create_time = ?, map = ?, position_x = ?, position_y = ?, position_z = ?, 
                                       last_modified_time = ?, closed_by = ?, assigned_to = ?, comment = ?, response = ?, completed = ?, escalated = ?, viewed = ?, 
                                       have_ticket = ?, ticket_type = ?, security_needed = ? 
               WHERE ticket_id = ?"#,
        )
        .bind(ticket.guid)
        .bind(&ticket.name)
        .bind(&ticket.message)
        .bind(ticket.create_time)
        .bind(ticket.map)
        .bind(ticket.position_x)
        .bind(ticket.position_y)
        .bind(ticket.position_z)
        .bind(ticket.last_modified_time)
        .bind(&ticket.closed_by)
        .bind(ticket.assigned_to)
        .bind(&ticket.comment)
        .bind(&ticket.response)
        .bind(ticket.completed)
        .bind(ticket.escalated)
        .bind(ticket.viewed)
        .bind(ticket.have_ticket)
        .bind(ticket.ticket_type)
        .bind(ticket.security_needed)
        .bind(ticket.ticket_id)
        .execute(&*self.pool)
        .await
        .context("Failed to update ticket")?;

        Ok(())
    }

    /// Close a ticket.
    pub async fn close_ticket(&self, ticket_id: u32, closed_by: &str) -> Result<()> {
        sqlx::query(r#"UPDATE gm_tickets SET closed_by = ? WHERE ticket_id = ?"#)
            .bind(closed_by)
            .bind(ticket_id)
            .execute(&*self.pool)
            .await
            .context("Failed to close ticket")?;

        Ok(())
    }

    /// Delete a ticket.
    pub async fn delete_ticket(&self, ticket_id: u32) -> Result<()> {
        sqlx::query("DELETE FROM gm_tickets WHERE ticket_id = ?")
            .bind(ticket_id)
            .execute(&*self.pool)
            .await
            .context("Failed to delete ticket")?;

        Ok(())
    }

    /// Create a new survey.
    pub async fn create_survey(&self, survey: &GmTicketSurveyRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO gm_surveys (survey_id, ticket_id, main_survey, overall_comment, response_time) 
               VALUES (?, ?, ?, ?, ?)"#,
        )
        .bind(survey.survey_id)
        .bind(survey.ticket_id)
        .bind(survey.main_survey)
        .bind(&survey.overall_comment)
        .bind(survey.response_time)
        .execute(&*self.pool)
        .await
        .context("Failed to create survey")?;

        Ok(())
    }
}
