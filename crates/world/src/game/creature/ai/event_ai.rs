//! EventAI: creature behaviour from creature_ai_events.
//!
//! Each event has a type (timer, health threshold, aggro, death, spell hit,
//! ...), a phase mask it is suppressed in, a chance, flags (repeatable, random
//! action, check result) and up to three actions, each a creature_ai_scripts
//! script run at once with the creature as source and the event's invoker (or
//! the victim) as target. Timed events are checked every half second. The
//! creature otherwise fights like a basic creature.

use super::types::{AIEvent, CombatEndReason};
use super::units::{self, UnitInfo};
use crate::game::db_scripts::{self, ScriptTable};
use crate::World;
use dashmap::DashMap;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub const EVENT_T_TIMER_IN_COMBAT: u32 = 0;
pub const EVENT_T_TIMER_OOC: u32 = 1;
pub const EVENT_T_HP: u32 = 2;
pub const EVENT_T_MANA: u32 = 3;
pub const EVENT_T_AGGRO: u32 = 4;
pub const EVENT_T_KILL: u32 = 5;
pub const EVENT_T_DEATH: u32 = 6;
pub const EVENT_T_EVADE: u32 = 7;
pub const EVENT_T_HIT_BY_SPELL: u32 = 8;
pub const EVENT_T_RANGE: u32 = 9;
pub const EVENT_T_OOC_LOS: u32 = 10;
pub const EVENT_T_SPAWNED: u32 = 11;
pub const EVENT_T_TARGET_HP: u32 = 12;
pub const EVENT_T_TARGET_CASTING: u32 = 13;
pub const EVENT_T_FRIENDLY_HP: u32 = 14;
pub const EVENT_T_FRIENDLY_IS_CC: u32 = 15;
pub const EVENT_T_FRIENDLY_MISSING_BUFF: u32 = 16;
pub const EVENT_T_SUMMONED_UNIT: u32 = 17;
pub const EVENT_T_TARGET_MANA: u32 = 18;
pub const EVENT_T_REACHED_HOME: u32 = 21;
pub const EVENT_T_RECEIVE_EMOTE: u32 = 22;
pub const EVENT_T_AURA: u32 = 23;
pub const EVENT_T_TARGET_AURA: u32 = 24;
pub const EVENT_T_SUMMONED_JUST_DIED: u32 = 25;
pub const EVENT_T_SUMMONED_JUST_DESPAWN: u32 = 26;
pub const EVENT_T_MISSING_AURA: u32 = 27;
pub const EVENT_T_TARGET_MISSING_AURA: u32 = 28;
pub const EVENT_T_MOVEMENT_INFORM: u32 = 29;
pub const EVENT_T_LEAVE_COMBAT: u32 = 30;
pub const EVENT_T_SCRIPT: u32 = 31;
pub const EVENT_T_GROUP_MEMBER_DIED: u32 = 32;
pub const EVENT_T_VICTIM_ROOTED: u32 = 33;
pub const EVENT_T_HIT_BY_AURA: u32 = 34;
pub const EVENT_T_STEALTH_ALERT: u32 = 35;
pub const EVENT_T_SPELL_HIT_TARGET: u32 = 36;

pub const EFLAG_REPEATABLE: u32 = 0x01;
pub const EFLAG_RANDOM_ACTION: u32 = 0x02;
pub const EFLAG_NOT_CASTING: u32 = 0x04;
pub const EFLAG_CHECK_RESULT: u32 = 0x08;
pub const EFLAG_DEBUG_ONLY: u32 = 0x10;

/// Timed events are checked this often.
const EVENT_UPDATE_TIME: u32 = 500;
/// Phases above this cannot be used in a phase mask.
const MAX_PHASE: u32 = 32;
/// OOC_LOS reactions.
const ULR_ANY: u32 = 0;
const ULR_HOSTILE: u32 = 1;
const ULR_NON_HOSTILE: u32 = 2;

