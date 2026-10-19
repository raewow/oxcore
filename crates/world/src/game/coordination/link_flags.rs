use oxcore_shared::protocol::ObjectGuid;

bitflags::bitflags! {
    /// creature_linking flags: what happens to a linked creature when its
    /// master (or, for the `TO_` flags, its slave) is pulled, evades, dies,
    /// respawns or despawns.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LinkFlags: u32 {
        /// Slave joins the fight when the master is pulled.
        const AGGRO_ON_AGGRO = 0x0001;
        /// Master joins the fight when the slave is pulled.
        const TO_AGGRO_ON_AGGRO = 0x0002;
        /// Dead slave respawns when the master evades.
        const RESPAWN_ON_EVADE = 0x0004;
        /// Dead master respawns when the slave evades.
        const TO_RESPAWN_ON_EVADE = 0x0008;
        /// Slave despawns when the master dies.
        const DESPAWN_ON_DEATH = 0x0010;
        /// Slave dies when the master dies.
        const SELFKILL_ON_DEATH = 0x0020;
        /// Dead slave respawns when the master dies.
        const RESPAWN_ON_DEATH = 0x0040;
        /// Dead slave respawns when the master respawns.
        const RESPAWN_ON_RESPAWN = 0x0080;
        /// Slave despawns when the master respawns.
        const DESPAWN_ON_RESPAWN = 0x0100;
        /// Slave follows the master.
        const FOLLOW = 0x0200;
        /// Slave cannot spawn while the master is dead.
        const CANT_SPAWN_IF_BOSS_DEAD = 0x0400;
        /// Slave cannot spawn while the master is alive.
        const CANT_SPAWN_IF_BOSS_ALIVE = 0x0800;
        /// Slave despawns when the master evades.
        const DESPAWN_ON_EVADE = 0x1000;
        /// Slave despawns when the master despawns.
        const DESPAWN_ON_DESPAWN = 0x2000;
        /// Slave evades when the master evades.
        const EVADE_ON_EVADE = 0x4000;
    }
}

/// Things that happen to a master creature that its linked creatures react to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkEvent {
    Aggro { enemy: ObjectGuid },
    Evade,
    Die,
    Respawn,
    Despawn,
}

impl LinkEvent {
    /// The slave flags this event can act on.
    pub fn slave_mask(&self) -> LinkFlags {
        match self {
            LinkEvent::Aggro { .. } => LinkFlags::AGGRO_ON_AGGRO,
            LinkEvent::Evade => {
                LinkFlags::RESPAWN_ON_EVADE | LinkFlags::DESPAWN_ON_EVADE | LinkFlags::EVADE_ON_EVADE
            }
            LinkEvent::Die => {
                LinkFlags::DESPAWN_ON_DEATH
                    | LinkFlags::SELFKILL_ON_DEATH
                    | LinkFlags::RESPAWN_ON_DEATH
                    | LinkFlags::FOLLOW
            }
            LinkEvent::Respawn => {
                LinkFlags::RESPAWN_ON_RESPAWN | LinkFlags::DESPAWN_ON_RESPAWN | LinkFlags::FOLLOW
            }
            LinkEvent::Despawn => LinkFlags::DESPAWN_ON_DESPAWN,
        }
    }

    /// The flag on the slave's own link that makes the event reach its master.
    pub fn master_mask(&self) -> LinkFlags {
        match self {
            LinkEvent::Aggro { .. } => LinkFlags::TO_AGGRO_ON_AGGRO,
            LinkEvent::Evade => LinkFlags::TO_RESPAWN_ON_EVADE,
            LinkEvent::Respawn => LinkFlags::FOLLOW,
            LinkEvent::Die | LinkEvent::Despawn => LinkFlags::empty(),
        }
    }
}
