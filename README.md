# rcore - World of Warcraft Vanilla Emulation Server

A World of Warcraft (Vanilla 1.12.x, Classic 1.14.x) private server implementation written in Rust. 

## Server Goals + Milestones

Creating an emulator for world of warcraft is a massive task, our end goal is something on par to vmangos, however our focused current goal is to get the emulator working allowing players to play any class up to level 20 without major issues. 


### Data Files

The server requires extracted game data files from the WoW client. You can use vmangos versions of the following:

1. **DBC Files** - Database Client files containing game definitions (spells, items, areas, etc.)
2. **Map Files** - Terrain heightmaps and liquid data
3. **VMap Files** - 3D collision geometry for buildings, objects, and line-of-sight calculations
4. **MMap Files** - Navigation meshes for NPC pathfinding


## Database Setup

The server requires **five separate MySQL databases**:

1. **auth** - Authentication and realm information
2. **world** - Game content (NPCs, items, quests, etc.)
3. **characters** - Player characters and account data
4. **logs** - Server logs and statistics
5. **web** - Player-portal sessions, identity tokens, and web-admin audit records

> Note: This database was copied from vmangos, and currently is largely the same however the project will eventually deviate, include all 3 expansions data and I'm thinking of moving to postgres too.


### Setting Up the Database

Create the five databases in MySQL first:

```sql
CREATE DATABASE world CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE auth CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE characters CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE logs CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE web CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

Then run the `db` tool to apply the base schema and any pending migrations:

```bash
# From repo root
cargo run --bin db -- migrate
```

This will apply base tables from `sql/base/<db>/` and then run any pending migrations from `sql/migrations/`.

#### Other db commands

```bash
# Check status of all databases
cargo run --bin db -- status

# Create a new migration file
cargo run --bin db -- new world add_creature_gossip_option
cargo run --bin db -- new characters add_character_pet
cargo run --bin db -- new web add_web_notification

# Show help
cargo run --bin db -- help
```

Migration files are created in `sql/migrations/` with the format `YYYYMMDDHHMMSS_<db>_<name>.sql`.

The tool reads database connection URLs from the same `config.toml` used by the servers. The web
portal requires both its existing auth database and its dedicated web database:

```toml
[web]
auth_database_url = "mysql://root:root@127.0.0.1:3306/auth"
web_database_url = "mysql://root:root@127.0.0.1:3306/web"
```

The `web` database currently has no base dump; its schema is created entirely through migrations.

### PostgreSQL Foundation

PostgreSQL is being introduced independently of the still-MySQL server runtime. The `db pg`
commands initialize one `oxcore` database with separate `auth`, `world`, `characters`, `logs`, and
`web` schemas. They do not translate the MySQL base dumps or switch any running server connection.

```bash
# Start the PostgreSQL development service, then initialize migration metadata and schemas.
podman compose up -d postgres
cargo run -p oxcore-db --bin db -- pg migrate

# Inspect PostgreSQL migration status or reset only application schemas in development.
cargo run -p oxcore-db --bin db -- pg status
cargo run -p oxcore-db --bin db -- pg fresh --yes

# Create a PostgreSQL migration in sql/postgres/migrations/auth/.
cargo run -p oxcore-db --bin db -- pg new auth create_accounts
```

PostgreSQL migrations are schema-specific files named
`sql/postgres/migrations/<schema>/YYYYMMDDHHMMSS_<name>.sql`. Each migration and its ledger entry
are committed in one transaction.

### Build Commands

```bash
# From repo root
# Run BOTH servers in one process with the shared TUI (recommended)
cargo run --bin oxcore

# Run only one server through the unified runtime
cargo run --bin oxcore -- --only auth
cargo run --bin oxcore -- --only world

# Run a single server standalone (also shows the TUI)
cargo run --bin auth
cargo run --bin world

# Run the player portal with SSR, Tailwind, and hydrated Leptos WASM
cargo install --locked cargo-leptos
rustup target add wasm32-unknown-unknown
cargo leptos watch -p oxcore-web

# Disable the TUI (plain stderr/file logging, for systemd/CI/piped logs)
cargo run --bin oxcore -- --headless

# Or do a build
cargo build --release

```

`cargo leptos watch -p oxcore-web` is the normal portal development command. It builds the
Axum SSR server, Tailwind stylesheet, and browser WASM client together; Leptos interactions such
as the password Show/Hide control require this hydrated client. Use `cargo leptos build --release -p oxcore-web` to create the deployable production assets under `target/site/`.

All binaries launch a shared ratatui terminal UI with tabs (**Both / Auth / World /
Performance**), an autosuggesting command input box, live connection/player counts, and
colour-coded logs. On the **Both** tab, prefix a command with `auth:` or `world:` to route
it (bare commands go to world); the Auth/World tabs route automatically. Press `Tab` to
switch tabs, `→` to accept an autosuggestion, `q`/`Ctrl+C` to quit. The UI auto-disables
when stdout is not a TTY, or pass `--headless`.

## Running the Server

### Step 1: Configure the Server

1. Copy the example configuration:
```bash
copy config.toml.example config.toml
```

2. Edit `config.toml` and configure:
   - Database connection URLs
   - Server ports and IP addresses
   - Data directory path (where your DBC/vmap/mmap files are located)
   - Logging settings

### Step 2: Start the Authentication Server

```bash
cargo run --release --bin auth

# Or if already built
target\release\auth.exe
```

The auth server will start on port 3724 (default) and handle client authentication.

Once the auth server is running, use the TUI input box (bottom of the screen) to create accounts and set GM levels:

```text
account create myuser mypassword
account setgm myuser
```

`account setgm` defaults to the maximum GM level (7). You can set it explicitly when creating an account or afterwards:

```text
account create myuser mypassword 7
account setgm myuser 7
```

Type `help` in the TUI input box for other console commands.

### Step 3: Start the World Server

In a separate terminal:

```bash
cargo run --release --bin world

# Or if already built
target\release\world.exe
```

> Tip: instead of running `auth` and `world` in two terminals, run `cargo run --release
> --bin oxcore` to start both in one process and switch between them with the TUI tabs.


### Client Configuration

Configure your WoW client to connect to your server by editing `realmlist.wtf`:

```
set realmlist 127.0.0.1
```

Or modify the `realmlist` table in your `auth` database to set the correct IP address.

## Configuration

The server uses TOML configuration files. See `config.toml.example` for all available options.


### Extracting Data Files

The project includes a Rust-based extractor tool that can extract all required data from your WoW client installation.

> Note: only the dbc extractor is working currently. However the plan is to include a universal extractor to extract all of the necessary files to run the server. In the meantime use another emulators map extraction tools.

#### Using the Extractor Tool

1. **Build the extractor**:
```bash
# Build or cargo run the extactor
cd tools/extractor
cargo build --release

# Extract everything to ./output directory
extractor all -i "C:\Games\WoWFolder" -o "./output"

# Or extract to your server's data directory
extractor all -i "C:\Games\WoWFolder" -o "C:\path\to\server\data"

# Extract only DBC files
extractor dbc -i "C:\Games\WoW\Data" -o "./output"

```

## Credits & Acknowledgments

A large portion of this project project has been directly ported from MaNGOS. The original MaNGOS project and its various forks have been instrumental in understanding WoW server architecture and implementing this Rust version. I want to make it super clear that this project would never have got anywhere without it, all of the contributers to that project over the years have made this possible.

### License

This project follows the GPL-2.0 license.
