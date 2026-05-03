# TS Olympos Rust DSL/Engine Gap Inputs

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `TS Olympos`, using the current 64-list local archetype pool and prioritizing high-frequency TS / Iliad / Olympos XII cards. This document is a spec-input artifact for compiling remaining cross-archetype Rust DSL and engine gaps. It is not the legacy Python-lane faithfulness report in `qa/archetype-qa/ts_olympos.md`.

## Verdict

`blocked`

TS Olympos is not currently faithfully implementable as executable Rust YAML DSL. The old QA report marks the archetype as faithful/fixed in the Python lane, but the current Rust DSL pack has only a small BT24 production slice and does not include the TS Olympos core cards.

The remaining most important blockers are reusable: cross-card effect re-firing, remaining cross-permanent replacement variants, source-stack aggregate predicates/dynamic amounts, effect-timing suppression variants, immediate may-attack effects, and dynamic option-use-from-hand flows. Top-security-to-hand, Recovery, multi-bucket reveal selection, BT24-040 targeted timing lock/protection, and BT24-101 security-loss protection now have focused Rust coverage and should stay out of the remaining-blocker backlog unless a new card exposes a new primitive gap.

## Coverage Snapshot

- Archetype source: `data/deck_library.json` entry `TS Olympos`.
- Local decklists: 64.
- Core cards by presence:
  - `BT24-102` Homeros: 64/64 lists.
  - `BT24-034` Aegiomon: 55/64 lists.
  - `BT24-100` In-Between Theater: 54/64 lists.
  - `BT24-031` Elecmon: 50/64 lists.
  - `BT24-041` Minervamon: 50/64 lists.
  - `BT24-040` Venusmon: 55/64 lists.
  - `BT24-085` Dan Yuki & Kanan Yuki: 48/64 lists.
  - `BT24-030` Neptunemon: 47/64 lists.
  - `BT24-043` Tapirmon: 41/64 lists.
  - `BT24-090` Abyss Sanctuary: Throne Room: 38/64 lists.
- Rust YAML currently found under `code/digimon-engine/cards/bt24/`: `BT24-001`, `BT24-008`, `BT24-011`, `BT24-012`, `BT24-016`, `BT24-017`, `BT24-018`, `BT24-031`, `BT24-040`, `BT24-082`, `BT24-089`, `BT24-101`.
- TS Olympos core cards currently missing production Rust YAML include `BT24-034`, `BT24-100`, `BT24-102`, `BT24-041`, `BT24-043`, `BT24-085`, `BT24-030`, `BT24-014`, `BT24-004`, `BT24-083`, `BT24-088`, `BT24-020`, `BT24-090`, `BT24-037`, `BT24-046`, and the common promo support cards.

## Reusable Gaps For Cross-Archetype Spec

### G-TS-TOP-SECURITY-TO-HAND

- **Type:** resolved reusable DSL primitive; remaining card-authoring / test coverage gap
- **Blocks TS Olympos cards:** `BT24-034`, `BT24-090`, plus security-cost/protection variants across the shell still need production YAML and card-shaped tests. `BT24-031` and `BT24-101` are no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype reuse:** Gallantmon, Training / Memory Boost security flows, Scramble-style security movement, any "add your top security card to hand" cost or effect.
- **Printed shape:** move the top card of a player's security stack to hand, preserving the security-removed event chain, sometimes as a cost before a player choice.
- **Current evidence:** As of 2026-05-03, `add_top_security_to_hand` is available through DSL lowering and the security-stack event chain is covered by focused DSL/effect-context/card tests for `BT24-031` and `BT24-101`.
- **Remaining work:** `BT24-034`, `BT24-090`, and sibling security-cost/protection cards still need faithful production YAML and behavioral tests for their printed cost gating, follow-up legality checks, and any security-placement/protection details. Keep those as card-authoring blockers unless implementation exposes a new reusable primitive gap.
- **Suggested DSL shape:**

  ```yaml
  - optional:
      condition:
        any_card_in_hand:
          of: you
          filter: { kind: tamer, trait_has: TS }
      then:
        - add_top_security_to_hand: { of: you }
        - select_hand:
            of: you
            filter:
              kind: tamer
              trait_has: TS
              not_same_name_as_any_own_tamer: true
            bind_as: t
            prompt: "Choose a TS Tamer to play"
        - play_from_hand_free: { of: you, hand_index: t }
  ```

