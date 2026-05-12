use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::shared::console::{CommandRegistry, ConsoleCommand};
use crate::shared::database::world::repositories::QuestTemplateRepository;
use crate::shared::database::Databases;
use crate::shared::protocol::ObjectGuid;
use crate::world::config::Config;
use crate::world::core::session::SessionManager;
use crate::world::dbc::DbcManager;
use crate::world::game::area_trigger::AreaTriggerManager;
use crate::world::game::broadcast_mgr::BroadcastManager;
use crate::world::game::coordination::{LinkingManager, PoolManager};
use crate::world::game::corpse::CorpseManager;
use crate::world::game::creature::{AddonManager, CreatureManager, WaypointManager};
use crate::world::game::gameobject::GameObjectManager;
use crate::world::game::items::ItemManager;
use crate::world::game::player::{PlayerManager, PlayerSystem};
use crate::world::game::spell::SpellManager;
use crate::world::game::SystemManager;
use crate::world::core::lua::LuaScriptManager;
use crate::world::map::manager::MapManager;
use crate::world::map::pathfinding::vmap::{VMapConfig, VMapManager};
use crate::world::map::pathfinding::{MMapManager, PathFinder};
use parking_lot::RwLock;
use tokio::sync::RwLock as TokioRwLock;

pub struct Managers {
    // TODO moves these out of this struct
    pub player_mgr: Arc<PlayerManager>,
    pub creature_mgr: Arc<CreatureManager>,
    pub gameobject_mgr: Arc<GameObjectManager>,
    pub corpse_mgr: Arc<CorpseManager>,
    pub item_mgr: Arc<ItemManager>,
    pub map_mgr: Arc<MapManager>,
    pub broadcast_mgr: Arc<BroadcastManager>,
    pub pool_mgr: Arc<PoolManager>,
    pub linking_mgr: Arc<LinkingManager>,
    pub addon_mgr: Arc<AddonManager>,
    pub vmap_mgr: Arc<VMapManager>,
    pub mmap_mgr: Arc<MMapManager>,
    pub pathfinder: Arc<PathFinder>,
    pub waypoint_mgr: Arc<WaypointManager>,
    pub area_trigger_mgr: Arc<AreaTriggerManager>,
    pub instance_mgr: Arc<crate::world::game::instance::InstanceMgr>,
    pub spell_mgr: Arc<SpellManager>,
    pub lua_mgr: Arc<LuaScriptManager>,
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
    /// Per-player packet handlers
    pub player_handlers: Arc<dashmap::DashMap<ObjectGuid, crate::world::core::network::player_handler::PlayerPacketHandler>>,
    /// Per-player movement buffers (socket writes, map update reads)
    pub movement_buffers: Arc<dashmap::DashMap<ObjectGuid, Arc<crate::world::core::network::movement_buffer::MovementBuffer>>>,
    pub data_dir: PathBuf,
}

