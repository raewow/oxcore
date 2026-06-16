use bitflags::bitflags;

/// Player rest state type (for rest experience bonus)
/// Reference: MaNGOS Player.h:575-580
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RestType {
    /// Not resting
    No = 0,
    /// Resting in a tavern (inn area trigger)
    InTavern = 1,
    /// Resting in a city (capital city zone)
    InCity = 2,
}

/// Per-player environment and rest state
#[derive(Debug, Clone)]
pub struct EnvironmentState {
    // ---- Rest tracking ----
    /// Current rest type (None, InTavern, InCity)
    pub rest_type: RestType,
    /// Accumulated rest XP bonus (up to 1.5 levels worth of XP)
    pub rest_bonus: f32,
    /// Area trigger ID that started the current rest period
    pub inn_trigger_id: u32,
    /// Unix timestamp when player entered the inn/rest area
    pub time_inn_enter: u64,

    // ---- Mirror timers ----
    /// Breath timer (drowning when fully submerged underwater)
    pub breath_timer: MirrorTimer,
    /// Fatigue timer (exhaustion in deep ocean / high seas)
    pub fatigue_timer: MirrorTimer,
    /// Environmental timer (lava/slime damage, hidden from client)
    pub environmental_timer: MirrorTimer,

    // ---- Environment flags ----
    /// Current liquid/environment state flags
    pub env_flags: EnvironmentFlags,
    /// Water breathing interval multiplier (modified by auras)
    pub breathing_multiplier: f32,
}

/// Mirror timer structure
/// Based on C++ MirrorTimer class from vmangos
///
/// The timer uses a scale system:
/// - scale < 0: Timer is counting down (depleting)
/// - scale > 0: Timer is regenerating (refilling)
/// - scale == 0: Timer is frozen (paused)
#[derive(Debug, Clone)]
pub struct MirrorTimer {
    /// Whether the timer is currently running
    pub active: bool,
    /// Scale factor: -1 = counting down, 0 = frozen, +1 = recovering
    pub scale: i32,
    /// Current tracker position (elapsed time within interval)
    pub current_ms: u32,
    /// Maximum duration (interval) in milliseconds
    pub max_ms: u32,
    /// Pulse accumulator for periodic damage ticks
    pub pulse_timer_ms: u32,
    /// Interval between damage pulses (typically 2000ms)
    pub pulse_interval_ms: u32,
    /// Associated spell ID (for Water Breathing, Feign Death, etc.)
    pub spell_id: u32,
    /// Network status tracking (unchanged, full update, status update)
    pub status: MirrorTimerStatus,
    /// Whether the timer is frozen (paused)
    pub frozen: bool,
}

/// Mirror timer network status for efficient client updates
/// Only send packets on state transitions, not every tick
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorTimerStatus {
    /// No change since last network send
    Unchanged = 0,
    /// Full update needed (timer started, scale changed, duration changed)
    FullUpdate = 1,
    /// Status update needed (paused/unpaused, stopped)
    StatusUpdate = 2,
}

/// Mirror timer types matching client expectations
/// The client only displays three timers (Fatigue, Breath, FeignDeath).
/// Environmental is index 3 and is never sent to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MirrorTimerType {
    Fatigue = 0,
    Breath = 1,
    FeignDeath = 2,
    Environmental = 3,
}

/// Number of client-visible timers (Fatigue, Breath, FeignDeath)
pub const NUM_CLIENT_TIMERS: usize = 3;
/// Total number of timers including hidden Environmental
pub const NUM_TIMERS: usize = 4;

impl MirrorTimerType {
    /// Returns true if the client should see this timer bar
    pub fn is_client_timer(&self) -> bool {
        (*self as usize) < NUM_CLIENT_TIMERS
    }
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            rest_type: RestType::No,
            rest_bonus: 0.0,
            inn_trigger_id: 0,
            time_inn_enter: 0,
            breath_timer: MirrorTimer::new(),
            fatigue_timer: MirrorTimer::new(),
            environmental_timer: MirrorTimer::new(),
            env_flags: EnvironmentFlags::NONE,
            breathing_multiplier: 1.0,
        }
    }
}

impl MirrorTimer {
    pub fn new() -> Self {
        Self {
            active: false,
            scale: -1,
            current_ms: 0,
            max_ms: 0,
            pulse_timer_ms: 0,
            pulse_interval_ms: MIRROR_TIMER_PULSE_INTERVAL,
            spell_id: 0,
            status: MirrorTimerStatus::Unchanged,
            frozen: false,
        }
    }

    /// Start the timer with a maximum duration and optional spell ID
    pub fn start(&mut self, max_duration_ms: u32, spell_id: u32) {
        if self.scale < 0 {
            self.active = true;
            self.current_ms = 0;
            self.max_ms = max_duration_ms;
            self.pulse_timer_ms = 0;
            self.pulse_interval_ms = MIRROR_TIMER_PULSE_INTERVAL;
            self.spell_id = spell_id;
            self.status = MirrorTimerStatus::FullUpdate;
        } else {
            self.stop();
        }
    }

    /// Start with a known remaining time (e.g., restoring from a buff)
    pub fn start_with_current(&mut self, remaining: u32, max: u32, spell_id: u32) {
        self.start(max, spell_id);
        if self.active {
            self.current_ms = max.saturating_sub(remaining);
            self.frozen = false;
        }
    }

