# Plan: spell_runtime_remediation

## Goal

Bring the live Rust spell runtime from partially connected ports to behaviourally
complete, testable flows. This plan covers runtime integration gaps, supporting
world primitives, and the remaining spell port-harness backlog. It is a tracking
document, not evidence that a task is complete.

## Baseline

- Harness feature `spells`: 587 tasks; 352 done, 61 rust_compiled, 43 blocked,
  63 discovered, 42 verified, and 26 reviewed.
- Feature call-graph coverage: 42 percent; 1,942 callees are unindexed.
- `cargo check -p oxcore-world --all-targets` has no spell dead-code warning.
- Most gaps are incomplete runtime connections rather than Rust-unreachable code.

## Status Legend

| Status | Meaning |
| --- | --- |
| Not started | No implementation work has begun. |
| Investigating | Dependencies and C++ behaviour are being mapped. |
| Ready | Dependencies, Rust module, symbols, and acceptance tests are known. |
| Implementing | A single module-sized batch is in progress. |
| Verified | Focused tests and behaviour review are complete. |
| Blocked | A named missing primitive prevents faithful implementation. |

## Rules

- Work one Rust module-sized batch at a time.
- Do not mark a port complete merely because it compiles or has unit tests for a
  pure helper. It must be reachable from the live cast, aura, or update path.
- Add state and world primitives before effect handlers that depend on them.
- Keep critical flows at `reviewed` until scenario tests cover both success and
  cleanup paths.
- Update the batch log below after each completed implementation batch.
- Update port-harness task status only after the listed verification command is
  green and the task has runtime evidence.

## Dependency Order

```text
Unit/aura state + map queries
    -> spell data + targeting/validation
    -> hit resolution + holder lifecycle
    -> cast/channel packets + periodic/proc execution
    -> dynamic objects + area auras
    -> effect families + persistence/handlers + AI/scripts
```

## Phase Tracker

| Phase | Scope | Primary Rust modules | Harness flows/tasks | Status | Exit gate |
| --- | --- | --- | --- | --- | --- |
| 0 | Classify all spell gaps and establish acceptance scenarios | `tools/port-harness/docs/plans/` | All blocked and rust_compiled tasks | Verified | Every gap has a dependency class, target batch, and minimal scenario. |
| 1 | Shared player/creature spell and aura runtime state | `game/player/auras/*`, `game/creature/*`, `game/player/spells/state.rs` | Aura holder lifecycle | Verified | Both target kinds retain full aura metadata and support common core storage, query, refresh, type removal, and expiry operations. |
| 2 | Spell data and world spatial/object queries | spell manager, DBC manager, map/world services | `spell_data`, target selection | Implementing | Targeting has real data, radius/cone, location, and object lookups. |
| 3 | Target validation, hit, immunity, combat, and DR | `spells/targets.rs`, `validation.rs`, `hit.rs`, `target_info.rs`, `diminishing.rs` | `spell_target_selection_and_registration`, effect application | Implementing | Scenario tests cover reject, immune, evade, miss, reflect, and hit. |
| 4 | Aura-holder lifecycle, stacks, and channel ownership | `auras/*`, `spells/state.rs`, `channeled_holders.rs` | Aura holder methods | Not started | No stale holder survives cancel, expiry, target loss, or channel end. |
| 5 | Cast pushback, channel lifecycle, and client packets | `spells/system.rs`, `delayed.rs`, `channel_visual.rs` | `spell_packet_and_channel_io` | Not started | Start, go, delayed, channel update, and interrupt packets match state. |
| 6 | Periodic auras and proc pipeline | `auras/periodic.rs`, `auras/proc.rs` | `aura_periodic_effect_execution`, `aura_proc_event_dispatch`, proc handlers | Not started | Direct and periodic effects share damage/heal/proc behaviour. |
| 7 | Dynamic objects and area aura propagation | dynamic-object manager, `effects/aura.rs`, area targeting | Persistent/area aura symbols | Not started | Enter, leave, movement, map change, and expiry are covered. |
| 8 | Effect families | `spells/effects/*` | Effect symbols in `SpellEffects.cpp` | Not started | Every dispatched effect is implemented, rejected, or explicitly unsupported. |
| 9 | Persistence and client spell handlers | cooldowns, learning, handlers, repositories | `spell_handler_client_requests` | Not started | Login/logout and client cancellation scenarios pass. |
| 10 | Creature AI and script integration | creature AI, script manager, spell system | AI reaction and script targeting flows | Not started | AI and scripts use the same live spell pipeline. |

