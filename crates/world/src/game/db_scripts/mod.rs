//! Database scripts: the step lists in `*_scripts` tables, run immediately by
//! EventAI or scheduled by delay (generic, creature spell and movement scripts).
//!
//! A step names a command, a condition, data fields and an optional "buddy"
//! target type. Its source and target come from whoever started the script,
//! optionally swapped, replaced by the buddy, or collapsed onto the source,
//! as the step's data flags say. A step that fails while flagged to abort ends
//! every remaining step of its script.

pub mod commands;

use crate::game::creature::ai::script_target::{self, Source};
use crate::World;
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::{Mutex, RwLock};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

/// Swap the starting source and target before the buddy is looked up.
pub const SF_GENERAL_SWAP_INITIAL_TARGETS: u32 = 0x01;
/// Swap the final source and target after the buddy is assigned.
pub const SF_GENERAL_SWAP_FINAL_TARGETS: u32 = 0x02;
/// The final target is the final source.
pub const SF_GENERAL_TARGET_SELF: u32 = 0x04;
/// A failed step ends the whole script.
pub const SF_GENERAL_ABORT_ON_FAILURE: u32 = 0x08;
/// A step whose source or target is missing is skipped quietly.
pub const SF_GENERAL_SKIP_MISSING_TARGETS: u32 = 0x10;

/// The command number of a disabled step.
const SCRIPT_COMMAND_CAST_SPELL: u32 = 15;

/// One step of a database script.
#[derive(Debug, Clone, Default)]
pub struct ScriptInfo {
    pub id: u32,
    /// Seconds after the script starts.
    pub delay: u32,
    pub command: u32,
    pub condition: u32,
    pub datalong: [u32; 4],
    pub data_flags: u32,
    pub dataint: [i32; 4],
    pub target_param1: u32,
    pub target_param2: u32,
    pub target_type: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

impl ScriptInfo {
    /// The value a failed command returns: whether to abort the script.
    pub fn abort_on_failure(&self) -> bool {
        self.data_flags & SF_GENERAL_ABORT_ON_FAILURE != 0
    }
}

/// The script tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptTable {
    /// EventAI actions (creature_ai_scripts).
    CreatureAi,
    /// Scripts started by other scripts (generic_scripts).
    Generic,
    /// Scripts started by creature spell list casts (creature_spells_scripts).
    CreatureSpells,
    /// Scripts started when a creature reaches a waypoint (creature_movement_scripts).
    CreatureMovement,
    /// Scripts started by spell and game object events (event_scripts).
    Event,
    /// Scripts started by gossip greetings and options (gossip_scripts).
    Gossip,
}

impl ScriptTable {
    fn table_name(self) -> &'static str {
        match self {
            ScriptTable::CreatureAi => "creature_ai_scripts",
            ScriptTable::Generic => "generic_scripts",
            ScriptTable::CreatureSpells => "creature_spells_scripts",
            ScriptTable::CreatureMovement => "creature_movement_scripts",
            ScriptTable::Event => "event_scripts",
            ScriptTable::Gossip => "gossip_scripts",
        }
    }

    const ALL: [ScriptTable; 6] = [
        ScriptTable::CreatureAi,
        ScriptTable::Generic,
        ScriptTable::CreatureSpells,
        ScriptTable::CreatureMovement,
        ScriptTable::Event,
        ScriptTable::Gossip,
    ];
}

type Store = HashMap<ScriptTable, HashMap<u32, Arc<Vec<ScriptInfo>>>>;

fn store() -> &'static RwLock<Store> {
    static STORE: OnceLock<RwLock<Store>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// The steps of script `id` in `table`, in delay order.
pub fn script(table: ScriptTable, id: u32) -> Option<Arc<Vec<ScriptInfo>>> {
    store().read().get(&table)?.get(&id).cloned()
}

/// Register a script (tests, scripted content).
pub fn insert_script(table: ScriptTable, id: u32, mut steps: Vec<ScriptInfo>) {
    steps.sort_by_key(|s| s.delay);
    store()
        .write()
        .entry(table)
        .or_default()
        .insert(id, Arc::new(steps));
}

