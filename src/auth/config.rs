use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // Database
    pub login_database_url: String,

    // Network
    pub realm_server_port: u16,
    pub bind_ip: String,

    // Directories
    #[serde(default = "default_patches_dir")]
    pub patches_dir: PathBuf,
    #[serde(default = "default_empty_path")]
    pub logs_dir: PathBuf,
    #[serde(default)]
    pub pid_file: Option<PathBuf>,

    // Timing
    #[serde(default = "default_max_ping_time")]
    pub max_ping_time: u32,
    #[serde(default = "default_min_realm_list_delay")]
    pub min_realm_list_delay: u32,
    #[serde(default = "default_realms_state_update_delay")]
    pub realms_state_update_delay: u32,
    #[serde(default = "default_max_session_duration")]
    pub max_session_duration: u32,
    #[serde(default = "default_realm_offline_threshold")]
    pub realm_offline_threshold: u64,

    // Security
    #[serde(default)]
    pub geo_locking: bool,
    #[serde(default = "default_strict_version_check")]
    pub strict_version_check: bool,
    #[serde(default)]
    pub req_email_verification: bool,
    #[serde(default)]
    pub req_email_since: u64,

    // Wrong password handling
    #[serde(default)]
    pub wrong_pass: WrongPassConfig,

    // Logging
    #[serde(default = "default_log_level")]
    pub log_level: u8,
    #[serde(default = "default_log_file")]
    pub log_file: String,
    #[serde(default)]
    pub log_file_level: u8,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WrongPassConfig {
    #[serde(default)]
    pub max_count: u32,
    #[serde(default = "default_ban_time")]
    pub ban_time: u32,
    #[serde(default)]
    pub ban_type: u32,
}

// Default value functions
fn default_patches_dir() -> PathBuf {
    PathBuf::from("./patches")
}

fn default_empty_path() -> PathBuf {
    PathBuf::from("")
}

fn default_max_ping_time() -> u32 {
    30
}

fn default_min_realm_list_delay() -> u32 {
    1
}

fn default_realms_state_update_delay() -> u32 {
    20
}

fn default_max_session_duration() -> u32 {
    300
}

fn default_realm_offline_threshold() -> u64 {
    60
}

fn default_strict_version_check() -> bool {
    true
}

fn default_log_level() -> u8 {
    0
}

fn default_log_file() -> String {
    "auth.log".to_string()
}

fn default_ban_time() -> u32 {
    600
}

// Legacy compatibility: expose fields from wrong_pass at top level
impl Config {
    pub fn wrong_pass_max_count(&self) -> u32 {
        self.wrong_pass.max_count
    }

    pub fn wrong_pass_ban_time(&self) -> u32 {
        self.wrong_pass.ban_time
    }

    pub fn wrong_pass_ban_type(&self) -> bool {
        self.wrong_pass.ban_type != 0
    }
}