- **First test:** Resolve `BT24-034` with one legal TS Tamer in hand and one same-name Tamer already in battle. Assert the mask offers only the legal non-duplicate Tamer; accepting moves exactly the top security to hand and plays the Tamer, while declining leaves security unchanged.
- **Spec note:** Do not reopen the reusable top-security-to-hand primitive for remaining TS/Olympos cards. Use card-local blockers for cost-gated optional branches, same-name Tamer restrictions, security-placement costs, and unimplemented card tests.
- **Updated 2026-05-03:** `add_top_security_to_hand` and Recovery deck-step lowering are implemented for `BT24-031` / `BT24-101` and verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- security_stack_steps --nocapture`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- security_stack_operations --nocapture`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_031 bt24_101 --nocapture`. TS Olympos remains `blocked` overall because many other core cards still lack faithful YAML or card-shaped tests.

### G-TS-MULTI-BUCKET-REVEAL-SEARCH

- **Type:** DSL selection / pending-selection gap
- **Status:** Reusable primitive resolved on 2026-05-03. `select_reveal_buckets` now parses, compiles, validates, lowers to `EffectContext::select_reveal_buckets`, binds bucket results, and prevents duplicate reveal-card picks across buckets when `no_duplicate_cards: true`.
- **Blocks TS Olympos cards:** `BT24-043`, `BT24-020`, `BT24-100`, `BT24-083`, and sibling TS searchers. `BT24-031` is no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype reuse:** searchers that say "add 1 A and 1 B", especially where a revealed card can satisfy more than one bucket and must not be selected twice.
- **Printed shape:** reveal N cards, add one card matching bucket A and one card matching bucket B, then bottom the rest.
- **Current evidence:** Focused coverage exercises compile lowering, runtime bucket binding into `add_to_hand_from_reveal`, and action-mask duplicate prevention across buckets.
- **Required capability:** closed for the reusable reveal-zone bucket selection primitive. Card-specific migration still needs to wire each TS/Olympos YAML body and verify remainder placement/card text details.
- **Suggested DSL shape:**

  ```yaml
  - reveal_top_deck: { of: you, count: 3, bind_as: r }
  - select_reveal_buckets:
      from: r
      buckets:
        - bind_as: iliad
          filter: { trait_has: Iliad }
          max: 1
        - bind_as: ts
          filter: { trait_has: TS }
          max: 1
      no_duplicate_cards: true
      prompt: "Choose cards to add"
  - add_to_hand_from_reveal: { of: you, card: iliad }
  - add_to_hand_from_reveal: { of: you, card: ts }
  - place_remainder_on_deck: { of: you, position: bottom }
  ```

- **First test:** `BT24-031` reveals one Iliad-only card, one TS-only card, and one Iliad+TS card. Assert the player can choose legal non-duplicate bucket assignments and cannot add the same revealed card twice.
- **Passing focused tests:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal_buckets --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2e_select_reveal phase2e_select_ordered_permutation phase2b_zone_moves_extra --nocapture`.
- **Spec note:** The generic reveal-zone selection capability is implemented; keep future follow-up notes card-specific rather than reopening this reusable gap.

### G-TS-CROSS-CARD-EFFECT-REFIRING

- **Type:** engine / DSL gap
- **Blocks TS Olympos cards:** `BT24-102` Homeros.
- **Cross-archetype reuse:** Apocalymon, Dark Masters, Royal Knights, and any effect that activates another card's printed triggered effect outside its normal timing.
- **Printed shape:** choose an Olympos XII Digimon and activate one of its `[On Play]` or `[When Digivolving]` effects at end of turn.
- **Current evidence:** Existing trackers describe cross-card / cross-timing effect re-firing as an open reusable blocker. Current DSL has no step that walks another permanent's registered effects, filters by timing, lets the player choose one when multiple are available, and enqueues it with correct source attribution.
- **Required capability:** an effect re-firing primitive that can select a permanent, enumerate eligible effects by timing, present an action-masked choice, and run the selected effect without pretending the target just played or digivolved.
- **Suggested DSL shape:**

  ```yaml
  - select_own_permanent:
      filter: { trait_has: "Olympos XII" }
      bind_as: olympus
      optional: true
      prompt: "Choose an Olympos XII Digimon"
  - activate_effect_of:
      target: olympus
      timings: [on_play, when_digivolving]
      attribution: source
      optional: true
  ```

