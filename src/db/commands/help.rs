pub fn run() {
    println!("db - Database tool for rcore");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin db -- <command> [options]");
    println!();
    println!("COMMANDS:");
    println!("    migrate          Apply base tables and any pending migrations to all databases");
    println!("    status           Show base table and migration status for all databases");
    println!("    new <db> <name>  Create a new migration file");
    println!("    help             Show this message");
    println!();
    println!("DATABASES:");
    println!("    world, auth, characters, logs");
    println!();
    println!("EXAMPLES:");
    println!("    cargo run --bin db -- migrate");
    println!("    cargo run --bin db -- status");
    println!("    cargo run --bin db -- new world add_creature_gossip_option");
    println!("    cargo run --bin db -- new characters add_character_pet");
    println!();
    println!("MIGRATION FILES:");
    println!("    Created in sql/migrations/ with format: YYYYMMDDHHMMSS_<db>_<name>.sql");
    println!();
    println!("CONFIG:");
    println!("    Reads database URLs from config.toml (same file as auth/world servers).");
    println!("    Use -c <path> to specify a different config file.");
}