## Gap Register

| ID | Gap | Current location | Dependency | Planned phase | Status |
| --- | --- | --- | --- | --- | --- |
| G-001 | Creature aura storage is simplified and cannot support full holder lifecycle | `game/creature/creature.rs` | Shared aura model | 1 | Verified |
| G-002 | Unit operations are split across player and creature code with incomplete parity | player/creature managers | Shared spell target surface | 1 | Not started |
| G-003 | Script targets, spell areas, chains, learn-spells, cones, and internal flags are incomplete | spell manager | Database and DBC loading | 2 | Implementing |
| G-004 | Radius, cone, visibility, line-of-sight, corpse, game-object, and dynamic-object queries are incomplete | map/world services | Spatial query surface | 2 | Implementing |
| G-005 | Faction is used as a polarity proxy; core active-aura effect/state/mechanic/school immunity, per-effect mechanic resistance, and spell reflection are live | `spells/target_info.rs`, `hit.rs` | Unit and target queries | 3 | Implementing |
| G-006 | DR snapshots and hostile-action side effects are live at spell hit; visibility gating and PvP flagging remain blocked | `spells/target_info.rs` | `IsVisibleForOrDetect`, `UpdatePvP` | 3-4 | Blocked |
| G-007 | Channel holder list has no live owner or callers | `spells/channeled_holders.rs` | Active-cast holder state | 4 | Not started |
| G-008 | Holder stacks, real caster, and visible-slot decisions are blocked | aura container/system | Holder model | 4 | Not started |
| G-009 | Delayed pushback omits `SMSG_SPELL_DELAYED` | `spells/delayed.rs` | Packet definitions | 5 | Ready |
| G-010 | Channel pushback does not delay holders/dynamic objects, update client, or interrupt at zero | `spells/delayed.rs` | G-007, dynamic objects, packet path | 5, 7 | Blocked |
| G-011 | Channel visual configuration is calculated but not persisted or scheduled | `spells/channel_visual.rs`, `spells/system.rs` | Active-cast state and SpellVisual data | 5 | Blocked |
| G-012 | Periodic effects bypass portions of the direct damage/heal and proc pipeline | `auras/periodic.rs` | G-003, G-005, proc dispatcher | 6 | Not started |
| G-013 | Proc eligibility and many proc handlers are unported | `auras/proc.rs` | Aura holder and spell trigger context | 6 | Not started |
| G-014 | Persistent and area auras use placeholders instead of dynamic objects and live target tracking | `spells/effects/aura.rs` | G-001, G-004 | 7 | Blocked |
| G-015 | Multiple effect families log or no-op because their owning systems are absent | `spells/effects/*` | Varies by family | 8 | Not started |
| G-016 | Cooldowns save as a no-op and never load on login | `spells/cooldowns.rs`, `spells/system.rs` | Character repository | 9 | Ready |
| G-017 | Pet-cancel and auto-repeat-cancel client handlers are blocked | spell handlers | Pet and cast lifecycle | 9 | Blocked |
| G-018 | Creature spell-list and spell-hit reaction paths are blocked | creature AI | Spell result callbacks | 10 | Blocked |

## Phase 0 Detailed Inventory

The table below is the detailed Phase 0 classification. File paths are relative to
`crates/world/src`. `Live` means the code is reached from a production cast, aura,
or update path; it does not mean the behavior is complete.

### Core Runtime and Targeting

