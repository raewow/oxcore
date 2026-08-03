//! Gossip Manager - state storage and database loading
//!
//! Manages gossip menu data loaded from the database.
//! Provides thread-safe access using DashMap.

use anyhow::Result;
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

use super::types::{BroadcastText, GossipMenu, GossipMenuItem, NpcText, NpcTextOption};
use oxcore_db::database::world::repositories::GossipRepository;

/// Manages gossip menu data (state storage + database loading)
pub struct GossipManager {
    /// Database pool for loading
    world_db: Arc<PgPool>,
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
    /// Gossip text override by creature spawn guid low
    npc_gossip_texts: DashMap<u32, u32>,
}

impl GossipManager {
    /// Create a new gossip manager with database pool
    pub fn new(world_db: Arc<PgPool>) -> Self {
        Self {
            world_db,
            menus: DashMap::new(),
            menu_items: DashMap::new(),
            npc_texts: DashMap::new(),
            broadcast_texts: DashMap::new(),
            creature_default_menus: DashMap::new(),
            npc_gossip_texts: DashMap::new(),
        }
    }

    /// Load all gossip data from the database
    pub async fn load(&self) -> Result<()> {
        let repo = GossipRepository::new(Arc::clone(&self.world_db));
        let data = repo.load_all().await?;

        // Load menus
        for row in &data.menus {
            self.add_menu(GossipMenu {
                entry: row.entry.try_into()?,
                text_id: row.text_id.try_into()?,
                script_id: 0,
                condition_id: row.condition_id.try_into()?,
            });
        }

        // Load menu items
        for row in &data.options {
            self.add_menu_item(GossipMenuItem {
                menu_id: row.menu_id.try_into()?,
                id: row.id.try_into()?,
                option_icon: row.option_icon.try_into()?,
                option_text: row.option_text.clone().unwrap_or_default(),
                option_broadcast_text: row.option_broadcast_text.try_into()?,
                option_id: row.option_id.try_into()?,
                npc_option_npcflag: row.npc_option_npcflag.try_into()?,
                action_menu_id: row.action_menu_id,
                action_poi_id: row.action_poi_id.try_into()?,
                action_script_id: row.action_script_id.try_into()?,
                box_coded: row.box_coded,
                box_money: row.box_money.try_into()?,
                box_text: row.box_text.clone().unwrap_or_default(),
                box_broadcast_text: row.box_broadcast_text.try_into()?,
                condition_id: row.condition_id.try_into()?,
            });
        }

        // Load NPC texts
        for row in &data.npc_texts {
            let mut text = NpcText::new(row.id.try_into()?);
            text.options[0] = NpcTextOption {
                probability: row.probability0,
                broadcast_text_id: row.broadcast_text_id0.try_into()?,
            };
            text.options[1] = NpcTextOption {
                probability: row.probability1,
                broadcast_text_id: row.broadcast_text_id1.try_into()?,
            };
            text.options[2] = NpcTextOption {
                probability: row.probability2,
                broadcast_text_id: row.broadcast_text_id2.try_into()?,
            };
            text.options[3] = NpcTextOption {
                probability: row.probability3,
                broadcast_text_id: row.broadcast_text_id3.try_into()?,
            };
            text.options[4] = NpcTextOption {
                probability: row.probability4,
                broadcast_text_id: row.broadcast_text_id4.try_into()?,
            };
            text.options[5] = NpcTextOption {
                probability: row.probability5,
                broadcast_text_id: row.broadcast_text_id5.try_into()?,
            };
            text.options[6] = NpcTextOption {
                probability: row.probability6,
                broadcast_text_id: row.broadcast_text_id6.try_into()?,
            };
            text.options[7] = NpcTextOption {
                probability: row.probability7,
                broadcast_text_id: row.broadcast_text_id7.try_into()?,
            };
            self.add_npc_text(text);
        }

        // Load broadcast texts
        for row in &data.broadcast_texts {
            self.add_broadcast_text(BroadcastText {
                entry: row.entry.try_into()?,
                male_text: row.male_text.clone().unwrap_or_default(),
                female_text: row.female_text.clone().unwrap_or_default(),
                chat_type: row.chat_type.try_into()?,
                language_id: row.language_id.try_into()?,
                sound_id: row.sound_id.try_into()?,
                emote_ids: [
                    row.emote_id1.try_into()?,
                    row.emote_id2.try_into()?,
                    row.emote_id3.try_into()?,
                ],
                emote_delays: [
                    row.emote_delay1.try_into()?,
                    row.emote_delay2.try_into()?,
                    row.emote_delay3.try_into()?,
                ],
            });
        }

        // Load creature default menus
        for row in &data.creature_menus {
            self.set_creature_menu(row.entry.try_into()?, row.gossip_menu_id.try_into()?);
        }

        for row in &data.npc_gossip {
            self.set_npc_gossip_text(row.npc_guid.try_into()?, row.textid.try_into()?);
        }

        info!(
            "GossipManager loaded: {} menus, {} options, {} npc_texts, {} broadcast_texts, {} creature_menus, {} npc_gossip overrides",
            data.menus.len(),
            data.options.len(),
            data.npc_texts.len(),
            data.broadcast_texts.len(),
            data.creature_menus.len(),
            data.npc_gossip.len()
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

    /// Get per-creature gossip text override by spawn guid low
    pub fn get_npc_gossip_text(&self, npc_guid_low: u32) -> Option<u32> {
        self.npc_gossip_texts.get(&npc_guid_low).map(|v| *v)
    }

    /// Set per-creature gossip text override by spawn guid low
    pub fn set_npc_gossip_text(&self, npc_guid_low: u32, text_id: u32) {
        self.npc_gossip_texts.insert(npc_guid_low, text_id);
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
