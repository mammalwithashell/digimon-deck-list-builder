# Medusamon Cross-Archetype DSL/Engine Gap Input

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `Medusamon`, using the 94 current
archetype lists as the card-priority signal and the existing Rust YAML under
`code/digimon-engine/cards/` as the executable implementation surface.

Purpose: normalize the remaining Medusamon blockers into reusable DSL/engine
capabilities that can feed a later cross-archetype gap specification. This file
is not a card implementation plan. It should help spec authors decide which
remaining primitives are worth closing once, then migrating Medusamon and other
archetypes onto the shared surface.

## Verdict

`blocked`

Medusamon has broad YAML coverage, and several blockers from the 2026-04-28
batch report are now stale or partially closed by later engine work. The
archetype is still not faithful enough for serious Rust-engine training because
the BT24 core depends on remaining reusable capabilities around selected
security/trash movement, immediate attack prompts, cross-permanent replacement
effects, and event-context bindings.

## Core Card Pressure

Highest-frequency Medusamon cards from the current deck library:

| Card | Role | Current pressure |
|---|---|---|
| `BT24-017` Medusamon | Main finisher | Needs exact opponent-trash return cost, Petrification Token follow-up, and DP scaling. |
| `BT24-016` Lamiamon | Main level 5 engine | Needs alt-path condition and refreshed opponent-choice/security tests. |
| `BT24-018` Styracomon | Main top end / protection | Needs selected security trash and cross-permanent would-leave replacement. |
| `BT21-081` / `BT24-082` Owen Dreadnought | Tamer engine | Immediate attack prompts resolved 2026-05-08; remaining BT24-082 risk is generic OPT enforcement. |
| `BT24-089` Unique Emblem | Delay option | Needs native event-gated Delay migration and body coverage. |
| `P-103` Offense Training / `LM-027` Red Scramble | Option package | Mostly authored, but security/Delay disposition needs migration/tests. |
| `BT21-029` / `EX11-012` Medusamon | Secondary bosses | `EX11-012` is close; `BT21-029` needs the omitted deletion observer arm. |

## Reusable Gap Candidates

### MED-GAP-01: Selected trash-to-deck movement with exact-count cost gates

- **Type:** hybrid, likely DSL-first if existing raw helpers can be generalized.
- **Tracker:** `qa/dsl-vocab-gaps.md` for the YAML step/lowering; escalate to
  `docs/RUST_ENGINE_GAPS.md` only if the engine lacks a faithful multi-card
  pending-selection primitive for opponent trash.
- **Blocks:** `BT24-017`, `LM-027`; likely future Paladin/Omnimon-style trash
  return effects.
- **Required behavior:** A player-visible selection over a player's trash, with
  exact-count requirements, destination top/bottom deck placement, and follow-up
  steps that run only if the required count was paid.
- **Evidence:** `BT24-017.yaml` references
  `bt24_017_return_selected_trash_to_deck_bottom`, but the raw function is not
  registered in `code/digimon-engine/src/cards/raw_rust/mod.rs`. `EX11-012`
  has a narrower registered helper for a single trash card, which suggests the
  reusable capability should be generalized instead of copied card by card.
- **First tests:**
  - `BT24-017` with exactly two opponent trash cards should prompt the opponent
    to select both, move them to deck bottom, then play two Petrification Tokens.
  - `BT24-017` with fewer than two opponent trash cards should skip the token
    follow-up.
  - `LM-027` Delay should move the selected red Digimon from trash to deck top
    at the start of the next turn.
- **Implementation hint:** Prefer a DSL step such as `select_trash` plus
  `return_bound_cards_to_deck` with `destination: top|bottom`, exact count, and
  an effect-success flag that gates the tail.

### MED-GAP-02: Selected non-top security trash and arbitrary security movement

- **Type:** hybrid.
- **Tracker:** `qa/dsl-vocab-gaps.md` for `select_security` /
  `trash_selected_security`; `docs/RUST_ENGINE_GAPS.md` if security selection
  needs a new action/pending-selection kind.