/// Load every script table.
pub async fn load(pool: &sqlx::PgPool) -> anyhow::Result<usize> {
    let mut total = 0;
    for table in ScriptTable::ALL {
        let query = format!(
            "SELECT id::BIGINT AS id, delay::BIGINT AS delay, priority::BIGINT AS priority, \
             command::BIGINT AS command, datalong::BIGINT AS datalong, datalong2::BIGINT AS datalong2, \
             datalong3::BIGINT AS datalong3, datalong4::BIGINT AS datalong4, \
             target_param1::BIGINT AS target_param1, target_param2::BIGINT AS target_param2, \
             target_type::BIGINT AS target_type, data_flags::BIGINT AS data_flags, \
             dataint::BIGINT AS dataint, dataint2::BIGINT AS dataint2, dataint3::BIGINT AS dataint3, \
             dataint4::BIGINT AS dataint4, x::REAL AS x, y::REAL AS y, z::REAL AS z, o::REAL AS o, \
             condition_id::BIGINT AS condition_id FROM world.{}",
            table.table_name()
        );
        let rows = match sqlx::query(&query).fetch_all(pool).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!("Skipping {}: {}", table.table_name(), e);
                continue;
            }
        };
        let mut scripts: HashMap<u32, Vec<(i64, ScriptInfo)>> = HashMap::new();
        for row in &rows {
            let get = |name: &str| row.try_get::<i64, _>(name).unwrap_or(0);
            let info = ScriptInfo {
                id: get("id") as u32,
                delay: get("delay") as u32,
                command: get("command") as u32,
                condition: get("condition_id") as u32,
                datalong: [
                    get("datalong") as u32,
                    get("datalong2") as u32,
                    get("datalong3") as u32,
                    get("datalong4") as u32,
                ],
                data_flags: get("data_flags") as u32,
                dataint: [
                    get("dataint") as i32,
                    get("dataint2") as i32,
                    get("dataint3") as i32,
                    get("dataint4") as i32,
                ],
                target_param1: get("target_param1") as u32,
                target_param2: get("target_param2") as u32,
                target_type: get("target_type") as u8,
                x: row.try_get::<f32, _>("x").unwrap_or(0.0),
                y: row.try_get::<f32, _>("y").unwrap_or(0.0),
                z: row.try_get::<f32, _>("z").unwrap_or(0.0),
                o: row.try_get::<f32, _>("o").unwrap_or(0.0),
            };
            scripts
                .entry(info.id)
                .or_default()
                .push((get("priority"), info));
        }
        total += rows.len();
        let mut loaded = HashMap::new();
        for (id, mut steps) in scripts {
            steps.sort_by_key(|(priority, s)| (s.delay, *priority));
            loaded.insert(id, Arc::new(steps.into_iter().map(|(_, s)| s).collect()));
        }
        store().write().insert(table, loaded);
    }
    Ok(total)
}

/// A step waiting for its time.
#[derive(Debug, Clone)]
struct Scheduled {
    due_ms: u64,
    seq: u64,
    table: ScriptTable,
    id: u32,
    step: usize,
    source: Option<ObjectGuid>,
    target: Option<ObjectGuid>,
}

fn queue() -> &'static Mutex<Vec<Scheduled>> {
    static QUEUE: OnceLock<Mutex<Vec<Scheduled>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// Schedule every step of a script, each at its delay from now.
pub fn start(world: &World, table: ScriptTable, id: u32, source: Option<ObjectGuid>, target: Option<ObjectGuid>) {
    let _ = world;
    let Some(steps) = script(table, id) else {
        tracing::debug!("No {} script {}", table.table_name(), id);
        return;
    };
    let now = crate::game::creature::ai::units::now_ms();
    let mut queue = queue().lock();
    for (step, info) in steps.iter().enumerate() {
        queue.push(Scheduled {
            due_ms: now + u64::from(info.delay) * 1000,
            seq: SEQ.fetch_add(1, Ordering::Relaxed),
            table,
            id,
            step,
            source,
            target,
        });
    }
}

/// Whether a step of the script is waiting to run for this source and target.
#[cfg(test)]
pub fn is_scheduled(
    table: ScriptTable,
    id: u32,
    source: Option<ObjectGuid>,
    target: Option<ObjectGuid>,
) -> bool {
    queue()
        .lock()
        .iter()
        .any(|s| s.table == table && s.id == id && s.source == source && s.target == target)
}

/// Run the steps whose time has come.
pub fn process(world: &World) {
    let now = crate::game::creature::ai::units::now_ms();
    loop {
        let next = {
            let mut queue = queue().lock();
            let Some(index) = queue
                .iter()
                .enumerate()
                .filter(|(_, s)| s.due_ms <= now)
                .min_by_key(|(_, s)| (s.due_ms, s.seq))
                .map(|(i, _)| i)
            else {
                return;
            };
            queue.swap_remove(index)
        };
        let Some(steps) = script(next.table, next.id) else {
            continue;
        };
        let Some(info) = steps.get(next.step) else {
            continue;
        };
        let source = next.source.filter(|g| object_in_world(world, *g));
        let target = next.target.filter(|g| object_in_world(world, *g));
        if info.data_flags & SF_GENERAL_SKIP_MISSING_TARGETS != 0
            && ((source.is_none() && next.source.is_some()) || (target.is_none() && next.target.is_some()))
        {
            continue;
        }
        if run_step(world, info, source, target) {
            terminate(next.id, next.source, next.target);
        }
    }
}

/// Drop every remaining step of a script started for this source and target.
fn terminate(id: u32, source: Option<ObjectGuid>, target: Option<ObjectGuid>) {
    queue()
        .lock()
        .retain(|s| !(s.id == id && (source.is_none() || s.source == source) && (target.is_none() || s.target == target)));
}

