use super::state::*;

/// Default breath timer maximum (60 seconds)
pub const BREATH_MAX_SECONDS: u32 = 60;
/// Default fatigue timer maximum (60 seconds)
pub const FATIGUE_MAX_SECONDS: u32 = 60;
/// Default environmental timer maximum (1 second, rapid first pulse)
pub const ENVIRONMENTAL_MAX_SECONDS: u32 = 1;

/// Minimum lava/slime environmental damage per tick
pub const ENVIRONMENTAL_DAMAGE_MIN: u32 = 50;
/// Maximum lava/slime environmental damage per tick
pub const ENVIRONMENTAL_DAMAGE_MAX: u32 = 250;

/// Check whether the breath timer should activate.
///
/// Activates when the player is fully underwater AND the water breathing
/// interval is non-zero (zero means infinite breathing from an aura).
pub fn should_activate_breath(env: &EnvironmentState) -> bool {
    env.env_flags.contains(EnvironmentFlags::UNDERWATER) && get_water_breathing_interval(env) > 0
}

/// Check whether the fatigue timer should activate.
///
/// Activates in high seas (deep ocean) when the player is not on a taxi
/// flight or transport.
pub fn should_activate_fatigue(
    env: &EnvironmentState,
    is_flying: bool,
    is_transport: bool,
) -> bool {
    env.env_flags.contains(EnvironmentFlags::HIGH_SEA) && !is_flying && !is_transport
}

/// Check whether the environmental timer should activate.
///
/// Activates when standing in lava or slime.
pub fn should_activate_environmental(env: &EnvironmentState) -> bool {
    env.env_flags.contains(EnvironmentFlags::MASK_LIQUID_HAZARD)
}

/// Check whether a timer should deactivate (stop counting).
///
/// Breath and Environmental: deactivate when not in liquid or when dead.
/// Fatigue: deactivate when not in liquid, or when dead and not a ghost.
pub fn should_deactivate(
    timer_type: MirrorTimerType,
    env: &EnvironmentState,
    is_alive: bool,
    is_ghost: bool,
) -> bool {
    let not_in_liquid = !env.env_flags.contains(EnvironmentFlags::LIQUID);

    match timer_type {
        MirrorTimerType::Breath => not_in_liquid || !is_alive,
        MirrorTimerType::Fatigue => not_in_liquid || (!is_alive && !is_ghost),
        MirrorTimerType::Environmental => not_in_liquid || !is_alive,
        MirrorTimerType::FeignDeath => false, // Handled separately by aura system
    }
}

/// Get the water breathing interval in milliseconds.
///
/// If the player has a Water Breathing aura, returns 0 (infinite).
/// Otherwise, returns BREATH_MAX * 1000 * breathing_multiplier.
pub fn get_water_breathing_interval(env: &EnvironmentState) -> u32 {
    // In actual implementation, check for AuraType::WaterBreathing here.
    // If present, return 0.
    (BREATH_MAX_SECONDS as f32 * 1000.0 * env.breathing_multiplier) as u32
}

/// Get the maximum duration for a mirror timer type in milliseconds.
pub fn get_max_duration(timer_type: MirrorTimerType, env: &EnvironmentState) -> u32 {
    match timer_type {
        MirrorTimerType::Fatigue => FATIGUE_MAX_SECONDS * 1000,
        MirrorTimerType::Breath => get_water_breathing_interval(env),
        MirrorTimerType::Environmental => ENVIRONMENTAL_MAX_SECONDS * 1000,
        MirrorTimerType::FeignDeath => 0, // Determined by aura duration
    }
}

/// Events produced by the mirror timer update
#[derive(Debug, Clone, Copy)]
pub enum MirrorTimerEvent {
    /// A timer started (may need to send SMSG_START_MIRROR_TIMER)
    Started(MirrorTimerType),
    /// A timer expired and a damage pulse should be applied
    DamagePulse(MirrorTimerType),
}

