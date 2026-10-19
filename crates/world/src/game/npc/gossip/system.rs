//! Gossip System - business logic for NPC gossip
//!
//! Builds the menu a player sees at a creature or game object, keeps it on the player so
//! the option they pick can be checked against what they were shown, and carries out
//! that option.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};

use super::manager::GossipManager;
use super::types::{
    gossip_option, GossipMenuItem, PlayerGossipMenu, PreparedGossipOption,
    DEFAULT_GOSSIP_MESSAGE,
};
use crate::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::game::conditions::ConditionContext;
use crate::game::creature::CreatureManager;
use crate::game::db_scripts::{self, ScriptTable};
use crate::game::gameobject::GameObjectType;
use crate::game::npc::quest::types::GossipQuestData;
use crate::game::npc::trainer::types::trainer_type;
use crate::game::player::PlayerManager;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::messages::{
    GossipOptionData, MsgTalentWipeConfirm, SmsgGossipComplete, SmsgGossipMessage, SmsgGossipPoi,
    SmsgShowBank,
};
use oxcore_shared::protocol::ObjectGuid;

const NPC_FLAG_GOSSIP: u32 = 0x0000_0001;
const NPC_FLAG_QUESTGIVER: u32 = 0x0000_0002;
const NPC_FLAG_VENDOR: u32 = 0x0000_0004;

const CLASS_HUNTER: u8 = 3;

/// Appended to options a game master sees only because they bypass its condition.
const GM_MODE_SUFFIX: &str = " (GM mode is ON)";

/// What a gossip menu belongs to.
#[derive(Debug, Clone, Copy)]
enum GossipSource {
    Creature { entry: u32, npc_flags: u32 },
    GameObject { go_type: GameObjectType },
}

impl GossipSource {
    fn resolve(world: &World, guid: ObjectGuid) -> Option<Self> {
        if guid.is_game_object() {
            world
                .managers
                .gameobject_mgr
                .with_gameobject(guid, |go| GossipSource::GameObject {
                    go_type: go.go_type,
                })
        } else {
            world
                .managers
                .creature_mgr
                .with_creature(guid, |c| GossipSource::Creature {
                    entry: c.entry,
                    npc_flags: c.npc_flags,
                })
        }
    }
}

/// The player details option filtering looks at.
struct Viewer {
    game_master: bool,
    alive: bool,
    class: u8,
    race: u8,
    level: u8,
    gender: u8,
}

/// Greeting shown above a quest list.
#[derive(Debug, Clone, Default)]
pub struct QuestListGreeting {
    pub text: String,
    pub emote_delay: u32,
    pub emote: u32,
}

/// Gossip system - handles gossip menu business logic
pub struct GossipSystem {
    manager: Arc<GossipManager>,
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    creature_mgr: Arc<CreatureManager>,
}

impl GossipSystem {
    /// The loaded gossip tables, for callers that need a raw lookup rather than a menu.
    pub fn manager(&self) -> &Arc<GossipManager> {
        &self.manager
    }

    /// Create a new gossip system
    pub fn new(
        manager: Arc<GossipManager>,
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
        creature_mgr: Arc<CreatureManager>,
    ) -> Self {
        Self {
            manager,
            broadcast_mgr,
            player_mgr,
            creature_mgr,
        }
    }

    /// The menu a creature or game object opens with.
    pub fn default_menu_id(&self, world: &World, source: ObjectGuid) -> u32 {
        if source.is_game_object() {
            let Some(entry) = world
                .managers
                .gameobject_mgr
                .with_gameobject(source, |go| go.entry)
            else {
                return 0;
            };
            return world
                .managers
                .gameobject_mgr
                .get_template(entry)
                .map(|t| t.gossip_menu_id())
                .unwrap_or(0);
        }
        self.creature_mgr
            .with_creature(source, |c| c.entry)
            .and_then(|entry| self.creature_mgr.get_template(entry))
            .map(|t| t.gossip_menu_id)
            .unwrap_or(0)
    }