    /// Stop and reset the timer
    pub fn stop(&mut self) {
        if self.active {
            self.active = false;
            self.pulse_timer_ms = 0;
            self.current_ms = 0;
            self.status = MirrorTimerStatus::StatusUpdate;
        }
    }

    /// Get remaining time before expiration
    pub fn remaining(&self) -> u32 {
        self.max_ms.saturating_sub(self.current_ms)
    }

    /// Fetch and reset the network status (consume the pending update)
    pub fn fetch_status(&mut self) -> MirrorTimerStatus {
        let s = self.status;
        self.status = MirrorTimerStatus::Unchanged;
        s
    }

    /// Update the timer by `diff` milliseconds.
    /// Returns false when the timer expires and a damage pulse should fire.
    pub fn update(&mut self, diff: u32) -> bool {
        if !self.active || self.frozen {
            return true;
        }

        let scaled_diff = diff * self.scale.unsigned_abs();

        if self.scale < 0 {
            // Counting down toward expiration
            self.current_ms = self.current_ms.saturating_add(scaled_diff);

            if self.current_ms < self.max_ms {
                return true; // Not yet expired
            }

            let overflow = self.current_ms.saturating_sub(self.max_ms);
            self.current_ms = self.max_ms; // Clamp

            // After initial expiration, use pulse timer for periodic damage
            if overflow == scaled_diff {
                self.pulse_timer_ms = self.pulse_timer_ms.saturating_add(overflow);
                if self.pulse_timer_ms < self.pulse_interval_ms {
                    return true; // Pulse not yet ready
                }
                self.pulse_timer_ms = 0;
            }

            false // Signal damage pulse
        } else {
            // Regenerating (refilling)
            if self.current_ms > scaled_diff {
                self.current_ms = self.current_ms.saturating_sub(scaled_diff);
                self.pulse_timer_ms = 0;
            } else {
                self.stop(); // Fully recovered
            }
            true
        }
    }

    /// Set the scale factor (negative = depleting, positive = recovering)
    pub fn set_scale(&mut self, scale: i32) {
        if scale == 0 {
            self.frozen = true;
            return;
        }
        if self.active && scale != self.scale {
            self.status = MirrorTimerStatus::FullUpdate;
        }
        self.scale = scale;
    }

    /// Set frozen (paused) state
    pub fn set_frozen(&mut self, state: bool) {
        if self.active && state != self.frozen {
            self.status = MirrorTimerStatus::StatusUpdate;
        }
        self.frozen = state;
    }

    /// Set maximum duration, triggering a full update if changed
    pub fn set_duration(&mut self, duration: u32) {
        if duration == 0 {
            return self.stop();
        }
        if self.active && duration != self.max_ms {
            self.status = MirrorTimerStatus::FullUpdate;
        }
        self.max_ms = duration;
    }
}

/// Pulse interval for damage ticks (2 seconds)
pub const MIRROR_TIMER_PULSE_INTERVAL: u32 = 2000;

bitflags! {
    /// Environment flags for liquid/environmental state tracking
    /// Updated each tick from terrain/liquid data based on player position
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EnvironmentFlags: u8 {
        /// No environment flags set
        const NONE       = 0x00;
        /// Swimming or standing in water
        const IN_WATER   = 0x01;
        /// Swimming or standing in magma (lava)
        const IN_MAGMA   = 0x02;
        /// Swimming or standing in slime
        const IN_SLIME   = 0x04;
        /// In deep water area (high seas / fatigue zone)
        const HIGH_SEA   = 0x08;
        /// Fully submerged underwater (head below surface)
        const UNDERWATER = 0x10;
        /// In liquid deep enough to swim
        const HIGH_LIQUID = 0x20;
        /// Anywhere inside area with any liquid
        const LIQUID      = 0x40;

        /// Composite: hazardous liquids (magma or slime)
        const MASK_LIQUID_HAZARD = Self::IN_MAGMA.bits() | Self::IN_SLIME.bits();
        /// Composite: any liquid contact
        const MASK_IN_LIQUID = Self::IN_WATER.bits() | Self::MASK_LIQUID_HAZARD.bits();
        /// Composite: all liquid-related flags
        const MASK_LIQUID_FLAGS = Self::UNDERWATER.bits() | Self::MASK_IN_LIQUID.bits()
            | Self::HIGH_SEA.bits() | Self::LIQUID.bits() | Self::HIGH_LIQUID.bits();
    }
}

/// Environmental damage types sent in SMSG_ENVIRONMENTALDAMAGELOG.
/// The client uses these to select the appropriate combat log string
/// and damage school icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnvironmentalDamageType {
    /// Fatigue/exhaustion damage (from deep water fatigue timer)
    Exhausted = 0,
    /// Drowning damage (from breath timer expiring underwater)
    Drowning = 1,
    /// Fall damage (from landing after a high fall)
    Fall = 2,
    /// Lava damage (from standing in magma)
    Lava = 3,
    /// Slime/Nature damage (from standing in slime)
    Slime = 4,
    /// Generic fire damage
    Fire = 5,
    /// Fall to void (custom: fall damage without durability loss)
    FallToVoid = 6,
}