impl World {
    pub fn new(databases: Arc<Databases>, config: Arc<Config>, update_interval_ms: u32, data_dir: std::path::PathBuf) -> Self {
        let session_mgr = Arc::new(SessionManager::new());
        let player_mgr = Arc::new(PlayerManager::new());
        let player_system = Arc::new(PlayerSystem::new(Arc::clone(&player_mgr)));
        let creature_mgr = Arc::new(CreatureManager::new(Arc::new(databases.world.clone())));
        let gameobject_mgr = Arc::new(GameObjectManager::new(Arc::new(databases.world.clone())));
        let corpse_mgr = Arc::new(CorpseManager::new());
        let item_mgr = Arc::new(ItemManager::new());
        let map_mgr = Arc::new(MapManager::new());

        // Create VMap manager for collision/LOS
        let vmap_config = VMapConfig {
            enable_los: config.vmap_enable_los,
            enable_height: config.vmap_enable_height,
            enable_indoor_check: config.vmap_enable_indoor_check,
        };
        let vmap_mgr = Arc::new(VMapManager::new(&data_dir, vmap_config));

        // Create MMap manager for navigation mesh pathfinding
        let mmap_mgr = Arc::new(MMapManager::new(&data_dir, Arc::clone(&vmap_mgr)));

        // Create PathFinder (integrates VMap LOS + NavMesh A* + obstacle avoidance)
        let pathfinder = Arc::new(PathFinder::new(Arc::clone(&mmap_mgr)));

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
        let instance_mgr = Arc::new(crate::world::game::instance::InstanceMgr::new());

        // Initialize Lua scripting system
        let lua_mgr = Arc::new(LuaScriptManager::new(&data_dir));

        let character_pool = Arc::new(databases.character.clone());
        let world_pool = Arc::new(databases.world.clone());
        let systems = Arc::new(SystemManager::new(
            Arc::clone(&character_pool),
            Arc::clone(&world_pool),
            Arc::clone(&broadcast_mgr),
            Arc::clone(&item_mgr),
            Arc::clone(&player_system),
            Arc::clone(&creature_mgr),
            Arc::clone(&pool_mgr),
            Arc::clone(&linking_mgr),
            Arc::clone(&addon_mgr),
        ));

        use crate::world::console::commands::register_all_commands;
        let mut command_registry = CommandRegistry::new();
        register_all_commands(&mut command_registry);

        Self {
            managers: Managers {
                player_mgr: systems.player.manager(),
                creature_mgr,
                gameobject_mgr,
                corpse_mgr,
                item_mgr,
                map_mgr,
                broadcast_mgr,
                pool_mgr,
                linking_mgr,
                addon_mgr,
                vmap_mgr,
                mmap_mgr,
                pathfinder,
                waypoint_mgr,
                area_trigger_mgr,
                instance_mgr,
                spell_mgr: Arc::new(SpellManager::new()),
                lua_mgr,
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
            player_handlers: Arc::new(dashmap::DashMap::new()),
            movement_buffers: Arc::new(dashmap::DashMap::new()),
            data_dir,
        }
    }

    pub async fn init(&self) -> Result<()> {
        // Load DBC data files
        tracing::info!("Loading DBC files...");
        {
            let dbc_path = self.data_dir.join("dbc");
            let mut dbc = self.dbc.write();
            dbc.load_all(dbc_path.to_str().unwrap())
                .context("Failed to load DBC files")?;
        }
        // Load spells from SQL (DBC field offsets are unreliable for vanilla 1.12.1)
        self.managers
            .spell_mgr
            .load(&self.databases.world)
            .await
            .context("Failed to load spells from SQL")?;

        // Initialize GUID generators from database
        self.managers
            .player_mgr
            .init_guid_generator(&self.databases.character)
            .await?;

        // Load item templates
        self.managers
            .item_mgr
            .load_item_templates(&self.databases.world)
            .await?;

        // Initialize item GUID generator from database
        self.managers
            .item_mgr
            .init_guid_generator(&self.databases.character)
            .await?;

        // Load creature templates and spawns
        self.managers.creature_mgr.load_templates().await?;
        self.managers.creature_mgr.load_model_info().await?;

        // Set patch from config (convert wow_patch like 112 to creature patch, cap at 10 for vanilla)
        let patch = self.config.wow_patch.unwrap_or(112);
        let creature_patch = (patch % 100).min(10) as u8; // 112 -> 12 -> 10 (capped)
        self.managers.creature_mgr.set_patch(creature_patch);

        self.managers.creature_mgr.load_spawns().await?;

        // Load gameobject templates and spawns
        self.managers.gameobject_mgr.set_patch(creature_patch);
        self.managers.gameobject_mgr.load_templates().await?;
        self.managers.gameobject_mgr.load_spawns().await?;

        // Load waypoint data
        use crate::world::game::creature::movement::waypoint_repository::WaypointRepository;
        let waypoint_repo = WaypointRepository::new(self.databases.world.clone());
        let waypoint_data = waypoint_repo.load_all().await
            .context("Failed to load waypoint data")?;
        self.managers.waypoint_mgr.load_from_data(waypoint_data);

        // Load loot tables
        self.systems.loot_manager
            .load_loot_tables(&self.databases.world).await
            .context("Failed to load loot tables")?;

        // Load area trigger data
        self.managers
            .area_trigger_mgr
            .load()
            .await
            .context("Failed to load area triggers")?;

        // Initialize instance manager
        self.managers
            .instance_mgr
            .initialize(&self.databases)
            .await
            .context("Failed to initialize instance manager")?;

        // Initialize Lua scripting
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
                tracing::warn!("Lua scripting initialization failed: {} (continuing without scripts)", e);
            }
        }

