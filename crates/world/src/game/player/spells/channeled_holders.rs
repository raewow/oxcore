//! Channeled aura-holder list maintenance (MaNGOS `Spell::RemoveChanneledAuraHolder`).
//!
//! A channeled spell tracks the per-target aura holders it applied so they can be
//! torn down atomically when the channel ends or is interrupted. While
//! `Spell::update` walks the holders list (advancing a "current element" cursor),
//! `RemoveChanneledAuraHolder` may be re-entered from the aura system to delete a
//! holder that expired externally. The danger is iterator invalidation: if the
//! removed holder is the one the update loop is currently visiting, the update
//! cursor must advance via `erase` (its Rust analogue: keep the same index, which
//! now points at the next element, or become `None` if the tail was removed).
//!
//! The list and cursor are not part of the existing `ActiveCast` state in this
//! crate yet; they live here as a small self-contained struct so the removal logic
//! can be ported and unit-tested world-free.

/// Aura-holder identifier matching the aura container's keying: `(spell_id,
/// effect_index)`. Replaces the C++ `SpellAuraHolder*` pointer used for equality
/// in `std::find`.
pub type AuraHolderId = (u32, u8);

/// The subset of the MaNGOS `AuraRemoveMode` enum that gates
/// `RemoveChanneledAuraHolder`. The three skip modes are handled elsewhere
/// (`Spell::update` or this spell's own channel cleanup) and must not double-erase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraRemoveMode {
    /// `AURA_REMOVE_BY_CHANNEL` — originated from this spell's own channel cleanup.
    ByChannel,
    /// `AURA_REMOVE_BY_GROUP` — handled in `Spell::update`.
    ByGroup,
    /// `AURA_REMOVE_BY_RANGE` — handled in `Spell::update`.
    ByRange,
    /// Any other removal cause; proceeds normally.
    Default,
}

/// `true` for the three removal modes that `RemoveChanneledAuraHolder` must skip.
/// `Default` (and any other non-listed mode) proceeds with the removal.
pub fn should_skip_removal(mode: AuraRemoveMode) -> bool {
    matches!(
        mode,
        AuraRemoveMode::ByChannel | AuraRemoveMode::ByGroup | AuraRemoveMode::ByRange
    )
}

/// Combines the three skip-mode filters and the C++ `!holder` null guard into a
/// single pure proceed/skip decision. Returns `Some(id)` when the removal should
/// go ahead, `None` when it must early-return (skip mode or null holder).
///
/// Kept separate so the full early-return table is unit-testable.
pub fn gate_removal(holder_id: Option<AuraHolderId>, mode: AuraRemoveMode) -> Option<AuraHolderId> {
    if should_skip_removal(mode) {
        return None;
    }
    holder_id
}

/// Outcome of a single channeled-holder removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// The holder was not in the list (genuine not-found, or the removal was
    /// skipped by a mode/null gate).
    NotFound,
    /// The holder was removed. `advanced_cursor` is `true` when the update
    /// cursor was the removed element and advanced via `erase` (the C++
    /// `iter == m_channeledUpdateIterator` branch); `false` when the cursor was
    /// untouched or merely shifted left to stay on the same element.
    Removed { advanced_cursor: bool },
}

/// Generic holders list with no cursor — the world-free substrate the pure
/// removal helper operates on. `H` is the holder-identity type (pointer in C++,
/// `AuraHolderId` here).
#[derive(Debug, Clone, Default)]
pub struct ChanneledHoldersList<H> {
    /// Holder identities in insertion order.
    pub holders: Vec<H>,
}

impl<H> ChanneledHoldersList<H> {
    /// Create an empty list.
    pub fn new() -> Self {
        Self {
            holders: Vec::new(),
        }
    }

    /// Push a holder identity onto the tail (for `AddChanneledAuraHolder`).
    pub fn push(&mut self, holder: H) {
        self.holders.push(holder);
    }