/// Run one step now. Returns true when the script should stop.
pub fn run_step(world: &World, info: &ScriptInfo, source: Option<ObjectGuid>, target: Option<ObjectGuid>) -> bool {
    let Some((source, target)) = final_targets(world, info, source, target) else {
        return info.abort_on_failure();
    };
    if info.condition != 0 {
        let context = crate::game::conditions::ConditionContext {
            target: target.unwrap_or_default(),
            source: source.unwrap_or_default(),
        };
        if !world.managers.condition_mgr.is_satisfied(info.condition, context, world) {
            return info.abort_on_failure();
        }
    }
    commands::execute(world, info, source, target)
}

/// Apply the step's target rules: swap, buddy lookup, swap again, target self.
fn final_targets(
    world: &World,
    info: &ScriptInfo,
    mut source: Option<ObjectGuid>,
    mut target: Option<ObjectGuid>,
) -> Option<(Option<ObjectGuid>, Option<ObjectGuid>)> {
    if info.data_flags & SF_GENERAL_SWAP_INITIAL_TARGETS != 0 {
        std::mem::swap(&mut source, &mut target);
    }
    if info.target_type != 0 {
        let searcher = source.or(target)?;
        let spell = (info.command == SCRIPT_COMMAND_CAST_SPELL)
            .then(|| world.managers.spell_mgr.get(info.datalong[0]))
            .flatten();
        let origin = source_of(world, searcher)?;
        let found = script_target::get_target_by_type(
            world,
            &origin,
            target,
            info.target_type,
            info.target_param1,
            info.target_param2,
            spell.as_deref(),
        );
        match found {
            Some(found) => target = Some(found),
            None => {
                if info.data_flags & SF_GENERAL_SKIP_MISSING_TARGETS == 0 {
                    tracing::debug!(
                        "Script {} found no target (type {}, params {} {})",
                        info.id,
                        info.target_type,
                        info.target_param1,
                        info.target_param2
                    );
                }
                return None;
            }
        }
    }
    if info.data_flags & SF_GENERAL_SWAP_FINAL_TARGETS != 0 {
        std::mem::swap(&mut source, &mut target);
    }
    if info.data_flags & SF_GENERAL_TARGET_SELF != 0 {
        target = source;
    }
    Some((source, target))
}

/// A script source for target lookups: creatures with their threat list,
/// players and game objects by position and faction.
pub fn source_of(world: &World, guid: ObjectGuid) -> Option<Source> {
    if guid.is_creature() {
        return Source::of_creature(world, guid);
    }
    if guid.is_game_object() {
        return world.managers.gameobject_mgr.with_gameobject(guid, |go| Source {
            guid,
            map_id: go.map_id,
            instance_id: go.instance_id,
            position: go.position,
            faction: go.faction,
            combat_reach: 0.0,
            victim: None,
            owner: None,
            threat: Vec::new(),
        });
    }
    let unit = crate::game::creature::ai::units::resolve_unit(world, guid)?;
    Some(Source {
        guid,
        map_id: unit.map_id,
        instance_id: unit.instance_id,
        position: unit.position,
        faction: unit.faction,
        combat_reach: unit.combat_reach,
        victim: unit.victim,
        owner: None,
        threat: Vec::new(),
    })
}

/// Whether the object still exists in the world.
pub fn object_in_world(world: &World, guid: ObjectGuid) -> bool {
    if guid.is_player() {
        world.managers.player_mgr.with_player(guid, |_| ()).is_some()
    } else if guid.is_game_object() {
        world
            .managers
            .gameobject_mgr
            .with_gameobject(guid, |go| go.in_world)
            .unwrap_or(false)
    } else {
        world
            .managers
            .creature_mgr
            .with_creature(guid, |c| c.in_world)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_are_kept_in_delay_order() {
        insert_script(
            ScriptTable::Generic,
            900_001,
            vec![
                ScriptInfo { id: 900_001, delay: 5, command: 0, ..Default::default() },
                ScriptInfo { id: 900_001, delay: 0, command: 1, ..Default::default() },
            ],
        );
        let steps = script(ScriptTable::Generic, 900_001).unwrap();
        assert_eq!(steps[0].command, 1);
        assert_eq!(steps[1].command, 0);
    }

    #[test]
    fn terminate_drops_only_the_matching_script() {
        let a = ObjectGuid::new_creature(1, 1);
        let b = ObjectGuid::new_creature(1, 2);
        {
            let mut q = queue().lock();
            for (id, source) in [(900_100, Some(a)), (900_100, Some(b)), (900_101, Some(a))] {
                q.push(Scheduled {
                    due_ms: u64::MAX,
                    seq: 0,
                    table: ScriptTable::Generic,
                    id,
                    step: 0,
                    source,
                    target: None,
                });
            }
        }
        terminate(900_100, Some(a), None);
        let q = queue().lock();
        assert!(!q.iter().any(|s| s.id == 900_100 && s.source == Some(a)));
        assert!(q.iter().any(|s| s.id == 900_100 && s.source == Some(b)));
        assert!(q.iter().any(|s| s.id == 900_101));
    }
}