- **First test:** Homeros is unsuspended with two Olympos XII Digimon in battle, one with an On Play effect and one with both On Play and When Digivolving effects. At end of turn, assert the mask first selects the Digimon, then selects one eligible effect, suspends Homeros as the cost, and resolves only that chosen effect.
- **Spec note:** The spec should define attribution and once-per-turn accounting explicitly. Homeros should not refresh a once-per-turn effect that has already been used unless the card text permits it.

### G-TS-CROSS-PERMANENT-REPLACEMENT-PREVENTION

- **Type:** engine / DSL gap
- **Blocks TS Olympos cards:** `BT24-041`, `BT24-030`, `BT24-037`, and related protection cards. `BT24-040` and `BT24-101` are no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype reuse:** Puppets, Dark Masters, Royal Knights, Armor Purge / Barrier / Decoy-adjacent protection, and "protect another permanent" effects.
- **Printed shape:** when one of your other or matching trait permanents would leave, pay a cost using another source/permanent/security card and prevent the leave.
- **Current evidence:** Replacement support exists for some source/self patterns, but cross-permanent prevention needs subject/source separation, cause filters, cost prompts, and cancellation of the original zone move. Older raw-Rust comments and trackers call out subject-matches limitations and removal-cause attribution for similar shapes.
- **Required capability:** replacement predicates over the leaving subject and effect cause, with a source permanent that can be different from the subject. Cost payment must be optional and must park a `PendingSelection` before cancellation.
- **Suggested DSL shape:**

  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      replacement_subject_is_mine: true
      replacement_cause_not: own_effect
      replacement_subject_trait_has: TS
    cost:
      - trash_top_security: { of: you }
    process:
      - cancel_replacement: {}
  ```

- **First test:** With `BT24-101` in battle and another TS Digimon about to leave, assert the player may trash top security to prevent the other Digimon from leaving; declining allows the leave; own-effect removal does not offer the prompt when the printed text excludes it.
- **Spec note:** This should be grouped with replacement-context predicate work, not with individual Olympos XII card authoring.
- **Updated 2026-05-03:** Subject/source/cause-filtered cross-permanent replacement prevention is implemented for `BT24-040` placement-cost protection and `BT24-101` trash-top-security protection. DSL replacement lowering now preflights required nested cost selections and `CannotAddSecurityByEffect` security-placement costs so unpayable optional replacements are not offered. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_040 bt24_101 --nocapture`.

### G-TS-SOURCE-STACK-AGGREGATES

- **Type:** hybrid engine / DSL gap
- **Blocks TS Olympos cards:** `BT24-041`, `BT24-030`, `BT24-059`, `BT24-090`. `BT24-040` is no longer blocked by the trash-all-sources slice after the 2026-05-03 production YAML/tests.
- **Cross-archetype reuse:** source-control archetypes, De-Digivolve variants, Mineral/Rock source-trash effects, "fewest sources" board clears.
- **Printed shape:** trash all digivolution cards of one permanent; De-Digivolve by a dynamic count; return all opponent Digimon with the fewest digivolution cards; place/remove source cards from security or under permanents.
- **Current evidence:** `trash_all_sources` is implemented and verified for `BT24-040`, and `de_digivolve` supports bounded peeling. `BT24-030` still needs an aggregate predicate over opponent permanents' source counts. `BT24-041` still needs a dynamic amount equal to the controller's Digimon count.
- **Required capability:** remaining stack-source aggregate predicates and formula-backed mutations:
  - `stack_size_matches_aggregate: lowest` predicates over battle-area Digimon;
  - formula-backed `de_digivolve.amount`.
- **Suggested DSL shape:**

  ```yaml
  - return_to_deck:
      target:
        kind: digimon
        owner: opponent
        stack_size_matches_aggregate:
          selector: lowest
          of: opponent
      position: bottom
      include_sources: true

  - de_digivolve:
      target: target
      amount:
        formula:
          count: { kind: digimon, controller: self }
  ```