| Source | Gap | Class | Live | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- | --- |
| `game/player/spells/state.rs:405-435` | Dynamic-object lookup/removal ignores spell/effect identity. | World query | No | Dynamic-object registry metadata. | Two objects with different spell/effect pairs remain independently addressable. |
| `game/player/spells/caster.rs:214,836-838,999,1021` | Script coefficients, totem owner bonuses, pet happiness, and pet bonus damage are absent. | Owning subsystem | Yes | Scripts, ownership, pet stats. | Scripted, totem, and pet casts use their respective modifiers. |
| `game/player/spells/hit.rs:413,490` | Melee/ranged spell hit is automatic; creature school resistance is zero. | Missing state | Yes | Combat table and creature resistance data. | Melee miss and partial fire resistance are possible. |
| `game/player/spells/area_targets.rs:238,314` | Area relationship selection uses object kind rather than faction. | World query | Yes | Faction and reputation resolver. | Friendly AoE excludes hostile units of the same object kind. |
| `game/player/spells/targets.rs:331,809,1314-1323,1349,1407` | Missing ignore-restriction attribute, faction hostility, rank selection, charm ownership, AoE immunity, and script target hooks. | Data/query | Yes | Spell attributes, faction/charm state, script hooks. | Rank-scaled aura selects correct rank; immune/script-rejected unit is excluded. |
| `game/player/spells/target_info.rs:203-324,390,456,484,566` | Real friendliness/visibility, hostile-action interrupt, PvP/DR, creature aura removal, and assist threat are incomplete. Per-effect resistance, immunity, and reflected recast are live. | Lifecycle hook | Yes | Unit query surface, creature auras, PvP/DR/combat APIs. | Delayed hostile cast is immune/evaded/reflected correctly and applies DR/combat effects. |
| `game/player/spells/modifiers.rs:51` | School power-cost aura fields are not represented. | Missing state | Yes | Aura-derived unit modifiers. | Cost-reduction aura changes school spell mana cost. |
| `game/player/spells/threat_bonus.rs:211-216` | Inverse effect mask is manually threaded because loaded spell-threat data is absent. | Persistent data | Yes | `SpellThreatEntry` loader and getter. | Threat applies only to effects allowed by inverse mask. |
| `game/player/spells/validation.rs:1371-1375` | Loatheb and battleground open-lock/banner validation are disconnected. | Lifecycle hook | Yes | Encounter script and battleground-object state. | Restricted heal/dispel/banner casts fail in the relevant encounter state. |

### Cast, Channel, and Holder Lifecycle

| Source | Gap | Class | Live | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- | --- |
| `game/player/spells/delayed.rs:27,198,279,427` | Pushback count ownership is weak; delayed packet, holder/dynamic-object delay, channel update, and zero-timer interrupt are missing. | State/packet/lifecycle | Yes | Active-cast state, packet serializer, holder and dynamic-object delay APIs. | Damage during cast/channel updates every affected timer and interrupts at zero. |
| `game/player/spells/channel_visual.rs:22-23` | Channel visual kit/timer cannot be stored or scheduled. | State/packet | No | ActiveCast fields and SpellVisual data. | Channel begins and refreshes its visual kit at the configured interval. |
| `game/player/spells/channeled_holders.rs:224-226` | Channeled holder helpers have no active-cast owner or production caller. | Lifecycle hook | No | ActiveCast-owned holder list. | Channel end removes only holders owned by that cast. |
| `game/player/spells/system.rs:336,427,980,1900,2775,2897-2911,3030,3346` | Faction, charge movement, channel visual resend, finisher miss behavior, haste GCD, passive/talent application, chain cap config, and permanent cooldowns are incomplete. | Mixed state/lifecycle | Yes | Faction, movement, visual, miss, haste, passive, config, and cooldown state. | Hasted GCD, missed-finisher combo retention, passive talent aura, and permanent cooldown checks work. |
| `game/player/auras/system.rs:88,390` | Creature AoE charm is explicitly unsupported. | Intentional unsupported | Yes | Creature charm/control system. | Creature charm grants and removes control correctly. |
| `game/player/auras/system.rs:2734-2736,3080,3338,3551-3553` | Shapeshift cleanup, invisibility visibility, stacked creature snare recalculation, resistance UI, and Faerie Fire side effect are incomplete. | Lifecycle/query/packet | Yes | Cross-aura removal, visibility, speed aggregation, client stat update, dispel immunity. | Form change cleans conflicts; stacked snares recalculate; invisibility and resistance are observable. |

### Data, Learning, and Persistence

| Source | Gap | Class | Live | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- | --- |
| `game/player/spells/learning.rs:125,193-196,225,244,259` | Dependent/rank learning, initial cooldown metadata, level auto-learn, and spell persistence are incomplete. | Persistent data | Yes | DBC relations and `character_spells` repository. | Learn rank/dependent spells and retain them across relog. |
| `game/player/spells/cooldowns.rs:283,293` | Cooldown save/load are no-ops; login never loads saved cooldowns. | Persistent data | Save only | Character cooldown repository and login hook. | Relog preserves a long cooldown with reduced remaining time. |
| `game/player/auras/persistence.rs:97-99` | Group aura-status synchronization is deferred. | Owning subsystem | Yes | Group login/status hook. | Group members receive updated status after aura-bearing player login. |
| Spell manager backlog | Spell chains, learn-spells, script targets, cones, areas, location rules, skill maps, and internal flags are discovered but not ported. | Persistent data | Partial | Database loaders and DBC mappings. | Data-backed target, location, rank, and learning decisions all pass. |

