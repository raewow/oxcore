pub const AFTER_HELP: &str = "\
MYSQL DATABASES:
    world, auth, characters, web

EXAMPLES:
    cargo run --bin db -- migrate
    cargo run --bin db -- status
    cargo run --bin db -- new world add_creature_gossip_option
    cargo run --bin db -- new characters add_character_pet
    cargo run --bin db -- fresh
    cargo run --bin db -- pg migrate

MIGRATION FILES:
    Created in sql/migrations/ with format: YYYYMMDDHHMMSS_<db>_<name>.sql

CONFIG:
    Reads MySQL URLs from config.toml and PostgreSQL from [postgres].
    Use -c <path> to specify a different config file.";