    /// Opens `source`'s default menu for `player`.
    pub async fn gossip_hello(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source: ObjectGuid,
    ) -> Result<()> {
        let menu_id = self.default_menu_id(world, source);
        self.prepare_gossip_menu(world, player_guid, source, menu_id);
        self.send_prepared_gossip(world, player_guid, source).await
    }

    /// Builds menu `menu_id` of `source` as `player_guid` should see it and keeps it on the
    /// player. Options are dropped when their condition fails, when the creature lacks the
    /// NPC flag they need, or when the service does not apply to this player. The source's
    /// quests are listed only on its default menu.
    pub fn prepare_gossip_menu(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
        menu_id: u32,
    ) {
        let Some(source) = GossipSource::resolve(world, source_guid) else {
            return;
        };
        let Some(viewer) = self.player_mgr.with_player(player_guid, |p| Viewer {
            game_master: p.gm_mode,
            alive: p.is_alive(),
            class: p.class,
            race: p.race,
            level: p.level,
            gender: p.gender,
        }) else {
            return;
        };

        let default_menu = menu_id == self.default_menu_id(world, source_guid);
        let mut can_see_quests = default_menu
            && match source {
                GossipSource::Creature { npc_flags, .. } => npc_flags & NPC_FLAG_QUESTGIVER != 0,
                GossipSource::GameObject { go_type } => go_type == GameObjectType::QuestGiver,
            };

        // A default menu without options of its own offers the shared service options.
        let mut items = self.manager.get_menu_items(menu_id);
        if items.is_empty() && default_menu {
            items = self.manager.get_menu_items(0);
        }

        let mut options = Vec::new();
        for item in &items {
            let mut shown_to_gm_only = false;
            if item.condition_id != 0
                && !self.condition_met(world, item.condition_id, player_guid, source_guid)
            {
                if viewer.game_master {
                    shown_to_gm_only = true;
                } else {
                    if item.option_id == gossip_option::QUESTGIVER {
                        can_see_quests = false;
                    }
                    continue;
                }
            }

            let shown = match source {
                GossipSource::Creature { entry, npc_flags } => {
                    item.npc_option_npcflag & npc_flags != 0
                        && self.creature_option_shown(world, item, entry, &viewer, player_guid)
                }
                GossipSource::GameObject { go_type } => {
                    item.option_id == gossip_option::GOSSIP
                        && matches!(go_type, GameObjectType::QuestGiver | GameObjectType::Goober)
                }
            };
            if !shown {
                continue;
            }

            let mut text = self.text_or_broadcast(&item.option_text, item.option_broadcast_text, viewer.gender);
            if shown_to_gm_only {
                text.push_str(GM_MODE_SUFFIX);
            }
            options.push(PreparedGossipOption {
                icon: item.option_icon,
                text,
                coded: item.box_coded,
                box_text: self.text_or_broadcast(&item.box_text, item.box_broadcast_text, viewer.gender),
                box_money: item.box_money,
                sender: 0,
                action: item.option_id,
                action_menu: item.action_menu_id,
                action_poi: item.action_poi_id,
                action_script: item.action_script_id,
            });
        }

        let quests = if can_see_quests {
            self.quest_menu(world, player_guid, source_guid)
        } else {
            Vec::new()
        };

        self.player_mgr.with_player_mut(player_guid, |p| {
            p.gossip = PlayerGossipMenu {
                source: source_guid,
                menu_id,
                options,
                quests,
                discovered_node: false,
            };
        });
    }

