//! Slim Player object - only identity and persistent data
//!
//! System state (stats, combat, auras, etc.) lives in respective systems.
//! Movement state (position, speeds) is embedded in the Player struct.
//! Visibility state (visible objects, pending notifications) is embedded in the Player struct.

use super::auras::AuraState;
use super::broadcaster::PlayerBroadcaster;
use super::death::DeathSystemState;
use super::environment::EnvironmentState;
use super::movement::MovementState;
use super::power::PowerState;
use super::reputation::ReputationState;
use super::settings::SettingsState;
use super::skills::SkillState;
use super::spells::SpellsState;
use super::stats::StatsState;
use super::talents::TalentState;
use super::visibility::VisibilityState;
use super::CombatState;
use crate::game::npc::quest::QuestProgress;
use oxcore_shared::protocol::ObjectGuid;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Slim player object
#[derive(Debug)]
pub struct Player {
    /// Unique identifier
    pub guid: ObjectGuid,
    /// Character name
    pub name: String,
    /// Current map ID
    pub map_id: u32,
    /// Current instance ID (0 for continents, >0 for dungeon/raid instances)
    pub instance_id: u32,
    /// Current zone ID
    pub zone_id: u32,
    /// Phase mask for visibility (bitfield, default 0x00000001 = normal phase)
    pub phase_mask: u32,
    /// Movement state (position, speeds, flags)
    pub movement: MovementState,
    /// Visibility state (visible objects, pending notifications)
    pub visibility: VisibilityState,
    /// Stats state (base + derived + modifier groups)
    pub stats: StatsState,
    /// Health floor enforced by temporary invulnerability effects such as Spirit of Redemption.
    pub invincibility_hp_threshold: u32,
    /// Power state (mana/rage/energy current/max + regen)
    pub power: PowerState,
    /// Combat state (timers, targets, weapon info)
    pub combat: CombatState,
    /// Creature currently charming this player.
    pub charmer_guid: Option<ObjectGuid>,
    /// Faction template temporarily inherited from the charmer.
    pub faction_override: Option<u32>,
    /// Unit currently controlling this player while charmed.
    pub controller_guid: Option<ObjectGuid>,
    /// Active controlled runtime pet.
    pub active_pet: Option<ObjectGuid>,
    /// Aura state (buffs/debuffs, slot management)
    pub auras: AuraState,
    /// Death state (death/resurrection state machine)
    pub death: DeathSystemState,
    /// Spells state (spellbook, cooldowns, active cast)
    pub spells: SpellsState,
    /// Skills state (weapon/defense skills)
    pub skills: SkillState,
    /// Talent state (talent allocations, free points)
    pub talents: TalentState,
    /// Character level (1-60)
    pub level: u8,
    /// Current experience points
    pub xp: u32,
    /// Experience required for next level
    pub next_level_xp: u32,
    /// Race ID
    pub race: u8,
    /// Class ID
    pub class: u8,
    /// Gender (0=male, 1=female)
    pub gender: u8,
    /// Appearance: Skin
    pub skin: u8,
    /// Appearance: Face
    pub face: u8,
    /// Appearance: Hair style
    pub hair_style: u8,
    /// Appearance: Hair color
    pub hair_color: u8,
    /// Appearance: Facial hair
    pub facial_hair: u8,
    /// Packet broadcaster for nearby players
    pub broadcaster: Option<Arc<PlayerBroadcaster>>,
    /// For rest state calculation
    pub rest_bonus: f32,
    /// From character_flags database
    pub player_flags: u32,
    /// Visual animation state
    pub stand_state: u8,
    /// Shapeshift form (0 = none)
    pub shapeshift_form: u8,
    /// Unit flags (UNIT_FLAG_DISABLE_MOVE, etc.)
    pub unit_flags: u32,
    /// Dynamic unit flags (UNIT_DYNFLAG_SPECIALINFO, etc.)
    pub dynamic_flags: u32,
    /// UNIT_FIELD_BYTES_1 byte offset 3 (vis flag byte): UNIT_VIS_FLAGS_CREEP (0x02) while stealthed.
    pub vis_flags_byte: u8,
    /// Bitmask of active SPELL_AURA_MOD_INVISIBILITY types (bit per invisibility type 0-31).
    pub invisibility_mask: u32,
    /// Bitmask of active SPELL_AURA_MOD_INVISIBILITY_DETECTION types.
    pub detect_invisibility_mask: u32,
    /// Model scale multiplier from SPELL_AURA_MOD_SCALE (1.0 = normal size).
    pub scale: f32,
    /// Creature display ID while mounted (0 = not mounted). Drives UNIT_FIELD_MOUNTDISPLAYID.
    pub mount_display_id: u32,
    /// Spell ID of the active SPELL_AURA_TRANSFORM aura (0 = not transformed).
    pub transform_spell_id: u32,
    /// Display ID applied by the active transform aura (0 = none / use native).
    pub transform_display_id: u32,
    /// PLAYER_TRACK_CREATURES bitmask (Track Beasts/Humanoids/etc).
    pub track_creatures_mask: u32,
    /// PLAYER_TRACK_RESOURCES bitmask (Track Herbs/Minerals/etc).
    pub track_resources_mask: u32,
    /// PLAYER_FIELD_BYTES2 byte offset 1 (extra flags byte): stealth/invis-glow/detect-amore/track-stealthed bits.
    pub player_bytes2_flags: u8,
    /// Aura state flags (bitmask of AURASTATE_* for spell requirements like Execute < 20% HP)
    pub aura_state_flags: u32,
    /// Active quests (quest log)
    pub active_quests: Vec<QuestProgress>,
    /// Completed and rewarded quests
    pub rewarded_quests: HashSet<u32>,
    /// Reputation state (64 faction slots)
    pub reputation: ReputationState,
    /// Settings state (action buttons, macros, tutorials, account data)
    pub settings: SettingsState,
    /// Money in copper
    pub money: u32,
    /// Equipped ammo item entry (`PLAYER_AMMO_ID`).
    pub ammo_id: u32,
    /// Environment state (rest XP, mirror timers, environmental hazards)
    pub environment: EnvironmentState,
    /// Target currently being looted
    pub looting_target: Option<ObjectGuid>,
    /// Current gossip menu ID (for tracking gossip state)
    pub current_gossip_menu_id: Option<u32>,
    /// Banker currently authorized for open bank operations.
    pub current_banker_guid: Option<ObjectGuid>,
    /// Currently selected unit/object (for targeting, gossip, vendors)
    pub selection: Option<ObjectGuid>,
    /// Auction access mode: 0 = normal, 1 = neutral, -1 = enemy faction
    /// (C++ Player::m_ExtraFlags PLAYER_EXTRA_AUCTION_NEUTRAL / PLAYER_EXTRA_AUCTION_ENEMY)
    pub auction_access_mode: i8,
    /// Account ID that owns this character (used for per-account DB tables such as tutorial flags).
    pub account_id: u32,
    /// Homebind map ID (hearthstone destination)
    pub homebind_map: u32,
    /// Homebind zone ID
    pub homebind_zone: u32,
    /// Homebind X coordinate
    pub homebind_x: f32,
    /// Homebind Y coordinate
    pub homebind_y: f32,
    /// Homebind Z coordinate
    pub homebind_z: f32,
    /// Self-resurrection spell ID (PLAYER_SELF_RES_SPELL update field).
    /// Set at death time if the player has an active Soulstone / Reincarnation
    /// / Twisting Nether / etc. The client reads this field to decide whether
    /// to light up the "Accept" button on the death screen. Cleared when
    /// CMSG_SELF_RES consumes it or on normal resurrection.
    pub self_res_spell: u32,
    /// Pending quest-share info set by a party member who wants to share a quest.
    /// Cleared when the player accepts/declines or when the sharer goes away.
    pub quest_share_info: Option<QuestShareInfo>,
    /// Active item set bonuses keyed by item set ID.
    pub item_set_effects: HashMap<u32, ItemSetEffect>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSetEffect {
    pub item_count: u32,
    pub spells: [Option<u32>; 8],
}

/// Tracks a quest shared by another player pending confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestShareInfo {
    /// GUID of the player who initiated the share.
    pub player_guid: ObjectGuid,
    /// Quest ID being shared.
    pub quest_id: u32,
}

