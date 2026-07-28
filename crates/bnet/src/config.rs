use serde::Deserialize;
use std::path::PathBuf;

/// Configuration for the Battle.net login server.
///
/// The client reaches us in two hops: an HTTPS REST login on [`Config::login_port`], then a
/// TLS protobuf-RPC channel on [`Config::bnet_port`]. Both use the same certificate, and both
/// hostnames must match what the patched client resolves (`<portal>.localhost`), because the
/// client validates the certificate against the bundle baked into its executable.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // Database
    pub login_database_url: String,

    // Network
    #[serde(default = "default_bind_ip")]
    pub bind_ip: String,
    /// Port for the BGS protobuf-RPC channel. The client hardcodes 1119 in most builds.
    #[serde(default = "default_bnet_port")]
    pub bnet_port: u16,
    /// Port for the HTTPS REST login service.
    #[serde(default = "default_login_port")]
    pub login_port: u16,
    /// Hostname handed to clients in `GET /bnetserver/portal/`. Must be a hostname, not an IP,
    /// and must match a name in the certificate.
    #[serde(default = "default_external_hostname")]
    pub external_hostname: String,
    /// Port advertised to a joining client instead of the realm's `realmlist.port`.
    ///
    /// Only modern clients ever reach realm-join, and they speak the 1.14 protocol on the world
    /// server's *modern* listener — a different port from the vanilla one the `realmlist` row
    /// carries for the legacy auth server. Leave unset when both listeners share a port.
    #[serde(default)]
    pub world_port: Option<u16>,

    // TLS
    #[serde(default = "default_cert_file")]
    pub cert_file: PathBuf,
    #[serde(default = "default_key_file")]
    pub key_file: PathBuf,

    // Login tickets
    /// Seconds a login ticket stays valid after issue.
    #[serde(default = "default_login_ticket_duration")]
    pub login_ticket_duration: u64,

    // Directories
    #[serde(default = "default_empty_path")]
    pub logs_dir: PathBuf,

    // Logging
    #[serde(default = "default_log_level")]
    pub log_level: u8,
    #[serde(default = "default_log_file")]
    pub log_file: String,
    #[serde(default)]
    pub log_file_level: u8,
}

fn default_bind_ip() -> String {
    "0.0.0.0".to_string()
}

fn default_bnet_port() -> u16 {
    1119
}

fn default_login_port() -> u16 {
    8081
}

fn default_external_hostname() -> String {
    "localhost".to_string()
}

fn default_cert_file() -> PathBuf {
    PathBuf::from("./bnet.cert.pem")
}

fn default_key_file() -> PathBuf {
    PathBuf::from("./bnet.key.pem")
}

fn default_login_ticket_duration() -> u64 {
    3600
}

fn default_empty_path() -> PathBuf {
    PathBuf::from("")
}

fn default_log_level() -> u8 {
    0
}

fn default_log_file() -> String {
    "bnet.log".to_string()
}

impl Config {
    /// The `host:port` string returned by `GET /bnetserver/portal/`.
    pub fn portal_address(&self) -> String {
        format!("{}:{}", self.external_hostname, self.bnet_port)
    }

    /// Base URL of the REST login service, as embedded in `OnExternalChallenge` payloads.
    pub fn login_base_url(&self) -> String {
        format!(
            "https://{}:{}/bnetserver/login/",
            self.external_hostname, self.login_port
        )
    }
}
