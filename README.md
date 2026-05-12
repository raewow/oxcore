# rcore - World of Warcraft Vanilla Emulation Server

A World of Warcraft (Vanilla 1.12.x) private server implementation written in Rust. 

## Server Goals 

While the server is still in early development, the long term vision for this server is one that will work for vanilla, tbc and wrath client depending on configuration. The other main goal for this server is to work with both the older clients as well as the modern version. 

## Current Features
- Auth Server
- World Server
  - Character Creation
  - NPCS
  - Combat
  - Spell and Aura System
 - Map System
  - Questing
  - Items and Equipment

### Data Files

The server requires extracted game data files from the WoW client. You must extract:

1. **DBC Files** - Database Client files containing game definitions (spells, items, areas, etc.)
2. **Map Files** - Terrain heightmaps and liquid data
3. **VMap Files** - 3D collision geometry for buildings, objects, and line-of-sight calculations
4. **MMap Files** - Navigation meshes for NPC pathfinding


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

## Database Setup

The server requires **four separate MySQL databases**:

1. **auth** - Authentication and realm information
2. **world** - Game content (NPCs, items, quests, etc.)
3. **characters** - Player characters and account data
4. **logs** - Server logs and statistics

> Note: This database was copied from vmangos, and currently is largely the same however the project will eventually deviate, include all 3 expansions data and I'm thinking of moving to postgres too.


### Setting Up the Database

Create the four databases in MySQL first:

```sql
CREATE DATABASE world CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE auth CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE characters CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE DATABASE logs CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
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

# Show help
cargo run --bin db -- help
```

Migration files are created in `sql/migrations/` with the format `YYYYMMDDHHMMSS_<db>_<name>.sql`.

The tool reads database connection URLs from the same `config.toml` used by the auth and world servers.

### Build Commands

```bash
# From repo root
# Run the binaries
cargo run --bin auth
cargo run --bin world

# Or do a build
cargo build --release

```

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

### Step 3: Start the World Server

In a separate terminal:

```bash
cargo run --release --bin world

# Or if already built
target\release\world.exe
```


### Client Configuration

Configure your WoW client to connect to your server by editing `realmlist.wtf`:

```
set realmlist 127.0.0.1
```

Or modify the `realmlist` table in your `auth` database to set the correct IP address.

## Configuration

The server uses TOML configuration files. See `config.toml.example` for all available options.

## Credits & Acknowledgments

A large portion of this project project has been directly ported from MaNGOS. The original MaNGOS project and its various forks have been instrumental in understanding WoW server architecture and implementing this Rust version. I want to make it super clear that this project would never have got anywhere without it, all of the contributers to that project over the years have made this possible.

### License

This project follows the GPL-2.0 license.