impl Player {
    /// Create a new player
    pub fn new(
        guid: ObjectGuid,
        name: String,
        map_id: u32,
        instance_id: u32,
        zone_id: u32,
        level: u8,
        race: u8,
        class: u8,
        gender: u8,
    ) -> Self {
        Self {
            guid,
            name,
            map_id,
            instance_id,
            zone_id,
            phase_mask: 0x00000001, // Default to normal phase
            movement: MovementState::default(),
            visibility: VisibilityState::default(),
            stats: StatsState::default(),
            invincibility_hp_threshold: 0,
            power: PowerState::default(),
            combat: CombatState::default(),
            charmer_guid: None,
            faction_override: None,
            controller_guid: None,
            active_pet: None,
            auras: AuraState::default(),
            death: DeathSystemState::default(),
            spells: SpellsState::default(),
            skills: SkillState::default(),
            talents: TalentState::default(),
            level,
            xp: 0,
            next_level_xp: 0,
            race,
            class,
            gender,
            skin: 0,
            face: 0,
            hair_style: 0,
            hair_color: 0,
            facial_hair: 0,
            broadcaster: None,
            rest_bonus: 0.0,
            player_flags: 0,
            stand_state: 0,
            shapeshift_form: 0,
            unit_flags: 0,
            dynamic_flags: 0,
            vis_flags_byte: 0,
            invisibility_mask: 0,
            detect_invisibility_mask: 0,
            scale: 1.0,
            mount_display_id: 0,
            transform_spell_id: 0,
            transform_display_id: 0,
            track_creatures_mask: 0,
            track_resources_mask: 0,
            player_bytes2_flags: 0,
            aura_state_flags: 0,
            active_quests: Vec::new(),
            rewarded_quests: HashSet::new(),
            reputation: ReputationState::new(),
            settings: SettingsState::default(),
            money: 0,
            ammo_id: 0,
            environment: EnvironmentState::default(),
            looting_target: None,
            current_gossip_menu_id: None,
            current_banker_guid: None,
            selection: None,
            auction_access_mode: 0,
            account_id: 0,
            homebind_map: map_id,
            homebind_zone: zone_id,
            homebind_x: 0.0,
            homebind_y: 0.0,
            homebind_z: 0.0,
            self_res_spell: 0,
            quest_share_info: None,
            item_set_effects: HashMap::new(),
        }
    }

