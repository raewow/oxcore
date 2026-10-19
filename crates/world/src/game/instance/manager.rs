//! Instance manager: dungeon and raid copies, the saves that bind players and
//! groups to them, the rules for entering one, and when they reset.
//!
//! Lifecycle summary:
//! - A normal dungeon copy expires two hours after creation unless kills push
//!   its reset further out (latest respawn + two hours). The reset only runs
//!   while the copy is empty; entering it cancels the pending reset.
//! - A raid copy never expires on its own. Every raid map resets globally on a
//!   fixed schedule (`map_template.reset_delay` days, at a configured hour),
//!   with warnings to players inside one hour, 15, 5 and 1 minute before.
//! - Entering creates temporary saves only. A raid save becomes permanent when
//!   a raid-locking creature dies; a permanently saved copy cannot be reset by
//!   players.

use anyhow::{Context, Result};
use oxcore_db::database::Databases;
use oxcore_shared::game::instance::{
    InstanceBinding, InstanceResetFailReason, InstanceResetMethod, InstanceState,
};
use oxcore_shared::protocol::ObjectGuid;
use parking_lot::{Mutex, RwLock};
use sqlx::{PgConnection, Row};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const PLAYER_BINDINGS_QUERY: &str = "SELECT i.map, b.instance, b.permanent, i.reset_time
     FROM characters.character_instance b JOIN characters.instance i ON i.id = b.instance
     WHERE b.guid = $1";
const GROUP_BINDINGS_QUERY: &str = "SELECT i.map, b.instance, b.permanent, i.reset_time
     FROM characters.group_instance b JOIN characters.instance i ON i.id = b.instance
     WHERE b.leader_guid = $1";

/// Distinct copies an account may enter within the window below.
const INSTANCE_ENTRIES_PER_HOUR: usize = 5;
/// An entry stops counting against the limit after one hour.
const INSTANCE_ENTRY_WINDOW_SECS: u64 = 3600;
/// A fresh normal dungeon in which nothing was killed resets this long after creation.
const DUNGEON_DEFAULT_RESET_SECS: u64 = 2 * 3600;
/// Kills keep a normal dungeon alive until this long after the latest respawn.
const DUNGEON_RESET_AFTER_RESPAWN_SECS: u64 = 2 * 3600;
/// Seconds before a global raid reset at which players inside are warned.
pub const RAID_RESET_WARNINGS: [u64; 4] = [3600, 900, 300, 60];
/// Default hour of day (UTC) at which raid resets happen.
pub const DEFAULT_RESET_HOUR: u64 = 4;
const DAY: u64 = 86_400;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Group context captured when a player starts entering a dungeon.
pub struct InstanceEntryGroup {
    pub leader_guid: ObjectGuid,
    /// Other online members outside the destination map. Their temporary solo
    /// saves are released when the group's first save for this map is created.
    pub members_outside: Vec<ObjectGuid>,
    /// The group is in raid mode, which raid maps require.
    pub is_raid: bool,
}

/// Why an instance entry was denied; each maps to a transfer-abort reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceAdmissionDenied {
    /// Raid map entered without a raid group.
    RaidGroupRequired,
    /// Too many distinct instances entered in the last hour.
    TooManyInstances,
    /// The copy is at its player cap.
    MaxPlayers,
    /// The copy's save was reset and the copy is being emptied.
    ResetInProgress,
    /// A raid encounter is engaged with the player's group inside.
    ZoneInCombat,
}

/// Live-world inputs the admission rules need but the instance manager cannot see.
pub struct InstanceAdmission<'a> {
    /// Destination map is a raid.
    pub is_raid: bool,
    /// The entering player's group is in raid mode.
    pub is_raid_group: bool,
    /// Destination map's player cap (`map_template.player_limit`); 0 is unlimited.
    pub max_players: u32,
    /// Players currently inside a given instance, excluding game masters.
    /// `None` treats every copy as empty.
    pub players_inside: Option<&'a (dyn Fn(u32, u32) -> usize + Send + Sync)>,
    /// Account the hourly entry limit is counted against; `None` counts per player.
    pub account_id: Option<u32>,
    /// Game masters bypass the raid-group, hourly and player-cap rules.
    pub is_game_master: bool,
    /// Whether a raid encounter in a given copy is engaged with the entering
    /// player's group in combat inside (and the player alive). `None` never refuses.
    pub encounter_in_progress: Option<&'a (dyn Fn(u32, u32) -> bool + Send + Sync)>,
}

impl InstanceAdmission<'_> {
    /// Admission with no live-world restrictions, for callers that only need
    /// selection/binding (tests and non-map callers).
    pub fn open() -> Self {
        Self {
            is_raid: false,
            is_raid_group: false,
            max_players: 0,
            players_inside: None,
            account_id: None,
            is_game_master: false,
            encounter_in_progress: None,
        }
    }

    fn players_inside(&self, map_id: u32, instance_id: u32) -> usize {
        self.players_inside
            .map(|count| count(map_id, instance_id))
            .unwrap_or(0)
    }
}

/// Failure to select, admit or bind a player for instance entry.
#[derive(Debug)]
pub enum InstanceEntryError {
    /// Entry is refused by an admission rule; the client gets the matching abort.
    Denied(InstanceAdmissionDenied),
    /// Infrastructure failure (database, cache). Entry must not proceed.
    Internal(anyhow::Error),
}

impl std::fmt::Display for InstanceEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied(reason) => write!(f, "instance entry denied: {reason:?}"),
            Self::Internal(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for InstanceEntryError {}

impl From<anyhow::Error> for InstanceEntryError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl From<InstanceAdmissionDenied> for InstanceEntryError {
    fn from(denied: InstanceAdmissionDenied) -> Self {
        Self::Denied(denied)
    }
}

/// Change in a player's removal timer (see [`InstanceMgr::track_unsaved_player`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedTimer {
    /// The countdown started: warn the player.
    Started,
    /// The countdown ran out: send the player home.
    Expired,
    /// The player is saved again: cancel the warning.
    Cleared,
}

/// Which saves a reset request covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetScope {
    /// The "reset all instances" request: normal dungeons only, with replies.
    All,
    /// A player joining a group drops their temporary solo saves.
    GroupJoin,
    /// A disbanding group drops all of its saves.
    GroupDisband,
}

/// Result of a player or group reset request.
#[derive(Debug, Default)]
pub struct ResetOutcome {
    /// Maps whose copy was reset (reported to the requester for `ResetScope::All`).
    pub reset_maps: Vec<u32>,
    /// Maps that could not be reset, with the reason to report.
    pub failed: Vec<(u32, InstanceResetFailReason)>,
    /// Copies whose save was deleted; their maps must be torn down.
    pub reset_copies: Vec<(u32, u32)>,
}

