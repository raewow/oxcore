use sqlx::FromRow;

#[derive(FromRow, Debug, Clone)]
pub struct GmTicketRow {
    pub ticket_id: u32,
    pub guid: u32,
    pub name: String,
    pub message: String,
    pub create_time: u64,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub last_modified_time: u64,
    pub closed_by: i32,
    pub assigned_to: u32,
    pub comment: String,
    pub response: String,
    pub completed: bool,
    pub escalated: u8,
    pub viewed: bool,
    pub have_ticket: bool,
    pub ticket_type: u8,
    pub security_needed: u32,
}

#[derive(FromRow, Debug, Clone)]
pub struct GmTicketSurveyRow {
    pub survey_id: u32,
    pub ticket_id: u32,
    pub main_survey: u8,
    pub overall_comment: String,
    pub response_time: u32,
}