/// One creature_ai_events row.
#[derive(Debug, Clone, Default)]
pub struct EventAiEvent {
    pub id: u32,
    pub creature_id: u32,
    pub condition_id: u32,
    pub event_type: u32,
    pub inverse_phase_mask: u32,
    pub chance: u32,
    pub flags: u32,
    pub params: [u32; 4],
    pub actions: [u32; 3],
}

fn events_by_entry() -> &'static DashMap<u32, Arc<Vec<EventAiEvent>>> {
    static EVENTS: OnceLock<DashMap<u32, Arc<Vec<EventAiEvent>>>> = OnceLock::new();
    EVENTS.get_or_init(DashMap::new)
}

/// Register the events of one creature entry.
pub fn insert_events(entry: u32, events: Vec<EventAiEvent>) {
    events_by_entry().insert(entry, Arc::new(events));
}

/// Load creature_ai_events.
pub async fn load(pool: &sqlx::PgPool) -> anyhow::Result<usize> {
    let rows = sqlx::query(
        "SELECT id::BIGINT AS id, creature_id::BIGINT AS creature_id, \
         condition_id::BIGINT AS condition_id, event_type::BIGINT AS event_type, \
         event_inverse_phase_mask::BIGINT AS event_inverse_phase_mask, \
         event_chance::BIGINT AS event_chance, event_flags::BIGINT AS event_flags, \
         event_param1::BIGINT AS event_param1, event_param2::BIGINT AS event_param2, \
         event_param3::BIGINT AS event_param3, event_param4::BIGINT AS event_param4, \
         action1_script::BIGINT AS action1_script, action2_script::BIGINT AS action2_script, \
         action3_script::BIGINT AS action3_script FROM world.creature_ai_events ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    let mut by_entry: HashMap<u32, Vec<EventAiEvent>> = HashMap::new();
    for row in &rows {
        let get = |name: &str| row.try_get::<i64, _>(name).unwrap_or(0);
        let event = EventAiEvent {
            id: get("id") as u32,
            creature_id: get("creature_id") as u32,
            condition_id: get("condition_id") as u32,
            event_type: get("event_type") as u32,
            inverse_phase_mask: get("event_inverse_phase_mask") as u32,
            chance: get("event_chance") as u32,
            flags: get("event_flags") as u32,
            params: [
                get("event_param1") as u32,
                get("event_param2") as u32,
                get("event_param3") as u32,
                get("event_param4") as u32,
            ],
            actions: [
                get("action1_script") as u32,
                get("action2_script") as u32,
                get("action3_script") as u32,
            ],
        };
        if event.flags & EFLAG_DEBUG_ONLY != 0 {
            continue;
        }
        by_entry.entry(event.creature_id).or_default().push(event);
    }
    for (entry, events) in by_entry {
        insert_events(entry, events);
    }
    Ok(rows.len())
}

/// Whether creatures of this entry have any events.
pub fn has_events(entry: u32) -> bool {
    events_by_entry().contains_key(&entry)
}

/// Whether creatures of this entry react to units coming into view out of combat.
pub fn has_ooc_los(entry: u32) -> bool {
    events_by_entry()
        .get(&entry)
        .is_some_and(|events| events.iter().any(|e| e.event_type == EVENT_T_OOC_LOS))
}

#[derive(Debug, Clone)]
struct Holder {
    event: EventAiEvent,
    time: u32,
    enabled: bool,
}

impl Holder {
    /// Set the repeat timer; a max below the min disables the event.
    fn update_repeat_timer(&mut self, min: u32, max: u32) -> bool {
        if min == max {
            self.time = min;
        } else if max > min {
            self.time = rand::thread_rng().gen_range(min..=max);
        } else {
            self.enabled = false;
            return false;
        }
        true
    }
}

#[derive(Debug, Clone)]
struct State {
    holders: Vec<Holder>,
    update_time: u32,
    event_diff: u32,
}

fn states() -> &'static DashMap<ObjectGuid, State> {
    static STATES: OnceLock<DashMap<ObjectGuid, State>> = OnceLock::new();
    STATES.get_or_init(DashMap::new)
}