/// Work produced by a scheduler tick for the world to carry out.
#[derive(Debug, Default)]
pub struct InstanceUpdate {
    /// `(map, seconds left)`: warn players inside copies of this raid map.
    pub warnings: Vec<(u32, u64)>,
    /// Copies whose save expired and was deleted; tear them down.
    pub reset_copies: Vec<(u32, u32)>,
    /// Raid maps reset globally: send everyone inside home and tear all copies down.
    pub reset_maps: Vec<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EntryOwner {
    Player(ObjectGuid),
    Group(ObjectGuid),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EnterOwner {
    Player(ObjectGuid),
    Account(u32),
}

/// Instance manager - handles all instance operations
pub struct InstanceMgr {
    /// Active instances by (map_id, instance_id)
    instances: Arc<RwLock<HashMap<(u32, u32), Arc<RwLock<InstanceState>>>>>,
    /// Player bindings (player_guid -> map_id -> binding)
    player_bindings: Arc<RwLock<HashMap<ObjectGuid, HashMap<u32, InstanceBinding>>>>,
    /// Players whose persisted personal saves have been read into `player_bindings`.
    loaded_players: RwLock<HashSet<ObjectGuid>>,
    /// Group bindings (leader_guid -> map_id -> binding), separate from personal saves.
    group_bindings: Arc<RwLock<HashMap<ObjectGuid, HashMap<u32, InstanceBinding>>>>,
    /// Highest allocated ID across all maps; zero is reserved for continents.
    last_instance_id: AtomicU32,
    /// Raid maps and their global reset period in days.
    raid_reset_periods: RwLock<HashMap<u32, u32>>,
    /// Next global reset (unix seconds) for each raid map.
    raid_reset_times: RwLock<HashMap<u32, u64>>,
    /// Smallest warning threshold already announced for each raid map's next reset.
    raid_warnings_sent: RwLock<HashMap<u32, u64>>,
    /// Hour of day (UTC) at which raid resets happen.
    reset_hour: AtomicU64,
    /// Empty normal-dungeon copies and when they reset unless re-entered.
    dungeon_reset_schedule: RwLock<HashMap<(u32, u32), u64>>,
    /// Copies whose save was deleted while the map still exists.
    resetting: RwLock<HashSet<(u32, u32)>>,
    /// Loaded copies with players inside, as last observed.
    occupied: Mutex<HashSet<(u32, u32)>>,
    /// When each loaded copy became empty.
    empty_since: Mutex<HashMap<(u32, u32), Instant>>,
    /// Players inside a copy they are not saved to, and when they are sent home.
    removal_deadlines: Mutex<HashMap<ObjectGuid, Instant>>,
    /// Recent distinct instance entries (owner -> instance_id -> enter time).
    instance_enter_times: Arc<RwLock<HashMap<EnterOwner, HashMap<u32, u64>>>>,
    /// Serialize selection and binding so simultaneous party arrivals share a copy.
    entry_locks: Mutex<HashMap<EntryOwner, Weak<tokio::sync::Mutex<()>>>>,
    /// Encounter script data saved with each loaded copy (`id:value` tokens in
    /// `instance.data`, next to the completed-encounter ids).
    script_data: RwLock<HashMap<(u32, u32), HashMap<u32, u32>>>,
}

impl InstanceMgr {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            player_bindings: Arc::new(RwLock::new(HashMap::new())),
            loaded_players: RwLock::new(HashSet::new()),
            group_bindings: Arc::new(RwLock::new(HashMap::new())),
            last_instance_id: AtomicU32::new(0),
            raid_reset_periods: RwLock::new(HashMap::new()),
            raid_reset_times: RwLock::new(HashMap::new()),
            raid_warnings_sent: RwLock::new(HashMap::new()),
            reset_hour: AtomicU64::new(DEFAULT_RESET_HOUR),
            dungeon_reset_schedule: RwLock::new(HashMap::new()),
            resetting: RwLock::new(HashSet::new()),
            occupied: Mutex::new(HashSet::new()),
            empty_since: Mutex::new(HashMap::new()),
            removal_deadlines: Mutex::new(HashMap::new()),
            instance_enter_times: Arc::new(RwLock::new(HashMap::new())),
            entry_locks: Mutex::new(HashMap::new()),
            script_data: RwLock::new(HashMap::new()),
        }
    }

    /// Tell the manager which maps are raids and how often each resets (days),
    /// and the hour of day (UTC) resets happen at.
    pub fn configure_raids(&self, periods: HashMap<u32, u32>, reset_hour: u64) {
        *self.raid_reset_periods.write() = periods;
        self.reset_hour.store(reset_hour.min(23), Ordering::Relaxed);
    }

    /// Whether a map resets on the global raid schedule.
    pub fn is_raid_map(&self, map_id: u32) -> bool {
        self.raid_reset_periods.read().contains_key(&map_id)
    }

    /// Initialize the global instance ID counter from persisted saves before entry.
    pub async fn initialize(&self, databases: &Databases) -> Result<()> {
        let max_id: Option<i64> = sqlx::query_scalar("SELECT MAX(id) FROM characters.instance")
            .fetch_one(&databases.character)
            .await
            .context("Failed to query maximum instance ID")?;
        let max_id = u32::try_from(max_id.unwrap_or(0))
            .context("Persisted instance ID is outside the supported u32 range")?;

        // Reinitialization must not reuse IDs already reserved by pending inserts.
        self.last_instance_id.fetch_max(max_id, Ordering::Relaxed);

        Ok(())
    }

    /// Startup housekeeping, run once after `initialize` and `configure_raids`:
    /// expire saves whose reset passed while the server was down, drop saves
    /// nobody is bound to, establish the global raid schedule, and schedule the
    /// reset of every surviving (necessarily empty) normal dungeon copy.
    pub async fn startup(&self, databases: &Databases) -> Result<()> {
        let pool = &databases.character;
        let now = now_secs();
        let raid_maps: Vec<i64> = self
            .raid_reset_periods
            .read()
            .keys()
            .map(|&map| i64::from(map))
            .collect();

        // Raid saves carry no individual reset time; older rows stored a sentinel.
        sqlx::query("UPDATE characters.instance SET reset_time = 0 WHERE reset_time < 0 OR (map = ANY($1) AND reset_time <> 0)")
            .bind(&raid_maps)
            .execute(pool)
            .await
            .context("Failed to normalize raid reset times")?;

        // Global raid schedule, applying any reset that came due while offline.
        let stored: Vec<(i64, i64)> =
            sqlx::query_as("SELECT map, reset_time FROM characters.instance_reset")
                .fetch_all(pool)
                .await
                .context("Failed to load raid reset times")?;
        let stored: HashMap<u32, u64> = stored
            .into_iter()
            .filter_map(|(map, time)| Some((u32::try_from(map).ok()?, u64::try_from(time).ok()?)))
            .collect();
        let periods = self.raid_reset_periods.read().clone();
        let hour = self.reset_hour.load(Ordering::Relaxed) * 3600;
        let today = now / DAY * DAY;
        for (&map_id, &days) in &periods {
            let period = u64::from(days.max(1)) * DAY;
            let mut next = stored
                .get(&map_id)
                .map(|&t| t / DAY * DAY + hour)
                .unwrap_or(today + period + hour);
            if next <= now {
                self.delete_raid_map_saves(databases, map_id).await?;
                let base = next / DAY * DAY;
                next = base + ((today.saturating_sub(base)) / period + 1) * period + hour;
            }
            self.store_raid_reset_time(databases, map_id, next).await?;
        }

        // Normal dungeon saves that expired while offline.
        let expired: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM characters.instance WHERE reset_time > 0 AND reset_time <= $1",
        )
        .bind(now as i64)
        .fetch_all(pool)
        .await
        .context("Failed to find expired instances")?;
        for id in expired {
            Self::delete_instance_rows(pool, u32::try_from(id)?).await?;
        }

        // Saves nobody is bound to anymore, and respawn rows of vanished copies.
        sqlx::query(
            "DELETE FROM characters.instance i WHERE NOT EXISTS
                 (SELECT 1 FROM characters.character_instance b WHERE b.instance = i.id)
             AND NOT EXISTS
                 (SELECT 1 FROM characters.group_instance g WHERE g.instance = i.id)",
        )
        .execute(pool)
        .await
        .context("Failed to delete unbound instances")?;
        for table in ["creature_respawn", "gameobject_respawn"] {
            sqlx::query(&format!(
                "DELETE FROM characters.{table} r WHERE r.instance > 0
                 AND NOT EXISTS (SELECT 1 FROM characters.instance i WHERE i.id = r.instance)"
            ))
            .execute(pool)
            .await
            .with_context(|| format!("Failed to clean {table}"))?;
        }

        // Every surviving normal dungeon copy is empty now; arm its reset.
        let dungeons: Vec<(i64, i64, i64)> = sqlx::query_as(
            "SELECT id, map, reset_time FROM characters.instance WHERE reset_time > 0",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load dungeon reset times")?;
        let mut schedule = self.dungeon_reset_schedule.write();
        for (id, map, reset_time) in dungeons {
            schedule.insert(
                (u32::try_from(map)?, u32::try_from(id)?),
                u64::try_from(reset_time)?,
            );
        }

        Ok(())
    }

