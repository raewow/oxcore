use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::core::lua::LuaScriptManager;
use crate::core::session::SessionManager;
use crate::database::repositories::QuestTemplateRepository;
use crate::dbc::DbcManager;
use crate::game::area_trigger::AreaTriggerManager;
use crate::game::auction::AuctionHouseManager;
use crate::game::broadcast_mgr::BroadcastManager;
use crate::game::conditions::ConditionManager;
use crate::game::coordination::{LinkingManager, PoolManager};
use crate::game::corpse::CorpseManager;
use crate::game::creature::{AddonManager, CreatureManager, WaypointManager};
use crate::game::gameobject::GameObjectManager;
use crate::game::items::{HotfixStore, ItemManager};
use crate::game::player::{PlayerManager, PlayerSystem};
use crate::game::spell::SpellManager;
use crate::game::SystemManager;
use crate::map::manager::MapManager;
use crate::map::pathfinding::vmap::{VMapConfig, VMapManager};
use crate::map::pathfinding::{MMapManager, PathFinder};
use crate::map::terrain::TerrainManager;
use oxcore_db::database::characters::repositories::{
    AuctionRepository, AuctionRepositoryTrait, CharacterRepository, ItemRepository,
    ItemRepositoryTrait, MailRepository, MailRepositoryTrait,
};
use oxcore_db::database::Databases;
use oxcore_shared::console::{CommandRegistry, ConsoleCommand};
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::RwLock;
use tokio::sync::RwLock as TokioRwLock;

/// Live update-loop performance stats, surfaced to the TUI Performance tab.
#[derive(Default, Clone)]
pub struct TickStats {
    /// Duration of the last full update() in milliseconds.
    pub last_tick_ms: f64,
    /// Exponential moving average of ticks per second.
    pub tps: f64,
    /// Per-phase timings for the last tick: (name, milliseconds).
    pub phases: Vec<(String, f64)>,
    /// Instant of the previous tick (for TPS calculation).
    pub last_update: Option<std::time::Instant>,
}

pub struct Managers {
    // TODO moves these out of this struct
    pub player_mgr: Arc<PlayerManager>,
    pub creature_mgr: Arc<CreatureManager>,
    pub gameobject_mgr: Arc<GameObjectManager>,
    pub corpse_mgr: Arc<CorpseManager>,
    pub item_mgr: Arc<ItemManager>,
    /// DB2 records the 1.14 client asks for by table and record id; empty for vanilla-only servers.
    pub hotfix_store: Arc<HotfixStore>,
    pub map_mgr: Arc<MapManager>,
    pub broadcast_mgr: Arc<BroadcastManager>,
    pub pool_mgr: Arc<PoolManager>,
    pub linking_mgr: Arc<LinkingManager>,
    pub addon_mgr: Arc<AddonManager>,
    pub vmap_mgr: Arc<VMapManager>,
    pub mmap_mgr: Arc<MMapManager>,
    pub terrain_mgr: Arc<TerrainManager>,
    pub pathfinder: Arc<PathFinder>,
    pub waypoint_mgr: Arc<WaypointManager>,
    pub area_trigger_mgr: Arc<AreaTriggerManager>,
    pub taxi_mgr: Arc<crate::game::taxi::TaxiManager>,
    pub battleground_mgr: Arc<crate::game::battleground::BattlegroundManager>,
    pub scope_states: Arc<crate::game::pvp::ScopeStates>,
    pub bg_events: Arc<crate::game::pvp::events::EventIndex>,
    pub condition_mgr: Arc<ConditionManager>,
    pub instance_mgr: Arc<crate::game::instance::InstanceMgr>,
    pub game_event_mgr: Arc<crate::game::game_event::GameEventManager>,
    pub spell_mgr: Arc<SpellManager>,
    pub lua_mgr: Arc<LuaScriptManager>,
    pub auction_mgr: Arc<AuctionHouseManager>,
}

pub struct World {
    pub managers: Managers,
    // TODO rename this to game
    pub systems: Arc<SystemManager>,
    pub session_mgr: Arc<SessionManager>,
    pub databases: Arc<Databases>,
    pub config: Arc<Config>,
    pub dbc: Arc<RwLock<DbcManager>>,
    pub update_interval: Duration,
    pub realm_id: std::sync::Arc<std::sync::atomic::AtomicI32>,
    pub shutdown_rx: std::sync::Arc<std::sync::Mutex<Option<tokio::sync::broadcast::Receiver<()>>>>,
    pub running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub console_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ConsoleCommand>>>,
    pub command_registry: Arc<TokioRwLock<CommandRegistry<World>>>,
    background_tasks: Arc<std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    /// Live update-loop performance stats for the TUI.
    pub tick_stats: Arc<parking_lot::Mutex<TickStats>>,
    /// Optional startup progress reporter, driven by `init()` for the TUI loading screen.
    pub progress: Arc<parking_lot::Mutex<Option<oxcore_tui::Progress>>>,
    /// Per-player packet handlers
    pub player_handlers: Arc<
        dashmap::DashMap<ObjectGuid, crate::core::network::player_handler::PlayerPacketHandler>,
    >,
    /// Per-player movement buffers (socket writes, map update reads)
    pub movement_buffers: Arc<
        dashmap::DashMap<ObjectGuid, Arc<crate::core::network::movement_buffer::MovementBuffer>>,
    >,
    pub data_dir: PathBuf,
}