- **Blocks:** `BT24-018`; related to any card that trashes or moves a chosen
  security card instead of the top card.
- **Required behavior:** Player-visible selection over a security stack,
  including non-top cards, with correct owner visibility, selection mask, trash
  disposition, and observer events for "security stack is removed from".
- **Evidence:** `BT24-018.yaml` references `bt24_018_trash_selected_security`,
  but the function is not registered. Existing `trash_top_security` style
  effects are not enough for "any 1 security card".
- **First test:** With three opponent security cards, `BT24-018` should let the
  controller choose the middle card, trash exactly that card, fire security
  removal observers once, then offer the printed unsuspend branch.
- **Implementation hint:** Add `SelectionKind::Security` or a security-zone
  binding path that preserves stable indices through resolution.

### MED-GAP-03: Immediate follow-up attack prompts

- **Type:** engine-gap plus DSL vocabulary.
- **Status:** resolved for BT21-081 and BT24-082 as of 2026-05-08.
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` "Force-follow-up-attack / may attack
  without suspending script helpers" and companion DSL entry for an
  `may_attack_now` / `force_attack` step.
- **Formerly blocked:** `BT21-081`, `BT24-082`; also many cross-archetype cards with
  "then this/that Digimon may attack" or "that Digimon attacks".
- **Required behavior:** After an effect resolves, expose a legal attack action
  for a specific Digimon, preserving printed optionality, attack legality,
  target selection, suspend rules, and action masking.
- **Evidence:** `BT21-081.yaml` now grants Piercing then uses mandatory
  `force_attack`. `BT24-082.yaml` buffs the digivolved event target then uses
  optional `may_attack_now`, with PASS exposed through the mask.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_081_end_of_turn_selected_digimon_attacks_after_piercing_grant bt24_082_clause2_may_attack_prompt_installs_after_dp_buff`.
- **Regression tests:**
  - End-of-turn `BT21-081` should suspend Owen, grant Piercing, then force the
    selected Reptile/Dragonkin Digimon to attack.
  - `BT24-082` should offer, not auto-run, the attack after the matching
    Reptile/Dragonkin digivolution trigger.
- **Implementation hint:** Reuse the normal attack flow and masks instead of
  creating a hidden combat shortcut. The follow-up should park an explicit
  pending action for the selected attacker.

### MED-GAP-04: Cross-permanent would-leave replacement with event subject/cause

