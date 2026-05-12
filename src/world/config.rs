use anyhow::Result;
use parking_lot::RwLock;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // Database connections (4 databases) - URL format
    pub world_database_url: String,
    pub character_database_url: String,
    pub login_database_url: String,
    pub logs_database_url: String,

    // Network
    pub world_server_port: u16,
    pub bind_ip: String,

    // Directories
    #[serde(default = "default_data_dir")]
    pub data_dir: std::path::PathBuf,

    // World Update Settings
    #[serde(default = "default_world_update_interval")]
    pub world_update_interval: u32, // milliseconds (default: 50ms)

    // Realm Settings
    #[serde(default = "default_realm_id")]
    pub realm_id: i32, // Realm ID (default: 1, -1 means all realms)

    // Session Settings
    #[serde(default = "default_session_idle_timeout")]
    pub session_idle_timeout: u32, // Session idle timeout in seconds (default: 900 = 15 minutes, 0 = disabled)
    #[serde(default = "default_logout_timer")]
    pub logout_timer: u32, // Logout timer in seconds (default: 20, 0 = instant logout)

    // Performance
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default)]
    pub player_limit: u32,

    // Character Creation Settings
    #[serde(default = "default_characters_per_realm")]
    pub characters_per_realm: u32,
    #[serde(default = "default_start_player_level")]
    pub start_player_level: u32,
    #[serde(default)]
    pub start_player_money: u32,
    #[serde(default = "default_min_player_name")]
    pub min_player_name: u32,
    #[serde(default = "default_max_player_name")]
    pub max_player_name: u32,
    #[serde(default)]
    pub strict_player_names: u32,
    #[serde(default)]
    pub characters_creating_disabled: u32, // Bitmask: bit 0 = Alliance, bit 1 = Horde

    // Character Deletion Settings
    #[serde(default)]
    pub char_delete_method: Option<u32>, // Character deletion method (0 = hard delete, 1 = soft delete, default: 0)
    #[serde(default)]
    pub char_delete_min_level: Option<u32>, // Minimum level for character deletion (default: 0 = no minimum)

    // PvP Realm Settings
    #[serde(default)]
    pub is_pvp_realm: bool,
    #[serde(default)]
    pub allow_two_side_accounts: bool,

    // Chat Settings
    #[serde(default = "default_allow_cross_faction_whispers")]
    pub allow_cross_faction_whispers: bool,

    // Cross-Faction Interaction Settings (matches MaNGOS AllowTwoSide.Interaction.*)
    #[serde(default)]
    pub allow_cross_faction_chat: bool, // Say/Yell/Emote (default: false)
    #[serde(default)]
    pub allow_cross_faction_channel: bool, // Channels (default: false)
    #[serde(default)]
    pub allow_cross_faction_group: bool, // Group invites (default: false)
    #[serde(default)]
    pub allow_cross_faction_guild: bool, // Guild invites (default: false)
    #[serde(default)]
    pub allow_cross_faction_trade: bool, // Trade (default: false)
    #[serde(default)]
    pub allow_cross_faction_auction: bool, // Merged AH (default: false)
    #[serde(default)]
    pub allow_cross_faction_mail: bool, // Mail (default: false)
    #[serde(default)]
    pub allow_cross_faction_add_friend: bool, // Add friend (default: false)

    #[serde(default = "default_chat_flood_protection_delay")]
    pub chat_flood_protection_delay: u32, // Minimum delay between messages in seconds
    #[serde(default = "default_chat_flood_protection_count")]
    pub chat_flood_protection_count: u32, // Max messages in window
    #[serde(default = "default_chat_flood_protection_window")]
    pub chat_flood_protection_window: u32, // Time window in seconds
    #[serde(default = "default_chat_flood_mute_time")]
    pub chat_flood_mute_time: u32, // Auto-mute duration in seconds (0 = disabled)
    #[serde(default)]
    pub chat_strict_link_checking_kick: bool, // Kick player for invalid links (default: false)

    // VMap Settings
    #[serde(default = "default_true")]
    pub vmap_enable_los: bool, // Enable line of sight checks
    #[serde(default = "default_true")]
    pub vmap_enable_height: bool, // Enable height calculations
    #[serde(default = "default_true")]
    pub vmap_enable_indoor_check: bool, // Enable indoor checks
    #[serde(default)]
    pub vmap_path: Option<String>, // Path to vmaps directory

    // MMap Settings
    #[serde(default = "default_true")]
    pub mmap_enabled: bool, // Enable mmaps
    #[serde(default = "default_true")]
    pub mmap_pathfinding: bool, // Enable pathfinding
    #[serde(default)]
    pub mmap_path: Option<String>, // Path to mmaps directory

    // Server Message Settings
    #[serde(default = "default_motd")]
    pub motd: String,
    #[serde(default = "default_autobroadcast_db_check_interval")]
    pub autobroadcast_db_check_interval: Option<u32>, // AutoBroadcast database check interval in milliseconds (default: 60 seconds = 60000)

    // Weather Settings
    #[serde(default = "default_change_weather_interval")]
    pub change_weather_interval: Option<u32>, // Weather change interval in milliseconds (default: 10 minutes = 600000)

    // Social/Who List Settings
    #[serde(default)]
    pub allow_two_side_who_list: Option<bool>, // Allow players to see opposite faction in who list (default: false)
    #[serde(default)]
    pub gm_level_in_who_list: Option<u32>, // Maximum GM level visible in who list for players (default: 0 = SEC_PLAYER)

    // Honor System Settings
    #[serde(default = "default_min_honor_kills")]
    pub min_honor_kills: Option<u32>, // Minimum honorable kills for standing (default: 15)
    #[serde(default = "default_pvp_pool_size")]
    pub pvp_pool_size_per_faction: Option<u32>, // Pool size for honor calculations (default: 0 = use actual size)
    #[serde(default = "default_rp_decay")]
    pub rp_decay: Option<f32>, // Rank point decay multiplier (default: 0.2)
    #[serde(default = "default_enable_city_protector")]
    pub enable_city_protector: Option<bool>, // Enable city protector titles (default: true)
    #[serde(default)]
    pub auto_honor_restart: Option<bool>, // Auto restart after honor maintenance (default: false)
    #[serde(default)]
    pub accurate_pvp_rewards: Option<bool>, // Use accurate PvP reward formulas (default: false)
    #[serde(default = "default_wow_patch")]
    pub wow_patch: Option<u32>, // WoW patch version (e.g., 112 for 1.12, default: 112)

    // Talent Reset (Respec) Settings
    #[serde(default = "default_respec_base_cost")]
    pub respec_base_cost: Option<u32>, // Base cost for first talent reset in gold (default: 1)
    #[serde(default = "default_respec_multiplicative_cost")]
    pub respec_multiplicative_cost: Option<u32>, // Multiplicative cost per reset in gold (default: 5)
    #[serde(default = "default_respec_min_multiplier")]
    pub respec_min_multiplier: Option<u32>, // Minimum multiplier after decay (default: 2)
    #[serde(default = "default_respec_max_multiplier")]
    pub respec_max_multiplier: Option<u32>, // Maximum multiplier cap (default: 10)
    #[serde(default)]
    pub no_respec_price_decay: Option<bool>, // Disable price decay over time (default: false)

    // Talent Rate Settings
    #[serde(default = "default_rate_talent")]
    pub rate_talent: Option<f32>, // Talent point earning rate multiplier (default: 1.0)

    // Shutdown Settings
    #[serde(default = "default_shutdown_timer")]
    pub default_shutdown_timer: String, // Default shutdown timer (e.g., "0s" for instant, "15m", "30s"). Use "0s" for instant.

    // Guild/Petition Settings
    #[serde(default = "default_min_petition_signs")]
    pub min_petition_signs: Option<u32>, // Minimum signatures required to turn in a guild charter (default: 9, range: 0-9)

    // Mail Settings
    #[serde(default = "default_mail_delivery_delay")]
    pub mail_delivery_delay: u32, // Delivery delay in seconds for items/money mail (0 = instant, 3600 = 1 hour retail default)

    // Logging Settings
    #[serde(default = "default_log_level")]
    pub log_level: u8, // Console log level (0=error, 1=warn, 2=info, 3=debug, 4=trace)
    #[serde(default = "default_log_file_level")]
    pub log_file_level: u8, // File log level
    #[serde(default)]
    pub log_file: String, // Log file path (empty = no file logging)
    #[serde(default)]
    pub logs_dir: std::path::PathBuf, // Directory for log files
    #[serde(default = "default_true")]
    pub log_wipe_on_start: bool, // Wipe log file on server start (default: true)

    // Realm heartbeat
    #[serde(default = "default_realm_heartbeat_interval")]
    pub realm_heartbeat_interval: u64, // Realm heartbeat interval in seconds (default: 30)

    // Packet Collapsing Settings (Performance Optimization)
    #[serde(default = "default_true")]
    pub packet_collapsing_enabled: bool, // Enable packet collapsing optimization (default: true)

    #[serde(default = "default_packet_queue_max_skippable")]
    pub packet_queue_max_skippable: usize, // Max skippable packets per player queue (default: 100)

    #[serde(default = "default_packet_queue_max_non_skippable")]
    pub packet_queue_max_non_skippable: usize, // Max non-skippable packets per player queue (default: 200)
}