    /// Apply health damage while respecting a temporary invincibility floor.
    /// Returns the actual health lost.
    pub fn apply_damage(&mut self, damage: u32) -> u32 {
        let old_health = self.stats.health;
        let new_health = old_health
            .saturating_sub(damage)
            .max(self.invincibility_hp_threshold.min(old_health));
        self.stats.health = new_health;
        self.stats.dirty = true;
        old_health - new_health
    }

    pub fn get_item_set_effect(&self, set_id: u32) -> Option<&ItemSetEffect> {
        self.item_set_effects.get(&set_id)
    }

    /// Apply creature charm state after the aura has successfully been added.
    pub fn apply_creature_charm(&mut self, charmer_guid: ObjectGuid, faction: u32) -> bool {
        if self.guid == charmer_guid || self.charmer_guid.is_some() {
            return false;
        }

        self.charmer_guid = Some(charmer_guid);
        self.faction_override = Some(faction);
        self.controller_guid = Some(charmer_guid);
        self.combat.in_combat = false;
        self.combat.combat_timer = 0;
        self.combat.attackers.clear();
        self.combat.stop_attack();
        self.combat.stop_shoot();
        true
    }

    /// Remove charm state only when it belongs to the given charmer.
    pub fn remove_creature_charm(&mut self, charmer_guid: ObjectGuid) -> bool {
        if self.charmer_guid != Some(charmer_guid) {
            return false;
        }

        self.charmer_guid = None;
        self.faction_override = None;
        self.controller_guid = None;
        true
    }