### Periodic and Proc Execution

| Source | Gap | Class | Live | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- | --- |
| `game/player/auras/periodic.rs:156` | Periodic aura log hardcodes spell school to zero. | Packet | Yes | School propagation in aura snapshot. | Fire DoT packet reports fire school. |
| `game/player/auras/proc.rs:528,559,569` | Trigger damage, Sweeping Strikes, and Retaliation log instead of performing combat effects. | Owning subsystem | Yes | Damage API and nearby-hostile query. | Proc damages attacker or a second eligible target. |
| `game/player/auras/periodic.rs` and port task `Aura::PeriodicTick` | Periodic damage/heal/leech/power execution lacks full direct-cast immunity, absorb, resist, crit, threat, and proc parity. | Lifecycle hook | Yes | Hit/combat/proc primitives. | Direct and periodic versions of an effect produce matching combat/proc outcomes. |
| Proc backlog | `Unit::IsTriggeredAtSpellProcEvent`, trigger dispatch, dummy, trigger spell/damage, reflect, power-cost, resistance, fear, and invisibility proc handlers remain discovered. | Lifecycle hook | Partial | Stable holder and trigger context. | Eligible event triggers once; ineligible event does not; handler applies concrete effect. |

### Area Auras and Dynamic Objects

| Source | Gap | Class | Live | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- | --- |
| `game/player/spells/effects/aura.rs:134-163` | Persistent area aura uses fixed range/duration, creates no DynamicObject, and applies to caster. | World/lifecycle | Yes | Spell radius/duration data and DynamicObject manager. | Ground aura affects eligible units in its DBC radius until expiry. |
| `game/player/spells/effects/aura.rs:231-260` | Party/pet/friend/enemy area auras use ordinary aura application instead of target propagation. | World/lifecycle | Yes | Area target polling and holder lifecycle. | Each area mode applies/removes only eligible targets as membership changes. |

### Effect Families

| Family | Source modules | Class | Required owning subsystem | Minimal scenario |
| --- | --- | --- | --- | --- |
| Core combat/control | `effects/combat.rs`, `effects/dispel.rs`, `effects/healing.rs`, `effects/damage.rs` | Lifecycle | Swing queue, threat, cast interruption, NPC movement, sanctuary, aura metadata, creature type/resistance. | Taunt, interrupt, dispel, distract, extra attack, mechanical heal, and spirit heal mutate real state. |
| Items and professions | `effects/item.rs`, `effects/profession.rs` | Owning subsystem | Inventory, enchant, durability, lock, skill, crafting, loot, corpse APIs. | Enchant, disenchant, feed pet, craft, skin, and remove insignia all mutate inventory/world state. |
| Objects | `effects/object.rs` | Owning subsystem | GameObject registry, state, spawn/despawn slots. | Unlock, activate, summon, and despawn object at spell target. |
| Summons and pets | `effects/summon.rs`, `effects/pet.rs` | Owning subsystem | Spawn, ownership, pet persistence, possession, totem slots. | Guardian, pet, totem, tame, teach, dismiss, and dead-pet summon use correct lifetime/owner. |
| Movement and transport | `effects/movement.rs`, `effects/teleport.rs` | Owning subsystem | Pathing, movement controller, teleport, taxi, homebind persistence. | Charge, leap, pull, summon/face, bind, unstuck, and taxi are authoritative. |
| Quest, PvP, and misc | `effects/quest.rs`, `effects/pvp.rs`, `effects/misc.rs` | Owning subsystem | Quest, honor, duel, drunk, language, packet, interrupt APIs. | Quest/honor/duel/language/inebriate/interrupt effects persist and notify correctly. |
| Scripts and unclassified dispatcher stubs | `effects/script.rs`, `effects/mod.rs` | Owning subsystem | Script registry, nested cast, missile, quest, inventory, and aura APIs. | Registered script, trigger spell, missile, and each dispatched effect have observable outcomes. |

### AI and Client Consumers

