//! State and timing shared by every transport (`GenericTransport`).
//!
//! The full `GenericTransport` is a GameObject with a passenger set and a path cursor; only
//! the infra-free timing is ported here. Passenger management and map relocation follow once
//! the Object/Map subsystems exist.

/// Milliseconds elapsed between two wrapping 32-bit millisecond clocks
/// (`WorldTimer::getMSTimeDiff`).
///
/// The server clock is a `uint32` of milliseconds that wraps roughly every 49 days. When
/// `old` reads larger than `now` the clock has either wrapped or drifted backwards, so the
/// smaller of the two interpretations - a full wrap or a plain backward step - is taken.
fn ms_time_diff(old_ms: u32, now_ms: u32) -> u32 {
    if old_ms > now_ms {
        let wrapped = (0xFFFF_FFFF - old_ms).wrapping_add(now_ms);
        let backward = old_ms - now_ms;
        wrapped.min(backward)
    } else {
        now_ms - old_ms
    }
}

/// Milliseconds since the transport was created (`GenericTransport::GetTimeSinceCreation`).
///
/// `creation_ms` is the server clock when the transport spawned and `now_ms` the current
/// clock; both are the wrapping 32-bit millisecond timer.
pub fn time_since_creation(creation_ms: u32, now_ms: u32) -> u32 {
    ms_time_diff(creation_ms, now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_is_the_forward_difference() {
        assert_eq!(time_since_creation(1_000, 4_500), 3_500);
        assert_eq!(time_since_creation(0, 0), 0);
    }

    #[test]
    fn a_wrapped_clock_reads_the_short_way_around() {
        // Created just before the u32 wrap, now just after: a few ms elapsed, not ~4 billion.
        // The C++ wrap formula ((0xFFFFFFFF - old) + new) is one short of the true modular
        // distance (which would be 3 here); reproducing that off-by-one keeps us faithful.
        assert_eq!(time_since_creation(0xFFFF_FFFE, 1), 2);
    }

    #[test]
    fn a_small_backward_step_is_preferred_over_a_full_wrap() {
        // now one tick behind old: the backward interpretation (1) beats the wrap.
        assert_eq!(time_since_creation(5_000, 4_999), 1);
    }
}