    async fn store_raid_reset_time(
        &self,
        databases: &Databases,
        map_id: u32,
        reset_time: u64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO characters.instance_reset (map, reset_time) VALUES ($1, $2)
             ON CONFLICT (map) DO UPDATE SET reset_time = EXCLUDED.reset_time",
        )
        .bind(i64::from(map_id))
        .bind(reset_time as i64)
        .execute(&databases.character)
        .await
        .context("Failed to store raid reset time")?;
        self.raid_reset_times.write().insert(map_id, reset_time);
        self.raid_warnings_sent.write().remove(&map_id);
        Ok(())
    }

    /// Next global reset (unix seconds) of a raid map.
    pub fn raid_reset_time(&self, map_id: u32) -> Option<u64> {
        self.raid_reset_times.read().get(&map_id).copied()
    }

    /// Create a new instance. Raid copies take their reset from the map's global
    /// schedule; normal dungeons get their own expiry, armed until someone enters.
    /// Take a fresh instance id without a save (battleground copies, which never persist).
    pub fn reserve_instance_id(&self) -> Option<u32> {
        self.last_instance_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(|id| id + 1)
            .ok()
    }

    pub async fn create_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        difficulty: u8,
        is_raid: bool,
    ) -> Result<u32> {
        // The database primary key is the instance ID alone, regardless of map.
        // Reserve it before awaiting the insert so concurrent entries stay unique.
        let instance_id = self
            .last_instance_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(|id| id + 1)
            .map_err(|_| anyhow::anyhow!("Instance ID space exhausted"))?;

        let now = now_secs();
        let reset_time = if is_raid {
            0
        } else {
            now + DUNGEON_DEFAULT_RESET_SECS
        };

        let instance = InstanceState {
            map_id,
            instance_id,
            difficulty,
            permanent: false,
            reset_time,
            created_time: now,
            completed_encounters: Vec::new(),
        };

        // Save to database (note: difficulty column doesn't exist in vanilla schema)
        sqlx::query::<sqlx::Postgres>(r#"INSERT INTO characters.instance (id, map, reset_time, data) VALUES ($1, $2, $3, '')"#)
            .bind(i64::from(instance_id))
            .bind(i64::from(map_id))
            .bind(reset_time as i64)
            .execute(&databases.character)
            .await
            .context("Failed to create instance")?;

        self.instances
            .write()
            .insert((map_id, instance_id), Arc::new(RwLock::new(instance)));
        if !is_raid {
            // Cancelled as soon as the first player arrives.
            self.dungeon_reset_schedule
                .write()
                .insert((map_id, instance_id), reset_time);
        }

        Ok(instance_id)
    }

    /// Load instance from database
    pub async fn load_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<Option<Arc<RwLock<InstanceState>>>> {
        if let Some(instance) = self.instances.read().get(&(map_id, instance_id)) {
            return Ok(Some(instance.clone()));
        }

        // Load from database (including data field for encounter state)
        let row = sqlx::query::<sqlx::Postgres>(
            r#"SELECT id, map, reset_time, data,
                      EXISTS (SELECT 1 FROM characters.character_instance b WHERE b.instance = i.id AND b.permanent <> 0)
                   OR EXISTS (SELECT 1 FROM characters.group_instance g WHERE g.instance = i.id AND g.permanent <> 0)
               FROM characters.instance i WHERE id = $1 AND map = $2"#,
        )
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .fetch_optional(&databases.character)
        .await
        .context("Failed to query instance")?;

        let Some(row) = row else {
            return Ok(None);
        };
        let reset_time: i64 = row.get(2);
        let data: Option<String> = row.get(3);
        let permanently_saved: bool = row.get(4);

        let data = data.unwrap_or_default();
        self.script_data
            .write()
            .insert((map_id, instance_id), Self::parse_script_data(&data));
        let instance = InstanceState {
            map_id,
            instance_id,
            difficulty: 0, // Difficulty not in vanilla schema
            permanent: permanently_saved,
            reset_time: u64::try_from(reset_time).unwrap_or(0),
            created_time: now_secs(),
            completed_encounters: Self::parse_instance_data(&data),
        };

        let instance = Arc::new(RwLock::new(instance));
        self.instances
            .write()
            .entry((map_id, instance_id))
            .or_insert_with(|| instance.clone());
        Ok(Some(instance))
    }

    /// Whether a copy's save is known (cached) right now.
    pub fn has_instance(&self, map_id: u32, instance_id: u32) -> bool {
        self.instances.read().contains_key(&(map_id, instance_id))
    }

    /// Parse instance data string to extract completed encounters
    /// Format: comma-separated encounter IDs (e.g., "1,5,10")
    fn parse_instance_data(data: &str) -> Vec<u32> {
        if data.is_empty() {
            return Vec::new();
        }

        data.split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect()
    }

    /// Script data tokens (`id:value`) from a saved data string.
    fn parse_script_data(data: &str) -> HashMap<u32, u32> {
        data.split(',')
            .filter_map(|token| {
                let (id, value) = token.trim().split_once(':')?;
                Some((id.parse().ok()?, value.parse().ok()?))
            })
            .collect()
    }

    /// Serialize completed encounters and script data to the saved data string:
    /// comma-separated encounter ids followed by `id:value` script tokens.
    fn serialize_instance_data(
        completed_encounters: &[u32],
        script_data: Option<&HashMap<u32, u32>>,
    ) -> String {
        let mut tokens: Vec<String> = completed_encounters
            .iter()
            .map(|id| id.to_string())
            .collect();
        if let Some(script_data) = script_data {
            let mut pairs: Vec<_> = script_data.iter().collect();
            pairs.sort();
            tokens.extend(pairs.into_iter().map(|(id, value)| format!("{id}:{value}")));
        }
        tokens.join(",")
    }

    /// Script data saved with a copy, for restoring its encounter state.
    pub fn saved_script_data(&self, map_id: u32, instance_id: u32) -> HashMap<u32, u32> {
        self.script_data
            .read()
            .get(&(map_id, instance_id))
            .cloned()
            .unwrap_or_default()
    }

    /// Persist a copy's encounter script data.
    pub async fn save_script_data(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
        data: HashMap<u32, u32>,
    ) -> Result<()> {
        if !self.has_instance(map_id, instance_id) {
            return Ok(());
        }
        self.script_data
            .write()
            .insert((map_id, instance_id), data);
        self.save_instance_data(databases, map_id, instance_id).await
    }

    /// Save instance data to database
    pub async fn save_instance_data(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        let data = {
            let instances = self.instances.read();
            if let Some(instance) = instances.get(&(map_id, instance_id)) {
                let instance = instance.read();
                Self::serialize_instance_data(
                    &instance.completed_encounters,
                    self.script_data.read().get(&(map_id, instance_id)),
                )
            } else {
                return Ok(()); // Instance not in cache
            }
        };

        sqlx::query::<sqlx::Postgres>(
            r#"UPDATE characters.instance SET data = $1 WHERE id = $2 AND map = $3"#,
        )
        .bind(&data)
        .bind(i64::from(instance_id))
        .bind(i64::from(map_id))
        .execute(&databases.character)
        .await
        .context("Failed to save instance data")?;

        Ok(())
    }

    /// Mark an encounter as completed for an instance
    pub fn mark_encounter_completed(&self, map_id: u32, instance_id: u32, encounter_id: u32) {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let mut instance = instance.write();
            if !instance.completed_encounters.contains(&encounter_id) {
                instance.completed_encounters.push(encounter_id);
            }
        }
    }

    /// Check if an encounter is completed for an instance
    pub fn is_encounter_completed(&self, map_id: u32, instance_id: u32, encounter_id: u32) -> bool {
        let instances = self.instances.read();
        if let Some(instance) = instances.get(&(map_id, instance_id)) {
            let instance = instance.read();
            instance.completed_encounters.contains(&encounter_id)
        } else {
            false
        }
    }

    fn mark_permanently_saved(&self, map_id: u32, instance_id: u32) {
        if let Some(instance) = self.instances.read().get(&(map_id, instance_id)) {
            instance.write().permanent = true;
        }
    }

    /// Whether players may reset this copy: false once anyone holds a permanent save.
    pub fn can_reset(&self, map_id: u32, instance_id: u32) -> bool {
        self.instances
            .read()
            .get(&(map_id, instance_id))
            .map_or(true, |instance| !instance.read().permanent)
    }

    /// Bind player to instance
    pub async fn bind_player_to_instance(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        permanent: bool,
        reset_time: u64,
    ) -> Result<()> {
        // Save to database
        sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO characters.character_instance (guid, instance, permanent, extend) VALUES ($1, $2, $3, 0) ON CONFLICT (guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent, extend = EXCLUDED.extend"#
        )
        .bind(i64::from(player_guid.low()))
        .bind(i64::from(instance_id))
        .bind(if permanent { 1i16 } else { 0i16 })
        .execute(&databases.character)
        .await
        .context("Failed to bind player to instance")?;

        self.player_bindings
            .write()
            .entry(player_guid)
            .or_default()
            .insert(
                map_id,
                InstanceBinding {
                    map_id,
                    instance_id,
                    permanent,
                    reset_time,
                },
            );
        if permanent {
            self.mark_permanently_saved(map_id, instance_id);
        }

        Ok(())
    }

    /// Bind group to instance (uses leader GUID as per vanilla schema)
    pub async fn bind_group_to_instance(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        permanent: bool,
        reset_time: u64,
    ) -> Result<()> {
        sqlx::query::<sqlx::Postgres>(
            r#"INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1, $2, $3) ON CONFLICT (leader_guid, instance) DO UPDATE SET permanent = EXCLUDED.permanent"#,
        )
        .bind(i64::from(leader_guid.low()))
        .bind(i64::from(instance_id))
        .bind(if permanent { 1i16 } else { 0i16 })
        .execute(&databases.character)
        .await
        .context("Failed to bind group to instance")?;

        self.group_bindings
            .write()
            .entry(leader_guid)
            .or_default()
            .insert(
                map_id,
                InstanceBinding {
                    map_id,
                    instance_id,
                    permanent,
                    reset_time,
                },
            );
        if permanent {
            self.mark_permanently_saved(map_id, instance_id);
        }

        Ok(())
    }

    /// Get player's instance binding for a map
    pub fn get_player_binding(
        &self,
        player_guid: ObjectGuid,
        map_id: u32,
    ) -> Option<InstanceBinding> {
        let bindings = self.player_bindings.read();
        bindings.get(&player_guid)?.get(&map_id).cloned()
    }

    /// Get group's instance binding for a map (via leader GUID)
    pub fn get_group_binding(
        &self,
        leader_guid: ObjectGuid,
        map_id: u32,
    ) -> Option<InstanceBinding> {
        self.group_bindings
            .read()
            .get(&leader_guid)?
            .get(&map_id)
            .cloned()
    }

    /// Remove a player's save for one copy without touching anyone else's.
    pub async fn unbind_player_from_instance(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        sqlx::query("DELETE FROM characters.character_instance WHERE guid = $1 AND instance = $2")
            .bind(i64::from(player_guid.low()))
            .bind(i64::from(instance_id))
            .execute(&databases.character)
            .await
            .context("Failed to unbind player from instance")?;
        if let Some(bindings) = self.player_bindings.write().get_mut(&player_guid) {
            if bindings
                .get(&map_id)
                .is_some_and(|binding| binding.instance_id == instance_id)
            {
                bindings.remove(&map_id);
            }
        }
        Ok(())
    }

    /// Remove a group save without touching any member's personal binding.
    pub async fn unbind_group_from_instance(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM characters.group_instance WHERE leader_guid = $1 AND instance = $2",
        )
        .bind(i64::from(leader_guid.low()))
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind group from instance")?;
        if let Some(bindings) = self.group_bindings.write().get_mut(&leader_guid) {
            if bindings
                .get(&map_id)
                .is_some_and(|binding| binding.instance_id == instance_id)
            {
                bindings.remove(&map_id);
            }
        }
        Ok(())
    }

    /// Forget a disbanded group's cache after its database rows have been removed.
    pub fn unload_group_bindings(&self, leader_guid: ObjectGuid) {
        self.group_bindings.write().remove(&leader_guid);
    }

    /// Transfer a group's instance bindings from the old leader to the new one.
    ///
    /// Publish the new group cache only after the database transfer commits. Personal
    /// saves remain with their owners, except for the explicit handoff of a replaced
    /// group save and the conversion of the new leader's temporary saves.
    pub async fn transfer_group_bindings(
        &self,
        databases: &Databases,
        old_leader: ObjectGuid,
        new_leader: ObjectGuid,
    ) -> Result<()> {
        if old_leader == new_leader {
            return Ok(());
        }
        let old_low = i64::from(old_leader.low());
        let new_low = i64::from(new_leader.low());
        let mut transaction = databases.character.begin().await?;
        let group_id: i64 = sqlx::query_scalar(
            "SELECT group_id FROM characters.groups WHERE leader_guid = $1 FOR UPDATE",
        )
        .bind(old_low)
        .fetch_one(&mut *transaction)
        .await
        .context("Failed to find group for instance binding transfer")?;
        let old_group =
            Self::read_bindings(&mut transaction, GROUP_BINDINGS_QUERY, old_leader).await?;
        let mut old_personal =
            Self::read_bindings(&mut transaction, PLAYER_BINDINGS_QUERY, old_leader).await?;
        let new_personal =
            Self::read_bindings(&mut transaction, PLAYER_BINDINGS_QUERY, new_leader).await?;
        let mut new_group = HashMap::new();

        for (map_id, binding) in old_group {
            // The new leader supplies permanent group saves. Uncontested
            // temporary group saves follow the group to its new leader.
            if binding.permanent {
                continue;
            }
            if !new_personal.contains_key(&map_id) {
                new_group.insert(map_id, binding);
            } else if !old_personal.contains_key(&map_id) {
                // A personal save for this map replaces the group copy, even
                // when it refers to a different instance of the same map.
                sqlx::query(
                    "INSERT INTO characters.character_instance (guid, instance, permanent, extend) VALUES ($1, $2, 0, 0)",
                )
                .bind(old_low)
                .bind(i64::from(binding.instance_id))
                .execute(&mut *transaction)
                .await
                .context("Failed to hand group save to old leader")?;
                old_personal.insert(map_id, binding);
            }
        }
        new_group.extend(new_personal.clone());

        // group_instance references groups.leader_guid without ON UPDATE CASCADE.
        // Remove the old rows, change the leader, then restore the group's saves
        // in the same transaction so the foreign key is valid throughout.
        sqlx::query("DELETE FROM characters.group_instance WHERE leader_guid = $1")
            .bind(old_low)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE characters.groups SET leader_guid = $1 WHERE group_id = $2")
            .bind(new_low)
            .bind(group_id)
            .execute(&mut *transaction)
            .await
            .context("Failed to change instance binding owner")?;
        for binding in new_group.values() {
            sqlx::query(
                "INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1, $2, $3)",
            )
            .bind(new_low)
            .bind(i64::from(binding.instance_id))
            .bind(i16::from(binding.permanent))
            .execute(&mut *transaction)
            .await?;
        }
        // Permanent personal locks are kept; temporary personal saves become group-owned.
        sqlx::query("DELETE FROM characters.character_instance WHERE guid = $1 AND permanent = 0")
            .bind(new_low)
            .execute(&mut *transaction)
            .await?;
        transaction
            .commit()
            .await
            .context("Failed to commit group binding transfer")?;

        let mut groups = self.group_bindings.write();
        let mut players = self.player_bindings.write();
        let mut loaded = self.loaded_players.write();
        groups.remove(&old_leader);
        groups.insert(new_leader, new_group);
        players.insert(old_leader, old_personal);
        players.insert(
            new_leader,
            new_personal
                .into_iter()
                .filter(|(_, binding)| binding.permanent)
                .collect(),
        );
        loaded.insert(old_leader);
        loaded.insert(new_leader);

        Ok(())
    }

    /// Delete every database row of one copy: the save, its bindings and respawns.
    async fn delete_instance_rows(pool: &sqlx::PgPool, instance_id: u32) -> Result<()> {
        let mut transaction = pool.begin().await?;
        for statement in [
            "DELETE FROM characters.character_instance WHERE instance = $1",
            "DELETE FROM characters.group_instance WHERE instance = $1",
            "DELETE FROM characters.creature_respawn WHERE instance = $1",
            "DELETE FROM characters.gameobject_respawn WHERE instance = $1",
            "DELETE FROM characters.instance WHERE id = $1",
        ] {
            sqlx::query(statement)
                .bind(i64::from(instance_id))
                .execute(&mut *transaction)
                .await
                .with_context(|| format!("Failed to delete instance {instance_id}"))?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Reset a copy: delete its save, every binding to it and its respawn times.
    ///
    /// The copy is marked as resetting so nobody can enter it again; the world
    /// tears its map down once it is empty and then calls [`Self::finish_teardown`].
    pub async fn reset_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
        _method: InstanceResetMethod,
    ) -> Result<()> {
        Self::delete_instance_rows(&databases.character, instance_id).await?;

        self.instances.write().remove(&(map_id, instance_id));
        self.script_data.write().remove(&(map_id, instance_id));
        self.dungeon_reset_schedule
            .write()
            .remove(&(map_id, instance_id));
        self.resetting.write().insert((map_id, instance_id));
        self.forget_bindings_to(map_id, instance_id);

        Ok(())
    }

    /// Drop cached saves pointing at one copy (a player or group may still be
    /// bound to a different copy of the same map).
    fn forget_bindings_to(&self, map_id: u32, instance_id: u32) {
        for bindings in self.player_bindings.write().values_mut() {
            if bindings
                .get(&map_id)
                .is_some_and(|binding| binding.instance_id == instance_id)
            {
                bindings.remove(&map_id);
            }
        }
        for bindings in self.group_bindings.write().values_mut() {
            if bindings
                .get(&map_id)
                .is_some_and(|binding| binding.instance_id == instance_id)
            {
                bindings.remove(&map_id);
            }
        }
    }

    /// Delete every save of a raid map (its global reset). Copies that are still
    /// loaded are marked as resetting.
    async fn delete_raid_map_saves(&self, databases: &Databases, map_id: u32) -> Result<()> {
        let mut transaction = databases.character.begin().await?;
        for statement in [
            "DELETE FROM characters.character_instance b USING characters.instance i WHERE b.instance = i.id AND i.map = $1",
            "DELETE FROM characters.group_instance b USING characters.instance i WHERE b.instance = i.id AND i.map = $1",
            "DELETE FROM characters.creature_respawn WHERE map = $1 AND instance > 0",
            "DELETE FROM characters.gameobject_respawn WHERE map = $1 AND instance > 0",
            "DELETE FROM characters.instance WHERE map = $1",
        ] {
            sqlx::query(statement)
                .bind(i64::from(map_id))
                .execute(&mut *transaction)
                .await
                .with_context(|| format!("Failed to reset raid map {map_id}"))?;
        }
        transaction.commit().await?;

        let ids: Vec<u32> = {
            let mut instances = self.instances.write();
            let ids: Vec<u32> = instances
                .keys()
                .filter(|(map, _)| *map == map_id)
                .map(|&(_, id)| id)
                .collect();
            for id in &ids {
                instances.remove(&(map_id, *id));
            }
            ids
        };
        self.script_data
            .write()
            .retain(|(map, _), _| *map != map_id);
        {
            let mut resetting = self.resetting.write();
            for id in ids {
                resetting.insert((map_id, id));
            }
        }
        for bindings in self.player_bindings.write().values_mut() {
            bindings.remove(&map_id);
        }
        for bindings in self.group_bindings.write().values_mut() {
            bindings.remove(&map_id);
        }
        Ok(())
    }

    /// Check if instance has expired
    pub fn is_instance_expired(&self, map_id: u32, instance_id: u32) -> bool {
        let reset_time = match self.instances.read().get(&(map_id, instance_id)) {
            Some(instance) => instance.read().reset_time,
            None => return true, // Instance doesn't exist, consider it expired
        };
        let now = now_secs();
        if reset_time > 0 {
            return now >= reset_time;
        }
        // Raid copies end with their map's global reset.
        self.raid_reset_time(map_id).is_some_and(|t| now >= t)
    }

    /// Get instance reset time remaining (in seconds)
    pub fn get_reset_time_remaining(&self, map_id: u32, instance_id: u32) -> Option<u64> {
        let reset_time = self.instances.read().get(&(map_id, instance_id))?.read().reset_time;
        let reset_time = if reset_time > 0 {
            reset_time
        } else {
            self.raid_reset_time(map_id)?
        };
        Some(reset_time.saturating_sub(now_secs()))
    }

    /// Load player bindings from database
    pub async fn load_player_bindings(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        let mut connection = databases.character.acquire().await?;
        let bindings =
            Self::read_bindings(&mut connection, PLAYER_BINDINGS_QUERY, player_guid).await?;

        self.player_bindings
            .write()
            .insert(player_guid, bindings.clone());
        self.loaded_players.write().insert(player_guid);

        Ok(bindings)
    }

    /// Read a player's persisted saves unless they are already cached.
    pub async fn ensure_player_bindings(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
    ) -> Result<()> {
        if !self.loaded_players.read().contains(&player_guid) {
            self.load_player_bindings(databases, player_guid).await?;
        }
        Ok(())
    }

    /// Read a group's persisted saves unless they are already cached.
    pub async fn ensure_group_bindings(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
    ) -> Result<()> {
        if !self.group_bindings.read().contains_key(&leader_guid) {
            self.load_group_bindings(databases, leader_guid).await?;
        }
        Ok(())
    }

    /// Load group bindings from database (via leader GUID)
    pub async fn load_group_bindings(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        let mut connection = databases.character.acquire().await?;
        let bindings =
            Self::read_bindings(&mut connection, GROUP_BINDINGS_QUERY, leader_guid).await?;

        self.group_bindings
            .write()
            .insert(leader_guid, bindings.clone());

        Ok(bindings)
    }

    async fn read_bindings(
        connection: &mut PgConnection,
        query: &str,
        owner: ObjectGuid,
    ) -> Result<HashMap<u32, InstanceBinding>> {
        let rows = sqlx::query_as::<_, (i64, i64, i16, i64)>(query)
            .bind(i64::from(owner.low()))
            .fetch_all(connection)
            .await
            .context("Failed to query instance bindings")?;
        rows.into_iter()
            .map(|(map_id, instance_id, permanent, reset_time)| {
                let map_id = u32::try_from(map_id)?;
                Ok((
                    map_id,
                    InstanceBinding {
                        map_id,
                        instance_id: u32::try_from(instance_id)?,
                        permanent: permanent != 0,
                        reset_time: u64::try_from(reset_time).unwrap_or(0),
                    },
                ))
            })
            .collect()
    }

    /// Get or create instance for a player/group, without admission or binding.
    /// Returns the instance ID that should be used.
    pub async fn get_or_create_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        player_guid: Option<ObjectGuid>,
        group_leader_guid: Option<ObjectGuid>,
        is_raid: bool,
    ) -> Result<u32> {
        if let Some(guid) = player_guid {
            // Cache entries may contain only saves written during this session.
            // Restore all persisted personal saves before treating a miss as unbound.
            self.load_player_bindings(databases, guid).await?;
            if let Some(binding) = self
                .valid_binding(databases, self.get_player_binding(guid, map_id))
                .await?
            {
                return Ok(binding.instance_id);
            }
        }

        if let Some(leader_guid) = group_leader_guid {
            if let Some(binding) = self
                .valid_binding(databases, self.get_group_binding(leader_guid, map_id))
                .await?
            {
                return Ok(binding.instance_id);
            }
        }

        self.create_instance(databases, map_id, 0, is_raid).await
    }

    /// Rules that need no chosen copy, checked before any instance is created so
    /// a denied entry leaves no row behind. `bound_instance_id` is the copy the
    /// player is already saved to, which does not count as a new entry.
    fn admit_entry(
        &self,
        player_guid: ObjectGuid,
        bound_instance_id: Option<u32>,
        admission: &InstanceAdmission<'_>,
    ) -> Result<(), InstanceAdmissionDenied> {
        if admission.is_game_master {
            return Ok(());
        }
        if admission.is_raid && !admission.is_raid_group {
            return Err(InstanceAdmissionDenied::RaidGroupRequired);
        }
        let owner = admission
            .account_id
            .map_or(EnterOwner::Player(player_guid), EnterOwner::Account);
        if !self.check_instance_count(owner, bound_instance_id) {
            return Err(InstanceAdmissionDenied::TooManyInstances);
        }
        Ok(())
    }

    /// Copy-local rules, checked once the copy is chosen.
    fn admit_copy(
        &self,
        map_id: u32,
        instance_id: u32,
        admission: &InstanceAdmission<'_>,
    ) -> Result<(), InstanceAdmissionDenied> {
        if self.is_reset_pending(map_id, instance_id) {
            return Err(InstanceAdmissionDenied::ResetInProgress);
        }
        if !admission.is_game_master
            && admission.max_players > 0
            && admission.players_inside(map_id, instance_id) >= admission.max_players as usize
        {
            return Err(InstanceAdmissionDenied::MaxPlayers);
        }
        // No zoning into a raid while one of its encounters is engaged with the
        // player's group fighting inside; resurrections are unaffected.
        if !admission.is_game_master
            && admission.is_raid
            && admission
                .encounter_in_progress
                .is_some_and(|engaged| engaged(map_id, instance_id))
        {
            return Err(InstanceAdmissionDenied::ZoneInCombat);
        }
        Ok(())
    }

    /// Allow if the copy was already entered (it does not consume a new slot), if
    /// fewer than the limit of distinct copies are recent, or if an old entry has
    /// aged out.
    fn check_instance_count(&self, owner: EnterOwner, instance_id: Option<u32>) -> bool {
        let mut enter_times = self.instance_enter_times.write();
        let Some(entries) = enter_times.get_mut(&owner) else {
            return true;
        };
        if instance_id.is_some_and(|id| entries.contains_key(&id))
            || entries.len() < INSTANCE_ENTRIES_PER_HOUR
        {
            return true;
        }
        let now = now_secs();
        if let Some((&expired, _)) = entries
            .iter()
            .find(|(_, entered)| **entered + INSTANCE_ENTRY_WINDOW_SECS < now)
        {
            entries.remove(&expired);
            return true;
        }
        false
    }

    fn record_enter(&self, owner: EnterOwner, instance_id: u32) {
        self.instance_enter_times
            .write()
            .entry(owner)
            .or_default()
            .insert(instance_id, now_secs());
    }

    /// Record a player's entry into a copy when no account is known.
    pub fn record_instance_enter(&self, player_guid: ObjectGuid, instance_id: u32) {
        self.record_enter(EnterOwner::Player(player_guid), instance_id);
    }

    /// Record an entry against an account's hourly limit, once the player is
    /// actually inside the copy.
    pub fn record_account_instance_enter(&self, account_id: u32, instance_id: u32) {
        self.record_enter(EnterOwner::Account(account_id), instance_id);
    }

    /// True while a copy's save has been deleted but its map is still being emptied.
    pub fn is_reset_pending(&self, map_id: u32, instance_id: u32) -> bool {
        self.resetting.read().contains(&(map_id, instance_id))
    }

    /// Refuse further entry to a copy that is being torn down.
    pub fn mark_resetting(&self, map_id: u32, instance_id: u32) {
        self.resetting.write().insert((map_id, instance_id));
    }

    /// The copy's map was torn down: forget its runtime bookkeeping.
    pub fn finish_teardown(&self, map_id: u32, instance_id: u32) {
        let key = (map_id, instance_id);
        self.resetting.write().remove(&key);
        self.occupied.lock().remove(&key);
        self.empty_since.lock().remove(&key);
        // A torn-down copy with a surviving save stays in the cache; it is
        // reloaded from the database on the next entry either way.
        self.instances.write().remove(&key);
        self.script_data.write().remove(&key);
    }

    /// Schedule an empty normal dungeon copy to reset `delay_seconds` from now.
    pub fn schedule_instance_reset(&self, map_id: u32, instance_id: u32, delay_seconds: u64) {
        self.dungeon_reset_schedule
            .write()
            .insert((map_id, instance_id), now_secs() + delay_seconds);
    }

    /// Note how many players a loaded copy holds. Arrival cancels a normal
    /// dungeon's pending reset; the last player leaving arms it again at the
    /// copy's reset time. Returns how long the copy has been empty, if it is.
    pub fn observe_copy_population(
        &self,
        map_id: u32,
        instance_id: u32,
        players: usize,
        now: Instant,
    ) -> Option<std::time::Duration> {
        let key = (map_id, instance_id);
        if players > 0 {
            if self.occupied.lock().insert(key) {
                self.dungeon_reset_schedule.write().remove(&key);
            }
            self.empty_since.lock().remove(&key);
            return None;
        }

        if self.occupied.lock().remove(&key) {
            let reset_time = self
                .instances
                .read()
                .get(&key)
                .map(|instance| instance.read().reset_time)
                .unwrap_or(0);
            if reset_time > 0 {
                self.dungeon_reset_schedule.write().insert(key, reset_time);
            }
        }
        let since = *self.empty_since.lock().entry(key).or_insert(now);
        Some(now.saturating_duration_since(since))
    }

    /// A creature in a normal dungeon died: keep the copy alive until two hours
    /// after it respawns.
    pub async fn extend_dungeon_reset(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
        respawn_at_secs: u64,
    ) -> Result<()> {
        let wanted = respawn_at_secs + DUNGEON_RESET_AFTER_RESPAWN_SECS;
        let extended = {
            let instances = self.instances.read();
            let Some(instance) = instances.get(&(map_id, instance_id)) else {
                return Ok(());
            };
            let mut instance = instance.write();
            if instance.reset_time == 0 || instance.reset_time >= wanted {
                false
            } else {
                instance.reset_time = wanted;
                true
            }
        };
        if !extended {
            return Ok(());
        }
        if let Some(scheduled) = self
            .dungeon_reset_schedule
            .write()
            .get_mut(&(map_id, instance_id))
        {
            *scheduled = wanted;
        }
        sqlx::query("UPDATE characters.instance SET reset_time = $1 WHERE id = $2")
            .bind(wanted as i64)
            .bind(i64::from(instance_id))
            .execute(&databases.character)
            .await
            .context("Failed to extend dungeon reset time")?;
        Ok(())
    }

    /// A raid-locking creature died: permanently save everyone inside the copy,
    /// and the group too when its leader is among them. Returns the players who
    /// gained a permanent save (they are told their save was created).
    pub async fn perm_bind_raid(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
        players_inside: &[ObjectGuid],
        group_leader: Option<ObjectGuid>,
    ) -> Result<Vec<ObjectGuid>> {
        if self.is_reset_pending(map_id, instance_id) {
            return Ok(Vec::new());
        }
        let reset_time = self.raid_reset_time(map_id).unwrap_or(0);
        let mut newly_saved = Vec::new();
        for &player in players_inside {
            self.ensure_player_bindings(databases, player).await?;
            let already = self
                .get_player_binding(player, map_id)
                .is_some_and(|binding| binding.permanent);
            if !already {
                self.bind_player_to_instance(
                    databases,
                    player,
                    map_id,
                    instance_id,
                    true,
                    reset_time,
                )
                .await?;
                newly_saved.push(player);
            }
        }
        if let Some(leader) = group_leader.filter(|leader| players_inside.contains(leader)) {
            self.bind_group_to_instance(databases, leader, map_id, instance_id, true, reset_time)
                .await?;
        }
        Ok(newly_saved)
    }

    /// Scheduler tick: expire empty normal dungeons whose reset came due, and run
    /// the global raid warnings and resets.
    pub async fn update(&self, databases: &Databases) -> Result<InstanceUpdate> {
        let now = now_secs();
        let mut update = InstanceUpdate::default();

        let due: Vec<(u32, u32)> = self
            .dungeon_reset_schedule
            .read()
            .iter()
            .filter(|(_, &time)| now >= time)
            .map(|(&key, _)| key)
            .collect();
        for (map_id, instance_id) in due {
            // Someone may have arrived since the schedule was read.
            if self.occupied.lock().contains(&(map_id, instance_id)) {
                self.dungeon_reset_schedule
                    .write()
                    .remove(&(map_id, instance_id));
                continue;
            }
            match self
                .reset_instance(databases, map_id, instance_id, InstanceResetMethod::Expire)
                .await
            {
                Ok(()) => update.reset_copies.push((map_id, instance_id)),
                Err(e) => tracing::warn!(
                    "Failed to reset expired instance {}:{}: {}",
                    map_id,
                    instance_id,
                    e
                ),
            }
        }

        let raids: Vec<(u32, u64)> = self
            .raid_reset_times
            .read()
            .iter()
            .map(|(&map, &time)| (map, time))
            .collect();
        for (map_id, reset_time) in raids {
            if now >= reset_time {
                self.delete_raid_map_saves(databases, map_id).await?;
                let next = self.next_raid_reset(map_id, reset_time);
                self.store_raid_reset_time(databases, map_id, next).await?;
                update.reset_maps.push(map_id);
                continue;
            }
            let left = reset_time - now;
            let Some(threshold) = RAID_RESET_WARNINGS
                .iter()
                .copied()
                .filter(|&threshold| left <= threshold)
                .min()
            else {
                continue;
            };
            let mut sent = self.raid_warnings_sent.write();
            if sent.get(&map_id).map_or(true, |&last| threshold < last) {
                sent.insert(map_id, threshold);
                update.warnings.push((map_id, left));
            }
        }

        Ok(update)
    }

    fn next_raid_reset(&self, map_id: u32, previous: u64) -> u64 {
        let days = self
            .raid_reset_periods
            .read()
            .get(&map_id)
            .copied()
            .unwrap_or(7)
            .max(1);
        let hour = self.reset_hour.load(Ordering::Relaxed) * 3600;
        (previous + 60) / DAY * DAY + u64::from(days) * DAY + hour
    }

    fn entry_lock(&self, owner: EntryOwner) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.entry_locks.lock();
        if let Some(lock) = locks.get(&owner).and_then(Weak::upgrade) {
            return lock;
        }
        locks.retain(|_, lock| lock.strong_count() > 0);
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        locks.insert(owner, Arc::downgrade(&lock));
        lock
    }

    async fn valid_binding(
        &self,
        databases: &Databases,
        binding: Option<InstanceBinding>,
    ) -> Result<Option<InstanceBinding>> {
        if let Some(binding) = binding {
            if self
                .load_instance(databases, binding.map_id, binding.instance_id)
                .await?
                .is_some()
                && !self.is_instance_expired(binding.map_id, binding.instance_id)
            {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    /// The copy a player would be sent to for this map: their own save first,
    /// then their group's. Loads both from the database if needed.
    pub async fn bound_copy_for(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        group_leader: Option<ObjectGuid>,
        map_id: u32,
    ) -> Result<Option<InstanceBinding>> {
        self.ensure_player_bindings(databases, player_guid).await?;
        if let Some(binding) = self
            .valid_binding(databases, self.get_player_binding(player_guid, map_id))
            .await?
        {
            return Ok(Some(binding));
        }
        let Some(leader) = group_leader else {
            return Ok(None);
        };
        self.ensure_group_bindings(databases, leader).await?;
        self.valid_binding(databases, self.get_group_binding(leader, map_id))
            .await
    }

    /// Whether a player inside a copy is still saved to it, personally or through
    /// their group. Uses cached saves only.
    pub fn is_saved_to_copy(
        &self,
        player_guid: ObjectGuid,
        group_leader: Option<ObjectGuid>,
        map_id: u32,
        instance_id: u32,
    ) -> bool {
        self.get_player_binding(player_guid, map_id)
            .is_some_and(|binding| binding.instance_id == instance_id)
            || group_leader.is_some_and(|leader| {
                self.get_group_binding(leader, map_id)
                    .is_some_and(|binding| binding.instance_id == instance_id)
            })
    }

    /// Start, keep or clear the removal timer of a player inside a copy they are
    /// not saved to, and say what changed.
    pub fn track_unsaved_player(
        &self,
        player_guid: ObjectGuid,
        saved: bool,
        now: Instant,
        grace: std::time::Duration,
    ) -> Option<UnsavedTimer> {
        let mut deadlines = self.removal_deadlines.lock();
        if saved {
            return deadlines
                .remove(&player_guid)
                .map(|_| UnsavedTimer::Cleared);
        }
        match deadlines.get(&player_guid) {
            None => {
                deadlines.insert(player_guid, now + grace);
                Some(UnsavedTimer::Started)
            }
            Some(&deadline) if now >= deadline => {
                deadlines.remove(&player_guid);
                Some(UnsavedTimer::Expired)
            }
            Some(_) => None,
        }
    }

    /// Forget a player's removal timer (they left the copy).
    pub fn clear_unsaved_player(&self, player_guid: ObjectGuid) {
        self.removal_deadlines.lock().remove(&player_guid);
    }

    /// Enter an instance using the personal/group ownership rules, then run the
    /// admission rules before anything is bound. Returns the instance ID to use.
    pub async fn enter_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        player_guid: ObjectGuid,
        group: Option<&InstanceEntryGroup>,
        is_raid: bool,
        admission: &InstanceAdmission<'_>,
    ) -> Result<u32, InstanceEntryError> {
        let owner = group.map_or(EntryOwner::Player(player_guid), |group| {
            EntryOwner::Group(group.leader_guid)
        });
        let lock = self.entry_lock(owner);
        let _entry_guard = lock.lock().await;

        // A missing cache entry is not proof that this player has no saved copy.
        // Read before selection (and before any binding changes), including when
        // another map's save has already populated part of the player's cache.
        self.load_player_bindings(databases, player_guid).await?;

        if let Some(group) = group {
            self.ensure_group_bindings(databases, group.leader_guid)
                .await?;
        }
        let player_binding = self
            .valid_binding(databases, self.get_player_binding(player_guid, map_id))
            .await?;
        let group_binding = self
            .valid_binding(
                databases,
                group.and_then(|group| self.get_group_binding(group.leader_guid, map_id)),
            )
            .await?;

        // Copy-independent rules run before any copy is created or bound, so a
        // denied raid/limit check cannot leave an instance row behind.
        self.admit_entry(
            player_guid,
            player_binding
                .as_ref()
                .or(group_binding.as_ref())
                .map(|binding| binding.instance_id),
            admission,
        )?;

        // Any valid personal save wins; group transitions, not selection, handle
        // temporary personal saves.
        let instance_id = match player_binding.as_ref().or(group_binding.as_ref()) {
            Some(binding) => binding.instance_id,
            None => self.create_instance(databases, map_id, 0, is_raid).await?,
        };

        // Copy-local rules inspect the chosen copy, so they run after selection
        // but still before any binding or transfer is committed.
        self.admit_copy(map_id, instance_id, admission)?;

        // Visiting a permanently saved personal copy never changes the group's save.
        if player_binding
            .as_ref()
            .is_some_and(|binding| binding.permanent)
        {
            return Ok(instance_id);
        }

        let instance = self
            .load_instance(databases, map_id, instance_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to load instance"))?;
        let reset_time = {
            let instance_guard = instance.read();
            instance_guard.reset_time
        };

        if let Some(group) = group {
            if let Some(binding) = group_binding {
                // A permanently saved group saves each member who enters as well.
                if player_binding.is_none() && binding.permanent {
                    self.bind_player_to_instance(
                        databases,
                        player_guid,
                        map_id,
                        instance_id,
                        true,
                        reset_time,
                    )
                    .await?;
                }
            } else {
                let mut released_players = group.members_outside.clone();
                if player_binding.is_some() {
                    released_players.push(player_guid);
                }
                self.bind_new_group_on_entry(
                    databases,
                    group.leader_guid,
                    InstanceBinding {
                        map_id,
                        instance_id,
                        permanent: false,
                        reset_time,
                    },
                    &released_players,
                )
                .await?;
            }
        } else if player_binding.is_none() {
            self.bind_player_to_instance(
                databases,
                player_guid,
                map_id,
                instance_id,
                false,
                reset_time,
            )
            .await?;
        }

        Ok(instance_id)
    }

    async fn bind_new_group_on_entry(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        binding: InstanceBinding,
        released_players: &[ObjectGuid],
    ) -> Result<()> {
        let mut transaction = databases.character.begin().await?;
        // Replace any expired save for this map and convert temporary solo
        // bindings in one transaction; keep the instance data itself intact.
        sqlx::query("DELETE FROM characters.group_instance b USING characters.instance i WHERE b.instance = i.id AND b.leader_guid = $1 AND i.map = $2")
            .bind(i64::from(leader_guid.low()))
            .bind(i64::from(binding.map_id))
            .execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO characters.group_instance (leader_guid, instance, permanent) VALUES ($1, $2, 0)")
            .bind(i64::from(leader_guid.low()))
            .bind(i64::from(binding.instance_id))
            .execute(&mut *transaction).await?;
        let player_ids: Vec<i64> = released_players
            .iter()
            .map(|guid| i64::from(guid.low()))
            .collect();
        sqlx::query("DELETE FROM characters.character_instance b USING characters.instance i WHERE b.instance = i.id AND b.guid = ANY($1) AND i.map = $2 AND b.permanent = 0")
            .bind(&player_ids)
            .bind(i64::from(binding.map_id))
            .execute(&mut *transaction).await?;
        transaction.commit().await?;

        let mut groups = self.group_bindings.write();
        let mut players = self.player_bindings.write();
        for guid in released_players {
            if let Some(bindings) = players.get_mut(guid) {
                if bindings
                    .get(&binding.map_id)
                    .is_some_and(|binding| !binding.permanent)
                {
                    bindings.remove(&binding.map_id);
                }
            }
        }
        groups
            .entry(leader_guid)
            .or_default()
            .insert(binding.map_id, binding);
        Ok(())
    }

    /// Unbind all players and groups from instance (called on reset)
    pub async fn unbind_instance(
        &self,
        databases: &Databases,
        map_id: u32,
        instance_id: u32,
    ) -> Result<()> {
        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.character_instance WHERE instance = $1"#,
        )
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind players from instance")?;

        sqlx::query::<sqlx::Postgres>(
            r#"DELETE FROM characters.group_instance WHERE instance = $1"#,
        )
        .bind(i64::from(instance_id))
        .execute(&databases.character)
        .await
        .context("Failed to unbind groups from instance")?;

        // The database delete above is keyed by the instance, so only drop cache
        // entries that point at this same copy.
        self.forget_bindings_to(map_id, instance_id);

        Ok(())
    }

    /// Reset a player's own saves. Saves that are permanent, for the map the
    /// player is on, or (for `ResetScope::All`) raids are skipped; a copy with
    /// players inside cannot be reset.
    pub async fn reset_player_instances(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        current_map: Option<u32>,
        scope: ResetScope,
        is_raid_map: impl Fn(u32) -> bool,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> ResetOutcome {
        let mut outcome = ResetOutcome::default();
        if let Err(e) = self.ensure_player_bindings(databases, player_guid).await {
            tracing::warn!("Failed to load saves of {:?} for reset: {}", player_guid, e);
            return outcome;
        }
        let bindings = self
            .player_bindings
            .read()
            .get(&player_guid)
            .cloned()
            .unwrap_or_default();

        for (map_id, binding) in bindings {
            if binding.permanent || current_map == Some(map_id) {
                continue;
            }
            if scope == ResetScope::All && is_raid_map(map_id) {
                continue;
            }
            if let Ok(Some(_)) = self
                .load_instance(databases, map_id, binding.instance_id)
                .await
            {
                if !self.can_reset(map_id, binding.instance_id) {
                    continue;
                }
            }
            if !get_players_in_instance(map_id, binding.instance_id).is_empty() {
                if scope == ResetScope::All {
                    outcome
                        .failed
                        .push((map_id, InstanceResetFailReason::General));
                }
                continue;
            }
            match self
                .reset_instance(
                    databases,
                    map_id,
                    binding.instance_id,
                    InstanceResetMethod::Manual,
                )
                .await
            {
                Ok(()) => {
                    outcome.reset_maps.push(map_id);
                    outcome.reset_copies.push((map_id, binding.instance_id));
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to reset instance {} for {:?}: {}",
                        binding.instance_id,
                        player_guid,
                        e
                    );
                    if scope == ResetScope::All {
                        outcome
                            .failed
                            .push((map_id, InstanceResetFailReason::General));
                    }
                }
            }
        }

        outcome
    }

    /// Reset a group's saves. For `ResetScope::All` raids are skipped and any
    /// offline member blocks each reset; a copy with players inside is not reset.
    /// A disbanding group drops every save: resettable copies are deleted, while
    /// copies someone is permanently saved to only lose the group's binding.
    pub async fn reset_group_instances(
        &self,
        databases: &Databases,
        leader_guid: ObjectGuid,
        scope: ResetScope,
        members_offline: bool,
        is_raid_map: impl Fn(u32) -> bool,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> ResetOutcome {
        let mut outcome = ResetOutcome::default();
        if let Err(e) = self.ensure_group_bindings(databases, leader_guid).await {
            tracing::warn!("Failed to load group saves of {:?}: {}", leader_guid, e);
            return outcome;
        }
        let bindings = self
            .group_bindings
            .read()
            .get(&leader_guid)
            .cloned()
            .unwrap_or_default();

        for (map_id, binding) in bindings {
            let instance_id = binding.instance_id;
            let loaded = matches!(
                self.load_instance(databases, map_id, instance_id).await,
                Ok(Some(_))
            );
            let resettable = !binding.permanent && (!loaded || self.can_reset(map_id, instance_id));
            if !resettable && scope != ResetScope::GroupDisband {
                continue;
            }
            if scope == ResetScope::All {
                if is_raid_map(map_id) {
                    continue;
                }
                if members_offline {
                    outcome
                        .failed
                        .push((map_id, InstanceResetFailReason::Offline));
                    continue;
                }
            }

            let empty = get_players_in_instance(map_id, instance_id).is_empty();
            if !empty && scope != ResetScope::GroupDisband {
                outcome
                    .failed
                    .push((map_id, InstanceResetFailReason::General));
                continue;
            }

            let result = if resettable {
                self.reset_instance(databases, map_id, instance_id, InstanceResetMethod::Manual)
                    .await
                    .map(|()| {
                        outcome.reset_copies.push((map_id, instance_id));
                    })
            } else {
                self.unbind_group_from_instance(databases, leader_guid, map_id, instance_id)
                    .await
            };
            match result {
                Ok(()) => outcome.reset_maps.push(map_id),
                Err(e) => {
                    tracing::warn!(
                        "Failed to reset group instance {} for {:?}: {}",
                        instance_id,
                        leader_guid,
                        e
                    );
                    outcome
                        .failed
                        .push((map_id, InstanceResetFailReason::General));
                }
            }
        }

        outcome
    }

    /// A player in a group left a dungeon. A temporary solo save they kept
    /// because they were inside it when they joined is now replaced by the
    /// group's save for that map: reset if it is a different copy, otherwise
    /// just dropped in favour of the group's binding.
    pub async fn release_solo_save_on_leave(
        &self,
        databases: &Databases,
        player_guid: ObjectGuid,
        group_leader: ObjectGuid,
        map_id: u32,
        get_players_in_instance: impl Fn(u32, u32) -> Vec<ObjectGuid>,
    ) -> Result<Option<(u32, u32)>> {
        self.ensure_player_bindings(databases, player_guid).await?;
        self.ensure_group_bindings(databases, group_leader).await?;
        let Some(personal) = self
            .get_player_binding(player_guid, map_id)
            .filter(|binding| !binding.permanent)
        else {
            return Ok(None);
        };
        let Some(group) = self.get_group_binding(group_leader, map_id) else {
            return Ok(None);
        };
        if group.instance_id != personal.instance_id
            && get_players_in_instance(map_id, personal.instance_id).is_empty()
            && (!matches!(
                self.load_instance(databases, map_id, personal.instance_id)
                    .await,
                Ok(Some(_))
            ) || self.can_reset(map_id, personal.instance_id))
        {
            self.reset_instance(
                databases,
                map_id,
                personal.instance_id,
                InstanceResetMethod::Manual,
            )
            .await?;
            return Ok(Some((map_id, personal.instance_id)));
        }
        self.unbind_player_from_instance(databases, player_guid, map_id, personal.instance_id)
            .await?;
        Ok(None)
    }

    /// Permanent raid saves to list in the raid information window:
    /// `(map, seconds until reset, instance)`.
    pub fn raid_info(&self, player_guid: ObjectGuid) -> Vec<(u32, u64, u32)> {
        let now = now_secs();
        self.player_bindings
            .read()
            .get(&player_guid)
            .map(|bindings| {
                bindings
                    .values()
                    .filter(|binding| binding.permanent)
                    .map(|binding| {
                        let reset = self
                            .raid_reset_time(binding.map_id)
                            .unwrap_or(binding.reset_time);
                        (binding.map_id, reset.saturating_sub(now), binding.instance_id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for InstanceMgr {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