| Source | Gap | Class | Prerequisite | Minimal scenario |
| --- | --- | --- | --- | --- |
| Spell handlers | Pet aura cancel and auto-repeat cancel are blocked. | Lifecycle hook | Pet/aura and active-cast ownership. | Client cancellation removes the intended aura/cast only. |
| Creature AI | Spell list setup and `SpellHit`/`SpellHitTarget` reactions are blocked. | Owning subsystem | Spell result callbacks and creature spell list data. | AI casts configured spell and receives hit/target callbacks. |
| Script targeting | `Spell::CheckScriptTargeting` is blocked. | Data/query | Script-target data and world object queries. | Script target accepts matching entry and rejects all others. |

## Planned Batches

### B-001: Gap Inventory and Scenario Matrix

- Phase: 0.
- Target: planning documents and port-harness metadata only.
- Inputs: all `TODO`, `not ported`, `placeholder`, `rust_compiled`, and `blocked` spell items.
- Deliverable: the Phase 0 detailed inventory, gap register, and this batch list.
- Verification: every identified runtime gap has source, dependency class, and minimal scenario.
- Result: complete on 2026-07-24. The first implementation batch is B-002.

### B-002: Shared Aura and Target Surface

- Phase: 1.
- Target: player aura container, creature aura state, and world target helpers.
- Required decisions: whether creature auras reuse `AuraContainer` directly or use an equivalent adapter; where real caster and ownership metadata live.
- Acceptance scenarios: apply, refresh, stack, query, remove by spell/type/caster, expire, and periodic tick for player and creature targets.
- Verification: `cargo test -p oxcore-world --lib game::player::auras` plus creature aura tests.
- Result: complete on 2026-07-24. Creatures now retain full `Aura` records in `AuraContainer`; supported creature modifiers use the same stored effects, and expiry runs through `AuraSystem`. Creature periodic, proc, and stat-modifier execution remain Phase 6 and Phase 8 work.

### B-003: Spatial and Spell Data Primitives

- Phase: 2.
- Target: spell manager, DBC data, map/world query interfaces.
- Includes: script targets, spell areas, chains, learn-spells, cones, radius, corpses, game objects, visibility, and line of sight.
- Acceptance scenarios: script target match/miss, area restriction accept/reject, radius and cone inclusion/exclusion, hidden target rejection.
- Verification: focused spell manager, targets, and map-query tests.
- Progress (2026-07-24): `TARGET_UNIT_SCRIPT_NEAR_CASTER` and
  `TARGET_GAMEOBJECT_SCRIPT_NEAR_CASTER` now consume loaded
  `spell_script_target` rows. Resolution preserves `inverseEffectMask`,
  prefers a valid explicit unit target, otherwise selects the nearest matching
  creature/dead creature/player/gameobject in the caster's map, instance, phase,
  and effect radius. Focused creature and gameobject tests pass. Remaining:
  `TARGET_LOCATION_SCRIPT_NEAR_CASTER` now resolves to a destination retained in
  `ResolvedTargets` and used by later implicit-target resolution in the same cast.
  Unit script-AOE modes now gather all nearby units and filter by configured
  entry/type/effect mask; source-AOE excludes the caster. Gameobject script-AOE
  modes resolve configured gameobjects around source/destination. Script-cone mode 60
  now resolves and filters configured units. A shared condition manager loads and evaluates
  every condition type reachable from `spell_script_target`: AND/OR/NOT, source entry,
  health percentage, GO spawned state, loot state, and GO state, including reverse/swap
  flags and cycle protection. Unknown condition kinds fail closed. Spell-focus casts now
  require an active matching spell-focus gameobject in the caster's map, phase, and template
  range. Script destination-AOE persistent aura and summon exceptions now match the source
  target registration behavior. `spell_area` loading now validates DBC areas, quest templates,
  duplicate requirements, aura requirements, autocast chains, playable race masks, and genders
  before exposing restrictions to live cast validation. Spell-chain DBC forward links now resolve
  predecessor-first with cycle protection, preserving rank roots independently of hash iteration
  order. Ordinary casts now use a side-effect-free script-target
  preflight and reject missing required unit/location targets before costs; triggered casts
  fail silently with `DONT_REPORT`. Execution remains the only resolver mode that consumes
  spell-magnet charges. Dead script targets now require visible creature-corpse state rather
  than any zero-health creature. Remaining: broader condition-table coverage and map-instance
  gameobject ownership.

### B-004: Target Hit Parity