    /// Number of tracked holders.
    pub fn len(&self) -> usize {
        self.holders.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.holders.is_empty()
    }

    /// Iterate over holder identities.
    pub fn iter(&self) -> std::slice::Iter<'_, H> {
        self.holders.iter()
    }
}

impl<H: PartialEq + Clone> ChanneledHoldersList<H> {
    /// Remove a holder by equality (`std::find` analogue), advancing the update
    /// cursor the way C++ `erase` does when the cursor was the removed element.
    ///
    /// Cursor semantics, indexed to mirror `std::list::iterator`:
    /// - cursor `Some(c) == idx` (pointing at the removed element): after
    ///   `remove(idx)`, if elements remain at/after `idx` the cursor stays at
    ///   `idx` (now the next element — the `erase`-return analogue); if `idx` is
    ///   past the new tail (removed last element) the cursor becomes `None`
    ///   (`end()`). `advanced_cursor = true`.
    /// - cursor `Some(c) > idx` (pointing past the removed element): indices
    ///   shift left by one, so the cursor decrements to `c - 1` to keep pointing
    ///   at the same element. `advanced_cursor = false`.
    /// - cursor `Some(c) < idx` or `None`: untouched. `advanced_cursor = false`.
    pub fn remove(&mut self, holder: &H, update_cursor: &mut Option<usize>) -> RemoveOutcome {
        let Some(idx) = self.holders.iter().position(|h| h == holder) else {
            return RemoveOutcome::NotFound;
        };
        self.holders.remove(idx);
        let advanced;
        match *update_cursor {
            Some(c) if c == idx => {
                // erase returned iterator → next element, or end() if tail removed.
                if idx < self.holders.len() {
                    *update_cursor = Some(idx);
                } else {
                    *update_cursor = None;
                }
                advanced = true;
            }
            Some(c) if c > idx => {
                // an element before the cursor was removed → shift left.
                *update_cursor = Some(c - 1);
                advanced = false;
            }
            _ => {
                // cursor before the removed element, or no cursor → unchanged.
                advanced = false;
            }
        }
        RemoveOutcome::Removed {
            advanced_cursor: advanced,
        }
    }
}

/// Concrete channeled-holders state: the holder list plus the
/// `m_channeledUpdateIterator` cursor (as an index, since a borrowed cursor
/// cannot live alongside mutation of the backing `Vec`).
#[derive(Debug, Clone, Default)]
pub struct ChanneledHolders {
    /// Tracked holder identities (`m_channeledHolders`).
    pub holders: Vec<AuraHolderId>,
    /// Index of the holder the update loop is currently visiting
    /// (`m_channeledUpdateIterator`); `None` means not iterating / `end()`.
    pub update_index: Option<usize>,
}

impl ChanneledHolders {
    /// Create empty channeled-holders state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a channeled aura holder (superseded by [`add_channeled_aura_holder`]).
    pub fn add(&mut self, holder: AuraHolderId) {
        self.holders.push(holder);
    }

    /// `Spell::AddChanneledAuraHolder` — adds a channeled aura holder with the
    /// null/channeled guard. Does not perform `SetInUse(true)` (see the module-level
    /// function for details).
    pub fn add_guarded(&mut self, holder_id: AuraHolderId, is_channeled: bool) -> bool {
        if holder_id.0 == 0 || !is_channeled {
            return false;
        }
        self.holders.push(holder_id);
        true
    }

    /// Remove a channeled aura holder, faithfully replicating the
    /// cursor-invalidation branch. Pure (world-free).
    pub fn remove(&mut self, holder: &AuraHolderId) -> RemoveOutcome {
        let mut list = ChanneledHoldersList {
            holders: std::mem::take(&mut self.holders),
        };
        let outcome = list.remove(holder, &mut self.update_index);
        self.holders = list.holders;
        outcome
    }
}

