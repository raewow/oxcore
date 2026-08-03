pub const AFTER_HELP: &str = "\
EXAMPLES:
    cargo run --bin db -- pg migrate
    cargo run --bin db -- pg status
    cargo run --bin db -- pg fresh --yes
    cargo run --bin db -- pg new world add_creature_gossip_option

MIGRATION FILES:
    Created in crates/db/migrations/<schema>/ with format: YYYYMMDDHHMMSS_<name>.sql

CONFIG:
    Reads the PostgreSQL URL from [postgres] in config.toml.
    Use -c <path> to specify a different config file.";
