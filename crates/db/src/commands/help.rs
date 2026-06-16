pub const AFTER_HELP: &str = "\
DATABASES:
    world, auth, characters, logs

EXAMPLES:
    cargo run --bin db -- migrate
    cargo run --bin db -- status
    cargo run --bin db -- new world add_creature_gossip_option
    cargo run --bin db -- new characters add_character_pet

MIGRATION FILES:
    Created in sql/migrations/ with format: YYYYMMDDHHMMSS_<db>_<name>.sql

CONFIG:
    Reads database URLs from config.toml (same file as auth/world servers).
    Use -c <path> to specify a different config file.";
