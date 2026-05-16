use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::shared::database::characters::repositories::InventoryRepository;
use crate::shared::database::characters::repositories::{
    GroupRepository, GuildRepository, QuestRepository, SocialRepository, TicketRepository,
};
use crate::shared::protocol::{ObjectGuid, Position};
use crate::world::game::broadcast_mgr::BroadcastManagerTrait;
use crate::world::game::chat::ChatSystem;
use crate::world::game::combat::CombatSystem;
use crate::world::game::coordination::{LinkingManager, LinkingSystem, PoolManager, PoolSystem};
use crate::world::game::creature::ai::AIEventQueue;
use crate::world::game::creature::movement::MovementSystem;
use crate::world::game::creature::respawn::RespawnSystem;
use crate::world::game::creature::{AddonManager, AddonSystem, CreatureManager};
use crate::world::game::group::GroupSystem;
use crate::world::game::guild::GuildSystem;
use crate::world::game::inventory::InventorySystem;
use crate::world::game::loot::{LootManager, LootSystem};
use crate::world::game::npc::{
    GossipManager, GossipSystem, QuestManager, QuestSystem, TrainerManager, VendorManager,
    VendorSystem,
};
use crate::world::game::player::auras::AuraSystem;
use crate::world::game::player::death::DeathSystem;
use crate::world::game::player::environment::EnvironmentSystem;
use crate::world::game::player::experience::ExperienceSystem;
use crate::world::game::player::power::PowerSystem;
use crate::world::game::player::reputation::ReputationSystem;
use crate::world::game::player::settings::SettingsSystem;
use crate::world::game::player::skills::SkillSystem;
use crate::world::game::player::spells::SpellSystem;
use crate::world::game::player::stats::StatsSystem;
use crate::world::game::player::talents::TalentSystem;
use crate::world::game::player::PlayerSystem;
use crate::world::game::social::SocialSystem;
use crate::world::game::ticket::TicketSystem;
use crate::world::game::trade::TradeSystem;
use crate::world::game::visibility::VisibilitySystem;
use crate::world::game::{BroadcastManager, ItemManager};
use crate::world::map::grid::GridSystem;
use crate::world::World;

pub struct SystemManager {
    pub player: Arc<PlayerSystem>,
    pub combat: Arc<CombatSystem>,
    // TODO visibility should live on player
    pub visibility: Arc<VisibilitySystem>,
    pub inventory: Arc<InventorySystem>,
    pub social: Arc<SocialSystem>,
    pub guild: Arc<GuildSystem>,
    pub chat: Arc<ChatSystem>,
    pub trade: Arc<TradeSystem>,
    pub group: Arc<GroupSystem>,
    pub item_mgr: Arc<ItemManager>,
    pub ticket: Arc<TicketSystem>,
    pub experience: Arc<ExperienceSystem>,
    pub grid: Arc<GridSystem>,
    pub gossip_manager: Arc<GossipManager>,
    pub gossip: Arc<GossipSystem>,
    pub trainer_manager: Arc<TrainerManager>,
    pub vendor_manager: Arc<VendorManager>,
    pub vendor: Arc<VendorSystem>,
    pub quest_manager: Arc<QuestManager>,
    pub quest: Arc<QuestSystem>,
    pub stats: Arc<StatsSystem>,
    pub reputation: Arc<ReputationSystem>,
    pub power: Arc<PowerSystem>,
    pub auras: Arc<AuraSystem>,
    pub settings: Arc<SettingsSystem>,
    pub skills: Arc<SkillSystem>,
    pub spells: Arc<SpellSystem>,
    pub talents: Arc<TalentSystem>,
    pub death: Arc<DeathSystem>,
    pub environment: Arc<EnvironmentSystem>,
    pub honor: Arc<crate::world::game::player::honor::HonorSystem>,
    /// AI event queue for creature AI events
    pub ai_event_queue: Arc<AIEventQueue>,
    /// Creature respawn system
    pub creature_respawn: Arc<RespawnSystem>,
    /// Creature movement system
    pub creature_movement: Arc<MovementSystem>,
    /// Loot manager for loot tables
    pub loot_manager: Arc<LootManager>,
    /// Loot system for loot handling
    pub loot: Arc<LootSystem>,
    /// Pool system for spawn pools
    pub pool: Arc<PoolSystem>,
    /// Linking system for creature linking
    pub linking: Arc<LinkingSystem>,
    /// Addon system for creature addons
    pub addon: Arc<AddonSystem>,
}