impl World {
    /// The generation stamp on every DB2 hotfix reply.
    ///
    /// The client caches records against it and re-fetches when it changes. Our rows come from
    /// `item_template`, which is read once at startup and never revised while the world runs, so the
    /// process start time is both stable within a session and new after a restart — which is exactly
    /// when a row may have changed underneath a client that cached it.
    pub fn hotfix_timestamp(&self) -> u32 {
        static STARTED_AT: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *STARTED_AT.get_or_init(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs() as u32)
        })
    }

    pub fn new(
        databases: Arc<Databases>,
        config: Arc<Config>,
        update_interval_ms: u32,
        data_dir: std::path::PathBuf,
    ) -> Self {
        let session_mgr = Arc::new(SessionManager::new());
        let player_mgr = Arc::new(PlayerManager::new());
        let player_system = Arc::new(PlayerSystem::new(Arc::clone(&player_mgr)));
        let creature_mgr = Arc::new(CreatureManager::new(Arc::new(databases.world.clone())));
        let gameobject_mgr = Arc::new(GameObjectManager::new(Arc::new(databases.world.clone())));
        let corpse_mgr = Arc::new(CorpseManager::new());
        let item_mgr = Arc::new(ItemManager::new());
        let hotfix_store = Arc::new(HotfixStore::load(&data_dir));
        let map_mgr = Arc::new(MapManager::new());

        // Create VMap manager for collision/LOS
        let vmap_config = VMapConfig {
            enable_los: config.vmap_enable_los,
            enable_height: config.vmap_enable_height,
            enable_indoor_check: config.vmap_enable_indoor_check,
        };
        let vmap_mgr = Arc::new(VMapManager::new(&data_dir, vmap_config));

        // Terrain (.map) data: area ids, height mesh, and the liquid layer
        let terrain_mgr = Arc::new(TerrainManager::new(&data_dir));

        // Create MMap manager for navigation mesh pathfinding
        let mmap_mgr = Arc::new(MMapManager::new(&data_dir, Arc::clone(&vmap_mgr)));

        // Create PathFinder (integrates VMap LOS + NavMesh A* + obstacle avoidance)
        let pathfinder = Arc::new(PathFinder::with_terrain(
            Arc::clone(&mmap_mgr),
            Arc::clone(&terrain_mgr),
        ));

        let broadcast_mgr = Arc::new(BroadcastManager::new(
            Arc::clone(&session_mgr),
            Arc::clone(&player_mgr),
        ));
        let dbc = Arc::new(RwLock::new(DbcManager::new()));

        // Coordination managers
        let pool_mgr = Arc::new(PoolManager::new());
        let linking_mgr = Arc::new(LinkingManager::new());
        let addon_mgr = Arc::new(AddonManager::new());
        let waypoint_mgr = Arc::new(WaypointManager::new());
        let area_trigger_mgr = Arc::new(AreaTriggerManager::new(Arc::new(databases.world.clone())));
        let condition_mgr = Arc::new(ConditionManager::new());
        let instance_mgr = Arc::new(crate::game::instance::InstanceMgr::new());

        // Initialize Lua scripting system
        let lua_mgr = Arc::new(LuaScriptManager::new(&data_dir));

        let character_pool = Arc::new(databases.character.clone());
        let world_pool = Arc::new(databases.world.clone());

        let auction_repo: Arc<dyn AuctionRepositoryTrait> =
            Arc::new(AuctionRepository::new(Arc::clone(&character_pool)));
        let mail_repo: Arc<dyn MailRepositoryTrait> =
            Arc::new(MailRepository::new(Arc::clone(&character_pool)));
        let item_repo: Arc<dyn ItemRepositoryTrait> =
            Arc::new(ItemRepository::new(Arc::clone(&character_pool)));
        let auction_mgr = Arc::new(AuctionHouseManager::new(
            auction_repo,
            Arc::new(CharacterRepository::new(Arc::clone(&character_pool))),
            mail_repo,
            item_repo,
            Arc::clone(&dbc),
            Arc::clone(&item_mgr),
        ));

        let systems = Arc::new(SystemManager::new(
            Arc::clone(&character_pool),
            Arc::clone(&world_pool),
            Arc::new(databases.logs.clone()),
            config.chat_log_enabled,
            Arc::clone(&broadcast_mgr),
            Arc::clone(&item_mgr),
            Arc::clone(&player_system),
            Arc::clone(&creature_mgr),
            Arc::clone(&pool_mgr),
            Arc::clone(&linking_mgr),
            Arc::clone(&addon_mgr),
            config.allow_cross_faction_group,
        ));

        use crate::console::commands::register_all_commands;
        let mut command_registry = CommandRegistry::new();
        register_all_commands(&mut command_registry);

        Self {
            managers: Managers {
                player_mgr: systems.player.manager(),
                creature_mgr,
                gameobject_mgr,
                corpse_mgr,
                item_mgr,
                hotfix_store,
                map_mgr,
                broadcast_mgr,
                pool_mgr,
                linking_mgr,
                addon_mgr,
                vmap_mgr,
                terrain_mgr,
                mmap_mgr,
                pathfinder,
                waypoint_mgr,
                area_trigger_mgr,
                taxi_mgr: Arc::new(crate::game::taxi::TaxiManager::new()),
                battleground_mgr: Arc::new(crate::game::battleground::BattlegroundManager::new()),
                scope_states: Arc::new(crate::game::pvp::ScopeStates::new()),
                bg_events: Arc::new(crate::game::pvp::events::EventIndex::new()),
                condition_mgr,
                instance_mgr,
                game_event_mgr: Arc::new(crate::game::game_event::GameEventManager::new()),
                spell_mgr: Arc::new(SpellManager::new()),
                lua_mgr,
                auction_mgr,
            },
            systems,
            session_mgr,
            databases,
            config,
            dbc,
            update_interval: Duration::from_millis(update_interval_ms as u64),
            realm_id: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(1)),
            shutdown_rx: std::sync::Arc::new(std::sync::Mutex::new(None)),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            console_rx: Arc::new(tokio::sync::Mutex::new(tokio::sync::mpsc::channel(1).1)),
            command_registry: Arc::new(TokioRwLock::new(command_registry)),
            background_tasks: Arc::new(std::sync::Mutex::new(Vec::new())),
            tick_stats: Arc::new(parking_lot::Mutex::new(TickStats::default())),
            progress: Arc::new(parking_lot::Mutex::new(None)),
            player_handlers: Arc::new(dashmap::DashMap::new()),
            movement_buffers: Arc::new(dashmap::DashMap::new()),
            data_dir,
        }
    }

    pub async fn init(&self) -> Result<()> {
        // Startup progress reporting for the TUI loading screen (no-op when unset).
        let progress = self.progress.lock().clone();
        let step = |label: &str| {
            if let Some(p) = &progress {
                p.step(label);
            }
        };
        if let Some(p) = &progress {
            p.set_total(11);
        }

        // Load DBC data files
        step("Loading DBC files");
        tracing::info!("Loading DBC files...");
        {
            let dbc_path = self.data_dir.join("dbc");
            let mut dbc = self.dbc.write();
            dbc.load_all(dbc_path.to_str().unwrap())
                .context("Failed to load DBC files")?;
        }

        self.load_map_templates()
            .await
            .context("Failed to load map templates")?;
        self.install_map_config_provider();
        // Load spells from SQL (DBC field offsets are unreliable for vanilla 1.12.1)
        self.managers
            .condition_mgr
            .load(&self.databases.world)
            .await
            .context("Failed to load conditions")?;
        // Spell-area loading validates quest requirements against the loaded templates.
        step("Loading quests");
        QuestTemplateRepository::load(&self.systems.quest_manager, &self.databases.world)
            .await
            .context("Failed to load quest data")?;
        step("Loading spells");
        {
            let dbc = self.dbc.read();
            self.managers
                .spell_mgr
                .load(&self.databases.world, &dbc, &self.systems.quest_manager)
                .await
                .context("Failed to load spells from SQL")?;
            self.managers
                .spell_mgr
                .load_spell_chains(&self.databases.world, &dbc)
                .await
                .context("Failed to load spell chains")?;
        }

        // Initialize GUID generators from database
        step("Loading items & GUIDs");
        self.managers
            .player_mgr
            .init_guid_generator(&self.databases.character)
            .await
            .context("Failed to initialize player GUID generator")?;

        // Load item templates
        self.managers
            .item_mgr
            .load_item_templates(&self.databases.world)
            .await?;

        // Load item required-target rules (item_required_target)
        self.managers
            .item_mgr
            .load_item_required_targets(&self.databases.world)
            .await?;

        // Initialize item GUID generator from database
        self.managers
            .item_mgr
            .init_guid_generator(&self.databases.character)
            .await?;

        // Load auction house maps and auction data (requires DBC + item templates)
        step("Loading auction houses");
        let wow_patch = self.config.wow_patch.unwrap_or(112);
        self.managers
            .auction_mgr
            .load_auction_houses(
                self.config.allow_cross_faction_auction,
                self.config.unlinked_auction_houses,
                wow_patch,
            )
            .context("Failed to load auction house maps")?;
        self.managers
            .auction_mgr
            .load_auction_items()
            .await
            .context("Failed to load auction items")?;
        self.managers
            .auction_mgr
            .load_auctions()
            .await
            .context("Failed to load auctions")?;

        // Load creature templates and spawns
        step("Loading creatures");
        self.managers.creature_mgr.load_templates().await?;
        self.managers
            .creature_mgr
            .load_display_scales(&self.dbc.read());
        self.managers.creature_mgr.load_model_info().await?;

        let creature_patch = self.content_patch();
        self.managers.creature_mgr.set_patch(creature_patch);

        self.managers.creature_mgr.load_spawns().await?;

        // Load gameobject templates and spawns
        step("Loading gameobjects");
        self.managers.gameobject_mgr.set_patch(creature_patch);
        self.managers.gameobject_mgr.load_templates().await?;
        if let Err(e) = self.managers.gameobject_mgr.load_display_bounds().await {
            tracing::warn!("{e:#} (game objects are measured from their centre)");
        }
        self.managers.gameobject_mgr.load_spawns().await?;

        // World events decide which event-linked spawns exist.
        self.managers
            .game_event_mgr
            .load(&self.databases.world, creature_patch)
            .await
            .context("Failed to load game events")?;

        // Load waypoint data
        step("Loading waypoints & loot");
        use crate::game::creature::movement::waypoint_repository::WaypointRepository;
        let waypoint_repo = WaypointRepository::new(self.databases.world.clone());
        let waypoint_data = waypoint_repo
            .load_all()
            .await
            .context("Failed to load waypoint data")?;
        self.managers.waypoint_mgr.load_from_data(waypoint_data);

        // Load loot tables
        self.systems
            .loot_manager
            .load_loot_tables(&self.databases.world)
            .await
            .context("Failed to load loot tables")?;
        self.systems
            .loot_manager
            .load_gameobject_loot_tables(&self.databases.world)
            .await
            .context("Failed to load gameobject loot tables")?;

        // Load spawn pools and roll their initial rosters. Must run after
        // creature and gameobject spawns are loaded — pool members reference
        // spawn ids — and before grids load, which consults the rosters.
        {
            use crate::game::coordination::PoolRepository;
            let pool_repo =
                PoolRepository::new(self.databases.world.clone()).with_patch(creature_patch);
            match pool_repo.load_all_pools().await {
                Ok(pool_data) => {
                    self.managers.pool_mgr.load_from_repository(pool_data);
                    self.systems.pool.initialize();
                }
                Err(e) => {
                    tracing::error!("Failed to load spawn pools: {} (pools disabled)", e);
                }
            }
        }

        // Creature linking: spawns and entries that react to their master.
        {
            use crate::game::coordination::LinkingRepository;
            let repo = LinkingRepository::new(self.databases.world.clone());
            match (repo.load_all_links().await, repo.load_all_entry_links().await) {
                (Ok(links), Ok(entry_links)) => {
                    self.managers.linking_mgr.load_from_repository(links);
                    self.managers.linking_mgr.load_entry_links(entry_links);
                    tracing::info!(
                        "Loaded {} creature links",
                        self.managers.linking_mgr.link_count()
                    );
                }
                (Err(e), _) | (_, Err(e)) => {
                    tracing::error!("Failed to load creature linking: {} (linking disabled)", e);
                }
            }
        }

        match crate::game::player::exploration::load_base_xp(&self.databases.world).await {
            Ok(count) => tracing::info!("Loaded {} exploration experience levels", count),
            Err(e) => tracing::error!("Failed to load exploration experience: {e:#}"),
        }
        match crate::game::db_scripts::load(&self.databases.world).await {
            Ok(count) => tracing::info!("Loaded {} database script steps", count),
            Err(e) => tracing::error!("Failed to load database scripts: {}", e),
        }
        match crate::game::creature::ai::event_ai::load(&self.databases.world).await {
            Ok(count) => tracing::info!("Loaded {} EventAI events", count),
            Err(e) => tracing::error!("Failed to load EventAI events: {}", e),
        }
        match crate::game::creature::formation::load(&self.databases.world).await {
            Ok(count) => tracing::info!("Loaded {} creature group members", count),
            Err(e) => tracing::error!("Failed to load creature groups: {} (groups disabled)", e),
        }

        // Load zone weather chances (`game_weather`)
        if let Err(e) = self
            .systems
            .weather_manager
            .load(&self.databases.world)
            .await
        {
            tracing::error!("Failed to load weather data: {} (weather disabled)", e);
        }

        // Load area trigger data
        step("Loading area triggers & instances");
        self.managers
            .area_trigger_mgr
            .load(self.content_patch())
            .await
            .context("Failed to load area triggers")?;

        // Flight paths. Paths only scripted spells fly are kept off the
        // flight-master network.
        let spell_flight_paths: std::collections::HashSet<u32> = self
            .managers
            .spell_mgr
            .effect_misc_values(crate::game::taxi::SPELL_EFFECT_SEND_TAXI)
            .into_iter()
            .filter_map(|path| u32::try_from(path).ok())
            .collect();
        self.managers
            .taxi_mgr
            .load(&self.databases.world, &self.dbc, &spell_flight_paths)
            .await
            .context("Failed to load taxi network")?;

        // Battlegrounds: templates, battlemasters, entrances and event spawns.
        crate::game::battleground::system::load(self)
            .await
            .context("Failed to load battlegrounds")?;
        // Catch up on any weekly honour ranking missed while the server was down.
        if let Err(e) = crate::game::player::honor::weekly_maintenance(self).await {
            tracing::warn!("Honour maintenance failed: {}", e);
        }

        // Initialize instance manager: id allocation, the raid reset schedule,
        // expired/unbound saves, then the respawn deadlines that survive.
        self.managers
            .instance_mgr
            .initialize(&self.databases)
            .await
            .context("Failed to initialize instance manager")?;
        {
            let raid_periods: std::collections::HashMap<u32, u32> = self
                .dbc
                .read()
                .map
                .entries()
                .filter(|(_, entry)| entry.is_raid())
                .map(|(id, entry)| (*id, entry.reset_delay.max(1)))
                .collect();
            self.managers
                .instance_mgr
                .configure_raids(raid_periods, self.config.instance_reset_hour);
        }
        self.managers
            .instance_mgr
            .startup(&self.databases)
            .await
            .context("Failed to clean up instance saves")?;
        crate::game::common::respawn_persistence::load_respawn_times(self)
            .await
            .context("Failed to load respawn times")?;

        // Initialize Lua scripting
        step("Loading Lua scripts");
        match self.managers.lua_mgr.initialize() {
            Ok(result) => {
                let stats = self.managers.lua_mgr.stats();
                tracing::info!(
                    "Loaded {} Lua scripts ({} creature AI, {} instance, {} zone, {} gossip)",
                    result.loaded,
                    stats.creature_ai,
                    stats.instance,
                    stats.zone,
                    stats.gossip,
                );
                if !result.errors.is_empty() {
                    for error in &result.errors {
                        tracing::error!("Lua script error: {}", error);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Lua scripting initialization failed: {} (continuing without scripts)",
                    e
                );
            }
        }

        // Initialize game systems
        step("Initializing systems");
        self.systems.init_all().await?;
        self.systems
            .gossip_manager
            .drop_missing_conditions(&self.managers.condition_mgr);
        if let Err(e) = self
            .systems
            .guild
            .petitions
            .load(&self.databases.character)
            .await
        {
            tracing::error!("Failed to load guild charters: {e:#}");
        }
        if let Err(e) = self.systems.pet.load(&self.databases.character).await {
            tracing::error!("Failed to load hunter pet numbering: {e:#}");
        }

        // Load graveyard data (needs both DBC and world DB)
        {
            let dbc = self.dbc.read();
            self.systems
                .death
                .load_graveyards(
                    std::sync::Arc::new(self.databases.world.clone()),
                    &dbc,
                    self.content_patch(),
                )
                .await?;
        }

        // Register vendor template IDs from creature templates
        // This must happen after both creature_mgr.load_templates() and vendor_manager.load()
        for entry in self.managers.creature_mgr.all_templates() {
            if entry.vendor_id > 0 {
                self.systems
                    .vendor_manager
                    .register_creature_vendor_template(entry.entry, entry.vendor_id);
            }
            if entry.trainer_id > 0 {
                self.systems
                    .trainer_manager
                    .register_creature_trainer_template(entry.entry, entry.trainer_id);
            }
        }

        if let Some(p) = &progress {
            p.finish();
        }

        Ok(())
    }

    /// Run main update loop
    pub async fn run(&self) -> Result<()> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut interval = tokio::time::interval(self.update_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while self.running.load(std::sync::atomic::Ordering::SeqCst) {
            interval.tick().await;
            self.update().await?;
        }

        tracing::info!("World update loop stopped");
        Ok(())
    }

    async fn update(&self) -> Result<()> {
        let tick_start = std::time::Instant::now();
        let diff = self.update_interval;
        let diff_ms = diff.as_millis() as u32;

        // --- Phase: grid loading/unloading ---
        let phase_grid = std::time::Instant::now();
        // Process grid loading/unloading for all active maps (continents and instances)
        // This handles lazy creature spawning based on player proximity
        for (map_id, instance_id) in self.managers.map_mgr.get_active_map_keys() {
            if let Err(e) = self
                .systems
                .grid
                .process_map_grids(map_id, instance_id, self)
                .await
            {
                tracing::error!(
                    "Grid processing failed for map {}:{}: {}",
                    map_id,
                    instance_id,
                    e
                );
            }
        }
        let grid_ms = phase_grid.elapsed().as_secs_f64() * 1000.0;

        // --- Phase: movement + maps ---
        let phase_maps = std::time::Instant::now();
        // Process creature movement FIRST so positions are current for combat
        // and AI range checks (unit movement update runs before AI)
        self.systems
            .creature_movement
            .update_creatures(diff_ms, self)?;

        crate::game::map_update::update_all_maps(&self.managers.map_mgr, diff, self).await?;

        // Flush queued packets (packet collapsing optimization)
        // Movement packets accumulated during map updates are sent here with collapsing applied
        self.managers.broadcast_mgr.flush_all_queues();
        let maps_ms = phase_maps.elapsed().as_secs_f64() * 1000.0;

        // --- Phase: systems (general + spells + auras) ---
        let phase_systems = std::time::Instant::now();
        self.systems.update_all(diff, self)?;

        // Update spell cast timers for all players
        self.systems.spells.update_all_casts(diff, self).await?;

        // Update aura durations and periodic effects for all players
        self.systems.auras.update_all_auras(diff, self).await?;

        // Tick Lua zone scripts (world events, outdoor PvP objectives)
        if let Err(e) = crate::core::lua::update_zone_scripts(diff, self).await {
            tracing::error!("Zone script update error: {}", e);
        }
        let systems_ms = phase_systems.elapsed().as_secs_f64() * 1000.0;

        // --- Phase: creatures (deaths/respawn/combat/ai/regen) ---
        let phase_creatures = std::time::Instant::now();
        // Process creature deaths (Phase 3)
        crate::game::creature::death::process_deaths(self).await?;

        // Process corpse decay (Phase 3)
        crate::game::creature::death::process_corpse_decay(self, diff_ms).await?;

        // Process respawns (Phase 6)
        self.systems.creature_respawn.process_respawns(self).await?;
        crate::game::gameobject::system::update_respawns(self);

        // Process creature combat timers and melee attacks
        // Timer countdown + attack execution in one pass
        crate::game::creature::combat_update::update_creature_combat(self, diff_ms);

        // Process AI updates (after combat timers so AI sees current timer state)
        crate::game::creature::ai::update_creature_ai(self)?;

        // Scheduled database script steps.
        crate::game::db_scripts::process(self);

        // Temporary summons despawn on their own schedule.
        crate::game::creature::summon::update_summons(self, diff_ms);
        crate::game::creature::summon::update_temporary_gameobjects(self, diff_ms);

        // Process creature regeneration (health + mana)
        crate::game::creature::regen::update_regeneration(self, diff_ms);
        let creatures_ms = phase_creatures.elapsed().as_secs_f64() * 1000.0;

        // Process timed quest expiry
        self.systems.quest.update_quest_timers(diff_ms, self);

        // Flush dirty inventory data to DB periodically (every 20 ticks = ~1 second at 50ms)
        static INVENTORY_SAVE_COUNTER: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);
        let save_count = INVENTORY_SAVE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if save_count % 20 == 0 {
            // Reduced from 100 (5s) to 20 (1s) to reduce data loss window
            if let Err(e) = self.systems.inventory.flush_pending_ops().await {
                tracing::error!("Failed to flush inventory ops: {}", e);
            }
        }

        // Auction expiry tick - every 1200 ticks (~60s at 50ms)
        static AUCTION_UPDATE_COUNTER: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);
        let auction_count =
            AUCTION_UPDATE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if auction_count % 1200 == 0 {
            if let Err(e) = self.managers.auction_mgr.update().await {
                tracing::error!("Auction update failed: {}", e);
            }
        }

        // Aggro scanning (Phase 5) - every 4th tick for performance
        static AGGRO_SCAN_COUNTER: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);
        let count = AGGRO_SCAN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 4 == 0 {
            crate::game::creature::ai::scan_for_aggro(self).await?;
        }

        // Dungeon lifecycle - about once a second: reset schedule, raid warnings,
        // removal of unsaved players, and teardown of empty copies.
        static INSTANCE_LIFECYCLE_COUNTER: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(0);
        let lifecycle_count =
            INSTANCE_LIFECYCLE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if lifecycle_count % 20 == 0 {
            if let Err(e) = crate::game::instance::lifecycle::update(self).await {
                tracing::error!("Instance lifecycle update failed: {}", e);
            }
            // World events starting or stopping add or remove their spawns.
            crate::game::game_event::update(self);
            // Formation members fall in behind their leaders.
            crate::game::creature::formation::update_formations(self);
        }

        // Players on a flight move along it.
        crate::game::taxi::flight::update(self).await?;

        // Battleground queues and matches; PvP flags count down.
        crate::game::battleground::system::update(self, diff_ms).await;
        crate::game::pvp::flag::update(self, diff_ms);

        // The weekly honour ranking: checked about once an hour.
        static HONOR_MAINTENANCE_COUNTER: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(1);
        if HONOR_MAINTENANCE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 72_000 == 0 {
            if let Err(e) = crate::game::player::honor::weekly_maintenance(self).await {
                tracing::warn!("Honour maintenance failed: {}", e);
            }
        }

        self.session_mgr.update_logout_timers(self).await?;

        // Keep the 1.14 client's clock reference fresh (periodic SMSG_TIME_SYNC_REQUEST).
        self.session_mgr.update_time_sync();

        if let Ok(_) = self
            .command_registry
            .read()
            .await
            .process_commands(&self.console_rx, self)
            .await
        {
            // Console processed successfully
        }

        // Record performance stats for this tick (consumed by the TUI Performance tab).
        {
            let now = std::time::Instant::now();
            let total_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
            let mut stats = self.tick_stats.lock();
            let instant_tps = match stats.last_update {
                Some(prev) => {
                    let dt = now.duration_since(prev).as_secs_f64();
                    if dt > 0.0 {
                        1.0 / dt
                    } else {
                        0.0
                    }
                }
                None => 0.0,
            };
            stats.tps = if stats.tps > 0.0 {
                stats.tps * 0.9 + instant_tps * 0.1
            } else {
                instant_tps
            };
            stats.last_tick_ms = total_ms;
            stats.last_update = Some(now);
            stats.phases = vec![
                ("grid".to_string(), grid_ms),
                ("maps".to_string(), maps_ms),
                ("systems".to_string(), systems_ms),
                ("creatures".to_string(), creatures_ms),
            ];
        }

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        // Save creature respawn states before shutting down systems
        self.systems.grid.save_all_creature_states(self);

        self.systems.shutdown_all().await?;

        // Abort all background tasks
        let mut tasks = self.background_tasks.lock().unwrap();
        for task in tasks.drain(..) {
            task.abort();
        }

        // Wait a bit for tasks to finish
        tokio::time::sleep(tokio::time::Duration::from_millis(100));

        Ok(())
    }

    /// Teach the map manager what kind of map each id is, and how far players see
    /// on it.
    ///
    /// Maps are created lazily from dozens of call sites with no access to DBC or
    /// config, so the kind table is snapshotted once here and moved into the
    /// resolver. Capturing the DBC handle instead would mean taking its lock
    /// during map creation — which can happen while DBC is already write-locked.
    /// Content patch used to pick progressive-patch rows: `wow_patch` 102
    /// (1.2) is patch 0, each later minor version adds one, 112 is patch 10.
    pub(crate) fn content_patch(&self) -> u8 {
        content_patch_for(self.config.wow_patch.unwrap_or(112))
    }

    /// Merge `world.map_template` into the DBC map table.
    ///
    /// Vanilla `Map.dbc` has no player cap, reset period, parent or ghost entrance,
    /// so instance admission, raid resets and ghost routing all depend on these
    /// rows. Each map uses its newest row at or below the configured patch.
    async fn load_map_templates(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, (i32, i64, i16, i16, i64, i16, f32, f32, String)>(
            "SELECT DISTINCT ON (entry) entry, parent, map_type, player_limit, reset_delay,
                    ghost_entrance_map, ghost_entrance_x, ghost_entrance_y, map_name
             FROM world.map_template
             WHERE patch <= $1
             ORDER BY entry, patch DESC",
        )
        .bind(i16::from(self.content_patch()))
        .fetch_all(&self.databases.world)
        .await?;

        let mut dbc = self.dbc.write();
        let count = rows.len();
        for (
            entry,
            parent,
            map_type,
            player_limit,
            reset_delay,
            ghost_map,
            ghost_x,
            ghost_y,
            name,
        ) in rows
        {
            dbc.apply_map_template(crate::dbc::structures::MapEntry {
                id: u32::try_from(entry)?,
                map_type: u32::try_from(map_type)?,
                max_players: u32::try_from(player_limit)?,
                reset_delay: u32::try_from(reset_delay)?,
                parent: u32::try_from(parent)?,
                ghost_entrance_map: i32::from(ghost_map),
                ghost_entrance_x: ghost_x,
                ghost_entrance_y: ghost_y,
                name,
            });
        }
        tracing::info!("Loaded {} map templates", count);
        Ok(())
    }

    fn install_map_config_provider(&self) {
        use crate::map::{MapConfig, MapKind};
        use std::collections::HashMap;

        let kinds: HashMap<u32, MapKind> = {
            let dbc = self.dbc.read();
            dbc.map
                .entries()
                .map(|(id, entry)| (*id, MapKind::from_dbc_map_type(entry.map_type)))
                .collect()
        };

        let continents = self.config.visibility_distance_continents;
        let instances = self.config.visibility_distance_instances;
        let battlegrounds = self.config.visibility_distance_bg;
        let min_distance = self.config.visibility_distance_min;
        let unload_delay = std::time::Duration::from_secs(self.config.grid_unload_delay_secs);

        tracing::info!(
            "Map visibility: continents {:.0}y, instances {:.0}y, battlegrounds {:.0}y ({} maps known)",
            continents,
            instances,
            battlegrounds,
            kinds.len()
        );

        self.managers
            .map_mgr
            .set_config_provider(std::sync::Arc::new(move |map_id: u32, _instance: u32| {
                let kind = kinds.get(&map_id).copied().unwrap_or(MapKind::Continent);
                let distance = match kind {
                    MapKind::Continent => continents,
                    MapKind::Dungeon | MapKind::Raid => instances,
                    MapKind::BattleGround => battlegrounds,
                };

                let mut config = MapConfig::for_kind(kind).with_visibility_distance(distance);
                config.min_visibility_distance = min_distance.min(distance);
                config.min_grid_activation_distance = min_distance.min(distance);
                config.grid_unload_delay = unload_delay;
                config
            }));
    }

    pub fn setup_logging(&self, config: &crate::config::Config) -> Result<()> {
        crate::logging::init_logging(config)
    }

    pub async fn set_shutdown_receiver(&self, rx: tokio::sync::broadcast::Receiver<()>) {
        let mut shutdown_rx = self.shutdown_rx.lock().unwrap();
        *shutdown_rx = Some(rx);
    }

    pub fn start_realm_heartbeat(&self) {
        let heartbeat_interval = crate::config::get_config_mgr()
            .get()
            .realm_heartbeat_interval;
        let heartbeat_repo = oxcore_db::database::RealmRepository::new(std::sync::Arc::new(
            self.databases.auth.clone(),
        ));
        let heartbeat_realm_id = self.get_realm_id();

        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval));
            loop {
                interval.tick().await;
                if let Err(e) = heartbeat_repo.heartbeat(heartbeat_realm_id).await {
                    tracing::error!("Failed to update realm heartbeat: {}", e);
                }
            }
        });

        let mut tasks = self.background_tasks.lock().unwrap();
        tasks.push(handle);

        tracing::debug!(
            "Realm heartbeat task started (interval: {}s)",
            heartbeat_interval
        );
    }

    pub async fn set_console_receiver(&self, rx: tokio::sync::mpsc::Receiver<ConsoleCommand>) {
        *self.console_rx.lock().await = rx;
    }

    /// Install a startup progress reporter (driven by `init()` for the TUI loading screen).
    pub fn set_progress(&self, progress: oxcore_tui::Progress) {
        *self.progress.lock() = Some(progress);
    }

    pub async fn get_command_registry(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, CommandRegistry<World>> {
        self.command_registry.read().await
    }

    pub fn start_shutdown_signal_handler(&self) {
        let world_for_shutdown = self.clone();
        let running_flag = self.running.clone();

        let handle = tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = match signal(SignalKind::terminate()) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        tracing::warn!("Failed to register SIGTERM handler: {}", e);
                        None
                    }
                };

                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        world_for_shutdown.stop();
                    }
                    _ = async {
                        if let Some(ref mut sigterm) = sigterm {
                            sigterm.recv().await;
                        }
                    } => {
                        world_for_shutdown.stop();
                    }
                }
            }

            #[cfg(not(unix))]
            {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to register Ctrl+C handler");
                world_for_shutdown.stop();
            }
        });

        let mut tasks = self.background_tasks.lock().unwrap();
        tasks.push(handle);
    }

    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn start(&self, config: &crate::config::Config) -> Result<()> {
        // NOTE: logging is initialised once by the caller (runtime/bin) before start(),
        // and shutdown is driven by the caller's broadcast channel — so this no longer
        // installs a tracing subscriber or its own ctrl-c handler.
        if let Err(e) = self.init().await {
            tracing::error!("World initialization FAILED: {:?}", e);
            return Err(e);
        }
        // TODO magic string and swap to repository - create some realm system in core to handle this and heartbeat
        let builds = "5875 6005 6141";
        match oxcore_db::database::RealmRepository::new(std::sync::Arc::new(
            self.databases.auth.clone(),
        ))
        .set_online(self.get_realm_id(), &config.realm_name, builds)
        .await
        {
            Ok(result) => {
                if result > 0 {
                    tracing::info!(
                        "Realm {} set to online in realmlist ({} rows affected)",
                        self.get_realm_id(),
                        result
                    );
                } else {
                    tracing::error!(
                        "Failed to update realmlist: No realm found with id={}",
                        self.get_realm_id()
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to update realmlist: {}", e);
            }
        }

        self.start_realm_heartbeat();

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_realm_id(&self) -> i32 {
        self.realm_id.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn set_realm_id(&self, id: i32) {
        self.realm_id.store(id, std::sync::atomic::Ordering::SeqCst)
    }

    /// Remove player packet handler (called on logout/disconnect)
    pub fn remove_player_handler(&self, player_guid: ObjectGuid) {
        if let Some((_, _handler)) = self.player_handlers.remove(&player_guid) {
            tracing::debug!(
                "[HANDLER] Removed packet handler for player {}",
                player_guid
            );
            // Handler task will shut down when channel closes (on drop)
        }
    }

    /// Create a movement buffer for a player (called during login)
    pub fn create_movement_buffer(
        &self,
        player_guid: ObjectGuid,
    ) -> Arc<crate::core::network::movement_buffer::MovementBuffer> {
        let buffer = Arc::new(crate::core::network::movement_buffer::MovementBuffer::new(
            player_guid,
        ));
        self.movement_buffers
            .insert(player_guid, Arc::clone(&buffer));
        buffer
    }

    /// Remove movement buffer for a player (called on logout/disconnect)
    pub fn remove_movement_buffer(&self, player_guid: ObjectGuid) {
        if self.movement_buffers.remove(&player_guid).is_some() {
            tracing::trace!(
                "[MOVEMENT] Removed movement buffer for player {}",
                player_guid
            );
        }
    }
}

impl Clone for World {
    fn clone(&self) -> Self {
        Self {
            managers: Managers {
                player_mgr: Arc::clone(&self.managers.player_mgr),
                creature_mgr: Arc::clone(&self.managers.creature_mgr),
                gameobject_mgr: Arc::clone(&self.managers.gameobject_mgr),
                corpse_mgr: Arc::clone(&self.managers.corpse_mgr),
                item_mgr: Arc::clone(&self.managers.item_mgr),
                hotfix_store: Arc::clone(&self.managers.hotfix_store),
                map_mgr: Arc::clone(&self.managers.map_mgr),
                broadcast_mgr: Arc::clone(&self.managers.broadcast_mgr),
                pool_mgr: Arc::clone(&self.managers.pool_mgr),
                linking_mgr: Arc::clone(&self.managers.linking_mgr),
                addon_mgr: Arc::clone(&self.managers.addon_mgr),
                vmap_mgr: Arc::clone(&self.managers.vmap_mgr),
                terrain_mgr: Arc::clone(&self.managers.terrain_mgr),
                mmap_mgr: Arc::clone(&self.managers.mmap_mgr),
                pathfinder: Arc::clone(&self.managers.pathfinder),
                waypoint_mgr: Arc::clone(&self.managers.waypoint_mgr),
                area_trigger_mgr: Arc::clone(&self.managers.area_trigger_mgr),
                taxi_mgr: Arc::clone(&self.managers.taxi_mgr),
                battleground_mgr: Arc::clone(&self.managers.battleground_mgr),
                scope_states: Arc::clone(&self.managers.scope_states),
                bg_events: Arc::clone(&self.managers.bg_events),
                condition_mgr: Arc::clone(&self.managers.condition_mgr),
                instance_mgr: Arc::clone(&self.managers.instance_mgr),
                game_event_mgr: Arc::clone(&self.managers.game_event_mgr),
                spell_mgr: Arc::clone(&self.managers.spell_mgr),
                lua_mgr: Arc::clone(&self.managers.lua_mgr),
                auction_mgr: Arc::clone(&self.managers.auction_mgr),
            },
            systems: Arc::clone(&self.systems),
            session_mgr: Arc::clone(&self.session_mgr),
            databases: Arc::clone(&self.databases),
            config: Arc::clone(&self.config),
            dbc: Arc::clone(&self.dbc),
            update_interval: self.update_interval,
            realm_id: Arc::clone(&self.realm_id),
            shutdown_rx: Arc::clone(&self.shutdown_rx),
            running: self.running.clone(),
            console_rx: Arc::clone(&self.console_rx),
            command_registry: Arc::clone(&self.command_registry),
            background_tasks: Arc::clone(&self.background_tasks),
            tick_stats: Arc::clone(&self.tick_stats),
            progress: Arc::clone(&self.progress),
            player_handlers: Arc::clone(&self.player_handlers),
            movement_buffers: Arc::clone(&self.movement_buffers),
            data_dir: self.data_dir.clone(),
        }
    }
}

/// Progressive-patch number for a `wow_patch` setting (102 = 1.2 is patch 0,
/// 112 = 1.12 is patch 10). Settings outside that range clamp to its ends.
fn content_patch_for(wow_patch: u32) -> u8 {
    (wow_patch.clamp(102, 112) - 102) as u8
}

#[cfg(test)]
mod content_patch_tests {
    use super::content_patch_for;

    #[test]
    fn wow_patch_maps_onto_progressive_patches() {
        assert_eq!(content_patch_for(102), 0);
        assert_eq!(content_patch_for(104), 2);
        assert_eq!(content_patch_for(111), 9);
        assert_eq!(content_patch_for(112), 10);
        assert_eq!(content_patch_for(200), 10);
        assert_eq!(content_patch_for(0), 0);
    }
}