/// Main mirror timer update loop.
///
/// For each timer type, checks activation/deactivation conditions,
/// starts or stops timers, and processes damage pulses on expiration.
///
/// Called every world tick (typically 50ms) from the player update loop.
pub fn update_mirror_timers(
    env: &mut EnvironmentState,
    diff_ms: u32,
    is_alive: bool,
    is_ghost: bool,
    is_flying: bool,
    is_transport: bool,
    has_water_breathing: bool,
) -> Vec<MirrorTimerEvent> {
    let mut events = Vec::new();

    // Process each timer type
    for (timer_type, timer) in [
        (MirrorTimerType::Fatigue, &mut env.fatigue_timer),
        (MirrorTimerType::Breath, &mut env.breath_timer),
        (MirrorTimerType::Environmental, &mut env.environmental_timer),
    ] {
        let was_active = timer.active;

        let should_activate = match timer_type {
            MirrorTimerType::Fatigue => {
                env.env_flags.contains(EnvironmentFlags::HIGH_SEA) && !is_flying && !is_transport
            }
            MirrorTimerType::Breath => {
                env.env_flags.contains(EnvironmentFlags::UNDERWATER) && !has_water_breathing
            }
            MirrorTimerType::Environmental => {
                env.env_flags.contains(EnvironmentFlags::MASK_LIQUID_HAZARD)
            }
            _ => false,
        };

        let should_deactivate = {
            let not_in_liquid = !env.env_flags.contains(EnvironmentFlags::LIQUID);
            match timer_type {
                MirrorTimerType::Breath => not_in_liquid || !is_alive,
                MirrorTimerType::Fatigue => not_in_liquid || (!is_alive && !is_ghost),
                MirrorTimerType::Environmental => not_in_liquid || !is_alive,
                _ => false,
            }
        };

        if was_active || should_activate {
            if should_deactivate {
                timer.stop();
            } else if was_active {
                // Timer is running: update and check for damage pulse
                if !timer.update(diff_ms) {
                    events.push(MirrorTimerEvent::DamagePulse(timer_type));
                }
            } else {
                // Timer needs to start
                let max_duration = match timer_type {
                    MirrorTimerType::Fatigue => FATIGUE_MAX_SECONDS * 1000,
                    MirrorTimerType::Breath => {
                        (BREATH_MAX_SECONDS as f32 * 1000.0 * env.breathing_multiplier) as u32
                    }
                    MirrorTimerType::Environmental => ENVIRONMENTAL_MAX_SECONDS * 1000,
                    _ => 0,
                };
                timer.start(max_duration, 0);
                events.push(MirrorTimerEvent::Started(timer_type));
            }
        }
    }

    events
}

/// Action to take after a mirror timer pulse
#[derive(Debug, Clone, Copy)]
pub enum MirrorTimerAction {
    /// No action needed
    None,
    /// Apply environmental damage
    Damage {
        damage_type: EnvironmentalDamageType,
        amount: u32,
    },
    /// Teleport the ghost to the nearest graveyard
    TeleportToGraveyard,
}

/// Handle a damage pulse from an expired mirror timer.
///
/// Called when `MirrorTimer::update()` returns false, indicating the timer
/// has expired and a pulse interval has elapsed.
///
/// # Arguments
/// * `timer_type` - Which timer expired
/// * `max_health` - Player's maximum health
/// * `level` - Player's level (for random damage component)
/// * `is_alive` - Whether the player is currently alive
/// * `is_ghost` - Whether the player is in ghost form
/// * `env_flags` - Current environment flags (to distinguish lava vs slime)
///
/// # Returns
/// An action to take: apply damage, or teleport ghost to graveyard
pub fn on_mirror_timer_expiration_pulse(
    timer_type: MirrorTimerType,
    max_health: u32,
    level: u8,
    is_alive: bool,
    is_ghost: bool,
    env_flags: EnvironmentFlags,
) -> MirrorTimerAction {
    match timer_type {
        MirrorTimerType::Fatigue => {
            if is_alive {
                // Fatigue damage: max_health/5 + random(0..level)
                let base = max_health / 5;
                let random_part = if level > 1 {
                    rand::random::<u32>() % level as u32
                } else {
                    0
                };
                MirrorTimerAction::Damage {
                    damage_type: EnvironmentalDamageType::Exhausted,
                    amount: base + random_part,
                }
            } else if is_ghost {
                // Ghost in fatigue zone: teleport to nearest graveyard
                MirrorTimerAction::TeleportToGraveyard
            } else {
                MirrorTimerAction::None
            }
        }

        MirrorTimerType::Breath => {
            // Drowning damage: max_health/5 + random(0..level)
            let base = max_health / 5;
            let random_part = if level > 1 {
                rand::random::<u32>() % level as u32
            } else {
                0
            };
            MirrorTimerAction::Damage {
                damage_type: EnvironmentalDamageType::Drowning,
                amount: base + random_part,
            }
        }

        MirrorTimerType::Environmental => {
            // Lava or slime damage: random between 50 and 250
            let amount = ENVIRONMENTAL_DAMAGE_MIN
                + rand::random::<u32>() % (ENVIRONMENTAL_DAMAGE_MAX - ENVIRONMENTAL_DAMAGE_MIN);

            let damage_type = if env_flags.contains(EnvironmentFlags::IN_MAGMA) {
                EnvironmentalDamageType::Lava
            } else {
                EnvironmentalDamageType::Slime
            };

            MirrorTimerAction::Damage {
                damage_type,
                amount,
            }
        }

        MirrorTimerType::FeignDeath => {
            // Feign death expired - handled by aura system
            MirrorTimerAction::None
        }
    }
}