impl SystemManager {
    pub fn new(
        character_pool: Arc<sqlx::MySqlPool>,
        world_pool: Arc<sqlx::MySqlPool>,
        broadcast_mgr: Arc<BroadcastManager>,
        item_mgr: Arc<ItemManager>,
        player_system: Arc<PlayerSystem>,
        creature_mgr: Arc<CreatureManager>,
        pool_mgr: Arc<PoolManager>,
        linking_mgr: Arc<LinkingManager>,
        addon_mgr: Arc<AddonManager>,
    ) -> Self {
        let player_mgr = player_system.manager();
        let inventory_repo = Arc::new(InventoryRepository::new(character_pool.clone()));
        let social_repo = Arc::new(SocialRepository::new(character_pool.clone()));
        let guild_repo = Arc::new(GuildRepository::new(character_pool.clone()));
        let group_repo = Arc::new(GroupRepository::new(character_pool.clone()));
        let ticket_repo = Arc::new(TicketRepository::new(character_pool.clone()));

        let social = Arc::new(SocialSystem::new(
            social_repo,
            broadcast_mgr.clone(),
            player_mgr.clone(),
        ));

        let guild = Arc::new(GuildSystem::new(
            guild_repo,
            broadcast_mgr.clone(),
            player_mgr.clone(),
            item_mgr.clone(),
        ));

        let chat = Arc::new(ChatSystem::new(broadcast_mgr.clone(), player_mgr.clone()));

        let inventory = Arc::new(InventorySystem::new(
            inventory_repo,
            broadcast_mgr.clone(),
            item_mgr.clone(),
        ));

        let trade = Arc::new(TradeSystem::new(
            broadcast_mgr.clone(),
            player_mgr.clone(),
            inventory.clone(),
            item_mgr.clone(),
            false, // allow_cross_faction_trade - TODO: make configurable
        ));

        let group = Arc::new(GroupSystem::new(
            group_repo,
            broadcast_mgr.clone(),
            player_mgr.clone(),
            false, // allow_cross_faction_group - TODO: make configurable
        ));

        let ticket = Arc::new(TicketSystem::new(
            ticket_repo,
            broadcast_mgr.clone(),
            player_mgr.clone(),
        ));

        let experience = Arc::new(ExperienceSystem::new(
            broadcast_mgr.clone(),
            player_mgr.clone(),
        ));

        let grid = Arc::new(GridSystem::new());

        // Create gossip manager and system
        let gossip_manager = Arc::new(GossipManager::new(world_pool.clone()));
        let gossip = Arc::new(GossipSystem::new(
            gossip_manager.clone(),
            broadcast_mgr.clone(),
            player_mgr.clone(),
            creature_mgr.clone(),
        ));

        // Create trainer manager
        let trainer_manager = Arc::new(TrainerManager::new(world_pool.clone()));

        // Create vendor manager and system
        let vendor_manager = Arc::new(VendorManager::new(world_pool.clone()));
        let vendor = Arc::new(VendorSystem::new(
            vendor_manager.clone(),
            broadcast_mgr.clone(),
            creature_mgr.clone(),
            player_mgr.clone(),
            item_mgr.clone(),
            inventory.clone(),
        ));

        // Create quest manager and system
        let quest_manager = Arc::new(QuestManager::new(world_pool.clone()));
        let quest_repo: Arc<
            dyn crate::shared::database::characters::repositories::QuestRepositoryTrait,
        > = Arc::new(QuestRepository::new(character_pool.clone()));
        let quest = Arc::new(QuestSystem::new(
            quest_manager.clone(),
            quest_repo,
            broadcast_mgr.clone(),
            player_mgr.clone(),
            creature_mgr,
            item_mgr.clone(),
            inventory.clone(),
            experience.clone(),
        ));

        let stats = Arc::new(StatsSystem::new(
            broadcast_mgr.clone(),
            player_mgr.clone(),
            world_pool,
        ));

        let reputation = Arc::new(ReputationSystem::new(broadcast_mgr.clone()));

        let power = Arc::new(PowerSystem::new(broadcast_mgr.clone()));

        let auras = Arc::new(AuraSystem::new(broadcast_mgr.clone()));

        let settings = Arc::new(SettingsSystem::new(broadcast_mgr.clone()));

        let combat = Arc::new(CombatSystem::new(broadcast_mgr.clone()));
        experience.set_stats_system(Arc::clone(&stats));

        let spells = Arc::new(SpellSystem::new(broadcast_mgr.clone()));

        let skills = Arc::new(SkillSystem::new(broadcast_mgr.clone()));

        // Create talent system with empty store (will be loaded from DBC later)
        let talents = Arc::new(TalentSystem::new(std::sync::Arc::new(
            crate::world::game::player::talents::TalentStore::load(vec![], vec![]),
        )));

        let death = Arc::new(DeathSystem::new(broadcast_mgr.clone()));

        let environment = Arc::new(EnvironmentSystem::new());

        let honor = Arc::new(crate::world::game::player::honor::HonorSystem::new());

        // Create AI event queue for creature AI
        let ai_event_queue = Arc::new(AIEventQueue::new());

        // Create creature respawn system
        let creature_respawn = Arc::new(RespawnSystem::new(broadcast_mgr.clone()));

        // Create creature movement system
        let creature_movement = Arc::new(MovementSystem::new(broadcast_mgr.clone()));

        // Create loot manager and system
        let loot_manager = Arc::new(LootManager::new());
        let loot = Arc::new(LootSystem::new(
            loot_manager.clone(),
            broadcast_mgr.clone(),
            item_mgr.clone(),
            player_mgr.clone(),
        ));

        // Create coordination systems
        let pool = Arc::new(PoolSystem::new(pool_mgr, broadcast_mgr.clone()));

        let linking = Arc::new(LinkingSystem::new(linking_mgr, broadcast_mgr.clone()));

        let addon = Arc::new(AddonSystem::new(addon_mgr, broadcast_mgr.clone()));

        Self {
            player: player_system,
            combat,
            visibility: Arc::new(VisibilitySystem::new()),
            social,
            guild,
            chat,
            inventory,
            trade,
            group,
            item_mgr,
            ticket,
            experience,
            grid,
            gossip_manager,
            gossip,
            trainer_manager,
            vendor_manager,
            vendor,
            quest_manager,
            quest,
            stats,
            reputation,
            power,
            auras,
            settings,
            skills,
            spells,
            talents,
            death,
            environment,
            honor,
            ai_event_queue,
            creature_respawn,
            creature_movement,
            loot_manager,
            loot,
            pool,
            linking,
            addon,
        }
    }

