//! Player System
//!
//! Manages online players, GUID generation, and player data.

pub mod auras;
pub mod broadcaster;
pub mod death;
pub mod environment;
pub mod experience;
pub mod honor;
pub mod manager;
pub mod movement;
pub mod name_validation;
pub mod packet_queue;
pub mod player;
pub mod power;
pub mod reputation;
pub mod settings;
pub mod skills;
pub mod spells;
pub mod stats;
pub mod system;
pub mod talents;
pub mod visibility;

// Re-export combat types from game::combat
pub use crate::world::game::combat::{
    calculate_hit_table, calculate_melee_damage, AttackHand, AttackOutcome, CombatSnapshot,
    CombatState, DamageResult, PendingAttack,
};

pub use auras::{
    Aura, AuraContainer, AuraFlags, AuraState, AuraSystem, AuraTickSnapshot, ProcCandidate,
    MAX_NEGATIVE_AURA_SLOTS, MAX_PASSIVE_AURA_SLOTS, MAX_POSITIVE_AURA_SLOTS, MAX_SPELL_EFFECTS,
    MAX_TOTAL_AURA_SLOTS, NEGATIVE_SLOT_END, NEGATIVE_SLOT_START, PASSIVE_SLOT_END,
    PASSIVE_SLOT_START, POSITIVE_SLOT_END, POSITIVE_SLOT_START,
};
pub use broadcaster::PlayerBroadcaster;
pub use death::{
    apply_death_durability_loss, apply_spirit_healer_durability_loss, build_player_repop,
    can_reclaim_corpse, can_release_spirit, compute_resurrection_sickness,
    create_corpse_from_player, decline_resurrection, find_closest_graveyard,
    get_corpse_reclaim_delay, get_ghost_speed_multiplier, get_resurrection_sickness_duration,
    get_resurrection_sickness_spell_id, is_resurrection_requested_by,
    is_within_corpse_reclaim_range, offer_resurrection, remove_ghost_form, resurrect_at_corpse,
    resurrect_at_spirit_healer, resurrect_from_spell, should_apply_durability_loss, team_from_race,
    tick_death_timer, Corpse, CorpseType, DeathState, DeathSystem, DeathSystemState, GraveyardData,
    ResurrectionData, ResurrectionMethod, CORPSE_RECLAIM_DELAY_NORMAL, CORPSE_RECLAIM_DELAY_PVP,
    CORPSE_RECLAIM_RADIUS, CORPSE_REPOP_TIME_MS, DEATH_DURABILITY_LOSS, DEATH_SICKNESS_LEVEL,
    GHOST_SPEED_MULTIPLIER, GHOST_SPEED_MULTIPLIER_BG, PLAYER_FLAGS_GHOST, SPELL_AURA_GHOST,
    SPELL_RESURRECTION_SICKNESS, SPELL_WISP_FORM, SPIRIT_HEALER_DURABILITY_LOSS, TEAM_ALLIANCE,
    TEAM_BOTH, TEAM_HORDE, UNIT_FLAG_DISABLE_MOVE,
};
pub use environment::{
    apply_rest_bonus, calculate_fall_damage, calculate_offline_rest,
    on_mirror_timer_expiration_pulse, on_rest_login, set_rest_type,
    update_environment_flags_internal, update_mirror_timers, update_rest_bonus, EnvironmentFlags,
    EnvironmentState, EnvironmentSystem, LiquidStatus, LiquidType, MirrorTimer, MirrorTimerAction,
    MirrorTimerEvent, MirrorTimerStatus, MirrorTimerType, RestType, BREATH_MAX_SECONDS,
    ENVIRONMENTAL_DAMAGE_MAX, ENVIRONMENTAL_DAMAGE_MIN, ENVIRONMENTAL_MAX_SECONDS,
    FATIGUE_MAX_SECONDS, PLAYER_FLAGS_RESTING, REST_RATE_PER_SECOND, SAFE_FALL_DISTANCE,
};
pub use experience::ExperienceSystem;
pub use manager::PlayerManager;
pub use movement::{MovementState, MovementSystem};
pub use player::Player;
pub use power::{PowerState, PowerSystem, PowerType};
pub use reputation::{FactionEntry, FactionStanding, ReputationState, ReputationSystem};
pub use settings::{
    compress_account_data, decompress_account_data, AccountDataEntry, AccountDataType,
    ActionButton, MacroEntry, SettingsState, SettingsSystem, ACTION_BUTTON_ITEM,
    ACTION_BUTTON_MACRO, ACTION_BUTTON_SPELL, MAX_ACTION_BUTTONS, MAX_MACROS,
    NUM_ACCOUNT_DATA_TYPES, TUTORIAL_FLAG_COUNT,
};
pub use skills::{
    calculate_defense_skill_up_chance, calculate_weapon_skill_up_chance, get_initial_skill_value,
    get_skill_max_for_level, initialize_default_skills, ProficiencyMessage, SkillData,
    SkillSaveState, SkillState, SkillSystem, PLAYER_MAX_SKILLS, SKILL_2H_AXES, SKILL_2H_MACES,
    SKILL_2H_SWORDS, SKILL_AXES, SKILL_BOWS, SKILL_CLOTH, SKILL_CROSSBOWS, SKILL_DAGGERS,
    SKILL_DEFENSE, SKILL_DUAL_WIELD, SKILL_FIST_WEAPONS, SKILL_GUNS, SKILL_LEATHER, SKILL_MACES,
    SKILL_MAIL, SKILL_PLATE_MAIL, SKILL_POLEARMS, SKILL_SHIELD, SKILL_STAVES, SKILL_SWORDS,
    SKILL_THROWN, SKILL_UNARMED, SKILL_WANDS,
};
pub use spells::{
    ActiveCast, SpellCastError, SpellCastResult, SpellMod, SpellModOp, SpellModType, SpellSchool,
    SpellSystem, SpellsState, NUM_SPELL_SCHOOLS,
};
pub use system::PlayerSystem;
pub use talents::{
    TalentInfo, TalentState, TalentStore, TalentSystem, TalentTabInfo, MAX_TALENT_COLUMNS,
    MAX_TALENT_RANK, MAX_TALENT_ROWS, MAX_TALENT_TABS, POINTS_PER_ROW,
};
pub use visibility::{VisibilityState, VisibilitySubsystem};