/// `Spell::AddChanneledAuraHolder` — adds a channeled aura holder to the list.
///
/// Returns `false` if the holder was skipped (null spell_id or not channeled),
/// `true` if it was added to the list.
///
/// The C++ `SetInUse(true)` call on the holder is omitted because Rust's borrow
/// checker prevents the delete-during-iteration problem that `SetInUse` protects
/// against in C++ (see also `RemoveChanneledAuraHolder`).
pub fn add_channeled_aura_holder(
    holders: &mut ChanneledHolders,
    holder_id: Option<AuraHolderId>,
    is_channeled: bool,
) -> bool {
    let Some(id) = holder_id else { return false; };
    holders.add_guarded(id, is_channeled)
}

/// Isolated port of `Spell::RemoveChanneledAuraHolder`.
///
/// Skips the three mode filters and the null-holder guard via [`gate_removal`],
/// then runs the cursor-aware removal.
///
/// `ActiveCast` does not yet own channeled-holder state and Rust aura holders do
/// not track their C++ `in_use` flag, so this is deliberately not wired into the
/// live spell/aura path and cannot perform `SetInUse(false)`.
pub fn remove_channeled_aura_holder(
    holders: &mut ChanneledHolders,
    holder_id: Option<AuraHolderId>,
    mode: AuraRemoveMode,
) -> RemoveOutcome {
    let Some(holder) = gate_removal(holder_id, mode) else {
        return RemoveOutcome::NotFound;
    };

    holders.remove(&holder)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids<const N: usize>(spell_ids: [u32; N]) -> Vec<AuraHolderId> {
        spell_ids.into_iter().map(|sid| (sid, 0)).collect()
    }

    // --- should_skip_removal: the three skip-mode early-returns ----------------

    #[test]
    fn skip_modes_filter_by_channel_group_range_only() {
        assert!(should_skip_removal(AuraRemoveMode::ByChannel));
        assert!(should_skip_removal(AuraRemoveMode::ByGroup));
        assert!(should_skip_removal(AuraRemoveMode::ByRange));
        assert!(!should_skip_removal(AuraRemoveMode::Default));
    }

    // --- gate_removal: null holder + skip modes → skip (pure, no DB) -----------

    #[test]
    fn gate_skips_null_holder_and_skip_modes_proceeds_default() {
        // null holder (the C++ `!holder` early-return) → skip.
        assert_eq!(gate_removal(None, AuraRemoveMode::Default), None);
        // skip modes → skip even with a real holder.
        assert_eq!(
            gate_removal(Some((100, 0)), AuraRemoveMode::ByChannel),
            None
        );
        assert_eq!(gate_removal(Some((100, 0)), AuraRemoveMode::ByGroup), None);
        assert_eq!(gate_removal(Some((100, 0)), AuraRemoveMode::ByRange), None);
        // real holder + Default → proceed with the id.
        assert_eq!(
            gate_removal(Some((100, 0)), AuraRemoveMode::Default),
            Some((100, 0))
        );
    }

    #[test]
    fn entry_point_skips_null_and_skip_modes_without_mutating_the_list() {
        let mut holders = ChanneledHolders {
            holders: ids([100, 200]),
            update_index: Some(0),
        };

        for (holder, mode) in [
            (None, AuraRemoveMode::Default),
            (Some((100, 0)), AuraRemoveMode::ByChannel),
            (Some((100, 0)), AuraRemoveMode::ByGroup),
            (Some((100, 0)), AuraRemoveMode::ByRange),
        ] {
            assert_eq!(
                remove_channeled_aura_holder(&mut holders, holder, mode),
                RemoveOutcome::NotFound
            );
            assert_eq!(holders.holders, ids([100, 200]));
            assert_eq!(holders.update_index, Some(0));
        }
    }

    #[test]
    fn entry_point_removes_default_mode_holder_and_updates_cursor() {
        let mut holders = ChanneledHolders {
            holders: ids([100, 200, 300]),
            update_index: Some(1),
        };

        let outcome =
            remove_channeled_aura_holder(&mut holders, Some((200, 0)), AuraRemoveMode::Default);

        assert_eq!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: true
            }
        );
        assert_eq!(holders.holders, ids([100, 300]));
        assert_eq!(holders.update_index, Some(1));
    }

    // --- not-found / null no-op ------------------------------------------------

    #[test]
    fn not_found_is_noop() {
        let mut list = ChanneledHoldersList::new();
        list.push((100, 0));
        list.push((200, 0));
        let mut cursor: Option<usize> = Some(0);

        let outcome = list.remove(&(300, 0), &mut cursor);

        assert_eq!(outcome, RemoveOutcome::NotFound);
        assert_eq!(list.holders, ids([100, 200]));
        assert_eq!(cursor, Some(0)); // untouched
    }

    // --- remove a non-cursor element → cursor unchanged ------------------------

    #[test]
    fn remove_non_cursor_element_leaves_cursor_unchanged() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        let mut cursor = Some(1); // visiting (20,0)

        let outcome = list.remove(&(30, 0), &mut cursor); // remove at idx 2, after cursor

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: false,
            }
        ));
        assert_eq!(list.holders, ids([10, 20]));
        assert_eq!(cursor, Some(1)); // still (20,0), untouched
    }

    // --- remove the cursor element → cursor advances to next index -------------

    #[test]
    fn remove_cursor_element_advances_to_next_index() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        let mut cursor = Some(1); // visiting (20,0)

        let outcome = list.remove(&(20, 0), &mut cursor);

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: true,
            }
        ));
        assert_eq!(list.holders, ids([10, 30]));
        assert_eq!(cursor, Some(1)); // idx 1 now holds (30,0) — the next element
    }

    // --- remove the cursor when it is the last element → cursor becomes None ----

    #[test]
    fn remove_cursor_last_element_sets_cursor_none() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        let mut cursor = Some(1); // visiting (20,0), the tail

        let outcome = list.remove(&(20, 0), &mut cursor);

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: true,
            }
        ));
        assert_eq!(list.holders, ids([10]));
        assert_eq!(cursor, None); // erase returned end()
    }

    // --- remove the cursor when it is the first element with followers ----------

    #[test]
    fn remove_cursor_first_element_with_followers_stays_at_zero() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        let mut cursor = Some(0); // visiting the head

        let outcome = list.remove(&(10, 0), &mut cursor);

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: true,
            }
        ));
        assert_eq!(list.holders, ids([20]));
        assert_eq!(cursor, Some(0)); // idx 0 now holds (20,0)
    }

    // --- remove an element before the cursor → cursor shifts left by one ------

    #[test]
    fn remove_element_before_cursor_shifts_cursor_left() {
        let mut list = ChanneledHoldersList::new();
        let mut cursor = Some(2); // visiting (30,0)

        // build [10,20,30]
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        // remove the element at idx 0, before the cursor

        let outcome = list.remove(&(10, 0), &mut cursor);

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: false,
            }
        ));
        assert_eq!(list.holders, ids([20, 30]));
        assert_eq!(cursor, Some(1)); // shifted left by one; still points at (30,0)
    }

    // --- remove an element before the cursor with cursor at the tail -----------

    #[test]
    fn remove_element_before_cursor_at_tail_keeps_tail_operand() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        list.push((40, 0));
        let mut cursor = Some(3); // visiting (40,0), the tail

        let outcome = list.remove(&(20, 0), &mut cursor); // remove at idx 1

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: false,
            }
        ));
        assert_eq!(list.holders, ids([10, 30, 40]));
        assert_eq!(cursor, Some(2)); // shifted left by one; still points at (40,0)
    }

    // --- multiple auras removed in one update pass (iterator-stability chain) ---

    #[test]
    fn multiple_removals_in_one_pass_preserve_cursor_target() {
        // Simulate an update pass that removes two earlier holders while the
        // cursor sits on a later one; the cursor must keep pointing at the same
        // holder identity across both removals.
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        list.push((40, 0));
        list.push((50, 0));
        let mut cursor = Some(3); // visiting (40,0)

        list.remove(&(10, 0), &mut cursor); // cursor → 2, still (40,0)
        assert_eq!(cursor, Some(2));
        list.remove(&(20, 0), &mut cursor); // cursor → 1, still (40,0)
        assert_eq!(cursor, Some(1));

        assert_eq!(list.holders, ids([30, 40, 50]));
        assert_eq!(cursor, Some(1)); // pointing at (40,0)
    }

    // --- remove the cursor element, then the new cursor element ----------------

    #[test]
    fn remove_cursor_then_new_cursor_advances_again() {
        let mut list = ChanneledHoldersList::new();
        list.push((10, 0));
        list.push((20, 0));
        list.push((30, 0));
        let mut cursor = Some(1); // visiting (20,0)

        list.remove(&(20, 0), &mut cursor); // cursor stays 1, now (30,0)
        assert_eq!(cursor, Some(1));
        list.remove(&(30, 0), &mut cursor); // cursor was 1 (tail of ([10,30])? now (10,))
                                            // after first remove list is [10,30]; cursor 1 → (30,0); removing (30,0)
                                            // at idx 1 == cursor 1 → tail removed → None.
        assert_eq!(cursor, None);
        assert_eq!(list.holders, ids([10]));
    }

    // --- ChanneledHolders (concrete) wraps the pure helper ---------------------

    #[test]
    fn channeled_holders_remove_routes_through_pure_helper() {
        let mut state = ChanneledHolders::new();
        state.add((10, 0));
        state.add((20, 0));
        state.add((30, 0));
        state.update_index = Some(1);

        let outcome = state.remove(&(20, 0));

        assert!(matches!(
            outcome,
            RemoveOutcome::Removed {
                advanced_cursor: true,
            }
        ));
        assert_eq!(state.holders, ids([10, 30]));
        assert_eq!(state.update_index, Some(1));
    }

    // --- empty-list / single-element removal sanity ----------------------------

    #[test]
    fn remove_from_empty_list_is_not_found() {
        let mut list = ChanneledHoldersList::<AuraHolderId>::new();
        let mut cursor: Option<usize> = None;

        let outcome = list.remove(&(10, 0), &mut cursor);

        assert_eq!(outcome, RemoveOutcome::NotFound);
        assert!(list.is_empty());
        assert_eq!(cursor, None);
    }

    // ── add_channeled_aura_holder (Spell::AddChanneledAuraHolder) ───

    #[test]
    fn add_null_holder_is_noop() {
        let mut holders = ChanneledHolders::new();
        assert!(!add_channeled_aura_holder(&mut holders, None, true));
        assert!(holders.holders.is_empty());
    }

    #[test]
    fn add_non_channeled_holder_is_noop() {
        let mut holders = ChanneledHolders::new();
        assert!(!add_channeled_aura_holder(&mut holders, Some((100, 0)), false));
        assert!(holders.holders.is_empty());
    }

    #[test]
    fn add_channeled_holder_appends_to_list() {
        let mut holders = ChanneledHolders::new();
        assert!(add_channeled_aura_holder(&mut holders, Some((100, 0)), true));
        assert_eq!(holders.holders, ids([100]));
    }

    #[test]
    fn add_multiple_channeled_holders_appends_in_order() {
        let mut holders = ChanneledHolders::new();
        assert!(add_channeled_aura_holder(&mut holders, Some((100, 0)), true));
        assert!(add_channeled_aura_holder(&mut holders, Some((200, 1)), true));
        assert!(add_channeled_aura_holder(&mut holders, Some((300, 2)), true));
        assert_eq!(
            holders.holders,
            vec![(100, 0), (200, 1), (300, 2)]
        );
    }
}