        // Load quest data from world database
        QuestTemplateRepository::load(&self.systems.quest_manager, &self.databases.world).await
            .context("Failed to load quest data")?;

        // Initialize game systems
        self.systems.init_all().await?;

        // Load graveyard data (needs both DBC and world DB)
        {
            let dbc = self.dbc.read();
            self.systems.death.load_graveyards(
                std::sync::Arc::new(self.databases.world.clone()),
                &dbc,
            ).await?;
        }

        // Register vendor template IDs from creature templates
        // This must happen after both creature_mgr.load_templates() and vendor_manager.load()
        for entry in self.managers.creature_mgr.all_templates() {
            if entry.vendor_id > 0 {
                self.systems.vendor_manager.register_creature_vendor_template(entry.entry, entry.vendor_id);
            }
            if entry.trainer_id > 0 {
                self.systems.trainer_manager.register_creature_trainer_template(entry.entry, entry.trainer_id);
            }
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
        let diff = self.update_interval;
        let diff_ms = diff.as_millis() as u32;

        // Process grid loading/unloading for all active maps (continents and instances)
        // This handles lazy creature spawning based on player proximity
        for (map_id, instance_id) in self.managers.map_mgr.get_active_map_keys() {
            if let Err(e) = self.systems.grid.process_map_grids(map_id, instance_id, self).await {
                tracing::error!("Grid processing failed for map {}:{}: {}", map_id, instance_id, e);
            }
        }

        // Process creature movement FIRST so positions are current for combat
        // and AI range checks (matches vmangos: Unit::Update does spline/movement
        // before AI UpdateAI calls DoMeleeAttackIfReady)
        self.systems.creature_movement.update_creatures(diff_ms, self)?;

        self.managers.map_mgr.update_all(diff, self).await?;

        // Flush queued packets (packet collapsing optimization)
        // Movement packets accumulated during map updates are sent here with collapsing applied
        self.managers.broadcast_mgr.flush_all_queues();

        self.systems.update_all(diff, self)?;

        // Update spell cast timers for all players
        self.systems.spells.update_all_casts(diff, self).await?;

        // Update aura durations and periodic effects for all players
        self.systems.auras.update_all_auras(diff, self).await?;

        // Process creature deaths (Phase 3)
        crate::world::game::creature::death::process_deaths(self).await?;

        // Process corpse decay (Phase 3)
        crate::world::game::creature::death::process_corpse_decay(self, diff_ms).await?;

        // Process respawns (Phase 6)
        self.systems.creature_respawn.process_respawns(self).await?;

        // Process creature combat timers and melee attacks
        // Timer countdown + attack execution in one pass (matches vmangos
        // Unit::Update timer decrement -> AI DoMeleeAttackIfReady order)
        crate::world::game::creature::combat_update::update_creature_combat(self, diff_ms);

        // Process AI updates (after combat timers so AI sees current timer state)
        crate::world::game::creature::ai::update_creature_ai(self)?;

        // Process creature regeneration (health + mana)
        crate::world::game::creature::regen::update_regeneration(self, diff_ms);

        // Process timed quest expiry
        self.systems.quest.update_quest_timers(diff_ms, self);

        // Flush dirty inventory data to DB periodically (every 20 ticks = ~1 second at 50ms)
        static INVENTORY_SAVE_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let save_count = INVENTORY_SAVE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if save_count % 20 == 0 {  // Reduced from 100 (5s) to 20 (1s) to reduce data loss window
            if let Err(e) = self.systems.inventory.flush_pending_ops().await {
                tracing::error!("Failed to flush inventory ops: {}", e);
            }
        }

        // Aggro scanning (Phase 5) - every 4th tick for performance
        static AGGRO_SCAN_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = AGGRO_SCAN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 4 == 0 {
            crate::world::game::creature::ai::scan_for_aggro(self).await?;
        }

        self.session_mgr.update_logout_timers(self).await?;

        if let Ok(_) = self
            .command_registry
            .read().await
            .process_commands(&self.console_rx, self).await

        {
            // Console processed successfully
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

    pub fn setup_logging(&self, config: &crate::world::config::Config) -> Result<()> {
        crate::world::logging::init_logging(config)
    }

    pub async fn set_shutdown_receiver(&self, rx: tokio::sync::broadcast::Receiver<()>) {
        let mut shutdown_rx = self.shutdown_rx.lock().unwrap();
        *shutdown_rx = Some(rx);
    }

    pub fn start_realm_heartbeat(&self) {
        let heartbeat_interval = crate::world::config::get_config_mgr()
            .get()
            .realm_heartbeat_interval;
        let heartbeat_pool = self.databases.auth.clone();
        let heartbeat_realm_id = self.get_realm_id();

        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(heartbeat_interval));
            loop {
                interval.tick().await;
                if let Err(e) =
                    sqlx::query("UPDATE `realmlist` SET `last_seen` = NOW() WHERE `id` = ?")
                        .bind(heartbeat_realm_id)
                        .execute(&heartbeat_pool).await
                {
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
                tokio::signal::ctrl_c().await
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

    pub async fn start(&self, config: &crate::world::config::Config) -> Result<()> {
        self.setup_logging(config)?;
        if let Err(e) = self.init().await {
            tracing::error!("World initialization FAILED: {:?}", e);
            return Err(e);
        }
        // TODO magic string and swap to repository - create some realm system in core to handle this and heartbeat
        let builds = "5875 6005 6141";
        match sqlx::query(
            "UPDATE `realmlist` SET `realmflags` = `realmflags` & ~(2), `population` = 0, `realmbuilds` = ?, `last_seen` = NOW() WHERE `id` = ?"
        )
        .bind(builds)
        .bind(self.get_realm_id())
        .execute(&self.databases.auth).await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    tracing::info!(
                        "Realm {} set to online in realmlist ({} rows affected)",
                        self.get_realm_id(),
                        result.rows_affected()
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
        self.start_shutdown_signal_handler();

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
            tracing::debug!("[HANDLER] Removed packet handler for player {}", player_guid);
            // Handler task will shut down when channel closes (on drop)
        }
    }

    /// Create a movement buffer for a player (called during login)
    pub fn create_movement_buffer(&self, player_guid: ObjectGuid) -> Arc<crate::world::core::network::movement_buffer::MovementBuffer> {
        let buffer = Arc::new(crate::world::core::network::movement_buffer::MovementBuffer::new(player_guid));
        self.movement_buffers.insert(player_guid, Arc::clone(&buffer));
        buffer
    }

    /// Remove movement buffer for a player (called on logout/disconnect)
    pub fn remove_movement_buffer(&self, player_guid: ObjectGuid) {
        if self.movement_buffers.remove(&player_guid).is_some() {
            tracing::trace!("[MOVEMENT] Removed movement buffer for player {}", player_guid);
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
                map_mgr: Arc::clone(&self.managers.map_mgr),
                broadcast_mgr: Arc::clone(&self.managers.broadcast_mgr),
                pool_mgr: Arc::clone(&self.managers.pool_mgr),
                linking_mgr: Arc::clone(&self.managers.linking_mgr),
                addon_mgr: Arc::clone(&self.managers.addon_mgr),
                vmap_mgr: Arc::clone(&self.managers.vmap_mgr),
                mmap_mgr: Arc::clone(&self.managers.mmap_mgr),
                pathfinder: Arc::clone(&self.managers.pathfinder),
                waypoint_mgr: Arc::clone(&self.managers.waypoint_mgr),
                area_trigger_mgr: Arc::clone(&self.managers.area_trigger_mgr),
                instance_mgr: Arc::clone(&self.managers.instance_mgr),
                spell_mgr: Arc::clone(&self.managers.spell_mgr),
                lua_mgr: Arc::clone(&self.managers.lua_mgr),
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
            player_handlers: Arc::clone(&self.player_handlers),
            movement_buffers: Arc::clone(&self.movement_buffers),
            data_dir: self.data_dir.clone(),
        }
    }
}