- **First test:** For `BT24-030`, set opponent stacks with 0, 1, and 2 sources and assert only the 0-source Digimon are bottom-decked. For `BT24-041`, control three Digimon, resolve the dynamic De-Digivolve branch, and assert exactly three peel attempts are made subject to normal De-Digivolve caps.
- **Spec note:** Split this into smaller implementation tasks if needed: source-count aggregate predicate, then dynamic De-Digivolve formula. Do not reopen the closed `trash_all_sources` slice unless a new card proves a missing source-trash variant.
- **Updated 2026-05-03:** The unbounded `trash_all_sources` slice is implemented and verified for `BT24-040` by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregates --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`. Source-count aggregate predicates and dynamic De-Digivolve formulas remain open for the other listed cards.

### G-TS-TIMING-SUPPRESSION-MODIFIERS

- **Type:** engine / DSL gap
- **Blocks TS Olympos cards:** `BT10-042` and tech cards that suppress `[When Digivolving]` / `[When Attacking]` effects. `BT24-040` is no longer blocked by this item after the 2026-05-03 production YAML/tests.
- **Cross-archetype reuse:** Venusmon variants, Dark Masters, Queen Device, and other per-permanent effect-locking cards.
- **Printed shape:** selected opponent Digimon or Tamers cannot suspend and/or cannot activate effects of a named timing until an expiry.
- **Current evidence:** `docs/RUST_ENGINE_GAPS.md` already tracks permanent-scoped timing suppression. Existing modifier mappings include some coarse suppressors, but the queue fan-out must consult a timing-parametric modifier at enqueue time.
- **Required capability:** `ModifierType::CannotActivateEffectsByTiming(EffectTiming)` or equivalent, plus DSL lowering for targeted and aura-like grants. The dispatch layer must skip suppressed effects while leaving unrelated timings legal.
- **Suggested DSL shape:**

  ```yaml
  - select_opponent_permanent:
      filter:
        any_of:
          - kind: digimon
          - kind: tamer
      bind_as: lock_a
      prompt: "Choose a Digimon or Tamer"
  - add_modifier:
      target: lock_a
      modifier: CannotActivateEffectsByTiming
      timing: when_digivolving
      expiry: end_of_opponents_turn
  - add_modifier:
      target: lock_a
      modifier: CannotSuspend
      value: 1
      expiry: end_of_opponents_turn
  ```

- **First test:** Resolve `BT24-040`, select an opponent Digimon with a When Digivolving effect, then digivolve it. Assert the When Digivolving effect is not enqueued, while its On Deletion or When Attacking effects remain unaffected unless separately suppressed.
- **Spec note:** Avoid encoding this as player-wide effect lockout; Venusmon targets specific permanents.
- **Updated 2026-05-03:** Targeted `CannotSuspend` plus `CannotActivateEffectsByTiming(WhenDigivolving)` is implemented for `BT24-040` and verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_040 --nocapture`. Aura-style suppression and other timing/card variants remain open until separately tested.

### G-TS-IMMEDIATE-MAY-ATTACK

- **Type:** engine gap
- **Blocks TS Olympos cards:** `BT24-085`, `BT24-037`, `BT24-091`, `BT24-095`, plus any TS option/tamer that says a Digimon may attack after another effect resolves.
- **Cross-archetype reuse:** Royal Knights, Zephagamon, Silphymon DNA shells, and many "then, 1 of your Digimon may attack" effects.
- **Printed shape:** after an effect resolves, choose one eligible Digimon and optionally attack, sometimes without suspending or with temporary modifiers.
- **Current evidence:** Existing tracker entries call out force-follow-up attack / may-attack helpers as blocking. Granting Rush or auto-attacking does not preserve the player decision.
- **Required capability:** a pending attack action opened from an effect, with target selection, optional decline, legality checks, and any "without suspending" or temporary rider metadata preserved.
- **Suggested DSL shape:**

  ```yaml
  - select_own_permanent:
      filter: { kind: digimon, trait_has: TS }
      bind_as: attacker
      optional: true
      prompt: "Choose a Digimon to attack"
  - may_attack_now:
      attacker: attacker
      without_suspending: false
  ```

- **First test:** Resolve `BT24-037` after DNA digivolving, select one of your Digimon for the may-attack effect, and assert the engine opens a normal legal attack flow with PASS available before the attack is committed.
- **Spec note:** This should share the same engine work as Royal Knights' end-of-turn attack and Zephagamon's attack/battle branches, while keeping effect battles separate from attacks.

### G-TS-OPTION-USE-FROM-HAND-BY-COST-CEILING

