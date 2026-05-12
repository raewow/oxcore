//! Gossip Manager - state storage and database loading
//!
//! Manages gossip menu data loaded from the database.
//! Provides thread-safe access using DashMap.

use anyhow::Result;
use dashmap::DashMap;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::info;

use super::types::{BroadcastText, GossipMenu, GossipMenuItem, NpcText, NpcTextOption};
use crate::shared::database::world::repositories::GossipRepository;

/// Manages gossip menu data (state storage + database loading)
pub struct GossipManager {
    /// Database pool for loading
    world_db: Arc<MySqlPool>,
    /// Menus by entry ID (can have multiple menus with same entry for conditions)
    menus: DashMap<u32, Vec<Arc<GossipMenu>>>,
    /// Menu items by menu entry ID
    menu_items: DashMap<u32, Vec<GossipMenuItem>>,
    /// NPC text by ID
    npc_texts: DashMap<u32, NpcText>,
    /// Broadcast text by entry ID
    broadcast_texts: DashMap<u32, BroadcastText>,
    /// Default gossip menu ID by creature entry
    creature_default_menus: DashMap<u32, u32>,
}

impl GossipManager {
    /// Create a new gossip manager with database pool
    pub fn new(world_db: Arc<MySqlPool>) -> Self {
        Self {
            world_db,
            menus: DashMap::new(),
            menu_items: DashMap::new(),
            npc_texts: DashMap::new(),
            broadcast_texts: DashMap::new(),
            creature_default_menus: DashMap::new(),
        }
    }

    /// Load all gossip data from the database
    pub async fn load(&self) -> Result<()> {
        let repo = GossipRepository::new(Arc::clone(&self.world_db));
        let data = repo.load_all().await?;

        // Load menus
        for row in &data.menus {
            self.add_menu(GossipMenu {
                entry: row.entry,
                text_id: row.text_id,
                script_id: 0,
                condition_id: row.condition_id,
            });
        }

        // Load menu items
        for row in &data.options {
            self.add_menu_item(GossipMenuItem {
                menu_id: row.menu_id,
                id: row.id,
                option_icon: row.option_icon,
                option_text: row.option_text.clone().unwrap_or_default(),
                option_broadcast_text: row.option_broadcast_text,
                option_id: row.option_id,
                npc_option_npcflag: row.npc_option_npcflag,
                action_menu_id: row.action_menu_id,
                action_poi_id: row.action_poi_id,
                action_script_id: row.action_script_id,
                box_coded: row.box_coded,
                box_money: row.box_money,
                box_text: row.box_text.clone().unwrap_or_default(),
                box_broadcast_text: row.box_broadcast_text,
                condition_id: row.condition_id,
            });
        }

        // Load NPC texts
        for row in &data.npc_texts {
            let mut text = NpcText::new(row.id);
            text.options[0] = NpcTextOption {
                probability: row.probability0,
                broadcast_text_id: row.broadcast_text_id0,
            };
            text.options[1] = NpcTextOption {
                probability: row.probability1,
                broadcast_text_id: row.broadcast_text_id1,
            };
            text.options[2] = NpcTextOption {
                probability: row.probability2,
                broadcast_text_id: row.broadcast_text_id2,
            };
            text.options[3] = NpcTextOption {
                probability: row.probability3,
                broadcast_text_id: row.broadcast_text_id3,
            };
            text.options[4] = NpcTextOption {
                probability: row.probability4,
                broadcast_text_id: row.broadcast_text_id4,
            };
            text.options[5] = NpcTextOption {
                probability: row.probability5,
                broadcast_text_id: row.broadcast_text_id5,
            };
            text.options[6] = NpcTextOption {
                probability: row.probability6,
                broadcast_text_id: row.broadcast_text_id6,
            };
            text.options[7] = NpcTextOption {
                probability: row.probability7,
                broadcast_text_id: row.broadcast_text_id7,
            };
            self.add_npc_text(text);
        }

        // Load broadcast texts
        for row in &data.broadcast_texts {
            self.add_broadcast_text(BroadcastText {
                entry: row.entry,
                male_text: row.male_text.clone().unwrap_or_default(),
                female_text: row.female_text.clone().unwrap_or_default(),
                chat_type: row.chat_type,
                language_id: row.language_id as u32,
                sound_id: row.sound_id as u32,
                emote_ids: [
                    row.emote_id1 as u32,
                    row.emote_id2 as u32,
                    row.emote_id3 as u32,
                ],
                emote_delays: [row.emote_delay1, row.emote_delay2, row.emote_delay3],
            });
        }

        // Load creature default menus
        for row in &data.creature_menus {
            self.set_creature_menu(row.entry, row.gossip_menu_id);
        }

        info!(
            "GossipManager loaded: {} menus, {} options, {} npc_texts, {} broadcast_texts, {} creature_menus",
            data.menus.len(),
            data.options.len(),
            data.npc_texts.len(),
            data.broadcast_texts.len(),
            data.creature_menus.len()
        );

        Ok(())
    }

    /// Get all menus for an entry ID
    pub fn get_menus(&self, entry: u32) -> Vec<Arc<GossipMenu>> {
        self.menus
            .get(&entry)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get menu items for a menu entry
    pub fn get_menu_items(&self, menu_id: u32) -> Vec<GossipMenuItem> {
        self.menu_items
            .get(&menu_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get NPC text by ID
    pub fn get_npc_text(&self, text_id: u32) -> Option<NpcText> {
        self.npc_texts.get(&text_id).map(|t| t.clone())
    }

    /// Get broadcast text by entry ID
    pub fn get_broadcast_text(&self, entry: u32) -> Option<BroadcastText> {
        self.broadcast_texts.get(&entry).map(|t| t.clone())
    }

    /// Get default gossip menu ID for a creature
    pub fn get_creature_menu_id(&self, entry: u32) -> Option<u32> {
        self.creature_default_menus.get(&entry).map(|v| *v)
    }

    /// Add a gossip menu
    pub fn add_menu(&self, menu: GossipMenu) {
        self.menus
            .entry(menu.entry)
            .or_insert_with(Vec::new)
            .push(Arc::new(menu));
    }

    /// Add a menu item
    pub fn add_menu_item(&self, item: GossipMenuItem) {
        self.menu_items
            .entry(item.menu_id)
            .or_insert_with(Vec::new)
            .push(item);
    }

    /// Add an NPC text entry
    pub fn add_npc_text(&self, text: NpcText) {
        self.npc_texts.insert(text.id, text);
    }

    /// Add a broadcast text entry
    pub fn add_broadcast_text(&self, text: BroadcastText) {
        self.broadcast_texts.insert(text.entry, text);
    }

    /// Set default gossip menu for a creature
    pub fn set_creature_menu(&self, entry: u32, menu_id: u32) {
        self.creature_default_menus.insert(entry, menu_id);
    }

    /// Get the text ID for a menu entry
    /// Returns the first menu's text_id (for simple cases without conditions)
    pub fn get_text_id(&self, entry: u32) -> Option<u32> {
        self.menus
            .get(&entry)
            .and_then(|v| v.first().map(|m| m.text_id))
    }

    /// Get localized text for a broadcast text entry
    /// Returns male text if available, female text as fallback
    pub fn get_localized_text(&self, entry: u32, _is_female: bool) -> Option<String> {
        self.get_broadcast_text(entry)
            .map(|t| {
                if t.male_text.is_empty() {
                    t.female_text
                } else {
                    t.male_text
                }
            })
            .filter(|s| !s.is_empty())
    }
}