// Default value functions
fn default_data_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("data")
}

fn default_world_update_interval() -> u32 {
    50 // 50ms
}

fn default_realm_id() -> i32 {
    1
}

fn default_logout_timer() -> u32 {
    20 // Blizzlike 20 seconds, set to 0 for instant logout
}

fn default_session_idle_timeout() -> u32 {
    900 // 15 minutes
}

fn default_max_players() -> u32 {
    100
}

fn default_characters_per_realm() -> u32 {
    10
}

fn default_start_player_level() -> u32 {
    1
}

fn default_min_player_name() -> u32 {
    2
}

fn default_max_player_name() -> u32 {
    12
}

fn default_allow_cross_faction_whispers() -> bool {
    true
}

fn default_chat_flood_protection_delay() -> u32 {
    1
}

fn default_chat_flood_protection_count() -> u32 {
    10
}

fn default_packet_queue_max_skippable() -> usize {
    100
}

fn default_packet_queue_max_non_skippable() -> usize {
    200
}

fn default_chat_flood_protection_window() -> u32 {
    5
}

fn default_chat_flood_mute_time() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

fn default_motd() -> String {
    "Welcome to rcore!".to_string()
}

fn default_autobroadcast_db_check_interval() -> Option<u32> {
    Some(60 * 1000) // 60 seconds
}