- **Type:** hybrid.
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` if replacement context cannot carry
  the threatened permanent, cause player, and cancellation target; otherwise
  `qa/dsl-vocab-gaps.md` for replacement-context predicates and bindings.
- **Blocks:** `BT24-018`, `BT24-012`; related to many "when one of your other
  Digimon would leave" protection effects.
- **Required behavior:** A permanent that is not the leaving subject may respond
  to another eligible permanent leaving, pay a cost, then prevent that original
  leave-field event. Predicates must distinguish "other", owner, trait, and
  opponent-effect cause.
- **Evidence:** `BT24-018.yaml` references an unregistered
  `bt24_018_would_leave_replacement` helper. `BT24-012` uses a registered raw
  helper, but comments indicate it approximates cause/owner/subject handling.
- **First tests:**
  - `BT24-018` should protect a different Reptile/Dragonkin Digimon by deleting
    the opponent's lowest-DP Digimon, then leave the protected Digimon in play.
  - The same effect should not protect non-Reptile/Dragonkin permanents, should
    not fire for the wrong cause, and should enforce once-per-turn.
- **Implementation hint:** Replacement lowering needs named bindings for
  `replacement_subject`, `replacement_source_player`, and `source_permanent`,
  with `cancel_leave` applied to the subject rather than the effect source.

### MED-GAP-05: Event-target bindings for "that Digimon" observer bodies

- **Type:** hybrid, but likely card-migration/test-gap where the reusable event
  context has already landed.
- **Tracker:** `qa/dsl-vocab-gaps.md` for missing predicate aliases or lowering;
  do not file a new engine gap until current `event_target` and
  `event_card_trait_has` support is rechecked.
- **Blocks:** `EX11-054`, `BT21-029`; related to any observer that
  grants DP, suspends, draws, or plays a token based on the permanent involved
  in the triggering event.
- **Required behavior:** Observer effects need to inspect and bind the card or
  permanent that just digivolved, entered play, was deleted, or caused security
  removal.
- **Evidence:** `BT24-082` was rechecked and now uses `event_card_trait_has`
  plus `target: event_target` for the exact digivolved permanent. `EX11-054` has a registered raw no-op observer.
  `BT21-029` omits the opponent-deletion token arm even though current predicate
  code appears to include `event_target_owner`.
- **First tests / evidence:**
  - `BT24-082` now fires only when a Digimon digivolves into a
    Reptile/Dragonkin and grants +3000 DP to that exact Digimon; covered by
    `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_082_clause2_may_attack_prompt_installs_after_dp_buff`.
  - `BT21-029` should play a Petrification Token when an opponent Digimon is
    deleted, not when your own Digimon is deleted.
- **Implementation hint:** Prefer native `target: event_target` and
  event-card predicates over manual follow-up selections.

### MED-GAP-06: Attacker-context predicates for attack-target-change observers

- **Type:** DSL gap unless the combat event lacks the attacker payload.
- **Tracker:** `qa/dsl-vocab-gaps.md` as `attacker_trait_has` /
  `event_attacker_trait_has`.
- **Blocks:** `BT21-025`; likely other Raid or target-switch reward effects.
- **Required behavior:** An `on_attack_target_change` effect must evaluate the
  attacking Digimon's traits, not merely the controller's board state.
- **Evidence:** `BT21-025` currently approximates its inherited target-change
  gate by checking for any Reptile/Dragonkin on the field.
- **First test:** With a Reptile/Dragonkin and a non-Reptile/Dragonkin attacker
  in play, only the Reptile/Dragonkin attacker should enable the Lamiamon
  security-trash reward after an attack target changes.
- **Implementation hint:** Thread the attacker permanent handle through the
  attack-target-change trigger context and expose a predicate leaf against it.

### MED-GAP-07: Conditional alt-path authoring

- **Type:** DSL gap / data-gap.
- **Tracker:** `qa/dsl-vocab-gaps.md` for `AltPathSpec.condition`, unless the
  condition is better represented as runtime `CardData` requirement metadata.
- **Blocks:** `BT24-016` Lamiamon and future alt-digivolve routes gated by a
  Tamer, name, trait, board state, or source.
- **Required behavior:** Alternate digivolution paths need pre-mask conditions
  so the legal action appears only while the printed condition is true.
- **Evidence:** `BT24-016` can express the Elizamon target and Dimetromon trash
  cost shape, but comments note it cannot gate the alt path on Owen presence.
- **First test:** `BT24-016` should expose its Hand/Main alt digivolve path only
  while the controller has an Owen Dreadnought in play.
- **Implementation hint:** Add condition evaluation to alt-path registration and
  ensure action masks query it before exposing the route.

### MED-GAP-08: Native aggregate highest/lowest target operations

- **Type:** DSL gap or card-local raw helper debt, depending on current aggregate
  predicate coverage.
- **Tracker:** `qa/dsl-vocab-gaps.md` for missing highest/lowest selector
  syntax; avoid a new engine entry if existing aggregate predicates can cover it.
- **Blocks:** `BT21-093`; related to broad delete-highest/delete-lowest effects.
- **Required behavior:** Select or iterate opponent permanents that match an
  aggregate DP/play-cost condition, then apply a normal mutation.
- **Evidence:** `BT21-093` registers `bt21_093_delete_highest_dp_opponent`, but
  the helper is a no-op stub. Other cards already use lowest-DP patterns, so
  this should become native or a real shared helper.
- **First test:** `BT21-093` should delete only the opponent Digimon with the
  highest DP, including tie behavior if multiple share the maximum.
- **Implementation hint:** Check current `CompiledAggregateSelector` coverage
  before adding syntax; this may now be a migration/test gap rather than a new
  primitive.

## Migration And Test Debt, Not Fresh Gap Work

Several old Medusamon blocker labels should be revalidated before they are used
as spec inputs. Source inspection on 2026-05-03 suggests the reusable substrate
exists or is partially closed:

| Old label | Current guidance |
|---|---|
| `G-INHERITED-DISPATCH` | Do not refile blindly. Inherited triggered dispatch and permanent-backed triggered OPT have current engine support and active `BT21-008`-style tests. Refresh stale card tests/YAML first. |
| `G-OPT-TRIGGERED` | Same guidance as inherited dispatch. Confirm per-card max-per-turn behavior through behavioral tests. |
| `G-PRED-DP-LTE` / `G-PLAY-COST-LTE` | Static DP and play-cost filters appear implemented for common selection paths. Remaining ignores may be stale migration debt. |
| `G-PLACE-SELF-AS-OPTION-PERMANENT` | `place_self_as_delay_option` exists for the narrow option-placement slice. Migrate `P-103` security and similar empty processes before planning a new primitive. |
| `G-DELAY-START-OF-TURN` | `DelayTrigger::StartOfYourNextTurn` appears present. Migrate `LM-027` away from `lm_027_delay_start_of_turn_noop` and test the body before filing new engine work. |
| `G-DELAY-SUSPEND-CONDITION` | Event-gated Delay on `on_suspend` appears partially supported. Migrate `BT24-089` to native `kind: delay` with an Owen predicate before filing a new gap. |
| `G-DECLARATIVE-KEYWORD` | Common keyword grants and auras have improved. Verify card-specific runtime behavior instead of assuming the old blanket gap remains. |

## Recommended Spec Grouping

1. **Security/trash zone movement group:** MED-GAP-01 and MED-GAP-02. These
   share selection-over-hidden/ordered zones, destination disposition, and
   observer-event correctness.
2. **Attack prompt group:** MED-GAP-03. Keep this separate because it touches
   combat action masks and should not change `ACTION_SPACE_SIZE` unless an
   explicit action/tensor contract spec is approved.
3. **Replacement/event-context group:** MED-GAP-04 and MED-GAP-05. These both
   require reliable event subject/cause bindings and should share tests for
   `target: event_target` and replacement-subject cancellation.
4. **Predicate/authoring cleanup group:** MED-GAP-06 through MED-GAP-08 plus
   the migration debts above. This is mostly YAML/lowering/test cleanup once
   current predicate support is confirmed.

## First Regression Set

These tests give the smallest useful proof that the cross-archetype surface is
ready for Medusamon migration:

1. `BT24-017`: exact two opponent-trash cards returned to deck bottom gates two
   Petrification Tokens and DP scaling.
2. `BT24-018`: selected non-top security card is trashed and security-removal
   observers fire exactly once.
3. `BT21-081`: Owen's end-turn effect creates the mandatory follow-up attack
   through the normal action mask.
4. `BT24-018`: cross-permanent would-leave protection prevents the original
   leave event after paying the printed delete-lowest-DP cost.
5. `BT24-082`: "that Digimon" from an on-digivolve event receives the DP buff
   and optional attack prompt without a manual replacement selection.
6. `BT21-025`: target-change reward checks the actual attacker trait.
7. `LM-027` and `BT24-089`: native Delay migrations replace no-op raw helpers
   and prove start/event-gated Delay bodies execute after placement-turn gating.

## Notes For Spec Authors

- Keep gap entries capability-centric. For example, "selected trash-to-deck
  movement with exact-count cost gate" is reusable; "`BT24-017` raw helper
  missing" is only evidence.
- Preserve the no-approximations policy: do not replace player selections with
  auto-picks, especially for security, trash, and attack prompts.
- Before planning work from old Medusamon ignore labels, inspect current
  lowering and tests. The engine changed significantly after the 2026-04-28
  final report.
- Do not expand `ACTION_SPACE_SIZE` or active observation tensor contracts as a
  side effect of these card unlocks. If a new legal choice cannot reuse pending
  selections and existing action IDs, split it into an action/tensor contract
  spec first.
