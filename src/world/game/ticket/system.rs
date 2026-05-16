//! GM Ticket System
//!
//! Manages player-submitted support tickets.

use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::shared::database::characters::repositories::TicketRepository;
use crate::shared::game::ticket::{
    GmTicketEscalationStatus, GmTicketResponse, GmTicketStatus, GmTicketSystemStatus, GmTicketType,
};
use crate::shared::protocol::ObjectGuid;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::world::game::player::PlayerManager;

use super::types::TicketEntry;

pub struct TicketSystem {
    /// Per-player ticket state
    tickets: DashMap<ObjectGuid, TicketEntry>,

    /// Dependencies
    repository: Arc<TicketRepository>,
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,

    /// Config
    system_enabled: bool,
    next_ticket_id: Arc<std::sync::atomic::AtomicU32>,
}

impl TicketSystem {
    pub fn new(
        repository: Arc<TicketRepository>,
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
    ) -> Self {
        Self {
            tickets: DashMap::new(),
            repository,
            broadcast_mgr,
            player_mgr,
            system_enabled: true,
            next_ticket_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    // ========== Lifecycle Methods ==========

    pub async fn init(&self) -> Result<()> {
        // Load max ticket ID from database
        if let Some(max_id) = self.repository.get_max_ticket_id().await? {
            self.next_ticket_id
                .store(max_id + 1, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn update(&self, _diff: Duration) -> Result<()> {
        // No periodic updates needed for basic ticket system
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        // Save all tickets to database
        use crate::shared::database::characters::models::ticket::GmTicketRow;
        for entry in self.tickets.iter() {
            let ticket = entry.value();
            let row = GmTicketRow {
                ticket_id: ticket.ticket_id,
                guid: ticket.player_guid.counter(),
                name: ticket.player_name.clone(),
                message: ticket.message.clone(),
                create_time: ticket.create_time,
                map: ticket.map_id as u32,
                position_x: ticket.position.0,
                position_y: ticket.position.1,
                position_z: ticket.position.2,
                last_modified_time: ticket.last_modified_time,
                closed_by: 0,
                assigned_to: 0,
                comment: String::new(),
                response: String::new(),
                completed: false,
                escalated: ticket.escalated_status as u8,
                viewed: ticket.viewed,
                have_ticket: true,
                ticket_type: ticket.ticket_type as u8,
                security_needed: 0,
            };
            self.repository.update_ticket(&row).await?;
        }
        Ok(())
    }

    pub async fn on_player_login(&self, _guid: ObjectGuid) -> Result<()> {
        // Tickets are loaded on-demand when player queries
        Ok(())
    }

    pub async fn on_player_logout(&self, guid: ObjectGuid) -> Result<()> {
        // Save ticket if exists and remove from cache
        if let Some((_guid, ticket)) = self.tickets.remove(&guid) {
            use crate::shared::database::characters::models::ticket::GmTicketRow;
            let row = GmTicketRow {
                ticket_id: ticket.ticket_id,
                guid: ticket.player_guid.counter(),
                name: ticket.player_name.clone(),
                message: ticket.message.clone(),
                create_time: ticket.create_time,
                map: ticket.map_id as u32,
                position_x: ticket.position.0,
                position_y: ticket.position.1,
                position_z: ticket.position.2,
                last_modified_time: ticket.last_modified_time,
                closed_by: 0,
                assigned_to: 0,
                comment: String::new(),
                response: String::new(),
                completed: false,
                escalated: ticket.escalated_status as u8,
                viewed: ticket.viewed,
                have_ticket: true,
                ticket_type: ticket.ticket_type as u8,
                security_needed: 0,
            };
            self.repository.update_ticket(&row).await?;
        }
        Ok(())
    }

    // ========== Core Ticket Operations ==========

    pub async fn create_ticket(
        &self,
        player_guid: ObjectGuid,
        player_name: String,
        ticket_type: GmTicketType,
        map_id: u16,
        position: (f32, f32, f32),
        message: String,
    ) -> Result<GmTicketResponse> {
        // Check if player already has a ticket
        if self.tickets.contains_key(&player_guid) {
            return Ok(GmTicketResponse::AlreadyExist);
        }

        // Generate ticket ID
        let ticket_id = self
            .next_ticket_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Create ticket entry
        let ticket = TicketEntry {
            ticket_id,
            player_guid,
            player_name: player_name.clone(),
            message: message.clone(),
            ticket_type,
            map_id,
            position,
            create_time: now,
            last_modified_time: now,
            escalated_status: GmTicketEscalationStatus::NotAssigned,
            viewed: false,
        };

        // Save to database (INSERT)
        use crate::shared::database::characters::models::ticket::GmTicketRow;
        let row = GmTicketRow {
            ticket_id,
            guid: player_guid.counter(),
            name: player_name,
            message,
            create_time: now,
            map: map_id as u32,
            position_x: position.0,
            position_y: position.1,
            position_z: position.2,
            last_modified_time: now,
            closed_by: 0,
            assigned_to: 0,
            comment: String::new(),
            response: String::new(),
            completed: false,
            escalated: GmTicketEscalationStatus::NotAssigned as u8,
            viewed: false,
            have_ticket: true,
            ticket_type: ticket_type as u8,
            security_needed: 0,
        };
        self.repository.create_ticket(&row).await?;

        // Cache in memory
        self.tickets.insert(player_guid, ticket);

        Ok(GmTicketResponse::CreateSuccess)
    }

    pub async fn update_ticket_text(
        &self,
        player_guid: ObjectGuid,
        message: String,
    ) -> Result<GmTicketResponse> {
        if let Some(mut entry) = self.tickets.get_mut(&player_guid) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            entry.message = message.clone();
            entry.last_modified_time = now;

            // Update database
            use crate::shared::database::characters::models::ticket::GmTicketRow;
            let row = GmTicketRow {
                ticket_id: entry.ticket_id,
                guid: entry.player_guid.counter(),
                name: entry.player_name.clone(),
                message,
                create_time: entry.create_time,
                map: entry.map_id as u32,
                position_x: entry.position.0,
                position_y: entry.position.1,
                position_z: entry.position.2,
                last_modified_time: now,
                closed_by: 0,
                assigned_to: 0,
                comment: String::new(),
                response: String::new(),
                completed: false,
                escalated: entry.escalated_status as u8,
                viewed: entry.viewed,
                have_ticket: true,
                ticket_type: entry.ticket_type as u8,
                security_needed: 0,
            };
            self.repository.update_ticket(&row).await?;

            Ok(GmTicketResponse::UpdateSuccess)
        } else {
            Ok(GmTicketResponse::NotExist)
        }
    }

    pub async fn delete_ticket(&self, player_guid: ObjectGuid) -> Result<GmTicketResponse> {
        if let Some((_guid, ticket)) = self.tickets.remove(&player_guid) {
            self.repository.delete_ticket(ticket.ticket_id).await?;
            Ok(GmTicketResponse::TicketDeleted)
        } else {
            Ok(GmTicketResponse::NotExist)
        }
    }

    // ========== Broadcast Methods (send via BroadcastManager) ==========

    pub fn send_ticket(&self, player_guid: ObjectGuid) {
        use crate::shared::messages::ticket::{SmsgGmTicketGetTicket, TicketData};

        let msg = if let Some(ticket) = self.tickets.get(&player_guid) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            SmsgGmTicketGetTicket {
                status: GmTicketStatus::HasText as u32,
                ticket: Some(TicketData {
                    text: ticket.message.clone(),
                    ticket_type: ticket.ticket_type as u8,
                    days_since_creation: ((now - ticket.create_time) as f32) / 86400.0,
                    days_since_oldest: self.get_oldest_ticket_age(),
                    days_since_last_update: ((now - ticket.last_modified_time) as f32) / 86400.0,
                    escalation_status: ticket.escalated_status as u8,
                    read_by_gm: ticket.viewed,
                }),
            }
        } else {
            SmsgGmTicketGetTicket {
                status: GmTicketStatus::Default as u32,
                ticket: None,
            }
        };

        self.broadcast_mgr.send_msg_to_player(player_guid, msg);
    }

    pub fn send_system_status(&self, player_guid: ObjectGuid) {
        use crate::shared::messages::ticket::SmsgGmTicketSystemStatus;

        let status = if self.system_enabled {
            GmTicketSystemStatus::Enabled as u32
        } else {
            GmTicketSystemStatus::Disabled as u32
        };

        self.broadcast_mgr
            .send_msg_to_player(player_guid, SmsgGmTicketSystemStatus { status });
    }

    pub fn send_create_response(&self, player_guid: ObjectGuid, result: GmTicketResponse) {
        use crate::shared::messages::ticket::SmsgGmTicketCreate;
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGmTicketCreate {
                result: result as u32,
            },
        );
    }

    pub fn send_update_response(&self, player_guid: ObjectGuid, result: GmTicketResponse) {
        use crate::shared::messages::ticket::SmsgGmTicketUpdateText;
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGmTicketUpdateText {
                result: result as u32,
            },
        );
    }

    pub fn send_delete_response(&self, player_guid: ObjectGuid, result: GmTicketResponse) {
        use crate::shared::messages::ticket::SmsgGmTicketDeleteTicket;
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGmTicketDeleteTicket {
                result: result as u32,
            },
        );
    }

    // ========== Helper Methods ==========

    fn get_oldest_ticket_age(&self) -> f32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.tickets
            .iter()
            .map(|entry| ((now - entry.create_time) as f32) / 86400.0)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }
}