fn default_change_weather_interval() -> Option<u32> {
    Some(10 * 60 * 1000) // 10 minutes
}

fn default_min_honor_kills() -> Option<u32> {
    Some(15)
}

fn default_pvp_pool_size() -> Option<u32> {
    Some(0)
}

fn default_rp_decay() -> Option<f32> {
    Some(0.2)
}

fn default_enable_city_protector() -> Option<bool> {
    Some(true)
}

fn default_wow_patch() -> Option<u32> {
    Some(112)
}

fn default_respec_base_cost() -> Option<u32> {
    Some(1)
}

fn default_respec_multiplicative_cost() -> Option<u32> {
    Some(5)
}

fn default_respec_min_multiplier() -> Option<u32> {
    Some(2)
}

fn default_respec_max_multiplier() -> Option<u32> {
    Some(10)
}

fn default_rate_talent() -> Option<f32> {
    Some(1.0)
}

fn default_shutdown_timer() -> String {
    "0s".to_string() // Instant shutdown by default
}

fn default_min_petition_signs() -> Option<u32> {
    Some(9) // Vanilla WoW requires 9 signatures (range: 0-9)
}

fn default_mail_delivery_delay() -> u32 {
    3600 // 1 hour in seconds (retail default)
}

fn default_log_level() -> u8 {
    2 // info
}

fn default_log_file_level() -> u8 {
    2 // info
}