    /// Shows the menu prepared for `player_guid`: a quest-only creature (or a game object
    /// without a menu) opens its quests directly, anything else gets the gossip window.
    pub async fn send_prepared_gossip(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
    ) -> Result<()> {
        let Some(menu) = self.player_mgr.with_player(player_guid, |p| p.gossip.clone()) else {
            return Ok(());
        };
        let Some(source) = GossipSource::resolve(world, source_guid) else {
            return Ok(());
        };

        match source {
            GossipSource::Creature { npc_flags, .. } => {
                if menu.discovered_node && menu.quests.is_empty() {
                    return Ok(());
                }
                // Without the gossip flag the client expects the quest window itself; vendors
                // keep their gossip window so the trade option stays reachable. A gossip NPC
                // with no menu at all and nothing else to offer also goes straight to its
                // quests rather than a placeholder greeting.
                let nothing_to_say = menu.menu_id == 0 && menu.options.is_empty();
                if (npc_flags & NPC_FLAG_GOSSIP == 0 || nothing_to_say)
                    && npc_flags & NPC_FLAG_VENDOR == 0
                    && !menu.quests.is_empty()
                {
                    return self.send_prepared_quest(world, player_guid, source_guid).await;
                }
            }
            GossipSource::GameObject { .. } => {
                if menu.menu_id == 0 && !menu.quests.is_empty() {
                    return self.send_prepared_quest(world, player_guid, source_guid).await;
                }
            }
        }

        let text_id = if menu.menu_id != 0 {
            self.gossip_text_id(world, player_guid, source_guid, menu.menu_id)
        } else {
            self.source_text_id(world, source_guid)
        };

        // Game objects with nothing to say stay silent.
        if matches!(source, GossipSource::GameObject { .. })
            && menu.options.is_empty()
            && menu.quests.is_empty()
            && text_id == DEFAULT_GOSSIP_MESSAGE
        {
            return Ok(());
        }

        self.send_gossip_menu(player_guid, source_guid, &menu, text_id);
        Ok(())
    }

    /// Opens the quest window for the prepared quest list: a single quest goes straight to
    /// its details or turn-in window, several are listed under the source's greeting.
    pub async fn send_prepared_quest(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
    ) -> Result<()> {
        let Some(quests) = self
            .player_mgr
            .with_player(player_guid, |p| p.gossip.quests.clone())
        else {
            return Ok(());
        };
        let Some(first) = quests.first() else {
            return Ok(());
        };

        let quest_system = &world.systems.quest;
        if quests.len() == 1 {
            return quest_system
                .send_single_quest(player_guid, source_guid, first.quest_id, first.icon, world)
                .await;
        }

        let greeting = self.quest_list_greeting(world, player_guid, source_guid);
        quest_system.send_quest_list(player_guid, source_guid, &quests, &greeting);
        Ok(())
    }

