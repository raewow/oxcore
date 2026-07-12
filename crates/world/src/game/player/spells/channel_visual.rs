//! Channeled spell visual kit setup (MaNGOS `Spell::InitializeChanneledVisualTimer`).
//!
//! Reads a spell's `custom` flags and `spell_visual` id and, when all three guards
//! pass, produces the per-cast `ChannelVisualConfig` that the cast state would have
//! stored as `m_channeledVisualKit` / `m_channeledVisualTimer`. The decision logic is
//! split into a pure, DB-free helper so it can be unit-tested directly.

use oxcore_dbc::structures::spell::SpellEntry;

/// Custom-flag bitmask: when set, a channeling spell periodically re-sends its
/// channel visual kit while channeling (MaNGOS `SPELL_CUSTOM_SEND_CHANNEL_VISUAL`,
/// reference/core SpellDefines.h:975).
pub const SPELL_CUSTOM_SEND_CHANNEL_VISUAL: u32 = 0x800;

/// Fixed refresh interval (ms) for re-sending the channeled visual kit while it
/// is active (MaNGOS `SPELL_CHANNEL_VISUAL_TIMER`, reference/core Spell.cpp:55).
pub const SPELL_CHANNEL_VISUAL_TIMER: u32 = 800;

/// Per-cast output of `InitializeChanneledVisualTimer`: the channel kit id to play
/// and the refresh interval at which to re-send it. Carried out of the cast state
/// (which does not yet own `channeled_visual_kit`/`channeled_visual_timer` fields)
/// so the caller can apply it to its own channeled-spell representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelVisualConfig {
    /// `m_channeledVisualKit` — the `SpellVisual.dbc` channelKit id to play.
    pub kit: u32,
    /// `m_channeledVisualTimer` — always `SPELL_CHANNEL_VISUAL_TIMER` on success.
    pub timer_ms: u32,
}

/// Pure decision logic for `Spell::InitializeChanneledVisualTimer` — no DB / DBC
/// access, fully unit-testable.
///
/// Returns `Some(ChannelVisualConfig { kit, timer_ms: SPELL_CHANNEL_VISUAL_TIMER })`
/// only when **all** of:
/// - `custom_flags` has `SPELL_CUSTOM_SEND_CHANNEL_VISUAL` set,
/// - `spell_visual` is non-zero,
/// - `lookup(spell_visual)` returns `Some(channel_kit)` with `channel_kit != 0`.
///
/// Otherwise returns `None`, matching the three early-return guards in the reference
/// implementation (no assignment to the channel visual kit/timer).
pub fn compute_channel_visual(
    custom_flags: u32,
    spell_visual: u32,
    lookup: impl Fn(u32) -> Option<u32>,
) -> Option<ChannelVisualConfig> {
    if custom_flags & SPELL_CUSTOM_SEND_CHANNEL_VISUAL == 0 {
        return None;
    }
    if spell_visual == 0 {
        return None;
    }
    let channel_kit = lookup(spell_visual)?;
    if channel_kit == 0 {
        return None;
    }
    Some(ChannelVisualConfig {
        kit: channel_kit,
        timer_ms: SPELL_CHANNEL_VISUAL_TIMER,
    })
}

/// World-coupled entry mirroring `Spell::InitializeChanneledVisualTimer`: reads the
/// `custom` and `spell_visual` fields from a spell entry and resolves the channel kit
/// via a `SpellVisual.dbc` channelKit lookup.
///
/// `lookup` yields the `channelKit` column of the `SpellVisual.dbc` entry for a
/// `spell_visual` id (i.e. `pSpellVisual->channelKit`), or `None` when the entry is
/// missing — which the reference code treats identically to a zero `channelKit`.
///
/// // TODO: wire to the real `SpellVisual` DBC store once it exists; until then the
/// // caller supplies the lookup.
pub fn initialize_channeled_visual_timer(
    spell: &SpellEntry,
    lookup: impl Fn(u32) -> Option<u32>,
) -> Option<ChannelVisualConfig> {
    compute_channel_visual(spell.custom, spell.spell_visual, lookup)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAG: u32 = SPELL_CUSTOM_SEND_CHANNEL_VISUAL;

    #[test]
    fn missing_custom_flag_returns_none() {
        assert_eq!(compute_channel_visual(0, 100, |_| Some(7)), None);
        assert_eq!(
            compute_channel_visual(0x0000_0400, 100, |_| Some(7)),
            None,
            "without the SEND_CHANNEL_VISUAL bit set there is nothing to init"
        );
    }

    #[test]
    fn zero_spell_visual_returns_none() {
        assert_eq!(compute_channel_visual(FLAG, 0, |_| Some(7)), None);
    }

    #[test]
    fn missing_spell_visual_entry_returns_none() {
        assert_eq!(compute_channel_visual(FLAG, 100, |_| None), None);
    }

    #[test]
    fn zero_channel_kit_returns_none() {
        assert_eq!(compute_channel_visual(FLAG, 100, |_| Some(0)), None);
    }

    #[test]
    fn all_guards_pass_returns_config() {
        let cfg = compute_channel_visual(FLAG, 100, |_| Some(7));
        assert_eq!(
            cfg,
            Some(ChannelVisualConfig {
                kit: 7,
                timer_ms: SPELL_CHANNEL_VISUAL_TIMER,
            })
        );
    }

    #[test]
    fn lookup_is_not_consulted_when_visual_is_zero() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let cfg = compute_channel_visual(FLAG, 0, |id| {
            calls.set(calls.get() + 1);
            Some(id)
        });
        assert_eq!(calls.get(), 0, "second guard must short-circuit before the lookup");
        assert_eq!(cfg, None);
    }

    #[test]
    fn lookup_is_not_consulted_when_custom_flag_missing() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let cfg = compute_channel_visual(0, 100, |id| {
            calls.set(calls.get() + 1);
            Some(id)
        });
        assert_eq!(calls.get(), 0, "first guard must short-circuit before the lookup");
        assert_eq!(cfg, None);
    }
}