    /// Initialize all systems
    pub async fn init_all(&self) -> Result<()> {
        self.stats.init().await?; // Load base stats from DB (before other systems)
        self.player.init().await?; // Initializes movement subsystem
        self.combat.init().await?;
        self.visibility.init().await?;
        self.inventory.init().await?;
        self.social.init().await?;
        self.guild.init().await?;
        self.chat.init().await?;
        self.trade.init().await?;
        self.group.init().await?;
        self.ticket.init().await?;
        self.experience.init().await?;
        self.grid.init().await?;
        self.gossip_manager.load().await?;
        self.gossip.init().await?;
        self.trainer_manager.load().await?;
        self.vendor_manager.load().await?;
        self.vendor.init().await?;
        self.quest_manager.load().await?;
        self.quest.init().await?;
        self.death.init().await?;
        self.loot.init().await?;
        // Power system needs no async init
        // Talent system needs no async init (DBC loaded separately)
        // Environment system needs no async init

        Ok(())
    }

    /// Update all systems
    pub fn update_all(&self, diff: Duration, world: &World) -> Result<()> {
        let player_mgr = self.player.manager();

        // Update in priority order (lower priority first)
        self.player.update(diff)?; // Global player updates (movement handled per-map)
        self.combat.update(diff, &player_mgr)?;
        self.power.update(diff, world)?; // Power regeneration
        self.environment.update(diff, world, &player_mgr)?; // Environment system (rest XP, mirror timers)
        self.social.update(diff)?;
        self.guild.update(diff)?;
        self.chat.update(diff)?;
        self.trade.update(diff)?;
        self.group.update(diff)?;
        self.ticket.update(diff)?;
        self.experience.update(diff)?;
        self.grid.update(diff)?;
        // Gossip doesn't need periodic updates

        // Death system: tick death timers, handle auto-release
        if let Err(e) = self.death.update(diff, world) {
            tracing::error!("Death system update error: {}", e);
        }

        Ok(())
    }