- **Type:** hybrid engine / DSL gap
- **Blocks TS Olympos cards:** `BT24-085` Dan Yuki & Kanan Yuki and TS Option-heavy variants.
- **Cross-archetype reuse:** Tamer effects that use an Option from hand without paying cost under a dynamic cost ceiling.
- **Printed shape:** at end of turn, suspend the Tamer, use one TS Option from hand with use cost less than or equal to the opponent's memory, then open a may-attack branch.
- **Current evidence:** Option play flow and Delay lifecycle have improved, but this exact "use an Option from hand as an effect with a dynamic ceiling" needs card-level proof and likely DSL sugar for filtering by opponent memory.
- **Required capability:** select an Option in hand by trait and use-cost formula, then invoke its Main/Security-equivalent effect path without paying cost, preserving option disposition and pending selections.
- **Suggested DSL shape:**

  ```yaml
  - select_hand:
      of: you
      filter:
        kind: option
        trait_has: TS
        play_cost_lte:
          formula:
            per: opponent_memory
            delta: 1
      bind_as: option
      optional: true
      prompt: "Choose a TS Option to use"
  - use_option_from_hand_free: { hand_index: option }
  - may_attack_now:
      attacker_filter: { trait_has: TS }
      optional: true
  ```

- **First test:** With opponent memory at 3, `BT24-085` suspended as cost, and TS Options of use cost 2 and 4 in hand, assert only the cost-2 Option is selectable and that its printed Option flow/disposition resolves before the may-attack prompt.
- **Spec note:** Promote only the dynamic use-from-hand capability. Specific TS Option bodies remain card migration work.

## Card-Local Authoring And Test Backlog

These items should not become cross-archetype gaps unless a failing Rust test proves current reusable primitives cannot express them.

Task 10 production-authoring audit update (2026-05-03): `BT24-031`, `BT24-040`, and `BT24-101` now have production YAML and focused behavioral tests. This closes their listed card-specific blockers: `BT24-031` On Play multi-bucket reveal plus inherited top-security-to-hand/Recovery; `BT24-040` trash-all-sources, two-target suspension/WhenDigivolving lock, cost reduction, Lv5 TS alt path, placement-cost protection, no-cost-body replacement preflight, and CannotAddSecurityByEffect security-placement cost preflight/resolution; `BT24-101` standard Lv5 yellow cost-5 digivolve route, Lv5 TS cost-3 alt path, dynamic Lv5 Aegiochusmon-name alt path, top-security trash/Recovery, OnLoseSecurity observer, and trash-security protection. TS Olympos remains `blocked` because many other core cards below still lack faithful YAML or remain blocked by reusable primitives.

| Card(s) | Status | Next Rust test |
|---|---|---|
| `BT24-034` Aegiomon | blocked by top-security-to-hand + duplicate-name Tamer filter | Optional cost branch, non-duplicate TS Tamer selection, free play, OnMove/OnPlay/WhenDigivolving all share body |
| `BT24-102` Homeros | blocked by cross-card effect re-firing | Start-main memory/draw, TS DP aura, EOT reactivation with Homeros suspend cost |
| `BT24-100` In-Between Theater | authoring / test gap after generic Delay support | Ignore color with TS field presence, reveal-add TS, place as Delay, Delay gain 2, Security places in battle area |
| `BT24-031` Elecmon | implemented 2026-05-03; production YAML and 5 focused behavioral tests pass | Keep in validated-cards report as implemented; no remaining BT24-031-specific blocker from this audit. |
| `BT24-043`, `BT24-020` | reusable multi-bucket reveal search primitive closed; production authoring still needed for their printed inherited variants | Reveal 3 with bucketed choices, no duplicate reveal card, correct bottom remainder, then any printed inherited security movement / Recovery. |
| `BT24-040` Venusmon | implemented 2026-05-03; production YAML and 10 focused behavioral tests pass | Keep in validated-cards report as implemented; no-cost-body and CannotAddSecurityByEffect replacement preflight are covered; no remaining BT24-040-specific blocker from this audit. |
| `BT24-041` Minervamon | blocked by dynamic De-Digivolve count + play-cost reduction + aura keywords | Free-play Iliad cost <=5, De-Digivolve count equals own Digimon count, Reboot/Blocker aura on opponent turn |
| `BT24-030` Neptunemon | blocked by source-count aggregate + cross-permanent protection | Bottom-deck all fewest-source opponent Digimon, unsuspend self once, opponent-effect protection by suspending self |
| `BT24-101` Jupitermon | implemented 2026-05-03; production YAML and 12 focused behavioral tests pass | Keep in validated-cards report as implemented; standard Lv5 yellow cost-5, Lv5 TS cost-3, and dynamic Aegiochusmon route precedence are covered; no remaining BT24-101-specific blocker from this audit. |
| `BT24-085` Dan Yuki & Kanan Yuki | blocked by Option use from hand and may-attack | End-turn suspend cost, dynamic Option use ceiling, then TS may-attack |
| `BT24-037` Silphymon | blocked by immediate may-attack and DNA-origin riders | On Play/WD -5000 DP, may-attack, DNA-origin Security A.+1/+5000 DP |
| `BT24-083`, `BT24-088` | authoring / test gap with existing play-from-hand/trash helpers | Return Tamer to deck as cost, free-play matching card, On Play search/trash-draw |
| `BT24-090` Abyss Sanctuary | blocked by top/bottom security movement and option self-disposition tests | Main bottom-security-to-hand, self face-up bottom security, reduced-cost play, Security hand/trash free play |