// Phases live apart from the event state so that an action script can change
// the phase while the creature's events are being processed.
fn phases() -> &'static DashMap<ObjectGuid, u32> {
    static PHASES: OnceLock<DashMap<ObjectGuid, u32>> = OnceLock::new();
    PHASES.get_or_init(DashMap::new)
}

/// The creature's current EventAI phase.
pub fn phase(guid: ObjectGuid) -> Option<u32> {
    phases().get(&guid).map(|p| *p)
}

/// Set the creature's EventAI phase. False when it has no EventAI or the phase
/// cannot be masked.
pub fn set_phase(guid: ObjectGuid, phase: u32) -> bool {
    if phase >= MAX_PHASE {
        tracing::warn!("EventAI phase {} for {:?} is out of range", phase, guid);
        return false;
    }
    match phases().get_mut(&guid) {
        Some(mut current) => {
            *current = phase;
            true
        }
        None => false,
    }
}

/// Forget a creature's EventAI state (despawn).
pub fn forget(guid: ObjectGuid) {
    states().remove(&guid);
    phases().remove(&guid);
}

/// What the event checks need to know about the creature.
struct View {
    me: UnitInfo,
    current_mana: u32,
    max_mana: u32,
    victim: Option<UnitInfo>,
}

fn view(world: &World, guid: ObjectGuid) -> Option<View> {
    let me = units::resolve_unit(world, guid)?;
    let (current_mana, max_mana) = world
        .managers
        .creature_mgr
        .with_creature(guid, |c| (c.current_mana, c.max_mana))?;
    let victim = me.victim.and_then(|v| units::resolve_unit(world, v));
    Some(View {
        me,
        current_mana,
        max_mana,
        victim,
    })
}

fn aura_stacks(world: &World, unit: ObjectGuid, spell_id: u32) -> u32 {
    let count = |auras: &crate::game::player::auras::AuraContainer| {
        auras
            .all_auras()
            .filter(|a| a.spell_id == spell_id)
            .map(|a| a.stack_count as u32)
            .max()
            .unwrap_or(0)
    };
    if unit.is_player() {
        world
            .managers
            .player_mgr
            .with_player(unit, |p| count(&p.auras.container))
            .unwrap_or(0)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(unit, |c| count(&c.auras))
            .unwrap_or(0)
    }
}

fn mana_percent(world: &World, unit: &UnitInfo) -> Option<u32> {
    if unit.is_player {
        world
            .managers
            .player_mgr
            .with_player(unit.guid, |p| {
                (p.stats.max_mana > 0).then(|| p.stats.mana * 100 / p.stats.max_mana)
            })
            .flatten()
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(unit.guid, |c| {
                (c.max_mana > 0).then(|| c.current_mana * 100 / c.max_mana)
            })
            .flatten()
    }
}

/// Friendly units of the creature within `radius`, alive.
fn friends(world: &World, me: &UnitInfo, radius: f32) -> Vec<UnitInfo> {
    let Some(map) = world.managers.map_mgr.get_map(me.map_id, me.instance_id) else {
        return Vec::new();
    };
    map.get_objects_in_range(me.position, radius)
        .into_iter()
        .filter(|g| g.is_creature() || g.is_player())
        .filter_map(|g| units::resolve_unit(world, g))
        .filter(|u| {
            u.alive
                && u.map_id == me.map_id
                && u.instance_id == me.instance_id
                && units::distance(&u.position, &me.position) <= radius
                && units::reaction_to(world, me.faction, u) >= ReputationRank::Friendly
        })
        .collect()
}

fn max_health(world: &World, unit: &UnitInfo) -> u32 {
    if unit.is_player {
        world
            .managers
            .player_mgr
            .with_player(unit.guid, |p| p.stats.max_health)
            .unwrap_or(0)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(unit.guid, |c| c.max_health)
            .unwrap_or(0)
    }
}