    /// Carries out option `list_index` of the menu `player_guid` was shown at `source_guid`.
    /// Options that were never shown, or belong to another menu, are ignored.
    pub async fn on_gossip_select(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
        list_index: u32,
    ) -> Result<()> {
        let Some(option) = self.shown_option(player_guid, source_guid, list_index) else {
            debug!(
                "Player {:?} picked gossip option {} that {:?} never offered",
                player_guid, list_index, source_guid
            );
            return Ok(());
        };

        if source_guid.is_game_object() && option.action > gossip_option::QUESTGIVER {
            warn!(
                "Player {:?} requested a service option from game object {:?}",
                player_guid, source_guid
            );
            return Ok(());
        }

        match option.action {
            gossip_option::GOSSIP => {
                if option.action_poi != 0 {
                    self.send_point_of_interest(player_guid, option.action_poi);
                }
                if option.action_menu > 0 {
                    self.prepare_gossip_menu(
                        world,
                        player_guid,
                        source_guid,
                        option.action_menu as u32,
                    );
                    self.send_prepared_gossip(world, player_guid, source_guid)
                        .await?;
                } else if option.action_menu < 0 {
                    self.send_gossip_complete(player_guid);
                    if let Some(entry) = self
                        .creature_mgr
                        .with_creature(source_guid, |c| c.entry)
                    {
                        world
                            .systems
                            .quest
                            .talked_to_creature(player_guid, entry, source_guid);
                    }
                }
            }
            gossip_option::SPIRITHEALER => {
                let dead = self
                    .player_mgr
                    .with_player(player_guid, |p| !p.is_alive())
                    .unwrap_or(false);
                if dead {
                    self.broadcast_mgr.send_msg_to_player(
                        player_guid,
                        oxcore_shared::messages::death::SmsgSpiritHealerConfirm {
                            healer_guid: source_guid,
                        },
                    );
                }
            }
            gossip_option::QUESTGIVER => {
                let quests = self.quest_menu(world, player_guid, source_guid);
                self.player_mgr
                    .with_player_mut(player_guid, |p| p.gossip.quests = quests);
                self.send_prepared_quest(world, player_guid, source_guid)
                    .await?;
            }
            gossip_option::VENDOR | gossip_option::ARMORER => {
                // A positive sub-menu number picks the vendor's own goods (1), its shared
                // template (2) or both.
                let menu = match option.action_menu {
                    menu if menu > 0 => menu as u32,
                    _ => crate::game::npc::vendor::manager::VENDOR_MENU_ALL,
                };
                world
                    .systems
                    .vendor
                    .send_vendor_menu(player_guid, source_guid, menu)
                    .await?;
            }
            gossip_option::STABLEPET => {
                debug!("Stable masters are not implemented");
            }
            gossip_option::TRAINER => {
                crate::handlers::trainer_handler::send_trainer_list(
                    player_guid,
                    source_guid,
                    world,
                )
                .await?;
            }
            gossip_option::UNLEARNTALENTS => {
                self.send_gossip_complete(player_guid);
                let cost = world.systems.talents.reset_cost(player_guid, world);
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    MsgTalentWipeConfirm {
                        trainer_guid: source_guid,
                        cost,
                    },
                );
            }
            gossip_option::UNLEARNPETSKILLS => {
                // Never offered: hunter pets keep no spell book to untrain.
                self.send_gossip_complete(player_guid);
            }
            gossip_option::TAXIVENDOR => {
                crate::game::taxi::flight::talk_to_flight_master(world, player_guid, source_guid);
            }
            gossip_option::INNKEEPER => {
                // The client confirms the new home with CMSG_BINDER_ACTIVATE.
                self.send_gossip_complete(player_guid);
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    oxcore_shared::messages::login::SmsgBinderConfirm {
                        binder: source_guid,
                    },
                );
            }
            gossip_option::BANKER => {
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    SmsgShowBank {
                        banker_guid: source_guid,
                    },
                );
                self.player_mgr.with_player_mut(player_guid, |p| {
                    p.current_banker_guid = Some(source_guid);
                });
            }
            gossip_option::PETITIONER => {
                self.send_gossip_complete(player_guid);
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    oxcore_shared::messages::SmsgPetitionShowlist {
                        npc_guid: source_guid,
                    },
                );
            }
            gossip_option::TABARDDESIGNER => {
                self.send_gossip_complete(player_guid);
                self.broadcast_mgr.send_msg_to_player(
                    player_guid,
                    oxcore_shared::messages::MsgTabardVendorActivate {
                        designer_guid: source_guid,
                    },
                );
            }
            gossip_option::AUCTIONEER => {
                if let Some(session) = world.session_mgr.get_session_by_player(player_guid) {
                    crate::handlers::auction::send_auction_hello_response(
                        &session,
                        player_guid,
                        source_guid,
                        &world.managers.player_mgr,
                        &world.managers.creature_mgr,
                        &world.managers.auction_mgr,
                    )?;
                }
            }
            gossip_option::SPIRITGUIDE => {
                self.prepare_gossip_menu(world, player_guid, source_guid, 0);
                self.send_prepared_gossip(world, player_guid, source_guid)
                    .await?;
            }
            gossip_option::BATTLEFIELD => {
                let Some(entry) = self.creature_mgr.with_creature(source_guid, |c| c.entry) else {
                    return Ok(());
                };
                match world.managers.battleground_mgr.battlemaster_bg(entry) {
                    Some(bg_type) => crate::game::battleground::system::send_list(
                        world,
                        player_guid,
                        source_guid,
                        bg_type,
                    ),
                    None => warn!(
                        "Player {:?} asked creature entry {} for battlegrounds but it is no battlemaster",
                        player_guid, entry
                    ),
                }
            }
            other => {
                debug!("Gossip option type {} has no handler", other);
            }
        }

        // An option's script runs with the creature acting on the player; a game object's
        // runs from the player towards the object.
        if option.action_script != 0 {
            let (source, target) = if source_guid.is_game_object() {
                (player_guid, source_guid)
            } else {
                (source_guid, player_guid)
            };
            db_scripts::start(
                world,
                ScriptTable::Gossip,
                option.action_script,
                Some(source),
                Some(target),
            );
        }

        Ok(())
    }

    /// The option at `list_index` of the menu `player_guid` was shown at `source_guid`.
    pub fn shown_option(
        &self,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
        list_index: u32,
    ) -> Option<PreparedGossipOption> {
        self.player_mgr
            .with_player(player_guid, |p| {
                (p.gossip.source == source_guid)
                    .then(|| p.gossip.options.get(list_index as usize).cloned())
                    .flatten()
            })
            .flatten()
    }

    /// Marks a point of interest on the player's map.
    pub fn send_point_of_interest(&self, player_guid: ObjectGuid, poi_id: u32) {
        let Some(poi) = self.manager.get_point_of_interest(poi_id) else {
            warn!("Gossip option points to unknown point of interest {}", poi_id);
            return;
        };
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGossipPoi {
                poi_id: poi.entry,
                x: poi.x,
                y: poi.y,
                icon: poi.icon,
                flags: poi.flags,
                data: poi.data,
                name: poi.icon_name,
            },
        );
    }

    /// Send gossip complete (close window)
    pub fn send_gossip_complete(&self, player_guid: ObjectGuid) {
        self.broadcast_mgr
            .send_msg_to_player(player_guid, SmsgGossipComplete);
    }

    /// The greeting for `menu_id`: of the rows whose condition the player meets, the one
    /// with the highest condition id; an unconditioned row only counts when no
    /// conditioned row is met. The chosen row's gossip script starts, with the source
    /// acting on the player.
    pub fn gossip_text_id(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
        menu_id: u32,
    ) -> u32 {
        if menu_id == 0 {
            return DEFAULT_GOSSIP_MESSAGE;
        }
        let mut text_id = DEFAULT_GOSSIP_MESSAGE;
        let mut script_id = 0;
        let mut best_condition = 0;
        for menu in self.manager.get_menus(menu_id) {
            let fits = if menu.condition_id == 0 {
                best_condition == 0
            } else {
                menu.condition_id > best_condition
                    && self.condition_met(world, menu.condition_id, player_guid, source_guid)
            };
            if fits {
                best_condition = menu.condition_id;
                text_id = menu.text_id;
                script_id = menu.script_id;
            }
        }
        if script_id != 0 {
            db_scripts::start(
                world,
                ScriptTable::Gossip,
                script_id,
                Some(source_guid),
                Some(player_guid),
            );
        }
        text_id
    }

    /// The greeting a creature spawn shows without a menu (`npc_gossip`).
    fn source_text_id(&self, world: &World, source_guid: ObjectGuid) -> u32 {
        if !source_guid.is_creature() {
            return DEFAULT_GOSSIP_MESSAGE;
        }
        world
            .managers
            .creature_mgr
            .with_creature(source_guid, |c| c.spawn_id)
            .and_then(|spawn_id| self.manager.get_npc_gossip_text(spawn_id))
            .unwrap_or(DEFAULT_GOSSIP_MESSAGE)
    }

    /// The first line of a creature's greeting, used as the heading of its quest list.
    fn quest_list_greeting(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
    ) -> QuestListGreeting {
        if !source_guid.is_creature_or_pet() {
            return QuestListGreeting::default();
        }
        let menu_id = self.default_menu_id(world, source_guid);
        let text_id = if menu_id != 0 {
            self.gossip_text_id(world, player_guid, source_guid, menu_id)
        } else {
            self.source_text_id(world, source_guid)
        };
        if text_id == DEFAULT_GOSSIP_MESSAGE {
            return QuestListGreeting::default();
        }
        let Some(broadcast) = self
            .manager
            .get_npc_text(text_id)
            .map(|t| t.options[0].broadcast_text_id)
            .filter(|&id| id != 0)
            .and_then(|id| self.manager.get_broadcast_text(id))
        else {
            return QuestListGreeting::default();
        };
        let gender = creature_gender(world, source_guid);
        QuestListGreeting {
            text: broadcast_text_for(&broadcast, gender),
            emote_delay: broadcast.emote_delays[0],
            emote: broadcast.emote_ids[0],
        }
    }

    fn send_gossip_menu(
        &self,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
        menu: &PlayerGossipMenu,
        text_id: u32,
    ) {
        let options = menu
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| GossipOptionData {
                index: index as u32,
                icon: option.icon,
                coded: option.coded,
                money: option.box_money,
                text: option.text.clone(),
            })
            .collect();
        let quests = menu
            .quests
            .iter()
            .map(|q| oxcore_shared::messages::GossipQuestData {
                quest_id: q.quest_id,
                icon: q.icon,
                level: q.level,
                title: q.title.clone(),
            })
            .collect();
        self.broadcast_mgr.send_msg_to_player(
            player_guid,
            SmsgGossipMessage {
                source_guid,
                menu_id: menu.menu_id,
                text_id,
                options,
                quests,
            },
        );
    }

    /// The source's quests as the player sees them in a menu.
    fn quest_menu(
        &self,
        world: &World,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
    ) -> Vec<GossipQuestData> {
        let quest = &world.systems.quest;
        if source_guid.is_game_object() {
            world
                .managers
                .gameobject_mgr
                .with_gameobject(source_guid, |go| go.entry)
                .map(|entry| quest.prepare_gameobject_quest_menu(player_guid, entry, world))
                .unwrap_or_default()
        } else {
            self.creature_mgr
                .with_creature(source_guid, |c| c.entry)
                .map(|entry| quest.prepare_quest_menu(player_guid, entry, world))
                .unwrap_or_default()
        }
    }

    /// Whether a creature's service option applies to this player.
    fn creature_option_shown(
        &self,
        world: &World,
        item: &GossipMenuItem,
        entry: u32,
        viewer: &Viewer,
        player_guid: ObjectGuid,
    ) -> bool {
        let template = self.creature_mgr.get_template(entry);
        let trainer_kind = template.as_ref().map(|t| t.trainer_type);
        let trainer = self.creature_mgr.trainer_requirements(entry);
        match item.option_id {
            gossip_option::GOSSIP | gossip_option::TAXIVENDOR => true,
            // Quests have their own section of the window, and repairs come with the
            // vendor window.
            gossip_option::QUESTGIVER | gossip_option::ARMORER => false,
            gossip_option::SPIRITHEALER => !viewer.alive,
            gossip_option::VENDOR => {
                let has_items = !world
                    .systems
                    .vendor_manager
                    .get_vendor_items(entry)
                    .is_empty();
                if !has_items {
                    warn!("Creature entry {} is a vendor with nothing to sell", entry);
                }
                has_items
            }
            gossip_option::TRAINER => trainer_kind
                .is_some_and(|kind| self.is_trainer_of(world, entry, kind, viewer, player_guid)),
            gossip_option::UNLEARNTALENTS => {
                viewer.level >= 10
                    && trainer_kind == Some(trainer_type::CLASS)
                    && viewer.class == trainer.class
            }
            // Hunter pets do not keep a spell book yet, so there is nothing to untrain.
            gossip_option::UNLEARNPETSKILLS => false,
            gossip_option::BATTLEFIELD => world
                .managers
                .battleground_mgr
                .battlemaster_bg(entry)
                .is_some_and(|bg| {
                    crate::game::battleground::system::has_level_access(world, player_guid, bg)
                }),
            gossip_option::STABLEPET => viewer.class == CLASS_HUNTER,
            gossip_option::SPIRITGUIDE
            | gossip_option::INNKEEPER
            | gossip_option::BANKER
            | gossip_option::PETITIONER
            | gossip_option::TABARDDESIGNER
            | gossip_option::AUCTIONEER => true,
            other => {
                warn!(
                    "Creature entry {} has unknown gossip option {} in menu {}",
                    entry, other, item.menu_id
                );
                false
            }
        }
    }

    /// Whether the trainer teaches this player: class trainers their class, pet trainers
    /// hunters, mount trainers their race (or anyone exalted with them), profession
    /// trainers those who know the profession.
    fn is_trainer_of(
        &self,
        world: &World,
        entry: u32,
        kind: u8,
        viewer: &Viewer,
        player_guid: ObjectGuid,
    ) -> bool {
        if world.systems.trainer_manager.get_trainer_spells(entry).is_empty() {
            warn!("Creature entry {} is a trainer with nothing to teach", entry);
            return false;
        }
        let requirements = self.creature_mgr.trainer_requirements(entry);
        match kind {
            trainer_type::CLASS => viewer.class == requirements.class,
            trainer_type::PET => viewer.class == CLASS_HUNTER,
            trainer_type::MOUNT => {
                requirements.race == 0
                    || viewer.race == requirements.race
                    || self.exalted_with_creature(world, entry, player_guid)
            }
            trainer_type::TRADESKILL => {
                requirements.spell == 0
                    || self
                        .player_mgr
                        .with_player(player_guid, |p| p.spells.knows_spell(requirements.spell))
                        .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn exalted_with_creature(&self, world: &World, entry: u32, player_guid: ObjectGuid) -> bool {
        let Some(faction_template) = self.creature_mgr.get_template(entry).map(|t| t.faction)
        else {
            return false;
        };
        let Some(faction) = world
            .dbc
            .read()
            .get_faction_template(faction_template)
            .map(|t| t.faction)
        else {
            return false;
        };
        world.systems.reputation.get_rank(player_guid, faction, world) == Some(ReputationRank::Exalted)
    }

    fn condition_met(
        &self,
        world: &World,
        condition_id: u32,
        player_guid: ObjectGuid,
        source_guid: ObjectGuid,
    ) -> bool {
        world.managers.condition_mgr.is_satisfied(
            condition_id,
            ConditionContext {
                target: player_guid,
                source: source_guid,
            },
            world,
        )
    }

    /// The broadcast text in the player's gender when one is set, else the literal text.
    fn text_or_broadcast(&self, literal: &str, broadcast_id: u32, gender: u8) -> String {
        if broadcast_id != 0 {
            if let Some(broadcast) = self.manager.get_broadcast_text(broadcast_id) {
                return broadcast_text_for(&broadcast, gender);
            }
        }
        literal.to_string()
    }

    /// Initialize the gossip system
    pub async fn init(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the gossip system
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// The female text for women and the genderless when there is one, else the male text.
fn broadcast_text_for(broadcast: &super::types::BroadcastText, gender: u8) -> String {
    const GENDER_MALE: u8 = 0;
    if gender != GENDER_MALE && !broadcast.female_text.is_empty() {
        broadcast.female_text.clone()
    } else {
        broadcast.male_text.clone()
    }
}

fn creature_gender(world: &World, guid: ObjectGuid) -> u8 {
    world
        .managers
        .creature_mgr
        .with_creature(guid, |c| c.display_id)
        .and_then(|display| world.managers.creature_mgr.get_model_info(display))
        .map(|info| info.gender)
        .unwrap_or(0)
}