    /// Current faction template, including a temporary charm override.
    pub fn faction_template(&self) -> u32 {
        self.faction_override.unwrap_or_else(|| {
            crate::game::common::player_constants::get_faction_for_race(self.race)
        })
    }

    pub fn add_item_set_effect(&mut self, set_id: u32) -> &mut ItemSetEffect {
        self.item_set_effects.entry(set_id).or_default()
    }

    pub fn remove_item_set_effect(&mut self, set_id: u32) {
        self.item_set_effects.remove(&set_id);
    }

    /// Set appearance
    pub fn set_appearance(
        &mut self,
        skin: u8,
        face: u8,
        hair_style: u8,
        hair_color: u8,
        facial_hair: u8,
    ) {
        self.skin = skin;
        self.face = face;
        self.hair_style = hair_style;
        self.hair_color = hair_color;
        self.facial_hair = facial_hair;
    }

    /// Set the packet broadcaster
    pub fn set_broadcaster(&mut self, broadcaster: Arc<PlayerBroadcaster>) {
        self.broadcaster = Some(broadcaster);
    }

    /// Get the packet broadcaster
    pub fn broadcaster(&self) -> Option<Arc<PlayerBroadcaster>> {
        self.broadcaster.as_ref().map(Arc::clone)
    }

    /// Clear the packet broadcaster (for logout)
    pub fn clear_broadcaster(&mut self) {
        if let Some(broadcaster) = self.broadcaster.take() {
            broadcaster.free_at_logout();
        }
    }

    /// Check if player is alive
    pub fn is_alive(&self) -> bool {
        use super::death::DeathState;
        matches!(self.death.death_state, DeathState::Alive)
    }

    /// Get the player's team (Alliance/Horde/None)
    pub fn get_team(&self) -> oxcore_shared::game::chat::Team {
        oxcore_shared::game::chat::Team::from_race(self.race)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> Player {
        Player::new(
            ObjectGuid::new_player(1),
            "TestPlayer".to_string(),
            0,
            0,
            0,
            60,
            1,
            1,
            0,
        )
    }

    #[test]
    fn item_set_effects_start_empty() {
        let player = test_player();

        assert!(player.item_set_effects.is_empty());
        assert_eq!(player.get_item_set_effect(100), None);
    }

    #[test]
    fn add_item_set_effect_inserts_default_effect() {
        let mut player = test_player();

        let effect = player.add_item_set_effect(100);
        effect.item_count = 2;
        effect.spells[0] = Some(1234);

        assert_eq!(player.get_item_set_effect(100).unwrap().item_count, 2);
        assert_eq!(
            player.get_item_set_effect(100).unwrap().spells[0],
            Some(1234)
        );
    }

    #[test]
    fn remove_item_set_effect_erases_only_requested_set() {
        let mut player = test_player();
        player.add_item_set_effect(100).item_count = 2;
        player.add_item_set_effect(200).item_count = 4;

        player.remove_item_set_effect(100);

        assert_eq!(player.get_item_set_effect(100), None);
        assert_eq!(player.get_item_set_effect(200).unwrap().item_count, 4);
    }

    #[test]
    fn remove_missing_item_set_effect_is_noop() {
        let mut player = test_player();
        player.add_item_set_effect(200).item_count = 4;

        player.remove_item_set_effect(100);

        assert_eq!(player.item_set_effects.len(), 1);
        assert_eq!(player.get_item_set_effect(200).unwrap().item_count, 4);
    }
}