- Phase: 3.
- Target: `targets.rs`, `validation.rs`, `hit.rs`, and `target_info.rs`.
- Includes: hostility, immunity, reflect, effect resistance, delayed rechecks, DR snapshot, and combat/threat side effects.
- Harness symbols: `Spell::FindCorpseUsing`, `Spell::CheckScriptTargeting`, and the blocked chunks of `Spell::DoAllEffectOnTarget`.
- Verification: scenario tests for direct and delayed player/creature casts.
- Progress (2026-07-25): `AuraContainer` now resolves active effect, state, and mechanic
  immunity modifiers for both player and creature targets. `do_spell_hit_on_unit` removes
  only immune effect bits before live effect dispatch. School immunity now rejects an entire
  hit when the active aura school mask matches, respecting source exemptions, polarity, and
  all-polarity attributes. Per-effect mechanic resistance now removes only the resisted effect
  before dispatch.
- Progress (2026-07-25, reflect): `roll_spell_hit` now checks damage immunity before rolling
  reflect, so an invulnerable victim never burns a reflect charge. Reflectability follows
  `SpellEntry::IsReflectableSpell` (magic damage class; not `IS_ABILITY`, `PASSIVE`,
  `NO_IMMUNITIES`, or `EX_NO_REFLECTION`), and the chance sums `SPELL_AURA_REFLECT_SPELLS`
  plus every school-matching `SPELL_AURA_REFLECT_SPELLS_SCHOOL` on the victim. `TargetInfo`
  gained `reflect_result`; a reflected cast resolves it against the caster and, when the
  caster can take the spell back, redirects `do_spell_hit_on_unit` and every effect dispatch
  onto the caster instead of the reflecting victim. `SMSG_SPELL_GO` now writes the
  reflect-result byte after a `SPELL_MISS_REFLECT` entry and only after that entry. Fixed
  `AURA_SPELL_MAGNET`, which was defined as 28 (`SPELL_AURA_REFLECT_SPELLS`) instead of 96,
  so spell-magnet targeting had been reading reflect auras. Remaining: the `PROC_EX_REFLECT`
  victim proc (phase 6) and the full `IsImmuneToSpell` precheck.
- Progress (2026-07-26, diminishing returns): step 7 of `DoSpellHitOnUnit` is now ported.
  The DR group and level are sampled once per unit hit into `TargetInfo::diminishing` and
  travel to every aura of that cast through `EffectInput`, so a multi-aura spell advances the
  counter once and diminishes all of its auras by one rate — the reason MaNGOS samples on hit
  rather than on aura add. The counter only advances when the surviving effect mask actually
  applies an aura. Creatures gained their own `DiminishingState`, so `DRTYPE_ALL` (the stun
  family) now diminishes them too, while `DRTYPE_PLAYER` groups stay confined to
  player-versus-player pairs. Aura apply/remove hold and release the group symmetrically for
  both target kinds, so the 15-second reset window only starts once the last aura drops. A
  fully diminished hit lands no aura at all. Divergences: `caster->IsFriendlyTo(target)` is
  approximated by spell polarity and `m_triggeredByAuraSpell` by `is_triggered`; the
  `DIMINISHING_LIMITONLY` 10-second cap is kept as it was.
- Progress (2026-07-26, hostile-action side effects): the `pRealCaster != unit` guard is now
  in place, so a reflected cast landing back on its caster no longer breaks the caster's own
  stealth or puts it in combat with itself. Direct-damage hits remove the target's
  `HOSTILE_ACTION_RECEIVED_CANCELS` auras; invisibility removal now uses
  `RemoveNonPassiveSpellsCausingAura` semantics so passive invisibility survives; and the
  caster's own stealth/invisibility drops once the hit passes the threat gate. Fixed a
  pre-existing bug behind all of this: every `SPELL_ATTR_EX*` test in the hit path went
  through `SpellEntry::has_attribute`, which only reads the base `Attributes` column, so
  `ALLOW_WHILE_STEALTHED`, `ALLOW_WHILE_INVISIBLE`, `NOT_AN_ACTION`, `NO_THREAT`,
  `THREAT_ONLY_ON_MISS`, `NO_INITIAL_THREAT`, `ONLY_PEACEFUL_TARGETS`, and `PVP_ENABLING`
  were all reading unrelated bits — `NOT_AN_ACTION` collided exactly with
  `NOT_IN_COMBAT_ONLY_PEACEFUL`. Added `has_attribute_ex/ex2/ex3` and corrected all 14 call
  sites. The attribute tests that should have caught this only asserted the function
  returned, so they were strengthened to assert real aura state. Still blocked on missing
  primitives: `IsVisibleForOrDetect` (the delayed-spell evade check and the caster-visibility
  gate) and `UpdatePvP` flagging.

