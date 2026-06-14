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
use crate::shared::protocol::ObjectGuid;
use crate::world::game::npc::quest::QuestProgress;
use std::collections::HashSet;
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
    /// Power state (mana/rage/energy current/max + regen)
    pub power: PowerState,
    /// Combat state (timers, targets, weapon info)
    pub combat: CombatState,
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
    /// Environment state (rest XP, mirror timers, environmental hazards)
    pub environment: EnvironmentState,
    /// Target currently being looted
    pub looting_target: Option<ObjectGuid>,
    /// Current gossip menu ID (for tracking gossip state)
    pub current_gossip_menu_id: Option<u32>,
    /// Currently selected unit/object (for targeting, gossip, vendors)
    pub selection: Option<ObjectGuid>,
    /// Auction access mode: 0 = normal, 1 = neutral, -1 = enemy faction
    /// (C++ Player::m_ExtraFlags PLAYER_EXTRA_AUCTION_NEUTRAL / PLAYER_EXTRA_AUCTION_ENEMY)
    pub auction_access_mode: i8,
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
            power: PowerState::default(),
            combat: CombatState::default(),
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
            aura_state_flags: 0,
            active_quests: Vec::new(),
            rewarded_quests: HashSet::new(),
            reputation: ReputationState::new(),
            settings: SettingsState::default(),
            money: 0,
            environment: EnvironmentState::default(),
            looting_target: None,
            current_gossip_menu_id: None,
            selection: None,
            auction_access_mode: 0,
            homebind_map: map_id,
            homebind_zone: zone_id,
            homebind_x: 0.0,
            homebind_y: 0.0,
            homebind_z: 0.0,
            self_res_spell: 0,
        }
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
    pub fn get_team(&self) -> crate::shared::game::chat::Team {
        crate::shared::game::chat::Team::from_race(self.race)
    }
}