## Stale Tracker Cleanup Candidates

Before compiling the cross-archetype spec, review these older TS Olympos notes so the roadmap does not reopen closed generic work:

- `G-ON-MOVE`: current docs say `EffectTiming::OnMove`, `when: on_move`, and moved-permanent trigger context are implemented and tested. TS cards still need authoring/tests, but `[When Moving]` is no longer the broad primitive blocker.
- Delay placement-turn gating and start/event Delay timing: Group 5 work closed much of this. TS Options still need card tests and possibly option-use-from-hand support.
- Battle-area filtered aura runtime: Group 6 resolved filtered aura materialization for battle-area sources. Homeros' TS DP aura should be treated as card authoring/test work unless a focused Rust test proves a remaining aura bug.
- `select_effect_choice`: current `SelectionKind::EffectChoice` exists. Homeros still needs cross-card effect enumeration/refiring, not merely a menu primitive.
- `add_to_hand_from_security`: a specific-card movement helper exists. TS needs top-security binding/syntax and cost-gating behavior, not a raw ability to move an arbitrary known handle.

## Suggested Spec Compilation Order

1. Keep `G-TS-TOP-SECURITY-TO-HAND` open only for unimplemented cards such as Aegiomon and Abyss Sanctuary; the `BT24-031` / `BT24-101` slice is implemented and verified.
2. Migrate remaining TS/Olympos searcher YAML to the closed `select_reveal_buckets` primitive and keep remaining blockers card-specific; `BT24-031` is already implemented.
3. Promote `G-TS-CROSS-CARD-EFFECT-REFIRING` with Homeros and Apocalymon-style cases in the same spec group.
4. Keep `G-TS-CROSS-PERMANENT-REPLACEMENT-PREVENTION` open for unimplemented cards; `BT24-040` and `BT24-101` protection are implemented and verified.
5. Promote remaining `G-TS-SOURCE-STACK-AGGREGATES` slices for source-count aggregate predicate and dynamic De-Digivolve amount; `BT24-040` trash-all-sources is implemented and verified.
6. Keep `G-TS-TIMING-SUPPRESSION-MODIFIERS` open for aura/other-card variants; `BT24-040` targeted lockout is implemented and verified.
7. Promote `G-TS-IMMEDIATE-MAY-ATTACK` jointly with Royal Knights and Zephagamon immediate-attack needs.
8. Promote `G-TS-OPTION-USE-FROM-HAND-BY-COST-CEILING` only if Dan Yuki & Kanan Yuki cannot be authored with existing Option flow and formula predicates.
9. Keep the rest as TDD card migration under `code/digimon-engine/tests/cards_behavioral/bt24/` and `code/digimon-engine/cards/bt24/`.

## Spec Input Checklist

A future cross-archetype spec should require each promoted reusable gap to include:

- one failing Rust behavioral test under `code/digimon-engine/tests/`;
- one DSL parsing/lowering test when YAML vocabulary changes;
- action-mask or `PendingSelection` assertions for every player-visible choice;
- explicit source attribution, controller, and once-per-turn semantics for refired or delayed effects;
- no `ACTION_SPACE_SIZE` or tensor contract expansion unless `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, wrappers, frontend constants, and model metadata update together;
- tracker updates in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and this file when a reusable gap closes, splits, or is demoted to card-local authoring.