    /// Shutdown all systems (reverse order)
    pub async fn shutdown_all(&self) -> Result<()> {
        self.experience.shutdown().await?;
        self.ticket.shutdown().await?;
        self.group.shutdown().await?;
        self.trade.shutdown().await?;
        self.combat.shutdown().await?;
        self.visibility.shutdown().await?;
        self.inventory.shutdown().await?;
        self.chat.shutdown().await?;
        self.guild.shutdown().await?;
        self.social.shutdown().await?;
        self.player.shutdown().await?; // Shuts down movement subsystem
        self.grid.shutdown().await?;
        self.gossip.shutdown().await?;
        self.vendor.shutdown().await?;
        self.quest.shutdown().await?;
        self.stats.shutdown().await?;
        self.death.shutdown().await?;
        self.loot.shutdown().await?;
        // Environment system needs no async shutdown
        Ok(())
    }

    /// Notify all systems of player login
    pub async fn on_player_login(
        &self,
        guid: ObjectGuid,
        position: Option<Position>,
        world: &World,
    ) -> Result<()> {
        // Initialize player movement state
        if let Some(pos) = position {
            self.player.manager().update_movement(guid, |state| {
                state.position = pos;
            });
        }

        self.player.on_player_login(guid).await?;
        self.stats.on_player_login(guid)?; // Calculate stats before combat/experience
        self.power.on_login(guid, world)?; // Initialize power after stats (reads stats.max_mana)
        self.combat.on_player_login(guid)?;
        self.visibility.on_player_login(guid, position)?;
        self.inventory.on_player_login(guid)?;
        self.social.on_player_login(guid)?;
        self.guild.on_player_login(guid)?;
        self.chat.on_player_login(guid)?;
        self.group.on_player_login(guid).await?;
        self.ticket.on_player_login(guid).await?;
        self.experience.on_player_login(guid)?;
        self.talents.on_player_login(guid, world).await?; // Reapply talent effects
        Ok(())
    }

    /// Notify all systems of player logout
    pub async fn on_player_logout(&self, guid: ObjectGuid, world: &World) -> Result<()> {
        let player_mgr = self.player.manager();
        self.player.on_player_logout(guid, world).await?;
        self.trade.on_player_logout(guid)?;
        self.visibility.on_player_logout(guid)?;
        self.combat.on_player_logout(guid, &player_mgr)?;
        self.inventory.on_player_logout(guid).await?;
        self.social.on_player_logout(guid).await?;
        self.guild.on_player_logout(guid)?;
        self.chat.on_player_logout(guid)?;
        self.group.on_player_logout(guid).await?;
        self.ticket.on_player_logout(guid).await?;
        self.experience.on_player_logout(guid)?;
        self.stats.on_player_logout(guid)?;
        self.quest.on_player_logout(guid).await?;
        Ok(())
    }

    /// Initialize talent system from DBC data.
    ///
    /// This must be called after DBC files are loaded to populate the talent store
    /// with actual talent and talent tab definitions.
    pub fn init_talents_from_dbc(&self, dbc_mgr: &crate::world::dbc::DbcManager) {
        use crate::world::game::player::talents::TalentStore;

        // Collect talent entries from DBC
        let talent_entries: Vec<(u32, crate::world::dbc::structures::TalentEntry)> = dbc_mgr
            .get_all_talents()
            .map(|(id, entry)| (*id, entry.clone()))
            .collect();

        // Collect talent tab entries from DBC
        let talent_tab_entries: Vec<(u32, crate::world::dbc::structures::TalentTabEntry)> = dbc_mgr
            .get_all_talent_tabs()
            .map(|(id, entry)| (*id, entry.clone()))
            .collect();

        if talent_entries.is_empty() && talent_tab_entries.is_empty() {
            tracing::warn!("Talent DBC data is empty - talent system will not function correctly");
            return;
        }

        // Create new talent store from DBC data
        let store = Arc::new(TalentStore::from_dbc(&talent_entries, &talent_tab_entries));

        // Reload the talent system with the new store
        self.talents.reload_from_dbc(store);
    }
}