### B-005: Holder and Channel Lifecycle

- Phase: 4-5.
- Target: active cast state, aura holder/container, channel holders, spell packets.
- Includes: holder references, stack updates, visible slots, real caster, channel visual persistence, pushback packets, zero-duration interruption.
- Harness symbols: `SpellAuraHolder::{ModStackAmount,SetStackAmount,GetRealCaster,IsNeedVisibleSlot}`, `Spell::SendChannelStart`, and `Spell::WriteSpellGoTargets`.
- Verification: cast start, target aura application, channel pushback, cancel, expiry, and cleanup packet fixtures.

### B-006: Periodic and Proc Core

- Phase: 6.
- Target: periodic aura dispatcher, proc dispatcher, trigger context.
- Includes: periodic damage/heal/leech/power effects, `Aura::PeriodicTick`, `Unit::IsTriggeredAtSpellProcEvent`, and `Unit::TriggerProccedSpell`.
- Excludes: large dummy and override-class-script handler batches.
- Verification: deterministic periodic and proc-chain scenarios without duplicate triggers.

### B-007: Dynamic Objects and Area Auras

- Phase: 7.
- Target: dynamic-object manager, area targeting, aura effects, holder refresh.
- Includes: persistent area aura, party/raid/pet/friend/enemy aura propagation, movement and cleanup.
- Verification: target enters/leaves radius, caster map change, channel interrupt, and expiry scenarios.

### B-008: Effect Families

- Phase: 8.
- Target: one effect family per Rust module batch.
- Order: combat/control, inventory/loot/profession, summons/pets/totems, teleport/world interaction, then scripts.
- Verification: dispatch-level tests that prove each effect mutates its owning subsystem.

### B-009: Persistence and Client Requests

- Phase: 9.
- Target: cooldown/learning persistence and spell opcode handlers.
- Includes: cooldown load before login packet send, spellbook persistence, passive/talent behaviour, pet aura cancel, and auto-repeat cancel.
- Verification: logout/login and client request scenarios.

### B-010: AI and Script Consumers

- Phase: 10.
- Target: creature AI and script manager.
- Includes: creature spell lists, spell-hit callbacks, script targeting, and event reactions.
- Verification: AI cast and scripted spell-hit integration scenario.

## Effect Family Order

| Wave | Families | Prerequisites |
| --- | --- | --- |
| A | Damage, healing, dispel, interrupt, taunt, threat, crowd control | Phases 1-6 |
| B | Items, locks, loot, professions, trade targets | Inventory, game-object, and repository APIs |
| C | Pets, guardians, totems, possession, summons | Ownership and creature spawning APIs |
| D | Teleport, taxi, quests, PvP, duels, languages | Map, transport, quest, PvP APIs |
| E | Script effects and spell-specific dummy behaviour | Script registry and all relevant core primitives |

## Decision Log

| ID | Decision | Rationale | Status |
| --- | --- | --- | --- |
| D-001 | Keep the current manager-based Rust architecture rather than introducing a broad `Unit` trait immediately. | Avoid an abstraction that hides lock and ownership boundaries before common operations are proven. | Accepted |
| D-002 | Require player and creature coverage for generic target/aura scenarios. | Player-only spell behaviour is the main source of current false parity. | Accepted |
| D-003 | Treat dynamic objects as a world-owned subsystem. | Area auras, persistent effects, and channel cleanup require map/phase/lifetime ownership. | Accepted |
| D-004 | Delay broad scripted dummy spell support until the proc and script registry contracts are stable. | Large per-spell switches should consume stable primitives, not define them. | Accepted |

## Batch Log

