# DSL Vocabulary Gaps Tracker

Resolved DSL gaps have been moved to [qa/resolved-gaps.md](resolved-gaps.md). This file tracks only open gaps and partial slices with remaining follow-up work.

This file accumulates `BLOCKED` verdicts whose `gap_kind` is `dsl` (the engine has the primitive but the DSL lacks a verb that lowers to it). Entries are appended by `/batch-implement-cards-rust-dsl`.

## EX10-020 — [Hand][Main] reduced-cost SELF-play + delete-at-EoT rider  [G-DSL-HAND-MAIN-SELF-PLAY-REDUCED]

Surfaced: 2026-06-10, judge-quiz Q3 authoring. EX10-020 Puppetmon PARTIAL (the Q3-relevant clauses are complete).

- **Card text:** "[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters] in their texts, you may play this card with the play cost reduced by 5. At turn end, delete the Digimon this effect played."
- **DCGO:** `EX10_020.cs` OnDeclaration — temp `ChangeCostClass` −5 on `UntilCalculateFixedCostEffect`, `PlayPermanentCards(self, payCost: true)`, then an `UntilOwnerTurnEndEffects` "[End of Your Turn] delete" attached to the played permanent.
- **Gap:** the `main_from_hand` timing exists, but every play verb plays a SELECTED card — there is no "play THIS CARD from hand, paying its cost with a delta" verb, and no rider to schedule the played permanent's EoT delete from the same activation. (`play_from_hand` + `cost_delta` exists for selected cards; the SELF-play form with pay-cost semantics is the missing piece.)
- **Consumers:** EX10-020 Puppetmon; the Q29 EX10 Bagra family shares the idiom (EX10-031 DarkKnightmon, EX10-056 Bagramon, EX10-059 DarknessBagramon) — land the verb with that cluster.

## EX10-020 — [Security] "if this card was face-up" gate  [G-DSL-SECURITY-WAS-FACE-UP-GATE]

Surfaced: 2026-06-10, judge-quiz Q3 authoring. EX10-020 Puppetmon PARTIAL.

- **Card text:** "[Security] If this card was face-up, you may play 1 level 5 or lower card with [Dark Masters] in its text from your hand or trash without paying the cost."
- **DCGO:** `EX10_020.cs` SecuritySkill gated `!CardEffectCommons.GetFaceDownFromHashtable(hashtable)` — the security card must have been FACE-UP when checked (e.g. placed face-up by its own [On Deletion]).
- **Gap:** the `on_security` trigger has no condition leaf exposing whether the checked security card was already face-up. Authoring the clause without the gate would over-fire on every normal (face-down) check — unfaithful, so the clause is OMITTED. Fix shape: thread the face-up bit from the security-check dispatch (`Player.face_up_security` membership at check time) into the trigger context + a `security_card_was_face_up: bool` condition leaf.
- **Body once unblocked:** `select_union_zone` (hand, trash) over `{ kind: digimon, level_lte: 5, effect_text_contains: "Dark Masters" }` + `play_union_bound_free` (BT25-094 idiom).

> **Substring trait predicate `trait_contains` — RESOLVED 2026-06-03**
> (EX3-014 Dorbickmon code-review fix). The DSL had only `trait_has`, an EXACT
> case-insensitive trait match (`x.eq_ignore_ascii_case(t)`). Printed text of the
> form "[Dragon], [saur] or [Ceratopsian] in **any of its traits**" is a SUBSTRING
> reading, matching DCGO `CardSource.HasDragonTraits` → `ContainsTraits("...")`
> (`DCGO/Assets/Scripts/Script/CardSource.cs`). Under exact match the `saur` clause
> was completely DEAD (no card carries a standalone "saur" trait — it only appears
> inside `Dinosaur` ×92 / `Ankylosaur` ×11 / `Plesiosaur` ×9), and `Dragon` mostly
> appears as a substring (`Dragonkin` ×92, `Dark Dragon` ×36, ...), so the EX3-014
> DP cap massively undercounted and `[Dinosaur]` Digimon could not be picked as
> DigiXros materials — a faithfulness + no-approximations violation. New leaf
> `trait_contains: <token>` is the substring sibling of `trait_has`: matches when
> ANY subject trait CONTAINS the token (case-insensitive,
> `subject_traits.iter().any(|x| x.to_lowercase().contains(&t.to_lowercase()))`).
> Threaded identically to `trait_has` — spec field
> (`digimon-dsl/src/predicate.rs`) → compiled field (`compiled.rs`) → lowering
> (`compile.rs`) → engine card-field eval AND synth-identity / `ChangeTraits`
> overlay path (`digimon-engine/src/dsl_cards/predicate.rs`), plus the
> `eval_no_subject_fields` subject-field guard. Works inside the
> `per: { source_stack_count: { filter } }` selector and DigiXros material filters
> (same `CompiledPredicate`). Unblocks the "[Dragon]/[saur]/[Ceratopsian]"-family
> matching. Pinned by `tests/cards_behavioral/ex3/ex3_014.rs` — esp.
> `ex3_014_dinosaur_source_counts_via_saur_substring` (the load-bearing `saur`
> substring proof). G-DSL-TRAIT-CONTAINS-SUBSTRING.

> **Trait-filtered carrier source count as a `per` selector — RESOLVED 2026-06-03**
> (EX3-014 Dorbickmon authoring). The `BasePerDelta` formula now accepts a new
> `per: { source_stack_count: { filter: <predicate> } }` selector that counts the
> effect carrier's own digivolution sources (the cards beneath its top card)
> matching a predicate. The engine already had the raw machinery (the top-level
> `source_stack_count` FormulaSpec + `eval_predicate_with_bindings`), but a raw
> count cannot be offset/scaled — there is no `add`/`mul` formula combinator. As a
> `per` selector it composes in `base + count * delta`, letting a card scale a
> numeric (here a DP cap) by the number of its sources matching a trait:
> Dorbickmon's "for each card with [Dragon], [saur] or [Ceratopsian] in any of its
> traits in this Digimon's digivolution cards, add 2000 to the maximum DP you can
> choose" → `dp_lte: { formula: { base: 3000, per: { source_stack_count: { filter:
> { any_of: [trait_has: Dragon, ...] } } }, delta: 2000 } }`. Spec
> `PerSelector::SourceStackCount(SourceStackCountSpec)` → compiled
> `CompiledPerSelector::SourceStackCountFiltered { filter }`; evaluated by
> `formula_eval::source_stack_count_filtered` (reads `ctx.source_permanent`). Pinned
> by `tests/cards_behavioral/ex3/ex3_014.rs` (scaling-cap behavioral tests).
> G-DSL-PER-SOURCE-STACK-COUNT-FILTERED.

> **`select_opponent_play_cost_budget.play_cost_budget` scalar → FormulaSpec — RESOLVED 2026-07-03**
> (P-094 Destromon authoring). The play-cost-budget multi-select step
> (`G-MULTI-SELECT-OPP-PLAY-COST-SUM`) previously took a plain `i32`
> `play_cost_budget`. Widened to `crate::formula::FormulaSpec`, mirroring the
> sibling `SelectOpponentDpBudgetArgs.dp_budget: FormulaSpec`. A bare integer YAML
> literal still parses (FormulaSpec's first untagged variant is `Literal(i32)`),
> so the existing scalar user EX4-073 is untouched (13/13 tests green). The formula
> is evaluated once at install time against the effect context (both the installer
> in `dsl_cards/step/selections.rs` and the replacement pre-check in
> `dsl_cards/lower_replacement.rs`), exactly like the DP path. Lets P-094 model
> "delete up to 3 play cost's total worth … for each [Vemmon] in this Digimon's
> digivolution cards add 1 to the maximum" →
> `play_cost_budget: { base: 3, per: { source_stack_count: { filter: { name_is:
> "Vemmon" } } }, delta: 1 }`. Pinned by `tests/cards_behavioral/p/p_094.rs`
> (baseline-3 + scaling-by-Vemmon behavioral tests). G-MULTI-SELECT-OPP-PLAY-COST-SUM.

> **`source_count` predicate leaf (filtered digivolution-source count ≥ N) — RESOLVED 2026-07-03**
> (P-094 Destromon authoring). New permanent-subject predicate leaf
> `source_count: { filter: <predicate>, at_least: N }` — true when the candidate
> carries ≥ N digivolution SOURCE cards (the cards beneath its top card) matching
> `filter`. Models the DCGO `DigivolutionCards.Count(predicate) >= N` idiom, which
> had no DSL expression (`materials_count_gte` counts ALL sources by raw stack
> length, not a name/trait-filtered subset). The nested `filter` is a full
> `PredicateSpec` evaluated per source card via `eval_card_fields` (source
> subject), so it accepts `name_is`/`name_contains`/`trait_has`/`kind`/etc.
> Threaded spec→compiled (`Option<(Box<CompiledPredicate>, u8)>`)→eval in BOTH the
> battle-area and breeding-area permanent evaluators (`dsl_cards/predicate.rs`),
> plus the permanent-only-leaf gates in `formula_eval.rs` +
> `lower_replacement.rs`, the validator recursion, and the pack raw-rust-fn walk.
> Gates P-094's inherited redirect: only a [Galacticmon] carrying ≥2 [Vemmon]
> sources is offered for the return-2-Vemmon cost. Pinned by
> `tests/cards_behavioral/p/p_094.rs` (`inherited_no_fire_without_galacticmon_
> carrying_two_vemmon`). G-DSL-SOURCE-COUNT-FILTERED.

> **`TreatAsDigimon` / `SynthIdentity` payload — RESOLVED 2026-05-30** (judge-quiz
> cluster-B authoring, Greymon/Marcus line). The DSL `add_modifier` step now accepts
> a structured `synth_identity:` block (`dp` required; `kind` defaults Digimon;
> `level`/`colors`/`traits` optional), lowering to the engine's pre-existing
> `ModifierPayload::SynthIdentity` via a new `EffectContext::add_modifier_with_payload`.
> This closes the Track C "rich payload parser pending" slice for the
> treat-a-Tamer-as-a-Digimon mechanic (RizeGreymon BT21-044's 3000 DP grant,
> ShineGreymon: Burst Mode BT13-020's 12000 DP grant). The validator requires
> `synth_identity` for `TreatAsDigimon` and forbids it on any other modifier.
> Pinned by `digimon-dsl` `parse_synth_identity` (3) + `validator::tests`
> `treat_as_digimon_without_synth_identity_is_rejected` /
> `synth_identity_on_non_treat_as_digimon_is_rejected`. The remaining Track C
> string/list payload variants (non-TreatAsDigimon) stay pending — see that note below.

> **ST-2 Cocytus Blue substrate closure — 2026-05-29:** ST2 introduced no
> remaining open DSL vocabulary gap. The new `trash_bottom_sources` step and
> `battle_opponent_no_sources` predicate are implemented and archived in
> `qa/resolved-gaps.md`; Kaiser Nail is covered by existing
> `select_material` / `play_from_materials`. Do not file ST2 bottom-source
> cards under `select_opponent_sources`: those printed effects choose the
> Digimon only, then deterministically trash the bottom source(s).

> **ST5 Machine Black closure — 2026-05-29:** `digimon_attacked_this_turn:
> you|opponent` is now a closed DSL predicate leaf, backed by engine attack
> history and consumed by ST5-04/ST5-06 inherited draw clauses. ST5-14 Tai
> Kamiya's Blocker response was expressible with the existing
> `on_attack_target_change` / `attack_target_change_reason: blocker` context
> after the blocker declaration path was corrected to suspend the blocker before
> target-change observers run. No open DSL vocabulary gap remains for ST5; full
> closure details live in [qa/resolved-gaps.md](resolved-gaps.md).

> **TS Olympos representative unlock — 2026-05-24:** The
> `close-ts-olympos-rust-gaps` change added and consumed the DSL surfaces
> needed for the representative TS Olympos deck: `materials_count_matches_aggregate`,
> `de_digivolve.amount_fn`, predicate-scoped timing suppression,
> effect-driven `use_option_from_hand`, `face_up_security_count_lte/gte`,
> and `add_bottom_security_to_hand`. These are closed for the
> representative deck and should not be re-filed as open DSL vocabulary
> gaps unless a future broad-pool card proves a distinct missing variant.

> **Xros Heart DigiXros closure — 2026-05-24:** The
> `close-xros-heart-digixros-gaps` change adds production DSL vocabulary and
> lowering for `kind: digixros` recipe paths, material zones, per-material
> cost deltas, transaction-local zone allowances, pre-attached materials,
> one-shot transaction cost deltas, and Material Save lowering from a
> DigiXros recipe. BT10-009, BT10-013, BT10-087, and BT12-112 now ship as
> pure production YAML. Remaining Xros Heart DSL work should be tracked as
> card-specific follow-up, such as BT10-111's turn-scoped DigiXros wildcard
> modifier, rather than a generic DigiXros/Material Save vocabulary gap.

> **Xros Heart reusable primitive closure — 2026-05-24:** The
> `author-xros-heart-reusable-primitives` change adds production DSL
> vocabulary and lowering for selecting cards under Tamers, placing
> hand/trash/union-zone cards under Tamers, playing selected under-Tamer
> cards for free or at reduced cost, moving filtered source cards under
> Tamers with moved-count bindings, top-N opponent stack trashing,
> sourceless-target filters, scoped DigiXros wildcard substitution, and
> effect-created attack prompts. BT21-083, BT11-095, P-224, BT19-090,
> BT21-092, BT10-111, BT21-027, and BT19-061 now ship as production YAML
> acceptance fixtures. These shapes are no longer open Xros Heart DSL
> vocabulary gaps.

> **Xros Heart reveal-play slice — 2026-05-24:** `choose_from_reveal`
> now accepts `destination: play_free`, lowering to
> `EffectContext::play_from_reveal_free` after the existing reveal pending
> selection. The selected revealed card is played without paying its cost, and
> cancellation/failed would-play replacement restores it to the reveal pool.
> `BT19-008` now uses this pure YAML route for its On Deletion reveal/play
> clause.

> **Xros Heart stack-metric and lockout slice — 2026-05-24:** The
> `complete-xros-heart-authoring-substrate` change adds DSL formula/lowering
> support for `source_color_count` both as `{ formula: { source_color_count:
> {} } }` and as `per: source_color_count` inside base/per/delta formulas,
> plus `source_stack_count` for count bounds and memory/DP math over
> predicate-matched source cards. The same slice covers permanent-scoped
> temporary lockout modifiers for `CannotActivateOnPlayEffects`,
> `CannotActivateWhenDigivolvingEffects`, and `CannotUnsuspend` with explicit
> expiry. These shapes cover the BT19-014, AD1-006, AD1-013, BT19-026,
> BT21-030, BT19-038, BT19-051, BT19-035, BT20-037, and BT19-079 fixture set
> and are no longer open Xros Heart DSL vocabulary gaps.

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. The Track E zone-movement DSL verb table (below) is
> current as of PR #454. The Track C modifier-payload schema gap is
> still the principal open DSL item; the matching engine substrate
> landed in PR #455 (typed `ModifierPayload`), so the remaining work
> is structured payload schema + parser. The `OnSuspend` /
> `name-filtered DelayTrigger` shape (BT24-089) and the bilateral
> player-scoped passive modifier shape (Rocks) remain open. See
> `docs/RUST_ENGINE_GAPS.md` for the canonical engine-side closures
> driving DSL substrate. Pre-scaling cleanup batch §2 narrative in
> `.claude/plans/pre-scaling-cleanup-batch.md`.

> **Tracker hygiene sweep — 2026-05-15:** Post-rebaseline audit cleanup
> per [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md).
> The canonical engine-side tracker `docs/RUST_ENGINE_GAPS.md` was
> compacted: ~54 closed entries moved to `qa/resolved-gaps.md`, ~10
> entries had headline severity reframed from 🔴 to 🟡 PARTIAL with
> narrowed residual titles (e.g. "Decode residual: EX10-061 Apocalymon
> batch + different-name source play DSL sugar", "Conditional
> security-in-stack trigger residual: start-of-turn variants"), and
> the at-a-glance table was rewritten. No new DSL-vocab gap entries
> surfaced; existing entries in this file remain accurate.
>
> **Tracker hygiene sweep — 2026-05-14:** Cross-referenced against PRs
> #459–#473. The Track E zone-movement DSL verb table remains current
> (no new entries). Track C modifier-payload structured YAML parser is
> still the principal open DSL item — engine substrate from PR #455
> is unchanged, parser work has not landed yet. New since 2026-05-10:
>
> - **Track H aura DSL (PR #467):** `grant_triggered_effect` step
>   (target / timing / expiry / body), `kind: aura` materialization
>   for battle-area and security-zone scopes, plus the typed
>   `AuraScope` / `AuraGrant` builder all lower through existing DSL
>   schema. Card authoring for EX1-068 Ice Wall! and BT21-095 Wind
>   Guardians is now pure DSL; no new vocabulary gap surfaced.
> - **Alter-S Ladder DSL (PR #468):** EX9-021 Omnimon Alter-S and DNA
>   Omnimon ladder cards land using existing zone-movement /
>   replacement / source-selection verbs. No new DSL verb required.
> - **Formula-threshold DSL (PR #470):** `play_cost_lte` /
>   `binding_play_cost` / `distinct_colors_count` formula leaves
>   activated for BT15-096 and BT21-102. The shape is shared with
>   level/DP/material/memory/security aggregate predicate leaves —
>   see the "Track J formula/result substrate slice (2026-05-10)"
>   paragraph in `docs/RUST_ENGINE_GAPS.md`.
> - **Puppet observer DSL (PR #472):** predicate leaves
>   `event_target_kind`, `event_target_trait_has`,
>   `event_permanent_is_source`, and `source_is_unsuspended` are
>   wired through existing lowering paths. PUPPETS-G011 closed.
>
> The `OnSuspend` / name-filtered DelayTrigger shape (BT24-089) and
> bilateral player-scoped passive modifier shape (Rocks) remain open
> from the 2026-05-10 sweep.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 rollup — Tracks A–J, PR #480):**
> The Phase 2 pilot-archetype unblock work landed as 10 tracks in PR #480
> (`claude/musing-ishizaka-c4b355` against `main`). All sub-entries below
> have been swept; the per-track sweep paragraphs that follow cover Tracks
> F and G in detail. Closure pointers for the other tracks:
>
> - **Track A** — DSL eval-arm sweep (commit `b91816b5`). Closes
>   `G-PRED-DP-LTE` (card-zone subjects via `eval_card_fields`),
>   `G-COUNT-GTE-NOT-EVALUATED`, `G-FORMULA-SOURCE-DP`,
>   `G-DSL-DISTINCT-TAMER-COLORS` + `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`
>   (paired BoolPredicate + formula), and `G-ALT-PATH-CONDITION` stale
>   placeholder-test sweep. 13+ tests un-ignored. Full entry in
>   `qa/resolved-gaps.md` § "Phase 2 Track A closure".
> - **Track B** — `activation_cost(...)` builder (commit `2c2c4632`).
>   Engine substrate: `EffectBuilder::activation_cost`,
>   `ctx.suspend_self_as_cost()`,
>   `ctx.return_self_to_deck_bottom_as_cost()`. DSL:
>   `CompiledStep::ActivationCost { kind: SuspendSelf |
>   ReturnSelfToDeckBottom }`. Cost-failure consumes OPT slot per Working
>   Rule §17. Downstream consumers: BT4-097, BT8-090, ST6-14, BT8-094,
>   EX9-068, BT13-102, RB1-035, P-136, BT22-094, BT17-093, EX11-071.
>   Full entry in `qa/resolved-gaps.md` § "Phase 2 Track B closure".
> - **Track C** — G-OPT-TRIGGERED (commit `dd9b8a46`). Substrate was
>   already correct; the gap proved phantom. 23 stale `#[ignore]`
>   annotations removed; slot-key semantics documented (per-carrier
>   HashMap keyed by `(source_card_handle, effect_slot)`, fully cleared
>   on `Permanent::new_turn`). G-OPT-RESET-VIA-ATTACK-CYCLE was a
>   test-setup misdiagnosis (deck-out before second turn cycle); fixed
>   in test files only.
> - **Track D** — G-INHERITED-DISPATCH (commit `bc852640`).
>   `enqueue_from_permanent` now walks `permanent.card_sources` so
>   inherited triggered effects fire from below-the-top cards. Stable
>   slot keying via the existing `(source_card_handle, effect_slot)`
>   shape — no OPT collision. `G-WHEN-DIGIVOLVING-DISPATCH` absorbed.
>   Regression test in `tests/timing_dispatch.rs`. 18 tests un-ignored.
> - **Track E** — Rocks pilot reveal-ordering (commit `bac197ea`).
>   Detailed in "Phase 2 Track E (2026-05-17)" §149 below.
> - **Track H** — BG Imperial substrate (commit `2b083c5a`). Closes
>   `G-BEFORE-PAY-COST-DIGIVOLVE-TARGET` (cost-target predicate +
>   `source_is_cost_target_permanent`), `G-BEFORE-PAY-COST-GAIN-MEMORY`
>   (`Effect::before_pay_cost_observe` sibling builder),
>   `G-OPTIONAL-SELECTION-CONTINUE-TAIL` (select_trash slice only —
>   other `select_*` installers remain follow-up), `G-PLAY-FROM-HAND-FREE-BIND-AS`.
>   **DEFERRED:** `G-COST-REDUCE-ALLY-DIGIVOLVE` (per Track H's discovery
>   rider; entangled with armed-observer + suspend-cost sub-gaps).
> - **Track I** — Puppets pilot (commit `26e27ccc`). Closes
>   `PUPPETS-G008` / `G-OPPONENT-SECURITY-DP-AURA` (inherited aura with
>   `applies_to_opponent_security_dp`), `PUPPETS-G009` (Delay [Main]
>   action), `PUPPETS-G003` (ProvenanceToken cleanup), end-of-attack
>   mandatory self-delete chain. EX4-074 ShineGreymon: Ruin Mode
>   IMPLEMENTED.
> - **Track J** — Royal Knights substrate + cards (commits `48fbfd76` +
>   `3a6aaee1`). Closes `RK-G001` (filtered breeding permanent target),
>   `RK-G002` (source-bound return-self cost into reduced-cost hand
>   play — leverages Track B's `activation_cost`), `RK-G003` (Delay/
>   keyword leave-prevention replacements). Plus token registry entries
>   for Atho / Rene / Por.
>
> **Net cumulative test deltas (vs. pre-wave-1 baseline post-PR #475):**
> `cards_behavioral` 2355 pass / 0 fail / 355 ignored — was ~2300 / 1
> pre-existing flake / 596 ignored. **Phase 2 killed the long-standing
> `ex11_054` Medusamon flake** as part of Track G's `[All Turns]`
> entering-permanent observer migration.
>
> See `qa/resolved-gaps.md` for full per-track closure details. The
> entries below that match these closure tags have been annotated
> inline with "RESOLVED 2026-05-17 (Phase 2 Track X)" pointers; legacy
> entry bodies are preserved for reference but the heading line carries
> the closure stamp.

> **Tracker hygiene sweep — 2026-05-20 (Puppets substrate sweep):** 15
> reusable substrate gaps closed on branch `claude/stoic-moser-0ef79e`.
> DSL-vocab entries closed in this file: `PUPPETS-G023` (BT13-101/P-136
> event-card color predicates), `PUPPETS-G024` (BT16-055 narrow
> opponent-effect protection), `PUPPETS-G025` (BT16-055 rules_text_contains
> predicate), `PUPPETS-G028` (BT22-088 return-self-to-deck-bottom cost +
> branch), `PUPPETS-G030` (BT5-106 suppress_on_play flag). All four entry
> headings below carry inline RESOLVED stamps; legacy bodies preserved for
> reference. See `docs/RUST_ENGINE_GAPS.md` and `qa/resolved-gaps.md` for
> engine-side closures.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 Track F):** Five DNA
> Omnimon DSL/substrate gaps closed; full closure summaries in
> [resolved-gaps.md](resolved-gaps.md) under "Phase 2 Track F closure":
>
> - `G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM` — new deterministic verb;
>   BT23-008 / BT23-018 production YAML authored.
> - `G-DSL-GAIN-MEMORY-FN` — formula-valued memory mutation step.
> - `G-DSL-HAS-ON-DELETION-EFFECT` — new permanent predicate
>   consulting `effects_for_card` for `OnDeletion` timing. EX1-021
>   both clauses authored.
> - `G-ALT-PATH-DIRECTION-INTO` — `AltPathSpec.direction: into` schema
>   extension + route-resolution threading. Substrate only; ST20-10
>   warp YAML pending its companion `G-DSL-DISTINCT-TAMER-COLORS`
>   predicate leaf.
> - `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` —
>   resolved as **phantom**; the chain dispatcher already worked.
>   5 tests (BT16-040 / BT17-015 / BT17-027 / BT22-013 / BT22-026)
>   un-ignored.
>
> Plus `G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA` verified as
> already-shipped upstream; regression coverage added + P-182
> [All Turns] aura authored.
>
> Still open from Track F's plan: `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH`
> (deferred — entangled with `G-SELECT-MULTI-MIN` and
> `G-ZONE-TRASH-TO-DECK` sub-gaps). EX5-015 Clause C remains BLOCKED.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 Track G):** Medusamon
> pilot completion. One new DSL predicate substrate ships
> (`G-OPP-SECURITY-COUNT-LTE`), closure summary in
> [resolved-gaps.md](resolved-gaps.md) under "Phase 2 Track G closure".
> Five Track G plan-named DSL gaps (G-EVENT-TARGET-OWNER,
> G-PLACE-SELF-AS-OPTION-PERMANENT, G-ADD-OPTION-SELF-TO-HAND,
> G-DSL-LINK-VERB, G-DSL-LINKED-SCOPE, G-MAY-ATTACK-NOW) were already
> resolved by earlier upstream substrate work; Track G's role for those
> was the test-tree sweep — stale `#[ignore]` annotations retagged from
> "BLOCKED: G-XYZ" to "card-local body not authored; substrate closed"
> across BT21-024 / BT21-025 / BT21-026 / BT21-029 / BT24-016 /
> BT24-082 / LM-055 / EX11-054. The BT21-026 deletion arm migrated to
> live YAML using `event_target_owner: opponent`; the BT21-093
> cost-reduction clause migrated from a `count_lte` aggregate over
> opponent security to the new native `opponent_security_count_lte`
> predicate (raw_rust formula `bt21_093_cost_reduction_amount`
> removed); EX11-054 [All Turns] clause migrated to Track B's
> `activation_cost: { suspend_self: true }` so the suspend-as-cost
> semantics gate the body correctly per the engine's single-trigger
> drainer model. **12 Medusamon cards advanced PARTIAL → IMPLEMENTED.**
>
> Still open from Track G's plan: G-AURA-DP-FORMULA (BT21-072 formula
> AuraBody DP), G-DELAY-SUSPEND-CONDITION (BT24-089 OnSuspend Delay),
> G-ZONE-TRASH-TO-DECK (BT24-017 trash-to-deck verb), G-AS-SELECTING-PLAYER
> (BT24-016 cross-permanent select-on-behalf), G-PRED-DP-LTE-AGGREGATE
> (BT21-093 highest-DP delete).

## Track C modifier payload YAML shape (2026-05-09) — rich payload parser pending

The Rust engine now has typed `ModifierPayload` storage and consult sites for
the deferred Track C identity/metadata modifiers:
`ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`,
`ChangeCardNamesForDigiXros`, `TreatAsDigimon`, `ChangePermanentLevel`,
`ChangeCardDP`, `ChangeOriginDP`, `ChangeSAttack`,
`ChangeEndTurnMinMemory`, `ChangeLinkCost`, and `ChangeLinkMax`. The scalar
`add_modifier` / `add_player_modifier` DSL slots can still install variants
that are representable as `value: i32`, and the modifier-name tables include
`CannotPlayFromTrash` and `OpponentCannotReduceDigivolveCost`.

Remaining DSL work: add a structured payload schema for list/string/profile
modifiers, e.g.:

```yaml
- add_modifier:
    target: source
    modifier: ChangeTraits
    payload: { add: [Holy], replace: false }
    expiry: until_leave_field
- add_modifier:
    target: source
    modifier: TreatAsDigimon
    payload:
      level: 4
      colors: [Yellow]
      traits: [Holy]
      dp: 5000
    expiry: until_leave_field
```

Until that parser lands, cards needing string/list/profile payloads should use
`raw_rust` install hooks rather than hidden scalar encodings.

## Phase 2 Track E (2026-05-17) — reveal-ordering DSL verbs landed

The author-facing residual from `G-ROCKS-REVEAL-ORDERING` has landed: two
new DSL verbs lower onto the already-shipped `select_reveal` /
`select_effect_choice` / `select_ordered_permutation` / `place_remainder_on_deck`
engine helpers. Together with the existing `reveal_top_deck` they express
the canonical "reveal N, choose 1 to hand or as source, place rest top or
bottom in any order" pattern that recurs across Rocks search effects and
every general-purpose Training / Memory Boost / search clause.

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl track_e_reveal_ordering`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_167 ex8_047 bt9_103`

| DSL verb | Engine target | Card drivers |
|---|---|---|
| `choose_from_reveal: { of, filter, destination, bind_as?, optional?, prompt }` | `EffectContext::select_reveal` + routing to `add_to_hand_from_reveal` / `return_to_deck_from_reveal` / `place_as_bottom_source` / `play_from_reveal_free` | P-167 (hand and `bottom_source_of`), EX8-047 (two sequential hand picks), BT19-008 (`play_free`) |
| `order_remainder: { of, destinations: [deck_top, deck_bottom?] }` | `EffectContext::select_effect_choice` (when two destinations) + `select_ordered_permutation` + the `place_remainder_on_deck` placement loop | P-167 (player choice), EX8-047 (single `[deck_bottom]`) |

The `destination` enum for `choose_from_reveal` accepts the bare scalars
`hand`, `deck_top`, `deck_bottom`, `play_free`, or the mapping
`bottom_source_of: { target: <binding> }` — matching the routing shapes now
needed by Rocks and Xros Heart reveal text.

Closure scope: `G-ROCKS-REVEAL-ORDERING` from
`qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md` §50 is closed.
`G-ROCKS-OPTION-SELF-DISPOSITION` §123 has its single remaining raw_rust
target removed (P-206 → native `add_this_option_to_hand`; the other five
target YAML files were already DSL-clean by 2026-05-10). The
`G-ADD-OPTION-SELF-TO-HAND` DSL entry called out in P-206 test comments is
also closed.

## Track E (2026-05-08) — engine helpers shipped, DSL verbs landed

Track E shipped 8 zone-movement helpers + the owner-routing fix at the engine layer. The ten deferred DSL verbs now parse, validate, compile, and lower into the corresponding helpers. Evidence:

- `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_zone_movement_steps`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`

| DSL verb | Engine target | Card driver |
|---|---|---|
| `place_self_at_security: { position, face }` | `place_self_at_security` | EX9-021 (top, face-up), EX4-060-style self placement |
| `place_self_option_at_security: { position, face }` | `place_self_option_at_security` | ST20-15 (top, face-up Option flavor) |
| `bounce_self: {}` | `bounce_self` | BT24-012 Dimetromon self-bounce cost shapes |
| `security_place_top_stacked_card: { carrier, of, position, face }` | `security_place_top_stacked_card` | Puppets G027 |
| `security_place_stacked_card: { carrier, source/source_index_from_top, of, position, face }` | `security_place_stacked_card` | follow-up Puppets / Mineral cards |
| `return_all_trash_to_deck_bottom: { of }` | `return_all_trash_to_deck_bottom` | BT17-077 Imperialdramon: Paladin Mode |
| `trash_top_n_digivolution_cards_of_each: { of, n }` | `trash_top_n_digivolution_cards_of_each` | BT12-028 |
| `trash_opponent_hand_to_count: { opponent, target_count }` | `trash_opponent_hand_to_count` | BT19-075 MoonMillenniummon |
| `search_own_security_stack: { filter, prompt, bind_as, on_select, on_no_match }` | `search_own_security_stack` | TS Olympos cards |
| `scheduled_delayed_return: { subject, destination, position, fire_at }` | `schedule_delayed` (substrate already exists) | BG Imperial G-BG-02 |

The remaining Track E item in this table is unrelated to the ten deferred zone-movement verbs: `scheduled_delayed_return` is still a separate BG Imperial delayed-return shape.

Format per entry:

```
## <CARD_ID> — <clause name>
- Effect text: "..."
- Missing DSL verb / step kind / predicate: ...
- Lowers to engine API: <method on EffectContext that already exists>
- Suggested DSL syntax: <YAML shape>
- First reported: YYYY-MM-DD
```

## Royal Knights — filtered breeding permanent target  [RK-G001]
- Status 2026-05-17: **CLOSED for substrate** by Phase 2 Track J PR 1.
  `SelectOwnBreedingPermanentArgs::filter: PredicateSpec` is now wired
  through compile → lowering → install: the predicate is evaluated against
  `PredicateSubject::BreedingPermanent` before `select_own_breeding_permanent`
  opens, so a non-matching breeding permanent short-circuits the step
  instead of opening a misleading prompt. The companion `BreedingPermanentRef`
  binding now resolves to a sentinel `PermanentHandle { index: BREEDING_TARGET }`,
  which `place_as_bottom_source_observed` already recognizes — so the
  printed shape "place a hand card under a [King Drasil_7D6] in breeding"
  is expressible end-to-end. Proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2g_breeding_selection::select_own_breeding_permanent_filter`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2f1_placement_steps::place_as_bottom_source_accepts_breeding_permanent_target_from_hand_source`.
  Card authoring for BT13-093 / BT20-083 / BT13-110 / EX11-053 lands in
  Phase 2 Track J PR 2.
- Status 2026-05-22: **CLOSED for optionality** by
  `close-royal-knights-substrate-gaps`. Optional
  `select_own_breeding_permanent` now exposes PASS and declines without
  running the placement/play tail; mandatory and no-candidate paths remain
  distinct. BT20-083 and BT13-110 consume this slice in active card-shaped
  tests. EX11-053 no longer blocks on hand-to-fielded-source placement; its
  residual is only the On Deletion Omnimon X hand/source play plus attach-self
  shape.
- Effect text: BT13-093: "[On Deletion] Place 1 Digimon card with the [Royal Knight] trait from your hand under a [King Drasil_7D6] in the breeding area as its bottom digivolution card." BT20-083: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area."
- DSL surface (production form):
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
      prompt: "Choose your [King Drasil_7D6]"
      then:
        - place_as_bottom_source: { source: <hand-binding>, target: kd }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights — source-bound return-self cost into reduced-cost hand play  [RK-G002]
- Effect text: EX11-071: "[Main] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your hand with the play cost reduced by 2."
- Status 2026-05-17: the **return-self-cost half** of this gap closed under Phase 2 Track B (Engine Gap: Generic `.activation_cost(...)` builder hook for triggered abilities, see `qa/resolved-gaps.md`). DSL `activation_cost: { return_self_to_deck_bottom: true }` lifts onto `EffectBuilder::activation_cost(ctx.return_self_to_deck_bottom_as_cost)`; the chained body fires after the source Tamer has left the field. The remaining **reduced-cost hand play half** is card-author DSL: stitch the existing hand selection + `play_from_hand: { cost: { reduce: 2 } }` after the activation-cost step.
- Missing DSL verb / step kind / predicate: a Main-phase activation that pays a source-bound `return_to_deck { target: source, position: bottom }` cost and then opens a player-visible hand play selection whose actual payment is reduced by 2.
- Lowers to engine API: existing source permanent binding, hand selection, and pay-cost flow need a reusable action/pending-selection wrapper so the return cost and reduced play payment stay one legal choice.
- Suggested DSL syntax:
  ```yaml
  - when: main
    optional: true
    pay_cost:
      - return_to_deck: { target: source, position: bottom }
    process:
      - select_hand:
          bind_as: played
          filter:
            all_of:
              - play_cost_gte: 4
              - any_of:
                  - trait_has: "Royal Knight"
                  - trait_has: LIBERATOR
          prompt: "Play a cost 4+ Royal Knight/LIBERATOR"
      - play_from_hand:
          target: played
          cost: { reduce: 2 }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights full pool pass — residual reusable DSL/engine gaps  [RK-G005]
- Status: PARTIAL pool pass completed on 2026-05-05. The Royal Knights resolver pool has 72 unique cards and now has 72 Rust DSL YAML entries. Fully unsupported clauses were left as explicit YAML comments plus ignored Rust tests instead of hidden approximations.
- Newly routed or reaffirmed blocked cards/clauses: `BT13-019`, `BT13-030`, `BT13-075`, `BT13-087`, `BT13-102`, `BT13-111`, `BT13-112`, `BT15-092`, `BT17-077`, `BT19-093`, `BT20-017`, `BT20-021`, `BT20-045`, `BT20-056`, `BT22-025`, `BT22-041`, `BT22-052`, `BT23-013`, `BT23-035`, `BT23-047`, `BT23-057`, `BT23-072`, `EX8-073`, `EX10-068`, and `EX11-053`.
- Missing DSL/engine areas: broader union selection across hand/trash/breeding/source stacks with uniqueness/name-exclusion filters; union hand/trash source-placement costs; opponent hidden-hand choices; result-dependent fallback branches; combined trash/security/color/source-count formulas; card-specific post-Blast-DNA effect bodies after the covered field+hand-material Counter path (`BT17-078`, `BT20-045`, `BT20-060`, `BT20-076`, `BT20-081`, `EX6-011`, `EX6-029`); residual native `<Blast Digivolve>` helper APIs; Option battle-area carrier lifecycle for non-Delay options; security-trash self-dispatch; security search/play card-local follow-up bodies; security-removed card-local follow-up shapes beyond the now-wired battle/effect `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` timing payloads; generalized source-list Partition lowering beyond authored card clauses; and unusual replacement/security-trash costs tied atomically to prevention. **Updated 2026-05-20 (Track J S1.2 + S1.3):** count-capped / name-unique multi-pick play from a carrier's digivolution sources is now FULLY CLOSED via the `select_materials` DSL step + batch `play_from_materials` (see `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`); S1.3 closed the final breeding-carrier residual (King Drasil's resident stack) by appending the `BREEDING_SOURCE_SELECT` action sub-range (`ACTION_SPACE_SIZE` 2168→2192). `when: on_place_security`, alias `when: on_added_to_security`, `when: on_discard_security`, and the printed-text alias `when: on_any_digimon_played` are now wired as of 2026-05-08 with event-card/effect-cause payloads where applicable. Immediate may-attack / force-attack / cancel-attack / open-counter-window prompts are now covered by the Track D DSL verbs listed below. **Track E (2026-05-09)** shipped DSL verbs for self-to-security, Option-self-to-security, stacked-card-to-security, bulk trash/deck movement, forced hand reduction, self-bounce, permanent-to-security observed movement, and security-stack search; remaining card-side work is called out under the narrower per-card gaps below. **Updated 2026-05-20 (Track J S2.1):** the ally-played may-attack observer shape (`G-ALLY-PLAYED-MAY-ATTACK`, BT20-017 / BT23-013) was filed and closed as already-composable — `may_attack_now` accepts `attacker: this | event_target | <named binding>`, so the printed "this Digimon may attack" / "1 of your Digimon may attack" clauses lower from landed primitives; see that entry above. **Updated 2026-05-20 (Track J S2.2):** the Jesmon-family hand/trash name-excluded play (`G-UNION-HAND-TRASH-NAME-EXCLUSION`, BT23-013) is now RESOLVED — the DSL `select_union_zone` lowering now applies its `filter` (it previously dropped it), and a new `name_not_shared_by_field_digimon` predicate leaf models "can't play cards with the same names as any of your Digimon"; see that entry below. Step 0 against printed text corrected the plan premise: BT20-017 has no union play, BT13-019 plays from trash-or-breeding-sources (separate gap), and BT20-021 *places* a source as a cost (separate gap). **Updated 2026-05-20 (Track J S2.3):** the last Royal Knights token, Hinukamuy (BT23-057 Gankoomon), is now registered in `code/digimon-engine/src/token_registry.rs` with its printed stats — Digimon/White/6000 DP/`<Alliance> <Reboot> <Blocker>` — mirroring the Atho/René/Por registration. Token registration for Atho/René/Por and Hinukamuy is now FULLY CLOSED; the remaining BT23-057 work (multi-card trash-to-deck cost reduction, dynamic play-cost delete) is unchanged.
- Updated 2026-05-22 (`close-royal-knights-substrate-gaps`): optional breeding-permanent selection, BT17-018 DP-budget delete migration, BT13-112 source-play payoff, BT13-110 source-placement/Delay source play, BT20-083 optional breeding tuck and inherited source play, BT20-017 token/delete/may-attack observer, BT23-072 hand-main/source-play/played-Digimon keyword grants, BT23-013 token-or-Sistermon branch plus may-attack observer, and EX11-053 On Play hand-to-fielded-King-Drasil source placement now have production YAML plus focused behavioral coverage. Residual Royal Knights blockers remain capability-centric: `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (`BT13-019`), `G-UNION-HAND-TRASH-SOURCE-COST` plus source-count/security formulas (`BT20-021`), BT23-057 multi-card trash-to-deck cost reduction and dynamic play-cost delete, BT23-058 self-scoped on-suspend plus aggregate lowest play-cost delete-all, and EX11-053 On Deletion union hand/source play plus attach-self.
- Workaround policy: no approximations were used for these blockers. If a printed clause required one of the missing primitives, the YAML either implemented an independent faithful slice such as a keyword/security play/simple trigger, or used a load-only gap stub.
- Verification: targeted `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_filter> --nocapture` passed for the final 25 filters, with one active load test and one ignored gap test per card.
- First reported: 2026-05-05 Royal Knights full pool implementation pass.

## Rocks pool pass residual DSL/engine gaps
- Status: PARTIAL pool pass completed on 2026-05-04. After pulling main, production YAML/test slices now exist for 40 of 47 Rocks pool cards; the remaining 7 were explicitly routed as blocked rather than no-op authored.
- Remaining blocked cards: `BT9-103`, `EX8-070`. `EX10-003` moved to production YAML/test coverage on 2026-05-08. `P-130`, `EX11-065`, `EX11-038`, `BT20-055`, `BT23-096`, `BT8-094`, `BT23-059`, `EX11-044`, `EX10-034`, and `EX8-050` now have production YAML/test coverage for the slices closed or verified by `complete-rocks-archetype`.
- Missing DSL/engine areas: Save/Xros routing; source placement from hand/trash; lowest-play-cost delete; and same-side/costed `[When Moving]` follow-up shapes beyond the resolved base OnMove timing.
- First reported: 2026-05-04 Rocks pool implementation pass.

## Zephagamon / Vortexdramon — remaining battle-engine prep gaps
- Status: partial readiness slice added 2026-05-03. `EX11-074.yaml` now covers static `<Piercing>`, `<Vortex>`, `<Blocker>`, and a focused `battle:` pathway. The regression in `tests/cards_behavioral/ex11/ex11_074.rs` proves that an effect battle deletes the defender through DP battle but is not an attack: it must not trigger Piercing/security and must not leave `pending_attack` populated.
- Rule boundary: `battle:` is the correct DSL step for effects that say a Digimon battles another Digimon. Do not model these as `attack` or force-follow-up attack effects. Attack-only timings and Piercing security continuation remain tied to declared attacks, not effect battles.
- EX11-074 remaining gap: the printed "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon..." branch needs a binding/condition result from the suspend step. The DSL can select and suspend, but cannot yet branch on "this effect suspended your Digimon" and bind that cost/result into the follow-up +6000 DP and immunity-until-opponent-turn-ends clause.
- EX11-074 remaining gap: full `[All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. Then, this Digimon may battle 1 opponent Digimon` still needs faithful optional trigger ordering and the unsuspend-then-optional-battle branch. The readiness fixture keeps the battle path focused instead of auto-implementing the whole printed clause.
- BT20-101 remaining gap: Zephagamon needs a formula that counts suspended Digimon, divides that count by 2, and uses the capped result as the number of opponent Digimon selected to place at the bottom of the deck. Existing count-capped multi-select support needs this suspended-count / division formula vocabulary and bottom-deck target movement wiring for the full clause.
- EX11-035 remaining gap: the green Avian/Bird play effect needs a formula DP cap for the target card. The DSL needs a predicate/formula shape that computes the allowed play target's DP ceiling from the printed condition rather than a fixed literal.
- EX11-062 remaining gap: the card needs a conditional `VortexCanAttackPlayer` aura while the opponent has no unsuspended Digimon. The engine now has the `VortexCanAttackPlayer` modifier type and the runtime `Expiry::UntilCondition` continuous controller, but the DSL still needs aura/active_when lowering that attaches the compiled BoolPredicate to the modifier entry's `until_condition` field.
- Gap kind: hybrid. Some engine primitives exist (`battle:`, static keyword grants, `ModifierType::VortexCanAttackPlayer`), but the remaining Zephagamon clauses need DSL result bindings, formulas, conditional aura lowering, and card-specific faithful branch wiring.
- First reported: 2026-05-03 (Zephagamon Battle Engine Prep Task 4)

## BT22-098 / P-229 — event-gated Delay activation windows — RESOLVED 2026-05-21
- Effect text: BT22-098: "[Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> ... 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3." P-229: "[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay> ... 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- Status: **closed** 2026-05-21 (gap id `PUPPETS-G004`). The BT22-098 `on_suspend`
  slice closed 2026-05-02; the P-229 `on_ally_played` slice closed 2026-05-21.
- Closed via (two halves):
  - DSL: `code/digimon-engine/src/dsl_cards/lower_delay.rs` now maps
    `CompiledTiming::OnAllyPlayed` → `DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed)`
    alongside the existing `on_suspend` / `on_unsuspend` arm.
  - Engine: `code/digimon-engine/src/effect_queue.rs` `enqueue_triggered` now fans
    `TriggerSource::EnteredField` dispatches out to
    `enqueue_event_gated_delayed_options` (previously only `EventObserved` /
    `AttackTargetChanged` reached it).
- Working DSL syntax (P-229 production YAML, `cards/p/P-229.yaml`):
  ```yaml
  - kind: delay
    trigger: on_ally_played
    active_when:
      your_turn: true
      event_target_owner: you
      event_card_name_contains: "Mirai Kinosaki"
    process:
      - select_own_permanent:
          bind_as: target
          optional: true
          filter: { all_of: [ { kind: digimon }, { zone: [battle_area] } ] }
      - select_hand:
          of: you
          bind_as: evo
          optional: false
          filter:
            all_of: [ { kind: digimon }, { trait_has: LIBERATOR }, { level_lte: 6 } ]
      - effect_initiated_digivolve:
          target: target
          from_hand: evo
          cost: { reduce: 3 }
          ignore_requirements: false
  ```
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_229` — 13 tests pass, 0 ignored.
- First reported: 2026-04-28 (Puppets archetype assessment); resolved 2026-05-21.

## EX9-032 / EX7-027 / BT22-036 — replacement cause predicate and `active_when` lowering
- Effect text: "[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving."
- Status: PARTIALLY RESOLVED on 2026-05-03. Replacement clauses now preserve replacement subject/source/cause predicates through lowering, apply `active_when`, and can protect a different subject than the replacement source. This is verified for `BT24-040`/`BT24-101`-style TS protection and `BT17-097` Delay replacement continuation.
- Updated 2026-05-06 (Track B): replacement timing vocabulary now includes named pre-move triggers `when_would_digivolve`, `when_would_play`, and `when_would_link`, mapping respectively to `EffectTiming::WhenPermanentWouldDigivolve`, `EffectTiming::WhenPermanentWouldPlay`, and `EffectTiming::WhenWouldLink`. Mandatory cancel dispatch is covered at the engine fire-sites; optional `Card`-subject accept/decline resume remains an engine follow-up before optional DSL card text should target these windows.
- Updated 2026-05-08 (Track B): inherited replacement dispatch now scans buried source effects, and the Puppet/token cost body is live for `BT22-036`, `EX11-022`, `EX9-032`, `EX7-027`, and `ST19-11`. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_036_inherited_replacement`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_022_inherited_leave_prevention`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_032_inherited_prevents`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex7_027_inherited`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st19_11_inherited`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- Remaining missing DSL/card work: none for inherited Token/Puppet leave-prevention dispatch itself; adjacent active-effect gaps on those cards remain independently tracked.
- Lowers to engine API: replacement evaluator context plus `EffectContext` replacement outcome setters such as `cancel_leave`.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      replacement_cause_not: own_effect
    process:
      - select_own_permanent:
          as: cost
          filter:
            any_of:
              - kind: token
              - trait_has: Puppet
            other_than_source: true
      - delete_permanent: { target: cost }
      - cancel_replacement: {}
  ```
- Gap kind: partially resolved hybrid. The reusable replacement-context predicate/lowering slice is closed; unimplemented card bodies remain card-authoring work unless they surface new reusable primitives.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 bt24_101 --nocapture`; named pre-move vocabulary proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- First reported: 2026-04-28 (Puppets archetype assessment)

## BT24-080 — delete all opponent Digimon with the lowest level
- Status: PARTIALLY RESOLVED for the reusable lowest-level permanent predicate on 2026-05-02. `CompiledPredicate::level_matches_aggregate` can match permanents whose top card level equals `CompiledAggregateSelector::LowestLevel` for a player scope, skipping Tamers/Options with no top-card level. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon`.
- Effect text: "[On Play] [When Digivolving] [On Deletion] Delete all of your opponent's lowest level Digimon."
- Remaining DSL verb / step kind / predicate: card-specific authoring still needs to wire the aggregate predicate through the surrounding delete-all flow. Repeat target-selection blockers elsewhere are unrelated and remain open.
- Lowers to engine API: engine-side iteration over opponent battle-area permanents plus `delete_permanent` is sufficient once the minimum-level candidate set can be computed.
- Suggested DSL syntax:
  ```yaml
  - delete_all:
      of: opponent
      filter:
        kind: digimon
        level_is: { aggregate: minimum, over: opponent_battle_area }
  ```
- First reported: 2026-04-28

## Rocks archetype refresh — source-selection and cost-payment DSL surface  [G-ROCKS-SOURCE-SELECTION-DSL]
- Effect text: Rocks core repeatedly uses "by trashing any 1/3 [Mineral] or [Rock] trait card(s) from your Digimon's digivolution cards" and "place up to N [Mineral]/[Rock] cards from your trash as bottom digivolution cards." Examples: `EX10-032`, `P-167`, `EX10-036`, `EX10-033`, `EX8-055`, `EX10-028`, `EX8-070`, `EX10-025`.
- Missing DSL verb / step kind / predicate: First-class source-zone selectors for digivolution cards across all of your own stacks, including exact-N, up-to-N with PASS terminator, and single-pick forms. Current DSL has `place_as_bottom_source` and `trash_top_source`, but no `select_source_across_own_permanents` / `select_n_sources_across_own_permanents` step that can bind `(PermanentHandle, source_index)` choices and then trash/place exactly the selected cards.
- Companion engine gap: `docs/RUST_ENGINE_GAPS.md` tracks the engine half under "Cross-permanent count-capped multi-select" and the cost-ordering half under "`.pay_cost()` builder hook for triggered non-cost-reduction effects." This entry tracks the YAML vocabulary and lowering shape that should sit on top of those primitives once available.
- Lowers to engine API: proposed `ctx.select_source_across_own_permanents(...)`, `ctx.select_n_sources_across_own_permanents(...)`, and `EffectBuilder::pay_cost_trash_n_own_sources_by_trait(...)`.
- Suggested DSL syntax:
  ```yaml
  - pay_cost:
      select_sources:
        of: you
        from: any_own_digimon
        count: 1
        filter:
          any_of:
            - trait_has: Mineral
            - trait_has: Rock
        bind_as: trashed_sources
      then:
        - trash_selected_sources: trashed_sources
  ```
  Up-to-N variants should use `max_count: 3` and surface PASS as a legal terminator so RL sees the "stop selecting" choice.
- Gap kind: hybrid (engine selection/action support is still required; DSL needs the reusable vocabulary and lowering once that lands).
- Workaround: Do not auto-pick sources. The Rocks assessment on 2026-04-28 found this to be the core no-approximations blocker for the archetype.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — event-card predicates for Mineral/Rock observers  [G-ROCKS-EVENT-CARD-PREDICATES]
- Effect text: Rocks Tamers and inherited effects gate on the card or host involved in a just-fired event, for example "when any of your Digimon digivolve into a [Mineral] or [Rock] trait Digimon" (`EX8-067`) and "when effects trash digivolution cards of any of your [Mineral] or [Rock] trait Digimon" (`EX10-063`, `P-169`, `EX11-065`).
- DSL predicate coverage: reusable predicate leaves for `trashed_source_trait_has`, `trashed_source_card_id_is`, and `host_permanent_trait_has` are implemented for event payloads with host/source context. Broader aliases such as `digivolving_card_trait_has` remain vocabulary work if card authors need that spelling; existing source-relative leaves such as `source_permanent_trait_has` are not enough unless the lowering receives the correct event subject and distinguishes observer permanent, host permanent, and trashed source card.
- Companion engine gap: the engine still needs full `OnDigivolutionCardTrashed` fan-out with host/source context; see `docs/RUST_ENGINE_GAPS.md` "OnDigivolutionCardTrashed observer timing" and related Rocks entries.
- Updated 2026-04-29: the OnDigivolve half now has runtime event-card and event-target context for normal `Game::digivolve_from_hand`; `event_card_trait_has` reads the new top card, and `target: event_target` binds the just-digivolved permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`.
- Updated 2026-04-29: `Game::return_to_hand` source disposition now carries `event_card` / `event_source_card` for the trashed source and `event_host_card` for the former host top card, so `event_card_trait_has` can match sources trashed by that path. Runtime `event_host_permanent()` only exposes the stored host handle if it still resolves to that same card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source source_trash_host_context_does_not_alias_shifted_permanent` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source`. Remaining source-trash gaps include cross-permanent source selection, source-trash paths other than `return_to_hand`, and first-class DSL leaves for trashed-source / host-permanent predicates.
- Updated 2026-05-02: first-class predicate leaves now compile for `event_target_owner`, `host_permanent_trait_has`, `trashed_source_trait_has`, and `trashed_source_card_id_is`; runtime coverage exercises `TriggerSource::SourceTrashedFromStack` with live host/trashed-source context. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context`. Remaining source-trash producer paths not covered here should stay open until each producer proves it supplies host/source context rather than relying on fallback guessing.
- Updated 2026-05-03: Task 6 audit found the reusable source-trash payload and DSL predicate leaves already implemented. Added focused regression coverage that an actual `EffectContext::trash_card_source` producer supplies the exact trashed source card and live host into `trashed_source_trait_has`, `trashed_source_card_id_is`, `host_permanent_trait_has`, and `event_target_owner`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context_bindings group6_dynamic_formulas group7_predicate_batch --nocapture`. No new event payload, predicate, formula, action, or tensor primitive was added.
- Updated 2026-05-07: Return-to-deck source disposition and de-digivolve now emit `TriggerSource::SourceTrashedFromStack` through `Game::fire_digivolution_card_trashed(...)`, including cause and moved-card payload data. `host_permanent_trait_has` now falls back to the event host-card snapshot after the host leaves the battle area. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_return_to_deck_carries_host_and_trashed_source on_digivolution_card_trashed_de_digivolve_carries_host_and_trashed_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_051_inherited_source_trash_dedigivolves_after_host_return_to_deck`. Remaining source-trash DSL work is producer/card-local for additional source-trash cost shapes.
- Updated 2026-05-07: `select_own_sources` now accepts `target: <binding-ref>`, so inline source costs can be restricted to the activating permanent (`target: source`) rather than all own stacks. BT4-072 proves exact-N Digi-Burst authoring with a target-scoped source selection, `trash_selected_sources`, and the follow-up DP target choice. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_sources`.
- Updated 2026-05-07: `digi_burst` is now a reusable DSL step that lowers to the canonical self-source exact-N selection and inserted trash-cost step before the nested body. BT4-072 now uses this wrapper, and printed keyword parsing carries `Keyword::DigiBurst(N)`. Covered by `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_source_selection_steps` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parser_digi_burst_parametric`.
- Updated 2026-05-08: `digi_burst` now has a count-2 regression fixture proving exact-N self-stack masking, no PASS before the required count, per-selected-source `OnDigivolutionCardTrashed` emission, and continuation into the nested body after the source-trash cost. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digi_burst_two_selects_exact_self_sources_and_fires_source_trash_per_card`.
- Lowers to engine API: `TriggerContext` / event payload fields containing `{host_permanent, trashed_card, trashed_source_index, cause_player}` plus predicate evaluation against those fields.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - host_permanent_trait_has: Mineral
      - trashed_source_trait_has: Rock
  ```
  Trait alternatives should compose through existing `any_of`.
- Gap kind: hybrid (requires engine event context plus DSL predicate leaves).
- Workaround: None faithful. Scanning trash after the fact loses which source card was trashed from which host, and can trigger the wrong inherited card.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — authored YAML coverage note
- Assessment target: the `Rocks` / `RockClose` archetype in `data/deck_library.json`, refreshed on 2026-04-28.
- Finding: as of the 2026-05-04 Rocks batch plus the pulled main updates, 40 of 47 Rocks pool cards have Rust YAML under `code/digimon-engine/cards/`. New Rocks pass coverage added or audited the `EX8`/`EX10`/`EX11`/`P-167` shell; the remaining missing cards are tracked in the residual gap entry above.
- Existing DSL gaps reaffirmed by the refresh:
  - `EX11-008 — [When Moving] timing` no longer blocks on the `on_move` token or moved-card event context as of 2026-04-29; card bodies may still need separate target-selection, reveal, or follow-up action primitives.
  - `P-189 — play cost <= filter` was closed on 2026-05-01 for static `play_cost_lte` filters on `select_hand` / `select_trash`; remaining Rocks blockers are tracked separately.
  - `P-206 — Board-color cross-reference predicate` was closed on 2026-05-02 for dynamic `color_matches_any_field_digimon` card predicates; any remaining P-206 Delay, Option, or action-flow blockers are separate.
  - `P-107 — place_self_as_delay_option` remains relevant to `P-107`, `P-039`, `BT23-096`, and related Delay/security disposition effects.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## BT22-015 — grant "this Digimon may attack" after When Digivolving
- Status: RESOLVED for the immediate printed follow-up attack (2026-05-08). `may_attack_now` is available in YAML and lowers to the centralized attack-open flow with PASS exposed through pending selection. BT22-015 uses this for "Then, this Digimon may attack."
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- may_attack_now_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_037`.
- Additional Track D coverage: BT24-037 Silphymon uses the same `may_attack_now` step after its shared On Play/When Digivolving -5000 DP selection, proving the TS Olympos "1 of your Digimon may attack" branch with PASS before attack commitment.
- Effect text: "[When Digivolving] ... Then, this Digimon may attack."
- Previous missing DSL verb / step kind / predicate: `ModifierType::MayAttack` / immediate attack permission was not exposed by the DSL modifier map, and there was no declarative step that lowered to the engine's attack-permission helper once the effect resolved.
- Lowers to engine API: `ModifierType::MayAttack` / `ModifierType::CanAttackUnsuspended` or the force-follow-up attack helper tracked in `docs/RUST_ENGINE_GAPS.md`.
- Supported DSL syntax for the resolved immediate prompt: `may_attack_now: { attacker: source, targets: any, optional: true }`. Persistent attack-permission grants remain a separate modifier/aura problem.
- First reported: 2026-04-28

## Royal Knights — ally-played may-attack observer  [G-ALLY-PLAYED-MAY-ATTACK]
- Status: **RESOLVED / already-composable on 2026-05-20** (Phase 2 Track J Task S2.1). Filed and closed in the same pass — this gap previously had no canonical entry, only a name in the Royal Knights `RK-G005` rollup. No engine or DSL code change was needed. Full resolution detail in [qa/resolved-gaps.md](resolved-gaps.md#engine--dsl-gap-g-ally-played-may-attack--already-composable-2026-05-20-phase-2-track-j-task-s21).
- Card consumers: `BT20-017` Jesmon, `BT23-013` Jesmon.
- Effect text: BT20-017 — "[Your Turn] [Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of your Digimon may attack." BT23-013 — "[Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack."
- Step 0 finding: `may_attack_now` is NOT hard-bound to `self`. Its `attacker:` is a `BindingRef`; the lowering (`combat.rs::resolve_permanent_ref` → `binding_ref.rs::resolve_binding_ref`) already resolves `event_target` to the event-played permanent (`CompiledBindingRef::EventTarget` ← `TriggerSource::EnteredField.event_permanent`), as well as `this` (`Source`) and any named `bind_as` binding (`Binding`). The printed text moreover differs from the original substrate-plan premise: BT23-013 grants the attack to "**this** Digimon" (the observer source, `attacker: this`) and BT20-017 to "**1 of your** Digimon" (a player choice — `select_own_permanent` `bind_as` then `attacker: <binding>`); neither uses the event-played Digimon as the attacker. All three attacker shapes were already composable from primitives landed on/before BASE.
- Supported DSL syntax: `may_attack_now: { attacker: event_target, targets: any, optional: true }` (event-played Digimon), `attacker: this` (observer source), or `attacker: <named binding>` from a prior `select_*` step. PASS surfaces through `build_action_mask` for the optional `may` (§17).
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- effect_granted_attack` (15 pass — incl. `may_attack_now_event_target_yaml_lowers_to_event_target_binding`, `may_attack_now_event_target_grants_attack_to_event_played_digimon`, `may_attack_now_event_target_decline_branch_starts_no_attack`, `may_attack_now_this_vs_event_target_select_different_attackers`, `may_attack_now_event_target_respects_summoning_sickness`).
- Card-authoring status 2026-05-22: production `BT20-017.yaml` and `BT23-013.yaml` now consume this primitive in active behavioral tests. This entry remains only as the reusable substrate closure record.
- First reported: 2026-05-05 (Royal Knights full pool implementation pass, as part of `RK-G005`).

## Royal Knights — Jesmon-family hand/trash name-excluded play  [G-UNION-HAND-TRASH-NAME-EXCLUSION]
- Status: **RESOLVED on 2026-05-20** (Phase 2 Track J Task S2.2). Two genuine substrate pieces were missing and are now closed. Filed and closed in the same pass — this gap previously had no canonical entry, only a name in the Royal Knights `RK-G005` rollup and in the BT23-013 test ignore string. Full resolution detail in [qa/resolved-gaps.md](resolved-gaps.md#engine--dsl-gap-g-union-hand-trash-name-exclusion--resolved-2026-05-20-phase-2-track-j-task-s22).
- Card consumers: `BT23-013` Jesmon is the **only** genuine consumer of this exact "hand OR trash, name-restricted, exclude names already in play" shape. The substrate-plan premise also named BT20-017 / BT13-019 / BT20-021, but Step 0 against printed text found those are different mechanics — see "Plan-premise correction" below.
- Effect text (BT23-013, the genuine consumer): "[When Digivolving] [When Attacking] You may play 1 [Atho, René & Por] Token (…) or, **from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.**"
- Step 0 finding — what was actually missing:
  - (a) The DSL `select_union_zone` step (hand+trash in one prompt) carried a `filter: PredicateSpec`, but the engine lowering `install_select_union_zone` in `code/digimon-engine/src/dsl_cards/step/selections.rs` passed a hardcoded `|_game, _card| true` accept-all closure and **dropped the compiled filter entirely**. The engine helper `EffectContext::select_union_zone` itself already applies whatever filter it is given (proven by `tests/selection/union_zone.rs::filter_restricts_valid_action_ids`) — so this was a DSL-lowering bug, not an engine-helper gap. Name-restriction (`name_contains: Sistermon`) was silently inoperative for every union-zone card.
  - (b) No predicate leaf could express "this candidate card's name is NOT shared by any of my battle-area Digimon". The existing `no_permanent` existential matches against fixed predicate fields and cannot reference the candidate card's own name; `color_matches_any_field_digimon` was the closest analog but for colors.
- What was added:
  - `name_not_shared_by_field_digimon: { of: <player> }` — a card-subject predicate leaf. True when no battle-area Digimon of the scoped player has the candidate card's effective name (field names read via `synth_identity`, so a `ChangeBaseCardName` overlay on a field Digimon is respected; the candidate's own name respects a reveal overlay). Exact, case-sensitive comparison, consistent with `name_is` / `name_in`.
  - The `select_union_zone` lowering now builds an `EffectReadContext` per candidate and evaluates the compiled `filter` against each hand/trash `CardSource`, exactly as `install_select_hand` / `install_select_trash` already did.
- Supported DSL syntax:
  ```yaml
  - select_union_zone:
      of: you
      zones: [hand, trash]
      optional: true            # printed "You may …" — PASS stays legal (§17)
      prompt: Play 1 Sistermon from hand or trash
      filter:
        all_of:
          - name_contains: Sistermon
          - name_not_shared_by_field_digimon: { of: you }
  ```
  The name-exclusion shapes the legal action mask — every surviving candidate from hand AND trash surfaces through `pending_selection`; it never auto-picks.
- Plan-premise correction: the substrate plan named four cards. Printed text (Step 0) shows only BT23-013 matches the "hand+trash + own-name-exclusion" shape. **BT20-017** has no union play at all (only an Atho/René/Por token play). **BT13-019** Gankoomon plays from *trash OR a breeding-area Digimon's digivolution sources* with a *fixed* name-exclusion (`Gankoomon` / `Omnimon`) — a genuinely distinct gap, now filed canonically as `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (see entry below), untouched here. **BT20-021** Jesmon GX *places* a Royal Knight card from hand/trash as a digivolution source — a **cost**, not a play, with no name-exclusion — also a distinct gap, now filed canonically as `G-UNION-HAND-TRASH-SOURCE-COST` (see entry below). Both spin-off IDs were discovered during this task's Step 0 and, per the Discovery rider, are filed as their own canonical tracker entries rather than left as narrative-only descriptions here. "Union" is informal shorthand: no printed `<Union>` keyword exists (verified absent from `docs/RULES_CONTEXT.md`).
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- s2_2_union` (2 behavioral tests: `union_zone_filter_excludes_in_play_name_across_hand_and_trash`, `union_zone_filter_keeps_all_sistermon_when_field_empty`); `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- name_not_shared_by_field_digimon` (lowering test) and `-- parse_leaf_predicates` (parse test).
- First failing test (TDD red): `union_zone_filter_excludes_in_play_name_across_hand_and_trash` — before the fix it offered 3 candidates (filter dropped, `name_not_shared_by_field_digimon` silently swallowed into `PredicateSpec::extra`) instead of the 2 legal Sistermon names.
- Card-authoring status 2026-05-22: production `BT23-013.yaml` now implements `<Rush>`, `<Alliance>`, the token-or-Sistermon effect choice, hand/trash Sistermon filtering, and the other-Digimon-played may-attack observer with active card-shaped coverage. This entry remains only as the reusable substrate closure record.
- First reported: 2026-05-05 (Royal Knights full pool implementation pass, as part of `RK-G005`).

## Royal Knights — Gankoomon trash-OR-breeding-source play with fixed name-exclusion  [G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY]
- Status: **BLOCKED**.
- Card consumer: `BT13-019` Gankoomon is the genuine consumer of this dual-zone "trash OR a breeding-area Digimon's digivolution sources" play shape with a *fixed* name-exclusion.
- Effect text (BT13-019, verified against `data/cards.json`): "＜Blocker＞ (This Digimon can block in the blocker timing.) [On Play] [When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your trash or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect."
- What DSL/engine surface is missing:
  - A single optional play prompt that draws candidates from *two heterogeneous sources at once* — the player's trash AND the digivolution cards (sources) of Digimon in the player's breeding area — each half carrying its own filter (`name_contains: Sistermon` for the trash half; `trait_has: "Royal Knight"` for the breeding-source half).
  - A *fixed*-name exclusion applied across both halves (`name_in`-style: can't play `[Gankoomon]` or `[Omnimon]`). This is distinct from `name_not_shared_by_field_digimon` (Task S2.2), which is a dynamic exclusion against the names of your field Digimon; here the excluded names are printed literals.
- The breeding-area-source half is **already covered** by the resolved gap `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` (`select_materials` / `select_material` against breeding-area carriers, closed 2026-05-20 Track J S1.3 — see that entry below). The residual unique to this gap is therefore: (1) the *dual-zone* selection that unions a trash filter with a breeding-area-source filter in one player-visible prompt, and (2) the *fixed* `name_in`-style exclusion gating that combined candidate set.
- Plan-premise note: this card was named in the original substrate plan for `G-UNION-HAND-TRASH-NAME-EXCLUSION` but Step 0 against printed text showed it is a different mechanic; it is filed here as its own canonical entry per the Discovery rider.
- First reported: 2026-05-20 via S2.2 Step 0.

## Royal Knights — Jesmon GX hand/trash digivolution-source placement cost  [G-UNION-HAND-TRASH-SOURCE-COST]
- Status: **BLOCKED**.
- Card consumer: `BT20-021` Jesmon GX is the genuine consumer of this "place a card from hand or trash as a digivolution source" *cost* shape.
- Effect text (BT20-021, verified against `data/cards.json`): "[Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this card without paying the cost.) [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this Digimon. [When Attacking] [Once Per Turn] This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash your opponent's top security card." (The clause relevant to this gap is the second one: "By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete …".)
- What DSL/engine surface is missing: a *cost* (not a play) that requires the player to select 1 card matching a filter (`trait_has: "Royal Knight"`) from a union of two zones — hand OR trash — and place it as the **bottom** digivolution card of the source Digimon, as the price of activating the rest of the effect. There is **no name-exclusion** on this clause. Distinct from `G-UNION-HAND-TRASH-NAME-EXCLUSION` (a *play* with a dynamic own-name exclusion) and from `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (a *play* with a fixed name-exclusion, trash-OR-breeding zones): this is a hand-OR-trash *cost* that places the selected card as a bottom digivolution source rather than playing it.
- Plan-premise note: this card was named in the original substrate plan for `G-UNION-HAND-TRASH-NAME-EXCLUSION` but Step 0 against printed text showed it is a different mechanic (a cost, not a play); it is filed here as its own canonical entry per the Discovery rider.
- First reported: 2026-05-20 via S2.2 Step 0.

## BT22-015 — count same-level pairs in own stack
- Status: RESOLVED on 2026-05-07. `CompiledPerSelector::SameLevelPairsInSources` counts source cards below the top card by level and sums `count / 2` per level bucket; `select_count_capped_multi.max` now accepts `{ formula: ... }`; and the DSL wrapper supports `zone: battle_area` to bind a `PermanentList` for `per_selected`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregate_formula_reads_source_levels phase2d_select_count_capped_multi` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair`.
- Effect text: "[When Digivolving] For every 2 cards with the same level in this Digimon's digivolution cards, return 1 of your opponent's Digimon to the bottom of the deck."
- Former missing DSL verb / step kind / predicate: repeat-count target selection derived from a formula.
- Lowers to engine API: stack inspection plus repeated `return_to_deck(..., DeckEnd::Bottom)` after each player-visible target selection.
- DSL syntax: `select_count_capped_multi: { zone: battle_area, max: { formula: { base: 0, per: same_level_pairs_in_sources, delta: 1 } }, ... }` followed by `per_selected` over the bound permanent list.
- First reported: 2026-04-28

## BT17-078 — bottom-deck all opponent Digimon sharing chosen level
- Status: RESOLVED on 2026-05-07. The DSL now supports `bind_permanent_property` for selected permanent properties and `level_eq_binding` for later permanent/card predicates; BT17-078 uses this to bind the chosen opponent Digimon's level, for-each every opponent Digimon with that level, bottom-deck them, then surface the mandatory delete prompt. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_bind_permanent_level_property_step bind_permanent_level_filters_for_each_same_level_permanents` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078`.
- Effect text: "[On Play] [When Digivolving] ... place all of your opponent's Digimon with the same level as 1 of their Digimon at the bottom of the deck."
- Former missing DSL verb / step kind / predicate: Binding one selected opponent Digimon's level and applying a mass same-level filter to every opponent permanent. Closed by `bind_permanent_property` plus `level_eq_binding`.
- Lowers to engine API: select opponent permanent, read selected level, then call `return_to_deck(..., DeckEnd::Bottom)` for each opponent permanent whose top card has that level.
- DSL syntax: `bind_permanent_property: { from: chosen_dig, property: level, bind_as: chosen_level }` followed by `for_each: { over: { level_eq_binding: chosen_level }, ... }`.
- First reported: 2026-04-28
---

## BT23-005 — [Your Turn] cost reduction when digivolving into Reptile/Dragonkin  [G-BEFORE-PAY-COST-DIGIVOLVE-TARGET]
- **Status: RESOLVED 2026-05-17** (Phase 2 Track H). See `qa/resolved-gaps.md` § "Phase 2 Track H closure — 2026-05-17" for the substrate landed (`cost_target` + `source_is_cost_target_permanent` predicates, digivolve-cost-calc target threading).
- Authoring pattern:
  ```yaml
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when:
      all_of:
        - your_turn: true
        - source_is_cost_target_permanent: true
        - cost_target: { trait_has: [Reptile, Dragonkin] }
    amount: 1
  ```
- Card-authoring note: BT23-005 YAML still needs to be updated to use the new pattern; P-117 has been migrated as the proof-of-substrate (`code/digimon-engine/cards/p/P-117.yaml`).
- First reported: 2026-04-27 (BT23-005 batch-implement-cards-rust-dsl)
- Also blocks (now resolvable): P-117 clause 0 — "[Your Turn][OPT] When this Digimon would digivolve into a card with the [Free] trait, if you have a Tamer, reduce the digivolution cost by 1." Migrated and validated 2026-05-17.

---

## P-117 — inherited When Attacking color-count predicate  [G-DSL-SELF-COLOR-COUNT-GTE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — `self_color_count_gte` exists; P-117 IMPLEMENTED. The "Remaining sibling blocker" note below is now stale: BT12-031 clause 1b ships via `self_color_count_gte` in a `while_condition` (see the BT12-031 entry).

- Effect text: "[When Attacking] If this Digimon has 2 or more colors, ＜Draw 1＞ (Draw 1 card from your deck.)"
- Status 2026-05-11: resolved for top-card color counts. `self_color_count_gte: N` is now in `PredicateSpec` / `CompiledPredicate` and evaluates the predicate subject/source permanent's synthesized top-card colors.
- DCGO reference: `P_117.cs` lines 203-211 — `card.PermanentOfThisCard().TopCard.CardColors.Count >= 2`. Note: DCGO checks ONLY the top card's colors, not the union of the full digivolution stack. The DSL predicate should align with DCGO behavior: count the top card's colors only.
- Lowers to engine API: `Game::player(p).battle_area[i].top_card()` → `card_data[idx].colors.len()` comparison; no new engine primitive needed, only a DSL predicate leaf that invokes `ctx.source_permanent` top-card color count.
- DSL syntax:
  ```yaml
  condition:
    self_color_count_gte: 2
  ```
  Evaluates as: `ctx.source_permanent.and_then(|h| perm.top_card().colors().len()).unwrap_or(0) >= 2`.
  Alternative: `source_top_card_color_count_gte: 2` if the naming convention favors explicit subject.
- Gap kind: DSL only (engine has the data; only the predicate leaf is missing).
- Cards unblocked: P-117 clause 1 (inherited When Attacking).
- Remaining sibling blocker: BT12-031 clause 1b still needs a distinct stack-union color-count predicate ("2+ colors in digi-cards"), not this top-card-only predicate.
- First reported: 2026-05-04 (P-117 batch-implement-cards-rust-dsl)

---

## BT21-025 — `attacker_trait_has` predicate on `on_attack_target_change` clauses  [G-ATK-TRAIT-FILTER]
- Effect text: "[Your Turn][Once Per Turn] When any of your [Reptile] or [Dragonkin] trait Digimon's attack targets change, trash your opponent's top security card."
- Missing DSL verb / step kind / predicate: `attacker_trait_has` (and likely `attacker_owner_is_you`) predicates to gate `on_attack_target_change` clauses by the attacking permanent's traits/owner.
- Status (2026-05-07): narrowed. `on_attack_target_change` now carries structured payload predicates for `attack_target_change_reason`, `attacker_trait_has`, `event_target_is_player`, `event_target_was_self`, and new-target `event_target_owner`/`event_target_trait_has`; the owner-specific predicate in this gap remains open. Coverage for the closed payload leaves: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- attack_target_change_`.
- Lowers to engine API: `TriggerContext` already carries `source_permanent` for `PlayerBattleArea` triggers; a predicate could inspect `ctx.trigger_context.source_permanent.traits()`. No new engine API needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    attacker_trait_has: Reptile
    # or any_of: [{ attacker_trait_has: Reptile }, { attacker_trait_has: Dragonkin }]
  ```
- Workaround used: `any_permanent` filter over your battle area with `trait_has: Reptile/Dragonkin` — necessary but not sufficient (over-fires when a non-matching attacker switches target while a matching ally is on board).
- First reported: 2026-04-27 (BT21-025 batch-implement-cards-rust-dsl)

---

## ~~BT24-016 — `condition:` field on `AltPathSpec` (alt-digivolve activation gates)  [G-ALT-PATH-CONDITION]~~ — RESOLVED 2026-05-15

- **Status:** Schema + consumer wired. `AltPathSpec.condition: Option<PredicateSpec>` is now accepted by the DSL parser, compiles to `CompiledAltPath.condition: Option<Box<CompiledPredicate>>`, and is evaluated in `code/digimon-engine/src/dna_digivolve.rs::find_matching_alt_path` after the source-filter check (Digivolve route).
- **Card-side authoring follow-up:** BT24-016's YAML still leaves the Owen Dreadnought gate unenforced; populating `condition:` on the activated_digivolve path is card-local work, not substrate.
- **Evidence:** `cargo test --manifest-path code/digimon-dsl/Cargo.toml`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage`.
- **Full entry archived to:** `qa/resolved-gaps.md` under "DSL Gap: `AltPathSpec.condition` field for alt-digivolve activation gates".

---

## EX11-054 — [All Turns] entering-permanent trait gate  [G-ENTERING-PERMANENT-TRAIT]

- Effect text: "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP for the turn."
- Missing DSL verb / step kind / predicate: `entering_permanent_trait_has` / `digivolving_permanent_trait_has` — BoolPredicate leaves to gate an observer clause on the traits of the card that JUST entered the field or digivolved. The `event_target_trait_has` predicate evaluates `TriggerContext.target_permanent`, which for `OnEnterFieldAnyone` / `OnDigivolve` observers is the OBSERVER's own permanent handle (not the entering/digivolving card).
- Companion engine gap: `trigger_context_for_source` in `effect_queue.rs` sets `target_permanent = source_permanent` (the observer itself) when iterating `TriggerSource::PlayerBattleArea(pid)`. The entering card's handle is not threaded into `TriggerContext`. Additionally, `GameEvent::Digivolve` is "defined for future wiring — not emitted yet" (events.rs), blocking event-log-based detection of the digivolving permanent.
- Updated 2026-04-29: the digivolve half is now partially closed for normal `Game::digivolve_from_hand`: `GameEvent::Digivolve` is emitted and `TriggerSource::Digivolved` populates `TriggerContext.event_permanent` / `event_card` with the just-digivolved permanent and new top card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`. `OnEnterFieldAnyone`, effect-initiated digivolve, DNA digivolve, and breeding-area digivolve remain open.
- Updated 2026-04-29: the enter-field half is now partially closed for normal hand-played battle-area permanents: `TriggerSource::EnteredField` populates `TriggerContext.event_permanent` / `event_card` with the entering permanent and card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card`. Effect-created permanents, token play, option placement, play-from-trash context, and breeding-area observer fan-out remain open.
- Lowers to engine API: covered enter-field and digivolve paths now use `TriggerContext.event_permanent` / `event_card`; remaining dedicated `entering_permanent_trait_has` / `digivolving_permanent_trait_has` syntax, if added, should lower to those fields and keep untested entry/digivolve paths gated until separate dispatch tests exist.
- Suggested DSL syntax:
  ```yaml
  condition:
    any_of:
      - entering_permanent_trait_has: Reptile
      - entering_permanent_trait_has: Dragonkin
  # (same shape for digivolve half with digivolving_permanent_trait_has)
  ```
- Gap kind: hybrid (engine doesn't thread the entering-permanent handle through TriggerContext; DSL has no predicate leaf to read it even if it did).
- Workaround: `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). All related tests `#[ignore]`'d with `entering_permanent_trigger_context` tag.
- First reported: 2026-04-27 (EX11-054 batch-implement-cards-rust-dsl)

---

## ~~BT21-024 — Opponent security count condition  [G-OPP-SECURITY-COUNT-LTE]~~ — RESOLVED 2026-05-17 (Phase 2 Track G)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track G closure" entry.
`PredicateSpec.opponent_security_count_lte: Option<DpConstraint>` and the
`_gte` sibling now compile through `CompiledPredicate` and evaluate
against `rctx.security_count(rctx.opponent_id())`. BT21-093 cost-reduction
clause migrated to use the new predicate; BT21-024's negative-condition
test was already passing through the `count_lte` aggregate over
`{ zone: [security], owner: opponent }`.

[ORIGINAL ENTRY BELOW]

- Effect text: "[On Play][When Digivolving] If your opponent has 5 or fewer security cards, they place 1 card from their hand as the bottom security card. Then, trash their top security card."
- Missing DSL verb / step kind / predicate: `opponent_security_count_lte` — a `PredicateSpec` / `BoolPredicate` leaf that checks the OPPONENT's (not controller's) security stack count. The existing `security_count_lte: u8` field in `PredicateSpec` evaluates `rctx.security_count(rctx.player)` (controller's security). No `of:` field exists on the predicate to redirect the player lookup. A separate `opponent_security_count_lte: Option<u8>` field is needed.
- Lowers to engine API: `rctx.security_count(rctx.opponent())` — `security_count(player_id)` already exists on `EffectReadContext`. The gap is that the predicate evaluator has no branch to call it with the opponent ID.
- Suggested DSL syntax:
  ```yaml
  condition:
    opponent_security_count_lte: 5
  ```
  Alternatively, extend `security_count_lte` to accept an `of:` modifier:
  ```yaml
  condition:
    security_count_lte: { count: 5, of: opponent }
  ```
- Gap kind: dsl (engine primitive exists; predicate evaluator just needs the branch and an `of:` routing parameter or a sibling field).
- Workaround: Clause runs unconditionally (matching DCGO behavior where `trash_top_security` runs outside the inner `if (SecurityCards.Count <= 5)` block). The condition gates only the `select_hand` + `place_on_security` sub-step in DCGO. Negative condition test is `#[ignore = "pending: G-OPP-SECURITY-COUNT-LTE"]`.
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## ~~BT21-024 — Outer-tail continuation lost when `select_hand` has no candidates  [G-SELECT-EMPTY-OUTER-TAIL]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The `select_hand` empty-candidate path now drains the outer tail —
  when `install_select_hand` finds no valid candidates it runs the parked outer-tail steps
  (e.g. `trash_top_security`) instead of silently discarding them. Landed in the
  `complete-dna-omnimon-archetype` change; the empty-hand behavioral test is re-enabled and
  passing. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion closure —
  2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: "[On Play][When Digivolving] ... Then, trash their top security card." — the `trash_top_security` step after `as_selecting_player` must fire even when the opponent has no hand cards.
- Engine gap: `install_select_hand` in `code/digimon-engine/src/effect_context/selections.rs` (lines 177–179) returns early without installing a `PendingSelection` when `valid_action_ids.is_empty()` (opponent has no hand cards). When this early-return fires, no selection callback is ever installed, so `drain_dsl_outer_tail` (which is called from the selection callback in `selections.rs:47`) is never executed. Steps that `park_outer_tail` placed after the `as_selecting_player` block — specifically `trash_top_security` — are silently discarded.
- Root cause: the outer-tail drain relies on the inner select completing through its callback. An empty-selection skip short-circuits before the callback is installed.
- Lowers to engine API: no new method needed. Fix options: (1) in `install_select_hand`, when `valid_action_ids.is_empty()` and the call is not optional, immediately call `drain_dsl_outer_tail(ctx)` before returning; (2) alternatively, make the outer-tail drain happen in the park/skip path rather than only in the callback; (3) add an `on_skip` path analogous to `on_decline` for optional selections that fires the continuation.
- Suggested fix path: option (1) — cheapest, no new API surface:
  ```rust
  if valid_action_ids.is_empty() {
      // No candidates: skip the selection but still drain the outer tail.
      drain_dsl_outer_tail(ctx);
      return;
  }
  ```
- Gap kind: engine (the DSL YAML is correctly structured; the lowering engine loses the continuation in the empty-hand case).
- Workaround: Test for the empty-hand case is `#[ignore = "pending: G-SELECT-EMPTY-OUTER-TAIL"]`. In practice, the YAML behavior deviates from printed card text only when the opponent has an empty hand (rare competitive scenario).
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## ~~BT17-018 — `lose_count_bound` step verb (count-driven security trash loop)~~  [G-LOSE-COUNT-BOUND] — RESOLVED 2026-05-22

- **Resolved** by adding an optional `count: FormulaSpec` field to the existing
  `trash_top_security` verb (`TrashTopSecurityArgs` in `digimon-dsl/src/step.rs`).
  The engine handler (`step/draw.rs`) evaluates the formula and loops
  `trash_top_security` that many times, bailing early when the stack empties.
  A dedicated `lose_count_bound` / `repeat_n` combinator was not needed — the
  `count` field on the existing verb is the smaller surface. BT17-018's
  `[When Attacking]` clause now ships as pure DSL:
  ```yaml
  - trash_top_security:
      of: opponent
      count:
        floor_div:
          - { base: 0, per: { card_count_in_zone: { of: any, zone: trash } }, delta: 1 }
          - 10
  ```
  raw_rust `bt17_018_trash_security_per_ten_trash` removed.
- First reported: 2026-04-27 (BT17-018 batch-implement-cards-rust-dsl)

---

## Royal Knights — `on_option_placed` timing lowerer  [G-OPTION-PLACED-TIMING]

- Effect text: `BT13-007` King Drasil_7D6 inherited: "[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory."
- Missing DSL verb / step kind / predicate: `when: on_option_placed` is accepted by the DSL compiler as `CompiledTiming::OnOptionPlaced`, but the Rust engine timing map returns `None` for it, so no `EffectTiming` is emitted and no clause can fire.
- Companion engine gap: the Rust engine has no `EffectTiming::OnOptionPlaced` dispatch site when a Delay/Training/field Option is placed in the battle area. `BT13-110` Royal Knights of the Purge and `BT20-100` The Last Guardian both make this timing matter for the Royal Knights loop.
- Lowers to engine API: needs a new `EffectTiming::OnOptionPlaced` (or equivalent observer timing) plus a dispatch after Option placement in `Game::dispose_option` / option placement helpers. The trigger context should identify the placed Option card and controller so `event_card_trait_has: "Royal Knight"` can be evaluated.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: on_option_placed
    active_when: { in_breeding: true }
    once_per_turn: true
    condition: { event_card_trait_has: "Royal Knight" }
    process:
      - gain_memory: 1
  ```
- Gap kind: hybrid (DSL has the token but no lowering target; engine lacks the timing dispatch).
- Workaround: None faithful. The memory-gain trigger is omitted at runtime.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-04-29: `when: on_option_placed` now lowers to `EffectTiming::OnOptionPlaced`, and Delay-style Option placement through `Game::play_option_from_hand` supplies the placed Option through `event_card` / `event_permanent`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`.
- Updated 2026-05-02: Group 5 Task 4 covers Link, Training, inherited/security self-placement, and top-card plus inherited breeding-area observer fan-out for `OnOptionPlaced`, with placed Option context available via `event_card` and Link host context via `event_host_permanent` / `event_host_card`. Link placement resumes `OnLink` after placed-option selections settle, and breeding-source `max_per_turn` accounting is covered for this queued observer path. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card link_on_option_placed_selection_resumes_on_link_after_choice_resolves on_option_placed_scans_inherited_sources_under_breeding_top_card once_per_turn_breeding_on_option_placed_observer_fires_once_not_zero`. Transient Standard options remain open because they are not battle-area placements.

---

## Royal Knights — selecting permanents in the breeding area  [G-BREEDING-PERMANENT-SELECTION]

- Effect text: `BT20-083` Omekamon: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area." Similar Royal Knights effects target or play from the breeding-area King Drasil stack (`BT13-093`, `BT13-110`, `BT13-112`, `EX11-053`, `BT23-072`).
- Status: selection is resolved; effect movement support is partially resolved. `select_own_breeding_permanent` now installs a breeding-specific pending selection and binding without fake battle-area handles. Group 4 also lets `place_as_bottom_source` target the real breeding slot via `BREEDING_TARGET`.
- Companion engine state: `SelectionKind::BreedingPermanent`, `BreedingPermanentSelectionRef`, and phase-scoped breeding select actions cover the player-visible choice. `EffectContext::move_from_breeding_by_effect` and `play_to_breeding_from_hand` cover direct effect movement to/from the real breeding slot.
- Lowers to engine API: `select_own_breeding_permanent` for the choice, `place_as_bottom_source` for tucking under the selected breeding stack, and source-parametric `effect_initiated_digivolve` for non-hand result cards once a source binding is available.
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: kd
      filter:
        all_of:
          - name_is: "King Drasil_7D6"
          - zone: [breeding]
      prompt: "Choose your King Drasil_7D6 in breeding"
  ```
  Alternatively, add an explicit sugar step:
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
  ```
- Gap kind: hybrid (the YAML shape exists, but lowering/runtime selection ignore breeding).
- Workaround: None faithful. Auto-targeting the only breeding permanent would hide a player-visible selection and violates the no-approximations policy.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-05-02: remaining open follow-ups are breeding-area trigger fan-out (`G-BREEDING-TRIGGER-DISPATCH`) and card-specific optional/filter wrappers, not the basic breeding selection or real-zone movement primitives.
- Updated 2026-05-08: Track A resolved the security-removal breeding fan-out slice: `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` scan the observer player's breeding slot through the existing top-card/inherited breeding enqueue path and carry the `TriggerSource::SecurityRemoved` payload. This narrows BT20-083 to its printed body support: suspend a breeding carrier as the cost and play an [Omekamon] from the selected breeding stack's materials without paying the cost. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload`.

---

## ~~P-130 — effect move-from-breeding DSL verb~~  [G-MOVE-BREEDING-DSL] — RESOLVED 2026-05-23

Moved to [`qa/resolved-gaps.md`](resolved-gaps.md). `move_from_breeding` now lowers to `EffectContext::move_from_breeding_by_effect`, and `select_own_breeding_permanent` supports a level filter plus optional accept/decline prompt for P-130's printed `[On Play]` clause.

---

## BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Status: RESOLVED for reusable formula-zone count filters on 2026-05-02. `card_count_in_zone` payloads now accept `filter: { ... }`; the compiler carries the predicate into filtered count IR, and runtime evaluation counts only representable subjects that satisfy the predicate instead of falling back to an unfiltered count.
- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Implemented DSL form: `card_count_in_zone` formulas can now apply a `kind: digimon` filter. `BT8-097.yaml` uses this filtered form so Tamers and Option permanents no longer reduce Crimson Blaze's play cost.
- Lowers to engine API: the engine can inspect each battle-area permanent and test `Permanent::is_digimon(&card_data)`; the formula DSL needs a filtered-count form that passes a compiled predicate into formula evaluation.
- Suggested DSL syntax:
  ```yaml
  amount_fn:
    base: 0
    per:
      card_count_in_zone:
        of: opponent
        zone: battle_area
        filter: { kind: digimon }
    delta: 1
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for filtered zone-count formulas.
- Workaround: no longer needed for BT8-097 or other `card_count_in_zone` formulas with simple predicate filters.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch phase3d_formula_zone_count`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097`.

---

## AD1-012 — `on_opponent_attack` Timing variant on triggered clauses  [G-DSL-ON-OPPONENT-ATTACK]
- Effect text: AD1-012 CresGarurumon: "[Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, you may change the attack target to 1 of your Digimon."
- Status (2026-05-08): closed. `on_opponent_attack` parses, compiles to `CompiledTiming::OnOpponentAttack`, maps to `EffectTiming::OnOpponentAttack`, and is dispatched from the combat flow. Coverage includes `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_clauses phase2a_triggered` and existing combat timing tests.
- Previous missing DSL verb / step kind / predicate: `Timing::OnOpponentAttack` variant on `digimon_dsl::clause::Timing` (`code/digimon-dsl/src/clause.rs:83-125`); no mapping in `compile_timing` (`code/digimon-dsl/src/compile.rs:173-216`).
- Lowers to engine API: `Effect::on_opponent_attack` (`code/digimon-engine/src/effect.rs:427`) — engine timing dispatch already handles `EffectTiming::OnOpponentAttack` (`lower_triggered.rs:181`) and the combat state machine fires it (`combat.rs:2237-2242`). The hybrid declared-attack-observer engine slice closed 2026-04-29 unblocks the engine half; DSL just lacks the timing token.
- Suggested DSL syntax:
  ```yaml
  - when: on_opponent_attack
    active_when: { opponents_turn: true }
    once_per_turn: true
    optional: true
    process: [...]
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2a_triggered parse_clauses`.
- Implementation: add `Timing::OnOpponentAttack` variant + serde wiring + `compile_timing` arm; the existing `lower_triggered.rs` already routes `EffectTiming::OnOpponentAttack`, so no new lowering code needed.
- Gap kind: dsl, closed. AD1-012's Opp-Turn clause remains blocked by the defender-side effect DNA route into Omnimon Alter-S (and the separate redirect-attack-target step), not by this timing token.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## AD1-012 — `redirect_attack_target` step verb  [G-DSL-REDIRECT-ATTACK-TARGET]
- Effect text: AD1-012 CresGarurumon (sub-step of the Opp-Turn clause): "Then, you may change the attack target to 1 of your Digimon."
- Previous missing DSL verb / step kind / predicate: No `redirect_attack_target` entry in the `StepSpec` enum / serde tag table at `code/digimon-dsl/src/step.rs`. No `CompiledStep::RedirectAttackTarget` variant.
- Status (2026-05-07): closed for bound permanent and player retargets. `redirect_attack_target` now parses, compiles, and lowers to `ctx.redirect_attack`, supporting `new_target: <binding>` and `player: you|opponent|active`. Runtime coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target`.
- Lowers to engine API: `EffectContext::redirect_attack(new_target_perm)` (`code/digimon-engine/src/effect_context/mod.rs:3099`) — exists and is used by hand-written cards (BT22-061, EX11-042, P-094 in legacy Python).
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: redirect_target
      optional: true
      filter: { kind: digimon }
      prompt: "Change attack target to 1 of your Digimon"
  - redirect_attack_target: { new_target: redirect_target }
  ```
- Implementation: add `StepSpec::RedirectAttackTarget { new_target: BindingRef }` + serde + `CompiledStep` variant + lowering arm in `dsl_cards/step/combat.rs` that resolves the binding to a `PermanentHandle` and calls `ctx.redirect_attack(perm_handle)`.
- Gap kind: dsl, closed. AD1-012 Opp-Turn redirect substep is now blocked by the effect DNA setup before it, not by the redirect verb.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## Effect-created attack verbs — `force_attack` / `cancel_attack` / `open_counter_window`  [G-DSL-FORCE-CANCEL-ATTACK]
- Missing DSL verb / step kind / predicate: Several audit notes used placeholder names such as `force_attack_now` or omitted attack cancellation bodies because only engine-side helpers existed.
- Status (2026-05-08): closed for immediate effect-created forced attacks, legal-window attack cancellation, and the named Counter-window bridge. `force_attack` parses/compiles/lowers to `ctx.force_opponent_attack(...)`; `cancel_attack: {}` parses/compiles/lowers to `ctx.cancel_pending_attack()`; `open_counter_window: {}` parses/compiles/lowers to `ctx.open_counter_window()` and reuses the normal Counter pending-selection scan. BT20-102 now uses `force_attack` + `without_suspending: true` for its DCGO-matched optional-trigger/mandatory-attack flow. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- force_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- cancel_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- open_counter_window_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_102`.
- Supported DSL syntax:
  ```yaml
  - force_attack:
      attacker: forced
      targets: player # any | player | digimon
      without_suspending: true
  - cancel_attack: {}
  - open_counter_window: {}
  ```
- Remaining caveat: card YAML that used old commented placeholder names still needs card-specific rework.

---

## BT15-101 — Self-target predicate for event triggers (`event_target_is_source`)  [G-DSL-EVENT-TARGET-IS-SELF]
- Effect text: BT15-101 MetalGarurumon: "[All Turns] [Once Per Turn] When this Digimon becomes suspended, you may unsuspend it."
- Missing DSL verb / step kind / predicate: No `event_target_is_source` (or equivalent `event_target_is_self`) BoolPredicate leaf that evaluates whether the suspended/affected permanent equals the source permanent. The existing event predicates (`event_target_owner`, `event_target_kind`, `event_target_trait_has`) only inspect the target's owner/kind/traits. The DSL `equals: [...]` predicate compares only integers (literals + integer bindings via `Bindings::get_literal`) — it cannot compare permanent handles.
- Lowers to engine API: `event_target_card(rctx)` already returns the `CardHandle` of the suspended permanent's top card; `rctx.source_permanent` carries the source permanent handle. A new predicate could compare `current_trigger_context.event_permanent` against `rctx.source_permanent_handle()`.
- Suggested DSL syntax: add `event_target_is_source: bool` BoolPredicate leaf evaluating `rctx.game.current_trigger_context?.event_permanent == Some(rctx.source_permanent_handle()?)`.
  ```yaml
  - when: on_suspend
    active_when: { all_turns: true }
    once_per_turn: true
    optional: true
    condition: { event_target_is_source: true }
    process:
      - unsuspend: { target: source }
  ```
- Implementation: add `event_target_is_source: Option<bool>` to `PredicateSpec`, compile to a new `CompiledPredicate` field, evaluate inside `eval_event_fields` in `dsl_cards/predicate.rs`.
- Updated 2026-05-08: Implemented under the clearer name `event_permanent_is_source: true`, comparing `TriggerContext.event_permanent` to the observer's `source_permanent`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and BT23-077's card-shaped fixture. BT15-101 still needs card-local YAML/test adoption before this card entry can be closed.
- Gap kind: dsl. Engine has the comparison primitive (handles are equality-comparable).
- Workaround: AD1-014 pattern (`event_target_owner: you, event_target_kind: digimon`) — over-fires when ANY of the controller's Digimon (allies) suspend, so OPT may be consumed at the wrong moment and a "may unsuspend" prompt may appear when the source is not actually suspended. Faithful for "any of your Digimon"-style triggers (AD1-014, BT13-012); approximation-only for "this Digimon" triggers (BT15-101).
- First reported: 2026-05-03 (BT15-101 batch-implement-cards-rust-dsl)

## BT21-102 — `on_ally_attack` / `on_opponent_attack` timings missing from DSL
- Effect text: BT21-102 Tai Kamiya — "[Your Turn] When one of your Digimon attacks, by suspending this Tamer, ＜Draw 1＞."
- Status: resolved for the timing tokens. `on_ally_attack` and `on_opponent_attack` parse, compile, and lower to the engine timings.
- Former missing DSL verb / step kind / predicate: `digimon_dsl::clause::Timing` enum (`code/digimon-dsl/src/clause.rs`) did not include `OnAllyAttack` or `OnOpponentAttack`, making the engine mappings unreachable from YAML.
- Lowers to engine API: `Effect::on_ally_attack(card)` / `Effect::on_opponent_attack(card)` already exist (`code/digimon-engine/src/effect.rs` line 421+).
- Suggested DSL syntax:
  ```yaml
  - when: on_ally_attack
    optional: true
    active_when: { your_turn: true }
    process:
      - suspend: { target: source }
      - draw: { of: you, count: 1 }
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2a_triggered parse_clauses`.
- Gap kind: resolved DSL timing surface. Card-local YAML can now use the faithful timing token instead of the `when_attacking` workaround.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl)

## BT21-102 / BT15-096 — `play_cost_lte` formula-valued / binding-relative variant  [G-DSL-DISTINCT-TAMER-COLORS-FORMULA]
- Effect text: BT21-102 Tai Kamiya — "[Main] [Once Per Turn] You may play 1 [ADVENTURE] or [Hero] trait card with a play cost of 2 or less from your hand without paying the cost. For each of your Tamers' colors, add 1 to this effect's play cost maximum."
- Effect text: BT15-096 Supreme Connection! — "[Delay] 1 of your Digimon with the [Machine] or [Cyborg] trait may play 1 Digimon card with a play cost less than or equal to that Digimon's play cost from your hand with the play cost reduced by 3."
- **Status: RESOLVED 2026-05-17** (Phase 2 Track A finalization; formula primitive landed 2026-05-10). Phase 2 Track A swept stale references and confirmed coverage in `tests/dsl/group7_predicate_batch.rs` + `tests/dsl/group7_formula_batch.rs`. The companion BoolPredicate wrapping `G-DSL-DISTINCT-TAMER-COLORS` (ST20-10 disjunct) is closed by the same formula leaf — `play_cost_lte: { formula: { distinct_colors_count: ... } }` covers both shapes.
- Status (legacy): RESOLVED on 2026-05-10. `PredicateSpec::play_cost_lte` now accepts either the legacy literal threshold or `{ formula: ... }`. Formula thresholds compile through `CompiledDpConstraint`, evaluate during selection-mask construction, and can read `binding_play_cost` from a previously selected card/permanent binding. BT21-102's color-scaled cap is also covered by `distinct_colors_count`.
- Lowers to engine API: `card.play_cost <= rctx.eval_formula(formula)` — engine already has formula evaluation and per-card play_cost reads.
- DSL syntax:
  ```yaml
  filter:
    play_cost_lte:
      formula:
        base: 2
        per:
          distinct_colors_count:
            of: you
            zone: [battle_area]
            filter: { kind: tamer }
        delta: 0
  ```
- Binding-relative syntax:
  ```yaml
  filter:
    play_cost_lte:
      formula:
        binding_play_cost: source_digimon
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_formula_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_102 -- --nocapture`.
- Gap kind: dsl. Companion to G-DSL-DISTINCT-TAMER-COLORS-FORMULA for BT21-102; independently blocks BT15-096's Delay clause.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl). Binding-relative variant reaffirmed 2026-05-10 (BT15-096 Supreme Connection!, Alter-S Ladder batch).

## EX9-066 — Binding-presence predicate (`binding_present`/`binding_absent`)  [G-DSL-BIND-PRESENT]
- Effect text: EX9-066 Tai Kamiya & Matt Ishida — "[On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or [Omnimon] in its name from your trash to the hand. If this effect didn't return, ＜Draw 1＞." Also EX11-074 — "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon, ..."
- Status: NARROWED on 2026-05-10. The pure binding-presence predicate primitive is implemented as `binding_present` / `binding_absent` plus aliases `binding_is_present` / `binding_is_none`, compiled to `CompiledPredicate`, and evaluated against the threaded `Bindings`. This does not close richer result-log predicates such as "this effect suspended your Digimon" when the mutation itself must be distinguished from a selected target.
- Former missing DSL verb / step kind / predicate: no `binding_present: <name>` or `binding_absent: <name>` BoolPredicate leaf that evaluates whether a prior `bind_as:` step (e.g. an optional `select_trash` / `select_hand` / `select_own_permanent` that the player may have declined) actually produced a value. The existing `equals: [<binding>, <literal>]` compare on `CompiledBindingCompare` only supports integer-valued bindings (literals + integer bindings via `Bindings::get_literal`) — it cannot distinguish a permanent/card binding that was set vs absent.
- Lowers to engine API: `Bindings::get_card(name).is_some()` / `Bindings::get_permanent(name).is_some()` / `Bindings::get_literal(name).is_some()` — engine already has these read paths through `digimon_dsl::compiled::Bindings` and `effect_context::Bindings`.
- Suggested DSL syntax:
  ```yaml
  - select_trash:
      bind_as: pick
      optional: true
      filter: { ... }
  - if:
      condition: { binding_present: pick }
      then: [ add_to_hand_from_trash: { card: pick } ]
      else: [ draw: 1 ]
  ```
- Implementation: added `binding_present: Option<String>` and `binding_absent: Option<String>` BoolPredicate leaves to `PredicateSpec`, compile to `CompiledPredicate` fields, and evaluate inside `eval_predicate_with_bindings` in `dsl_cards/predicate.rs` by checking the named binding in the threaded `Bindings`.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`.
- Gap kind: dsl. Engine has the comparison primitive (binding presence is a trivial Option check).
- Workaround used in EX9-066: drop the binding-result check entirely; present a binary `select_effect_choice [Return / Draw]` so the player explicitly picks the branch up front. The Return branch's inner `select_trash` is `optional: true` so it degrades gracefully when no eligible cards exist. Case C (no eligible card + player picked Return) becomes a no-op rather than a forced draw — diverges from DCGO but the action mask still surfaces the Decline → Draw alternative, so a faithful RL agent learns to pick Decline in case C. No auto-selection is performed on the agent's behalf; the no-approximations policy is preserved.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## BT24-008 / EX9-066 — General `count_gte` / `count_lte` predicate not evaluated  [G-COUNT-GTE-NOT-EVALUATED] — RESOLVED 2026-05-17 (Phase 2 Track A)
- **Status:** Closed. `eval_predicate_with_bindings` now consults `count_gte` / `count_lte` via a generalized `count_matching_in_zone` walker. See `qa/resolved-gaps.md` § "Phase 2 Track A closure" for full details. BT24-008 / EX9-066 / EX1-021 chained-`if` workarounds are now substrate-correct.
- Effect text: BT24-008 Lv4 Reptile/Dragonkin/LIBERATOR — "[On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR] trait from your hand, <Draw 2>." (condition gates on `count_gte` over hand). EX9-066 — needs gating on `count_gte` over trash zone for the trash-or-draw branch.
- Status (legacy): OPEN (filed 2026-05-03 during EX9-066 batch-implement-cards-rust-dsl). Previously documented inline in BT24-008.yaml header but not as a standalone gap entry.
- Missing engine evaluation: `PredicateSpec::count_gte: Option<CountAggregate>` and `count_lte: Option<CountAggregate>` parse correctly into `CompiledPredicate.count_gte` / `count_lte` (`compiled.rs` lines 223-224), but `dsl_cards/predicate.rs::eval_predicate_with_bindings` does NOT consult these fields — only the specialized `security_count_gte` / `security_count_lte` (predicate.rs lines 73-82) and `materials_count_gte` / `materials_count_lte` (predicate.rs lines 834-842) are wired. So `condition: { count_gte: { filter: ..., n: 1 } }` is a no-op that always evaluates as TRUE, which means `if count_gte ≥ 1 then [...] else [...]` always takes the `then` branch regardless of the actual card count.
- Lowers to engine API: needs a generic `count_matching_in_zone` walker that takes a `CompiledPredicate` filter (with `zone:` constraints) and counts matches across the named player's hand / trash / battle_area / security / deck. The existing `existential_any` walker (predicate.rs:279) only iterates `battle_area` and stops at first match — needs to be generalized to iterate the requested zones and count instead of short-circuit.
- Suggested DSL syntax (already accepted by the parser — only evaluation is missing):
  ```yaml
  condition:
    count_gte:
      filter:
        of: you
        zone: [trash]
        kind: digimon
        any_of:
          - name_contains: "Greymon"
          - name_contains: "Garurumon"
          - name_contains: "Omnimon"
      n: 1
  ```
- Implementation: add a `count_in_zones(filter: &CompiledPredicate, target: PlayerRef, rctx, bindings) -> u32` helper in `dsl_cards/predicate.rs` that iterates the player's hand / trash / battle_area / security / deck per the filter's `zone:` field and counts matches via per-card / per-permanent predicate evaluation. Then check `count >= agg.n` (gte) / `count <= agg.n` (lte) inside `eval_predicate_with_bindings`.
- Gap kind: engine evaluation gap (DSL surface complete; runtime evaluation missing).
- Workaround used in EX9-066: drop the count_gte pre-gate entirely; always present the binary [Return / Draw] choice and rely on the inner `select_trash` being `optional: true`. Acceptable because the action mask still surfaces both branches faithfully. BT24-008 has the same pending workaround documented in its YAML header.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## ~~BT22-017 — `text_contains` (effect-text scan) predicate  [G-DSL-PREDICATE-TEXT-CONTAINS]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The `effect_text_contains` predicate leaf landed in the
  `complete-dna-omnimon-archetype` change; it scans a candidate's printed
  effect/inherited/security text by case-insensitive substring, lowering through
  `CompiledPredicate`. BT22-017's bucket-1 filter now uses `effect_text_contains: "Omnimon"`
  and the `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text` test is
  re-enabled and passing. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion
  closure — 2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: BT22-017 [On Play] "Reveal the top 3 cards of your deck. Add 1 card with [Omnimon] in its TEXT and 1 card with the [CS] trait among them to the hand."
- Missing DSL verb / step kind / predicate: `text_contains: Option<String>` leaf on `predicate::PredicateSpec`. The DSL exposes `name_contains` / `name_is` / `name_in` for card-name scans, but has no leaf that scans a candidate's printed `effect_text` / `inherited_text` / `security_text`. DCGO uses `source.HasText("Omnimon")` (BT22_017.cs line 63) which scans the card's effect text for the literal substring.
- Engine data IS present: `code/digimon-engine/src/card_data.rs` carries `effect_text`, `inherited_text`, and `security_text` fields on `CardData` (lines 87, 99, 124). Only the DSL predicate verb is missing.
- Lowers to engine API: a new `text_contains` leaf compiled through `CompiledPredicate` and evaluated in `dsl_cards/predicate.rs` by case-insensitive substring scan against the candidate's combined text. The existing `name_contains` evaluator at `dsl_cards/predicate.rs:705` is the lookalike to clone.
- Suggested DSL syntax:
  ```yaml
  filter:
    text_contains: "Omnimon"
  ```
- Approximation used in BT22-017 today: `name_contains: "Omnimon"`. Narrows correctly for printed Omnimon-named cards (BT12-085, BT22-015, etc.) because their card_name itself carries "Omnimon", but WRONGLY excludes cards that mention `[Omnimon]` only in their effect_text without carrying it in their name (e.g. tutors / supports printed "search for [Omnimon]"). Faithfulness divergence is asserted-and-#[ignore]'d in `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text`.
- Also blocks: any future card whose printed text uses an `in its text` (rather than `in its name`) bucket-filter — including BT12-059's bucket 1 if it were to switch from name-based to text-based per a future erratum.
- Gap kind: DSL vocabulary gap (engine data is present; no DSL surface to filter on it).
- First reported: 2026-05-03 (BT22-017 Gabumon, batch-implement-cards-rust-dsl)

## EX1-068 — grant a triggered effect to opponent's permanent  [G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT] — RESOLVED 2026-05-29

Closed by change `add-grant-triggered-effect-dsl`. The `grant_triggered_effect`
step + `ModifierType::GrantedTrigger` slot already existed (EX10-034 grant-to-
binding work); the remaining slice was (a) opponent-set targeting — a predicate
`target: { of: opponent, ... }` already walks both battle areas and snapshots the
match set, and (b) cause attribution for the `<Progress>` interaction — the
granted-trigger dispatch (`enqueue_from_permanent`) now skips firing when the
carrier is unaffected by the GRANTOR's effects (`progress_excludes`). EX1-068's
`[Main]` clause is authored; judge-quiz **Q2** pins it (Medusamon `<Progress>`
loses no memory; a non-Progress control loses 2). Full note in
`qa/resolved-gaps.md`. **Q16 also closed (2026-05-29):** EX6-057 Lilithmon
authored; the granted body now runs as the carrier's OWN effect (D4/DCGO —
sourced from `selectedPermanent.TopCard`), so its granted `[EoT] Delete this` is
OwnEffect and `<Partition>` skips it (judge-quiz Q16 PASS). **Q17 also closed
(2026-05-29):** the granted-trigger dispatch also gates on
`permanent_is_unaffected_by_effect`, so a carrier immune to the grantor's
effects (Magnamon X BT16-102's "isn't affected by your opponent's effects")
suppresses the granted clause (judge-quiz Q17 PASS). All three directions
(Q2/Q16/Q17) are now resolved; full entry in `qa/resolved-gaps.md`.

[ORIGINAL ENTRY BELOW]

- Effect text: EX1-068 [Main] "All of your opponent's Digimon gain '[When Attacking] lose 2 memory' until the end of their next turn."
- Missing DSL verb / step kind / predicate: A `grant_triggered_effect` step that installs a NEW triggered clause (timing + process body) on a SET of cross-permanent targets with a turn-scoped expiry. The DSL today exposes grants for STATIC effects only — `grant_keyword`, `add_modifier` / `add_dp_modifier`, `grant_effect_immunity`. None of those install a clause that itself fires on a future trigger (`when_attacking`, `when_digivolving`, `on_deletion`, ...) on the granted permanent.
- Engine substrate: the Python engine handles this via `permanent.grant_temp_effect(effect, expiry_turn)` + `clear_expired_effects()` (see `qa/archetype-qa/engine-gaps.md` line 33, RESOLVED 2026-03-14 in Python). The Rust engine has the modifier-registry + expiry-tick substrate (`ModifierRegistry` carries per-permanent typed modifiers with `Expiry`), but it does NOT carry a typed `GrantedTriggeredEffect` slot, and there is no `CompiledStep::GrantTriggeredEffect`.
- Lowers to engine API: needs (a) a new `ModifierRegistry` slot (or sibling registry) for per-permanent granted clauses with expiry; (b) the runtime clause dispatcher to consult granted slots when firing a timing on a permanent; (c) a `CompiledStep::GrantTriggeredEffect` whose payload is an inline `CompiledTriggeredClause` (or a registry-keyed template name) lowered against the granted permanent, NOT the source permanent.
- Suggested DSL syntax (option A — inline body):
  ```yaml
  - grant_triggered_effect:
      target:
        of: opponent
        zone: [battle_area]
        kind: digimon
      when: when_attacking
      process:
        - lose_memory: 2     # affects the granted permanent's controller
      expiry: end_of_opponents_turn
  ```
  (Option B — named template: `grant_named_effect: { id: "MemoryMinus2WhenAttacking", target: ..., expiry: ... }` with templates living in a new `code/digimon-engine/src/cards/granted_effects/` registry.)
- Approximation that would VIOLATE no-approximations: a clause that subtracts 2 memory whenever the opponent declares any attack within the expiry window. This over-fires on opponent Digimon played AFTER this Option resolves (DCGO's per-Permanent foreach loop runs ONCE at resolution time and snapshots the eligible Digimon set, so a Digimon played later does not carry the granted clause). Per no-approximations, EX1-068's [Main] clause is OMITTED entirely until the gap closes.
- Also blocks: any "[Main|On Play|When Digivolving] all (your|opponent's) Digimon gain '<bracketed-timing> <body>' until <expiry>" card text. DCGO grep for `UntilOpponentTurnEndEffects.Add` and `UntilOwnerTurnEndEffects.Add` returns ~20+ cards across sets — examples include several Memory-control Options and Tamer support effects across blue/yellow/black.
- Companion engine gap: tracked in `qa/archetype-qa/engine-gaps.md` line 33 as RESOLVED for Python; OPEN for the Rust engine's modifier registry.
- Gap kind: hybrid (Rust engine modifier registry needs a typed grant slot; DSL needs the verb + lowering).
- First reported: 2026-05-03 (EX1-068 Ice Wall!, batch-implement-cards-rust-dsl)
- Judge-quiz consumer (2026-05-28): **Q2** of the judge-quiz faithfulness suite (`add-judge-quiz-faithfulness-suite`) is BLOCKED on this gap. Q2 stages Medusamon (BT24-017) `<Progress>` against the Ice-Wall-granted "[When Attacking] lose 2 memory" and asserts NO memory loss. The Progress half is implemented (`Game::progress_excludes`, combat.rs:2667); only this grant primitive is missing. Test `a_immunity_scope::q2_medusamon_progress_blocks_ice_wall_memory_loss` is `#[ignore]`-blocked citing this gap. When closed, the suite gains a Progress-vs-granted-effect immunity assertion for free.

## EX1-021 — Formula-valued `gain_memory` step  [G-DSL-GAIN-MEMORY-FN] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry for
the closure summary. `gain_memory_fn: { formula: ... }` + `lose_memory_fn`
ship; EX1-021 production YAML authored.

[ORIGINAL ENTRY BELOW]

## EX1-021 — Formula-valued `gain_memory` step  [G-DSL-GAIN-MEMORY-FN] (legacy)
- Effect text: EX1-021 MetalGarurumon — "[When Digivolving] Gain 1 memory for every 4 cards in your hand." DCGO: `count() = card.Owner.HandCards.Count / 4; AddMemory(count())`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `StepSpec::GainMemory(i32)` (`code/digimon-dsl/src/step.rs` line 67) is literal-only. There is no `gain_memory_fn:` variant that consumes a `FormulaSpec`. The same shape already exists for cost-reduction declarative bodies (`amount_fn:` on `kind: cost_reduction`, see BT8-097 / BT21-026 / BT24-017) — this gap is about extending the pattern to imperative `process:` steps.
- Lowers to engine API: `EffectContext::add_memory(player, n)` already accepts a runtime-computed integer. The lowering path needs to evaluate the formula via `formula_eval::evaluate_read_with_bindings(&formula, rctx, source_handle, bindings)` then pass the result to `add_memory`.
- Suggested DSL syntax:
  ```yaml
  - gain_memory_fn:
      formula:
        floor_div:
          - card_count_in_zone: { of: you, zone: hand }
          - 4
  ```
- Implementation: add `StepSpec::GainMemoryFn { formula: FormulaSpec }` + serde + `CompiledStep` variant; lowering arm in `dsl_cards/step/memory.rs` (or wherever `GainMemory` lowers today) that evaluates the formula and calls `ctx.add_memory(ctx.source_player(), result)`. Mirror the same shape for `LoseMemoryFn` for symmetry (no current cards request it, but it costs nothing to ship together).
- Workaround attempted: chained `if count_gte hand n: 4k then [gain_memory: 1]` blocks. BLOCKED at runtime by the pre-existing **G-COUNT-GTE-NOT-EVALUATED** gap — generic `count_gte` always evaluates TRUE, so the chained-`if` workaround would always award the full +N memory regardless of hand size. EX1-021 falls back to `process: []` until either gap closes.
- Also blocks: any `gain X memory for every Y of Z` printed-text family. DCGO grep for `AddMemory(.* / .*)` and `AddMemory(.*Count.*)` returns multiple cards across sets including BT5-095 (gain N where N depends on board state), several Tamer EOT memory grants tied to suspended-tamer counts, etc.
- Gap kind: dsl. Engine has `add_memory` and formula evaluation; only the DSL surface is missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX1-021 — `has_on_deletion_effect` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry.
EX1-021 production YAML authored.

[ORIGINAL ENTRY BELOW]

## EX1-021 — `has_on_deletion_effect` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT] (legacy)
- Effect text: EX1-021 MetalGarurumon — "[When Attacking] If you have 8 or more cards in your hand and a Tamer in play, return 1 of your opponent's Digimon **that has an [On Deletion] effect** to the bottom of its owners deck." DCGO: `permanent.HasOnDeletionEffect`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `PredicateSpec` has no leaf that asks "does this permanent's top card (or any card in its digivolution stack) carry a triggered effect with `EffectTiming::OnDeletion`?" The closest existing leaf is `has_keyword` (which inspects `Keyword` modifiers on the permanent, not effect timings on the underlying card data).
- Engine data IS present: each `CardData` carries the compiled `CompiledCard` (when DSL-authored) with its `effects: Vec<CompiledClause>`; the `CompiledTriggered` clauses include a `when: Vec<CompiledTiming>` that encodes `OnDeletion`. Hand-written `CardEffect` impls expose effects through `card_effects(EffectTiming::OnDeletion, &card)` returning a non-empty list. A new evaluator could walk both surfaces.
- Lowers to engine API: a new `permanent_top_or_sources_have_timing(perm, EffectTiming::OnDeletion)` walker in `dsl_cards/predicate.rs` that checks every card in the permanent's stack (top + sources) for either:
  (a) a compiled DSL clause with `CompiledTiming::OnDeletion` in `when`, or
  (b) a hand-written `CardEffect` impl whose `card_effects(EffectTiming::OnDeletion, ...)` returns non-empty.
  Per the printed text the gate is on the existence of the timing in the card's printed text, not the runtime-active effect set; checking compiled clauses + hand-written impls covers both authoring paths.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: digimon
      - has_on_deletion_effect: true
  ```
- Implementation: add `has_on_deletion_effect: Option<bool>` to `PredicateSpec` + `CompiledPredicate`. Evaluate inside `eval_permanent_fields` by walking `perm.card_sources` and consulting each card's `compiled_card` (DSL path) or registry-resolved `CardEffect` (hand-written path) for `OnDeletion`-timed clauses.
- Workaround: omit the `[On Deletion]` filter entirely. NOT acceptable per no-approximations — over-includes opponent Digimon without [On Deletion], so the player would be forced to pick a non-printed-text-eligible target. EX1-021 falls back to `process: []` until the gap closes.
- Also blocks: any "your opponent's Digimon that has an [On Deletion] effect" or "Digimon with a [When Attacking] effect" / "Tamer with a [Your Turn] effect" printed-text family. DCGO grep for `HasOnDeletionEffect` returns ~5 cards; `Has<Timing>Effect` patterns across all timings extend the impact.
- Gap kind: dsl. Engine data is present; only the DSL surface and walker are missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX4-060 / BT22-015 — Play card from own digivolution sources  [G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from this Digimon's digivolution cards without paying the costs." BT22-015 Omnimon — "<Decode (Red/Black Lv.3)> / <Decode (Blue/Yellow Lv.3)> (When this Digimon would leave the battle area other than in battle, you may play 1 [color] [level] Digimon card from its digivolution cards without paying the cost.)"
- Status: FULLY CLOSED for the reusable source-play substrate on 2026-05-20 (Track J S1.3). Filed 2026-05-03 during EX4-060 batch-implement-cards-rust-dsl; narrowed 2026-05-07, 2026-05-08; the residual multi-source / different-name DSL sugar landed 2026-05-19 (S1.2); the final breeding-carrier residual closed 2026-05-20 (S1.3). BT22-015's Decode entry is closed through a color/level-gated `select_material` plus `play_from_materials` binding, with the original leave event proceeding. EX4-060 is closed by sequential `select_material` / `play_from_materials` steps plus `place_permanent_on_security_and_handle_replacement`. EX9-021's End of Attack source plays are closed through the same source-selection path. The batch / "1 of each different name" form is closed by the `select_materials` count-capped multi-pick step. Breeding-carrier source picks (King Drasil's resident stack) are now closed by the `BREEDING_SOURCE_SELECT` action sub-range (S1.3, `ACTION_SPACE_SIZE` 2168→2192). Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_021`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- play_from_materials`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- Landed DSL verb / step kind: `select_materials` — the batch sibling of `select_material`. Picks up to N digivolution sources of a carrier permanent in ONE count-capped multi-pick; `uniqueness: name` enforces "1 of each different name". `play_from_materials` consumes the bound `CardList` as a batch (each picked source becomes a fresh permanent), composing with `suppress_on_play`.
- Engine substrate: `EffectContext::select_count_capped_multi` with `CountCappedZone::Material(PermanentHandle)` + `DistinctByMode::Name`. `select_materials` lowers straight onto it. For battle-area carriers it reuses the existing `SOURCE_SELECT` action range; for breeding-area carriers it uses the appended `BREEDING_SOURCE_SELECT` sub-range (S1.3). No new `SelectionKind` variant or `play_from_own_digivolution_cards` helper was needed.
- DSL syntax (landed):
  ```yaml
  - select_materials:
      of_permanent: <carrier-binding>  # battle-area permanent (matches select_material)
      max: 4
      uniqueness: name              # "1 of each different name"
      filter: { trait_has: "Royal Knight" }
      bind_as: picked
  - play_from_materials:
      source_index: picked          # batch — all picked sources played
      target: <carrier-binding>
      cost_delta: free
      suppress_on_play: true        # composes with the S1.1 flag
  ```
- Note: batch `play_from_materials` `bind_as` binds only the *last-played* permanent. A future card needing "do X to each played source" will require a `PermanentList` binding.
- Breeding-area carriers (CLOSED 2026-05-20, Track J S1.3): `select_materials` / `select_material` against a `BREEDING_TARGET`-sentinel carrier binding now install a real `pending_selection`. Task S1.3 appended a 24-slot `BREEDING_SOURCE_SELECT` action sub-range (`2168..2192`, keyed by carrier owner), raising `ACTION_SPACE_SIZE` 2168→2192. `material_zone_geometry` is the single branch point — battle-area carriers read `battle_area[index]`, breeding-area carriers read `breeding_area`. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier`.
- Gap kind: dsl + engine. FULLY CLOSED for battle-area AND breeding-area source plays (single, sequential, batch / different-name).
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause documented earlier under BT22-015 Decode.

## EX4-060 — Place self at bottom of own security stack face down  [G-PLACE-SELF-AT-SECURITY-BOTTOM]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, ... Then, place this Digimon at the bottom of your security stack face down."
- Status: CLOSED for EX4-060 on 2026-05-08. The DSL now has `place_permanent_on_security_and_handle_replacement`, which can target `replacement_subject`, choose top/bottom/random security placement, preserve face-down placement, trash leftover sources, and mark the active replacement custom-handled. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- Landed DSL verb / step kind: `place_permanent_on_security_and_handle_replacement`, used from a `kind: replacement` clause whose target is `replacement_subject`. Track E note: a sibling `EffectContext::place_self_at_security` (resolves `self.source_permanent` automatically) shipped on the same day for cards where the active resolver is itself the subject without needing an explicit binding; both helpers coexist.
- Closest pre-existing primitives (none of which sufficed before the new verb landed):

  - `add_this_option_to_hand: {}` — routes an Option from security-resolution staging to hand. Wrong destination zone and wrong subject scope.
  - `place_permanent_bottom_security_and_cancel_replacement` — targets ANOTHER permanent (selected via a binding) and CANCELS the replacement. Wrong subject (binding-selected, not self) and wrong outcome (cancel vs proceed-with-reroute).
- Engine substrate landed: `EffectContext::place_permanent_on_security_and_handle_current_replacement` delegates to `Game::place_permanent_on_security_without_leave_replacement`, which consumes the leaving permanent, consults `CannotAddSecurityByEffect`, places the top card into security, trashes leftover sources/linked cards, clears modifiers, and marks the replacement custom-handled. DCGO models the card-side shape via `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable(activateClass), toTop: false).PutSecurity()`.
- Replacement-outcome semantics: the step internally consumes the leave and routes the cards itself, then writes `CustomHandled` to the active replacement outcome.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_source: true
        - none_of:
            - replacement_cause: own_effect
    process:
      # ... other steps ...
      - place_permanent_on_security_and_handle_replacement:
          target: replacement_subject
          position: bottom
          face_up: false
  ```
- Workaround that would VIOLATE no-approximations: no longer needed for EX4-060.
- Also blocks: no longer blocks EX4-060. Keep this entry as a reference for any future card that needs a different timing surface from a leave-replacement body.
- Gap kind: dsl + engine, closed for the EX4-060 replacement-body form.
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl)

## ~~EX4-039 / EX4-038 — Event-target-not-source predicate for OnDigivolve  [G-EVENT-TARGET-NOT-SOURCE]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. This gap was STALE — the engine already carried both data points
  (`event_permanent` on `TriggerContext` for `Digivolved`, `source_permanent` on
  `EffectReadContext`) and the DSL predicate evaluator branch was present. The
  `complete-dna-omnimon-archetype` change authored the DNA Omnimon card clauses against
  the existing substrate and re-enabled the
  `ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves` behavioral test, which
  now passes. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion closure —
  2026-05-20" (STALE gaps list). Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text (both): "[Your Turn] [Once Per Turn] When one of your **other** Digimon digivolves, gain 1 memory."
- Status: OPEN as of 2026-05-03. EX4-039 surfaces it; EX4-038 has the same printed-text family.
- Missing DSL verb / step kind / predicate: a `CompiledPredicate` leaf such as `event_target_not_source: true` (or equivalently `event_permanent_not_source: true`) that returns false when the OnDigivolve trigger's `event_permanent` equals the inherited clause's `source_permanent` (the carrier permanent EX4-039 sits under). DCGO encodes this as `permanent != card.PermanentOfThisCard()` inside `CanTriggerWhenPermanentDigivolving`'s `PermanentCondition`.
- Lowers to engine API: `EffectReadContext::source_permanent()` already returns `Option<&Permanent>`; the trigger context's `event_permanent: Option<PermanentHandle>` is populated by `TriggerSource::Digivolved`. Comparing the two handles is a pure read — no new engine method needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - event_target_owner: you
      - event_target_kind: digimon
      - event_target_not_source: true
  ```
- Workaround applied today: `event_target_owner: you` + `event_target_kind: digimon`. Over-fires when the carrier permanent itself digivolves further (e.g. CARRIER-Lv4 → CARRIER-Lv5 while EX4-039 is a source under CARRIER). `once_per_turn: true` softens the impact to at most +1 spurious memory per turn. The negative-case behavioral test (`ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves`) is `#[ignore]`'d pending closure.
- Also blocks: EX4-038 Agumon (sister card, identical inherited text). Other "When one of your other Digimon ..." printed-text families across EX4 and BT5/BT12 will reuse the same predicate. DCGO grep for `permanent != card.PermanentOfThisCard()` inside `OnDigivolve` / `OnEnterFieldAnyone` PermanentCondition shows the pattern recurs across cards.
- Gap kind: dsl. Engine already has both data points (`event_permanent` on `TriggerContext` for `Digivolved`, `source_permanent` on `EffectReadContext`); only the DSL predicate surface and its evaluator branch in `eval_event_fields` are missing.
- First reported: 2026-05-03 (EX4-039 Gabumon, batch-implement-cards-rust-dsl)

## EX9-021 — `is_dna_digivolving` predicate on triggered clauses  [G-DSL-IS-DNA-DIGIVOLVING]
- Effect text: EX9-021 Omnimon Alter-S — "[When Digivolving] **If DNA digivolving**, your opponent's effects don't affect this Digimon for the turn. Then, delete all of their Digimon with the highest level." DCGO splits the body on `CardEffectCommons.IsJogress(_hashtable)` — a per-trigger hashtable flag set when the digivolve was a DNA / jogress path.
- Status: RESOLVED 2026-05-08 for the reusable event predicate under the engine/DSL spelling `dna_origin: true` / `false`. `TriggerSource::Digivolved` now carries `dna_origin`, `TriggerContext` stores it, `EffectReadContext` / `EffectContext` expose `event_dna_origin()`, and DNA digivolve drains set the bit for `WhenDigivolving`, `OnDnaDigivolve`, and global `OnDigivolve`. Effect-initiated DNA additionally sets `effect_initiated` on the global payload, so `event_is_effect_initiated` composes with `dna_origin`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3_dna_digivolve_triggers` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_when_digivolving`.
- Remaining limits: EX9-021 and BT17-078 still have card-local body gaps (`G-BIND-SELECTED-PROPERTY-FOR-EACH`, additional authored bodies, etc.), and BT16-085 still needs `G-SELECT-OPPONENT-SOURCES` for the DNA trash rider. Do not keep `G-DSL-IS-DNA-DIGIVOLVING` or the now-closed reusable self-to-security verb as the blocker for new authoring; use `dna_origin` plus the Track E zone-movement verbs.
- Missing DSL verb / step kind / predicate: `PredicateSpec` exposes no `is_dna_digivolving: bool` leaf, and the `condition:` shape on a triggered clause has no equivalent. There is also no clause-level `if:` form (matches in `process:` body) that can branch on the DNA-vs-standard digivolve origin.
- Engine substrate also missing: `TriggerSource::Digivolved { player, permanent, card }` (`code/digimon-engine/src/selection.rs:352`) has NO `via_dna` / `from_dna_pair` flag. The DNA digivolve action path (`Game::initiate_dna_digivolve` etc.) does not currently enqueue a distinct trigger source for the DNA case. The dispatch code that lifts `Digivolved { ... }` into `TriggerContext` (`effect_queue.rs` around line 479) builds a context with `event_permanent` / `event_card` / `source_player` but no DNA discriminator.
- Lowers to engine API: needs (a) `via_dna: bool` (or `dna_pair: Option<(CardHandle, CardHandle)>`) field on `TriggerSource::Digivolved`, populated from the DNA-digivolve action handler; (b) surfacing on `TriggerContext` so DSL predicates can read it; (c) DSL `is_dna_digivolving: Option<bool>` leaf on `PredicateSpec` + `CompiledPredicate` with an evaluator that consults the trigger context flag (false at non-trigger-time, same convention as `event_target_owner`).
- Suggested DSL syntax:
  ```yaml
  - when: when_digivolving
    condition:
      dna_origin: true
    process:
      - grant_effect_immunity:
          target: source
          source_kind: any
          source_controller: opponent
          expiry: end_of_turn
  ```
  (Optional symmetric dual: `is_standard_digivolving: true` for "[If standard digivolving] X" forms.)
- Workaround that would VIOLATE no-approximations: always grant the immunity (over-fires on the standard-digivolve path), or never grant it (under-fires on DNA — the printed protection is lost). Both are unfaithful. Per no-approximations the DNA-gated immunity arm is OMITTED. The unconditional delete-highest tail of EX9-021's [When Digivolving] IS implemented (printed grammar + DCGO sequencing both confirm the delete fires regardless of the DNA gate).
- Also blocks: any future card with "[When Digivolving] If DNA digivolving, X" or "[When Digivolving] If you DNA digivolved, X" style printed text. DCGO grep for `IsJogress(` returns multiple cards across sets (notably Omnimon-family / DNA-archetype cards). Sibling-but-distinct from AD1-001's `dna_origin: true` predicate, which reads card-data origin metadata rather than per-trigger event metadata.
- Gap kind: hybrid (engine TriggerSource needs the flag + dispatch wiring; DSL needs the predicate). Tests `ex9_021_when_digivolving_dna_path_grants_self_opp_effect_immunity` and `ex9_021_when_digivolving_standard_path_does_not_grant_immunity` are `#[ignore]`'d under this gap tag.
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl).

## EX9-021 — Place self at TOP of own security stack face-up  [G-PLACE-SELF-AT-SECURITY-TOP]
- Status: CLOSED for the reusable Track E DSL verb on 2026-05-09. YAML can now use `place_self_at_security: { position: top, face: up }`, lowering to `EffectContext::place_self_at_security`. EX9-021's production fixture currently uses the explicit binding form `place_permanent_on_security` because its "if this effect played" tail is already bound to the source permanent; the reusable self verb is covered by `parse_zone_movement_steps` and `zone_movement_verbs`.

[ORIGINAL ENTRY BELOW]

- Effect text: EX9-021 Omnimon Alter-S — "[End of Attack] ... If this effect played, place this Digimon as your top security card." DCGO: `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable, toTop: true).PutSecurity()` — places this permanent (top + sources) at the TOP of the controller's security stack (face-up; printed text does not specify face-down).
- Status: CLOSED for reusable DSL/security-placement vocabulary; original notes retained for provenance.
- Landed DSL verb / step kind: `place_self_at_security: { position: top|bottom|random, face: up|down }`.
- Engine substrate landed: `EffectContext::place_self_at_security(StackPosition, face_up)`.
- Suggested DSL syntax (option A — separate verbs):
  ```yaml
  - place_self_at_security_top: {}           # face-up by default
  ```
  (Option B — unified):
  ```yaml
  - place_self_at_security:
      position: top                          # top | bottom
      face: up                               # up | down (printed default
                                             # for top is up; for bottom is down)
  ```
- Workaround that would VIOLATE no-approximations: no longer needed for the reusable security-placement verb.
- Also blocks: no longer blocks future self-to-security placement syntax. Card-local source-play/result gates should be tracked separately.
- Gap kind: closed for the Track E verb.
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause tracked at `G-PLACE-SELF-AT-SECURITY-BOTTOM` (EX4-060).

## ST20-10 — Inverse alt-path direction: "this card may digivolve INTO X"  [G-ALT-PATH-DIRECTION-INTO] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry.
Schema + lowering + route resolution all ship. ST20-10's warp clause
remains BLOCKED on the companion `G-DSL-DISTINCT-TAMER-COLORS` predicate
leaf (the Tamer-colour disjunct of its condition); the opp-DP disjunct
is satisfiable today.

[ORIGINAL ENTRY BELOW]

## ST20-10 — Inverse alt-path direction (legacy)
- Effect text: ST20-10 Agumon — "[Your Turn] While your opponent has a Digimon with 10000 DP or more, or your Tamers have 3 or more total colors, this Digimon can digivolve into [WarGreymon] in the hand for a digivolution cost of 4, ignoring digivolution requirements." Other warp-style printed effects with the "this Digimon can digivolve into [Card] in the hand" shape are likely siblings (DCGO grep for `cardCondition: ... CardSource.EqualsCardName(...)` paired with `permanentCondition: ... == card.PermanentOfThisCard()` inside `AddSelfDigivolutionRequirementStaticEffect`).
- Status: OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `AltPathSpec` (in `digimon-dsl/src/alt_path.rs`) is implicitly source-directed — `from:` filters the SOURCE permanent / hand card that may digivolve INTO the carrier. There is no inverse form for "this card grants ITSELF the ability to digivolve into card X in hand." Authoring the alt-path on the destination card (WarGreymon's YAML) would over-broadcast: every Lv3 Agumon-named card on the field would be presented the path, and the destination YAML would have to enumerate every "warp into me" effect across the card pool. Authoring on the source (ST20-10) is the natural printed-text home but the DSL has no syntax for it.
- Lowers to engine API: the engine's activated-digivolve mechanism already supports both `cardCondition` (target hand-card filter) and `permanentCondition: target == self` (source = this card) in DCGO's `AddSelfDigivolutionRequirementStaticEffect`. The gap is purely DSL-side: a new `AltPathSpec` direction flag (or a new `kind: warp_into_hand` variant) needs to flip the semantic of `from:` to filter the destination instead of the source.
- Suggested DSL syntax (option A — direction flag):
  ```yaml
  alt_paths:
    - kind: activated_digivolve
      direction: into            # NEW: source = self, target = `into:` filter
      into:
        zone: [hand]
        of: you
        name_is: "WarGreymon"
      cost: 4
      ignore_requirements: true
  ```
  (Option B — dedicated kind): `kind: warp_into_hand` with required `into:` field (no `from:`); same lowering on the engine side.
- Workaround that would VIOLATE no-approximations: silently move the alt-path to WarGreymon's YAML (over-broadcasts to every Lv3 controller) or omit the gating predicate (path always available regardless of opp DP / Tamer colours). Per no-approximations the warp clause is OMITTED until this gap closes. Five behavioral tests in `code/digimon-engine/tests/cards_behavioral/st20/st20_10.rs` are `#[ignore]`'d under this gap tag (paired with `G-PRED-DP-LTE` or `G-DSL-DISTINCT-TAMER-COLORS`; the previously-companion `G-ALT-PATH-CONDITION` was RESOLVED 2026-05-15).
- Also blocks: any future "this Digimon can digivolve into [Card] in the hand for cost N" warp effect printed on the source card with a self-controller-state gate.
- Gap kind: dsl. Engine substrate already exists (DCGO uses the same `AddSelfDigivolutionRequirementStaticEffect` factory regardless of direction).
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Originally paired with `G-ALT-PATH-CONDITION` (BT24-016); that companion gap was RESOLVED 2026-05-15, so the inverse-direction hole is now the sole substrate blocker on this clause.

## ST20-10 — Distinct-Tamer-colours-on-field BoolPredicate  [G-DSL-DISTINCT-TAMER-COLORS] — RESOLVED 2026-05-17 (Phase 2 Track A)
- **Status:** Closed. The BoolPredicate wrapping is now covered by the formula leaf `play_cost_lte: { formula: { distinct_colors_count: { of: you, zone: battle_area, filter: { kind: tamer } } } }` shape. Phase 2 Track A swept stale references; the ST20-10 warp clause remains BLOCKED on its other companion gap `G-ALT-PATH-DIRECTION-INTO`'s ST20-specific YAML authoring (which has substrate but no card YAML yet) — but the Tamer-color disjunct itself is no longer the blocker.
- Effect text: ST20-10 Agumon — "...or your Tamers have 3 or more total colors..." (gating disjunct of the [Your Turn] warp clause). Sibling form of BT21-102 Tai Kamiya's "For each of your Tamers' colors, add 1 to this effect's play cost maximum" — both reference the same per-colour-count computation, but BT21-102 needs the value as a `FormulaSpec::per` aggregate (tracked under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) while ST20-10 needs it as a BoolPredicate threshold ("3 or more").
- Status (legacy): OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: no `distinct_tamer_colors_gte: <N>` (or generalised `distinct_colors_count_gte: <N>` over a controller / kind / zone selector) BoolPredicate leaf on `PredicateSpec`. The existing `distinct_colors_count` (added under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) is only available inside `FormulaSpec::per` — it cannot appear as a standalone boolean condition. `color_only` / `color_is` filter individual permanents by colour but do not aggregate colour counts across a permanent set.
- Lowers to engine API: DCGO's `Combinations.GetDifferenetColorCardCount(tamerCards) >= 3` returns the count of distinct colours present across the supplied permanent set, then thresholds. The engine's `eval_aggregate` (already used by `FormulaSpec::per: distinct_colors_count`) covers the count primitive — only the BoolPredicate wrapping is missing.
- Suggested DSL syntax (option A — dedicated leaf):
  ```yaml
  condition:
    distinct_tamer_colors_gte: 3
  ```
  (Option B — generalised over a permanent selector):
  ```yaml
  condition:
    distinct_colors_count:
      of: you
      zone: [battle_area]
      filter: { kind: tamer }
      gte: 3
  ```
- Workaround that would VIOLATE no-approximations: drop the disjunct entirely (gate fires only on opp ≥10000 DP, never on Tamer colours), or replace with a coarser proxy like "you have 3+ Tamers" (over-fires on three same-colour Tamers, under-fires on 3 distinct-colour Tamers some of which are deleted). Per no-approximations the entire warp clause is OMITTED until this gap (paired with `G-ALT-PATH-DIRECTION-INTO`) closes. The earlier-paired `G-ALT-PATH-CONDITION` was RESOLVED 2026-05-15.
- Also blocks: any future "while your Tamers have N or more total colours" or "if you have N or more distinct-colour Tamers" gate. Sibling to `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102) — the formula-aggregate form lands the underlying primitive; this gap closes the BoolPredicate wrapping. Both should land together once the formula primitive is generalised to also expose its result as a comparable scalar.
- Gap kind: dsl. Engine has the count primitive via `eval_aggregate`.
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Sibling of `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102).

## Puppets Resolver Residual DSL/Hybrid Gaps (2026-05-04)

## BT13-101 / P-136 — event predicates with suspend-this-Tamer cost  [PUPPETS-G023] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: `BT13-101`: "[All Turns] When you play a 2-color black/yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory." `P-136`: "[Your Turn] [Once Per Turn] When one of your Digimon digivolves into a Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). Event-card color predicates (`event_card_color_only`, `event_card_color_count`) landed, completing the second half of this gap. `BT13-101`'s All Turns observer and `P-136`'s digivolve observer are now expressible in YAML. The `bt13_101_all_turns_*` tests are un-ignored. See `qa/resolved-gaps.md` for the engine-side substrate closure.
- Status 2026-05-17: the **activation-cost half** of this gap closed under Phase 2 Track B. DSL `activation_cost: { suspend_self: true }` lifts onto `EffectBuilder::activation_cost(ctx.suspend_self_as_cost)`; cost failure (already-suspended source) consumes the OPT slot and skips the body silently (no decline-vs-fail elision). The **event-card colour predicates half** was still open at that point. See `qa/resolved-gaps.md` § Engine Gap: Generic `.activation_cost(...)` builder hook for triggered abilities for the substrate closure.
- Missing DSL verb / step kind / predicate: event-card predicates for exact color sets and color count, event-target owner/trait predicates for digivolve observers where needed, plus declarative source-bound triggered activation costs.
- Companion engine state: the generic triggered activation-cost hook is now resolved (`qa/resolved-gaps.md`); DSL `activation_cost: { suspend_self: true }` is wired and preflight comes for free via `EffectContext::suspend_self_as_cost` returning `false` on already-suspended sources.
- Suggested DSL syntax:
  ```yaml
  condition:
    all:
      - event_card_kind: digimon
      - event_card_color_only: [black, yellow]
      - event_card_color_count: 2
      # or, for P-136-style digivolve observers:
      - event_target_owner: you
      - event_card_trait_has: Puppet
  activation_cost:
    suspend_this_tamer: {}
  ```
- Gap kind: hybrid. Event-card color predicates are DSL/evaluator vocabulary; source-bound triggered cost preflight needs the engine cost surface.
- Workaround: None faithful. Name, trait, or broad color-includes filters would admit illegal cards for `BT13-101`, and auto-suspending the Tamer would hide a player-visible cost for both cards.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT13-101). Updated 2026-05-04 by Batch 11 for `P-136`.

---

## BT16-055 — narrow protection and inherited rules-text predicate  [PUPPETS-G024/PUPPETS-G025] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects." / "[Your Turn] While this Digimon has [Pulsemon] in its text, it gets +1000 DP."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). `grant_narrow_opponent_effect_protection` (PUPPETS-G024) and `rules_text_contains` predicate (PUPPETS-G025) both landed. `BT16-055` is now fully expressible in YAML. See `qa/resolved-gaps.md` for engine-side details.
- Missing DSL verb / step kind / predicate: category-scoped protection modifiers for opponent DP reduction and opponent De-Digivolve; inherited predicate over the carrier stack's printed rules text.
- Companion engine state: broad `CannotBeAffected` is too strong for the protection branch, and current inherited predicates do not inspect rules text on the carrier.
- Suggested DSL syntax:
  ```yaml
  protection:
    from: opponent
    categories: [dp_reduction, de_digivolve]
    while: { security_count_gte: 3 }

  active_when:
    carrier_text_contains: "Pulsemon"
  ```
- Gap kind: hybrid for narrow protection, DSL for rules-text contains predicate.
- Workaround: None faithful. Broad immunity or name predicates would over- or under-match printed behavior.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT16-055)

---

## EX11-060 — deletion event cause predicate for Overclock branch  [PUPPETS-G022]

- Effect text: "[All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted, by suspending this Tamer, <Draw 1>. If this effect was activated by <Overclock>, you may play 1 level 4 or lower [Puppet] trait Digimon card from your hand without paying the cost."
- Status 2026-05-06: `PUPPETS-G022` closed. Predicate leaf `event_cause` now compiles and evaluates against `TriggerContext.cause`; `overclock` is available as a first-class observer cause. Overclock sacrifice deletion preserves `ReplacementCause::Cost` for replacement windows while publishing `EventCause::Overclock` to `OnAnyDeletion` observers.
- Implemented DSL syntax:
  ```yaml
  condition: { event_cause: overclock }
  ```
- Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_060` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`.

---

## BT20-084 — trash-resident effect digivolve and stacked-card-to-security  [PUPPETS-G026/PUPPETS-G027]

- Effect text: "[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel]s may digivolve into this card without paying the cost." / "[End of All Turns] Place this Digimon's top stacked card as the top security card."
- Status 2026-05-09: `PUPPETS-G026` and the reusable `PUPPETS-G027` Track E verb are closed. DSL `when: on_ally_played` covers the trash-resident observer, and `security_place_top_stacked_card` now places the card below the visible top into security.
- Implemented trash-observer DSL syntax:
  ```yaml
  - when: on_ally_played
    optional: true
    condition: { event_target_kind: digimon }
    process:
      - select_own_permanent:
          bind_as: ciel
          filter: { name_is: "Sistermon Ciel" }
      - effect_initiated_digivolve:
          target: ciel
          source: self
          cost: free
          ignore_requirements: true
  ```
- Landed stacked-card DSL syntax:
  ```yaml
  - security_place_top_stacked_card:
      carrier: source
      of: you
      position: top
      face: up
  ```
- Gap kind: closed for the reusable top-stacked-card security movement. Future variants that select an arbitrary source use `security_place_stacked_card`.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt20_084_end_of_all_turns`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT20-084)

---

## BT22-088 — return-this-Tamer cost before branch free-play  [PUPPETS-G028] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] with a different card number in your hand without paying the cost, or play 1 [Shoemon] from your hand or trash without paying the cost."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). The `choose_one` branch selector with origin-preserving hand/trash play consumers (PUPPETS-G028) landed. `BT22-088`'s Start-of-Main tests are un-ignored. See `qa/resolved-gaps.md` for engine-side details.
- Status 2026-05-17: the **return-self-cost half** of this gap closed under Phase 2 Track B. DSL `activation_cost: { return_self_to_deck_bottom: true }` lifts onto `EffectBuilder::activation_cost(ctx.return_self_to_deck_bottom_as_cost)`; the engine queue's source-liveness check after the cost is now bypassed so the chained free-play branch can fire even though the source Tamer has left the field. The **branch selector half** was still open at that point.
- Missing DSL verb / step kind / predicate: optional triggered activation cost that moves the source permanent to the bottom of deck, then an in-effect branch selector with origin-preserving hand/trash play consumers.
- Companion engine state: the generic triggered activation-cost hook is now resolved (`qa/resolved-gaps.md`); the source-zone move helper lives on `EffectContext::return_self_to_deck_bottom_as_cost`. The chained branch selector with hand/trash consumers is still card-author DSL surface.
- Suggested DSL syntax:
  ```yaml
  activation_cost:
    return_this_tamer_to_bottom_deck: {}
  choose_one:
    - play_from_hand_free:
        filter:
          all_of:
            - name_is: "Arisa Kinosaki"
            - card_id_not: "BT22-088"
    - play_from_hand_or_trash_free:
        filter: { name_is: "Shoemon" }
  ```
- Gap kind: hybrid. The cost/preflight is engine-facing; branch and origin-preserving selection need DSL vocabulary.
- Workaround: None faithful. Auto-returning the Tamer or auto-selecting Shoemon/Arisa would hide printed player-visible choices.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT22-088)
- Status 2026-05-11: Still open for the Start of Your Main Phase return-this-Tamer cost and chained Arisa/Shoemon free-play branches. The separate All Turns Token/Puppet played observer is now implemented in `BT22-088.yaml` using `source_is_unsuspended`, visible suspend/decline selection, and event-target Token/Puppet filters.

---

## BT23-077 — self-scoped OnSuspend event predicate  [PUPPETS-G029]

- Effect text: "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon."
- Status 2026-05-08: `PUPPETS-G029` closed. `event_permanent_is_source` compiles and evaluates against `TriggerContext.event_permanent` and the observer source permanent, and BT23-077 now uses it for the printed self-suspend `<De-Digivolve 1>` clause.
- Companion engine state: `OnSuspend` dispatch exists and event context is available for observed suspend events; this slice adds the missing self-scoped predicate.
- Suggested DSL syntax:
  ```yaml
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
      - de_digivolve: { target: target, count: 1 }
  ```
- Gap kind: dsl predicate/evaluator gap, closed for BT23-077.
- Workaround: no longer needed for BT23-077. A broad `on_suspend` trigger remains an approximation for any future "this permanent" authoring that does not use `event_permanent_is_source`.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT23-077)
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.

---

## BT5-106 — effect-play On Play suppression provenance  [PUPPETS-G030] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). `suppress_on_play` flag on effect-play helpers (PUPPETS-G030) landed. `BT5-106`'s Security slice is now expressible in YAML. See `qa/resolved-gaps.md` for engine-side details. (Phase 2 Track J Task S1.1 closed the same gap independently — see [`qa/resolved-gaps.md`](resolved-gaps.md); the merged engine keeps the Puppets-sweep design.)
- Missing DSL verb / step kind / predicate: a play-from-trash/free-play consumer that carries `suppress_on_play: true` provenance for the played Digimon only.
- Companion engine state: ordinary effect play from trash can enter the Digimon and normally fire On Play; this card needs the same player-visible trash selection but must skip the played permanent's On Play enqueue for that play event.
- DSL syntax (shipped): `suppress_on_play: true` is honored ONLY by `play_from_trash_free`; the compiler rejects it on `play_from_hand` / `play_from_trash`.
  ```yaml
  - play_from_trash_free:
      of: you
      hand_index: revived
      suppress_on_play: true
  ```
- Gap kind: hybrid. Engine play provenance needed an On Play suppression flag, and DSL needed vocabulary to request it — both shipped.
- Deferred follow-up: `suppress_on_play` on `play_from_materials` (Royal Knights source-play payoffs) is NOT wired — the merged engine threads suppression only through `play_from_trash_free`. Re-wiring the `play_from_materials` path is follow-up work for when the RK source-play cards are authored.
- Workaround: no longer needed.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT5-106)
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- suppress_on_play`.

## BT3-002 — `carrier_has_keyword` predicate for inherited clause conditions  [G-DSL-CARRIER-HAS-KEYWORD]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; `has_keyword` already resolves against the carrier permanent for inherited clauses (`enqueue_from_permanent` sets `source_permanent` to the carrier handle). No predicate added.

- Effect text: "Inherited Effect [When Attacking] [Once Per Turn] If this Digimon has <Jamming>, <Draw 1> (Draw 1 card from your deck.)"
- Card first discovered in: BT3-002 DemiVeemon (Digi-Egg, Lv.2, Blue)
- Missing DSL verb / step kind / predicate: `carrier_has_keyword` — a `PredicateSpec` / `BoolPredicate` leaf for inherited triggered clauses that checks whether the TOP CARD of the permanent carrying the egg source has a given keyword (printed OR modifier-granted). The existing `has_keyword` predicate in `CompiledPredicate` evaluates on `source_permanent` (the egg slot itself), not the carrier permanent. For inherited effects, `source_permanent` is the bottom-of-stack source card, not the carrier Digimon.
- Lowers to engine API: `Game::has_keyword(carrier_handle, Keyword::Jamming)` — the engine has this method (used in `combat.rs`, `game.rs`). The gap is that the DSL predicate evaluator has no path to resolve the carrier handle from `EffectReadContext` for inherited clauses. The carrier handle is `EffectReadContext.source_permanent` (if it exists) but only when the source IS the top card; for sub-stack inherited sources, the context's `source_permanent` is the egg, not the carrier.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    optional: true
    condition: { carrier_has_keyword: Jamming }
    process:
      - draw: { of: you, count: 1 }
  ```
- Gap kind: dsl (engine has `Game::has_keyword` and modifier tracking; DSL lowering just needs a new predicate leaf that reads the carrier handle from the inherited-effect dispatch context rather than the source permanent).
- Workaround: Omit the `condition` from the YAML entirely (preferred). The clause over-fires without the Jamming gate — any carrier with BT3-002 in its digivolution cards will draw on attack regardless of Jamming. The over-fire is documented in BT3-002.yaml. The negative-condition test `bt3_002_does_not_fire_without_jamming` is `#[ignore = "pending: G-DSL-CARRIER-HAS-KEYWORD from qa/dsl-vocab-gaps.md"]`.
- Trade-off of omission vs. un-gated clause: omission is preferred because the Draw 1 step is safe (no permanent game-state harm), the positive case (carrier has Jamming → draw) is the common path this egg was designed for, and over-firing without Jamming is a minor accuracy loss rather than a silent break.

---

<!-- ───────────────────────────────────────────────────────────────────────
  BG IMPERIAL PHASE 0 RE-AUDIT — 2026-05-20 (`bg-imperial-substrate-closeout`)

  The BG Imperial entries below were re-verified against current source.
  STALE (primitive shipped — entry should be closed at change closeout):
    G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED (lower_grant_keyword.rs:18-36
      now consumes active_when), G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME (already
      marked resolved), G-IS-EFFECT-INITIATED (event_is_effect_initiated exists),
      G-BEFORE-PAY-COST-GAIN-MEMORY (resolved Track H), G-EFFECT-INITIATED-
      DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET (resolved Track F),
      G-DSL-TRASH-TOP-N-DIGI-CARDS (closed), G-DSL-UNION-PLAY-FREE
      (play_union_bound_free / PUPPETS-G014 shipped), G-DSL-SELF-COLOR-COUNT-GTE
      (self_color_count_gte shipped), G-EVENT-CARD-COLOR-IS partial
      (event_card_color_only/_count shipped — only a `_has`-semantics leaf
      remains), G-FORMULA-SOURCE-DP (source_dp formula shipped).
  GENUINE — now CLOSED by `bg-imperial-substrate-closeout` (2026-05-20), see
    qa/resolved-gaps.md § "BG Imperial substrate closeout": G-DSL-EFFECT-
    SUSPENDED-RESULT (`effect_suspended_any_opponent_digimon`),
    G-EVENT-CARD-COLOR-IS (`event_card_color_has`), G-SELECT-OPPONENT-SOURCES
    (`select_opponent_sources`), G-ZONE-SELECTED-TRASH-TO-DECK-TOP
    (`move_trash_card_to_deck_top`), G-ANY-RETURNED-CARD-PREDICATE
    (`returned_card_matching`).
  REDUNDANT — audit correction: these 4 were NOT genuine gaps; pre-existing
    capability already covers them, no predicate was added —
    G-PRED-STACK-SIZE-LTE-SOURCE / G-DSL-STACK-SIZE-LTE-SOURCE
    (`materials_count_lte` + `source_material_count` formula),
    G-DSL-CARRIER-HAS-KEYWORD (`has_keyword` resolves to carrier for inherited
    clauses), G-DSL-AURA-TARGET-SOURCE-PERMANENT (`scope: inherited` +
    `target: {}` self-aura), G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT
    (`source_permanent_trait_has`). See qa/resolved-gaps.md § "BG Imperial
    substrate closeout" → "Audit correction". The 2026-05-22 BG Imperial
    readiness reconciliation verified that the deck-library pool now consumes
    the new + pre-existing vocabulary without live raw_rust escapes.
  Verified per-card classification:
    openspec/changes/bg-imperial-substrate-closeout/phase-0-audit.md
─────────────────────────────────────────────────────────────────────── -->

## BT12-022 — `active_when` on `kind: grant_keyword` declarative clauses is not consumed  [G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — stale; `lower_grant_keyword.rs` already consumes `active_when` on `kind: grant_keyword` declarative clauses.

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" (BT12-022 ExVeemon, inherited)
- Missing DSL verb / step kind / predicate: `DeclarativeClause.active_when` is compiled into `CompiledDeclarativeClause::GrantKeyword { active_when, .. }` but is silently discarded by `lower_grant_keyword::lower` in `code/digimon-engine/src/dsl_cards/mod.rs` (line 82-98 uses `..` to destructure, ignoring `active_when`). The `lower_grant_keyword::lower` function signature has no `active_when` parameter.
- Companion state: `CompiledDeclarativeClause::GrantKeyword` does carry the `active_when: Option<CompiledPredicate>` field (compiled.rs:432). The `lower_aura::lower` function accepts and uses `active_when` correctly. The gap is that `lower_grant_keyword::lower` does not accept or apply it.
- Consequence: any `kind: grant_keyword` clause with `active_when:` specified will grant the keyword unconditionally — the condition is silently dropped. Cards relying on `active_when` to gate keyword grants over-fire.
- Lowers to engine API: `Effect::declarative(card).condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None))` — the condition closure already exists in `lower_aura::lower`; the same pattern needs to be applied in `lower_grant_keyword::lower`. Additionally, `Game::has_keyword` checks `effect.condition` for inherited declarative effects (game.rs lines 1717-1727) — so adding the condition to the `Effect` struct (not only the modifier tick) would gate the keyword check correctly without a declarative tick.
- Suggested fix:
  1. Add `active_when: Option<CompiledPredicate>` parameter to `lower_grant_keyword::lower`.
  2. In `mod.rs`, pass `active_when.clone()` to the call.
  3. Inside `lower_grant_keyword::lower`, add `if let Some(aw) = active_when { builder = builder.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None)); }`.
- Gap kind: dsl (engine has condition support on `Effect` struct; only the lowering wire-up is missing).
- Workaround: none needed. BT12-022 now ships with `active_when` consumed by
  `grant_keyword` lowering and focused negative-condition coverage active.
- Cards affected: BT12-022 ExVeemon (inherited conditional Jamming).
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)

---

## BT12-022 — BeforePayCost triggered gain_memory for "would DNA digivolve into" target  [G-BEFORE-PAY-COST-GAIN-MEMORY]

- **Status: RESOLVED 2026-05-17** (Phase 2 Track H). See `qa/resolved-gaps.md` § "Phase 2 Track H closure — 2026-05-17" for the substrate landed (sibling `Effect::before_pay_cost_observe` builder + `EffectTiming::BeforePayCostObserve` + `scan_before_pay_cost_observers` dispatch).
- Authoring pattern:
  ```yaml
  - when: before_pay_cost_observe
    active_when:
      all_of:
        - your_turn: true
        - dna_origin: true
        - source_is_cost_target_permanent: true
        - cost_target: { color_is: green, kind: digimon }
    process:
      - gain_memory: 1
  ```
- Cards implemented and validated: BT12-022 ExVeemon (clause 0), BT12-050 Stingmon (clause 0).
- Companion gap (also resolved): G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — see entry above.
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)
- First reported: 2026-05-04 (BT3-002 DemiVeemon DSL implementation)

## EX1-014 — `aura` declarative target scoping  [G-DSL-AURA-TARGET-SOURCE-PERMANENT]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; a `kind: aura` with `scope: inherited` + `target: {}` is a carrier-only self-aura. No leaf added.

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" — should grant Jamming ONLY to the carrier permanent (the Digimon containing this card in its digivolution stack), not all controller-side Digimon.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue), also in BT12-022 (sister card).
- Missing DSL verb / step kind / predicate: `target_is_source: true` BoolPredicate (or equivalent) usable inside `kind: aura` `target:` filter, so the aura applies only to the carrier of the source permanent — not the entire `target: { owner: you, kind: digimon }` set. Currently `lower_aura.rs` applies to all matches of the target predicate.
- Lowers to engine API: `target` filter check `handle == ctx.source_permanent` (or `handle == carrier_of(source)` for inherited-source clauses).
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    target: { owner: you, kind: digimon, is_carrier_of_source: true }
    grant_keyword: jamming
    active_when: { ... }
  ```
- Gap kind: dsl. Engine has the carrier handle resolution; only the predicate leaf is missing.
- Workaround: ship aura with broad target (over-fires to all your Digimon).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## EX1-014 — `self_digivolution_contains_trait` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; the existing `source_permanent_trait_has` predicate covers EX1-014's `[Free]`-trait arm. No predicate added.

- Effect text: "...has [Imperialdramon] in its name or the [Free] trait..." — needs a predicate that evaluates whether the carrier permanent's digivolution stack contains a card with a given trait.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_trait: <trait>` — boolean predicate over carrier permanent's digivolution stack. `source_permanent_trait_has` exists in `CompiledPredicate` spec but is not evaluated at runtime in `predicate.rs`.
- Lowers to engine API: `rctx.source_permanent()?.has_trait(name, rctx.card_data())` — engine has the data.
- Suggested DSL syntax:
  ```yaml
  active_when: { self_digivolution_contains_trait: "Free" }
  ```
- Gap kind: dsl.
- Workaround: omit the trait arm of the active_when (only name arm fires).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## BT16-040 — chained selection → effect_initiated_digivolve  [G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET] — RESOLVED 2026-05-17 (Phase 2 Track F)

Resolved as **phantom** — see [resolved-gaps.md](resolved-gaps.md)
"Phase 2 Track F closure". The chain dispatcher
(`run_tail_preserving_trigger_context`) was already driving the chain
to completion; the prior tests asserted mid-chain state and panicked
on the auto-resolved post-state. 5 tests now active.

[ORIGINAL ENTRY BELOW]

## BT16-040 — effect-initiated digivolve chain (legacy)

- Effect text: "[Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Insectoid] or [Free] trait in your trash with the digivolution cost reduced by 1." — process chain: select_own_permanent → select_trash_card → effect_initiated_digivolve.
- Card first discovered in: BT16-040 Wormmon (Digimon, Lv.3, Green/White). Same gap blocks BT17-015, BT17-027 clause 0.
- Missing DSL verb / step kind / predicate: process chain terminates after the permanent pick; the trash-pick prompt and `effect_initiated_digivolve` verb never execute when the source target is bound from a previous `select_own_permanent` step.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve` exists; the chain orchestration in the lowering layer does not resume after the first pick when the resolved source binding feeds into a subsequent select prompt.
- Suggested DSL syntax: existing chain syntax should work; the gap is in the process-step continuation mechanism.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; structural test passes, behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-040 batch-implement-cards-rust-dsl)

## BT12-028 / BT16-025 / BT16-027 — `stack_size_lte_source` predicate  [G-PRED-STACK-SIZE-LTE-SOURCE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; `materials_count_lte: { formula: { source_material_count: {} } }` already expresses "as many or fewer digivolution cards as this Digimon". No predicate added.

- Effect text variants: "Return 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon to the bottom of the deck." (BT16-027) / "Suspend all of your opponent's Digimon with as many or fewer digivolution cards as this Digimon" (BT16-025).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT16-025 Paildramon (same gap).
- Missing DSL verb / step kind / predicate: `stack_size_lte_source: bool` BoolPredicate leaf evaluating `candidate.card_sources.len() <= source_permanent.card_sources.len()` at runtime. The existing `stack_size_lte: <u8>` takes a literal, not a dynamic source-stack reference.
- Lowers to engine API: `Game::permanent(handle).card_sources.len()` for both candidate and source — engine has the data; only the predicate dispatch is missing.
- Suggested DSL syntax: `filter: { stack_size_lte_source: true }` inside `select_opp_field` / `select_permanent`.
- Gap kind: dsl.
- Workaround: clauses omitted from runtime; structural tests pass; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## ~~BT12-028 / BT16-027 — `self_digivolution_contains_name` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The sources-only `self_digivolution_sources_contain_name` predicate
  leaf landed in the `complete-dna-omnimon-archetype` change, evaluating whether the source
  permanent's own `card_sources` stack contains a card matching the given name via
  `Permanent::contains_card_name`. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon
  completion closure — 2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: "if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards" (BT16-027). Sister of `G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT` (EX1-014).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT12-028 (`source_name_contains` family).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_name: <name>` BoolPredicate leaf evaluating whether the source permanent's own `card_sources` stack contains a card matching the given name. `source_name_contains` is defined in `PredicateSpec` and validated, but has no runtime evaluation branch in `predicate.rs`.
- Lowers to engine API: `Permanent::contains_card_name` — engine has the primitive; only the predicate dispatch wiring is missing.
- Suggested DSL syntax: `condition: { self_digivolution_contains_name: "Imperialdramon: Dragon Mode" }`.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## BT12-028 — `trash_top_n_digivolution_cards` step + engine primitive  [G-DSL-TRASH-TOP-N-DIGI-CARDS]

- Effect text: "Trash the top 3 digivolution cards of all of your opponent's Digimon." (BT12-028 clause 0a).
- Card first discovered in: BT12-028 Paildramon. Sibling to G-ASL-07 (BT17-077 all-source mass trash).
- Status: CLOSED for the reusable Track E DSL verb on 2026-05-09. YAML can now use `trash_top_n_digivolution_cards_of_each: { of: opponent, n: 3 }`, which lowers to `EffectContext::trash_top_n_digivolution_cards_of_each`. Evidence: `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_zone_movement_steps`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`.
- Landed DSL verb / step kind: `trash_top_n_digivolution_cards_of_each: { of: opponent, n: 3 }`.
- Lowers to engine API: `EffectContext::trash_top_n_digivolution_cards_of_each(target_player, n)`.
- Gap kind: closed for the bounded top-N-each reusable primitive. BT17-077's
  "all sources" sibling is also covered by the later BG Imperial substrate
  closeout.
- Workaround: no longer needed; BT12-028 is implemented in production YAML.
- First reported: 2026-05-04 (BT12-028 batch-implement-cards-rust-dsl).

---

## BT16-025 — `binding_is_none` / "if-no-target" predicate  [G-DSL-IF-NO-TARGET]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `binding_present` / `binding_absent` predicates (alias `binding_is_none`) exist and cover the "if this effect didn't suspend" branch.

- Effect text: "Suspend 1 of your opponent's unsuspended Digimon. If this effect didn't suspend, unsuspend this Digimon." (BT16-025 clause 2).
- Card first discovered in: BT16-025 Paildramon.
- Missing DSL verb / step kind / predicate: `select_opponent_permanent` with `optional: true` skips silently when no targets exist, but does not bind a "skipped" flag. Need `binding_is_none: <name>` BoolPredicate for subsequent `if` conditions to test whether the previous selection was taken or skipped.
- Lowers to engine API: existing binding mechanism — only the BoolPredicate leaf is missing.
- Suggested DSL syntax:
  ```yaml
  - if:
      condition: { binding_is_none: tgt }
      then: [ unsuspend: { target: source } ]
  ```
- Gap kind: dsl.
- Workaround: conditional unsuspend-self omitted from runtime; behavioral test `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-025 batch-implement-cards-rust-dsl).
- Also blocks: BT16-028 clause 0b — "[When Digivolving] by suspending 1 of their Digimon or Tamers, unsuspend 1 of your Digimon." Same structural gap: the optional suspend-cost step produces no binding result flag, so the own-unsuspend reward arm cannot be made conditional on the cost being paid. Cross-listed 2026-05-04.

---

## BT16-028 — `event_is_effect_initiated` predicate  [G-IS-EFFECT-INITIATED]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `event_is_effect_initiated` predicate exists and BT16-028 consumes it for the effect-play/digivolve observer gate.

- Effect text: "[All Turns] When an effect plays or digivolves an opponent's Digimon, if you have a Tamer, this Digimon may digivolve into [Imperialdramon: Fighter Mode] in the hand without paying the cost."
- Card first discovered in: BT16-028 Imperialdramon: Dragon Mode (2026-05-04).
- Status 2026-05-08: PARTIALLY RESOLVED. `PredicateSpec::event_is_effect_initiated` now compiles and evaluates against `TriggerContext.effect_initiated`. `TriggerSource::EnteredField` and `TriggerSource::Digivolved` carry the flag; normal hand play/digivolve set it false, while effect play helpers and `effect_initiated_digivolve` set it true. BT16-028 now authors the effect-play/digivolve observer with this gate.
- Remaining limits: This closes the reusable "by an effect" flag for `OnEnterFieldAnyone` / standard `OnDigivolve` observer predicates. It does not close stricter "by THIS effect" per-activation identity, effect-spawned permanent cleanup tokens, or DNA-specific origin flags.
- Lowers to engine API: `TriggerContext.effect_initiated`.
- Suggested DSL syntax:
  ```yaml
  - when: [on_enter_field_anyone, on_digivolve]
    optional: true
    active_when: { all_turns: true }
    condition:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
        - event_is_effect_initiated: true    # ← new predicate leaf
        - any_permanent:
            of: you
            zone: [battle_area]
            kind: tamer
    process:
      - select_hand:
          of: you
          bind_as: fighter
          filter:
            all_of:
              - kind: digimon
              - name_contains: "Imperialdramon: Fighter Mode"
          prompt: "Digivolve into Imperialdramon: Fighter Mode (free, ignore reqs)"
      - effect_initiated_digivolve:
          target: source
          from_hand: fighter
          cost: 0
          ignore_requirements: true
  ```
- Gap kind: hybrid (engine must thread the cause flag through TriggerContext; DSL then needs the predicate leaf).
- Workaround: no longer needed for BT16-028's effect-play half. Remaining ignored BT16-028 subtests cover narrower card-local follow-ups, not the reusable predicate itself.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_028`.
- First reported: 2026-05-04 (BT16-028 batch-implement-cards-rust-dsl).

---

## BT12-031 — Alt-cost: return named source card from own digi-stack to hand  [G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — `EffectContext::return_card_source_to_hand` + the `return_selected_sources_to_hand` DSL verb landed; BT12-031 Step C now ships → BT12-031 IMPLEMENTED. Full record in `qa/resolved-gaps.md` § "Follow-up engine gaps closed (2026-05-21)" and `docs/RUST_ENGINE_GAPS.md`.

- **Canonical record relocated 2026-05-21 (`bg-imperial-substrate-closeout`): this is an ENGINE gap, not DSL-only.**
  The full scoping entry now lives in
  [`docs/RUST_ENGINE_GAPS.md`](../docs/RUST_ENGINE_GAPS.md#return-a-selected-digivolution-stack-source-card-to-its-owners-hand)
  ("Return a selected digivolution-stack source card to its owner's hand",
  🟡 PARTIAL) — consult that entry for the suggested `EffectContext` /
  DSL-verb / YAML shape, likely files, complexity estimate, first test,
  and known interactions. This `qa/dsl-vocab-gaps.md` entry is retained
  only as a redirect; do not treat the notes below as the live spec.
  Summary: the two sub-gaps below (select-own-sources filter; `binding_present`)
  are both resolved, but BT12-031 Step C is still BLOCKED on a genuine missing
  engine primitive — there is **no DSL verb / `EffectContext` method that
  returns a single selected digivolution-stack source card to its owner's
  hand**. The only source-ref consumers are `trash_selected_sources` and
  `play_selected_sources_free`; `return_to_hand` moves a whole permanent, not
  one source card. BT12-031 Step C stays omitted, 2 tests `#[ignore]`'d, card
  verdict PARTIAL. Suggested fix: an `EffectContext` method + DSL verb (e.g.
  `return_selected_sources_to_hand`) routing each chosen source `Card` to its
  owner's hand. BT12-031 clause 1b (Security A.+1 + Blocker via
  `self_color_count_gte` `while_condition`) IS implemented.
- Effect text (BT12-031 Clause 0, Step C): "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's digivolution cards to its owner's hand, return all of your opponent's suspended Digimon to the bottom of their owners' decks instead."
- Missing DSL verb / step kind / predicate: Two sub-gaps combine to block this step:
  1. **G-DSL-SELECT-OWN-SOURCES-FILTER** — resolved 2026-05-08. `select_own_sources` now accepts `filter:` and evaluates it against each source card, with optional `from:` host restriction.
  2. **G-DSL-BIND-PRESENT** (see EX9-066 entry) — After the optional selection, the alternative outcome must be conditioned on whether the player made a selection or passed. The `binding_present` predicate does not exist.
- Synthesizing gap ID: `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` — filing as a composite gap for the BT12-031 context.
- DCGO reference: `BT12_031.cs` — step C via optional `AddSelectCard` from own digi-cards filtered by `EqualsCardName("Imperialdramon: Dragon Mode")`, `canNoSelect: () => true`. If selected, card returns to hand and all suspended opp Digimon return to bottom of deck; if declined, only the single return-to-hand fires.
- Suggested DSL syntax:
  ```yaml
  - select_own_sources:
      bind_as: dragon_mode_src
      optional: true
      filter:
        name_is: "Imperialdramon: Dragon Mode"
      prompt: "Return [Imperialdramon: Dragon Mode] from your digivolution cards to hand to return ALL opponent suspended Digimon to bottom of decks instead"
  - if:
      condition:
        binding_present: dragon_mode_src
      then:
        - return_to_hand: { target: dragon_mode_src }
        - for_each:
            over:
              all_of:
                - of: opponent
                - zone: [battle_area]
                - kind: digimon
                - is_suspended: true
            bind_as: susp_opp
            body:
              - return_to_deck:
                  target: susp_opp
                  position: bottom
                  include_sources: false
      else:
        - select_opponent_permanent:
            bind_as: suspended_target
            filter:
              all_of:
                - kind: digimon
                - is_suspended: true
            prompt: "Return 1 of your opponent's suspended Digimon to its owner's hand"
        - return_to_hand: { target: suspended_target }
  ```
- Lowers to engine API: `select_own_sources` filtering is now in place; remaining work is DSL-only.
  - `binding_present` predicate: add leaf that checks `ctx.bindings.get(name).is_some()`.
- Updated 2026-05-07: `select_own_sources.target` can now restrict the picker to a specific permanent binding, which covers self-stack cost shapes like Digi-Burst. This does **not** close the card-name source filter needed here; BT12-031 still needs `filter:` over source card identity plus `binding_present`.
- Gap kind: DSL only.
- Workaround: Steps A (for_each suspend no-digi-card targets) and B (select 1 suspended opp → return to hand) are authored in BT12-031.yaml. Step C is commented out as BLOCKED.
- Behavioral tests: 2 tests `#[ignore = "pending: G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME from qa/dsl-vocab-gaps.md ..."]` in `code/digimon-engine/tests/cards_behavioral/bt12/bt12_031.rs`.
- First reported: 2026-05-04 (BT12-031 TDD implementation).

---

## BT17-077 — `return_all_trash_to_deck_bottom` step + player-choice target  [G-RETURN-ALL-TRASH-TO-DECK-BOTTOM]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the player-choice-of-trash branch is composable via `select_effect_choice` + `if` branching `return_all_trash_to_deck_bottom: { of: you|opponent }`; BT17-077 Clause 1b now ships.

- Effect text: "Then, return all cards from your or your opponent's trash to the bottom of the deck." (BT17-077 Clause 1b).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode.
- Status: PARTIALLY CLOSED on 2026-05-09 for the reusable bulk-zone DSL verb. YAML can now call `return_all_trash_to_deck_bottom: { of: you|opponent }`, and owner-routing is covered by `zone_movement_verbs::bulk_trash_and_hand_reduction_verbs_call_helpers`. The remaining printed-card gap is the player-choice branch for "your or your opponent's trash" and the returned-card result predicate for the memory rider.
- Landed DSL verb / step kind: `return_all_trash_to_deck_bottom: { of: <player_ref> }` — moves every card currently in the specified player's trash zone to the bottom of its owner's deck.
- Lowers to engine API: `EffectContext::return_all_trash_to_deck_bottom(player)`.
- Companion gap: the printed text says "your or your opponent's trash" — the choice of whose trash is returned is a player decision (DCGO: `BoolSelection`). This requires either `select_effect_choice` (choose 0 or 1) + `if` conditional wiring the correct `of:` player, or a single parametric verb `return_all_trash_to_deck_bottom: { of: chosen_player }` where `chosen_player` is a binding. Neither is currently in the DSL.
- Suggested DSL syntax:
  ```yaml
  - select_effect_choice:
      bind_as: whose_trash
      labels: ["Your Trash", "Opponent's Trash"]
      prompt: "Return all cards from your or your opponent's trash to the bottom of the deck"
  - if:
      condition: { equals: [whose_trash, 0] }
      then:
        - return_all_trash_to_deck_bottom: { of: you }    # ← new verb
      else:
        - return_all_trash_to_deck_bottom: { of: opponent }  # ← new verb
  ```
- Gap kind: closed. Engine bulk-move, DSL verb, owner routing,
  player-choice branching, and the returned-card predicate are covered for
  BT17-077's full printed clause.
- Workaround: none needed. BT17-077 Clause 1b and the dependent Clause 1c memory
  rider ship in YAML and are covered by focused behavioral tests.
- Cross-ref: G-ASL-07 (qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md) tracks the remaining all-source/player-choice/result-predicate family.
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).

---

## BT17-077 — `any_returned_card` result predicate  [G-ANY-RETURNED-CARD-PREDICATE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `returned_card_matching` filtered result predicate landed (`bg-imperial-substrate-closeout` Tier 2); BT17-077 Clause 1c now ships.

- Effect text: "If this effect returned a white level 7 card, gain 3 memory." (BT17-077 Clause 1c).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode. Clause 1c fires after the `return_all_trash_to_deck_bottom` step (Clause 1b) completes; the memory gain is conditional on at least one of the moved cards satisfying `color: white AND level: 7`.
- Missing DSL verb / step kind / predicate: `any_returned_card: { color_is: white, level_eq: 7 }` — a BoolPredicate that evaluates to true if the immediately preceding zone-move step returned at least one card matching the given filter. There is no "result-set predicate" that can inspect the set of cards moved by a prior step.
- Lowers to engine API: the step would need to bind a `Vec<CardData>` of moved cards as an effect-local result, which the subsequent `if` condition can test via `any_returned_card` iterating over that result set.
- Suggested DSL syntax:
  ```yaml
  - return_all_trash_to_deck_bottom:
      of: opponent
      bind_returned_as: returned_cards    # optional result binding
  - if:
      condition:
        any_returned_card:                # new BoolPredicate leaf
          binding: returned_cards
          color_is: white
          level_eq: 7
      then:
        - gain_memory: 3
  ```
- Gap kind: dsl (engine result-binding infrastructure would also need extending for the `bind_returned_as` step argument).
- Workaround: none needed. Clause 1c ships in BT17-077.yaml using
  `returned_card_matching`.
- Cross-ref: G-RETURN-ALL-TRASH-TO-DECK-BOTTOM (above) must close first (Clause 1b provides the moved-card set that Clause 1c predicates on).
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).
---

## Royal Knights — Delay/keyword leave-prevention replacements  [RK-G003]

- Effect text: `BT20-100` The Last Guardian: "[All Turns] When any of your Digimon with [Omnimon] in its name would leave the battle area, <Delay> ... 1 of those Digimon doesn't leave." `BT23-054` Magnamon: "<Armor Purge> (When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.)"
- Status: closed for the Track B consumers. BT20-100's option-as-Delay source cost is represented by the replacement lowering shape `delete_permanent: { target: source }` followed by `cancel_replacement: {}`; the lowering only cancels after the delayed option actually reaches trash. BT23-054 uses the Armor Purge keyword replacement, prompts accept/decline, and trashes the top source only on accept.
- Companion engine state: Delay and Armor Purge both route through the shared replacement framework and existing pending-selection masks; no action-space expansion was required.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    source_is_delay_option: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - name_contains: "Omnimon"
    cost:
      trash_source_delay_option: {}
    process:
      - cancel_replacement: {}

  - kind: grant_keyword
    keyword: ArmorPurge
  ```
- Gap kind: closed for `BT20-100` and `BT23-054`; future cards should file a new narrower gap only if their cost/filter shape cannot be expressed through `kind: replacement` or the Armor Purge keyword.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_100_delay`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_054_armor_purge`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- First reported: 2026-05-05 (Royal Knights Batch 2: BT20-100, BT23-054).

---

## Royal Knights — would-leave observer that plays from hand without cancelling  [RK-G004]

- Effect text: `BT20-091` Cool Boy: "[Opponent's Turn] [Once Per Turn] When any of your Digimon with the [Royal Knight] trait would leave the battle area, you may play 1 [Omekamon] from your hand without paying the cost."
- Status: narrowed/closed for `BT20-091`. A `kind: replacement` clause can intentionally leave the outcome unset, which runs the side-effect and then lets the original leave event proceed. The `select_hand` step is required (`optional: false`) so the replacement is not offered when no Omekamon can be played; optionality lives on the outer replacement prompt.
- Companion engine state: `kind: replacement` observes would-leave events with event subject filters, OPT accounting, and ordinary pending hand selection/play. Non-cancelling subscribers are represented by replacement processes that do not call `cancel_replacement`, `redirect_replacement`, `substitute_replacement`, or `handle_replacement`.
- Suggested DSL syntax:
  ```yaml
  - when: when_would_leave_battle_area
    active_when:
      all_of:
        - opponents_turn: true
        - replacement_subject_is_mine: true
        - trait_has: "Royal Knight"
    optional: true
    once_per_turn: true
    process:
      - select_hand:
          bind_as: omekamon
          filter: { name_is: "Omekamon" }
      - play_from_hand_free: { of: you, hand_index: omekamon }
  ```
- Gap kind: closed for the BT20-091 shape. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave bt20_091_decline_would_leave_response_proceeds_without_playing_omekamon bt20_091_no_omekamon_in_hand_does_not_offer_response`.
- Workaround: no workaround needed for BT20-091; use the documented non-outcome replacement form.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT20-091).

---

## Royal Knights — attack target retarget response  [G-ATTACK-RETARGET]

- Effect text: `BT19-072` LordKnightmon: "[Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, you may switch the attack target to 1 of your Digimon with the [Royal Knight] trait."
- Status (2026-05-08): resolved for the BT19-072 card-shaped route. Production YAML uses `when: on_opponent_attack`, optional `select_own_permanent` filtered to Royal Knight Digimon, and `redirect_attack_target`. The combat flow emits the interrupt-time pending selection and mutates the active attack target through `ctx.redirect_attack`.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt19_072_opponents_turn_switches_attack_target_to_royal_knight`; shared verb coverage `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target`.
- Previous missing DSL verb / step kind / predicate: attack-state pending selection that can replace the current defender/security target with a selected own permanent matching a filter.
- Companion engine state: attack declaration and blocker/Raid-like retargeting are action-state concerns; a normal triggered effect after attack declaration cannot faithfully mutate the target without a dedicated interrupt point.
- Supported DSL syntax:
  ```yaml
  - when: on_opponent_attack
    optional: true
    once_per_turn: true
    process:
      - select_own_permanent:
          bind_as: new_target
          filter: { kind: digimon, trait_has: "Royal Knight" }
      - redirect_attack_target: { new_target: new_target }
  ```
- Gap kind: engine and DSL, closed for current script-facing retarget effects.
- Workaround: None needed for current script-facing retarget effects.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT19-072).

## ~~BT17-102 — dynamic name alias from digivolution-source stack  [G-DYNAMIC-NAME-ALIAS-FROM-STACK]~~ — RESOLVED 2026-05-22

- Effect text: BT17-102 Greymon "[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards."
- Status: RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. `identity.source_name_aliases` compiles a source-derived effective-name overlay, the engine synthesized identity includes those names, and name predicates consult the synthesized set.
- Evidence: `cargo test -p digimon-engine --test cards_behavioral bt17_102 -- --nocapture` passes with `bt17_102_all_turns_aliases_low_level_material_names` enabled.
- Companion engine gap: resolved in [docs/RUST_ENGINE_GAPS.md](../docs/RUST_ENGINE_GAPS.md) (`G-DYNAMIC-NAME-ALIAS-FROM-STACK`).
- Gap kind: hybrid, closed.
- Workaround: none needed.
- First reported: 2026-05-20 (`complete-dna-omnimon-archetype` closure — BT17-102 Greymon).

## ~~BT23-096 — `<Delay>`-on-attack-event clause  [G-DSL-DELAY-ON-ATTACK-EVENT]~~ — RESOLVED 2026-05-22

- Effect text: BT23-096 Comet Hammer — `<Delay>` body gated on an ally-attack event.
- Status: RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. `lower_delay.rs` maps attack timings to `DelayTrigger::OnEvent`, attack dispatch fans into event-gated delayed options with attacker context, and `attacker_trait_has` can evaluate ordinary attack context.
- Evidence: `cargo test -p digimon-engine --test cards_behavioral bt23_096 -- --nocapture` passes with the CS attack Delay and non-CS negative tests enabled.
- Already-present substrate: `G-DSL-ON-ALLY-ATTACK-TIMING` and `G-ATK-TRAIT-FILTER` remain noted as pre-existing halves; this change closed the missing delay/attack-event dispatch wiring.
- Companion engine gap: resolved in [docs/RUST_ENGINE_GAPS.md](../docs/RUST_ENGINE_GAPS.md) (`G-DSL-DELAY-ON-ATTACK-EVENT`).
- Gap kind: hybrid, closed.
- Workaround: none needed.
- First reported: 2026-05-20 (`complete-dna-omnimon-archetype` closure — BT23-096 Comet Hammer).

## Zephagamon — prompted attack target retarget to another Digimon or player  [ZEPH-G005]

- Status (2026-05-08): resolved for the ST18-14 Shoto Kazama card-shaped route. `redirect_attack_target` now supports a prompted form with `targets: any | player | digimon`, `optional`, and `prompt` fields when no fixed `new_target`/`player` is supplied. The prompt reuses attack-target action IDs, excludes the current target, can include the defending player, and exposes PASS when optional.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target_prompt_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st18_14`.
- Supported DSL syntax:
  ```yaml
  - redirect_attack_target:
      targets: any
      optional: true
      prompt: "Change the attack target to another Digimon or the player"
  ```

## Zephagamon — result-bound predicates and suspended-count formulas  [ZEPH-G002/ZEPH-G003/ZEPH-G005]

- Status (2026-05-10): narrowed. DSL predicate `binding_owner: { binding, of }` still covers the BT24-047 owner branch. Track J additionally added per-effect result-log predicates (`effect_suspended_any_own_digimon`, `effect_returned_any_card`, and sibling delete/play/digivolve/add-to-hand leaves) plus `suspended_count: { of: ... }` as a formula per-selector usable by formula-backed selection counts and thresholds. Production Zephagamon YAML still needs to be expanded for EX11-074 / BT20-101 / EX11-035 card-shaped coverage.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl suspended_count -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- binding_owner_predicate_matches_bound_permanent_controller`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_047`.
- Supported DSL syntax:
  ```yaml
  - if:
      condition:
        binding_owner: { binding: suspended, of: you }
      then:
        - may_attack_now: { attacker: suspended, targets: any, optional: true }
  - if:
      condition: { effect_suspended_any_own_digimon: true }
      then:
        - add_modifier: ...

  max:
    formula:
      base: 0
      per:
        suspended_count: { of: any }
      delta: 1
  ```
- Remaining adjacent card-authoring work: migrate the Zephagamon bodies that need these primitives and add card-shaped fixtures. If a printed card needs to distinguish a failed/protected mutation in a way the append-only result log cannot express, file that as a narrower `bind_result_as` payload gap.

## Track H §1 — Aura `security_attack: i32` flat slot (2026-05-10) — RESOLVED

The DSL `kind: aura` body now accepts a typed `security_attack: i32` field
alongside the pre-existing dynamic `security_attack_fn`. It lowers to a
`ModifierType::SecurityAttackChange` modifier carrying the literal delta
on each match, read at the security-resolution consult site
(`combat.rs:2326`). Negative deltas flow through unchanged; the combat
clamp at `combat.rs:2347` (max 0) governs the floor.

```yaml
# all your Olympos XII Digimon get <Security A. +1>
effects:
  - kind: aura
    target: { owner: you, trait: "Olympos XII" }
    security_attack: 1
```

Self, filter, and cross-side variants all land through the same path —
authors do not need to drop into raw_rust or formula DSL for flat ±N
grants. The dynamic `security_attack_fn` slot remains for cards whose
delta depends on board state.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_self_grants_flat_security_attack_plus_one aura_filter_grants_flat_security_attack_to_all_olympos_xii_digimon aura_filter_grants_flat_security_attack_minus_one_via_negative_delta`

## Track H §4 — Aura `while_condition` install-once continuous gate (2026-05-10) — PARTIAL

The DSL `kind: aura` body now accepts a `while_condition: <predicate>`
field that lowers to `Expiry::UntilCondition` on the installed
modifier. The UntilCondition controller (PR #458) handles eviction;
per the printed-semantics rule, `false → true` does NOT re-install.

```yaml
# this Digimon gains <Vortex>-can-attack-player while opponent has no
# unsuspended Digimon (canonical ZEPH-G004 fixture; uses
# memory_gte: 0 in v1 because VortexCanAttackPlayer's consult site is
# itself a separate gap)
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition:
      count_lte:
        n: 0
        filter:
          owner: opponent
          kind: digimon
          is_unsuspended: true
```

Distinct from `active_when` (per-tick re-evaluation, symmetric).
`while_condition` installs ONCE at OnPlay or OnDigivolve, the
controller evicts on predicate-false, and the install does NOT
re-fire. DCGO reference: `Vortex.cs:PermanentHasVortexCanAttackPlayers`
implements the lazy-filter pattern via `CanUse(null)` at attack-target
time; the Rust path achieves identical end behavior via
mutation-event-driven eviction.

**v1 supports**: self-aura with `dp_modifier`, `security_attack`, or
named `modifier` grants. Combine freely; all install with
`Expiry::UntilCondition` carrying the same compiled predicate.

**v1 does NOT support yet**:
- Filter-aura + `while_condition` — install-once would miss future
  permanents joining the filter set. Needs the lazy-filter shape
  from spec §2 (consult-time filter evaluation rather than
  install-time enumeration).
- Keyword-grant + `while_condition` — `KeywordEntry` lacks an
  `until_condition` field; the keyword registry needs the same
  extension `ModifierEntry` already has.
- Player-scoped (`target_player`) + `while_condition` — same
  install-once vs. lazy-filter design choice.

New raw_rust API:
- `EffectContext::add_modifier_with_until_condition(target, modifier, value, predicate_arc)`
  — typed wrapper that honors the `can_affect_permanent` guard, used by
  both lower_aura's while_condition path and any raw_rust card script
  that needs to install a controller-evicted modifier directly.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- while_condition`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat until_condition_controller`

## Track H §5 — Security-zone-sourced auras (2026-05-10) — PARTIAL

The DSL `kind: aura, scope: security` clause now lowers correctly. The
engine's `tick_declarative_effects` iterates face-up cards in each
player's security stack (gated on `player.face_up_security`); the
existing filter-aura process closure runs with `source_permanent =
None` and installs DP / keyword / security-attack / named-modifier
grants on field-side matches.

```yaml
# BT21-095-style: while this Option is face-up in security, all your
# [WG] Digimon gain Vortex.
card: BT21-095
name: Wind Guardians
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
```

End behavior matches DCGO `BT21_095.cs:CanUseCondition →
IsExistInSecurity(card, false)`:
- Face-down security sources do NOT fire.
- Source leaving security evicts the grant on next tick (no explicit
  OnLoseSecurity wiring needed — the materialized-declarative
  clear+re-install pattern handles it).
- New field entries pick up the grant on next tick (lazy-filter end
  behavior via the existing per-tick scan).
- Owner-scoped target filters work (your-side vs. opponent-side
  matches).

Outstanding: tensor/mask paths that pre-compute aura state from
sources directly (rather than reading modifier registry) still need a
`SecuritySource` enumeration. For raw_rust card scripts that need to
read their own security-zone position, the `EffectContext` source
discriminator is still `source_permanent: Option<PermanentHandle>` +
`source_card: CardHandle`; promoting to a typed `SecuritySource
{ player, security_index, card_index }` is a follow-up.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- security_zone_aura`

## Track H §3 — Granted triggered ability (2026-05-10) — PARTIAL

The engine primitive landed for the canonical OnDeletion case (DCGO
`AddSkillClass.cs` analog). Raw_rust card scripts can now grant a
closure-bodied triggered effect to a target permanent:

```rust
// Inside an effect's process closure, with `ctx: &mut EffectContext`:
ctx.grant_triggered_effect(
    carrier_handle,
    EffectTiming::OnDeletion,
    Expiry::Permanent,           // or EndOfTurn / EndOfYourTurn / etc.
    move |inner| {
        // Body fires when carrier is deleted, with:
        //   inner.source_card       == grantor card  (DCGO EffectSourceCard)
        //   inner.source_permanent  == carrier       (DCGO EffectSourcePermanent)
        //   inner.player            == grantor's controller
        inner.gain_memory(2);
    },
);
```

End behavior pinned by tests:
- Grantor installs grant on carrier; pre-deletion the body has not
  fired; deleting the carrier fires the body with carrier+source
  attribution preserved.
- `clear_permanent` evicts on carrier-leave (covers paths that bypass
  OnDeletion such as return-to-hand).
- `expire_end_of_turn` evicts time-bound grants per the same
  `source_player`-keyed rules as ModifierEntry.

DSL surface: not yet wired. A future `kind: grant_triggered` clause
would lower to this engine primitive. For now, granted triggered
abilities require raw_rust authoring.

Limitations of v1:
- **Timing coverage**: dispatch hook calls
  `fire_granted_triggered_effects(handle, timing)` only at the two
  OnDeletion firing sites. Other timings (OnAttack, OnSuspend, OnPlay,
  OnEnterFieldAnyone, etc.) install fine but never fire — extend each
  timing's canonical firing site as it comes online.
- **No selection support**: bodies fire inline, before the standard
  drain. A body that calls `ctx.install_pending_selection(...)` won't
  compose correctly with the surrounding firing sequence. For
  selection-driving granted bodies, the proper path is `QueuedEffect`
  with a `granted_effect_id` discriminator + lookup in
  `run_queued_effect_inner`. That's a follow-up.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_triggered_effect`

## Track H Phase 4 — Multi-timing dispatch, EX1-068, BT21-095, cross-track integration (2026-05-10)

### Phase 4a — `Expiry::EndOfOpponentsNextTurn` / `EndOfYourNextTurn` DSL keys

DSL string keys `end_of_opponents_next_turn` / `end_of_your_next_turn`
round-trip through `expiry_map.rs` to the new engine variants. v1
aliases the removal predicates to `EndOfOpponentsTurn` /
`EndOfYourTurn` semantics (correct for installs on source's own turn —
the common case for `[Main]` / `WhenDigivolving`). Mid-opp-turn install
nuance ("skip current opp-turn-end, expire on next") is a separate
follow-up requiring per-entry `pending_skips: u8` counter.

```yaml
- add_modifier:
    target: opponent
    modifier: ChangeDp
    value: -2000
    expiry: end_of_opponents_next_turn
```

### Phase 4b — Multi-timing dispatch for granted triggered abilities

`Game::pending_granted_fires` field accumulates carrier+timing pairs
discovered during `enqueue_from_permanent` /
`enqueue_from_breeding_permanent`; `drain_effect_queue` flushes them
inline AFTER its main loop drains. ALL `EffectTiming` variants are
covered automatically — no per-timing call-site additions needed.
Order: printed observers first, granted bodies second (matches DCGO's
"appended to effect list" semantic).

EX1-068 Ice Wall! ("All of your opponent's Digimon gain `[When
Attacking] lose 2 memory` until the end of their next turn") is wired
end-to-end as a raw_rust behavioral fixture — exercises:
- `EffectTiming::WhenAttacking` granted dispatch
- `Expiry::EndOfOpponentsNextTurn` carrying through expire_end_of_turn
- Per-carrier installation with multi-target enumeration
- Post-expiry attacks correctly do NOT fire the granted body

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- ex1_068`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_triggered_effect_fires_at_when_attacking`

### Phase 4c — Inherited filter aura (§6)

Filter auras with `scope: inherited` correctly emit when the source is
a card under a digivolution stack (not the top card). Verified by
`group6_auras::inherited_filter_aura_emits_grants_from_under_stack_source`
— a [Beast]-trait DigiEgg-style under-stack source publishes a
"+1000 DP to all your [Beast] Digimon" filter aura; the field
permanents matching the filter receive the grant including the
stack-top itself.

### Phase 4d — Cross-track integration (Track H × Track C)

`predicate.rs::eval_permanent_fields` now consults the synth-identity
overlay's traits union when evaluating `trait_has`. Without this
fix, a Track C `ChangeTraits` overlay (e.g., a Tamer treated as
[Holy] for the turn) was invisible to Track H aura filters. Pinned
by `aura_filter_includes_track_c_change_traits_overlay`. Other Track C
overlays (`ChangeBaseCardName`, `ChangeBaseCardColor`) follow the same
pattern but aren't yet propagated through the corresponding predicate
fields (`name_*`, `color_*`); separate follow-up.

### Phase 4f — EX1-068 Ice Wall! end-to-end raw_rust fixture

DCGO reference: EX1-068 grants `[When Attacking] lose 2 memory` to
all opp Digimon "until the end of their next turn." The Rust fixture
in `group6_auras::ex1_068_ice_wall_grants_when_attacking_loses_2_memory_to_all_opp_digimon`
walks opp's battle area at the source's [Main] effect time and calls
`ctx.grant_triggered_effect(opp_h, EffectTiming::WhenAttacking,
Expiry::EndOfOpponentsNextTurn, |inner| inner.gain_memory(-2))`.

DSL `kind: grant_triggered` clause (which would let EX1-068 land as
pure YAML) is a separate Phase 4e gap. Today the card requires
raw_rust authoring.

### Phase 4g — BT21-095 Wind Guardians real card YAML

`code/digimon-engine/cards/bt21/BT21-095.yaml` lands the [Security]
[All Turns] aura half via `kind: aura, scope: security` +
`grant_keyword: { keyword: Vortex }`. Behavioral fixture in
`code/digimon-engine/tests/cards_behavioral/bt21/bt21_095.rs`
covers: face-up grants, face-down does NOT grant, leave-security
evicts on next tick, owner-scope filter excludes opp [WG] Digimon.
Other clauses (IgnoreColorRequirement, [Main] replace-bottom-security,
[Security] play-WG-from-hand) are tracked under separate gap entries.

### Phase 4h — KeywordEntry `until_condition` extension

`KeywordEntry` gains `until_condition: Option<UntilConditionFn>` and
shares the globally-monotone `next_install_order` counter with
`ModifierEntry` / `PlayerModifierEntry`. The UntilCondition controller
now walks all three stores. New API:
`EffectContext::grant_keyword_with_until_condition(target, keyword,
predicate_arc)`. The DSL `while_condition` aura slot now lowers
keyword grants through this path:

```yaml
# ZEPH-G004-style: this Digimon gains <Vortex> while opponent has no
# unsuspended Digimon (memory_gte: 0 used as stand-in until
# VortexCanAttackPlayer's own consult site lands).
effects:
  - kind: aura
    grant_keyword: { keyword: Vortex }
    while_condition:
      count_lte:
        n: 0
        filter: { owner: opponent, kind: digimon, is_unsuspended: true }
```

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- while_condition_keyword_grant_lands_via_keyword_entry_until_condition`

### Phase 4e — DSL `grant_triggered_effect` step

The new step `grant_triggered_effect` lets card authors install a
granted triggered ability through pure YAML — no raw_rust required.

```yaml
# EX1-068 Ice Wall! authored as pure DSL.
effects:
  - when: main_from_hand
    optional: false
    process:
      - grant_triggered_effect:
          target: { owner: opponent, kind: digimon }
          timing: when_attacking
          expiry: end_of_opponents_next_turn
          body:
            - gain_memory: -2
```

Walks battle areas for `target` matches at the step's resolution
time and installs a granted-triggered-effect entry on each. The
body is a step list (anything `run_steps` can execute). Carrier vs.
source attribution flows through automatically — when the body
fires, `EffectContext::source_card` is the grantor and
`source_permanent` is the carrier (DCGO `EffectSourceCard` /
`EffectSourcePermanent`).

`timing:` accepts snake_case names: `on_play`, `on_digivolve`,
`when_digivolving`, `when_attacking`, `on_attack`, `end_of_attack`,
`end_of_battle`, `on_deletion`, `on_any_deletion`, `on_enter_field`,
`on_enter_field_anyone`, `on_suspend`, `on_unsuspend`,
`start_of_your_turn`, `start_of_opponents_turn`,
`start_of_your_main_phase`, `end_of_your_turn`,
`end_of_opponents_turn`, `on_ally_played`, `on_ally_attack`,
`on_opponent_attack`, `on_attack_target_change`. Unknown names
no-op silently with a debug-build warning.

`expiry:` uses the standard expiry-map keys (Phase 4a added
`end_of_opponents_next_turn` / `end_of_your_next_turn`).

v1 limitations:
- Bodies are non-selection (run inline after the printed-observer
  drain). Selection-driving bodies still require raw_rust until the
  `QueuedEffect.granted_effect_id` plumbing lands.
- The walk is at install-time; permanents that join the filter set
  AFTER the step resolves don't receive the grant. For
  install-once-then-leave-frozen semantics this is correct (matches
  EX1-068's printed text "all of your opponent's Digimon" snapshots
  current state).

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- dsl_grant_triggered_effect_step`

### Phase 4 cross-track integration (§10)

Three focused fixtures pin Track H's compatibility with adjacent
tracks at the consult-site level:

- **Track B (replacement) × H** — an aura granting `CannotBeDestroyed`
  via the `modifier:` slot installs a passive replacement modifier
  visible to Track B's deletion replacement window. Test:
  `aura_grant_cannot_be_destroyed_modifier_reaches_track_b_replacement_framework`.
- **Track D (combat) × H** — a self-aura granting `Piercing`
  surfaces through `Game::has_keyword` so Track D's combat
  security-check pipeline applies the Piercing follow-up. Test:
  `aura_grant_piercing_keyword_propagates_through_combat_consult`.
- **Track G (keyword payloads) × H** — a self-aura granting
  `Decoy(color)` preserves the parametric color discriminator
  through the registry so opponent's attack-target resolution
  filters correctly. Test:
  `aura_grant_decoy_keyword_includes_color_filter_payload`.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_grant`

### Phase 4i — Queue-based granted-body dispatch + selection support

`QueuedEffect.granted_effect_id: Option<u64>` discriminates granted
entries from printed-effect entries. `Game::granted_effect_bodies`
holds the closure bodies indexed by id. Granted entries flow through
the standard queue/drain pipeline so:
- Selection-installing bodies park correctly on `pending_selection`;
  the queue holds the entry alive while the selection resolves.
- The standard FIFO ordering (turn-player-bundle-first → trigger-order
  prompt for multi-trigger bundles) applies uniformly to granted and
  printed entries inside the same timing.
- The drainer skips the standard condition/pay_cost/max_per_turn
  gates for granted entries (they're closure-bodied with no Effect
  metadata).

Replaces the inline-fire `pending_granted_fires` flush (Phase 4b) that
worked only for non-selection bodies.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_body_runs_via_queue_with_correct_attribution`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_body_installing_selection_parks_via_pending_selection`

### Phase 4k — Typed `AuraScope` / `AuraGrant` builder API (raw Rust)

New `code/digimon-engine/src/aura.rs` module ships a typed fluent
builder for raw_rust card scripts that author auras programmatically.
YAML-authored cards continue to use the field-slot DSL (`kind: aura`
body) — that path is unchanged.

```rust
use digimon_engine::aura::{AuraScope, AuraGrant};
use digimon_engine::effect::Effect;
use digimon_engine::enums::{Expiry, Keyword};

Effect::declarative(card)
    .name("All your Holy Digimon gain +1000 DP")
    .aura()
        .scope(AuraScope::Player(controller))
        .target_filter(|rctx, h| {
            // ... per-permanent filter predicate
            true
        })
        .grants(AuraGrant::Dp { value: 1000, base: false, origin: false })
        .duration(Expiry::EndOfYourTurn)
    .build()
```

`AuraScope` variants: `Permanent(handle)`, `Player(player_id)`,
`OpponentPlayer(source_player)`, `Bilateral`, `SecurityZone(player_id)`.

`AuraGrant` variants: `Keyword(Keyword)`, `Dp { value, base, origin }`,
`SecurityAttack(i32)`, `PlayCost(i32)`, `DigivolutionCost(i32)`,
`LinkCost(i32)`, `Immunity(ImmunityKind)`, `Cannot(CannotKind)`,
`Modifier { ty, value }` (escape hatch for any ModifierType).

`ImmunityKind`: `OpponentDpReduction`, `OpponentDeDigivolve`,
`BattleDeletion`, `EffectDeletion`.

`CannotKind`: `Suspend`, `Unsuspend`, `Block`, `Attack`,
`AttackPlayer`, `ReturnToHand`, `ReturnToDeck`, `DeDigivolve`.

The builder routes through the existing typed install APIs:
- `AuraGrant::Keyword` → `grant_declarative_keyword` (or
  `grant_keyword_with_until_condition` when `while_condition` is set)
- All other grants → `add_declarative_modifier` (or
  `add_modifier_with_until_condition` when `while_condition` is set)

End behavior identical to direct API calls; pinned by tests
confirming the same modifier-registry state for both paths.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- typed_aura_builder`

### Phase 4 — `pending_skips` for `*NextTurn` expiry mid-opp-turn install

`ModifierEntry.pending_skips: u8` enables accurate
`EndOfOpponentsNextTurn`/`EndOfYourNextTurn` semantics for the rare
mid-opp-turn install case. Default 0 preserves source-turn-install
alias to `EndOfOpponentsTurn`. Set to 1 via
`.with_pending_skips(1)` when installing during the same player's
turn whose end would otherwise immediately expire the entry — the
current firing decrements (instead of expires), the next firing
expires. Matches printed text "until end of their NEXT turn" exactly
for all install timings.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- end_of_opponents_next_turn_with_pending_skips`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- end_of_opponents_next_turn_without_pending_skips`

### G-DSL-MODIFIER-PENDING-SKIPS — DSL `add_modifier` step cannot set `pending_skips` — RESOLVED 2026-05-21

- **Discovered in:** Puppets `/batch-implement-cards-rust-dsl` completion run, EX4-074 ShineGreymon: Ruin Mode (2026-05-21).
- **Card(s):** EX4-074 — `[When Digivolving][On Deletion] Until the end of your opponent's next turn, all of your opponent's Digimon get -5000 DP.`
- **Was missing:** the DSL `add_modifier` / `add_dp_modifier` step had no way to set `ModifierEntry.pending_skips`; its lowering routed through `ModifierEntry::simple` (hard-coded `pending_skips: 0`), so a DSL-installed `expiry: end_of_opponents_next_turn` modifier aliased to `EndOfOpponentsTurn` semantics and expired one turn early when installed mid-opponent-turn.
- **Resolution:** rather than a DSL field (the correct `pending_skips` is runtime turn-state, not authoring-time data), the engine now auto-computes it. `modifiers::pending_skips_for_install(expiry, source_player, turn_player)` returns `1` exactly for the `*NextTurn` install that would otherwise expire one turn early; `EffectContext::add_modifier` calls it and threads the result through `ModifierEntry::with_pending_skips`. Every `add_modifier` / `add_dp_modifier` caller (DSL and hand-written) now gets faithful "until end of next turn" semantics for free — no new DSL vocab needed.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers::tests::pending_skips`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_074` (6 passed, 0 ignored — the 3 formerly-deferred −5000 DP tests now run).

### G-ZONE-SELECTED-TRASH-TO-DECK-TOP — selected trash card → deck top — RESOLVED 2026-05-21

- **Discovered in:** Puppets `/batch-implement-cards-rust-dsl` completion run, LM-029 Yellow Scramble (shared by the LM-027 / LM-030 Scramble Delay clauses).
- **Card(s):** LM-029 — `[Start of Your Turn] <Delay> ... Return 1 yellow Digimon card from your trash to the top of the deck.`
- **Was missing:** no DSL verb moved a *selected* trash card to the *top* of the deck. The only trash→deck verbs (`return_all_trash_to_deck_bottom`, `return_trash_list_to_deck_bottom`) are bottom-only.
- **Resolution:** added the `return_trash_list_to_deck_top` DSL verb (exact mirror of `return_trash_list_to_deck_bottom` — `StepSpec::ReturnTrashListToDeckTop` / `CompiledStep::ReturnTrashListToDeckTop`, fields `of` + `cards`) lowering to the new `EffectContext::return_trash_cards_to_deck_top` engine method (mirror of `return_trash_cards_to_deck_bottom` but `deck.push` to the `Vec` end = deck top = drawn first).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm_029` (16 passed, 0 ignored — LM-029's `[Start of Your Turn]` Delay clause fully implemented).
- **Note:** LM-027 / LM-030 Scramble Delay clauses share this gap and are now unblockable (not implemented here — out of Puppets scope).

### Phase 4l — Track C overlay propagation (full set)

`predicate.rs::eval_permanent_fields` now consults the synth-identity
overlay union for ALL overlayable card-level fields:
- `trait_has` ← `synth_identity.traits` (covers Track C `ChangeTraits`)
- `name_is`, `name_contains`, `name_in` ← `synth_identity.card_name`
  (covers Track C `ChangeBaseCardName`)
- `color_is`, `color_only` ← `synth_identity.colors` (covers Track C
  `ChangeBaseCardColor`)

Previously Track C overlays were invisible to Track H aura filters
unless the predicate tested only `kind` (which already routed through
synth_identity). Now the full identity overlay union propagates,
matching DCGO's `Permanent.HasTrait` / `Permanent.GetCardName` /
`Permanent.GetColors` behavior — which all consult the live overlay
state.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_filter_includes_track_c_change_traits_overlay`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_filter_includes_track_c_change_base_card_name_overlay`

---

## ~~BT8-094 / RB1-035 — event-target level predicates~~  [G-EVENT-TARGET-LEVEL-LTE] — RESOLVED 2026-05-23

- **Status:** RESOLVED 2026-05-23 (`complete-rocks-archetype` task 10.1). `event_target_level_eq`, `event_target_level_lte`, and `event_target_level_gte` now flow through `PredicateSpec` -> `CompiledPredicate` -> compiler -> runtime evaluator. `BT8-094` Clauses A and B are authored and verified. `MovedFromBreeding` observer dispatch now scans both players' battle areas so opponent-side Tamers can faithfully observe the moved event target.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_094 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_082 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_130 --nocapture`.

Historical note:

- **Effect text (BT8-094 Clause A):** "[All Turns] When one of your opponent's
  level 5 or lower Digimon is deleted, you may suspend this Tamer to ＜Draw 1＞."
- **Effect text (BT8-094 Clause B):** "[Opponent's Turn] When one of your
  opponent's level 3 Digimon is moved from their breeding area to their battle
  area, gain 2 memory."
- **Effect text (RB1-035 Clause 2):** "[All Turns] When an opponent plays a
  Digimon, by suspending this Tamer, gain 1 memory if that Digimon is level 4
  or higher, and Draw 1 if it is level 3."
- **Missing DSL predicate:** The `PredicateSpec` struct in
  `code/digimon-dsl/src/predicate.rs` has no `event_target_level_lte`,
  `event_target_level_eq`, or `event_target_level_gte` leaf. The sibling
  `event_target_kind` / `event_target_trait_has` / `event_target_owner` /
  `event_target_color_any_of` leaves all exist but none expose the integer
  level of the event-target permanent's top card.
- **Why it can't be approximated:** omitting the level filter would make the
  deletion observer fire on Lv.6+ Digimon too, and the breeding-move observer
  fire on Lv.4+ Digimon — both violate the no-approximations policy.
- **Lowers to engine API:** `EffectReadContext::event_target_card()` already
  returns a `Card` snapshot; `Card::level` is available on that struct. Adding
  an `event_target_level_lte: Option<u8>` predicate leaf (and `_gte`, `_eq`
  siblings) is a small addition alongside the existing `event_target_kind` arm
  in `predicate.rs::eval_predicate_with_bindings` (`group6_event_target_*`
  block, ~line 900).
- **Suggested DSL syntax:**
  ```yaml
  condition:
    all_of:
      - event_target_owner: opponent
      - event_target_kind: digimon
      - event_target_level_lte: 5       # or event_target_level_eq / _gte
  ```
- **Gap kind:** DSL-only (engine event context already carries the level; the
  only missing piece is the predicate leaf in `predicate.rs` + `compiled.rs` +
  `compile.rs` + the evaluator arm).
- **Blocked cards / tests:**
  - `code/digimon-engine/cards/bt8/BT8-094.yaml` — Clauses A and B omitted;
    YAML comments document the intended shapes.
  - `code/digimon-engine/tests/cards_behavioral/bt8/bt8_094.rs` — 9 tests
    `#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE ..."]`.
  - `code/digimon-engine/cards/rb1/RB1-035.yaml` — [All Turns] clause noted
    as needing "event-card level predicates" in comment.
- **First reported:** 2026-05-22 (BT8-094 Pass 2 audit).

## ~~DSL Gap: LM-027 — Move a selected trash card to deck TOP~~  [G-ZONE-SELECTED-TRASH-TO-DECK-TOP] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `EffectContext::return_trash_cards_to_deck_top` + a `destination: top | bottom` field on the `return_trash_list_to_deck_bottom` step move a selected trash card to the deck top. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, LM-027 Red Scramble DSL implementation (2026-05-21). Also pre-noted as MED-GAP-01 in `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md` and `qa/archetype-qa/dsl/bg_imperial.md`.
- **Scope:** DSL + engine (hybrid). Cross-referenced in `qa/archetype-qa/engine-gaps.md` under the same gap ID.
- **Card(s):** LM-027 Red Scramble — `[Start of Your Turn] <Delay>` clause "Return 1 red Digimon card from your trash to the **top** of the deck." Also LM-029, LM-030, LM-031 (sibling Scramble cards with the same Delay body).
- **Effect text:** "Return 1 red Digimon card from your trash to the top of the deck."
- **What's missing:** No DSL verb / `EffectContext` method moves a *selected* trash card to the **top** of the owner's deck. Verified: `EffectContext::return_all_trash_to_deck_bottom` and `return_trash_cards_to_deck_bottom` (`effect_context/mod.rs`) both hard-code `deck.insert(0, card)` (deck bottom). DSL `step/zone_moves.rs` exposes only `ReturnAllTrashToDeckBottom` / `ReturnTrashListToDeckBottom` — bottom-only. `ReturnToDeckFromReveal` accepts a `position` but operates on the reveal pool, not the trash. Routing to deck bottom would be an unfaithful approximation (top vs bottom changes what is drawn) — forbidden by the no-approximations policy. The deck-**bottom** sibling gap `G-ZONE-TRASH-TO-DECK` and the timing gap `G-DELAY-START-OF-TURN` are both RESOLVED; this deck-TOP variant is genuinely new and distinct.
- **Suggested change:** Add a `destination: top | bottom` parameter to a generalized `return_bound_cards_to_deck` DSL step (or a dedicated `return_selected_trash_to_deck_top` verb) plus a matching `EffectContext::return_trash_cards_to_deck_top` engine method (mirror `return_trash_cards_to_deck_bottom` but `deck.push`).
- **Workaround:** LM-027 clause B retains a `raw_rust` no-op placeholder; 4 tests `#[ignore = "...G-ZONE-SELECTED-TRASH-TO-DECK-TOP"]`. Clauses A and C ship faithfully in pure DSL.

## ~~DSL Gap: BT21-072 — `save_in_text` predicate for alt-path `from:` filters~~  [G-ALT-PATH-SAVE-IN-TEXT] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). **No new predicate needed** — the existing `effect_text_contains` predicate already scans a candidate's printed text and `eval_predicate` already evaluates it against an alt-path `from:` candidate. BT21-072's cost-3 path uses `from: { any_of: [{level_eq:4, effect_text_contains:"＜Save＞"}, {level_eq:4, trait_has:Hero}] }`. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, BT21-072 Arresterdramon: Superior Mode (2026-05-21).
- **Scope:** DSL.
- **Card(s):** BT21-072 — `xros_req: "[Digivolve] Lv.4 w/＜Save＞ in text or w/[Hero] trait: Cost 3"`. Any card whose alt-digivolution requirement gates on the source permanent having ＜Save＞ printed in its effect text.
- **Effect text:** "[Digivolve] Lv.4 w/＜Save＞ in text or w/[Hero] trait: Cost 3"
- **What's missing:** Alt-path `from:` filters support `level_eq`, `trait_has`, `name_contains`, etc., but there is no `save_in_text: true` predicate to match a source permanent whose top card has the ＜Save＞ keyword in its printed effect text. The "w/[Hero] trait" half is expressible (`trait_has: Hero`); the "w/＜Save＞ in text" half is not — so the cost-3 alt-path cannot be faithfully expressed as a whole (it is a single OR-condition path).
- **Suggested change:** Add a `save_in_text: bool` (or a generalized `keyword_in_text: <keyword>`) predicate leaf usable in alt-path `from:` filters, evaluated against the source card's printed effect text / parsed keyword set.
- **Workaround:** None faithful. BT21-072's `alt_paths` ships the standard cost-4 path only; the cost-3 ＜Save＞/Hero path is omitted. BT21-072 is PARTIAL.

## ~~DSL Gap: BT21-093 — declinable `activation_cost: { trash_self: true }`~~  [G-ACTIVATION-COST-TRASH-SELF] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `activation_cost: { trash_self: true }` added (`CompiledActivationCostKind::TrashSelf` → `EffectContext::trash_self_as_cost`); declinable per Comprehensive Rules 16-16-2. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, BT21-093 Raging Serpentine (2026-05-21).
- **Scope:** DSL.
- **Card(s):** BT21-093 Raging Serpentine — `[All Turns] ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)`. Any ＜Delay＞ card whose activation cost is "by trashing this card".
- **Effect text:** "＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)"
- **What's missing:** `activation_cost:` accepts only `suspend_self` and `return_self_to_deck_bottom` (per `compile.rs`). There is no `trash_self: true` variant. Per Comprehensive Rules 16-16-2, ＜Delay＞ processing is OPTIONAL — the controller may decline to trash the card and activate the effect. Without a declinable `trash_self` activation cost, the trash-self must be modeled as the first mandatory body step, which forces the activation once the trigger fires and a valid target exists — suppressing a rules-mandated player choice (a no-approximations violation).
- **Suggested change:** Add `activation_cost: { trash_self: true }` — a declinable activation cost that trashes the source card and gates the body; declining skips the whole Delay.
- **Workaround:** BT21-093 models the trash as a mandatory first body step. PARTIAL.

## DSL Gap: BT25-066 — trash a permanent's own LINK card as a would-leave replacement cost  [G-DSL-LINK-TRASH-AS-REPLACEMENT-COST]
- **Status:** CLOSED (2026-06-07). `cost: { trash_own_link_card: true }` on a `when_would_leave_battle_area` replacement (gap-3a, commit 297a00ab) lowers to `CompiledStep::TrashOwnLinkCardAndCancelLeave`; the preflight gates the optional accept on `replacement_subject.linked_cards.len() >= 1` and surfaces the WHICH-link-card choice even for a single card. **link-finish-replacement slice (2026-06-07)** extends it to the **`scope: linked`** case (an Option's link-card ESS, BT25-101 Divine Arms Version Ω): `lower_replacement.rs` and `lower_aura.rs` now emit `.linked()` for `CompiledScope::Linked`; `replacement.rs::collect_candidates` scans each permanent's `linked_cards` for `.linked()` would-* replacement effects; `source_permanent_is_still_active` accepts the `linked_cards` zone. BT25-066 IMPLEMENTED (`cards/bt25/BT25-066.yaml`, 8/8); BT25-101 inherited leave-replacement IMPLEMENTED (7/7). Verify: `cargo test --test cards_behavioral -- bt25_066 bt25_101`; `cargo test --test option_flow` (126/126).
- **Discovered in:** BT25 "machine" slice, BT25-066 Guardromon (batch-implement-cards-rust-dsl, 2026-06-05).
- **Scope:** DSL.
- **Card(s):** BT25-066 Guardromon — `[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave.` Generalizes to any "by trashing 1 of its link cards, it doesn't leave / prevent" replacement.
- **Effect text:** "[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- **What's missing:** No DSL verb selects and trashes one of a permanent's **own link cards** as the cost of a `kind: replacement` clause. The `ReplacementCostBody` only supports `delay_self: true`; `ReplacementChooseBody.from` only supports `hand`. There is no `select_linked_card` / `trash_linked_card` step, and no `from: linked_cards` for the replacement `choose:`. The engine DOES model the substrate: `Permanent.linked_cards`, `EffectTiming::OnLinkedCardTrashed` (fired in `combat.rs`), and `cancel_replacement` all exist — only the DSL vocabulary to pick + trash a self link card (and gate the cancel on that cost being paid, optionally per DCGO `SetIsSkippable(true)`) is missing. Without it the link-trash player choice cannot surface (no-approximations violation), so the whole card is BLOCKED even though its other clauses (Blocker, inherited +1000 DP, TS-trait alt-digivolve) are individually expressible.
- **Lowers to engine API:** a new `EffectContext` selection over `source_permanent.linked_cards` + trash of the chosen link card (with `OnLinkedCardTrashed` dispatch, already wired) + `cancel_replacement`. The cancel/replacement plumbing already exists.
- **Suggested DSL syntax:**
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true            # DCGO SetIsSkippable(true): the controller may decline
    active_when: { replacement_subject_is_mine: true }
    choose:
      from: linked_cards       # NEW from-zone: this permanent's own link cards
      min: 1
      max: 1
    outcome: prevent           # trashing the chosen link card cancels the leave
  ```
  (Alternatively a dedicated `trash_linked_card_and_cancel_replacement: { of_subject: true }` step usable inside the replacement `process:`.)
- **Workaround:** None faithful. BT25-066 ships no YAML; BLOCKED (dsl).

## DSL Gap: BT25-074 — play a revealed card with the play cost REDUCED by N (not free)  [G-DSL-PLAY-FROM-REVEALED-COST-REDUCED]
- **Status:** CLOSED (2026-06-05). `play_from_revealed_free` now accepts an optional `cost_delta: { reduce: N }` (default free, preserving prior behavior). Engine adds `EffectContext::play_from_revealed_with_cost(player, card, CostDelta)`; `play_from_revealed_free` delegates with `Free`. DSL: `PlayFromRevealedFreeArgs.cost_delta: Option<CostDelta>` → `CompiledStep::PlayFromRevealedFree.cost_delta` (lowered via `compile_cost_delta`) → handler routes through `play_from_revealed_with_cost` (None ⇒ Free, NOT `lower_cost_delta`'s Reduce(0)). Test: `tests/dsl/phase2f1_play_steps.rs::play_from_revealed_with_cost_delta_reduce_pays_remainder` (cost 3 − reduce 2 ⇒ pays 1, no over-credit). BT25-074 is unblocked on this gap.
- **Discovered in:** BT25 "machine" slice, BT25-074 Tankdramon (batch-implement-cards-rust-dsl, 2026-06-05).
- **Scope:** DSL.
- **Card(s):** BT25-074 Tankdramon — `[When Digivolving] [When Attacking] [Once Per Turn] Reveal the top 3 cards of your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait Digimon card among them with the cost reduced by 5. Trash the rest.` Generalizes to any "reveal N, play 1 among them with cost reduced by X" (X > 0, the controller pays the remainder).
- **Effect text:** "Reveal the top 3 cards of your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait Digimon card among them with the cost reduced by 5. Trash the rest."
- **What's missing:** The reveal-pool play steps only support a FREE play. `play_from_revealed_free` hard-codes `crate::enums::CostDelta::Free` (`effect_context/mod.rs:3547`); `choose_from_reveal`'s `play_free` destination is likewise free-only. There is no reveal-pool play step that takes a `cost_delta` to pay a non-zero reduced cost. (The hand analog `play_from_hand` DOES carry `cost_delta: Option<CostDelta>`, and BT15-096 plays from hand with cost reduced by 3 — so the gap is reveal-pool-specific.)
- **Lowers to engine API:** already-present primitive — `play_from_revealed_free` internally calls `Game::play_from_hand_with_cost_result_from_origin(... CostDelta ..., PendingWouldPlayOrigin::Reveal { .. })`, which accepts any `CostDelta`. Only the DSL step pins it to `Free`. `enums::CostDelta::Reduce(i16)` exists.
- **Suggested DSL syntax:** add an optional `cost_delta: CostDelta` (default `Free`) to `play_from_revealed_free` (or a new `play_from_revealed: { of, card, cost_delta }` step), threaded into the existing `from_origin` call instead of the hard-coded `Free`:
  ```yaml
  - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
  - select_reveal_buckets:
      from: revealed
      buckets:
        - bind_as: play_pick
          min: 0
          max: 1
          filter:
            all_of:
              - kind: digimon
              - any_of: [ { trait_has: D-Brigade }, { trait_has: ACCEL } ]
              - play_cost_lte: 12
  - play_from_revealed_free: { of: you, card: play_pick, cost_delta: { reduce: 5 } }
  - per_selected: { selection: revealed, bind_as: rest, body: [ { trash_from_reveal: { of: you, card: rest } } ] }
  ```
- **Workaround:** None faithful (playing for free over-credits the player by 5+ memory; a no-approximations violation). BT25-074 ships no YAML; BLOCKED (dsl). The card's secondary clauses ([All Turns][OPT] on_ally_played → opponent CannotDigivolve, and the inherited [Opponent's Turn] Reboot+Blocker) are individually expressible but cannot ship without the main WD/WA clause.

---

## G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER — multi-card / multi-Tamer face-down trash cost (BT25 BEATBREAK)

- **Status:** ✅ CLOSED (2026-06-15). New DSL verb
  `trash_bottom_face_down_sources_under_tamers: { of, count }` (step.rs
  `TrashBottomFaceDownSourcesUnderTamersArgs` → `CompiledStep` →
  `dsl_cards/step/selections.rs::install_trash_n_bottom_face_down_sources_under_tamers`).
  It trashes `count` bottom face-down sources total, distributed across the
  controller's Tamers — "N from one Tamer" or "1 from each of N Tamers" — by
  installing one single-Tamer bottom-trash `select_own_permanent` pick per
  source, re-evaluating eligibility each time, so every Tamer pick surfaces as a
  real `PendingSelection` (no auto-resolve, DCGO `CanEndSelectCondition`
  reachable). Unpayable (fewer than `count` total face-down sources) ⇒
  `cost_unpayable` + clause abort, like the single-trash verb. Paired with a new
  no-subject predicate `face_down_sources_under_tamers_gte: <N>` that gates the
  optional digivolve on the cost being payable. Driver BT25-035 ships
  IMPLEMENTED (`cards/bt25/BT25-035.yaml`, `tests/cards_behavioral/bt25/bt25_035.rs` 12/12).
- **Cards:** BT25-035 Cougarmon (`[On Play][When Digivolving] ... by trashing 2 bottom face-down cards from under any of your Tamers, this Digimon may digivolve into a [Glowing Dawn] Digimon for free`). Likely also BT25-019 / other BEATBREAK "trash N" cost cards.
- **What's missing:** The shipping verb `trash_bottom_face_down_source_under_tamer: { of }` trashes exactly **one** bottom face-down source from **one** chosen Tamer (it installs a single `select_own_permanent` over `{ kind: tamer, has_face_down_source: true }` and trashes that Tamer's bottom face-down card, then runs the tail). It has no `count:` parameter and no support for distributing the cost across multiple Tamers (DCGO BT25_035: `maxCount: 2`, `canEndNotMax: true`, `CanEndSelectCondition = (picked==2) || (picked==1 && that Tamer has >=2 face-down sources)` — i.e. "trash 2 total: either 2 from one Tamer, or 1 from each of two Tamers"). Chaining the single-trash verb twice does NOT work: each invocation installs a selection and runs the *captured tail* on resolution, so two sequential invocations cannot share one continuation cleanly, and the "2 from one Tamer OR 1+1 from two Tamers" choice is not expressible.
- **Lowers to engine API:** the engine already has the per-Tamer bottom-face-down trash primitive (`install_trash_bottom_face_down_source_under_tamer` in `dsl_cards/step/selections.rs`, and DCGO mirrors it with `TrashDigivolutionCardsFromTopOrBottom(trashCount: N, isFromTop: false, CanTrashCard)`). The missing piece is a DSL step that drives an N-total multi-pick over Tamers with the DCGO end-condition.
- **Suggested DSL syntax:** a `count:`-carrying variant, e.g.
  ```yaml
  - trash_bottom_face_down_sources_under_tamers: { of: you, count: 2 }
  ```
  with semantics: pick Tamers (each must carry >=1 face-down source) until `count` face-down sources are trashed total; a single Tamer with >=`count` face-down sources may satisfy it alone (DCGO `CanEndSelectCondition`). The whole step is the activation cost: if fewer than `count` face-down sources exist across all Tamers, the cost is unpayable → abort the clause (`TailAlreadyRan`), matching the single-card verb's unpayable behavior.
- **Workaround:** None faithful for BT25-035. The single-trash verb under-charges (trashes 1 instead of 2) — a no-approximations violation. BT25-035 BLOCKED (dsl) on this gap. (Its [On Play][When Digivolving] -3000 DP rider IS expressible; the free-digivolve-by-2-trash cost is the blocked part.) **[RESOLVED — see Status above.]**

---

## G-DSL-PLACE-REVEALED-CARD-UNDER-TAMER — place a revealed card face-down under a chosen Tamer (BEATBREAK reveal-pool stash)

- **Status:** ✅ CLOSED (2026-06-15). The existing `place_selected_card_under_tamer` DSL step now resolves a **reveal-pool**-bound card (in addition to hand / trash / union-zone): its `ResolvedBinding::Card` / singleton-`CardList` arm scans `Game::revealed_cards` and calls the new `EffectContext::place_revealed_card_under_tamer` (engine helper that places a `CardSourceRef::Reveal` card as the bottom-most, optionally face-down, source of a chosen own Tamer). Driver ST23-06 Gekkomon ships IMPLEMENTED (`cards/st23/ST23-06.yaml`, `tests/cards_behavioral/st23/st23_06.rs` 7/7).
- **Cards:** ST23-06 Gekkomon (`[When Moving][On Play] Reveal the top 3 cards of your deck. Among them, add 1 [Glowing Dawn] card to the hand AND place 1 [Glowing Dawn] card face down under any of your [Glowing Dawn] trait Tamers. Return the rest to the bottom of the deck`). Likely also ST24 / other BEATBREAK reveal-and-stash cards.
- **What was missing:** `place_selected_card_under_tamer` resolved only hand / trash / union-zone card bindings; a `select_reveal_buckets` / `select_reveal` pick (which still lives in the transient reveal pool, stored as a one-element `CardList`) fell through to the `_ => None` arm, so the second revealed card was never tucked under the Tamer (it leaked into the deck-bottom remainder).
- **Lowers to engine API:** the placement substrate (`place_as_bottom_source` honoring `CardSourceRef::Reveal` + `face_down`) already existed; the gap was a DSL-lowering reveal-pool branch + a thin `place_revealed_card_under_tamer` helper.
- **Note:** when no [Glowing Dawn] Tamer exists, only the add-to-hand bucket runs (DCGO `HasMatchConditionOwnersPermanent` gate) — modelled with an `if any_permanent { tamer + Glowing Dawn }` / `else` two-path reveal.

---

## BT25 "titan" slice — BLOCKED cards (2026-06-06)

Implemented in this slice: BT25-006, BT25-068, BT25-071, BT25-019 (all IMPLEMENTED).
The four cards below are BLOCKED; each is cross-referenced to the controlling gap.

### BT25-069 Raremon — `[On Play][When Digivolving] link 1 [TS] card from your trash to 1 of your Digimon for free`  [gap_kind: dsl]
- **What's missing:** A DSL step that **selects a card from the trash and links it** to a chosen own Digimon. The shipping `link_to_own_digimon` verb is hardwired to link the *carrier Option card* (it reads `pending_option` / installs a `LinkSelectHost` over the carrier — see `dsl_cards/step/mod.rs::try_run_link_step`). It cannot select an arbitrary trash card as the link card. DCGO `BT25_069.cs` uses `SelectCardEffect Root.Trash` → `selectedPermanent.AddLinkCard(cardForLinking)`.
- **Lowers to engine API:** the engine already has the host/link substrate (`Permanent.linked_cards`, link-host selection) — the missing piece is a DSL verb to pick a trash card + pick a host Digimon + attach. Belongs to the broader **`[Link]` subsystem** gap in `docs/RUST_ENGINE_GAPS.md` (item 9: "alternate-source linking from trash").
- **Suggested DSL syntax:** `link_card_from_trash_to_own_digimon: { of: you, free: true, card_filter: { trait_has: TS }, host_filter: { kind: digimon } }`.
- **Other clauses** (Jamming, inherited +1000 DP, TS-trait alt-digivolve) are individually expressible; the card ships no YAML because its sole active clause is blocked.

### BT25-073 Dragomon — trash a link card as an ACTIVATION cost  [gap_kind: dsl]  [G-DSL-LINK-TRASH-AS-COST]
- **Status:** OPEN (re-adjudicated 2026-06-07, link-finish-replacement slice). gap_kind narrowed `hybrid → dsl`: the inherited leave-replacement is now expressible (see G-DSL-LINK-TRASH-AS-REPLACEMENT-COST, CLOSED) and Jamming is declarative, but the **Main clause activation cost is still BLOCKED** on the DSL — so the card ships no YAML.
- **Main clause** `[On Play][When Digivolving] By trashing 1 of your Digimon's link cards, you may play or use 1 [TS] cost<=5 card from hand free`: needs a step that **selects an own Digimon (with >=1 link card), selects one of ITS link cards, and trashes it as an ACTIVATION cost**, then runs a gated play/use of the chosen hand card. DCGO `BT25_073.cs`: `SelectPermanentEffect` (own Digimon, `!HasNoLinkCards`) → `SelectCardEffect Root.LinkedCards` (maxCount 1) → `TrashLinkCardsAndProcessAccordingToResult` → `successProcess` plays/uses the hand card free.
- **What's missing:** the only link-card-trash vocabulary, `cost: { trash_own_link_card: true }`, is **replacement-only** — it reads the `replacement_subject` binding, cancels the leave, and is valid solely on a `when_would_leave_battle_area` replacement (`outcome: prevent`). There is no **general activation-cost** step that picks an arbitrary own Digimon + one of its link cards + trashes it (then continues a tail). The `link_cards` step family only *attaches* cards; none trashes a permanent's link card.
- **Lowers to engine API:** substrate exists — `Permanent.linked_cards`, `Game::trash_specific_link_card(host, card)` (added in gap-3a, fires `OnLinkedCardTrashed`), and the standard play/use-free flow. The missing piece is a DSL cost step, e.g. `trash_link_card_of_own_digimon: { of: you }` that installs the own-Digimon select (filter `has_link_cards`) → link-card select → `trash_specific_link_card`, exposed as an activation cost whose success gates the tail (the play/use). Unpayable (no own Digimon has a link card) ⇒ abort the clause.
- **Suggested DSL syntax:**
  ```yaml
  - when: [on_play, when_digivolving]
    process:
      - trash_link_card_of_own_digimon: { of: you }   # NEW cost step
      - select_hand:
          of: you
          bind_as: play_pick
          optional: true
          filter: { all_of: [ { trait_has: TS }, { play_or_use_cost_lte: 5 } ] }
      - play_or_use_from_hand: { of: you, card: play_pick, cost: free }
  ```
- **Other clauses** (Jamming; inherited leave-replacement) are expressible, but the Main clause is the defining active clause, so BT25-073 ships no YAML. BLOCKED (dsl).

### BT25-083 LadyDevimon — bottom-source-from-hand/trash + trash-digivolution-option-as-cost + cost-reduced trash-option use  [gap_kind: hybrid]
- **Clause 1** `[On Play][When Digivolving] By placing 1 [Three Musketeers] card from your hand OR trash as any of your Digimon's bottom digivolution cards, <Draw 1>`: needs a **zone-choice (hand|trash) picker that places the chosen card as a bottom digivolution source** of a selected Digimon. `place_as_bottom_source` exists for reveal/deck-sourced cards but there is no hand-or-trash-sourced bottom-placement verb with the DCGO 3-way `SetIntSelection` (Hand / Trash / Don't place).
- **Clause 2** `[When Digivolving][When Attacking][OPT] By trashing 1 Option card from any of your Digimon's digivolution cards, you may use 1 [Three Musketeers] Option from your trash with cost reduced by 3`: needs (a) a step to **select+trash an Option from a permanent's digivolution stack as a cost**, and (b) **use a trash Option with a play/use cost reduction** (`UseOptionFromTrash` with `cost: { reduce: 3 }`). The DSL has `use_option_from_hand` but no trash-rooted reduced-cost option-use, and no "trash an option from digivolution cards" cost step.
- **Inherited OnDeletion** "play a level 4 or lower [Three Musketeers]-text Digimon from trash free" IS expressible; the card ships no YAML because the two active clauses are blocked.

### BT25-091 Monica Simmons — `[Your Turn] When you use [TS] Option cards` trigger  [G-DSL-ON-USE-OPTION-TIMING]  [gap_kind: dsl]
- **Card(s):** BT25-091 Monica Simmons (clause 3). Generalizes to any "[Your Turn] When you use … Option cards, …" trigger.
- **Effect text:** "[Your Turn] When you use [TS] trait Option cards, by suspending this Tamer, 1 of your opponent's Digimon can't attack until their turn ends."
- **What's missing:** No DSL `when:` token lowers the engine timing `EffectTiming::OnUseOption` (defined in `code/digimon-engine/src/enums.rs:318`, fired by the engine when a player uses an Option). `digimon-dsl/src/compile.rs` has no `on_use_option` / `when_you_use_option` arm, so the clause cannot be authored. DCGO `BT25_091.cs` uses `EffectTiming.OnUseOption` + `CanTriggerWhenOwnerUseOption(OptionTrigger)`.
- **Lowers to engine API:** the engine timing + dispatch already exist; only the DSL needs a `when: on_use_option` token (optionally with an `option_filter:` predicate for the "[TS] trait Option" gate). The clause body (`activation_cost: { suspend_self: true }` + `select_opponent_permanent` + `add_modifier: CannotAttack` `expiry: end_of_opponents_turn`) is otherwise fully expressible.
- **Suggested DSL syntax:** `when: on_use_option` with optional `option_filter: { trait_has: TS }`.
- **Other clauses** (start-of-turn set-memory-to-3, On Play return-or-draw, [Security] play-self) are implemented and tested (`bt25_091.rs`); BT25-091 ships PARTIAL with this one clause deferred.

### BT25-092 Asuna Shiroki — `[Main]` digivolve into a card from {hand|trash} with a {hand|digivolution-card} trash cost  [G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST]  [gap_kind: dsl]
- **Card(s):** BT25-092 Asuna Shiroki (clause 2).
- **Effect text:** "[Main] By suspending this Tamer and trashing 1 Option card from your hand or your Digimon's digivolution cards, 1 of your Digimon may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand or trash with the cost reduced by 1."
- **What's missing:** two distinct union-zone gaps in one clause:
  1. The digivolve **result** is chosen from a union of **{hand, trash}**, but `effect_initiated_digivolve`'s `from_hand` binding does not accept a `select_union_zone` (hand-or-trash) result, and digivolve-into-a-card-resident-in-trash is unverified in the `EffectInitiatedDigivolve` lowering (`dsl_cards/step/play_digivolve.rs`).
  2. The **cost** trash is from a union of **{your hand, your Digimon's digivolution cards}** — `select_union_zone` does not span a *permanent's own digivolution sources*, and there is no single verb to trash an Option from "hand OR digivolution cards" as a cost.
- **Lowers to engine API:** the substrate pieces exist individually (`select_union_zone`, `select_own_sources`, `trash_selected_sources`, `effect_initiated_digivolve` with `cost: { reduce: 1 }`, `activation_cost: { suspend_self: true }`); the missing piece is (a) digivolve `from_source` accepting a union/trash-bound result, and (b) a cost-trash union spanning hand + a chosen permanent's digivolution stack. Authoring a hand-only-result / hand-only-cost reduction would silently drop player choices (no-approximations), so the clause is left BLOCKED rather than approximated.
- **Suggested DSL syntax:** allow `effect_initiated_digivolve: { source: <select_union_zone binding over {hand, trash}>, cost: { reduce: 1 } }` plus a cost step like `trash_selected_from_union: { zones: [hand, digivolution_cards], filter: { kind: option } }`.
- **Other clauses** (start-of-main trash-to-draw+memory, [Security] play-self) are implemented and tested (`bt25_092.rs`); BT25-092 ships PARTIAL with clause 2 deferred.

### BT25-101 Divine Arms Version Ω — link a [TS] card from trash + inherited link-ESS clauses  [gap_kind: hybrid]
- **Status:** RESOLVED → **IMPLEMENTED** (link-finish-replacement slice, 2026-06-07). `cards/bt25/BT25-101.yaml`, `tests/cards_behavioral/bt25/bt25_101.rs` (7/7).
- **Card(s):** BT25-101 Divine Arms Version Ω.
- **Main clause** `[Main] By trashing 1 [TS] card from hand, <Draw 2>; After, you may link this card OR 1 [TS] card from your trash to 1 of your Digimon without paying the cost`: now expressed with `link_cards: { from: [self_option, trash], filter: { trait_has: TS }, to: own_digimon, count: { up_to: 1 }, cost: free }` (gap-2 `link_cards` step + gap-3b `self_option` from-zone). The "link THIS card" branch attaches the Option to itself; the "1 [TS] from trash" branch links a chosen trash card; a zone-choice surfaces when both qualify. Trash-hand-cost gates Draw 2 + the link via `select_hand { cost: true }` → `if binding_present` → `draw` → `link_cards`.
- **Inherited link-ESS clauses** (`<Security A. +1>`, `<Reboot>`, leave-replacement): authored as **`scope: linked`** (the working link-ESS convention) and now reach the host. Required engine widening this slice: `lower_aura.rs` + `lower_replacement.rs` emit `.linked()` for `CompiledScope::Linked`; `replacement.rs::collect_candidates` scans `linked_cards` for `.linked()` would-* replacement effects; `source_permanent_is_still_active` accepts the `linked_cards` zone. (The keyword/DP/formula ESS host-reach via `tick_declarative_effects` / `live_declarative_formula_sum` linked passes was already closed — see `docs/RUST_ENGINE_GAPS.md` G-LINK-INHERITED-ESS.)
- **Verdict:** IMPLEMENTED. The DCGO `EqualsCardName("Vulcanusmon")` host gate is an implementation detail not in the printed text; the link requirement is modeled as a generic Digimon-host link (cost 3) per printed text.

---

## BT25 "orphan-d" slice — BLOCKED cards (2026-06-06)

Implemented in this slice: BT25-055 Deramon, BT25-042 ClavisAngemon (both IMPLEMENTED).
The five cards below are BLOCKED; each is cross-referenced to the controlling gap.

> **Re-adjudicated 2026-06-07 ("link-appmon-2" slice — BT25-070/056/072/060).** Re-ran
> these (plus BT25-070 Logamon, same family) against current `main`. Update: the
> **`when: when_linked` DSL token HAS since landed** (`Timing::WhenLinked` →
> `CompiledTiming::WhenLinked`, `clause.rs:146` / `compile.rs:258`; G-DSL-WHEN-LINKED-TIMING
> closed), so the secondary "no when-token" blocker noted below is stale. The **controlling
> blocker is unchanged and still open**: there is no engine primitive — and no DSL verb
> lowering to one — to link a *chosen* card from `{hand | trash | digivolution-cards}` onto
> a host Digimon (the shipping `link_to_own_digimon` links only the carrier Option; the
> 2026-06-06 Shape-B substrate only *absorbs a standing permanent*, root `None`). This is
> facet **#9** of the `docs/RUST_ENGINE_GAPS.md` `[Link]` keyword subsystem (DCGO
> `ILinkCard(cardSource, host)` / `Permanent.AddLinkCard(cardSource)` with `SelectCardEffect.Root`
> = `Hand`/`Trash`/`DigivolutionCards`). All four remain **BLOCKED (hybrid)**; ship no YAML
> under the no-approximations policy (the link clause is each card's central mechanic and
> gates the `WhenLinked`/`[When Linking]` payloads). DCGO confirmed this run:
> `BT25_070.cs:181`, `BT25_056.cs:196`, `BT25_072.cs:201`, `BT25_060.cs:160`.

### BT25-056 Bootmon — host-Digimon link-from-{hand|digivolution-cards} + `When this gets linked` trigger  [gap_kind: hybrid]
- **Effect text:** "[On Play][When Digivolving][When Attacking] If it's your turn, you may link 1 [Social], [Tool] or [Game] trait Digimon card from your hand or this Digimon's digivolution cards to this Digimon with the cost reduced by 2. [All Turns] When this Digimon gets linked, suspend 1 of your opponent's Digimon or Tamers." Plus `<Barrier>` (expressible) and inherited "Return 1 of your opponent's suspended Digimon to the bottom of the deck" (expressible).
- **What's missing (two facets):**
  1. **Host-Digimon link from a chosen card in hand / digivolution-cards** (with a `-2` link-cost reduction). The shipping `link_to_own_digimon` links only the *carrier Option* (reads `pending_option` / installs `LinkSelectHost` over the carrier). No DSL verb selects a Digimon card from hand or this permanent's digivolution stack and attaches it as a link card to the carrier Digimon. DCGO `BT25_056.cs`: `AddSelfLinkConditionStaticEffect` + 3-way `SetIntSelection` (Hand / DigivolutionCards / Don't) → `ILinkCard(cardSource, card.PermanentOfThisCard())` + `GrantedReduceLinkCostClass`. Same family as `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem (facet #6) and the documented `link_card_from_trash_to_own_digimon` gap (BT25-069/101) — here zones are hand + digivolution-cards.
  2. **`When this Digimon gets linked` trigger.** Engine has `EffectTiming::WhenLinked` (`enums.rs:333`) but no DSL `when:` token lowers to it (`CompiledTiming` has no `WhenLinked`; `compile.rs` has no `when_linked`/`gets_linked` arm). Consumer side of `docs/RUST_ENGINE_GAPS.md` facet #11.
- **Lowers to engine API:** link substrate (`Permanent.linked_cards`, `attach_linked_card`, `ChangeLinkCost`) + `EffectTiming::WhenLinked` dispatch exist; missing are (a) a DSL host-link-from-source verb and (b) a `when: when_linked` token.
- **Verdict:** BLOCKED (hybrid). Ships no YAML. Cross-ref `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem + BT25-069/101.

### BT25-072 Shutmon — host-Digimon link-from-{trash|digivolution-cards} + `When this gets linked` deny-digivolve  [gap_kind: hybrid]
- **Effect text:** "[On Play][When Digivolving][When Attacking] If it's your turn, you may link 1 [Social], [Tool] or [Game] trait Digimon card from your trash or this Digimon's digivolution cards to this Digimon with the cost reduced by 2. [All Turns][Once Per Turn] When this Digimon gets linked, 1 of your opponent's Digimon or Tamers can't digivolve until their turn ends." Plus `<Jamming>` (expressible) and inherited "2 of your opponent's Digimon or Tamers can't unsuspend until their turn ends" (expressible).
- **What's missing:** identical two facets as BT25-056 — (1) host-Digimon link from a chosen card (here **trash** + digivolution-cards), exactly the documented `link_card_from_trash_to_own_digimon` gap (BT25-069/101) extended to also span the carrier's digivolution stack; (2) the `When this gets linked` (`WhenLinked`) DSL `when:` token. DCGO `BT25_072.cs`: `ILinkCard` from `SelectCardEffect.Root.Trash`/`DigivolutionCards` + `WhenLinked` ActivateClass installing `CannotDigivolve` until the opponent's turn ends.
- **Lowers to engine API:** as BT25-056; plus `ModifierType::CannotDigivolve` (exists) for the gets-linked body.
- **Verdict:** BLOCKED (hybrid). Cross-ref BT25-056, BT25-069/101, `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

### BT25-060 Rebootmon — host-Digimon free-link-from-{hand|digivolution-cards} + `When this gets linked OR unsuspends` self-buff  [gap_kind: hybrid]
- **Effect text:** "[When Digivolving][When Attacking][Once Per Turn] By linking 1 [Appmon] trait Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost, 1 of your Digimon may unsuspend. [All Turns][Once Per Turn] When this Digimon gets linked or unsuspends, until your turn ends, this Digimon gains <Piercing> and <Blocker>, and your opponent's Digimon effects don't affect it." Plus `<Security A. +1>`, `<Reboot>`, `<Link +1>` (declarative — expressible).
- **What's missing:** (1) host-Digimon **free link from hand / digivolution-cards** as an *activation cost* (the "by linking …, 1 may unsuspend" is gated on the link succeeding) — same missing host-link-from-source verb as BT25-056; (2) the **`When this gets linked OR unsuspends`** trigger — `OnUnsuspend` has a DSL token but **`WhenLinked` does not**, so one leg of the multi-timing trigger is unauthorable. DCGO `BT25_060.cs`: `AddLinkCard(cardSource)` as cost → unsuspend; `WhenLinked` + `OnUnTapped` ActivateClasses granting Piercing/Blocker + DigimonEffectImmunity.
- **Lowers to engine API:** link substrate + `EffectTiming::WhenLinked`/`OnUnsuspend` exist; `grant_keyword` Piercing/Blocker + `grant_effect_immunity` exist for the body. Missing: host-link-from-source verb (as activation cost) + `when: when_linked` token.
- **Verdict:** BLOCKED (hybrid). Cross-ref BT25-056, `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem (facets #6/#10/#11).

### BT25-085 BeelStarmon — use-Option-from-{hand|digivolution-cards} free + trash-Option-from-{digivolution|link}-cards-as-cost → unsuspend  [G-DSL-USE-OPTION-FROM-SOURCES + G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST]  [gap_kind: dsl]
- **Effect text:** "[When Digivolving][When Attacking][Once Per Turn] You may use 1 [Three Musketeers] or [TS] trait Option card from your hand or this Digimon's digivolution cards without paying the cost. [When Digivolving][When Attacking][Counter][Once Per Turn] By trashing 1 Option card from any of your Digimon's digivolution cards or link cards, this Digimon unsuspends." Plus `<Blocker>` (expressible). Inherited [Main]: "Delete 1 of your opponent's highest level Digimon. Then, you may place 1 card from your hand or trash as any of your Digimon's bottom digivolution card."
- **What's missing (two facets):**
  1. **Use an Option card from a permanent's digivolution stack** (not just hand). Shipping `use_option_from_hand` is hand-only (`UseOptionFromHandArgs` reads `select_hand`). DCGO `BT25_085.cs` offers a `SelectCardEffect.Root.DigivolutionCards` branch with `customRootCardList`.
  2. **Trash 1 Option from any of your Digimon's digivolution OR link cards as an activation cost** (then unsuspend self). DCGO uses `permanent.DigivolutionOrLinkCards` as the trash pool. No DSL step selects+trashes an Option from the union of a permanent's digivolution + link cards as a cost. Same family as BT25-083's "trash an Option from digivolution cards as cost", extended to also span link cards.
  (The inherited [Main] highest-level delete is expressible; the bottom-source-from-{hand|trash} place is the same gap as BT25-083 clause 1.)
- **Lowers to engine API:** the use-Option / source-trash primitives exist in principle; missing is DSL vocabulary to root a use/trash at a permanent's digivolution+link stack.
- **Verdict:** BLOCKED (dsl). Ships no YAML — both [WD][WA] active clauses depend on source-rooted Option verbs.

### BT25-076 Ghoulmon — `When this would be played, by deleting your own Digimon, reduce cost by the deleted Digimon's play cost`  [G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION]  [gap_kind: hybrid]
- **Effect text:** "When this card would be played, by deleting 1 of your play cost 11 or lower Digimon with [Negamon] in its digivolution cards and [Negamon] in its text, reduce the cost by the deleted Digimon's play cost." Plus `<Rush>`, `<Reboot>`, `<Blocker>` (declarative — expressible) and "[On Play][When Attacking][On Deletion] Delete 1 of your opponent's lowest-play-cost Digimon; if it didn't delete, trash your opponent's top security" (expressible: `select_opponent_permanent` over a lowest-play-cost gate → `delete_permanent`, else `trash_top_security`).
- **What's missing:** a **`BeforePayCost` cost reducer whose payment is a player-selected deletion of an OWN permanent and whose reduction amount is that permanent's play cost (variable)**. Shipping `BeforePayCost` reducers (`lower_cost_reduction.rs`) carry a passive `amount`/`amount_fn`/`raw_rust` value; none install an interactive `select_own_permanent` + `delete_permanent` *as the cost*, nor read the deleted permanent's `GetCostItself` as the reduction delta. DCGO `BT25_076.cs` `EffectTiming.BeforePayCost`: optional `SelectPermanent` (canNoSelect modulated by affordability) over own Negamon-text + Negamon-source cost<=11 Digimon → `DeletePeremanentAndProcessAccordingToResult` → register a `ChangeCostClass` of `-reducedCost`.
- **Why not approximated:** authoring an `amount_fn` (max-cost-Negamon) without the player-selectable deletion would (a) silently auto-pick which Negamon to delete and (b) silently delete it — two no-approximations violations; DCGO presents an explicit `canNoSelect` choice. The cost-reduction-by-player-deletion is this card's core play-enabler, so the card is BLOCKED rather than shipping a PARTIAL that drops the choice.
- **Lowers to engine API:** deletion, play-cost read, and cost-delta primitives exist individually; missing is a DSL `BeforePayCost` reduction clause that drives an interactive delete-own-as-cost with a variable, deleted-permanent-sourced reduction. Engine-side, the `BeforePayCost` dispatch would need to host an interactive selection at cost-calc time (currently passive) — hence `gap_kind: hybrid`.
- **Verdict:** BLOCKED (hybrid). Ships no YAML.

### BT25-061 Offmon — Appmon `<Link>` keyword on a Digimon + `[When Linking]` trigger  [gap_kind: dsl]
- **✅ RESOLVED 2026-06-07.** Both cited facets landed with DigiLink Shape-B (`G-DSL-DIGILINK`, 2026-06-06): facet 1 → `kind: link_condition` (Digimon self link-condition), facet 2 → `when: when_linked`. BT25-061 now ships `code/digimon-engine/cards/bt25/BT25-061.yaml` + 7 green tests (`tests/cards_behavioral/bt25/bt25_061.rs`); verdict IMPLEMENTED in `validated_cards_dsl.json`. (Original gap text retained below for history.)
- **Effect text:** "[Start of Your Main Phase] By trashing 1 card with the [Appmon] trait from your hand, <Draw 1> and gain 1 memory. [When Linking] 1 of your opponent's Digimon can't unsuspend until their turn ends." Plus the Appmon `<Link>` keyword (Offmon is itself a *Digimon* that gets linked to another [Appmon] host) and alt-digivolve Lv.2 [Appmon] / Cost 0 (alt-path is expressible). NOTE: cards.json labels the "can't unsuspend" line as the *inherited* effect, but DCGO `BT25_061.cs` implements it as a `WhenLinked` ActivateClass behind the card's Link keyword (`AddSelfLinkConditionStaticEffect` + `LinkEffect` + `WhenLinked`).
- **What's missing (two facets, both already-documented families):**
  1. **A Digimon declaring itself as a Link-keyword Digimon** — DCGO `AddSelfLinkConditionStaticEffect(permanentCondition: HasAppmonTraits, linkCost: 1)`. The shipping DSL `kind: link_requirement` is documented as "for Link **Options**" only (`LinkRequirementBody`), and every current consumer is `kind: option`. No vocabulary lets a `kind: digimon` card register itself as a host-attachable Link card with a host predicate + link cost.
  2. **The `[When Linking]` / `WhenLinked` trigger** — `EffectTiming::WhenLinked` exists in `enums.rs` but no DSL `when:` token lowers to it (`CompiledTiming` has no `WhenLinked`; `compile.rs` has no `when_linked`/`when_linking`/`gets_linked` arm). This is the SAME consumer-side gap already logged for BT25-056 / BT25-072 / BT25-060 — cross-ref `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem facet #11.
- **Why BLOCKED not PARTIAL:** the Start-of-Your-Main-Phase trash→draw+memory clause and the alt-digivolve ARE expressible, but the `<Link>` keyword and its `[When Linking]` payload are the card's defining Appmon mechanic; shipping them dropped would be a silent omission (no-approximations). Ships no YAML.
- **Lowers to engine API:** `WhenLinked` timing + link substrate exist; missing is (a) a Digimon-scoped self-link-condition DSL declarative and (b) a `when: when_linked` token. The "can't unsuspend until their turn ends" payload itself (`CannotUnsuspend` + `UntilOpponentTurnEnd` over a selected opponent Digimon) is expressible once the trigger exists.
- **Verdict:** BLOCKED (dsl). Cross-ref BT25-056 / BT25-072 / BT25-060 (`WhenLinked` token) and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

### BT25-086 Dan Yuki — DP modifier formula `× opponent's memory count`  [G-DSL-FORMULA-OPPONENT-MEMORY]  [gap_kind: dsl]
- **Effect text:** "[Start of Your Main Phase] If you have 4 or less memory, gain 1 memory. [End of Your Turn] By suspending this Tamer, 1 of your [TS] trait Digimon gains +1000 DP for the turn for each memory your opponent has. Then, it may attack. [Security] Play this card without paying the cost." (Cost 3 [TS] Tamer.)
- **What's missing:** a **formula source that reads a player's memory-gauge value as a scalar**, so `add_dp_modifier` can compute `+1000 × opponent_memory`. `FormulaSpec::BasePerDelta`'s `PerSelector` enum (`formula.rs`) covers `material_count` / `stack_size` / `ally_count` / `suspended_count` / color counts / `card_count_in_zone` — but there is **no `per: opponent_memory`** (nor `your_memory`) selector, and `CompoundFormula::Aggregate` only ranks permanents by DP/level/cost. The memory gauge is a single signed integer on `Game` (`game.memory` / `gain_memory_for_player`), not a zone count, so `card_count_in_zone` cannot express it either. DCGO `BT25_086.cs`: `dpGain = Math.Max(0, card.Owner.Enemy.MemoryForPlayer * 1000)`.
- **Why BLOCKED not PARTIAL:** the Start-of-Your-Main-Phase memory-floor gain (`condition: { memory_lte: 4 }` → `gain_memory: 1`) and the `[Security] play_from_security` clause ARE expressible; the suspend-self cost (`activation_cost: { suspend_self: true }`) and `may_attack_now` exist too. But the End-of-Turn clause's DP grant is *variable in the opponent's memory* — authoring it with a literal or any existing `per:` source would misstate the buff (no-approximations). The End-of-Turn clause is the card's whole payoff, so it ships no YAML rather than a PARTIAL that fakes the DP amount.
- **Lowers to engine API:** `ctx` can already read `game.memory` (and DCGO reads `Enemy.MemoryForPlayer`); the gap is purely a DSL formula vocabulary one — add a `PerSelector::PlayerMemory { of: PlayerRef }` (or a `FormulaSpec::PlayerMemory`) that the existing `formula_eval` evaluates against the gauge, then `add_dp_modifier: { value: { formula: { base: 0, per: { player_memory: { of: opponent } }, delta: 1000 } }, expiry: end_of_turn }`.
- **Verdict:** BLOCKED (dsl). Ships no YAML.

## G-DSL-WHEN-LINKED-TIMING — [When Linking] triggered clause has no `when:` timing (2026-06-06)

- **✅ RESOLVED 2026-06-07 (the DSL timing).** `when: when_linked` landed with DigiLink Shape-B (`G-DSL-DIGILINK`, 2026-06-06): `Timing::WhenLinked` → `CompiledTiming::WhenLinked` → `EffectTiming::OnLink` + forced `.linked()` + self-filter. The `[When Linking]` clause is now authorable (see BT25-007/BT25-061 which use it). **BT25-036 itself remains BLOCKED**, but on a *different* primitive: **App Fuse** (`AddAppfuseMethodByName`) is not implemented in the engine (no lowering, no handler; `AltPathKind::AppFusion` parses but resolves to nothing) — re-classified to `gap_kind: engine`, tracked in `docs/RUST_ENGINE_GAPS.md` App Fuse entry. (Original gap text retained below for history.)
- **Card:** BT25-036 Craftmon (orphan-b slice). DCGO `BT25_036.cs` region "When Linking" uses `EffectTiming.WhenLinked` + `SetIsLinkedEffect(true)` for "[When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>."
- **Gap:** the `digimon_dsl::clause::Timing` enum (the `when:` surface) has no `WhenLinked` / `Linked` variant. `compiled::CompiledTiming::Linked` and `compiled::CompiledScope::Linked` exist, and `lower_triggered.rs` already routes `CompiledScope::Linked` through `builder.linked()`, but there is no `Timing` string that lowers to `CompiledTiming::Linked`, so a "[When Linking]" triggered body cannot be authored.
- **Lowers to engine API:** the engine already fires a link-established event (DCGO `EffectTiming.WhenLinked`); the gap is purely DSL-side — add a `Timing::WhenLinked` variant (serde `when_linked`), map it `S::WhenLinked => CompiledTiming::Linked` in `compile.rs`, and confirm `engine_timing` lowering wires `CompiledTiming::Linked` to the engine's link-established timing.
- **Suggested DSL syntax:** `- when: when_linked` (optionally `scope: linked`) with a `process:` body (here: `select_hand` trash 1 Appmon → `draw 2`).
- **Verdict:** BLOCKED (dsl). BT25-036 ships no YAML — its App-fusion alt-path, link condition, [Security] play-self, and OnPlay add-top-security + <Recovery +1> are all expressible, but the [When Linking] clause is its mandatory inherited payoff and cannot be silently dropped under the no-approximations policy.

## BT25 "beatbreak" slice — BLOCKED / PARTIAL notes (2026-06-06)

Implemented this slice: BT25-081 (IMPLEMENTED); BT25-088, BT25-090, BT25-049,
BT25-035, BT25-041 (PARTIAL — each ships its expressible clauses with one
BLOCKED clause omitted). BT25-057 BLOCKED. Cross-references below; the
controlling engine gap for the cost-reduction clauses is
`G-COST-REDUCTION-INTERACTIVE-PAY-COST` in `docs/RUST_ENGINE_GAPS.md`.

### Glowing Dawn "trash a face-down card under a Tamer → reduce a card's cost" — BLOCKED (engine)
- **Cards:** BT25-088 Kyo Sawashiro (clause 3, play -1), BT25-090 Tomoro Tenma
  (clause 3, Option-use -1), BT25-049 Armalizamon (clause 2, Option-use -3),
  and the cost-reduced half of BT25-041's main clause.
- **What's missing:** `kind: cost_reduction` with a `pay_cost` that installs an
  interactive selection drops the reduction credit. The
  `trash_bottom_face_down_source_under_tamer` verb ALWAYS installs a Tamer-pick
  selection (no-approximations), so as a `pay_cost` it parks
  (`RunOutcome::Parked`), `apply_cost_reduction_candidate` returns `None`, and
  the `amount` is discarded while the face-down card is still trashed. Full
  root-cause + suggested engine fix in `docs/RUST_ENGINE_GAPS.md`
  `G-COST-REDUCTION-INTERACTIVE-PAY-COST`. (The same verb works fine as a
  *process activation cost* — see BT25-041's inherited unsuspend and BT25-057's
  De-Digivolve, which DO compile.)
- **Verdict:** the affected clause is OMITTED from each card's YAML (PARTIAL);
  authoring it would either drop the reduction (over-charge) or require
  auto-resolving the Tamer pick (no-approximations violation).

### BT25-035 Cougarmon — trash-2 free-digivolve — BLOCKED (dsl)
- The "Then, by trashing 2 bottom face-down cards … may digivolve into a
  [Glowing Dawn] card in hand for free" half is the existing
  `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER` gap (multi-count / multi-Tamer
  trash) plus an effect-driven free-digivolve-into-a-hand-card. Omitted; the
  -3000 DP, inherited Barrier, and Glowing Dawn alt-digivolve ship (PARTIAL).

### BT25-057 Monarchlizamon / "Final Judgment" — DUAL card — RESOLVED 2026-06-15  [G-DSL-DUAL-PER-FACE-EFFECTS + G-DSL-ARTS-DIGIVOLVE]

> **RESOLVED 2026-06-15 (`gap/dual-per-face-arts`).** Both gaps closed. The DUAL
> faces now carry their own `effects:` and the Option face an `arts_digivolve:`
> shorthand:
> - **Per-face effects sink (G-DSL-DUAL-PER-FACE-EFFECTS):** `DualDigimonSpec`
>   and `DualOptionSpec` gained an `effects: Vec<ClauseSpec>` field (`spec.rs`),
>   compiled onto `CompiledDualDigimon.effects` / `CompiledDualOption.effects`
>   (`compiled.rs` + `compile.rs`), validated per-face (`validator.rs`), and
>   lowered by `DslCardEffect::effects()` (`dsl_cards/mod.rs`): Digimon-face
>   clauses lower with the Digimon identity (natural timings), Option-face
>   clauses with the Dual identity so `when: main` → `EffectTiming::OptionMain`
>   and the `dual.option.use_requirement` color bypass applies. `clause_index`
>   is offset per face so multi-timing OPT keys never collide. Digimon-face
>   `grant_keyword` declaratives are also scanned into the top-card native
>   `keywords` (`card_data_from_compiled`) so static keywords (Security A.+1 /
>   Reboot / Blocker) are live on field.
> - **Arts Digivolve (G-DSL-ARTS-DIGIVOLVE):** `dual.option.arts_digivolve: true`
>   compiles into the `ArtsDigivolve` option-face keyword, which the existing
>   engine path (`pending_option_can_arts_digivolve` →
>   `install_arts_digivolve_selection`) reads — no engine change needed. The
>   Digimon-face evo table is backfilled from the alt-digivolve box
>   (`compiled_dual_to_engine` now threads the computed `evo_costs`) so the Arts
>   `can_digivolve` gate works for DSL-loaded dual cards.
> - **Cards shipped:** ST23-09 Atratusmon (IMPLEMENTED), BT25-057 Monarchlizamon
>   (IMPLEMENTED — cards.json mislabel corrected via the DUAL YAML), BT25-043
>   Habakirimon (upgraded PARTIAL → IMPLEMENTED, Option side now ships). Tests:
>   `tests/cards_behavioral/{st23/st23_09,bt25/bt25_057,bt25/bt25_043}.rs` (26
>   tests, all green).
> - **Residual (separate pre-existing limitation, NOT this gap):** the engine
>   `can_digivolve` / `can_basic_digivolve` gate is color+level only — a
>   trait-gated digivolution box ("Lv.N w/ [Glowing Dawn]: Cost C") is not
>   enforceable as a static `EvoCost` row. These cards author BOTH a color-form
>   alt-digivolve (the cards.json evo table — backfills the static evo_cost) and
>   the printed trait-form alt-path, matching every other DSL card's
>   digivolution authoring. A faithful trait-gated `can_digivolve` is a
>   standalone engine gap.

### Original entry (history)
- **Note:** `data/cards.json` mislabels this as a plain Digimon (`card_kind: 0`).
  The card IMAGE + DCGO `BT25_057.cs` confirm it is a **DUAL** card: a Lv.5
  Cyborg/Glowing Dawn/BEATBREAK Digimon face AND an Option face "Final
  Judgment" (Use 4).
- **What's missing (DSL):** `digimon-dsl`'s `DualSpec` (`spec.rs`) carries only
  metadata + text for each face (`DualDigimonSpec` / `DualOptionSpec` have NO
  `effects:` field). A dual card therefore cannot attach behavioral clauses to
  either face via the `dual:` block. (Whether a dual card's top-level `effects:`
  route correctly to its Digimon face is also unexercised — no shipping card
  uses `kind: dual` with effects, and the `tests/dual_cards/` suite drives only
  hand-written `CardData`.)
- **What's missing (engine/DSL — Arts Digivolve):** the Option face's "Arts
  Digivolve (instead of trashing after use, your cards may digivolve into this
  card without paying the cost)" has no DSL authoring path. `Keyword::ArtsDigivolve`
  exists and the engine has `install_arts_digivolve_selection` /
  `pending_option_can_arts_digivolve` (checked against `dual.option.keywords`),
  but `DualOptionSpec.keywords` (a `Vec<String>`) is the only hook and the full
  arts-digivolve authoring + behavioral path is untested from YAML.
- **Card faces (all individually expressible IF the dual-per-face-effects gap
  closes):** Digimon face — alt-digivolve Lv.4 Glowing Dawn cost 3; [WD][WA][OPT]
  trash a bottom face-down card under a Tamer (process cost, works) → De-Digivolve
  1 opp Digimon; [WD] this Digimon may battle 1 opp Digimon (`battle:`). Option
  face — Use Req Glowing Dawn (ignore color), [Main] 1 of your Digimon gains
  <Rush> + <Security A. +1> + 5000 DP for the turn, then may attack; Arts
  Digivolve.
- **Suggested DSL syntax:** add `effects:` (and optionally `inherited`/`security`
  scoping) to `DualDigimonSpec` and `DualOptionSpec`, lowering each face's
  clauses onto the appropriate face of the compiled dual card; add an
  `arts_digivolve: true` shorthand (or `keywords: [ArtsDigivolve]`) on the
  option face wired to the existing engine arts-digivolve selection.
- **Verdict:** BLOCKED (hybrid). BT25-057 ships no YAML — both faces' behavioral
  effects are unauthorable and shipping a stat-only dual card with a
  non-functional Option face would be an approximation.

---

## ~~G-DSL-SELECT-OPP-SOURCES-DYNAMIC-CROSS-PERMANENT~~ — RESOLVED 2026-06-13 — player-choice trash of a dynamic count of opponent digivolution source cards across all opponent Digimon
- **RESOLVED 2026-06-13 (G-DSL-SELECT-SOURCES-FORMULA-COUNT):** `SelectOpponentSourcesArgs` now carries `max: CountBound` (accepts a `FormulaSpec`, e.g. `{ source_material_count: {} }`), `clamp_to_available: bool`, and cross-permanent selection (omit `target`). The suggested YAML below is now expressible. BT25-103 needs **re-assessment** (verified stale 2026-06-14, fix-dsl-substrate-rot-and-bugs §6.3) — it may now be fully authorable, or blocked on a *different* clause; re-run the slice rather than trusting the stale BLOCKED verdict below.
- **Discovered by:** BT25-103 GraceNovamon (aegiomon-3 slice), 2026-06-06.
- **Clause:** "[When Attacking] [Counter] [Once Per Turn] For each of this Digimon's digivolution cards, you may trash any 1 digivolution card from your opponent's Digimon. Then, you may end this attack."
- **DCGO (BT25_103.cs):** `CardEffectCommons.SelectTrashDigivolutionCards(permanentCondition: IsEnemyDigimon, maxCount: card.PermanentOfThisCard().DigivolutionCards.Count, canNoTrash: true, isFromOnly1Permanent: false, ...)` — the player picks up to N digivolution source cards (N = this Digimon's digivolution-card count) from **any** opponent Digimon (not restricted to one permanent), each pick optional. Then a separate Yes/No "end the attack?" prompt.
- **What the DSL has:** `select_opponent_sources` (BindingRef-bound source picker) with `min`/`max` as **`u8` literals** and an optional `target` that **restricts the picker to ONE opponent permanent's** digivolution stack (see BT16-085). `trash_selected_sources` + `end_attack` steps both exist; `Counter` timing and `once_per_turn` exist.
- **Why blocked:** two missing capabilities on `select_opponent_sources`:
  1. **Dynamic count** — `max` must accept a `FormulaSpec` (here `{ source_material_count: {} }`, the same formula clause 6's bounce uses), not just a literal `u8`.
  2. **Cross-permanent selection** — when `target` is omitted, the picker must span **all** opponent Digimon's digivolution stacks (DCGO `isFromOnly1Permanent: false`), with each pick choosing both which Digimon and which source card.
- **Lowers to engine API:** the selection machinery for opponent digivolution sources already exists (single-permanent path); the gap is (a) threading a formula-resolved max and (b) a cross-permanent candidate set. Likely a `max_fn: Option<FormulaSpec>` + a `cross_permanent: bool` (or `target` omitted ⇒ cross-permanent) on `SelectOpponentSourcesArgs` and the corresponding engine candidate enumeration.
- **Suggested DSL syntax:**
  ```yaml
  - select_opponent_sources:
      max_fn: { source_material_count: {} }   # dynamic count = this Digimon's digivolution cards
      min: 0                                    # canNoTrash: true (each pick optional)
      cross_permanent: true                     # span all opponent Digimon (isFromOnly1Permanent: false)
      bind_as: trashed
      then:
        - trash_selected_sources: { source_refs: trashed }
  ```
- **Faithfulness impact:** stubbing this as `trash_top_n_digivolution_cards_of_each` (trashes the top-N of EACH opp Digimon with no player choice of which Digimon or which card) would be an auto-selection — a no-approximations violation. Card cannot ship until the dynamic-count cross-permanent source picker exists.
- **Verdict:** BT25-103 BLOCKED (gap_kind: dsl). No YAML shipped (clauses 1–6 are expressible but the whole card is gated on the Counter clause).

---

## G-DSL-BATTLE-WINNER-BOARDWIDE — gate a trigger on "when any of your [trait] Digimon win a battle"
- **Discovered by:** BT25-020 Marsmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[All Turns] [Once Per Turn] When any of your [TS] trait Digimon win a battle, trash your opponent's top security card."
- **DCGO (BT25_020.cs):** `EffectTiming.OnEndBattle` ActivateClass, OPT, gated by `CardEffectCommons.CanTriggerWhenWinBattle(winnerCondition: permanent => permanent.TopCard.Owner == card.Owner && permanent.TopCard.HasTSTraits)`. The trigger fires whenever ANY winner permanent on the controller's side has the [TS] trait — not just the carrier itself.
- **What the DSL has:** `source_deleted_battle_opponent: true` predicate — fires only when the **carrier** is the battle winner (the "this Digimon wins a battle" idiom, ST4-11; used by BT25-048/051/054). There is **no** board-wide "any ally with trait X won a battle" predicate, and `on_any_deletion`'s `event_target_*` predicates describe the **deleted** permanent (the loser), not the winner.
- **Why blocked:** the body (`trash_top_security: { of: opponent }`) and `once_per_turn` exist, but the trigger cannot be gated to "any of your [TS] Digimon win a battle". Shipping it on `on_any_deletion` ungated would fire on every deletion (including the opponent's wins) — an approximation. Narrowing to `source_deleted_battle_opponent` would silently drop the board-wide scope (only the carrier's own wins would count) — also an approximation.
- **Lowers to engine API:** a battle-resolution event already exists (security checks, deletion). The gap is exposing the **winner** permanent (controller + trait) to the triggered-effect predicate layer: e.g. a new timing `on_ally_won_battle` (or `on_battle_end` with `event_winner_*` predicates: `event_winner_owner`, `event_winner_trait_has`).
- **Suggested DSL syntax:**
  ```yaml
  - when: on_ally_won_battle        # fires when a permanent the controller owns wins a battle
    once_per_turn: true
    active_when:
      all_of:
        - all_turns: true
        - event_winner_trait_has: TS
    process:
      - trash_top_security: { of: opponent }
  ```
- **Faithfulness impact:** BT25-020's other clauses (mandatory cost reduction; [OP][WD][WA] +3000 DP + may-battle) are fully expressible, but the card cannot ship faithfully until the board-wide battle-winner predicate exists. No YAML shipped.
- **Verdict:** BT25-020 BLOCKED (gap_kind: dsl).

## G-DSL-PROTECT-OTHER-BY-SELF-DELETE — board-wide "when another of your X would leave, by deleting THIS Digimon, they don't leave"
- **Discovered by:** BT25-039 Sirenmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[All Turns] When any of your other [Shaman] or [Iliad] trait Digimon or Tamers would leave the battle area other than by your effects, by deleting this Digimon, they don't leave."
- **DCGO (BT25_039.cs):** `EffectTiming.WhenRemoveField` ActivateClass whose `CanUseCondition` matches when **another** owner permanent (Digimon or Tamer, Shaman/Iliad trait, `!IsByEffect(owner)`) would leave; the body deletes THIS permanent (`DeletePeremanentAndProcessAccordingToResult`) and, on success, sets `willBeRemoveField = false` on all such protected permanents (cancels their leave).
- **What the DSL has:** the existing replacement substrate (`kind: replacement`, `trigger: when_would_leave_battle_area`) and the `Keyword::Decode`/Barrier/Evade auto-installs are all **self-scoped** — they only fire on the carrier's own would-leave (the `replacement_subject == me` guard in `keyword_effects.rs`). There is no replacement/observer that fires on **another** permanent's would-leave with a trait/owner filter and cancels it by paying a self-deletion cost.
- **Why blocked:** modeling this needs (a) a would-leave replacement whose **subject is a filtered set of OTHER owner permanents** (not self), (b) a cost step that deletes the carrier, and (c) cancelling the original leave for every matching protected permanent. None of these compose from current vocabulary.
- **Lowers to engine API:** the parked-replacement substrate (`cancel_leave` / `handle_replacement`) and `delete_permanent` exist; the gap is a replacement clause with a non-self subject filter (`replacement_subject_is_mine` exists as a predicate but only on the carrier path) plus a cause filter (`other than by your effects`).
- **Suggested DSL syntax:**
  ```yaml
  - kind: replacement
    trigger: when_other_would_leave_battle_area
    subject_filter:
      all_of:
        - of: you
        - other: true
        - any_of: [ { kind: digimon }, { kind: tamer } ]
        - any_of: [ { trait_has: Shaman }, { trait_has: Iliad } ]
    active_when: { none_of: [ { replacement_cause: own_effect } ] }
    process:
      - delete_permanent: { target: source }   # cost
      - cancel_leave: { target: replacement_subject }
  ```
- **Verdict:** contributes to BT25-039 BLOCKED (gap_kind: dsl).

## G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER — security-zone End-of-Turn play of a named card at reduced cost, then place this security card as the played Digimon's bottom digivolution source
- **Discovered by:** BT25-039 Sirenmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[Security] [End of Your Turn] You may play 1 [Ceresmon] from your hand with the cost reduced by 7. If this effect played, you may place this card as the played Digimon's bottom digivolution card."
- **DCGO (BT25_039.cs):** `EffectTiming.OnEndTurn` ActivateClass gated by `IsExistInSecurity(card, false)` (this card is face-up/in the security stack) `&& IsOwnerTurn`. Body: `SelectHandEffect` Mode.PlayForCost over `EqualsCardName("Ceresmon") && CanPlayAsNewPermanent(fixedCost: cost-7)` with `SetReducedCostTuple((7, null))`; then a Yes/No prompt to `AddDigivolutionCardsBottom([this])` onto the played Ceresmon and `ReduceSecurity()` (move self out of security under the new Digimon).
- **What the DSL has:** security-scope clauses exist (`scope: security`), `end_of_your_turn` timing exists, and `play_from_hand` with cost reduction exists. What is missing: (a) a security-zone EOT trigger keyed on **this card living in the security stack** (not a face-up battle-area permanent), and (b) a "place THIS security card as the just-played permanent's bottom digivolution source" movement that consumes the play result binding.
- **Lowers to engine API:** play-from-hand-reduced and add-to-digivolution-bottom both have engine primitives; the gap is the security-resident self trigger at EOT plus binding the freshly-played permanent and moving this security card under it.
- **Suggested DSL syntax:**
  ```yaml
  - scope: security
    when: end_of_your_turn
    optional: true
    process:
      - play_from_hand:
          of: you
          filter: { name_contains: "Ceresmon" }
          cost_delta: { reduce: 7 }
          bind_played_as: ceres
      - if: { binding_present: ceres }
        then:
          - place_self_as_bottom_source: { of_permanent: ceres }   # move this security card under the played Digimon
  ```
- **Verdict:** contributes to BT25-039 BLOCKED (gap_kind: dsl).

## G-DSL-BEATBREAK-ARTS-OPTION — no dual Digimon+Option (BEATBREAK / Arts Digivolve) identity — RESOLVED 2026-06-15

> **RESOLVED 2026-06-15 (`gap/dual-per-face-arts`).** Folded into the
> per-face-effects + Arts-digivolve close (see the BT25-057 entry above).
> A BEATBREAK card is authored as `kind: dual` with the Digimon clauses on
> `dual.digimon.effects` and the Option `[Main]` body on `dual.option.effects`
> (`when: main` → `OptionMain`); `dual.option.arts_digivolve: true` arms the
> engine arts-digivolve selection. The old "Option side OMITTED per the BT25-041
> precedent" workaround is retired. BT25-043 Habakirimon is upgraded from
> PARTIAL to IMPLEMENTED (Option side ships): `[Main]` -8000 single target →
> by-trashing-top-security (player Yes/No) → all opp -5000 for the turn, plus
> Arts Digivolve. NOTE: BT25-041 Murasamemon remains Digimon-side-only for an
> UNRELATED reason (its [WD/WA] pay-one-of-two-costs → cost-reduced play/use is
> a different open gap, G-COST-REDUCTION-INTERACTIVE-PAY-COST); its Option side
> (if any) can now be authored with this substrate.
> Tests: `tests/cards_behavioral/bt25/bt25_043.rs` (11, green).

### Original entry (history)
- **Discovered by:** BT25-043 Habakirimon (aegiomon-2 slice), 2026-06-06. (Same family blocks the Option side of every BEATBREAK card; cf. BT25-041 Murasamemon, which shipped Digimon-side-only.)
- **Clause (Option side):** "Use Req: [Glowing Dawn] trait. [Main] 1 of your opponent's Digimon gets -8000 DP for the turn. Then, by trashing your top security card, all of your opponent's Digimon get -5000 DP for the turn. Arts Digivolve."
- **DCGO (BT25_043.cs):** the card is BOTH a Digimon and an Option — `EffectTiming.OptionSkill` (the [Main] play body) plus `CardEffectFactory.ArtsDigivolveEffect` and `UseRequirements`. A BEATBREAK card can be played as a Digimon OR used as an Option (Arts Digivolve).
- **What the DSL has:** `kind: digimon` and `kind: option` are mutually exclusive top-level kinds; there is no `arts_digivolve` alt-path kind (`CompiledAltPathKind` has Digivolve/DnaDigivolve/DigiXros/BurstDigivolve/Assembly/ActivatedDigivolve/BlastDnaDigivolve — no Arts) and no way to attach an Option `[Main]` clause to a `kind: digimon` card.
- **Lowers to engine API:** would need an engine notion of a card with two play identities (Digimon stat-line + Option [Main]/Arts-Digivolve), surfaced to the action space as two distinct play actions.
- **Suggested DSL syntax:** a `kind: beatbreak` (or `also_option:` block on a Digimon) carrying the Option [Main] `process:` and an `arts_digivolve:` alt-path.
- **Verdict:** contributes to BT25-043 PARTIAL (gap_kind: dsl). Digimon-side clauses (Recovery+unsuspend, Glowing-Dawn leave-prevention) ship; the Option side is omitted (per the BT25-041 precedent).

## G-DSL-PLAYER-CANNOT-SUSPEND-FILTER — player-level CannotSuspend/effect-immunity with a dynamic permanent filter
- **Discovered by:** BT25-028 Dianamon and BT25-059 Ceresmon (aegiomon-2 slice), 2026-06-06.
- **Clause (Dianamon):** "None of your opponent's Digimon with 1 or fewer digivolution cards can suspend until their turn ends." **(Ceresmon):** "none of your suspended [Vegetation] or [TS] trait Digimon are affected by your opponent's Digimon effects until their turn ends."
- **DCGO:** installs a player-level `CanNotSuspendClass` / `CanNotAffectedClass` carrying a `PermanentCondition` that is **re-evaluated on each suspend attempt** (Dianamon: `DigivolutionCards.Count <= 1`; Ceresmon: own suspended Veg/TS). So a Digimon that becomes eligible LATER this turn is also covered.
- **What the DSL has:** `add_player_modifier` (`AddPlayerModifierArgs`) installs a blanket player modifier with NO permanent filter; per-target `add_modifier`/`grant_effect_immunity` only apply to a specific bound permanent.
- **Current modelling:** a `for_each` over the currently-matching set applying the per-target modifier at install time (a snapshot). Practical per-turn outcome matches in the common case; the dynamic re-check nuance (a permanent that enters the matching set later in the turn) is lost.
- **Suggested DSL syntax:** `add_player_modifier:` with an optional `permanent_filter:` predicate that the engine re-evaluates per suspend/effect-application.
- **Verdict:** modelled as a documented snapshot; both cards ship IMPLEMENTED with this nuance noted.

## G-DSL-BOARD-LEVEL-SUM — no board-wide level/stat sum predicate
- **Discovered by:** BT25-077 Bacchusmon (aegiomon-2 slice), 2026-06-06.
- **Clause:** "When this card would be played, if there are 12 or more levels' total worth of Digimon, reduce the cost by 5."
- **DCGO (BT25_077.cs):** sums `permanent.Level` across ALL battle-area Digimon of BOTH players and checks `>= 12`.
- **What the DSL has:** `count_gte` (counts permanents), `card_count_in_zone` (counts cards in a zone), per-permanent `level_*` predicates, and `source_stack_dp_sum` (one permanent's stack) — but NO aggregate that sums a stat (level / DP) across a player/board set.
- **Suggested DSL syntax:** a `board_level_sum_gte` / `stat_sum` predicate (e.g. `{ stat: level, scope: any, zone: battle_area, kind: digimon } >= N`).
- **Verdict:** contributes to BT25-077 PARTIAL (gap_kind: dsl). The cost-reduction clause is omitted (rather than approximated); the two main clauses ship.

## G-DSL-SELF-COLOR-COUNT-LTE — no "distinct colors <= N" / "without N colors" base filter
- **Discovered by:** BT25-084 Titamon (aegiomon-2 slice), 2026-06-06.
- **Clause (alt-digivolve box):** "[Digivolve] [Titamon] w/o 3 colors: Cost 2."
- **DCGO (BT25_084.cs):** `AddSelfDigivolutionRequirementStaticEffect(permanentCondition: TopCard.EqualsCardName("Titamon") && TopCard.CardColors.Distinct().Count() != 3, cost 2)`.
- **What the DSL has:** `self_color_count_gte` (>= only). There is no `self_color_count_lte` / `!= N` for the base-card `from:` filter.
- **Suggested DSL syntax:** `self_color_count_lte: N` (and/or `self_color_count_eq`) usable inside an alt-path `from:` predicate.
- **Verdict:** contributes to BT25-084 PARTIAL (gap_kind: dsl). The standard Lv.5 Purple and Lv.5 [TS] cost-4 alt-paths ship; the Titamon-3-color cost-2 path is omitted.

## DSL Vocabulary ADDED: DigiLink Shape-B (Appmon Link Digimon)  [G-DSL-DIGILINK] — LANDED 2026-06-06
- **Status:** LANDED 2026-06-06 (OpenSpec `implement-digilink-mechanic` §7). New YAML vocabulary for authoring Shape-B Appmon Link *Digimon* (the `[Link]` keyword on `kind: digimon` cards, e.g. BT21-009 Gatchmon) — distinct from the existing Option-scoped `kind: link_requirement` (Plug-Ins).
- **Scope:** DSL.
- **Added vocabulary:**
  - `kind: link_condition` (declarative, body `{ cost, filter }`) — a Digimon's static self link-condition. Lowers to `Effect::link_condition(card).link_host(cost, filter)` at `EffectTiming::LinkCondition`, read by `Game::digimon_link_condition_targets`. Reuses `LinkRequirementBody`.
  - `when: when_linked` (timing) — "when this Digimon gets linked". Lowers to `EffectTiming::OnLink` + forced `.linked()` + an injected self-filter (`event_card == source_card`) so it fires once for the just-linked card, not on sibling links (design D6). Use on a `scope: linked` effect.
  - `scope: linked` + `kind: grant_keyword` (and DP grants) — a linked card's Link-ESS now sets `.linked()` (previously only `scope: inherited` set `.inherited()`), so the grant materializes onto the HOST via the `tick_declarative_effects` linked-card pass (mirrors DCGO `RaidSelfEffect(isLinkedEffect: true)`).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- dsl_digimon_link_card_full_flow` (authors a full Appmon Link Digimon in YAML — link_condition + when_linked draw + linked Raid ESS — and exercises the real link-activate → absorb → OnLink path).
- **Residual:** from-hand Digimon-link initiation + rarer source origins (trash / under-stack / re-link) are not yet wired (engine-side); see `docs/RUST_ENGINE_GAPS.md` 2026-06-06 Shape-B note. Authoring the *named* acceptance cards (BT21-009 Gatchmon etc., with their alt-digivolve / specific WhenLinked bodies) is the §2 follow-up.

## BT25 "link-ts" slice — BLOCKED card (2026-06-07)

Re-run of the BT25 link-ts slice (BT25-069, BT25-066, BT25-075, BT25-101, BT25-102, BT25-089)
against the post–DigiLink-Shape-B substrate (commit 5514135c, 2026-06-07). Shape-B added the
player-activated link of a *standing Digimon onto a host* (root `None`) plus the `kind: link_condition`
/ `when: when_linked` / `scope: linked` authoring layer — but it did **NOT** add a verb/primitive for
"link a card **chosen from trash / hand / digivolution-cards** to one of your Digimon as an effect"
(the deferred residual at `docs/RUST_ENGINE_GAPS.md` §"[Link] subsystem", Shape-B note line ~585:
"from-hand Digimon-link initiation and the rarer source origins (trash / under-stack / re-link) are
not yet wired"). All six slice cards remain BLOCKED on that same residual. Five were already tracked
(BT25-069/066/101 here; BT25-102 in engine-gaps; BT25-089 in RUST_ENGINE_GAPS); BT25-075 is added below.

### BT25-075 Vulcanusmon — link up-to-2 chosen cards from {hand|trash} + per-link <De-Digivolve 1> + aura <Link +1>  [gap_kind: hybrid]
- **Card(s):** BT25-075 Vulcanusmon (Lv.6 Black, Undead/Titan/TS).
- **Effect text:**
  - `When this card would be played, if you have fewer Digimon than your opponent, reduce the cost by 5.` (expressible — fewer-own-Digimon cost reducer.)
  - `[On Play] [When Digivolving] You may link up to 2 cards from your hand or trash to any of your Digimon without paying the cost. Then, for each of your link cards, <De-Digivolve 1> all of your opponent's Digimon.` — **BLOCKED.** Linking up-to-2 cards **chosen from hand or trash** (DCGO `BT25_075.cs`: per-card `SetIntSelection` "from Hand / from Trash / Do not link" → `ILinkCard`) is exactly the deferred "link a chosen card from hand/trash" primitive (cross-ref BT25-069/072/101/056/089 and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem facet #9). The `link_to_own_digimon` DSL verb links only the carrier Option; there is no verb to attach an arbitrary hand/trash card as a link to *any* of your Digimon. The follow-up `<De-Digivolve 1> all opp Digimon for each of your link cards` is expressible in isolation but can only fire after the (unauthorable) link step, so it cannot ship.
  - `[All Turns] All of your [TS] trait Digimon gain <Rush> and <Link +1>.` — Rush grant is expressible (`grant_keyword: Rush` aura); the **`<Link +1>`** grant is **BLOCKED** by the same engine gap as BT25-102 (`G-ENGINE-AURA-GRANT-LINK-MAX` in `qa/archetype-qa/engine-gaps.md` — auras apply `ModifierType::ChangeLinkMax` with a hardcoded value of 0, so a +1 max-link grant is unauthorable without an approximation).
  - `[Your Turn] When your Digimon get linked, one of them may attack.` — depends on a `WhenLinked` host trigger over *any* of your Digimon getting linked (not the self-link of `when: when_linked`, which is self-filtered to the just-linked card). Even were that authorable, it is downstream of the blocked link step.
- **What's missing (two facets):**
  1. **Link N chosen cards from hand/trash to any own Digimon (free).** Engine has the link substrate (`Permanent.linked_cards`, `attach_linked_card`) but no effect-driven "pick a card from hand/trash and attach it as a link to a chosen Digimon" path; the DSL has no verb. (Shape-B only absorbs a *standing Digimon* onto a host.)
  2. **Aura-granted `<Link +1>` carrying a non-zero value** — `G-ENGINE-AURA-GRANT-LINK-MAX` (engine; see engine-gaps.md).
- **Lowers to engine API:** facet 1 → a new effect-link-chosen-card primitive over `Permanent.linked_cards`; facet 2 → `ModifierType::ChangeLinkMax(+1)` via an aura modifier that can carry a value.
- **Verdict:** BLOCKED (hybrid). Ships no YAML — every active clause depends on the chosen-card link primitive and/or the valued aura-Link+1 grant. Cross-ref BT25-069/072/101/056/089, BT25-102 (`G-ENGINE-AURA-GRANT-LINK-MAX`), and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

## DSL Vocabulary ADDED: host-side `[When Linked]` timing  [G-DSL-WHEN-LINKED-HOST] — LANDED 2026-06-07
- **Status:** LANDED 2026-06-07. New timing `when: when_card_linked_to_this` — the host-POV "[When Linked] when a card gets linked **to this Digimon**" (DCGO `CardEffectCommons.CanTriggerWhenLinked`). Lives on a face-up `scope` effect on the host. Lowers to `EffectTiming::OnLink` + a host self-filter (`event_permanent() == source_permanent`) so it fires once for the receiving host only, not for a sibling host. Distinct from the card-POV `when: when_linked` (`event_card == source_card`).
- **Scope:** DSL. Plumbing: `Timing::WhenCardLinkedToThis` (clause.rs) → `CompiledTiming::WhenCardLinkedToThis` (compiled.rs / compile.rs) → `timing_map.rs` (→ `OnLink`) + `lower_triggered.rs` (`is_host_linked` forces the host self-filter).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- dsl_host_side_when_card_linked_to_this_fires_on_attach host_side_when_linked_fires_for_receiving_host_only`.

## RETIRED → folded into `link_cards`: `link_card_to_self` step (facet #9 authoring verb)  [G-DSL-LINK-CARD-FROM-ZONE]
- **RETIRED 2026-06-19 (collapse-dsl-step-idioms §4).** `link_card_to_self` is DELETED — the `StepSpec`/`CompiledStep::LinkCardToSelf` variants, `LinkCardToSelfArgs`/`LinkFromZone`/`LinkToHost` types, the `compile.rs` arm, the `lower_triggered` outer-optional arm, and the whole `src/dsl_cards/step/link_card.rs` lowering are gone. All 11 users (ST22-12, BT21-023/073/101, BT25-052/056/060/069/070/072/089) migrated to the more general `link_cards { from, filter, to: self|own_digimon, count: { up_to: 1 }, cost: free }` (zone-name map `digivolution_sources → self_sources`, `chosen_own_digimon → own_digimon`). This was also a **faithfulness improvement**: `link_card_to_self` presented a single union-of-zones selection, whereas `link_cards` (and DCGO's actual `SetBoolSelection`) present a zone-choice-first flow. BT25-060's "By linking 1 …, 1 of your Digimon may unsuspend" `if (linked)` gate is now modeled by the new `link_cards` **`bind_as`** field (captures the linked card only on a real link) + `if { binding_present }`. The dropped `link_cost_delta_for_player` application was a no-op for every real user (all `cost: 0`). 121 behavioral tests across the 11 cards stay green. Historical detail below.
- **Status (historical):** LANDED 2026-06-07. DSL step `link_card_to_self` authored: `{ from: [hand|trash|digivolution_sources], filter: PredicateSpec, to: self|chosen_own_digimon (default self), cost: u16 (default 0), optional: bool }`. Lowering in `code/digimon-engine/src/dsl_cards/step/link_card.rs` gathers candidates across the requested zones into ONE RL-visible `SelectionKind::Target` prompt (no auto-pick — disjoint per-zone action ranges so the union is unambiguous), and on resolution computes effective cost (`cost + link_cost_delta_for_player`).max(0), pays it, and calls the primitive `Game::link_chosen_card_into_host(host, chosen, source_zone)`. With `to: chosen_own_digimon` a SECOND RL-visible selection over the controller's standing Digimon picks the host ("link to 1 of your Digimon" — e.g. BT25-069/089). DSL surface: `StepSpec::LinkCardToSelf` / `LinkCardToSelfArgs` / `LinkFromZone` / `LinkToHost` in `code/digimon-dsl/src/step.rs`; `CompiledStep::LinkCardToSelf` in `compiled.rs`; lowering in `compile.rs`. **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- link_card_to_self_links_chosen_hand_card_pays_cost_and_fires_onlink link_card_to_self_applies_change_link_cost_reduction` (both green; first pins selection + cost + OnLink propagation via a host-side `when_card_linked_to_this` reaction, second pins the `ChangeLinkCost` reduction path). Chosen-host path pinned by `bt25_069_on_play_links_ts_from_trash_to_chosen_own_digimon` + `bt25_089_main_links_appmon_from_hand_to_chosen_digimon`. Cards authored on it: BT25-052/056/070/072 (self-host), BT25-069/089 (chosen-host). Pairs with facet #10's flat `ChangeLinkCost`.
- **Cost-modeling note:** the printed "with the cost reduced by N" is a reduction on the LINKED card's own link cost. DSL card fixtures carry no engine-side link cost on the linked candidate, so cards author `cost: 0` (the reduction makes the typical 1–2 link free in practice); the flat `ChangeLinkCost` path covers any nonzero residual. Faithful for the no-engine-link-cost fixtures; revisit if a linked candidate carries a nonzero engine link cost.
- **Residual (NOT yet authored — separate gaps):**
  - `G-DSL-LINK-N-CARDS-PER-HOST` (BT25-075): "link up to 2 cards from hand/trash, each to a *separately chosen* Digimon" — the single-card step does not loop with per-card host selection. Needs a `count: N` extension that repeats the (card → host) pair selection. **NOT done — BT25-075 left BLOCKED.**
  - `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` (BT25-089 [Main]): "from your *Digimon's* digivolution cards" scans EVERY own Digimon's under-sources; the step's `digivolution_sources` zone anchors only to the effect's own permanent. BT25-089 authored the hand source (PARTIAL — that source clause omitted).
- **Superseded OPEN notes below (historical):**
- **gap_kind:** dsl.
- **What it authors:** "[…] you may link 1 [card matching FILTER] from your hand / this Digimon's digivolution cards / your trash to this Digimon […]" (DCGO `ILinkCard.LinkCard` with `root != None` → `Permanent.AddLinkCard`). Representative card: **ST22-12** ("link 1 Digimon card with [Social]/[Navi]/[Tool] from your hand or this Digimon's digivolution cards to this Digimon with the link cost reduced by 2").
- **Engine primitive (ready, exposed):** `EffectContext::link_chosen_card_into_host(host, card, LinkCardSource)` → `Game::link_chosen_card_into_host` — lifts the chosen card out of `LinkCardSource::{Hand|Trash|DigivolutionSource}`, attaches onto the host's `linked_cards`, fires `OnLink`. Tested: `facet9_link_chosen_card_from_hand_attaches_and_fires_onlink`, `facet9_link_chosen_card_from_digivolution_sources`.
- **Remaining DSL surface:** a `StepSpec` (e.g. `link_card_to_self: { from: [hand, digivolution_sources, trash], filter: PredicateSpec, cost: N, optional: bool }`) that (1) gathers candidate cards across the chosen zones, (2) installs the zone→card selection, (3) on resolution computes effective cost (`base − link_cost_delta_for_player`), pays it, and calls the primitive with the host = the effect's own permanent. Distinct from the existing `link_to_own_digimon` step (Plug-In Option self-link, host-selection, tied to `pending_option`). Pairs with facet #10's flat `ChangeLinkCost` for the "link cost reduced by N" clause.

## DEFERRED (no card needs it): predicated `ChangeLinkCost` reduction  [G-DSL-LINK-COST-PREDICATED]
- **Status:** DEFERRED-until-needed (not a blocking gap). The **flat** player-/permanent-scoped `ModifierType::ChangeLinkCost` (DSL-authorable; summed by `link_cost_delta_for_player`; consulted at all three link-cost sites) covers every real cost-reducer — DCGO's `GrantedReduceLinkCostClass` is invoked with `_ => true` for all of `cardSourceCondition`/`permanentCondition`/`rootCondition` (see ST22-12). DCGO's general `ChangeLinkCostClass` supports per-(source/host/root) predicates, but no printed card exercises them, so building predicated reduction now would be speculative machinery. File a concrete card here if one is found. Confirming test for the flat path: `facet10_change_link_cost_reduces_paid_link_cost`.

## DSL Gap: BT25-075 — formula source "number of your own link cards" (total across all your Digimon)  [G-DSL-FORMULA-OWN-LINK-CARD-COUNT]
- **Discovered by:** BT25-075 Vulcanusmon (link-finish-aura slice), 2026-06-07.
- **Effect text:** "[On Play] [When Digivolving] You may link up to 2 cards from your hand or trash to any of your Digimon without paying the cost. **Then, for each of your link cards, ＜De-Digivolve 1＞ all of your opponent's Digimon.**"
- **DCGO (BT25_075.cs):** `int degenerationCount = card.Owner.GetBattleAreaDigimons().Map(p => p.LinkedCards).Flat().Count();` then loops `IMassDegeneration(enemy Digimon, 1)` that many times — i.e. De-Digivolve 1 applied to **all** opponent Digimon, repeated N times where N = the total count of link cards across every one of the controller's battle-area Digimon (counted *after* the link step above resolves).
- **What's already expressible (today):** the link half ships via the new `link_cards` step (`from: [hand, trash]`, `to: own_digimon`, `count: { up_to: 2 }`, `cost: free`) — that step was authored partly for this card (its doc names BT25-075). The `<Link +1>`/`<Rush>` `[All Turns]` aura ships via aura `modifier: ChangeLinkMax` + `modifier_value: 1` and `grant_keyword: Rush` (G-ENGINE-AURA-GRANT-LINK-MAX resolved 2026-06-07). The `de_digivolve` step exists with `amount` / `amount_fn` (FormulaSpec) and can target all opp Digimon.
- **What's missing:** a **`FormulaSpec` / `PerSelector` source that counts own link cards**. `code/digimon-dsl/src/formula.rs` `PerSelector` has `MaterialCount`, `SuspendedCount`, `AllyCount`, `CardCountInZone`, etc., but nothing that sums `permanent.linked_cards.len()` across the controller's battle-area Digimon. Without it, `de_digivolve: { target: all_opp_digimon, amount_fn: <own-link-card-count> }` cannot be authored, and the De-Digivolve clause's magnitude (a player-visible board swing) cannot be modeled → no-approximations violation, whole card BLOCKED.
- **Lowers to engine API:** the substrate exists — `Permanent.linked_cards` is populated and counted at multiple sites (e.g. `game_actions.rs:1494`, `tensor_v1.rs:267`). The missing piece is purely a DSL formula selector + its evaluator reading `ctx`'s controller battle-area Digimon and summing `linked_cards.len()`.
- **Suggested DSL syntax:** a `FormulaSpec` variant `{ own_link_card_count: { of: you } }` (or a `PerSelector::OwnLinkCardCount { of }` usable in `base_per_delta`), evaluating to `Σ over of.battle_area Digimon of permanent.linked_cards.len()`. Used as `de_digivolve: { target: <all opp digimon>, amount_fn: { own_link_card_count: { of: you } } }` — but note DCGO applies De-Digivolve-1 N *separate* times to the whole opp board, not De-Digivolve-N once; the lowering must repeat the mass De-Digivolve-1 N times (or `amount: 1` with an outer `repeat: <formula>`), matching `IMassDegeneration(..., 1)` × N. A `repeat_n: <FormulaSpec>` wrapper around a step would also close this.

## LM-020 — return a selected SECURITY card to a deck  [G-DSL-RETURN-SELECTED-SECURITY-TO-DECK]

**CLOSED 2026-06-05.** Added the `return_selected_security_to_deck` DSL verb
(`ReturnToDeckArgs`: of/card/position) + the engine primitive
`EffectContext::return_security_card_to_deck(player, card, to_bottom)` and a new
`SecurityRemovalDestination::Deck { owner, to_bottom }` handled in
`complete_effect_security_removal` (Digi-Eggs route to the digitama deck; fires the
OnLoseSecurity / OnOpponentSecurityRemoved observer chain). LM-020 Quantumon is now
fully authored (`code/digimon-engine/cards/lm/LM-020.yaml`, both clauses) and
judge-quiz **Q18 → PASS**. A second small gap surfaced while authoring clause 2 —
no predicate compared a *bound card's* category to a declared one — closed by the
new `binding_card_kind: { binding, kind }` predicate. Tests:
`tests/effect_context/security_stack_operations.rs` (3) +
`tests/cards_behavioral/lm/lm_020.rs` (4) + judge-quiz Q18.

Surfaced: 2026-05-29, judge-quiz first wave (`batch-implement-cards-rust-dsl`). LM-020 Quantumon BLOCKED.

- **Missing DSL verb:** `return_selected_security_to_deck` — route a `select_security`-bound `CardHandle` to the owner's deck **top or bottom**. The three verbs that consume a `select_security` pick route it to hand (`add_to_hand_from_security`), play (`play_security_card`), or trash (`trash_selected_security`) — never to a deck.
  - Suggested YAML (mirrors `return_to_deck_from_reveal { of, card, position }`):
    ```yaml
    - return_selected_security_to_deck: { of: opponent, card: picked_sec, position: top }   # top | bottom
    ```
- **Engine prerequisite (root cause — also logged in `docs/RUST_ENGINE_GAPS.md`):** no public `EffectContext` method moves a security card to a deck. The private `move_card_to_deck` helper (`effect_context/mod.rs`) is sourced from trash only. Suggested `pub fn return_security_card_to_deck(&mut self, player, card, to_bottom) -> bool`: find the card in `player.security`, `ensure_security_materialized`, remove it, drop from `face_up_security`, fire `fire_security_removed_observers` (add a `SecurityRemovalDestination::Deck` variant alongside `::Hand`), then route through the existing trash->deck `move_card_to_deck` path. Lower the new verb in `dsl_cards/step/zone_moves.rs` alongside `AddToHandFromSecurity` / `TrashSelectedSecurity`.
- **Card text:** LM-020 [When Digivolving] "... reveal all of your opponent's security cards, and place 1 card among them on top of your opponent's deck. Shuffle the rest and return them to the security stack." DCGO `LM_020.cs`: `IReduceSecurity` -> `AddLibraryTopCards` -> shuffle.
- **Blocks:** LM-020 (Quantumon) -> judge-quiz Q18. (LM-020's `[Start of Opponent's Turn]` category-immunity clause is independently implementable; only the security->deck clause is blocked.) Likely shared by other "place a security card on top/bottom of deck" cards. Re-attempt LM-020 once the verb lands.

## BT13-088 — place a card as the TOP digivolution source  [G-DSL-PLACE-AS-TOP-SOURCE]

Surfaced: 2026-05-29, judge-quiz first wave. BT13-088 Belphemon: Sleep Mode shipped PARTIAL.

- **Missing DSL verb / engine primitive:** a "place card as the TOP digivolution source" (just below the face card) — DCGO `AddDigivolutionCardsTop`. The engine ships `place_as_bottom_source` (inserts at index 0) only; no top-source insertion.
- **Resolution used:** BT13-088 uses `place_as_bottom_source` for "place [Belphemon: Rage Mode] on top of this Digimon's digivolution cards." Position is **behaviorally inert** for this card (it only needs Rage Mode IN the stack to gain the inherited effect; no mechanic reads the top-source slot) -> shipped PARTIAL, not BLOCKED. A future card whose text/behavior depends on the top-source position would need this verb (+ an `EffectContext::place_as_top_source` primitive).

## EX5-060 — opponent plays from their OWN trash SUSPENDED + played-permanent-level formula  [G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED] / [G-EVENT-PLAYED-LEVEL-FORMULA] — **RESOLVED 2026-06-11 (judge-quiz Q28)**

> **RESOLVED 2026-06-11.** All pieces landed with the Q28 slice: (1)
> `play_from_trash_free` now plays for the BINDING OWNER's side (the trash
> owner) — `play_from_trash_free_unsuspended_for(controller, …)`; (2) new
> `suspended: true` arg (G-PLAY-ENTERS-SUSPENDED — the permanent ENTERS the
> battle area suspended, before play-event observers, via
> `Game::play_enters_suspended` consumed at the single commit site); (3) new
> `event_target_level: {}` FormulaSpec leaf (reads the trigger's event card's
> level — DCGO `LevelJustAfterPlayed`) usable inside `level_lte: { formula: … }`;
> (4) the `suppress_on_play` rider is consult-gated on
> `permanent_is_unaffected_by_effect` vs the recorded suppressor identity
> (`Game::on_play_suppressor`) — a protected played Digimon still fires its
> [On Play] (the Q28 ruling). The `event_played_by_effect` predicate from the
> original sketch was NOT needed — the existing `event_is_effect_initiated`
> leaf covers it (the suspend-bit work threaded `effect_initiated` through
> `TriggerSource::EnteredField`). EX5-060 Dragomon IMPLEMENTED; pins:
> `cards_behavioral/ex5/ex5_060.rs` (5) +
> `judge_quiz a::q28_*` (pin + control). RELATED: BT20-059's board-wide
> protection re-authored as the CONTINUOUS `grant_effect_immunity` form
> (`continuous: true` + `targets:` → floating mass modifier carrying an
> `EffectImmunityFilter` payload), closing
> G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA — the per-tick re-scan covers
> permanents played later in the window (the judge's "persistent effect").

### Original entry (history)

Surfaced: 2026-05-29, judge-quiz wave (`batch-implement-cards-rust-dsl`). EX5-060 Dragomon BLOCKED (pins Q28 alongside BT20-059 Gankoomon X).

- **Card text:**
  - Clause 1 [On Play][When Digivolving]: "Your opponent plays 1 level 4 or lower Digimon card from their trash **suspended** without paying the cost. [On Play] effects on Digimon played by this effect don't activate."
  - Clause 2 [All Turns][Once Per Turn]: "When an effect plays an opponent's Digimon, you may play 1 purple Digimon card with **a level less than or equal to it** from your trash without paying the cost."

- **Clause 1 — DSL gap (root cause is the engine gap `G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED` in `docs/RUST_ENGINE_GAPS.md`):** `play_from_trash_free` cannot (a) play from the **opponent's** trash — its `of:` field is dropped at the engine boundary (lowers to `play_from_trash_free_unsuspended*`, which hardcodes `self.player`) — or (b) play **suspended** (no `suspended:` flag anywhere in the play-from-trash chain). The `[On Play] don't activate` half already works via `suppress_on_play: true`. Intended shape once both land:
  ```yaml
  - as_selecting_player:
      of: opponent
      body:
        - select_trash: { of: opponent, bind_as: opp_pick, filter: { kind: digimon, level_lte: 4 }, prompt: "..." }
        - play_from_trash_free: { of: opponent, hand_index: opp_pick, suspended: true, suppress_on_play: true }  # of:opponent + suspended NEW
  ```

- **Clause 2 — DSL gaps:**
  - **`event_played_by_effect` predicate** — `on_any_digimon_played` cannot distinguish a normal hard-play from an effect-play. DCGO `EX5_060.cs` gates Clause 3 on `IsByEffect`. No `by_effect`/`event_played_by_effect` predicate leaf exists in `predicate.rs`.
  - **`event_target_level` FORMULA** — "a level less than or equal to **it**" bounds the own-trash recursion filter by the *played opponent permanent's level*. Only the predicate leaves `event_target_level_lte/_eq/_gte` exist (compare against a literal); there is no `FormulaSpec::EventTargetLevel` to feed `level_lte: { formula: ... }`. DCGO reads `permanent.LevelJustAfterPlayed`. The trigger timing + `event_target_owner: opponent` + `event_target_kind: digimon` predicates DO exist.
  Intended shape once both land:
  ```yaml
  - when: on_any_digimon_played
    active_when: { all_turns: true }
    once_per_turn: true
    optional: true
    condition: { all_of: [ { event_target_owner: opponent }, { event_target_kind: digimon }, { event_played_by_effect: true } ] }  # by_effect NEW
    process:
      - select_trash: { of: you, bind_as: recur, optional: true, filter: { all_of: [ { kind: digimon }, { color_is: purple }, { level_lte: { formula: { event_target_level: {} } } } ] }, prompt: "..." }  # event_target_level formula NEW
      - play_from_trash_free: { of: you, hand_index: recur }
  ```

- **Blocks:** EX5-060 (judge-quiz Q28). `code/digimon-engine/cards/ex5/EX5-060.yaml` Clauses 1 & 2 declared with faithful timing / OPT / optional flags but empty (gap-blocked) `process` bodies — never resolve a wrong approximation. Inherited ＜Piercing＞ is fully supported and authored live. Tests in `code/digimon-engine/tests/cards_behavioral/ex5/ex5_060.rs`: `ex5_060_clause1_*` `#[ignore]`'d with `G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED`; `ex5_060_clause2_*` `#[ignore]`'d with `G-EVENT-PLAYED-LEVEL-FORMULA`. The Q28 negative (`ex5_060_lock_does_not_attach_to_effect_immune_target`) runs LIVE.


## ST17-07 — opponent-scoped effect-protection from `add_modifier`  [G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL]

**Surfaced 2026-05-30** (judge-quiz cluster B, ST17-07 Rapidmon). PARTIAL: the
green-Tamer rider "until end of opp turn, your OPPONENT'S effects can't delete
this Digimon or return it to hand/deck" is omitted.

- **Problem.** The DSL `add_modifier` step lowers through
  `EffectContext::add_modifier` → `ModifierEntry::simple`, whose `cause_filter`
  is `None` — the replacement fire-site treats `None` as CAUSE-AGNOSTIC, so
  `add_modifier { CannotBeDestroyedByEffect | CannotBeReturnedToHand |
  CannotBeReturnedToDeck }` blocks the controller's OWN effects too. DCGO scopes
  all three protections to `IsOpponentEffect`. `default_passive_cause_filter`
  (which would scope the Return ones to OpponentEffect) is consulted ONLY by
  `ModifierEntry::passive_replacement`, never by `ctx.add_modifier`.
- **Latent class.** Existing cards using `add_modifier` for these protections
  (BT18-064, P-215, EX8-070) silently ship cause-agnostic and only assert the
  modifier is *present* (never own-vs-opponent scope), so the divergence is
  currently unverified across the codebase — a widening here would correct them.
- **Engine half is mostly present.** `ModifierEntry::opponent_only()`
  (modifiers.rs) forces `cause_filter = Some(OpponentEffect)` and the fire-site
  honors it; the missing piece is exposing an installer that uses
  `passive_replacement(...).opponent_only()` from the DSL.
- **Suggested widening (backward-compatible, opt-in).** Add `opponent_only:
  bool` (default false) to the `add_modifier` DSL step; when true, route the
  install through `passive_replacement(modifier, expiry, player).opponent_only()`
  instead of `ModifierEntry::simple`. Existing cards (flag unset) are unchanged.
  Deferred as a deliberate cross-cutting change (it changes the *meaning* of
  these protections for shipped cards) — should regress BT18-064/P-215/EX8-070.
  Until landed, ST17-07's rider stays omitted (NOT shipped cause-agnostically)
  with `st17_07::st17_07_green_tamer_grants_opponent_only_delete_protection` /
  `..._protection_not_installed_without_green_tamer` `#[ignore]`'d citing this ID.

## DSL Gap: BT3-109 — no "deleted self card in trash" binding for granted [On Deletion] trash-play  [G-DSL-DELETED-SELF-TRASH-BINDING]
- **Status:** CLOSED 2026-06-05. BT3-109 authored (`code/digimon-engine/cards/bt3/BT3-109.yaml`), behavioral test green (`tests/cards_behavioral/bt3/bt3_109.rs`), and judge-quiz **Q21 → PASS**. The premise was partly wrong: the `event_card` / `event_target` bindings ALREADY resolve the just-deleted carrier's top card in trash (`binding_ref.rs` reads `DeletedObjectSnapshot.top_card` for both). The only real missing link was that `play_from_trash_free` accepted a `TrashIndex` binding but not a `Card`-handle binding — so a card-identity binding like `event_card` couldn't feed it. Fixed by making the `PlayFromTrashFree` step arm also accept `ResolvedBinding::Card(h)` (`code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`); the engine call `play_from_trash_free_unsuspended` self-guards that the handle is in the controller's trash. No new `StructuredBindingRef` variant was needed. Composes with the Q19 top-most-card-in-trash gate: replaying the carrier suppresses its remaining [On Deletion] bundle. `suppress_on_play: true` covers "Any [On Play] effects ... don't activate". The "BLOCKED guard test" mentioned below was never committed; the real test is the live behavioral + judge-quiz pin.
- **Status (historical):** OPEN (discovered 2026-06-03, BT3-109 Back for Revenge! DSL implementation — judge-quiz Q21 cluster).
- **Scope:** DSL.
- **Card(s):** BT3-109 Back for Revenge! — `[Main] 1 of your Digimon gains "[On Deletion] Play this card without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate." for the turn.` Generalizes to any granted-or-printed [On Deletion] body that must play "this card" (the just-deleted carrier's own top card, now in the trash) back from trash.
- **Recovered text source:** DCGO `DCGO/Assets/Scripts/CardEffect/BT3/Purple/BT3_109.cs` (cards.json `effect_description_eng` is garbled with doubled/nested quotes). DCGO: OptionSkill selects exactly 1 of your Digimon (mandatory, `canNoSelect: false`); grants it an `OnDestroyedAnyone` ActivateClass with `EffectDuration.UntilEachTurnEnd`; the granted body plays `selectedPermanent.TopCard` from `root: Trash`, `payCost: false`, `activateETB: false`. No level/cost cap.
- **What's missing:** `play_from_trash_free` (and `play_from_trash`) take `hand_index: <BindingRef>` — a binding to a SPECIFIC trash card. "This card" is the carrier's own top card after it moved to trash on deletion, but `StructuredBindingRef` (`code/digimon-dsl/src/step.rs:1040`) exposes only `permanent` / `source_permanent` / `zone` / `of_permanent` / `deck_top` — there is NO binding resolving "the just-deleted self card now in the trash". A generic `select_trash` is not a faithful substitute: "this card" is one specific card, and there is no identity predicate tying a trash card back to the carrier permanent that was just deleted, so a filter-based pick over-exposes every other matching trash card as an illegal choice (no-approximations / rule 17 violation). NOTE: the earlier-suspected second blocker (granted-body selection support) is CLOSED — Phase 4i "Queue-based granted-body dispatch + selection support" parks selection-installing granted bodies via `pending_selection`. The stale "v1 limitation" comment in `code/digimon-engine/src/dsl_cards/step/grant_triggered.rs` predates Phase 4i.
- **Suggested change:** Add a card-identity binding for the deleted-self card in trash usable inside an [On Deletion] body / granted [On Deletion] body (e.g. a `StructuredBindingRef` variant `deleted_self_in_trash` or a `trigger_self_card` binding that resolves the carrier's pre-deletion top card now in the trash), accepted by `play_from_trash` / `play_from_trash_free` `hand_index`. Pairs with the existing `suppress_on_play: true` to express "Any [On Play] effects ... don't activate" faithfully.
- **Workaround:** None faithful. BT3-109 is BLOCKED — left UNIMPLEMENTED (no YAML in the embedded pack) rather than stubbed with an auto-selection or an approximate "play any Digimon from trash" surrogate. A BLOCKED guard test pins the absence in `code/digimon-engine/tests/cards_behavioral/bt3/bt3_109.rs`.

## DSL Gap: BT13-103 — cost_reduction amount driven by an interactive in-cost deletion  [G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT]
- **Status:** OPEN (discovered 2026-06-03, BT13-103 Akihiro Kurata DSL implementation).
- **Scope:** DSL + engine (the engine half of the cost-reduction scan must also change).
- **Card(s):** BT13-103 Akihiro Kurata Clause 1 — `[Your Turn] When you would play a card with [Belphemon] in its name, by deleting 1 of your Digimon with [Gizmon] in its name, reduce the play cost by the play cost of the deleted Digimon.` Generalizes to any BeforePayCost reduction whose **amount is set by a permanent the player interactively selects and deletes/pays during the cost** (the reduction = the deleted/paid permanent's printed cost).
- **Authoritative source:** DCGO `DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_103.cs` (EffectTiming.BeforePayCost). `SelectPermanentEffect` over own non-immune [Gizmon]-name Digimon, `canNoSelect: true` (optional); on a pick, `DeletePeremanentAndProcessAccordingToResult`, then installs a `ChangeCostClass` of `-permanent.CostJustBeforeRemoveField` for the current play. The reduction magnitude is the *selected* Digimon's cost — known only AFTER the in-cost selection. (DCGO also ships an AI-only `EffectTiming.None` mirror that auto-picks `gizmonCosts.Max()` — an approximation we may NOT replicate under rule 17.)
- **What's missing:** the DSL `kind: cost_reduction` clause splits the amount from the cost across two callbacks that cannot share the selection:
  - `amount` / `amount_fn` is evaluated in `cost_reduction_fn` (READ context) by `apply_cost_reduction_candidate` (`code/digimon-engine/src/game_actions.rs:5848`) **before** `pay_cost_fn` runs.
  - `pay_cost_fn` (`code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs:193`) builds a **fresh** `Bindings`, so any permanent selected/bound inside `pay_cost` is invisible to `amount_fn`. No `FormulaSpec` (`code/digimon-dsl/src/formula.rs`) reads "the cost of the permanent paid as the cost" — `BindingPlayCost` reads a *prior* `bind_as` binding only, unreachable from the isolated pay_cost scope.
  - `pay_cost_fn` is additionally gated to `RunOutcome::Synchronous` (`lower_cost_reduction.rs:195`); an interactive `select_own_permanent` parks (non-synchronous) → the cost reads as failed → the reduction is dropped. So even the *selection* cannot surface through `pending_selection`. (See also `game_actions.rs:5678`, which skips paid reducers entirely without a real cost target.)
- **Suggested change:** make the cost-reduction `pay_cost` (i) able to surface an interactive `pending_selection` and resume, and (ii) able to bind the selected/paid permanent into a binding scope that `amount_fn` can read (e.g. a `BindingPlayCost` over a `pay_cost`-produced binding, or a dedicated `paid_cost_total` formula that sums the printed cost of permanents deleted/paid during this pay_cost). Backward-compatible: literal `amount:` and the existing synchronous self-suspend / return-to-deck pay_costs are unchanged.
- **Workaround:** None faithful. BT13-103 Clause 1 is BLOCKED — left UNIMPLEMENTED in `code/digimon-engine/cards/bt13/BT13-103.yaml` (Clauses 2 & 3 ARE authored) rather than approximated. The Clause-1 behavioral test `bt13_103_belphemon_play_cost_reduced_by_deleted_gizmon_cost` in `code/digimon-engine/tests/cards_behavioral/bt13/bt13_103.rs` is `#[ignore]`'d citing this ID.
### RESOLVED 2026-06-02 — `may_attack_now: { windowed: true }` (deferred EOT attack grant)
- **Gap:** `[End of Your Turn] 1 of your Digimon may attack` (AD1-004 WarGreymon)
  was authored with inline `may_attack_now`, which declares AND resolves the
  attack synchronously inside the trigger. That leaves no window for a sibling
  end-of-turn effect (e.g. an inherited DNA digivolve) to resolve first and
  remove the attacker, so the attack could never fizzle — contrary to
  general_rule.pdf §15-4-2-3 (EOT triggers activate one at a time) + the
  "attack ends if the attacker has left" rule.
- **Resolution:** added a `windowed: bool` flag to the `may_attack_now` step
  (`MayAttackNowArgs` → `CompiledStep::MayAttackNow`). When true, the step grants
  the chosen attacker a `MayAttack` (+`CanAttackUnsuspended` iff `without_suspending`)
  modifier with `Expiry::EndOfTurn` — the same windowed mechanism the `<Execute>`
  keyword uses — instead of declaring inline. The attack becomes a deferred
  EOT-action; if a sibling EOT effect removes the attacker, the grant is orphaned
  and no attack happens. AD1-004's YAML now sets `windowed: true`. Default is
  false, so all other `may_attack_now` users (AD1-009, BT12/17/20/…) are unchanged.
- **Tests:** `cards_behavioral/ad1/ad1_004.rs::{ad1_004_eot_attack_is_windowed_grant_not_synchronous,
  ad1_004_eot_attack_fizzles_when_attacker_is_removed_before_it_acts}`.
- **Note:** AD1-004 stays PARTIAL overall — its `[On Play][When Digivolving]`
  "delete opponent Digimon with DP ≤ this" is still blocked on G-FORMULA-SOURCE-DP
  (unrelated to this fix).

## RESOLVED 2026-06-03 — `select_count_capped_multi.clamp_to_available` (mandatory "N of opponent" target count)

**Gap (FAQ MP-30/31, discovered via `tests/rules_faq/effect_resolution.rs`):** the DSL
`select_count_capped_multi` step had no way to express a mandatory **"N of your opponent's
Digimon"** target count. Its `min` field carries *cost* semantics (no-op when fewer than `min`
candidates exist), and with `min` absent the floor defaults to 1 — so a card whose text reads
"Suspend **2**" (BT24-051 Merukimon) let the player stop after suspending **one** when two were
available (violates MP-31), while naïvely setting `min: 2` would fizzle the effect when only one
target is in play (violates MP-30, which requires affecting `min(N, available)`).

**Fix (widened the substrate, per rule 28):** added a `clamp_to_available: bool` field to
`SelectCountCappedArgs` (and the compiled step). When true, the required floor is clamped to
`min(max, available_candidates)` and the step never no-ops for "fewer than N" — the rules-correct
"affect as many as possible, up to N" semantics. Implemented in the battle-area path
(`install_select_count_capped_permanents`, `dsl_cards/step/selections.rs`); orthogonal to the
existing cost `min`. BT24-051 now sets `clamp_to_available: true`. Hand/Trash zones thread the flag
but do not yet act on it (no card needs it; future extension).

Files: `code/digimon-dsl/src/{step,compiled,compile}.rs`,
`code/digimon-engine/src/dsl_cards/step/selections.rs`,
`code/digimon-engine/cards/bt24/BT24-051.yaml`. Tests: `mp30_*` / `mp31_*` in `rules_faq`.

**DCGO-verified (2026-06-03).** The fix matches the battle-tested DCGO behavior exactly:
`BT24_051.cs` uses `maxCount = Math.Min(2, available)` + `canEndNotMax: false` (must pick all
`min(2, available)`) — which is precisely `clamp_to_available: true`. DCGO's `canEndNotMax` is the
general distinguisher: `false` ⇒ `clamp_to_available: true`; `true` ⇒ the default "up to N"
(`optional_zero`). **Sibling sweep (implemented pool, mandatory "≥2 of your opponent's"):** only
BT24-051 was affected. ST5-15 Laser Eye is genuinely "up to 2" (`ST5_15.cs` `canEndNotMax: true`;
cards.json dropped the "up to") — its existing `optional_zero: true` authoring is correct, NOT a gap.

**Second sibling found + fixed (2026-06-03): BT12-028 Paildramon.** A DCGO-grounded sweep of the
whole implemented pool (DCGO `canEndNotMax:false` + `Math.Min(≥2,...)`, intersected with cards using
`select_count_capped_multi` on `zone: battle_area`) found one more instance of the identical bug:
BT12-028's "[When DNA Digivolving] **2** of your opponent's Digimon with no digivolution cards can't
attack" was authored `max: 2, optional_zero: false` with no clamp — letting the player lock only 1 of
2. Fixed with `clamp_to_available: true` (DCGO `BT12_028.cs` confirms `canEndNotMax: false`). Existing
BT12-028 behavioral tests (9) still pass. **Sweep otherwise clean:** BT24-040 / BT15-101 hand-roll N
mandatory `select_opponent_permanent` calls with `not_in_binding` dedup (faithful); ST5-12/ST5-15/
ST6-12 are genuinely "up to/may" (`optional_zero: true`); formula-`max` cards are "up to <X>".

## RESOLVED 2026-06-03 — `optional: true` on MANDATORY single-target selects (FAQ MP-29)

**Gap class (cousin of the MP-30/31 multi-target bug).** A select step authored `optional: true`
over-exposes an illegal *decline* (PASS) for an effect whose printed text is **mandatory** (no
"may"/"can"/"up to"). DCGO signal: `canEndNotMax: false` / `isOptional = -1`. Found by sweeping
implemented cards for `select_opponent_permanent { optional: true }` whose card text is a mandatory
"Suspend/Delete 1 of your opponent's Digimon", cross-checked against DCGO.

Two instances found + fixed:
- **BT21-037 Lighdramon** — "[When Digivolving] Suspend 1 of your opponent's Digimon. Then +2000 DP."
  The select was `optional: true` (the YAML comment even documented the intended `optional: false`).
  The author used `optional: true` to keep the unconditional DP buff firing when no target exists —
  but `install_select_opponent_permanent` already skips the tail on empty candidates, so that didn't
  even work, and it wrongly exposed a decline with a target present. **Fix:** guard select+suspend
  behind `if any_permanent(opponent, digimon)` (mandatory select only reached when a target exists),
  DP buff unconditional afterward. DCGO `isOptional=false`. Caught by `rules_faq::…::mp29_*`; both the
  with-target suspend test and the no-target DP-buff test stay green.
- **AD1-018 LordKnightmon** — "[Security] <De-Digivolve 1>, then **delete** 1 of your opponent's
  Digimon with play cost 3 or less." The delete select was `optional: true`; DCGO `AD1_018.cs`
  SecuritySkill uses `canEndNotMax: false`. **Fix:** removed `optional: true` (delete is the final
  step, so the empty-candidate no-op loses nothing).

Sweep otherwise clean: these were the only two implemented `select_opponent_permanent {optional:true}`
cards whose text is mandatory single-target.

## Alt-digivolve `from:` requiring ≥N sources carrying a trait  [G-DSL-DIGISOURCE-TRAIT-COUNT-GTE]  — OPEN 2026-06-05

Surfaced by **AD1-002 Aldamon** (judge-quiz Q4 authoring). The alt-digivolve line is
"[Digivolve] [Takuya Kanbara] w/ **2 or more [Hybrid] trait cards under**: Cost 3" — a digivolution
path whose `from:` base must additionally have **≥2 digivolution sources carrying the [Hybrid]
trait** beneath it.

- **What's missing (DSL):** an alt-path `from:` predicate that **counts** sources by trait. The
  closest existing leaf, `self_digivolution_sources_trait_has`, is (a) a ≥1 boolean presence check
  (no count threshold) and (b) a carrier/permanent-subject predicate, not an alt-path `from`-base
  predicate (alt-path `from` constrains the base being digivolved *from*, not the resulting stack's
  source multiset). Neither `materials_count_gte` (whole-stack count, trait-agnostic) nor
  `trait_has`/`trait_contains` (subject-trait match, no count) expresses "≥2 sources with trait T".
- **Impact:** AD1-002's alt-path enforces only the [Takuya Kanbara] base name; the "≥2 [Hybrid]
  sources" qualifier is inexpressible and is omitted with an explicit YAML comment. The **standard**
  Lv4/Red/Cost-3 digivolution (from `cards.json` `evo_costs`) keeps Aldamon reachable/attackable, so
  the judge-quiz Q4 pin (which only needs Aldamon on the field) is unaffected. **No-approximations
  note:** the alt-path is left UNDER-constrained on a cost-reduction line — a player could reach
  Cost 3 from a [Takuya Kanbara] base without the 2 Hybrid sources. Acceptable only because it is a
  rarely-reachable alt-cost and is flagged; close before relying on AD1-002 in deck legality.
- **Audit hazard discovered alongside:** the predicate spec struct does **not** set
  `deny_unknown_fields`, so a made-up key (e.g. `digivolution_trait_count_gte:`) parses **silently as
  a no-op** rather than erroring — worth a lint / `deny_unknown_fields` sweep so accidental
  under-constraint surfaces at load time.
- **Suggested fix:** add a `digisource_trait_count` formula/predicate leaf usable in alt-path `from:`
  filters — `{ digisource_trait_count: { trait: Hybrid }, gte: 2 }` — counting the base's
  digivolution sources whose trait set matches (exact via `trait_has` / substring via
  `trait_contains`). Threads like the EX3-014 `source_stack_count` selector but as a `from`-base
  predicate.
- **Blocks:** AD1-002 (alt-digivolve line only). YAML: `code/digimon-engine/cards/ad1/AD1-002.yaml`
  (comment marks the omission); per-card tests `code/digimon-engine/tests/cards_behavioral/ad1/ad1_002.rs`.

## "When an effect trashes this card from your security stack" carrier trigger  [G-DSL-ON-DISCARD-SECURITY-TRIGGER]  — OPEN 2026-06-06

Surfaced by **BT15-037 Gatomon** (judge-quiz Q9 authoring). Card text: "When an
effect trashes this card from the security stack, you may play it without paying
the cost."

- **What's missing (DSL):** there is no `when:` trigger token for "this card was
  trashed/discarded from the security stack by an effect" — DCGO
  `EffectTiming.OnDiscardSecurity` + `CanTriggerOnTrashSelfSecurity(.., cardEffect
  != null, card)`. The DSL has `on_security`, `on_own_security_removed`,
  `on_opponent_security_removed`, `on_check_face_up_security`, `on_lose_security`
  — none fire for the *card itself being discarded from security by an effect* with
  a follow-on "play this card free" body.
- **Impact:** Gatomon's "play this when trashed from your security" clause is
  omitted (flagged in the YAML header, no stub). The other 3 clauses (`<Barrier>`
  face + inherited, `[All Turns][OPT]` gain-memory) are implemented. Does NOT
  affect the Q9 ruling: Gatomon playing out *after* Mastemon's trim adds no
  security-removal memory (the removals already happened while it was in security).
- **Suggested fix:** add an `on_discard_security` (or `on_self_trashed_from_security`)
  carrier trigger token gated on effect-initiated discard of the carrier from its
  own security, exposing the carrier as `event_card` so a `play_from_security`-style
  free-play body can consume it. Likely shared by other "when trashed from security,
  you may play it" Digimon.
- **Blocks:** BT15-037 (the play-from-security-when-trashed clause). YAML:
  `code/digimon-engine/cards/bt15/BT15-037.yaml`; per-card tests
  `code/digimon-engine/tests/cards_behavioral/bt15/bt15_037.rs`.

## RESOLVED 2026-06-10 — controller-relative memory predicate  [G-DSL-OWN-MEMORY-PREDICATE]

Surfaced: judge-quiz Q15 authoring (EX8-073 / BT17-016 memory-gated immunities).

- **Card text shape:** "While **you** have 0 or less memory, this Digimon isn't affected by …" — the bound is on the CARD CONTROLLER's signed memory, but `memory_lte`/`memory_gte` compare the raw turn-player-perspective gauge, which cannot express a controller-relative bound for the non-turn player.
- **Resolution:** new predicate leaves `own_memory_lte` / `own_memory_gte` — evaluate the controller's signed memory (`game.memory` when it is the controller's turn, `-game.memory` otherwise). Spec `digimon-dsl/src/predicate.rs` → compiled (`compiled.rs`) → compile copy-through → engine eval (`dsl_cards/predicate.rs`).
- **Consumers:** EX8-073 Gallantmon (X Antibody) `[All Turns]` immunity, BT17-016 Gallantmon `[Your Turn]` immunity (both `active_when` gates on continuous auras).

## RESOLVED 2026-06-10 — continuous effect-immunity aura payload  [G-DSL-AURA-EFFECT-IMMUNITY]

Surfaced: judge-quiz Q15 authoring (EX8-073's stub header listed "memory aura immunity" as a gap).

- **Card text shape:** "[All Turns] While …, this Digimon isn't affected by [your opponent's] [Digimon] effects" — a CONTINUOUS immunity (DCGO `CanNotAffectedClass` with a `CanUseCondition`), not the one-shot expiry-bound `grant_effect_immunity` step.
- **Resolution:** new `kind: aura` body slot `effect_immunity: { source_kind?: digimon|tamer|option|rule, source_controller: any|opponent|own }` (omit `source_kind` for all-kind immunity). Self-aura only (`target: {}`). Lowered on the declarative-tick path to a MATERIALIZED filtered `CannotBeAffected` install (`EffectContext::add_declarative_effect_immunity_modifier`), re-evaluated each tick under `active_when` — so the immunity turns on/off with its printed gate, including MID-De-Digivolve via the per-pop re-tick in `Game::de_digivolve_core` (judge-quiz Q15).
- **Consumers:** EX8-073 (opponent Digimon effects, `own_memory_lte: 0`), BT17-016 (all opponent effects, `your_turn` + `own_memory_lte: 0`).

## Result-log invisible across an `if:`-body park  [G-DSL-IF-BODY-PARK-RESULT-LOG]  — OPEN 2026-06-10 (pitfall)

Surfaced: judge-quiz Q15 authoring (BT17-016 first draft).

- **Symptom:** wrapping `select_* → delete_permanent` inside an `if: { condition: any_permanent…, then: […] }` and following the `if` with `if: { condition: { effect_deleted_any_opponent_digimon: false } … }` makes the deleted-tracker read FALSE NEGATIVE: the select inside the `if` body parks, `park_outer_tail` captures the clause's remaining steps with a CLONE of the bindings taken BEFORE the deletion is recorded, so the outer `effect_deleted_*` predicate never sees the result log written by the continuation.
- **Workaround (validated idiom, BT25-014):** keep `select_* + delete_permanent` at the TOP LEVEL of the process — an empty mandatory select is skipped silently and the result log stays on the single continuation chain. BT17-016 / BT12-016 / EX3-057 / EX8-073 all use this shape.
- **Fix shape (if ever needed):** share the result log via the `EffectContext`/game rather than per-continuation `Bindings` clones, or merge the continuation's result log into the parked outer-tail bindings at drain time.

## Q29 EX10 Bagra cluster — new gaps (2026-06-11, judge-quiz Q29 authoring)

### BT10-093 / EX10-056 — "when a card is placed under this permanent" trigger  [G-DSL-ON-CARD-PLACED-UNDER-TRIGGER]

- **Card text:** BT10-093 Yuu Amano "[All Turns][Once Per Turn] When a purple card is placed under this Tamer, <Draw 1> and gain 1 memory." / EX10-056 Bagramon's [All Turns] observer also fires when "effects place cards under" opponent Digimon/Tamers (that half omitted; the digivolve half is authored).
- **DCGO:** `BT10_093.cs` `CanTriggerOnAddDigivolutionCard(permanent == self, card has Purple)`.
- **Gap:** the DSL has `on_digivolution_card_trashed` (the REMOVAL direction) but no ADDITION-direction timing ("card placed under this/any permanent"). Fix shape: fire a `DigivolutionCardAdded` event from `push_under`/`place_as_bottom_source`/DigiXros commit sites, expose `when: on_card_placed_under` + host/event-card filters.
- **Consumers:** BT10-093 (clause 1, OMITTED), EX10-056 (observer's placed-under half, OMITTED).

### EX10-031 — would-leave triggered observer with stack access  [G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER]

- **Card text:** "[All Turns][Once Per Turn] When this Digimon would leave the battle area, you may play 1 play cost 4 or lower card from its digivolution cards without paying the cost."
- **DCGO:** `EX10_031.cs` plays the card from the still-intact stack in the WOULD-LEAVE window; the leave still happens (non-replacement).
- **Gap:** DSL would-leave lowering covers REPLACEMENTS (cancel/substitute) only; a triggered observer in that window that reads the carrier's digivolution cards has no vocabulary. OMITTED.

### EX10-056 — place an opponent PERMANENT as a digivolution source  [G-DSL-PLACE-PERMANENT-AS-SOURCE]

- **Card text:** "[On Play][When Digivolving] You may place 1 of your opponent's Digimon as any of their other Digimon's bottom digivolution card or under any of their Tamers."
- **Gap:** `place_as_bottom_source` moves CARDS; tucking a battle-area PERMANENT must move the whole stack with leave semantics, and the destination is OPPONENT-controlled (own-side selects only today). OMITTED.

### EX10-059 — blind opponent-hand pick + cross-player tuck  [G-DSL-BLIND-OPP-HAND-PLACE]

- **Card text:** "[On Play][When Digivolving] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers."
- **Gap:** no unrevealed/blind opponent-hand selection, and no cross-player tuck destination flow. Sentence 2 ("by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's TOP digivolution cards, delete 1 of their Digimon or Tamers with cards under it") additionally needs the pre-existing G-DSL-PLACE-AS-TOP-SOURCE (BT13-088). Both sentences OMITTED.

### EX10-059 — gain sources' [All Turns] effects  [G-DSL-GAIN-ALL-TURNS-FROM-SOURCES]

- **Card text:** "[All Turns] This Digimon gains all [All Turns] effects on all level 6 [Bagra Army] trait Digimon cards in its digivolution cards."
- **DCGO:** source-card effect adoption (reads the source CARDS' text boxes).
- **Gap:** no DSL/engine machinery grants a permanent the printed effects of its digivolution source cards. OMITTED.

> **Pre-attach outside the recipe — RESOLVED 2026-06-11 (judge-quiz Q29).**
> `preattach_digixros_material` previously *validated the card against the
> DigiXros recipe slots* (`try_pre_attach_material` → `resolve_material_origin`),
> silently dropping any pre-attach that matched no slot — which broke Yuu Amano
> (BT10-093): its would-play hook places arbitrary purple Digimon from under
> Tamers, none of which are `[Bagramon]`/`[DarkKnightmon]` recipe materials.
> DCGO parity (`SelectDigiXrosClass.AddDigivolutionCardInfos`) does not
> recipe-validate pre-attached cards. Fixed: `EffectContext::
> preattach_digixros_material` now falls back to the new slot-independent
> `DigiXrosTransaction::pre_attach_extra_material` (recipe_slot `None`), so
> the card joins the transaction with its cost delta and the pre-attached
> placement order. BT12-112 (whose pre-attach coincidentally matches its own
> recipe) keeps the slot-resolving path. Pinned by
> `judge_quiz::e_partition_digixros::q29_*`.

## RESOLVED 2026-06-12 — `on_any_link` board-wide link observer  [G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED]

**Status: RESOLVED 2026-06-12 (Appmon BT21 wave).**

Cards of the form "[Your Turn] When your Digimon get linked, …" (a Tamer or a
Digimon observing a link onto *any* of the controller's Digimon, not just
itself) had no DSL timing. The two extant OnLink timings both force a filter:
`when_linked` (self-filter `event_card == source_card`, requires `scope: linked`)
and `when_card_linked_to_this` (host self-filter `event_permanent ==
source_permanent`). Neither expresses a board-wide observer on a third party.

**Resolution:** added `when: on_any_link` (`Timing::OnAnyLink` →
`CompiledTiming::OnAnyLink` → `EffectTiming::OnLink` in `timing_map.rs`). It
lowers to `OnLink` with NO forced self/host filter — scope is gated entirely by
`active_when:` predicates that already read the Linked trigger payload:
`event_target_owner: you` (the link HOST's controller), `event_card_trait_has:`
(the just-linked card's traits), and `your_turn: true`. Pair with
`source_is_unsuspended:` + `activation_cost: { suspend_self: true }` (or a body
`unsuspend: { target: source }`) for the common "by suspending/unsuspending this"
cost. First production users (all green): BT21-084, BT21-101, P-217 (and BT21-009
family via the host-side timing). Same timing unblocks P-241, BT23-079, BT24-087,
BT25-075's observer sub-clause.

## RESOLVED 2026-06-13 — `app_fuse` step (effect-initiated App Fuse)  [G-DSL-APP-FUSE]
**Status: RESOLVED 2026-06-13.** New DSL step `app_fuse: { from: hand|trash, result_filter?, optional }` for the printed "1 of your Digimon may app fuse into a Digimon card in the hand/trash" rider. Lowers to `CompiledStep::AppFuse` → `EffectContext::initiate_effect_app_fuse`. Added to `body_first_step_is_declinable` (installs its own PASS-able selections). First users: BT21-084, BT23-079, P-241, BT24-087, BT25-089. See `docs/RUST_ENGINE_GAPS.md` "Effect-initiated App Fuse — RESOLVED 2026-06-13".


## G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH — continuous mass "treat as a <DP> Digimon" aura with a synth identity (DATA SQUAD)
- **Card(s):** BT25-104 ShineGreymon: Burst Mode (Option face), clause "[Your Turn] All of your [Marcus Damon]s are also treated as 12000 DP Digimon and gain <Rush>". Generalizes to any "treat your Tamer(s) as a Digimon with DP X" continuous effect.
- **Status:** RESOLVED 2026-06-18. BT25-104 now ships FULLY IMPLEMENTED (the [Your Turn] aura is green — `bt25_104_your_turn_marcus_treated_as_12000_digimon_with_rush`). The substrate was widened along BOTH paths that previously dropped the payload.
- **What was done:**
  - **Declarative `kind: aura` path (the one BT25-104 uses):** added a `synth_identity` axis to `AuraBody` (`code/digimon-dsl/src/clause.rs`) → `CompiledDeclarativeClause::Aura.synth_identity` (`compiled.rs`) → compiled via `compile_synth_identity` (`compile.rs`) → threaded through `lower_aura::lower_all`/`lower` to the filter-install site, which now calls `add_declarative_modifier_with_payload` with the `ModifierPayload::SynthIdentity` (built by the now-`pub(crate)` `build_synth_payload`). Re-applied each tick over the live filter, so Marcus Damons played mid-turn are covered and it reverts at end of your turn. Authoring shape: `kind: aura` with `target: { of: you, kind: tamer, name_is: "Marcus Damon" }` (fold ownership into the FILTER — a `target_player: you` + filter combo routes to the wrong branch), `modifier: TreatAsDigimon`, `synth_identity: { dp: 12000 }`, `grant_keyword: { keyword: Rush }`.
  - **`add_modifier { continuous: true }` floating-mass path:** `FloatingMassModifier` gained a `payload` field threaded through `add_floating_mass_modifier` + the per-tick materialization in `game/triggers.rs` (symmetric fix to the latent drop; the lowering at `dsl_cards/step/modifiers.rs` now passes the computed payload instead of `None`).
  - `effective_dp` and the treat-as-Digimon machinery already read the synth DP (proven by the single-target cards BT13-020/BT21-044/AD1-021), so no combat/zone changes were needed.

## G-DSL-FIELD-SELECTOR-LOWEST-LEVEL — `selector: lowest_level` for select_* clauses (field selector, not just aggregate)
- **Card(s):** BT25-029 MirageGaogamon ("return 1 of your opponent's lowest level Digimon to the hand"); AD1-012 Omnimon Alter-S (same wording). 
- **Status:** RESOLVED 2026-06-18 — NOT actually needed (was a misdiagnosis). These cards are better served by the **AggregateSelector FILTER** path, which is already wired: `filter: { level_matches_aggregate: { selector: lowest_level, of: opponent } }` inside a `select_opponent_permanent`. That path is *more faithful* than a `FieldSelector` auto-pick: DCGO's `LowestLevelPermanentCondition` (`IsMinLevel`) is a target FILTER + a `SelectPermanent`, so the player chooses among tied lowest-level Digimon (rule 17), whereas a `FieldSelector { selector: lowest_level }` would auto-select the extreme and hide that choice. BT25-029 ships IMPLEMENTED on this path (8/8 green); AD1-012 should use the same shape. No new `FieldSelector` vocabulary required — the unevaluated `CompiledFieldSelector::LowestLevel/HighestLevel` can stay dormant until a card genuinely needs a field auto-pick by level (none known).
## OPEN 2026-06-14 — starter-deck (ST1-6) audit action-space-fidelity divergences  [G-AUDIT-ST1-6]
**Status: OPEN, deferred.** Surfaced by the `battle-test-starter-decks-st1-6` faithfulness re-audit (see `openspec/changes/battle-test-starter-decks-st1-6/notes/phase1-audit-findings.md`). All are minor action-space-fidelity divergences (no wrong outcomes / crashes / soft-locks); none block training-readiness.

1. **Suspend-target over-restriction (`is_unsuspended: true`)** — ST4-13 HerculesKabuterimon `<Digi-Burst 2>` suspend, ST4-15 Needle Spray suspend (and ~46 other cards repo-wide) filter the suspend target to `is_unsuspended: true`. DCGO (`ST4_13.cs`/`ST4_15.cs`: plain `IsPermanentExistsOnOpponentBattleAreaDigimon`) and rule 15-15-6-3 permit choosing ANY opponent Digimon, including an already-suspended one (the suspend is then a no-op). No new vocab needed — fix is to drop the filter — but it is a **cross-cutting bulk card-data change (~46 cards + action-space)**, out of scope for an ST1-6-only change. Best handled as its own change with a shared smoke/soft-lock test.

2. **ST2-15 Kaiser Nail — missing "playable-as-new-permanent" source filter predicate.** The card plays a selected digivolution-card source "as another Digimon". DCGO `ST2_15.cs` gates the source pick with `CanPlayAsNewPermanent(payCost:false)` (field not full / no play-lock); the YAML's `select_material { kind: digimon }` exposes any Digimon source and the play silently fizzles if unplayable. Genuine **DSL-vocab gap**: a source/card filter predicate meaning "can legally be played as a new permanent right now". Behavior converges (you can't play it either way); cosmetic for outcomes, real for RL action-mask fidelity.

3. **ST6-13 CresGarurumon — `<Digi-Burst 2>` over-gated activation.** YAML's `[Main]` `condition` requires a valid Lv3 purple Digimon already in trash before Digi-Burst can be activated; DCGO `ST6_13.cs` gates activation only on `CanDigiBurst()` (≥2 sources) and plays nothing if no target. Removing the trash-target gate would restore the (never-correct) "pay Digi-Burst with no play" line, but the mandatory inner `select_trash` would then need a skip-if-empty path to avoid a soft-lock. Deferred: current behavior is strictly safe and the removed line is never optimal play.

ST6-12 VenomMyotismon was flagged by the auditor (`optional_zero` vs DCGO force-≥1) but is a **false positive** — "up to N" permits 0 per rule 15-10-2-2 (PDF outranks DCGO's UI quirk), consistent with ST5-12/ST5-15 and the `reference_dsl_optional_mandatory_selection_pitfall` convention. No change.

## OPEN 2026-06-14 — Royal Knights re-audit pass: newly surfaced gaps

The 2026-06-14 Royal Knights audit/implementation pass closed ~17 cards whose
prior gap markers were stale. The following gaps remain genuinely open and were
surfaced or sharpened during the pass.

### `G-DSL-EVENT-CARD-TEXT-CONTAINS` — event predicate on the played card's effect TEXT
- **Consumer:** AD1-018 LordKnightmon ([All Turns][OPT] "When you play a Digimon with [Knightmon]/[Lucemon] in its text, <De-Digivolve 2>").
- **Missing:** DSL has `event_card_name_contains` and `event_card_trait_has` leaves, plus a static `effect_text_contains`, but no event-card leaf that matches the *played* card's effect text. The De-Digivolve-2 observer cannot gate on "in its text".
- **Suggested API:** add an `event_card_text_contains: "<substr>"` predicate leaf (sibling of `event_card_name_contains`) reading the played card's effect_description.
- **First test:** play a Digimon whose effect text contains "Lucemon" while AD1-018 is in play; assert the De-Digivolve-2 prompt fires; play one without it and assert no fire.

### `G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM` — return a selected digivolution source to deck bottom
- **Consumer:** BT13-075 Alphamon ([All Turns][OPT] would-leave self-protection: "by returning 1 [X Antibody]/[Royal Knight] card from this Digimon's digivolution cards to the BOTTOM OF YOUR DECK, it doesn't leave").
- **Missing:** the only source-return DSL verb is `return_selected_sources_to_hand` (to hand). No sibling returns a chosen digivolution source to the deck bottom; returning to hand would be an approximation (disallowed).
- **Suggested API:** `return_selected_sources_to_deck { position: bottom }` (or a `destination` param on the existing verb).
- **First test:** BT13-075 with X-Antibody/Royal-Knight sources, trigger a would-leave-by-effect; assert the pay-cost selection returns a chosen source to deck bottom and cancels the departure; decline path lets it leave.

### `G-PLAY-COST-GTE-MODIFIER-AURA` — continuous can't-attack-players aura keyed on opponent play cost ≥ N
- **Consumer:** BT13-075 Alphamon ([On Play][When Digivolving] "opponent's Digimon with play cost 10 or higher can't attack players until end of opp turn"); related BT20-021.
- **Missing:** a continuous, re-evaluated `CannotAttackPlayer` aura filtered by `play_cost_gte: N` that also covers opponent Digimon entering after resolution. Current snapshot/for_each modifiers don't cover late entrants. The clause is also atomic with a source-placement cost.
- **Suggested API:** an aura step with `target_filter: { play_cost_gte: N }` applying `CannotAttackPlayer` with `Expiry::end_of_opponents_turn`.
- **First test:** resolve BT13-075's On Play; a ≥10-cost opponent Digimon that enters AFTER resolution still can't attack players that turn; a <10-cost one can.

### `G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD` — deleted-permanent DP on the effect-deleted result payload
- **Consumer:** EX4-065 Trident Gaia ("If a Digimon with 13000 DP or more is deleted by this effect, trash opp's top security card").
- **Missing:** the effect-deleted result payload (`effect_deleted_any_opponent_digimon`) stores only PermanentHandles — no DP, and the carrier has moved to trash by the time the rider evaluates. No predicate exposes "the DP of the just-deleted permanent ≥ N".
- **Suggested API:** capture deleted-permanent DP (pre-removal snapshot, cf. rule 25) into the effect-deleted payload and add a `deleted_dp_gte: N` result predicate.
- **First test:** EX4-065 deletes a 13000-DP highest opponent Digimon → opp top security trashed; deletes a 12000-DP one → no trash.

### `G-FOR-EACH-COUNTED-FIELD-OBJECTS` — repeat an op N times where N counts over multiple field-object groups
- **Consumer:** BT13-030 UlforceVeedramon ([On Play][When Digivolving] "for each of your Royal Knights AND each of your blue Tamers, trash the top 2 digivolution cards of 1 opponent Digimon").
- **Missing:** an iteration count derived from the sum of two distinct own-field object groups (Royal-Knight Digimon + blue Tamers) driving N repetitions of a per-target trash-2-sources op.
- **Suggested API:** a `repeat: { count: <formula over multiple count_in_zone terms> }` wrapper, or extend `for_each` to accept a numeric repeat-count formula.
- **First test:** 2 Royal Knights + 1 blue Tamer → 3 iterations of "trash top 2 sources of a chosen opponent Digimon".

### `G-SOURCE-COUNT-SECURITY-TRASH` — trait-count-in-this-permanent's-sources formula
- **Consumer:** BT20-021 Jesmon GX ([When Attacking][OPT] "unsuspend self, then trash opp top security for every 2 [Royal Knight] cards in this Digimon's digivolution cards").
- **Missing:** no formula counts cards of a given trait among *this permanent's* digivolution sources (only `same_level_pairs_in_sources` exists). Need `trait_count_in_sources { trait: "Royal Knight" }` → floor-div 2 → N security trashes.
- **Suggested API:** a `trait_count_in_sources` formula term; drive `trash_top_security` repeated floor(count/2) times.
- **First test:** BT20-021 with 4 Royal-Knight sources → 2 security trashes; 3 sources → 1; 1 source → 0.


## RESOLVED / RECLASSIFIED 2026-06-15 — Royal Knights engine-gap closure pass

Adversarial scoping of the ~30 Royal-Knights-"blocking" gaps found that **14 were
not real gaps** (composable from shipped vocabulary today) and closed **6 genuine
small/medium gaps** via TDD. Net: only a handful of true RK gaps remain (the large
frameworks below).

### CLOSED this pass (TDD, consumer card now fully faithful)
- **G-DSL-EVENT-CARD-TEXT-CONTAINS** — new event-predicate leaf `event_card_text_contains` (played card's printed text). Consumer AD1-018. Commit `19be5a16`.
- **G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM** — new DSL verb `return_selected_sources_to_deck { position }`. Consumer BT13-075. Commit `a83d2827`.
- **G-RETURNED-CARD-COLOR-BINDING** — new predicate leaf `color_matches_returned_card` (reads the effect's returned-to-deck result log). Consumer EX10-068. Commit `78c84132`.
- **G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION** — engine fix: free digivolve-cost reducer auto-applies (no spurious accept/decline). Consumer ST12-15. Commit `b414917f`.
- **G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD** — effect-result log now carries each deleted permanent's pre-removal DP; new predicate `effect_deleted_opponent_digimon_dp_gte`. Consumer EX4-065. Commit `ba9afcee`.
- **G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY** — `select_union_zone` extended with a `material` zone (`material_of: { own_breeding }`) + per-zone filters. Consumer BT13-019. Commit `59eb5994`.

### RECLASSIFIED — NOT a gap (authorable-now with existing vocabulary)
Per-card scout verdicts (status `authorable-now-no-gap`) — these need CARD AUTHORING, not engine work. Do NOT re-file as engine/DSL gaps:
- **G-PLAY-COST-GTE-MODIFIER-AURA** (BT13-075) — continuous CannotAttackPlayer aura + `play_cost_gte` filter; authored in BT13-075 this pass.
- **G-DISTINCT-COLOR-COUNT** (EX10-068) — `distinct_colors_count` formula; authored in EX10-068 this pass.
- **G-FOR-EACH-COUNTED-FIELD-OBJECTS** (BT13-030) — repeat-count over summed field groups.
- **G-SOURCE-COUNT-SECURITY-TRASH** (BT20-021) — trait-count-in-sources formula already composable.
- **G-UNION-HAND-TRASH-SOURCE-COST** (BT20-021) — hand/trash place-as-source cost composable.
- **G-ALLY-PLAYED-OTHER-EVENT** (BT13-087) — `on_ally_played` + event filters compose it.
- **G-SECURITY-REMOVED-OBSERVER-UNIFIED** (BT20-056, BT20-060) — composable from the shipped on_own/on_opponent security-removed timings.
- **G-SUSPEND-OBSERVER-UNSUSPEND** (BT20-045) — any-suspend observer composable.
- **G-HIGHEST-DP-SWEEP** (BT20-045) — highest-DP aggregate sweep composable.
- **G-EFFECT-INITIATED-DIGIVOLVE-FROM-TRASH-ON-ATTACK** (EX11-069) — composable.
- **G-END-OF-ALL-TURNS-SUSPEND-COST-TRASH-RECURSION** (EX11-069) — composable.
- **G-EFFECT-RESULT-FALLBACK** (BT13-111) — composable.
- **G-COMBINED-TRASH-COUNT-COST** (BT13-111) — both-players-trash count formula composable.
- **G-SAME-LEVEL-X-DIGIVOLVE-OBSERVER** (BT9-092) — composable.
- **G-DSL-ON-DISCARD-SECURITY-TRIGGER** (BT15-084, BT15-092) — already CLOSED (shipped earlier).

### Still genuinely OPEN (deferred — large frameworks / not yet scoped)
- **G-BREEDING-DIGIVOLVE-UNION-ZONES** (BT20-056) — size L; attack-context breeding digivolve from hand/trash union.
- **G-UNION-HAND-SOURCE-PLAY** (EX11-053), **G-OPPONENT-PLAYED-DIGIMON-LEVEL-BRANCH** (RB1-035), **G-OWN-SECURITY-ADDED-OBSERVER** (BT8-090, likely authorable — re-verify), **G-SECURITY-END-OF-BATTLE-PLAY** (BT22-009), **G-ONDECLINE-CALLBACK** + **G-WAS-PLAYED-BY-EFFECT-OBSERVER** (BT13-102, engine), **G-OPTION-BATTLE-AREA-CARRIER** (BT19-093, engine, size L) — rate-limited out of the scoping pass; scope before authoring their cards.


## OPEN 2026-06-15 — Royal Knights final-3 residual gaps

After authoring all 16 remaining Royal Knights cards, exactly THREE cards retain
one clause each on a genuine residual gap (RK is now 69 IMPLEMENTED / 3 PARTIAL /
0 BLOCKED of 72). These are the only Royal-Knights-blocking gaps left.

### `G-BREEDING-DIGIVOLVE-UNION-ZONES` — attack-context breeding digivolve from hand/trash union
- **Consumer:** BT20-056 Alphamon. "[On Play][When Digivolving] then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a Lv.6-or-lower [Chronicle] Digimon in your hand OR trash, free."
- **Missing:** an effect-initiated digivolve where the DIGIVOLVING permanent is a breeding-area Digimon and the digivolve TARGET is sourced from a hand∪trash union, gated on an in-attack condition.
- **Suggested API:** extend the effect-digivolve step to accept a breeding-area subject + a `from: { zones: [hand, trash] }` union target with a `during_attack` condition.
- **First test:** BT20-056 in play attacking, a breeding Digimon present, a Lv.6 [Chronicle] in hand and one in trash → assert both are offered as free digivolve targets onto the breeding Digimon.

### `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN` — effect-play observer with opponent's-turn suspend cost
- **Consumer:** BT13-102 Keenan Crier. "[Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer, gain 1 memory."
- **Missing:** combine a `was_played_by_effect` observer (effect-plays only) firing on the OPPONENT's turn with a source-bound suspend activation cost. The On Play on-decline clause is authored; this observer remains.
- **First test:** opponent's turn, an effect plays a Digimon → assert an optional "suspend Keenan to gain 1 memory" prompt; a normal (non-effect) play does NOT fire it.

### `G-OPTION-PERSIST-AS-FIELD-CARRIER` (+ `G-OPTION-SELF-TRASH-TRIGGER`) — Option self-places/persists in the battle area
- **Consumer:** BT19-093 Queen Device. "[Main] … then, place this card in the battle area" (a persistent Option carrier), and "When this card is trashed from the battle area, …".
- **Missing:** an Option self-placing into the battle area as a persistent carrier, plus a `when_trashed_from_battle_area` trigger on that Option carrier. The color-bypass + [Main]/[Security] debuff clauses are authored; the self-place/persist + trash-from-battle trigger remain.
- **First test:** resolve BT19-093 [Main] → assert this Option is now a battle-area permanent; trash it from battle → assert the trash-from-battle clause fires.
## EX11-027 Maquinamon link substrate — RESOLVED 2026-06-20 (collapse-dsl-step-idioms §4.5)
The four EX11-027 link gaps are CLOSED and moved to [qa/resolved-gaps.md](resolved-gaps.md):
`G-DSL-LINK-RELINK-STANDING-PERMANENT` (the `relink_self_to_own_digimon` verb),
`G-DSL-LINK-HOST-FILTER` (`link_cards` `host_filter` + `exclude_source`),
`G-DSL-LINK-HETEROGENEOUS-CHOICE` (if-gated `select_effect_choice`, no new vocab), and
`G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE` (the `place_link_card_as_bottom_digivolution`
replacement cost). EX11-027 Maquinamon is now pure DSL (off test-only raw_rust); the
`dsl-substrate-integrity` loader guard is promoted to a hard error.

### `G-DSL-ALT-PATH-GATE-CONDITIONALS` — alt-digivolve `from:` predicate lacks board-state / compound / negative-colour gates
Surfaced by: the pool-wide alt-path authoring audit (promote-official-bandai-card-source, 2026-06-20).
The alt-digivolve `from:` predicate supports single level/colour/trait/name gates (+ `all_of`/`any_of`),
but four implemented cards print special-digivolution conditions it can't express, so those routes are
intentionally omitted (the cheaper standard/encoded routes ship; the conditional route does not):
- **Board-state conditional name gate** — BT23-013 ("[Huckmon] while opponent has a 10000 DP or higher
  Digimon: Cost 5") and BT15-101 (Tamer-presence + opponent-DP threshold). Needs a `from:` that can read
  game/board state (opponent field DP, own Tamer presence) at digivolve-eligibility time.
- **Compound multi-card gate** — EX11-074 ("While you have [Shoto Kazama], [GrandGalemon]: Cost 6"):
  a conjunction of a controller-has-named-card condition AND a base-name gate.
- **Negative tri-colour gate** — BT25-084 ("[Titamon] w/o 3 colors: Cost 2"): a name gate combined with
  a NEGATIVE colour-count condition (base has fewer than 3 colours).
These are tracked as allowlisted entries in `code/tests/test_alt_path_authoring_parity.py` (engine_gap
reason) so the authoring-parity guard stays green while documenting the omission.
- **Suggested API:** extend the alt-path `from:` predicate vocabulary with a board-state condition
  (`controller_has_named`, `opponent_has_dp_gte`) and a colour-count predicate (`color_count_lt`).
- **First test:** BT23-013 in hand over a Lv.5 base while the opponent has a 10000-DP Digimon → assert the
  cost-5 [Huckmon] route is offered; with no such opponent Digimon → not offered.

## Binding-scoped exact-name predicate (`binding_card_name_is`)  [G-DSL-BINDING-CARD-NAME-EQUALS]  — RESOLVED 2026-07-03
> **RESOLVED 2026-07-03 (leaves II):** `binding_card_name_is: {binding, name_is}` — effective-name (printed + also_treated_as) exact comparison. BT21-087's name-branch approximation can be replaced.
Consumer: BT21-087 Zenith ([On Play] reveal 3, choose 1 [Vemmon]-text card: if its name IS [Vemmon] → play-free-or-add choice, else add to hand). The `if:` branch needs to test the BOUND revealed card's exact printed name; the predicate surface has `binding_card_kind` (kind) and (filed 2026-07-02) `binding_card_color` (color) but no name analogue, so BT21-087 approximates via a nested re-select shape documented in its YAML header. Suggested: `binding_card_name_is: { binding: <name>, name: "<literal>" }` — sibling of `binding_card_kind`, resolving the named card binding and comparing the effective (synth-identity-aware) card name. Workaround shipped in BT21-087 is faithful for its single-pick flow but repeats per card; the leaf amortizes.


> **Interactive pay_cost delete reducers (fixed + variable) — RESOLVED 2026-07-03** (store-champs
> round 1). `kind: cost_reduction` supports an interactive `pay_cost` selecting+deleting an own
> permanent, static `amount:` (fixed) or omitted + `delete_for_cost_reduction` (variable = the
> deleted permanent's printed play cost, rule-25 snapshot into
> `Game::pending_cost_reduction_amount_override`). Clone-safe (resumable VM). Closes
> G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT and
> G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION (BT25-076 Ghoulmon shape now
> expressible). Drivers: BT18-073, BT13-083, BT13-103 (authored). Tests:
> tests/cost_hooks/pay_cost_play_delete_reducer.rs.

> **Memory-count formula (`player_memory`) — RESOLVED 2026-07-03** (store-champs leaves I). See
> G-DSL-MEMORY-COUNT-FORMULA in docs/RUST_ENGINE_GAPS.md. BT25-086's End-of-Turn DP scaling is
> now expressible via `dp_modifier_fn: {base: 0, per: {player_memory: {of: opponent}}, delta: 1000}`.


> **Option USE from revealed/sources/union origins — RESOLVED 2026-07-03** (option-verbs
> reconciliation onto the round-1 `Game::use_option_from` core). DSL verbs
> `use_option_from_revealed {of, card, cost?}`, `use_option_from_sources {of, card, cost?}`
> (Source origin resolves card_sources AND linked_cards), `use_option_bound {binding, cost?}`
> (select_union_zone hand-or-trash consumer). Drivers EX7-048 cl.1 / BT25-085 use-facet /
> BT21-062. Facet 2 (trash-Option-from-sources-as-COST) remains OPEN.


> **G-DSL-BATTLE-WINNER-BOARDWIDE — RESOLVED 2026-07-03** (trigger-timings round 2). DSL `when: on_ally_won_battle` (EffectTiming::EndOfBattle via TriggerSource::BattleResolved{winner}; tie = no winner, matching DCGO !WasTie) + predicate leaves `event_winner_owner` / `event_winner_trait_has`. Direct player attacks never fire it. Unblocks BT25-020 Marsmon + the Olympos XII win-a-battle line. Tests: tests/battle_winner_boardwide.rs (6).

> **G-DSL-LINK-TRASH-AS-COST — RESOLVED 2026-07-03** (link-economy round 2). Cost step `trash_link_card_of_own_digimon {of, optional}`: two RL-visible selections (which Digimon with >=1 link card, which of its link cards), trashes via trash_specific_link_card (fires OnLinkedCardTrashed), unpayable => clause aborts. Clone-safe (TrashLinkCardOfDigimonSelection resume frame + clone test). Unblocks BT25-073 Dragomon. Tests: tests/dsl/trash_link_card_of_own_digimon.rs (11 w/ formula tests).

> **G-DSL-FORMULA-OWN-LINK-CARD-COUNT (+ SourceLinkCardCount facet of G-DSL-LINK-N-CARDS-PER-HOST) — RESOLVED 2026-07-03** (link-economy round 2). PerSelector variants `own_link_card_count {of}` (board-wide sum) and `source_link_card_count` (per-host), usable in base_per_delta / de_digivolve amount_fn. Unblocks BT25-075 Vulcanusmon's mass De-Digivolve magnitude.

> **G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New step
> `place_self_under_permanent: { target: <binding>, face_down: <bool, default false> }` — dispatches to the
> already-shipped engine primitive `EffectContext::place_self_under_permanent` (claims the in-flight
> `pending_option` on the [Main] Option-play path, so the Option is seated FACE-UP under the chosen own
> permanent instead of trashed; a live field-Option source routes to `move_field_option_under_permanent`).
> Silently no-ops on an unset target binding (the preceding select self-skipped — DCGO's silent skip).
> Consumers: P-180 / EX7-070 / EX7-071 (the "Then, place this card as the bottom digivolution card of 1 of
> your [Three Musketeers] Digimon" [Main] tail — EX7-071 authored + green this round). Tests:
> tests/dsl/option_lifecycle_cluster.rs (gap1_dsl_*), tests/cards_behavioral/ex7/ex7_071.rs.

> **G-DSL-DNA-TRASH-PARTNER — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New step
> `effect_initiated_dna_digivolve_trash_partner: { target, trash_partner, from_hand, cost, ignore_requirements }`
> — the trash-material sibling of `effect_initiated_dna_digivolve_hand_partner`, lowering to the engine
> primitive `EffectContext::effect_initiated_dna_digivolve_trash_partner` (G-ENGINE-DNA-TRASH-MATERIAL,
> resolved 2026-07-03: trash material moves STRAIGHT into the merged stack, no [On Play]; DCGO
> CreateNewPermanent + jogress). `cost: printed` resolves via `printed_dna_cost_for_field_trash_pair`;
> recipe enforcement composes. Consumers: BT18-015 (authored + green this round), BT18-073 (same shape,
> still to author). Tests: tests/cards_behavioral/bt18/bt18_015.rs.

> **G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New
> no-subject predicate leaf `own_source_stack_color_count_gte: <N>` — distinct colors among the effect
> CARRIER's NON-FLIPPED digivolution sources (the shared `non_flipped_source_colors` extraction, same as
> `color_matches_own_source_stack`, so the gate and the Branch-A filter always agree; top card and
> face-down sources excluded; no carrier → fails closed). The YAML-reachable branch discriminant for
> EX9-074 Kimeramon's "if this Digimon has 6 or more colors in its digivolution cards, instead …" —
> authored as `if: { condition: { own_source_stack_color_count_gte: 6 }, then: [delete_one_per_opponent_color],
> else: [same-color single delete] }`. Consumer: EX9-074 (both branches authored + green this round).
> Tests: tests/dsl/kimeramon_color_mass_delete.rs (own_source_stack_* + yaml_branch_gate_*),
> tests/cards_behavioral/ex9/ex9_074.rs (SECTION 6).

# DSL Vocabulary Gaps Tracker

Resolved DSL gaps have been moved to [qa/resolved-gaps.md](resolved-gaps.md). This file tracks only open gaps and partial slices with remaining follow-up work.

This file accumulates `BLOCKED` verdicts whose `gap_kind` is `dsl` (the engine has the primitive but the DSL lacks a verb that lowers to it). Entries are appended by `/batch-implement-cards-rust-dsl`.

## EX10-020 — [Hand][Main] reduced-cost SELF-play + delete-at-EoT rider  [G-DSL-HAND-MAIN-SELF-PLAY-REDUCED]

Surfaced: 2026-06-10, judge-quiz Q3 authoring. EX10-020 Puppetmon PARTIAL (the Q3-relevant clauses are complete).

- **Card text:** "[Hand] [Main] If you don't have any Digimon other than Digimon with [Dark Masters] in their texts, you may play this card with the play cost reduced by 5. At turn end, delete the Digimon this effect played."
- **DCGO:** `EX10_020.cs` OnDeclaration — temp `ChangeCostClass` −5 on `UntilCalculateFixedCostEffect`, `PlayPermanentCards(self, payCost: true)`, then an `UntilOwnerTurnEndEffects` "[End of Your Turn] delete" attached to the played permanent.
- **Gap:** the `main_from_hand` timing exists, but every play verb plays a SELECTED card — there is no "play THIS CARD from hand, paying its cost with a delta" verb, and no rider to schedule the played permanent's EoT delete from the same activation. (`play_from_hand` + `cost_delta` exists for selected cards; the SELF-play form with pay-cost semantics is the missing piece.)
- **Consumers:** EX10-020 Puppetmon; the Q29 EX10 Bagra family shares the idiom (EX10-031 DarkKnightmon, EX10-056 Bagramon, EX10-059 DarknessBagramon) — land the verb with that cluster.

## EX10-020 — [Security] "if this card was face-up" gate  [G-DSL-SECURITY-WAS-FACE-UP-GATE]

Surfaced: 2026-06-10, judge-quiz Q3 authoring. EX10-020 Puppetmon PARTIAL.

- **Card text:** "[Security] If this card was face-up, you may play 1 level 5 or lower card with [Dark Masters] in its text from your hand or trash without paying the cost."
- **DCGO:** `EX10_020.cs` SecuritySkill gated `!CardEffectCommons.GetFaceDownFromHashtable(hashtable)` — the security card must have been FACE-UP when checked (e.g. placed face-up by its own [On Deletion]).
- **Gap:** the `on_security` trigger has no condition leaf exposing whether the checked security card was already face-up. Authoring the clause without the gate would over-fire on every normal (face-down) check — unfaithful, so the clause is OMITTED. Fix shape: thread the face-up bit from the security-check dispatch (`Player.face_up_security` membership at check time) into the trigger context + a `security_card_was_face_up: bool` condition leaf.
- **Body once unblocked:** `select_union_zone` (hand, trash) over `{ kind: digimon, level_lte: 5, effect_text_contains: "Dark Masters" }` + `play_union_bound_free` (BT25-094 idiom).

> **Substring trait predicate `trait_contains` — RESOLVED 2026-06-03**
> (EX3-014 Dorbickmon code-review fix). The DSL had only `trait_has`, an EXACT
> case-insensitive trait match (`x.eq_ignore_ascii_case(t)`). Printed text of the
> form "[Dragon], [saur] or [Ceratopsian] in **any of its traits**" is a SUBSTRING
> reading, matching DCGO `CardSource.HasDragonTraits` → `ContainsTraits("...")`
> (`DCGO/Assets/Scripts/Script/CardSource.cs`). Under exact match the `saur` clause
> was completely DEAD (no card carries a standalone "saur" trait — it only appears
> inside `Dinosaur` ×92 / `Ankylosaur` ×11 / `Plesiosaur` ×9), and `Dragon` mostly
> appears as a substring (`Dragonkin` ×92, `Dark Dragon` ×36, ...), so the EX3-014
> DP cap massively undercounted and `[Dinosaur]` Digimon could not be picked as
> DigiXros materials — a faithfulness + no-approximations violation. New leaf
> `trait_contains: <token>` is the substring sibling of `trait_has`: matches when
> ANY subject trait CONTAINS the token (case-insensitive,
> `subject_traits.iter().any(|x| x.to_lowercase().contains(&t.to_lowercase()))`).
> Threaded identically to `trait_has` — spec field
> (`digimon-dsl/src/predicate.rs`) → compiled field (`compiled.rs`) → lowering
> (`compile.rs`) → engine card-field eval AND synth-identity / `ChangeTraits`
> overlay path (`digimon-engine/src/dsl_cards/predicate.rs`), plus the
> `eval_no_subject_fields` subject-field guard. Works inside the
> `per: { source_stack_count: { filter } }` selector and DigiXros material filters
> (same `CompiledPredicate`). Unblocks the "[Dragon]/[saur]/[Ceratopsian]"-family
> matching. Pinned by `tests/cards_behavioral/ex3/ex3_014.rs` — esp.
> `ex3_014_dinosaur_source_counts_via_saur_substring` (the load-bearing `saur`
> substring proof). G-DSL-TRAIT-CONTAINS-SUBSTRING.

> **Trait-filtered carrier source count as a `per` selector — RESOLVED 2026-06-03**
> (EX3-014 Dorbickmon authoring). The `BasePerDelta` formula now accepts a new
> `per: { source_stack_count: { filter: <predicate> } }` selector that counts the
> effect carrier's own digivolution sources (the cards beneath its top card)
> matching a predicate. The engine already had the raw machinery (the top-level
> `source_stack_count` FormulaSpec + `eval_predicate_with_bindings`), but a raw
> count cannot be offset/scaled — there is no `add`/`mul` formula combinator. As a
> `per` selector it composes in `base + count * delta`, letting a card scale a
> numeric (here a DP cap) by the number of its sources matching a trait:
> Dorbickmon's "for each card with [Dragon], [saur] or [Ceratopsian] in any of its
> traits in this Digimon's digivolution cards, add 2000 to the maximum DP you can
> choose" → `dp_lte: { formula: { base: 3000, per: { source_stack_count: { filter:
> { any_of: [trait_has: Dragon, ...] } } }, delta: 2000 } }`. Spec
> `PerSelector::SourceStackCount(SourceStackCountSpec)` → compiled
> `CompiledPerSelector::SourceStackCountFiltered { filter }`; evaluated by
> `formula_eval::source_stack_count_filtered` (reads `ctx.source_permanent`). Pinned
> by `tests/cards_behavioral/ex3/ex3_014.rs` (scaling-cap behavioral tests).
> G-DSL-PER-SOURCE-STACK-COUNT-FILTERED.

> **`select_opponent_play_cost_budget.play_cost_budget` scalar → FormulaSpec — RESOLVED 2026-07-03**
> (P-094 Destromon authoring). The play-cost-budget multi-select step
> (`G-MULTI-SELECT-OPP-PLAY-COST-SUM`) previously took a plain `i32`
> `play_cost_budget`. Widened to `crate::formula::FormulaSpec`, mirroring the
> sibling `SelectOpponentDpBudgetArgs.dp_budget: FormulaSpec`. A bare integer YAML
> literal still parses (FormulaSpec's first untagged variant is `Literal(i32)`),
> so the existing scalar user EX4-073 is untouched (13/13 tests green). The formula
> is evaluated once at install time against the effect context (both the installer
> in `dsl_cards/step/selections.rs` and the replacement pre-check in
> `dsl_cards/lower_replacement.rs`), exactly like the DP path. Lets P-094 model
> "delete up to 3 play cost's total worth … for each [Vemmon] in this Digimon's
> digivolution cards add 1 to the maximum" →
> `play_cost_budget: { base: 3, per: { source_stack_count: { filter: { name_is:
> "Vemmon" } } }, delta: 1 }`. Pinned by `tests/cards_behavioral/p/p_094.rs`
> (baseline-3 + scaling-by-Vemmon behavioral tests). G-MULTI-SELECT-OPP-PLAY-COST-SUM.

> **`source_count` predicate leaf (filtered digivolution-source count ≥ N) — RESOLVED 2026-07-03**
> (P-094 Destromon authoring). New permanent-subject predicate leaf
> `source_count: { filter: <predicate>, at_least: N }` — true when the candidate
> carries ≥ N digivolution SOURCE cards (the cards beneath its top card) matching
> `filter`. Models the DCGO `DigivolutionCards.Count(predicate) >= N` idiom, which
> had no DSL expression (`materials_count_gte` counts ALL sources by raw stack
> length, not a name/trait-filtered subset). The nested `filter` is a full
> `PredicateSpec` evaluated per source card via `eval_card_fields` (source
> subject), so it accepts `name_is`/`name_contains`/`trait_has`/`kind`/etc.
> Threaded spec→compiled (`Option<(Box<CompiledPredicate>, u8)>`)→eval in BOTH the
> battle-area and breeding-area permanent evaluators (`dsl_cards/predicate.rs`),
> plus the permanent-only-leaf gates in `formula_eval.rs` +
> `lower_replacement.rs`, the validator recursion, and the pack raw-rust-fn walk.
> Gates P-094's inherited redirect: only a [Galacticmon] carrying ≥2 [Vemmon]
> sources is offered for the return-2-Vemmon cost. Pinned by
> `tests/cards_behavioral/p/p_094.rs` (`inherited_no_fire_without_galacticmon_
> carrying_two_vemmon`). G-DSL-SOURCE-COUNT-FILTERED.

> **`TreatAsDigimon` / `SynthIdentity` payload — RESOLVED 2026-05-30** (judge-quiz
> cluster-B authoring, Greymon/Marcus line). The DSL `add_modifier` step now accepts
> a structured `synth_identity:` block (`dp` required; `kind` defaults Digimon;
> `level`/`colors`/`traits` optional), lowering to the engine's pre-existing
> `ModifierPayload::SynthIdentity` via a new `EffectContext::add_modifier_with_payload`.
> This closes the Track C "rich payload parser pending" slice for the
> treat-a-Tamer-as-a-Digimon mechanic (RizeGreymon BT21-044's 3000 DP grant,
> ShineGreymon: Burst Mode BT13-020's 12000 DP grant). The validator requires
> `synth_identity` for `TreatAsDigimon` and forbids it on any other modifier.
> Pinned by `digimon-dsl` `parse_synth_identity` (3) + `validator::tests`
> `treat_as_digimon_without_synth_identity_is_rejected` /
> `synth_identity_on_non_treat_as_digimon_is_rejected`. The remaining Track C
> string/list payload variants (non-TreatAsDigimon) stay pending — see that note below.

> **ST-2 Cocytus Blue substrate closure — 2026-05-29:** ST2 introduced no
> remaining open DSL vocabulary gap. The new `trash_bottom_sources` step and
> `battle_opponent_no_sources` predicate are implemented and archived in
> `qa/resolved-gaps.md`; Kaiser Nail is covered by existing
> `select_material` / `play_from_materials`. Do not file ST2 bottom-source
> cards under `select_opponent_sources`: those printed effects choose the
> Digimon only, then deterministically trash the bottom source(s).

> **ST5 Machine Black closure — 2026-05-29:** `digimon_attacked_this_turn:
> you|opponent` is now a closed DSL predicate leaf, backed by engine attack
> history and consumed by ST5-04/ST5-06 inherited draw clauses. ST5-14 Tai
> Kamiya's Blocker response was expressible with the existing
> `on_attack_target_change` / `attack_target_change_reason: blocker` context
> after the blocker declaration path was corrected to suspend the blocker before
> target-change observers run. No open DSL vocabulary gap remains for ST5; full
> closure details live in [qa/resolved-gaps.md](resolved-gaps.md).

> **TS Olympos representative unlock — 2026-05-24:** The
> `close-ts-olympos-rust-gaps` change added and consumed the DSL surfaces
> needed for the representative TS Olympos deck: `materials_count_matches_aggregate`,
> `de_digivolve.amount_fn`, predicate-scoped timing suppression,
> effect-driven `use_option_from_hand`, `face_up_security_count_lte/gte`,
> and `add_bottom_security_to_hand`. These are closed for the
> representative deck and should not be re-filed as open DSL vocabulary
> gaps unless a future broad-pool card proves a distinct missing variant.

> **Xros Heart DigiXros closure — 2026-05-24:** The
> `close-xros-heart-digixros-gaps` change adds production DSL vocabulary and
> lowering for `kind: digixros` recipe paths, material zones, per-material
> cost deltas, transaction-local zone allowances, pre-attached materials,
> one-shot transaction cost deltas, and Material Save lowering from a
> DigiXros recipe. BT10-009, BT10-013, BT10-087, and BT12-112 now ship as
> pure production YAML. Remaining Xros Heart DSL work should be tracked as
> card-specific follow-up, such as BT10-111's turn-scoped DigiXros wildcard
> modifier, rather than a generic DigiXros/Material Save vocabulary gap.

> **Xros Heart reusable primitive closure — 2026-05-24:** The
> `author-xros-heart-reusable-primitives` change adds production DSL
> vocabulary and lowering for selecting cards under Tamers, placing
> hand/trash/union-zone cards under Tamers, playing selected under-Tamer
> cards for free or at reduced cost, moving filtered source cards under
> Tamers with moved-count bindings, top-N opponent stack trashing,
> sourceless-target filters, scoped DigiXros wildcard substitution, and
> effect-created attack prompts. BT21-083, BT11-095, P-224, BT19-090,
> BT21-092, BT10-111, BT21-027, and BT19-061 now ship as production YAML
> acceptance fixtures. These shapes are no longer open Xros Heart DSL
> vocabulary gaps.

> **Xros Heart reveal-play slice — 2026-05-24:** `choose_from_reveal`
> now accepts `destination: play_free`, lowering to
> `EffectContext::play_from_reveal_free` after the existing reveal pending
> selection. The selected revealed card is played without paying its cost, and
> cancellation/failed would-play replacement restores it to the reveal pool.
> `BT19-008` now uses this pure YAML route for its On Deletion reveal/play
> clause.

> **Xros Heart stack-metric and lockout slice — 2026-05-24:** The
> `complete-xros-heart-authoring-substrate` change adds DSL formula/lowering
> support for `source_color_count` both as `{ formula: { source_color_count:
> {} } }` and as `per: source_color_count` inside base/per/delta formulas,
> plus `source_stack_count` for count bounds and memory/DP math over
> predicate-matched source cards. The same slice covers permanent-scoped
> temporary lockout modifiers for `CannotActivateOnPlayEffects`,
> `CannotActivateWhenDigivolvingEffects`, and `CannotUnsuspend` with explicit
> expiry. These shapes cover the BT19-014, AD1-006, AD1-013, BT19-026,
> BT21-030, BT19-038, BT19-051, BT19-035, BT20-037, and BT19-079 fixture set
> and are no longer open Xros Heart DSL vocabulary gaps.

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. The Track E zone-movement DSL verb table (below) is
> current as of PR #454. The Track C modifier-payload schema gap is
> still the principal open DSL item; the matching engine substrate
> landed in PR #455 (typed `ModifierPayload`), so the remaining work
> is structured payload schema + parser. The `OnSuspend` /
> `name-filtered DelayTrigger` shape (BT24-089) and the bilateral
> player-scoped passive modifier shape (Rocks) remain open. See
> `docs/RUST_ENGINE_GAPS.md` for the canonical engine-side closures
> driving DSL substrate. Pre-scaling cleanup batch §2 narrative in
> `.claude/plans/pre-scaling-cleanup-batch.md`.

> **Tracker hygiene sweep — 2026-05-15:** Post-rebaseline audit cleanup
> per [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](../docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md).
> The canonical engine-side tracker `docs/RUST_ENGINE_GAPS.md` was
> compacted: ~54 closed entries moved to `qa/resolved-gaps.md`, ~10
> entries had headline severity reframed from 🔴 to 🟡 PARTIAL with
> narrowed residual titles (e.g. "Decode residual: EX10-061 Apocalymon
> batch + different-name source play DSL sugar", "Conditional
> security-in-stack trigger residual: start-of-turn variants"), and
> the at-a-glance table was rewritten. No new DSL-vocab gap entries
> surfaced; existing entries in this file remain accurate.
>
> **Tracker hygiene sweep — 2026-05-14:** Cross-referenced against PRs
> #459–#473. The Track E zone-movement DSL verb table remains current
> (no new entries). Track C modifier-payload structured YAML parser is
> still the principal open DSL item — engine substrate from PR #455
> is unchanged, parser work has not landed yet. New since 2026-05-10:
>
> - **Track H aura DSL (PR #467):** `grant_triggered_effect` step
>   (target / timing / expiry / body), `kind: aura` materialization
>   for battle-area and security-zone scopes, plus the typed
>   `AuraScope` / `AuraGrant` builder all lower through existing DSL
>   schema. Card authoring for EX1-068 Ice Wall! and BT21-095 Wind
>   Guardians is now pure DSL; no new vocabulary gap surfaced.
> - **Alter-S Ladder DSL (PR #468):** EX9-021 Omnimon Alter-S and DNA
>   Omnimon ladder cards land using existing zone-movement /
>   replacement / source-selection verbs. No new DSL verb required.
> - **Formula-threshold DSL (PR #470):** `play_cost_lte` /
>   `binding_play_cost` / `distinct_colors_count` formula leaves
>   activated for BT15-096 and BT21-102. The shape is shared with
>   level/DP/material/memory/security aggregate predicate leaves —
>   see the "Track J formula/result substrate slice (2026-05-10)"
>   paragraph in `docs/RUST_ENGINE_GAPS.md`.
> - **Puppet observer DSL (PR #472):** predicate leaves
>   `event_target_kind`, `event_target_trait_has`,
>   `event_permanent_is_source`, and `source_is_unsuspended` are
>   wired through existing lowering paths. PUPPETS-G011 closed.
>
> The `OnSuspend` / name-filtered DelayTrigger shape (BT24-089) and
> bilateral player-scoped passive modifier shape (Rocks) remain open
> from the 2026-05-10 sweep.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 rollup — Tracks A–J, PR #480):**
> The Phase 2 pilot-archetype unblock work landed as 10 tracks in PR #480
> (`claude/musing-ishizaka-c4b355` against `main`). All sub-entries below
> have been swept; the per-track sweep paragraphs that follow cover Tracks
> F and G in detail. Closure pointers for the other tracks:
>
> - **Track A** — DSL eval-arm sweep (commit `b91816b5`). Closes
>   `G-PRED-DP-LTE` (card-zone subjects via `eval_card_fields`),
>   `G-COUNT-GTE-NOT-EVALUATED`, `G-FORMULA-SOURCE-DP`,
>   `G-DSL-DISTINCT-TAMER-COLORS` + `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`
>   (paired BoolPredicate + formula), and `G-ALT-PATH-CONDITION` stale
>   placeholder-test sweep. 13+ tests un-ignored. Full entry in
>   `qa/resolved-gaps.md` § "Phase 2 Track A closure".
> - **Track B** — `activation_cost(...)` builder (commit `2c2c4632`).
>   Engine substrate: `EffectBuilder::activation_cost`,
>   `ctx.suspend_self_as_cost()`,
>   `ctx.return_self_to_deck_bottom_as_cost()`. DSL:
>   `CompiledStep::ActivationCost { kind: SuspendSelf |
>   ReturnSelfToDeckBottom }`. Cost-failure consumes OPT slot per Working
>   Rule §17. Downstream consumers: BT4-097, BT8-090, ST6-14, BT8-094,
>   EX9-068, BT13-102, RB1-035, P-136, BT22-094, BT17-093, EX11-071.
>   Full entry in `qa/resolved-gaps.md` § "Phase 2 Track B closure".
> - **Track C** — G-OPT-TRIGGERED (commit `dd9b8a46`). Substrate was
>   already correct; the gap proved phantom. 23 stale `#[ignore]`
>   annotations removed; slot-key semantics documented (per-carrier
>   HashMap keyed by `(source_card_handle, effect_slot)`, fully cleared
>   on `Permanent::new_turn`). G-OPT-RESET-VIA-ATTACK-CYCLE was a
>   test-setup misdiagnosis (deck-out before second turn cycle); fixed
>   in test files only.
> - **Track D** — G-INHERITED-DISPATCH (commit `bc852640`).
>   `enqueue_from_permanent` now walks `permanent.card_sources` so
>   inherited triggered effects fire from below-the-top cards. Stable
>   slot keying via the existing `(source_card_handle, effect_slot)`
>   shape — no OPT collision. `G-WHEN-DIGIVOLVING-DISPATCH` absorbed.
>   Regression test in `tests/timing_dispatch.rs`. 18 tests un-ignored.
> - **Track E** — Rocks pilot reveal-ordering (commit `bac197ea`).
>   Detailed in "Phase 2 Track E (2026-05-17)" §149 below.
> - **Track H** — BG Imperial substrate (commit `2b083c5a`). Closes
>   `G-BEFORE-PAY-COST-DIGIVOLVE-TARGET` (cost-target predicate +
>   `source_is_cost_target_permanent`), `G-BEFORE-PAY-COST-GAIN-MEMORY`
>   (`Effect::before_pay_cost_observe` sibling builder),
>   `G-OPTIONAL-SELECTION-CONTINUE-TAIL` (select_trash slice only —
>   other `select_*` installers remain follow-up), `G-PLAY-FROM-HAND-FREE-BIND-AS`.
>   **DEFERRED:** `G-COST-REDUCE-ALLY-DIGIVOLVE` (per Track H's discovery
>   rider; entangled with armed-observer + suspend-cost sub-gaps).
> - **Track I** — Puppets pilot (commit `26e27ccc`). Closes
>   `PUPPETS-G008` / `G-OPPONENT-SECURITY-DP-AURA` (inherited aura with
>   `applies_to_opponent_security_dp`), `PUPPETS-G009` (Delay [Main]
>   action), `PUPPETS-G003` (ProvenanceToken cleanup), end-of-attack
>   mandatory self-delete chain. EX4-074 ShineGreymon: Ruin Mode
>   IMPLEMENTED.
> - **Track J** — Royal Knights substrate + cards (commits `48fbfd76` +
>   `3a6aaee1`). Closes `RK-G001` (filtered breeding permanent target),
>   `RK-G002` (source-bound return-self cost into reduced-cost hand
>   play — leverages Track B's `activation_cost`), `RK-G003` (Delay/
>   keyword leave-prevention replacements). Plus token registry entries
>   for Atho / Rene / Por.
>
> **Net cumulative test deltas (vs. pre-wave-1 baseline post-PR #475):**
> `cards_behavioral` 2355 pass / 0 fail / 355 ignored — was ~2300 / 1
> pre-existing flake / 596 ignored. **Phase 2 killed the long-standing
> `ex11_054` Medusamon flake** as part of Track G's `[All Turns]`
> entering-permanent observer migration.
>
> See `qa/resolved-gaps.md` for full per-track closure details. The
> entries below that match these closure tags have been annotated
> inline with "RESOLVED 2026-05-17 (Phase 2 Track X)" pointers; legacy
> entry bodies are preserved for reference but the heading line carries
> the closure stamp.

> **Tracker hygiene sweep — 2026-05-20 (Puppets substrate sweep):** 15
> reusable substrate gaps closed on branch `claude/stoic-moser-0ef79e`.
> DSL-vocab entries closed in this file: `PUPPETS-G023` (BT13-101/P-136
> event-card color predicates), `PUPPETS-G024` (BT16-055 narrow
> opponent-effect protection), `PUPPETS-G025` (BT16-055 rules_text_contains
> predicate), `PUPPETS-G028` (BT22-088 return-self-to-deck-bottom cost +
> branch), `PUPPETS-G030` (BT5-106 suppress_on_play flag). All four entry
> headings below carry inline RESOLVED stamps; legacy bodies preserved for
> reference. See `docs/RUST_ENGINE_GAPS.md` and `qa/resolved-gaps.md` for
> engine-side closures.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 Track F):** Five DNA
> Omnimon DSL/substrate gaps closed; full closure summaries in
> [resolved-gaps.md](resolved-gaps.md) under "Phase 2 Track F closure":
>
> - `G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM` — new deterministic verb;
>   BT23-008 / BT23-018 production YAML authored.
> - `G-DSL-GAIN-MEMORY-FN` — formula-valued memory mutation step.
> - `G-DSL-HAS-ON-DELETION-EFFECT` — new permanent predicate
>   consulting `effects_for_card` for `OnDeletion` timing. EX1-021
>   both clauses authored.
> - `G-ALT-PATH-DIRECTION-INTO` — `AltPathSpec.direction: into` schema
>   extension + route-resolution threading. Substrate only; ST20-10
>   warp YAML pending its companion `G-DSL-DISTINCT-TAMER-COLORS`
>   predicate leaf.
> - `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` —
>   resolved as **phantom**; the chain dispatcher already worked.
>   5 tests (BT16-040 / BT17-015 / BT17-027 / BT22-013 / BT22-026)
>   un-ignored.
>
> Plus `G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA` verified as
> already-shipped upstream; regression coverage added + P-182
> [All Turns] aura authored.
>
> Still open from Track F's plan: `G-DSL-INHERITED-SUBSTITUTE-RETURN-TRASH`
> (deferred — entangled with `G-SELECT-MULTI-MIN` and
> `G-ZONE-TRASH-TO-DECK` sub-gaps). EX5-015 Clause C remains BLOCKED.

> **Tracker hygiene sweep — 2026-05-17 (Phase 2 Track G):** Medusamon
> pilot completion. One new DSL predicate substrate ships
> (`G-OPP-SECURITY-COUNT-LTE`), closure summary in
> [resolved-gaps.md](resolved-gaps.md) under "Phase 2 Track G closure".
> Five Track G plan-named DSL gaps (G-EVENT-TARGET-OWNER,
> G-PLACE-SELF-AS-OPTION-PERMANENT, G-ADD-OPTION-SELF-TO-HAND,
> G-DSL-LINK-VERB, G-DSL-LINKED-SCOPE, G-MAY-ATTACK-NOW) were already
> resolved by earlier upstream substrate work; Track G's role for those
> was the test-tree sweep — stale `#[ignore]` annotations retagged from
> "BLOCKED: G-XYZ" to "card-local body not authored; substrate closed"
> across BT21-024 / BT21-025 / BT21-026 / BT21-029 / BT24-016 /
> BT24-082 / LM-055 / EX11-054. The BT21-026 deletion arm migrated to
> live YAML using `event_target_owner: opponent`; the BT21-093
> cost-reduction clause migrated from a `count_lte` aggregate over
> opponent security to the new native `opponent_security_count_lte`
> predicate (raw_rust formula `bt21_093_cost_reduction_amount`
> removed); EX11-054 [All Turns] clause migrated to Track B's
> `activation_cost: { suspend_self: true }` so the suspend-as-cost
> semantics gate the body correctly per the engine's single-trigger
> drainer model. **12 Medusamon cards advanced PARTIAL → IMPLEMENTED.**
>
> Still open from Track G's plan: G-AURA-DP-FORMULA (BT21-072 formula
> AuraBody DP), G-DELAY-SUSPEND-CONDITION (BT24-089 OnSuspend Delay),
> G-ZONE-TRASH-TO-DECK (BT24-017 trash-to-deck verb), G-AS-SELECTING-PLAYER
> (BT24-016 cross-permanent select-on-behalf), G-PRED-DP-LTE-AGGREGATE
> (BT21-093 highest-DP delete).

## Track C modifier payload YAML shape (2026-05-09) — rich payload parser pending

The Rust engine now has typed `ModifierPayload` storage and consult sites for
the deferred Track C identity/metadata modifiers:
`ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`,
`ChangeCardNamesForDigiXros`, `TreatAsDigimon`, `ChangePermanentLevel`,
`ChangeCardDP`, `ChangeOriginDP`, `ChangeSAttack`,
`ChangeEndTurnMinMemory`, `ChangeLinkCost`, and `ChangeLinkMax`. The scalar
`add_modifier` / `add_player_modifier` DSL slots can still install variants
that are representable as `value: i32`, and the modifier-name tables include
`CannotPlayFromTrash` and `OpponentCannotReduceDigivolveCost`.

Remaining DSL work: add a structured payload schema for list/string/profile
modifiers, e.g.:

```yaml
- add_modifier:
    target: source
    modifier: ChangeTraits
    payload: { add: [Holy], replace: false }
    expiry: until_leave_field
- add_modifier:
    target: source
    modifier: TreatAsDigimon
    payload:
      level: 4
      colors: [Yellow]
      traits: [Holy]
      dp: 5000
    expiry: until_leave_field
```

Until that parser lands, cards needing string/list/profile payloads should use
`raw_rust` install hooks rather than hidden scalar encodings.

## Phase 2 Track E (2026-05-17) — reveal-ordering DSL verbs landed

The author-facing residual from `G-ROCKS-REVEAL-ORDERING` has landed: two
new DSL verbs lower onto the already-shipped `select_reveal` /
`select_effect_choice` / `select_ordered_permutation` / `place_remainder_on_deck`
engine helpers. Together with the existing `reveal_top_deck` they express
the canonical "reveal N, choose 1 to hand or as source, place rest top or
bottom in any order" pattern that recurs across Rocks search effects and
every general-purpose Training / Memory Boost / search clause.

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl track_e_reveal_ordering`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_167 ex8_047 bt9_103`

| DSL verb | Engine target | Card drivers |
|---|---|---|
| `choose_from_reveal: { of, filter, destination, bind_as?, optional?, prompt }` | `EffectContext::select_reveal` + routing to `add_to_hand_from_reveal` / `return_to_deck_from_reveal` / `place_as_bottom_source` / `play_from_reveal_free` | P-167 (hand and `bottom_source_of`), EX8-047 (two sequential hand picks), BT19-008 (`play_free`) |
| `order_remainder: { of, destinations: [deck_top, deck_bottom?] }` | `EffectContext::select_effect_choice` (when two destinations) + `select_ordered_permutation` + the `place_remainder_on_deck` placement loop | P-167 (player choice), EX8-047 (single `[deck_bottom]`) |

The `destination` enum for `choose_from_reveal` accepts the bare scalars
`hand`, `deck_top`, `deck_bottom`, `play_free`, or the mapping
`bottom_source_of: { target: <binding> }` — matching the routing shapes now
needed by Rocks and Xros Heart reveal text.

Closure scope: `G-ROCKS-REVEAL-ORDERING` from
`qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md` §50 is closed.
`G-ROCKS-OPTION-SELF-DISPOSITION` §123 has its single remaining raw_rust
target removed (P-206 → native `add_this_option_to_hand`; the other five
target YAML files were already DSL-clean by 2026-05-10). The
`G-ADD-OPTION-SELF-TO-HAND` DSL entry called out in P-206 test comments is
also closed.

## Track E (2026-05-08) — engine helpers shipped, DSL verbs landed

Track E shipped 8 zone-movement helpers + the owner-routing fix at the engine layer. The ten deferred DSL verbs now parse, validate, compile, and lower into the corresponding helpers. Evidence:

- `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_zone_movement_steps`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`

| DSL verb | Engine target | Card driver |
|---|---|---|
| `place_self_at_security: { position, face }` | `place_self_at_security` | EX9-021 (top, face-up), EX4-060-style self placement |
| `place_self_option_at_security: { position, face }` | `place_self_option_at_security` | ST20-15 (top, face-up Option flavor) |
| `bounce_self: {}` | `bounce_self` | BT24-012 Dimetromon self-bounce cost shapes |
| `security_place_top_stacked_card: { carrier, of, position, face }` | `security_place_top_stacked_card` | Puppets G027 |
| `security_place_stacked_card: { carrier, source/source_index_from_top, of, position, face }` | `security_place_stacked_card` | follow-up Puppets / Mineral cards |
| `return_all_trash_to_deck_bottom: { of }` | `return_all_trash_to_deck_bottom` | BT17-077 Imperialdramon: Paladin Mode |
| `trash_top_n_digivolution_cards_of_each: { of, n }` | `trash_top_n_digivolution_cards_of_each` | BT12-028 |
| `trash_opponent_hand_to_count: { opponent, target_count }` | `trash_opponent_hand_to_count` | BT19-075 MoonMillenniummon |
| `search_own_security_stack: { filter, prompt, bind_as, on_select, on_no_match }` | `search_own_security_stack` | TS Olympos cards |
| `scheduled_delayed_return: { subject, destination, position, fire_at }` | `schedule_delayed` (substrate already exists) | BG Imperial G-BG-02 |

The remaining Track E item in this table is unrelated to the ten deferred zone-movement verbs: `scheduled_delayed_return` is still a separate BG Imperial delayed-return shape.

Format per entry:

```
## <CARD_ID> — <clause name>
- Effect text: "..."
- Missing DSL verb / step kind / predicate: ...
- Lowers to engine API: <method on EffectContext that already exists>
- Suggested DSL syntax: <YAML shape>
- First reported: YYYY-MM-DD
```

## Royal Knights — filtered breeding permanent target  [RK-G001]
- Status 2026-05-17: **CLOSED for substrate** by Phase 2 Track J PR 1.
  `SelectOwnBreedingPermanentArgs::filter: PredicateSpec` is now wired
  through compile → lowering → install: the predicate is evaluated against
  `PredicateSubject::BreedingPermanent` before `select_own_breeding_permanent`
  opens, so a non-matching breeding permanent short-circuits the step
  instead of opening a misleading prompt. The companion `BreedingPermanentRef`
  binding now resolves to a sentinel `PermanentHandle { index: BREEDING_TARGET }`,
  which `place_as_bottom_source_observed` already recognizes — so the
  printed shape "place a hand card under a [King Drasil_7D6] in breeding"
  is expressible end-to-end. Proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2g_breeding_selection::select_own_breeding_permanent_filter`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2f1_placement_steps::place_as_bottom_source_accepts_breeding_permanent_target_from_hand_source`.
  Card authoring for BT13-093 / BT20-083 / BT13-110 / EX11-053 lands in
  Phase 2 Track J PR 2.
- Status 2026-05-22: **CLOSED for optionality** by
  `close-royal-knights-substrate-gaps`. Optional
  `select_own_breeding_permanent` now exposes PASS and declines without
  running the placement/play tail; mandatory and no-candidate paths remain
  distinct. BT20-083 and BT13-110 consume this slice in active card-shaped
  tests. EX11-053 no longer blocks on hand-to-fielded-source placement; its
  residual is only the On Deletion Omnimon X hand/source play plus attach-self
  shape.
- Effect text: BT13-093: "[On Deletion] Place 1 Digimon card with the [Royal Knight] trait from your hand under a [King Drasil_7D6] in the breeding area as its bottom digivolution card." BT20-083: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area."
- DSL surface (production form):
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
      prompt: "Choose your [King Drasil_7D6]"
      then:
        - place_as_bottom_source: { source: <hand-binding>, target: kd }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights — source-bound return-self cost into reduced-cost hand play  [RK-G002]
- Effect text: EX11-071: "[Main] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your hand with the play cost reduced by 2."
- Status 2026-05-17: the **return-self-cost half** of this gap closed under Phase 2 Track B (Engine Gap: Generic `.activation_cost(...)` builder hook for triggered abilities, see `qa/resolved-gaps.md`). DSL `activation_cost: { return_self_to_deck_bottom: true }` lifts onto `EffectBuilder::activation_cost(ctx.return_self_to_deck_bottom_as_cost)`; the chained body fires after the source Tamer has left the field. The remaining **reduced-cost hand play half** is card-author DSL: stitch the existing hand selection + `play_from_hand: { cost: { reduce: 2 } }` after the activation-cost step.
- Missing DSL verb / step kind / predicate: a Main-phase activation that pays a source-bound `return_to_deck { target: source, position: bottom }` cost and then opens a player-visible hand play selection whose actual payment is reduced by 2.
- Lowers to engine API: existing source permanent binding, hand selection, and pay-cost flow need a reusable action/pending-selection wrapper so the return cost and reduced play payment stay one legal choice.
- Suggested DSL syntax:
  ```yaml
  - when: main
    optional: true
    pay_cost:
      - return_to_deck: { target: source, position: bottom }
    process:
      - select_hand:
          bind_as: played
          filter:
            all_of:
              - play_cost_gte: 4
              - any_of:
                  - trait_has: "Royal Knight"
                  - trait_has: LIBERATOR
          prompt: "Play a cost 4+ Royal Knight/LIBERATOR"
      - play_from_hand:
          target: played
          cost: { reduce: 2 }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights full pool pass — residual reusable DSL/engine gaps  [RK-G005]
- Status: PARTIAL pool pass completed on 2026-05-05. The Royal Knights resolver pool has 72 unique cards and now has 72 Rust DSL YAML entries. Fully unsupported clauses were left as explicit YAML comments plus ignored Rust tests instead of hidden approximations.
- Newly routed or reaffirmed blocked cards/clauses: `BT13-019`, `BT13-030`, `BT13-075`, `BT13-087`, `BT13-102`, `BT13-111`, `BT13-112`, `BT15-092`, `BT17-077`, `BT19-093`, `BT20-017`, `BT20-021`, `BT20-045`, `BT20-056`, `BT22-025`, `BT22-041`, `BT22-052`, `BT23-013`, `BT23-035`, `BT23-047`, `BT23-057`, `BT23-072`, `EX8-073`, `EX10-068`, and `EX11-053`.
- Missing DSL/engine areas: broader union selection across hand/trash/breeding/source stacks with uniqueness/name-exclusion filters; union hand/trash source-placement costs; opponent hidden-hand choices; result-dependent fallback branches; combined trash/security/color/source-count formulas; card-specific post-Blast-DNA effect bodies after the covered field+hand-material Counter path (`BT17-078`, `BT20-045`, `BT20-060`, `BT20-076`, `BT20-081`, `EX6-011`, `EX6-029`); residual native `<Blast Digivolve>` helper APIs; Option battle-area carrier lifecycle for non-Delay options; security-trash self-dispatch; security search/play card-local follow-up bodies; security-removed card-local follow-up shapes beyond the now-wired battle/effect `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` timing payloads; generalized source-list Partition lowering beyond authored card clauses; and unusual replacement/security-trash costs tied atomically to prevention. **Updated 2026-05-20 (Track J S1.2 + S1.3):** count-capped / name-unique multi-pick play from a carrier's digivolution sources is now FULLY CLOSED via the `select_materials` DSL step + batch `play_from_materials` (see `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`); S1.3 closed the final breeding-carrier residual (King Drasil's resident stack) by appending the `BREEDING_SOURCE_SELECT` action sub-range (`ACTION_SPACE_SIZE` 2168→2192). `when: on_place_security`, alias `when: on_added_to_security`, `when: on_discard_security`, and the printed-text alias `when: on_any_digimon_played` are now wired as of 2026-05-08 with event-card/effect-cause payloads where applicable. Immediate may-attack / force-attack / cancel-attack / open-counter-window prompts are now covered by the Track D DSL verbs listed below. **Track E (2026-05-09)** shipped DSL verbs for self-to-security, Option-self-to-security, stacked-card-to-security, bulk trash/deck movement, forced hand reduction, self-bounce, permanent-to-security observed movement, and security-stack search; remaining card-side work is called out under the narrower per-card gaps below. **Updated 2026-05-20 (Track J S2.1):** the ally-played may-attack observer shape (`G-ALLY-PLAYED-MAY-ATTACK`, BT20-017 / BT23-013) was filed and closed as already-composable — `may_attack_now` accepts `attacker: this | event_target | <named binding>`, so the printed "this Digimon may attack" / "1 of your Digimon may attack" clauses lower from landed primitives; see that entry above. **Updated 2026-05-20 (Track J S2.2):** the Jesmon-family hand/trash name-excluded play (`G-UNION-HAND-TRASH-NAME-EXCLUSION`, BT23-013) is now RESOLVED — the DSL `select_union_zone` lowering now applies its `filter` (it previously dropped it), and a new `name_not_shared_by_field_digimon` predicate leaf models "can't play cards with the same names as any of your Digimon"; see that entry below. Step 0 against printed text corrected the plan premise: BT20-017 has no union play, BT13-019 plays from trash-or-breeding-sources (separate gap), and BT20-021 *places* a source as a cost (separate gap). **Updated 2026-05-20 (Track J S2.3):** the last Royal Knights token, Hinukamuy (BT23-057 Gankoomon), is now registered in `code/digimon-engine/src/token_registry.rs` with its printed stats — Digimon/White/6000 DP/`<Alliance> <Reboot> <Blocker>` — mirroring the Atho/René/Por registration. Token registration for Atho/René/Por and Hinukamuy is now FULLY CLOSED; the remaining BT23-057 work (multi-card trash-to-deck cost reduction, dynamic play-cost delete) is unchanged.
- Updated 2026-05-22 (`close-royal-knights-substrate-gaps`): optional breeding-permanent selection, BT17-018 DP-budget delete migration, BT13-112 source-play payoff, BT13-110 source-placement/Delay source play, BT20-083 optional breeding tuck and inherited source play, BT20-017 token/delete/may-attack observer, BT23-072 hand-main/source-play/played-Digimon keyword grants, BT23-013 token-or-Sistermon branch plus may-attack observer, and EX11-053 On Play hand-to-fielded-King-Drasil source placement now have production YAML plus focused behavioral coverage. Residual Royal Knights blockers remain capability-centric: `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (`BT13-019`), `G-UNION-HAND-TRASH-SOURCE-COST` plus source-count/security formulas (`BT20-021`), BT23-057 multi-card trash-to-deck cost reduction and dynamic play-cost delete, BT23-058 self-scoped on-suspend plus aggregate lowest play-cost delete-all, and EX11-053 On Deletion union hand/source play plus attach-self.
- Workaround policy: no approximations were used for these blockers. If a printed clause required one of the missing primitives, the YAML either implemented an independent faithful slice such as a keyword/security play/simple trigger, or used a load-only gap stub.
- Verification: targeted `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_filter> --nocapture` passed for the final 25 filters, with one active load test and one ignored gap test per card.
- First reported: 2026-05-05 Royal Knights full pool implementation pass.

## Rocks pool pass residual DSL/engine gaps
- Status: PARTIAL pool pass completed on 2026-05-04. After pulling main, production YAML/test slices now exist for 40 of 47 Rocks pool cards; the remaining 7 were explicitly routed as blocked rather than no-op authored.
- Remaining blocked cards: `BT9-103`, `EX8-070`. `EX10-003` moved to production YAML/test coverage on 2026-05-08. `P-130`, `EX11-065`, `EX11-038`, `BT20-055`, `BT23-096`, `BT8-094`, `BT23-059`, `EX11-044`, `EX10-034`, and `EX8-050` now have production YAML/test coverage for the slices closed or verified by `complete-rocks-archetype`.
- Missing DSL/engine areas: Save/Xros routing; source placement from hand/trash; lowest-play-cost delete; and same-side/costed `[When Moving]` follow-up shapes beyond the resolved base OnMove timing.
- First reported: 2026-05-04 Rocks pool implementation pass.

## Zephagamon / Vortexdramon — remaining battle-engine prep gaps
- Status: partial readiness slice added 2026-05-03. `EX11-074.yaml` now covers static `<Piercing>`, `<Vortex>`, `<Blocker>`, and a focused `battle:` pathway. The regression in `tests/cards_behavioral/ex11/ex11_074.rs` proves that an effect battle deletes the defender through DP battle but is not an attack: it must not trigger Piercing/security and must not leave `pending_attack` populated.
- Rule boundary: `battle:` is the correct DSL step for effects that say a Digimon battles another Digimon. Do not model these as `attack` or force-follow-up attack effects. Attack-only timings and Piercing security continuation remain tied to declared attacks, not effect battles.
- EX11-074 remaining gap: the printed "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon..." branch needs a binding/condition result from the suspend step. The DSL can select and suspend, but cannot yet branch on "this effect suspended your Digimon" and bind that cost/result into the follow-up +6000 DP and immunity-until-opponent-turn-ends clause.
- EX11-074 remaining gap: full `[All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. Then, this Digimon may battle 1 opponent Digimon` still needs faithful optional trigger ordering and the unsuspend-then-optional-battle branch. The readiness fixture keeps the battle path focused instead of auto-implementing the whole printed clause.
- BT20-101 remaining gap: Zephagamon needs a formula that counts suspended Digimon, divides that count by 2, and uses the capped result as the number of opponent Digimon selected to place at the bottom of the deck. Existing count-capped multi-select support needs this suspended-count / division formula vocabulary and bottom-deck target movement wiring for the full clause.
- EX11-035 remaining gap: the green Avian/Bird play effect needs a formula DP cap for the target card. The DSL needs a predicate/formula shape that computes the allowed play target's DP ceiling from the printed condition rather than a fixed literal.
- EX11-062 remaining gap: the card needs a conditional `VortexCanAttackPlayer` aura while the opponent has no unsuspended Digimon. The engine now has the `VortexCanAttackPlayer` modifier type and the runtime `Expiry::UntilCondition` continuous controller, but the DSL still needs aura/active_when lowering that attaches the compiled BoolPredicate to the modifier entry's `until_condition` field.
- Gap kind: hybrid. Some engine primitives exist (`battle:`, static keyword grants, `ModifierType::VortexCanAttackPlayer`), but the remaining Zephagamon clauses need DSL result bindings, formulas, conditional aura lowering, and card-specific faithful branch wiring.
- First reported: 2026-05-03 (Zephagamon Battle Engine Prep Task 4)

## BT22-098 / P-229 — event-gated Delay activation windows — RESOLVED 2026-05-21
- Effect text: BT22-098: "[Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> ... 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3." P-229: "[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay> ... 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- Status: **closed** 2026-05-21 (gap id `PUPPETS-G004`). The BT22-098 `on_suspend`
  slice closed 2026-05-02; the P-229 `on_ally_played` slice closed 2026-05-21.
- Closed via (two halves):
  - DSL: `code/digimon-engine/src/dsl_cards/lower_delay.rs` now maps
    `CompiledTiming::OnAllyPlayed` → `DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed)`
    alongside the existing `on_suspend` / `on_unsuspend` arm.
  - Engine: `code/digimon-engine/src/effect_queue.rs` `enqueue_triggered` now fans
    `TriggerSource::EnteredField` dispatches out to
    `enqueue_event_gated_delayed_options` (previously only `EventObserved` /
    `AttackTargetChanged` reached it).
- Working DSL syntax (P-229 production YAML, `cards/p/P-229.yaml`):
  ```yaml
  - kind: delay
    trigger: on_ally_played
    active_when:
      your_turn: true
      event_target_owner: you
      event_card_name_contains: "Mirai Kinosaki"
    process:
      - select_own_permanent:
          bind_as: target
          optional: true
          filter: { all_of: [ { kind: digimon }, { zone: [battle_area] } ] }
      - select_hand:
          of: you
          bind_as: evo
          optional: false
          filter:
            all_of: [ { kind: digimon }, { trait_has: LIBERATOR }, { level_lte: 6 } ]
      - effect_initiated_digivolve:
          target: target
          from_hand: evo
          cost: { reduce: 3 }
          ignore_requirements: false
  ```
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_229` — 13 tests pass, 0 ignored.
- First reported: 2026-04-28 (Puppets archetype assessment); resolved 2026-05-21.

## EX9-032 / EX7-027 / BT22-036 — replacement cause predicate and `active_when` lowering
- Effect text: "[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving."
- Status: PARTIALLY RESOLVED on 2026-05-03. Replacement clauses now preserve replacement subject/source/cause predicates through lowering, apply `active_when`, and can protect a different subject than the replacement source. This is verified for `BT24-040`/`BT24-101`-style TS protection and `BT17-097` Delay replacement continuation.
- Updated 2026-05-06 (Track B): replacement timing vocabulary now includes named pre-move triggers `when_would_digivolve`, `when_would_play`, and `when_would_link`, mapping respectively to `EffectTiming::WhenPermanentWouldDigivolve`, `EffectTiming::WhenPermanentWouldPlay`, and `EffectTiming::WhenWouldLink`. Mandatory cancel dispatch is covered at the engine fire-sites; optional `Card`-subject accept/decline resume remains an engine follow-up before optional DSL card text should target these windows.
- Updated 2026-05-08 (Track B): inherited replacement dispatch now scans buried source effects, and the Puppet/token cost body is live for `BT22-036`, `EX11-022`, `EX9-032`, `EX7-027`, and `ST19-11`. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_036_inherited_replacement`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_022_inherited_leave_prevention`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_032_inherited_prevents`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex7_027_inherited`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st19_11_inherited`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- Remaining missing DSL/card work: none for inherited Token/Puppet leave-prevention dispatch itself; adjacent active-effect gaps on those cards remain independently tracked.
- Lowers to engine API: replacement evaluator context plus `EffectContext` replacement outcome setters such as `cancel_leave`.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      replacement_cause_not: own_effect
    process:
      - select_own_permanent:
          as: cost
          filter:
            any_of:
              - kind: token
              - trait_has: Puppet
            other_than_source: true
      - delete_permanent: { target: cost }
      - cancel_replacement: {}
  ```
- Gap kind: partially resolved hybrid. The reusable replacement-context predicate/lowering slice is closed; unimplemented card bodies remain card-authoring work unless they surface new reusable primitives.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 bt24_101 --nocapture`; named pre-move vocabulary proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- First reported: 2026-04-28 (Puppets archetype assessment)

## BT24-080 — delete all opponent Digimon with the lowest level
- Status: PARTIALLY RESOLVED for the reusable lowest-level permanent predicate on 2026-05-02. `CompiledPredicate::level_matches_aggregate` can match permanents whose top card level equals `CompiledAggregateSelector::LowestLevel` for a player scope, skipping Tamers/Options with no top-card level. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon`.
- Effect text: "[On Play] [When Digivolving] [On Deletion] Delete all of your opponent's lowest level Digimon."
- Remaining DSL verb / step kind / predicate: card-specific authoring still needs to wire the aggregate predicate through the surrounding delete-all flow. Repeat target-selection blockers elsewhere are unrelated and remain open.
- Lowers to engine API: engine-side iteration over opponent battle-area permanents plus `delete_permanent` is sufficient once the minimum-level candidate set can be computed.
- Suggested DSL syntax:
  ```yaml
  - delete_all:
      of: opponent
      filter:
        kind: digimon
        level_is: { aggregate: minimum, over: opponent_battle_area }
  ```
- First reported: 2026-04-28

## Rocks archetype refresh — source-selection and cost-payment DSL surface  [G-ROCKS-SOURCE-SELECTION-DSL]
- Effect text: Rocks core repeatedly uses "by trashing any 1/3 [Mineral] or [Rock] trait card(s) from your Digimon's digivolution cards" and "place up to N [Mineral]/[Rock] cards from your trash as bottom digivolution cards." Examples: `EX10-032`, `P-167`, `EX10-036`, `EX10-033`, `EX8-055`, `EX10-028`, `EX8-070`, `EX10-025`.
- Missing DSL verb / step kind / predicate: First-class source-zone selectors for digivolution cards across all of your own stacks, including exact-N, up-to-N with PASS terminator, and single-pick forms. Current DSL has `place_as_bottom_source` and `trash_top_source`, but no `select_source_across_own_permanents` / `select_n_sources_across_own_permanents` step that can bind `(PermanentHandle, source_index)` choices and then trash/place exactly the selected cards.
- Companion engine gap: `docs/RUST_ENGINE_GAPS.md` tracks the engine half under "Cross-permanent count-capped multi-select" and the cost-ordering half under "`.pay_cost()` builder hook for triggered non-cost-reduction effects." This entry tracks the YAML vocabulary and lowering shape that should sit on top of those primitives once available.
- Lowers to engine API: proposed `ctx.select_source_across_own_permanents(...)`, `ctx.select_n_sources_across_own_permanents(...)`, and `EffectBuilder::pay_cost_trash_n_own_sources_by_trait(...)`.
- Suggested DSL syntax:
  ```yaml
  - pay_cost:
      select_sources:
        of: you
        from: any_own_digimon
        count: 1
        filter:
          any_of:
            - trait_has: Mineral
            - trait_has: Rock
        bind_as: trashed_sources
      then:
        - trash_selected_sources: trashed_sources
  ```
  Up-to-N variants should use `max_count: 3` and surface PASS as a legal terminator so RL sees the "stop selecting" choice.
- Gap kind: hybrid (engine selection/action support is still required; DSL needs the reusable vocabulary and lowering once that lands).
- Workaround: Do not auto-pick sources. The Rocks assessment on 2026-04-28 found this to be the core no-approximations blocker for the archetype.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — event-card predicates for Mineral/Rock observers  [G-ROCKS-EVENT-CARD-PREDICATES]
- Effect text: Rocks Tamers and inherited effects gate on the card or host involved in a just-fired event, for example "when any of your Digimon digivolve into a [Mineral] or [Rock] trait Digimon" (`EX8-067`) and "when effects trash digivolution cards of any of your [Mineral] or [Rock] trait Digimon" (`EX10-063`, `P-169`, `EX11-065`).
- DSL predicate coverage: reusable predicate leaves for `trashed_source_trait_has`, `trashed_source_card_id_is`, and `host_permanent_trait_has` are implemented for event payloads with host/source context. Broader aliases such as `digivolving_card_trait_has` remain vocabulary work if card authors need that spelling; existing source-relative leaves such as `source_permanent_trait_has` are not enough unless the lowering receives the correct event subject and distinguishes observer permanent, host permanent, and trashed source card.
- Companion engine gap: the engine still needs full `OnDigivolutionCardTrashed` fan-out with host/source context; see `docs/RUST_ENGINE_GAPS.md` "OnDigivolutionCardTrashed observer timing" and related Rocks entries.
- Updated 2026-04-29: the OnDigivolve half now has runtime event-card and event-target context for normal `Game::digivolve_from_hand`; `event_card_trait_has` reads the new top card, and `target: event_target` binds the just-digivolved permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`.
- Updated 2026-04-29: `Game::return_to_hand` source disposition now carries `event_card` / `event_source_card` for the trashed source and `event_host_card` for the former host top card, so `event_card_trait_has` can match sources trashed by that path. Runtime `event_host_permanent()` only exposes the stored host handle if it still resolves to that same card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source source_trash_host_context_does_not_alias_shifted_permanent` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source`. Remaining source-trash gaps include cross-permanent source selection, source-trash paths other than `return_to_hand`, and first-class DSL leaves for trashed-source / host-permanent predicates.
- Updated 2026-05-02: first-class predicate leaves now compile for `event_target_owner`, `host_permanent_trait_has`, `trashed_source_trait_has`, and `trashed_source_card_id_is`; runtime coverage exercises `TriggerSource::SourceTrashedFromStack` with live host/trashed-source context. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context`. Remaining source-trash producer paths not covered here should stay open until each producer proves it supplies host/source context rather than relying on fallback guessing.
- Updated 2026-05-03: Task 6 audit found the reusable source-trash payload and DSL predicate leaves already implemented. Added focused regression coverage that an actual `EffectContext::trash_card_source` producer supplies the exact trashed source card and live host into `trashed_source_trait_has`, `trashed_source_card_id_is`, `host_permanent_trait_has`, and `event_target_owner`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context_bindings group6_dynamic_formulas group7_predicate_batch --nocapture`. No new event payload, predicate, formula, action, or tensor primitive was added.
- Updated 2026-05-07: Return-to-deck source disposition and de-digivolve now emit `TriggerSource::SourceTrashedFromStack` through `Game::fire_digivolution_card_trashed(...)`, including cause and moved-card payload data. `host_permanent_trait_has` now falls back to the event host-card snapshot after the host leaves the battle area. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_return_to_deck_carries_host_and_trashed_source on_digivolution_card_trashed_de_digivolve_carries_host_and_trashed_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_051_inherited_source_trash_dedigivolves_after_host_return_to_deck`. Remaining source-trash DSL work is producer/card-local for additional source-trash cost shapes.
- Updated 2026-05-07: `select_own_sources` now accepts `target: <binding-ref>`, so inline source costs can be restricted to the activating permanent (`target: source`) rather than all own stacks. BT4-072 proves exact-N Digi-Burst authoring with a target-scoped source selection, `trash_selected_sources`, and the follow-up DP target choice. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_sources`.
- Updated 2026-05-07: `digi_burst` is now a reusable DSL step that lowers to the canonical self-source exact-N selection and inserted trash-cost step before the nested body. BT4-072 now uses this wrapper, and printed keyword parsing carries `Keyword::DigiBurst(N)`. Covered by `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_source_selection_steps` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parser_digi_burst_parametric`.
- Updated 2026-05-08: `digi_burst` now has a count-2 regression fixture proving exact-N self-stack masking, no PASS before the required count, per-selected-source `OnDigivolutionCardTrashed` emission, and continuation into the nested body after the source-trash cost. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digi_burst_two_selects_exact_self_sources_and_fires_source_trash_per_card`.
- Lowers to engine API: `TriggerContext` / event payload fields containing `{host_permanent, trashed_card, trashed_source_index, cause_player}` plus predicate evaluation against those fields.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - host_permanent_trait_has: Mineral
      - trashed_source_trait_has: Rock
  ```
  Trait alternatives should compose through existing `any_of`.
- Gap kind: hybrid (requires engine event context plus DSL predicate leaves).
- Workaround: None faithful. Scanning trash after the fact loses which source card was trashed from which host, and can trigger the wrong inherited card.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — authored YAML coverage note
- Assessment target: the `Rocks` / `RockClose` archetype in `data/deck_library.json`, refreshed on 2026-04-28.
- Finding: as of the 2026-05-04 Rocks batch plus the pulled main updates, 40 of 47 Rocks pool cards have Rust YAML under `code/digimon-engine/cards/`. New Rocks pass coverage added or audited the `EX8`/`EX10`/`EX11`/`P-167` shell; the remaining missing cards are tracked in the residual gap entry above.
- Existing DSL gaps reaffirmed by the refresh:
  - `EX11-008 — [When Moving] timing` no longer blocks on the `on_move` token or moved-card event context as of 2026-04-29; card bodies may still need separate target-selection, reveal, or follow-up action primitives.
  - `P-189 — play cost <= filter` was closed on 2026-05-01 for static `play_cost_lte` filters on `select_hand` / `select_trash`; remaining Rocks blockers are tracked separately.
  - `P-206 — Board-color cross-reference predicate` was closed on 2026-05-02 for dynamic `color_matches_any_field_digimon` card predicates; any remaining P-206 Delay, Option, or action-flow blockers are separate.
  - `P-107 — place_self_as_delay_option` remains relevant to `P-107`, `P-039`, `BT23-096`, and related Delay/security disposition effects.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## BT22-015 — grant "this Digimon may attack" after When Digivolving
- Status: RESOLVED for the immediate printed follow-up attack (2026-05-08). `may_attack_now` is available in YAML and lowers to the centralized attack-open flow with PASS exposed through pending selection. BT22-015 uses this for "Then, this Digimon may attack."
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- may_attack_now_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_037`.
- Additional Track D coverage: BT24-037 Silphymon uses the same `may_attack_now` step after its shared On Play/When Digivolving -5000 DP selection, proving the TS Olympos "1 of your Digimon may attack" branch with PASS before attack commitment.
- Effect text: "[When Digivolving] ... Then, this Digimon may attack."
- Previous missing DSL verb / step kind / predicate: `ModifierType::MayAttack` / immediate attack permission was not exposed by the DSL modifier map, and there was no declarative step that lowered to the engine's attack-permission helper once the effect resolved.
- Lowers to engine API: `ModifierType::MayAttack` / `ModifierType::CanAttackUnsuspended` or the force-follow-up attack helper tracked in `docs/RUST_ENGINE_GAPS.md`.
- Supported DSL syntax for the resolved immediate prompt: `may_attack_now: { attacker: source, targets: any, optional: true }`. Persistent attack-permission grants remain a separate modifier/aura problem.
- First reported: 2026-04-28

## Royal Knights — ally-played may-attack observer  [G-ALLY-PLAYED-MAY-ATTACK]
- Status: **RESOLVED / already-composable on 2026-05-20** (Phase 2 Track J Task S2.1). Filed and closed in the same pass — this gap previously had no canonical entry, only a name in the Royal Knights `RK-G005` rollup. No engine or DSL code change was needed. Full resolution detail in [qa/resolved-gaps.md](resolved-gaps.md#engine--dsl-gap-g-ally-played-may-attack--already-composable-2026-05-20-phase-2-track-j-task-s21).
- Card consumers: `BT20-017` Jesmon, `BT23-013` Jesmon.
- Effect text: BT20-017 — "[Your Turn] [Once Per Turn] When any of your other Digimon are played, delete 1 of your opponent's Digimon with 8000 DP or less. Then, 1 of your Digimon may attack." BT23-013 — "[Your Turn] [Once Per Turn] When any of your other Digimon are played, this Digimon may attack."
- Step 0 finding: `may_attack_now` is NOT hard-bound to `self`. Its `attacker:` is a `BindingRef`; the lowering (`combat.rs::resolve_permanent_ref` → `binding_ref.rs::resolve_binding_ref`) already resolves `event_target` to the event-played permanent (`CompiledBindingRef::EventTarget` ← `TriggerSource::EnteredField.event_permanent`), as well as `this` (`Source`) and any named `bind_as` binding (`Binding`). The printed text moreover differs from the original substrate-plan premise: BT23-013 grants the attack to "**this** Digimon" (the observer source, `attacker: this`) and BT20-017 to "**1 of your** Digimon" (a player choice — `select_own_permanent` `bind_as` then `attacker: <binding>`); neither uses the event-played Digimon as the attacker. All three attacker shapes were already composable from primitives landed on/before BASE.
- Supported DSL syntax: `may_attack_now: { attacker: event_target, targets: any, optional: true }` (event-played Digimon), `attacker: this` (observer source), or `attacker: <named binding>` from a prior `select_*` step. PASS surfaces through `build_action_mask` for the optional `may` (§17).
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- effect_granted_attack` (15 pass — incl. `may_attack_now_event_target_yaml_lowers_to_event_target_binding`, `may_attack_now_event_target_grants_attack_to_event_played_digimon`, `may_attack_now_event_target_decline_branch_starts_no_attack`, `may_attack_now_this_vs_event_target_select_different_attackers`, `may_attack_now_event_target_respects_summoning_sickness`).
- Card-authoring status 2026-05-22: production `BT20-017.yaml` and `BT23-013.yaml` now consume this primitive in active behavioral tests. This entry remains only as the reusable substrate closure record.
- First reported: 2026-05-05 (Royal Knights full pool implementation pass, as part of `RK-G005`).

## Royal Knights — Jesmon-family hand/trash name-excluded play  [G-UNION-HAND-TRASH-NAME-EXCLUSION]
- Status: **RESOLVED on 2026-05-20** (Phase 2 Track J Task S2.2). Two genuine substrate pieces were missing and are now closed. Filed and closed in the same pass — this gap previously had no canonical entry, only a name in the Royal Knights `RK-G005` rollup and in the BT23-013 test ignore string. Full resolution detail in [qa/resolved-gaps.md](resolved-gaps.md#engine--dsl-gap-g-union-hand-trash-name-exclusion--resolved-2026-05-20-phase-2-track-j-task-s22).
- Card consumers: `BT23-013` Jesmon is the **only** genuine consumer of this exact "hand OR trash, name-restricted, exclude names already in play" shape. The substrate-plan premise also named BT20-017 / BT13-019 / BT20-021, but Step 0 against printed text found those are different mechanics — see "Plan-premise correction" below.
- Effect text (BT23-013, the genuine consumer): "[When Digivolving] [When Attacking] You may play 1 [Atho, René & Por] Token (…) or, **from your hand or trash, 1 Digimon card with [Sistermon] in its name without paying the cost. This effect can't play cards with the same names as any of your Digimon.**"
- Step 0 finding — what was actually missing:
  - (a) The DSL `select_union_zone` step (hand+trash in one prompt) carried a `filter: PredicateSpec`, but the engine lowering `install_select_union_zone` in `code/digimon-engine/src/dsl_cards/step/selections.rs` passed a hardcoded `|_game, _card| true` accept-all closure and **dropped the compiled filter entirely**. The engine helper `EffectContext::select_union_zone` itself already applies whatever filter it is given (proven by `tests/selection/union_zone.rs::filter_restricts_valid_action_ids`) — so this was a DSL-lowering bug, not an engine-helper gap. Name-restriction (`name_contains: Sistermon`) was silently inoperative for every union-zone card.
  - (b) No predicate leaf could express "this candidate card's name is NOT shared by any of my battle-area Digimon". The existing `no_permanent` existential matches against fixed predicate fields and cannot reference the candidate card's own name; `color_matches_any_field_digimon` was the closest analog but for colors.
- What was added:
  - `name_not_shared_by_field_digimon: { of: <player> }` — a card-subject predicate leaf. True when no battle-area Digimon of the scoped player has the candidate card's effective name (field names read via `synth_identity`, so a `ChangeBaseCardName` overlay on a field Digimon is respected; the candidate's own name respects a reveal overlay). Exact, case-sensitive comparison, consistent with `name_is` / `name_in`.
  - The `select_union_zone` lowering now builds an `EffectReadContext` per candidate and evaluates the compiled `filter` against each hand/trash `CardSource`, exactly as `install_select_hand` / `install_select_trash` already did.
- Supported DSL syntax:
  ```yaml
  - select_union_zone:
      of: you
      zones: [hand, trash]
      optional: true            # printed "You may …" — PASS stays legal (§17)
      prompt: Play 1 Sistermon from hand or trash
      filter:
        all_of:
          - name_contains: Sistermon
          - name_not_shared_by_field_digimon: { of: you }
  ```
  The name-exclusion shapes the legal action mask — every surviving candidate from hand AND trash surfaces through `pending_selection`; it never auto-picks.
- Plan-premise correction: the substrate plan named four cards. Printed text (Step 0) shows only BT23-013 matches the "hand+trash + own-name-exclusion" shape. **BT20-017** has no union play at all (only an Atho/René/Por token play). **BT13-019** Gankoomon plays from *trash OR a breeding-area Digimon's digivolution sources* with a *fixed* name-exclusion (`Gankoomon` / `Omnimon`) — a genuinely distinct gap, now filed canonically as `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (see entry below), untouched here. **BT20-021** Jesmon GX *places* a Royal Knight card from hand/trash as a digivolution source — a **cost**, not a play, with no name-exclusion — also a distinct gap, now filed canonically as `G-UNION-HAND-TRASH-SOURCE-COST` (see entry below). Both spin-off IDs were discovered during this task's Step 0 and, per the Discovery rider, are filed as their own canonical tracker entries rather than left as narrative-only descriptions here. "Union" is informal shorthand: no printed `<Union>` keyword exists (verified absent from `docs/RULES_CONTEXT.md`).
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- s2_2_union` (2 behavioral tests: `union_zone_filter_excludes_in_play_name_across_hand_and_trash`, `union_zone_filter_keeps_all_sistermon_when_field_empty`); `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- name_not_shared_by_field_digimon` (lowering test) and `-- parse_leaf_predicates` (parse test).
- First failing test (TDD red): `union_zone_filter_excludes_in_play_name_across_hand_and_trash` — before the fix it offered 3 candidates (filter dropped, `name_not_shared_by_field_digimon` silently swallowed into `PredicateSpec::extra`) instead of the 2 legal Sistermon names.
- Card-authoring status 2026-05-22: production `BT23-013.yaml` now implements `<Rush>`, `<Alliance>`, the token-or-Sistermon effect choice, hand/trash Sistermon filtering, and the other-Digimon-played may-attack observer with active card-shaped coverage. This entry remains only as the reusable substrate closure record.
- First reported: 2026-05-05 (Royal Knights full pool implementation pass, as part of `RK-G005`).

## Royal Knights — Gankoomon trash-OR-breeding-source play with fixed name-exclusion  [G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY]
- Status: **BLOCKED**.
- Card consumer: `BT13-019` Gankoomon is the genuine consumer of this dual-zone "trash OR a breeding-area Digimon's digivolution sources" play shape with a *fixed* name-exclusion.
- Effect text (BT13-019, verified against `data/cards.json`): "＜Blocker＞ (This Digimon can block in the blocker timing.) [On Play] [When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your trash or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect."
- What DSL/engine surface is missing:
  - A single optional play prompt that draws candidates from *two heterogeneous sources at once* — the player's trash AND the digivolution cards (sources) of Digimon in the player's breeding area — each half carrying its own filter (`name_contains: Sistermon` for the trash half; `trait_has: "Royal Knight"` for the breeding-source half).
  - A *fixed*-name exclusion applied across both halves (`name_in`-style: can't play `[Gankoomon]` or `[Omnimon]`). This is distinct from `name_not_shared_by_field_digimon` (Task S2.2), which is a dynamic exclusion against the names of your field Digimon; here the excluded names are printed literals.
- The breeding-area-source half is **already covered** by the resolved gap `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` (`select_materials` / `select_material` against breeding-area carriers, closed 2026-05-20 Track J S1.3 — see that entry below). The residual unique to this gap is therefore: (1) the *dual-zone* selection that unions a trash filter with a breeding-area-source filter in one player-visible prompt, and (2) the *fixed* `name_in`-style exclusion gating that combined candidate set.
- Plan-premise note: this card was named in the original substrate plan for `G-UNION-HAND-TRASH-NAME-EXCLUSION` but Step 0 against printed text showed it is a different mechanic; it is filed here as its own canonical entry per the Discovery rider.
- First reported: 2026-05-20 via S2.2 Step 0.

## Royal Knights — Jesmon GX hand/trash digivolution-source placement cost  [G-UNION-HAND-TRASH-SOURCE-COST]
- Status: **BLOCKED**.
- Card consumer: `BT20-021` Jesmon GX is the genuine consumer of this "place a card from hand or trash as a digivolution source" *cost* shape.
- Effect text (BT20-021, verified against `data/cards.json`): "[Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this card without paying the cost.) [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with as much or less DP as this Digimon. [When Attacking] [Once Per Turn] This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in this Digimon's digivolution cards, trash your opponent's top security card." (The clause relevant to this gap is the second one: "By placing 1 [Royal Knight] trait card from your hand or trash as this Digimon's bottom digivolution card, delete …".)
- What DSL/engine surface is missing: a *cost* (not a play) that requires the player to select 1 card matching a filter (`trait_has: "Royal Knight"`) from a union of two zones — hand OR trash — and place it as the **bottom** digivolution card of the source Digimon, as the price of activating the rest of the effect. There is **no name-exclusion** on this clause. Distinct from `G-UNION-HAND-TRASH-NAME-EXCLUSION` (a *play* with a dynamic own-name exclusion) and from `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (a *play* with a fixed name-exclusion, trash-OR-breeding zones): this is a hand-OR-trash *cost* that places the selected card as a bottom digivolution source rather than playing it.
- Plan-premise note: this card was named in the original substrate plan for `G-UNION-HAND-TRASH-NAME-EXCLUSION` but Step 0 against printed text showed it is a different mechanic (a cost, not a play); it is filed here as its own canonical entry per the Discovery rider.
- First reported: 2026-05-20 via S2.2 Step 0.

## BT22-015 — count same-level pairs in own stack
- Status: RESOLVED on 2026-05-07. `CompiledPerSelector::SameLevelPairsInSources` counts source cards below the top card by level and sums `count / 2` per level bucket; `select_count_capped_multi.max` now accepts `{ formula: ... }`; and the DSL wrapper supports `zone: battle_area` to bind a `PermanentList` for `per_selected`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- source_stack_aggregate_formula_reads_source_levels phase2d_select_count_capped_multi` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair`.
- Effect text: "[When Digivolving] For every 2 cards with the same level in this Digimon's digivolution cards, return 1 of your opponent's Digimon to the bottom of the deck."
- Former missing DSL verb / step kind / predicate: repeat-count target selection derived from a formula.
- Lowers to engine API: stack inspection plus repeated `return_to_deck(..., DeckEnd::Bottom)` after each player-visible target selection.
- DSL syntax: `select_count_capped_multi: { zone: battle_area, max: { formula: { base: 0, per: same_level_pairs_in_sources, delta: 1 } }, ... }` followed by `per_selected` over the bound permanent list.
- First reported: 2026-04-28

## BT17-078 — bottom-deck all opponent Digimon sharing chosen level
- Status: RESOLVED on 2026-05-07. The DSL now supports `bind_permanent_property` for selected permanent properties and `level_eq_binding` for later permanent/card predicates; BT17-078 uses this to bind the chosen opponent Digimon's level, for-each every opponent Digimon with that level, bottom-deck them, then surface the mandatory delete prompt. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_bind_permanent_level_property_step bind_permanent_level_filters_for_each_same_level_permanents` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078`.
- Effect text: "[On Play] [When Digivolving] ... place all of your opponent's Digimon with the same level as 1 of their Digimon at the bottom of the deck."
- Former missing DSL verb / step kind / predicate: Binding one selected opponent Digimon's level and applying a mass same-level filter to every opponent permanent. Closed by `bind_permanent_property` plus `level_eq_binding`.
- Lowers to engine API: select opponent permanent, read selected level, then call `return_to_deck(..., DeckEnd::Bottom)` for each opponent permanent whose top card has that level.
- DSL syntax: `bind_permanent_property: { from: chosen_dig, property: level, bind_as: chosen_level }` followed by `for_each: { over: { level_eq_binding: chosen_level }, ... }`.
- First reported: 2026-04-28
---

## BT23-005 — [Your Turn] cost reduction when digivolving into Reptile/Dragonkin  [G-BEFORE-PAY-COST-DIGIVOLVE-TARGET]
- **Status: RESOLVED 2026-05-17** (Phase 2 Track H). See `qa/resolved-gaps.md` § "Phase 2 Track H closure — 2026-05-17" for the substrate landed (`cost_target` + `source_is_cost_target_permanent` predicates, digivolve-cost-calc target threading).
- Authoring pattern:
  ```yaml
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when:
      all_of:
        - your_turn: true
        - source_is_cost_target_permanent: true
        - cost_target: { trait_has: [Reptile, Dragonkin] }
    amount: 1
  ```
- Card-authoring note: BT23-005 YAML still needs to be updated to use the new pattern; P-117 has been migrated as the proof-of-substrate (`code/digimon-engine/cards/p/P-117.yaml`).
- First reported: 2026-04-27 (BT23-005 batch-implement-cards-rust-dsl)
- Also blocks (now resolvable): P-117 clause 0 — "[Your Turn][OPT] When this Digimon would digivolve into a card with the [Free] trait, if you have a Tamer, reduce the digivolution cost by 1." Migrated and validated 2026-05-17.

---

## P-117 — inherited When Attacking color-count predicate  [G-DSL-SELF-COLOR-COUNT-GTE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — `self_color_count_gte` exists; P-117 IMPLEMENTED. The "Remaining sibling blocker" note below is now stale: BT12-031 clause 1b ships via `self_color_count_gte` in a `while_condition` (see the BT12-031 entry).

- Effect text: "[When Attacking] If this Digimon has 2 or more colors, ＜Draw 1＞ (Draw 1 card from your deck.)"
- Status 2026-05-11: resolved for top-card color counts. `self_color_count_gte: N` is now in `PredicateSpec` / `CompiledPredicate` and evaluates the predicate subject/source permanent's synthesized top-card colors.
- DCGO reference: `P_117.cs` lines 203-211 — `card.PermanentOfThisCard().TopCard.CardColors.Count >= 2`. Note: DCGO checks ONLY the top card's colors, not the union of the full digivolution stack. The DSL predicate should align with DCGO behavior: count the top card's colors only.
- Lowers to engine API: `Game::player(p).battle_area[i].top_card()` → `card_data[idx].colors.len()` comparison; no new engine primitive needed, only a DSL predicate leaf that invokes `ctx.source_permanent` top-card color count.
- DSL syntax:
  ```yaml
  condition:
    self_color_count_gte: 2
  ```
  Evaluates as: `ctx.source_permanent.and_then(|h| perm.top_card().colors().len()).unwrap_or(0) >= 2`.
  Alternative: `source_top_card_color_count_gte: 2` if the naming convention favors explicit subject.
- Gap kind: DSL only (engine has the data; only the predicate leaf is missing).
- Cards unblocked: P-117 clause 1 (inherited When Attacking).
- Remaining sibling blocker: BT12-031 clause 1b still needs a distinct stack-union color-count predicate ("2+ colors in digi-cards"), not this top-card-only predicate.
- First reported: 2026-05-04 (P-117 batch-implement-cards-rust-dsl)

---

## BT21-025 — `attacker_trait_has` predicate on `on_attack_target_change` clauses  [G-ATK-TRAIT-FILTER]
- Effect text: "[Your Turn][Once Per Turn] When any of your [Reptile] or [Dragonkin] trait Digimon's attack targets change, trash your opponent's top security card."
- Missing DSL verb / step kind / predicate: `attacker_trait_has` (and likely `attacker_owner_is_you`) predicates to gate `on_attack_target_change` clauses by the attacking permanent's traits/owner.
- Status (2026-05-07): narrowed. `on_attack_target_change` now carries structured payload predicates for `attack_target_change_reason`, `attacker_trait_has`, `event_target_is_player`, `event_target_was_self`, and new-target `event_target_owner`/`event_target_trait_has`; the owner-specific predicate in this gap remains open. Coverage for the closed payload leaves: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- attack_target_change_`.
- Lowers to engine API: `TriggerContext` already carries `source_permanent` for `PlayerBattleArea` triggers; a predicate could inspect `ctx.trigger_context.source_permanent.traits()`. No new engine API needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    attacker_trait_has: Reptile
    # or any_of: [{ attacker_trait_has: Reptile }, { attacker_trait_has: Dragonkin }]
  ```
- Workaround used: `any_permanent` filter over your battle area with `trait_has: Reptile/Dragonkin` — necessary but not sufficient (over-fires when a non-matching attacker switches target while a matching ally is on board).
- First reported: 2026-04-27 (BT21-025 batch-implement-cards-rust-dsl)

---

## ~~BT24-016 — `condition:` field on `AltPathSpec` (alt-digivolve activation gates)  [G-ALT-PATH-CONDITION]~~ — RESOLVED 2026-05-15

- **Status:** Schema + consumer wired. `AltPathSpec.condition: Option<PredicateSpec>` is now accepted by the DSL parser, compiles to `CompiledAltPath.condition: Option<Box<CompiledPredicate>>`, and is evaluated in `code/digimon-engine/src/dna_digivolve.rs::find_matching_alt_path` after the source-filter check (Digivolve route).
- **Card-side authoring follow-up:** BT24-016's YAML still leaves the Owen Dreadnought gate unenforced; populating `condition:` on the activated_digivolve path is card-local work, not substrate.
- **Evidence:** `cargo test --manifest-path code/digimon-dsl/Cargo.toml`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage`.
- **Full entry archived to:** `qa/resolved-gaps.md` under "DSL Gap: `AltPathSpec.condition` field for alt-digivolve activation gates".

---

## EX11-054 — [All Turns] entering-permanent trait gate  [G-ENTERING-PERMANENT-TRAIT]

- Effect text: "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP for the turn."
- Missing DSL verb / step kind / predicate: `entering_permanent_trait_has` / `digivolving_permanent_trait_has` — BoolPredicate leaves to gate an observer clause on the traits of the card that JUST entered the field or digivolved. The `event_target_trait_has` predicate evaluates `TriggerContext.target_permanent`, which for `OnEnterFieldAnyone` / `OnDigivolve` observers is the OBSERVER's own permanent handle (not the entering/digivolving card).
- Companion engine gap: `trigger_context_for_source` in `effect_queue.rs` sets `target_permanent = source_permanent` (the observer itself) when iterating `TriggerSource::PlayerBattleArea(pid)`. The entering card's handle is not threaded into `TriggerContext`. Additionally, `GameEvent::Digivolve` is "defined for future wiring — not emitted yet" (events.rs), blocking event-log-based detection of the digivolving permanent.
- Updated 2026-04-29: the digivolve half is now partially closed for normal `Game::digivolve_from_hand`: `GameEvent::Digivolve` is emitted and `TriggerSource::Digivolved` populates `TriggerContext.event_permanent` / `event_card` with the just-digivolved permanent and new top card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`. `OnEnterFieldAnyone`, effect-initiated digivolve, DNA digivolve, and breeding-area digivolve remain open.
- Updated 2026-04-29: the enter-field half is now partially closed for normal hand-played battle-area permanents: `TriggerSource::EnteredField` populates `TriggerContext.event_permanent` / `event_card` with the entering permanent and card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card`. Effect-created permanents, token play, option placement, play-from-trash context, and breeding-area observer fan-out remain open.
- Lowers to engine API: covered enter-field and digivolve paths now use `TriggerContext.event_permanent` / `event_card`; remaining dedicated `entering_permanent_trait_has` / `digivolving_permanent_trait_has` syntax, if added, should lower to those fields and keep untested entry/digivolve paths gated until separate dispatch tests exist.
- Suggested DSL syntax:
  ```yaml
  condition:
    any_of:
      - entering_permanent_trait_has: Reptile
      - entering_permanent_trait_has: Dragonkin
  # (same shape for digivolve half with digivolving_permanent_trait_has)
  ```
- Gap kind: hybrid (engine doesn't thread the entering-permanent handle through TriggerContext; DSL has no predicate leaf to read it even if it did).
- Workaround: `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). All related tests `#[ignore]`'d with `entering_permanent_trigger_context` tag.
- First reported: 2026-04-27 (EX11-054 batch-implement-cards-rust-dsl)

---

## ~~BT21-024 — Opponent security count condition  [G-OPP-SECURITY-COUNT-LTE]~~ — RESOLVED 2026-05-17 (Phase 2 Track G)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track G closure" entry.
`PredicateSpec.opponent_security_count_lte: Option<DpConstraint>` and the
`_gte` sibling now compile through `CompiledPredicate` and evaluate
against `rctx.security_count(rctx.opponent_id())`. BT21-093 cost-reduction
clause migrated to use the new predicate; BT21-024's negative-condition
test was already passing through the `count_lte` aggregate over
`{ zone: [security], owner: opponent }`.

[ORIGINAL ENTRY BELOW]

- Effect text: "[On Play][When Digivolving] If your opponent has 5 or fewer security cards, they place 1 card from their hand as the bottom security card. Then, trash their top security card."
- Missing DSL verb / step kind / predicate: `opponent_security_count_lte` — a `PredicateSpec` / `BoolPredicate` leaf that checks the OPPONENT's (not controller's) security stack count. The existing `security_count_lte: u8` field in `PredicateSpec` evaluates `rctx.security_count(rctx.player)` (controller's security). No `of:` field exists on the predicate to redirect the player lookup. A separate `opponent_security_count_lte: Option<u8>` field is needed.
- Lowers to engine API: `rctx.security_count(rctx.opponent())` — `security_count(player_id)` already exists on `EffectReadContext`. The gap is that the predicate evaluator has no branch to call it with the opponent ID.
- Suggested DSL syntax:
  ```yaml
  condition:
    opponent_security_count_lte: 5
  ```
  Alternatively, extend `security_count_lte` to accept an `of:` modifier:
  ```yaml
  condition:
    security_count_lte: { count: 5, of: opponent }
  ```
- Gap kind: dsl (engine primitive exists; predicate evaluator just needs the branch and an `of:` routing parameter or a sibling field).
- Workaround: Clause runs unconditionally (matching DCGO behavior where `trash_top_security` runs outside the inner `if (SecurityCards.Count <= 5)` block). The condition gates only the `select_hand` + `place_on_security` sub-step in DCGO. Negative condition test is `#[ignore = "pending: G-OPP-SECURITY-COUNT-LTE"]`.
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## ~~BT21-024 — Outer-tail continuation lost when `select_hand` has no candidates  [G-SELECT-EMPTY-OUTER-TAIL]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The `select_hand` empty-candidate path now drains the outer tail —
  when `install_select_hand` finds no valid candidates it runs the parked outer-tail steps
  (e.g. `trash_top_security`) instead of silently discarding them. Landed in the
  `complete-dna-omnimon-archetype` change; the empty-hand behavioral test is re-enabled and
  passing. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion closure —
  2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: "[On Play][When Digivolving] ... Then, trash their top security card." — the `trash_top_security` step after `as_selecting_player` must fire even when the opponent has no hand cards.
- Engine gap: `install_select_hand` in `code/digimon-engine/src/effect_context/selections.rs` (lines 177–179) returns early without installing a `PendingSelection` when `valid_action_ids.is_empty()` (opponent has no hand cards). When this early-return fires, no selection callback is ever installed, so `drain_dsl_outer_tail` (which is called from the selection callback in `selections.rs:47`) is never executed. Steps that `park_outer_tail` placed after the `as_selecting_player` block — specifically `trash_top_security` — are silently discarded.
- Root cause: the outer-tail drain relies on the inner select completing through its callback. An empty-selection skip short-circuits before the callback is installed.
- Lowers to engine API: no new method needed. Fix options: (1) in `install_select_hand`, when `valid_action_ids.is_empty()` and the call is not optional, immediately call `drain_dsl_outer_tail(ctx)` before returning; (2) alternatively, make the outer-tail drain happen in the park/skip path rather than only in the callback; (3) add an `on_skip` path analogous to `on_decline` for optional selections that fires the continuation.
- Suggested fix path: option (1) — cheapest, no new API surface:
  ```rust
  if valid_action_ids.is_empty() {
      // No candidates: skip the selection but still drain the outer tail.
      drain_dsl_outer_tail(ctx);
      return;
  }
  ```
- Gap kind: engine (the DSL YAML is correctly structured; the lowering engine loses the continuation in the empty-hand case).
- Workaround: Test for the empty-hand case is `#[ignore = "pending: G-SELECT-EMPTY-OUTER-TAIL"]`. In practice, the YAML behavior deviates from printed card text only when the opponent has an empty hand (rare competitive scenario).
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## ~~BT17-018 — `lose_count_bound` step verb (count-driven security trash loop)~~  [G-LOSE-COUNT-BOUND] — RESOLVED 2026-05-22

- **Resolved** by adding an optional `count: FormulaSpec` field to the existing
  `trash_top_security` verb (`TrashTopSecurityArgs` in `digimon-dsl/src/step.rs`).
  The engine handler (`step/draw.rs`) evaluates the formula and loops
  `trash_top_security` that many times, bailing early when the stack empties.
  A dedicated `lose_count_bound` / `repeat_n` combinator was not needed — the
  `count` field on the existing verb is the smaller surface. BT17-018's
  `[When Attacking]` clause now ships as pure DSL:
  ```yaml
  - trash_top_security:
      of: opponent
      count:
        floor_div:
          - { base: 0, per: { card_count_in_zone: { of: any, zone: trash } }, delta: 1 }
          - 10
  ```
  raw_rust `bt17_018_trash_security_per_ten_trash` removed.
- First reported: 2026-04-27 (BT17-018 batch-implement-cards-rust-dsl)

---

## Royal Knights — `on_option_placed` timing lowerer  [G-OPTION-PLACED-TIMING]

- Effect text: `BT13-007` King Drasil_7D6 inherited: "[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory."
- Missing DSL verb / step kind / predicate: `when: on_option_placed` is accepted by the DSL compiler as `CompiledTiming::OnOptionPlaced`, but the Rust engine timing map returns `None` for it, so no `EffectTiming` is emitted and no clause can fire.
- Companion engine gap: the Rust engine has no `EffectTiming::OnOptionPlaced` dispatch site when a Delay/Training/field Option is placed in the battle area. `BT13-110` Royal Knights of the Purge and `BT20-100` The Last Guardian both make this timing matter for the Royal Knights loop.
- Lowers to engine API: needs a new `EffectTiming::OnOptionPlaced` (or equivalent observer timing) plus a dispatch after Option placement in `Game::dispose_option` / option placement helpers. The trigger context should identify the placed Option card and controller so `event_card_trait_has: "Royal Knight"` can be evaluated.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: on_option_placed
    active_when: { in_breeding: true }
    once_per_turn: true
    condition: { event_card_trait_has: "Royal Knight" }
    process:
      - gain_memory: 1
  ```
- Gap kind: hybrid (DSL has the token but no lowering target; engine lacks the timing dispatch).
- Workaround: None faithful. The memory-gain trigger is omitted at runtime.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-04-29: `when: on_option_placed` now lowers to `EffectTiming::OnOptionPlaced`, and Delay-style Option placement through `Game::play_option_from_hand` supplies the placed Option through `event_card` / `event_permanent`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`.
- Updated 2026-05-02: Group 5 Task 4 covers Link, Training, inherited/security self-placement, and top-card plus inherited breeding-area observer fan-out for `OnOptionPlaced`, with placed Option context available via `event_card` and Link host context via `event_host_permanent` / `event_host_card`. Link placement resumes `OnLink` after placed-option selections settle, and breeding-source `max_per_turn` accounting is covered for this queued observer path. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card link_on_option_placed_selection_resumes_on_link_after_choice_resolves on_option_placed_scans_inherited_sources_under_breeding_top_card once_per_turn_breeding_on_option_placed_observer_fires_once_not_zero`. Transient Standard options remain open because they are not battle-area placements.

---

## Royal Knights — selecting permanents in the breeding area  [G-BREEDING-PERMANENT-SELECTION]

- Effect text: `BT20-083` Omekamon: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area." Similar Royal Knights effects target or play from the breeding-area King Drasil stack (`BT13-093`, `BT13-110`, `BT13-112`, `EX11-053`, `BT23-072`).
- Status: selection is resolved; effect movement support is partially resolved. `select_own_breeding_permanent` now installs a breeding-specific pending selection and binding without fake battle-area handles. Group 4 also lets `place_as_bottom_source` target the real breeding slot via `BREEDING_TARGET`.
- Companion engine state: `SelectionKind::BreedingPermanent`, `BreedingPermanentSelectionRef`, and phase-scoped breeding select actions cover the player-visible choice. `EffectContext::move_from_breeding_by_effect` and `play_to_breeding_from_hand` cover direct effect movement to/from the real breeding slot.
- Lowers to engine API: `select_own_breeding_permanent` for the choice, `place_as_bottom_source` for tucking under the selected breeding stack, and source-parametric `effect_initiated_digivolve` for non-hand result cards once a source binding is available.
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: kd
      filter:
        all_of:
          - name_is: "King Drasil_7D6"
          - zone: [breeding]
      prompt: "Choose your King Drasil_7D6 in breeding"
  ```
  Alternatively, add an explicit sugar step:
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
  ```
- Gap kind: hybrid (the YAML shape exists, but lowering/runtime selection ignore breeding).
- Workaround: None faithful. Auto-targeting the only breeding permanent would hide a player-visible selection and violates the no-approximations policy.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-05-02: remaining open follow-ups are breeding-area trigger fan-out (`G-BREEDING-TRIGGER-DISPATCH`) and card-specific optional/filter wrappers, not the basic breeding selection or real-zone movement primitives.
- Updated 2026-05-08: Track A resolved the security-removal breeding fan-out slice: `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` scan the observer player's breeding slot through the existing top-card/inherited breeding enqueue path and carry the `TriggerSource::SecurityRemoved` payload. This narrows BT20-083 to its printed body support: suspend a breeding carrier as the cost and play an [Omekamon] from the selected breeding stack's materials without paying the cost. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload`.

---

## ~~P-130 — effect move-from-breeding DSL verb~~  [G-MOVE-BREEDING-DSL] — RESOLVED 2026-05-23

Moved to [`qa/resolved-gaps.md`](resolved-gaps.md). `move_from_breeding` now lowers to `EffectContext::move_from_breeding_by_effect`, and `select_own_breeding_permanent` supports a level filter plus optional accept/decline prompt for P-130's printed `[On Play]` clause.

---

## BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Status: RESOLVED for reusable formula-zone count filters on 2026-05-02. `card_count_in_zone` payloads now accept `filter: { ... }`; the compiler carries the predicate into filtered count IR, and runtime evaluation counts only representable subjects that satisfy the predicate instead of falling back to an unfiltered count.
- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Implemented DSL form: `card_count_in_zone` formulas can now apply a `kind: digimon` filter. `BT8-097.yaml` uses this filtered form so Tamers and Option permanents no longer reduce Crimson Blaze's play cost.
- Lowers to engine API: the engine can inspect each battle-area permanent and test `Permanent::is_digimon(&card_data)`; the formula DSL needs a filtered-count form that passes a compiled predicate into formula evaluation.
- Suggested DSL syntax:
  ```yaml
  amount_fn:
    base: 0
    per:
      card_count_in_zone:
        of: opponent
        zone: battle_area
        filter: { kind: digimon }
    delta: 1
  ```
- Gap kind: resolved dsl vocabulary/evaluator gap for filtered zone-count formulas.
- Workaround: no longer needed for BT8-097 or other `card_count_in_zone` formulas with simple predicate filters.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch phase3d_formula_zone_count`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097`.

---

## AD1-012 — `on_opponent_attack` Timing variant on triggered clauses  [G-DSL-ON-OPPONENT-ATTACK]
- Effect text: AD1-012 CresGarurumon: "[Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, you may change the attack target to 1 of your Digimon."
- Status (2026-05-08): closed. `on_opponent_attack` parses, compiles to `CompiledTiming::OnOpponentAttack`, maps to `EffectTiming::OnOpponentAttack`, and is dispatched from the combat flow. Coverage includes `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- parse_clauses phase2a_triggered` and existing combat timing tests.
- Previous missing DSL verb / step kind / predicate: `Timing::OnOpponentAttack` variant on `digimon_dsl::clause::Timing` (`code/digimon-dsl/src/clause.rs:83-125`); no mapping in `compile_timing` (`code/digimon-dsl/src/compile.rs:173-216`).
- Lowers to engine API: `Effect::on_opponent_attack` (`code/digimon-engine/src/effect.rs:427`) — engine timing dispatch already handles `EffectTiming::OnOpponentAttack` (`lower_triggered.rs:181`) and the combat state machine fires it (`combat.rs:2237-2242`). The hybrid declared-attack-observer engine slice closed 2026-04-29 unblocks the engine half; DSL just lacks the timing token.
- Suggested DSL syntax:
  ```yaml
  - when: on_opponent_attack
    active_when: { opponents_turn: true }
    once_per_turn: true
    optional: true
    process: [...]
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2a_triggered parse_clauses`.
- Implementation: add `Timing::OnOpponentAttack` variant + serde wiring + `compile_timing` arm; the existing `lower_triggered.rs` already routes `EffectTiming::OnOpponentAttack`, so no new lowering code needed.
- Gap kind: dsl, closed. AD1-012's Opp-Turn clause remains blocked by the defender-side effect DNA route into Omnimon Alter-S (and the separate redirect-attack-target step), not by this timing token.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## AD1-012 — `redirect_attack_target` step verb  [G-DSL-REDIRECT-ATTACK-TARGET]
- Effect text: AD1-012 CresGarurumon (sub-step of the Opp-Turn clause): "Then, you may change the attack target to 1 of your Digimon."
- Previous missing DSL verb / step kind / predicate: No `redirect_attack_target` entry in the `StepSpec` enum / serde tag table at `code/digimon-dsl/src/step.rs`. No `CompiledStep::RedirectAttackTarget` variant.
- Status (2026-05-07): closed for bound permanent and player retargets. `redirect_attack_target` now parses, compiles, and lowers to `ctx.redirect_attack`, supporting `new_target: <binding>` and `player: you|opponent|active`. Runtime coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target`.
- Lowers to engine API: `EffectContext::redirect_attack(new_target_perm)` (`code/digimon-engine/src/effect_context/mod.rs:3099`) — exists and is used by hand-written cards (BT22-061, EX11-042, P-094 in legacy Python).
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: redirect_target
      optional: true
      filter: { kind: digimon }
      prompt: "Change attack target to 1 of your Digimon"
  - redirect_attack_target: { new_target: redirect_target }
  ```
- Implementation: add `StepSpec::RedirectAttackTarget { new_target: BindingRef }` + serde + `CompiledStep` variant + lowering arm in `dsl_cards/step/combat.rs` that resolves the binding to a `PermanentHandle` and calls `ctx.redirect_attack(perm_handle)`.
- Gap kind: dsl, closed. AD1-012 Opp-Turn redirect substep is now blocked by the effect DNA setup before it, not by the redirect verb.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## Effect-created attack verbs — `force_attack` / `cancel_attack` / `open_counter_window`  [G-DSL-FORCE-CANCEL-ATTACK]
- Missing DSL verb / step kind / predicate: Several audit notes used placeholder names such as `force_attack_now` or omitted attack cancellation bodies because only engine-side helpers existed.
- Status (2026-05-08): closed for immediate effect-created forced attacks, legal-window attack cancellation, and the named Counter-window bridge. `force_attack` parses/compiles/lowers to `ctx.force_opponent_attack(...)`; `cancel_attack: {}` parses/compiles/lowers to `ctx.cancel_pending_attack()`; `open_counter_window: {}` parses/compiles/lowers to `ctx.open_counter_window()` and reuses the normal Counter pending-selection scan. BT20-102 now uses `force_attack` + `without_suspending: true` for its DCGO-matched optional-trigger/mandatory-attack flow. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- force_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- cancel_attack`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- open_counter_window_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_102`.
- Supported DSL syntax:
  ```yaml
  - force_attack:
      attacker: forced
      targets: player # any | player | digimon
      without_suspending: true
  - cancel_attack: {}
  - open_counter_window: {}
  ```
- Remaining caveat: card YAML that used old commented placeholder names still needs card-specific rework.

---

## BT15-101 — Self-target predicate for event triggers (`event_target_is_source`)  [G-DSL-EVENT-TARGET-IS-SELF]
- Effect text: BT15-101 MetalGarurumon: "[All Turns] [Once Per Turn] When this Digimon becomes suspended, you may unsuspend it."
- Missing DSL verb / step kind / predicate: No `event_target_is_source` (or equivalent `event_target_is_self`) BoolPredicate leaf that evaluates whether the suspended/affected permanent equals the source permanent. The existing event predicates (`event_target_owner`, `event_target_kind`, `event_target_trait_has`) only inspect the target's owner/kind/traits. The DSL `equals: [...]` predicate compares only integers (literals + integer bindings via `Bindings::get_literal`) — it cannot compare permanent handles.
- Lowers to engine API: `event_target_card(rctx)` already returns the `CardHandle` of the suspended permanent's top card; `rctx.source_permanent` carries the source permanent handle. A new predicate could compare `current_trigger_context.event_permanent` against `rctx.source_permanent_handle()`.
- Suggested DSL syntax: add `event_target_is_source: bool` BoolPredicate leaf evaluating `rctx.game.current_trigger_context?.event_permanent == Some(rctx.source_permanent_handle()?)`.
  ```yaml
  - when: on_suspend
    active_when: { all_turns: true }
    once_per_turn: true
    optional: true
    condition: { event_target_is_source: true }
    process:
      - unsuspend: { target: source }
  ```
- Implementation: add `event_target_is_source: Option<bool>` to `PredicateSpec`, compile to a new `CompiledPredicate` field, evaluate inside `eval_event_fields` in `dsl_cards/predicate.rs`.
- Updated 2026-05-08: Implemented under the clearer name `event_permanent_is_source: true`, comparing `TriggerContext.event_permanent` to the observer's `source_permanent`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and BT23-077's card-shaped fixture. BT15-101 still needs card-local YAML/test adoption before this card entry can be closed.
- Gap kind: dsl. Engine has the comparison primitive (handles are equality-comparable).
- Workaround: AD1-014 pattern (`event_target_owner: you, event_target_kind: digimon`) — over-fires when ANY of the controller's Digimon (allies) suspend, so OPT may be consumed at the wrong moment and a "may unsuspend" prompt may appear when the source is not actually suspended. Faithful for "any of your Digimon"-style triggers (AD1-014, BT13-012); approximation-only for "this Digimon" triggers (BT15-101).
- First reported: 2026-05-03 (BT15-101 batch-implement-cards-rust-dsl)

## BT21-102 — `on_ally_attack` / `on_opponent_attack` timings missing from DSL
- Effect text: BT21-102 Tai Kamiya — "[Your Turn] When one of your Digimon attacks, by suspending this Tamer, ＜Draw 1＞."
- Status: resolved for the timing tokens. `on_ally_attack` and `on_opponent_attack` parse, compile, and lower to the engine timings.
- Former missing DSL verb / step kind / predicate: `digimon_dsl::clause::Timing` enum (`code/digimon-dsl/src/clause.rs`) did not include `OnAllyAttack` or `OnOpponentAttack`, making the engine mappings unreachable from YAML.
- Lowers to engine API: `Effect::on_ally_attack(card)` / `Effect::on_opponent_attack(card)` already exist (`code/digimon-engine/src/effect.rs` line 421+).
- Suggested DSL syntax:
  ```yaml
  - when: on_ally_attack
    optional: true
    active_when: { your_turn: true }
    process:
      - suspend: { target: source }
      - draw: { of: you, count: 1 }
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2a_triggered parse_clauses`.
- Gap kind: resolved DSL timing surface. Card-local YAML can now use the faithful timing token instead of the `when_attacking` workaround.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl)

## BT21-102 / BT15-096 — `play_cost_lte` formula-valued / binding-relative variant  [G-DSL-DISTINCT-TAMER-COLORS-FORMULA]
- Effect text: BT21-102 Tai Kamiya — "[Main] [Once Per Turn] You may play 1 [ADVENTURE] or [Hero] trait card with a play cost of 2 or less from your hand without paying the cost. For each of your Tamers' colors, add 1 to this effect's play cost maximum."
- Effect text: BT15-096 Supreme Connection! — "[Delay] 1 of your Digimon with the [Machine] or [Cyborg] trait may play 1 Digimon card with a play cost less than or equal to that Digimon's play cost from your hand with the play cost reduced by 3."
- **Status: RESOLVED 2026-05-17** (Phase 2 Track A finalization; formula primitive landed 2026-05-10). Phase 2 Track A swept stale references and confirmed coverage in `tests/dsl/group7_predicate_batch.rs` + `tests/dsl/group7_formula_batch.rs`. The companion BoolPredicate wrapping `G-DSL-DISTINCT-TAMER-COLORS` (ST20-10 disjunct) is closed by the same formula leaf — `play_cost_lte: { formula: { distinct_colors_count: ... } }` covers both shapes.
- Status (legacy): RESOLVED on 2026-05-10. `PredicateSpec::play_cost_lte` now accepts either the legacy literal threshold or `{ formula: ... }`. Formula thresholds compile through `CompiledDpConstraint`, evaluate during selection-mask construction, and can read `binding_play_cost` from a previously selected card/permanent binding. BT21-102's color-scaled cap is also covered by `distinct_colors_count`.
- Lowers to engine API: `card.play_cost <= rctx.eval_formula(formula)` — engine already has formula evaluation and per-card play_cost reads.
- DSL syntax:
  ```yaml
  filter:
    play_cost_lte:
      formula:
        base: 2
        per:
          distinct_colors_count:
            of: you
            zone: [battle_area]
            filter: { kind: tamer }
        delta: 0
  ```
- Binding-relative syntax:
  ```yaml
  filter:
    play_cost_lte:
      formula:
        binding_play_cost: source_digimon
  ```
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_formula_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_102 -- --nocapture`.
- Gap kind: dsl. Companion to G-DSL-DISTINCT-TAMER-COLORS-FORMULA for BT21-102; independently blocks BT15-096's Delay clause.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl). Binding-relative variant reaffirmed 2026-05-10 (BT15-096 Supreme Connection!, Alter-S Ladder batch).

## EX9-066 — Binding-presence predicate (`binding_present`/`binding_absent`)  [G-DSL-BIND-PRESENT]
- Effect text: EX9-066 Tai Kamiya & Matt Ishida — "[On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or [Omnimon] in its name from your trash to the hand. If this effect didn't return, ＜Draw 1＞." Also EX11-074 — "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon, ..."
- Status: NARROWED on 2026-05-10. The pure binding-presence predicate primitive is implemented as `binding_present` / `binding_absent` plus aliases `binding_is_present` / `binding_is_none`, compiled to `CompiledPredicate`, and evaluated against the threaded `Bindings`. This does not close richer result-log predicates such as "this effect suspended your Digimon" when the mutation itself must be distinguished from a selected target.
- Former missing DSL verb / step kind / predicate: no `binding_present: <name>` or `binding_absent: <name>` BoolPredicate leaf that evaluates whether a prior `bind_as:` step (e.g. an optional `select_trash` / `select_hand` / `select_own_permanent` that the player may have declined) actually produced a value. The existing `equals: [<binding>, <literal>]` compare on `CompiledBindingCompare` only supports integer-valued bindings (literals + integer bindings via `Bindings::get_literal`) — it cannot distinguish a permanent/card binding that was set vs absent.
- Lowers to engine API: `Bindings::get_card(name).is_some()` / `Bindings::get_permanent(name).is_some()` / `Bindings::get_literal(name).is_some()` — engine already has these read paths through `digimon_dsl::compiled::Bindings` and `effect_context::Bindings`.
- Suggested DSL syntax:
  ```yaml
  - select_trash:
      bind_as: pick
      optional: true
      filter: { ... }
  - if:
      condition: { binding_present: pick }
      then: [ add_to_hand_from_trash: { card: pick } ]
      else: [ draw: 1 ]
  ```
- Implementation: added `binding_present: Option<String>` and `binding_absent: Option<String>` BoolPredicate leaves to `PredicateSpec`, compile to `CompiledPredicate` fields, and evaluate inside `eval_predicate_with_bindings` in `dsl_cards/predicate.rs` by checking the named binding in the threaded `Bindings`.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`.
- Gap kind: dsl. Engine has the comparison primitive (binding presence is a trivial Option check).
- Workaround used in EX9-066: drop the binding-result check entirely; present a binary `select_effect_choice [Return / Draw]` so the player explicitly picks the branch up front. The Return branch's inner `select_trash` is `optional: true` so it degrades gracefully when no eligible cards exist. Case C (no eligible card + player picked Return) becomes a no-op rather than a forced draw — diverges from DCGO but the action mask still surfaces the Decline → Draw alternative, so a faithful RL agent learns to pick Decline in case C. No auto-selection is performed on the agent's behalf; the no-approximations policy is preserved.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## BT24-008 / EX9-066 — General `count_gte` / `count_lte` predicate not evaluated  [G-COUNT-GTE-NOT-EVALUATED] — RESOLVED 2026-05-17 (Phase 2 Track A)
- **Status:** Closed. `eval_predicate_with_bindings` now consults `count_gte` / `count_lte` via a generalized `count_matching_in_zone` walker. See `qa/resolved-gaps.md` § "Phase 2 Track A closure" for full details. BT24-008 / EX9-066 / EX1-021 chained-`if` workarounds are now substrate-correct.
- Effect text: BT24-008 Lv4 Reptile/Dragonkin/LIBERATOR — "[On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR] trait from your hand, <Draw 2>." (condition gates on `count_gte` over hand). EX9-066 — needs gating on `count_gte` over trash zone for the trash-or-draw branch.
- Status (legacy): OPEN (filed 2026-05-03 during EX9-066 batch-implement-cards-rust-dsl). Previously documented inline in BT24-008.yaml header but not as a standalone gap entry.
- Missing engine evaluation: `PredicateSpec::count_gte: Option<CountAggregate>` and `count_lte: Option<CountAggregate>` parse correctly into `CompiledPredicate.count_gte` / `count_lte` (`compiled.rs` lines 223-224), but `dsl_cards/predicate.rs::eval_predicate_with_bindings` does NOT consult these fields — only the specialized `security_count_gte` / `security_count_lte` (predicate.rs lines 73-82) and `materials_count_gte` / `materials_count_lte` (predicate.rs lines 834-842) are wired. So `condition: { count_gte: { filter: ..., n: 1 } }` is a no-op that always evaluates as TRUE, which means `if count_gte ≥ 1 then [...] else [...]` always takes the `then` branch regardless of the actual card count.
- Lowers to engine API: needs a generic `count_matching_in_zone` walker that takes a `CompiledPredicate` filter (with `zone:` constraints) and counts matches across the named player's hand / trash / battle_area / security / deck. The existing `existential_any` walker (predicate.rs:279) only iterates `battle_area` and stops at first match — needs to be generalized to iterate the requested zones and count instead of short-circuit.
- Suggested DSL syntax (already accepted by the parser — only evaluation is missing):
  ```yaml
  condition:
    count_gte:
      filter:
        of: you
        zone: [trash]
        kind: digimon
        any_of:
          - name_contains: "Greymon"
          - name_contains: "Garurumon"
          - name_contains: "Omnimon"
      n: 1
  ```
- Implementation: add a `count_in_zones(filter: &CompiledPredicate, target: PlayerRef, rctx, bindings) -> u32` helper in `dsl_cards/predicate.rs` that iterates the player's hand / trash / battle_area / security / deck per the filter's `zone:` field and counts matches via per-card / per-permanent predicate evaluation. Then check `count >= agg.n` (gte) / `count <= agg.n` (lte) inside `eval_predicate_with_bindings`.
- Gap kind: engine evaluation gap (DSL surface complete; runtime evaluation missing).
- Workaround used in EX9-066: drop the count_gte pre-gate entirely; always present the binary [Return / Draw] choice and rely on the inner `select_trash` being `optional: true`. Acceptable because the action mask still surfaces both branches faithfully. BT24-008 has the same pending workaround documented in its YAML header.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## ~~BT22-017 — `text_contains` (effect-text scan) predicate  [G-DSL-PREDICATE-TEXT-CONTAINS]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The `effect_text_contains` predicate leaf landed in the
  `complete-dna-omnimon-archetype` change; it scans a candidate's printed
  effect/inherited/security text by case-insensitive substring, lowering through
  `CompiledPredicate`. BT22-017's bucket-1 filter now uses `effect_text_contains: "Omnimon"`
  and the `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text` test is
  re-enabled and passing. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion
  closure — 2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: BT22-017 [On Play] "Reveal the top 3 cards of your deck. Add 1 card with [Omnimon] in its TEXT and 1 card with the [CS] trait among them to the hand."
- Missing DSL verb / step kind / predicate: `text_contains: Option<String>` leaf on `predicate::PredicateSpec`. The DSL exposes `name_contains` / `name_is` / `name_in` for card-name scans, but has no leaf that scans a candidate's printed `effect_text` / `inherited_text` / `security_text`. DCGO uses `source.HasText("Omnimon")` (BT22_017.cs line 63) which scans the card's effect text for the literal substring.
- Engine data IS present: `code/digimon-engine/src/card_data.rs` carries `effect_text`, `inherited_text`, and `security_text` fields on `CardData` (lines 87, 99, 124). Only the DSL predicate verb is missing.
- Lowers to engine API: a new `text_contains` leaf compiled through `CompiledPredicate` and evaluated in `dsl_cards/predicate.rs` by case-insensitive substring scan against the candidate's combined text. The existing `name_contains` evaluator at `dsl_cards/predicate.rs:705` is the lookalike to clone.
- Suggested DSL syntax:
  ```yaml
  filter:
    text_contains: "Omnimon"
  ```
- Approximation used in BT22-017 today: `name_contains: "Omnimon"`. Narrows correctly for printed Omnimon-named cards (BT12-085, BT22-015, etc.) because their card_name itself carries "Omnimon", but WRONGLY excludes cards that mention `[Omnimon]` only in their effect_text without carrying it in their name (e.g. tutors / supports printed "search for [Omnimon]"). Faithfulness divergence is asserted-and-#[ignore]'d in `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text`.
- Also blocks: any future card whose printed text uses an `in its text` (rather than `in its name`) bucket-filter — including BT12-059's bucket 1 if it were to switch from name-based to text-based per a future erratum.
- Gap kind: DSL vocabulary gap (engine data is present; no DSL surface to filter on it).
- First reported: 2026-05-03 (BT22-017 Gabumon, batch-implement-cards-rust-dsl)

## EX1-068 — grant a triggered effect to opponent's permanent  [G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT] — RESOLVED 2026-05-29

Closed by change `add-grant-triggered-effect-dsl`. The `grant_triggered_effect`
step + `ModifierType::GrantedTrigger` slot already existed (EX10-034 grant-to-
binding work); the remaining slice was (a) opponent-set targeting — a predicate
`target: { of: opponent, ... }` already walks both battle areas and snapshots the
match set, and (b) cause attribution for the `<Progress>` interaction — the
granted-trigger dispatch (`enqueue_from_permanent`) now skips firing when the
carrier is unaffected by the GRANTOR's effects (`progress_excludes`). EX1-068's
`[Main]` clause is authored; judge-quiz **Q2** pins it (Medusamon `<Progress>`
loses no memory; a non-Progress control loses 2). Full note in
`qa/resolved-gaps.md`. **Q16 also closed (2026-05-29):** EX6-057 Lilithmon
authored; the granted body now runs as the carrier's OWN effect (D4/DCGO —
sourced from `selectedPermanent.TopCard`), so its granted `[EoT] Delete this` is
OwnEffect and `<Partition>` skips it (judge-quiz Q16 PASS). **Q17 also closed
(2026-05-29):** the granted-trigger dispatch also gates on
`permanent_is_unaffected_by_effect`, so a carrier immune to the grantor's
effects (Magnamon X BT16-102's "isn't affected by your opponent's effects")
suppresses the granted clause (judge-quiz Q17 PASS). All three directions
(Q2/Q16/Q17) are now resolved; full entry in `qa/resolved-gaps.md`.

[ORIGINAL ENTRY BELOW]

- Effect text: EX1-068 [Main] "All of your opponent's Digimon gain '[When Attacking] lose 2 memory' until the end of their next turn."
- Missing DSL verb / step kind / predicate: A `grant_triggered_effect` step that installs a NEW triggered clause (timing + process body) on a SET of cross-permanent targets with a turn-scoped expiry. The DSL today exposes grants for STATIC effects only — `grant_keyword`, `add_modifier` / `add_dp_modifier`, `grant_effect_immunity`. None of those install a clause that itself fires on a future trigger (`when_attacking`, `when_digivolving`, `on_deletion`, ...) on the granted permanent.
- Engine substrate: the Python engine handles this via `permanent.grant_temp_effect(effect, expiry_turn)` + `clear_expired_effects()` (see `qa/archetype-qa/engine-gaps.md` line 33, RESOLVED 2026-03-14 in Python). The Rust engine has the modifier-registry + expiry-tick substrate (`ModifierRegistry` carries per-permanent typed modifiers with `Expiry`), but it does NOT carry a typed `GrantedTriggeredEffect` slot, and there is no `CompiledStep::GrantTriggeredEffect`.
- Lowers to engine API: needs (a) a new `ModifierRegistry` slot (or sibling registry) for per-permanent granted clauses with expiry; (b) the runtime clause dispatcher to consult granted slots when firing a timing on a permanent; (c) a `CompiledStep::GrantTriggeredEffect` whose payload is an inline `CompiledTriggeredClause` (or a registry-keyed template name) lowered against the granted permanent, NOT the source permanent.
- Suggested DSL syntax (option A — inline body):
  ```yaml
  - grant_triggered_effect:
      target:
        of: opponent
        zone: [battle_area]
        kind: digimon
      when: when_attacking
      process:
        - lose_memory: 2     # affects the granted permanent's controller
      expiry: end_of_opponents_turn
  ```
  (Option B — named template: `grant_named_effect: { id: "MemoryMinus2WhenAttacking", target: ..., expiry: ... }` with templates living in a new `code/digimon-engine/src/cards/granted_effects/` registry.)
- Approximation that would VIOLATE no-approximations: a clause that subtracts 2 memory whenever the opponent declares any attack within the expiry window. This over-fires on opponent Digimon played AFTER this Option resolves (DCGO's per-Permanent foreach loop runs ONCE at resolution time and snapshots the eligible Digimon set, so a Digimon played later does not carry the granted clause). Per no-approximations, EX1-068's [Main] clause is OMITTED entirely until the gap closes.
- Also blocks: any "[Main|On Play|When Digivolving] all (your|opponent's) Digimon gain '<bracketed-timing> <body>' until <expiry>" card text. DCGO grep for `UntilOpponentTurnEndEffects.Add` and `UntilOwnerTurnEndEffects.Add` returns ~20+ cards across sets — examples include several Memory-control Options and Tamer support effects across blue/yellow/black.
- Companion engine gap: tracked in `qa/archetype-qa/engine-gaps.md` line 33 as RESOLVED for Python; OPEN for the Rust engine's modifier registry.
- Gap kind: hybrid (Rust engine modifier registry needs a typed grant slot; DSL needs the verb + lowering).
- First reported: 2026-05-03 (EX1-068 Ice Wall!, batch-implement-cards-rust-dsl)
- Judge-quiz consumer (2026-05-28): **Q2** of the judge-quiz faithfulness suite (`add-judge-quiz-faithfulness-suite`) is BLOCKED on this gap. Q2 stages Medusamon (BT24-017) `<Progress>` against the Ice-Wall-granted "[When Attacking] lose 2 memory" and asserts NO memory loss. The Progress half is implemented (`Game::progress_excludes`, combat.rs:2667); only this grant primitive is missing. Test `a_immunity_scope::q2_medusamon_progress_blocks_ice_wall_memory_loss` is `#[ignore]`-blocked citing this gap. When closed, the suite gains a Progress-vs-granted-effect immunity assertion for free.

## EX1-021 — Formula-valued `gain_memory` step  [G-DSL-GAIN-MEMORY-FN] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry for
the closure summary. `gain_memory_fn: { formula: ... }` + `lose_memory_fn`
ship; EX1-021 production YAML authored.

[ORIGINAL ENTRY BELOW]

## EX1-021 — Formula-valued `gain_memory` step  [G-DSL-GAIN-MEMORY-FN] (legacy)
- Effect text: EX1-021 MetalGarurumon — "[When Digivolving] Gain 1 memory for every 4 cards in your hand." DCGO: `count() = card.Owner.HandCards.Count / 4; AddMemory(count())`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `StepSpec::GainMemory(i32)` (`code/digimon-dsl/src/step.rs` line 67) is literal-only. There is no `gain_memory_fn:` variant that consumes a `FormulaSpec`. The same shape already exists for cost-reduction declarative bodies (`amount_fn:` on `kind: cost_reduction`, see BT8-097 / BT21-026 / BT24-017) — this gap is about extending the pattern to imperative `process:` steps.
- Lowers to engine API: `EffectContext::add_memory(player, n)` already accepts a runtime-computed integer. The lowering path needs to evaluate the formula via `formula_eval::evaluate_read_with_bindings(&formula, rctx, source_handle, bindings)` then pass the result to `add_memory`.
- Suggested DSL syntax:
  ```yaml
  - gain_memory_fn:
      formula:
        floor_div:
          - card_count_in_zone: { of: you, zone: hand }
          - 4
  ```
- Implementation: add `StepSpec::GainMemoryFn { formula: FormulaSpec }` + serde + `CompiledStep` variant; lowering arm in `dsl_cards/step/memory.rs` (or wherever `GainMemory` lowers today) that evaluates the formula and calls `ctx.add_memory(ctx.source_player(), result)`. Mirror the same shape for `LoseMemoryFn` for symmetry (no current cards request it, but it costs nothing to ship together).
- Workaround attempted: chained `if count_gte hand n: 4k then [gain_memory: 1]` blocks. BLOCKED at runtime by the pre-existing **G-COUNT-GTE-NOT-EVALUATED** gap — generic `count_gte` always evaluates TRUE, so the chained-`if` workaround would always award the full +N memory regardless of hand size. EX1-021 falls back to `process: []` until either gap closes.
- Also blocks: any `gain X memory for every Y of Z` printed-text family. DCGO grep for `AddMemory(.* / .*)` and `AddMemory(.*Count.*)` returns multiple cards across sets including BT5-095 (gain N where N depends on board state), several Tamer EOT memory grants tied to suspended-tamer counts, etc.
- Gap kind: dsl. Engine has `add_memory` and formula evaluation; only the DSL surface is missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX1-021 — `has_on_deletion_effect` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry.
EX1-021 production YAML authored.

[ORIGINAL ENTRY BELOW]

## EX1-021 — `has_on_deletion_effect` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT] (legacy)
- Effect text: EX1-021 MetalGarurumon — "[When Attacking] If you have 8 or more cards in your hand and a Tamer in play, return 1 of your opponent's Digimon **that has an [On Deletion] effect** to the bottom of its owners deck." DCGO: `permanent.HasOnDeletionEffect`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `PredicateSpec` has no leaf that asks "does this permanent's top card (or any card in its digivolution stack) carry a triggered effect with `EffectTiming::OnDeletion`?" The closest existing leaf is `has_keyword` (which inspects `Keyword` modifiers on the permanent, not effect timings on the underlying card data).
- Engine data IS present: each `CardData` carries the compiled `CompiledCard` (when DSL-authored) with its `effects: Vec<CompiledClause>`; the `CompiledTriggered` clauses include a `when: Vec<CompiledTiming>` that encodes `OnDeletion`. Hand-written `CardEffect` impls expose effects through `card_effects(EffectTiming::OnDeletion, &card)` returning a non-empty list. A new evaluator could walk both surfaces.
- Lowers to engine API: a new `permanent_top_or_sources_have_timing(perm, EffectTiming::OnDeletion)` walker in `dsl_cards/predicate.rs` that checks every card in the permanent's stack (top + sources) for either:
  (a) a compiled DSL clause with `CompiledTiming::OnDeletion` in `when`, or
  (b) a hand-written `CardEffect` impl whose `card_effects(EffectTiming::OnDeletion, ...)` returns non-empty.
  Per the printed text the gate is on the existence of the timing in the card's printed text, not the runtime-active effect set; checking compiled clauses + hand-written impls covers both authoring paths.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: digimon
      - has_on_deletion_effect: true
  ```
- Implementation: add `has_on_deletion_effect: Option<bool>` to `PredicateSpec` + `CompiledPredicate`. Evaluate inside `eval_permanent_fields` by walking `perm.card_sources` and consulting each card's `compiled_card` (DSL path) or registry-resolved `CardEffect` (hand-written path) for `OnDeletion`-timed clauses.
- Workaround: omit the `[On Deletion]` filter entirely. NOT acceptable per no-approximations — over-includes opponent Digimon without [On Deletion], so the player would be forced to pick a non-printed-text-eligible target. EX1-021 falls back to `process: []` until the gap closes.
- Also blocks: any "your opponent's Digimon that has an [On Deletion] effect" or "Digimon with a [When Attacking] effect" / "Tamer with a [Your Turn] effect" printed-text family. DCGO grep for `HasOnDeletionEffect` returns ~5 cards; `Has<Timing>Effect` patterns across all timings extend the impact.
- Gap kind: dsl. Engine data is present; only the DSL surface and walker are missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX4-060 / BT22-015 — Play card from own digivolution sources  [G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from this Digimon's digivolution cards without paying the costs." BT22-015 Omnimon — "<Decode (Red/Black Lv.3)> / <Decode (Blue/Yellow Lv.3)> (When this Digimon would leave the battle area other than in battle, you may play 1 [color] [level] Digimon card from its digivolution cards without paying the cost.)"
- Status: FULLY CLOSED for the reusable source-play substrate on 2026-05-20 (Track J S1.3). Filed 2026-05-03 during EX4-060 batch-implement-cards-rust-dsl; narrowed 2026-05-07, 2026-05-08; the residual multi-source / different-name DSL sugar landed 2026-05-19 (S1.2); the final breeding-carrier residual closed 2026-05-20 (S1.3). BT22-015's Decode entry is closed through a color/level-gated `select_material` plus `play_from_materials` binding, with the original leave event proceeding. EX4-060 is closed by sequential `select_material` / `play_from_materials` steps plus `place_permanent_on_security_and_handle_replacement`. EX9-021's End of Attack source plays are closed through the same source-selection path. The batch / "1 of each different name" form is closed by the `select_materials` count-capped multi-pick step. Breeding-carrier source picks (King Drasil's resident stack) are now closed by the `BREEDING_SOURCE_SELECT` action sub-range (S1.3, `ACTION_SPACE_SIZE` 2168→2192). Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_021`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- play_from_materials`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- Landed DSL verb / step kind: `select_materials` — the batch sibling of `select_material`. Picks up to N digivolution sources of a carrier permanent in ONE count-capped multi-pick; `uniqueness: name` enforces "1 of each different name". `play_from_materials` consumes the bound `CardList` as a batch (each picked source becomes a fresh permanent), composing with `suppress_on_play`.
- Engine substrate: `EffectContext::select_count_capped_multi` with `CountCappedZone::Material(PermanentHandle)` + `DistinctByMode::Name`. `select_materials` lowers straight onto it. For battle-area carriers it reuses the existing `SOURCE_SELECT` action range; for breeding-area carriers it uses the appended `BREEDING_SOURCE_SELECT` sub-range (S1.3). No new `SelectionKind` variant or `play_from_own_digivolution_cards` helper was needed.
- DSL syntax (landed):
  ```yaml
  - select_materials:
      of_permanent: <carrier-binding>  # battle-area permanent (matches select_material)
      max: 4
      uniqueness: name              # "1 of each different name"
      filter: { trait_has: "Royal Knight" }
      bind_as: picked
  - play_from_materials:
      source_index: picked          # batch — all picked sources played
      target: <carrier-binding>
      cost_delta: free
      suppress_on_play: true        # composes with the S1.1 flag
  ```
- Note: batch `play_from_materials` `bind_as` binds only the *last-played* permanent. A future card needing "do X to each played source" will require a `PermanentList` binding.
- Breeding-area carriers (CLOSED 2026-05-20, Track J S1.3): `select_materials` / `select_material` against a `BREEDING_TARGET`-sentinel carrier binding now install a real `pending_selection`. Task S1.3 appended a 24-slot `BREEDING_SOURCE_SELECT` action sub-range (`2168..2192`, keyed by carrier owner), raising `ACTION_SPACE_SIZE` 2168→2192. `material_zone_geometry` is the single branch point — battle-area carriers read `battle_area[index]`, breeding-area carriers read `breeding_area`. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier`.
- Gap kind: dsl + engine. FULLY CLOSED for battle-area AND breeding-area source plays (single, sequential, batch / different-name).
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause documented earlier under BT22-015 Decode.

## EX4-060 — Place self at bottom of own security stack face down  [G-PLACE-SELF-AT-SECURITY-BOTTOM]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, ... Then, place this Digimon at the bottom of your security stack face down."
- Status: CLOSED for EX4-060 on 2026-05-08. The DSL now has `place_permanent_on_security_and_handle_replacement`, which can target `replacement_subject`, choose top/bottom/random security placement, preserve face-down placement, trash leftover sources, and mark the active replacement custom-handled. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- Landed DSL verb / step kind: `place_permanent_on_security_and_handle_replacement`, used from a `kind: replacement` clause whose target is `replacement_subject`. Track E note: a sibling `EffectContext::place_self_at_security` (resolves `self.source_permanent` automatically) shipped on the same day for cards where the active resolver is itself the subject without needing an explicit binding; both helpers coexist.
- Closest pre-existing primitives (none of which sufficed before the new verb landed):

  - `add_this_option_to_hand: {}` — routes an Option from security-resolution staging to hand. Wrong destination zone and wrong subject scope.
  - `place_permanent_bottom_security_and_cancel_replacement` — targets ANOTHER permanent (selected via a binding) and CANCELS the replacement. Wrong subject (binding-selected, not self) and wrong outcome (cancel vs proceed-with-reroute).
- Engine substrate landed: `EffectContext::place_permanent_on_security_and_handle_current_replacement` delegates to `Game::place_permanent_on_security_without_leave_replacement`, which consumes the leaving permanent, consults `CannotAddSecurityByEffect`, places the top card into security, trashes leftover sources/linked cards, clears modifiers, and marks the replacement custom-handled. DCGO models the card-side shape via `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable(activateClass), toTop: false).PutSecurity()`.
- Replacement-outcome semantics: the step internally consumes the leave and routes the cards itself, then writes `CustomHandled` to the active replacement outcome.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_source: true
        - none_of:
            - replacement_cause: own_effect
    process:
      # ... other steps ...
      - place_permanent_on_security_and_handle_replacement:
          target: replacement_subject
          position: bottom
          face_up: false
  ```
- Workaround that would VIOLATE no-approximations: no longer needed for EX4-060.
- Also blocks: no longer blocks EX4-060. Keep this entry as a reference for any future card that needs a different timing surface from a leave-replacement body.
- Gap kind: dsl + engine, closed for the EX4-060 replacement-body form.
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl)

## ~~EX4-039 / EX4-038 — Event-target-not-source predicate for OnDigivolve  [G-EVENT-TARGET-NOT-SOURCE]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. This gap was STALE — the engine already carried both data points
  (`event_permanent` on `TriggerContext` for `Digivolved`, `source_permanent` on
  `EffectReadContext`) and the DSL predicate evaluator branch was present. The
  `complete-dna-omnimon-archetype` change authored the DNA Omnimon card clauses against
  the existing substrate and re-enabled the
  `ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves` behavioral test, which
  now passes. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon completion closure —
  2026-05-20" (STALE gaps list). Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text (both): "[Your Turn] [Once Per Turn] When one of your **other** Digimon digivolves, gain 1 memory."
- Status: OPEN as of 2026-05-03. EX4-039 surfaces it; EX4-038 has the same printed-text family.
- Missing DSL verb / step kind / predicate: a `CompiledPredicate` leaf such as `event_target_not_source: true` (or equivalently `event_permanent_not_source: true`) that returns false when the OnDigivolve trigger's `event_permanent` equals the inherited clause's `source_permanent` (the carrier permanent EX4-039 sits under). DCGO encodes this as `permanent != card.PermanentOfThisCard()` inside `CanTriggerWhenPermanentDigivolving`'s `PermanentCondition`.
- Lowers to engine API: `EffectReadContext::source_permanent()` already returns `Option<&Permanent>`; the trigger context's `event_permanent: Option<PermanentHandle>` is populated by `TriggerSource::Digivolved`. Comparing the two handles is a pure read — no new engine method needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - event_target_owner: you
      - event_target_kind: digimon
      - event_target_not_source: true
  ```
- Workaround applied today: `event_target_owner: you` + `event_target_kind: digimon`. Over-fires when the carrier permanent itself digivolves further (e.g. CARRIER-Lv4 → CARRIER-Lv5 while EX4-039 is a source under CARRIER). `once_per_turn: true` softens the impact to at most +1 spurious memory per turn. The negative-case behavioral test (`ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves`) is `#[ignore]`'d pending closure.
- Also blocks: EX4-038 Agumon (sister card, identical inherited text). Other "When one of your other Digimon ..." printed-text families across EX4 and BT5/BT12 will reuse the same predicate. DCGO grep for `permanent != card.PermanentOfThisCard()` inside `OnDigivolve` / `OnEnterFieldAnyone` PermanentCondition shows the pattern recurs across cards.
- Gap kind: dsl. Engine already has both data points (`event_permanent` on `TriggerContext` for `Digivolved`, `source_permanent` on `EffectReadContext`); only the DSL predicate surface and its evaluator branch in `eval_event_fields` are missing.
- First reported: 2026-05-03 (EX4-039 Gabumon, batch-implement-cards-rust-dsl)

## EX9-021 — `is_dna_digivolving` predicate on triggered clauses  [G-DSL-IS-DNA-DIGIVOLVING]
- Effect text: EX9-021 Omnimon Alter-S — "[When Digivolving] **If DNA digivolving**, your opponent's effects don't affect this Digimon for the turn. Then, delete all of their Digimon with the highest level." DCGO splits the body on `CardEffectCommons.IsJogress(_hashtable)` — a per-trigger hashtable flag set when the digivolve was a DNA / jogress path.
- Status: RESOLVED 2026-05-08 for the reusable event predicate under the engine/DSL spelling `dna_origin: true` / `false`. `TriggerSource::Digivolved` now carries `dna_origin`, `TriggerContext` stores it, `EffectReadContext` / `EffectContext` expose `event_dna_origin()`, and DNA digivolve drains set the bit for `WhenDigivolving`, `OnDnaDigivolve`, and global `OnDigivolve`. Effect-initiated DNA additionally sets `effect_initiated` on the global payload, so `event_is_effect_initiated` composes with `dna_origin`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3_dna_digivolve_triggers` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt17_078_when_digivolving`.
- Remaining limits: EX9-021 and BT17-078 still have card-local body gaps (`G-BIND-SELECTED-PROPERTY-FOR-EACH`, additional authored bodies, etc.), and BT16-085 still needs `G-SELECT-OPPONENT-SOURCES` for the DNA trash rider. Do not keep `G-DSL-IS-DNA-DIGIVOLVING` or the now-closed reusable self-to-security verb as the blocker for new authoring; use `dna_origin` plus the Track E zone-movement verbs.
- Missing DSL verb / step kind / predicate: `PredicateSpec` exposes no `is_dna_digivolving: bool` leaf, and the `condition:` shape on a triggered clause has no equivalent. There is also no clause-level `if:` form (matches in `process:` body) that can branch on the DNA-vs-standard digivolve origin.
- Engine substrate also missing: `TriggerSource::Digivolved { player, permanent, card }` (`code/digimon-engine/src/selection.rs:352`) has NO `via_dna` / `from_dna_pair` flag. The DNA digivolve action path (`Game::initiate_dna_digivolve` etc.) does not currently enqueue a distinct trigger source for the DNA case. The dispatch code that lifts `Digivolved { ... }` into `TriggerContext` (`effect_queue.rs` around line 479) builds a context with `event_permanent` / `event_card` / `source_player` but no DNA discriminator.
- Lowers to engine API: needs (a) `via_dna: bool` (or `dna_pair: Option<(CardHandle, CardHandle)>`) field on `TriggerSource::Digivolved`, populated from the DNA-digivolve action handler; (b) surfacing on `TriggerContext` so DSL predicates can read it; (c) DSL `is_dna_digivolving: Option<bool>` leaf on `PredicateSpec` + `CompiledPredicate` with an evaluator that consults the trigger context flag (false at non-trigger-time, same convention as `event_target_owner`).
- Suggested DSL syntax:
  ```yaml
  - when: when_digivolving
    condition:
      dna_origin: true
    process:
      - grant_effect_immunity:
          target: source
          source_kind: any
          source_controller: opponent
          expiry: end_of_turn
  ```
  (Optional symmetric dual: `is_standard_digivolving: true` for "[If standard digivolving] X" forms.)
- Workaround that would VIOLATE no-approximations: always grant the immunity (over-fires on the standard-digivolve path), or never grant it (under-fires on DNA — the printed protection is lost). Both are unfaithful. Per no-approximations the DNA-gated immunity arm is OMITTED. The unconditional delete-highest tail of EX9-021's [When Digivolving] IS implemented (printed grammar + DCGO sequencing both confirm the delete fires regardless of the DNA gate).
- Also blocks: any future card with "[When Digivolving] If DNA digivolving, X" or "[When Digivolving] If you DNA digivolved, X" style printed text. DCGO grep for `IsJogress(` returns multiple cards across sets (notably Omnimon-family / DNA-archetype cards). Sibling-but-distinct from AD1-001's `dna_origin: true` predicate, which reads card-data origin metadata rather than per-trigger event metadata.
- Gap kind: hybrid (engine TriggerSource needs the flag + dispatch wiring; DSL needs the predicate). Tests `ex9_021_when_digivolving_dna_path_grants_self_opp_effect_immunity` and `ex9_021_when_digivolving_standard_path_does_not_grant_immunity` are `#[ignore]`'d under this gap tag.
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl).

## EX9-021 — Place self at TOP of own security stack face-up  [G-PLACE-SELF-AT-SECURITY-TOP]
- Status: CLOSED for the reusable Track E DSL verb on 2026-05-09. YAML can now use `place_self_at_security: { position: top, face: up }`, lowering to `EffectContext::place_self_at_security`. EX9-021's production fixture currently uses the explicit binding form `place_permanent_on_security` because its "if this effect played" tail is already bound to the source permanent; the reusable self verb is covered by `parse_zone_movement_steps` and `zone_movement_verbs`.

[ORIGINAL ENTRY BELOW]

- Effect text: EX9-021 Omnimon Alter-S — "[End of Attack] ... If this effect played, place this Digimon as your top security card." DCGO: `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable, toTop: true).PutSecurity()` — places this permanent (top + sources) at the TOP of the controller's security stack (face-up; printed text does not specify face-down).
- Status: CLOSED for reusable DSL/security-placement vocabulary; original notes retained for provenance.
- Landed DSL verb / step kind: `place_self_at_security: { position: top|bottom|random, face: up|down }`.
- Engine substrate landed: `EffectContext::place_self_at_security(StackPosition, face_up)`.
- Suggested DSL syntax (option A — separate verbs):
  ```yaml
  - place_self_at_security_top: {}           # face-up by default
  ```
  (Option B — unified):
  ```yaml
  - place_self_at_security:
      position: top                          # top | bottom
      face: up                               # up | down (printed default
                                             # for top is up; for bottom is down)
  ```
- Workaround that would VIOLATE no-approximations: no longer needed for the reusable security-placement verb.
- Also blocks: no longer blocks future self-to-security placement syntax. Card-local source-play/result gates should be tracked separately.
- Gap kind: closed for the Track E verb.
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause tracked at `G-PLACE-SELF-AT-SECURITY-BOTTOM` (EX4-060).

## ST20-10 — Inverse alt-path direction: "this card may digivolve INTO X"  [G-ALT-PATH-DIRECTION-INTO] — RESOLVED 2026-05-17 (Phase 2 Track F)

See [resolved-gaps.md](resolved-gaps.md) "Phase 2 Track F closure" entry.
Schema + lowering + route resolution all ship. ST20-10's warp clause
remains BLOCKED on the companion `G-DSL-DISTINCT-TAMER-COLORS` predicate
leaf (the Tamer-colour disjunct of its condition); the opp-DP disjunct
is satisfiable today.

[ORIGINAL ENTRY BELOW]

## ST20-10 — Inverse alt-path direction (legacy)
- Effect text: ST20-10 Agumon — "[Your Turn] While your opponent has a Digimon with 10000 DP or more, or your Tamers have 3 or more total colors, this Digimon can digivolve into [WarGreymon] in the hand for a digivolution cost of 4, ignoring digivolution requirements." Other warp-style printed effects with the "this Digimon can digivolve into [Card] in the hand" shape are likely siblings (DCGO grep for `cardCondition: ... CardSource.EqualsCardName(...)` paired with `permanentCondition: ... == card.PermanentOfThisCard()` inside `AddSelfDigivolutionRequirementStaticEffect`).
- Status: OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `AltPathSpec` (in `digimon-dsl/src/alt_path.rs`) is implicitly source-directed — `from:` filters the SOURCE permanent / hand card that may digivolve INTO the carrier. There is no inverse form for "this card grants ITSELF the ability to digivolve into card X in hand." Authoring the alt-path on the destination card (WarGreymon's YAML) would over-broadcast: every Lv3 Agumon-named card on the field would be presented the path, and the destination YAML would have to enumerate every "warp into me" effect across the card pool. Authoring on the source (ST20-10) is the natural printed-text home but the DSL has no syntax for it.
- Lowers to engine API: the engine's activated-digivolve mechanism already supports both `cardCondition` (target hand-card filter) and `permanentCondition: target == self` (source = this card) in DCGO's `AddSelfDigivolutionRequirementStaticEffect`. The gap is purely DSL-side: a new `AltPathSpec` direction flag (or a new `kind: warp_into_hand` variant) needs to flip the semantic of `from:` to filter the destination instead of the source.
- Suggested DSL syntax (option A — direction flag):
  ```yaml
  alt_paths:
    - kind: activated_digivolve
      direction: into            # NEW: source = self, target = `into:` filter
      into:
        zone: [hand]
        of: you
        name_is: "WarGreymon"
      cost: 4
      ignore_requirements: true
  ```
  (Option B — dedicated kind): `kind: warp_into_hand` with required `into:` field (no `from:`); same lowering on the engine side.
- Workaround that would VIOLATE no-approximations: silently move the alt-path to WarGreymon's YAML (over-broadcasts to every Lv3 controller) or omit the gating predicate (path always available regardless of opp DP / Tamer colours). Per no-approximations the warp clause is OMITTED until this gap closes. Five behavioral tests in `code/digimon-engine/tests/cards_behavioral/st20/st20_10.rs` are `#[ignore]`'d under this gap tag (paired with `G-PRED-DP-LTE` or `G-DSL-DISTINCT-TAMER-COLORS`; the previously-companion `G-ALT-PATH-CONDITION` was RESOLVED 2026-05-15).
- Also blocks: any future "this Digimon can digivolve into [Card] in the hand for cost N" warp effect printed on the source card with a self-controller-state gate.
- Gap kind: dsl. Engine substrate already exists (DCGO uses the same `AddSelfDigivolutionRequirementStaticEffect` factory regardless of direction).
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Originally paired with `G-ALT-PATH-CONDITION` (BT24-016); that companion gap was RESOLVED 2026-05-15, so the inverse-direction hole is now the sole substrate blocker on this clause.

## ST20-10 — Distinct-Tamer-colours-on-field BoolPredicate  [G-DSL-DISTINCT-TAMER-COLORS] — RESOLVED 2026-05-17 (Phase 2 Track A)
- **Status:** Closed. The BoolPredicate wrapping is now covered by the formula leaf `play_cost_lte: { formula: { distinct_colors_count: { of: you, zone: battle_area, filter: { kind: tamer } } } }` shape. Phase 2 Track A swept stale references; the ST20-10 warp clause remains BLOCKED on its other companion gap `G-ALT-PATH-DIRECTION-INTO`'s ST20-specific YAML authoring (which has substrate but no card YAML yet) — but the Tamer-color disjunct itself is no longer the blocker.
- Effect text: ST20-10 Agumon — "...or your Tamers have 3 or more total colors..." (gating disjunct of the [Your Turn] warp clause). Sibling form of BT21-102 Tai Kamiya's "For each of your Tamers' colors, add 1 to this effect's play cost maximum" — both reference the same per-colour-count computation, but BT21-102 needs the value as a `FormulaSpec::per` aggregate (tracked under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) while ST20-10 needs it as a BoolPredicate threshold ("3 or more").
- Status (legacy): OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: no `distinct_tamer_colors_gte: <N>` (or generalised `distinct_colors_count_gte: <N>` over a controller / kind / zone selector) BoolPredicate leaf on `PredicateSpec`. The existing `distinct_colors_count` (added under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) is only available inside `FormulaSpec::per` — it cannot appear as a standalone boolean condition. `color_only` / `color_is` filter individual permanents by colour but do not aggregate colour counts across a permanent set.
- Lowers to engine API: DCGO's `Combinations.GetDifferenetColorCardCount(tamerCards) >= 3` returns the count of distinct colours present across the supplied permanent set, then thresholds. The engine's `eval_aggregate` (already used by `FormulaSpec::per: distinct_colors_count`) covers the count primitive — only the BoolPredicate wrapping is missing.
- Suggested DSL syntax (option A — dedicated leaf):
  ```yaml
  condition:
    distinct_tamer_colors_gte: 3
  ```
  (Option B — generalised over a permanent selector):
  ```yaml
  condition:
    distinct_colors_count:
      of: you
      zone: [battle_area]
      filter: { kind: tamer }
      gte: 3
  ```
- Workaround that would VIOLATE no-approximations: drop the disjunct entirely (gate fires only on opp ≥10000 DP, never on Tamer colours), or replace with a coarser proxy like "you have 3+ Tamers" (over-fires on three same-colour Tamers, under-fires on 3 distinct-colour Tamers some of which are deleted). Per no-approximations the entire warp clause is OMITTED until this gap (paired with `G-ALT-PATH-DIRECTION-INTO`) closes. The earlier-paired `G-ALT-PATH-CONDITION` was RESOLVED 2026-05-15.
- Also blocks: any future "while your Tamers have N or more total colours" or "if you have N or more distinct-colour Tamers" gate. Sibling to `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102) — the formula-aggregate form lands the underlying primitive; this gap closes the BoolPredicate wrapping. Both should land together once the formula primitive is generalised to also expose its result as a comparable scalar.
- Gap kind: dsl. Engine has the count primitive via `eval_aggregate`.
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Sibling of `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102).

## Puppets Resolver Residual DSL/Hybrid Gaps (2026-05-04)

## BT13-101 / P-136 — event predicates with suspend-this-Tamer cost  [PUPPETS-G023] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: `BT13-101`: "[All Turns] When you play a 2-color black/yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory." `P-136`: "[Your Turn] [Once Per Turn] When one of your Digimon digivolves into a Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). Event-card color predicates (`event_card_color_only`, `event_card_color_count`) landed, completing the second half of this gap. `BT13-101`'s All Turns observer and `P-136`'s digivolve observer are now expressible in YAML. The `bt13_101_all_turns_*` tests are un-ignored. See `qa/resolved-gaps.md` for the engine-side substrate closure.
- Status 2026-05-17: the **activation-cost half** of this gap closed under Phase 2 Track B. DSL `activation_cost: { suspend_self: true }` lifts onto `EffectBuilder::activation_cost(ctx.suspend_self_as_cost)`; cost failure (already-suspended source) consumes the OPT slot and skips the body silently (no decline-vs-fail elision). The **event-card colour predicates half** was still open at that point. See `qa/resolved-gaps.md` § Engine Gap: Generic `.activation_cost(...)` builder hook for triggered abilities for the substrate closure.
- Missing DSL verb / step kind / predicate: event-card predicates for exact color sets and color count, event-target owner/trait predicates for digivolve observers where needed, plus declarative source-bound triggered activation costs.
- Companion engine state: the generic triggered activation-cost hook is now resolved (`qa/resolved-gaps.md`); DSL `activation_cost: { suspend_self: true }` is wired and preflight comes for free via `EffectContext::suspend_self_as_cost` returning `false` on already-suspended sources.
- Suggested DSL syntax:
  ```yaml
  condition:
    all:
      - event_card_kind: digimon
      - event_card_color_only: [black, yellow]
      - event_card_color_count: 2
      # or, for P-136-style digivolve observers:
      - event_target_owner: you
      - event_card_trait_has: Puppet
  activation_cost:
    suspend_this_tamer: {}
  ```
- Gap kind: hybrid. Event-card color predicates are DSL/evaluator vocabulary; source-bound triggered cost preflight needs the engine cost surface.
- Workaround: None faithful. Name, trait, or broad color-includes filters would admit illegal cards for `BT13-101`, and auto-suspending the Tamer would hide a player-visible cost for both cards.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT13-101). Updated 2026-05-04 by Batch 11 for `P-136`.

---

## BT16-055 — narrow protection and inherited rules-text predicate  [PUPPETS-G024/PUPPETS-G025] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects." / "[Your Turn] While this Digimon has [Pulsemon] in its text, it gets +1000 DP."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). `grant_narrow_opponent_effect_protection` (PUPPETS-G024) and `rules_text_contains` predicate (PUPPETS-G025) both landed. `BT16-055` is now fully expressible in YAML. See `qa/resolved-gaps.md` for engine-side details.
- Missing DSL verb / step kind / predicate: category-scoped protection modifiers for opponent DP reduction and opponent De-Digivolve; inherited predicate over the carrier stack's printed rules text.
- Companion engine state: broad `CannotBeAffected` is too strong for the protection branch, and current inherited predicates do not inspect rules text on the carrier.
- Suggested DSL syntax:
  ```yaml
  protection:
    from: opponent
    categories: [dp_reduction, de_digivolve]
    while: { security_count_gte: 3 }

  active_when:
    carrier_text_contains: "Pulsemon"
  ```
- Gap kind: hybrid for narrow protection, DSL for rules-text contains predicate.
- Workaround: None faithful. Broad immunity or name predicates would over- or under-match printed behavior.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT16-055)

---

## EX11-060 — deletion event cause predicate for Overclock branch  [PUPPETS-G022]

- Effect text: "[All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted, by suspending this Tamer, <Draw 1>. If this effect was activated by <Overclock>, you may play 1 level 4 or lower [Puppet] trait Digimon card from your hand without paying the cost."
- Status 2026-05-06: `PUPPETS-G022` closed. Predicate leaf `event_cause` now compiles and evaluates against `TriggerContext.cause`; `overclock` is available as a first-class observer cause. Overclock sacrifice deletion preserves `ReplacementCause::Cost` for replacement windows while publishing `EventCause::Overclock` to `OnAnyDeletion` observers.
- Implemented DSL syntax:
  ```yaml
  condition: { event_cause: overclock }
  ```
- Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_060` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`.

---

## BT20-084 — trash-resident effect digivolve and stacked-card-to-security  [PUPPETS-G026/PUPPETS-G027]

- Effect text: "[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel]s may digivolve into this card without paying the cost." / "[End of All Turns] Place this Digimon's top stacked card as the top security card."
- Status 2026-05-09: `PUPPETS-G026` and the reusable `PUPPETS-G027` Track E verb are closed. DSL `when: on_ally_played` covers the trash-resident observer, and `security_place_top_stacked_card` now places the card below the visible top into security.
- Implemented trash-observer DSL syntax:
  ```yaml
  - when: on_ally_played
    optional: true
    condition: { event_target_kind: digimon }
    process:
      - select_own_permanent:
          bind_as: ciel
          filter: { name_is: "Sistermon Ciel" }
      - effect_initiated_digivolve:
          target: ciel
          source: self
          cost: free
          ignore_requirements: true
  ```
- Landed stacked-card DSL syntax:
  ```yaml
  - security_place_top_stacked_card:
      carrier: source
      of: you
      position: top
      face: up
  ```
- Gap kind: closed for the reusable top-stacked-card security movement. Future variants that select an arbitrary source use `security_place_stacked_card`.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt20_084_end_of_all_turns`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT20-084)

---

## BT22-088 — return-this-Tamer cost before branch free-play  [PUPPETS-G028] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] with a different card number in your hand without paying the cost, or play 1 [Shoemon] from your hand or trash without paying the cost."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). The `choose_one` branch selector with origin-preserving hand/trash play consumers (PUPPETS-G028) landed. `BT22-088`'s Start-of-Main tests are un-ignored. See `qa/resolved-gaps.md` for engine-side details.
- Status 2026-05-17: the **return-self-cost half** of this gap closed under Phase 2 Track B. DSL `activation_cost: { return_self_to_deck_bottom: true }` lifts onto `EffectBuilder::activation_cost(ctx.return_self_to_deck_bottom_as_cost)`; the engine queue's source-liveness check after the cost is now bypassed so the chained free-play branch can fire even though the source Tamer has left the field. The **branch selector half** was still open at that point.
- Missing DSL verb / step kind / predicate: optional triggered activation cost that moves the source permanent to the bottom of deck, then an in-effect branch selector with origin-preserving hand/trash play consumers.
- Companion engine state: the generic triggered activation-cost hook is now resolved (`qa/resolved-gaps.md`); the source-zone move helper lives on `EffectContext::return_self_to_deck_bottom_as_cost`. The chained branch selector with hand/trash consumers is still card-author DSL surface.
- Suggested DSL syntax:
  ```yaml
  activation_cost:
    return_this_tamer_to_bottom_deck: {}
  choose_one:
    - play_from_hand_free:
        filter:
          all_of:
            - name_is: "Arisa Kinosaki"
            - card_id_not: "BT22-088"
    - play_from_hand_or_trash_free:
        filter: { name_is: "Shoemon" }
  ```
- Gap kind: hybrid. The cost/preflight is engine-facing; branch and origin-preserving selection need DSL vocabulary.
- Workaround: None faithful. Auto-returning the Tamer or auto-selecting Shoemon/Arisa would hide printed player-visible choices.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT22-088)
- Status 2026-05-11: Still open for the Start of Your Main Phase return-this-Tamer cost and chained Arisa/Shoemon free-play branches. The separate All Turns Token/Puppet played observer is now implemented in `BT22-088.yaml` using `source_is_unsuspended`, visible suspend/decline selection, and event-target Token/Puppet filters.

---

## BT23-077 — self-scoped OnSuspend event predicate  [PUPPETS-G029]

- Effect text: "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon."
- Status 2026-05-08: `PUPPETS-G029` closed. `event_permanent_is_source` compiles and evaluates against `TriggerContext.event_permanent` and the observer source permanent, and BT23-077 now uses it for the printed self-suspend `<De-Digivolve 1>` clause.
- Companion engine state: `OnSuspend` dispatch exists and event context is available for observed suspend events; this slice adds the missing self-scoped predicate.
- Suggested DSL syntax:
  ```yaml
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
      - de_digivolve: { target: target, count: 1 }
  ```
- Gap kind: dsl predicate/evaluator gap, closed for BT23-077.
- Workaround: no longer needed for BT23-077. A broad `on_suspend` trigger remains an approximation for any future "this permanent" authoring that does not use `event_permanent_is_source`.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT23-077)
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.

---

## BT5-106 — effect-play On Play suppression provenance  [PUPPETS-G030] — RESOLVED 2026-05-20 (Puppets substrate sweep)

- Effect text: "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- Status 2026-05-20: **FULLY CLOSED** by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`). `suppress_on_play` flag on effect-play helpers (PUPPETS-G030) landed. `BT5-106`'s Security slice is now expressible in YAML. See `qa/resolved-gaps.md` for engine-side details. (Phase 2 Track J Task S1.1 closed the same gap independently — see [`qa/resolved-gaps.md`](resolved-gaps.md); the merged engine keeps the Puppets-sweep design.)
- Missing DSL verb / step kind / predicate: a play-from-trash/free-play consumer that carries `suppress_on_play: true` provenance for the played Digimon only.
- Companion engine state: ordinary effect play from trash can enter the Digimon and normally fire On Play; this card needs the same player-visible trash selection but must skip the played permanent's On Play enqueue for that play event.
- DSL syntax (shipped): `suppress_on_play: true` is honored ONLY by `play_from_trash_free`; the compiler rejects it on `play_from_hand` / `play_from_trash`.
  ```yaml
  - play_from_trash_free:
      of: you
      hand_index: revived
      suppress_on_play: true
  ```
- Gap kind: hybrid. Engine play provenance needed an On Play suppression flag, and DSL needed vocabulary to request it — both shipped.
- Deferred follow-up: `suppress_on_play` on `play_from_materials` (Royal Knights source-play payoffs) is NOT wired — the merged engine threads suppression only through `play_from_trash_free`. Re-wiring the `play_from_materials` path is follow-up work for when the RK source-play cards are authored.
- Workaround: no longer needed.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT5-106)
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- suppress_on_play`.

## BT3-002 — `carrier_has_keyword` predicate for inherited clause conditions  [G-DSL-CARRIER-HAS-KEYWORD]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; `has_keyword` already resolves against the carrier permanent for inherited clauses (`enqueue_from_permanent` sets `source_permanent` to the carrier handle). No predicate added.

- Effect text: "Inherited Effect [When Attacking] [Once Per Turn] If this Digimon has <Jamming>, <Draw 1> (Draw 1 card from your deck.)"
- Card first discovered in: BT3-002 DemiVeemon (Digi-Egg, Lv.2, Blue)
- Missing DSL verb / step kind / predicate: `carrier_has_keyword` — a `PredicateSpec` / `BoolPredicate` leaf for inherited triggered clauses that checks whether the TOP CARD of the permanent carrying the egg source has a given keyword (printed OR modifier-granted). The existing `has_keyword` predicate in `CompiledPredicate` evaluates on `source_permanent` (the egg slot itself), not the carrier permanent. For inherited effects, `source_permanent` is the bottom-of-stack source card, not the carrier Digimon.
- Lowers to engine API: `Game::has_keyword(carrier_handle, Keyword::Jamming)` — the engine has this method (used in `combat.rs`, `game.rs`). The gap is that the DSL predicate evaluator has no path to resolve the carrier handle from `EffectReadContext` for inherited clauses. The carrier handle is `EffectReadContext.source_permanent` (if it exists) but only when the source IS the top card; for sub-stack inherited sources, the context's `source_permanent` is the egg, not the carrier.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    optional: true
    condition: { carrier_has_keyword: Jamming }
    process:
      - draw: { of: you, count: 1 }
  ```
- Gap kind: dsl (engine has `Game::has_keyword` and modifier tracking; DSL lowering just needs a new predicate leaf that reads the carrier handle from the inherited-effect dispatch context rather than the source permanent).
- Workaround: Omit the `condition` from the YAML entirely (preferred). The clause over-fires without the Jamming gate — any carrier with BT3-002 in its digivolution cards will draw on attack regardless of Jamming. The over-fire is documented in BT3-002.yaml. The negative-condition test `bt3_002_does_not_fire_without_jamming` is `#[ignore = "pending: G-DSL-CARRIER-HAS-KEYWORD from qa/dsl-vocab-gaps.md"]`.
- Trade-off of omission vs. un-gated clause: omission is preferred because the Draw 1 step is safe (no permanent game-state harm), the positive case (carrier has Jamming → draw) is the common path this egg was designed for, and over-firing without Jamming is a minor accuracy loss rather than a silent break.

---

<!-- ───────────────────────────────────────────────────────────────────────
  BG IMPERIAL PHASE 0 RE-AUDIT — 2026-05-20 (`bg-imperial-substrate-closeout`)

  The BG Imperial entries below were re-verified against current source.
  STALE (primitive shipped — entry should be closed at change closeout):
    G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED (lower_grant_keyword.rs:18-36
      now consumes active_when), G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME (already
      marked resolved), G-IS-EFFECT-INITIATED (event_is_effect_initiated exists),
      G-BEFORE-PAY-COST-GAIN-MEMORY (resolved Track H), G-EFFECT-INITIATED-
      DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET (resolved Track F),
      G-DSL-TRASH-TOP-N-DIGI-CARDS (closed), G-DSL-UNION-PLAY-FREE
      (play_union_bound_free / PUPPETS-G014 shipped), G-DSL-SELF-COLOR-COUNT-GTE
      (self_color_count_gte shipped), G-EVENT-CARD-COLOR-IS partial
      (event_card_color_only/_count shipped — only a `_has`-semantics leaf
      remains), G-FORMULA-SOURCE-DP (source_dp formula shipped).
  GENUINE — now CLOSED by `bg-imperial-substrate-closeout` (2026-05-20), see
    qa/resolved-gaps.md § "BG Imperial substrate closeout": G-DSL-EFFECT-
    SUSPENDED-RESULT (`effect_suspended_any_opponent_digimon`),
    G-EVENT-CARD-COLOR-IS (`event_card_color_has`), G-SELECT-OPPONENT-SOURCES
    (`select_opponent_sources`), G-ZONE-SELECTED-TRASH-TO-DECK-TOP
    (`move_trash_card_to_deck_top`), G-ANY-RETURNED-CARD-PREDICATE
    (`returned_card_matching`).
  REDUNDANT — audit correction: these 4 were NOT genuine gaps; pre-existing
    capability already covers them, no predicate was added —
    G-PRED-STACK-SIZE-LTE-SOURCE / G-DSL-STACK-SIZE-LTE-SOURCE
    (`materials_count_lte` + `source_material_count` formula),
    G-DSL-CARRIER-HAS-KEYWORD (`has_keyword` resolves to carrier for inherited
    clauses), G-DSL-AURA-TARGET-SOURCE-PERMANENT (`scope: inherited` +
    `target: {}` self-aura), G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT
    (`source_permanent_trait_has`). See qa/resolved-gaps.md § "BG Imperial
    substrate closeout" → "Audit correction". The 2026-05-22 BG Imperial
    readiness reconciliation verified that the deck-library pool now consumes
    the new + pre-existing vocabulary without live raw_rust escapes.
  Verified per-card classification:
    openspec/changes/bg-imperial-substrate-closeout/phase-0-audit.md
─────────────────────────────────────────────────────────────────────── -->

## BT12-022 — `active_when` on `kind: grant_keyword` declarative clauses is not consumed  [G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — stale; `lower_grant_keyword.rs` already consumes `active_when` on `kind: grant_keyword` declarative clauses.

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" (BT12-022 ExVeemon, inherited)
- Missing DSL verb / step kind / predicate: `DeclarativeClause.active_when` is compiled into `CompiledDeclarativeClause::GrantKeyword { active_when, .. }` but is silently discarded by `lower_grant_keyword::lower` in `code/digimon-engine/src/dsl_cards/mod.rs` (line 82-98 uses `..` to destructure, ignoring `active_when`). The `lower_grant_keyword::lower` function signature has no `active_when` parameter.
- Companion state: `CompiledDeclarativeClause::GrantKeyword` does carry the `active_when: Option<CompiledPredicate>` field (compiled.rs:432). The `lower_aura::lower` function accepts and uses `active_when` correctly. The gap is that `lower_grant_keyword::lower` does not accept or apply it.
- Consequence: any `kind: grant_keyword` clause with `active_when:` specified will grant the keyword unconditionally — the condition is silently dropped. Cards relying on `active_when` to gate keyword grants over-fire.
- Lowers to engine API: `Effect::declarative(card).condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None))` — the condition closure already exists in `lower_aura::lower`; the same pattern needs to be applied in `lower_grant_keyword::lower`. Additionally, `Game::has_keyword` checks `effect.condition` for inherited declarative effects (game.rs lines 1717-1727) — so adding the condition to the `Effect` struct (not only the modifier tick) would gate the keyword check correctly without a declarative tick.
- Suggested fix:
  1. Add `active_when: Option<CompiledPredicate>` parameter to `lower_grant_keyword::lower`.
  2. In `mod.rs`, pass `active_when.clone()` to the call.
  3. Inside `lower_grant_keyword::lower`, add `if let Some(aw) = active_when { builder = builder.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None)); }`.
- Gap kind: dsl (engine has condition support on `Effect` struct; only the lowering wire-up is missing).
- Workaround: none needed. BT12-022 now ships with `active_when` consumed by
  `grant_keyword` lowering and focused negative-condition coverage active.
- Cards affected: BT12-022 ExVeemon (inherited conditional Jamming).
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)

---

## BT12-022 — BeforePayCost triggered gain_memory for "would DNA digivolve into" target  [G-BEFORE-PAY-COST-GAIN-MEMORY]

- **Status: RESOLVED 2026-05-17** (Phase 2 Track H). See `qa/resolved-gaps.md` § "Phase 2 Track H closure — 2026-05-17" for the substrate landed (sibling `Effect::before_pay_cost_observe` builder + `EffectTiming::BeforePayCostObserve` + `scan_before_pay_cost_observers` dispatch).
- Authoring pattern:
  ```yaml
  - when: before_pay_cost_observe
    active_when:
      all_of:
        - your_turn: true
        - dna_origin: true
        - source_is_cost_target_permanent: true
        - cost_target: { color_is: green, kind: digimon }
    process:
      - gain_memory: 1
  ```
- Cards implemented and validated: BT12-022 ExVeemon (clause 0), BT12-050 Stingmon (clause 0).
- Companion gap (also resolved): G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — see entry above.
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)
- First reported: 2026-05-04 (BT3-002 DemiVeemon DSL implementation)

## EX1-014 — `aura` declarative target scoping  [G-DSL-AURA-TARGET-SOURCE-PERMANENT]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; a `kind: aura` with `scope: inherited` + `target: {}` is a carrier-only self-aura. No leaf added.

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" — should grant Jamming ONLY to the carrier permanent (the Digimon containing this card in its digivolution stack), not all controller-side Digimon.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue), also in BT12-022 (sister card).
- Missing DSL verb / step kind / predicate: `target_is_source: true` BoolPredicate (or equivalent) usable inside `kind: aura` `target:` filter, so the aura applies only to the carrier of the source permanent — not the entire `target: { owner: you, kind: digimon }` set. Currently `lower_aura.rs` applies to all matches of the target predicate.
- Lowers to engine API: `target` filter check `handle == ctx.source_permanent` (or `handle == carrier_of(source)` for inherited-source clauses).
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    target: { owner: you, kind: digimon, is_carrier_of_source: true }
    grant_keyword: jamming
    active_when: { ... }
  ```
- Gap kind: dsl. Engine has the carrier handle resolution; only the predicate leaf is missing.
- Workaround: ship aura with broad target (over-fires to all your Digimon).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## EX1-014 — `self_digivolution_contains_trait` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; the existing `source_permanent_trait_has` predicate covers EX1-014's `[Free]`-trait arm. No predicate added.

- Effect text: "...has [Imperialdramon] in its name or the [Free] trait..." — needs a predicate that evaluates whether the carrier permanent's digivolution stack contains a card with a given trait.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_trait: <trait>` — boolean predicate over carrier permanent's digivolution stack. `source_permanent_trait_has` exists in `CompiledPredicate` spec but is not evaluated at runtime in `predicate.rs`.
- Lowers to engine API: `rctx.source_permanent()?.has_trait(name, rctx.card_data())` — engine has the data.
- Suggested DSL syntax:
  ```yaml
  active_when: { self_digivolution_contains_trait: "Free" }
  ```
- Gap kind: dsl.
- Workaround: omit the trait arm of the active_when (only name arm fires).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## BT16-040 — chained selection → effect_initiated_digivolve  [G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET] — RESOLVED 2026-05-17 (Phase 2 Track F)

Resolved as **phantom** — see [resolved-gaps.md](resolved-gaps.md)
"Phase 2 Track F closure". The chain dispatcher
(`run_tail_preserving_trigger_context`) was already driving the chain
to completion; the prior tests asserted mid-chain state and panicked
on the auto-resolved post-state. 5 tests now active.

[ORIGINAL ENTRY BELOW]

## BT16-040 — effect-initiated digivolve chain (legacy)

- Effect text: "[Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Insectoid] or [Free] trait in your trash with the digivolution cost reduced by 1." — process chain: select_own_permanent → select_trash_card → effect_initiated_digivolve.
- Card first discovered in: BT16-040 Wormmon (Digimon, Lv.3, Green/White). Same gap blocks BT17-015, BT17-027 clause 0.
- Missing DSL verb / step kind / predicate: process chain terminates after the permanent pick; the trash-pick prompt and `effect_initiated_digivolve` verb never execute when the source target is bound from a previous `select_own_permanent` step.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve` exists; the chain orchestration in the lowering layer does not resume after the first pick when the resolved source binding feeds into a subsequent select prompt.
- Suggested DSL syntax: existing chain syntax should work; the gap is in the process-step continuation mechanism.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; structural test passes, behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-040 batch-implement-cards-rust-dsl)

## BT12-028 / BT16-025 / BT16-027 — `stack_size_lte_source` predicate  [G-PRED-STACK-SIZE-LTE-SOURCE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — redundant; `materials_count_lte: { formula: { source_material_count: {} } }` already expresses "as many or fewer digivolution cards as this Digimon". No predicate added.

- Effect text variants: "Return 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon to the bottom of the deck." (BT16-027) / "Suspend all of your opponent's Digimon with as many or fewer digivolution cards as this Digimon" (BT16-025).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT16-025 Paildramon (same gap).
- Missing DSL verb / step kind / predicate: `stack_size_lte_source: bool` BoolPredicate leaf evaluating `candidate.card_sources.len() <= source_permanent.card_sources.len()` at runtime. The existing `stack_size_lte: <u8>` takes a literal, not a dynamic source-stack reference.
- Lowers to engine API: `Game::permanent(handle).card_sources.len()` for both candidate and source — engine has the data; only the predicate dispatch is missing.
- Suggested DSL syntax: `filter: { stack_size_lte_source: true }` inside `select_opp_field` / `select_permanent`.
- Gap kind: dsl.
- Workaround: clauses omitted from runtime; structural tests pass; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## ~~BT12-028 / BT16-027 — `self_digivolution_contains_name` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME]~~ — RESOLVED 2026-05-20 (DNA Omnimon completion)

- **Status:** Closed. The sources-only `self_digivolution_sources_contain_name` predicate
  leaf landed in the `complete-dna-omnimon-archetype` change, evaluating whether the source
  permanent's own `card_sources` stack contains a card matching the given name via
  `Permanent::contains_card_name`. See `qa/resolved-gaps.md` § "Phase 2 / DNA Omnimon
  completion closure — 2026-05-20". Original entry retained below for provenance.

[ORIGINAL ENTRY BELOW]

- Effect text: "if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards" (BT16-027). Sister of `G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT` (EX1-014).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT12-028 (`source_name_contains` family).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_name: <name>` BoolPredicate leaf evaluating whether the source permanent's own `card_sources` stack contains a card matching the given name. `source_name_contains` is defined in `PredicateSpec` and validated, but has no runtime evaluation branch in `predicate.rs`.
- Lowers to engine API: `Permanent::contains_card_name` — engine has the primitive; only the predicate dispatch wiring is missing.
- Suggested DSL syntax: `condition: { self_digivolution_contains_name: "Imperialdramon: Dragon Mode" }`.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## BT12-028 — `trash_top_n_digivolution_cards` step + engine primitive  [G-DSL-TRASH-TOP-N-DIGI-CARDS]

- Effect text: "Trash the top 3 digivolution cards of all of your opponent's Digimon." (BT12-028 clause 0a).
- Card first discovered in: BT12-028 Paildramon. Sibling to G-ASL-07 (BT17-077 all-source mass trash).
- Status: CLOSED for the reusable Track E DSL verb on 2026-05-09. YAML can now use `trash_top_n_digivolution_cards_of_each: { of: opponent, n: 3 }`, which lowers to `EffectContext::trash_top_n_digivolution_cards_of_each`. Evidence: `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_zone_movement_steps`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl zone_movement_verbs`.
- Landed DSL verb / step kind: `trash_top_n_digivolution_cards_of_each: { of: opponent, n: 3 }`.
- Lowers to engine API: `EffectContext::trash_top_n_digivolution_cards_of_each(target_player, n)`.
- Gap kind: closed for the bounded top-N-each reusable primitive. BT17-077's
  "all sources" sibling is also covered by the later BG Imperial substrate
  closeout.
- Workaround: no longer needed; BT12-028 is implemented in production YAML.
- First reported: 2026-05-04 (BT12-028 batch-implement-cards-rust-dsl).

---

## BT16-025 — `binding_is_none` / "if-no-target" predicate  [G-DSL-IF-NO-TARGET]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `binding_present` / `binding_absent` predicates (alias `binding_is_none`) exist and cover the "if this effect didn't suspend" branch.

- Effect text: "Suspend 1 of your opponent's unsuspended Digimon. If this effect didn't suspend, unsuspend this Digimon." (BT16-025 clause 2).
- Card first discovered in: BT16-025 Paildramon.
- Missing DSL verb / step kind / predicate: `select_opponent_permanent` with `optional: true` skips silently when no targets exist, but does not bind a "skipped" flag. Need `binding_is_none: <name>` BoolPredicate for subsequent `if` conditions to test whether the previous selection was taken or skipped.
- Lowers to engine API: existing binding mechanism — only the BoolPredicate leaf is missing.
- Suggested DSL syntax:
  ```yaml
  - if:
      condition: { binding_is_none: tgt }
      then: [ unsuspend: { target: source } ]
  ```
- Gap kind: dsl.
- Workaround: conditional unsuspend-self omitted from runtime; behavioral test `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-025 batch-implement-cards-rust-dsl).
- Also blocks: BT16-028 clause 0b — "[When Digivolving] by suspending 1 of their Digimon or Tamers, unsuspend 1 of your Digimon." Same structural gap: the optional suspend-cost step produces no binding result flag, so the own-unsuspend reward arm cannot be made conditional on the cost being paid. Cross-listed 2026-05-04.

---

## BT16-028 — `event_is_effect_initiated` predicate  [G-IS-EFFECT-INITIATED]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `event_is_effect_initiated` predicate exists and BT16-028 consumes it for the effect-play/digivolve observer gate.

- Effect text: "[All Turns] When an effect plays or digivolves an opponent's Digimon, if you have a Tamer, this Digimon may digivolve into [Imperialdramon: Fighter Mode] in the hand without paying the cost."
- Card first discovered in: BT16-028 Imperialdramon: Dragon Mode (2026-05-04).
- Status 2026-05-08: PARTIALLY RESOLVED. `PredicateSpec::event_is_effect_initiated` now compiles and evaluates against `TriggerContext.effect_initiated`. `TriggerSource::EnteredField` and `TriggerSource::Digivolved` carry the flag; normal hand play/digivolve set it false, while effect play helpers and `effect_initiated_digivolve` set it true. BT16-028 now authors the effect-play/digivolve observer with this gate.
- Remaining limits: This closes the reusable "by an effect" flag for `OnEnterFieldAnyone` / standard `OnDigivolve` observer predicates. It does not close stricter "by THIS effect" per-activation identity, effect-spawned permanent cleanup tokens, or DNA-specific origin flags.
- Lowers to engine API: `TriggerContext.effect_initiated`.
- Suggested DSL syntax:
  ```yaml
  - when: [on_enter_field_anyone, on_digivolve]
    optional: true
    active_when: { all_turns: true }
    condition:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
        - event_is_effect_initiated: true    # ← new predicate leaf
        - any_permanent:
            of: you
            zone: [battle_area]
            kind: tamer
    process:
      - select_hand:
          of: you
          bind_as: fighter
          filter:
            all_of:
              - kind: digimon
              - name_contains: "Imperialdramon: Fighter Mode"
          prompt: "Digivolve into Imperialdramon: Fighter Mode (free, ignore reqs)"
      - effect_initiated_digivolve:
          target: source
          from_hand: fighter
          cost: 0
          ignore_requirements: true
  ```
- Gap kind: hybrid (engine must thread the cause flag through TriggerContext; DSL then needs the predicate leaf).
- Workaround: no longer needed for BT16-028's effect-play half. Remaining ignored BT16-028 subtests cover narrower card-local follow-ups, not the reusable predicate itself.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_028`.
- First reported: 2026-05-04 (BT16-028 batch-implement-cards-rust-dsl).

---

## BT12-031 — Alt-cost: return named source card from own digi-stack to hand  [G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — `EffectContext::return_card_source_to_hand` + the `return_selected_sources_to_hand` DSL verb landed; BT12-031 Step C now ships → BT12-031 IMPLEMENTED. Full record in `qa/resolved-gaps.md` § "Follow-up engine gaps closed (2026-05-21)" and `docs/RUST_ENGINE_GAPS.md`.

- **Canonical record relocated 2026-05-21 (`bg-imperial-substrate-closeout`): this is an ENGINE gap, not DSL-only.**
  The full scoping entry now lives in
  [`docs/RUST_ENGINE_GAPS.md`](../docs/RUST_ENGINE_GAPS.md#return-a-selected-digivolution-stack-source-card-to-its-owners-hand)
  ("Return a selected digivolution-stack source card to its owner's hand",
  🟡 PARTIAL) — consult that entry for the suggested `EffectContext` /
  DSL-verb / YAML shape, likely files, complexity estimate, first test,
  and known interactions. This `qa/dsl-vocab-gaps.md` entry is retained
  only as a redirect; do not treat the notes below as the live spec.
  Summary: the two sub-gaps below (select-own-sources filter; `binding_present`)
  are both resolved, but BT12-031 Step C is still BLOCKED on a genuine missing
  engine primitive — there is **no DSL verb / `EffectContext` method that
  returns a single selected digivolution-stack source card to its owner's
  hand**. The only source-ref consumers are `trash_selected_sources` and
  `play_selected_sources_free`; `return_to_hand` moves a whole permanent, not
  one source card. BT12-031 Step C stays omitted, 2 tests `#[ignore]`'d, card
  verdict PARTIAL. Suggested fix: an `EffectContext` method + DSL verb (e.g.
  `return_selected_sources_to_hand`) routing each chosen source `Card` to its
  owner's hand. BT12-031 clause 1b (Security A.+1 + Blocker via
  `self_color_count_gte` `while_condition`) IS implemented.
- Effect text (BT12-031 Clause 0, Step C): "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's digivolution cards to its owner's hand, return all of your opponent's suspended Digimon to the bottom of their owners' decks instead."
- Missing DSL verb / step kind / predicate: Two sub-gaps combine to block this step:
  1. **G-DSL-SELECT-OWN-SOURCES-FILTER** — resolved 2026-05-08. `select_own_sources` now accepts `filter:` and evaluates it against each source card, with optional `from:` host restriction.
  2. **G-DSL-BIND-PRESENT** (see EX9-066 entry) — After the optional selection, the alternative outcome must be conditioned on whether the player made a selection or passed. The `binding_present` predicate does not exist.
- Synthesizing gap ID: `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` — filing as a composite gap for the BT12-031 context.
- DCGO reference: `BT12_031.cs` — step C via optional `AddSelectCard` from own digi-cards filtered by `EqualsCardName("Imperialdramon: Dragon Mode")`, `canNoSelect: () => true`. If selected, card returns to hand and all suspended opp Digimon return to bottom of deck; if declined, only the single return-to-hand fires.
- Suggested DSL syntax:
  ```yaml
  - select_own_sources:
      bind_as: dragon_mode_src
      optional: true
      filter:
        name_is: "Imperialdramon: Dragon Mode"
      prompt: "Return [Imperialdramon: Dragon Mode] from your digivolution cards to hand to return ALL opponent suspended Digimon to bottom of decks instead"
  - if:
      condition:
        binding_present: dragon_mode_src
      then:
        - return_to_hand: { target: dragon_mode_src }
        - for_each:
            over:
              all_of:
                - of: opponent
                - zone: [battle_area]
                - kind: digimon
                - is_suspended: true
            bind_as: susp_opp
            body:
              - return_to_deck:
                  target: susp_opp
                  position: bottom
                  include_sources: false
      else:
        - select_opponent_permanent:
            bind_as: suspended_target
            filter:
              all_of:
                - kind: digimon
                - is_suspended: true
            prompt: "Return 1 of your opponent's suspended Digimon to its owner's hand"
        - return_to_hand: { target: suspended_target }
  ```
- Lowers to engine API: `select_own_sources` filtering is now in place; remaining work is DSL-only.
  - `binding_present` predicate: add leaf that checks `ctx.bindings.get(name).is_some()`.
- Updated 2026-05-07: `select_own_sources.target` can now restrict the picker to a specific permanent binding, which covers self-stack cost shapes like Digi-Burst. This does **not** close the card-name source filter needed here; BT12-031 still needs `filter:` over source card identity plus `binding_present`.
- Gap kind: DSL only.
- Workaround: Steps A (for_each suspend no-digi-card targets) and B (select 1 suspended opp → return to hand) are authored in BT12-031.yaml. Step C is commented out as BLOCKED.
- Behavioral tests: 2 tests `#[ignore = "pending: G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME from qa/dsl-vocab-gaps.md ..."]` in `code/digimon-engine/tests/cards_behavioral/bt12/bt12_031.rs`.
- First reported: 2026-05-04 (BT12-031 TDD implementation).

---

## BT17-077 — `return_all_trash_to_deck_bottom` step + player-choice target  [G-RETURN-ALL-TRASH-TO-DECK-BOTTOM]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the player-choice-of-trash branch is composable via `select_effect_choice` + `if` branching `return_all_trash_to_deck_bottom: { of: you|opponent }`; BT17-077 Clause 1b now ships.

- Effect text: "Then, return all cards from your or your opponent's trash to the bottom of the deck." (BT17-077 Clause 1b).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode.
- Status: PARTIALLY CLOSED on 2026-05-09 for the reusable bulk-zone DSL verb. YAML can now call `return_all_trash_to_deck_bottom: { of: you|opponent }`, and owner-routing is covered by `zone_movement_verbs::bulk_trash_and_hand_reduction_verbs_call_helpers`. The remaining printed-card gap is the player-choice branch for "your or your opponent's trash" and the returned-card result predicate for the memory rider.
- Landed DSL verb / step kind: `return_all_trash_to_deck_bottom: { of: <player_ref> }` — moves every card currently in the specified player's trash zone to the bottom of its owner's deck.
- Lowers to engine API: `EffectContext::return_all_trash_to_deck_bottom(player)`.
- Companion gap: the printed text says "your or your opponent's trash" — the choice of whose trash is returned is a player decision (DCGO: `BoolSelection`). This requires either `select_effect_choice` (choose 0 or 1) + `if` conditional wiring the correct `of:` player, or a single parametric verb `return_all_trash_to_deck_bottom: { of: chosen_player }` where `chosen_player` is a binding. Neither is currently in the DSL.
- Suggested DSL syntax:
  ```yaml
  - select_effect_choice:
      bind_as: whose_trash
      labels: ["Your Trash", "Opponent's Trash"]
      prompt: "Return all cards from your or your opponent's trash to the bottom of the deck"
  - if:
      condition: { equals: [whose_trash, 0] }
      then:
        - return_all_trash_to_deck_bottom: { of: you }    # ← new verb
      else:
        - return_all_trash_to_deck_bottom: { of: opponent }  # ← new verb
  ```
- Gap kind: closed. Engine bulk-move, DSL verb, owner routing,
  player-choice branching, and the returned-card predicate are covered for
  BT17-077's full printed clause.
- Workaround: none needed. BT17-077 Clause 1b and the dependent Clause 1c memory
  rider ship in YAML and are covered by focused behavioral tests.
- Cross-ref: G-ASL-07 (qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md) tracks the remaining all-source/player-choice/result-predicate family.
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).

---

## BT17-077 — `any_returned_card` result predicate  [G-ANY-RETURNED-CARD-PREDICATE]

**Status: RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`)** — the `returned_card_matching` filtered result predicate landed (`bg-imperial-substrate-closeout` Tier 2); BT17-077 Clause 1c now ships.

- Effect text: "If this effect returned a white level 7 card, gain 3 memory." (BT17-077 Clause 1c).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode. Clause 1c fires after the `return_all_trash_to_deck_bottom` step (Clause 1b) completes; the memory gain is conditional on at least one of the moved cards satisfying `color: white AND level: 7`.
- Missing DSL verb / step kind / predicate: `any_returned_card: { color_is: white, level_eq: 7 }` — a BoolPredicate that evaluates to true if the immediately preceding zone-move step returned at least one card matching the given filter. There is no "result-set predicate" that can inspect the set of cards moved by a prior step.
- Lowers to engine API: the step would need to bind a `Vec<CardData>` of moved cards as an effect-local result, which the subsequent `if` condition can test via `any_returned_card` iterating over that result set.
- Suggested DSL syntax:
  ```yaml
  - return_all_trash_to_deck_bottom:
      of: opponent
      bind_returned_as: returned_cards    # optional result binding
  - if:
      condition:
        any_returned_card:                # new BoolPredicate leaf
          binding: returned_cards
          color_is: white
          level_eq: 7
      then:
        - gain_memory: 3
  ```
- Gap kind: dsl (engine result-binding infrastructure would also need extending for the `bind_returned_as` step argument).
- Workaround: none needed. Clause 1c ships in BT17-077.yaml using
  `returned_card_matching`.
- Cross-ref: G-RETURN-ALL-TRASH-TO-DECK-BOTTOM (above) must close first (Clause 1b provides the moved-card set that Clause 1c predicates on).
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).
---

## Royal Knights — Delay/keyword leave-prevention replacements  [RK-G003]

- Effect text: `BT20-100` The Last Guardian: "[All Turns] When any of your Digimon with [Omnimon] in its name would leave the battle area, <Delay> ... 1 of those Digimon doesn't leave." `BT23-054` Magnamon: "<Armor Purge> (When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.)"
- Status: closed for the Track B consumers. BT20-100's option-as-Delay source cost is represented by the replacement lowering shape `delete_permanent: { target: source }` followed by `cancel_replacement: {}`; the lowering only cancels after the delayed option actually reaches trash. BT23-054 uses the Armor Purge keyword replacement, prompts accept/decline, and trashes the top source only on accept.
- Companion engine state: Delay and Armor Purge both route through the shared replacement framework and existing pending-selection masks; no action-space expansion was required.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    source_is_delay_option: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - name_contains: "Omnimon"
    cost:
      trash_source_delay_option: {}
    process:
      - cancel_replacement: {}

  - kind: grant_keyword
    keyword: ArmorPurge
  ```
- Gap kind: closed for `BT20-100` and `BT23-054`; future cards should file a new narrower gap only if their cost/filter shape cannot be expressed through `kind: replacement` or the Armor Purge keyword.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_100_delay`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_054_armor_purge`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- First reported: 2026-05-05 (Royal Knights Batch 2: BT20-100, BT23-054).

---

## Royal Knights — would-leave observer that plays from hand without cancelling  [RK-G004]

- Effect text: `BT20-091` Cool Boy: "[Opponent's Turn] [Once Per Turn] When any of your Digimon with the [Royal Knight] trait would leave the battle area, you may play 1 [Omekamon] from your hand without paying the cost."
- Status: narrowed/closed for `BT20-091`. A `kind: replacement` clause can intentionally leave the outcome unset, which runs the side-effect and then lets the original leave event proceed. The `select_hand` step is required (`optional: false`) so the replacement is not offered when no Omekamon can be played; optionality lives on the outer replacement prompt.
- Companion engine state: `kind: replacement` observes would-leave events with event subject filters, OPT accounting, and ordinary pending hand selection/play. Non-cancelling subscribers are represented by replacement processes that do not call `cancel_replacement`, `redirect_replacement`, `substitute_replacement`, or `handle_replacement`.
- Suggested DSL syntax:
  ```yaml
  - when: when_would_leave_battle_area
    active_when:
      all_of:
        - opponents_turn: true
        - replacement_subject_is_mine: true
        - trait_has: "Royal Knight"
    optional: true
    once_per_turn: true
    process:
      - select_hand:
          bind_as: omekamon
          filter: { name_is: "Omekamon" }
      - play_from_hand_free: { of: you, hand_index: omekamon }
  ```
- Gap kind: closed for the BT20-091 shape. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave bt20_091_decline_would_leave_response_proceeds_without_playing_omekamon bt20_091_no_omekamon_in_hand_does_not_offer_response`.
- Workaround: no workaround needed for BT20-091; use the documented non-outcome replacement form.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT20-091).

---

## Royal Knights — attack target retarget response  [G-ATTACK-RETARGET]

- Effect text: `BT19-072` LordKnightmon: "[Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, you may switch the attack target to 1 of your Digimon with the [Royal Knight] trait."
- Status (2026-05-08): resolved for the BT19-072 card-shaped route. Production YAML uses `when: on_opponent_attack`, optional `select_own_permanent` filtered to Royal Knight Digimon, and `redirect_attack_target`. The combat flow emits the interrupt-time pending selection and mutates the active attack target through `ctx.redirect_attack`.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt19_072_opponents_turn_switches_attack_target_to_royal_knight`; shared verb coverage `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target`.
- Previous missing DSL verb / step kind / predicate: attack-state pending selection that can replace the current defender/security target with a selected own permanent matching a filter.
- Companion engine state: attack declaration and blocker/Raid-like retargeting are action-state concerns; a normal triggered effect after attack declaration cannot faithfully mutate the target without a dedicated interrupt point.
- Supported DSL syntax:
  ```yaml
  - when: on_opponent_attack
    optional: true
    once_per_turn: true
    process:
      - select_own_permanent:
          bind_as: new_target
          filter: { kind: digimon, trait_has: "Royal Knight" }
      - redirect_attack_target: { new_target: new_target }
  ```
- Gap kind: engine and DSL, closed for current script-facing retarget effects.
- Workaround: None needed for current script-facing retarget effects.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT19-072).

## ~~BT17-102 — dynamic name alias from digivolution-source stack  [G-DYNAMIC-NAME-ALIAS-FROM-STACK]~~ — RESOLVED 2026-05-22

- Effect text: BT17-102 Greymon "[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards."
- Status: RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. `identity.source_name_aliases` compiles a source-derived effective-name overlay, the engine synthesized identity includes those names, and name predicates consult the synthesized set.
- Evidence: `cargo test -p digimon-engine --test cards_behavioral bt17_102 -- --nocapture` passes with `bt17_102_all_turns_aliases_low_level_material_names` enabled.
- Companion engine gap: resolved in [docs/RUST_ENGINE_GAPS.md](../docs/RUST_ENGINE_GAPS.md) (`G-DYNAMIC-NAME-ALIAS-FROM-STACK`).
- Gap kind: hybrid, closed.
- Workaround: none needed.
- First reported: 2026-05-20 (`complete-dna-omnimon-archetype` closure — BT17-102 Greymon).

## ~~BT23-096 — `<Delay>`-on-attack-event clause  [G-DSL-DELAY-ON-ATTACK-EVENT]~~ — RESOLVED 2026-05-22

- Effect text: BT23-096 Comet Hammer — `<Delay>` body gated on an ally-attack event.
- Status: RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. `lower_delay.rs` maps attack timings to `DelayTrigger::OnEvent`, attack dispatch fans into event-gated delayed options with attacker context, and `attacker_trait_has` can evaluate ordinary attack context.
- Evidence: `cargo test -p digimon-engine --test cards_behavioral bt23_096 -- --nocapture` passes with the CS attack Delay and non-CS negative tests enabled.
- Already-present substrate: `G-DSL-ON-ALLY-ATTACK-TIMING` and `G-ATK-TRAIT-FILTER` remain noted as pre-existing halves; this change closed the missing delay/attack-event dispatch wiring.
- Companion engine gap: resolved in [docs/RUST_ENGINE_GAPS.md](../docs/RUST_ENGINE_GAPS.md) (`G-DSL-DELAY-ON-ATTACK-EVENT`).
- Gap kind: hybrid, closed.
- Workaround: none needed.
- First reported: 2026-05-20 (`complete-dna-omnimon-archetype` closure — BT23-096 Comet Hammer).

## Zephagamon — prompted attack target retarget to another Digimon or player  [ZEPH-G005]

- Status (2026-05-08): resolved for the ST18-14 Shoto Kazama card-shaped route. `redirect_attack_target` now supports a prompted form with `targets: any | player | digimon`, `optional`, and `prompt` fields when no fixed `new_target`/`player` is supplied. The prompt reuses attack-target action IDs, excludes the current target, can include the defending player, and exposes PASS when optional.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target_prompt_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st18_14`.
- Supported DSL syntax:
  ```yaml
  - redirect_attack_target:
      targets: any
      optional: true
      prompt: "Change the attack target to another Digimon or the player"
  ```

## Zephagamon — result-bound predicates and suspended-count formulas  [ZEPH-G002/ZEPH-G003/ZEPH-G005]

- Status (2026-05-10): narrowed. DSL predicate `binding_owner: { binding, of }` still covers the BT24-047 owner branch. Track J additionally added per-effect result-log predicates (`effect_suspended_any_own_digimon`, `effect_returned_any_card`, and sibling delete/play/digivolve/add-to-hand leaves) plus `suspended_count: { of: ... }` as a formula per-selector usable by formula-backed selection counts and thresholds. Production Zephagamon YAML still needs to be expanded for EX11-074 / BT20-101 / EX11-035 card-shaped coverage.
- Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group7_predicate_batch -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl suspended_count -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- binding_owner_predicate_matches_bound_permanent_controller`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_047`.
- Supported DSL syntax:
  ```yaml
  - if:
      condition:
        binding_owner: { binding: suspended, of: you }
      then:
        - may_attack_now: { attacker: suspended, targets: any, optional: true }
  - if:
      condition: { effect_suspended_any_own_digimon: true }
      then:
        - add_modifier: ...

  max:
    formula:
      base: 0
      per:
        suspended_count: { of: any }
      delta: 1
  ```
- Remaining adjacent card-authoring work: migrate the Zephagamon bodies that need these primitives and add card-shaped fixtures. If a printed card needs to distinguish a failed/protected mutation in a way the append-only result log cannot express, file that as a narrower `bind_result_as` payload gap.

## Track H §1 — Aura `security_attack: i32` flat slot (2026-05-10) — RESOLVED

The DSL `kind: aura` body now accepts a typed `security_attack: i32` field
alongside the pre-existing dynamic `security_attack_fn`. It lowers to a
`ModifierType::SecurityAttackChange` modifier carrying the literal delta
on each match, read at the security-resolution consult site
(`combat.rs:2326`). Negative deltas flow through unchanged; the combat
clamp at `combat.rs:2347` (max 0) governs the floor.

```yaml
# all your Olympos XII Digimon get <Security A. +1>
effects:
  - kind: aura
    target: { owner: you, trait: "Olympos XII" }
    security_attack: 1
```

Self, filter, and cross-side variants all land through the same path —
authors do not need to drop into raw_rust or formula DSL for flat ±N
grants. The dynamic `security_attack_fn` slot remains for cards whose
delta depends on board state.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_self_grants_flat_security_attack_plus_one aura_filter_grants_flat_security_attack_to_all_olympos_xii_digimon aura_filter_grants_flat_security_attack_minus_one_via_negative_delta`

## Track H §4 — Aura `while_condition` install-once continuous gate (2026-05-10) — PARTIAL

The DSL `kind: aura` body now accepts a `while_condition: <predicate>`
field that lowers to `Expiry::UntilCondition` on the installed
modifier. The UntilCondition controller (PR #458) handles eviction;
per the printed-semantics rule, `false → true` does NOT re-install.

```yaml
# this Digimon gains <Vortex>-can-attack-player while opponent has no
# unsuspended Digimon (canonical ZEPH-G004 fixture; uses
# memory_gte: 0 in v1 because VortexCanAttackPlayer's consult site is
# itself a separate gap)
effects:
  - kind: aura
    dp_modifier: 1000
    while_condition:
      count_lte:
        n: 0
        filter:
          owner: opponent
          kind: digimon
          is_unsuspended: true
```

Distinct from `active_when` (per-tick re-evaluation, symmetric).
`while_condition` installs ONCE at OnPlay or OnDigivolve, the
controller evicts on predicate-false, and the install does NOT
re-fire. DCGO reference: `Vortex.cs:PermanentHasVortexCanAttackPlayers`
implements the lazy-filter pattern via `CanUse(null)` at attack-target
time; the Rust path achieves identical end behavior via
mutation-event-driven eviction.

**v1 supports**: self-aura with `dp_modifier`, `security_attack`, or
named `modifier` grants. Combine freely; all install with
`Expiry::UntilCondition` carrying the same compiled predicate.

**v1 does NOT support yet**:
- Filter-aura + `while_condition` — install-once would miss future
  permanents joining the filter set. Needs the lazy-filter shape
  from spec §2 (consult-time filter evaluation rather than
  install-time enumeration).
- Keyword-grant + `while_condition` — `KeywordEntry` lacks an
  `until_condition` field; the keyword registry needs the same
  extension `ModifierEntry` already has.
- Player-scoped (`target_player`) + `while_condition` — same
  install-once vs. lazy-filter design choice.

New raw_rust API:
- `EffectContext::add_modifier_with_until_condition(target, modifier, value, predicate_arc)`
  — typed wrapper that honors the `can_affect_permanent` guard, used by
  both lower_aura's while_condition path and any raw_rust card script
  that needs to install a controller-evicted modifier directly.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- while_condition`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat until_condition_controller`

## Track H §5 — Security-zone-sourced auras (2026-05-10) — PARTIAL

The DSL `kind: aura, scope: security` clause now lowers correctly. The
engine's `tick_declarative_effects` iterates face-up cards in each
player's security stack (gated on `player.face_up_security`); the
existing filter-aura process closure runs with `source_permanent =
None` and installs DP / keyword / security-attack / named-modifier
grants on field-side matches.

```yaml
# BT21-095-style: while this Option is face-up in security, all your
# [WG] Digimon gain Vortex.
card: BT21-095
name: Wind Guardians
kind: option
color: [green]
cost: 2
traits: [WG]
effects:
  - kind: aura
    scope: security
    target: { owner: you, kind: digimon, trait: WG }
    grant_keyword: { keyword: Vortex }
```

End behavior matches DCGO `BT21_095.cs:CanUseCondition →
IsExistInSecurity(card, false)`:
- Face-down security sources do NOT fire.
- Source leaving security evicts the grant on next tick (no explicit
  OnLoseSecurity wiring needed — the materialized-declarative
  clear+re-install pattern handles it).
- New field entries pick up the grant on next tick (lazy-filter end
  behavior via the existing per-tick scan).
- Owner-scoped target filters work (your-side vs. opponent-side
  matches).

Outstanding: tensor/mask paths that pre-compute aura state from
sources directly (rather than reading modifier registry) still need a
`SecuritySource` enumeration. For raw_rust card scripts that need to
read their own security-zone position, the `EffectContext` source
discriminator is still `source_permanent: Option<PermanentHandle>` +
`source_card: CardHandle`; promoting to a typed `SecuritySource
{ player, security_index, card_index }` is a follow-up.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- security_zone_aura`

## Track H §3 — Granted triggered ability (2026-05-10) — PARTIAL

The engine primitive landed for the canonical OnDeletion case (DCGO
`AddSkillClass.cs` analog). Raw_rust card scripts can now grant a
closure-bodied triggered effect to a target permanent:

```rust
// Inside an effect's process closure, with `ctx: &mut EffectContext`:
ctx.grant_triggered_effect(
    carrier_handle,
    EffectTiming::OnDeletion,
    Expiry::Permanent,           // or EndOfTurn / EndOfYourTurn / etc.
    move |inner| {
        // Body fires when carrier is deleted, with:
        //   inner.source_card       == grantor card  (DCGO EffectSourceCard)
        //   inner.source_permanent  == carrier       (DCGO EffectSourcePermanent)
        //   inner.player            == grantor's controller
        inner.gain_memory(2);
    },
);
```

End behavior pinned by tests:
- Grantor installs grant on carrier; pre-deletion the body has not
  fired; deleting the carrier fires the body with carrier+source
  attribution preserved.
- `clear_permanent` evicts on carrier-leave (covers paths that bypass
  OnDeletion such as return-to-hand).
- `expire_end_of_turn` evicts time-bound grants per the same
  `source_player`-keyed rules as ModifierEntry.

DSL surface: not yet wired. A future `kind: grant_triggered` clause
would lower to this engine primitive. For now, granted triggered
abilities require raw_rust authoring.

Limitations of v1:
- **Timing coverage**: dispatch hook calls
  `fire_granted_triggered_effects(handle, timing)` only at the two
  OnDeletion firing sites. Other timings (OnAttack, OnSuspend, OnPlay,
  OnEnterFieldAnyone, etc.) install fine but never fire — extend each
  timing's canonical firing site as it comes online.
- **No selection support**: bodies fire inline, before the standard
  drain. A body that calls `ctx.install_pending_selection(...)` won't
  compose correctly with the surrounding firing sequence. For
  selection-driving granted bodies, the proper path is `QueuedEffect`
  with a `granted_effect_id` discriminator + lookup in
  `run_queued_effect_inner`. That's a follow-up.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_triggered_effect`

## Track H Phase 4 — Multi-timing dispatch, EX1-068, BT21-095, cross-track integration (2026-05-10)

### Phase 4a — `Expiry::EndOfOpponentsNextTurn` / `EndOfYourNextTurn` DSL keys

DSL string keys `end_of_opponents_next_turn` / `end_of_your_next_turn`
round-trip through `expiry_map.rs` to the new engine variants. v1
aliases the removal predicates to `EndOfOpponentsTurn` /
`EndOfYourTurn` semantics (correct for installs on source's own turn —
the common case for `[Main]` / `WhenDigivolving`). Mid-opp-turn install
nuance ("skip current opp-turn-end, expire on next") is a separate
follow-up requiring per-entry `pending_skips: u8` counter.

```yaml
- add_modifier:
    target: opponent
    modifier: ChangeDp
    value: -2000
    expiry: end_of_opponents_next_turn
```

### Phase 4b — Multi-timing dispatch for granted triggered abilities

`Game::pending_granted_fires` field accumulates carrier+timing pairs
discovered during `enqueue_from_permanent` /
`enqueue_from_breeding_permanent`; `drain_effect_queue` flushes them
inline AFTER its main loop drains. ALL `EffectTiming` variants are
covered automatically — no per-timing call-site additions needed.
Order: printed observers first, granted bodies second (matches DCGO's
"appended to effect list" semantic).

EX1-068 Ice Wall! ("All of your opponent's Digimon gain `[When
Attacking] lose 2 memory` until the end of their next turn") is wired
end-to-end as a raw_rust behavioral fixture — exercises:
- `EffectTiming::WhenAttacking` granted dispatch
- `Expiry::EndOfOpponentsNextTurn` carrying through expire_end_of_turn
- Per-carrier installation with multi-target enumeration
- Post-expiry attacks correctly do NOT fire the granted body

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- ex1_068`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_triggered_effect_fires_at_when_attacking`

### Phase 4c — Inherited filter aura (§6)

Filter auras with `scope: inherited` correctly emit when the source is
a card under a digivolution stack (not the top card). Verified by
`group6_auras::inherited_filter_aura_emits_grants_from_under_stack_source`
— a [Beast]-trait DigiEgg-style under-stack source publishes a
"+1000 DP to all your [Beast] Digimon" filter aura; the field
permanents matching the filter receive the grant including the
stack-top itself.

### Phase 4d — Cross-track integration (Track H × Track C)

`predicate.rs::eval_permanent_fields` now consults the synth-identity
overlay's traits union when evaluating `trait_has`. Without this
fix, a Track C `ChangeTraits` overlay (e.g., a Tamer treated as
[Holy] for the turn) was invisible to Track H aura filters. Pinned
by `aura_filter_includes_track_c_change_traits_overlay`. Other Track C
overlays (`ChangeBaseCardName`, `ChangeBaseCardColor`) follow the same
pattern but aren't yet propagated through the corresponding predicate
fields (`name_*`, `color_*`); separate follow-up.

### Phase 4f — EX1-068 Ice Wall! end-to-end raw_rust fixture

DCGO reference: EX1-068 grants `[When Attacking] lose 2 memory` to
all opp Digimon "until the end of their next turn." The Rust fixture
in `group6_auras::ex1_068_ice_wall_grants_when_attacking_loses_2_memory_to_all_opp_digimon`
walks opp's battle area at the source's [Main] effect time and calls
`ctx.grant_triggered_effect(opp_h, EffectTiming::WhenAttacking,
Expiry::EndOfOpponentsNextTurn, |inner| inner.gain_memory(-2))`.

DSL `kind: grant_triggered` clause (which would let EX1-068 land as
pure YAML) is a separate Phase 4e gap. Today the card requires
raw_rust authoring.

### Phase 4g — BT21-095 Wind Guardians real card YAML

`code/digimon-engine/cards/bt21/BT21-095.yaml` lands the [Security]
[All Turns] aura half via `kind: aura, scope: security` +
`grant_keyword: { keyword: Vortex }`. Behavioral fixture in
`code/digimon-engine/tests/cards_behavioral/bt21/bt21_095.rs`
covers: face-up grants, face-down does NOT grant, leave-security
evicts on next tick, owner-scope filter excludes opp [WG] Digimon.
Other clauses (IgnoreColorRequirement, [Main] replace-bottom-security,
[Security] play-WG-from-hand) are tracked under separate gap entries.

### Phase 4h — KeywordEntry `until_condition` extension

`KeywordEntry` gains `until_condition: Option<UntilConditionFn>` and
shares the globally-monotone `next_install_order` counter with
`ModifierEntry` / `PlayerModifierEntry`. The UntilCondition controller
now walks all three stores. New API:
`EffectContext::grant_keyword_with_until_condition(target, keyword,
predicate_arc)`. The DSL `while_condition` aura slot now lowers
keyword grants through this path:

```yaml
# ZEPH-G004-style: this Digimon gains <Vortex> while opponent has no
# unsuspended Digimon (memory_gte: 0 used as stand-in until
# VortexCanAttackPlayer's own consult site lands).
effects:
  - kind: aura
    grant_keyword: { keyword: Vortex }
    while_condition:
      count_lte:
        n: 0
        filter: { owner: opponent, kind: digimon, is_unsuspended: true }
```

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- while_condition_keyword_grant_lands_via_keyword_entry_until_condition`

### Phase 4e — DSL `grant_triggered_effect` step

The new step `grant_triggered_effect` lets card authors install a
granted triggered ability through pure YAML — no raw_rust required.

```yaml
# EX1-068 Ice Wall! authored as pure DSL.
effects:
  - when: main_from_hand
    optional: false
    process:
      - grant_triggered_effect:
          target: { owner: opponent, kind: digimon }
          timing: when_attacking
          expiry: end_of_opponents_next_turn
          body:
            - gain_memory: -2
```

Walks battle areas for `target` matches at the step's resolution
time and installs a granted-triggered-effect entry on each. The
body is a step list (anything `run_steps` can execute). Carrier vs.
source attribution flows through automatically — when the body
fires, `EffectContext::source_card` is the grantor and
`source_permanent` is the carrier (DCGO `EffectSourceCard` /
`EffectSourcePermanent`).

`timing:` accepts snake_case names: `on_play`, `on_digivolve`,
`when_digivolving`, `when_attacking`, `on_attack`, `end_of_attack`,
`end_of_battle`, `on_deletion`, `on_any_deletion`, `on_enter_field`,
`on_enter_field_anyone`, `on_suspend`, `on_unsuspend`,
`start_of_your_turn`, `start_of_opponents_turn`,
`start_of_your_main_phase`, `end_of_your_turn`,
`end_of_opponents_turn`, `on_ally_played`, `on_ally_attack`,
`on_opponent_attack`, `on_attack_target_change`. Unknown names
no-op silently with a debug-build warning.

`expiry:` uses the standard expiry-map keys (Phase 4a added
`end_of_opponents_next_turn` / `end_of_your_next_turn`).

v1 limitations:
- Bodies are non-selection (run inline after the printed-observer
  drain). Selection-driving bodies still require raw_rust until the
  `QueuedEffect.granted_effect_id` plumbing lands.
- The walk is at install-time; permanents that join the filter set
  AFTER the step resolves don't receive the grant. For
  install-once-then-leave-frozen semantics this is correct (matches
  EX1-068's printed text "all of your opponent's Digimon" snapshots
  current state).

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- dsl_grant_triggered_effect_step`

### Phase 4 cross-track integration (§10)

Three focused fixtures pin Track H's compatibility with adjacent
tracks at the consult-site level:

- **Track B (replacement) × H** — an aura granting `CannotBeDestroyed`
  via the `modifier:` slot installs a passive replacement modifier
  visible to Track B's deletion replacement window. Test:
  `aura_grant_cannot_be_destroyed_modifier_reaches_track_b_replacement_framework`.
- **Track D (combat) × H** — a self-aura granting `Piercing`
  surfaces through `Game::has_keyword` so Track D's combat
  security-check pipeline applies the Piercing follow-up. Test:
  `aura_grant_piercing_keyword_propagates_through_combat_consult`.
- **Track G (keyword payloads) × H** — a self-aura granting
  `Decoy(color)` preserves the parametric color discriminator
  through the registry so opponent's attack-target resolution
  filters correctly. Test:
  `aura_grant_decoy_keyword_includes_color_filter_payload`.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_grant`

### Phase 4i — Queue-based granted-body dispatch + selection support

`QueuedEffect.granted_effect_id: Option<u64>` discriminates granted
entries from printed-effect entries. `Game::granted_effect_bodies`
holds the closure bodies indexed by id. Granted entries flow through
the standard queue/drain pipeline so:
- Selection-installing bodies park correctly on `pending_selection`;
  the queue holds the entry alive while the selection resolves.
- The standard FIFO ordering (turn-player-bundle-first → trigger-order
  prompt for multi-trigger bundles) applies uniformly to granted and
  printed entries inside the same timing.
- The drainer skips the standard condition/pay_cost/max_per_turn
  gates for granted entries (they're closure-bodied with no Effect
  metadata).

Replaces the inline-fire `pending_granted_fires` flush (Phase 4b) that
worked only for non-selection bodies.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_body_runs_via_queue_with_correct_attribution`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- granted_body_installing_selection_parks_via_pending_selection`

### Phase 4k — Typed `AuraScope` / `AuraGrant` builder API (raw Rust)

New `code/digimon-engine/src/aura.rs` module ships a typed fluent
builder for raw_rust card scripts that author auras programmatically.
YAML-authored cards continue to use the field-slot DSL (`kind: aura`
body) — that path is unchanged.

```rust
use digimon_engine::aura::{AuraScope, AuraGrant};
use digimon_engine::effect::Effect;
use digimon_engine::enums::{Expiry, Keyword};

Effect::declarative(card)
    .name("All your Holy Digimon gain +1000 DP")
    .aura()
        .scope(AuraScope::Player(controller))
        .target_filter(|rctx, h| {
            // ... per-permanent filter predicate
            true
        })
        .grants(AuraGrant::Dp { value: 1000, base: false, origin: false })
        .duration(Expiry::EndOfYourTurn)
    .build()
```

`AuraScope` variants: `Permanent(handle)`, `Player(player_id)`,
`OpponentPlayer(source_player)`, `Bilateral`, `SecurityZone(player_id)`.

`AuraGrant` variants: `Keyword(Keyword)`, `Dp { value, base, origin }`,
`SecurityAttack(i32)`, `PlayCost(i32)`, `DigivolutionCost(i32)`,
`LinkCost(i32)`, `Immunity(ImmunityKind)`, `Cannot(CannotKind)`,
`Modifier { ty, value }` (escape hatch for any ModifierType).

`ImmunityKind`: `OpponentDpReduction`, `OpponentDeDigivolve`,
`BattleDeletion`, `EffectDeletion`.

`CannotKind`: `Suspend`, `Unsuspend`, `Block`, `Attack`,
`AttackPlayer`, `ReturnToHand`, `ReturnToDeck`, `DeDigivolve`.

The builder routes through the existing typed install APIs:
- `AuraGrant::Keyword` → `grant_declarative_keyword` (or
  `grant_keyword_with_until_condition` when `while_condition` is set)
- All other grants → `add_declarative_modifier` (or
  `add_modifier_with_until_condition` when `while_condition` is set)

End behavior identical to direct API calls; pinned by tests
confirming the same modifier-registry state for both paths.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- typed_aura_builder`

### Phase 4 — `pending_skips` for `*NextTurn` expiry mid-opp-turn install

`ModifierEntry.pending_skips: u8` enables accurate
`EndOfOpponentsNextTurn`/`EndOfYourNextTurn` semantics for the rare
mid-opp-turn install case. Default 0 preserves source-turn-install
alias to `EndOfOpponentsTurn`. Set to 1 via
`.with_pending_skips(1)` when installing during the same player's
turn whose end would otherwise immediately expire the entry — the
current firing decrements (instead of expires), the next firing
expires. Matches printed text "until end of their NEXT turn" exactly
for all install timings.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- end_of_opponents_next_turn_with_pending_skips`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- end_of_opponents_next_turn_without_pending_skips`

### G-DSL-MODIFIER-PENDING-SKIPS — DSL `add_modifier` step cannot set `pending_skips` — RESOLVED 2026-05-21

- **Discovered in:** Puppets `/batch-implement-cards-rust-dsl` completion run, EX4-074 ShineGreymon: Ruin Mode (2026-05-21).
- **Card(s):** EX4-074 — `[When Digivolving][On Deletion] Until the end of your opponent's next turn, all of your opponent's Digimon get -5000 DP.`
- **Was missing:** the DSL `add_modifier` / `add_dp_modifier` step had no way to set `ModifierEntry.pending_skips`; its lowering routed through `ModifierEntry::simple` (hard-coded `pending_skips: 0`), so a DSL-installed `expiry: end_of_opponents_next_turn` modifier aliased to `EndOfOpponentsTurn` semantics and expired one turn early when installed mid-opponent-turn.
- **Resolution:** rather than a DSL field (the correct `pending_skips` is runtime turn-state, not authoring-time data), the engine now auto-computes it. `modifiers::pending_skips_for_install(expiry, source_player, turn_player)` returns `1` exactly for the `*NextTurn` install that would otherwise expire one turn early; `EffectContext::add_modifier` calls it and threads the result through `ModifierEntry::with_pending_skips`. Every `add_modifier` / `add_dp_modifier` caller (DSL and hand-written) now gets faithful "until end of next turn" semantics for free — no new DSL vocab needed.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers::tests::pending_skips`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_074` (6 passed, 0 ignored — the 3 formerly-deferred −5000 DP tests now run).

### G-ZONE-SELECTED-TRASH-TO-DECK-TOP — selected trash card → deck top — RESOLVED 2026-05-21

- **Discovered in:** Puppets `/batch-implement-cards-rust-dsl` completion run, LM-029 Yellow Scramble (shared by the LM-027 / LM-030 Scramble Delay clauses).
- **Card(s):** LM-029 — `[Start of Your Turn] <Delay> ... Return 1 yellow Digimon card from your trash to the top of the deck.`
- **Was missing:** no DSL verb moved a *selected* trash card to the *top* of the deck. The only trash→deck verbs (`return_all_trash_to_deck_bottom`, `return_trash_list_to_deck_bottom`) are bottom-only.
- **Resolution:** added the `return_trash_list_to_deck_top` DSL verb (exact mirror of `return_trash_list_to_deck_bottom` — `StepSpec::ReturnTrashListToDeckTop` / `CompiledStep::ReturnTrashListToDeckTop`, fields `of` + `cards`) lowering to the new `EffectContext::return_trash_cards_to_deck_top` engine method (mirror of `return_trash_cards_to_deck_bottom` but `deck.push` to the `Vec` end = deck top = drawn first).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- lm_029` (16 passed, 0 ignored — LM-029's `[Start of Your Turn]` Delay clause fully implemented).
- **Note:** LM-027 / LM-030 Scramble Delay clauses share this gap and are now unblockable (not implemented here — out of Puppets scope).

### Phase 4l — Track C overlay propagation (full set)

`predicate.rs::eval_permanent_fields` now consults the synth-identity
overlay union for ALL overlayable card-level fields:
- `trait_has` ← `synth_identity.traits` (covers Track C `ChangeTraits`)
- `name_is`, `name_contains`, `name_in` ← `synth_identity.card_name`
  (covers Track C `ChangeBaseCardName`)
- `color_is`, `color_only` ← `synth_identity.colors` (covers Track C
  `ChangeBaseCardColor`)

Previously Track C overlays were invisible to Track H aura filters
unless the predicate tested only `kind` (which already routed through
synth_identity). Now the full identity overlay union propagates,
matching DCGO's `Permanent.HasTrait` / `Permanent.GetCardName` /
`Permanent.GetColors` behavior — which all consult the live overlay
state.

Coverage:
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_filter_includes_track_c_change_traits_overlay`
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_filter_includes_track_c_change_base_card_name_overlay`

---

## ~~BT8-094 / RB1-035 — event-target level predicates~~  [G-EVENT-TARGET-LEVEL-LTE] — RESOLVED 2026-05-23

- **Status:** RESOLVED 2026-05-23 (`complete-rocks-archetype` task 10.1). `event_target_level_eq`, `event_target_level_lte`, and `event_target_level_gte` now flow through `PredicateSpec` -> `CompiledPredicate` -> compiler -> runtime evaluator. `BT8-094` Clauses A and B are authored and verified. `MovedFromBreeding` observer dispatch now scans both players' battle areas so opponent-side Tamers can faithfully observe the moved event target.
- **Coverage:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_094 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl_eval_arm_coverage -- --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_082 --nocapture`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- p_130 --nocapture`.

Historical note:

- **Effect text (BT8-094 Clause A):** "[All Turns] When one of your opponent's
  level 5 or lower Digimon is deleted, you may suspend this Tamer to ＜Draw 1＞."
- **Effect text (BT8-094 Clause B):** "[Opponent's Turn] When one of your
  opponent's level 3 Digimon is moved from their breeding area to their battle
  area, gain 2 memory."
- **Effect text (RB1-035 Clause 2):** "[All Turns] When an opponent plays a
  Digimon, by suspending this Tamer, gain 1 memory if that Digimon is level 4
  or higher, and Draw 1 if it is level 3."
- **Missing DSL predicate:** The `PredicateSpec` struct in
  `code/digimon-dsl/src/predicate.rs` has no `event_target_level_lte`,
  `event_target_level_eq`, or `event_target_level_gte` leaf. The sibling
  `event_target_kind` / `event_target_trait_has` / `event_target_owner` /
  `event_target_color_any_of` leaves all exist but none expose the integer
  level of the event-target permanent's top card.
- **Why it can't be approximated:** omitting the level filter would make the
  deletion observer fire on Lv.6+ Digimon too, and the breeding-move observer
  fire on Lv.4+ Digimon — both violate the no-approximations policy.
- **Lowers to engine API:** `EffectReadContext::event_target_card()` already
  returns a `Card` snapshot; `Card::level` is available on that struct. Adding
  an `event_target_level_lte: Option<u8>` predicate leaf (and `_gte`, `_eq`
  siblings) is a small addition alongside the existing `event_target_kind` arm
  in `predicate.rs::eval_predicate_with_bindings` (`group6_event_target_*`
  block, ~line 900).
- **Suggested DSL syntax:**
  ```yaml
  condition:
    all_of:
      - event_target_owner: opponent
      - event_target_kind: digimon
      - event_target_level_lte: 5       # or event_target_level_eq / _gte
  ```
- **Gap kind:** DSL-only (engine event context already carries the level; the
  only missing piece is the predicate leaf in `predicate.rs` + `compiled.rs` +
  `compile.rs` + the evaluator arm).
- **Blocked cards / tests:**
  - `code/digimon-engine/cards/bt8/BT8-094.yaml` — Clauses A and B omitted;
    YAML comments document the intended shapes.
  - `code/digimon-engine/tests/cards_behavioral/bt8/bt8_094.rs` — 9 tests
    `#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE ..."]`.
  - `code/digimon-engine/cards/rb1/RB1-035.yaml` — [All Turns] clause noted
    as needing "event-card level predicates" in comment.
- **First reported:** 2026-05-22 (BT8-094 Pass 2 audit).

## ~~DSL Gap: LM-027 — Move a selected trash card to deck TOP~~  [G-ZONE-SELECTED-TRASH-TO-DECK-TOP] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `EffectContext::return_trash_cards_to_deck_top` + a `destination: top | bottom` field on the `return_trash_list_to_deck_bottom` step move a selected trash card to the deck top. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, LM-027 Red Scramble DSL implementation (2026-05-21). Also pre-noted as MED-GAP-01 in `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md` and `qa/archetype-qa/dsl/bg_imperial.md`.
- **Scope:** DSL + engine (hybrid). Cross-referenced in `qa/archetype-qa/engine-gaps.md` under the same gap ID.
- **Card(s):** LM-027 Red Scramble — `[Start of Your Turn] <Delay>` clause "Return 1 red Digimon card from your trash to the **top** of the deck." Also LM-029, LM-030, LM-031 (sibling Scramble cards with the same Delay body).
- **Effect text:** "Return 1 red Digimon card from your trash to the top of the deck."
- **What's missing:** No DSL verb / `EffectContext` method moves a *selected* trash card to the **top** of the owner's deck. Verified: `EffectContext::return_all_trash_to_deck_bottom` and `return_trash_cards_to_deck_bottom` (`effect_context/mod.rs`) both hard-code `deck.insert(0, card)` (deck bottom). DSL `step/zone_moves.rs` exposes only `ReturnAllTrashToDeckBottom` / `ReturnTrashListToDeckBottom` — bottom-only. `ReturnToDeckFromReveal` accepts a `position` but operates on the reveal pool, not the trash. Routing to deck bottom would be an unfaithful approximation (top vs bottom changes what is drawn) — forbidden by the no-approximations policy. The deck-**bottom** sibling gap `G-ZONE-TRASH-TO-DECK` and the timing gap `G-DELAY-START-OF-TURN` are both RESOLVED; this deck-TOP variant is genuinely new and distinct.
- **Suggested change:** Add a `destination: top | bottom` parameter to a generalized `return_bound_cards_to_deck` DSL step (or a dedicated `return_selected_trash_to_deck_top` verb) plus a matching `EffectContext::return_trash_cards_to_deck_top` engine method (mirror `return_trash_cards_to_deck_bottom` but `deck.push`).
- **Workaround:** LM-027 clause B retains a `raw_rust` no-op placeholder; 4 tests `#[ignore = "...G-ZONE-SELECTED-TRASH-TO-DECK-TOP"]`. Clauses A and C ship faithfully in pure DSL.

## ~~DSL Gap: BT21-072 — `save_in_text` predicate for alt-path `from:` filters~~  [G-ALT-PATH-SAVE-IN-TEXT] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). **No new predicate needed** — the existing `effect_text_contains` predicate already scans a candidate's printed text and `eval_predicate` already evaluates it against an alt-path `from:` candidate. BT21-072's cost-3 path uses `from: { any_of: [{level_eq:4, effect_text_contains:"＜Save＞"}, {level_eq:4, trait_has:Hero}] }`. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, BT21-072 Arresterdramon: Superior Mode (2026-05-21).
- **Scope:** DSL.
- **Card(s):** BT21-072 — `xros_req: "[Digivolve] Lv.4 w/＜Save＞ in text or w/[Hero] trait: Cost 3"`. Any card whose alt-digivolution requirement gates on the source permanent having ＜Save＞ printed in its effect text.
- **Effect text:** "[Digivolve] Lv.4 w/＜Save＞ in text or w/[Hero] trait: Cost 3"
- **What's missing:** Alt-path `from:` filters support `level_eq`, `trait_has`, `name_contains`, etc., but there is no `save_in_text: true` predicate to match a source permanent whose top card has the ＜Save＞ keyword in its printed effect text. The "w/[Hero] trait" half is expressible (`trait_has: Hero`); the "w/＜Save＞ in text" half is not — so the cost-3 alt-path cannot be faithfully expressed as a whole (it is a single OR-condition path).
- **Suggested change:** Add a `save_in_text: bool` (or a generalized `keyword_in_text: <keyword>`) predicate leaf usable in alt-path `from:` filters, evaluated against the source card's printed effect text / parsed keyword set.
- **Workaround:** None faithful. BT21-072's `alt_paths` ships the standard cost-4 path only; the cost-3 ＜Save＞/Hero path is omitted. BT21-072 is PARTIAL.

## ~~DSL Gap: BT21-093 — declinable `activation_cost: { trash_self: true }`~~  [G-ACTIVATION-COST-TRASH-SELF] — RESOLVED 2026-05-21
- **Status:** RESOLVED 2026-05-21 (`unblock-medusamon-partial-cards`). `activation_cost: { trash_self: true }` added (`CompiledActivationCostKind::TrashSelf` → `EffectContext::trash_self_as_cost`); declinable per Comprehensive Rules 16-16-2. Full entry archived in `qa/resolved-gaps.md`.
- **Discovered in:** Medusamon archetype re-attempt run, BT21-093 Raging Serpentine (2026-05-21).
- **Scope:** DSL.
- **Card(s):** BT21-093 Raging Serpentine — `[All Turns] ＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)`. Any ＜Delay＞ card whose activation cost is "by trashing this card".
- **Effect text:** "＜Delay＞ (By trashing this card after the placing turn, activate the effect below.)"
- **What's missing:** `activation_cost:` accepts only `suspend_self` and `return_self_to_deck_bottom` (per `compile.rs`). There is no `trash_self: true` variant. Per Comprehensive Rules 16-16-2, ＜Delay＞ processing is OPTIONAL — the controller may decline to trash the card and activate the effect. Without a declinable `trash_self` activation cost, the trash-self must be modeled as the first mandatory body step, which forces the activation once the trigger fires and a valid target exists — suppressing a rules-mandated player choice (a no-approximations violation).
- **Suggested change:** Add `activation_cost: { trash_self: true }` — a declinable activation cost that trashes the source card and gates the body; declining skips the whole Delay.
- **Workaround:** BT21-093 models the trash as a mandatory first body step. PARTIAL.

## DSL Gap: BT25-066 — trash a permanent's own LINK card as a would-leave replacement cost  [G-DSL-LINK-TRASH-AS-REPLACEMENT-COST]
- **Status:** CLOSED (2026-06-07). `cost: { trash_own_link_card: true }` on a `when_would_leave_battle_area` replacement (gap-3a, commit 297a00ab) lowers to `CompiledStep::TrashOwnLinkCardAndCancelLeave`; the preflight gates the optional accept on `replacement_subject.linked_cards.len() >= 1` and surfaces the WHICH-link-card choice even for a single card. **link-finish-replacement slice (2026-06-07)** extends it to the **`scope: linked`** case (an Option's link-card ESS, BT25-101 Divine Arms Version Ω): `lower_replacement.rs` and `lower_aura.rs` now emit `.linked()` for `CompiledScope::Linked`; `replacement.rs::collect_candidates` scans each permanent's `linked_cards` for `.linked()` would-* replacement effects; `source_permanent_is_still_active` accepts the `linked_cards` zone. BT25-066 IMPLEMENTED (`cards/bt25/BT25-066.yaml`, 8/8); BT25-101 inherited leave-replacement IMPLEMENTED (7/7). Verify: `cargo test --test cards_behavioral -- bt25_066 bt25_101`; `cargo test --test option_flow` (126/126).
- **Discovered in:** BT25 "machine" slice, BT25-066 Guardromon (batch-implement-cards-rust-dsl, 2026-06-05).
- **Scope:** DSL.
- **Card(s):** BT25-066 Guardromon — `[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave.` Generalizes to any "by trashing 1 of its link cards, it doesn't leave / prevent" replacement.
- **Effect text:** "[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- **What's missing:** No DSL verb selects and trashes one of a permanent's **own link cards** as the cost of a `kind: replacement` clause. The `ReplacementCostBody` only supports `delay_self: true`; `ReplacementChooseBody.from` only supports `hand`. There is no `select_linked_card` / `trash_linked_card` step, and no `from: linked_cards` for the replacement `choose:`. The engine DOES model the substrate: `Permanent.linked_cards`, `EffectTiming::OnLinkedCardTrashed` (fired in `combat.rs`), and `cancel_replacement` all exist — only the DSL vocabulary to pick + trash a self link card (and gate the cancel on that cost being paid, optionally per DCGO `SetIsSkippable(true)`) is missing. Without it the link-trash player choice cannot surface (no-approximations violation), so the whole card is BLOCKED even though its other clauses (Blocker, inherited +1000 DP, TS-trait alt-digivolve) are individually expressible.
- **Lowers to engine API:** a new `EffectContext` selection over `source_permanent.linked_cards` + trash of the chosen link card (with `OnLinkedCardTrashed` dispatch, already wired) + `cancel_replacement`. The cancel/replacement plumbing already exists.
- **Suggested DSL syntax:**
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true            # DCGO SetIsSkippable(true): the controller may decline
    active_when: { replacement_subject_is_mine: true }
    choose:
      from: linked_cards       # NEW from-zone: this permanent's own link cards
      min: 1
      max: 1
    outcome: prevent           # trashing the chosen link card cancels the leave
  ```
  (Alternatively a dedicated `trash_linked_card_and_cancel_replacement: { of_subject: true }` step usable inside the replacement `process:`.)
- **Workaround:** None faithful. BT25-066 ships no YAML; BLOCKED (dsl).

## DSL Gap: BT25-074 — play a revealed card with the play cost REDUCED by N (not free)  [G-DSL-PLAY-FROM-REVEALED-COST-REDUCED]
- **Status:** CLOSED (2026-06-05). `play_from_revealed_free` now accepts an optional `cost_delta: { reduce: N }` (default free, preserving prior behavior). Engine adds `EffectContext::play_from_revealed_with_cost(player, card, CostDelta)`; `play_from_revealed_free` delegates with `Free`. DSL: `PlayFromRevealedFreeArgs.cost_delta: Option<CostDelta>` → `CompiledStep::PlayFromRevealedFree.cost_delta` (lowered via `compile_cost_delta`) → handler routes through `play_from_revealed_with_cost` (None ⇒ Free, NOT `lower_cost_delta`'s Reduce(0)). Test: `tests/dsl/phase2f1_play_steps.rs::play_from_revealed_with_cost_delta_reduce_pays_remainder` (cost 3 − reduce 2 ⇒ pays 1, no over-credit). BT25-074 is unblocked on this gap.
- **Discovered in:** BT25 "machine" slice, BT25-074 Tankdramon (batch-implement-cards-rust-dsl, 2026-06-05).
- **Scope:** DSL.
- **Card(s):** BT25-074 Tankdramon — `[When Digivolving] [When Attacking] [Once Per Turn] Reveal the top 3 cards of your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait Digimon card among them with the cost reduced by 5. Trash the rest.` Generalizes to any "reveal N, play 1 among them with cost reduced by X" (X > 0, the controller pays the remainder).
- **Effect text:** "Reveal the top 3 cards of your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait Digimon card among them with the cost reduced by 5. Trash the rest."
- **What's missing:** The reveal-pool play steps only support a FREE play. `play_from_revealed_free` hard-codes `crate::enums::CostDelta::Free` (`effect_context/mod.rs:3547`); `choose_from_reveal`'s `play_free` destination is likewise free-only. There is no reveal-pool play step that takes a `cost_delta` to pay a non-zero reduced cost. (The hand analog `play_from_hand` DOES carry `cost_delta: Option<CostDelta>`, and BT15-096 plays from hand with cost reduced by 3 — so the gap is reveal-pool-specific.)
- **Lowers to engine API:** already-present primitive — `play_from_revealed_free` internally calls `Game::play_from_hand_with_cost_result_from_origin(... CostDelta ..., PendingWouldPlayOrigin::Reveal { .. })`, which accepts any `CostDelta`. Only the DSL step pins it to `Free`. `enums::CostDelta::Reduce(i16)` exists.
- **Suggested DSL syntax:** add an optional `cost_delta: CostDelta` (default `Free`) to `play_from_revealed_free` (or a new `play_from_revealed: { of, card, cost_delta }` step), threaded into the existing `from_origin` call instead of the hard-coded `Free`:
  ```yaml
  - reveal_top_deck: { of: you, count: 3, bind_as: revealed }
  - select_reveal_buckets:
      from: revealed
      buckets:
        - bind_as: play_pick
          min: 0
          max: 1
          filter:
            all_of:
              - kind: digimon
              - any_of: [ { trait_has: D-Brigade }, { trait_has: ACCEL } ]
              - play_cost_lte: 12
  - play_from_revealed_free: { of: you, card: play_pick, cost_delta: { reduce: 5 } }
  - per_selected: { selection: revealed, bind_as: rest, body: [ { trash_from_reveal: { of: you, card: rest } } ] }
  ```
- **Workaround:** None faithful (playing for free over-credits the player by 5+ memory; a no-approximations violation). BT25-074 ships no YAML; BLOCKED (dsl). The card's secondary clauses ([All Turns][OPT] on_ally_played → opponent CannotDigivolve, and the inherited [Opponent's Turn] Reboot+Blocker) are individually expressible but cannot ship without the main WD/WA clause.

---

## G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER — multi-card / multi-Tamer face-down trash cost (BT25 BEATBREAK)

- **Status:** ✅ CLOSED (2026-06-15). New DSL verb
  `trash_bottom_face_down_sources_under_tamers: { of, count }` (step.rs
  `TrashBottomFaceDownSourcesUnderTamersArgs` → `CompiledStep` →
  `dsl_cards/step/selections.rs::install_trash_n_bottom_face_down_sources_under_tamers`).
  It trashes `count` bottom face-down sources total, distributed across the
  controller's Tamers — "N from one Tamer" or "1 from each of N Tamers" — by
  installing one single-Tamer bottom-trash `select_own_permanent` pick per
  source, re-evaluating eligibility each time, so every Tamer pick surfaces as a
  real `PendingSelection` (no auto-resolve, DCGO `CanEndSelectCondition`
  reachable). Unpayable (fewer than `count` total face-down sources) ⇒
  `cost_unpayable` + clause abort, like the single-trash verb. Paired with a new
  no-subject predicate `face_down_sources_under_tamers_gte: <N>` that gates the
  optional digivolve on the cost being payable. Driver BT25-035 ships
  IMPLEMENTED (`cards/bt25/BT25-035.yaml`, `tests/cards_behavioral/bt25/bt25_035.rs` 12/12).
- **Cards:** BT25-035 Cougarmon (`[On Play][When Digivolving] ... by trashing 2 bottom face-down cards from under any of your Tamers, this Digimon may digivolve into a [Glowing Dawn] Digimon for free`). Likely also BT25-019 / other BEATBREAK "trash N" cost cards.
- **What's missing:** The shipping verb `trash_bottom_face_down_source_under_tamer: { of }` trashes exactly **one** bottom face-down source from **one** chosen Tamer (it installs a single `select_own_permanent` over `{ kind: tamer, has_face_down_source: true }` and trashes that Tamer's bottom face-down card, then runs the tail). It has no `count:` parameter and no support for distributing the cost across multiple Tamers (DCGO BT25_035: `maxCount: 2`, `canEndNotMax: true`, `CanEndSelectCondition = (picked==2) || (picked==1 && that Tamer has >=2 face-down sources)` — i.e. "trash 2 total: either 2 from one Tamer, or 1 from each of two Tamers"). Chaining the single-trash verb twice does NOT work: each invocation installs a selection and runs the *captured tail* on resolution, so two sequential invocations cannot share one continuation cleanly, and the "2 from one Tamer OR 1+1 from two Tamers" choice is not expressible.
- **Lowers to engine API:** the engine already has the per-Tamer bottom-face-down trash primitive (`install_trash_bottom_face_down_source_under_tamer` in `dsl_cards/step/selections.rs`, and DCGO mirrors it with `TrashDigivolutionCardsFromTopOrBottom(trashCount: N, isFromTop: false, CanTrashCard)`). The missing piece is a DSL step that drives an N-total multi-pick over Tamers with the DCGO end-condition.
- **Suggested DSL syntax:** a `count:`-carrying variant, e.g.
  ```yaml
  - trash_bottom_face_down_sources_under_tamers: { of: you, count: 2 }
  ```
  with semantics: pick Tamers (each must carry >=1 face-down source) until `count` face-down sources are trashed total; a single Tamer with >=`count` face-down sources may satisfy it alone (DCGO `CanEndSelectCondition`). The whole step is the activation cost: if fewer than `count` face-down sources exist across all Tamers, the cost is unpayable → abort the clause (`TailAlreadyRan`), matching the single-card verb's unpayable behavior.
- **Workaround:** None faithful for BT25-035. The single-trash verb under-charges (trashes 1 instead of 2) — a no-approximations violation. BT25-035 BLOCKED (dsl) on this gap. (Its [On Play][When Digivolving] -3000 DP rider IS expressible; the free-digivolve-by-2-trash cost is the blocked part.) **[RESOLVED — see Status above.]**

---

## G-DSL-PLACE-REVEALED-CARD-UNDER-TAMER — place a revealed card face-down under a chosen Tamer (BEATBREAK reveal-pool stash)

- **Status:** ✅ CLOSED (2026-06-15). The existing `place_selected_card_under_tamer` DSL step now resolves a **reveal-pool**-bound card (in addition to hand / trash / union-zone): its `ResolvedBinding::Card` / singleton-`CardList` arm scans `Game::revealed_cards` and calls the new `EffectContext::place_revealed_card_under_tamer` (engine helper that places a `CardSourceRef::Reveal` card as the bottom-most, optionally face-down, source of a chosen own Tamer). Driver ST23-06 Gekkomon ships IMPLEMENTED (`cards/st23/ST23-06.yaml`, `tests/cards_behavioral/st23/st23_06.rs` 7/7).
- **Cards:** ST23-06 Gekkomon (`[When Moving][On Play] Reveal the top 3 cards of your deck. Among them, add 1 [Glowing Dawn] card to the hand AND place 1 [Glowing Dawn] card face down under any of your [Glowing Dawn] trait Tamers. Return the rest to the bottom of the deck`). Likely also ST24 / other BEATBREAK reveal-and-stash cards.
- **What was missing:** `place_selected_card_under_tamer` resolved only hand / trash / union-zone card bindings; a `select_reveal_buckets` / `select_reveal` pick (which still lives in the transient reveal pool, stored as a one-element `CardList`) fell through to the `_ => None` arm, so the second revealed card was never tucked under the Tamer (it leaked into the deck-bottom remainder).
- **Lowers to engine API:** the placement substrate (`place_as_bottom_source` honoring `CardSourceRef::Reveal` + `face_down`) already existed; the gap was a DSL-lowering reveal-pool branch + a thin `place_revealed_card_under_tamer` helper.
- **Note:** when no [Glowing Dawn] Tamer exists, only the add-to-hand bucket runs (DCGO `HasMatchConditionOwnersPermanent` gate) — modelled with an `if any_permanent { tamer + Glowing Dawn }` / `else` two-path reveal.

---

## BT25 "titan" slice — BLOCKED cards (2026-06-06)

Implemented in this slice: BT25-006, BT25-068, BT25-071, BT25-019 (all IMPLEMENTED).
The four cards below are BLOCKED; each is cross-referenced to the controlling gap.

### BT25-069 Raremon — `[On Play][When Digivolving] link 1 [TS] card from your trash to 1 of your Digimon for free`  [gap_kind: dsl]
- **What's missing:** A DSL step that **selects a card from the trash and links it** to a chosen own Digimon. The shipping `link_to_own_digimon` verb is hardwired to link the *carrier Option card* (it reads `pending_option` / installs a `LinkSelectHost` over the carrier — see `dsl_cards/step/mod.rs::try_run_link_step`). It cannot select an arbitrary trash card as the link card. DCGO `BT25_069.cs` uses `SelectCardEffect Root.Trash` → `selectedPermanent.AddLinkCard(cardForLinking)`.
- **Lowers to engine API:** the engine already has the host/link substrate (`Permanent.linked_cards`, link-host selection) — the missing piece is a DSL verb to pick a trash card + pick a host Digimon + attach. Belongs to the broader **`[Link]` subsystem** gap in `docs/RUST_ENGINE_GAPS.md` (item 9: "alternate-source linking from trash").
- **Suggested DSL syntax:** `link_card_from_trash_to_own_digimon: { of: you, free: true, card_filter: { trait_has: TS }, host_filter: { kind: digimon } }`.
- **Other clauses** (Jamming, inherited +1000 DP, TS-trait alt-digivolve) are individually expressible; the card ships no YAML because its sole active clause is blocked.

### BT25-073 Dragomon — trash a link card as an ACTIVATION cost  [gap_kind: dsl]  [G-DSL-LINK-TRASH-AS-COST]
- **Status:** OPEN (re-adjudicated 2026-06-07, link-finish-replacement slice). gap_kind narrowed `hybrid → dsl`: the inherited leave-replacement is now expressible (see G-DSL-LINK-TRASH-AS-REPLACEMENT-COST, CLOSED) and Jamming is declarative, but the **Main clause activation cost is still BLOCKED** on the DSL — so the card ships no YAML.
- **Main clause** `[On Play][When Digivolving] By trashing 1 of your Digimon's link cards, you may play or use 1 [TS] cost<=5 card from hand free`: needs a step that **selects an own Digimon (with >=1 link card), selects one of ITS link cards, and trashes it as an ACTIVATION cost**, then runs a gated play/use of the chosen hand card. DCGO `BT25_073.cs`: `SelectPermanentEffect` (own Digimon, `!HasNoLinkCards`) → `SelectCardEffect Root.LinkedCards` (maxCount 1) → `TrashLinkCardsAndProcessAccordingToResult` → `successProcess` plays/uses the hand card free.
- **What's missing:** the only link-card-trash vocabulary, `cost: { trash_own_link_card: true }`, is **replacement-only** — it reads the `replacement_subject` binding, cancels the leave, and is valid solely on a `when_would_leave_battle_area` replacement (`outcome: prevent`). There is no **general activation-cost** step that picks an arbitrary own Digimon + one of its link cards + trashes it (then continues a tail). The `link_cards` step family only *attaches* cards; none trashes a permanent's link card.
- **Lowers to engine API:** substrate exists — `Permanent.linked_cards`, `Game::trash_specific_link_card(host, card)` (added in gap-3a, fires `OnLinkedCardTrashed`), and the standard play/use-free flow. The missing piece is a DSL cost step, e.g. `trash_link_card_of_own_digimon: { of: you }` that installs the own-Digimon select (filter `has_link_cards`) → link-card select → `trash_specific_link_card`, exposed as an activation cost whose success gates the tail (the play/use). Unpayable (no own Digimon has a link card) ⇒ abort the clause.
- **Suggested DSL syntax:**
  ```yaml
  - when: [on_play, when_digivolving]
    process:
      - trash_link_card_of_own_digimon: { of: you }   # NEW cost step
      - select_hand:
          of: you
          bind_as: play_pick
          optional: true
          filter: { all_of: [ { trait_has: TS }, { play_or_use_cost_lte: 5 } ] }
      - play_or_use_from_hand: { of: you, card: play_pick, cost: free }
  ```
- **Other clauses** (Jamming; inherited leave-replacement) are expressible, but the Main clause is the defining active clause, so BT25-073 ships no YAML. BLOCKED (dsl).

### BT25-083 LadyDevimon — bottom-source-from-hand/trash + trash-digivolution-option-as-cost + cost-reduced trash-option use  [gap_kind: hybrid]
- **Clause 1** `[On Play][When Digivolving] By placing 1 [Three Musketeers] card from your hand OR trash as any of your Digimon's bottom digivolution cards, <Draw 1>`: needs a **zone-choice (hand|trash) picker that places the chosen card as a bottom digivolution source** of a selected Digimon. `place_as_bottom_source` exists for reveal/deck-sourced cards but there is no hand-or-trash-sourced bottom-placement verb with the DCGO 3-way `SetIntSelection` (Hand / Trash / Don't place).
- **Clause 2** `[When Digivolving][When Attacking][OPT] By trashing 1 Option card from any of your Digimon's digivolution cards, you may use 1 [Three Musketeers] Option from your trash with cost reduced by 3`: needs (a) a step to **select+trash an Option from a permanent's digivolution stack as a cost**, and (b) **use a trash Option with a play/use cost reduction** (`UseOptionFromTrash` with `cost: { reduce: 3 }`). The DSL has `use_option_from_hand` but no trash-rooted reduced-cost option-use, and no "trash an option from digivolution cards" cost step.
- **Inherited OnDeletion** "play a level 4 or lower [Three Musketeers]-text Digimon from trash free" IS expressible; the card ships no YAML because the two active clauses are blocked.

### BT25-091 Monica Simmons — `[Your Turn] When you use [TS] Option cards` trigger  [G-DSL-ON-USE-OPTION-TIMING]  [gap_kind: dsl]
- **Card(s):** BT25-091 Monica Simmons (clause 3). Generalizes to any "[Your Turn] When you use … Option cards, …" trigger.
- **Effect text:** "[Your Turn] When you use [TS] trait Option cards, by suspending this Tamer, 1 of your opponent's Digimon can't attack until their turn ends."
- **What's missing:** No DSL `when:` token lowers the engine timing `EffectTiming::OnUseOption` (defined in `code/digimon-engine/src/enums.rs:318`, fired by the engine when a player uses an Option). `digimon-dsl/src/compile.rs` has no `on_use_option` / `when_you_use_option` arm, so the clause cannot be authored. DCGO `BT25_091.cs` uses `EffectTiming.OnUseOption` + `CanTriggerWhenOwnerUseOption(OptionTrigger)`.
- **Lowers to engine API:** the engine timing + dispatch already exist; only the DSL needs a `when: on_use_option` token (optionally with an `option_filter:` predicate for the "[TS] trait Option" gate). The clause body (`activation_cost: { suspend_self: true }` + `select_opponent_permanent` + `add_modifier: CannotAttack` `expiry: end_of_opponents_turn`) is otherwise fully expressible.
- **Suggested DSL syntax:** `when: on_use_option` with optional `option_filter: { trait_has: TS }`.
- **Other clauses** (start-of-turn set-memory-to-3, On Play return-or-draw, [Security] play-self) are implemented and tested (`bt25_091.rs`); BT25-091 ships PARTIAL with this one clause deferred.

### BT25-092 Asuna Shiroki — `[Main]` digivolve into a card from {hand|trash} with a {hand|digivolution-card} trash cost  [G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST]  [gap_kind: dsl]
- **Card(s):** BT25-092 Asuna Shiroki (clause 2).
- **Effect text:** "[Main] By suspending this Tamer and trashing 1 Option card from your hand or your Digimon's digivolution cards, 1 of your Digimon may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand or trash with the cost reduced by 1."
- **What's missing:** two distinct union-zone gaps in one clause:
  1. The digivolve **result** is chosen from a union of **{hand, trash}**, but `effect_initiated_digivolve`'s `from_hand` binding does not accept a `select_union_zone` (hand-or-trash) result, and digivolve-into-a-card-resident-in-trash is unverified in the `EffectInitiatedDigivolve` lowering (`dsl_cards/step/play_digivolve.rs`).
  2. The **cost** trash is from a union of **{your hand, your Digimon's digivolution cards}** — `select_union_zone` does not span a *permanent's own digivolution sources*, and there is no single verb to trash an Option from "hand OR digivolution cards" as a cost.
- **Lowers to engine API:** the substrate pieces exist individually (`select_union_zone`, `select_own_sources`, `trash_selected_sources`, `effect_initiated_digivolve` with `cost: { reduce: 1 }`, `activation_cost: { suspend_self: true }`); the missing piece is (a) digivolve `from_source` accepting a union/trash-bound result, and (b) a cost-trash union spanning hand + a chosen permanent's digivolution stack. Authoring a hand-only-result / hand-only-cost reduction would silently drop player choices (no-approximations), so the clause is left BLOCKED rather than approximated.
- **Suggested DSL syntax:** allow `effect_initiated_digivolve: { source: <select_union_zone binding over {hand, trash}>, cost: { reduce: 1 } }` plus a cost step like `trash_selected_from_union: { zones: [hand, digivolution_cards], filter: { kind: option } }`.
- **Other clauses** (start-of-main trash-to-draw+memory, [Security] play-self) are implemented and tested (`bt25_092.rs`); BT25-092 ships PARTIAL with clause 2 deferred.

### BT25-101 Divine Arms Version Ω — link a [TS] card from trash + inherited link-ESS clauses  [gap_kind: hybrid]
- **Status:** RESOLVED → **IMPLEMENTED** (link-finish-replacement slice, 2026-06-07). `cards/bt25/BT25-101.yaml`, `tests/cards_behavioral/bt25/bt25_101.rs` (7/7).
- **Card(s):** BT25-101 Divine Arms Version Ω.
- **Main clause** `[Main] By trashing 1 [TS] card from hand, <Draw 2>; After, you may link this card OR 1 [TS] card from your trash to 1 of your Digimon without paying the cost`: now expressed with `link_cards: { from: [self_option, trash], filter: { trait_has: TS }, to: own_digimon, count: { up_to: 1 }, cost: free }` (gap-2 `link_cards` step + gap-3b `self_option` from-zone). The "link THIS card" branch attaches the Option to itself; the "1 [TS] from trash" branch links a chosen trash card; a zone-choice surfaces when both qualify. Trash-hand-cost gates Draw 2 + the link via `select_hand { cost: true }` → `if binding_present` → `draw` → `link_cards`.
- **Inherited link-ESS clauses** (`<Security A. +1>`, `<Reboot>`, leave-replacement): authored as **`scope: linked`** (the working link-ESS convention) and now reach the host. Required engine widening this slice: `lower_aura.rs` + `lower_replacement.rs` emit `.linked()` for `CompiledScope::Linked`; `replacement.rs::collect_candidates` scans `linked_cards` for `.linked()` would-* replacement effects; `source_permanent_is_still_active` accepts the `linked_cards` zone. (The keyword/DP/formula ESS host-reach via `tick_declarative_effects` / `live_declarative_formula_sum` linked passes was already closed — see `docs/RUST_ENGINE_GAPS.md` G-LINK-INHERITED-ESS.)
- **Verdict:** IMPLEMENTED. The DCGO `EqualsCardName("Vulcanusmon")` host gate is an implementation detail not in the printed text; the link requirement is modeled as a generic Digimon-host link (cost 3) per printed text.

---

## BT25 "orphan-d" slice — BLOCKED cards (2026-06-06)

Implemented in this slice: BT25-055 Deramon, BT25-042 ClavisAngemon (both IMPLEMENTED).
The five cards below are BLOCKED; each is cross-referenced to the controlling gap.

> **Re-adjudicated 2026-06-07 ("link-appmon-2" slice — BT25-070/056/072/060).** Re-ran
> these (plus BT25-070 Logamon, same family) against current `main`. Update: the
> **`when: when_linked` DSL token HAS since landed** (`Timing::WhenLinked` →
> `CompiledTiming::WhenLinked`, `clause.rs:146` / `compile.rs:258`; G-DSL-WHEN-LINKED-TIMING
> closed), so the secondary "no when-token" blocker noted below is stale. The **controlling
> blocker is unchanged and still open**: there is no engine primitive — and no DSL verb
> lowering to one — to link a *chosen* card from `{hand | trash | digivolution-cards}` onto
> a host Digimon (the shipping `link_to_own_digimon` links only the carrier Option; the
> 2026-06-06 Shape-B substrate only *absorbs a standing permanent*, root `None`). This is
> facet **#9** of the `docs/RUST_ENGINE_GAPS.md` `[Link]` keyword subsystem (DCGO
> `ILinkCard(cardSource, host)` / `Permanent.AddLinkCard(cardSource)` with `SelectCardEffect.Root`
> = `Hand`/`Trash`/`DigivolutionCards`). All four remain **BLOCKED (hybrid)**; ship no YAML
> under the no-approximations policy (the link clause is each card's central mechanic and
> gates the `WhenLinked`/`[When Linking]` payloads). DCGO confirmed this run:
> `BT25_070.cs:181`, `BT25_056.cs:196`, `BT25_072.cs:201`, `BT25_060.cs:160`.

### BT25-056 Bootmon — host-Digimon link-from-{hand|digivolution-cards} + `When this gets linked` trigger  [gap_kind: hybrid]
- **Effect text:** "[On Play][When Digivolving][When Attacking] If it's your turn, you may link 1 [Social], [Tool] or [Game] trait Digimon card from your hand or this Digimon's digivolution cards to this Digimon with the cost reduced by 2. [All Turns] When this Digimon gets linked, suspend 1 of your opponent's Digimon or Tamers." Plus `<Barrier>` (expressible) and inherited "Return 1 of your opponent's suspended Digimon to the bottom of the deck" (expressible).
- **What's missing (two facets):**
  1. **Host-Digimon link from a chosen card in hand / digivolution-cards** (with a `-2` link-cost reduction). The shipping `link_to_own_digimon` links only the *carrier Option* (reads `pending_option` / installs `LinkSelectHost` over the carrier). No DSL verb selects a Digimon card from hand or this permanent's digivolution stack and attaches it as a link card to the carrier Digimon. DCGO `BT25_056.cs`: `AddSelfLinkConditionStaticEffect` + 3-way `SetIntSelection` (Hand / DigivolutionCards / Don't) → `ILinkCard(cardSource, card.PermanentOfThisCard())` + `GrantedReduceLinkCostClass`. Same family as `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem (facet #6) and the documented `link_card_from_trash_to_own_digimon` gap (BT25-069/101) — here zones are hand + digivolution-cards.
  2. **`When this Digimon gets linked` trigger.** Engine has `EffectTiming::WhenLinked` (`enums.rs:333`) but no DSL `when:` token lowers to it (`CompiledTiming` has no `WhenLinked`; `compile.rs` has no `when_linked`/`gets_linked` arm). Consumer side of `docs/RUST_ENGINE_GAPS.md` facet #11.
- **Lowers to engine API:** link substrate (`Permanent.linked_cards`, `attach_linked_card`, `ChangeLinkCost`) + `EffectTiming::WhenLinked` dispatch exist; missing are (a) a DSL host-link-from-source verb and (b) a `when: when_linked` token.
- **Verdict:** BLOCKED (hybrid). Ships no YAML. Cross-ref `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem + BT25-069/101.

### BT25-072 Shutmon — host-Digimon link-from-{trash|digivolution-cards} + `When this gets linked` deny-digivolve  [gap_kind: hybrid]
- **Effect text:** "[On Play][When Digivolving][When Attacking] If it's your turn, you may link 1 [Social], [Tool] or [Game] trait Digimon card from your trash or this Digimon's digivolution cards to this Digimon with the cost reduced by 2. [All Turns][Once Per Turn] When this Digimon gets linked, 1 of your opponent's Digimon or Tamers can't digivolve until their turn ends." Plus `<Jamming>` (expressible) and inherited "2 of your opponent's Digimon or Tamers can't unsuspend until their turn ends" (expressible).
- **What's missing:** identical two facets as BT25-056 — (1) host-Digimon link from a chosen card (here **trash** + digivolution-cards), exactly the documented `link_card_from_trash_to_own_digimon` gap (BT25-069/101) extended to also span the carrier's digivolution stack; (2) the `When this gets linked` (`WhenLinked`) DSL `when:` token. DCGO `BT25_072.cs`: `ILinkCard` from `SelectCardEffect.Root.Trash`/`DigivolutionCards` + `WhenLinked` ActivateClass installing `CannotDigivolve` until the opponent's turn ends.
- **Lowers to engine API:** as BT25-056; plus `ModifierType::CannotDigivolve` (exists) for the gets-linked body.
- **Verdict:** BLOCKED (hybrid). Cross-ref BT25-056, BT25-069/101, `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

### BT25-060 Rebootmon — host-Digimon free-link-from-{hand|digivolution-cards} + `When this gets linked OR unsuspends` self-buff  [gap_kind: hybrid]
- **Effect text:** "[When Digivolving][When Attacking][Once Per Turn] By linking 1 [Appmon] trait Digimon card from your hand or this Digimon's digivolution cards to this Digimon without paying the cost, 1 of your Digimon may unsuspend. [All Turns][Once Per Turn] When this Digimon gets linked or unsuspends, until your turn ends, this Digimon gains <Piercing> and <Blocker>, and your opponent's Digimon effects don't affect it." Plus `<Security A. +1>`, `<Reboot>`, `<Link +1>` (declarative — expressible).
- **What's missing:** (1) host-Digimon **free link from hand / digivolution-cards** as an *activation cost* (the "by linking …, 1 may unsuspend" is gated on the link succeeding) — same missing host-link-from-source verb as BT25-056; (2) the **`When this gets linked OR unsuspends`** trigger — `OnUnsuspend` has a DSL token but **`WhenLinked` does not**, so one leg of the multi-timing trigger is unauthorable. DCGO `BT25_060.cs`: `AddLinkCard(cardSource)` as cost → unsuspend; `WhenLinked` + `OnUnTapped` ActivateClasses granting Piercing/Blocker + DigimonEffectImmunity.
- **Lowers to engine API:** link substrate + `EffectTiming::WhenLinked`/`OnUnsuspend` exist; `grant_keyword` Piercing/Blocker + `grant_effect_immunity` exist for the body. Missing: host-link-from-source verb (as activation cost) + `when: when_linked` token.
- **Verdict:** BLOCKED (hybrid). Cross-ref BT25-056, `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem (facets #6/#10/#11).

### BT25-085 BeelStarmon — use-Option-from-{hand|digivolution-cards} free + trash-Option-from-{digivolution|link}-cards-as-cost → unsuspend  [G-DSL-USE-OPTION-FROM-SOURCES + G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST]  [gap_kind: dsl]
- **Effect text:** "[When Digivolving][When Attacking][Once Per Turn] You may use 1 [Three Musketeers] or [TS] trait Option card from your hand or this Digimon's digivolution cards without paying the cost. [When Digivolving][When Attacking][Counter][Once Per Turn] By trashing 1 Option card from any of your Digimon's digivolution cards or link cards, this Digimon unsuspends." Plus `<Blocker>` (expressible). Inherited [Main]: "Delete 1 of your opponent's highest level Digimon. Then, you may place 1 card from your hand or trash as any of your Digimon's bottom digivolution card."
- **What's missing (two facets):**
  1. **Use an Option card from a permanent's digivolution stack** (not just hand). Shipping `use_option_from_hand` is hand-only (`UseOptionFromHandArgs` reads `select_hand`). DCGO `BT25_085.cs` offers a `SelectCardEffect.Root.DigivolutionCards` branch with `customRootCardList`.
  2. **Trash 1 Option from any of your Digimon's digivolution OR link cards as an activation cost** (then unsuspend self). DCGO uses `permanent.DigivolutionOrLinkCards` as the trash pool. No DSL step selects+trashes an Option from the union of a permanent's digivolution + link cards as a cost. Same family as BT25-083's "trash an Option from digivolution cards as cost", extended to also span link cards.
  (The inherited [Main] highest-level delete is expressible; the bottom-source-from-{hand|trash} place is the same gap as BT25-083 clause 1.)
- **Lowers to engine API:** the use-Option / source-trash primitives exist in principle; missing is DSL vocabulary to root a use/trash at a permanent's digivolution+link stack.
- **Verdict:** BLOCKED (dsl). Ships no YAML — both [WD][WA] active clauses depend on source-rooted Option verbs.

### BT25-076 Ghoulmon — `When this would be played, by deleting your own Digimon, reduce cost by the deleted Digimon's play cost`  [G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION]  [gap_kind: hybrid]
- **Effect text:** "When this card would be played, by deleting 1 of your play cost 11 or lower Digimon with [Negamon] in its digivolution cards and [Negamon] in its text, reduce the cost by the deleted Digimon's play cost." Plus `<Rush>`, `<Reboot>`, `<Blocker>` (declarative — expressible) and "[On Play][When Attacking][On Deletion] Delete 1 of your opponent's lowest-play-cost Digimon; if it didn't delete, trash your opponent's top security" (expressible: `select_opponent_permanent` over a lowest-play-cost gate → `delete_permanent`, else `trash_top_security`).
- **What's missing:** a **`BeforePayCost` cost reducer whose payment is a player-selected deletion of an OWN permanent and whose reduction amount is that permanent's play cost (variable)**. Shipping `BeforePayCost` reducers (`lower_cost_reduction.rs`) carry a passive `amount`/`amount_fn`/`raw_rust` value; none install an interactive `select_own_permanent` + `delete_permanent` *as the cost*, nor read the deleted permanent's `GetCostItself` as the reduction delta. DCGO `BT25_076.cs` `EffectTiming.BeforePayCost`: optional `SelectPermanent` (canNoSelect modulated by affordability) over own Negamon-text + Negamon-source cost<=11 Digimon → `DeletePeremanentAndProcessAccordingToResult` → register a `ChangeCostClass` of `-reducedCost`.
- **Why not approximated:** authoring an `amount_fn` (max-cost-Negamon) without the player-selectable deletion would (a) silently auto-pick which Negamon to delete and (b) silently delete it — two no-approximations violations; DCGO presents an explicit `canNoSelect` choice. The cost-reduction-by-player-deletion is this card's core play-enabler, so the card is BLOCKED rather than shipping a PARTIAL that drops the choice.
- **Lowers to engine API:** deletion, play-cost read, and cost-delta primitives exist individually; missing is a DSL `BeforePayCost` reduction clause that drives an interactive delete-own-as-cost with a variable, deleted-permanent-sourced reduction. Engine-side, the `BeforePayCost` dispatch would need to host an interactive selection at cost-calc time (currently passive) — hence `gap_kind: hybrid`.
- **Verdict:** BLOCKED (hybrid). Ships no YAML.

### BT25-061 Offmon — Appmon `<Link>` keyword on a Digimon + `[When Linking]` trigger  [gap_kind: dsl]
- **✅ RESOLVED 2026-06-07.** Both cited facets landed with DigiLink Shape-B (`G-DSL-DIGILINK`, 2026-06-06): facet 1 → `kind: link_condition` (Digimon self link-condition), facet 2 → `when: when_linked`. BT25-061 now ships `code/digimon-engine/cards/bt25/BT25-061.yaml` + 7 green tests (`tests/cards_behavioral/bt25/bt25_061.rs`); verdict IMPLEMENTED in `validated_cards_dsl.json`. (Original gap text retained below for history.)
- **Effect text:** "[Start of Your Main Phase] By trashing 1 card with the [Appmon] trait from your hand, <Draw 1> and gain 1 memory. [When Linking] 1 of your opponent's Digimon can't unsuspend until their turn ends." Plus the Appmon `<Link>` keyword (Offmon is itself a *Digimon* that gets linked to another [Appmon] host) and alt-digivolve Lv.2 [Appmon] / Cost 0 (alt-path is expressible). NOTE: cards.json labels the "can't unsuspend" line as the *inherited* effect, but DCGO `BT25_061.cs` implements it as a `WhenLinked` ActivateClass behind the card's Link keyword (`AddSelfLinkConditionStaticEffect` + `LinkEffect` + `WhenLinked`).
- **What's missing (two facets, both already-documented families):**
  1. **A Digimon declaring itself as a Link-keyword Digimon** — DCGO `AddSelfLinkConditionStaticEffect(permanentCondition: HasAppmonTraits, linkCost: 1)`. The shipping DSL `kind: link_requirement` is documented as "for Link **Options**" only (`LinkRequirementBody`), and every current consumer is `kind: option`. No vocabulary lets a `kind: digimon` card register itself as a host-attachable Link card with a host predicate + link cost.
  2. **The `[When Linking]` / `WhenLinked` trigger** — `EffectTiming::WhenLinked` exists in `enums.rs` but no DSL `when:` token lowers to it (`CompiledTiming` has no `WhenLinked`; `compile.rs` has no `when_linked`/`when_linking`/`gets_linked` arm). This is the SAME consumer-side gap already logged for BT25-056 / BT25-072 / BT25-060 — cross-ref `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem facet #11.
- **Why BLOCKED not PARTIAL:** the Start-of-Your-Main-Phase trash→draw+memory clause and the alt-digivolve ARE expressible, but the `<Link>` keyword and its `[When Linking]` payload are the card's defining Appmon mechanic; shipping them dropped would be a silent omission (no-approximations). Ships no YAML.
- **Lowers to engine API:** `WhenLinked` timing + link substrate exist; missing is (a) a Digimon-scoped self-link-condition DSL declarative and (b) a `when: when_linked` token. The "can't unsuspend until their turn ends" payload itself (`CannotUnsuspend` + `UntilOpponentTurnEnd` over a selected opponent Digimon) is expressible once the trigger exists.
- **Verdict:** BLOCKED (dsl). Cross-ref BT25-056 / BT25-072 / BT25-060 (`WhenLinked` token) and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

### BT25-086 Dan Yuki — DP modifier formula `× opponent's memory count`  [G-DSL-FORMULA-OPPONENT-MEMORY]  [gap_kind: dsl]
- **Effect text:** "[Start of Your Main Phase] If you have 4 or less memory, gain 1 memory. [End of Your Turn] By suspending this Tamer, 1 of your [TS] trait Digimon gains +1000 DP for the turn for each memory your opponent has. Then, it may attack. [Security] Play this card without paying the cost." (Cost 3 [TS] Tamer.)
- **What's missing:** a **formula source that reads a player's memory-gauge value as a scalar**, so `add_dp_modifier` can compute `+1000 × opponent_memory`. `FormulaSpec::BasePerDelta`'s `PerSelector` enum (`formula.rs`) covers `material_count` / `stack_size` / `ally_count` / `suspended_count` / color counts / `card_count_in_zone` — but there is **no `per: opponent_memory`** (nor `your_memory`) selector, and `CompoundFormula::Aggregate` only ranks permanents by DP/level/cost. The memory gauge is a single signed integer on `Game` (`game.memory` / `gain_memory_for_player`), not a zone count, so `card_count_in_zone` cannot express it either. DCGO `BT25_086.cs`: `dpGain = Math.Max(0, card.Owner.Enemy.MemoryForPlayer * 1000)`.
- **Why BLOCKED not PARTIAL:** the Start-of-Your-Main-Phase memory-floor gain (`condition: { memory_lte: 4 }` → `gain_memory: 1`) and the `[Security] play_from_security` clause ARE expressible; the suspend-self cost (`activation_cost: { suspend_self: true }`) and `may_attack_now` exist too. But the End-of-Turn clause's DP grant is *variable in the opponent's memory* — authoring it with a literal or any existing `per:` source would misstate the buff (no-approximations). The End-of-Turn clause is the card's whole payoff, so it ships no YAML rather than a PARTIAL that fakes the DP amount.
- **Lowers to engine API:** `ctx` can already read `game.memory` (and DCGO reads `Enemy.MemoryForPlayer`); the gap is purely a DSL formula vocabulary one — add a `PerSelector::PlayerMemory { of: PlayerRef }` (or a `FormulaSpec::PlayerMemory`) that the existing `formula_eval` evaluates against the gauge, then `add_dp_modifier: { value: { formula: { base: 0, per: { player_memory: { of: opponent } }, delta: 1000 } }, expiry: end_of_turn }`.
- **Verdict:** BLOCKED (dsl). Ships no YAML.

## G-DSL-WHEN-LINKED-TIMING — [When Linking] triggered clause has no `when:` timing (2026-06-06)

- **✅ RESOLVED 2026-06-07 (the DSL timing).** `when: when_linked` landed with DigiLink Shape-B (`G-DSL-DIGILINK`, 2026-06-06): `Timing::WhenLinked` → `CompiledTiming::WhenLinked` → `EffectTiming::OnLink` + forced `.linked()` + self-filter. The `[When Linking]` clause is now authorable (see BT25-007/BT25-061 which use it). **BT25-036 itself remains BLOCKED**, but on a *different* primitive: **App Fuse** (`AddAppfuseMethodByName`) is not implemented in the engine (no lowering, no handler; `AltPathKind::AppFusion` parses but resolves to nothing) — re-classified to `gap_kind: engine`, tracked in `docs/RUST_ENGINE_GAPS.md` App Fuse entry. (Original gap text retained below for history.)
- **Card:** BT25-036 Craftmon (orphan-b slice). DCGO `BT25_036.cs` region "When Linking" uses `EffectTiming.WhenLinked` + `SetIsLinkedEffect(true)` for "[When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>."
- **Gap:** the `digimon_dsl::clause::Timing` enum (the `when:` surface) has no `WhenLinked` / `Linked` variant. `compiled::CompiledTiming::Linked` and `compiled::CompiledScope::Linked` exist, and `lower_triggered.rs` already routes `CompiledScope::Linked` through `builder.linked()`, but there is no `Timing` string that lowers to `CompiledTiming::Linked`, so a "[When Linking]" triggered body cannot be authored.
- **Lowers to engine API:** the engine already fires a link-established event (DCGO `EffectTiming.WhenLinked`); the gap is purely DSL-side — add a `Timing::WhenLinked` variant (serde `when_linked`), map it `S::WhenLinked => CompiledTiming::Linked` in `compile.rs`, and confirm `engine_timing` lowering wires `CompiledTiming::Linked` to the engine's link-established timing.
- **Suggested DSL syntax:** `- when: when_linked` (optionally `scope: linked`) with a `process:` body (here: `select_hand` trash 1 Appmon → `draw 2`).
- **Verdict:** BLOCKED (dsl). BT25-036 ships no YAML — its App-fusion alt-path, link condition, [Security] play-self, and OnPlay add-top-security + <Recovery +1> are all expressible, but the [When Linking] clause is its mandatory inherited payoff and cannot be silently dropped under the no-approximations policy.

## BT25 "beatbreak" slice — BLOCKED / PARTIAL notes (2026-06-06)

Implemented this slice: BT25-081 (IMPLEMENTED); BT25-088, BT25-090, BT25-049,
BT25-035, BT25-041 (PARTIAL — each ships its expressible clauses with one
BLOCKED clause omitted). BT25-057 BLOCKED. Cross-references below; the
controlling engine gap for the cost-reduction clauses is
`G-COST-REDUCTION-INTERACTIVE-PAY-COST` in `docs/RUST_ENGINE_GAPS.md`.

### Glowing Dawn "trash a face-down card under a Tamer → reduce a card's cost" — BLOCKED (engine)
- **Cards:** BT25-088 Kyo Sawashiro (clause 3, play -1), BT25-090 Tomoro Tenma
  (clause 3, Option-use -1), BT25-049 Armalizamon (clause 2, Option-use -3),
  and the cost-reduced half of BT25-041's main clause.
- **What's missing:** `kind: cost_reduction` with a `pay_cost` that installs an
  interactive selection drops the reduction credit. The
  `trash_bottom_face_down_source_under_tamer` verb ALWAYS installs a Tamer-pick
  selection (no-approximations), so as a `pay_cost` it parks
  (`RunOutcome::Parked`), `apply_cost_reduction_candidate` returns `None`, and
  the `amount` is discarded while the face-down card is still trashed. Full
  root-cause + suggested engine fix in `docs/RUST_ENGINE_GAPS.md`
  `G-COST-REDUCTION-INTERACTIVE-PAY-COST`. (The same verb works fine as a
  *process activation cost* — see BT25-041's inherited unsuspend and BT25-057's
  De-Digivolve, which DO compile.)
- **Verdict:** the affected clause is OMITTED from each card's YAML (PARTIAL);
  authoring it would either drop the reduction (over-charge) or require
  auto-resolving the Tamer pick (no-approximations violation).

### BT25-035 Cougarmon — trash-2 free-digivolve — BLOCKED (dsl)
- The "Then, by trashing 2 bottom face-down cards … may digivolve into a
  [Glowing Dawn] card in hand for free" half is the existing
  `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER` gap (multi-count / multi-Tamer
  trash) plus an effect-driven free-digivolve-into-a-hand-card. Omitted; the
  -3000 DP, inherited Barrier, and Glowing Dawn alt-digivolve ship (PARTIAL).

### BT25-057 Monarchlizamon / "Final Judgment" — DUAL card — RESOLVED 2026-06-15  [G-DSL-DUAL-PER-FACE-EFFECTS + G-DSL-ARTS-DIGIVOLVE]

> **RESOLVED 2026-06-15 (`gap/dual-per-face-arts`).** Both gaps closed. The DUAL
> faces now carry their own `effects:` and the Option face an `arts_digivolve:`
> shorthand:
> - **Per-face effects sink (G-DSL-DUAL-PER-FACE-EFFECTS):** `DualDigimonSpec`
>   and `DualOptionSpec` gained an `effects: Vec<ClauseSpec>` field (`spec.rs`),
>   compiled onto `CompiledDualDigimon.effects` / `CompiledDualOption.effects`
>   (`compiled.rs` + `compile.rs`), validated per-face (`validator.rs`), and
>   lowered by `DslCardEffect::effects()` (`dsl_cards/mod.rs`): Digimon-face
>   clauses lower with the Digimon identity (natural timings), Option-face
>   clauses with the Dual identity so `when: main` → `EffectTiming::OptionMain`
>   and the `dual.option.use_requirement` color bypass applies. `clause_index`
>   is offset per face so multi-timing OPT keys never collide. Digimon-face
>   `grant_keyword` declaratives are also scanned into the top-card native
>   `keywords` (`card_data_from_compiled`) so static keywords (Security A.+1 /
>   Reboot / Blocker) are live on field.
> - **Arts Digivolve (G-DSL-ARTS-DIGIVOLVE):** `dual.option.arts_digivolve: true`
>   compiles into the `ArtsDigivolve` option-face keyword, which the existing
>   engine path (`pending_option_can_arts_digivolve` →
>   `install_arts_digivolve_selection`) reads — no engine change needed. The
>   Digimon-face evo table is backfilled from the alt-digivolve box
>   (`compiled_dual_to_engine` now threads the computed `evo_costs`) so the Arts
>   `can_digivolve` gate works for DSL-loaded dual cards.
> - **Cards shipped:** ST23-09 Atratusmon (IMPLEMENTED), BT25-057 Monarchlizamon
>   (IMPLEMENTED — cards.json mislabel corrected via the DUAL YAML), BT25-043
>   Habakirimon (upgraded PARTIAL → IMPLEMENTED, Option side now ships). Tests:
>   `tests/cards_behavioral/{st23/st23_09,bt25/bt25_057,bt25/bt25_043}.rs` (26
>   tests, all green).
> - **Residual (separate pre-existing limitation, NOT this gap):** the engine
>   `can_digivolve` / `can_basic_digivolve` gate is color+level only — a
>   trait-gated digivolution box ("Lv.N w/ [Glowing Dawn]: Cost C") is not
>   enforceable as a static `EvoCost` row. These cards author BOTH a color-form
>   alt-digivolve (the cards.json evo table — backfills the static evo_cost) and
>   the printed trait-form alt-path, matching every other DSL card's
>   digivolution authoring. A faithful trait-gated `can_digivolve` is a
>   standalone engine gap.

### Original entry (history)
- **Note:** `data/cards.json` mislabels this as a plain Digimon (`card_kind: 0`).
  The card IMAGE + DCGO `BT25_057.cs` confirm it is a **DUAL** card: a Lv.5
  Cyborg/Glowing Dawn/BEATBREAK Digimon face AND an Option face "Final
  Judgment" (Use 4).
- **What's missing (DSL):** `digimon-dsl`'s `DualSpec` (`spec.rs`) carries only
  metadata + text for each face (`DualDigimonSpec` / `DualOptionSpec` have NO
  `effects:` field). A dual card therefore cannot attach behavioral clauses to
  either face via the `dual:` block. (Whether a dual card's top-level `effects:`
  route correctly to its Digimon face is also unexercised — no shipping card
  uses `kind: dual` with effects, and the `tests/dual_cards/` suite drives only
  hand-written `CardData`.)
- **What's missing (engine/DSL — Arts Digivolve):** the Option face's "Arts
  Digivolve (instead of trashing after use, your cards may digivolve into this
  card without paying the cost)" has no DSL authoring path. `Keyword::ArtsDigivolve`
  exists and the engine has `install_arts_digivolve_selection` /
  `pending_option_can_arts_digivolve` (checked against `dual.option.keywords`),
  but `DualOptionSpec.keywords` (a `Vec<String>`) is the only hook and the full
  arts-digivolve authoring + behavioral path is untested from YAML.
- **Card faces (all individually expressible IF the dual-per-face-effects gap
  closes):** Digimon face — alt-digivolve Lv.4 Glowing Dawn cost 3; [WD][WA][OPT]
  trash a bottom face-down card under a Tamer (process cost, works) → De-Digivolve
  1 opp Digimon; [WD] this Digimon may battle 1 opp Digimon (`battle:`). Option
  face — Use Req Glowing Dawn (ignore color), [Main] 1 of your Digimon gains
  <Rush> + <Security A. +1> + 5000 DP for the turn, then may attack; Arts
  Digivolve.
- **Suggested DSL syntax:** add `effects:` (and optionally `inherited`/`security`
  scoping) to `DualDigimonSpec` and `DualOptionSpec`, lowering each face's
  clauses onto the appropriate face of the compiled dual card; add an
  `arts_digivolve: true` shorthand (or `keywords: [ArtsDigivolve]`) on the
  option face wired to the existing engine arts-digivolve selection.
- **Verdict:** BLOCKED (hybrid). BT25-057 ships no YAML — both faces' behavioral
  effects are unauthorable and shipping a stat-only dual card with a
  non-functional Option face would be an approximation.

---

## ~~G-DSL-SELECT-OPP-SOURCES-DYNAMIC-CROSS-PERMANENT~~ — RESOLVED 2026-06-13 — player-choice trash of a dynamic count of opponent digivolution source cards across all opponent Digimon
- **RESOLVED 2026-06-13 (G-DSL-SELECT-SOURCES-FORMULA-COUNT):** `SelectOpponentSourcesArgs` now carries `max: CountBound` (accepts a `FormulaSpec`, e.g. `{ source_material_count: {} }`), `clamp_to_available: bool`, and cross-permanent selection (omit `target`). The suggested YAML below is now expressible. BT25-103 needs **re-assessment** (verified stale 2026-06-14, fix-dsl-substrate-rot-and-bugs §6.3) — it may now be fully authorable, or blocked on a *different* clause; re-run the slice rather than trusting the stale BLOCKED verdict below.
- **Discovered by:** BT25-103 GraceNovamon (aegiomon-3 slice), 2026-06-06.
- **Clause:** "[When Attacking] [Counter] [Once Per Turn] For each of this Digimon's digivolution cards, you may trash any 1 digivolution card from your opponent's Digimon. Then, you may end this attack."
- **DCGO (BT25_103.cs):** `CardEffectCommons.SelectTrashDigivolutionCards(permanentCondition: IsEnemyDigimon, maxCount: card.PermanentOfThisCard().DigivolutionCards.Count, canNoTrash: true, isFromOnly1Permanent: false, ...)` — the player picks up to N digivolution source cards (N = this Digimon's digivolution-card count) from **any** opponent Digimon (not restricted to one permanent), each pick optional. Then a separate Yes/No "end the attack?" prompt.
- **What the DSL has:** `select_opponent_sources` (BindingRef-bound source picker) with `min`/`max` as **`u8` literals** and an optional `target` that **restricts the picker to ONE opponent permanent's** digivolution stack (see BT16-085). `trash_selected_sources` + `end_attack` steps both exist; `Counter` timing and `once_per_turn` exist.
- **Why blocked:** two missing capabilities on `select_opponent_sources`:
  1. **Dynamic count** — `max` must accept a `FormulaSpec` (here `{ source_material_count: {} }`, the same formula clause 6's bounce uses), not just a literal `u8`.
  2. **Cross-permanent selection** — when `target` is omitted, the picker must span **all** opponent Digimon's digivolution stacks (DCGO `isFromOnly1Permanent: false`), with each pick choosing both which Digimon and which source card.
- **Lowers to engine API:** the selection machinery for opponent digivolution sources already exists (single-permanent path); the gap is (a) threading a formula-resolved max and (b) a cross-permanent candidate set. Likely a `max_fn: Option<FormulaSpec>` + a `cross_permanent: bool` (or `target` omitted ⇒ cross-permanent) on `SelectOpponentSourcesArgs` and the corresponding engine candidate enumeration.
- **Suggested DSL syntax:**
  ```yaml
  - select_opponent_sources:
      max_fn: { source_material_count: {} }   # dynamic count = this Digimon's digivolution cards
      min: 0                                    # canNoTrash: true (each pick optional)
      cross_permanent: true                     # span all opponent Digimon (isFromOnly1Permanent: false)
      bind_as: trashed
      then:
        - trash_selected_sources: { source_refs: trashed }
  ```
- **Faithfulness impact:** stubbing this as `trash_top_n_digivolution_cards_of_each` (trashes the top-N of EACH opp Digimon with no player choice of which Digimon or which card) would be an auto-selection — a no-approximations violation. Card cannot ship until the dynamic-count cross-permanent source picker exists.
- **Verdict:** BT25-103 BLOCKED (gap_kind: dsl). No YAML shipped (clauses 1–6 are expressible but the whole card is gated on the Counter clause).

---

## G-DSL-BATTLE-WINNER-BOARDWIDE — gate a trigger on "when any of your [trait] Digimon win a battle"
- **Discovered by:** BT25-020 Marsmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[All Turns] [Once Per Turn] When any of your [TS] trait Digimon win a battle, trash your opponent's top security card."
- **DCGO (BT25_020.cs):** `EffectTiming.OnEndBattle` ActivateClass, OPT, gated by `CardEffectCommons.CanTriggerWhenWinBattle(winnerCondition: permanent => permanent.TopCard.Owner == card.Owner && permanent.TopCard.HasTSTraits)`. The trigger fires whenever ANY winner permanent on the controller's side has the [TS] trait — not just the carrier itself.
- **What the DSL has:** `source_deleted_battle_opponent: true` predicate — fires only when the **carrier** is the battle winner (the "this Digimon wins a battle" idiom, ST4-11; used by BT25-048/051/054). There is **no** board-wide "any ally with trait X won a battle" predicate, and `on_any_deletion`'s `event_target_*` predicates describe the **deleted** permanent (the loser), not the winner.
- **Why blocked:** the body (`trash_top_security: { of: opponent }`) and `once_per_turn` exist, but the trigger cannot be gated to "any of your [TS] Digimon win a battle". Shipping it on `on_any_deletion` ungated would fire on every deletion (including the opponent's wins) — an approximation. Narrowing to `source_deleted_battle_opponent` would silently drop the board-wide scope (only the carrier's own wins would count) — also an approximation.
- **Lowers to engine API:** a battle-resolution event already exists (security checks, deletion). The gap is exposing the **winner** permanent (controller + trait) to the triggered-effect predicate layer: e.g. a new timing `on_ally_won_battle` (or `on_battle_end` with `event_winner_*` predicates: `event_winner_owner`, `event_winner_trait_has`).
- **Suggested DSL syntax:**
  ```yaml
  - when: on_ally_won_battle        # fires when a permanent the controller owns wins a battle
    once_per_turn: true
    active_when:
      all_of:
        - all_turns: true
        - event_winner_trait_has: TS
    process:
      - trash_top_security: { of: opponent }
  ```
- **Faithfulness impact:** BT25-020's other clauses (mandatory cost reduction; [OP][WD][WA] +3000 DP + may-battle) are fully expressible, but the card cannot ship faithfully until the board-wide battle-winner predicate exists. No YAML shipped.
- **Verdict:** BT25-020 BLOCKED (gap_kind: dsl).

## G-DSL-PROTECT-OTHER-BY-SELF-DELETE — board-wide "when another of your X would leave, by deleting THIS Digimon, they don't leave"
- **Discovered by:** BT25-039 Sirenmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[All Turns] When any of your other [Shaman] or [Iliad] trait Digimon or Tamers would leave the battle area other than by your effects, by deleting this Digimon, they don't leave."
- **DCGO (BT25_039.cs):** `EffectTiming.WhenRemoveField` ActivateClass whose `CanUseCondition` matches when **another** owner permanent (Digimon or Tamer, Shaman/Iliad trait, `!IsByEffect(owner)`) would leave; the body deletes THIS permanent (`DeletePeremanentAndProcessAccordingToResult`) and, on success, sets `willBeRemoveField = false` on all such protected permanents (cancels their leave).
- **What the DSL has:** the existing replacement substrate (`kind: replacement`, `trigger: when_would_leave_battle_area`) and the `Keyword::Decode`/Barrier/Evade auto-installs are all **self-scoped** — they only fire on the carrier's own would-leave (the `replacement_subject == me` guard in `keyword_effects.rs`). There is no replacement/observer that fires on **another** permanent's would-leave with a trait/owner filter and cancels it by paying a self-deletion cost.
- **Why blocked:** modeling this needs (a) a would-leave replacement whose **subject is a filtered set of OTHER owner permanents** (not self), (b) a cost step that deletes the carrier, and (c) cancelling the original leave for every matching protected permanent. None of these compose from current vocabulary.
- **Lowers to engine API:** the parked-replacement substrate (`cancel_leave` / `handle_replacement`) and `delete_permanent` exist; the gap is a replacement clause with a non-self subject filter (`replacement_subject_is_mine` exists as a predicate but only on the carrier path) plus a cause filter (`other than by your effects`).
- **Suggested DSL syntax:**
  ```yaml
  - kind: replacement
    trigger: when_other_would_leave_battle_area
    subject_filter:
      all_of:
        - of: you
        - other: true
        - any_of: [ { kind: digimon }, { kind: tamer } ]
        - any_of: [ { trait_has: Shaman }, { trait_has: Iliad } ]
    active_when: { none_of: [ { replacement_cause: own_effect } ] }
    process:
      - delete_permanent: { target: source }   # cost
      - cancel_leave: { target: replacement_subject }
  ```
- **Verdict:** contributes to BT25-039 BLOCKED (gap_kind: dsl).

## G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER — security-zone End-of-Turn play of a named card at reduced cost, then place this security card as the played Digimon's bottom digivolution source
- **Discovered by:** BT25-039 Sirenmon (aegiomon-1 slice), 2026-06-06.
- **Clause:** "[Security] [End of Your Turn] You may play 1 [Ceresmon] from your hand with the cost reduced by 7. If this effect played, you may place this card as the played Digimon's bottom digivolution card."
- **DCGO (BT25_039.cs):** `EffectTiming.OnEndTurn` ActivateClass gated by `IsExistInSecurity(card, false)` (this card is face-up/in the security stack) `&& IsOwnerTurn`. Body: `SelectHandEffect` Mode.PlayForCost over `EqualsCardName("Ceresmon") && CanPlayAsNewPermanent(fixedCost: cost-7)` with `SetReducedCostTuple((7, null))`; then a Yes/No prompt to `AddDigivolutionCardsBottom([this])` onto the played Ceresmon and `ReduceSecurity()` (move self out of security under the new Digimon).
- **What the DSL has:** security-scope clauses exist (`scope: security`), `end_of_your_turn` timing exists, and `play_from_hand` with cost reduction exists. What is missing: (a) a security-zone EOT trigger keyed on **this card living in the security stack** (not a face-up battle-area permanent), and (b) a "place THIS security card as the just-played permanent's bottom digivolution source" movement that consumes the play result binding.
- **Lowers to engine API:** play-from-hand-reduced and add-to-digivolution-bottom both have engine primitives; the gap is the security-resident self trigger at EOT plus binding the freshly-played permanent and moving this security card under it.
- **Suggested DSL syntax:**
  ```yaml
  - scope: security
    when: end_of_your_turn
    optional: true
    process:
      - play_from_hand:
          of: you
          filter: { name_contains: "Ceresmon" }
          cost_delta: { reduce: 7 }
          bind_played_as: ceres
      - if: { binding_present: ceres }
        then:
          - place_self_as_bottom_source: { of_permanent: ceres }   # move this security card under the played Digimon
  ```
- **Verdict:** contributes to BT25-039 BLOCKED (gap_kind: dsl).

## G-DSL-BEATBREAK-ARTS-OPTION — no dual Digimon+Option (BEATBREAK / Arts Digivolve) identity — RESOLVED 2026-06-15

> **RESOLVED 2026-06-15 (`gap/dual-per-face-arts`).** Folded into the
> per-face-effects + Arts-digivolve close (see the BT25-057 entry above).
> A BEATBREAK card is authored as `kind: dual` with the Digimon clauses on
> `dual.digimon.effects` and the Option `[Main]` body on `dual.option.effects`
> (`when: main` → `OptionMain`); `dual.option.arts_digivolve: true` arms the
> engine arts-digivolve selection. The old "Option side OMITTED per the BT25-041
> precedent" workaround is retired. BT25-043 Habakirimon is upgraded from
> PARTIAL to IMPLEMENTED (Option side ships): `[Main]` -8000 single target →
> by-trashing-top-security (player Yes/No) → all opp -5000 for the turn, plus
> Arts Digivolve. NOTE: BT25-041 Murasamemon remains Digimon-side-only for an
> UNRELATED reason (its [WD/WA] pay-one-of-two-costs → cost-reduced play/use is
> a different open gap, G-COST-REDUCTION-INTERACTIVE-PAY-COST); its Option side
> (if any) can now be authored with this substrate.
> Tests: `tests/cards_behavioral/bt25/bt25_043.rs` (11, green).

### Original entry (history)
- **Discovered by:** BT25-043 Habakirimon (aegiomon-2 slice), 2026-06-06. (Same family blocks the Option side of every BEATBREAK card; cf. BT25-041 Murasamemon, which shipped Digimon-side-only.)
- **Clause (Option side):** "Use Req: [Glowing Dawn] trait. [Main] 1 of your opponent's Digimon gets -8000 DP for the turn. Then, by trashing your top security card, all of your opponent's Digimon get -5000 DP for the turn. Arts Digivolve."
- **DCGO (BT25_043.cs):** the card is BOTH a Digimon and an Option — `EffectTiming.OptionSkill` (the [Main] play body) plus `CardEffectFactory.ArtsDigivolveEffect` and `UseRequirements`. A BEATBREAK card can be played as a Digimon OR used as an Option (Arts Digivolve).
- **What the DSL has:** `kind: digimon` and `kind: option` are mutually exclusive top-level kinds; there is no `arts_digivolve` alt-path kind (`CompiledAltPathKind` has Digivolve/DnaDigivolve/DigiXros/BurstDigivolve/Assembly/ActivatedDigivolve/BlastDnaDigivolve — no Arts) and no way to attach an Option `[Main]` clause to a `kind: digimon` card.
- **Lowers to engine API:** would need an engine notion of a card with two play identities (Digimon stat-line + Option [Main]/Arts-Digivolve), surfaced to the action space as two distinct play actions.
- **Suggested DSL syntax:** a `kind: beatbreak` (or `also_option:` block on a Digimon) carrying the Option [Main] `process:` and an `arts_digivolve:` alt-path.
- **Verdict:** contributes to BT25-043 PARTIAL (gap_kind: dsl). Digimon-side clauses (Recovery+unsuspend, Glowing-Dawn leave-prevention) ship; the Option side is omitted (per the BT25-041 precedent).

## G-DSL-PLAYER-CANNOT-SUSPEND-FILTER — player-level CannotSuspend/effect-immunity with a dynamic permanent filter
- **Discovered by:** BT25-028 Dianamon and BT25-059 Ceresmon (aegiomon-2 slice), 2026-06-06.
- **Clause (Dianamon):** "None of your opponent's Digimon with 1 or fewer digivolution cards can suspend until their turn ends." **(Ceresmon):** "none of your suspended [Vegetation] or [TS] trait Digimon are affected by your opponent's Digimon effects until their turn ends."
- **DCGO:** installs a player-level `CanNotSuspendClass` / `CanNotAffectedClass` carrying a `PermanentCondition` that is **re-evaluated on each suspend attempt** (Dianamon: `DigivolutionCards.Count <= 1`; Ceresmon: own suspended Veg/TS). So a Digimon that becomes eligible LATER this turn is also covered.
- **What the DSL has:** `add_player_modifier` (`AddPlayerModifierArgs`) installs a blanket player modifier with NO permanent filter; per-target `add_modifier`/`grant_effect_immunity` only apply to a specific bound permanent.
- **Current modelling:** a `for_each` over the currently-matching set applying the per-target modifier at install time (a snapshot). Practical per-turn outcome matches in the common case; the dynamic re-check nuance (a permanent that enters the matching set later in the turn) is lost.
- **Suggested DSL syntax:** `add_player_modifier:` with an optional `permanent_filter:` predicate that the engine re-evaluates per suspend/effect-application.
- **Verdict:** modelled as a documented snapshot; both cards ship IMPLEMENTED with this nuance noted.

## G-DSL-BOARD-LEVEL-SUM — no board-wide level/stat sum predicate
- **Discovered by:** BT25-077 Bacchusmon (aegiomon-2 slice), 2026-06-06.
- **Clause:** "When this card would be played, if there are 12 or more levels' total worth of Digimon, reduce the cost by 5."
- **DCGO (BT25_077.cs):** sums `permanent.Level` across ALL battle-area Digimon of BOTH players and checks `>= 12`.
- **What the DSL has:** `count_gte` (counts permanents), `card_count_in_zone` (counts cards in a zone), per-permanent `level_*` predicates, and `source_stack_dp_sum` (one permanent's stack) — but NO aggregate that sums a stat (level / DP) across a player/board set.
- **Suggested DSL syntax:** a `board_level_sum_gte` / `stat_sum` predicate (e.g. `{ stat: level, scope: any, zone: battle_area, kind: digimon } >= N`).
- **Verdict:** contributes to BT25-077 PARTIAL (gap_kind: dsl). The cost-reduction clause is omitted (rather than approximated); the two main clauses ship.

## G-DSL-SELF-COLOR-COUNT-LTE — no "distinct colors <= N" / "without N colors" base filter
- **Discovered by:** BT25-084 Titamon (aegiomon-2 slice), 2026-06-06.
- **Clause (alt-digivolve box):** "[Digivolve] [Titamon] w/o 3 colors: Cost 2."
- **DCGO (BT25_084.cs):** `AddSelfDigivolutionRequirementStaticEffect(permanentCondition: TopCard.EqualsCardName("Titamon") && TopCard.CardColors.Distinct().Count() != 3, cost 2)`.
- **What the DSL has:** `self_color_count_gte` (>= only). There is no `self_color_count_lte` / `!= N` for the base-card `from:` filter.
- **Suggested DSL syntax:** `self_color_count_lte: N` (and/or `self_color_count_eq`) usable inside an alt-path `from:` predicate.
- **Verdict:** contributes to BT25-084 PARTIAL (gap_kind: dsl). The standard Lv.5 Purple and Lv.5 [TS] cost-4 alt-paths ship; the Titamon-3-color cost-2 path is omitted.

## DSL Vocabulary ADDED: DigiLink Shape-B (Appmon Link Digimon)  [G-DSL-DIGILINK] — LANDED 2026-06-06
- **Status:** LANDED 2026-06-06 (OpenSpec `implement-digilink-mechanic` §7). New YAML vocabulary for authoring Shape-B Appmon Link *Digimon* (the `[Link]` keyword on `kind: digimon` cards, e.g. BT21-009 Gatchmon) — distinct from the existing Option-scoped `kind: link_requirement` (Plug-Ins).
- **Scope:** DSL.
- **Added vocabulary:**
  - `kind: link_condition` (declarative, body `{ cost, filter }`) — a Digimon's static self link-condition. Lowers to `Effect::link_condition(card).link_host(cost, filter)` at `EffectTiming::LinkCondition`, read by `Game::digimon_link_condition_targets`. Reuses `LinkRequirementBody`.
  - `when: when_linked` (timing) — "when this Digimon gets linked". Lowers to `EffectTiming::OnLink` + forced `.linked()` + an injected self-filter (`event_card == source_card`) so it fires once for the just-linked card, not on sibling links (design D6). Use on a `scope: linked` effect.
  - `scope: linked` + `kind: grant_keyword` (and DP grants) — a linked card's Link-ESS now sets `.linked()` (previously only `scope: inherited` set `.inherited()`), so the grant materializes onto the HOST via the `tick_declarative_effects` linked-card pass (mirrors DCGO `RaidSelfEffect(isLinkedEffect: true)`).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- dsl_digimon_link_card_full_flow` (authors a full Appmon Link Digimon in YAML — link_condition + when_linked draw + linked Raid ESS — and exercises the real link-activate → absorb → OnLink path).
- **Residual:** from-hand Digimon-link initiation + rarer source origins (trash / under-stack / re-link) are not yet wired (engine-side); see `docs/RUST_ENGINE_GAPS.md` 2026-06-06 Shape-B note. Authoring the *named* acceptance cards (BT21-009 Gatchmon etc., with their alt-digivolve / specific WhenLinked bodies) is the §2 follow-up.

## BT25 "link-ts" slice — BLOCKED card (2026-06-07)

Re-run of the BT25 link-ts slice (BT25-069, BT25-066, BT25-075, BT25-101, BT25-102, BT25-089)
against the post–DigiLink-Shape-B substrate (commit 5514135c, 2026-06-07). Shape-B added the
player-activated link of a *standing Digimon onto a host* (root `None`) plus the `kind: link_condition`
/ `when: when_linked` / `scope: linked` authoring layer — but it did **NOT** add a verb/primitive for
"link a card **chosen from trash / hand / digivolution-cards** to one of your Digimon as an effect"
(the deferred residual at `docs/RUST_ENGINE_GAPS.md` §"[Link] subsystem", Shape-B note line ~585:
"from-hand Digimon-link initiation and the rarer source origins (trash / under-stack / re-link) are
not yet wired"). All six slice cards remain BLOCKED on that same residual. Five were already tracked
(BT25-069/066/101 here; BT25-102 in engine-gaps; BT25-089 in RUST_ENGINE_GAPS); BT25-075 is added below.

### BT25-075 Vulcanusmon — link up-to-2 chosen cards from {hand|trash} + per-link <De-Digivolve 1> + aura <Link +1>  [gap_kind: hybrid]
- **Card(s):** BT25-075 Vulcanusmon (Lv.6 Black, Undead/Titan/TS).
- **Effect text:**
  - `When this card would be played, if you have fewer Digimon than your opponent, reduce the cost by 5.` (expressible — fewer-own-Digimon cost reducer.)
  - `[On Play] [When Digivolving] You may link up to 2 cards from your hand or trash to any of your Digimon without paying the cost. Then, for each of your link cards, <De-Digivolve 1> all of your opponent's Digimon.` — **BLOCKED.** Linking up-to-2 cards **chosen from hand or trash** (DCGO `BT25_075.cs`: per-card `SetIntSelection` "from Hand / from Trash / Do not link" → `ILinkCard`) is exactly the deferred "link a chosen card from hand/trash" primitive (cross-ref BT25-069/072/101/056/089 and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem facet #9). The `link_to_own_digimon` DSL verb links only the carrier Option; there is no verb to attach an arbitrary hand/trash card as a link to *any* of your Digimon. The follow-up `<De-Digivolve 1> all opp Digimon for each of your link cards` is expressible in isolation but can only fire after the (unauthorable) link step, so it cannot ship.
  - `[All Turns] All of your [TS] trait Digimon gain <Rush> and <Link +1>.` — Rush grant is expressible (`grant_keyword: Rush` aura); the **`<Link +1>`** grant is **BLOCKED** by the same engine gap as BT25-102 (`G-ENGINE-AURA-GRANT-LINK-MAX` in `qa/archetype-qa/engine-gaps.md` — auras apply `ModifierType::ChangeLinkMax` with a hardcoded value of 0, so a +1 max-link grant is unauthorable without an approximation).
  - `[Your Turn] When your Digimon get linked, one of them may attack.` — depends on a `WhenLinked` host trigger over *any* of your Digimon getting linked (not the self-link of `when: when_linked`, which is self-filtered to the just-linked card). Even were that authorable, it is downstream of the blocked link step.
- **What's missing (two facets):**
  1. **Link N chosen cards from hand/trash to any own Digimon (free).** Engine has the link substrate (`Permanent.linked_cards`, `attach_linked_card`) but no effect-driven "pick a card from hand/trash and attach it as a link to a chosen Digimon" path; the DSL has no verb. (Shape-B only absorbs a *standing Digimon* onto a host.)
  2. **Aura-granted `<Link +1>` carrying a non-zero value** — `G-ENGINE-AURA-GRANT-LINK-MAX` (engine; see engine-gaps.md).
- **Lowers to engine API:** facet 1 → a new effect-link-chosen-card primitive over `Permanent.linked_cards`; facet 2 → `ModifierType::ChangeLinkMax(+1)` via an aura modifier that can carry a value.
- **Verdict:** BLOCKED (hybrid). Ships no YAML — every active clause depends on the chosen-card link primitive and/or the valued aura-Link+1 grant. Cross-ref BT25-069/072/101/056/089, BT25-102 (`G-ENGINE-AURA-GRANT-LINK-MAX`), and `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem.

## DSL Vocabulary ADDED: host-side `[When Linked]` timing  [G-DSL-WHEN-LINKED-HOST] — LANDED 2026-06-07
- **Status:** LANDED 2026-06-07. New timing `when: when_card_linked_to_this` — the host-POV "[When Linked] when a card gets linked **to this Digimon**" (DCGO `CardEffectCommons.CanTriggerWhenLinked`). Lives on a face-up `scope` effect on the host. Lowers to `EffectTiming::OnLink` + a host self-filter (`event_permanent() == source_permanent`) so it fires once for the receiving host only, not for a sibling host. Distinct from the card-POV `when: when_linked` (`event_card == source_card`).
- **Scope:** DSL. Plumbing: `Timing::WhenCardLinkedToThis` (clause.rs) → `CompiledTiming::WhenCardLinkedToThis` (compiled.rs / compile.rs) → `timing_map.rs` (→ `OnLink`) + `lower_triggered.rs` (`is_host_linked` forces the host self-filter).
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- dsl_host_side_when_card_linked_to_this_fires_on_attach host_side_when_linked_fires_for_receiving_host_only`.

## RETIRED → folded into `link_cards`: `link_card_to_self` step (facet #9 authoring verb)  [G-DSL-LINK-CARD-FROM-ZONE]
- **RETIRED 2026-06-19 (collapse-dsl-step-idioms §4).** `link_card_to_self` is DELETED — the `StepSpec`/`CompiledStep::LinkCardToSelf` variants, `LinkCardToSelfArgs`/`LinkFromZone`/`LinkToHost` types, the `compile.rs` arm, the `lower_triggered` outer-optional arm, and the whole `src/dsl_cards/step/link_card.rs` lowering are gone. All 11 users (ST22-12, BT21-023/073/101, BT25-052/056/060/069/070/072/089) migrated to the more general `link_cards { from, filter, to: self|own_digimon, count: { up_to: 1 }, cost: free }` (zone-name map `digivolution_sources → self_sources`, `chosen_own_digimon → own_digimon`). This was also a **faithfulness improvement**: `link_card_to_self` presented a single union-of-zones selection, whereas `link_cards` (and DCGO's actual `SetBoolSelection`) present a zone-choice-first flow. BT25-060's "By linking 1 …, 1 of your Digimon may unsuspend" `if (linked)` gate is now modeled by the new `link_cards` **`bind_as`** field (captures the linked card only on a real link) + `if { binding_present }`. The dropped `link_cost_delta_for_player` application was a no-op for every real user (all `cost: 0`). 121 behavioral tests across the 11 cards stay green. Historical detail below.
- **Status (historical):** LANDED 2026-06-07. DSL step `link_card_to_self` authored: `{ from: [hand|trash|digivolution_sources], filter: PredicateSpec, to: self|chosen_own_digimon (default self), cost: u16 (default 0), optional: bool }`. Lowering in `code/digimon-engine/src/dsl_cards/step/link_card.rs` gathers candidates across the requested zones into ONE RL-visible `SelectionKind::Target` prompt (no auto-pick — disjoint per-zone action ranges so the union is unambiguous), and on resolution computes effective cost (`cost + link_cost_delta_for_player`).max(0), pays it, and calls the primitive `Game::link_chosen_card_into_host(host, chosen, source_zone)`. With `to: chosen_own_digimon` a SECOND RL-visible selection over the controller's standing Digimon picks the host ("link to 1 of your Digimon" — e.g. BT25-069/089). DSL surface: `StepSpec::LinkCardToSelf` / `LinkCardToSelfArgs` / `LinkFromZone` / `LinkToHost` in `code/digimon-dsl/src/step.rs`; `CompiledStep::LinkCardToSelf` in `compiled.rs`; lowering in `compile.rs`. **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- link_card_to_self_links_chosen_hand_card_pays_cost_and_fires_onlink link_card_to_self_applies_change_link_cost_reduction` (both green; first pins selection + cost + OnLink propagation via a host-side `when_card_linked_to_this` reaction, second pins the `ChangeLinkCost` reduction path). Chosen-host path pinned by `bt25_069_on_play_links_ts_from_trash_to_chosen_own_digimon` + `bt25_089_main_links_appmon_from_hand_to_chosen_digimon`. Cards authored on it: BT25-052/056/070/072 (self-host), BT25-069/089 (chosen-host). Pairs with facet #10's flat `ChangeLinkCost`.
- **Cost-modeling note:** the printed "with the cost reduced by N" is a reduction on the LINKED card's own link cost. DSL card fixtures carry no engine-side link cost on the linked candidate, so cards author `cost: 0` (the reduction makes the typical 1–2 link free in practice); the flat `ChangeLinkCost` path covers any nonzero residual. Faithful for the no-engine-link-cost fixtures; revisit if a linked candidate carries a nonzero engine link cost.
- **Residual (NOT yet authored — separate gaps):**
  - `G-DSL-LINK-N-CARDS-PER-HOST` (BT25-075): "link up to 2 cards from hand/trash, each to a *separately chosen* Digimon" — the single-card step does not loop with per-card host selection. Needs a `count: N` extension that repeats the (card → host) pair selection. **NOT done — BT25-075 left BLOCKED.**
  - `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` (BT25-089 [Main]): "from your *Digimon's* digivolution cards" scans EVERY own Digimon's under-sources; the step's `digivolution_sources` zone anchors only to the effect's own permanent. BT25-089 authored the hand source (PARTIAL — that source clause omitted).
- **Superseded OPEN notes below (historical):**
- **gap_kind:** dsl.
- **What it authors:** "[…] you may link 1 [card matching FILTER] from your hand / this Digimon's digivolution cards / your trash to this Digimon […]" (DCGO `ILinkCard.LinkCard` with `root != None` → `Permanent.AddLinkCard`). Representative card: **ST22-12** ("link 1 Digimon card with [Social]/[Navi]/[Tool] from your hand or this Digimon's digivolution cards to this Digimon with the link cost reduced by 2").
- **Engine primitive (ready, exposed):** `EffectContext::link_chosen_card_into_host(host, card, LinkCardSource)` → `Game::link_chosen_card_into_host` — lifts the chosen card out of `LinkCardSource::{Hand|Trash|DigivolutionSource}`, attaches onto the host's `linked_cards`, fires `OnLink`. Tested: `facet9_link_chosen_card_from_hand_attaches_and_fires_onlink`, `facet9_link_chosen_card_from_digivolution_sources`.
- **Remaining DSL surface:** a `StepSpec` (e.g. `link_card_to_self: { from: [hand, digivolution_sources, trash], filter: PredicateSpec, cost: N, optional: bool }`) that (1) gathers candidate cards across the chosen zones, (2) installs the zone→card selection, (3) on resolution computes effective cost (`base − link_cost_delta_for_player`), pays it, and calls the primitive with the host = the effect's own permanent. Distinct from the existing `link_to_own_digimon` step (Plug-In Option self-link, host-selection, tied to `pending_option`). Pairs with facet #10's flat `ChangeLinkCost` for the "link cost reduced by N" clause.

## DEFERRED (no card needs it): predicated `ChangeLinkCost` reduction  [G-DSL-LINK-COST-PREDICATED]
- **Status:** DEFERRED-until-needed (not a blocking gap). The **flat** player-/permanent-scoped `ModifierType::ChangeLinkCost` (DSL-authorable; summed by `link_cost_delta_for_player`; consulted at all three link-cost sites) covers every real cost-reducer — DCGO's `GrantedReduceLinkCostClass` is invoked with `_ => true` for all of `cardSourceCondition`/`permanentCondition`/`rootCondition` (see ST22-12). DCGO's general `ChangeLinkCostClass` supports per-(source/host/root) predicates, but no printed card exercises them, so building predicated reduction now would be speculative machinery. File a concrete card here if one is found. Confirming test for the flat path: `facet10_change_link_cost_reduces_paid_link_cost`.

## DSL Gap: BT25-075 — formula source "number of your own link cards" (total across all your Digimon)  [G-DSL-FORMULA-OWN-LINK-CARD-COUNT]
- **Discovered by:** BT25-075 Vulcanusmon (link-finish-aura slice), 2026-06-07.
- **Effect text:** "[On Play] [When Digivolving] You may link up to 2 cards from your hand or trash to any of your Digimon without paying the cost. **Then, for each of your link cards, ＜De-Digivolve 1＞ all of your opponent's Digimon.**"
- **DCGO (BT25_075.cs):** `int degenerationCount = card.Owner.GetBattleAreaDigimons().Map(p => p.LinkedCards).Flat().Count();` then loops `IMassDegeneration(enemy Digimon, 1)` that many times — i.e. De-Digivolve 1 applied to **all** opponent Digimon, repeated N times where N = the total count of link cards across every one of the controller's battle-area Digimon (counted *after* the link step above resolves).
- **What's already expressible (today):** the link half ships via the new `link_cards` step (`from: [hand, trash]`, `to: own_digimon`, `count: { up_to: 2 }`, `cost: free`) — that step was authored partly for this card (its doc names BT25-075). The `<Link +1>`/`<Rush>` `[All Turns]` aura ships via aura `modifier: ChangeLinkMax` + `modifier_value: 1` and `grant_keyword: Rush` (G-ENGINE-AURA-GRANT-LINK-MAX resolved 2026-06-07). The `de_digivolve` step exists with `amount` / `amount_fn` (FormulaSpec) and can target all opp Digimon.
- **What's missing:** a **`FormulaSpec` / `PerSelector` source that counts own link cards**. `code/digimon-dsl/src/formula.rs` `PerSelector` has `MaterialCount`, `SuspendedCount`, `AllyCount`, `CardCountInZone`, etc., but nothing that sums `permanent.linked_cards.len()` across the controller's battle-area Digimon. Without it, `de_digivolve: { target: all_opp_digimon, amount_fn: <own-link-card-count> }` cannot be authored, and the De-Digivolve clause's magnitude (a player-visible board swing) cannot be modeled → no-approximations violation, whole card BLOCKED.
- **Lowers to engine API:** the substrate exists — `Permanent.linked_cards` is populated and counted at multiple sites (e.g. `game_actions.rs:1494`, `tensor_v1.rs:267`). The missing piece is purely a DSL formula selector + its evaluator reading `ctx`'s controller battle-area Digimon and summing `linked_cards.len()`.
- **Suggested DSL syntax:** a `FormulaSpec` variant `{ own_link_card_count: { of: you } }` (or a `PerSelector::OwnLinkCardCount { of }` usable in `base_per_delta`), evaluating to `Σ over of.battle_area Digimon of permanent.linked_cards.len()`. Used as `de_digivolve: { target: <all opp digimon>, amount_fn: { own_link_card_count: { of: you } } }` — but note DCGO applies De-Digivolve-1 N *separate* times to the whole opp board, not De-Digivolve-N once; the lowering must repeat the mass De-Digivolve-1 N times (or `amount: 1` with an outer `repeat: <formula>`), matching `IMassDegeneration(..., 1)` × N. A `repeat_n: <FormulaSpec>` wrapper around a step would also close this.

## LM-020 — return a selected SECURITY card to a deck  [G-DSL-RETURN-SELECTED-SECURITY-TO-DECK]

**CLOSED 2026-06-05.** Added the `return_selected_security_to_deck` DSL verb
(`ReturnToDeckArgs`: of/card/position) + the engine primitive
`EffectContext::return_security_card_to_deck(player, card, to_bottom)` and a new
`SecurityRemovalDestination::Deck { owner, to_bottom }` handled in
`complete_effect_security_removal` (Digi-Eggs route to the digitama deck; fires the
OnLoseSecurity / OnOpponentSecurityRemoved observer chain). LM-020 Quantumon is now
fully authored (`code/digimon-engine/cards/lm/LM-020.yaml`, both clauses) and
judge-quiz **Q18 → PASS**. A second small gap surfaced while authoring clause 2 —
no predicate compared a *bound card's* category to a declared one — closed by the
new `binding_card_kind: { binding, kind }` predicate. Tests:
`tests/effect_context/security_stack_operations.rs` (3) +
`tests/cards_behavioral/lm/lm_020.rs` (4) + judge-quiz Q18.

Surfaced: 2026-05-29, judge-quiz first wave (`batch-implement-cards-rust-dsl`). LM-020 Quantumon BLOCKED.

- **Missing DSL verb:** `return_selected_security_to_deck` — route a `select_security`-bound `CardHandle` to the owner's deck **top or bottom**. The three verbs that consume a `select_security` pick route it to hand (`add_to_hand_from_security`), play (`play_security_card`), or trash (`trash_selected_security`) — never to a deck.
  - Suggested YAML (mirrors `return_to_deck_from_reveal { of, card, position }`):
    ```yaml
    - return_selected_security_to_deck: { of: opponent, card: picked_sec, position: top }   # top | bottom
    ```
- **Engine prerequisite (root cause — also logged in `docs/RUST_ENGINE_GAPS.md`):** no public `EffectContext` method moves a security card to a deck. The private `move_card_to_deck` helper (`effect_context/mod.rs`) is sourced from trash only. Suggested `pub fn return_security_card_to_deck(&mut self, player, card, to_bottom) -> bool`: find the card in `player.security`, `ensure_security_materialized`, remove it, drop from `face_up_security`, fire `fire_security_removed_observers` (add a `SecurityRemovalDestination::Deck` variant alongside `::Hand`), then route through the existing trash->deck `move_card_to_deck` path. Lower the new verb in `dsl_cards/step/zone_moves.rs` alongside `AddToHandFromSecurity` / `TrashSelectedSecurity`.
- **Card text:** LM-020 [When Digivolving] "... reveal all of your opponent's security cards, and place 1 card among them on top of your opponent's deck. Shuffle the rest and return them to the security stack." DCGO `LM_020.cs`: `IReduceSecurity` -> `AddLibraryTopCards` -> shuffle.
- **Blocks:** LM-020 (Quantumon) -> judge-quiz Q18. (LM-020's `[Start of Opponent's Turn]` category-immunity clause is independently implementable; only the security->deck clause is blocked.) Likely shared by other "place a security card on top/bottom of deck" cards. Re-attempt LM-020 once the verb lands.

## BT13-088 — place a card as the TOP digivolution source  [G-DSL-PLACE-AS-TOP-SOURCE]

Surfaced: 2026-05-29, judge-quiz first wave. BT13-088 Belphemon: Sleep Mode shipped PARTIAL.

- **Missing DSL verb / engine primitive:** a "place card as the TOP digivolution source" (just below the face card) — DCGO `AddDigivolutionCardsTop`. The engine ships `place_as_bottom_source` (inserts at index 0) only; no top-source insertion.
- **Resolution used:** BT13-088 uses `place_as_bottom_source` for "place [Belphemon: Rage Mode] on top of this Digimon's digivolution cards." Position is **behaviorally inert** for this card (it only needs Rage Mode IN the stack to gain the inherited effect; no mechanic reads the top-source slot) -> shipped PARTIAL, not BLOCKED. A future card whose text/behavior depends on the top-source position would need this verb (+ an `EffectContext::place_as_top_source` primitive).

## EX5-060 — opponent plays from their OWN trash SUSPENDED + played-permanent-level formula  [G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED] / [G-EVENT-PLAYED-LEVEL-FORMULA] — **RESOLVED 2026-06-11 (judge-quiz Q28)**

> **RESOLVED 2026-06-11.** All pieces landed with the Q28 slice: (1)
> `play_from_trash_free` now plays for the BINDING OWNER's side (the trash
> owner) — `play_from_trash_free_unsuspended_for(controller, …)`; (2) new
> `suspended: true` arg (G-PLAY-ENTERS-SUSPENDED — the permanent ENTERS the
> battle area suspended, before play-event observers, via
> `Game::play_enters_suspended` consumed at the single commit site); (3) new
> `event_target_level: {}` FormulaSpec leaf (reads the trigger's event card's
> level — DCGO `LevelJustAfterPlayed`) usable inside `level_lte: { formula: … }`;
> (4) the `suppress_on_play` rider is consult-gated on
> `permanent_is_unaffected_by_effect` vs the recorded suppressor identity
> (`Game::on_play_suppressor`) — a protected played Digimon still fires its
> [On Play] (the Q28 ruling). The `event_played_by_effect` predicate from the
> original sketch was NOT needed — the existing `event_is_effect_initiated`
> leaf covers it (the suspend-bit work threaded `effect_initiated` through
> `TriggerSource::EnteredField`). EX5-060 Dragomon IMPLEMENTED; pins:
> `cards_behavioral/ex5/ex5_060.rs` (5) +
> `judge_quiz a::q28_*` (pin + control). RELATED: BT20-059's board-wide
> protection re-authored as the CONTINUOUS `grant_effect_immunity` form
> (`continuous: true` + `targets:` → floating mass modifier carrying an
> `EffectImmunityFilter` payload), closing
> G-DSL-CONTINUOUS-CONTROLLED-IMMUNITY-AURA — the per-tick re-scan covers
> permanents played later in the window (the judge's "persistent effect").

### Original entry (history)

Surfaced: 2026-05-29, judge-quiz wave (`batch-implement-cards-rust-dsl`). EX5-060 Dragomon BLOCKED (pins Q28 alongside BT20-059 Gankoomon X).

- **Card text:**
  - Clause 1 [On Play][When Digivolving]: "Your opponent plays 1 level 4 or lower Digimon card from their trash **suspended** without paying the cost. [On Play] effects on Digimon played by this effect don't activate."
  - Clause 2 [All Turns][Once Per Turn]: "When an effect plays an opponent's Digimon, you may play 1 purple Digimon card with **a level less than or equal to it** from your trash without paying the cost."

- **Clause 1 — DSL gap (root cause is the engine gap `G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED` in `docs/RUST_ENGINE_GAPS.md`):** `play_from_trash_free` cannot (a) play from the **opponent's** trash — its `of:` field is dropped at the engine boundary (lowers to `play_from_trash_free_unsuspended*`, which hardcodes `self.player`) — or (b) play **suspended** (no `suspended:` flag anywhere in the play-from-trash chain). The `[On Play] don't activate` half already works via `suppress_on_play: true`. Intended shape once both land:
  ```yaml
  - as_selecting_player:
      of: opponent
      body:
        - select_trash: { of: opponent, bind_as: opp_pick, filter: { kind: digimon, level_lte: 4 }, prompt: "..." }
        - play_from_trash_free: { of: opponent, hand_index: opp_pick, suspended: true, suppress_on_play: true }  # of:opponent + suspended NEW
  ```

- **Clause 2 — DSL gaps:**
  - **`event_played_by_effect` predicate** — `on_any_digimon_played` cannot distinguish a normal hard-play from an effect-play. DCGO `EX5_060.cs` gates Clause 3 on `IsByEffect`. No `by_effect`/`event_played_by_effect` predicate leaf exists in `predicate.rs`.
  - **`event_target_level` FORMULA** — "a level less than or equal to **it**" bounds the own-trash recursion filter by the *played opponent permanent's level*. Only the predicate leaves `event_target_level_lte/_eq/_gte` exist (compare against a literal); there is no `FormulaSpec::EventTargetLevel` to feed `level_lte: { formula: ... }`. DCGO reads `permanent.LevelJustAfterPlayed`. The trigger timing + `event_target_owner: opponent` + `event_target_kind: digimon` predicates DO exist.
  Intended shape once both land:
  ```yaml
  - when: on_any_digimon_played
    active_when: { all_turns: true }
    once_per_turn: true
    optional: true
    condition: { all_of: [ { event_target_owner: opponent }, { event_target_kind: digimon }, { event_played_by_effect: true } ] }  # by_effect NEW
    process:
      - select_trash: { of: you, bind_as: recur, optional: true, filter: { all_of: [ { kind: digimon }, { color_is: purple }, { level_lte: { formula: { event_target_level: {} } } } ] }, prompt: "..." }  # event_target_level formula NEW
      - play_from_trash_free: { of: you, hand_index: recur }
  ```

- **Blocks:** EX5-060 (judge-quiz Q28). `code/digimon-engine/cards/ex5/EX5-060.yaml` Clauses 1 & 2 declared with faithful timing / OPT / optional flags but empty (gap-blocked) `process` bodies — never resolve a wrong approximation. Inherited ＜Piercing＞ is fully supported and authored live. Tests in `code/digimon-engine/tests/cards_behavioral/ex5/ex5_060.rs`: `ex5_060_clause1_*` `#[ignore]`'d with `G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED`; `ex5_060_clause2_*` `#[ignore]`'d with `G-EVENT-PLAYED-LEVEL-FORMULA`. The Q28 negative (`ex5_060_lock_does_not_attach_to_effect_immune_target`) runs LIVE.


## ST17-07 — opponent-scoped effect-protection from `add_modifier`  [G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL]

**Surfaced 2026-05-30** (judge-quiz cluster B, ST17-07 Rapidmon). PARTIAL: the
green-Tamer rider "until end of opp turn, your OPPONENT'S effects can't delete
this Digimon or return it to hand/deck" is omitted.

- **Problem.** The DSL `add_modifier` step lowers through
  `EffectContext::add_modifier` → `ModifierEntry::simple`, whose `cause_filter`
  is `None` — the replacement fire-site treats `None` as CAUSE-AGNOSTIC, so
  `add_modifier { CannotBeDestroyedByEffect | CannotBeReturnedToHand |
  CannotBeReturnedToDeck }` blocks the controller's OWN effects too. DCGO scopes
  all three protections to `IsOpponentEffect`. `default_passive_cause_filter`
  (which would scope the Return ones to OpponentEffect) is consulted ONLY by
  `ModifierEntry::passive_replacement`, never by `ctx.add_modifier`.
- **Latent class.** Existing cards using `add_modifier` for these protections
  (BT18-064, P-215, EX8-070) silently ship cause-agnostic and only assert the
  modifier is *present* (never own-vs-opponent scope), so the divergence is
  currently unverified across the codebase — a widening here would correct them.
- **Engine half is mostly present.** `ModifierEntry::opponent_only()`
  (modifiers.rs) forces `cause_filter = Some(OpponentEffect)` and the fire-site
  honors it; the missing piece is exposing an installer that uses
  `passive_replacement(...).opponent_only()` from the DSL.
- **Suggested widening (backward-compatible, opt-in).** Add `opponent_only:
  bool` (default false) to the `add_modifier` DSL step; when true, route the
  install through `passive_replacement(modifier, expiry, player).opponent_only()`
  instead of `ModifierEntry::simple`. Existing cards (flag unset) are unchanged.
  Deferred as a deliberate cross-cutting change (it changes the *meaning* of
  these protections for shipped cards) — should regress BT18-064/P-215/EX8-070.
  Until landed, ST17-07's rider stays omitted (NOT shipped cause-agnostically)
  with `st17_07::st17_07_green_tamer_grants_opponent_only_delete_protection` /
  `..._protection_not_installed_without_green_tamer` `#[ignore]`'d citing this ID.

## DSL Gap: BT3-109 — no "deleted self card in trash" binding for granted [On Deletion] trash-play  [G-DSL-DELETED-SELF-TRASH-BINDING]
- **Status:** CLOSED 2026-06-05. BT3-109 authored (`code/digimon-engine/cards/bt3/BT3-109.yaml`), behavioral test green (`tests/cards_behavioral/bt3/bt3_109.rs`), and judge-quiz **Q21 → PASS**. The premise was partly wrong: the `event_card` / `event_target` bindings ALREADY resolve the just-deleted carrier's top card in trash (`binding_ref.rs` reads `DeletedObjectSnapshot.top_card` for both). The only real missing link was that `play_from_trash_free` accepted a `TrashIndex` binding but not a `Card`-handle binding — so a card-identity binding like `event_card` couldn't feed it. Fixed by making the `PlayFromTrashFree` step arm also accept `ResolvedBinding::Card(h)` (`code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`); the engine call `play_from_trash_free_unsuspended` self-guards that the handle is in the controller's trash. No new `StructuredBindingRef` variant was needed. Composes with the Q19 top-most-card-in-trash gate: replaying the carrier suppresses its remaining [On Deletion] bundle. `suppress_on_play: true` covers "Any [On Play] effects ... don't activate". The "BLOCKED guard test" mentioned below was never committed; the real test is the live behavioral + judge-quiz pin.
- **Status (historical):** OPEN (discovered 2026-06-03, BT3-109 Back for Revenge! DSL implementation — judge-quiz Q21 cluster).
- **Scope:** DSL.
- **Card(s):** BT3-109 Back for Revenge! — `[Main] 1 of your Digimon gains "[On Deletion] Play this card without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate." for the turn.` Generalizes to any granted-or-printed [On Deletion] body that must play "this card" (the just-deleted carrier's own top card, now in the trash) back from trash.
- **Recovered text source:** DCGO `DCGO/Assets/Scripts/CardEffect/BT3/Purple/BT3_109.cs` (cards.json `effect_description_eng` is garbled with doubled/nested quotes). DCGO: OptionSkill selects exactly 1 of your Digimon (mandatory, `canNoSelect: false`); grants it an `OnDestroyedAnyone` ActivateClass with `EffectDuration.UntilEachTurnEnd`; the granted body plays `selectedPermanent.TopCard` from `root: Trash`, `payCost: false`, `activateETB: false`. No level/cost cap.
- **What's missing:** `play_from_trash_free` (and `play_from_trash`) take `hand_index: <BindingRef>` — a binding to a SPECIFIC trash card. "This card" is the carrier's own top card after it moved to trash on deletion, but `StructuredBindingRef` (`code/digimon-dsl/src/step.rs:1040`) exposes only `permanent` / `source_permanent` / `zone` / `of_permanent` / `deck_top` — there is NO binding resolving "the just-deleted self card now in the trash". A generic `select_trash` is not a faithful substitute: "this card" is one specific card, and there is no identity predicate tying a trash card back to the carrier permanent that was just deleted, so a filter-based pick over-exposes every other matching trash card as an illegal choice (no-approximations / rule 17 violation). NOTE: the earlier-suspected second blocker (granted-body selection support) is CLOSED — Phase 4i "Queue-based granted-body dispatch + selection support" parks selection-installing granted bodies via `pending_selection`. The stale "v1 limitation" comment in `code/digimon-engine/src/dsl_cards/step/grant_triggered.rs` predates Phase 4i.
- **Suggested change:** Add a card-identity binding for the deleted-self card in trash usable inside an [On Deletion] body / granted [On Deletion] body (e.g. a `StructuredBindingRef` variant `deleted_self_in_trash` or a `trigger_self_card` binding that resolves the carrier's pre-deletion top card now in the trash), accepted by `play_from_trash` / `play_from_trash_free` `hand_index`. Pairs with the existing `suppress_on_play: true` to express "Any [On Play] effects ... don't activate" faithfully.
- **Workaround:** None faithful. BT3-109 is BLOCKED — left UNIMPLEMENTED (no YAML in the embedded pack) rather than stubbed with an auto-selection or an approximate "play any Digimon from trash" surrogate. A BLOCKED guard test pins the absence in `code/digimon-engine/tests/cards_behavioral/bt3/bt3_109.rs`.

## DSL Gap: BT13-103 — cost_reduction amount driven by an interactive in-cost deletion  [G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT]
- **Status:** OPEN (discovered 2026-06-03, BT13-103 Akihiro Kurata DSL implementation).
- **Scope:** DSL + engine (the engine half of the cost-reduction scan must also change).
- **Card(s):** BT13-103 Akihiro Kurata Clause 1 — `[Your Turn] When you would play a card with [Belphemon] in its name, by deleting 1 of your Digimon with [Gizmon] in its name, reduce the play cost by the play cost of the deleted Digimon.` Generalizes to any BeforePayCost reduction whose **amount is set by a permanent the player interactively selects and deletes/pays during the cost** (the reduction = the deleted/paid permanent's printed cost).
- **Authoritative source:** DCGO `DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_103.cs` (EffectTiming.BeforePayCost). `SelectPermanentEffect` over own non-immune [Gizmon]-name Digimon, `canNoSelect: true` (optional); on a pick, `DeletePeremanentAndProcessAccordingToResult`, then installs a `ChangeCostClass` of `-permanent.CostJustBeforeRemoveField` for the current play. The reduction magnitude is the *selected* Digimon's cost — known only AFTER the in-cost selection. (DCGO also ships an AI-only `EffectTiming.None` mirror that auto-picks `gizmonCosts.Max()` — an approximation we may NOT replicate under rule 17.)
- **What's missing:** the DSL `kind: cost_reduction` clause splits the amount from the cost across two callbacks that cannot share the selection:
  - `amount` / `amount_fn` is evaluated in `cost_reduction_fn` (READ context) by `apply_cost_reduction_candidate` (`code/digimon-engine/src/game_actions.rs:5848`) **before** `pay_cost_fn` runs.
  - `pay_cost_fn` (`code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs:193`) builds a **fresh** `Bindings`, so any permanent selected/bound inside `pay_cost` is invisible to `amount_fn`. No `FormulaSpec` (`code/digimon-dsl/src/formula.rs`) reads "the cost of the permanent paid as the cost" — `BindingPlayCost` reads a *prior* `bind_as` binding only, unreachable from the isolated pay_cost scope.
  - `pay_cost_fn` is additionally gated to `RunOutcome::Synchronous` (`lower_cost_reduction.rs:195`); an interactive `select_own_permanent` parks (non-synchronous) → the cost reads as failed → the reduction is dropped. So even the *selection* cannot surface through `pending_selection`. (See also `game_actions.rs:5678`, which skips paid reducers entirely without a real cost target.)
- **Suggested change:** make the cost-reduction `pay_cost` (i) able to surface an interactive `pending_selection` and resume, and (ii) able to bind the selected/paid permanent into a binding scope that `amount_fn` can read (e.g. a `BindingPlayCost` over a `pay_cost`-produced binding, or a dedicated `paid_cost_total` formula that sums the printed cost of permanents deleted/paid during this pay_cost). Backward-compatible: literal `amount:` and the existing synchronous self-suspend / return-to-deck pay_costs are unchanged.
- **Workaround:** None faithful. BT13-103 Clause 1 is BLOCKED — left UNIMPLEMENTED in `code/digimon-engine/cards/bt13/BT13-103.yaml` (Clauses 2 & 3 ARE authored) rather than approximated. The Clause-1 behavioral test `bt13_103_belphemon_play_cost_reduced_by_deleted_gizmon_cost` in `code/digimon-engine/tests/cards_behavioral/bt13/bt13_103.rs` is `#[ignore]`'d citing this ID.
### RESOLVED 2026-06-02 — `may_attack_now: { windowed: true }` (deferred EOT attack grant)
- **Gap:** `[End of Your Turn] 1 of your Digimon may attack` (AD1-004 WarGreymon)
  was authored with inline `may_attack_now`, which declares AND resolves the
  attack synchronously inside the trigger. That leaves no window for a sibling
  end-of-turn effect (e.g. an inherited DNA digivolve) to resolve first and
  remove the attacker, so the attack could never fizzle — contrary to
  general_rule.pdf §15-4-2-3 (EOT triggers activate one at a time) + the
  "attack ends if the attacker has left" rule.
- **Resolution:** added a `windowed: bool` flag to the `may_attack_now` step
  (`MayAttackNowArgs` → `CompiledStep::MayAttackNow`). When true, the step grants
  the chosen attacker a `MayAttack` (+`CanAttackUnsuspended` iff `without_suspending`)
  modifier with `Expiry::EndOfTurn` — the same windowed mechanism the `<Execute>`
  keyword uses — instead of declaring inline. The attack becomes a deferred
  EOT-action; if a sibling EOT effect removes the attacker, the grant is orphaned
  and no attack happens. AD1-004's YAML now sets `windowed: true`. Default is
  false, so all other `may_attack_now` users (AD1-009, BT12/17/20/…) are unchanged.
- **Tests:** `cards_behavioral/ad1/ad1_004.rs::{ad1_004_eot_attack_is_windowed_grant_not_synchronous,
  ad1_004_eot_attack_fizzles_when_attacker_is_removed_before_it_acts}`.
- **Note:** AD1-004 stays PARTIAL overall — its `[On Play][When Digivolving]`
  "delete opponent Digimon with DP ≤ this" is still blocked on G-FORMULA-SOURCE-DP
  (unrelated to this fix).

## RESOLVED 2026-06-03 — `select_count_capped_multi.clamp_to_available` (mandatory "N of opponent" target count)

**Gap (FAQ MP-30/31, discovered via `tests/rules_faq/effect_resolution.rs`):** the DSL
`select_count_capped_multi` step had no way to express a mandatory **"N of your opponent's
Digimon"** target count. Its `min` field carries *cost* semantics (no-op when fewer than `min`
candidates exist), and with `min` absent the floor defaults to 1 — so a card whose text reads
"Suspend **2**" (BT24-051 Merukimon) let the player stop after suspending **one** when two were
available (violates MP-31), while naïvely setting `min: 2` would fizzle the effect when only one
target is in play (violates MP-30, which requires affecting `min(N, available)`).

**Fix (widened the substrate, per rule 28):** added a `clamp_to_available: bool` field to
`SelectCountCappedArgs` (and the compiled step). When true, the required floor is clamped to
`min(max, available_candidates)` and the step never no-ops for "fewer than N" — the rules-correct
"affect as many as possible, up to N" semantics. Implemented in the battle-area path
(`install_select_count_capped_permanents`, `dsl_cards/step/selections.rs`); orthogonal to the
existing cost `min`. BT24-051 now sets `clamp_to_available: true`. Hand/Trash zones thread the flag
but do not yet act on it (no card needs it; future extension).

Files: `code/digimon-dsl/src/{step,compiled,compile}.rs`,
`code/digimon-engine/src/dsl_cards/step/selections.rs`,
`code/digimon-engine/cards/bt24/BT24-051.yaml`. Tests: `mp30_*` / `mp31_*` in `rules_faq`.

**DCGO-verified (2026-06-03).** The fix matches the battle-tested DCGO behavior exactly:
`BT24_051.cs` uses `maxCount = Math.Min(2, available)` + `canEndNotMax: false` (must pick all
`min(2, available)`) — which is precisely `clamp_to_available: true`. DCGO's `canEndNotMax` is the
general distinguisher: `false` ⇒ `clamp_to_available: true`; `true` ⇒ the default "up to N"
(`optional_zero`). **Sibling sweep (implemented pool, mandatory "≥2 of your opponent's"):** only
BT24-051 was affected. ST5-15 Laser Eye is genuinely "up to 2" (`ST5_15.cs` `canEndNotMax: true`;
cards.json dropped the "up to") — its existing `optional_zero: true` authoring is correct, NOT a gap.

**Second sibling found + fixed (2026-06-03): BT12-028 Paildramon.** A DCGO-grounded sweep of the
whole implemented pool (DCGO `canEndNotMax:false` + `Math.Min(≥2,...)`, intersected with cards using
`select_count_capped_multi` on `zone: battle_area`) found one more instance of the identical bug:
BT12-028's "[When DNA Digivolving] **2** of your opponent's Digimon with no digivolution cards can't
attack" was authored `max: 2, optional_zero: false` with no clamp — letting the player lock only 1 of
2. Fixed with `clamp_to_available: true` (DCGO `BT12_028.cs` confirms `canEndNotMax: false`). Existing
BT12-028 behavioral tests (9) still pass. **Sweep otherwise clean:** BT24-040 / BT15-101 hand-roll N
mandatory `select_opponent_permanent` calls with `not_in_binding` dedup (faithful); ST5-12/ST5-15/
ST6-12 are genuinely "up to/may" (`optional_zero: true`); formula-`max` cards are "up to <X>".

## RESOLVED 2026-06-03 — `optional: true` on MANDATORY single-target selects (FAQ MP-29)

**Gap class (cousin of the MP-30/31 multi-target bug).** A select step authored `optional: true`
over-exposes an illegal *decline* (PASS) for an effect whose printed text is **mandatory** (no
"may"/"can"/"up to"). DCGO signal: `canEndNotMax: false` / `isOptional = -1`. Found by sweeping
implemented cards for `select_opponent_permanent { optional: true }` whose card text is a mandatory
"Suspend/Delete 1 of your opponent's Digimon", cross-checked against DCGO.

Two instances found + fixed:
- **BT21-037 Lighdramon** — "[When Digivolving] Suspend 1 of your opponent's Digimon. Then +2000 DP."
  The select was `optional: true` (the YAML comment even documented the intended `optional: false`).
  The author used `optional: true` to keep the unconditional DP buff firing when no target exists —
  but `install_select_opponent_permanent` already skips the tail on empty candidates, so that didn't
  even work, and it wrongly exposed a decline with a target present. **Fix:** guard select+suspend
  behind `if any_permanent(opponent, digimon)` (mandatory select only reached when a target exists),
  DP buff unconditional afterward. DCGO `isOptional=false`. Caught by `rules_faq::…::mp29_*`; both the
  with-target suspend test and the no-target DP-buff test stay green.
- **AD1-018 LordKnightmon** — "[Security] <De-Digivolve 1>, then **delete** 1 of your opponent's
  Digimon with play cost 3 or less." The delete select was `optional: true`; DCGO `AD1_018.cs`
  SecuritySkill uses `canEndNotMax: false`. **Fix:** removed `optional: true` (delete is the final
  step, so the empty-candidate no-op loses nothing).

Sweep otherwise clean: these were the only two implemented `select_opponent_permanent {optional:true}`
cards whose text is mandatory single-target.

## Alt-digivolve `from:` requiring ≥N sources carrying a trait  [G-DSL-DIGISOURCE-TRAIT-COUNT-GTE]  — OPEN 2026-06-05

Surfaced by **AD1-002 Aldamon** (judge-quiz Q4 authoring). The alt-digivolve line is
"[Digivolve] [Takuya Kanbara] w/ **2 or more [Hybrid] trait cards under**: Cost 3" — a digivolution
path whose `from:` base must additionally have **≥2 digivolution sources carrying the [Hybrid]
trait** beneath it.

- **What's missing (DSL):** an alt-path `from:` predicate that **counts** sources by trait. The
  closest existing leaf, `self_digivolution_sources_trait_has`, is (a) a ≥1 boolean presence check
  (no count threshold) and (b) a carrier/permanent-subject predicate, not an alt-path `from`-base
  predicate (alt-path `from` constrains the base being digivolved *from*, not the resulting stack's
  source multiset). Neither `materials_count_gte` (whole-stack count, trait-agnostic) nor
  `trait_has`/`trait_contains` (subject-trait match, no count) expresses "≥2 sources with trait T".
- **Impact:** AD1-002's alt-path enforces only the [Takuya Kanbara] base name; the "≥2 [Hybrid]
  sources" qualifier is inexpressible and is omitted with an explicit YAML comment. The **standard**
  Lv4/Red/Cost-3 digivolution (from `cards.json` `evo_costs`) keeps Aldamon reachable/attackable, so
  the judge-quiz Q4 pin (which only needs Aldamon on the field) is unaffected. **No-approximations
  note:** the alt-path is left UNDER-constrained on a cost-reduction line — a player could reach
  Cost 3 from a [Takuya Kanbara] base without the 2 Hybrid sources. Acceptable only because it is a
  rarely-reachable alt-cost and is flagged; close before relying on AD1-002 in deck legality.
- **Audit hazard discovered alongside:** the predicate spec struct does **not** set
  `deny_unknown_fields`, so a made-up key (e.g. `digivolution_trait_count_gte:`) parses **silently as
  a no-op** rather than erroring — worth a lint / `deny_unknown_fields` sweep so accidental
  under-constraint surfaces at load time.
- **Suggested fix:** add a `digisource_trait_count` formula/predicate leaf usable in alt-path `from:`
  filters — `{ digisource_trait_count: { trait: Hybrid }, gte: 2 }` — counting the base's
  digivolution sources whose trait set matches (exact via `trait_has` / substring via
  `trait_contains`). Threads like the EX3-014 `source_stack_count` selector but as a `from`-base
  predicate.
- **Blocks:** AD1-002 (alt-digivolve line only). YAML: `code/digimon-engine/cards/ad1/AD1-002.yaml`
  (comment marks the omission); per-card tests `code/digimon-engine/tests/cards_behavioral/ad1/ad1_002.rs`.

## "When an effect trashes this card from your security stack" carrier trigger  [G-DSL-ON-DISCARD-SECURITY-TRIGGER]  — OPEN 2026-06-06

Surfaced by **BT15-037 Gatomon** (judge-quiz Q9 authoring). Card text: "When an
effect trashes this card from the security stack, you may play it without paying
the cost."

- **What's missing (DSL):** there is no `when:` trigger token for "this card was
  trashed/discarded from the security stack by an effect" — DCGO
  `EffectTiming.OnDiscardSecurity` + `CanTriggerOnTrashSelfSecurity(.., cardEffect
  != null, card)`. The DSL has `on_security`, `on_own_security_removed`,
  `on_opponent_security_removed`, `on_check_face_up_security`, `on_lose_security`
  — none fire for the *card itself being discarded from security by an effect* with
  a follow-on "play this card free" body.
- **Impact:** Gatomon's "play this when trashed from your security" clause is
  omitted (flagged in the YAML header, no stub). The other 3 clauses (`<Barrier>`
  face + inherited, `[All Turns][OPT]` gain-memory) are implemented. Does NOT
  affect the Q9 ruling: Gatomon playing out *after* Mastemon's trim adds no
  security-removal memory (the removals already happened while it was in security).
- **Suggested fix:** add an `on_discard_security` (or `on_self_trashed_from_security`)
  carrier trigger token gated on effect-initiated discard of the carrier from its
  own security, exposing the carrier as `event_card` so a `play_from_security`-style
  free-play body can consume it. Likely shared by other "when trashed from security,
  you may play it" Digimon.
- **Blocks:** BT15-037 (the play-from-security-when-trashed clause). YAML:
  `code/digimon-engine/cards/bt15/BT15-037.yaml`; per-card tests
  `code/digimon-engine/tests/cards_behavioral/bt15/bt15_037.rs`.

## RESOLVED 2026-06-10 — controller-relative memory predicate  [G-DSL-OWN-MEMORY-PREDICATE]

Surfaced: judge-quiz Q15 authoring (EX8-073 / BT17-016 memory-gated immunities).

- **Card text shape:** "While **you** have 0 or less memory, this Digimon isn't affected by …" — the bound is on the CARD CONTROLLER's signed memory, but `memory_lte`/`memory_gte` compare the raw turn-player-perspective gauge, which cannot express a controller-relative bound for the non-turn player.
- **Resolution:** new predicate leaves `own_memory_lte` / `own_memory_gte` — evaluate the controller's signed memory (`game.memory` when it is the controller's turn, `-game.memory` otherwise). Spec `digimon-dsl/src/predicate.rs` → compiled (`compiled.rs`) → compile copy-through → engine eval (`dsl_cards/predicate.rs`).
- **Consumers:** EX8-073 Gallantmon (X Antibody) `[All Turns]` immunity, BT17-016 Gallantmon `[Your Turn]` immunity (both `active_when` gates on continuous auras).

## RESOLVED 2026-06-10 — continuous effect-immunity aura payload  [G-DSL-AURA-EFFECT-IMMUNITY]

Surfaced: judge-quiz Q15 authoring (EX8-073's stub header listed "memory aura immunity" as a gap).

- **Card text shape:** "[All Turns] While …, this Digimon isn't affected by [your opponent's] [Digimon] effects" — a CONTINUOUS immunity (DCGO `CanNotAffectedClass` with a `CanUseCondition`), not the one-shot expiry-bound `grant_effect_immunity` step.
- **Resolution:** new `kind: aura` body slot `effect_immunity: { source_kind?: digimon|tamer|option|rule, source_controller: any|opponent|own }` (omit `source_kind` for all-kind immunity). Self-aura only (`target: {}`). Lowered on the declarative-tick path to a MATERIALIZED filtered `CannotBeAffected` install (`EffectContext::add_declarative_effect_immunity_modifier`), re-evaluated each tick under `active_when` — so the immunity turns on/off with its printed gate, including MID-De-Digivolve via the per-pop re-tick in `Game::de_digivolve_core` (judge-quiz Q15).
- **Consumers:** EX8-073 (opponent Digimon effects, `own_memory_lte: 0`), BT17-016 (all opponent effects, `your_turn` + `own_memory_lte: 0`).

## Result-log invisible across an `if:`-body park  [G-DSL-IF-BODY-PARK-RESULT-LOG]  — OPEN 2026-06-10 (pitfall)

Surfaced: judge-quiz Q15 authoring (BT17-016 first draft).

- **Symptom:** wrapping `select_* → delete_permanent` inside an `if: { condition: any_permanent…, then: […] }` and following the `if` with `if: { condition: { effect_deleted_any_opponent_digimon: false } … }` makes the deleted-tracker read FALSE NEGATIVE: the select inside the `if` body parks, `park_outer_tail` captures the clause's remaining steps with a CLONE of the bindings taken BEFORE the deletion is recorded, so the outer `effect_deleted_*` predicate never sees the result log written by the continuation.
- **Workaround (validated idiom, BT25-014):** keep `select_* + delete_permanent` at the TOP LEVEL of the process — an empty mandatory select is skipped silently and the result log stays on the single continuation chain. BT17-016 / BT12-016 / EX3-057 / EX8-073 all use this shape.
- **Fix shape (if ever needed):** share the result log via the `EffectContext`/game rather than per-continuation `Bindings` clones, or merge the continuation's result log into the parked outer-tail bindings at drain time.

## Q29 EX10 Bagra cluster — new gaps (2026-06-11, judge-quiz Q29 authoring)

### BT10-093 / EX10-056 — "when a card is placed under this permanent" trigger  [G-DSL-ON-CARD-PLACED-UNDER-TRIGGER]

- **Card text:** BT10-093 Yuu Amano "[All Turns][Once Per Turn] When a purple card is placed under this Tamer, <Draw 1> and gain 1 memory." / EX10-056 Bagramon's [All Turns] observer also fires when "effects place cards under" opponent Digimon/Tamers (that half omitted; the digivolve half is authored).
- **DCGO:** `BT10_093.cs` `CanTriggerOnAddDigivolutionCard(permanent == self, card has Purple)`.
- **Gap:** the DSL has `on_digivolution_card_trashed` (the REMOVAL direction) but no ADDITION-direction timing ("card placed under this/any permanent"). Fix shape: fire a `DigivolutionCardAdded` event from `push_under`/`place_as_bottom_source`/DigiXros commit sites, expose `when: on_card_placed_under` + host/event-card filters.
- **Consumers:** BT10-093 (clause 1, OMITTED), EX10-056 (observer's placed-under half, OMITTED).

### EX10-031 — would-leave triggered observer with stack access  [G-DSL-WOULD-LEAVE-TRIGGERED-OBSERVER]

- **Card text:** "[All Turns][Once Per Turn] When this Digimon would leave the battle area, you may play 1 play cost 4 or lower card from its digivolution cards without paying the cost."
- **DCGO:** `EX10_031.cs` plays the card from the still-intact stack in the WOULD-LEAVE window; the leave still happens (non-replacement).
- **Gap:** DSL would-leave lowering covers REPLACEMENTS (cancel/substitute) only; a triggered observer in that window that reads the carrier's digivolution cards has no vocabulary. OMITTED.

### EX10-056 — place an opponent PERMANENT as a digivolution source  [G-DSL-PLACE-PERMANENT-AS-SOURCE]

- **Card text:** "[On Play][When Digivolving] You may place 1 of your opponent's Digimon as any of their other Digimon's bottom digivolution card or under any of their Tamers."
- **Gap:** `place_as_bottom_source` moves CARDS; tucking a battle-area PERMANENT must move the whole stack with leave semantics, and the destination is OPPONENT-controlled (own-side selects only today). OMITTED.

### EX10-059 — blind opponent-hand pick + cross-player tuck  [G-DSL-BLIND-OPP-HAND-PLACE]

- **Card text:** "[On Play][When Digivolving] Choose 1 card in your opponent's hand without looking and place it as any of their Digimon's bottom digivolution card or under any of their Tamers."
- **Gap:** no unrevealed/blind opponent-hand selection, and no cross-player tuck destination flow. Sentence 2 ("by placing 3 [Bagra Army] trait Digimon cards from your trash as this Digimon's TOP digivolution cards, delete 1 of their Digimon or Tamers with cards under it") additionally needs the pre-existing G-DSL-PLACE-AS-TOP-SOURCE (BT13-088). Both sentences OMITTED.

### EX10-059 — gain sources' [All Turns] effects  [G-DSL-GAIN-ALL-TURNS-FROM-SOURCES]

- **Card text:** "[All Turns] This Digimon gains all [All Turns] effects on all level 6 [Bagra Army] trait Digimon cards in its digivolution cards."
- **DCGO:** source-card effect adoption (reads the source CARDS' text boxes).
- **Gap:** no DSL/engine machinery grants a permanent the printed effects of its digivolution source cards. OMITTED.

> **Pre-attach outside the recipe — RESOLVED 2026-06-11 (judge-quiz Q29).**
> `preattach_digixros_material` previously *validated the card against the
> DigiXros recipe slots* (`try_pre_attach_material` → `resolve_material_origin`),
> silently dropping any pre-attach that matched no slot — which broke Yuu Amano
> (BT10-093): its would-play hook places arbitrary purple Digimon from under
> Tamers, none of which are `[Bagramon]`/`[DarkKnightmon]` recipe materials.
> DCGO parity (`SelectDigiXrosClass.AddDigivolutionCardInfos`) does not
> recipe-validate pre-attached cards. Fixed: `EffectContext::
> preattach_digixros_material` now falls back to the new slot-independent
> `DigiXrosTransaction::pre_attach_extra_material` (recipe_slot `None`), so
> the card joins the transaction with its cost delta and the pre-attached
> placement order. BT12-112 (whose pre-attach coincidentally matches its own
> recipe) keeps the slot-resolving path. Pinned by
> `judge_quiz::e_partition_digixros::q29_*`.

## RESOLVED 2026-06-12 — `on_any_link` board-wide link observer  [G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED]

**Status: RESOLVED 2026-06-12 (Appmon BT21 wave).**

Cards of the form "[Your Turn] When your Digimon get linked, …" (a Tamer or a
Digimon observing a link onto *any* of the controller's Digimon, not just
itself) had no DSL timing. The two extant OnLink timings both force a filter:
`when_linked` (self-filter `event_card == source_card`, requires `scope: linked`)
and `when_card_linked_to_this` (host self-filter `event_permanent ==
source_permanent`). Neither expresses a board-wide observer on a third party.

**Resolution:** added `when: on_any_link` (`Timing::OnAnyLink` →
`CompiledTiming::OnAnyLink` → `EffectTiming::OnLink` in `timing_map.rs`). It
lowers to `OnLink` with NO forced self/host filter — scope is gated entirely by
`active_when:` predicates that already read the Linked trigger payload:
`event_target_owner: you` (the link HOST's controller), `event_card_trait_has:`
(the just-linked card's traits), and `your_turn: true`. Pair with
`source_is_unsuspended:` + `activation_cost: { suspend_self: true }` (or a body
`unsuspend: { target: source }`) for the common "by suspending/unsuspending this"
cost. First production users (all green): BT21-084, BT21-101, P-217 (and BT21-009
family via the host-side timing). Same timing unblocks P-241, BT23-079, BT24-087,
BT25-075's observer sub-clause.

## RESOLVED 2026-06-13 — `app_fuse` step (effect-initiated App Fuse)  [G-DSL-APP-FUSE]
**Status: RESOLVED 2026-06-13.** New DSL step `app_fuse: { from: hand|trash, result_filter?, optional }` for the printed "1 of your Digimon may app fuse into a Digimon card in the hand/trash" rider. Lowers to `CompiledStep::AppFuse` → `EffectContext::initiate_effect_app_fuse`. Added to `body_first_step_is_declinable` (installs its own PASS-able selections). First users: BT21-084, BT23-079, P-241, BT24-087, BT25-089. See `docs/RUST_ENGINE_GAPS.md` "Effect-initiated App Fuse — RESOLVED 2026-06-13".


## G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH — continuous mass "treat as a <DP> Digimon" aura with a synth identity (DATA SQUAD)
- **Card(s):** BT25-104 ShineGreymon: Burst Mode (Option face), clause "[Your Turn] All of your [Marcus Damon]s are also treated as 12000 DP Digimon and gain <Rush>". Generalizes to any "treat your Tamer(s) as a Digimon with DP X" continuous effect.
- **Status:** RESOLVED 2026-06-18. BT25-104 now ships FULLY IMPLEMENTED (the [Your Turn] aura is green — `bt25_104_your_turn_marcus_treated_as_12000_digimon_with_rush`). The substrate was widened along BOTH paths that previously dropped the payload.
- **What was done:**
  - **Declarative `kind: aura` path (the one BT25-104 uses):** added a `synth_identity` axis to `AuraBody` (`code/digimon-dsl/src/clause.rs`) → `CompiledDeclarativeClause::Aura.synth_identity` (`compiled.rs`) → compiled via `compile_synth_identity` (`compile.rs`) → threaded through `lower_aura::lower_all`/`lower` to the filter-install site, which now calls `add_declarative_modifier_with_payload` with the `ModifierPayload::SynthIdentity` (built by the now-`pub(crate)` `build_synth_payload`). Re-applied each tick over the live filter, so Marcus Damons played mid-turn are covered and it reverts at end of your turn. Authoring shape: `kind: aura` with `target: { of: you, kind: tamer, name_is: "Marcus Damon" }` (fold ownership into the FILTER — a `target_player: you` + filter combo routes to the wrong branch), `modifier: TreatAsDigimon`, `synth_identity: { dp: 12000 }`, `grant_keyword: { keyword: Rush }`.
  - **`add_modifier { continuous: true }` floating-mass path:** `FloatingMassModifier` gained a `payload` field threaded through `add_floating_mass_modifier` + the per-tick materialization in `game/triggers.rs` (symmetric fix to the latent drop; the lowering at `dsl_cards/step/modifiers.rs` now passes the computed payload instead of `None`).
  - `effective_dp` and the treat-as-Digimon machinery already read the synth DP (proven by the single-target cards BT13-020/BT21-044/AD1-021), so no combat/zone changes were needed.

## G-DSL-FIELD-SELECTOR-LOWEST-LEVEL — `selector: lowest_level` for select_* clauses (field selector, not just aggregate)
- **Card(s):** BT25-029 MirageGaogamon ("return 1 of your opponent's lowest level Digimon to the hand"); AD1-012 Omnimon Alter-S (same wording). 
- **Status:** RESOLVED 2026-06-18 — NOT actually needed (was a misdiagnosis). These cards are better served by the **AggregateSelector FILTER** path, which is already wired: `filter: { level_matches_aggregate: { selector: lowest_level, of: opponent } }` inside a `select_opponent_permanent`. That path is *more faithful* than a `FieldSelector` auto-pick: DCGO's `LowestLevelPermanentCondition` (`IsMinLevel`) is a target FILTER + a `SelectPermanent`, so the player chooses among tied lowest-level Digimon (rule 17), whereas a `FieldSelector { selector: lowest_level }` would auto-select the extreme and hide that choice. BT25-029 ships IMPLEMENTED on this path (8/8 green); AD1-012 should use the same shape. No new `FieldSelector` vocabulary required — the unevaluated `CompiledFieldSelector::LowestLevel/HighestLevel` can stay dormant until a card genuinely needs a field auto-pick by level (none known).
## OPEN 2026-06-14 — starter-deck (ST1-6) audit action-space-fidelity divergences  [G-AUDIT-ST1-6]
**Status: OPEN, deferred.** Surfaced by the `battle-test-starter-decks-st1-6` faithfulness re-audit (see `openspec/changes/battle-test-starter-decks-st1-6/notes/phase1-audit-findings.md`). All are minor action-space-fidelity divergences (no wrong outcomes / crashes / soft-locks); none block training-readiness.

1. **Suspend-target over-restriction (`is_unsuspended: true`)** — ST4-13 HerculesKabuterimon `<Digi-Burst 2>` suspend, ST4-15 Needle Spray suspend (and ~46 other cards repo-wide) filter the suspend target to `is_unsuspended: true`. DCGO (`ST4_13.cs`/`ST4_15.cs`: plain `IsPermanentExistsOnOpponentBattleAreaDigimon`) and rule 15-15-6-3 permit choosing ANY opponent Digimon, including an already-suspended one (the suspend is then a no-op). No new vocab needed — fix is to drop the filter — but it is a **cross-cutting bulk card-data change (~46 cards + action-space)**, out of scope for an ST1-6-only change. Best handled as its own change with a shared smoke/soft-lock test.

2. **ST2-15 Kaiser Nail — missing "playable-as-new-permanent" source filter predicate.** The card plays a selected digivolution-card source "as another Digimon". DCGO `ST2_15.cs` gates the source pick with `CanPlayAsNewPermanent(payCost:false)` (field not full / no play-lock); the YAML's `select_material { kind: digimon }` exposes any Digimon source and the play silently fizzles if unplayable. Genuine **DSL-vocab gap**: a source/card filter predicate meaning "can legally be played as a new permanent right now". Behavior converges (you can't play it either way); cosmetic for outcomes, real for RL action-mask fidelity.

3. **ST6-13 CresGarurumon — `<Digi-Burst 2>` over-gated activation.** YAML's `[Main]` `condition` requires a valid Lv3 purple Digimon already in trash before Digi-Burst can be activated; DCGO `ST6_13.cs` gates activation only on `CanDigiBurst()` (≥2 sources) and plays nothing if no target. Removing the trash-target gate would restore the (never-correct) "pay Digi-Burst with no play" line, but the mandatory inner `select_trash` would then need a skip-if-empty path to avoid a soft-lock. Deferred: current behavior is strictly safe and the removed line is never optimal play.

ST6-12 VenomMyotismon was flagged by the auditor (`optional_zero` vs DCGO force-≥1) but is a **false positive** — "up to N" permits 0 per rule 15-10-2-2 (PDF outranks DCGO's UI quirk), consistent with ST5-12/ST5-15 and the `reference_dsl_optional_mandatory_selection_pitfall` convention. No change.

## OPEN 2026-06-14 — Royal Knights re-audit pass: newly surfaced gaps

The 2026-06-14 Royal Knights audit/implementation pass closed ~17 cards whose
prior gap markers were stale. The following gaps remain genuinely open and were
surfaced or sharpened during the pass.

### `G-DSL-EVENT-CARD-TEXT-CONTAINS` — event predicate on the played card's effect TEXT
- **Consumer:** AD1-018 LordKnightmon ([All Turns][OPT] "When you play a Digimon with [Knightmon]/[Lucemon] in its text, <De-Digivolve 2>").
- **Missing:** DSL has `event_card_name_contains` and `event_card_trait_has` leaves, plus a static `effect_text_contains`, but no event-card leaf that matches the *played* card's effect text. The De-Digivolve-2 observer cannot gate on "in its text".
- **Suggested API:** add an `event_card_text_contains: "<substr>"` predicate leaf (sibling of `event_card_name_contains`) reading the played card's effect_description.
- **First test:** play a Digimon whose effect text contains "Lucemon" while AD1-018 is in play; assert the De-Digivolve-2 prompt fires; play one without it and assert no fire.

### `G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM` — return a selected digivolution source to deck bottom
- **Consumer:** BT13-075 Alphamon ([All Turns][OPT] would-leave self-protection: "by returning 1 [X Antibody]/[Royal Knight] card from this Digimon's digivolution cards to the BOTTOM OF YOUR DECK, it doesn't leave").
- **Missing:** the only source-return DSL verb is `return_selected_sources_to_hand` (to hand). No sibling returns a chosen digivolution source to the deck bottom; returning to hand would be an approximation (disallowed).
- **Suggested API:** `return_selected_sources_to_deck { position: bottom }` (or a `destination` param on the existing verb).
- **First test:** BT13-075 with X-Antibody/Royal-Knight sources, trigger a would-leave-by-effect; assert the pay-cost selection returns a chosen source to deck bottom and cancels the departure; decline path lets it leave.

### `G-PLAY-COST-GTE-MODIFIER-AURA` — continuous can't-attack-players aura keyed on opponent play cost ≥ N
- **Consumer:** BT13-075 Alphamon ([On Play][When Digivolving] "opponent's Digimon with play cost 10 or higher can't attack players until end of opp turn"); related BT20-021.
- **Missing:** a continuous, re-evaluated `CannotAttackPlayer` aura filtered by `play_cost_gte: N` that also covers opponent Digimon entering after resolution. Current snapshot/for_each modifiers don't cover late entrants. The clause is also atomic with a source-placement cost.
- **Suggested API:** an aura step with `target_filter: { play_cost_gte: N }` applying `CannotAttackPlayer` with `Expiry::end_of_opponents_turn`.
- **First test:** resolve BT13-075's On Play; a ≥10-cost opponent Digimon that enters AFTER resolution still can't attack players that turn; a <10-cost one can.

### `G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD` — deleted-permanent DP on the effect-deleted result payload
- **Consumer:** EX4-065 Trident Gaia ("If a Digimon with 13000 DP or more is deleted by this effect, trash opp's top security card").
- **Missing:** the effect-deleted result payload (`effect_deleted_any_opponent_digimon`) stores only PermanentHandles — no DP, and the carrier has moved to trash by the time the rider evaluates. No predicate exposes "the DP of the just-deleted permanent ≥ N".
- **Suggested API:** capture deleted-permanent DP (pre-removal snapshot, cf. rule 25) into the effect-deleted payload and add a `deleted_dp_gte: N` result predicate.
- **First test:** EX4-065 deletes a 13000-DP highest opponent Digimon → opp top security trashed; deletes a 12000-DP one → no trash.

### `G-FOR-EACH-COUNTED-FIELD-OBJECTS` — repeat an op N times where N counts over multiple field-object groups
- **Consumer:** BT13-030 UlforceVeedramon ([On Play][When Digivolving] "for each of your Royal Knights AND each of your blue Tamers, trash the top 2 digivolution cards of 1 opponent Digimon").
- **Missing:** an iteration count derived from the sum of two distinct own-field object groups (Royal-Knight Digimon + blue Tamers) driving N repetitions of a per-target trash-2-sources op.
- **Suggested API:** a `repeat: { count: <formula over multiple count_in_zone terms> }` wrapper, or extend `for_each` to accept a numeric repeat-count formula.
- **First test:** 2 Royal Knights + 1 blue Tamer → 3 iterations of "trash top 2 sources of a chosen opponent Digimon".

### `G-SOURCE-COUNT-SECURITY-TRASH` — trait-count-in-this-permanent's-sources formula
- **Consumer:** BT20-021 Jesmon GX ([When Attacking][OPT] "unsuspend self, then trash opp top security for every 2 [Royal Knight] cards in this Digimon's digivolution cards").
- **Missing:** no formula counts cards of a given trait among *this permanent's* digivolution sources (only `same_level_pairs_in_sources` exists). Need `trait_count_in_sources { trait: "Royal Knight" }` → floor-div 2 → N security trashes.
- **Suggested API:** a `trait_count_in_sources` formula term; drive `trash_top_security` repeated floor(count/2) times.
- **First test:** BT20-021 with 4 Royal-Knight sources → 2 security trashes; 3 sources → 1; 1 source → 0.


## RESOLVED / RECLASSIFIED 2026-06-15 — Royal Knights engine-gap closure pass

Adversarial scoping of the ~30 Royal-Knights-"blocking" gaps found that **14 were
not real gaps** (composable from shipped vocabulary today) and closed **6 genuine
small/medium gaps** via TDD. Net: only a handful of true RK gaps remain (the large
frameworks below).

### CLOSED this pass (TDD, consumer card now fully faithful)
- **G-DSL-EVENT-CARD-TEXT-CONTAINS** — new event-predicate leaf `event_card_text_contains` (played card's printed text). Consumer AD1-018. Commit `19be5a16`.
- **G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM** — new DSL verb `return_selected_sources_to_deck { position }`. Consumer BT13-075. Commit `a83d2827`.
- **G-RETURNED-CARD-COLOR-BINDING** — new predicate leaf `color_matches_returned_card` (reads the effect's returned-to-deck result log). Consumer EX10-068. Commit `78c84132`.
- **G-DELAY-NEXT-DIGIVOLVE-COST-REDUCTION** — engine fix: free digivolve-cost reducer auto-applies (no spurious accept/decline). Consumer ST12-15. Commit `b414917f`.
- **G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD** — effect-result log now carries each deleted permanent's pre-removal DP; new predicate `effect_deleted_opponent_digimon_dp_gte`. Consumer EX4-065. Commit `ba9afcee`.
- **G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY** — `select_union_zone` extended with a `material` zone (`material_of: { own_breeding }`) + per-zone filters. Consumer BT13-019. Commit `59eb5994`.

### RECLASSIFIED — NOT a gap (authorable-now with existing vocabulary)
Per-card scout verdicts (status `authorable-now-no-gap`) — these need CARD AUTHORING, not engine work. Do NOT re-file as engine/DSL gaps:
- **G-PLAY-COST-GTE-MODIFIER-AURA** (BT13-075) — continuous CannotAttackPlayer aura + `play_cost_gte` filter; authored in BT13-075 this pass.
- **G-DISTINCT-COLOR-COUNT** (EX10-068) — `distinct_colors_count` formula; authored in EX10-068 this pass.
- **G-FOR-EACH-COUNTED-FIELD-OBJECTS** (BT13-030) — repeat-count over summed field groups.
- **G-SOURCE-COUNT-SECURITY-TRASH** (BT20-021) — trait-count-in-sources formula already composable.
- **G-UNION-HAND-TRASH-SOURCE-COST** (BT20-021) — hand/trash place-as-source cost composable.
- **G-ALLY-PLAYED-OTHER-EVENT** (BT13-087) — `on_ally_played` + event filters compose it.
- **G-SECURITY-REMOVED-OBSERVER-UNIFIED** (BT20-056, BT20-060) — composable from the shipped on_own/on_opponent security-removed timings.
- **G-SUSPEND-OBSERVER-UNSUSPEND** (BT20-045) — any-suspend observer composable.
- **G-HIGHEST-DP-SWEEP** (BT20-045) — highest-DP aggregate sweep composable.
- **G-EFFECT-INITIATED-DIGIVOLVE-FROM-TRASH-ON-ATTACK** (EX11-069) — composable.
- **G-END-OF-ALL-TURNS-SUSPEND-COST-TRASH-RECURSION** (EX11-069) — composable.
- **G-EFFECT-RESULT-FALLBACK** (BT13-111) — composable.
- **G-COMBINED-TRASH-COUNT-COST** (BT13-111) — both-players-trash count formula composable.
- **G-SAME-LEVEL-X-DIGIVOLVE-OBSERVER** (BT9-092) — composable.
- **G-DSL-ON-DISCARD-SECURITY-TRIGGER** (BT15-084, BT15-092) — already CLOSED (shipped earlier).

### Still genuinely OPEN (deferred — large frameworks / not yet scoped)
- **G-BREEDING-DIGIVOLVE-UNION-ZONES** (BT20-056) — size L; attack-context breeding digivolve from hand/trash union.
- **G-UNION-HAND-SOURCE-PLAY** (EX11-053), **G-OPPONENT-PLAYED-DIGIMON-LEVEL-BRANCH** (RB1-035), **G-OWN-SECURITY-ADDED-OBSERVER** (BT8-090, likely authorable — re-verify), **G-SECURITY-END-OF-BATTLE-PLAY** (BT22-009), **G-ONDECLINE-CALLBACK** + **G-WAS-PLAYED-BY-EFFECT-OBSERVER** (BT13-102, engine), **G-OPTION-BATTLE-AREA-CARRIER** (BT19-093, engine, size L) — rate-limited out of the scoping pass; scope before authoring their cards.


## OPEN 2026-06-15 — Royal Knights final-3 residual gaps

After authoring all 16 remaining Royal Knights cards, exactly THREE cards retain
one clause each on a genuine residual gap (RK is now 69 IMPLEMENTED / 3 PARTIAL /
0 BLOCKED of 72). These are the only Royal-Knights-blocking gaps left.

### `G-BREEDING-DIGIVOLVE-UNION-ZONES` — attack-context breeding digivolve from hand/trash union
- **Consumer:** BT20-056 Alphamon. "[On Play][When Digivolving] then, if during an attack, 1 of your Digimon in the breeding area may digivolve into a Lv.6-or-lower [Chronicle] Digimon in your hand OR trash, free."
- **Missing:** an effect-initiated digivolve where the DIGIVOLVING permanent is a breeding-area Digimon and the digivolve TARGET is sourced from a hand∪trash union, gated on an in-attack condition.
- **Suggested API:** extend the effect-digivolve step to accept a breeding-area subject + a `from: { zones: [hand, trash] }` union target with a `during_attack` condition.
- **First test:** BT20-056 in play attacking, a breeding Digimon present, a Lv.6 [Chronicle] in hand and one in trash → assert both are offered as free digivolve targets onto the breeding Digimon.

### `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN` — effect-play observer with opponent's-turn suspend cost
- **Consumer:** BT13-102 Keenan Crier. "[Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer, gain 1 memory."
- **Missing:** combine a `was_played_by_effect` observer (effect-plays only) firing on the OPPONENT's turn with a source-bound suspend activation cost. The On Play on-decline clause is authored; this observer remains.
- **First test:** opponent's turn, an effect plays a Digimon → assert an optional "suspend Keenan to gain 1 memory" prompt; a normal (non-effect) play does NOT fire it.

### `G-OPTION-PERSIST-AS-FIELD-CARRIER` (+ `G-OPTION-SELF-TRASH-TRIGGER`) — Option self-places/persists in the battle area
- **Consumer:** BT19-093 Queen Device. "[Main] … then, place this card in the battle area" (a persistent Option carrier), and "When this card is trashed from the battle area, …".
- **Missing:** an Option self-placing into the battle area as a persistent carrier, plus a `when_trashed_from_battle_area` trigger on that Option carrier. The color-bypass + [Main]/[Security] debuff clauses are authored; the self-place/persist + trash-from-battle trigger remain.
- **First test:** resolve BT19-093 [Main] → assert this Option is now a battle-area permanent; trash it from battle → assert the trash-from-battle clause fires.
## EX11-027 Maquinamon link substrate — RESOLVED 2026-06-20 (collapse-dsl-step-idioms §4.5)
The four EX11-027 link gaps are CLOSED and moved to [qa/resolved-gaps.md](resolved-gaps.md):
`G-DSL-LINK-RELINK-STANDING-PERMANENT` (the `relink_self_to_own_digimon` verb),
`G-DSL-LINK-HOST-FILTER` (`link_cards` `host_filter` + `exclude_source`),
`G-DSL-LINK-HETEROGENEOUS-CHOICE` (if-gated `select_effect_choice`, no new vocab), and
`G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE` (the `place_link_card_as_bottom_digivolution`
replacement cost). EX11-027 Maquinamon is now pure DSL (off test-only raw_rust); the
`dsl-substrate-integrity` loader guard is promoted to a hard error.

### `G-DSL-ALT-PATH-GATE-CONDITIONALS` — alt-digivolve `from:` predicate lacks board-state / compound / negative-colour gates
Surfaced by: the pool-wide alt-path authoring audit (promote-official-bandai-card-source, 2026-06-20).
The alt-digivolve `from:` predicate supports single level/colour/trait/name gates (+ `all_of`/`any_of`),
but four implemented cards print special-digivolution conditions it can't express, so those routes are
intentionally omitted (the cheaper standard/encoded routes ship; the conditional route does not):
- **Board-state conditional name gate** — BT23-013 ("[Huckmon] while opponent has a 10000 DP or higher
  Digimon: Cost 5") and BT15-101 (Tamer-presence + opponent-DP threshold). Needs a `from:` that can read
  game/board state (opponent field DP, own Tamer presence) at digivolve-eligibility time.
- **Compound multi-card gate** — EX11-074 ("While you have [Shoto Kazama], [GrandGalemon]: Cost 6"):
  a conjunction of a controller-has-named-card condition AND a base-name gate.
- **Negative tri-colour gate** — BT25-084 ("[Titamon] w/o 3 colors: Cost 2"): a name gate combined with
  a NEGATIVE colour-count condition (base has fewer than 3 colours).
These are tracked as allowlisted entries in `code/tests/test_alt_path_authoring_parity.py` (engine_gap
reason) so the authoring-parity guard stays green while documenting the omission.
- **Suggested API:** extend the alt-path `from:` predicate vocabulary with a board-state condition
  (`controller_has_named`, `opponent_has_dp_gte`) and a colour-count predicate (`color_count_lt`).
- **First test:** BT23-013 in hand over a Lv.5 base while the opponent has a 10000-DP Digimon → assert the
  cost-5 [Huckmon] route is offered; with no such opponent Digimon → not offered.

## Binding-scoped exact-name predicate (`binding_card_name_is`)  [G-DSL-BINDING-CARD-NAME-EQUALS]  — RESOLVED 2026-07-03
> **RESOLVED 2026-07-03 (leaves II):** `binding_card_name_is: {binding, name_is}` — effective-name (printed + also_treated_as) exact comparison. BT21-087's name-branch approximation can be replaced.
Consumer: BT21-087 Zenith ([On Play] reveal 3, choose 1 [Vemmon]-text card: if its name IS [Vemmon] → play-free-or-add choice, else add to hand). The `if:` branch needs to test the BOUND revealed card's exact printed name; the predicate surface has `binding_card_kind` (kind) and (filed 2026-07-02) `binding_card_color` (color) but no name analogue, so BT21-087 approximates via a nested re-select shape documented in its YAML header. Suggested: `binding_card_name_is: { binding: <name>, name: "<literal>" }` — sibling of `binding_card_kind`, resolving the named card binding and comparing the effective (synth-identity-aware) card name. Workaround shipped in BT21-087 is faithful for its single-pick flow but repeats per card; the leaf amortizes.


> **Interactive pay_cost delete reducers (fixed + variable) — RESOLVED 2026-07-03** (store-champs
> round 1). `kind: cost_reduction` supports an interactive `pay_cost` selecting+deleting an own
> permanent, static `amount:` (fixed) or omitted + `delete_for_cost_reduction` (variable = the
> deleted permanent's printed play cost, rule-25 snapshot into
> `Game::pending_cost_reduction_amount_override`). Clone-safe (resumable VM). Closes
> G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT and
> G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION (BT25-076 Ghoulmon shape now
> expressible). Drivers: BT18-073, BT13-083, BT13-103 (authored). Tests:
> tests/cost_hooks/pay_cost_play_delete_reducer.rs.

> **Memory-count formula (`player_memory`) — RESOLVED 2026-07-03** (store-champs leaves I). See
> G-DSL-MEMORY-COUNT-FORMULA in docs/RUST_ENGINE_GAPS.md. BT25-086's End-of-Turn DP scaling is
> now expressible via `dp_modifier_fn: {base: 0, per: {player_memory: {of: opponent}}, delta: 1000}`.


> **Option USE from revealed/sources/union origins — RESOLVED 2026-07-03** (option-verbs
> reconciliation onto the round-1 `Game::use_option_from` core). DSL verbs
> `use_option_from_revealed {of, card, cost?}`, `use_option_from_sources {of, card, cost?}`
> (Source origin resolves card_sources AND linked_cards), `use_option_bound {binding, cost?}`
> (select_union_zone hand-or-trash consumer). Drivers EX7-048 cl.1 / BT25-085 use-facet /
> BT21-062. Facet 2 (trash-Option-from-sources-as-COST) remains OPEN.


> **G-DSL-BATTLE-WINNER-BOARDWIDE — RESOLVED 2026-07-03** (trigger-timings round 2). DSL `when: on_ally_won_battle` (EffectTiming::EndOfBattle via TriggerSource::BattleResolved{winner}; tie = no winner, matching DCGO !WasTie) + predicate leaves `event_winner_owner` / `event_winner_trait_has`. Direct player attacks never fire it. Unblocks BT25-020 Marsmon + the Olympos XII win-a-battle line. Tests: tests/battle_winner_boardwide.rs (6).

> **G-DSL-LINK-TRASH-AS-COST — RESOLVED 2026-07-03** (link-economy round 2). Cost step `trash_link_card_of_own_digimon {of, optional}`: two RL-visible selections (which Digimon with >=1 link card, which of its link cards), trashes via trash_specific_link_card (fires OnLinkedCardTrashed), unpayable => clause aborts. Clone-safe (TrashLinkCardOfDigimonSelection resume frame + clone test). Unblocks BT25-073 Dragomon. Tests: tests/dsl/trash_link_card_of_own_digimon.rs (11 w/ formula tests).

> **G-DSL-FORMULA-OWN-LINK-CARD-COUNT (+ SourceLinkCardCount facet of G-DSL-LINK-N-CARDS-PER-HOST) — RESOLVED 2026-07-03** (link-economy round 2). PerSelector variants `own_link_card_count {of}` (board-wide sum) and `source_link_card_count` (per-host), usable in base_per_delta / de_digivolve amount_fn. Unblocks BT25-075 Vulcanusmon's mass De-Digivolve magnitude.

> **G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New step
> `place_self_under_permanent: { target: <binding>, face_down: <bool, default false> }` — dispatches to the
> already-shipped engine primitive `EffectContext::place_self_under_permanent` (claims the in-flight
> `pending_option` on the [Main] Option-play path, so the Option is seated FACE-UP under the chosen own
> permanent instead of trashed; a live field-Option source routes to `move_field_option_under_permanent`).
> Silently no-ops on an unset target binding (the preceding select self-skipped — DCGO's silent skip).
> Consumers: P-180 / EX7-070 / EX7-071 (the "Then, place this card as the bottom digivolution card of 1 of
> your [Three Musketeers] Digimon" [Main] tail — EX7-071 authored + green this round). Tests:
> tests/dsl/option_lifecycle_cluster.rs (gap1_dsl_*), tests/cards_behavioral/ex7/ex7_071.rs.

> **G-DSL-DNA-TRASH-PARTNER — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New step
> `effect_initiated_dna_digivolve_trash_partner: { target, trash_partner, from_hand, cost, ignore_requirements }`
> — the trash-material sibling of `effect_initiated_dna_digivolve_hand_partner`, lowering to the engine
> primitive `EffectContext::effect_initiated_dna_digivolve_trash_partner` (G-ENGINE-DNA-TRASH-MATERIAL,
> resolved 2026-07-03: trash material moves STRAIGHT into the merged stack, no [On Play]; DCGO
> CreateNewPermanent + jogress). `cost: printed` resolves via `printed_dna_cost_for_field_trash_pair`;
> recipe enforcement composes. Consumers: BT18-015 (authored + green this round), BT18-073 (same shape,
> still to author). Tests: tests/cards_behavioral/bt18/bt18_015.rs.

> **G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD — ✅ RESOLVED (2026-07-03)** (DSL-wiring round). New
> no-subject predicate leaf `own_source_stack_color_count_gte: <N>` — distinct colors among the effect
> CARRIER's NON-FLIPPED digivolution sources (the shared `non_flipped_source_colors` extraction, same as
> `color_matches_own_source_stack`, so the gate and the Branch-A filter always agree; top card and
> face-down sources excluded; no carrier → fails closed). The YAML-reachable branch discriminant for
> EX9-074 Kimeramon's "if this Digimon has 6 or more colors in its digivolution cards, instead …" —
> authored as `if: { condition: { own_source_stack_color_count_gte: 6 }, then: [delete_one_per_opponent_color],
> else: [same-color single delete] }`. Consumer: EX9-074 (both branches authored + green this round).
> Tests: tests/dsl/kimeramon_color_mass_delete.rs (own_source_stack_* + yaml_branch_gate_*),
> tests/cards_behavioral/ex9/ex9_074.rs (SECTION 6).

## Event-target attack-capability trigger gate  [G-DSL-EVENT-TARGET-CAN-ATTACK]  — OPEN 2026-07-04 (minor, over-exposure only)

Surfaced: 2026-07-04, BT25-075 Vulcanusmon on_any_link debugging.

- **Card text (driver):** BT25-075 "[Your Turn] When your Digimon get linked, one of them may attack."
- **DCGO:** `BT25_075.cs` WhenLinked — `CanUseCondition` ANDs `permanent.CanAttack(activateClass)` into
  `CanTriggerWhenLinked`'s `PermanentCondition`, so the trigger does not even FIRE when the just-linked
  host cannot attack (notably summoning sickness: `Permanent.CanAttackTargetDigimon` returns false when
  `EnterFieldTurnCount == TurnCount && !HasRush`).
- **Gap:** the DSL has no predicate leaf exposing an event-target's attack capability (something like
  `event_target_can_attack: true` over `effect_attack_target_action_ids`). The clause instead relies on
  `may_attack_now`'s empty-candidate silent no-op, which is user-visibly equivalent for a solo trigger
  (declinable first step → no outer prompt installs). **Only divergence:** in a same-chooser TriggerOrder
  bundle, the drain's `non_firing_queued_effect_indices_for` filter evaluates the clause CONDITION only —
  with no attack-capability condition the Vulcanusmon entry stays in the bundle and can surface as a
  no-op ordering choice where DCGO would omit the trigger entirely (an over-exposed no-op action in the
  RL action space, same family as the optional-on-mandatory pitfall).
- **Consumers:** BT25-075 (shipped with the no-op fallback; behavior pinned by
  `tests/cards_behavioral/bt25/bt25_075.rs::bt25_075_on_any_link_no_attack_offer_for_summoning_sick_host`).
  Any future "when X, it may attack" observer shares the shape.