fn default_realm_heartbeat_interval() -> u64 {
    30
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            world_database_url: String::new(),
            character_database_url: String::new(),
            login_database_url: String::new(),
            logs_database_url: String::new(),
            world_server_port: 8085,
            bind_ip: "0.0.0.0".to_string(),
            data_dir: default_data_dir(),
            world_update_interval: default_world_update_interval(),
            realm_id: default_realm_id(),
            logout_timer: default_logout_timer(),
            session_idle_timeout: default_session_idle_timeout(),
            max_players: default_max_players(),
            player_limit: 0,
            characters_per_realm: default_characters_per_realm(),
            start_player_level: default_start_player_level(),
            start_player_money: 0,
            min_player_name: default_min_player_name(),
            max_player_name: default_max_player_name(),
            strict_player_names: 0,
            characters_creating_disabled: 0,
            char_delete_method: None,
            char_delete_min_level: None,
            is_pvp_realm: false,
            allow_two_side_accounts: false,
            allow_cross_faction_whispers: default_allow_cross_faction_whispers(),
            allow_cross_faction_chat: false,
            allow_cross_faction_channel: false,
            allow_cross_faction_group: false,
            allow_cross_faction_guild: false,
            allow_cross_faction_trade: false,
            allow_cross_faction_auction: false,
            allow_cross_faction_mail: false,
            allow_cross_faction_add_friend: false,
            chat_flood_protection_delay: default_chat_flood_protection_delay(),
            chat_flood_protection_count: default_chat_flood_protection_count(),
            chat_flood_protection_window: default_chat_flood_protection_window(),
            chat_flood_mute_time: default_chat_flood_mute_time(),
            chat_strict_link_checking_kick: false,
            vmap_enable_los: default_true(),
            vmap_enable_height: default_true(),
            vmap_enable_indoor_check: default_true(),
            vmap_path: None,
            mmap_enabled: default_true(),
            mmap_pathfinding: default_true(),
            mmap_path: None,
            motd: default_motd(),
            autobroadcast_db_check_interval: default_autobroadcast_db_check_interval(),
            change_weather_interval: default_change_weather_interval(),
            allow_two_side_who_list: None,
            gm_level_in_who_list: None,
            min_honor_kills: default_min_honor_kills(),
            pvp_pool_size_per_faction: default_pvp_pool_size(),
            rp_decay: default_rp_decay(),
            enable_city_protector: default_enable_city_protector(),
            auto_honor_restart: None,
            accurate_pvp_rewards: None,
            wow_patch: default_wow_patch(),
            respec_base_cost: default_respec_base_cost(),
            respec_multiplicative_cost: default_respec_multiplicative_cost(),
            respec_min_multiplier: default_respec_min_multiplier(),
            respec_max_multiplier: default_respec_max_multiplier(),
            no_respec_price_decay: None,
            rate_talent: default_rate_talent(),
            default_shutdown_timer: default_shutdown_timer(),
            min_petition_signs: default_min_petition_signs(),
            mail_delivery_delay: default_mail_delivery_delay(),
            packet_collapsing_enabled: default_true(),
            packet_queue_max_skippable: default_packet_queue_max_skippable(),
            packet_queue_max_non_skippable: default_packet_queue_max_non_skippable(),
            log_level: default_log_level(),
            log_file_level: default_log_file_level(),
            log_file: String::new(),
            logs_dir: std::path::PathBuf::new(),
            log_wipe_on_start: default_true(),
            realm_heartbeat_interval: default_realm_heartbeat_interval(),
        }
    }
}

/// Global configuration manager singleton
pub struct ConfigMgr {
    config: Arc<RwLock<Config>>,
}

impl ConfigMgr {
    pub fn initialize(config: Config) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub fn get(&self) -> parking_lot::RwLockReadGuard<'_, Config> {
        self.config.read()
    }

    pub fn get_mut(&self) -> parking_lot::RwLockWriteGuard<'_, Config> {
        self.config.write()
    }
}

static mut CONFIG_MGR: Option<ConfigMgr> = None;
static CONFIG_MGR_INIT: std::sync::Once = std::sync::Once::new();

/// Get the global config manager instance (panics if not initialized)
#[allow(static_mut_refs)]
pub fn get_config_mgr() -> &'static ConfigMgr {
    unsafe {
        CONFIG_MGR
            .as_ref()
            .expect("ConfigMgr not initialized. Call initialize_config_mgr() first.")
    }
}

/// Initialize the global config manager (call once at startup)
pub fn initialize_config_mgr(config: Config) {
    CONFIG_MGR_INIT.call_once(|| unsafe {
        CONFIG_MGR = Some(ConfigMgr::initialize(config));
    });
}