fn is_casting(world: &World, unit: ObjectGuid) -> bool {
    use crate::game::player::spells::state::CurrentSpellType;
    unit.is_player()
        && world
            .managers
            .player_mgr
            .with_player(unit, |p| {
                p.spells.current_spells[CurrentSpellType::Generic as usize].is_some()
                    || p.spells.current_spells[CurrentSpellType::Channeled as usize].is_some()
            })
            .unwrap_or(false)
}

fn is_rooted(world: &World, unit: ObjectGuid) -> bool {
    const SPELL_AURA_MOD_ROOT: u32 = 26;
    if unit.is_player() {
        world
            .managers
            .player_mgr
            .with_player(unit, |p| p.auras.container.has_aura_type(SPELL_AURA_MOD_ROOT))
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(unit, |c| c.auras.has_aura_type(SPELL_AURA_MOD_ROOT))
            .unwrap_or(false)
    }
}

/// Check an event's own conditions, set its repeat timer and run its actions.
/// Returns true when it fired.
fn process_event(world: &World, guid: ObjectGuid, holder: &mut Holder, phase: u32, invoker: Option<ObjectGuid>) -> bool {
    if !holder.enabled || holder.time != 0 {
        return false;
    }
    let event = holder.event.clone();
    if event.inverse_phase_mask & (1u32.checked_shl(phase).unwrap_or(0)) != 0 {
        return false;
    }
    let Some(view) = view(world, guid) else {
        return false;
    };
    let mut invoker = invoker;
    if event.condition_id != 0 {
        let context = crate::game::conditions::ConditionContext {
            target: invoker.or(view.me.victim).unwrap_or_default(),
            source: guid,
        };
        if !world
            .managers
            .condition_mgr
            .is_satisfied(event.condition_id, context, world)
        {
            return false;
        }
    }
    let p = event.params;
    let in_combat = view.me.in_combat;
    let percent = |current: u32, max: u32| if max == 0 { None } else { Some(current * 100 / max) };
    let in_range = |value: u32| value <= p[0] && value >= p[1];

    match event.event_type {
        EVENT_T_TIMER_IN_COMBAT => {
            if !in_combat {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_TIMER_OOC => {
            if in_combat || view.me.evading {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_HP => {
            let Some(hp) = percent(
                (view.me.health_pct * 100.0).round() as u32,
                100,
            ) else {
                return false;
            };
            if !in_combat || !in_range(hp) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_MANA => {
            let Some(mana) = percent(view.current_mana, view.max_mana) else {
                return false;
            };
            if !in_combat || !in_range(mana) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_AGGRO | EVENT_T_DEATH | EVENT_T_EVADE | EVENT_T_LEAVE_COMBAT | EVENT_T_SPAWNED
        | EVENT_T_REACHED_HOME | EVENT_T_RECEIVE_EMOTE | EVENT_T_SCRIPT | EVENT_T_GROUP_MEMBER_DIED => {}
        EVENT_T_KILL => {
            if p[2] != 0 && !invoker.is_some_and(|g| g.is_player()) {
                return false;
            }
            holder.update_repeat_timer(p[0], p[1]);
        }
        EVENT_T_HIT_BY_SPELL | EVENT_T_SPELL_HIT_TARGET | EVENT_T_RANGE | EVENT_T_OOC_LOS
        | EVENT_T_MOVEMENT_INFORM | EVENT_T_HIT_BY_AURA => {
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_STEALTH_ALERT | EVENT_T_TARGET_CASTING | EVENT_T_VICTIM_ROOTED => {
            match event.event_type {
                EVENT_T_TARGET_CASTING => {
                    if !in_combat || !view.victim.as_ref().is_some_and(|v| is_casting(world, v.guid)) {
                        return false;
                    }
                }
                EVENT_T_VICTIM_ROOTED => {
                    if !view.victim.as_ref().is_some_and(|v| is_rooted(world, v.guid)) {
                        return false;
                    }
                }
                _ => {}
            }
            holder.update_repeat_timer(p[0], p[1]);
        }
        EVENT_T_TARGET_HP => {
            let Some(victim) = view.victim.as_ref() else {
                return false;
            };
            let hp = (victim.health_pct * 100.0).round() as u32;
            if !in_combat || !in_range(hp) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_TARGET_MANA => {
            let Some(mana) = view.victim.as_ref().and_then(|v| mana_percent(world, v)) else {
                return false;
            };
            if !in_combat || !in_range(mana) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_FRIENDLY_HP => {
            if !in_combat {
                return false;
            }
            let found = friends(world, &view.me, p[1] as f32)
                .into_iter()
                .filter_map(|f| {
                    let max = max_health(world, &f);
                    let missing = max.saturating_sub((f.health_pct * max as f32) as u32);
                    (missing >= p[0]).then_some((f.guid, missing))
                })
                .max_by_key(|(_, missing)| *missing);
            let Some((friend, _)) = found else {
                return false;
            };
            invoker = Some(friend);
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_FRIENDLY_IS_CC => {
            if !in_combat {
                return false;
            }
            const CC_FLAGS: u32 = crate::game::common::unit_flags::STUNNED
                | crate::game::common::unit_flags::CONFUSED
                | crate::game::common::unit_flags::FLEEING;
            let Some(friend) = friends(world, &view.me, p[1] as f32)
                .into_iter()
                .find(|f| f.unit_flags & CC_FLAGS != 0)
            else {
                return false;
            };
            invoker = Some(friend.guid);
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_FRIENDLY_MISSING_BUFF => {
            let Some(friend) = friends(world, &view.me, p[1] as f32)
                .into_iter()
                .find(|f| aura_stacks(world, f.guid, p[0]) == 0)
            else {
                return false;
            };
            invoker = Some(friend.guid);
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_SUMMONED_UNIT | EVENT_T_SUMMONED_JUST_DIED | EVENT_T_SUMMONED_JUST_DESPAWN => {
            let Some(summoned) = invoker.filter(|g| g.is_creature()) else {
                return false;
            };
            if summoned.entry() != p[0] {
                return false;
            }
            holder.update_repeat_timer(p[1], p[2]);
        }
        EVENT_T_AURA => {
            if aura_stacks(world, guid, p[0]) < p[1].max(1) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_TARGET_AURA => {
            let Some(victim) = view.victim.as_ref().filter(|_| in_combat) else {
                return false;
            };
            if aura_stacks(world, victim.guid, p[0]) < p[1].max(1) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_MISSING_AURA => {
            if aura_stacks(world, guid, p[0]) >= p[1].max(1) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        EVENT_T_TARGET_MISSING_AURA => {
            let Some(victim) = view.victim.as_ref().filter(|_| in_combat) else {
                return false;
            };
            if aura_stacks(world, victim.guid, p[0]) >= p[1].max(1) {
                return false;
            }
            holder.update_repeat_timer(p[2], p[3]);
        }
        other => {
            tracing::debug!("EventAI event {} has unsupported type {}", event.id, other);
        }
    }

    if event.flags & EFLAG_REPEATABLE == 0 {
        holder.enabled = false;
    }

    let roll: u32 = rand::thread_rng().gen_range(0..100);
    if event.chance <= roll {
        return false;
    }

    let target = invoker.or(view.me.victim);
    let actions: Vec<u32> = event.actions.iter().copied().filter(|a| *a != 0).collect();
    let failed = if event.flags & EFLAG_RANDOM_ACTION == 0 {
        let mut failed = false;
        for action in &actions {
            if run_action(world, guid, *action, target) {
                failed = true;
            }
        }
        failed
    } else if actions.is_empty() {
        false
    } else {
        let pick = actions[rand::thread_rng().gen_range(0..actions.len())];
        run_action(world, guid, pick, target)
    };

    if failed && event.flags & EFLAG_CHECK_RESULT != 0 {
        holder.enabled = true;
        holder.time = 0;
        return false;
    }
    true
}

/// Run an action script now. Returns true when a step failed.
fn run_action(world: &World, guid: ObjectGuid, script_id: u32, target: Option<ObjectGuid>) -> bool {
    let Some(steps) = db_scripts::script(ScriptTable::CreatureAi, script_id) else {
        return false;
    };
    steps
        .iter()
        .any(|step| db_scripts::run_step(world, step, Some(guid), target))
}

/// Run every enabled event of these types, one invoker for all.
fn fire(world: &World, guid: ObjectGuid, state: &mut State, types: &[u32], invoker: Option<ObjectGuid>, filter: impl Fn(&EventAiEvent) -> bool) {
    let phase = phase(guid).unwrap_or(0);
    for holder in state.holders.iter_mut() {
        if types.contains(&holder.event.event_type) && filter(&holder.event) {
            process_event(world, guid, holder, phase, invoker);
        }
    }
}

/// Re-arm the out-of-combat timers (spawn, return home, death).
fn reset(state: &mut State) {
    state.update_time = EVENT_UPDATE_TIME;
    state.event_diff = 0;
    for holder in state.holders.iter_mut() {
        if holder.event.event_type == EVENT_T_TIMER_OOC {
            let (min, max) = (holder.event.params[0], holder.event.params[1]);
            if holder.update_repeat_timer(min, max) {
                holder.enabled = true;
            }
        }
    }
}

/// Entering combat: aggro events fire, in-combat timers start on their initial
/// delays, everything else is re-armed.
fn enter_combat(world: &World, guid: ObjectGuid, state: &mut State, enemy: ObjectGuid) {
    let phase = phase(guid).unwrap_or(0);
    for holder in state.holders.iter_mut() {
        match holder.event.event_type {
            EVENT_T_AGGRO => {
                holder.enabled = true;
                process_event(world, guid, holder, phase, Some(enemy));
            }
            EVENT_T_TIMER_IN_COMBAT => {
                let (min, max) = (holder.event.params[0], holder.event.params[1]);
                if holder.update_repeat_timer(min, max) {
                    holder.enabled = true;
                }
            }
            _ => {
                holder.enabled = true;
                holder.time = 0;
            }
        }
    }
    state.update_time = EVENT_UPDATE_TIME;
    state.event_diff = 0;
}

fn new_state(entry: u32) -> Option<State> {
    let events = events_by_entry().get(&entry)?.clone();
    Some(State {
        holders: events
            .iter()
            .map(|e| Holder {
                event: e.clone(),
                time: 0,
                enabled: true,
            })
            .collect(),
        update_time: EVENT_UPDATE_TIME,
        event_diff: 0,
    })
}

/// Run a creature's EventAI for one update: react to the queued AI events and
/// check the timed events.
pub fn update(world: &World, guid: ObjectGuid, entry: u32, events: &[AIEvent], diff_ms: u32, has_victim: bool) {
    let mut state = match states().remove(&guid) {
        Some((_, state)) => state,
        None => {
            let Some(mut state) = new_state(entry) else {
                return;
            };
            phases().insert(guid, 0);
            fire(world, guid, &mut state, &[EVENT_T_SPAWNED], None, |_| true);
            reset(&mut state);
            state
        }
    };

    for event in events {
        match event {
            AIEvent::CombatStarted { initial_aggressor } => {
                enter_combat(world, guid, &mut state, *initial_aggressor)
            }
            AIEvent::Died { killer_guid } => {
                reset(&mut state);
                fire(world, guid, &mut state, &[EVENT_T_DEATH], *killer_guid, |_| true);
                phases().insert(guid, 0);
            }
            AIEvent::TargetKilled { victim_guid } if victim_guid.is_player() => {
                fire(world, guid, &mut state, &[EVENT_T_KILL], Some(*victim_guid), |_| true);
            }
            AIEvent::SpellHit {
                caster_guid,
                spell_id,
                ..
            } => {
                let Some(spell) = world.managers.spell_mgr.get(*spell_id) else {
                    continue;
                };
                let school_mask = 1u32 << spell.school.min(31);
                fire(world, guid, &mut state, &[EVENT_T_HIT_BY_SPELL], Some(*caster_guid), |e| {
                    (e.params[0] == 0 || e.params[0] == *spell_id) && school_mask & e.params[1] != 0
                });
                fire(world, guid, &mut state, &[EVENT_T_HIT_BY_AURA], Some(*caster_guid), |e| {
                    e.params[0] == 0 || spell.effect_apply_aura_name.contains(&e.params[0])
                });
            }
            AIEvent::SpellHitTarget { target_guid, spell_id } => {
                let Some(spell) = world.managers.spell_mgr.get(*spell_id) else {
                    continue;
                };
                let school_mask = 1u32 << spell.school.min(31);
                fire(world, guid, &mut state, &[EVENT_T_SPELL_HIT_TARGET], Some(*target_guid), |e| {
                    (e.params[0] == 0 || e.params[0] == *spell_id) && school_mask & e.params[1] != 0
                });
            }
            AIEvent::MovementComplete { motion_type, point_id } => {
                fire(world, guid, &mut state, &[EVENT_T_MOVEMENT_INFORM], None, |e| {
                    e.params[0] == *motion_type && e.params[1] == *point_id
                });
            }
            AIEvent::ReachedHome => {
                fire(world, guid, &mut state, &[EVENT_T_REACHED_HOME], None, |_| true);
                reset(&mut state);
            }
            AIEvent::CombatEnded {
                reason: CombatEndReason::Evaded | CombatEndReason::TargetsOutOfRange,
            } => {
                fire(world, guid, &mut state, &[EVENT_T_EVADE], None, |_| true);
                fire(world, guid, &mut state, &[EVENT_T_LEAVE_COMBAT], None, |_| true);
            }
            AIEvent::Respawned => {
                reset(&mut state);
                fire(world, guid, &mut state, &[EVENT_T_SPAWNED], None, |_| true);
            }
            AIEvent::SummonedCreature { summoned_guid, .. } => {
                fire(world, guid, &mut state, &[EVENT_T_SUMMONED_UNIT], Some(*summoned_guid), |_| true);
            }
            AIEvent::SummonedCreatureDied { summoned_guid } => {
                fire(world, guid, &mut state, &[EVENT_T_SUMMONED_JUST_DIED], Some(*summoned_guid), |_| true);
            }
            AIEvent::SummonedCreatureDespawned { summoned_guid } => {
                fire(world, guid, &mut state, &[EVENT_T_SUMMONED_JUST_DESPAWN], Some(*summoned_guid), |_| true);
            }
            AIEvent::GroupMemberDied {
                member_guid,
                member_entry,
                was_leader,
            } => {
                fire(world, guid, &mut state, &[EVENT_T_GROUP_MEMBER_DIED], Some(*member_guid), |e| {
                    (e.params[0] == 0 || e.params[0] == *member_entry) && (e.params[1] != 0) == *was_leader
                });
            }
            AIEvent::ScriptEvent {
                event_id,
                data,
                invoker,
            } => {
                fire(world, guid, &mut state, &[EVENT_T_SCRIPT], *invoker, |e| {
                    e.params[0] == *event_id && e.params[1] == *data
                });
            }
            AIEvent::UnitInLineOfSight { unit_guid, is_hostile } if !has_victim => {
                let distance = units::resolve_unit(world, *unit_guid).and_then(|u| {
                    units::resolve_unit(world, guid).map(|me| units::distance(&me.position, &u.position))
                });
                let Some(distance) = distance else {
                    continue;
                };
                fire(world, guid, &mut state, &[EVENT_T_OOC_LOS], Some(*unit_guid), |e| {
                    distance <= e.params[1] as f32
                        && match e.params[0] {
                            ULR_ANY => true,
                            ULR_HOSTILE => *is_hostile,
                            ULR_NON_HOSTILE => !*is_hostile,
                            _ => false,
                        }
                });
            }
            AIEvent::StealthAlert { unit_guid } => {
                fire(world, guid, &mut state, &[EVENT_T_STEALTH_ALERT], Some(*unit_guid), |_| true);
            }
            _ => {}
        }
    }

    update_timed(world, guid, &mut state, diff_ms, has_victim);
    states().insert(guid, state);
}

/// The half-second pass over timer and threshold events.
fn update_timed(world: &World, guid: ObjectGuid, state: &mut State, diff_ms: u32, combat: bool) {
    if state.update_time >= diff_ms {
        state.event_diff += diff_ms;
        state.update_time -= diff_ms;
        return;
    }
    state.event_diff += diff_ms;
    let elapsed = state.event_diff;
    let phase = phase(guid).unwrap_or(0);
    let victim_distance = || {
        let me = units::resolve_unit(world, guid)?;
        let victim = units::resolve_unit(world, me.victim?)?;
        (victim.map_id == me.map_id && victim.instance_id == me.instance_id)
            .then(|| units::distance(&me.position, &victim.position))
    };
    for holder in state.holders.iter_mut() {
        if holder.time != 0 {
            if holder.time > elapsed {
                if holder.event.inverse_phase_mask & (1u32.checked_shl(phase).unwrap_or(0)) == 0 {
                    holder.time -= elapsed;
                }
                continue;
            }
            holder.time = 0;
        }
        match holder.event.event_type {
            EVENT_T_TIMER_OOC | EVENT_T_FRIENDLY_MISSING_BUFF => {
                process_event(world, guid, holder, phase, None);
            }
            EVENT_T_TIMER_IN_COMBAT | EVENT_T_MANA | EVENT_T_HP | EVENT_T_TARGET_HP
            | EVENT_T_TARGET_CASTING | EVENT_T_FRIENDLY_HP | EVENT_T_FRIENDLY_IS_CC
            | EVENT_T_AURA | EVENT_T_TARGET_AURA | EVENT_T_MISSING_AURA
            | EVENT_T_TARGET_MISSING_AURA | EVENT_T_VICTIM_ROOTED => {
                if combat {
                    process_event(world, guid, holder, phase, None);
                }
            }
            EVENT_T_RANGE => {
                if combat {
                    let (min, max) = (holder.event.params[0] as f32, holder.event.params[1] as f32);
                    if victim_distance().is_some_and(|d| d >= min && d <= max) {
                        process_event(world, guid, holder, phase, None);
                    }
                }
            }
            _ => {}
        }
    }
    state.event_diff = 0;
    state.update_time = EVENT_UPDATE_TIME;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holder(event_type: u32, params: [u32; 4]) -> Holder {
        Holder {
            event: EventAiEvent {
                event_type,
                params,
                chance: 100,
                ..Default::default()
            },
            time: 0,
            enabled: true,
        }
    }

    #[test]
    fn repeat_timer_uses_the_range_or_disables_on_bad_ranges() {
        let mut h = holder(EVENT_T_TIMER_IN_COMBAT, [0; 4]);
        assert!(h.update_repeat_timer(500, 500));
        assert_eq!(h.time, 500);
        assert!(h.update_repeat_timer(100, 200));
        assert!((100..=200).contains(&h.time));
        assert!(!h.update_repeat_timer(200, 100));
        assert!(!h.enabled);
    }

    #[test]
    fn reset_arms_out_of_combat_timers_on_their_initial_delay() {
        let mut state = State {
            holders: vec![holder(EVENT_T_TIMER_OOC, [1000, 1000, 5000, 5000]), holder(EVENT_T_HP, [50, 0, 0, 0])],
            update_time: 0,
            event_diff: 0,
        };
        state.holders[0].enabled = false;
        reset(&mut state);
        assert!(state.holders[0].enabled);
        assert_eq!(state.holders[0].time, 1000);
        assert_eq!(state.holders[1].time, 0);
    }

    #[test]
    fn phase_must_fit_the_mask() {
        let guid = ObjectGuid::new_creature(1, 77_001);
        phases().insert(guid, 0);
        assert!(set_phase(guid, 3));
        assert_eq!(phase(guid), Some(3));
        assert!(!set_phase(guid, 32));
        forget(guid);
        assert!(!set_phase(guid, 1));
    }
}
