## Context

The baseline audit covers 280 unique cards across 13 archetypes. Three native
regressions reproduce ST23-04 and EX4-074 activating after unrelated attacks.
EndOfAttack is a broad engine event, also used by native observers and deferred
processing. The currently deployed run remains explicitly diagnostic.

## Goals / Non-Goals

Goals: correct printed timing semantics, extend evidence to other timing classes,
and prepare exact recent global EX13 lists with honest train/held-out separation.
Non-goals: changing action/tensor contracts, inventing deck substitutions, changing
search budgets, automatically replacing the live experiment, or certifying every
card from file presence or admission alone.

## Decisions

1. Preserve the broad engine event and qualify printed DSL End of Attack at
   trigger capture using the actual declared attacker's stable identity. The
   authored trigger predicates remain conjunctive. Top-card and inherited
   effects use their carrier, not the physical source card. Restricting the whole
   broadcast would incorrectly suppress broad observers and scheduled effects.
   Trigger capture and later availability are separate: field compaction must
   not transfer a pending effect to an unrelated object.
2. Test real production cards through legal actions in both seats, including
   successful and unsuccessful attacks, unrelated allies/opponents, departure,
   cloning and compaction. Preserve direct DebugRunner timing injection as an
   explicit test event; it must not redefine production broadcast semantics.
3. Inventory every timing declaration in the frozen union and refreshed additions.
   Investigate event-kind/controller filters, turn gates, carrier scope and
   optional/once-per-turn processing. Concrete new bugs get failing regressions
   before implementation. Static review, native verification and oracle evidence
   remain separate categories.
4. Freeze a new pool version, sourcing recent EX13 exact deck lists globally.
   Rank candidates using recency, relative finish, event size and event/list
   diversity. Reject altered/duplicate/unadmitted/illegal lists. Target two
   training and two held-out variants per archetype. If sourcing is insufficient,
   retain an explicit pet fallback and expose the shortfall instead of inventing
   unseen-list coverage. Preserve original pets in a separate regression panel.
5. Use existing GeneralistDeckPool snapshots and repair balanced_deck_pairs
   variant rotation to traverse the Cartesian product within each archetype
   matchup. The old lockstep rotation omitted combinations; new reports identify
   archetype-paired-v2. Record source URLs, dates, IDs, counts, selection rationale, rejected
   candidates and artifact hashes. Preserve old snapshots for fixed comparisons.
6. Capture due start-turn Delay entries into the ordinary simultaneous trigger
   window. Preserve processing-condition rechecks, decline recurrence and native
   versus DSL trash-cost ownership across selections and clones. Reset per-turn
   state before this window, once, so prior-turn uses cannot suppress derived
   triggers and new uses cannot be erased on entry into Main.

## Risks / Trade-offs

- Shared timing changes can affect cards outside the pool -> inspect every
  production EndOfAttack clause and run combat/DSL/clone controls.
- New competitive lists may contain unsupported cards -> reject exact candidates
  with named reasons; never remove the offending card to pass admission.
- Some pet archetypes lack competitive published lists -> retain labeled
  fallbacks and distinguish same-list regression from held-out evaluation.
- Winner-only selection can bias a panel -> preserve multiple events and builds,
  publish selection criteria and retain original benchmark separately.
- Current models learned under the old engine -> qualify a repaired image and
  repeat fixed evaluations before strength comparison or promotion.

## Migration Plan

Validate local changes and versioned assets first. A subsequent image/run rollout
uses a new run directory and engine/pool hashes; it must not overwrite the current
diagnostic run or mix its replay into a supposedly repaired dataset implicitly.
The current task does not deploy, restart, or promote.