| Batch | Date | Scope | Result | Verification | Harness updates |
| --- | --- | --- | --- | --- | --- |
| None | - | - | Plan created; no implementation begun. | `cargo check -p oxcore-world --all-targets` completed during investigation. | None |
| B-001 | 2026-07-24 | Runtime gap inventory and scenario matrix. | Complete; detailed inventory identifies core dependency layers and selects B-002 as the first implementation batch. | Source/reference audit; every substantive gap has a dependency class and minimal scenario. | None; Phase 0 does not advance implementation tasks. |
| B-002 | 2026-07-24 | Shared creature/player aura storage and core creature lifecycle. | Complete; creature auras now use `AuraContainer`, preserve metadata, expire through `AuraSystem`, and re-sum remaining speed modifiers on removal. | `cargo check -p oxcore-world --all-targets`; `cargo test -p oxcore-world --lib game::player::auras::system`; `cargo test -p oxcore-world --lib game::player::spells::target_info`. | None; this foundation batch extends already-done lifecycle symbols without a new discrete harness task. |
| B-003 | 2026-07-24 | Script target data, spatial resolution, conditions, spell-focus validation, spell-area loading, and spell-chain resolution. | In progress; script-near, location, unit/GO AoE, and script-cone target modes consume live data. Condition filtering, spell-focus lookup, source-valid `spell_area` loading, and deterministic DBC forward-chain resolution are wired; remaining source parity is recorded above. | `cargo test -p oxcore-world --lib game::spell::manager`; `cargo check -p oxcore-world --all-targets`. | `SpellMgr::LoadSpellAreas` and `SpellMgr::LoadSpellChains` chunks advanced to done; `SpellMgr::LoadSpellScriptTarget` remains `rust_compiled`; `Spell::CheckScriptTargeting` remains blocked pending remaining branches. |
| B-004 | 2026-07-25 | Shared active-aura immunity and mechanic-resistance hit filtering. | In progress; player and creature targets use the same `AuraContainer` query to reject matching effect/state/mechanic effects and whole hits covered by school immunity. Effect-specific mechanics now roll active mechanic resistance before dispatch. Reflect and packets remain. | `cargo test -p oxcore-world --lib game::player::auras::container`; `cargo test -p oxcore-world --lib game::player::spells::target_info`; `cargo check -p oxcore-world --all-targets`. | `Unit::IsImmuneToSpellEffect`, `Unit::IsImmuneToSchool`, and `Unit::IsEffectResist` advanced to done. |
| B-004 | 2026-07-26 | Spell reflection and the `SMSG_SPELL_GO` reflect-result byte. | In progress; reflect is rolled from live victim auras after the immunity check, the reflected cast resolves its own outcome against the caster and redirects hit processing plus every effect onto the caster, and the packet carries the reflect result. Also corrected `AURA_SPELL_MAGNET` (28 to 96), which had aliased `SPELL_AURA_REFLECT_SPELLS`. Remaining for phase 3: DR snapshots and complete combat/threat transitions (G-006). | `cargo test -p oxcore-world --lib` (1732 passed); `cargo test -p oxcore-shared --lib` (188 passed); `cargo check --workspace --all-targets`. | `SpellCaster::SpellHitResult` advanced to done. |
| B-004 | 2026-07-26 | Per-hit diminishing-returns snapshot shared across a cast's auras, for player and creature targets. | In progress; DR moved from per-aura application to the spell-hit step it belongs to, creatures gained their own counters so `DRTYPE_ALL` groups diminish them, and apply/remove now hold the reset window symmetrically. Combat/threat transitions (the other half of G-006) remain. | `cargo test -p oxcore-world --lib` (1746 passed); `cargo test -p oxcore-shared --lib` (188 passed); `cargo check --workspace --all-targets`. | `Unit::GetDiminishing`, `Unit::IncrDiminishing`, `Unit::ApplyDiminishingToDuration`, `Unit::ApplyDiminishingAura`, and `Spells::GetDiminishingReturnsGroupType` advanced to done. |
| B-004 | 2026-07-26 | Hostile-action side effects in `DoSpellHitOnUnit`, plus the `SPELL_ATTR_EX*` accessor fix. | In progress; self-hit guard, hostile-action interrupt removal, non-passive invisibility semantics, and caster-side stealth removal are live. Every extended-attribute test in the hit path was reading the base `Attributes` column and is now corrected, with the smoke tests that missed it strengthened to assert aura state. Remaining in G-006: visibility gating and PvP flagging, both blocked on absent primitives. | `cargo test -p oxcore-world --lib` (1750 passed); `cargo test -p oxcore-shared --lib` (188 passed); `cargo test -p oxcore-dbc --lib` (4 passed); `cargo check --workspace --all-targets`. | `Spell::DoSpellHitOnUnit:chunk_0` advanced to verified. |

## Milestone Verification

- Per batch: `cargo test -p oxcore-world --lib <module-path>`.
- Per phase: focused player, creature, packet, and world integration scenarios named in the batch.
- Before advancing a critical flow: `cargo test --workspace` and a reference-claim review.
- Final milestone: no runtime spell TODO/no-op remains without an explicit unsupported-feature decision and test.
