# Rust Engine Gaps

Capability gaps in the Rust engine's scripting surface (`code/digimon-engine/`), discovered during archetype audits by `assess-rust-engine-archetype`. Distinct from [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md), which tracks Rust↔Python divergences in shared subsystems — this document catalogs **net-new primitives** the Rust scripting API needs before a given archetype can be implemented under the no-approximations policy (CLAUDE.md §17–18).

> **Attribute-predicate matching unimplemented — OPEN (2026-06-20).** The DSL
> predicate fields `attribute_is` / `form_is` are hard-coded to "no match"
> (`dsl_cards/predicate.rs` returns `false` whenever they're set — "attribute
> not yet tracked on CardData"). `CardData` *does* fold the attribute into its
> merged `traits` list (`card_data.rs` extends `traits` with `attribute_eng`),
> but there is no first-class attribute field nor a working attribute predicate.
> Consequence surfaced by the `promote-official-bandai-card-source` change: cards
> with a printed **`(Rule) Trait: Has [Free] attribute.`** grant (e.g. BT16-102
> Magnamon X, BT17-077 Imperialdramon: Paladin Mode) now have the `Free`
> attribute recovered into `attribute_eng` from the official Bandai DB, but any
> requirement keyed on the `[Free]` attribute still won't match until
> attribute-predicate matching is implemented. (Rule-granted *Type* traits are
> fully fixed by that change — they route through `trait_has`, which already
> matches `CardData.traits`.) Scope when picked up: add an attribute query
> (a real `CardData.attribute` field, or an `attribute_is`/`attribute_in`
> predicate that consults the trait list), keeping the RL mask and API in
> lockstep.

> **Permanent-scoped `CannotSuspend` / `CannotUnsuspend` enforcement — RESOLVED 2026-06-13**
> (Iceclad Liberator pool authoring: EX7-023, EX8-023, EX11-017). The
> permanent-scoped `ModifierType::CannotSuspend` / `CannotUnsuspend` modifiers
> were installable and expired correctly but had **no consult site** — a locked
> Digimon could still declare attacks, block, and be suspended by costs/effects
> (`tests/flood_gates/mask_gates.rs` even noted "no mask bit exists"). The
> engine now enforces them at: basic-attack declaration (`combat/mod.rs`
> `can_attack*` + `action/mask.rs` `can_basic_attack`, kept in lockstep so the
> RL mask and the API agree), block / Alliance / Counter candidate walks,
> suspend-as-cost helpers (`effect_context/action/suspend.rs`,
> `game_actions/mod.rs`), and the universal `game/suspend.rs` chokepoint
> (effect-driven suspend/unsuspend no-op on locked permanents; turn-start
> unsuspend skips `CannotUnsuspend`). Covered by
> `tests/combat/cannot_suspend_enforcement.rs` (10 tests) plus the now-un-ignored
> EX7-023 / EX11-017 enforcement assertions. **Still open:** the *player-scope
> mass* `CannotSuspend` / `CannotUnsuspend` auras below (lines for "none of your
> opponent's Digimon can suspend") — those need the player-scoped registry +
> future-play tracking, NOT just per-permanent enforcement; this closure is the
> foundation they build on, not a replacement.

> **Xros Heart DigiXros closure — 2026-05-24:** The
> `close-xros-heart-digixros-gaps` change closed the reusable Xros Heart
> DigiXros transaction substrate: recipe material prompts from hand,
> battle area, trash, and under-Tamer origins; per-material cost deltas;
> pre-attached materials; transaction-scoped zone allowances; selected-source
> attachment after successful payment; `digixros_count`; and deletion-timed
> `<Material Save>` recipe filtering. Production YAML and behavioral coverage
> landed for BT10-009, BT10-013, BT10-087, and BT12-112. Remaining open
> entries that merely "resemble DigiXros" should be read as non-DigiXros
> residuals, such as Apocalymon-style different-name cast-time assembly.

> **Xros Heart reusable primitive closure — 2026-05-24:** The
> `author-xros-heart-reusable-primitives` change closes the next reusable
> Xros Heart layer: card selection from under Tamers with origin identity,
> hand/trash/union placement under Tamers, free and reduced-cost play from
> under Tamers, generalized source movement and leave-battle source rescue,
> turn-scoped DigiXros wildcard substitution, and effect-created attack
> windows routed through normal attack prompts. Production YAML and focused
> behavioral tests now cover BT21-083, BT11-095, P-224, BT19-090, BT21-092,
> BT10-111, BT21-027, and BT19-061 without `raw_rust` placeholders. Remaining
> Xros Heart work should be tracked as card authoring or as non-Xros-specific
> residual primitives when a later card proves one.

> **Xros Heart stack-metric and lockout closure — 2026-05-24:** The
> `complete-xros-heart-authoring-substrate` change adds `source_color_count`
> as a source-relative formula and `per:` selector, plus `source_stack_count`
> for counting predicate-matched source cards beneath a target binding. The
> same change wires permanent-scoped `CannotActivateOnPlayEffects`,
> `CannotActivateWhenDigivolvingEffects`, and `CannotUnsuspend` through
> expiring modifiers. Stack-derived and lockout fixtures now have production
> YAML for BT19-014, AD1-006, AD1-013, BT19-026, BT21-030, BT19-038,
> BT19-051, BT19-035, BT20-037, and BT19-079. Remaining Xros Heart pool work
> should be treated as card authoring unless a later card proves a new
> non-Xros-specific residual primitive.
>
> **Xros Heart card-authoring residuals — 2026-05-24:** A follow-up
> authoring pass added production YAML and focused behavioral coverage for
> BT10-003, BT10-029, BT19-033, and BT19-047. The follow-up
> same-effect DP modifier selection primitive is now closed: permanent
> `dp_lte` / `dp_eq` / `dp_gte` predicates delegate to `effective_dp` after
> same-process `ChangeDp` steps rather than re-checking printed `CardData.dp`.
> BT19-012 and BT21-011 are now card-authoring follow-up work unless their
> production tests prove a narrower residual.

> **Tracker hygiene sweep — 2026-05-15:** Post-rebaseline audit cleanup
> per [`docs/superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md`](superpowers/audits/2026-05-14-rust-engine-gap-rebaseline.md).
> 8 audit-flagged CLOSED entries plus ~46 NARROW closed-core halves
> (54 total) moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md);
> ~12 entries had headline severity reframed from 🔴 → 🟡 PARTIAL
> with narrowed residual titles (e.g. "Decode residual: EX10-061
> Apocalymon batch + different-name source play DSL sugar",
> "Conditional security-in-stack trigger residual: start-of-turn
> variants"); new residual sub-entries spun off where the original
> umbrella had multiple distinct shapes (e.g.
> `play_from_revealed_free`, `play_from_security_at(index)`,
> "Top-N security trash + face-up flip", "Alt-digivolve with
> override-cost + ignore-reqs"). 2 UNCLEAR entries (EX9-032 Costed
> self-digivolve, EX4-074 mandatory chain) received audit footers
> recommending first-test writes before further engine work. The
> at-a-glance table was rewritten end-to-end. Cross-references in
> `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` were
> swept with sweep notes. Open-gap headings shrunk from ~107 to 42,
> with 65 redirect markers pointing at relocated entries. No engine
> code, tests, or card YAML were modified.
>
> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. The audit-table summary at the top is consistent with the
> per-entry status below. New since the previous sweep: Track B (PR
> #449 replacement framework), Track A (PR #451 event payload + Track A
> `ProvenanceToken`), Track C (PR #452 modifier taxonomy + 10 wired
> consult sites; PR #455 deferred modifier variants + `ModifierPayload`
> typed payloads), Track D (PR #450 attack-flow centralization), Track
> E (PR #453 zone-movement helpers + owner-routing fix; PR #454 ten
> deferred DSL verbs), Track G (PR #457 keyword library close — Evade
> fix, Decoy color-filter, Progress backfill), and the
> `Expiry::UntilCondition` runtime controller (PR #458). The owner-
> routing fix from PR #453 is now exercised end-to-end through real
> card flows by `code/digimon-engine/tests/owner_routing_live.rs`
> (pre-scaling cleanup batch §1). See `.claude/plans/pre-scaling-cleanup-batch.md`
> for the full closure-index narrative.

> **Tracker hygiene sweep — 2026-05-14:** Cross-referenced every entry
> against PRs #459–#473 and the per-archetype DSL gap input documents in
> `qa/archetype-qa/dsl/`. New material closures since the 2026-05-10
> sweep:
>
> - **Track H aura system (PR #467):** typed `AuraScope` / `AuraGrant`
>   builder API (`code/digimon-engine/src/aura.rs`), security-zone aura
>   tick dispatch, queue-based granted-triggered-effect dispatch with
>   parked-selection support, and `Expiry::EndOfOpponentsNextTurn` /
>   `EndOfYourNextTurn` (+ `pending_skips` for mid-opp-turn installs).
>   "Granted triggered ability", "Named-target declarative aura", and
>   "Declarative aura sourced from security zone" entries already
>   absorbed these per-phase notes inline (Phases 4a/4b/4e/4i/4k and
>   §3/§5/§9/§10); no entry-level status changes required this sweep.
> - **Alter-S Ladder DSL cards + tests (PR #468):** EX9-021 Omnimon
>   Alter-S and DNA Omnimon ladder fixtures landed using existing
>   substrate (sequential source-play, `place_permanent_on_security`,
>   replacement framework). The DNA Omnimon gap doc's "Decode and
>   play-from-material" and "WhenWouldBeDeleted" entries already cite
>   the EX4-060 / EX9-021 narrowing; no new engine gap surfaced.
> - **Formula-valued predicate thresholds for BT15-096 / BT21-102
>   (PR #470):** validates the "Track J formula/result substrate slice
>   (2026-05-10)" paragraph above against real card-shaped fixtures —
>   `play_cost_lte`, `distinct_colors_count`, `binding_play_cost`, and
>   `effect_suspended_any_own_digimon` formula leaves now drive
>   BT15-096's six active behavioral tests and BT21-102's Tamer-color
>   play-cost cap. Remaining Track J work (Zephagamon / TS Olympos /
>   BG Imperial card authoring) is unchanged.
> - **Puppet DSL observers (PR #472):** card-local fixtures for
>   `OnAnyDeletion` consumers (BT22-002, BT22-088, EX9-033, EX11-023,
>   ST19-14) using `event_target_kind` / `event_target_trait_has` /
>   `event_permanent_is_source` / `source_is_unsuspended`. These run on
>   the existing event-payload substrate; no new engine timing was
>   needed. The "Global OnAnyDigimonPlayed / OnAnyDeletion observer
>   timings" entry already absorbed the 2026-05-11 PUPPETS-G011
>   consumer adoption notes inline.
>
> Docs-only / non-engine PRs in this window: #459 AGENTS guidance,
> #460 Tauri CardData drift, #461 Option Plug-In lifecycle review fixes,
> #462 Tauri tensor-profile id test, #463 cross-card effect refire,
> #464 / #465 DSL batch skill metadata, #466 pre-scaling cleanup,
> #469 agent engineering guidelines, #471 DSL agent guide refresh,
> #473 commit-message context update.
>
> No new engine gap entries surfaced from the per-archetype DSL gap
> inputs (DNA Omnimon, Medusamon, Alter-S Ladder, BG Imperial, Chaos
> Control, Millenniummon, Puppets, Red Hybrid AncientGreymon, Rocks,
> Royal Knights, TS Olympos, Zephagamon). All reusable primitives
> called out by those documents are already represented by an entry
> in this file or as an open verb in `qa/dsl-vocab-gaps.md`.
>
> **Tracker hygiene sweep — Phase 2 rollup (2026-05-17, Tracks A–J, PR #480):**
> The Phase 2 pilot-archetype unblock work landed as 10 tracks in PR #480
> (`claude/musing-ishizaka-c4b355` against `main`). Substrate items closed:
>
> - **Track B (commit `2c2c4632`)** — `Effect::activation_cost(...)` builder
>   hook + `ctx.suspend_self_as_cost()` / `ctx.return_self_to_deck_bottom_as_cost()`
>   helpers. Cost failure consumes the OPT slot per Working Rule §17. Already
>   marked in the at-a-glance table at line 185.
> - **Track C (commit `dd9b8a46`)** — `G-OPT-TRIGGERED` and
>   `G-OPT-RESET-VIA-ATTACK-CYCLE` diagnosed as already-closed; substrate
>   verified correct, 23 stale `#[ignore]` annotations removed. Already
>   marked in the at-a-glance table at lines 186–187.
> - **Track D (commit `bc852640`)** — Inherited triggered-effect dispatch
>   walk in `enqueue_from_permanent` completed. The 2026-05-15 sweep had
>   already redirected the entry to `qa/resolved-gaps.md` (see line 684
>   below); Track D added the dedicated regression test in
>   `tests/timing_dispatch.rs` and un-ignored 18 dependent tests.
> - **Track F (commit `5cae5006`)** — `EffectContext::place_top_source_as_bottom`
>   helper (substrate); chained `select_own_permanent → select_* →
>   effect_initiated_digivolve` dispatcher confirmed already-correct
>   (phantom gap); `AltPathSpec.direction: into` schema for warp-shape
>   alt-paths. Closes G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET
>   without engine code changes (just tests un-ignored).
> - **Track G (commit `48e1255f`)** — `opponent_security_count_lte` /
>   `_gte` predicates, Plug-In Link DSL surface (consuming Track I's
>   substrate), Medusamon card-author sweep. EX11-054 [All Turns] clause
>   migrated to Track B's `activation_cost`. **Killed the long-standing
>   `ex11_054` Medusamon flake.**
> - **Track H (commit `2b083c5a`)** — BeforePayCost substrate extensions:
>   `cost_target` + `source_is_cost_target_permanent` predicates (digivolve
>   target predicate evaluation), `Effect::before_pay_cost_observe(card)`
>   sibling builder for BeforePayCost gain-memory clauses,
>   `select_trash` declined-optional outer-tail continuation,
>   `PlayFromHandFreeArgs.bind_as` for delayed-return clauses. DEFERRED:
>   G-COST-REDUCE-ALLY-DIGIVOLVE per Track H's discovery rider.
> - **Track I (commit `26e27ccc`)** — `applies_to_opponent_security_dp`
>   inherited aura flag (PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA);
>   Plug-In re-link substrate (`Game::orphan_linked_plug_in`,
>   `Game::relink_plug_in`, `OptionFieldState::LinkedPlugIn` /
>   `OrphanedPlugIn`); end-of-attack mandatory self-delete chain (already
>   marked in table line 198); PUPPETS-G009 Delay [Main] action — see
>   "Standard Delay main-phase activation action" §346 below, now closed.
> - **Track J (commits `48fbfd76` + `3a6aaee1`)** — RK-G001 breeding
>   permanent target predicate filter
>   (`SelectOwnBreedingPermanentArgs::filter`); RK-G002 via Track B's
>   `activation_cost`; RK-G003 via existing replacement framework + Armor
>   Purge keyword. Token registry entries for Atho / Rene / Por.
>
> **Tracks A, E, F (DSL), G (DSL), H (some), I (some)** lowered to DSL-only
> sweeps tracked in [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md).
>
> **Net cumulative test deltas:** `cards_behavioral` 2355 pass / 0 fail /
> 355 ignored — was ~2300 / 1 pre-existing flake / 596 ignored.
> See `qa/resolved-gaps.md` for full per-track closure details.

> **Tracker hygiene sweep — Phase 2 Track E (2026-05-17):** Rocks-pilot
> author-facing residual closures. The Rocks gap-input doc's
> `G-ROCKS-REVEAL-ORDERING`, `G-ROCKS-OPTION-SELF-DISPOSITION`, and
> `G-ROCKS-PLAYER-SCOPED-PASSIVE-MODIFIERS` entries are all closed by a
> single PR. The new `choose_from_reveal` + `order_remainder` DSL verbs
> (see `qa/dsl-vocab-gaps.md` "Phase 2 Track E (2026-05-17)" block and
> `qa/resolved-gaps.md` "Phase 2 Track E closure" block) lower onto the
> already-shipped `select_reveal` / `select_effect_choice` /
> `select_ordered_permutation` / `place_remainder_on_deck` engine
> helpers — no new engine substrate. The "Selection: ordered
> permutation" headline (already RESOLVED 2026-05-15 in
> `qa/resolved-gaps.md`) stays closed; this PR consumes the substrate
> through an author-facing DSL surface. Authored card drivers:
> P-167 (full reveal/source-placement clause via the new verbs), EX8-047
> (two-pick reveal clause), BT9-103 (Main + Security mirror via
> `add_player_modifier` + `for_each` + `add_modifier`); P-206 raw_rust
> shim removed in favour of native `add_this_option_to_hand`.

> **DNA Omnimon completion closure — 2026-05-20:** The
> `complete-dna-omnimon-archetype` change drove the DNA Omnimon archetype
> (64 cards) to **62 IMPLEMENTED / 2 PARTIAL / 0 BLOCKED** (Phase A
> baseline 34 / 25 / 5). ~20+ engine/DSL substrate gaps were closed and
> ~18 stale-tracker gaps were confirmed and used; full per-gap closure
> record in [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) § "Phase 2 /
> DNA Omnimon completion closure — 2026-05-20". Engine-level substrate
> closed by this change includes `effect_initiated_dna_digivolve_with_hand_partner`
> (G-DSL-DNA-FROM-HAND-PARTNER), `ForEach` stable per-iteration top-card
> identity (G-FOR-EACH-DELETE-INDEX-SHIFT), `OnLeaveField` timing fired
> from deletion + return paths, AD1-012's defender-side
> effect-initiated DNA mid-attack-interrupt, `source_dp` /
> `source_material_count` formula inputs, `play_security_card` +
> `EffectContext::play_from_security_card`, and
> `Modifiers::granted_security_attack_keyword_bonus`.
>
> **DNA Omnimon partial-gap closure — 2026-05-22:** The follow-up
> `close-dna-omnimon-partial-gaps` change resolved the two remaining
> DNA Omnimon partial gaps: `G-DYNAMIC-NAME-ALIAS-FROM-STACK`
> (BT17-102 source-derived effective names) and
> `G-DSL-DELAY-ON-ATTACK-EVENT` (BT23-096 event-backed
> ally-attack Delay dispatch). The archetype ledger is now
> **64 IMPLEMENTED / 0 PARTIAL / 0 BLOCKED** with zero live
> `raw_rust` escapes.

> **TS Olympos representative unlock — 2026-05-24:** The
> `close-ts-olympos-rust-gaps` change closes the representative-deck
> blockers called out by the TS Olympos DSL gap input: source-stack
> material aggregates, formula-valued De-Digivolve amounts,
> predicate-scoped `[When Attacking]` / `[When Digivolving]` timing
> suppression, effect-driven Option use from hand through the normal
> Option lifecycle, face-up-security count predicates, and bottom-security
> to-hand movement. The resolver snapshot is now 23/23 representative
> cards authored in Rust YAML, with 62/117 broad-pool cards authored and
> 55 broad residuals documented in
> `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`.
> No tensor or action-space contract changed.

There are two related assessment workflows:

- `.codex/skills/assess-rust-engine-archetype/` is the Codex read-only readiness workflow. It inspects printed text, current DSL schema/lowering, engine action/pending-selection support, and tests, then reports `ready`, `dsl-gap`, `engine-gap`, `rules-gap`, `test-gap`, or `data-gap` findings. It should cite this tracker when a known primitive blocks an archetype, but it does not modify files.
- `.claude/skills/assess-archetype-rust/` is the legacy Claude gap-filing workflow. It appends or deduplicates entries in this file and writes `.claude/plans/rust-engine-gaps-*.md` prompts for engine gap-closure planning.

Format and conventions mirror `qa/archetype-qa/engine-gaps.md` (Python-scoped). Gap titles are **capability-centric**, never card-centric.

Resolved reusable gap entries and group summaries are archived in [qa/resolved-gaps.md](../qa/resolved-gaps.md). Keep this document focused on open gaps, partial slices, and historical audit context that still informs open work.

Each entry lists the cards that surfaced it, but the entry itself describes a reusable engine primitive. If two cards need the same primitive, they share one entry — not two. When a new archetype audit surfaces the same primitive, the existing entry's `Discovered in:` and `Card(s):` lines accumulate — they do not fork into a new entry.

> **Canonical API signatures live here.** Fix-plans in `.claude/plans/rust-engine-gaps-*.md` should reference gap titles rather than restate signatures, to prevent divergence as the engine evolves.

## Severity legend

- **🔴 BLOCKING** — no faithful workaround exists; affected cards cannot be authored without this primitive.
- **🟡 PARTIAL** — a workaround exists with a specific fidelity cost. Sub-kinds marked inline: *ergonomics / sugar* (expressible today but awkward; scripts reach around `EffectContext`); *primitive-with-fidelity-cost* (modifier or keyword exists but its scope is too coarse for the card text's restriction).
- Pure verification / test-coverage items are **not** filed as gaps — see the "Deferred" section at the bottom of this file.

## Audit index

| Archetype | Audited | Cards | 🟢 Supported | 🟡 Partial | 🔴 Blocked |
|---|---|---|---|---|---|
| Medusamon | 2026-04-17 | — | — | — | — |
| DNA Omnimon | 2026-04-17; completed 2026-05-20; partial gaps closed 2026-05-22 | 64 | 64 | 0 | 0 |
| TS Olympos | 2026-04-18; representative unlock 2026-05-24 | 117 broad / 23 representative | 62 broad / 23 representative | 0 representative | 55 broad residuals |
| Rocks | 2026-04-18; refreshed 2026-04-28 | 47 | 0 | 0 | 47 |
| Dark Masters | 2026-04-18 | 58 | 0 | 0 | 58 |
| ST-23 BEATBREAK | 2026-05-17 | 15 | 2 | 4 | 9 |
| ST-24 DATA SQUAD | 2026-05-17 | 15 | 3 | 5 | 7 |
| Three Musketeers BeelStarmon (store-champs June-2026 slice) | 2026-07-02 | 12 | 2 | 4 | 6 |
| Galacticmon (store-champs June-2026 slice) | 2026-07-02 | 16 | 7 | 3 | 6 |
| Millenniummon (store-champs June-2026 slice) | 2026-07-02 | 29 (28 missing + BT19-075 re-audit) | 20 | 3 | 6 |
| ShineGreymon (store-champs June-2026 slice) | 2026-07-02 | 9 | 8 | 0 | 1 |
| Time Strangers support (store-champs June-2026 slice) | 2026-07-02 | 24 | 16 | 2 | 6 |
| Cross-deck staples + Alter-S (store-champs June-2026 slice) | 2026-07-02 | 13 | 10 | 2 | 1 |

### Rocks refresh notes (2026-04-28)

- **Assessment target:** `Rocks` / `RockClose` from `data/deck_library.json`, with 47 unique card IDs across the local archetype decklists. The most common core cards are `EX10-032`, `P-167`, `EX8-047`, `EX8-005`, `EX10-036`, `EX10-069`, `EX8-067`, `BT21-055`, `P-107`, `BT16-082`, `EX8-048`, `EX10-063`, `EX8-051`, `EX10-028`, `EX10-033`, `EX10-025`, `LM-031`, `P-169`, `P-039`, and `EX8-055`.
- **Current Rust DSL coverage:** only `BT14-009`, `BT16-082`, `EX7-074`, and `P-206` have YAML under `code/digimon-engine/cards/`; the main `EX8`/`EX10`/`EX11`/`P-167` shell is not authored yet. `BT16-082` is no longer a `G-ON-MOVE` no-op placeholder as of 2026-05-04; it now has native reveal/add/remainder/hatch DSL backed by `can_hatch`. `P-206` and `EX7-074` retain `G-PLAY-COST-LTE` and broader Option disposition gaps; `BT14-009` still depends on the bilateral player-scoped passive modifier shape documented below. `G-IGNORE-COLOR-MASK` is implemented in the Rust engine as of 2026-05-02.
- **Blocking primitives reaffirmed:** Rocks remains blocked primarily by `OnDigivolutionCardTrashed` coverage beyond the `Game::return_to_hand` source-disposition slice, granted `Collision`, option/Delay disposition outside the Group 3 replacement slice, and player-scoped passive modifiers. These are reusable engine capabilities, not card-local implementation chores.
- **Updated 2026-04-29:** `Game::return_to_hand` now fires `OnDigivolutionCardTrashed` with `event_card` / `event_source_card` set to the trashed below-top source and `event_host_card` set to the former host top card. `event_host_permanent()` validates the stored handle against that card before exposing it, so removed-stack handles cannot alias shifted battle-area permanents. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source source_trash_host_context_does_not_alias_shifted_permanent` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source`. At that point, remaining source-trash blockers were `return_to_deck` / de-digivolve / Armor Purge / Fragment / Digi-Burst paths, and host-trait DSL predicates beyond `event_card`; later updates below narrow these.
- **Updated 2026-04-29 (Group 2):** Cross-permanent source selection is now a first-class pending selection and DSL binding path. `select_own_sources` covers exact-N and up-to-N source picks across own battle-area stacks, exposes PASS only after the minimum count, binds stable source refs, and `trash_selected_sources` trashes the chosen refs. Covered by `source_multi::exact_two_sources_can_be_selected_across_own_battle_area`, `source_multi::up_to_sources_enables_pass_only_after_minimum_is_met`, `source_multi_mask_only_exposes_selecting_players_pending_actions`, `select_own_sources_binds_source_refs_for_trashing`, and `empty_select_own_sources_runs_outer_tail_synchronously`.
- **Updated 2026-05-07 (Digi-Burst producer slice):** `select_own_sources` accepts `target: <binding-ref>`, allowing exact-N source costs to be restricted to the activating/source permanent instead of every own stack. `BT4-072` now proves an inline `<Digi-Burst 1>` authoring path that exposes only the carrier's own source action IDs, trashes the selected source through `trash_selected_sources`, and resolves the printed +2000 DP target choice. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_sources`.
- **Updated 2026-05-07 (reusable Digi-Burst DSL):** `digi_burst: { count: N, then: [...] }` now lowers to the same target-scoped exact-N source selection with `trash_selected_sources` inserted before the effect body. Printed keyword parsing also carries `Keyword::DigiBurst(N)` for card data. BT4-072 now uses this wrapper rather than spelling the cost sequence by hand. Covered by `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_source_selection_steps` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing -- parser_digi_burst_parametric`.
- **Updated 2026-05-08:** `select_own_sources` now also accepts `from: <binding>` as a compatibility alias for `target: <binding-ref>` plus `filter: <predicate>` in YAML. The filter is evaluated against each candidate source card, and `from: source` / `target: source` restricts inherited effects to the current carrier stack. Coverage: `cargo test --manifest-path code/digimon-dsl/Cargo.toml --test parse_source_selection_steps select_own_sources_accepts_host_and_card_filter`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_own_sources_filters_cards_from_source_carrier_only`.
- **First regression to write:** start with `EX10-032` Proganomon or `P-167` Landramon. The test should install a player-visible source selection across all own Digimon stacks, trash exactly the chosen `[Mineral]`/`[Rock]` source, fire only that specific source card's inherited "when this card is trashed from digivolution cards" effect, and then resolve the printed follow-up.

## At a glance

Rows link to the detailed entry below. `#cards` is the Medusamon-archetype count — most primitives unblock many more cards archetype-wide (DNA Omnimon audit surfaces ~30 of the 32 entries below as further evidence; see per-entry `Card(s):` lines). `Key files` is the primary surface the fix touches.

| Gap | Severity | #cards | Key files |
|---|---|---|---|
| ~~[Same-effect DP modifier visibility in subsequent `dp_lte` selections](#same-effect-dp-modifier-visibility-in-subsequent-dp_lte-selections)~~ — RESOLVED 2026-05-24 | ✅ | — | `dsl_cards/predicate.rs` |
| [Selection: aggregate-sum residual sub-shapes (self-stack material / cost-time placement)](#selection-aggregate-sum-residual-sub-shapes) | 🟡 | 2+ | `effect_context.rs`, `action/` |
| [Selection: `select_any_permanent` curated helper + `select_dna_pair` plumbing audit](#selection-select_any_permanent-curated-helper--select_dna_pair-plumbing-audit) | 🟡 | 4+ | `effect_context.rs`, `dsl_cards/step/selections.rs` |
| ~~[`play_from_revealed_free` (EX8-050 Gogmamon)](#play_from_revealed_free-ex8-050-gogmamon)~~ — RESOLVED 2026-05-23 | ✅ | — | — |
| [`play_from_security_at(index)` (BT13-012 GeoGreymon, BT14-033 Patamon)](#play_from_security_atindex-bt13-012-geogreymon-bt14-033-patamon) | 🟡 | 2 | `effect_context.rs` |
| [Zone-manipulation: return-to-deck-top / self-return-as-cost](#zone-manipulation-return-to-deck-top--self-return-as-cost) | 🟡 | 4+ | `effect_context.rs`, `permanent.rs` |
| ~~[Zone-manipulation: reveal-top-N residual (`play_from_revealed_free`)](#zone-manipulation-reveal-top-n-residual-play_from_revealed_free)~~ — RESOLVED 2026-05-23 | ✅ | — | — |
| [Zone-manipulation: top-N security trash + face-up security flip/extraction](#zone-manipulation-top-n-security-trash--face-up-security-flipextraction) | 🟡 | 3+ | `effect_context.rs`, `combat.rs` |
| [Alt-digivolve with override-cost + ignore-reqs + face-down placement](#alt-digivolve-with-override-cost--ignore-reqs--face-down-placement) | 🟡 | 4+ | `effect_context.rs`, `permanent.rs`, `game.rs` |
| [`<Training>` keyword](#training-keyword) | 🔴 | 1 | `enums.rs`, `card_source.rs`, `effect_context.rs`, `action/` |
| [Dynamic DP scaling residual (non-aura temporary dynamic DP grants)](#dynamic-dp-scaling-residual-non-aura-temporary-dynamic-dp-grants) | 🟡 | 1 | `effect.rs`, `tensor.rs` |
| [Condition-gated modifier residual: filter-aura + `while_condition` lazy-filter rewrite](#condition-gated-modifier-residual-filter-aura--while_condition-lazy-filter-rewrite) | 🟡 | 1 | `modifiers.rs`, `effect.rs` |
| [Player-scoped modifier registry residual: bilateral `UntilLeaveField` delivery (BT14-009)](#player-scoped-modifier-registry-residual-bilateral-untilleavefield-delivery-bt14-009) | 🟡 | 1 | `modifiers.rs`, `enums.rs` |
| [Option card play flow residual: place-Option-in-battle-area + [Hand][Main] Plug-In flow](#option-card-play-flow-residual-place-option-in-battle-area--handmain-plug-in-flow) | 🟡 | 11 | `game.rs`, `effect.rs`, `effect_context.rs`, `action/` |
| [Standard Delay main-phase activation action](#standard-delay-main-phase-activation-action) | 🟡 | 3+ | `game_actions.rs`, `action/mask.rs`, `effect_context.rs` |
| [Trait-filter helpers on `CardSource` / `Permanent`](#trait-filter-helpers-on-cardsource--permanent) | 🟡 | pervasive | `card_source.rs`, `permanent.rs` |
| ~~[Digivolution-stack name overlay ("has all names of materials") (`G-DYNAMIC-NAME-ALIAS-FROM-STACK`)](#digivolution-stack-name-overlay-has-all-names-of-materials)~~ — RESOLVED 2026-05-22 | ✅ | — | — |
| ~~[Delay-on-attack-event dispatch (`<Delay>` body gated on an attack event) (`G-DSL-DELAY-ON-ATTACK-EVENT`)](#delay-on-attack-event-dispatch-delay-body-gated-on-an-attack-event)~~ — RESOLVED 2026-05-22 | ✅ | — | — |
| [Decode residual: EX10-061 Apocalymon batch + different-name source play DSL sugar](#decode-residual-ex10-061-apocalymon-batch--different-name-source-play-dsl-sugar) | 🟡 | 1 | `effect.rs` |
| [Ergonomics partials](#ergonomics-partials) | 🟡 | pervasive | `effect.rs`, `effect_context.rs` |
| [Grant Security A. ±N modifier — targeted typed sugar](#grant-security-a-n-modifier-to-a-targeted-permanent-parametric-securityattackchange) | 🟡 | 3+ | `effect_context.rs` |
| [Play / digivolve origin context flag — effect-spawned cleanup token half](#play--digivolve-origin-context-flag-if-played-by-effects-if-digivolved-by-this-effect) | 🟡 | 4+ | `effect.rs`, `effect_context.rs` |
| [Generic `pop_top_digivolution_source` for arbitrary re-routing (BT24-093)](#digivolution-stack-source-extraction-pop_top_source-from-named-permanent) | 🟡 | 1 | `effect_context.rs`, `permanent.rs` |
| [Conditional digivolve-target restriction (filter on candidate top-card name/trait/level/color)](#conditional-digivolve-target-restriction-filter-on-candidate-top-card-nametraitlevelcolor) | 🔴 | 7+ | `modifiers.rs`, `effect.rs` |
| ~~[Effect-spawned permanent with end-of-turn deletion rider](#effect-spawned-permanent-with-end-of-turn-deletion-rider-delete-the-digimon-this-effect-played)~~ — RESOLVED 2026-05-20 (Puppets substrate sweep, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~[Cast-time stack-construction for cost reduction (BT15-102 Apocalymon)](#cast-time-stack-construction-for-cost-reduction-place-n-differently-named-cards-from-battle-areatrash-under-the-played-card)~~ — RESOLVED 2026-07-05 (`cast_time_assembly` alt-path over the DigiXros transaction substrate) | ✅ | — | — |
| ~~[Cross-card effect re-firing — foreign-card source-card variant (BT15-102)](#cross-card-effect-re-firing--activate-a-foreign-cards-on-play-effect-attributed-to-the-source)~~ — RESOLVED 2026-07-05 (`EffectContext::activate_foreign_card_effect` + `refire_card_effect` DSL verb) | ✅ | — | — |
| [Reveal-zone overlay (declarative type/level synthesized while card is in deck or being revealed)](#reveal-zone-overlay-declarative-typelevel-synthesized-while-card-is-in-deck-or-being-revealed) | 🔴 | 1 | `effect.rs`, `card_source.rs` |
| [Effect-initiated play from face-up security stack (search-then-play-free)](#effect-initiated-play-from-face-up-security-stack-search-then-play-free) | 🔴 | 5+ | `effect_context.rs` |
| ~~Generic `.activation_cost(...)` builder hook for triggered abilities~~ — RESOLVED 2026-05-17 (Phase 2 Track B) | ✅ | — | — |
| ~~Once-per-turn enforcement for triggered effects (`G-OPT-TRIGGERED`)~~ — RESOLVED 2026-05-17 (Phase 2 Track C: diagnosed as already-closed; 23 stale `#[ignore]` annotations removed, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~OPT slot reset across turn cycle (`G-OPT-RESET-VIA-ATTACK-CYCLE`)~~ — RESOLVED 2026-05-17 (Phase 2 Track C: misdiagnosis; test-setup-only fix, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Inherited triggered-effect dispatch (`enqueue_from_permanent` digivolution-stack walk)~~ — RESOLVED 2026-05-17 (Phase 2 Track D: substrate completion + regression test + 18 tests un-ignored, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Standard Delay main-phase activation action (`PUPPETS-G009`)~~ — RESOLVED 2026-05-20 (Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~BeforePayCost cost-target predicate + sibling observer builder (`G-BEFORE-PAY-COST-DIGIVOLVE-TARGET` + `G-BEFORE-PAY-COST-GAIN-MEMORY`)~~ — RESOLVED 2026-05-17 (Phase 2 Track H, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~`play_from_hand_free` `bind_as` PermanentHandle output (`G-PLAY-FROM-HAND-FREE-BIND-AS`)~~ — RESOLVED 2026-05-17 (Phase 2 Track H, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Inherited aura `applies_to_opponent_security_dp` (`PUPPETS-G008` / `G-OPPONENT-SECURITY-DP-AURA`)~~ — RESOLVED 2026-05-17 (Phase 2 Track I, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Filtered breeding permanent target selection (`RK-G001`)~~ — RESOLVED 2026-05-17 (Phase 2 Track J PR 1, see `qa/resolved-gaps.md`) | ✅ | — | — |
| [Per-N-suspended scaling threshold residual (count-bound multi-select + formula downstream filter)](#per-n-suspended-scaling-threshold-for-deletion--damage-effects-count-bounded-multi-select-with-derived-threshold) | 🟡 | 1 | `effect_context.rs` |
| [Player-scope mass `CannotSuspend` aura on opponent (condition-gated)](#player-scope-mass-cannotsuspend-aura-on-opponent-condition-gated-and--or-stack-depth-filtered) | 🔴 | 2 | `modifiers.rs`, `effect.rs` |
| [Conditional security-in-stack trigger residual: start-of-turn / start-of-opponent-turn variants](#conditional-security-in-stack-trigger-security-end-of-opponents-turn--security-start-of-your-turn-etc) | 🟡 | 1 | `enums.rs`, `effect_queue.rs` |
| [Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`)](#declarative-aura--player-scoped-modifier-delivery-bilateral-untilleavefield) | 🟡 | 1 | `effect.rs`, `modifiers.rs` |
| [Global `OnOptionCardTrashed` observer residual: legacy Option trash paths](#global-onoptioncardtrashed-observer-timing) | 🟡 | 1 | `effect_queue.rs`, `game.rs` |
| [Plug-In re-link from battle area source zone residual](#plug-in-re-link-from-battle-area-source-zone) | 🟡 | 1 | `effect_context.rs` |
| [`ctx.move_from_breeding()` optional level-filtered prompt wrapper](#ctxmove_from_breeding-effectcontext-helper) | 🟡 | 1 | `effect_context.rs` |
| ~~[Costed self-digivolve stable source binding](#costed-self-digivolve-stable-source-binding)~~ — RESOLVED 2026-05-20 (Puppets substrate sweep, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~[Narrow opponent-effect protection for DP reduction and De-Digivolve](#narrow-opponent-effect-protection-for-dp-reduction-and-de-digivolve)~~ — RESOLVED 2026-05-20 (Puppets substrate sweep, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~[Effect play with played-Digimon On Play suppression (`PUPPETS-G030`)](#effect-play-with-played-digimon-on-play-suppression)~~ — RESOLVED 2026-05-20 (Puppets substrate sweep; `suppress_on_play` wired through `play_from_trash_free`, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Ally-played may-attack observer (`G-ALLY-PLAYED-MAY-ATTACK`)~~ — already-composable, no engine change | ✅ | — | RESOLVED 2026-05-20 (Phase 2 Track J Task S2.1) — see [qa/resolved-gaps.md](../qa/resolved-gaps.md#engine--dsl-gap-g-ally-played-may-attack--already-composable-2026-05-20-phase-2-track-j-task-s21) |
| ~~Union hand/trash name-excluded play (`G-UNION-HAND-TRASH-NAME-EXCLUSION`)~~ — `select_union_zone` lowering now applies its `filter`; new `name_not_shared_by_field_digimon` predicate leaf | ✅ | — | RESOLVED 2026-05-20 (Phase 2 Track J Task S2.2) — see [qa/resolved-gaps.md](../qa/resolved-gaps.md#engine--dsl-gap-g-union-hand-trash-name-exclusion--resolved-2026-05-20-phase-2-track-j-task-s22) |
| ~~Optional breeding-permanent selection decline path (`select_own_breeding_permanent optional: true`)~~ — PASS now declines optional breeding-target prompts without running the tail | ✅ | — | RESOLVED 2026-05-22 (`close-royal-knights-substrate-gaps`), covered by focused DSL/selection/card tests for BT13-110 and BT20-083 |
| ~~Event-bound played-Digimon keyword grants (`event_target` keyword package)~~ — BT23-072 grants Rush/Raid/Reboot/Blocker only to the triggering played Digimon | ✅ | — | RESOLVED 2026-05-22 (`close-royal-knights-substrate-gaps`), covered by `bt23_072_played_digimon_observer_*` |
| ~~End-of-attack mandatory self-delete chain (EX4-074)~~ | ✅ | — | RESOLVED 2026-05-17 (Track I first-test confirmed existing primitives suffice) — see [qa/resolved-gaps.md](../qa/resolved-gaps.md#engine-gap-end-of-attack-mandatory-self-delete-chain-with-recovery-and-conditional-hatch--resolved-2026-05-17-track-i) |
| ~~Return a selected digivolution-stack source card to its owner's hand (`G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME`)~~ — RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Player-scoped one-shot future-digivolve cost reducer with a paid cost (`G-COST-REDUCE-ALLY-DIGIVOLVE`)~~ — RESOLVED 2026-05-21 (`bg-imperial-substrate-closeout`, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Xros Heart DigiXros transaction + Material Save substrate~~ — RESOLVED 2026-05-24 (`close-xros-heart-digixros-gaps`, see `qa/resolved-gaps.md`) | ✅ | — | `digixros.rs`, `game_actions.rs`, `keyword_effects.rs`, `digimon-dsl` |

**Group 5 contract note (2026-05-02):** Group 5 did not change ACTION_SPACE_SIZE or TENSOR_SIZE. New Link/Delay choices reuse existing pending-selection masks.

**Zephagamon prep note (2026-05-03):** Task 4 added an EX11-074/Vortexdramon readiness slice in `code/digimon-engine/cards/ex11/EX11-074.yaml` and `code/digimon-engine/tests/cards_behavioral/ex11/ex11_074.rs`. The slice confirms the rule boundary that an effect battle resolves DP battle and `EndOfBattle`, but is not an attack: even if the attacker has `<Piercing>`, the `battle:` step must not trigger Piercing security checks and must not leave `pending_attack` populated. Remaining Zephagamon-specific blockers are documented in `qa/dsl-vocab-gaps.md`: conditional "if this effect suspended your Digimon" branch/binding support for EX11-074, BT20-101 suspended-Digimon count / divide-by-2 / capped multi-select bottom-deck formula, EX11-035 formula DP cap for green Avian/Bird play, and EX11-062 conditional `VortexCanAttackPlayer` aura while the opponent has no unsuspended Digimon.

**Track J formula/result substrate slice (2026-05-10):** Formula-valued `play_cost_lte` is now wired for selection filters, including `binding_play_cost` for a previously selected card/permanent and `distinct_colors_count` for BT21-102's Tamer-color cap. The same formula-threshold shape now covers the existing level, DP, stack/material-count, memory, security-count, and general count aggregate predicate leaves. Runtime bindings also carry an append-only per-effect result log for result-bound predicates such as `effect_suspended_any_own_digimon` and `effect_returned_any_card`, and formulas can count suspended battle-area permanents through `suspended_count`. The validator rejects `binding_dp` / `binding_play_cost` formulas that reference bindings before their declaring step. This closes the BT15-096 / BT21-102 play-cost-threshold gap and activates BT15-096's six behavioral tests. Coverage: `cargo test --manifest-path code/digimon-dsl/Cargo.toml`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_102 -- --nocapture`. Remaining Track J work is card authoring/fixture expansion for the Zephagamon, TS Olympos, and BG Imperial cards that need these primitives in full production YAML.

**Validated 2026-05-14 (PR #470):** BT15-096 Supreme Connection! and BT21-102 Undine behavioral tests now ship as card-shaped proof that the Track J substrate landed correctly on real cards. No new substrate surfaced; the slice remains closed.

## Open gaps

### `place_self_as_delay_option` does not compose with the real Option-play disposal lifecycle  [G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH]
> RESOLVED 2026-06-16. Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#place_self_as_delay_option-does-not-compose-with-the-real-option-play-disposal-lifecycle-g-option-place-self-as-delay-on-play-path--resolved-2026-06-16). The engine now claims the in-flight Option from `pending_option` inside `place_self_as_delay_option_permanent` (`src/effect_context/action/lifecycle.rs`), so a Standard Option's `[Main]` body seats it as a Delay on the real `play_option_from_hand` path. Covered by `omnimon_ace::combo1_mega_knight_*` + DNA Omnimon Combo B on the real play path.

### Multi-target `per_selected { delete_permanent }` mis-targets after the first deletion  [G-PER-SELECTED-DELETE-INDEX-SHIFT]
- **Severity:** ✅ RESOLVED 2026-06-03 (ST-1 Gaia Red interaction tests, `/archetype-interaction-test-author`).
- **Card(s):** ST1-15 Giga Destroyer (`select_count_capped_multi { max: 2, filter dp_lte 4000 } → per_selected { delete_permanent }`); any `select_count_capped_multi`/`select_*_multi` binding consumed by a `per_selected` body that removes permanents.
- **Root cause:** `iteration.rs`'s `PerSelected` `PermanentList` arm iterated the selected permanents by their **positional** `PermanentHandle` (`{player, index}`). After the first `delete_permanent` shifted `battle_area` down, the next handle's stale index pointed at the wrong (now lower-indexed) permanent. With Biyomon(3000)@0, Dracomon(4000)@1, Birdramon(5000)@2 and both ≤4000 bodies selected, deleting slot 0 shifted Birdramon into slot 1, so the second iteration deleted Birdramon (>4000) and left Dracomon — `survivors == ["Dracomon"]`. The matching `ForEach` arm already snapshotted top-card `CardHandle` stable identity and re-resolved per iteration (G-FOR-EACH-DELETE-INDEX-SHIFT); the `select_count_capped_multi → per_selected` path was never covered by that fix because it flows through `PerSelected`, not `ForEach`.
- **Fix:** Mirror the `ForEach` machinery in the `PerSelected` `PermanentList` arm (`src/dsl_cards/step/iteration.rs`): snapshot each selected permanent's stable top-card `CardHandle` up front via `top_card_handle`, then `resolve_by_top_card` at the START of each iteration, skipping any that already left play. `CardList`/`SourceRefs` arms and the parked-iteration abort are unaffected.
- **Gate:** `tests/archetypes/st1.rs::giga_destroyer_deletes_only_le_4000_opponents_and_tai_does_not_widen_window` (survivors == ["Birdramon"], the two ≤4000 deleted). No `cards_behavioral st1` regression.

### DSL `grant_keyword: Retaliation` is unrecognized and never fires  [G-DSL-GRANT-RETALIATION]
- **Severity:** ✅ RESOLVED 2026-06-03 (ST-6 Venomous Violet interaction tests, `/archetype-interaction-test-author`).
- **Card(s):** ST6-12 VenomMyotismon ("[When Digivolving] Up to 2 of your Digimon gain ＜Retaliation＞ …"); any DSL card that grants ＜Retaliation＞ at runtime.
- **Root cause (two parts):** (1) `dsl_cards/modifier_map.rs::lookup_keyword` had no `"Retaliation"` arm, so `CompiledStep::GrantKeyword` early-returned and the keyword was never even registered (`has_keyword` false). (2) Even with registration, ＜Retaliation＞ is a TRIGGERED on-deletion keyword: `effects_for_card` only synthesizes keyword auto-effects from PRINTED + DECLARATIVE-REGISTRY grants, and OnDeletion handlers re-fetch effects POST-TRASH (rule 25), so a board-position synthesis would vanish by drain time. A runtime modifier grant therefore never fired the auto-effect.
- **Fix:** (1) Add `"Retaliation" => Keyword::Retaliation` to `lookup_keyword` and `"Retaliation"` to `digimon-dsl`'s `validator::KNOWN_KEYWORD_KEYS`. (2) In `EffectContext::grant_keyword` (`src/effect_context/mod.rs`), after granting the keyword modifier, route the granted keyword's plain-`process` triggered auto-effect (`keyword_to_auto_effect`) through the BOARD-INDEPENDENT granted-triggered store (`grant_triggered_effect`) with the grant's expiry. Passive keywords yield an empty `keyword_to_auto_effect` (skip); replacement-process keywords carry no plain `process` (skip).
- **Gate:** `tests/archetypes/st6.rs::venommyotismon_grants_retaliation_and_trades_in_battle` — `modifiers.has_keyword(trader, Keyword::Retaliation)` is true AND the in-battle trade fires (trader loses to the 9000 opp, is deleted, and its ＜Retaliation＞ deletes the opponent it battled). Lib parity + `--test dsl` green.

### Same-effect DP modifier visibility in subsequent `dp_lte` selections
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#same-effect-dp-modifier-visibility-in-subsequent-dp_lte-selections--2026-05-24).
> Remaining BT19-012 work is production YAML/card behavior coverage.

### Global `OnOpponentSecurityRemoved` observer timing
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-global-onopponentsecurityremoved-observer-timing--resolved-2026-05-15-prs-449-phase-1-track-a-2026-05-0605-08) by the 2026-05-15 hygiene sweep. Core dispatch closed Phase 1 + Track A; card-local authoring is card-shaped follow-up.

### Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-global-onanydigimonplayed--onanydeletion-observer-timings--resolved-2026-05-15-prs-449-451-472) by the 2026-05-15 hygiene sweep.

### Phase-granular turn timings
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-phase-granular-turn-timings-startofyourturn-startofyourmainphase-whenattacking-endofattack-endofbattle--resolved-2026-05-15-pr-449) by the 2026-05-15 hygiene sweep.

### Observer timings tied to specific events (`OnDigivolve` trait-filter, `OnSuspend`, `OnAttackTargetChange`, `[When Moving]`, `OnHatch`, `OnAllyAttack`/`OnOpponentAttack`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-observer-timings-tied-to-specific-events-ondigivolve-trait-filter-onsuspend-onattacktargetchange-when-moving-onhatch-onallyattackonopponentattack--resolved-2026-05-15-prs-449-450-451) by the 2026-05-15 hygiene sweep.

### `WhenWouldBeDeleted` / leave-field replacement-effect framework
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-whenwouldbedeleted--leave-field-replacement-effect-framework--resolved-2026-05-15-prs-449-track-b-phase-c--phase-d) by the 2026-05-15 hygiene sweep.


### Selection: aggregate-sum residual sub-shapes
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18)
- **Card(s):** Self-stack material multi-select (EX4-073-style with level filter); cost-time placement variants. Headline aggregate-sum / count-capped primitive closed by Group 2 / Phase 4 — see [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).
- **Effect text:** "Choose any number of … whose total DP adds up to N or less and delete them" (closed); residual sub-shapes are self-stack material multi-select with a level/trait filter and cost-time multi-pick placement under the played card.
- **What's missing:** A cost-time placement multi-pick that decides between battle-area and trash sources at cast time (couples with EX8-074-style aggregate threshold derivations and EX10-061 Apocalymon-style cast-time stack-construction). **Updated 2026-05-19 (Track J S1.2):** the count-capped source-stack multi-select sub-shape is CLOSED — the `select_materials` DSL step (lowering to `select_count_capped_multi` + `CountCappedZone::Material` + `DistinctByMode`) covers level/trait/name-uniqueness filters over a carrier's digivolution sources. Only the cast-time placement multi-pick remains.
- **Suggested API shape:** A cast-time multi-pick variant on the play hook (battle-area-vs-trash source selection at cast time).
- **Workaround:** For the cast-time placement form: sequence single-source `select_material` picks + closure-captured running threshold — fidelity-preserving but burns extra RL decision points.
- **Related:** "Per-N-suspended scaling threshold for deletion / damage effects (count-bounded multi-select with derived threshold)" entry — same family.

### Selection: ordered permutation (place N cards in any order)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-selection-ordered-permutation-place-n-cards-in-any-order--resolved-2026-05-15-phase-4) by the 2026-05-15 hygiene sweep. (Headline "Selection: multi-select with aggregate-sum" closed by Group 2 — residual sub-shapes consolidated above into "Selection: aggregate-sum residual sub-shapes".)

### Selection: `select_any_permanent` curated helper + `select_dna_pair` plumbing audit
- **Severity:** 🟡 PARTIAL (ergonomic)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT20-102 Omnimon (X Antibody) (cross-side target), EX9-013 BlitzGreymon (DNA-pair), BT20-016 Paildramon (DNA-pair), P-186 Gallantmon (cross-side target — any Digimon with 13000+ DP), BT23-096 Comet Hammer (cross-side); plus DNA-pair fixtures BT22-008/BT22-017/BT17-019/BT17-007 etc.
- **Effect text:** "Choose 1 of both players' Digimon" / "2 of your Digimon may DNA digivolve into [X] in the hand"
- **What's missing:** Phase 4 closed opponent-as-selecting-player (`as_selecting_player`) and union-zone (`select_union_zone`). `install_select_any_permanent` and `install_select_dna_pair` exist in `dsl_cards/step/selections.rs:957,1045` as DSL step installers. The audit (2026-05-14) flagged that the curated `EffectContext::select_any_permanent` helper and `select_dna_pair` may still lack first-class `EffectContext` plumbing beyond the DSL step. Verifying DSL→engine plumbing for both is the residual.
- **Suggested API shape:** Confirm or add `ctx.select_any_permanent(prompt, filter, callback)` and `ctx.select_dna_pair(hand_index, callback)` as `EffectContext` methods (not just DSL installers).
- **Workaround:** Two-step `select_effect_choice` decomposition gives the player two prompts where the card describes one.
- **Related:** Parity §4.6d-residual.

### `play_from_revealed_free` (EX8-050 Gogmamon)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine--dsl-gap-play_from_revealed_free-ex8-050-gogmamon--resolved-2026-05-23-complete-rocks-archetype) by `complete-rocks-archetype` task 10.6. `ctx.play_from_revealed_free(player, card)` and DSL `play_from_revealed_free: { of, card }` now consume a reveal-pool card and route it through the normal effect-initiated play pipeline.

### `play_from_security_at(index)` (BT13-012 GeoGreymon, BT14-033 Patamon)
- **Severity:** 🟡 PARTIAL — sub-shape spun off from "Zone-manipulation: play-from-hand / trash without paying cost" headline closure (2026-05-15)
- **Discovered in:** DNA Omnimon (2026-04-17); TS Olympos (2026-04-18)
- **Card(s):** BT13-012 GeoGreymon (search then play from security stack), BT14-033 Patamon (search then play from security stack)
- **Effect text:** Play a chosen card from your own security stack without paying cost, distinct from `play_from_security()` which only consumes `pending_security` during attack-time checks.
- **What's missing:** A `play_from_security_at(player, security_index) -> Option<PermanentHandle>` curated helper that removes the chosen indexed security card (not the top of `pending_security`), instantiates it as a battle-area permanent firing OnPlay, fires `OnLoseSecurity` for that index, and leaves remaining security positions intact.
- **Suggested API shape:** `ctx.play_from_security_at(player, index) -> Option<PermanentHandle>`. Couples with the "Search-own-security-stack primitive" closure for the upstream selection.
- **Workaround:** None faithful.
- **Related:** "Search-own-security-stack primitive" (closed — see [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-search-own-security-stack-primitive-reveal-full-stack--select-by-filter--resolved-2026-05-15-track-e-2026-05-09)); "Effect-initiated digivolve from security stack" (closed — sibling primitive).

### Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-zone-manipulation-play-from-hand--trash-without-paying-cost--cost-override--resolved-2026-05-15-phase-2-pr-track-a-2026-05-08) by the 2026-05-15 hygiene sweep. Headline primitive closed by Phase 2 + Track A; residual sub-shapes `play_from_revealed_free` and `play_from_security_at(index)` are spun off as their own entries above.

### Zone-manipulation: effect-initiated digivolve (free / reduced / with trait filter / ignore requirements / DNA / Blast / detect-DNA-origin)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-zone-manipulation-effect-initiated-digivolve-free--reduced--with-trait-filter--ignore-requirements--dna--blast--detect-dna-origin--resolved-2026-05-15-phase-2-track-ac-2026-05-0809) by the 2026-05-15 hygiene sweep. Headline primitive closed by Phase 2 + Track A/C; BT17-095-style "DNA digivolve with field+hand material pair" is spun off as a narrow card-shape gap if it remains blocking.

### Zone-manipulation: return-to-deck-top / self-return-as-cost
- **Severity:** 🟡 PARTIAL (residual — headline return-to-hand / return-to-deck / bounce-self closed by Phase 2 + Track E; relocated bodies in [`qa/resolved-gaps.md`](../qa/resolved-gaps.md))
- **Discovered in:** DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** LM-031 Black Scramble / LM-032 Purple Scramble (return-from-trash-to-deck-TOP), EX10-074 Beelzemon (return 2 trash to deck top), BT22-089 Mirei Mikagura / BT21-102 Tai Kamiya / BT22-094 Yuugo Kamishiro / BT17-093 Tai Kamiya & Kari Kamiya (self-return to deck bottom as activation cost — needs `.pay_cost_return_self_to_deck_bottom()` builder hook)
- **Effect text:** "Return 2 of your trash to the top of the deck." / "By returning this Tamer to the bottom of the deck, ..."
- **What's missing:** `return_trash_to_deck(end=DeckEnd::Top)` variant and a `.pay_cost_return_self_to_deck_bottom()` builder hook for triggered abilities. Cross-permanent inherited Tamer closure-valued cost-delta interactions also remain.
- **Suggested API shape:** Extend `EffectContext::return_to_deck` and `return_trash_to_deck` to honor `DeckEnd::Top`; add a `.pay_cost_return_self_to_deck_bottom()` builder hook usable from triggered ability bodies.
- **Workaround:** None faithful.
- **Related:** "Generic `.activation_cost(...)` builder hook for triggered abilities" (sibling).

### Zone-manipulation: reveal-top-N residual (`play_from_revealed_free`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine--dsl-gap-play_from_revealed_free-ex8-050-gogmamon--resolved-2026-05-23-complete-rocks-archetype) by `complete-rocks-archetype` task 10.6.

### Zone-manipulation: top-N security trash + face-up security extraction
- **Severity:** 🟡 PARTIAL (residual — headline security-stack operations closed by Phase 2 + Track A/E)
- **Discovered in:** DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** EX4-073 Omnimon Alter-B (trash top 2 opp security — multi-N variant), EX10-061 Apocalymon (security-card extraction with face-up filter)
- **Effect text:** "trash top 2 opp security" / "place 1 of each face-up [Dark Masters] trait card with different names from your security stack under this card"
- **What's missing:** Headline security primitives shipped — `place_on_security` (Top/Bottom/Random), `trash_top_security` (single-card), `add_top_security_to_hand`, `recover_from_deck`, `place_self_at_security`, `place_self_option_at_security`, `security_place_stacked_card`, `security_place_top_stacked_card`, `place_permanent_on_security`, `search_own_security_stack`, `flip_security_face_up`, and attacker-side `on_check_face_up_security` dispatch. Residual: a multi-N `trash_top_security(player, N)` form (today's helper trashes exactly 1) and face-up security extraction with filter.
- **Suggested API shape:** Generalize `trash_top_security(player, count)` to handle N>1; add `extract_face_up_security(filter, callback)`.
- **Workaround:** Loop single-card `trash_top_security` for the multi-N case where order is irrelevant; face-up extraction has no faithful workaround.
- **Related:** Closed core in [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Zone-manipulation: security stack operations (trash top, place top/bottom, trash N, Recovery +N, shuffle security)
> Core security-stack primitives moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) by the 2026-05-15 hygiene sweep. Residual top-N security trash + face-up extraction tracked above as "[Zone-manipulation: top-N security trash + face-up security extraction](#zone-manipulation-top-n-security-trash--face-up-security-extraction)".

### Token creation + `CardKind::Token` + Petrification Token definition
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-token-creation--cardkindtoken--petrification-token-definition--resolved-2026-05-15-phase-10) by the 2026-05-15 hygiene sweep.

### Place card at a specific stack position (bottom-source / under another permanent) + alt-digivolve + stack reorder
> Core `place_as_bottom_source` primitive closed by Phase 2 (see [`qa/resolved-gaps.md`](../qa/resolved-gaps.md)). Residual alt-digivolve sub-shapes tracked as "[Alt-digivolve with override-cost + ignore-reqs + face-down placement](#alt-digivolve-with-override-cost--ignore-reqs--face-down-placement)" below.

### Alt-digivolve with override-cost + ignore-reqs + face-down placement
- **Severity:** 🟡 PARTIAL (residual — `place_as_bottom_source` core closed by Phase 2)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-016 Lamiamon (alt-digivolve variant); BT23-008 Greymon / BT23-018 Garurumon (`ctx.move_source_to_bottom`); EX10-032 Proganomon (alt-digi at cost 3 ignoring reqs); EX9-068 Analogman (face-down placement on a chosen ally Digimon); BT15-102 Apocalymon (level-filtered trash-source variant); EX2-007 Mother D-Reaper (extends primitive to from-permanent source where source's whole stack must move).
- **Effect text:** "alt-digivolve at cost N ignoring requirements" / "move top stacked card to own bottom source as activation cost" / "place 1 card face down as any of those Digimon's bottom digivolution card"
- **What's missing:** Alt-digivolve helper with override-cost + ignore-requirements flag; `move_source_to_bottom` stack reorder; `face_down: bool` axis on `place_as_bottom_source`; `place_as_top_source` sugar.
- **Suggested API shape:** `ctx.alt_digivolve(target, source, cost_override, ignore_reqs)`; `ctx.move_source_to_bottom(target, source_index)`; extend `place_as_bottom_source` with `face_down: bool`; add `place_as_top_source`.
- **Workaround:** None faithful for the face-down and alt-digivolve cases.
- **Related:** Closed core in [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Native printed keyword parsing (Rush, Raid, Piercing, Blocker, Reboot, Jamming, Blitz, Vortex, Alliance, Security A.±N, Fragment, Save, Collision, Retaliation)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-native-printed-keyword-parsing-rush-raid-piercing-blocker-reboot-jamming-blitz-vortex-alliance-security-an-fragment-save-collision-retaliation--resolved-2026-05-15-prs-phase-3--457-track-g--group-6) by the 2026-05-15 hygiene sweep.

### `<Progress>` keyword + `ImmunityToOpponentEffects` modifier
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-progress-keyword--immunitytoopponenteffects-modifier--resolved-2026-05-15-group-6--track-g) by the 2026-05-15 hygiene sweep.

### `<Armor Purge>` keyword (leave-field replacement variant)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-armor-purge-keyword-leave-field-replacement-variant--resolved-2026-05-15-phase-d-2026-04-25-track-b-2026-05-08) by the 2026-05-15 hygiene sweep.

### `<Training>` keyword
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Medusamon (2026-04-17)
- **Card(s):** EX9-008 Biyomon
- **Effect text:** "`<Training>` (In the main phase, by suspending this Digimon, place your deck's top card face down as this Digimon's bottom digivolution card. This effect can also activate in the breeding area.)"
- **What's missing:** (a) No `Keyword::Training` variant. (b) No primitive to move top-of-deck onto a permanent's `card_sources` at bottom position. (c) No `face_down: bool` flag on `CardSource` (with hidden-info tensor implications). (d) `[Main]` activation mask doesn't extend to breeding-area permanents.
- **Suggested API shape:** `Keyword::Training` + `ctx.push_deck_top_under_self(face_down: bool)` + `CardSource::face_down` field (zero-out data_index in observation tensors) + extend `MainOnField` activation to breeding-area when effect keyword is Training.
- **Workaround:** None — BLOCKED.
- **Related:** None.

**Updated 2026-05-02 (Group 5 Task 6):** Training Option permanents now carry explicit `trained: Option<TrainingBinding>` scope. Binding is by the specific Training permanent handle and records the intended carrier's top `CardHandle`, so duplicate Training copies can bind distinct carriers and stale `PermanentHandle` indices are ignored if they no longer point at the same physical top card. The sideways `.inherited()` scan and queued-effect liveness check only contribute a bound Training effect to its validated carrier (`None` remains the existing unbound compatibility path). This resolves the over-broad owner-wide Training fan-out slice for bound Training effects. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- training_sideways_effect_applies_only_to_its_intended_trained_permanent training_bound_to_removed_permanent_does_not_apply_to_reused_index duplicate_training_copies_bind_to_distinct_carriers_by_permanent_handle queued_training_effect_revalidates_bound_carrier_before_resolution`, plus the existing `training_parks_alongside_breeding training_trashes_on_breeding_promotion` flow checks.

### `<Delay>` keyword + placement-turn gating for Option cards
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-delay-keyword--placement-turn-gating-for-option-cards--resolved-2026-05-15-group-5-2026-05-02) by the 2026-05-15 hygiene sweep.

### Standard Delay main-phase activation action — RESOLVED 2026-05-20 (Puppets substrate sweep, branch `claude/stoic-moser-0ef79e`)
- **Status:** Closed. PUPPETS-G009 closure shipped in the Puppets substrate sweep (commits `44ce72a4` + `9afdfdb7`, 2026-05-20, branch `claude/stoic-moser-0ef79e`). The earlier Track I entry (commit `26e27ccc`, 2026-05-17) was optimistic — `DelayTrigger::MainPhaseActivated` and the main-phase activation action mask path did not actually exist before this sweep. Standard `<Delay>` Options on the field now expose a `[Main]` activation action through the normal main-phase action mask after the placing turn — the action trashes the Option as cost, then runs the stored Delay body. Pass/decline leaves the Option in the battle area for later legal activation. No `ACTION_SPACE_SIZE` change (Working Rule §1). Full closure details in `qa/resolved-gaps.md` § "Engine Gap: Standard Delay main-phase activation action (PUPPETS-G009)".
- **Severity (legacy):** 🟡 PARTIAL
- **Discovered in:** Puppets (2026-05-03 batch implementation)
- **Card(s):** P-037 Yellow Memory Boost!, P-105 Physical Training, LM-035 Physical Training, LM-037 Yellow Scramble, LM-054 Treadmill Training; also standard Memory Boost/Training/Scramble cards whose `<Delay>` text is activated by the controller during a later main phase.
- **Effect text:** "`<Delay>` (By trashing this card after the placing turn, activate the effect below.)"
- **What was missing (legacy):** The Group 5 Delay lifecycle supports persistent delayed Options, placement-turn gating, start/end/event drains, and replacement-aware self-trash costs. Standard main-phase Delay cards were still modeled in DSL/YAML as scheduled automatic effects such as `end_of_your_next_turn`, which hid the player's later `[Main]` decision to activate or decline the Delay effect after the placing turn.
- **Related:** `PUPPETS-G009` in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`; Option card play flow; Group 5 Delay lifecycle.

### Raid target-switch interrupt (scripting-surface, not mask-only) + effect-driven attack redirect
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-raid-target-switch-interrupt-scripting-surface-not-mask-only--effect-driven-attack-redirect--resolved-2026-05-15-track-d-2026-05-08) by the 2026-05-15 hygiene sweep.

### De-Digivolve N primitive (single + mass)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-de-digivolve-n-primitive-single--mass--resolved-2026-05-15-phase-10) by the 2026-05-15 hygiene sweep.

### Ace Overflow: inherited memory penalty on zone-change from field / under-card
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-ace-overflow-inherited-memory-penalty-on-zone-change-from-field--under-card--resolved-2026-05-15-group-8-2026-05-02) by the 2026-05-15 hygiene sweep.

### Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated + suspend/self-return as cost)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-dynamic-cost-reduction-at-beforepaycost-closure-valued--selection-gated--suspendself-return-as-cost--resolved-2026-05-15-group-3) by the 2026-05-15 hygiene sweep.

### Dynamic DP scaling modifier (per-stack-depth / per-opponent-board / per-color)
- **Severity:** 🟡 PARTIAL — narrowed title "Non-aura temporary dynamic DP grants" (audit 2026-05-15: continuous aura closed by Group 6 `dp_modifier_fn`. Residual: non-aura temporary dynamic DP grants such as opponent-board-scaling temporary +DP that the current snapshot semantics cannot faithfully track when the opponent's board changes mid-effect.)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17)
- **Card(s):** BT21-072 Arresterdramon: Superior Mode (per digivolution cards), BT24-017 Medusamon (per opponent Digimon) — DNA Omnimon adds: P-182 WarGreymon (+1000 DP per distinct color across own Digimon + Tamers)
- **Effect text:** "This Digimon gets +1000 DP for each of its digivolution cards." / "this Digimon gets +2000 DP for each of your opponent's Digimon until their turn ends."
- **Status:** RESOLVED by Group 6 for DSL formula-backed aura clauses. `kind: aura` accepts `dp_modifier_fn` and stores a live runtime formula; `effective_dp` and `source_dp_contribution` evaluate the formula at query/tensor time rather than materializing a stale modifier. `security_attack_fn` is covered by the same dynamic formula path at security-check resolution.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas --nocapture`; broader formula coverage remains `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- --nocapture`.
- **What's missing:** `EffectBuilder::dp_modifier(n)` is static. Per §13, modifier-registry DP grants are NOT summed into `source_dp_contribution` tensor slots — so `add_dp_modifier` also can't express tensor-correct dynamic DP.
- **Suggested API shape:** `.dp_modifier_fn(|&EffectReadContext| i16)` closure-valued variant evaluated at tensor-build time. Or `ModifierType::ChangeDpDynamic(Box<dyn Fn(...)>)` with tensor-aware summation.
- **What's still open:** Non-aura temporary dynamic DP grants and any formula selectors not expressible through the current DSL formula vocabulary should remain tracked separately. Static snapshot at cast time for temporary opponent-scaling variants still fails faithfulness when opponent board changes.
- **Workaround:** For continuous aura text, use `kind: aura` with `dp_modifier_fn`.
- **Related:** RUST_ENGINE_API §13.

### Condition-gated modifier entries + new `Expiry` variants
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; UntilCondition controller, while_condition aura slot, and keyword UntilCondition extension all shipped. Residual: filter-aura + `while_condition` lazy-filter rewrite — install-once on filter-aura misses future permanents joining the filter set; needs the lazy-filter rewrite from spec §2 for full coverage.)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18)
- **Card(s):** EX10-010 BlackWarGreymon (already listed) — DNA Omnimon adds: EX9-021 Omnimon Alter-S (source-scope condition — "opponent's effects only"), AD1-009 BlitzGreymon (same source-scope filter), AD1-014 MetalGarurumon ("can't suspend until their turn ends" — needs `Expiry::EndOfTargetsNextTurn` where the anchor is the MODIFIER TARGET's next turn end, not the source player's), EX1-068 Ice Wall! ("until the end of their next turn" — same `EndOfOpponentsNextTurn` / `EndOfTargetsNextTurn` need). Both the condition-closure and the new `Expiry` variants are prerequisites for `ModifierEntry` to faithfully represent these clauses — Rocks adds: BT9-103 Kongou (`CannotAttackPlayer` gated on play cost ≤7 with `EndOfOpponentsTurn`), BT4-072 Gogmamon (`Expiry::EndOfOpponentsNextTurn` on DP buff), ST22-11 Defense Plug-In F (`Expiry::EndOfOpponentsNextTurn` on Reboot + DP grant), BT18-064 Mercurymon (source-scoped return immunity with `Expiry::EndOfOpponentsTurn` — see new gap below for source-scoped return immunity variant), BT23-059 Justimon: Blitz Arm (turn-scoped `ImmunityToOpponentEffects`), EX8-055 Pyramidimon ("until your turn ends" DP + Security A. +1 grants — `Expiry::EndOfTurn` already present, keep documented)
- **Effect text:** "While your opponent has a Digimon with 13000 DP or more, your opponent's Digimon's effects don't affect this Digimon, and it gets +3000 DP."
- **What's missing:** `ModifierEntry` has no condition closure (parity §4.7x). Can't express "active only while opp has ≥13k DP Digimon" without an observer for arbitrary DP-threshold transitions.
- **Suggested API shape:** Add `condition: Option<Box<dyn Fn(&EffectReadContext) -> bool>>` to `ModifierEntry`; or passive `Effect::declarative(card).modifier_when(type, value, condition)` builder that the affect-resolution code consults per query.
- **Workaround:** Permanent grant over-applies when condition is false.
- **Related:** Parity §4.7x.
- **Updated 2026-05-06 (Track C taxonomy):** 🟡 PARTIAL — the `Expiry` enum now publishes `Expiry::UntilCondition`, `Expiry::OnceUsed(u32)`, and `Expiry::EndOfYourTurn`, with the corresponding DSL keys (`until_condition`, `end_of_your_turn`) round-tripping in `EXPIRY_TABLE` / `KNOWN_EXPIRY_KEYS`. `EndOfYourTurn` is fully enforced (mirror of `EndOfOpponentsTurn`). `UntilCondition` is reserved data only — entries persist through turn ends and the continuous controller wires up in a follow-up. `OnceUsed` likewise reserves the variant for consumer use. `ModifierEntry` already has a `replacement_condition` closure (Phase 7); `UntilCondition` re-uses that surface when the controller lands. Unit tests: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers -- end_of_your_turn until_condition_and_once_used --nocapture`.
- **Updated 2026-05-10 (UntilCondition controller):** 🟢 RESOLVED for runtime `UntilCondition` entries. `ModifierEntry` and `PlayerModifierEntry` now carry `until_condition` predicates, installs are ordered, the controller re-evaluates dirty entries after mutation-event drains, evicts false predicates FIFO, and never restores a removed entry on a later false -> true transition. Debug builds now reject only missing predicates; the release-build warn-and-persist path was removed. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat until_condition_controller`.
- **Updated 2026-05-10 (Track H §4 — DSL `while_condition` aura slot):** The `kind: aura` body now accepts a `while_condition: <predicate>` field that compiles the predicate onto the modifier's `until_condition` slot. Self-aura DP/security_attack/named-modifier grants are wired today: lower_aura emits `OnPlay` + `OnDigivolve` effects whose process closure installs the modifier with `Expiry::UntilCondition`. The controller (PR #458) handles eviction; the install-once-per-field-tenure shape preserves the asymmetric `false → true` non-restoration semantic. New typed surface for raw_rust card scripts: `EffectContext::add_modifier_with_until_condition(target, modifier, value, predicate_arc)` honors the `can_affect_permanent` guard. DCGO reference: `CardEffectCommons/KeyWordEffects/Vortex.cs` — DCGO consults `IVortexCanAttackPlayersEffect.CanUse(null)` lazily at attack-target time; we evict at mutation events. End behavior matches. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- while_condition`.
- **Updated 2026-05-10 (Track H Phase 4h — KeywordEntry `until_condition` extension):** `KeywordEntry` gains `until_condition: Option<UntilConditionFn>` and a globally-monotone `install_order: u64` field shared with `ModifierEntry`/`PlayerModifierEntry`. The UntilCondition controller's `until_condition_candidates` / `evaluate_until_condition` / `remove_until_condition_by_order` now walk keyword entries alongside modifier entries. New API: `EffectContext::grant_keyword_with_until_condition(target, keyword, predicate_arc)`. The `lower_aura` `while_condition` path now also handles keyword grants — ZEPH-G004-style "while X is true, gain <Vortex>" is authorable. Test: `group6_auras::while_condition_keyword_grant_lands_via_keyword_entry_until_condition`.
- **Remaining related work:** Filter-aura + `while_condition` lazy-filter shape (install-once on filter-aura would miss future permanents joining the filter set; needs the lazy-filter rewrite from spec §2 for full coverage). Bare schema usage with no predicate fails loudly in debug instead of silently becoming permanent.

### Player-scoped modifier registry (CannotPlayFromTrash, CannotPlayDigimonByEffect, OpponentCannotReduceDigivolveCost, IgnoreColorRequirement, MayAttackPlayerOnly, CannotReducePlayCost-bilateral, CannotAddSecurityByEffect)
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; nearly every listed variant is wired in enums.rs and consult sites are in place via Track C/D 2026-05-06/07/08/09. Residual: bilateral `UntilLeaveField` lifecycle for BT14-009 Gotsumon's player-aura delivery shape — see "Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`)" entry.)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT23-014 Gallantmon (CannotPlayFromTrash by effect), BT8-097 Crimson Blaze (CannotPlayDigimonByEffect), BT5-008 Gaossmon (opp cannot reduce digivolution costs), P-151 Digimon Liberator / EX7-074 Vortex Resonance / P-206 Digital Gate Open / ST22-08 Offensive Plug-In V (IgnoreColorRequirement aura) — DNA Omnimon adds: BT8-097 Crimson Blaze (also listed; `CannotPlayDigimonByEffect` specifically distinct from `CannotPlayFromHand` which is player-action-only), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure (IgnoreColorRequirement variants, several with condition closures). BT17-081 Tai Kamiya & Matt Ishida surfaces a sibling need — `ModifierType::MayAttackPlayerOnly` (grant attacks to a named permanent scoped to player-target only, unlike the existing `MayAttack` which mask-emits both Digimon and player targets) — Rocks adds: BT14-009 Gotsumon (bilateral `CannotPlayDigimonByEffect` emitted as a declarative aura from a field permanent with `UntilLeaveField` scope — new delivery shape: declarative aura → player-scoped modifier), ST13-08 Chikurimon (bilateral `CannotReducePlayCost` — new symmetric variant, counterpart to the existing `OpponentCannotReduceDigivolveCost`), BT9-103 Kongou (new `CannotAddSecurityByEffect` variant — opponent cannot add cards to security stack by effects), BT23-096 Comet Hammer (IgnoreColorRequirement while CS Digimon/Tamer on field), EX7-074 Vortex Resonance (IgnoreColorRequirement while LIBERATOR Digimon/Tamer), ST22-11 Defense Plug-In F (IgnoreColorRequirement while Tamer), P-206 Digital Gate Open (unconditional IgnoreColorRequirement), BT9-103 Kongou (`CannotAttackPlayer` filtered by play cost ≤7); `CannotPlayDigimonByEffect` specifically distinct from `CannotPlayFromHand` which is player-action-only), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure (IgnoreColorRequirement variants, several with condition closures). BT17-081 Tai Kamiya & Matt Ishida surfaces a sibling need — `ModifierType::MayAttackPlayerOnly` (grant attacks to a named permanent scoped to player-target only, unlike the existing `MayAttack` which mask-emits both Digimon and player targets) — Dark Masters adds: EX10-072 Spiral Mountain (IgnoreColorRequirement gated on board-state condition), BT19-093 Queen Device (IgnoreColorRequirement gated), EX8-026 MetalSeadramon (extends registry with `CannotSuspend` player-scope variant — see "Player-scope mass `CannotSuspend` aura on opponent"), BT16-026 Vikemon (same), BT9-103 Kongou (extends registry with `CannotAddSecurityByEffect` player-scope variant — "cards can't be added to security stacks by your opponent's effects")
- **Effect text:** "Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash." / "Your opponent can't play Digimon by effects until the end of their turn." / "[Opponent's Turn] Your opponent can't reduce digivolution costs." / "While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color requirements."
- **What's missing:** The player-scoped store and several player-scoped variants exist, but coverage is not complete. Remaining blockers include `CannotPlayFromTrash`, `OpponentCannotReduceDigivolveCost`, `MayAttackPlayerOnly`, bilateral/symmetric delivery shapes, live condition-gated player auras, and effect-vs-action-initiated play distinctions.
- **Status:** RESOLVED by Group 6 for `IgnoreColorRequirement` Option action-mask and decode/execution consumers.
- **Updated 2026-05-02 (Group 7):** BT8-097 no longer depends on this gap for `CannotPlayDigimonByEffect`; its YAML uses native `add_player_modifier` and is covered by `bt8_097_triggered_clauses_use_native_add_player_modifier_step`.
- **Passing command(s):** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test flood_gates -- group6_option_color --nocapture`.
- **Remaining related blocker:** the broader player-scoped registry remains open for `CannotPlayFromTrash`, `MayAttackPlayerOnly`, bilateral/symmetric delivery shapes, live condition-gated player auras, and effect-vs-action-initiated play distinctions.
- **Suggested API shape:** Continue extending `ModifierRegistry` player-scoped entries and query helpers for the remaining variants, with shared `Expiry` handling and condition-aware lookup where printed text requires it. Effect-play helpers and masks should consult the same legality helpers to keep decode and masks synchronized.
- **Workaround:** None — BLOCKED.
- **Related:** Parity §4.2b (IgnoreColorRequirement), §4.7x (context-aware modifier queries).
- **Updated 2026-05-06 (Track C taxonomy):** 🟡 PARTIAL — `ModifierType::MayAttackPlayerOnly` is now published as a player-scoped variant in `enums.rs` and exposed to DSL via `dsl_cards::modifier_map::lookup_modifier_type`. Combat-side enforcement (Track D's `combat::is_legal_attack_target`) is the remaining wire-up. Also published in this taxonomy round: `CannotMove`, `CannotSwitchAttackTarget`, `CannotBeRedirectedAsAttackTarget`, `CanAttackTargetDefendingPermanent`, `CannotAddMemory`, `CannotAddSecurity`, `ChangeEndTurnMinMemory`, `ImmuneFromDPMinus`, `ImmuneFromStackTrashing`, `DisableEffect` (with `disable_effect_timing` parameter on `ModifierEntry`), `TreatAsDigimon`, plus the DP/cost-scaling family (`ChangeCardDP`, `ChangeOriginDP`, `ChangeSAttack`, `ChangeLinkCost`, `ChangeLinkMax`, `ChangePermanentLevel`, `ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`, `ChangeCardLevelForAssembly`, `ChangeCardNamesForDigiXros`). Per-variant consult sites are documented in `docs/RUST_ENGINE_API.md` § "Modifier consult-site checklist". Bilateral `CannotReducePlayCost`, `CannotPlayFromTrash`, and `OpponentCannotReduceDigivolveCost` remain BLOCKING — the current `CannotReducePlayCost` variant doesn't yet carry a self-only / opponent-only / both selector. Passing command(s): `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib modifiers -- end_of_your_turn_player_scoped --nocapture`.
- **Updated 2026-05-07 (Track D consult sites):** ✅ `MayAttackPlayerOnly` is enforced in `combat::begin_attack_impl` — Digimon-target attacks return `Invalid` while player-target attacks remain legal. `CannotSwitchAttackTarget` (attacker-side) and `CannotBeRedirectedAsAttackTarget` (candidate-side) are wired into both `try_enter_block` and the unified `apply_attack_target_substitution` API. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_modifiers --nocapture` (5 tests).
- **Updated 2026-05-08 (Track B/C consult site):** ✅ `CannotMove` is enforced in `Game::move_from_breeding` — the gate covers both the player-action breeding→battle move and `move_from_breeding_by_effect` (which delegates to the same helper). Modifier installers should target the canonical breeding handle (`{ player, index: BREEDING_TARGET }`); permanent-scoped storage is keyed by handle, so other-side `CannotMove` modifiers do not leak across players. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cannot_move_breeding --nocapture` (4 tests).
- **Updated 2026-05-08 (Track C/D gain & protection gates):** ✅ Three permanent-scoped consult sites now read their respective Track C variants:
  - `CannotAddMemory` → `EffectContext::gain_memory` after the existing `CannotGainMemoryByEffect` / `CannotGainMemoryExceptFromTamers` checks. Scans the acting player's battle area for any permanent carrying the modifier.
  - `CannotAddSecurity` → `EffectContext::place_on_security` after the existing `CannotAddSecurityByEffect` check. Same scan shape.
  - `ImmuneFromStackTrashing` → `EffectContext::trash_top_source` reads the modifier on the host permanent before the stack-peel.
  Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test track_c_gain_gates --nocapture` (6 tests covering positive + per-player-isolation cases for each variant). The DP/identity-scaling family remains on the punch list (Track H aura territory).
- **Updated 2026-05-08 (Track D combat override):** ✅ `CanAttackTargetDefendingPermanent` is the affirmative inverse of `CannotAttackTarget` — when both modifiers are present on a target, the affirmative wins. Wired at every consult site that reads `CannotAttackTarget`: combat-side Raid retarget candidate filter (unsuspended + fallback passes), action decode validation, and three mask emission paths (standard, granted, per-attacker enumeration). Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test track_c_can_attack_override --nocapture` (3 tests covering baseline rejection + override + affirmative-alone no-op).
- **Updated 2026-05-08 (Track C/D DP protection):** ✅ `ImmuneFromDPMinus` is enforced in `Game::effective_dp` — when the target carries the modifier, negative `ChangeDp` entries are filtered out before the sum; positive `ChangeDp` and the dynamic-aura bonus path remain untouched. The `effect_immunity_filter` field on the entry is reserved for a future opponent-only / source-kind refinement. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test track_c_immune_dp_minus --nocapture` (4 tests covering baseline negative + filtering + positive-still-applies + per-permanent isolation). Remaining unwired Track C variants either need a `ModifierEntry` payload extension (`ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor` need string/discriminant payloads) or a multi-call-site identity-helper refactor (`TreatAsDigimon`, `ChangePermanentLevel`, `ChangeCardDP`/`ChangeOriginDP`/`ChangeSAttack`); these are next-batch work.
- **Updated 2026-05-09 (deferred Track C payload/identity wave):** 🟡 PARTIAL — `ModifierEntry` / `PlayerModifierEntry` now carry typed `ModifierPayload`; debug builds assert payload/type matches. Runtime consults are wired for `ChangeTraits`, `ChangeBaseCardName`, `ChangeBaseCardColor`, `ChangeCardNamesForDigiXros`, `TreatAsDigimon`, `ChangePermanentLevel`, `ChangeCardDP`, `ChangeOriginDP`, `ChangeSAttack`, `ChangeEndTurnMinMemory`, `ChangeLinkCost`, `ChangeLinkMax`, `CannotPlayFromTrash`, `CannotReducePlayCost` bilateral behavior, and `OpponentCannotReduceDigivolveCost`. `Permanent::synth_identity` is the central overlay helper for field identity, and attack/action-mask/Link/digivolve predicates route through it. `ChangeCardLevelForAssembly` remains storage/documentation only because cast-time assembly selection is not present yet. Rich YAML payload parsing for string/list payloads remains a DSL schema follow-up; scalar modifier slots and variant names are available. Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat track_c_deferred_modifiers -- --nocapture`.

### Option card play flow (resolve + trash vs. place-on-field; [Main]/[Security] activation) + Plug-In / Link mechanic + Security-effect return-to-hand / place-on-field
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; Group 4/5/7 closed Delay lifecycle, Link registration, OnOptionPlaced, transient Standard Option EOT replay, security-effect add-to-hand. Residual: place-Option-in-battle-area disposition, [Hand][Main] `<Blast Digivolve>` flow, Plug-In re-link from battle area — see standalone entries.)
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** P-103, LM-027, BT24-089, BT8-097, P-035, BT21-093, EX7-074, P-151, P-206, BT1-090 (Option cards); ST22-08 Offensive Plug-In V (Plug-In / Link mechanic) — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (Ace Option, Delay, security-effect "Then, add this card to the hand" — new disposition distinct from "trash" and "place on field"), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure / ST2-13 Hammer Spark / EX1-068 Ice Wall! (Option cards needing [Main] flow). ST20-15 surfaces a further sub-gap: security-effect "place this card in the battle area" disposition (OptionSecurity → battle-area permanent) — Rocks adds: EX10-069 Unique Emblem: Gravel Hearts (Delay + [Security] re-activates [Main]), LM-031 Black Scramble / LM-032 Purple Scramble (Delay + security "add this card to the hand" disposition), P-107 Defense Training / P-039 Black Memory Boost! (Delay + security "place this card in the battle area" disposition), EX8-070 Zofr Kabus ([Main] + security "delete lowest-play-cost opp"), BT9-103 Kongou ([Main] + security re-activates [Main]), P-206 Digital Gate Open ([Main] + place-on-field + Delay + security play-from-hand-or-trash + add-to-hand), EX7-074 Vortex Resonance ([Main] + security "return to hand"), BT23-096 Comet Hammer ([Main] + Delay + security "place in battle area"), ST22-11 Defense Plug-In F (Plug-In with plug-sideways-from-battle-area — NEW Plug-In sub-variant, see new gap below), BT23-059 Justimon: Blitz Arm (consumes "Option in battle area" as cost, presupposing Option-on-field persistence); ST22-08 Offensive Plug-In V (Plug-In / Link mechanic) — DNA Omnimon adds: BT17-095 Miraculous Mega Knight (Ace Option, Delay, security-effect "Then, add this card to the hand" — new disposition distinct from "trash" and "place on field"), LM-034 Wisteria Memory Boost! / BT22-099 Kuremi Detective Agency / BT23-096 Comet Hammer / ST20-15 Island of Adventure / ST2-13 Hammer Spark / EX1-068 Ice Wall! (Option cards needing [Main] flow). ST20-15 surfaces a further sub-gap: security-effect "place this card in the battle area" disposition (OptionSecurity → battle-area permanent) — Dark Masters adds: EX10-072 Spiral Mountain (`[Main] place this card in the battle area` + `[Security] add this card to the hand` after free-play), ST6-15 Death Claw (Option [Main] + [Security]), BT8-097 Crimson Blaze already listed, BT19-093 Queen Device (`[Main]` flow + `When this card is trashed from the battle area` observer + Security disposition adds-to-hand), BT23-007 Musclemon (inherited Plug-In `[Link]` mechanic), BT9-103 Kongou (Option `[Main]` + `[Security]`), EX2-067 Fire Ball (Option `[Main]` + `[Security]`)
- **Chaos Control evidence (2026-04-28):** EX3-072 Megiddo Flame, BT20-096 Black Sabbath, BT21-100 The Digimon I Designed, BT7-107 Calling From the Darkness, ST10-15 Darkness Wave, P-205 Insane Synthetic Monster. BT21-100 and P-205 also need "place this card in the battle area" + `<Delay>` persistence.
- **Effect text:** All [Main] top-line clauses of Option cards; all "[Main] You may link this card to 1 of your Digimon without paying the cost" of Plug-In cards.
- **What's missing:** Per RUST_ENGINE_API §9, "Option cards have no play flow yet (they hit the field as a permanent like Digimon)." Need: (a) play path that fires `OptionMain` then trashes the card (or places it on field when `<Delay>`); (b) security effects that re-activate the card's [Main]; (c) `ctx.place_self_in_battle_area()` / `ctx.trash_self_from_option()` / `ctx.activate_own_main_effects()` helpers; (d) Plug-In / Link mechanic: `Permanent.linked_cards: Vec<CardSource>` storage exists but no `ctx.link_card_to_permanent(card, target)` API, no play-flow for Plug-In card kind, no link-cost evaluation, no interaction between linked Plug-Ins and their carrier.
- **Updated 2026-05-01 (Group 4 narrow slice):** Security-effect "Then, add this card to the hand" is now supported for the currently resolving security Option via `EffectContext::add_pending_security_to_hand()` and DSL `add_this_option_to_hand: {}`. This consumes `Game.pending_security` and moves the revealed card to the defender/controller hand before dispose can trash it. Covered by `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand` and `lm_027_security_adds_card_to_hand_after_play`. Broader Option play flow, Delay, Plug-In/Link, and "place this card in the battle area" dispositions remain open here.
- **Updated 2026-05-02 (Group 7):** BT8-097's representative [Main]/[Security] Option flow is covered by native YAML behavioral tests and is no longer a raw-rust-retirement blocker. The broader Delay, Plug-In/Link, add-to-hand, and place-in-battle-area Option disposition cases remain open.
- **Suggested API shape:** `Game::play_option_from_hand` branched inside `play_from_hand` based on `CardKind::Option`. `EffectTiming::OptionMain` and `OptionSecurity` (variants exist; need dispatch). `ctx.place_self_in_battle_area()` + `ctx.activate_own_main_effects()`. For Plug-In: `ctx.link_from_hand_to_own_permanent(filter, callback)` + `EffectTiming::OnLink` / `WhileLinked` + `ModifierType::LinkRequirement` metadata on `CardData`.
- **Workaround:** None — BLOCKED. Option-card play flow is a foundational architectural gap; Plug-In is arguably its own sub-spec.
- **Related:** RUST_ENGINE_API §9.

**Updated 2026-04-29 (Task 4 / G-OPTION-PLACED-TIMING):** Delay-style Option placement through `Game::play_option_from_hand` now dispatches `EffectTiming::OnOptionPlaced` after the Option permanent is committed to the battle area, carrying `TriggerContext.event_permanent`, `event_card`, and `source_player` through `TriggerSource::OptionPlaced { player, permanent, card }`. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`. Transient Standard options, security-effect "place in battle area" disposition, Link, Training, and breeding-area observer fan-out remain open unless separately tested.

**Updated 2026-05-02 (Group 5 handoff):** Group 5 closes the shared Option Delay/Link state slices called out above: Delay start/end/event windows, replacement-aware Delay cost/resume, Link registration/action DSL, linked-card scope dispatch, inherited-security placement, `OnOptionPlaced` fan-out for Delay/Training/Link/inherited placement, Training binding scope, and transient Standard Option scheduled replay. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- delay link`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed`. Remaining card-level blockers are not generic Option state primitives: examples include `G-BINDING-DP-FORMULA`, `G-COLOR-MATCH-AGAINST-BOARD`, other predicate/filter gaps, and stale card YAML/tests that still need migration from raw placeholders.

**Updated 2026-06-06 (Shape-B Digimon-link audit — OpenSpec `implement-digilink-mechanic`):** The `[Link]` keyword has TWO card shapes sharing `Permanent.linked_cards`. **Shape A (Plug-In Options, ~10 cards)** is the built path above (option-play → `OptionSubtype::Link`). **Shape B (Appmon Link *Digimon*, ~34 cards: BT21-009 Gatchmon, BT25-052 Logimon, etc.)** is BLOCKED — a Digimon is linked onto a host via a player-activated `[Main]` ability (DCGO `CardEffectFactory.LinkEffect` + `AddSelfLinkConditionStaticEffect`), linkable from hand/trash/under-stack/another host's linked area/or a whole standing permanent (DCGO `ILinkCard.LinkCard`, root `Hand`/`Trash`/`DigivolutionCards`/`LinkedCards`/`None`). Missing: (a) self link-condition metadata on `kind: digimon` `CardData` (cost + host filter, distinct from the Option-scoped `link_requirement`); (b) a player-activated link initiation in the `FIELD_EFFECT` action range; (c) source-zone origins beyond just-played Option, esp. the standing-permanent absorb (`IPlacePermanentToLinkCards`). Two seams audited and confirmed via gating tests in `tests/option_flow/link_flow.rs`: **(D6, `d6_when_linked_via_on_link_fires_for_self_but_overfires_on_sibling`)** — `WhenLinked` is expressible as `OnLink` + `.linked()` (fires for the just-linked card) BUT over-fires on sibling attach because `OnLink` (`game_actions.rs:2765`) carries no just-linked-card identity → fix by adding that field to the OnLink trigger context + a `when_linked` self-filter, NO new timing; **(D7, `d7_linked_ess_keyword_grant_reaches_source_host_but_not_linked_host`)** — continuous ESS grants (keyword `Raid`, DP) from a linked card do NOT reach the host because `Game::has_keyword` (`game.rs:3406-3430`) scans `card_sources` only, never `linked_cards` → fix by extending that inherited-grant scan (and the DP computation) to also scan `linked_cards`. No parallel subsystem in either case.

**Updated 2026-06-07 (BT25 link-subsystem gaps — ALL 5 RESOLVED):** 🟢 The five BT25 link/Appmon gaps are implemented + tested (full `cards_behavioral` 3552 pass / 7 pre-existing DP failures; `option_flow` 124; `ACTION_SPACE_SIZE` unchanged at 2192). Design captured in `.claude/plans/rust-engine-gaps-bt25-link.md`.
- **Gap 1 — Link +N aura** (`G-ENGINE-AURA-GRANT-LINK-MAX`): `modifier_value` threaded through the aura path (see the dedicated note below). Cards BT25-060/075/102.
- **Gap 2 — `link_cards` DSL step** (`G-DSL-LINK-N-CARDS-PER-HOST`): authoring verb over `link_chosen_card_into_host` — zone-choice-first (DCGO ST22-12), `from`/`filter`/`to`/`count: exactly|up_to`/`cost`. Cards BT25-060/075/089. `code/digimon-engine/src/dsl_cards/step/link_cards.rs`.
- **Gap 5 — predicated WhenWouldLink cost-reduce**: `Game.pending_link_host` exposes the link target in the `WhenWouldLink` replacement window; `EffectContext::reduce_pending_link_cost` (one-shot, no modifier leak); DSL timing `when_would_link_to_this` + predicate `would_link_card_trait_any_of` + step `reduce_link_cost`. Optional/OPT via the replacement framework. Cards BT25-004/045.
- **Gap 3 — link-card-trash leave-replacement + Option self-as-link-source**: `WhenWouldLeaveBattleArea` cost `trash_own_link_card` (`Game::trash_specific_link_card`, which-card choice exposed) → cancel leave; `LinkCardSource::OptionInPlay` + `link_cards` `from: [self_option]` (Option attaches itself, lifted out of `pending_option` so dispose doesn't trash it). Cards BT25-066/073/101.
- **Gap 4 — App Fusion**: alt-play folded into the digivolve route funnel — eligibility mirrors DCGO `AddAppfusionMethod` (host top + a linked card match two distinct named conditions); resolution stacks the App-Fusion card on top + drains the host's `linked_cards` into the digivolution sources (`AddToSources(LinkedCard)`). DSL `kind: app_fusion` alt_path. Cards BT25-036/060. **Residual:** BT25-089's `[End of Your Turn]` effect-initiated hand-fuse needs a new host-then-hand-card selection step on top of this resolution core — designed, logged, not stubbed.

**Updated 2026-06-07 (G-ENGINE-AURA-GRANT-LINK-MAX — RESOLVED):** 🟢 A `kind: aura` with a named scalar `modifier:` (e.g. `ChangeLinkMax` for "Link +N", `ChangeLinkCost`) previously installed the modifier with a **hardcoded `0`** value at all three `lower_aura.rs` apply sites (self-aura, filter-target, player-scope) plus the `while_condition` path — so "Link +1" auras were silent no-ops. Added `AuraBody.modifier_value: Option<i32>` (DSL crate: `clause.rs` / `compile.rs` / `compiled.rs`) threaded through `lower_aura::{lower_all, lower, lower_self_while_condition}` and the `dsl_cards::mod.rs` caller; the scalar (default `0`) now reaches `add_declarative_modifier` / `add_declarative_player_modifier` / `add_modifier_with_until_condition`. `link_max_delta` reads the generic `entry.value` so a `ChangeLinkMax` aura with `modifier_value: N` raises a host's link-max from the default 5 to `5+N`. Cards: BT25-060 (self Link +1), BT25-075 / BT25-102 (aura granting Link +1 to [TS] Digimon). Tests `tests/option_flow/link_flow.rs::gap1_self_link_max_aura_grants_nonzero_delta`, `gap1_filter_target_link_max_aura_grants_nonzero_delta`. Full `cards_behavioral` regression unchanged (3548 pass / 7 pre-existing DP failures).

**Updated 2026-06-07 (remaining `[Link]` facets #6/#11, #9, #10 — engine substrate RESOLVED):** 🟢 The three follow-up link facets from the Shape-B residual list now have working, tested engine substrate (full link suite `tests/option_flow/link_flow.rs` green — 106 tests):

- **Facet #6/#11 — host-side `[When Linked]`** (a card gets linked *to this Digimon*; DCGO `CardEffectCommons.CanTriggerWhenLinked`). Already worked on the substrate: the `TriggerSource::Linked` context sets `event_permanent`/`event_host_permanent` = host, so a host effect timed `OnLink` with the self-filter `event_permanent() == source_permanent` fires once for the receiving host and not for a sibling host. Added a confirming Rust test (`host_side_when_linked_fires_for_receiving_host_only`) **and a DSL surface**: new timing `when: when_card_linked_to_this` (`Timing::WhenCardLinkedToThis` → `CompiledTiming::WhenCardLinkedToThis` → `OnLink` + the host self-filter, forced in `lower_triggered.rs`). DSL test `dsl_host_side_when_card_linked_to_this_fires_on_attach`. Distinct from the existing card-POV `when: when_linked` (`event_card == source_card`).
- **Facet #9 — link a *chosen* card from hand / trash / digivolution-sources** (DCGO `ILinkCard.LinkCard` with `root != None` → `Permanent.AddLinkCard`). New engine primitive `Game::link_chosen_card_into_host(host, card, LinkCardSource)` (`game_actions.rs`) + `EffectContext::link_chosen_card_into_host` wrapper + `enums::LinkCardSource { Hand(p) | Trash(p) | DigivolutionSource(perm) }`. Lifts the chosen card out of its zone (rejecting a stack *top* as an under-source), attaches it to the host's `linked_cards`, fires `OnLink` globally. Tests `facet9_link_chosen_card_from_hand_attaches_and_fires_onlink`, `facet9_link_chosen_card_from_digivolution_sources`. **Residual (logged, not blocking):** the *DSL authoring step* (`link_card_to_self: { from, filter, cost }` interactive zone→card selection) is a separable surface — the engine primitive is ready; logged to `qa/dsl-vocab-gaps.md`. Effect-driven links pay cost via the effect body (the standing-permanent `begin_digimon_link` path owns the interactive `WhenWouldLink` replacement window).
- **Facet #10 — `WhenWouldLink` cost reduction** (DCGO `ChangeLinkCostClass` / `LinkEffect.GrantedReduceLinkCostClass`). The **flat** reduction — which is what real cards actually use (e.g. ST22-12 passes `_ => true` for all of `cardSourceCondition`/`permanentCondition`/`rootCondition`) — already exists as `ModifierType::ChangeLinkCost` + `link_cost_delta_for_player`, DSL-authorable and consulted at all three link-cost sites (`option_legal_play_modes`, the option-link path, `commit_digimon_link`). Confirming test `facet10_change_link_cost_reduces_paid_link_cost`. The **predicated** (per source/host/root) reduction is speculative — no real card needs it (DCGO's real usage is unconditional during the link) — so it is deferred-until-needed rather than built as speculative machinery; noted in `qa/dsl-vocab-gaps.md`.

**Updated 2026-06-07 (G-LINK-INHERITED-ESS formula residual CLOSED):** 🟢 A link card's continuous **DP / Security-Attack formula and static-DP** Link-ESS now reaches its host. The two formula collectors `Game::static_dp_aura_bonus` and `Game::live_declarative_formula_sum` (`game.rs`) scanned `card_sources` only; each now also folds in every host's `linked_cards`, filtered to `.linked()` declarative effects, attributed to the host. Non-overlapping with the existing paths — keyword/DP *modifier* grants already flow via the `tick_declarative_effects` linked-card pass (§6.2) and `.linked()` *triggered* clauses via the `enqueue_from_permanent` linked scan; `static_dp_aura_bonus` deliberately skips `materializes_declarative_state` so the modifier-grant and formula paths don't double-count. Test `tests/option_flow/link_flow.rs::linked_card_dp_formula_and_static_ess_reach_host` (dynamic DP + static DP + Security-Attack formula). This closes the formula slice of the `[Link]` keyword subsystem's `G-LINK-INHERITED-ESS` (link-card inherited ESS → host) for the DSL `scope: linked` authoring convention used on this branch.

**Updated 2026-06-06 (Shape-B engine substrate LANDED — `implement-digilink-mechanic`):** 🟢 The Digimon-link engine mechanic is implemented and tested (full engine suite green apart from 7 unrelated pre-existing DP failures). Landed: (§3) `EffectTiming::LinkCondition` + `Effect::link_condition().link_host(cost, filter)` self-condition metadata, read by `Game::digimon_link_condition_targets`; (§6.1) `TriggerSource::Linked { player, host, card }` carrying the just-linked card so `WhenLinked` = `OnLink` + self-filter (`event_card == source_card`), no new timing; (§6.2) additive `linked_cards` pass in `tick_declarative_effects` so a linked card's `.linked()` declarative grants (keywords + DP) materialize onto the host; (§4) player-activated Link ability at `FIELD_EFFECT` sub-slot 3 (`FIELD_EFFECT_SLOT_FOR_LINK`, no `ACTION_SPACE_SIZE` change) → `activate_field_link` → host-selection → `begin_digimon_link` (fires `WhenWouldLink`, parks interactive replacements via `pending_digimon_link`) → `commit_digimon_link` (pays `link_cost_delta`-adjusted cost) → (§5) `absorb_standing_digimon_as_link` (canonical removal + `shift_handle_after_soft_remove`; per DCGO `DiscardEvoRoots` the under-stack is trashed and only the top card becomes a linked card — a flat `Vec<CardSource>` suffices because DCGO's `LinkedCards` is itself flat). Tests in `tests/option_flow/link_flow.rs` (`digimon_self_link_condition_*`, `digimon_link_activate_*`, `digimon_link_absorb_*`, `d6_*`, `d7_*`). **Residual (BLOCKED, deferred follow-up):** from-hand Digimon-link initiation and the rarer source origins (trash / under-stack / re-link-from-another-host) are not yet wired — DCGO `LinkEffect` allows `IsExistOnHand` + re-link, but the dominant BT21+ Appmon shape links a standing/digivolved Digimon onto a host (root `None`), which IS covered. The remaining **authoring layer** (DSL vocabulary for `link_condition` / `when_linked` / linked-ESS, and the acceptance cards BT21-009 Gatchmon etc.) is tracked in `openspec/changes/implement-digilink-mechanic` §7/§2.

### Scheduled end-of-turn effect queue (for transient Options)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-scheduled-end-of-turn-effect-queue-for-transient-options--resolved-2026-05-15-group-5-task-7) by the 2026-05-15 hygiene sweep.

### Effect re-firing / cross-timing self-trigger
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-re-firing--cross-timing-self-trigger--resolved-2026-05-15-task-9-2026-05-03-track-k-2026-05-10) by the 2026-05-15 hygiene sweep.

### Effect-initiated digivolve from non-hand source zones
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-initiated-digivolve-from-non-hand-source-zones--resolved-2026-05-15-group-4-2026-05-02) by the 2026-05-15 hygiene sweep.

### Force-follow-up-attack / "may attack without suspending" script helpers
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-force-follow-up-attack--may-attack-without-suspending-script-helpers--resolved-2026-05-15-2026-05-08) by the 2026-05-15 hygiene sweep.

### Trait-filter helpers on `CardSource` / `Permanent`
- **Severity:** 🟡 PARTIAL — *ergonomics / sugar*
- **Discovered in:** Medusamon (2026-04-17); DNA Omnimon (2026-04-17); Rocks (2026-04-18)
- **Card(s):** BT21-093, EX11-008, P-189, BT21-029, EX11-012 (LIBERATOR-typed), plus many search / filter effects — DNA Omnimon adds: BT22-005 Tsumemon ([Unidentified]/[CS]), BT22-089 Mirei Mikagura ([CS] / [Holy Beast] / [Angel] / [Archangel] / [Fallen Angel]), ST20-10 Agumon (cross-tamer trait union), BT22-099 Kuremi Detective Agency ([CS]), BT22-094 Yuugo Kamishiro ([CS]), BT22-084 Nokia Shiramine (named-trait aura), ST21-13 Matt Ishida & T.K. Takaishi ([ADVENTURE]) — Rocks adds (archetype-pervasive — [Mineral] + [Rock] + [LIBERATOR] pair + name-filter on [Close]/[Sunarizamon]/[Landramon]/[Proganomon] required on nearly every card): EX10-032, P-167, EX10-069, EX10-036, EX8-067, EX8-047, EX8-048, EX8-005, BT21-055, EX8-051, EX10-033, EX10-063, EX10-025, P-107, EX10-028, P-169, EX8-055, EX8-070, EX10-034, EX8-046, P-215 ([Ice-Snow]), EX7-049 ([Rock Dragon]/[Earth Dragon]/[Sky Dragon]), EX8-050, P-206, BT14-009, BT9-103, EX11-038, EX11-065, EX11-044, BT20-055, EX10-003, BT23-096 ([CS]), BT21-021 ([Xros Heart]/[Blue Flare]/[Hero]), EX7-074 ([LIBERATOR]), ST22-11 ([Plug-In])
- **Effect text:** "1 of your [Reptile] or [Dragonkin] trait Digimon …" / "1 card with the [LIBERATOR] trait …"
- **What's missing:** `CardData.type_eng` is present, but no ergonomic `CardSource::has_type(&str)` / `Permanent::has_any_type(&[&str])` accessor. Authors dip into `ctx.card_data()[idx].type_eng.contains(...)` directly — verbose, case-sensitivity bugs likely.
- **Suggested API shape:** `CardSource::has_type(card_data, trait_name)` + `Permanent::top_card_has_type(...)` / `has_any_type(...)`, case-insensitive.
- **Workaround:** Raw card_data scan — functional but API-convention-violating.
- **Related:** Parity §2.1b (same effect-listing / text-parsing class).

### Granted triggered ability — attach an `Effect` to another permanent
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-granted-triggered-ability--attach-an-effect-to-another-permanent--resolved-2026-05-15-pr-467-track-h) by the 2026-05-15 hygiene sweep.

### Named-target declarative aura (DP / keyword grants filtered by name/trait/level)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-named-target-declarative-aura-dp--keyword-grants-filtered-by-nametraitlevel--resolved-2026-05-15-group-6--track-h-9) by the 2026-05-15 hygiene sweep.

### Declarative aura sourced from security zone
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-declarative-aura-sourced-from-security-zone--resolved-2026-05-15-track-h-5-pr-467) by the 2026-05-15 hygiene sweep.

### Digivolution-stack name overlay ("has all names of materials")
- **Gap ID:** `G-DYNAMIC-NAME-ALIAS-FROM-STACK`
- **Severity:** ✅ RESOLVED
- **Status:** RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. BT17-102 now authors the `[All Turns]` clause with `identity.source_name_aliases: [{ level_lte: 3 }]`, name predicates consult synthesized permanent identity names, and `bt17_102_all_turns_aliases_low_level_material_names` is enabled and passing.
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-102 Greymon ("[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards.")
- **Effect text:** As above.
- **What closed:** A reusable source-name alias identity payload and modifier path now derives additional effective names from matching source cards and routes name predicates through the synthesized name set.
- **Related:** "Named-target declarative aura"; DSL/identity-layer face resolved in [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md) (`G-DYNAMIC-NAME-ALIAS-FROM-STACK`).

### Delay-on-attack-event dispatch (`<Delay>` body gated on an attack event)
- **Gap ID:** `G-DSL-DELAY-ON-ATTACK-EVENT`
- **Severity:** ✅ RESOLVED
- **Status:** RESOLVED 2026-05-22 by `close-dna-omnimon-partial-gaps`. BT23-096 now authors the `[Your Turn]` CS ally-attack Delay clause, and `bt23_096_your_turn_cs_attack_delay_dedigi4` plus the non-CS negative are enabled and passing.
- **Discovered in:** DNA Omnimon (2026-05-20)
- **Card(s):** BT23-096 Comet Hammer (`<Delay>` body gated on an ally-attack event).
- **Effect text:** `<Delay>` body that activates off an attack event.
- **What closed:** Delay lowering maps attack timings to `DelayTrigger::OnEvent`, combat dispatch carries attacker context into event-gated delayed options, and `attacker_trait_has` evaluates ordinary attack context as well as attack-target-change context.
- **Related:** "Standard Delay main-phase activation action" (RESOLVED Track I); DSL face resolved in [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md) (`G-DSL-DELAY-ON-ATTACK-EVENT`).

### Decode keyword (play from own digivolution stack without paying cost on non-battle leave)
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; BT22-015 Red/Black Decode, EX4-060 BlitzGreymon/CresGarurumon ladder, and EX9-021 End-of-Attack source-play all close. **Updated 2026-05-19 (Track J substrate S1.2):** the batch / different-name source-play DSL sugar is now CLOSED — see "What's closed" below. Residual is the native `Keyword::Decode` parsing sugar only.)
- **Discovered in:** DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** BT22-015 Omnimon ("＜Decode (Red/Black Lv.3)＞ — When this Digimon would leave the battle area other than in battle, you may play 1 Red or Black Level 3 Digimon card from its digivolution cards without paying the cost.") — Dark Masters adds: EX10-061 Apocalymon ("[On Play] [When Digivolving] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs").
- **Effect text:** As above.
- **Status:** Partially closed 2026-05-07 for BT22-015; narrowed again 2026-05-08 for EX4-060 and EX9-021. `select_material` now honors card predicates over the source stack, and `play_from_materials.source_index` may consume the selected `CardHandle` binding. `play_from_materials.bind_as` records a successful source play for follow-up gates. BT22-015's Red/Black Lv.3 and Blue/Yellow Lv.3 Decode clauses are faithful optional non-cancelling replacement subscribers. EX4-060's mandatory BlitzGreymon + CresGarurumon sequence is authored through sequential material selections, with its self-to-security tail handled by `place_permanent_on_security_and_handle_replacement`. EX9-021's End of Attack sequence is authored through the same source-play steps plus `binding_exists` and `place_permanent_on_security`. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_021`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- **What's closed (2026-05-19, Track J S1.2):** the batch / different-name source-play DSL sugar landed. A new `select_materials` DSL step picks *up to N* digivolution sources of a carrier permanent in ONE count-capped multi-pick, optionally constrained by `uniqueness: name` ("1 of each different name"). It lowers to `EffectContext::select_count_capped_multi` with `CountCappedZone::Material` + `DistinctByMode`. `play_from_materials` now consumes a `CardList` binding as a batch (each picked source becomes a fresh permanent), composing with the S1.1 `suppress_on_play` flag. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- **What's closed (2026-05-20, Track J S1.3):** the breeding-area carrier source-select residual. `select_material` / `select_materials` against a breeding-resident carrier (King Drasil) now install a real `pending_selection` whose action IDs use the appended `BREEDING_SOURCE_SELECT` sub-range (`2168..2192`, keyed by carrier owner; `ACTION_SPACE_SIZE` raised 2168→2192 — a deliberate version bump, existing trained RL models must be retrained). `material_zone_geometry` is the single battle-vs-breeding branch point. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_carrier`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier`.
- **What's missing:** Only the native `Keyword::Decode(Vec<Color>, u8)` printed-keyword parsing sugar + auto-emission in the leave-field replacement path. The batch / different-name source play is fully expressible today via `select_materials` + `play_from_materials`. **Updated 2026-07-05:** the "Cast-time stack-construction for cost reduction" entry is RESOLVED (`kind: cast_time_assembly`, landed for BT15-102) — but EX10-061 Apocalymon's cast-time half places from the **security stack**, and `DigiXrosMaterialZone` has no `Security` origin yet. EX10-061's residual is therefore: extend the transaction substrate with a `Security` material origin (face-up/face-down handling per its printed text) and allow `zones: [security]` on `cast_time_assembly` materials (the compile-time validator currently rejects non-hand/battle_area/trash zones by design, pointing here).
- **Suggested API shape:** `Keyword::Decode(Vec<Color>, u8)` + auto-emission in the leave-field replacement path.
- **Workaround:** None needed for batch / different-name source plays — `select_materials` covers them faithfully without auto-selection.
- **Related:** "WhenWouldBeDeleted / leave-field replacement-effect framework"; "Zone-manipulation: return-to-hand / return-to-deck / bounce self" (sibling for trash-stack-to-destination disposition); `select_source` is listed as 🔴-residual in RUST_PYTHON_PARITY §4.6d.

### Ergonomics partials

🟡 PARTIAL — *ergonomics / sugar*. These are expressible today but awkward; scripts currently reach around `EffectContext` or duplicate state. Filed to keep the authoring surface approachable as more cards land.

- **Per-permanent OPT activation recording** (BT23-008 Greymon, BT15-020 Gabumon, any `[Once Per Turn]` clause with compound sub-effects). `ctx.record_activation()` / `ctx.activation_count()` sugar over the existing `Permanent::record_activation` / `activation_count` methods, keyed by slot — flagged in RUST_ENGINE_API.md §13 as "nice follow-up".
- **Dual- / tri-timing composite clause builder** (ST20-11 WarGreymon, BT15-020 Gabumon — "[When Digivolving] [When Attacking] …"; Rocks adds: EX10-033 Pyramidimon, EX10-036 Magneticdramon, EX11-044 Pyramidimon (triple), EX7-049 Metallicdramon, EX8-055 Pyramidimon, EX10-034 Blastmon, EX10-032 Proganomon, P-215 Icemon ([When Moving][On Play][When Digivolving] tri), EX11-038 Sunarizamon ([When Moving][On Play]), EX10-028 Landramon ([On Play][When Digivolving]), EX8-070 Zofr Kabus). `EffectBuilder::on_timings(&[EffectTiming])` that stamps out multiple `Effect` records sharing an `Arc`'d process closure, avoiding manual closure duplication. Rocks makes this pervasive — likely should be 🔴 promoted.timing composite clause builder** (ST20-11 WarGreymon, BT15-020 Gabumon — "[When Digivolving] [When Attacking] …"). `EffectBuilder::on_timings(&[EffectTiming])` that stamps out multiple `Effect` records sharing an `Arc`'d process closure, avoiding manual closure duplication.
- **Aggregate filter helpers** (BT22-013 lowest DP, BT22-026 lowest level, AD1-012 lowest level, ST20-11 lowest DP, EX10-010 Raid highest DP; Rocks adds: EX8-070 Zofr Kabus (lowest play cost in security context), EX11-044 Pyramidimon (highest play cost Digimon OR Tamer — cross-kind), BT23-059 Justimon: Blitz Arm (lowest play cost)). `ctx.select_opp_permanent_min_by(|perm| extractor, …)` / `_max_by` sugar. For security-context tie-breaking, see new "Security context aggregate-filter targeting" below — likely needs promotion., EX10-010 Raid highest DP). `ctx.select_opp_permanent_min_by(|perm| extractor, …)` / `_max_by` sugar over the existing `select_opponent_permanent` filter closure — fully expressible today, just verbose.
- **If-effect-didn't-resolve on-decline callback** (EX9-066 Tai Kamiya & Matt Ishida, BT16-082 Ukkomon optional hatch tail; Rocks adds: P-186 Gallantmon "If this effect didn't delete, <Recovery +1 (Deck)>"). `PendingSelection.on_decline` field exists; no builder exposes it. Either `select_*_with_decline(..., on_decline)` or making the callback take `Option<usize>` where `None` means declined. Marked *primitive-with-fidelity-cost* (not pure sugar): today's closure-captured-bool workaround depends on the callback firing synchronously in the no-valid-targets / declined cases, which isn't guaranteed.; Dark Masters adds: BT13-102 Keenan Crier — "Your opponent may trash 1 Tamer card or Option card in their hand. If they don't, gain 1 memory and `<Draw 1>`"). `PendingSelection.on_decline` field exists; no builder exposes it. Either `select_*_with_decline(..., on_decline)` or making the callback take `Option<usize>` where `None` means declined. Marked *primitive-with-fidelity-cost* (not pure sugar): today's closure-captured-bool workaround depends on the callback firing synchronously in the no-valid-targets / declined cases, which isn't guaranteed.

### `<Barrier>` keyword (battle-only leave-field replacement with security-trash cost)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-barrier-keyword-battle-only-leave-field-replacement-with-security-trash-cost--resolved-2026-05-15-track-b-2026-05-08) by the 2026-05-15 hygiene sweep.

### `<Collision>` keyword (attack-scoped opposing Blocker aura + must-block enforcement)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-collision-keyword-attack-scoped-opposing-blocker-aura--must-block-enforcement--resolved-2026-05-15-group-6-task-4) by the 2026-05-15 hygiene sweep.

### `Keyword::Decoy` color-filter parameter + replacement-framework wiring
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-keyworddecoy-color-filter-parameter--replacement-framework-wiring--resolved-2026-05-15-phase-d--track-g-pr-457) by the 2026-05-15 hygiene sweep.

### Trash all digivolution cards of a permanent (unbounded stack-peel)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-trash-all-digivolution-cards-of-a-permanent-unbounded-stack-peel--resolved-2026-05-15-2026-05-03) by the 2026-05-15 hygiene sweep.

### Permanent-scoped modifier to suppress effect activation by timing
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-permanent-scoped-modifier-to-suppress-effect-activation-by-timing--resolved-2026-05-15-track-c-2026-05-06) by the 2026-05-15 hygiene sweep.

### Grant Security A. ±N modifier to a targeted permanent (parametric `SecurityAttackChange`)
- **Severity:** 🟡 PARTIAL
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT10-042 Venusmon, P-134 Shoemon — Dark Masters adds: BT19-093 Queen Device ("[Security] Give 2 of your opponent's Digimon `<Security A. -2>` for the turn"), BT16-046 GranKuwagamon ("1 of your Digimon gains `<Security A. +1>` for the turn")
- **Effect text:** "[When Digivolving] All of your opponent's Digimon gain ＜Security A. -1＞ until the end of your opponent's turn."
- **Status 2026-05-02:** Dynamic formula aura sibling is closed. `kind: aura` accepts `security_attack_fn`, and the combat security-check path recomputes it at resolution, then adds printed Security Attack keyword deltas and `ModifierType::SecurityAttackChange`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_dynamic_formulas phase2f2_formula_eval phase2f2_modifier_formula --nocapture`.
- **Updated 2026-05-10 (Track H §1):** Aura form is now wired through the typed flat slot. `kind: aura` accepts a top-level `security_attack: i32` field (alongside the existing dynamic `security_attack_fn`). `lower_aura` installs `ModifierType::SecurityAttackChange` carrying the literal delta as `value` on each match — the consult site `combat.rs:2326` reads it via `ModifierRegistry::sum`. Self, filter, and cross-side variants all land through the same path; negative deltas flow through unchanged (combat clamps the resulting check count to ≥0). Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl group6_auras -- aura_self_grants_flat_security_attack_plus_one aura_filter_grants_flat_security_attack_to_all_olympos_xii_digimon aura_filter_grants_flat_security_attack_minus_one_via_negative_delta`. The targeted (non-aura) flavor — "1 of your Digimon gains `<Security A. +1>` for the turn" — still lacks a typed `ctx.grant_security_attack_change(target, delta, expiry)` wrapper; the bare `add_modifier(target, ModifierType::SecurityAttackChange, delta as i32, expiry)` works, but the typed sugar reduces miscalls.
- **What's missing:** Targeted (non-aura) typed sugar `ctx.grant_security_attack_change(target, delta: i8, expiry)`. Plus `ctx.for_each_opponent_permanent(|h| …)` iteration sugar for snapshot mass application.
- **Suggested API shape:** `ctx.grant_security_attack_change(target, delta: i8, expiry)` wrapping `add_modifier(target, ModifierType::SecurityAttackChange, delta as i32, expiry)`. Plus `ctx.for_each_opponent_permanent(|h| …)` sugar for mass application.
- **Workaround:** For aura form: `kind: aura` + `security_attack: ±N` (closed). For targeted snapshot form: manual loop over `battle_area(opp_id)` at firing time + `add_modifier`.
- **Related:** "Named-target declarative aura (DP / keyword grants filtered by name/trait/level)"; "Native printed keyword parsing".

### Play / digivolve origin context flag ("if played by effects", "if digivolved by this effect")
- **Severity:** 🟢 RESOLVED for generic by-effect observer gates / 🟡 PARTIAL for cleanup-token half (audit 2026-05-15: Track A `effect_initiated` bit lands DSL `event_is_effect_initiated` end-to-end; BT16-028 proves the gate. Residual: per-activation identity for "digivolved by THIS effect" vs another effect + effect-spawned permanent cleanup tokens — ProvenanceToken is engine-DONE but DSL `bind_played_provenance`/`delete_provenance_token` authoring path still pending; see "Effect-played permanent cleanup provenance" entry in resolved-gaps.md.)
- **Discovered in:** TS Olympos (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** BT24-023 Calmaramon ("if played by effects, 1 of their Digimon or Tamers can't suspend"), BT14-033 Patamon ("If digivolved by this effect, you may place 1 yellow Vaccine card from hand to security bottom") — Dark Masters adds: EX10-012 / EX10-020 / EX10-035 / EX10-057 / EX10-061 ("delete the Digimon this effect played" — needs cause attribution to identify the cost-reduction-played permanent), BT13-102 Keenan Crier ("[Opponent's Turn] When an effect plays a Digimon" — needs `was_played_by_effect()` filter on the OnEnterFieldAnyone observer)
- **Effect text:** "if played by effects, …" / "If digivolved by this effect, …"
- **What's missing:** The generic "by an effect" flag is implemented for `OnEnterFieldAnyone` and standard `OnDigivolve` observer predicates, and DNA/Jogress origin is implemented as `dna_origin` / `event_dna_origin()`. Still missing: per-activation identity for "digivolved by THIS effect" vs. another effect and effect-spawned permanent cleanup tokens.
- **Suggested API shape:** Add `PlayCause { Action, Effect { source_card: CardHandle } }` threaded through `Game::play_from_hand` / `digivolve_from_hand`. Expose `ctx.was_played_by_effect()`, `ctx.was_digivolved_by_effect(self_source_card) -> bool` sugar. Fold into the same context struct as `was_dna_digivolve`.
- **Updated 2026-05-08 (Track A):** `TriggerSource::EnteredField` and `TriggerSource::Digivolved` carry `effect_initiated`, copied into `TriggerContext.effect_initiated`; DSL `event_is_effect_initiated: true/false` evaluates that payload field. Normal hand play/digivolve set it false; effect play helpers and `effect_initiated_digivolve` set it true. BT16-028 now authors its `[All Turns] When an effect plays or digivolves...` observer with the gate, proving effect-play offers the free Fighter Mode digivolve while normal player-action play does not. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_028`.
- **Workaround:** No longer needed for generic "by an effect" observer gates. Still none for stricter "by this effect" cleanup/identity semantics.
- **Related:** "Zone-manipulation: effect-initiated digivolve" (setter site); `ctx.was_dna_digivolve()` item within that entry.

### Search-own-security-stack primitive (reveal full stack + select by filter)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-search-own-security-stack-primitive-reveal-full-stack--select-by-filter--resolved-2026-05-15-track-e-2026-05-09) by the 2026-05-15 hygiene sweep.

### Effect-initiated digivolve from security stack (free, trait-filtered)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-initiated-digivolve-from-security-stack-free-trait-filtered--resolved-2026-05-15-group-4) by the 2026-05-15 hygiene sweep.

### `OnPlaceSecurity` / `OnAddedToSecurity` observer timing dispatch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-onplacesecurity--onaddedtosecurity-observer-timing-dispatch--resolved-2026-05-15-pr-451-track-a) by the 2026-05-15 hygiene sweep.

### `OnDiscardSecurity` — effect-driven security-card trash trigger
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-ondiscardsecurity--effect-driven-security-card-trash-trigger--resolved-2026-05-15-pr-451-track-a) by the 2026-05-15 hygiene sweep.

### `<Reboot>` keyword enforcement in opponent's unsuspend phase
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-reboot-keyword-enforcement-in-opponents-unsuspend-phase--resolved-2026-05-15-group-6-task-4) by the 2026-05-15 hygiene sweep.

### Digivolution-stack source extraction (`pop_top_source` from named permanent)
- **Severity:** 🟡 PARTIAL — narrowed title "Generic `pop_top_digivolution_source` for arbitrary re-routing (BT24-093)" (audit 2026-05-15: BT20-084 destination-specific shape closed via `security_place_top_stacked_card` Track E 2026-05-08. Residual: a general-purpose `pop_top_digivolution_source(target) -> Option<CardSource>` that returns the popped card for re-routing to arbitrary destinations — needed by BT24-093 Temple of Beginnings.)
- **Discovered in:** TS Olympos (2026-04-18)
- **Card(s):** BT24-093 Temple of Beginnings — Puppets adds: BT20-084 Sistermon Ciel (Awakened) ("Place this Digimon's top stacked card as the top security card" at End of All Turns; requires active-top-card extraction and legal empty-stack cleanup).
- **Effect text:** "You may place the top stacked card of any of your Digimon with [Aegiochusmon] or [Jupitermon] in their names as the top security card." / "Place this Digimon's top stacked card as the top security card."
- **What's missing:** No helper to extract a `CardSource` from the top of a specific permanent's `card_sources` for arbitrary re-routing. `ctx.de_digivolve` pops+trashes and does not return the extracted card. Needs `ctx.pop_top_digivolution_source(target) -> Option<CardSource>` that removes the topmost digivolution source (not the active top card), returning it for caller placement (e.g., to security top), with no `OnDeletion` fire since the card is moved not deleted.
- **Suggested API shape:** `ctx.pop_top_digivolution_source(target: PermanentHandle) -> Option<CardSource>` — removes `card_sources.last()`, returns it for caller re-routing. Combined with `ctx.place_security_top(player, card)` from the security-stack-operations gap.
- **Workaround:** "None — BLOCKED." Raw `battle_area[i].card_sources.pop()` skips any `OnLeaveField` / inherited-stack recomputation and breaks the curated-API contract.
- **Related:** "Zone-manipulation: security stack operations"; "Zone-manipulation: return-to-hand / return-to-deck / bounce self".

### Fixed attack target — `CannotBeRedirectedAsAttackTarget` / `CannotSwitchAttackTarget` modifiers
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-fixed-attack-target--cannotberedirectedasattacktarget--cannotswitchattacktarget-modifiers--resolved-2026-05-15-track-c--track-d-2026-05-0607) by the 2026-05-15 hygiene sweep.

### In-effect branch-choice selector (`select_effect_choice` / "choose one of N effects")
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-in-effect-branch-choice-selector-select_effect_choice--choose-one-of-n-effects--resolved-2026-05-15) by the 2026-05-15 hygiene sweep.

### Conditional digivolve-target restriction (filter on candidate top-card name/trait/level/color)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** EX10-012 MetalSeadramon, EX10-020 Puppetmon, EX10-035 Machinedramon, EX10-057 Piedmon, BT15-031 MetalSeadramon, BT15-066 Machinedramon, BT15-079 Piedmon
- **Effect text:** "[All Turns] This Digimon can only digivolve into [Apocalymon]." / "[Your Turn] This Digimon can only digivolve into white Digimon."
- **What's missing:** `ModifierType::CannotDigivolve` is a blanket flag — the action mask checks the modifier without consulting the candidate digivolve target's identity. There is no scripting-surface variant that gates digivolution on a predicate over the candidate top-card (name, trait, color, level). Two flavors surface: name-restricted ("can only digivolve into [Apocalymon]") and color-restricted ("can only digivolve into white Digimon"). Auto-blocking drops the legal target; auto-allowing drops the restriction.
- **Suggested API shape:** `ModifierType::DigivolveTargetRestriction(Box<dyn Fn(&CardSource, &[CardData]) -> bool>)` consulted by `Game::digivolve_from_hand` and the digivolve mask emitter against each hand card before lighting up `DIGIVOLVE_*` slots. Builder sugar: `Effect::declarative(card).restrict_digivolve_target(|src, data| src.contains_name("Apocalymon"))` or `_color(Color::White)`. Sibling `CanOnlyDigivolveIntoColor(u8 bitmask)` and `CanOnlyDigivolveIntoTrait(String)` shorthand variants for the most common flavors.
- **Workaround:** "None — BLOCKED." Blanket `CannotDigivolve` over-applies; omitting the clause silently allows any digivolution.
- **Related:** "Player-scoped modifier registry"; "Zone-manipulation: effect-initiated digivolve"; RUST_ENGINE_API.md §9.

### Effect-spawned permanent with end-of-turn deletion rider (`delete the Digimon this effect played`)
> Resolved by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`, 2026-05-20). `schedule_delete_played_at_turn_end` provenance-bound turn-end self-delete (`PUPPETS-G003`) and `play_token` `bind_as` + opponent-turn-end cleanup (`PUPPETS-G016`) both landed. Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Effect-driven play of a Digimon from hand to an empty breeding-area slot (without paying cost)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-driven-play-of-a-digimon-from-hand-to-an-empty-breeding-area-slot-without-paying-cost--resolved-2026-05-15-group-4) by the 2026-05-15 hygiene sweep.

### Cast-time stack-construction for cost reduction (place N differently-named cards from battle-area/trash UNDER the played card)
- **Severity:** ✅ RESOLVED 2026-07-05 (BT15-102 landing)
- **Landed shape — differs from the Track-E plan below (which predates the DigiXros transaction substrate):** instead of carving `commit_play_to_battle_area_without_on_play` out of the play pipeline, the assembly rides the **existing `DigiXrosTransaction` substrate** (which already implements everything the plan wanted to build: count-capped iterative material select over battle-area/trash/hand, `distinct_by: name` mask-level uniqueness, per-material cost deltas, cost paid at the reduced value, post-payment `push_under` placement with the consumed permanent's own stack trashed, `WhenWouldLeaveBattleArea` windows, OnPlay drained after attachment, and the resumable-VM `DigiXrosMaterialSelection` frame for clone-safety). New pieces:
  1. `AltPathKind::CastTimeAssembly` (`kind: cast_time_assembly` in YAML — materials carry `filter` / `repeat {min,max}` / `distinct_by` / `zones` / explicit `cost_delta`; zones are parametric per material so the EX10-061 security-stack sibling composes once `DigiXrosMaterialZone` grows a `Security` origin — that residual stays with the EX10-061 entry).
  2. `DigiXrosTransaction.is_digixros: bool` semantic firewall — `build_digixros_transaction_for_hand_card` accepts both kinds; `was_digixros()` / `digixros_count()` report `false`/`0` and DigiXros wildcards don't apply for a non-DigiXros assembly.
  3. `Game::cast_time_assembly_play_reduction_for_hand_card` — DCGO hidden-availability parity (`min(cap, unique-identity candidates) × delta`, computed by greedy fill of a scratch transaction); consumed by the PLAY_HAND mask so declare-then-pay legality uses the REDUCED cost.
  4. DCGO `CanNoSelect` decline gate in `install_pending_digixros_material_selection_or_finish`: the stop/finish PASS is masked while `(final_cost - total_reduction)` is unpayable (no-op for printed DigiXros, whose printed cost already gated the mask).
- **Tests:** `tests/cast_time_assembly.rs` (8 — optional-zero, distinct-name masking, DCGO decline gate, unique-name mask availability, non-DigiXros firewall, clone-mid-selection) + `tests/cards_behavioral/bt15/bt15_102.rs`.
- **Known residual (documented, unchanged from DigiXros):** the transaction is only built for `PendingWouldPlayOrigin::Hand` — an effect-initiated play from another zone skips the assembly (same limitation as printed DigiXros hosts).
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-102 Apocalymon
- **Effect text:** "When this card would be played, by placing up to 3 [Dark Masters] trait cards with different names from your battle area or trash under it, reduce the play cost by 4 for each one."
- **What's missing:** Xros Heart now has a dedicated DigiXros transaction substrate, but BT15-102 still needs the generic non-DigiXros assembly form: player-driven multi-select of UP-TO-N from a union of source zones (battle area + trash), under a different-name uniqueness constraint, with the placed cards becoming the new permanent's digivolution stack. This is distinct from recipe-slot DigiXros and from EX10-061 Apocalymon's "from your security stack" cast-time variant (sibling primitive, different source zone).
- **Suggested API shape:** `Effect::before_pay_cost(card).with_optional_under_placement(max_count: u8, source_zones: &[Zone], filter, uniqueness: UniquenessFilter::Name, cost_per_placed: i16, callback)` — surfaces a multi-select at cost-time; for each chosen card, removes from source zone, queues for stack-attachment after the play resolves; reduces effective `play_cost` by `cost_per_placed * count`.
- **Workaround:** "None — BLOCKED." Pre-deciding the placement count auto-selects on the player's behalf (violates §17); skipping the reduction makes Apocalymon unplayable.
- **Related:** "Dynamic cost reduction at `BeforePayCost`"; RUST_PYTHON_PARITY.md §4.7e (DigiXros cost-reduction); "Place card at a specific stack position".
- **Updated 2026-05-24:** Do not use this entry for printed DigiXros cards. `DigiXrosTransaction` covers Xros Heart-style recipes, cost deltas, origin-zone extensions, and post-payment source attachment. This entry remains open only for arbitrary cast-time assembly shapes that are not recipe-slot DigiXros.
- **Track E (2026-05-08) deferred — implementation strategy for follow-up:** The work requires surgery on `Game::play_from_hand_with_cost_result` (`code/digimon-engine/src/game_actions.rs`) to splice a pre-`OnPlay` assembly hook between the cost calculation step and the `OnPlay` drain. Current flow returns `Played(field_index)` only after `OnPlay` drains; a cast-time-assembly hook must run after the permanent enters battle area but before `OnPlay` triggers fire. Suggested implementation phases:
  1. Carve out an internal `commit_play_to_battle_area_without_on_play(player, hand_index, cost_delta) -> Option<usize>` from the existing `play_from_hand_with_cost_result` so the placement and the `OnPlay` drain become separable.
  2. Add `EffectContext::play_with_cast_time_assembly(player, hand_index, cost_delta, max_count, source_zones, filter, cost_per_placed)` that calls the inner placement, installs a count-capped multi-select over `source_zones` with `is_optional_zero=true`, and on resolve: (a) installs each chosen card under the new permanent via `place_as_bottom_source` (top-down), (b) reduces memory cost retroactively by `cost_per_placed * count`, (c) drains `OnPlay`.
  3. DSL verb `cast_time_assembly:` block within the `play:` step.
  ~~Until this lands, BT15-102 Apocalymon's [Main] play is OMITTED from any compiled YAML.~~ **Superseded 2026-07-05 — see the RESOLVED landed shape above** (the DSL surface is an `alt_paths:` kind, not a `play:` sub-block, and the plan's `commit_play…without_on_play` carve-out proved unnecessary once the transaction substrate existed).

### Cross-card effect re-firing — activate a foreign card's [On Play] effect attributed to the source
- **Severity:** ✅ RESOLVED 2026-07-05 (BT15-102 landing — extends Track K)
- **Landed shape:** `EffectContext::activate_foreign_card_effect(card_id, carrier: PermanentHandle, timing_filter, selecting_player)` in `effect_context/action/refire.rs`, backed by `enumerate_refireable_effects_for_card(game, card_id, carrier, timing_key)` in `effect.rs`. DCGO-parity carrier semantics (`EffectList_ForCard(timing, card)`): the foreign card's effect list is instantiated via `effects_for_card(card_id, carrier_top_handle)`, so the refired body reads the CARRIER as "this Digimon" (`source_card` / `source_permanent` = the carrier); `card_id` stays the foreign card's so `run_queued_effect` re-resolves the right effect list. Reuses the Track-K dispatch tail (`dispatch_refireable_effects`): single eligible effect runs directly, >1 surfaces a MANDATORY `EffectChoice` (data-driven `RefireEffectChoice` resume frame — clone-safe). Once-per-turn slot accounting is **bypassed** for the foreign entries (OPT keys `(source_card, slot)` on the carrier — consulting them would alias the carrier's own slots; and the placed card sat in the trash all turn, so it has no prior activations to gate on). DSL verb: `refire_card_effect: { card: <Card binding>, timing: on_play | when_digivolving | on_play_or_when_digivolving }`, fed by the new `place_as_bottom_source.bind_placed_as:` (binds the placed card's stable `CardHandle` post-move; absent on decline → the refire silently no-ops and the clause tail still runs). Companion vocab: `trash_from_top.count` widened to a `FormulaSpec` for the "for each of this Digimon's level 6 digivolution cards" mill.
- **Tests:** `tests/effect_context/effect_refiring.rs` (5 foreign-card tests incl. clone-mid-choice), `tests/dsl/effect_refiring.rs` (verb lowering + bad-timing rejection), `tests/cards_behavioral/bt15/bt15_102.rs` (end-to-end End-of-Turn clause).
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-102 Apocalymon
- **Effect text:** "by placing 1 level 6 or lower card from your trash as this Digimon's bottom digivolution card, activate 1 [On Play] effect on that card as an effect of this Digimon."
- **Status update 2026-05-10:** Track K resolved the permanent-target version used by BT24-102 Homeros. `EffectContext::refire_target_effect(target, TimingFilter::{OnPlay, WhenDigivolving, Either}, selecting_player, bypass_once_per_turn)` enumerates a target permanent's registered effects, filters On Play / When Digivolving timings, respects the target's once-per-turn accounting, exposes an `EffectChoice` when multiple effects are eligible, preserves carrier semantics on the target permanent, and keeps source attribution on the grantor. YAML `refire_effect` now accepts `timing: on_play_or_when_digivolving`, covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context effect_refiring`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl refire`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt24_102`.
- **What's still missing:** BT15-102 Apocalymon's clause invokes the [On Play] effect of a DIFFERENT card object just placed from trash as a digivolution source, not an existing battle-area permanent. That still needs a source-card / stack-card refire variant that can enumerate `CardEffectRegistry::get(other_card_id).effects(handle)` for the placed source card and bind carrier/source attribution to Apocalymon.
- **Suggested API shape:** `ctx.activate_foreign_on_play(other_card_id: &str, target_perm: PermanentHandle, attribution: SourceAttribution)` paired with the existing `select_effect_choice` gap when the foreign card has >1 OnPlay clause. SourceAttribution variants: `AttributeToSource` vs. `AttributeToOther`.
- **Workaround:** "None — BLOCKED." Inline-duplicating every Lv6-or-lower card's OnPlay text is unscalable.
- **Related:** "Effect re-firing / cross-timing self-trigger"; "In-effect branch-choice selector (`select_effect_choice` / \"choose one of N effects\")".

### `<Retaliation>` keyword + combat enforcement
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-retaliation-keyword-resolved-2026-05-15-group-6-task-4) by the 2026-05-15 hygiene sweep.

### Reveal-zone overlay (declarative type/level synthesized while card is in deck or being revealed)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT17-068 Mephistomon
- **Effect text:** "While this card is revealed from your deck, this card is also treated as level 6."
- **What's missing:** No mechanism to synthesize an alternate `level` (or other type/trait/color value) on a `CardSource` while it lives in zones outside the battle area. All level lookups read `CardData.level` directly; there's no per-`CardSource` overlay layer that effect-driven reveal/search filters consult. Required so that downstream filters ("add a level 6 [Dark Masters] Digimon among them to the hand") see Mephistomon as level 6 while it's a revealed deck card. Sibling shape to "Digivolution-stack name overlay" but for level (and zone scope = deck/reveal rather than digivolution stack).
- **Suggested API shape:** `Effect::declarative(card).level_overlay_in_zone(Zone::Deck | Zone::Revealed, |rctx| Some(6))`. Every `CardSource::level(card_data)` call routed through an `effective_level(card_source, zone, card_data)` helper that unions overlay sources. Tensor / mask / aura filter passes that key on level must consult the helper. Same plumbing reused for trait / color overlays.
- **Workaround:** "None — BLOCKED." Always-treat-as-level-6 violates the "while revealed from your deck" zone scope.
- **Related:** "Digivolution-stack name overlay (\"has all names of materials\")".

### `<Scapegoat>` keyword (leave-field replacement with "delete another own Digimon" cost)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-scapegoat-keyword-leave-field-replacement-with-delete-another-own-digimon-cost--resolved-2026-05-15-track-b-2026-05-08) by the 2026-05-15 hygiene sweep.

### Effect-initiated play from face-up security stack (search-then-play-free)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** P-216 WaruMonzaemon, EX10-012 MetalSeadramon (inherited security), EX10-020 Puppetmon (inh sec), EX10-035 Machinedramon (inh sec), EX10-057 Piedmon (inh sec), EX10-072 Spiral Mountain (Delay body)
- **Effect text:** "[On Deletion] You may play 1 face-up [Dark Masters] trait Digimon card from your security stack without paying the cost." / "You may play 1 face-up Digimon card with the [Dark Masters] trait from your security stack without paying the cost."
- **What's missing:** Distinct from the existing "Effect-initiated digivolve from security stack" gap (which targets digivolve, not play, and was filed against BT14-033 Patamon). Need a play-free flow that (a) prompts the controller to pick a face-up entry in their own security stack restricted to a trait filter, (b) removes that entry from `player.security`, (c) instantiates it as a battle-area permanent firing OnPlay through the standard queue, (d) leaves the rest of the security stack ordering intact. The face-up filter is load-bearing — many security cards face-down; only face-up entries are eligible per the card text.
- **Suggested API shape:** `ctx.play_face_up_security_free(player, filter, optional, callback: Fn(&mut Ctx, Option<usize>))` — installs a `SelectionKind::OwnFaceUpSecurity(filter)` selection, on resolution removes the entry from `player.security`, calls into `Game::play_from_security` (Tamer-aware path) bypassing memory cost, fires `OnPlay` and `OnLoseSecurity`. Distinct from existing `play_from_security` (which consumes `pending_security` during attack-time security check).
- **Workaround:** "None — BLOCKED." Faking a hand transit (security → hand → play free) violates no-approximations.
- **Related:** "Search-own-security-stack primitive (reveal full stack + select by filter)"; "Effect-initiated digivolve from security stack (free, trait-filtered)"; "Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)".

### OnDeletion cause discriminator ("if deleted by an effect" / "by battle" / "by your own effects")
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-ondeletion-cause-discriminator-if-deleted-by-an-effect--by-battle--by-your-own-effects--resolved-2026-05-15-phase-b-2026-04-24) by the 2026-05-15 hygiene sweep.

### Counter window + `<Blast Digivolve>` activation flow ([Hand][Counter] play path) [G-COUNTER-BLAST-DNA-ACTIVATION]
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-counter-window--blast-digivolve-activation-flow-handcounter-play-path--resolved-2026-05-15-track-d-2026-05-08) by the 2026-05-15 hygiene sweep.

### Global `OnOwnSecurityRemoved` observer timing (mirror of `OnOpponentSecurityRemoved`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-global-onownsecurityremoved-observer-timing-mirror-of-onopponentsecurityremoved--resolved-2026-05-15-track-a-2026-05-06) by the 2026-05-15 hygiene sweep.

### Generic `.activation_cost(...)` builder hook for triggered abilities (suspend-self / pay-as-cost on triggered abilities)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-generic-activation_cost-builder-hook-for-triggered-abilities--resolved-2026-05-17-phase-2-track-b) by Phase 2 Track B on 2026-05-17.

### Per-N-suspended scaling threshold for deletion / damage effects (count-bounded multi-select with derived threshold)
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; formula leaf `suspended_count` landed in `digimon-dsl/src/formula.rs:137` per Track J 2026-05-10. Residual: chained count-bound multi-select followed by formula-threshold downstream filter — the DSL/Rust shape that lets a downstream `select_opponent_permanent` consume an upstream pick count as a derived threshold.)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** EX8-074 MedievalGallantmon
- **Effect text:** "[When Digivolving] You may suspend 1 Digimon. Then, you may delete 1 of your opponent's 8000 DP or lower Digimon. For each other suspended Digimon, add 3000 to this DP deletion effect's maximum."
- **What's missing:** A scaling threshold derived from how many of your own Digimon are CURRENTLY suspended (including those just suspended by this card's optional sub-effect). Requires (a) the optional sub-selection to suspend any number (0–N) of own Digimon — sibling to "Selection: multi-select with aggregate-sum constraint" but with a *count*-based stop instead of an aggregate-sum constraint; (b) a derived dynamic threshold computed AFTER the suspend selection completes; (c) feeding that threshold into a downstream `select_opp_permanent` filter (DP ≤ 8000 + 3000*other_suspended). Cannot be expressed by the existing aggregate-sum gap (no aggregate sum here) nor by single-select sugar.
- **Suggested API shape:** `ctx.select_multiple_own_permanents_count(prompt, filter, optional, callback: Fn(&mut Ctx, Vec<PermanentHandle>))` (PASS-terminated multi-select for cost-payment) followed by a downstream `select_opponent_permanent_filter_dynamic(threshold_fn, …)`. Or chained-builder DSL.
- **Workaround:** "None — BLOCKED." Hard-coding the single-base threshold drops the scaling clause; auto-selecting maximum violates §17.
- **Related:** "Selection: multi-select with aggregate-sum constraint"; "Aggregate filter helpers" (Ergonomics partials).

### Player-scope mass `CannotSuspend` aura on opponent (condition-gated and / or stack-depth-filtered)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** EX8-026 MetalSeadramon (memory-state predicate), BT16-026 Vikemon (stack-depth filter)
- **Effect text:** "[All Turns] While you have 1 or more memory, none of your opponent's Digimon can suspend." / "none of their Digimon with 1 or fewer digivolution cards can suspend until the end of their turn"
- **What's missing:** Combination of three primitives: (a) a player-side mass aura that broadcasts `CannotSuspend` to every opponent battle-area permanent (sibling to "Player-scoped modifier registry" gap, which lists `CannotPlay*` / `IgnoreColorRequirement` but not `CannotSuspend`); (b) a live, continuously-evaluated condition closure keyed on the controller's own memory state OR on a per-target stack-depth predicate, NOT a one-shot snapshot; (c) live re-evaluation as state crosses thresholds; (d) future-permanent inclusion (cards the opponent plays during the window are subject to the aura if they meet the filter).
- **Suggested API shape:** Extend the "Player-scoped modifier registry" entry's `ModifierType` set with `CannotSuspend` (already exists permanent-scope, extend to player-scope). Pair with the "Condition-gated modifier entries" extension's predicate-closure form: `Effect::declarative(card).player_aura(opponent_id).cannot_suspend().while_(|rctx| rctx.memory(rctx.player) >= 1)`. For per-target filter (Vikemon's stack-depth case): `.target_filter(|rctx, h| stack_depth(h) <= 2)`. Suspend mask + force-suspend paths on the affected player must consult the gated player-modifier on each query.
- **Workaround:** "None — BLOCKED." Snapshot at OnPlay/EndOfTurn over-applies (doesn't track state crossings and excludes future plays).
- **Related:** "Player-scoped modifier registry"; "Condition-gated modifier entries + new Expiry variants"; "Named-target declarative aura".

### `OnAllyAttack` / `OnOpponentAttack` observer timing context
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-onallyattack--onopponentattack-observer-timing-context--resolved-2026-05-15-2026-04-29-substrate-dsl-predicate-spin-off) by the 2026-05-15 hygiene sweep.

### `EndOfOpponentsTurn` effect timing not dispatched
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-endofopponentsturn-effect-timing-not-dispatched--resolved-2026-05-15-phase-1-pr-449) by the 2026-05-15 hygiene sweep.

### Forced opponent hand reduction primitive (`ctx.trash_opponent_hand_to_count`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-forced-opponent-hand-reduction-primitive-ctxtrash_opponent_hand_to_count--resolved-2026-05-15-pr-454-track-e) by the 2026-05-15 hygiene sweep.

### Inherited triggered-effect dispatch: `enqueue_from_permanent` must walk digivolution stack
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-inherited-triggered-effect-dispatch-enqueue_from_permanent-must-walk-digivolution-stack--resolved-2026-05-15-2026-05-06) by the 2026-05-15 hygiene sweep. **Phase 2 Track D (commit `bc852640`) completed the closure** with a dedicated regression test (`tests/timing_dispatch.rs`) and 18 dependent tests un-ignored across `cards_behavioral` (BT22-005, BT24-012, BT24-016, BT21-025, BT16-040, BT17-015, BT17-018, BT21-001, BT21-017, BT22-005, P-189, EX4-003, EX11-008, BT14-001 + others). G-WHEN-DIGIVOLVING-DISPATCH absorbed by the same walk. See `qa/resolved-gaps.md` § "Phase 2 Track D closure" for full closure details.

### `CannotAttackPlayer` modifier enforcement (mask + combat)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-cannotattackplayer-modifier-enforcement-mask--combat--resolved-2026-05-15-track-d-2026-05-08) by the 2026-05-15 hygiene sweep.

### Return a selected digivolution-stack source card to its owner's hand — RESOLVED 2026-05-21
> Closed by `bg-imperial-substrate-closeout` — `EffectContext::return_card_source_to_hand`
> + `return_selected_sources_to_hand` DSL verb landed; BT12-031 → IMPLEMENTED. See
> [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) § "Follow-up engine gaps closed
> (2026-05-21)". Scoping detail retained below for reference.

- **Severity:** 🟡 PARTIAL (*ergonomics / primitive-with-fidelity-cost* — the selection half is fully closed; only the source-to-hand movement primitive is missing, and there is no faithful workaround for it, but the gap is narrow)
- **Gap ID:** `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` (canonical engine-gap record; supersedes the same ID in [`qa/dsl-vocab-gaps.md`](../qa/dsl-vocab-gaps.md), which is now a redirect)
- **Discovered in:** BG Imperial (2026-05-04, BT12-031 `batch-implement-cards-rust-dsl`); diagnosis corrected from "DSL-only" to engine gap 2026-05-21 (`bg-imperial-substrate-closeout`)
- **Card(s):** BT12-031 Imperialdramon: Paladin Mode (Clause 0, Step C — the last non-IMPLEMENTED clause; card verdict PARTIAL, 2 tests `#[ignore]`'d)
- **Effect text:** "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's digivolution cards to its owner's hand, return all of your opponent's suspended Digimon to the bottom of their owners' decks instead."
- **What's missing:** There is **no `EffectContext` method or DSL verb that returns a single selected digivolution-stack source card to its owner's hand.** `select_own_sources` already binds stable `SourceSelectionRef`s (`effect_context/selections.rs:383`, name-filtered via the `filter:` arg since 2026-05-08), and the `binding_present` branch for the optional-decline fall-through is solved card-side. But the only two consumers of those bound source refs are `TrashSelectedSources` → `ctx.trash_card_source` (routes the source `Card` to `owner.trash`, `effect_context/mod.rs:3477`) and `PlaySelectedSourcesFree` → `ctx.play_selected_sources_without_cost` (`effect_context/mod.rs:3616`). `EffectContext::return_to_hand` (`effect_context/mod.rs:3625`) bounces a whole **permanent** (top card + entire stack) — it cannot extract one below-top source. So a source card selected out of a digivolution stack can be trashed or replayed, but not handed back to its owner.
- **Suggested API shape:**
  - **`EffectContext` method** — sibling of `trash_card_source`, differing only in the destination zone (push to `owner.hand` instead of `owner.trash`); same `OnDigivolutionCardTrashed`-class observer fan-out is NOT appropriate (this is a return-to-hand, not a trash), but the source-leaves-stack event should still fire so leave-stack observers see it:
    ```rust
    /// Remove a single digivolution source card from `perm`'s stack and
    /// route it to its owner's hand. Mirrors `trash_card_source` but the
    /// destination is the owner's hand, not trash. Fires the source-leaves-
    /// stack observer path (NOT the trash-specific OnDigivolutionCardTrashed).
    pub fn return_card_source_to_hand(
        &mut self,
        perm: PermanentHandle,
        card: CardHandle,
    ) -> bool
    ```
    A `Vec`-taking convenience wrapper (`return_selected_sources_to_hand(&mut self, selected: Vec<SourceSelectionRef>)`) keeps parity with `play_selected_sources_without_cost`.
  - **DSL verb / `CompiledStep`** — mirror `TrashSelectedSources` exactly: a new `ReturnSelectedSourcesToHand { source_refs: String }` `CompiledStep` (`digimon-dsl/src/compiled.rs`, alongside line 1247), a `StepSpec::ReturnSelectedSourcesToHand(TrashSelectedSourcesArgs)` reusing the existing `TrashSelectedSourcesArgs { source_refs: String }` struct (`digimon-dsl/src/step.rs:1333`), the `compile.rs` arm (alongside line 1766), and the consumer arm in `dsl_cards/step/zone_moves.rs` (alongside the `TrashSelectedSources` arm at line 206) that reads `bindings.get_source_refs(...)` and calls the new `EffectContext` method per ref.
  - **YAML form:**
    ```yaml
    - select_own_sources:
        from: source            # restrict to this Digimon's own stack
        filter: { name_contains: "Imperialdramon: Dragon Mode" }
        min: 0
        max: 1
        bind_as: dragon_mode_source
        prompt: "..."
        then:
          - return_selected_sources_to_hand: { source_refs: dragon_mode_source }
          # binding_present branch then runs the return-all-to-bottom outcome
    ```
- **Workaround:** None faithful. Trashing the source instead of returning it to hand changes the printed cost; omitting Step C (current state) drops the entire alternative outcome.
- **Likely files:** `code/digimon-engine/src/effect_context/mod.rs` (new method next to `trash_card_source`), `code/digimon-engine/src/dsl_cards/step/zone_moves.rs` (consumer arm), `code/digimon-dsl/src/compiled.rs` + `code/digimon-dsl/src/step.rs` + `code/digimon-dsl/src/compile.rs` (verb plumbing), `code/digimon-engine/cards/bt12/BT12-031.yaml` (un-block Step C), `code/digimon-engine/tests/cards_behavioral/bt12/bt12_031.rs` (un-ignore 2 tests).
- **Complexity estimate:** Small. One ~20-line `EffectContext` method (a near-copy of `trash_card_source` with the destination `Vec` swapped) + the standard 4-file DSL-verb plumbing. No new selection state, no mask change, no `ACTION_SPACE_SIZE` impact — the selection is already a closed `select_own_sources` flow.
- **First test:** `bt3`-style behavioral test under `tests/cards_behavioral/bt12/bt12_031.rs` (un-ignore the existing `#[ignore]`'d Step C tests at lines 345/362): set up Imperialdramon: Paladin Mode with an `Imperialdramon: Dragon Mode` card in its digivolution stack and 2+ suspended opponent Digimon; resolve the [When Digivolving] effect; accept the optional `select_own_sources` pick; assert the Dragon Mode source card lands in its owner's hand AND every suspended opponent Digimon is at the bottom of its owner's deck. The decline path test asserts that passing the optional selection falls through to the base "return 1 suspended opponent Digimon to hand" outcome with the Dragon Mode source untouched.
- **Known interactions / risks:**
  - Owner routing: the moved card must go to the source card's `owner` (the `CardSource.owner` field), not the controller's hand — `trash_card_source` already reads `removed.owner`; the new method must do the same so a source owned by the opponent (rare, but possible via control-transfer plays) routes correctly.
  - Stack invariant: extracting a below-top source must not disturb the host permanent's top card or remaining stack ordering — `trash_card_source` removes by `position(...)` rather than `pop()`, so the new method should do likewise.
  - Observer dispatch: this is a *return-to-hand*, so it must NOT fire `OnDigivolutionCardTrashed` (which would mis-attribute the move as a trash to Rocks-style source-trash listeners). Decide explicitly whether any source-leaves-stack observer should fire — the safest first cut fires nothing trash-specific.

### Player-scoped one-shot future-digivolve cost reducer with a paid cost — RESOLVED 2026-05-21
> Closed by `bg-imperial-substrate-closeout` — new `player_cost_reducer.rs`
> (`PlayerDigivolveCostReducer`), `EffectContext::arm_player_digivolve_cost_reducer`,
> a pre-cost accept/decline + suspend-cost `PendingSelection` chain in a split
> `digivolve_from_hand` / `digivolve_from_hand_inner` (the synchronous
> `scan_before_pay_cost_reduction_with_target` hot path was NOT touched), and the
> `arm_digivolve_cost_reducer` DSL step. BT3-103 → IMPLEMENTED. See
> [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) § "Follow-up engine gaps closed
> (2026-05-21)". Scoping detail retained below for reference.

- **Severity:** 🔴 BLOCKING
- **Gap ID:** `G-COST-REDUCE-ALLY-DIGIVOLVE` (umbrella; also covers the `G-COST-REDUCE-NEXT-SINGLE-FIRE` and `G-PAY-COST-SELECT-ARBITRARY-SUSPEND` sub-IDs cited in the BT3-103 test header — they are three facets of this one missing primitive)
- **Discovered in:** BG Imperial (2026-05-03 cross-archetype assessment, `G-BG-01`); explicitly **DEFERRED** by Phase 2 Track H's discovery rider (see line 127 above)
- **Card(s):** BT3-103 Hidden Potential Discovered! (Option, Green, cost 4 — the last non-IMPLEMENTED BG Imperial card; Clause 0 omitted from `BT3-103.yaml`, 5 tests `#[ignore]`'d). Cross-archetype: green/yellow Memory Boost / Training Options frequently install "the next time one of your Digimon would digivolve this turn" reducers with a paid condition.
- **Effect text:** "[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5."
- **What's missing:** A **player-scoped, one-shot, paid future-digivolve cost reducer** installed by a [Main] effect. No part of this shape exists today:
  1. **No player-scoped reducer registry.** `before_pay_cost_source_infos` (`game_actions.rs:4214`) gathers `BeforePayCost` effects only from battle-area permanents, breeding-area permanents, and the cost-target card itself. An Option that resolves and trashes itself (BT3-103 is not a Plug-In / not a Delay — it leaves the field on resolution) has no field permanent to host the reducer effect, so it can never be scanned. The `player_modifiers` registry (`modifiers.rs:707`, `PlayerModifierEntry`) is a passive *data* registry — `value` / `payload` / `expiry` only — with no slot for a `condition` / `cost_reduction_fn` / `pay_cost_fn` closure, so a reducer cannot live there either.
  2. **The digivolve cost path cannot prompt an optional/paid reducer.** `scan_before_pay_cost_reduction_with_target` (`game_actions.rs:3965`) — the function the digivolve cost-calc calls — **explicitly skips `candidate.optional` reducers** (line 3982: `if candidate.optional || (candidate.has_pay_cost && cost_target.is_none()) { continue; }`). The interactive accept/decline pending-selection chain (`continue_play_from_hand_cost_reduction_chain`, `game_actions.rs:465`) exists **only for `CostReductionKind::Play`** (play-from-hand). The digivolve path has no equivalent chain, so even a field-hosted optional reducer cannot surface its choice during a digivolution.
  3. **No "fires exactly once, then consumes itself" lifecycle.** `max_per_turn` (`inspect_cost_reduction_candidate`, `game_actions.rs:4100`) caps activations *per turn* but is keyed to a `source_permanent` via `cost_reducer_activation_count` (`game_actions.rs:4176`) — there is no permanent to key against here, and "next digivolve" means single-fire then removal, not a per-turn cap.
  4. **No `select`-an-arbitrary-Digimon-to-suspend cost inside the `BeforePayCost` flow.** `pay_cost_fn` runs synchronously inside `apply_cost_reduction_candidate` (`game_actions.rs:4156`); `suspend_self_as_cost` (`effect_context/mod.rs:2341`) only suspends the source. BT3-103's "by suspending 1 of your Digimon" requires a player-visible `select_own_permanent` *inside* the cost payment — an interactive selection nested in the cost flow.
- **Suggested API shape:**
  - **A closure-bearing player-scoped reducer registry.** Either (a) a new `Game` field `player_cost_reducers: HashMap<PlayerId, Vec<PlayerCostReducer>>` where `PlayerCostReducer` carries the same `condition` / `cost_reduction` / `pay_cost_fn` (or `activation_cost_fn`) closures as `Effect`, plus a `consumed: bool` / single-fire flag and a `CostReductionKind` filter; or (b) extend `before_pay_cost_source_infos` to also yield infos sourced from a player-scoped store. A new builder constructor:
    ```rust
    // Effect builder — install a turn-scoped, single-fire reducer onto a player.
    Effect::before_pay_cost_player_scoped(card)
        .cost_kind(CostReductionKind::Digivolve)
        .single_fire()                       // consume on first successful application
        .expiry(Expiry::EndOfTurn)           // "For the turn" upper bound
        .condition(|rctx| /* cost target permanent is a green Digimon */)
        .cost_reduction(5)
        .activation_cost(|ctx| ctx.select_one_own_digimon_to_suspend_as_cost())
    ```
    and an `EffectContext` install helper: `ctx.arm_player_digivolve_cost_reducer(player, reducer)`.
  - **Generalize the optional/paid reducer pending-selection chain to the digivolve path.** Factor the accept/decline `PendingSelection` loop currently inside `continue_play_from_hand_cost_reduction_chain` so the digivolve cost-calc (`scan_before_pay_cost_reduction_with_target`) can also install it instead of skipping optional/paid candidates. Decline must leave the unreduced cost; the reducer must NOT be consumed on decline.
  - **A cost-flow nested selection helper:** `ctx.select_one_own_digimon_to_suspend_as_cost()` — an interactive `select_own_permanent` (filter: own, unsuspended) returning `true` only after a suspend completes, surfaced through `pending_selection` so the RL action space sees it (Working Rule §17).
  - **YAML form (illustrative — exact verb naming TBD with DSL author):**
    ```yaml
    clauses:
      - timing: main
        process:
          - arm_digivolve_cost_reducer:
              scope: { of: you }
              expiry: this_turn
              single_fire: true
              target_filter: { color_has: green }   # the digivolving Digimon
              amount: 5
              pay_cost:
                - select_own_permanent:
                    filter: { suspended: false }
                    prompt: "Suspend 1 of your Digimon"
                    then: [ suspend: { target: selected } ]
    ```
- **Workaround:** None — BLOCKED. A static unconditional `-5` modifier hides the printed "by suspending 1 of your Digimon" choice, can apply to the wrong digivolution, and ignores the single-fire semantics. Auto-suspending violates §17.
- **Likely files:** `code/digimon-engine/src/game_actions.rs` (player-scoped reducer collection in `before_pay_cost_source_infos`; lift the optional/paid pending-selection chain out of `continue_play_from_hand_cost_reduction_chain` and reuse it on the digivolve path), `code/digimon-engine/src/effect.rs` (new `before_pay_cost_player_scoped` builder + single-fire flag), `code/digimon-engine/src/modifiers.rs` or a new `Game` field (the closure-bearing player-scoped store), `code/digimon-engine/src/effect_context/` (install helper + `select_one_own_digimon_to_suspend_as_cost`), `code/digimon-engine/src/action/mask.rs` (the nested suspend-cost selection must be maskable), `code/digimon-dsl/src/step.rs` + `code/digimon-engine/src/dsl_cards/step/` (the `arm_digivolve_cost_reducer` verb + lowering), `code/digimon-engine/cards/bt3/BT3-103.yaml` + `code/digimon-engine/tests/cards_behavioral/bt3/bt3_103.rs` (un-block Clause 0, un-ignore 5 tests).
- **Complexity estimate:** Large. Three substantive sub-systems: (1) a net-new closure-bearing player-scoped store with a single-fire/expiry lifecycle; (2) generalizing the optional/paid reducer accept/decline `PendingSelection` chain from the play-from-hand path to the digivolve path (a refactor touching live cost-calc code on every digivolution); (3) a selection nested inside `pay_cost`/`activation_cost` (interactive cost payment, mask-visible). Each carries regression risk against existing `BeforePayCost` behavior. The cross-archetype `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md` § `G-BG-01` sketch is directionally correct but **stale on one point**: it lists `code/digimon-engine/src/cost_hooks/` as a likely file — no such directory exists; the BeforePayCost machinery lives entirely in `game_actions.rs`. The sketch also predates Track B's `activation_cost` builder hook, which is the natural attachment point for the suspend cost.
- **First test:** `tests/cards_behavioral/bt3/bt3_103.rs` (un-ignore `bt3_103_main_arms_digivolve_cost_reduction_for_turn` and siblings). Play BT3-103 with one unsuspended own Digimon available; attempt a green digivolution; assert a suspend-cost prompt (`PendingSelection`) appears before the reduced cost is paid; **decline** keeps the unreduced cost and leaves the reducer armed; **accept** suspends the selected Digimon and applies `-5` exactly once; a second green digivolution in the same turn must NOT get the reduction (single-fire consumed); a non-green digivolution must never see the prompt (target-color filter).
- **Known interactions / risks:**
  - **Single most important risk:** generalizing the optional/paid reducer pending-selection chain to the digivolve path means inserting an interactive prompt into `scan_before_pay_cost_reduction_with_target` — a function on the hot path of *every* digivolution. It currently returns an `i32` synchronously; converting it to a possibly-suspending flow (pending-selection mid-cost-calc) risks regressing the many existing field-hosted mandatory reducers and the DNA / Blast digivolve cost paths that all call it. The play-from-hand chain proves the pattern is feasible but the digivolve cost-calc has more call sites (normal digivolve, DNA digivolve, Blast) that must each tolerate a `Pending` result.
  - Stacked reducers: a player-scoped reducer plus a field-hosted reducer must compose; processing order and whether each can independently decline must be defined (the play-from-hand chain already threads `processed: Vec<CostReductionKey>` for this).
  - Single-fire timing: "next digivolve" consumes on the first *successful* application — a declined prompt must leave it armed; a green digivolution where the player has no Digimon to suspend (cost-impossible) must also leave it armed (or define explicitly), unlike `activation_cost`'s silent-collapse-consumes-OPT rule.
  - `CannotReduceDigivolveCost` / `OpponentCannotReduceDigivolveCost` flood-gates (`collect_before_pay_cost_reducers`, `game_actions.rs:4025-4033`) must continue to suppress the player-scoped reducer too.

## Deferred — verification / test coverage only

Items where the existing primitive **likely works** but no behavioral test covers the specific pathway. Not engine gaps; filed here so they surface when the archetype moves to the Rust DSL implementation workflow and a faithful DebugRunner test must be written. **Do not count toward BLOCKING / PARTIAL tallies.**

- **Tamer play-from-security pipeline** — `ctx.play_from_security` was written against `CardKind::Digimon`; `CardKind::Tamer` routing through the same path + subsequent `[Your Turn]` / `[All Turns]` observers is unverified. Cards: BT17-081 Tai Kamiya & Matt Ishida, BT22-089 Mirei Mikagura, BT5-092 Nokia Shiramine, EX9-066 Tai Kamiya & Matt Ishida, ST20-15 Island of Adventure, EX4-061 Matt Ishida & Tai Kamiya (DNA Omnimon); Dark Masters adds: BT8-090 Kari Kamiya, ST6-14 Matt Ishida, BT4-097 Kari Kamiya, BT8-094 Digimon Emperor, BT13-102 Keenan Crier, EX9-068 Analogman, RB1-035 Hokuto Amanokawa (all Tamer security plays). See RUST_PYTHON_PARITY §2.5a, §2.5j.
- **Option multi-color match semantics** — RUST_PYTHON_PARITY §4.2 implements color match; verify multi-color Options require at least one matching own-side permanent **per** printed color (intersection), not any-one (union). Card: BT17-095 Miraculous Mega Knight (Red/Blue Option, DNA Omnimon). See RUST_PYTHON_PARITY §4.2, §4.2b.
- **Conditional inherited DP based on top-card name** — fully expressible today via `Effect::inherited(card).dp_modifier(n).condition(|ctx| ctx.source_permanent().map_or(false, |p| p.contains_card_name("X", ctx.card_data())))`. Confirm the per-source walker passes the correct `source_permanent` into the read context. Cards: BT12-059 Agumon, BT23-008 Greymon (DNA Omnimon).

### `OnDigivolutionCardTrashed` observer timing (card-source leaves a digivolution stack via effect)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-ondigivolutioncardtrashed-observer-timing--resolved-2026-05-15-phase-1-pr-449-2026-05-07-routing-fan-out) by the 2026-05-15 hygiene sweep.

### `<Fragment (N)>` keyword — leave-field replacement via N-source self-trash
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-fragment-n-keyword--leave-field-replacement-via-n-source-self-trash--resolved-2026-05-15-phase-d-2026-04-25) by the 2026-05-15 hygiene sweep.

### `<Piercing>` combat-time security continuation after a winning battle
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-piercing-combat-time-security-continuation-after-a-winning-battle--resolved-2026-05-15-group-6-task-4) by the 2026-05-15 hygiene sweep.

### `ModifierType::GrantCollision` + `combat::try_enter_block` honoring granted Collision
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-modifiertypegrantcollision--combattry_enter_block-honoring-granted-collision--resolved-2026-05-15-group-6-task-4) by the 2026-05-15 hygiene sweep.

### Cross-permanent count-capped multi-select (single-source and up-to-N across own stacks)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-cross-permanent-count-capped-multi-select-single-source-and-up-to-n-across-own-stacks--resolved-2026-05-15-group-2-2026-04-29) by the 2026-05-15 hygiene sweep.

### `.pay_cost()` builder hook for triggered non-cost-reduction effects (extends Dynamic cost reduction to arbitrary triggered bodies)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-pay_cost-builder-hook-for-triggered-non-cost-reduction-effects--resolved-2026-05-15-group-3) by the 2026-05-15 hygiene sweep.

### Source-scoped return-immunity modifiers (`CannotBeReturnedToHand` / `CannotBeReturnedToDeck` / `CannotBeDeDigivolved` by-opponent-effects-only)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-source-scoped-return-immunity-modifiers-cannotbereturnedtohand--cannotbereturnedtodeck--cannotbededigivolved-by-opponent-effects-only--resolved-2026-05-15) by the 2026-05-15 hygiene sweep.

### Conditional security-in-stack trigger (`[Security] [End of Opponent's Turn]` / `[Security] [Start of Your Turn]` etc.)
- **Severity:** 🟡 PARTIAL (audit 2026-05-23: narrowed; BT20-055's `[Security] [End of Opponent's Turn]` self-play and face-up security rider are closed. Residual: start-of-turn / start-of-opponent-turn security-stack timing variants need boundary-iteration extension to `begin_turn` / `rotate_turn_player`.)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** No active Rocks blocker remains. Originally surfaced by BT20-055 Invisimon; residual timing variants are kept for future security-stack cards with start-of-turn style text.
- **Effect text:** As above.
- **What's missing:** Current security-effect plumbing (RUST_PYTHON_PARITY §2.5a) fires `SecuritySkill` effects only when a security card is revealed during an attack's security check. A subset of cards carry security-slot effects that gate on **global turn-phase timings** while the card remains face-down in the stack. No scheduling pass iterates each security card's effects at turn boundaries. A dedicated `play_from_security_at(player, security_index)` path is required (distinct from the attack-time `play_from_security()` which reads `pending_security`).
- **Suggested API shape:** Add `EffectTiming::SecurityOnStartYourTurn` / `SecurityOnEndYourTurn` / `SecurityOnStartOpponentsTurn` / `SecurityOnEndOpponentsTurn` variants (or extend `SecuritySkill` with a turn-boundary gate). Iterate each player's security stack at `begin_turn` / `end_turn`, enqueue matching effects (the iterator must include face-down cards; the card text explicitly activates from security without being revealed by an attack). Add `ctx.play_from_security_at(player, index)` popping the indexed security card and playing it without paying cost.
- **Status (2026-05-08):** Narrowed for `[Security] [End of Opponent's Turn]` self-play. DSL `scope: security` now compiles to security-zone effects; `rotate_turn_player` scans the non-ending player's persistent security stack for `EndOfOpponentsTurn`; `play_from_security` removes the exact source card rather than blindly popping the top. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- security_scope_end_of_opponents_turn_plays_this_security_card`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_055_security_end_of_opponents_turn_plays_self_from_security`.
- **Remaining:** start-of-turn/start-of-opponent-turn security-stack timing variants.
- **Workaround:** None — BLOCKED. Without boundary iteration, Invisimon's defining control text never activates.
- **Related:** RUST_PYTHON_PARITY §2.5 (security-effect execution); existing "Zone-manipulation: play-from-hand / trash without paying cost" (free-play entry needed for the play path).

### Effect-driven attack cancellation (`ctx.end_pending_attack()`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-driven-attack-cancellation-ctxend_pending_attack--resolved-2026-05-15-group-3--track-d-2026-05-0708) by the 2026-05-15 hygiene sweep.

### Declarative-aura → player-scoped modifier delivery (bilateral, `UntilLeaveField`)
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: tick-driven path with `Expiry::Permanent` works; full `UntilLeaveField` lifecycle still incomplete — source-leave cleanup, duplicate-source semantics, and dedicated flood-gate tick coverage remain follow-ups.)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT14-009 Gotsumon (`[All Turns] Players can't play Digimon by effects.`)
- **Effect text:** As above.
- **Status 2026-05-02:** Declarative aura DSL now accepts `target_player` and lowers `modifier` to a player-scoped modifier when `target_player` is present. `Game::tick_declarative_effects` installs the player modifier from a face-up field source. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- group6_auras --nocapture`.
- **What's still missing:** The covered DSL path installs `Expiry::Permanent`; this does not close the full bilateral `UntilLeaveField` lifecycle problem for BT14-009-style cards. Source-leave cleanup, duplicate-source semantics, and dedicated flood-gate tick coverage remain separate follow-ups.
- **What was missing:** The existing "Player-scoped modifier registry" entry assumes imperative application via `ctx.add_player_modifier(...)`. BT14-009's clause is a **declarative passive aura** that (a) applies to **both players** from a single source permanent, (b) activates while the source is on the battle area, (c) deactivates when the source leaves the field (`Expiry::UntilLeaveField`). Naïve `OnPlay`-applies + `OnDeletion`-revokes is fragile: if the permanent is trashed via a path that skips `OnDeletion` fan-out, or if two BT14-009s are on the field and one leaves, the modifier should not drop. Needs an aura-query model (re-evaluated on demand) or a `ModifierRegistry` bucket keyed by source `PermanentHandle` so removal on leave is automatic.
- **Suggested API shape:** `Effect::declarative(card).grants_player_modifier(ModifierType::CannotPlayDigimonByEffect, scope: ModifierScope::BothPlayers)` where `ModifierScope = { Self, OpponentOnly, BothPlayers }`. Evaluated as an aura-query whenever `Game::can_play_digimon_by_effect(player)` is consulted — avoids materialization pitfalls. Alternative: materialize into a new `ModifierRegistry::PlayerModifierFromPermanent { source, target_players, modifier }` bucket cleared in `delete_permanent` / `return_permanent_*`.
- **Workaround:** For simple static tests, `kind: aura` with `target_player` can materialize the player modifier through `tick_declarative_effects`. Full `UntilLeaveField` fidelity still needs lifecycle cleanup.
- **Related:** Existing "Player-scoped modifier registry" (delivery-shape extension); existing "Named-target declarative aura" (aura-query evaluation pattern — sibling); existing "Condition-gated modifier entries" (evaluate-at-query-time principle).

### DigiXros name alias (`treated as [X] for DigiXros`)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-digixros-name-alias-treated-as-x-for-digixros--resolved-2026-05-15-group-8-2026-05-02) by the 2026-05-15 hygiene sweep.

### Global `OnOptionCardTrashed` observer timing
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; Track I substrate slice 2026-05-10 wired `OnOptionTrashed`, `TriggerSource::OptionTrashed`, `Game::trash_field_option`, `EffectContext::option_last_field_state`. Residual: route legacy Option trash paths through this API — standard dispose, Delay expiry/activation, linked-card cascade, security trash-after-resolve — and extend fan-out to hand/trash/security-resident observers if needed.)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT23-059 Justimon: Blitz Arm (`[All Turns] [Once Per Turn] When Option cards in the battle area are trashed, this Digimon unsuspends. Then, your opponent's Digimon's effects don't affect this Digimon for the turn.`)
- **Effect text:** As above.
- **What's missing:** Options that live on the battle area (Plug-Ins after link, Delay-placed Options, Option-as-cost-trash here) can exit via a trash path that bypasses `delete_permanent`. Existing `OnDeletion` fires only on permanent deletion, not on Option-specific trash; existing `OnTrash` in the enum is not dispatched for this case. Need a global fan-out filtered to `CardKind::Option` so an unrelated Digimon can observe the trash.
- **Suggested API shape:** Introduce `EffectTiming::OnOptionTrashedAnywhere` (or the broader parametric `OnCardKindTrashed(CardKind::Option)`). Fire from every Option-leave-battle-area path: cost-pay trash, resolve-and-trash, Delay activation trash, link-unlink trash. Fan out globally to battle-area / hand / inherited-stack observer effects with context `{trashed_card_id, former_controller}`. Couples with the Option card play flow gap.
- **Updated 2026-05-10 (Track I substrate slice):** Narrowed. `EffectTiming::OnOptionTrashed`, `TriggerSource::OptionTrashed`, `EffectContext::option_last_field_state()`, and `Game::trash_field_option(option_handle, cause)` now cover explicit lifecycle API trashes for standalone persistent field Options. Payload includes `event_card`, `event_cause`, `moved_card_sets`, and the last `OptionFieldState`; fan-out currently covers battle-area and breeding observers through the standard trigger queue. Remaining open work: wire every legacy Option trash path (standard dispose, Delay expiry/activation, linked-card cascade, security trash-after-resolve) through this API, and extend fan-out to hand/trash/security-resident observers if Track A requires those zones. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- **Workaround:** None — BLOCKED.
- **Related:** Existing "Option card play flow + Plug-In / Link mechanic"; existing "Global `OnAnyDigimonPlayed` / `OnAnyDeletion` observer timings" (same architectural class of global fan-out).

### Plug-In re-link from battle area source zone
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; Track I 2026-05-10 added `OptionFieldState::{LinkedPlugIn, OrphanedPlugIn}`, `Game::orphan_linked_plug_in`/`orphan_plug_in`/`relink_plug_in`. Residual: route carrier-loss cascades through orphaning where rules require survival, surface orphan candidates through pending selections, and lower the source-zone vocabulary into DSL.)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** ST22-11 Defense Plug-In F (inherited Link Requirements explicitly allow "plug this card from the **hand or battle area** sideways into the specified Digimon in the battle area")
- **Effect text:** As above.
- **What's missing:** The existing Plug-In sub-gap inside "Option card play flow" scopes the source zone to the hand. ST22-11 allows re-linking from the battle area — a Plug-In already on the field (unlinked or linked to a now-gone Digimon) can transfer to a new carrier. Needs a three-zone Plug-In state model: hand→link, battle-area-free→link, linked→battle-area-free on carrier loss.
- **Suggested API shape:** `ctx.link_plug_in(source: PlugInSource, target: PermanentHandle)` where `PlugInSource = Hand(index) | BattleArea(permanent_handle)`. Engine maintains per-permanent `linked_cards: Vec<CardSource>` and a reverse-lookup from orphaned Plug-In permanents. On carrier loss, orphaned Plug-Ins return to battle-area-free state (distinct from trash and distinct from return-to-hand).
- **Updated 2026-05-10 (Track I substrate slice):** Partially narrowed. `option_lifecycle::OptionFieldState` now defines `LinkedPlugIn` and `OrphanedPlugIn`; `Game::orphan_linked_plug_in`, `Game::orphan_plug_in`, and `Game::relink_plug_in` provide the observer-safe storage transitions for orphan/re-link flows. Remaining open work: route automatic carrier-loss cascades through orphaning instead of the existing trash cascade where printed rules require survival, surface orphan candidates through pending selections, and lower the source-zone vocabulary into DSL. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow`.
- **Workaround:** None — BLOCKED.
- **Related:** Existing "Option card play flow (resolve + trash vs. place-on-field; [Main]/[Security] activation) + Plug-In / Link mechanic".

### ~~`ctx.move_from_breeding()` EffectContext helper~~ — RESOLVED 2026-05-23
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#rocks-b2-move-from-breeding-dsl-step--resolved-2026-05-23). The P-130 optional level-filtered prompt wrapper now ships through the `move_from_breeding` DSL step and the existing `EffectContext::move_from_breeding_by_effect` path.

### `ModifierType::CannotAddSecurityByEffect` (player-scoped opponent-security-placement block)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-modifiertypecannotaddsecuritybyeffect-player-scoped-opponent-security-placement-block--resolved-2026-05-15-track-cd-2026-05-08) by the 2026-05-15 hygiene sweep.

### `<Digi-Burst N>` keyword — trash-N-own-digivolution-cards as [Main] activation cost
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-digi-burst-n-keyword--resolved-2026-05-15-track-g-pr-457) by the 2026-05-15 hygiene sweep.

### `<Decoy>` color-filter parameterisation (Track G close)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-keyworddecoy-color-filter-parameter--replacement-framework-wiring--resolved-2026-05-15-phase-d--track-g-pr-457) by the 2026-05-15 hygiene sweep.

### `<Evade>` printed semantics — suspend-and-cancel, NOT redirect-to-deck (Track G close)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-evade-printed-semantics--suspend-and-cancel-not-redirect-to-deck--resolved-2026-05-15-track-g-pr-457) by the 2026-05-15 hygiene sweep.

### `<Progress>` card-shaped test backfill (Track G close)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-progress-card-shaped-test-backfill--resolved-2026-05-15-track-g-pr-457) by the 2026-05-15 hygiene sweep.

## Puppets Batch 5/6 Residual Engine Gaps

### Costed self-digivolve stable source binding
> Resolved by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`, 2026-05-20). Stable `source_permanent` re-locate by `CardHandle` across mid-body delete (`PUPPETS-G018`) landed. Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Inherited Token/Puppet leave-prevention replacement dispatch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-inherited-tokenpuppet-leave-prevention-replacement-dispatch--resolved-2026-05-15-track-b-2026-05-08) by the 2026-05-15 hygiene sweep.

### Effect-played permanent cleanup provenance
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-played-permanent-cleanup-provenance--resolved-2026-05-15-track-a-pr-451) by the 2026-05-15 hygiene sweep.

### Suspend-this-Tamer deletion observer with Overclock cause branch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-suspend-this-tamer-deletion-observer-with-overclock-cause-branch--resolved-2026-05-15-2026-05-06) by the 2026-05-15 hygiene sweep.

### Narrow opponent-effect protection for DP reduction and De-Digivolve
> Resolved by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`, 2026-05-20). Security-gated narrow opponent-effect protection (`grant_narrow_opponent_effect_protection`, `PUPPETS-G024`) landed. Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Trash-resident observer with effect digivolve from trash
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-trash-resident-observer-with-effect-digivolve-from-trash--resolved-2026-05-15-2026-05-06) by the 2026-05-15 hygiene sweep.

### Effect play with played-Digimon On Play suppression
> Resolved by the Puppets substrate sweep (branch `claude/stoic-moser-0ef79e`, 2026-05-20). `suppress_on_play` flag on effect-play helpers (`PUPPETS-G030`) landed. Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md). DSL `suppress_on_play: true` is honored ONLY by `play_from_trash_free` (compiled to `play_from_trash_free_unsuspended_suppress_on_play`); the compiler rejects it on `play_from_hand` / `play_from_trash`. BT5-106's [Security] slice is authored in `code/digimon-engine/cards/bt5/BT5-106.yaml`.
>
> **Deferred follow-up:** `suppress_on_play` for `play_from_materials` (Royal Knights source-play payoffs that play a Digimon from a digivolution-source stack with its [On Play] suppressed) is NOT yet wired — the merged engine threads suppression only through `play_from_trash_free`. Re-wiring it for the `play_from_materials` path is follow-up work for when the RK source-play cards are authored.

### End-of-attack mandatory self-delete chain with recovery and conditional hatch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-end-of-attack-mandatory-self-delete-chain-with-recovery-and-conditional-hatch--resolved-2026-05-17-track-i) by the 2026-05-17 Track I first-test confirmation. Existing primitives (`delete_permanent { target: source }`, `select_opponent_permanent { optional: true }`, `recover`, `if { any_field_permanent + can_hatch } then hatch`) compose into a faithful chain — see `code/digimon-engine/cards/ex4/EX4-074.yaml` Clause 2 and `code/digimon-engine/tests/cards_behavioral/ex4/ex4_074.rs::ex4_074_end_of_attack_self_deletes_opponent_delete_recovers_and_hatches_with_tamer`.

### BEATBREAK / DATA SQUAD Tamer face-down stash substrate (place under chosen Tamer + cost-form trash bottom face-down)
- **Severity:** 🔴 BLOCKING — Phase A substrate landed 2026-05-17; residual Phases B–F remain blocking (see Status footer)
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); ST-24 DATA SQUAD (2026-05-17)
- **Card(s):**
  - **Place side (face-down deck-top under chosen Tamer):** ST23-06 Gekkomon, ST23-13 Tomoro Tenma & Kyo Sawashiro, ST23-14 Reina Sakuya & Makoto Kuonji, ST24-03 Gaogamon, ST24-09 Sunflowmon, ST24-13 Marcus Damon & Thomas H. Norstein, ST24-14 Yoshino Fujieda & Keenan Crier.
  - **Place side (face-down hand card under chosen Tamer):** ST23-10 Pristimon, ST24-02 Gaomon.
  - **Cost side (trash bottom face-down source from chosen own Tamer):** ST23-01 Kekkomon, ST23-03 Cougarmon, ST23-04 Murasamemon, ST23-08 Monarchlizamon, ST23-11 Wolvermon, ST23-12 Chiropmon, ST24-01 Koromon, ST24-06 RizeGreymon, ST24-10 Lilamon, ST24-11 Rosemon, ST24-12 Falcomon. Also the inherited [End of Attack] unsuspend on ST23-04/08 and the inherited self-or-friend-set leave-prevention on ST23-05/ST24-06/ST24-10. Sibling face-down placement card EX9-068 Analogman from the existing "Alt-digivolve with override-cost + ignore-reqs + face-down placement" entry shares the engine-side `face_down: bool` axis.
- **Effect text:** "place the top card of your deck face down under this Tamer" / "place 1 such card face down under any of your [Glowing Dawn]/[DATA SQUAD] trait Tamers" / "By placing 1 card from your hand face down under any of your Tamers, ..." / "by trashing the bottom face-down card from under any of your Tamers, ..."
- **What's missing:** Four coupled holes that together unblock the entire ST-23/ST-24 Tamer-stash archetype:
  1. **DSL `face_down: bool` axis on `place_as_bottom_source`** — the engine `CardSource::face_down: bool` field exists (`code/digimon-engine/src/card_source.rs:37`) but only `<Training>`'s hardcoded `training_place_deck_top_under_self_face_down` writes it. The general `EffectContext::place_card_under_permanent_bottom` (`effect_context/mod.rs:2870`) and the DSL `place_as_bottom_source` step always insert face-up. Add a `face_down: bool` parameter (default false) to both the engine helper and the DSL step; thread through `Game::place_as_bottom_source_observed`.
  2. **DSL `CardSourceRef::DeckTop` binding** — `resolve_card_source_ref` returns `None` for `DeckTop`. ST23-06 / ST23-13/14 / ST24-03 / ST24-09 / ST24-13/14 all need the deck top as a source-binding for `place_as_bottom_source { source: deck_top, target: tamer_pick, face_down: true }`. Add a curated `place_deck_top_under_permanent(target, face_down)` `EffectContext` helper and the DSL binding form, OR widen `resolve_card_source_ref` to accept `BindingRef::DeckTop { of: PlayerRef }`.
  3. **New `PredicateSpec` source leaves: `is_face_down: bool`, `is_bottom_source: bool` (or sugar over `source_index_eq: 0`), `host_kind_is: tamer`** — `SelectOwnSourcesArgs.filter` already accepts a `PredicateSpec`, but no leaf restricts to face-down sources, to the bottom-of-stack position, or to "this source's host permanent is a Tamer". The engine `SourceSelectionRef` (`selection.rs:64-69`) already carries `permanent`, `field_index`, `source_index`, `card` — the engine-side filter closures can express this trivially; only DSL vocabulary is missing.
  4. **Curated `trash_bottom_face_down_source(target)` helper + `has_face_down_source: bool` permanent filter** — the printed cost specifies the bottom face-down card, not any face-down card. Add `EffectContext::trash_bottom_face_down_source(target: PermanentHandle) -> bool` that pops `card_sources[0]` only if `face_down == true`, routes to owner's trash, fires `OnDigivolutionCardTrashed` with `event_host_permanent` set to the Tamer. Pair with `has_face_down_source: bool` on `PermanentPredicate` for the upstream `select_own_permanent { kind: tamer, has_face_down_source: true }` Tamer-pick gate, so the cost fail-cleans when no eligible Tamer exists.
- **Suggested API shape:**
  - Engine: `EffectContext::place_deck_top_under_permanent(target: PermanentHandle, face_down: bool) -> Option<CardHandle>`; widen `place_card_under_permanent_bottom` with `face_down: bool` parameter; `EffectContext::trash_bottom_face_down_source(target: PermanentHandle) -> bool`.
  - DSL steps: `place_deck_top_under_tamer: { of: you, target: <perm-binding|source>, face_down: bool }`, extend `place_as_bottom_source` with `face_down: bool`, new step `trash_bottom_face_down_source_under_tamer: { of: you }` (with internal two-stage `select_own_permanent` { kind: tamer, has_face_down_source: true } → `trash_bottom_face_down_source { target: pick }`).
  - New `PredicateSpec` leaves: `is_face_down: Option<bool>`, `is_bottom_source: Option<bool>`, `source_position_eq: Option<u8>`, `host_kind_is: Option<CardKind>`, `host_permanent_trait_has: Option<String>`. New `PermanentPredicate` leaf: `has_face_down_source: Option<bool>`.
- **Workaround:** None faithful. Auto-picking the Tamer or trashing a face-up source violates §17 no-approximations; substituting face-up placement breaks the downstream `is_face_down` cost predicate AND the tensor's face-down visibility convention (`tensor_v2_lite.rs:153,171,175`); skipping the cost makes rider effects free.
- **Related:** Existing "[Alt-digivolve with override-cost + ignore-reqs + face-down placement](#alt-digivolve-with-override-cost--ignore-reqs--face-down-placement)" (sibling `face_down: bool` axis on `place_as_bottom_source` already filed for EX9-068 — this entry expands the card list and adds the cost-form trash + DSL predicates); existing "[`<Training>` keyword](#training-keyword)" (the only existing face-down placement helper — scoped to self, deck-source only); existing "[Generic `.activation_cost(...)` builder hook for triggered abilities](#generic-activation_cost-builder-hook-for-triggered-abilities-suspend-self--pay-as-cost-on-triggered-abilities)" (the cost-form here also exercises that hook for ST23-04/08/10 inherited and triggered bodies).
- **Status — Phase A landed (2026-05-17):** The Tamer face-down stash substrate is implemented on branch `claude/nostalgic-saha-3ddfce` per [`docs/superpowers/plans/2026-05-17-rust-engine-tamer-face-down-stash-substrate.md`](../docs/superpowers/plans/2026-05-17-rust-engine-tamer-face-down-stash-substrate.md):
  - A1 — `face_down` axis on `place_card_under_permanent_bottom` / `place_as_bottom_source` (engine + DSL step). `face_down` is not honored for `CardSourceRef::Security` sources (always face-up, DCGO parity).
  - A2 — `place_deck_top_under_permanent` helper + `{ deck_top: <player> }` DSL binding (`StructuredBindingRef.deck_top`).
  - A3 — `is_face_down` / `is_bottom_source` / `host_kind_is` / `has_face_down_source` predicate leaves + `PredicateSubject::Source`.
  - A4 — `trash_bottom_face_down_source` helper + `trash_bottom_face_down_source_under_tamer` DSL verb. The helper does not honor `ImmuneFromStackTrashing` (voluntary cost, not involuntary peeling).
  - A5 — Tamer-host `OnDigivolutionCardTrashed` dispatch coverage confirmed.
  The placement + cost-form trash + DSL predicate trio called out as "what's missing" in this entry is now closed. The engine/DSL API surface is documented in `docs/RUST_ENGINE_API.md` (§ Placement, § Track E zone-movement DSL verbs, § DSL Tamer Face-Down Stash Substrate). The remaining ST-23/ST-24 gaps (Phases C–F of the fix-plan: the Option-lifecycle exit, unified play-or-use, `BeforePayCost` selection-bearing `pay_cost_fn`, and the cost-reduction target-card predicate trigger — each filed as its own entry below) are NOT addressed by Phase A and remain open. The 🔴 BLOCKING severity now applies only to those residual non-substrate gaps. `event_host_permanent_is_source` closed on 2026-05-23; see `qa/resolved-gaps.md`.

### Move existing field-Option face-down under chosen own permanent (new Option-lifecycle exit, distinct from trash)
- **Status:** ✅ CLOSED (2026-06-15). `Game::move_field_option_under_permanent(option, target, face_down)` + the `EffectContext::move_self_option_under_permanent(target, face_down)` wrapper (`option_lifecycle.rs` / `effect_context/action/lifecycle.rs`) pop the field Option's top `CardSource` and push it (optionally face-down) under the chosen own permanent's digivolution stack, firing NEITHER `OnOptionTrashed` (the lifecycle is "moved", not "trashed") NOR `OnDigivolutionCardTrashed` (no source is trashed). The target is resolved by stable top-card identity before the Option removal so battle-area index shifts don't misroute. DSL verb `move_self_option_under_permanent: { target, face_down: true }` (default `face_down: true`). Driver ST23-15 e-Pulse ships IMPLEMENTED (`cards/st23/ST23-15.yaml`, `tests/cards_behavioral/st23/st23_15.rs` 7/7); ST24-15 DNA Charge uses the same substrate. The `<Draw 1>` + memory gain run after a successful relocate; the relocate is declinable (DCGO `canNoSelect: true`).
- **Severity:** 🔴 BLOCKING (now resolved)
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST23-15 e-Pulse, ST24-15 DNA Charge — both Option cards with `[Start of Your Main Phase] By placing this card from the battle area face down under any of your [BEATBREAK]/[DATA SQUAD] trait Tamers, <Draw 1> and gain 1 memory.`
- **Effect text:** "By placing this card from the battle area face down under any of your [BEATBREAK]/[DATA SQUAD] trait Tamers, <Draw 1> and gain 1 memory."
- **What's missing:** `EffectContext::place_card_under_permanent_bottom` accepts a `CardHandle` and `EffectContext::place_as_bottom_source` accepts `CardSourceRef::{Hand, Trash, DeckTop, Security, Material, Reveal}` — neither has a `BattleAreaPermanent(PermanentHandle)` source variant for relocating an already-on-field Option permanent into another permanent's digivolution stack. This is distinct from `attach_tamer_to_digimon` (which keeps the Tamer as the top card and is for the printed `[Hand][Main]` Tamer-as-Plug-In flow); ST23-15 / ST24-15 retire the Option's standalone `OptionFieldState::OrdinaryFieldOption` lifecycle and insert the Option's top `CardSource` as a face-down digivolution source beneath the chosen Tamer's existing top. Must NOT fire `OnOptionTrashed` (the card isn't trashed — it moves zones).
- **Suggested API shape:** `EffectContext::move_self_option_under_permanent(target: PermanentHandle, face_down: bool) -> bool`. Internally: pop the source-permanent's top `CardSource`, clear that permanent's modifier entries via existing `clear_permanent_modifiers`, mark the Option's lifecycle as moved-not-trashed (new cause variant distinguishing `OptionMoveCause::MovedUnderPermanent` from `OptionTrashCause::*`), then push under target with `face_down`. Skip both `OnDigivolutionCardTrashed` (no source trash) and `OnOptionTrashed` (lifecycle is "moved," not "trashed"). DSL: `move_self_option_under_permanent: { target: tamer_pick, face_down: true }`. Couples with the BEATBREAK/DATA SQUAD Tamer-stash placement substrate above.
- **Workaround:** None faithful. Trashing the Option and creating a face-down source from elsewhere isn't the printed move — the card's identity and location are observably different (no `OnOptionTrashed` event, no trash placement).
- **Related:** Existing "[Option card play flow residual: place-Option-in-battle-area + [Hand][Main] Plug-In flow](#option-card-play-flow-residual-place-option-in-battle-area--handmain-plug-in-flow)" (this is a follow-up Option-lifecycle move on top of in-battle-area persistence); BEATBREAK / DATA SQUAD Tamer face-down stash substrate (sibling face-down placement family).

### `BeforePayCost` cost-reducer with selection-bearing `pay_cost_fn` (Parked-outcome handling)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); BT25 thomas-data-squad slice (2026-06-06)
- **Card(s):** ST23-03 Cougarmon ("[Your Turn] When this Digimon would digivolve into a [Glowing Dawn] trait Digimon card, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 2"). BT25 adds: **BT25-087 Thomas H. Norstein** clause 3 ("[Your Turn][OPT] When any of your Digimon would digivolve into a [DATA SQUAD] trait Digimon card, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 1") and **BT25-096 Mirage Beast Knight** clause 1 ("When this card would be used, by trashing the bottom face-down card from under any of your Tamers, reduce the use cost by 2"). The pay-cost is `trash_bottom_face_down_source_under_tamer`, whose Tamer-pick `select_own_permanent` always installs a `PendingSelection` (no auto-resolve even for one candidate — see `code/digimon-engine/src/dsl_cards/step/selections.rs:351-353`), so the `before_pay_cost` closure (`lower_cost_reduction.rs:193-197`, `matches!(.., RunOutcome::Synchronous)`) returns `false` and the reduction is dropped. Likely also applies to any future card whose printed text reads "When this Digimon would digivolve / when this card would be used, by trashing X (selection-bearing cost), reduce the cost by N".
- **Effect text:** As above.
- **What's missing:** The resolved gap "Dynamic cost reduction at `BeforePayCost`" (qa/resolved-gaps.md 2026-05-15 Group 3) closed selection-installing costs on **triggered** effects (`run_queued_effect` dispatch), but the `BeforePayCost` dispatch site retains the v1 synchronous-only constraint per `RUST_ENGINE_API.md` §11.5: "The closure is synchronous and must NOT install a `PendingSelection` inside it." The DSL lowering at `code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs:153-160` returns `matches!(..., RunOutcome::Synchronous)` — if cost steps install a `PendingSelection` (returning `Parked`), the closure returns `false` and the cost reduction is silently dropped (the cost selection itself runs, but the reducer's contribution is excluded).
- **Suggested API shape:** Extend the `BeforePayCost` dispatch site with a two-phase "park, pay, then re-enter scan" flow mirroring the optional-reducer accept/decline pattern (`code/digimon-engine/tests/cost_hooks/stacked_would_play_reducers.rs`). The reducer's contribution accumulates only after the selection resolves to a paid cost. Update `lower_cost_reduction.rs` to surface `Parked` outcomes instead of silently dropping them. Update API doc §11.5 v1 constraint once landed.
- **Workaround:** None — BLOCKED. Auto-paying violates §17 (hidden auto-selection across multiple Tamer candidates); omitting the reduction makes the entire cost-reduction clause non-functional.
- **Related:** Resolved "Dynamic cost reduction at `BeforePayCost` (closure-valued + selection-gated + suspend/self-return as cost)" — the selection-gated variant was claimed resolved but the regression coverage exercises only synchronous costs (suspend, trash-from-top, condition gating); no test covers `select_own_sources` inside a `BeforePayCost` `pay_cost_fn`. Couples with BEATBREAK / DATA SQUAD Tamer face-down stash substrate (Cougarmon's specific cost shape).

### Cost-reduction trigger with target-card trait / name predicate ("when this/any Digimon would digivolve into a {trait/name} hand card")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); previously surfaced in BT5-092 / BT23-005 audits
- **Card(s):** ST23-11 Wolvermon ("When this Digimon would digivolve into a [Glowing Dawn] trait Digimon card, by trashing the bottom face-down card from under any of your Tamers, reduce the cost by 2"); BT21-011 Shoutmon ("When this Digimon would digivolve into a Digimon card with the [Xros Heart] or [Hero] trait, reduce the digivolution cost by 1"); BT5-092 Nokia Shiramine ("When one of your Digimon would digivolve into a Digimon card in your hand with [Greymon], [Garurumon] or [Omnimon] in its name" — BT5-092.yaml header documents this gap); BT23-005 (per BT5-092 YAML header note); likely extends to every BEATBREAK Lv4 finisher.
- **Effect text:** As above.
- **What's missing:** `CostReductionBody` (`digimon-dsl/src/clause.rs:323-349`) exposes only `when_playing_this: bool` and `when_any_ally_played: PredicateSpec`. It has no `when_this_digivolves_into` / `when_any_ally_digivolves_into` trigger variants keyed on the **target** card's name/trait/level/color. The engine path `scan_before_pay_cost_reduction` does not thread the digivolution-target `CardSource` through to the condition closure, so a predicate could not inspect the target's properties even if the trigger variant existed.
- **Suggested API shape:** Add `when_this_digivolves_into: Option<PredicateSpec>` and `when_any_ally_digivolves_into: Option<PredicateSpec>` to `CostReductionBody`, where the predicate evaluates against the target card source (the hand card being digivolved INTO). Add target-side predicate leaves (`target_name_contains`, `target_trait_has`, `target_level_eq/lte/gte`, `target_color_has`), or reuse `event_card_*` predicate family after verifying scope semantics. Thread the target `CardSource` through `scan_before_pay_cost_reduction` → `EffectReadContext` so the predicate can see it.
- **Workaround:** None faithful. Omitting the trigger over-fires on every digivolve; auto-selecting BEATBREAK targets violates §17.
- **Related:** Existing "[Conditional digivolve-target restriction (filter on candidate top-card name/trait/level/color)](#conditional-digivolve-target-restriction-filter-on-candidate-top-card-nametraitlevelcolor)" (sibling restriction shape on the same target; this entry adds the cost-reduction trigger shape on the same target predicate substrate); BT5-092.yaml header `when_this_digivolves_into` anchor; qa/dsl-vocab-gaps.md.

### Unified `play_or_use_from_hand_free` helper (kind-bridging Digimon/Tamer/Option/Dual + non-Main phase lift)
- **Severity:** ✅ RESOLVED (2026-06-15) — for the cost-reduced/reduce/fixed/free variant. One residual sub-shape (a play-or-use cost CAP) remains, narrowed below.
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST23-04 Murasamemon, ST23-08 Monarchlizamon ("play or use 1 [Glowing Dawn] trait card from your hand with the cost reduced by 3") — **IMPLEMENTED**; BT25-041 Murasamemon (2-way pay-cost → play/use [Glowing Dawn] at -3) — **IMPLEMENTED**; ST24-06 RizeGreymon ("play or use 1 [DATA SQUAD] trait card with a play or use cost of 5 or less from your hand without paying the cost") — the unified verb + free CostDelta now exist; the only missing piece is the upstream **cost-cap filter** (`play_or_use_cost_lte: 5`), see residual below. "Play or use" is the standard Aces/BEATBREAK printed wording covering Digimon/Tamer (play) and Option/Dual-Option-face (use).
- **Effect text:** As above.
- **Resolution (2026-06-15, `G-PLAY-OR-USE-FROM-HAND`):**
  - `EffectContext::use_option_from_hand_with_cost(player, hand_index, CostDelta) -> OptionPlayResult` (`effect_context/action/play.rs`) — the Option-USE analogue of `play_from_hand_with_cost`. Routes to `Game::use_option_from_hand_with_cost`, which converts `CostDelta` → a new `OptionCostPolicy::{Reduce(i16), Fixed(i16)}` (alongside the prior `Pay`/`Free`) consumed by `play_option_core`'s `effective_cost` computation; the flat reduction STACKS on field-hosted `BeforePayCost` `OptionUse` reductions (same precedent as `CostDelta::Reduce` on the play half).
  - The Main-phase gate is LIFTED for an effect-driven Option use via a transient `Game::effect_driven_option_use` flag (mirrors `in_counter_window`); set around the `play_option_core` call inside the helper and kept set across a parked `Pending` (re-entries need the lifted gate). `Game::play_option_from_hand` defensively resets the flag so a manual top-level Option play never inherits a stuck lift.
  - `EffectContext::play_or_use_from_hand_with_cost(player, hand_index, CostDelta)` — single entry that inspects `CardKind`: Digimon/Tamer/DigiEgg → `play_from_hand_with_cost`, Option → `use_option_from_hand_with_cost`, **Dual** → a `select_effect_choice` with labels `["Play as Digimon", "Use as Option"]` whose callback routes to the play or use path (face choice is part of the use, §17).
  - DSL verb `play_or_use_from_hand: { of, hand_index: <binding>, cost_delta: <CostDelta> }` (`PlayOrUseFromHandArgs` → `CompiledStep::PlayOrUseFromHand`, run in `dsl_cards/step/play_digivolve.rs`).
  - Upstream `select_hand` multi-kind filter: **no new vocabulary needed** — a `select_hand` filter that simply omits `kind` (e.g. `{ trait_has: "Glowing Dawn" }`) already admits every kind, and the unified verb does the kind routing at resolution. ("play or use 1 [trait] card" has no kind restriction.)
  - Tests: `tests/cards_behavioral/st23/st23_04.rs`, `st23_08.rs`, `tests/cards_behavioral/bt25/bt25_041.rs` (all green).
- **Residual (still open, NARROWED):** the **play-or-use cost-CAP filter** `play_or_use_cost_lte: N` — a `select_hand` formula/filter leaf that admits a hand card iff `max(printed.play_cost for Digimon/Tamer, option_use_cost for Option) <= N` — is still unimplemented. ST24-06 RizeGreymon needs it ("a play or use cost of 5 or less"). The 3 shipped cards do not (they filter by trait + reduce by 3, no cap), so it was not added. Distinct from existing `play_cost_lte` (which only knows the Digimon/Tamer play cost, not the Option use cost).
- **Related:** Existing "[Option card play flow residual: place-Option-in-battle-area + [Hand][Main] Plug-In flow](#option-card-play-flow-residual-place-option-in-battle-area--handmain-plug-in-flow)"; RUST_ENGINE_API §8 "DUAL cards and Arts Digivolve".

### Filtered hand-or-trash origin-preserving free-play (PUPPETS-G014/G028 promotion)
- **Severity:** ✅ RESOLVED
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); ST-24 DATA SQUAD (2026-05-17); previously filed only in archetype QA (`qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md:163,326`)
- **Card(s):** ST23-15 e-Pulse ("play 1 [BEATBREAK] trait card with a play cost of 4 or less from your hand or trash without paying the cost"); ST24-07 ShineGreymon ("play 1 Tamer card with a play cost of 5 or less from your hand or trash without paying the cost"); ST24-15 DNA Charge (same shape as ST23-15); also Puppets cards ST19-08, BT22-098 already noted in archetype QA.
- **Effect text:** As above.
- **Resolution:** `select_union_zone` now preserves the pick origin in the binding and `play_union_bound_free` replays the bound card from hand, trash, or material. DSL lowering evaluates the selection filter against candidate cards. The 2026-05-22 Medusamon substrate pass extended the origin set to material/source picks for BT13-040's "from hand or this Digimon's sources" shape.
- **Workaround:** None needed for hand/trash/material origin-preserving free play. Broader play-or-use Option routing remains separate under the adjacent "Play or use" gap.
- **Related:** "[Option card play flow residual: place-Option-in-battle-area + [Hand][Main] Plug-In flow](#option-card-play-flow-residual-place-option-in-battle-area--handmain-plug-in-flow)"; PUPPETS-G014/G028 in `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`; unified `play_or_use_from_hand_free` (sibling — RizeGreymon needs both unified play-or-use AND the hand-or-trash origin preservation as a compound shape).

### Player selection by metric (`most_security_cards`, with active-player tie-break)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** ST-23 BEATBREAK (2026-05-17)
- **Card(s):** ST23-05 Habakirimon ("Then, by trashing the top security card of 1 player with the most security cards, <Recovery +1>"). Likely extends to any future card whose printed text picks "the player with the most/least X" where X is a player-scope metric (security, hand, trash, memory, suspended Digimon).
- **Effect text:** As above.
- **What's missing:** No engine helper or DSL verb selects a player by metric. `select_effect_choice` gives N-label branching but doesn't surface a player handle; existing selection helpers all target permanents, cards, or zones. On a tie (both players hold the same max), TCG rules require the resolving-player to choose, so the engine cannot auto-pick without §17 violation. The existing `trash_top_security(player)` (`effect_context/mod.rs:1863`) takes a `PlayerId` — once a player is selected the trash itself is a one-liner.
- **Suggested API shape:** `ctx.select_player_by_metric(metric: PlayerMetric, ordering: Extrema::Most | Extrema::Least, prompt, callback: |&mut EffectContext, PlayerId|)` that filters candidates to those holding the max/min metric, auto-resolves to single candidate, surfaces `PendingSelection` on tie. `PlayerMetric` enum starts with `SecurityCount`, with room for `HandSize`, `TrashSize`, `Memory`, `SuspendedCount`. DSL verb: `select_player_by_metric: { metric: most_security, prompt: ..., bind_as: target_player }` followed by `trash_top_security: { of: target_player }`.
- **Workaround:** None — BLOCKED. Auto-picking the source's controller or the opponent silently violates §17 when both players hold the tied maximum. Routing through `select_effect_choice` doesn't gate on "has the most security cards".
- **Related:** None in this tracker. Adjacent helpers: `select_effect_choice` (label-only branch), `select_opponent_permanents_by_dp_budget` (metric-bounded permanent selection, not player selection).

### Player-scope mass `CannotUnsuspend` aura on opponent (mirror of existing `CannotSuspend` gap)
- **Severity:** 🔴 BLOCKING
- **Discovered in:** ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST24-11 Rosemon ("by trashing the bottom face-down card from under any of your Tamers, none of their Digimon can unsuspend until their turn ends")
- **Effect text:** As above.
- **What's missing:** Sibling of "[Player-scope mass `CannotSuspend` aura on opponent](#player-scope-mass-cannotsuspend-aura-on-opponent-condition-gated-and--or-stack-depth-filtered)". The existing `ModifierType::CannotUnsuspend` is permanent-scope only — there is no broadcast variant that applies to every current and future opponent Digimon until `Expiry::EndOfOpponentsTurn`. DCGO call is `GainCanNotUnsuspendPlayerEffect(permanentCondition, EffectDuration.UntilOpponentTurnEnd)`. Must re-evaluate against newly-played Digimon during its lifetime.
- **Suggested API shape:** Extend the resolution path of the existing `CannotSuspend` player-aura gap to include `cannot_unsuspend()` on the same builder: `Effect::declarative(card).player_aura(opponent_id).cannot_unsuspend().expire_at(Expiry::EndOfOpponentsTurn)`. Implementation parallels the suspend-side: extend player-scoped `ModifierRegistry` with `CannotUnsuspend` entry, consult on every unsuspend-mask query and on the bulk turn-start unsuspend path.
- **Workaround:** Apply `CannotUnsuspend` to each opponent Digimon currently in play at resolution time — fails for future plays (Digimon entering the field during the lockdown window are unaffected), under-narrows "none of their Digimon".
- **Related:** "[Player-scope mass `CannotSuspend` aura on opponent (condition-gated and / or stack-depth-filtered)](#player-scope-mass-cannotsuspend-aura-on-opponent-condition-gated-and--or-stack-depth-filtered)" — same architecture, sister modifier; existing permanent-scope `CannotUnsuspend` (RUST_ENGINE_API.md §5 ModifierType); existing `EndOfOpponentsTurn` expiry (resolved 2026-05-15).

### Shared-OPT across heterogeneous-timing trigger pair with per-timing conditions
- **Severity:** 🟡 PARTIAL
- **Discovered in:** ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST24-11 Rosemon clause 2 ("[All Turns] [Once Per Turn] When any of your opponent's Digimon or Tamers suspend, or effects trash cards from under your Tamers, trash your opponent's top security card")
- **Effect text:** As above. DCGO implements this as two `ActivateClass` instances (one for `OnTappedAnyone`, one for `OnDigivolutionCardDiscarded`) that share `SetHashString("ST24_11_AT")` — DCGO's hash-keyed cross-effect OPT.
- **What's missing:** A way for a single DSL clause to (a) bind `when: [on_suspend, on_digivolution_card_trashed]` (already supported via `TimingSet::Multi`), (b) gate `condition` on the firing timing — apply the suspend-event condition when fired by `OnSuspend` (`event_permanent` populated) and the source-trash condition when fired by `OnDigivolutionCardTrashed` (`event_host_permanent` populated). Today the clause-level `condition` is a single predicate AST evaluated against `current_trigger_context`. An `any_of` composition can approximate it using `event_permanent_is_source` and the now-resolved `event_host_permanent_is_source`, but the composition is fragile because predicate leaves silently return false on the wrong timing.
- **Suggested API shape:** Either (a) add a `when_is: <Timing>` predicate leaf so authors can write `condition: { any_of: [{ all_of: [{when_is: on_suspend}, ...] }, { all_of: [{when_is: on_digivolution_card_trashed}, ...] }] }`; or (b) allow per-timing condition blocks: `triggers: [{ when: on_suspend, condition: {...} }, { when: on_digivolution_card_trashed, condition: {...} }]` inside a single clause, sharing the OPT counter. Option (b) is the cleaner authoring surface.
- **Workaround:** Split into two clauses — each gets its own OPT counter, violating printed "[Once Per Turn]" because the same security trash could fire twice in one turn (once from a suspend, once from a source-trash). Not faithful.
- **Related:** Existing OPT identity model (`(card_id, clause_index)`); existing `event_permanent_is_source` predicate; resolved companion predicate `event_host_permanent_is_source`.

### "Also treated as [X]/[Y]" Tamer name-rule (declarative card-name alias)
- **Severity:** 🟡 PARTIAL
- **Discovered in:** ST-23 BEATBREAK (2026-05-17); ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST23-13 Tomoro Tenma & Kyo Sawashiro, ST23-14 Reina Sakuya & Makoto Kuonji, ST24-13 Marcus Damon & Thomas H. Norstein, ST24-14 Yoshino Fujieda & Keenan Crier. Also covers prior dual-named Tamers (BT16-085 Davis Motomiya & Ken Ichijoji, BT17-081 Tai Kamiya & Matt Ishida, EX9-066 Tai & Matt, EX4-061 Matt & Tai, etc.).
- **Effect text:** "[Rule] Name: Also treated as [Marcus Damon]/[Thomas H. Norstein]" (per DCGO `ChangeCardNamesClass`). Printed text describes a passive name-alias so predicates on other cards ("1 of your [Marcus Damon]" etc.) match this Tamer.
- **What's missing:** The Rust engine has `Permanent::contains_card_name(name)` and a printed `card_data.name` field, but no declarative mechanism to layer additional "treated as" names onto a permanent or onto its underlying `CardData`. DCGO `ChangeCardNamesClass` returns an extended name list when consulted. The Rust engine has no name-overlay surface; cards filtering on "card with [Marcus Damon] in its name" miss this Tamer when checking `top_card.name`.
- **Suggested API shape:** Two options. (a) DSL clause `kind: name_alias` with `names: [<list>]` that lowers to a `CardNameOverlay` registered against the source permanent and consulted by `Permanent::contains_card_name` / `name_matches`. (b) Static `card_data` field `also_treated_as: Vec<String>` populated from YAML metadata for printed name overlays (no DSL effect needed). Option (b) is simpler and consistent with how DCGO's `ChangeCardNamesClass` always returns the same list per card. Engine `name_matches` paths must consult the overlay.
- **Workaround:** Authoring without the overlay leaves the standalone Tamer functional but breaks predicates on other cards searching for `[Marcus Damon]` etc. Future Glowing Dawn / DATA SQUAD support cards that filter Tamers by individual character name will misfire.
- **Related:** Existing "[Digivolution-stack name overlay (\"has all names of materials\")](#digivolution-stack-name-overlay-has-all-names-of-materials)" (different shape — that's stack-derived names; this is printed dual-name Tamers).

### Inherited-effect "Use Requirement: <trait>" activation gating (block-scoped trait gate on inherited Digimon effects)
- **Severity:** 🟡 PARTIAL — primitive-with-fidelity-cost (expressible per-effect via `condition` closure; printed structure diverges)
- **Discovered in:** ST-24 DATA SQUAD (2026-05-17)
- **Card(s):** ST24-07 ShineGreymon (inherited `Use Requirement: DATA SQUAD trait` / `[Main] -6000 DP then delete ≤7000 DP`)
- **Effect text:** "Use Requirement: DATA SQUAD trait / [Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, delete 1 of your opponent's Digimon with 7000 DP or less."
- **What's missing:** Not the same as DUAL Option `use_requirement` (color-substitute when using as Option from hand). This is a **block-level activation gate** on an inherited Digimon effect that resolves only when the carrier (top card of the digivolution stack hosting this source) currently has the specified trait. Today scripts must add `condition(|ctx| trait_for_rules(ctx.source_permanent.top_card_id()).contains("DATA SQUAD"))` to every triggered effect in the inherited block. The action-mask gate for `[Main]` activation also needs the same check before exposing the action ID.
- **Suggested API shape:** A `CardData`-level `inherited_use_requirement: Option<TraitFilter>` field that gates both effect dispatch and mask emission for inherited effects in a single declarative slot. DSL top-level: `inherited_use_requirement: { trait_has: "DATA SQUAD" }` (analogous to existing `use_requirement` but scoped to inherited-block activation).
- **Workaround:** Per-effect `condition` closure on every inherited effect — fidelity-preserving but card-data shape diverges from printed structure; easier to miss a clause during authoring.
- **Related:** Existing "DSL Option Use Requirements" / `use_requirement` (sibling but distinct — Option-use color-substitute vs. inherited-block trait-gate).

### `OnAddToHand` trigger is inert — no firing site, no event-target context ("when effects add cards to your opponent's hand")
- **Severity:** 🔴 BLOCKING
- **Discovered in:** BT25 thomas-data-squad slice (2026-06-06)
- **Card(s):** BT25-087 Thomas H. Norstein (clause 2: "[All Turns] When effects add cards to your opponent's hand, by suspending this Tamer, you may place the top 2 cards of your deck face down under this Tamer"); BT25-029 MirageGaogamon (clause "[All Turns][OPT] When effects add cards to your opponent's hand **or** trash cards from under your Tamers, this Digimon may unsuspend"). DCGO models both via `EffectTiming.OnAddHand` + `CardEffectCommons.CanTriggerWhenAddHand(hashtable, player => player == enemy, cardEffect => cardEffect != null)` — i.e. "the opponent's hand gained ≥1 card **by an effect** (not a draw-step / mulligan)".
- **Effect text:** As above.
- **What's missing:** `EffectTiming::OnAddToHand` exists in `code/digimon-engine/src/enums.rs:240` but is **never enqueued** — `grep -rn OnAddToHand` finds only the enum declaration. No add-to-hand path (`Game::add_to_hand_from_deck/trash/security/reveal`, `EffectContext::draw`, reveal-into-hand, etc.) fires triggers on this timing, and there is no trigger-context population for (a) *whose* hand gained cards, (b) how many, or (c) the load-bearing "by an effect" vs "by the normal draw step / mulligan" distinction that DCGO's `CanTriggerWhenAddHand` encodes (the `cardEffect != null` guard). The DSL `Timing` enum (`code/digimon-dsl/src/clause.rs`) also has no `on_add_to_hand` variant, so even once the engine fires it there is no DSL surface to bind.
- **Suggested API shape:** (1) Engine: enqueue `OnAddToHand` from the add-to-hand commit paths, populating a trigger context with the recipient `PlayerId`, the count added, and an "added by effect" flag (skip the per-turn draw step + mulligan + the `[Draw]`-phase draw). Mirror the existing event-target predicate family so a clause can gate `event_target_owner: opponent`. (2) DSL: add `Timing::OnAddToHand` (`on_add_to_hand`) and an `event_added_to_hand_owner` / reuse-`event_target_owner` predicate leaf. Couples with the existing "[BEATBREAK / DATA SQUAD Tamer face-down stash substrate]" (BT25-087's place-2-face-down body and BT25-029's unsuspend body are otherwise expressible once the trigger fires).
- **Workaround:** None faithful. Omitting the trigger silently drops a printed [All Turns] reaction; there is no synchronous proxy because the firing is opponent-hand-state-change driven, not a player action.
- **Related:** BT25-029 also pairs this with `OnDigivolutionCardTrashed` (already supported) under a shared OPT — see existing "[Shared-OPT across heterogeneous-timing trigger pair with per-timing conditions]".

### `[Link]` keyword subsystem (Appmon link-card attachment + activated link effect)
- **Severity:** 🔴 BLOCKED — net-new engine subsystem, NOT a `Keyword` enum flag. Comparable in scope to DigiXros.
- **Discovered in:** `/author-set` keyword gate auto-ingest attempt (2026-06-05). The DCGO keyword manifest classifies `link` as auto-ingestable (it has `CardEffectFactory/KeyWordEffects/Link.cs`), but the port-complexity assessment downgrades it: "DCGO has a KeyWordEffects file" is necessary but **not sufficient** — Link is a subsystem with its own attachment state, activated action, timing, and rule-check, not a triggered/replacement keyword.
- **Card(s):** BT22-055 Recomon, BT22-058 Dreammon, BT22-075 Fakemon, BT24-053 Protecmon, and the broader BT22/BT24 Appmon pool (every card with `Link Requirements [Link] [Appmon] trait: Cost N`). BT25 (orphan-staples-1 slice, 2026-06-06) adds: **BT25-007 Gatchmon** (`AddSelfLinkConditionStaticEffect` host-Appmon link-cost-1 + `WhenLinked` ActivateClass "delete 1 opp Digimon ≤3000 DP" — its printed *inherited* text is actually DCGO's `WhenLinked` effect), **BT25-004 Tapmon** (inherited `WhenWouldLink` ActivateClass "you may reduce link cost by 1"), **BT25-045 Onmon** (face-up `WhenWouldLink` ActivateClass "you may reduce link cost by 1" + `WhenLinked` ActivateClass "suspend 1 opp Digimon"). These surface two facets not yet enumerated below: (10) a **`WhenWouldLink`-timing triggered `ActivateClass`** (optional, OPT, host==self + linking-card-trait gated) that registers a **fixed-cost-time link-cost reducer** (DCGO `card.Owner.UntilCalculateFixedCostEffect.Add(GrantedReduceLinkCostClass(reducedCost, cardSourceCondition, permanentCondition, rootCondition))`) — distinct from the static player-scoped `ChangeLinkCost` modifier the engine already has (`Game::link_cost_delta_for_player`), which is unconditional, non-optional, and not host-scoped; and (11) the **`WhenLinked` ActivateClass as a host-Digimon self-effect** (delete/suspend an opponent Digimon when *this* Digimon is linked to) which is the consumer side of facet #6's `[When Linking]` timing. Note: the Rust engine's existing `WhenWouldLink` wiring is a **replacement** only (`commit_pending_would_link` handles None/Cancelled/Redirected/Substituted — no cost-reduction outcome) and fires post-payment in `attach_linked_card`; it cannot host an optional cost-reducing ActivateClass. BT25 (appmon slice, 2026-06-06) adds two more cards that exercise **facet 9 (alternate-source / cost-delta linking of a *chosen* card, not the carrier)**: **BT25-089 Kazuki & Itsuki** (Tamer) `[Main] By suspending this Tamer, you may link 1 [Appmon]-trait Digimon card **from your hand or your Digimon's digivolution cards** to 1 of your Digimon with the cost reduced by 2` — the existing `link_to_own_digimon` DSL verb / `attach_linked_card` path only links the *carrier* Option, so picking an arbitrary hand/digivolution-source card to attach as a link (with a −2 cost delta) is unsupported; and **BT25-052 Logimon** (Digimon) `[Main] [Once Per Turn] You may link 1 [Social], [Tool] or [Game]-trait Digimon card **from your hand or this Digimon's digivolution cards** to this Digimon with the cost reduced by 1` (plus `[Your Turn][OPT] When this Digimon gets linked, if you have 1 or fewer Tamers, you may play 1 [Kazuki & Itsuki] from your hand without paying the cost` — a `WhenLinked` host self-effect, facet 11). Both are BLOCKED (hybrid: engine lacks the link-a-chosen-card-from-hand/digivolution-cards primitive; the DSL lacks any verb that would lower to it). BT25-089 is *also* blocked on App Fuse (see entry below). BT25 (orphan-c slice, 2026-06-06) adds **BT25-070 Logamon** (Lv.4 Black/Purple, Logoff Sup.): `AddSelfLinkConditionStaticEffect` host predicate `HasAppmonTraits` link-cost-2; `[Main][OPT]` "link 1 [Social]/[Tool]/[Game] Digimon card **from your trash OR this Digimon's digivolution cards** to this Digimon with cost −1" = facet #9 alternate-source linking (from trash/sources, not hand) plus a body-scoped `GrantedReduceLinkCostClass` (facet #10); `[Your Turn][OPT] WhenLinked` "delete 1 opp Digimon with play cost ≤4" = facet #11; inherited `[When Linking]` "1 opp Digimon or Tamer can't unsuspend until their turn ends" = facet #6's `[When Linking]` consumer; plus `AddAppfuseMethodByName(Offmon, Hackmon)` App-Fusion (separate App Fuse gap). Verified against the engine on 2026-06-06: `EffectTiming` carries only `WhenWouldLink` (a replacement) — there is no `WhenLinked`/`[When Linking]` *triggered* timing in the enum nor in `code/digimon-engine/src/dsl_cards/timing_map.rs`, and the only link step the DSL exposes (`link_to_own_digimon`) links the *carrier Option* to a host, not a chosen Digimon card from trash/sources. **BT25-070 is BLOCKED in full** — only its standard `Logoff`-trait alt-digivolve *requirement* is expressible (not an effect), so there is no faithful partial.
- **Effect text (printed):** "Link Requirements [Link] [Appmon] trait: Cost N (Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)" Plus link-interacting clauses: BT22-058 "[All Turns] [Once Per Turn] When this Digimon gets linked, …"; BT22-075 "[On Play] [When Digivolving] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost." and "When this Digimon would leave the battle area, you may play 1 of this Digimon's link cards without paying the cost."
- **Authoritative behavior (sources):** `general_rule.pdf` §10-1-1 (a `[Link]` card is plugged sideways into a specified Digimon), §16-39 (`Link +X` DP buff), §15-16-6 (`[When Linking]` timing), §17-1-3-6 (a link card is trashed on the rule check if the host no longer meets the link requirement). DCGO `CardEffectFactory/KeyWordEffects/Link.cs` (`LinkEffect`) models it as an **`ActivateClass`** (a main-phase activated effect): on the owner's turn, with the card in hand OR on the battle area un-linked, and a matching host Digimon present, the player selects 1 host Digimon (`SelectPermanentEffect`, maxCount 1, no auto-select) and the card is attached via `ILinkCard.LinkCard()`. `AddLinkRequirement.cs` (`LinkCondition { digimonCondition, cost }`) carries the per-card host predicate + cost; `ChangeLinkCostClass` / `ChangeLinkMax` handle cost reduction and link-max.
- **What's missing (the whole subsystem — none of this exists in `code/digimon-engine/`):**
  1. **Link-card attachment state** — a Digimon hosting "link cards" plugged sideways (a distinct relationship from digivolution sources and from DigiXros materials; carries `is_linked`). Needs `Permanent` representation + zone/state plumbing.
  2. **A `[Link]` activated action in the 2192-action space** — the player activates Link on an eligible card and chooses a host. This is a **new action-space entry** → ripples to `tensor.rs`, `action/mask.rs`, `action/decoder`, and the rule-27 DCGO `ActionSpace.cs` codegen + drift CI. This is the heaviest dependency.
  3. **Link-requirement parsing** — `[Link] <trait/predicate>: Cost N` from card metadata into a structured `LinkRequirement { host_predicate, cost }` on `CardData`.
  4. **Pending-selection** for the host Digimon (no-approximations — maxCount 1, no auto-pick).
  5. **Link cost payment** + cost-reduction modifiers (`ChangeLinkCost`) + link-max (`ChangeLinkMax`).
  6. **`[When Linking]` trigger timing** (§15-16-6) — a new timing enum + dispatch (e.g. BT22-058).
  7. **Rule-check trashing** (§17-1-3-6) — trash a link card when its host stops meeting the requirement, integrated into the rule-check loop.
  8. **`Link +X`** (§16-39) DP-buff variant.
  9. **Leave-battle interaction** — "play this Digimon's link cards when it would leave" (BT22-075) and free/alternate-source linking (from trash / digivolution cards without paying cost).
- **Why it is NOT a `keyword_to_auto_effect` arm:** that machinery produces declarative / triggered / replacement effects bound to a carrier. Link is an **activated** effect that mutates board attachment state and is exposed in the action mask — a different category entirely. Adding a bare `Keyword::Link` variant without the subsystem would be a silent drop (a card "supporting" Link that cannot actually link), violating the no-approximations policy (CLAUDE.md §17).
- **Suggested approach:** Treat as a scheduled engine feature, not a keyword ingest. Mirror the DigiXros subsystem (`code/digimon-engine/src/digixros.rs`) for the attachment/transaction shape, add a `Link` activated action to the action space (with the full tensor/mask/decoder/ActionSpace.cs regen), a `LinkRequirement` on `CardData`, a `[WhenLinking]` timing, and the §17-1-3-6 rule-check hook. TDD per card (BT22-055 simplest: link-from-hand, fixed cost, trait predicate; then BT22-058 `[When Linking]` trigger; then BT22-075 alternate-source + leave-battle).
- **Workaround:** None faithful. BT22/BT24 Appmon cards whose ONLY engine-relevant clause is the link requirement are BLOCKED until the subsystem lands; cards with additional non-link clauses can have those clauses authored, with the link requirement explicitly marked BLOCKED (not stubbed).
- **Workflow implication (`author-set`):** the keyword gate's `auto_ingest` verdict must be **complexity-gated** before the auto-ingest barrier sub-pipeline ports it. A keyword that DCGO models as an `ActivateClass` touching board state / the action space is a subsystem, not a flag, and should be reclassified BLOCKED-subsystem (human-scheduled) rather than auto-ported. `Ascension` and `Blast DNA Digivolution` (the other two standing auto-ingest candidates) need the same triage before assuming they are cheap.
- **Related:** DigiXros subsystem (`src/digixros.rs`) as the structural template; `aura.rs` (attachment-adjacent); the action-space contract (`docs/ACTION_SPEC.md`) + rule-27 `ActionSpace.cs` codegen.

- **Updated 2026-06-07 (link-appmon-1 slice re-adjudication, post DigiLink Shape-B):** With the Shape-B engine substrate + DSL vocabulary LANDED (2026-06-06; engine note above + `qa/dsl-vocab-gaps.md` G-DSL-DIGILINK), the **standing-permanent-absorb** Appmon Link Digimon now ships in DSL. Implemented this slice: **BT25-007 Gatchmon** and **BT25-061 Offmon** — both author `kind: link_condition` (Appmon host, cost 1) + a cost-0 Appmon alt-digivolve + a `when: when_linked` payoff (007: delete opp DP≤3000; 061: opp `CannotUnsuspend` until their turn ends), plus their non-link clauses (007 OnPlay reveal-3 two-bucket add; 061 [Start Main] optional trash-Appmon→draw+memory). Tests in `tests/cards_behavioral/bt25/bt25_007.rs` + `bt25_061.rs` (7 each, green). The following remain BLOCKED on **residual** Link facets NOT closed by Shape-B: (#10 — host-filtered optional `WhenWouldLink` cost-reduction `ActivateClass`) **BT25-004 Tapmon** (its only clause) and **BT25-045 Onmon** (mandatory alongside an otherwise-expressible link payoff — BLOCKED not PARTIAL); (#9 — link a *chosen* card from hand/digivolution-cards, not the standing carrier) **BT25-052 Logimon** (`[Main][OPT]` link-of-chosen-card; the wired path only absorbs a standing permanent). Separately, **BT25-036 Craftmon** is BLOCKED on the **App Fuse** primitive (`AddAppfuseMethodByName`, see App Fuse entry below) — its prior `G-DSL-WHEN-LINKED-TIMING` block is now resolved.

### `OnAddDigivolutionCards` trigger timing (fires when cards are placed into a Digimon's digivolution stack)
- **Severity:** 🔴 BLOCKED — net-new trigger timing + dispatch hook. Not expressible by composing existing DSL vocabulary.
- **Discovered in:** BT25 orphan-staples-1 slice (2026-06-06).
- **Card(s):** **BT25-005 Pagumon** (inherited: "[Your Turn] [Once Per Turn] When [Three Musketeers] trait cards are placed in this Digimon's digivolution cards, it may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand with the cost reduced by 2."). DCGO's hash string (`EX11_074_OnAddDigivolutionCards`) indicates EX11-074 Vortexdramon shares the same trigger — so this unblocks ≥2 cards.
- **Effect text:** as above (BT25-005 inherited).
- **What's missing:** A trigger that fires when one or more digivolution-source cards are **added underneath** a permanent (e.g. by another effect placing sources, or by a material/under-placement step) and exposes the added cards + host permanent to a triggered clause. DCGO models this as `EffectTiming.OnAddDigivolutionCards` consumed via `CardEffectCommons.CanTriggerOnAddDigivolutionCard(hashtable, permanentCondition, _, cardCondition)` — `permanentCondition` pins the host to *this* permanent, `cardCondition` gates on the added card's traits ([Three Musketeers]). The Rust DSL `Timing` enum (`code/digimon-dsl/src/clause.rs`) has `OnDigivolutionCardTrashed` (the *opposite* event — a source leaving the stack) but no "source added/placed" counterpart, and the engine has no dispatch site that fires such a timing when `place_as_bottom_source` / under-placement runs.
- **Suggested API shape:** Add `Timing::OnAddDigivolutionCards` (DSL `when: on_add_digivolution_cards`) → `EffectTiming::OnAddDigivolutionCards`; fire it from the engine's stack-source-placement path (wherever sources are pushed under a permanent by effect), carrying `TriggerContext` with the host permanent + the added `CardSource`(s) so `event_card_trait_has` / a host-self predicate can gate it. The downstream body (optional OPT `effect_initiated_digivolve` from hand into a [Three Musketeers]-text/[TS]-trait card with `cost: { reduce: 2 }`) is already expressible once the trigger exists.
- **Workaround:** None faithful. Pagumon's sole clause depends on this trigger; authoring it would require either a stub (forbidden) or mis-binding to a different timing (silent behavior divergence). BLOCKED.
- **Related:** Existing `OnDigivolutionCardTrashed` timing (sibling — source-removed vs. this source-added); the `[Link]` keyword subsystem entry above (also surfaces missing digivolution/link-adjacent timings).

### `App Fuse` keyword/primitive (a Digimon "app fuses" into a Digimon card in the hand)
- **Severity:** 🔴 BLOCKED — net-new keyword primitive. No `app_fuse` / "app fuse" anywhere in `code/digimon-engine/src/`, `code/digimon-dsl/`, or the DSL process/trigger vocabulary (all `fuse` source hits are `confuse`/`refuse`). Not composable from existing DSL verbs.
- **Discovered in:** BT25 Kazuki & Itsuki / Appmon slice (2026-06-06), by `/archetype-interaction-test-author`.
- **Card(s):** **BT25-089 Kazuki & Itsuki** (Tamer): `[End of Your Turn] [Once Per Turn] 1 of your Digimon may app fuse into a Digimon card in the hand.` **BT25-060 Rebootmon** (Digimon, link-finish-aura slice, re-adjudicated 2026-06-07): registers `AddAppfuseMethodByName(Bootmon, Shutmon)` — an App-Fusion alt-play path. All of Rebootmon's *other* clauses are now expressible (Security+1, Reboot, Link+1 self-aura via modifier_value, link-from-hand/digivolution-sources-to-self via the `link_cards` step, the when_linked/on_unsuspend OPT Piercing/Blocker/effect-immunity grant), so App Fuse is its sole remaining blocker. This is the Appmon "App Fuse" mechanic shared across the BT25 Appmon package (and prior Appmon sets), so the primitive unblocks more than one card. **Updated 2026-06-12 (Appmon BT21 wave):** add **BT21-084 Haru Shinkai** (Tamer) — `[Your Turn] When your Digimon get linked, by suspending this Tamer, <Draw 1>. Then, 1 of your Digimon may app fuse into a Digimon card in the hand.` Its memory ramp, the suspend-cost `on_any_link` Draw 1, and the security play-free clause all ship; only the effect-initiated app-fuse rider is omitted → BT21-084 verdict PARTIAL (same gap as BT25-089). Also the **P-241 Yujin Ozora** promo (Tamer) carries the same effect-initiated app-fuse tail.
- **What's missing:** A keyword/effect that lets one of your battle-area Digimon "app fuse" into a Digimon card in hand — i.e. swap/merge the field Digimon with a hand Digimon card, carrying the appropriate state (digivolution sources / linked cards / placement) across the swap, with a pending-selection for *which* field Digimon and *which* hand card (no auto-pick, no-approximations CLAUDE.md §17). The exact carry-over semantics must be sourced from `general_rule.pdf` §16 (App Fuse) and DCGO's App Fuse keyword effect before authoring.
- **Source priority to consult before building:** `general_rule.pdf` §16 (App Fuse keyword semantics); DCGO C# `$BASE_DCGO/Assets/Scripts/CardEffect/.../KeyWordEffects` App Fuse effect + `BT25_089.cs` (if present) for the resolve order; card image for printed text. This skill run did not pull the DCGO source because the carrier card (BT25-089) is itself unimplemented — pull it when scheduling the primitive.
- **Workaround:** None faithful. BT25-089's end-of-turn clause depends on this; stubbing or auto-selecting violates §17. The card's other two clauses (Start-of-Main memory ramp; suspend-cost Appmon Link at cost −2 — Link is already supported) ARE expressible, so BT25-089 can be authored with the App Fuse clause explicitly marked BLOCKED (not stubbed) until this lands.
- **Related:** The `[Link]` subsystem (now wired — `attach_linked_card` + `OnLink`/`WhenWouldLink` dispatch in `game_actions.rs`) is the structural neighbor; App Fuse is a *field↔hand Digimon swap*, a different operation from link attachment and from DigiXros material assembly.

### `OnDiscardHand` / "when your hand is trashed from" trigger timing + "played by an effect" condition  [G-ENGINE-ON-DISCARD-HAND]
- **Status: RESOLVED (2026-07-03, trigger-timings round 2).** `EffectTiming::OnDiscardHand` + `TriggerSource::HandDiscarded{player, cause_controller}`: batch-coalesced once per affected owner at the outer drain boundary (DCGO DiscardHands semantics); draw/mulligan/rule discards never fire it. Predicates `event_discard_player`, `event_caused_by_own_effect` (ST16-14), and the separate `played_by_effect` (BT25-080 tail, reads PlaySource::ByEffect). Tests: tests/on_discard_hand.rs (8). Driver YAML intentionally left to card authoring.
- **Severity:** 🔴 BLOCKED — net-new trigger timing + dispatch hook, plus a new play-context condition. Not expressible by composing existing DSL vocabulary.
- **Discovered in:** BT25 "titan" slice (2026-06-06). **Canonical home for this gap** (formerly also logged in `qa/archetype-qa/engine-gaps.md`, which now cross-refs here — fix-dsl-substrate-rot-and-bugs §6.4).
- **Card(s):** **BT25-080 Witchmon** — inherited `[All Turns][Once Per Turn] When your hand is trashed from, if this Digimon has the [Titan] trait, delete 1 of your opponent's level 4 or lower Digimon`. DCGO `BT25_080.cs` consumes `EffectTiming.OnDiscardHand` via `CardEffectCommons.CanTriggerOnTrashHand(...)`. The hash string (`BT25_029_AT_ESS`) indicates **BT25-029** shares the same trigger. **BT25-084 Titamon** — inherited `[All Turns] When your hand is trashed from, delete 1 of your opponent's lowest DP Digimon` (DCGO `BT25_084.cs`, same `OnDiscardHand` ActivateClass; clause 3 omitted → BT25-084 verdict PARTIAL). Unblocks ≥3 cards (BT25-080 / BT25-029 / BT25-084).
- **What's missing (engine):** A trigger timing that fires when one or more cards leave a player's **hand to trash** (a discard/trash-from-hand event), exposing the trashing player so an inherited clause can gate on "your hand." The Rust `EffectTiming` enum has no hand-trash trigger (it has `OnDigivolutionCardTrashed`, `OnOptionTrashed`, `OnLinkedCardTrashed`, but nothing for hand→trash). No dispatch site fires such a timing from `trash_from_hand_by_index` / discard paths.
- **What's missing (condition):** BT25-080's *main* clause `[On Play][When Attacking][OPT] ... After, if played by an effect, delete 1 opp level 5 or lower Digimon` gates the delete on the card having been **played by an effect** (DCGO `CardEffectCommons.IsByEffect`). The DSL predicate surface has no `played_by_effect` / `is_by_effect` condition over the current play context. (The main clause's return-Titan-from-trash half IS expressible; the conditional delete tail is blocked on this condition.)
- **Suggested API shape:** Add `Timing::OnDiscardHand` (DSL `when: on_discard_hand`) → `EffectTiming::OnDiscardHand`, fired from the hand→trash path with a `TriggerContext` carrying the trashing player (so an inherited clause can gate `your_turn`-independent "your hand"); plus a `played_by_effect: true` predicate reading the active play context's `PlaySource::ByEffect`.
- **Workaround:** None faithful. BT25-080 ships no YAML: the inherited clause depends on the missing trigger and the main clause's conditional delete depends on the missing condition; authoring either would require a stub or a mis-bound timing (silent divergence). BLOCKED.
- **Related:** sibling `OnDigivolutionCardTrashed` / `OnOptionTrashed` / `OnLinkedCardTrashed` timings (this is the hand→trash counterpart); `PlaySource::ByEffect` already threaded through the play pipeline (the `played_by_effect` predicate just needs to read it).

### Link-card inherited (ESS) effects are not applied to the host  [G-LINK-INHERITED-ESS]
- **Status: RESOLVED (2026-07-03, link-economy round 2).** linked_cards folded into all five effect-source collectors (has_keyword gained a linked pass; materialize_declaratives_full / live_declarative_formula_sum / static_dp_aura_bonus / enqueue_from_permanent widened to effect.linked || effect.inherited; scope conventions: linked XOR inherited, debug_assert-enforced; no double-count). bt25_100_grants_inherited_piercing_to_host un-ignored and green.
- **Severity:** 🟡 PARTIAL — the link substrate exists (cards attach as `Permanent.linked_cards`, link/unlink/trash cascades fire `OnLinkedCardTrashed`), but a link card's **inherited effects do not flow onto its host Digimon**.
- **Discovered in:** orphan-staples-6 slice (2026-06-06) implementing BT25-100 Iron Slash; its inherited `<Piercing>` does not appear on the Digimon hosting the linked Option.
- **Card(s):** BT25-100 Iron Slash (inherited `<Piercing>`), BT25-093 Ignition Flare (inherited `[When Attacking][OPT]` delete), BT25-101 Divine Arms Version Ω (inherited `<Security A. +1>` / `<Reboot>`), and broadly every BT25 TS link-Option whose link-card half (ESS) grants a keyword or installs a triggered effect on the host. DCGO models these as link-ESS via `SetIsLinkedEffect(true)` clauses that fire while the card is a link card on a host.
- **What's missing:** The effect-source collectors (`Game::live_declarative_formula_sum`, the keyword-grant aggregation, and the triggered-effect source walk around `game.rs:3883`/`3958`/`4004`) iterate each permanent's `card_sources` (digivolution stack) + breeding-area top, but **never `perm.linked_cards`**. So a link card's `scope: inherited` (ESS) declarative/keyword/triggered clauses are compiled but never consulted while the card is attached as a link card. (A link card's inherited clauses *do* apply normally when it is a digivolution source, not a link card — only the link-attachment path is unwired.)
- **Suggested API shape:** In each source-collection loop, after walking `card_sources`, also fold in `perm.linked_cards` with `inherited_source = true` (a link card behaves as a below-top / inherited source for ESS purposes), so its `scope: inherited` declaratives (keyword grants, DP/`SecurityAttack` formulas) and triggered clauses (`when_attacking`, etc.) register on the host. Mirror the same in the keyword-grant and triggered-effect walkers. Guard against double-counting if a card is somehow both a digivolution source and a link card.
- **Workaround:** Author the inherited link-ESS clauses faithfully in YAML (they compile and will activate once this gap closes) and `#[ignore = "pending: G-LINK-INHERITED-ESS …"]` the host-keyword/host-trigger behavioral test. BT25-100/BT25-093 ship with their Main/Security/link-attach clauses working and the inherited ESS tagged pending; BT25-101 is BLOCKED for additional reasons (see `qa/dsl-vocab-gaps.md`).
- **Related:** the `[Link]` keyword subsystem entry above (this is the ESS-application slice of the same broader Link area); `OnLinkedCardTrashed` (already-wired link-leave cascade).

### ✅ RESOLVED (2026-06-15) — `cost_reduction` `pay_cost` drops the reduction when the cost is interactive (parks on a selection)  [G-COST-REDUCTION-INTERACTIVE-PAY-COST]
- **Severity:** ✅ CLOSED — all three cost paths now credit an interactive (parking) `pay_cost` reduction: **PLAY-FROM-HAND (2026-06-14)**, **DIGIVOLVE (2026-06-15)**, **OPTION-USE (2026-06-15)**.
- **✅ PLAY-FROM-HAND CLOSED (2026-06-14):** `Game::apply_cost_reduction_candidate` takes `allow_interactive_pay_cost: bool` (`game_actions/cost.rs`). The two play-from-hand chain call sites pass `true`: when the `pay_cost` PARKS on a selection (the interactive Tamer pick), the reduction `amount` is credited now and `continue_play_from_hand_cost_reduction_chain` wraps the play continuation behind the park. A genuine synchronous failure or an UNPAYABLE abort (no eligible Tamer → the FD-trash step sets the transient `EffectContext::cost_unpayable` flag, read by the generic `pay_cost_fn` lowering at `dsl_cards/lower_cost_reduction.rs`) credits nothing. The synchronous scan path keeps `allow_interactive_pay_cost = false`. Verified: BT25-088 clause 3 ships (`tests/cards_behavioral/bt25/bt25_088.rs` Section 5 — positive reduction + unpayable-control).
- **✅ DIGIVOLVE + OPTION-USE CLOSED (2026-06-15):** the synchronous `scan_before_pay_cost_reduction_with_target` cannot host a parking pay_cost (a park leaves a dangling `pending_selection` mid-action), so each path now runs a dedicated **pre-scan interactive prompt** BEFORE the scan, mirroring the play-from-hand chain:
  - **Discriminator:** a new `Effect::pay_cost_interactive` flag (set by the DSL cost-reduction lowering via `body_first_step_installs_selection(pay_cost)` in `dsl_cards/lower_triggered.rs`) marks a reducer whose `pay_cost` first step installs a selection (e.g. `trash_bottom_face_down_source_under_tamer`). The synchronous scan now **skips** `pay_cost_interactive` reducers entirely (so they are never double-applied on re-entry).
  - **Digivolve** (`game_actions/digivolve.rs::digivolve_from_hand_inner` + `game_actions/cost.rs::try_prompt_interactive_digivolve_cost_reducer`): before the scan (and after the player-scoped BT3-103 prompt), an interactive field reducer's `pay_cost` is run. On a PARK, the parked selection's resolution credits `amount` into a new `Game::pending_interactive_digivolve_reduction` and re-enters `digivolve_from_hand_inner(..., player_reducer_resolved = true)`; on a synchronous outcome the amount is credited inline (iff `!cost_unpayable`) and the original frame continues. An optional reducer gets an accept/decline gate first; a mandatory one runs the pay_cost directly. Cards: ST23-03 Cougarmon / ST23-11 Wolvermon ("when this would digivolve into a [Glowing Dawn] Digimon, by trashing the bottom face-down card under a Tamer, reduce the cost by 2").
  - **Option-use** (`game_actions/options.rs::play_option_core` + `try_prompt_interactive_option_use_cost_reducer`): the sibling prompt runs before the cost scan (Pay policy, non-Link mode only); on resolution it credits `Game::pending_interactive_option_use_reduction` and re-enters `play_option_core(player, source, Some(mode), cost_policy)`. Cards: BT25-049 Armalizamon c2 / BT25-090 Tomoro Tenma c3 ("when you would use a [Glowing Dawn] Option card, by trashing the bottom face-down card under a Tamer, reduce the cost by N").
  - **Tests:** `tests/cost_hooks/interactive_digivolve_reducer.rs` (3) + `tests/cost_hooks/interactive_option_use_reducer.rs` (3) — each: positive paid-park credits the reduction + trashes the FD source; unpayable control (no FD stash) → full cost, nothing trashed; negative (non-Glowing-Dawn target) → reducer inactive. Full `cards_behavioral` (5248 passed), `cost_hooks` (66), `dsl` (766), `archetypes` (200), and lib unit tests (232) green; no regression in the play path / BT5-092 digivolve reducer / BT10-087 DigiXros reducer / BT3-103 player-scoped reducer.
  - **No-approximations preserved:** the Tamer pick always surfaces as a real `PendingSelection` (a 1-option selection for a single eligible Tamer is NOT auto-resolved).
- **Discovered in:** BT25 "beatbreak" slice (2026-06-06).
- **Remaining card work (downstream, NOT engine):** the ST23-03 / ST23-11 and BT25-049 c2 / BT25-090 c3 YAML cost-reduction clauses can now be authored (`kind: cost_reduction` + `pay_cost: [trash_bottom_face_down_source_under_tamer]`, with `when_any_ally_digivolves_into` / option-target predicate). They were intentionally NOT authored as part of this engine-gap closure.
- **Related:** `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER` (`qa/dsl-vocab-gaps.md`, the multi-count sibling — BT25-035 trash-2); `G-COST-REDUCTION-DIGIVOLVE-INTO` (the resolved synchronous-suspend pay_cost precedent, BT5-092).

### Trait-gated digivolution requirement (`can_digivolve` / `can_basic_digivolve` are color+level only)
- **Severity:** 🟡 PARTIAL — pre-existing limitation, surfaced (not introduced) by the DUAL/BEATBREAK per-face work (`gap/dual-per-face-arts`, 2026-06-15).
- **Discovered by:** ST23-09 Atratusmon, BT25-057 Monarchlizamon, BT25-043 Habakirimon — BEATBREAK cards whose ONLY printed digivolution is trait-gated ("[Digivolve] Lv.N w/ [Glowing Dawn] trait: Cost C"), with no separate color line.
- **What's missing:** `Game::can_digivolve` (`game/queries.rs`) and the action-mask `can_basic_digivolve` (`action/mask.rs`) both match a base only against the candidate card's static `digivolution_costs()` table — `EvoCost { card_color, level, memory_cost }` — i.e. **color + level only**. `EvoCost` has no trait field, so a digivolution requirement gated on the *base's trait* ("Lv.5 with the [Glowing Dawn] trait") cannot be expressed as a static row. The DSL `kind: digivolve` alt-path *can* carry a trait `from:` filter, but `can_digivolve` (used by the basic-digivolve mask, breeding promotion, AND the Arts-digivolve gate) does not consult alt-paths — only the static evo table.
- **Workaround used (faithful enough to ship):** these cards author BOTH (1) a **color-form** alt-digivolve matching the printed card's color + level (backfilled into the static `EvoCost` table, the same table `data/cards.json` carries in production) so digivolve-as-Digimon, breeding promotion, and the Arts `can_digivolve` gate all function, AND (2) the **trait-form** alt-path expressing the printed [Glowing Dawn] restriction. Because the engine's basic-digivolve is color+level for *every* card today, this introduces no NEW approximation beyond the pre-existing deferred trait/color/CANNOT_DIGIVOLVE validation.
- **Suggested API shape:** a per-card `Vec<DigivolveRequirement { predicate, cost }>` (predicate over the base permanent's top card) layered on top of the static `EvoCost` table, consulted in `can_digivolve` + `can_basic_digivolve` + breeding.
## Engine correctness bugs (existing primitives) — found by archetype interaction tests

> Surfaced by the `/archetype-interaction-test-author` capstone on the ST-1…ST-6 starter decks (2026-05-30). **All three are FIXED (2026-05-30).** Two were engine bugs; the first ("base-fold") turned out to be a *card-authoring* bug once the engine's base-inclusive formula contract was understood — see below.

### ✅ FIXED (2026-05-30) — WarGreymon `<Security A.>` formula under-counted (CARD bug: formula authored as a bare delta, not base-inclusive)
- **Symptom:** a 4-source WarGreymon (ST1-11) checked only 2 security where it should check 3 (base-1 + `floor(4/2)`).
- **Root cause (the important part):** `security_attack_fn` formula auras are **base-inclusive by engine contract** — the formula returns the *total* base check count (it replaces the default base-1 while active), and multiple formula auras take the **MAX**, not the sum. Flat `<Security A.>` keyword/modifier deltas are then added on top. This contract is pinned by `tests/dsl/group6_dynamic_formulas.rs` (`{ base: 1, … }` ⇒ exactly 1 check). ST1-11's formula was authored `base: 0` (= `floor(n/2)`, a bare *delta*), so it dropped the base-1 and under-counted. An earlier attempt to "fix" this in `current_security_strike` (`1 + aura_bonus`) was the **wrong layer** — it double-counted the base for every correctly-authored base-inclusive formula and regressed the group6 dsl tests.
- **Fix (card layer):** author the formula base-inclusive — `cards/st1/ST1-11.yaml` now uses `floor_div([{base: 2, per: material_count, delta: 1}, 2]) = floor((2+n)/2) = 1 + floor(n/2)`. The engine keeps its base-inclusive `unwrap_or(1)` contract unchanged.
- **Test (now green):** `tests/archetypes/st1_gaia_red.rs::tall_stack_security_rush_aura_adds_to_base_one_check` (4-source WarGreymon, Greymon-free, `dynamic_security_attack_aura_bonus == Some(3)`, checks 3); plus the un-`#[ignore]`d full-line `tall_stack_security_rush_full_line_checks_four_security` (checks 4 with Greymon — see next entry).
- **Sources:** general_rule.pdf §16-3; DCGO `ST1_11.cs` (`count = DigivolutionCards.Count / 2`, applied as a delta — the engine folds in the base).

### ✅ FIXED (2026-05-30) — security-DP auras ignored their `active_when`/`condition` gate
- **Was:** `Game::defender_security_dp_adjustment` + mirror `attacker_security_dp_adjustment` (`combat.rs`) summed `applies_to_own/opponent_security_dp` auras without evaluating `effect.condition`, so T.K. Takaishi's "[Opponent's Turn] +2000" leaked onto the controller's own turn.
- **Fix:** both call sites now build an `EffectReadContext` (controller as `player`) and skip the aura when `effect.condition` is false — mirroring `static_dp_aura_bonus`.
- **Test (now green):** `tests/archetypes/st3_heavens_yellow.rs::tk_takaishi_aura_is_inactive_on_controllers_own_turn`.
- **Sources:** `ST3-12.json`; DCGO `ST3_12.cs` (`IsOpponentTurn`); general_rule.pdf 15-16-8-1.

### ✅ FIXED (2026-05-30) — declarative inherited `<Security A. +/-N>` grants now counted by the security strike (tick-fresh + de-overlapped)
- **Was:** the security strike under-counted inherited `<Security A.>` authored as a DSL `grant_keyword` (ST1-07 Greymon and any card whose inherited `<Security A. +/-N>` is a declarative grant rather than a printed `card_data` keyword). A real WarGreymon+Greymon stack checked **3**, not the card-faithful **4**.
- **Root cause (architectural, root-caused 2026-05-30).** `<Security A.>` for a permanent is aggregated by `Game::security_attack_keyword_bonus` across overlapping representations that must single-count:
  1. **Printed** `card_data` keywords (`face_keywords` top card, `inherited_keywords` buried sources).
  2. **Own** (face-up, non-inherited) DSL `grant_keyword` — *also* populates `card_data` (#1 already counts it) **and** materializes into `permanent_keywords` on tick → would double-count if both summed (BT21-029 Medusamon, ST5-13, ST6-13).
  3. **Inherited DIRECT** `grant_keyword` (`granted_keyword` set) — declarative-only; counted via the materialized registry (ST1-07, BT20-016).
  4. **Inherited/own AURA** grants (`kind: aura`, `granted_keyword` unset, grants via a process to filter-matched permanents) — materialize into the registry (ST2-08 WereGarurumon; BT5-093 aura-to-Omnimon).
  5. **Genuine modifier** grants (ally buffs, end-of-turn grants) — real `permanent_keywords` entries (`materialized_declarative = false`).

  The registry (#2–#5) is only fresh after `tick_declarative_effects` — run by the `decode_action`/`LiveGame` path but **not** by `DebugRunner::attack_player` — so a no-tick test read a stale cache. And #2 overlaps #1 (own grants land in both `card_data` and the registry), so a naive "printed + registry" sum double-counts own keywords.
- **Fix (two coordinated changes, full keyword-suite green):**
  - **Tick-fresh strike** — `Game::current_security_strike` (`combat.rs`) now calls `self.tick_declarative_effects()` before reading the keyword term, so the materialized registry reflects current inherited/aura grants regardless of the caller (DebugRunner or the real action path). It is idempotent and the per-iteration recompute already re-reads it.
  - **Tick de-overlap** — `tick_declarative_effects` (`game.rs`) now **skips materializing a non-inherited `granted_keyword` that is already face-printed** in `card_data`, so #2 never double-counts against #1. (Inherited and aura grants still materialize.) This single-counts all five representations: `security_attack_keyword_bonus = printed_true_keywords + registry`.
- **Tests (now green):** `tests/archetypes/st1_gaia_red.rs::tall_stack_security_rush_full_line_checks_four_security` (full WarGreymon+Greymon line checks 4); the Medusamon mid-attack recompute (BT21-029) and BT20-016 / ST2-08 / BT5-093 grant cases all still single-count (combat + cards_behavioral + dsl suites pass).
- **Formula + flat-delta co-existence (2026-05-30, BT1-085 Tai Kamiya).** BT1-085's "[Your Turn] your red Digimon with 4+ digivolution cards gain `<Security A. +1>`" was implemented (DSL, `cards/bt1/BT1-085.yaml`) specifically to exercise a formula aura (WarGreymon's `security_attack_fn`) and a flat aura `grant_keyword: SecurityAttackPlus(1)` on the SAME body. They never collide: the formula provides the base-inclusive count (MAX'd, but there is only one formula) and the flat grant is summed via `security_attack_keyword_bonus`. `tests/cards_behavioral/bt1/bt1_085.rs` proves WarGreymon+Tai checks 4 and WarGreymon+Greymon+Tai checks 5 — three independent `<Security A.>` sources (formula + inherited keyword + aura-granted keyword) single-counting and summing, matching DCGO `Strike_AllowMinus`. The flat-delta-vs-formula authoring rule: a count-conditional grant whose *value* is flat (count as FILTER) uses `grant_keyword`/`security_attack`; only a grant whose *value* varies with the count uses `security_attack_fn`.
- **Distinct, still-open facet:** the `#[ignore]`d `st1_07_security_attack_plus_installed_on_field_via_modifier` (and BT21-029's sibling) target **face-up OWN-scope** `grant_keyword` runtime installation (G-DECLARATIVE-KEYWORD) — a different path than the buried-source inherited-strike aggregation closed here. They stay ignored: ST1-07's grant is inherited-only and correctly does not apply when it is the top card.

### 🔴 OPEN (2026-06-02) — `event_target_*` predicates resolve to the NEW attack target, not the attacker whose target changed  [G-ATC-EVENT-TARGET-IS-NEW-TARGET]
- **Surfaced by:** the `/archetype-interaction-test-author` capstone on **Medusamon** — `tests/archetypes/medusamon.rs::raid_redirect_fires_lamiamon_attack_target_change_trash_security` (currently `#[ignore]`d with this gap id).
- **Symptom:** BT21-025 Lamiamon's `[Your Turn][OPT]` clause ("When any of **your** [Reptile]/[Dragonkin] trait Digimon's attack targets change, trash your opponent's top security") does **not** fire when an attack-target change is produced by the real `<Raid>` combat path (a Dragonkin `<Raid>` attacker, e.g. BT24-011 Cyclonemon, redirecting onto the opponent's highest-DP Digimon). Opponent security is untouched.
- **Root cause:** for an `OnAttackTargetChange` trigger, `dsl_cards/predicate.rs::event_target_owner` (and the sibling `event_target_trait_has` / `event_target_*` family) prioritize `trigger.attack_target_change.new_target` — i.e. the **redirected-to Digimon** — over `trigger.event_permanent`, which the combat layer sets to the **attacker** (`effect_queue.rs` ~line 1220: `AttackTargetChanged { … } => { event_permanent: Some(attacker), attack_target_change: Some(AttackTargetChange { attacker, … }) }`). So BT21-025's gate (`event_target_owner: you` + `event_target_trait_has: Dragonkin`) is evaluated against the opponent-owned, non-Dragonkin **new target** and fails.
- **Faithfulness basis:** the trait gate belongs to the **attacker whose target changed**, not the new target. DCGO `BT21/Red/BT21_025.cs` `PermanentCondition` → `CardEffectCommons.IsPermanentExistsOnOwnerBattleAreaDigimon(permanent, card)` + `EqualsTraits("Reptile"|"Dragonkin")` over the switching attacker; card text "any of **YOUR** [Reptile]/[Dragonkin] trait Digimon's attack targets change". general_rule.pdf §16 (`<Raid>` switch) + attack-target-change observer timing.
- **Why per-card tests miss it:** `tests/cards_behavioral/bt21/bt21_025.rs` fires the event via `TriggerSource::EventObserved { permanent: attacker }` (no `attack_target_change` context), so `event_target_owner` falls through to `event_permanent` = attacker and the clause fires. Only the real-combat (Raid redirect) path exposes the divergence — a cross-card / cross-path interaction the per-card TDD cannot see.
- **Suggested fix direction (not applied — interaction-test runs don't edit engine code):** either (a) add an attacker-scoped predicate family (`event_attacker_owner` / `event_attacker_trait_has`) that reads `attack_target_change.attacker`, and re-author BT21-025 (and any other "your X's attack target changes" card) to use it; or (b) decide which entity `event_target_*` should mean for `OnAttackTargetChange` and align both the predicate resolution and every consumer. Same trash-on-target-change shape recurs on other Reptile/Dragonkin cards, so the substrate choice amortizes.

## Resolved gaps

Resolved Rust engine group summaries have been moved to [qa/resolved-gaps.md](../qa/resolved-gaps.md#rust-engine-gap-group-summaries).

## G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED — trailing `if` step not executed after a parked selection resumes (2026-06-06)

- **Card(s):** BT25-050 Kiwimon (orphan-b slice). Printed: "[On Play][When Digivolving] You may suspend 1 Digimon. Then, **if there are 2 or more suspended Digimon**, 1 of your opponent's Digimon can't unsuspend until their turn ends."
- **What's broken:** when a DSL clause's `process` is `[ select_* (interactive), <mutation>, if { condition } { then: [...] } ]`, the trailing `if` step is **not executed** after the interactive selection parks-and-resumes. The process completes (`pending_selection` returns to `None`) without ever evaluating/entering the conditional block that follows the selection.
- **Proof (tests/cards_behavioral/bt25/bt25_050.rs):** with `select_any_permanent` (suspend) followed by `if { count_gte suspended >= 2 } { select_opponent_permanent + add_modifier CannotUnsuspend }`, the lock select never installs (`pending=None` immediately after the suspend action; no CannotUnsuspend applied) even when 3 Digimon are suspended. Replacing the `if { ... }` wrapper with an **unconditional** `select_opponent_permanent + add_modifier` lock fires correctly (`pending=Some(OppField)`, lock applied). So the trailing UNCONDITIONAL step resumes fine; only the trailing CONDITIONAL (`if`) step is skipped on resume. (Contrast: BT25-058 Callismon, whose post-suspend lock is unconditional, works today.)
- **Faithfulness impact:** the count gate is REQUIRED — locking unconditionally would lock an opponent Digimon even with fewer than 2 suspended, which is unfaithful. So BT25-050 cannot be shipped via the unconditional workaround.
- **Suggested fix:** in the effect-queue / DSL process-resumption path, after a parked `select_*` step resolves, continue executing ALL subsequent sibling steps including `CompiledStep::If` (re-evaluate its condition against post-selection state, then run its `then`/`else_branch`). Today the resume appears to stop at / skip the first conditional step after a selection.
- **Verdict:** BT25-050 BLOCKED (gap_kind: engine). YAML authored faithfully (count-gated lock) and committed; the lock behavioral test is `#[ignore = "pending: G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED"]`. Suspend (clause 1 step 1) and the inherited +1000 DP aura are implemented and tested green.

---

**Updated 2026-06-07 (link-finish-appmon slice — facet #10 + App Fuse RESOLUTIONS confirmed in production):** 🟢 Three BT25 Appmon Link cards previously logged BLOCKED here are now IMPLEMENTED as DSL YAML — confirming the underlying primitives landed and are production-faithful:
- **BT25-004 Tapmon** and **BT25-045 Onmon** (facet #10 — host-filtered optional `WhenWouldLink` cost reducer): the predicated reducer (Gap 5) is wired end-to-end via DSL `when: when_would_link_to_this` + `active_when: { would_link_card_trait_any_of: [Social, Tool, Game] }` + `process: [{ reduce_link_cost: { amount: 1 } }]` (optional + once_per_turn), lowering to `EffectContext::reduce_pending_link_cost` (one-shot, no modifier leak). Tapmon's reducer is `scope: inherited`; Onmon's is face-up. Behavioral tests prove a Social-trait link's cost drops 1→0 on accept and a non-matching trait pays full cost. `tests/cards_behavioral/bt25/bt25_004.rs` (5), `bt25_045.rs` (6).
- **BT25-036 Craftmon** (App Fuse): the App Fusion alt-play is implemented end-to-end (`alt_paths: [{ kind: app_fusion, materials: [{ filter: { name_in: [...] } }], cost: 0 }]` → `app_fusion_digivolve_route_for_card` host-eligibility (2 distinct named cards linked) → digivolve route stacks the App-Fusion card on top + drains the host's `linked_cards` under it as sources). Surfaces through the normal `encode_digivolve` action/mask. `tests/cards_behavioral/bt25/bt25_036.rs` (9) + `bt25/app_fusion.rs` (4, mechanic). The prior "AltPathKind::AppFusion parses but resolves to nothing" note is **stale**.

**Still open (BT25-089 Kazuki & Itsuki, PARTIAL):** the `[End of Your Turn][OPT]` clause "1 of your Digimon may **app fuse into a Digimon card in the hand**" is an *effect-initiated* app-fuse (a field Digimon fuses INTO a chosen hand card; DCGO `playCardClass.SetAppFusion`) — distinct from the alt-path App Fusion (which is a play-from-hand digivolve). No DSL process-step / engine primitive exists for the effect-initiated direction yet. BT25-089 also still omits the `[Main]` link source "from your Digimon's digivolution cards" (Tamer-anchored multi-Digimon source scan). Both remain logged; BT25-089 stays PARTIAL.

## Move a security card to a deck (top/bottom)  [G-ENGINE-SECURITY-TO-DECK]  — OPEN 2026-05-29

Surfaced by judge-quiz first wave (LM-020 Quantumon, BLOCKED). No public `EffectContext` method moves a card from a player's **security stack** to a **deck**. The private `move_card_to_deck` helper (`code/digimon-engine/src/effect_context/mod.rs`) is sourced from trash only; security removers route to hand / play / trash (`add_to_hand_from_security`, `play_security_card`, `trash_selected_security`).

- **Suggested primitive:** `pub fn return_security_card_to_deck(&mut self, player: PlayerId, card: CardHandle, to_bottom: bool) -> bool` — locate the card in `player.security`, `ensure_security_materialized`, remove it, drop from `face_up_security`, fire `fire_security_removed_observers` with a new `SecurityRemovalDestination::Deck` variant (parallel to `::Hand`), then route through the existing trash->deck `move_card_to_deck` path.
- **DSL prerequisite:** the verb `return_selected_security_to_deck` (`G-DSL-RETURN-SELECTED-SECURITY-TO-DECK` in `qa/dsl-vocab-gaps.md`) lowers to it.
- **Blocks:** LM-020 (judge-quiz Q18) [When Digivolving]. DCGO `LM_020.cs`: `IReduceSecurity` -> `AddLibraryTopCards` -> shuffle.


## Opponent plays a Digimon from THEIR OWN trash, SUSPENDED, opponent-selected  [G-OPPONENT-PLAY-FROM-OWN-TRASH-SUSPENDED]  — OPEN 2026-05-29

Surfaced by judge-quiz wave (EX5-060 Dragomon, BLOCKED; pins Q28 alongside BT20-059 Gankoomon X). Hybrid engine+DSL gap.

- **Effect text (EX5-060 Clause 1):** "[On Play] [When Digivolving] Your opponent plays 1 level 4 or lower Digimon card from their trash **suspended** without paying the cost. [On Play] effects on Digimon played by this effect don't activate."
- **What's missing (engine):** no `EffectContext` primitive plays a card from an *arbitrary* player's trash **suspended**. `play_from_trash_free_unsuspended*` hardcodes `self.player` (the controller) as the player who plays — the DSL `play_from_trash_free { of: opponent }` `of:` field is dropped at the engine boundary (the compiled handler looks up the trash card by the bound owner but then calls `ctx.play_from_trash_free_unsuspended(handle)`, which searches `self.player`'s trash and no-ops when the handle lives in the opponent's trash). It also always plays UNSUSPENDED — neither `play_from_trash_with_cost*`, `play_from_trash_free_unsuspended*`, nor the underlying `Game::play_from_trash_with_cost_suppress` chain has a `suspended`/`is_tapped` parameter (DCGO `EX5_060.cs`: `PlayPermanentCards(payCost:false, isTapped:true, root:Trash, activateETB:false, selectPlayer:card.Owner.Enemy)`).
- **Suggested primitive:** `pub fn play_from_trash_for_player_suspended(&mut self, player: PlayerId, trash_index: usize, suspended: bool, suppress_on_play: bool) -> Option<PermanentHandle>` — plays from `player`'s trash (not `self.player`'s), entering suspended when requested, and surfaces the selection to `player` (the opponent) via `override_selecting_player`. Thread a `suspended` bool through `Game::play_from_trash_with_cost_suppress` (and the shared `play_from_hand_with_cost_result_from_origin_suppress` path it delegates to) so the just-created permanent is marked `is_suspended` at materialization, parallel to the existing `suppress_on_play` thread (PUPPETS-G030). The `[On Play] don't activate` half is ALREADY supported via `suppress_on_play: true`.
- **DSL prerequisite:** extend `play_from_trash_free` with a `suspended: bool` flag AND honor its `of:` field for non-controller players (route to the new primitive when `of != controller`). Today `of:` is silently ignored. Pair with `as_selecting_player { of: opponent }` so the opponent makes the pick.
- **Q28 note:** the `[On Play] don't activate` lock is modeled as a `CannotActivateOnPlayEffects` modifier added to the just-played opponent permanent via `EffectContext::add_modifier`, whose `can_affect_permanent` guard already lets a protected target (Gankoomon X) dodge the lock — verified by the live `ex5_060_lock_does_not_attach_to_effect_immune_target` test. Only the play-and-suspend substrate is blocked, not the lock.
- **Blocks:** EX5-060 (judge-quiz Q28) Clause 1. `code/digimon-engine/cards/ex5/EX5-060.yaml` Clause 1 declared with faithful timing but empty (gap-blocked) `process`. Tests `ex5_060_clause1_*` `#[ignore]`'d with this gap-id.


## G-PLAY-TOKEN-FLOODGATE — `play_token` bypasses `CannotPlayDigimonByEffect` (RESOLVED 2026-05-30)

- **RESOLVED 2026-05-30.** `EffectContext::play_token`
  (`code/digimon-engine/src/effect_context/mod.rs`) now returns `None` (no token
  spawned) when the controller carries `CannotPlayDigimonByEffect`, mirroring the
  hand/trash play-gate — every registered token is a Digimon token (see
  `token_registry`), matching DCGO's `CanPlayAsNewPermanent` →
  `CanNotPutFieldClass(IsDigimon)`. Pinned by lib unit tests
  `effect_context::tests::{play_token_blocked_by_cannot_play_digimon_by_effect,
  play_token_allowed_without_floodgate}` and the un-ignored interaction tests
  `archetypes/puppets.rs::{s1_pillomon_floodgate_blocks_effect_token_plays,
  s1b_without_pillomon_the_same_token_play_succeeds}`. Additive guard (no-op when
  the modifier is absent); behavioral + archetypes suites regression-clean.
- **First seen:** 2026-05-30, Puppets archetype interaction test
  `s1_pillomon_floodgate_blocks_effect_token_plays` (`#[ignore]`'d) in
  `code/digimon-engine/tests/archetypes/puppets.rs`.
- **Symptom:** with BT9-033 Pillomon ("Players can't play Digimon by effects")
  on the field — installing `ModifierType::CannotPlayDigimonByEffect` — an
  effect that plays a **Familiar Token** (e.g. ST19-12 Cendrillmon's "play 2
  Familiar Tokens") still spawns the tokens. The flood-gate that correctly
  blocks effect-driven hand/trash plays does not block token spawns.
- **Root cause:** `EffectContext::play_token`
  (`code/digimon-engine/src/effect_context/mod.rs`) pushes the new token
  permanent directly onto `battle_area` and does NOT consult
  `CannotPlayDigimonByEffect`, unlike `play_from_hand_free` /
  `play_from_hand_with_cost` (which gate on it — see the `selections.rs` and
  `game_actions.rs` gate sites). Token plays therefore bypass the lock.
- **DCGO-verified faithful behaviour:** a Digimon Token is a Digimon; DCGO
  blocks its play under this lock. `CardEffectCommons.PlayToken` calls
  `CanPlayAsNewPermanent(...)`, which enforces BT9-033's `CanNotPutFieldClass`
  whose card-condition is `cardSource.IsDigimon || cardSource.IsDigiEgg` —
  `IsDigimon` is `true` for the Familiar Token, so the play is blocked
  (`$BASE_DCGO/Assets/Scripts/CardEffect/BT9/Yellow/BT9_033.cs`,
  `Script/CardEffectCommons.cs`).
- **Fix (engine primitive):** have `play_token` consult
  `CannotPlayDigimonByEffect` for the controller (the token is a Digimon
  played by an effect) and no-op the spawn when the gate is installed,
  mirroring the hand/trash play path. Then flip
  `s1_pillomon_floodgate_blocks_effect_token_plays` to un-ignored.
- **Blast radius:** every token-spawning effect interacting with a
  `CannotPlayDigimonByEffect` source (Pillomon BT9-033 and any future
  "can't play Digimon by effects" floodgate). Additive guard; no behaviour
  change when the modifier is absent.


## Observe an effect adding cards to a player's hand  [G-ON-ADD-TO-HAND-OBSERVER]  — RESOLVED 2026-06-04

**RESOLVED 2026-06-04.** Implemented the `OnAddToHand` observer end-to-end:
- **Engine:** new `TriggerSource::HandGained { player, effect_initiated }` (fans out to ALL battle areas, carrying the gaining player in `TriggerContext.affected_player`); `Game::fire_on_add_to_hand_by_effect(player)` enqueues it. Called from every EFFECT-driven hand-gain sink — `return_to_hand` (game-level), `add_to_hand_from_{deck,trash,security,reveal}`, `add_pending_security_to_hand`, `return_card_source_to_hand`, and `EffectContext::draw` (effect-draws like Akihiro Kurata's). The normal turn-start draw does NOT route through these, so it never over-fires (DCGO's `cardEffect != null` gate).
- **DSL:** `when: on_add_to_hand` trigger token (`Timing`/`CompiledTiming`/`compile_timing`/`timing_map`) + the `event_add_to_hand_player:` player-ref predicate (compares `affected_player` to you/opponent). Compose with the existing `event_is_effect_initiated:`.
- **Card:** BT11-033 MirageGaogamon clause 2 authored (`gain_memory_fn` over `floor_div(card_count_in_zone{of: opponent, zone: hand}, 4)`, `once_per_turn`, gated on `event_add_to_hand_player: opponent`).
- **Tests:** `tests/cards_behavioral/bt11/bt11_033.rs` — un-ignored memory tests (gain = floor(opp_hand/4): 3→0, 4→1, 8→2), opponent-only gating, OPT lockout + per-turn reset; structural positive assertion. Regression-clean (`cards_behavioral` 3894 pass, `judge_quiz` 25, `combat` 213, `option_flow` 93, lib 212).

Original report (for context):


Surfaced by BT11-033 MirageGaogamon (clause 2 BLOCKED, omitted from YAML per no-approximations). Hybrid engine + DSL gap. The card's clause 1 ([When Digivolving] return + security fallback) is fully implemented; only the second clause is blocked.

- **Effect text (BT11-033 clause 2):** "[All Turns] [Once Per Turn] When an effect adds cards to your opponent's hand, gain 1 memory for every 4 cards in your opponent's hand." DCGO `BT11_033.cs`: `EffectTiming.OnAddHand`, `CanUseCondition` gates on `CanTriggerWhenAddHand(player => player == Owner.Enemy, cardEffect => cardEffect != null)` — fires only for *effect-initiated* adds to the **opponent's** hand (not normal draws) — and gains `Owner.Enemy.HandCards.Count / 4` (integer floor, read at trigger time), OPT-locked via `SetHashString("GainMemory_BT11_033")`.
- **What's missing (engine):** `EffectTiming::OnAddToHand` is **defined** in `code/digimon-engine/src/enums.rs` but is **never dispatched** anywhere — no `enqueue_triggered(OnAddToHand, ..)` / `fire_*` call site exists. The card-to-hand movement paths (`Game::add_to_hand_from_*`, `return_to_hand`, `add_top_security_to_hand`, etc.) do not fan this timing out to battle-area observers, and the `TriggerContext` carries no "which player's hand gained cards / was it effect-initiated" payload for it.
- **What's missing (DSL):** the `TriggerWhen` enum (`code/digimon-dsl/src/clause.rs`) has no `on_add_to_hand` token and `compile.rs` has no mapping, so no `when: on_add_to_hand` clause can be authored. A companion event-card predicate (e.g. `event_add_to_hand_player: opponent`) plus the existing `event_is_effect_initiated` leaf would gate it to "an effect added cards to the **opponent's** hand."
- **Composable today (do NOT re-file):** the memory ARITHMETIC is already expressible. `gain_memory_fn` accepts a `FormulaSpec`, and `floor_div: [ {base:0, per:{card_count_in_zone:{zone: hand, of: opponent}}, delta:1}, 4 ]` computes `floor(opp_hand_size / 4)`. Only the **observer trigger** is missing — once it lands, append the clause sketched in the BT11-033 YAML header comment.
- **Suggested fix:** (1) fan `OnAddToHand` out to battle-area observers at each effect-initiated hand-gain commit (mirroring how `OnDrawCard` / `OnReturn` are fired), threading the gaining player + an `effect_initiated` flag into `TriggerContext`; (2) add the `on_add_to_hand` DSL trigger token + an `event_add_to_hand_player` predicate. Distinguish effect-initiated adds from normal draws (DCGO's `cardEffect != null` gate) so the clause does not over-fire on the draw step.
- **Blocks:** BT11-033 (clause 2). `code/digimon-engine/cards/bt11/BT11-033.yaml` omits the clause; tests `bt11_033_memory_gain_is_floor_of_opp_hand_over_four` and `bt11_033_memory_observer_is_once_per_turn_and_clears_after_end_turn` are `#[ignore]`'d with this gap-id (and `bt11_033_does_not_author_the_blocked_memory_observer` guards against an approximation landing on the wrong trigger). Likely shared by every "when an effect adds cards to (a player's) hand" observer in the pool.

## BeforePayCost reduction whose amount is set by an interactive in-cost selection  [G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT]  — RESOLVED 2026-07-03
**Status: RESOLVED (2026-07-03).** Interactive pay_cost park/resume existed (BT25-088); closed the two holes: (1) pay-cost actionability guard in lower_cost_reduction.rs (no phantom reduction when the sacrifice has no target, DCGO CanActivateCondition); (2) new DSL step `delete_for_cost_reduction` snapshots the deleted permanent's pre-removal printed cost (rule 25) into `Game::pending_cost_reduction_amount_override`, drained by the play/digivolve/option continuations. BT13-103 clause 1 authored + behavioral test un-ignored. Provers: tests/cost_hooks/pay_cost_play_delete_reducer.rs (10, incl. clone-mid-park).

Surfaced by BT13-103 Akihiro Kurata (clause 1 BLOCKED, omitted from YAML per no-approximations). Hybrid engine + DSL gap. Clauses 2 ([End of Opponent's Turn][OPT] draw/trash/place-self/delete) and 3 ([Security]) are fully implemented; only clause 1 is blocked.

- **Effect text (BT13-103 clause 1):** "[Your Turn] When you would play a card with [Belphemon] in its name, by deleting 1 of your Digimon with [Gizmon] in its name, reduce the play cost by the play cost of the deleted Digimon." DCGO `BT13_103.cs` (EffectTiming.BeforePayCost): `SelectPermanentEffect` over own non-immune [Gizmon]-name Digimon, `canNoSelect: true`; on a pick, delete it, then install a `ChangeCostClass` of `-permanent.CostJustBeforeRemoveField`. The reduction magnitude is the **interactively-selected** Digimon's printed cost — known only AFTER the in-cost selection. (DCGO's AI-only `EffectTiming.None` mirror auto-picks `gizmonCosts.Max()`; we may NOT replicate that approximation under rule 17.)
- **What's missing (engine):** the BeforePayCost cost-reduction scan computes the amount and pays the cost in two decoupled steps. `apply_cost_reduction_candidate` (`code/digimon-engine/src/game_actions.rs:5848`) evaluates the amount (`inspect_cost_reduction_candidate` → `cost_reduction_fn`, READ context) **before** invoking `pay_cost_fn`, so the amount cannot depend on anything the pay_cost selects. The scan also requires the pay_cost to resolve synchronously (no parked `pending_selection`) and skips paid reducers entirely when there is no real cost target (`game_actions.rs:5678`). There is no mechanism to (a) surface an interactive selection during a BeforePayCost pay_cost and resume, nor (b) feed the selected/paid permanent's cost back into the reduction amount.
- **What's missing (DSL):** the `kind: cost_reduction` lowering (`code/digimon-engine/src/dsl_cards/lower_cost_reduction.rs:193`) runs `pay_cost` in a **fresh** `Bindings`, isolated from the `amount_fn` callback, and treats only `RunOutcome::Synchronous` as success. No `FormulaSpec` (`code/digimon-dsl/src/formula.rs`) reads "the printed cost of the permanent(s) deleted/paid during this pay_cost" — `BindingPlayCost` reads a prior `bind_as` binding only.
- **Suggested fix:** (1) let a cost-reduction `pay_cost` install + resume an interactive `pending_selection` during the BeforePayCost scan; (2) bind the selected/paid permanent into a scope the amount can read, e.g. a `paid_cost_total` formula (sum of printed costs of permanents deleted/paid during this pay_cost) or a `BindingPlayCost` over a pay_cost-produced binding; compute the final reduction AFTER the cost resolves. Backward-compatible: literal `amount:` and the existing synchronous self-suspend / return-to-deck pay_costs are unchanged.
- **Blocks:** BT13-103 (clause 1). `code/digimon-engine/cards/bt13/BT13-103.yaml` omits the clause (documented BLOCKED in its header); test `bt13_103_belphemon_play_cost_reduced_by_deleted_gizmon_cost` in `code/digimon-engine/tests/cards_behavioral/bt13/bt13_103.rs` is `#[ignore]`'d with this gap-id. Shared by any "by deleting/paying X, reduce the play cost by X's cost" reducer where X is chosen interactively. Companion DSL-vocab entry: `qa/dsl-vocab-gaps.md` `G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT`.

## Granted `OnBlock` trigger fires for any carrier, not just the blocked permanent  [G-ENGINE-GRANTED-ONBLOCK-CARRIER-GATE]  — OPEN 2026-06-05

Surfaced by **BT4-098 Atomic Inferno** (judge-quiz Q4 authoring). The `[Main]` clause grants the
selected Digimon a temporary "[Your Turn] When **this Digimon** is blocked, gain +3 memory" trigger
via `grant_triggered_effect { timing: on_block, body: [gain_memory: 3] }`.

- **What's wrong:** the granted-trigger dispatch for `OnBlock` fires the granted body for **every**
  battle-area carrier holding the grant, without checking that the *blocked* permanent (the
  `BlockDeclared { attacker, .. }` event permanent) is the carrier. DCGO gates this via
  `CanTriggerOnAttack(selectedPermanent)` (the granted body's `CanUseCondition` requires the blocked
  attacker == the selected permanent). So if a *different* Digimon is blocked while a granted carrier
  is also on the field, the carrier's grant over-fires.
- **Scope of impact:** the **Q4-relevant** behavior is unaffected (Q4 uses Atomic Inferno's
  `<Security A.>` clause, not the on_block grant). The positive case the card tests assert (the
  granted Digimon IS the one blocked) is correct; only the cross-Digimon over-fire is wrong. The
  card YAML is faithful (`on_block` + `gain_memory: 3`); the gap is engine-side in the granted-trigger
  dispatch, not the DSL. Flagged in the BT4-098 YAML header.
- **Suggested fix:** in the `OnBlock` branch of the granted-trigger dispatch (`effect_queue.rs`,
  `BlockDeclared` source), gate firing on `event_permanent == carrier` (the granted body should run
  only when the carrier is the blocked permanent), mirroring DCGO `CanTriggerOnAttack(selected)`.
  Likely shared by any granted "[when this is blocked / when this attacks]" body that targets a
  specific carrier.
- **Blocks (precision only):** BT4-098 on_block grant. YAML:
  `code/digimon-engine/cards/bt4/BT4-098.yaml`; tests:
  `code/digimon-engine/tests/cards_behavioral/bt4/bt4_098.rs` (positive case green).

## Multi-card `trash_top_security` aborts when a per-removal observer installs a selection  [G-TRASH-SECURITY-BATCH-INTERRUPTED-BY-OBSERVER]  — OPEN 2026-06-06

Surfaced by **BT23-102 Mastemon** (judge-quiz Q9 authoring). Mastemon's [When
Digivolving] trims "both players' security stacks so they have 3 cards left" — a
`trash_top_security` of `count = security - 3` per player.

- **What's wrong:** when the trim removes the **controller's own** security, each
  removal synchronously fires the carrier's own `[All Turns]` `OnLoseSecurity` OPT
  (Mastemon's "you may place 1 Digimon as the bottom security card"). After the
  1st removed card, that observer installs an interactive `pending_selection`, and
  `EffectContext::trash_top_security` early-returns on `pending_selection.is_some()`
  — so the remaining controller removals are dropped (controller stays above 3).
  Root: `fire_effect_security_removal` enqueues + drains `OnLoseSecurity`
  synchronously per card (`effect_queue.rs`), and `trash_top_security` aborts its
  loop on the resulting pending selection (`effect_context/mod.rs`).
- **Scope of impact:** the **opponent**-side trim is unaffected (Mastemon's
  `OnLoseSecurity` is owner-gated, so removing the opponent's security fires no
  observer) and reaches exactly 3 — so the judge-quiz Q9 pin (opponent trim) is
  green. Only the controller-side multi-trim is incomplete.
- **Suggested fix:** let a multi-count `trash_top_security` either (a) complete its
  batch before draining the per-card `OnLoseSecurity` observers, or (b) park and
  resume across the interactive observer selection (capture the remaining count and
  continue after the selection resolves) rather than early-returning and dropping it.
- **Blocks:** BT23-102 controller-side [When Digivolving] trim. YAML:
  `code/digimon-engine/cards/bt23/BT23-102.yaml`; test
  `bt23_102_when_digivolving_controller_trim_removes_own_security` is `#[ignore]`'d
  citing this gap. Shared by any effect that removes >1 of its OWN security while
  carrying a self `OnLoseSecurity` observer.

## Parametric keyword token inside a conditional/formula clause is double-parsed as a flat keyword (2026-06-08)

- **Symptom:** WarGreymon (ST1-11) reports `<Security A.>` total **4** when it should
  be **3**. With 4 digivolution sources, the printed effect "[Your Turn] For every 2
  digivolution cards this Digimon has, it gains ＜Security A. +1＞" grants Security A.
  +2 (two pairs) on top of the base 1 check → 3 security checked. Engine resolves 4.
  This is a **combat** bug, not just display: `effective_security_strike` returns 4, so
  WarGreymon really checks one extra security card. Found live via the desktop debug
  bridge (`digimon-scenario-mcp`) during a starter-deck game.
- **Root cause:** double-count. The DSL models the effect correctly as a base-inclusive
  `security_attack_fn` formula (`ST1-11.yaml`): `floor((2 + material_count)/2) = 3` for
  4 sources. But `parse_printed_keywords` (`card_data.rs:531`) scans the effect text for
  any `＜…＞` token and extracts the literal `＜Security A. +1＞` (the per-pair unit inside
  the conditional clause) as an **unconditional** `Keyword::SecurityAttackPlus(1)`
  (`card_data.rs:595–612`). `raw_security_strike` (`combat/mod.rs:2485–2507`) then sums
  `base_checks (formula) 3 + sa_keyword (parsed) 1 = 4`. Both display wires
  (`serialization.rs:388`, `engine_commands.rs:488`) faithfully mirror the engine via
  `effective_security_strike - 1`, so the display is *correct given the wrong engine
  value* — `serialization.rs:385` even documents the mistaken belief that 4 is right.
- **Scope:** any card whose **conditional/formula** text embeds a parametric keyword
  token — `＜Security A. +N＞`, and likely `＜Draw N＞` / `＜De-Digivolve N＞` — inside a
  "For every / While / it gains …" clause is at risk of the same flat double-parse. The
  parser cannot distinguish a standalone granted keyword from one that is the unit of a
  formula already modeled by the DSL.
- **Suggested fix:** stop the parser from emitting a flat parametric keyword when the
  token is part of a conditional/formula grant. Options: (a) when a card defines a
  `security_attack_fn` (or analogous formula aura), suppress the auto-parsed flat
  `SecurityAttackPlus/Minus` for that card; (b) make `parse_printed_keywords` only treat
  a parametric keyword as flat when it is a standalone declaration (not preceded by
  "gains"/"For every"/"While" in the same clause); or (c) a per-card `card_overrides`
  keyword suppression. (a) is the most principled — the DSL formula is the source of
  truth and the printed token is reminder text.
- **Repro / regression:** `qa/scenarios/st1-11-wargreymon-security-attack-doublecount.json`
  (minimal: WarGreymon over 4 sources, P1's turn). The assertion vocabulary has **no
  security-attack assertion kind** (secondary gap, STILL OPEN — add `security_attack` to
  the `/debug` evaluator + fixture schema), so the durable test is a Rust real-card-data
  test asserting `game.effective_security_strike(handle) == 3`.
- **RESOLVED 2026-06-08 at the ROOT CAUSE (parser), via the `fix-innate-keyword-overextraction`
  change** — superseding an earlier combat-site patch. The real bug was that
  `parse_printed_keywords` (`card_data.rs`) inferred a card's INNATE keywords by scanning the
  whole effect blob, so WarGreymon's `<Security A. +1>` (the reminder unit of its
  base-inclusive `security_attack_fn` formula) was extracted as a flat `SecurityAttackPlus(1)`
  and added on top of the formula → 4 (and +1 even off-turn). The fix replaces the blob scan
  with a **per-token left-context classifier**: a `<kw>` is innate only when its left context
  is empty / ends with `]` (a `[Timing/Location]` label) / `)` (a prior keyword reminder) /
  `＞` (a chained keyword) — i.e. it is a printed attribute, not introduced by a grant verb
  (`gains <…>`), a filter (`with <…>`), or a DP-and-keyword comma. WarGreymon's token
  (`…it gains <Security A. +1>`) is now correctly NOT innate, so `security_attack_keyword_bonus`
  returns 0 and the formula alone governs: 3 on your turn, 1 off-turn. A pool-wide audit
  (`openspec/changes/fix-innate-keyword-overextraction/keyword-{diff,audit}.md`) confirmed
  0 genuine regressions across the implemented pool; full `cards_behavioral` suite: 4521 pass,
  0 fail. Regression tests (build from `full_card_data()` so the phantom is actually parsed):
  `cards_behavioral/st1/wargreymon_security_attack.rs::wargreymon_real_data_clean_stack_checks_three_on_your_turn`
  (→3) and `..._checks_one_off_turn` (→1). The earlier `combat/mod.rs` subtraction + the
  `top_card_has_security_attack_formula` / `top_face_security_attack_keyword_bonus` helpers
  were removed (one mechanism, at the root, not two).

## Keyword innate-vs-granted parsing — durable contract (2026-06-08)

- `parse_printed_keywords` (`card_data.rs`) now distinguishes a card's INNATE (printed-attribute)
  keywords from keywords its effects GRANT/reference, by per-token left-context (see the
  WarGreymon resolution above). This fixed a broad latent class: granted/conditional/formula
  keyword tokens (`Security A.`, `Draw`, `De-Digivolve`, boolean keywords) and target-filter
  references (`Delete a Digimon with <Blocker>`) were all being treated as innate attributes.
- **Deferred follow-up (not yet needed):** a fully DSL-authoritative keyword source — the
  unused `spec.keywords` field declaring innate keywords in YAML instead of parsing prose. The
  context-classifier proved robust enough (0 genuine regressions in the pool audit) that this
  re-architecture is not required now; revisit only if a future card layout defeats the
  context rule.

### App Fusion alt-play — RE-ADJUDICATION (2026-06-12, Appmon BT21/BT25 wave)
- **Status: ✅ NOT a gap (stale block cleared).** The App Fusion *alt-play digivolve method* (`AddAppfuseMethodByName`, DSL `kind: app_fusion`) is fully implemented and behaviorally green: `app_fusion_digivolve_route_for_card` (`code/digimon-engine/src/dna_digivolve.rs`) + `tests/cards_behavioral/bt25/app_fusion.rs` (`gap4_app_fusion_stacks_and_consumes_links`). **BT25-060 Rebootmon was re-adjudicated BLOCKED→IMPLEMENTED** on this basis (its sole prior blocker was the now-false "AltPathKind::AppFusion resolves to nothing"). Cards using the app_fusion alt-play path this wave: BT21-018, BT21-023, BT21-059, BT21-073, AD1-005, BT21-101, BT25-060 — all green.
- **Still a gap:** the *effect-initiated App Fuse* ("1 of your Digimon **may app fuse** into a Digimon card in the hand/trash") — see the "App Fuse keyword/primitive" entry above. Distinct operation (field↔hand/trash Digimon swap initiated by an effect), no `EffectContext::effect_initiated_app_fuse`. Blocks the *riders* of BT21-084, BT24-087, BT23-079, P-241, BT25-089 (those cards ship PARTIAL with the rider omitted).

### Effect-initiated App Fuse — RESOLVED 2026-06-13
- **Status: ✅ RESOLVED.** The effect-initiated App Fuse primitive shipped: DSL `app_fuse` step (`from: hand|trash`, optional `result_filter`, `optional`) → `EffectContext::initiate_effect_app_fuse` (two engine-driven selections: own field permanent that has the named App-Fusion materials, then the result Digimon card in the source zone) → `Game::commit_effect_app_fuse` (zone-parameterized; stacks the result on the host and folds the consumed link cards under the new top, reusing the alt-play app-fusion commit). Eligibility reuses `Game::can_app_fuse_onto` (the alt-play route). No auto-pick — both selections surface to the action space (CLAUDE.md §17).
- **Cards closed:** BT21-084, BT23-079, P-241, BT24-087 (trash + System/Life/Transmutation filter) — all PARTIAL→IMPLEMENTED. **BT25-089** had its app-fuse rider shipped too, but remains PARTIAL on the *separate* `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` DSL gap (the [Main] link source "from your Digimon's digivolution cards"), which is unrelated to App Fuse.
- **Tests:** `tests/cards_behavioral/app_fuse_primitive.rs` (8 primitive tests) + `tests/app_fuse_commit.rs` (zone-parameterized commit) + per-card app-fuse tests on the 5 cards. Spec: `docs/superpowers/specs/2026-06-13-effect-initiated-app-fuse-design.md`; plan: `docs/superpowers/plans/2026-06-13-effect-initiated-app-fuse.md`.

## Effect-initiated DNA digivolve does not pay the result card's printed DNA cost  [G-ENGINE-DNA-PRINTED-COST]
**Status: RESOLVED (2026-07-03, DNA workstream).** `lower_dna_cost` routes `cost: printed` through printed-cost lookup (pair/hand-partner/field-trash variants); cost 0/free users unaffected. Tests: tests/dna_printed_cost_dsl.rs (4).
Surfaced by: EX6-072 Mega Digimon Assembly! (migrate-examples-to-dsl, 2026-06-14); PRE-EXISTING, shared by EX3-008 (`cost: printed`) and BT17-095.
`effect_initiated_dna_digivolve` / `effect_initiated_dna_digivolve_hand_partner` take a `cost: CostDelta`; `cost: printed` lowers via `lower_cost_delta` to `Reduce(0)` (play_digivolve.rs — "DNA printed-cost lookup is not available here"), so the result card's printed `dna_costs` are never deducted from memory. DCGO passes `payCost=true`.
Fix: a `cost: printed`/`dna_printed` that resolves the result card's printed DNA digivolution cost and pays it for the effect-initiated DNA verbs.
Status: open (pre-existing; affects all effect-initiated DNA cards).

## Effect-initiated DNA digivolve does not validate the result card's printed recipe  [G-ENGINE-DNA-RECIPE-ENFORCEMENT]
**Status: RESOLVED (2026-07-03, DNA workstream).** Commit-time recipe backstop on all three effect-initiated DNA verbs (both orderings, slash-colours) + mask-level exclusion already via dna_pair_can_reach_hand_card (pinned). Also fixed the latent card_store fingerprint bug (dna_costs content now hashed).
Surfaced by: EX6-072; PRE-EXISTING, shared by BT17-095 / EX3-008.
`Game::dna_digivolve_hand_partner_inner` (game/mod.rs:1895) merges the two chosen materials unconditionally — it does NOT validate them against the result card's printed DNA recipe (`jogressCondition`). DCGO gates with `CardSource.CanJogressFromTargetPermanents` (each material vs the result's `jogressCondition.elements[i].EvoRootCondition`). DSL selection filters are static and cannot depend on the dynamically-chosen result, so enforcement must live in the engine primitive (validate/abort illegal pairings; ideally restrict the candidate set).
Status: open (pre-existing; affects effect-initiated DNA cards).

## Store-champs June-2026 audit — Three Musketeers BeelStarmon + Galacticmon (2026-07-02)

Audited by `/assess-archetype-rust` against the DigiLab store-championship deck scoping
(`qa/archetype-qa/store-champs-june-2026-scoping.md`). Per-card verdicts live in the fix plans
`.claude/plans/rust-engine-gaps-three-musketeers-beelstarmon.md` and
`.claude/plans/rust-engine-gaps-galacticmon.md`. Existing entries reconfirmed as drivers (no
re-file): `OnAddDigivolutionCards` (BT25-005 already listed), "Cast-time stack-construction for
cost reduction" + "Cross-card effect re-firing (source-card variant)" (BT15-102 — both RESOLVED
2026-07-05 by the BT15-102 landing; see their entries).
Note: the Satellamon (BT21-074) protection bundle cites the *resolved* "Source-scoped
return-immunity modifiers" entry — verify at implementation; likely no longer a gap.

### Option USE routed from non-hand origins (trash / hand-or-trash union / reveal pool / digivolution stack), free or with cost delta
- **Status: 🟢 ENGINE RESOLVED / DSL PARTIAL (2026-07-03).** `Game::use_option_from(player, OptionSource, CostDelta)`: `OptionSource` gains `Revealed(CardHandle)` + `Source{host,card}`; ALL origins route through `play_option_core` (correct [Main] resolution + disposal, `OnUseOption`, mode-select); memory actually paid for `Reduce(n)` (printed 5, reduce 3 → 2 paid), stacking on field `BeforePayCost` reducers; `CannotPlayFromTrash` honored. EffectContext: `use_option_from_trash`/`use_option_from_revealed`/`use_option_from_source`. DSL verb `use_option_from_trash {of, filter, cost, optional}` shipped clone-safe (`UseOptionFromTrashStep` resume frame). **Residual DSL verbs RESOLVED 2026-07-03 (option-verbs reconciliation):** `use_option_from_revealed`/`use_option_from_sources`/`use_option_bound` all ship (cost? = free|printed|{reduce:N}); the `Source` origin resolves BOTH card_sources AND linked_cards. EX7-048 clause 1, BT25-085 use-facet, and BT21-062 union-use are all expressible. Tests: tests/dsl/option_source_use_cluster.rs (7). BT25-085 remains blocked ONLY on the trash-Option-as-COST facet (G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST, round 3). Tests: `option_lifecycle_cluster.rs` gap2_* (5/5).
- **Severity (was):** 🔴 BLOCKING
- **Discovered in:** Three Musketeers BeelStarmon (2026-07-02); Galacticmon (2026-07-02)
- **Card(s):** BT25-083 LadyDevimon ("use 1 [Three Musketeers] trait Option card from your trash with the cost reduced by 3"), BT21-062 Galacticmon ("use 1 [Ragnarok Cannon] from your hand or trash without paying the cost"), EX7-048 Gundramon ("Reveal the top 6 cards... You may use 1 [Three Musketeers] trait Option card among them without paying the cost"), BT25-085 BeelStarmon ("use 1 [Three Musketeers] or [TS] trait Option card from your hand or this Digimon's digivolution cards without paying the cost")
- **Effect text:** as per card list above.
- **What's missing:** The only Option-USE verb is `use_option_from_hand` (hand-only; free or opp-memory-capped). Every non-hand origin fails to reach `play_option_core`: `play_from_trash_with_cost` (play.rs:882) has no `CardKind::Option` -> option-use branch (the hand path does, play.rs:343); `play_from_revealed_free` routes only through the Digimon/Tamer permanent-play path (`commit_play_from_hand_after_reductions`); `select_own_sources` consumers (trash / replay-Digimon / return) never route an Option source into the use flow. Additionally no one-shot cost *delta* exists for an Option use (BT25-083 pays use cost minus 3; DCGO `ChangeCostClass(-3)` + `PlayOptionCards(root: Trash, payCost: true)`).
- **Suggested API shape:** Generalize the option-use entry point to an origin-parameterized `EffectContext::use_option_from(origin: OptionUseOrigin, cost: CostDelta)` where `OptionUseOrigin` covers Hand(idx) / Trash(idx) / Revealed(card) / Source(card_source_ref), all routing through `play_option_core` (resolve + correct disposal). DSL verbs: `use_option_from_trash`, `use_option_from_revealed`, `use_option_from_sources`, and a `use_option_bound` that consumes a `select_union_zone` binding (hand-or-trash). `cost:` accepts `free | printed | {reduce: N}`.
- **Workaround:** None faithful — playing the Option as a permanent from trash skips [Main] resolution/disposal; add-to-hand-then-use changes timing and RL decisions; hand-only modeling drops legal branches. BLOCKED.
- **Related:** "Option card play flow (resolve + trash vs. place-on-field)"; resolved "Unified `play_or_use_from_hand_free`" (hand-only sibling); "Filtered hand-or-trash origin-preserving free-play" (Digimon/Tamer-play only); "No player-scoped reducer registry".

### `move_self_option_under_permanent` doesn't compose with the standard Option [Main]-play path
- **Status: ✅ RESOLVED (2026-07-03).** `EffectContext::place_self_under_permanent(target, face_down)` claims the in-flight `pending_option` (mirroring the resolved `place_self_as_delay_option_permanent` claim precedence: security → pending_option → hand/trash) and seats the Option FACE-UP (settable) via the shared `Game::seat_card_source_under_permanent` (stable-identity target re-resolution); claiming empties the slot so `dispose_option` skips the Standard trash; effect tails gate on the returned bool. Field-Option sources still route to `move_field_option_under_permanent`. Tests: `tests/dsl/option_lifecycle_cluster.rs` gap1_* (3/3). Unblocks the P-180/EX7-070/EX7-071 [Main] tail (cards still to author).
- **DSL verb (2026-07-03):** `place_self_under_permanent: { target, face_down }` now dispatches to this primitive (G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL, qa/dsl-vocab-gaps.md — ✅ RESOLVED); EX7-071's [Main] tail is authored + green (`cards/ex7/EX7-071.yaml`).
- **Severity (was):** 🔴 BLOCKING
- **Discovered in:** Three Musketeers BeelStarmon (2026-07-02)
- **Card(s):** P-180 Bind Red Trigger, EX7-071 Hurricane Screw Shot, EX7-070 Der Blitz — all [Main] Options ending "Then, place this card as the bottom digivolution card of 1 of your [Three Musketeers] trait Digimon."
- **Effect text:** "Then, place this card as the bottom digivolution card of 1 of your [Three Musketeers] trait Digimon."
- **What's missing:** `move_self_option_under_permanent` (`effect_context/action/lifecycle.rs:222`) early-returns `false` unless `self.source_permanent` is set — i.e. the Option must already be a battle-area field permanent (`Game::move_field_option_under_permanent`, `option_lifecycle.rs:334`). On the standard `play_option_from_hand` [Main] path the card is still the in-flight `pending_option`, never a field permanent, so the placement silently no-ops and the Option goes to trash on normal disposal. Same lifecycle hole fixed for `place_self_as_delay_option` (resolved G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH, 2026-06-16); the equivalent `pending_option` claim was never added here. Placement is face-up (a real digivolution source), so `face_down` must be settable to `false`.
- **Suggested API shape:** When `source_permanent` is `None`, claim the in-flight Option from `game.pending_option` (mirror `place_self_as_delay_option_permanent`) and seat it via `Game::place_in_flight_option_as_bottom_source(target, face_down)`; gate the effect tail on success.
- **Workaround:** None faithful — modeling as Delay changes timing and exposes an illegal decline; as-is the placement is silently dropped. BLOCKED.
- **Related:** Resolved G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH (identical claim fix, different disposition); "Option card play flow" residual; the `face_down: bool` axis from the BEATBREAK/DATA SQUAD Tamer-stash entry.

### Board-wide leave-field replacement protecting OTHER filtered Digimon, paid by trashing an Option from the carrier's digivolution cards
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Three Musketeers BeelStarmon (2026-07-02)
- **Card(s):** EX7-048 Gundramon
- **Effect text:** "[All Turns] When any of your [Three Musketeers] trait Digimon would leave the battle area other than by your effects, by trashing 1 Option card from this Digimon's digivolution cards, they don't leave."
- **What's missing:** (a) a would-leave replacement whose subject is a *filtered set of OTHER owner permanents* (cause-gated "other than by your effects") — the existing `kind: replacement` / `when_would_leave_battle_area` substrate is self-scoped; this is the G-DSL-PROTECT-OTHER-BY-SELF-DELETE shape (BT25-039) with (b) a different cost: trash 1 Option from THIS carrier's `card_sources` (the BT25-085/BT25-083 trash-Option-from-sources-as-cost family). DCGO: `WhenRemoveField` + `IsOwnerPermanentToBeDeletedCondition` (own 3M-trait perm, `!IsByEffect`), select+trash an Option from `thisCardPermanent.DigivolutionCards`, then `willBeRemoveField = false`.
- **Suggested API shape:** `trigger: when_other_would_leave_battle_area` + `subject_filter` (per G-DSL-PROTECT-OTHER-BY-SELF-DELETE) with a `ReplacementCostBody` variant `trash_option_from_own_digivolution_cards: { of: carrier }` (fires `OnDigivolutionCardTrashed`), gating `cancel_leave: { target: replacement_subject }`.
- **Workaround:** None faithful (auto-selecting the Option/subject or ignoring the cost violates §17). BLOCKED.
- **Related:** G-DSL-PROTECT-OTHER-BY-SELF-DELETE (qa/dsl-vocab-gaps.md, BT25-039); resolved `WhenWouldBeDeleted` leave-field replacement framework (self-scoped substrate this extends).

### Source-return-to-deck-bottom observer trigger (`OnDigivolutionCardReturnToDeckBottom`)
- **Status: RESOLVED (2026-07-03).** `EffectTiming::OnDigivolutionCardReturnedToDeckBottom` + `TriggerSource::SourceReturnedToDeckBottom{player, host, host_card, card, cause=DeckBottom}` fired from `return_card_source_to_deck` (bottom route, per source, drained synchronously - OPT proven across multi-source returns). DSL `when: on_source_returned_to_deck_bottom`; host scope via `event_host_permanent_is_source`, name gate via `event_card_name_contains`. Provers: tests/source_returned_to_deck_bottom_observer.rs (4).
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT21-058 Snatchmon (inherited "[All Turns][OPT] When any [Vemmon] are returned to the bottom of the deck from this Digimon's digivolution cards, delete 1 of your opponent's Digimon with a play cost of 4 or less."), BT18-065 Snatchmon (inherited "When any [Vemmon] return to the bottom of the deck from this Digimon's digivolution cards, this Digimon unsuspends and gains <Blocker> until the end of your opponent's turn.")
- **Effect text:** as above.
- **What's missing:** `return_selected_sources_to_deck` / `return_card_source_to_deck` (`effect_context/action/sources.rs:891`) move a source to the deck but fire **no observer trigger**. No `EffectTiming` matches DCGO's `EffectTiming.OnDigivolutionCardReturnToDeckBottom` (`CanTriggerOnReturnToLibraryBottomDigivolutionCard`). The archetype's engine loop is broken end-to-end: P-094/BT21-060 *produce* the event as costs; these two cards *consume* it.
- **Suggested API shape:** `EffectTiming::OnDigivolutionCardReturnedToDeckBottom` + `TriggerSource::DigivolutionCardReturnedToDeck { host_permanent, returned_cards, position }`, fired after the move; DSL `when: when_source_returned_to_deck_bottom_from_self` + event-card name predicate; respect OPT accounting.
- **Workaround:** None — the effect cannot observe the event. BLOCKED.
- **Related:** `return_selected_sources_to_deck` (emitter half, resolved 2026-06-14); sibling observer timings (`OnReturn`, `OnAnyDeletion`); the replacement-cost entry below (its payment must also fire this trigger).

### Leave-field replacement paid by returning N own digivolution sources to the deck bottom
- **Status: RESOLVED (2026-07-03).** `ReplacementCostBody::return_own_sources_to_deck{filter, count, position}` lowers to SelectOwnSources(min=max=count, target=source) + ReturnSelectedSourcesToDeck + CancelReplacement; requires outcome: prevent + optional: true; not-offered-when-unpayable via the filter+count-aware SelectOwnSources preflight (`matching_own_source_count`); payment fires the source-return observer (cross-gap prover). Provers: tests/return_own_sources_leave_replacement.rs (4).
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT21-062 Galacticmon ("[All Turns] When this Digimon would leave the battle area, by returning 4 [Vemmon] from its digivolution cards to the bottom of the deck, it doesn't leave.")
- **Effect text:** as above.
- **What's missing:** `ReplacementCostBody` offers only `delay_self`, `trash_own_link_card`, `place_link_card_as_bottom_digivolution`; `ReplacementChooseBody.from` supports only `Hand`. No cost variant returns N name-filtered own sources to the deck bottom with pay-in-full atomicity and not-offered-when-unpayable gating. `return_selected_sources_to_deck { position: bottom }` performs the move but free-form `process` steps don't integrate with leave-prevention.
- **Suggested API shape:** `ReplacementCostBody::return_own_sources_to_deck: { filter, count, position: bottom }` reusing `return_selected_sources_to_deck`, gated on >=count matching sources, exposing picks to the action space, owning `outcome: prevent`. Must fire the new source-return observer trigger above.
- **Workaround:** None faithful. BLOCKED.
- **Related:** resolved leave-field replacement framework + link-card cost variants (BT25-066/073/101, EX11-027); the observer-trigger entry above.

### Mass-delete/for-each cannot exclude a chosen binding ("choose 1, delete all their OTHER Digimon")
- **Status: RESOLVED as NOT-A-GAP (2026-07-03).** Already expressible: `select ... selector: highest_play_cost, bind_as: kept` + `for_each {over: {..., not_in_binding: kept}}`; the tie case is faithful (only the chosen permanent is excluded — pinned by `not_in_binding_excludes_only_the_chosen_binding_from_a_field_scan`). EX11-046 unblocked.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** EX11-046 Galacticmon ("Choose 1 of your opponent's highest play cost Digimon and delete all of their other Digimon.")
- **Effect text:** as above.
- **What's missing:** No predicate leaf compares a candidate permanent against a *bound* permanent (only `color_matches_binding` exists), so `for_each over {opponent digimon}` cannot express "not the selected one". A cost/DP filter mis-handles ties for highest cost (the other tied Digimon must still be deleted; DCGO filters `perm != selected`).
- **Suggested API shape:** predicate leaf `is_not_binding: <name>` (or `over.exclude: <binding>` on `for_each`) resolving the bound permanent's stable handle.
- **Workaround:** None — BLOCKED (ties make cost-filtering unfaithful; auto-picking violates §17).
- **Related:** `FieldSelector::HighestPlayCost` (present); `color_matches_binding` (only existing binding-comparison leaf).

### Name/text-filtered digivolution-source count as a threshold predicate (condition gate)
- **Status: RESOLVED (2026-07-03).** `self_source_count: {filter, op: gte|lte|eq, value}` no-subject leaf over the carrier's sources; BT21-006-shaped inline fixture proves +3000 at 4 [Vemmon] sources / none at 3 (tests/dsl/self_source_count_threshold.rs).
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT21-006 Tsumemon ("[All Turns] This Digimon with 4 or more [Vemmon] digivolution cards gets +3000 DP.")
- **Effect text:** as above.
- **What's missing:** No `PredicateSpec` leaf counts *filtered* sources under the carrier vs a threshold: `stack_size_gte`/`materials_count_gte` count the whole stack; `self_digivolution_sources_contain_name` is boolean; `source_stack_count` exists only as a DP-amount formula/`per`-selector.
- **Suggested API shape:** predicate leaf `self_source_count: { filter, op: gte, value: N }` (or comparator wrapper over the existing filtered `source_stack_count` formula, reusing `formula_eval::source_stack_count_filtered`).
- **Workaround:** None — auto-applying +3000 drops the gate; unfiltered `stack_size_gte` counts wrong cards. BLOCKED.
- **Related:** G-DSL-PER-SOURCE-STACK-COUNT-FILTERED (resolved formula form — extend to a condition comparator).

### Deck-construction per-card copy limit that RAISES the cap above the format default ("up to N copies")
- **Status: RESOLVED (2026-07-03, small-batch round 2).** card_legality_for_descriptor: explicit format restriction dominates downward, else max(format_default, intrinsic); singleton still clamps to 1. BT11-061 max_count_in_deck: 50 populated in cards.json + card_overrides.json. Desktop Tauri path confirmed engine-owned (thin wrapper). Tests: deck_tools 49/0 incl. 7 new.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT11-061 Vemmon ("You can include up to 50 copies of cards with this card's card number in your deck."); generalizes to the "any number of copies" family.
- **Effect text:** as above.
- **What's missing:** `deck_tools.rs::card_legality_for_descriptor` (~:552) computes `max_copies = base.min(intrinsic)` — an intrinsic `max_count_in_deck` can only lower the cap, never raise it above the format default (4). Secondarily, no `max_count_in_deck` is populated for BT11-061 in cards.json/card_overrides.json.
- **Suggested API shape:** treat card-printed allowances as overrides: `max_copies = restriction_limit.map_or(intrinsic.max(default), |r| r.min(intrinsic))` (bans/restrictions still dominate downward), or an explicit `deck_copy_override` field; then populate `max_count_in_deck: 50` for BT11-061 via `card_overrides.json`.
- **Workaround:** None — decks with more than 4 Vemmon are illegal to build; the card's deck-construction identity is unenforceable. BLOCKED.
- **Related:** `deck_tools.rs` format descriptor / `restriction.card_limits` (downward-only path works).

### Whole-card "[X] in its text" predicate (DCGO `HasText` scope: name + traits + all requirement text)
- **Status: RESOLVED (2026-07-03).** New leaf `in_text_contains` (+`event_card_in_text_contains`): case-insensitive over name + also_treated_as/DigiXros aliases + traits (incl. Rule-granted) + all printed text incl. dual faces (attribute line and numeric cost structs documented as not scanned; requirement wording lives in printed text). Trait-only regression (BT6-017/BT6-065/ST14-09) pinned. `effect_text_contains` intentionally NOT widened. Tests: tests/dsl/in_text_contains.rs.
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Three Musketeers BeelStarmon (2026-07-02); Galacticmon (2026-07-02)
- **Card(s):** BT25-005, EX7-051, EX7-008, EX7-048 (all "[Three Musketeers] in its text" filters); BT21-058, BT21-056, BT18-060, BT11-061, EX11-046, BT21-062, BT21-098, BT11-105 (all "[Vemmon] in its text" filters); generalizes to every DCGO `HasText` card.
- **Effect text:** e.g. "Add 1 card with [Three Musketeers] in its text among them to the hand"; "By placing 4 cards with [Vemmon] in their texts from your trash...".
- **What's missing:** `effect_text_contains` (dsl_cards/predicate.rs) scans only effect/inherited/security text. DCGO `HasText` (CardSource.cs:2120) + official Q&A additionally scan the card NAME, `also_treated_as`, trait line, dual/option text, and DNA/DigiXros/burst/App-Fusion/Link/Assembly requirement strings. Concrete misses: BT6-017 MagnaKidmon, BT6-065 Gundramon, ST14-09 BeelStarmon carry the trait but not the literal string in effect text.
- **Suggested API shape:** a distinct `in_text_contains` (whole-card `HasText`) predicate leaf unioning all those fields — do NOT widen `effect_text_contains` in place (some cards intend the narrow scope).
- **Workaround:** `any_of: [effect_text_contains: X, trait_has: X, name_contains: X]` recovers the known concrete misses for these archetypes (imperfect vs requirement-text-only matches). Ship-with-fidelity-note.
- **Related:** `effect_text_contains` (self-documented as `HasText` but narrower); the `event_card` "in its text" sibling in `qa/dsl-vocab-gaps.md`.

### `select_own_sources` candidate set excludes link cards (`linked_cards`)
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Three Musketeers BeelStarmon (2026-07-02)
- **Card(s):** BT25-085 BeelStarmon ("By trashing 1 Option card from any of your Digimon's digivolution cards **or link cards**, this Digimon unsuspends.")
- **Effect text:** as above.
- **What's missing:** `has_own_source_candidates` / the `SelectOwnSources` candidate builder scan only `perm.card_sources` (selections.rs:4034); DCGO's `DigivolutionOrLinkCards` unions `linked_cards`. The engine already models `Permanent.linked_cards` + `trash_specific_link_card` but the selection verb doesn't offer them.
- **Suggested API shape:** `select_own_sources: { include_link_cards: true, ... }` unioning `linked_cards` into candidates, routing picked link cards through `trash_specific_link_card`.
- **Workaround:** Digivolution-sources-only — silently narrows legal cost payments; not faithful.
- **Related:** `[Link]` subsystem entries (`link_cards` DSL step, `trash_own_link_card`).

### Conditional trash-origin enablement for DigiXros (board-state-gated material zone)
- **Status: RESOLVED (2026-07-03).** Alt-path key `extra_material_zones: [{zone, while: <PredicateSpec>}]`, evaluated once at `build_digixros_transaction_for_hand_card` (DCGO AddMaxTrashCountDigiXrosClass + CanUseCondition) -> allow_zone. BT18-065 gate expressible. Provers: tests/digixros_conditional_trash_zone.rs (2).
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT18-065 Snatchmon ("While you have no Digimon other than [Vemmon], cards in your trash can also be placed for this card's DigiXros.")
- **Effect text:** as above.
- **What's missing:** `DigiXrosMaterialZone::Trash` exists (digixros.rs:32,959) but no DSL surface gates the trash origin on a live board-state predicate at recipe-validation time (DCGO `AddMaxTrashCountDigiXrosClass` + `CanUseCondition`).
- **Suggested API shape:** DigiXros recipe key `extra_material_zones: [{zone: trash, while: <PredicateSpec>}]` consulted by `validate_material_origin`.
- **Workaround:** Unconditional trash origin over-permits; omitting drops the ability. Neither faithful.
- **Related:** Xros Heart DigiXros substrate (resolved 2026-05-24).

### `[End of Your Turn]` conditional cost-paying digivolve into a text-filtered hand card (source-count-gated)
- **Severity:** 🟡 PARTIAL (verify — the cost-paying effect-digivolve path appears to exist per BT11-105's `effect_initiated_digivolve { cost: printed }`; residual is the composition + the in-text filter scope above)
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT18-065 Snatchmon ("[End of Your Turn] If this Digimon has 4 or more digivolution cards, it may digivolve into a Digimon card with [Vemmon] in its text in the hand.")
- **Effect text:** as above.
- **What's missing:** Confirm `effect_initiated_digivolve` supports from-hand with `cost: printed` at an `end_of_your_turn` trigger gated on the unfiltered source count (>=4 digivolution cards), with the hand-candidate filter using the whole-card in-text predicate.
- **Suggested API shape:** composition of existing verbs; only the in-text predicate is net-new (see whole-card `HasText` entry).
- **Workaround:** `effect_text_contains`-filtered variant under-matches (see `HasText` entry).
- **Related:** DCGO `DigivolveIntoHandOrTrashCard(payCost: true)`; whole-card in-text predicate entry.

### Place ALL revealed matching cards under a target chosen from the triggering (played/digivolved) permanent set, with order selection
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** EX11-066 Xeno ("[All Turns] When your Digimon are played or digivolve, if any of them have [Vemmon] in their texts, by suspending this Tamer, reveal the top 2 cards of your deck. Place all [Vemmon] among them as any of those Digimon's bottom digivolution cards. Trash the rest.")
- **Effect text:** as above.
- **What's missing:** (1) an event-derived target *set* binding (the just-played/digivolved permanents as a selectable pool; `event_target` binds one), (2) a batch "place all filtered revealed cards" form (`choose_from_reveal` is single-pick), (3) intra-stack order selection when 2+ cards go under one target (DCGO `SelectCardEffect` ordering).
- **Suggested API shape:** `place_all_from_reveal_under_target: { of, filter, targets: <event-set binding>, order: player_choice, remainder: trash }` + an `event_permanent_set` binding for batch observers.
- **Workaround:** single-permanent-event approximation via `event_target` auto-picks the destination and drops ordering — violates section 17 for multi-card/multi-target cases.
- **Related:** resolved Rocks reveal verbs (`choose_from_reveal`/`order_remainder` — single-card precedent); resolved OnAllyPlayed observer context (single-permanent binding).

### `trash_top_security` cannot target a remaining count ("so it has N cards left") — no `Subtract` formula
- **Status: RESOLVED (2026-07-03).** BOTH shipped: `subtract:` compound formula (left-associative) AND `leave: N` on trash_top_security (max(0, count-leave) from top, mask-accurate, mutually exclusive with count). BT21-098 unblocked.
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT21-098 Ragnarok Cannon ("trash the top cards of your opponent's security stack so that it has 1 card left")
- **Effect text:** as above.
- **What's missing:** `TrashTopSecurityArgs.count` takes a `FormulaSpec` but the compound vocabulary (`formula.rs`) has only `FloorDiv`/`Max`/`Min` — no `Subtract`, so "security_count - 1" is unexpressible; no `leave: N` param exists either.
- **Suggested API shape:** add `Subtract` compound formula or a `leave: N` field on `trash_top_security`.
- **Workaround:** none currently derivable for "down to 1".
- **Related:** "Zone-manipulation: top-N security trash" (count form closed; arithmetic residual new).

### Deletion-result binding for "if this effect didn't delete" branches
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Galacticmon (2026-07-02)
- **Card(s):** BT21-098 Ragnarok Cannon ("Delete 1 of your opponent's Digimon with the lowest play cost. If this effect didn't delete, trash the top cards of your opponent's security stack...")
- **Effect text:** as above.
- **What's missing:** No step binds whether a `delete_permanent` actually removed its target, so a following `if` can't branch on "didn't delete". `if: {none: opponent digimon}` misses the survived-immune-target case.
- **Suggested API shape:** `delete_permanent: { target, bind_deleted_as: <name> }` + `if: { binding_absent: <name> }`.
- **Workaround:** opponent-has-0-Digimon gate — imperfect (over-suppresses when a target survives deletion).
- **Related:** G-PER-SELECTED-DELETE-INDEX-SHIFT (adjacent deletion mechanics).

## Store-champs June-2026 audit — Millenniummon + ShineGreymon (2026-07-02)

Second wave of the store-champs audit (`qa/archetype-qa/store-champs-june-2026-scoping.md`).
Fix plans: `.claude/plans/rust-engine-gaps-millenniummon.md`, `.claude/plans/rust-engine-gaps-shinegreymon.md`.
Existing entries reconfirmed with NEW drivers (append, no re-file):
- **`G-ENGINE-ON-DISCARD-HAND`** — add **ST16-14 Matt Ishida** ("[All Turns] When one of your effects trashes a card in your hand, by suspending this Tamer, gain 1 memory"; DCGO `OnDiscardHand` + owner-of-trashing-effect filter). Reinforces the "caused by YOUR OWN effect" source-filter facet. Discovered in: Millenniummon (2026-07-02).
- **BeforePayCost interactive-cost family** (`G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT`, BT13-103 / qa/dsl-vocab-gaps.md G-DSL-BEFORE-PAY-COST-DELETE-OWN-FOR-VARIABLE-REDUCTION) — see the new fixed-amount entry below (BT18-073, BT13-083).
- **G-DSL-PLACE-AS-TOP-SOURCE** (qa/dsl-vocab-gaps.md) — add EX9-074 Kimeramon (place trash card as TOP source; behaviorally inert for its colour-count → ship PARTIAL with note). ✅ RESOLVED 2026-07-05: `place_as_top_source` DSL verb + `Permanent::push_as_top_source` engine primitive landed (see qa/resolved-gaps.md); EX9-074 and BT13-088 flipped to it — EX9-074 now IMPLEMENTED.
- **G-ENGINE-DNA-PRINTED-COST / G-ENGINE-DNA-RECIPE-ENFORCEMENT** — both bite BT18-015/BT18-073's On-Deletion DNA (DCGO `payCost:true` + jogress validation); P-220's DNA path is printed-cost-0 (latent, harmless).
- **STALE ENTRY:** the `<Training>` keyword entry (§"`<Training>` keyword", ~line 534, 🔴) is stale — Training is fully wired (Phase F: `Keyword::Training`, action 1142, breeding-area [Main] mask emitter, face-down source; EX9-060 Devidramon confirms). Move to `qa/resolved-gaps.md` on the next hygiene sweep.
- **Data fix:** BT18-073 / BT19-065 print `(Rule) Trait: Has [Composite]` — reconcile into `card_overrides.json` (API-dropped rule trait).

### Effect-initiated DNA digivolve with a TRASH material (field + trash pair, result card from hand)  [G-ENGINE-DNA-TRASH-MATERIAL]
- **Status: RESOLVED (2026-07-03, DNA workstream).** `effect_initiated_dna_digivolve_trash_partner` -> `Game::dna_digivolve_trash_partner_inner`: trash material moves STRAIGHT into the merged stack (no independent play, no OnPlay — faithful to DCGO CreateNewPermanent); atomic + clone-safe. Composes with printed-cost + recipe enforcement. BT18-015/BT18-073 shapes proven via fixtures.
- **DSL verb (2026-07-03):** `effect_initiated_dna_digivolve_trash_partner: { target, trash_partner, from_hand, cost, ignore_requirements }` now lowers to this primitive (G-DSL-DNA-TRASH-PARTNER, qa/dsl-vocab-gaps.md — ✅ RESOLVED); BT18-015 authored + green (`cards/bt18/BT18-015.yaml`), BT18-073's [On Deletion] clause is the remaining consumer.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** BT18-015 Kimeramon ("[On Deletion] 1 of your [Machinedramon] and 1 [Kimeramon] in the trash may DNA digivolve into [Millenniummon] in the hand"), BT18-073 Machinedramon ("[On Deletion] 1 of your [Kimeramon] in play and 1 [Machinedramon] in the trash may DNA digivolve into [Millenniummon] in the hand")
- **Effect text:** as above — material_a on the field, material_b in the TRASH, result card from the hand.
- **What's missing:** Both effect-initiated DNA verbs assume field/hand materials: `EffectDnaDigivolveArgs {target_a, target_b, from_hand}` (two field permanents) and `EffectDnaDigivolveHandPartnerArgs {target, hand_partner, from_hand}` (field + hand). Neither sources a DNA material from the trash. DCGO (`BT18_015.cs`/`BT18_073.cs`) plays the trash material out as a permanent first (`CreateNewPermanent`), then DNA-merges via `PlayCardClass.SetJogress(payCost:true)`.
- **Suggested API shape:** a DNA-verb variant accepting a `CardSourceRef::Trash` material (materialize-then-merge, mirroring DCGO), e.g. `effect_initiated_dna_digivolve_trash_partner {target, trash_partner, from_hand, cost}`. Must compose with the two open DNA gaps (printed cost payment, recipe enforcement).
- **Workaround:** None faithful. BLOCKED (only the On-Deletion DNA clauses; the rest of both cards is expressible).
- **Related:** G-ENGINE-DNA-PRINTED-COST, G-ENGINE-DNA-RECIPE-ENFORCEMENT (both apply — DCGO passes payCost:true); resolved G-DSL-DNA-FROM-HAND-PARTNER (field+hand sibling).

### BeforePayCost play-cost reduction paid by an interactive delete-own-permanent cost (fixed amount)  [G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST]
- **Status: RESOLVED (2026-07-03).** See G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT resolution (fixed + variable both shipped). BT18-073 clause 3 + BT13-083 clause 1 unblocked.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** BT18-073 Machinedramon ("When this card would be played, by deleting 1 of your Digimon with the [Composite] trait, reduce the play cost by 4"), BT13-083 Gizmon: AT ("When you would play this card, by deleting 1 of your level 3 Digimon, reduce the play cost by 4")
- **Effect text:** as above.
- **What's missing:** `cost_reduction` `pay_cost_fn` is gated to `RunOutcome::Synchronous` (`lower_cost_reduction.rs:195`); an interactive `select_own_permanent` + `delete_permanent` inside pay_cost parks on a selection → the cost reads as failed → the reduction is dropped. DCGO BeforePayCost runs a `SelectPermanentEffect` (optional), deletes, then installs `ChangeCostClass(-4)`. Because the reduction amount is a constant here, this is a strictly simpler sibling of `G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT` (BT13-103, whose amount additionally reads the deleted permanent's cost) — fixing the park-and-resume half of pay_cost unblocks BOTH these cards immediately.
- **Suggested API shape:** allow a `cost_reduction` pay_cost body to surface a `pending_selection` and resume (the resumable-VM path per rule 28's clone-safety constraint), with a literal `amount` decoupled from the paid permanent.
- **Workaround:** None faithful — auto-selecting the sacrifice violates §17. BLOCKED.
- **Related:** G-DSL-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT (BT13-103 superset); G-COST-REDUCTION-INTERACTIVE-PAYCOST-AMOUNT (GAPS.md:1527).

### Predicate: candidate color ∈ carrier's digivolution-source color set  [G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK]
- **Status: RESOLVED (2026-07-03).** `color_matches_own_source_stack: {of: self}` — non-flipped source color set (shared extraction non_flipped_source_colors, also feeding the branch gate). Tests: tests/dsl/kimeramon_color_mass_delete.rs.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** EX9-074 Kimeramon (≤5-colour branch: "delete 1 of your opponent's Digimon with the same color as any of this Digimon's digivolution cards")
- **Effect text:** as above.
- **What's missing:** No `PredicateSpec` leaf tests "candidate top-card color ∈ {distinct colors of the carrier's non-flipped digivolution-source cards}". `color_matches_any_field_digimon` reads a field Digimon's top-card color; `color_matches_binding` a bound permanent. DCGO: `carrier.DigivolutionCards.Filter(!IsFlipped).SelectMany(CardColors).Distinct()` ∩ candidate colors ≠ ∅.
- **Suggested API shape:** `color_matches_own_source_stack: {of: self}`; share the color-set extraction helper with the existing `DistinctColorsCount` formula.
- **Workaround:** None faithful.
- **Related:** `color_matches_returned_card` (analogous result-log leaf); `DistinctColorsCount` formula (same extraction).

### Mass per-color deletion: delete 1 opponent Digimon of each distinct color present  [G-DSL-DELETE-ONE-PER-DISTINCT-OPPONENT-COLOR]
- **Status: RESOLVED (2026-07-03).** Step `delete_one_per_opponent_color` on a new ResumeFrame::PerColorDeleteStep (per-color mandatory picks, already-picked excluded, batch delete via delete_permanents_batch, clone-safe test). EX9-074 branch B expressible; branch switch via the distinct-color gate.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** EX9-074 Kimeramon (6+-colour branch: "instead delete 1 of each of your opponent's Digimon with different colors"; official Q&A: one mandatory pick per color present, no repeats, batch delete)
- **Effect text:** as above.
- **What's missing:** No `for_each` over a color enumeration with per-iteration exclusion of already-chosen permanents. DCGO loops the 7 canonical colors, prompting a mandatory pick per color present (`CanTargetCondition_ByPreSelecetedList`), then batch-deletes.
- **Suggested API shape:** `for_each {over: opponent_present_colors, exclude_already_chosen: true, body: [select 1 of that color, accumulate]}` + batch delete; every pick exposed to the action space; colors with no legal target skipped.
- **Workaround:** None faithful.
- **Related:** "Mass-delete/for-each cannot exclude a chosen binding" (EX11-046 — same per-iteration-exclusion family); G-DSL-COLOR-MATCHES-OWN-SOURCE-STACK (sibling branch, same card).

### Result-count binding for "cards trashed by this effect" (floor-div riders)  [G-DSL-TRASH-COUNT-RESULT-BINDING]
- **Status: RESOLVED (2026-07-03, small-batch round 2).** `bind_count_as` on trash_opponent_hand_to_count publishes the trashed count as a Literal binding (resume terminal + synchronous path), consumed via binding_value/floor_div. BT19-075's first printed sentence authored in its YAML (8/1 tests; residual ignore = its Composite leave-replacement clause, card-pass item). Provers: tests/cards_behavioral/trash_count_binding_primitive.rs (4).
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** BT19-075 MoonMillenniummon ("Your opponent trashes cards in their hand until they have 5 left. For every 2 cards trashed by this effect, delete 1 of your opponent's Tamers.") — already flagged inline in `cards/bt19/BT19-075.yaml`'s TODO.
- **Effect text:** as above.
- **What's missing:** `TrashOpponentHandToCountArgs` binds no trashed-count; `FormulaSpec` has no "count of a prior step's result" source. `bind_count_as` exists only on `PlaceSelectedSourcesUnderTamer`/`MoveMatchingSourcesUnderTamer`. So `floor_div(trashed, 2)`-capped Tamer deletion is unexpressible. (Contrast BT18-019's memory-per-returned rider, sidesteppable via `for_each` + `gain_memory: 1` over an iterable card-list binding — the hand-trash step produces no such binding.)
- **Suggested API shape:** add `bind_count_as: Option<String>` to `TrashOpponentHandToCountArgs` (and hand/trash select steps generally); consume via `max: {floor_div: [{binding_value: trashed}, 2]}` on a capped opponent-Tamer multi-select (both formula pieces already exist).
- **Workaround:** None faithful — the whole first sentence stays load-only, as the YAML already does.
- **Related:** DCGO `BT19_075.cs` `FloorToInt(trashed.Count/2)`; `binding_value`/`floor_div` formulas (present); `bind_count_as` precedent on the under-Tamer steps.

### `event_target` predicate cannot read the deletion-subject's digivolution-source count  [G-DSL-EVENT-TARGET-SOURCE-COUNT]
- **Status: RESOLVED (2026-07-03).** `event_target_has_digivolution_cards` + `event_target_stack_size_gte: N` read the rule-25 pre-removal snapshot (or live card_sources for non-deletion targets). EX1-066's gate now faithful.
- **Severity:** 🟡 PARTIAL
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** EX1-066 Analog Youth ("[All Turns] When one of your level 5 or higher Digimon **with a digivolution card** is deleted, by suspending this Tamer, gain 1 memory…")
- **Effect text:** as above.
- **What's missing:** The `event_target_*` predicate family exposes owner/kind/trait/name/level/dp but no source-count leaf. DCGO gates with `!permanent.HasNoDigivolutionCards` on the deletion subject (pre-removal). Rule 25's deletion snapshot has the data (`deleted_self_source_count`) but only self-scoped, not surfaced to cross-permanent watchers.
- **Suggested API shape:** `event_target_stack_size_gte: N` / `event_target_has_digivolution_cards` reading the pre-removal snapshot.
- **Workaround:** gate on `event_target_level_gte: 5` only — over-permits a bare (source-less) L5+; ship-with-fidelity-note.
- **Related:** rule 25 deletion snapshot; `event_target_is_source`.

### Candidate play-cost filter relative to a bound/event permanent's cost (+offset)  [G-DSL-COST-RELATIVE-TO-EVENT-SUBJECT]
- **Status: RESOLVED (2026-07-03).** `play_cost_eq_binding: {binding|event_target, offset, op}` (deletion snapshot cost_just_before for event targets); plus `level_lte_binding`/`level_gte_binding` siblings (BT8-107 verify-note closed). BT19-099 unblocked.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Millenniummon (2026-07-02)
- **Card(s):** BT19-099 The Wicked God Descends! ("…you may play 1 [Wicked God] trait Digimon card **with a play cost 1 higher than that Digimon** from your hand or trash without paying the cost" — "that Digimon" = the leaving [Millenniummon]-name Digimon)
- **Effect text:** as above.
- **What's missing:** No predicate leaf compares a candidate card's play cost to a bound/event permanent's cost with an offset. Existing binding comparators: `level_eq_binding`, `color_matches_binding` only. DCGO: `leaving.TopCard.GetCostItself + 1 == candidate.GetCostItself` over the `WhenRemoveField` subject. The leave-observer + self-delete Delay cost + union-zone free play are all covered by the BT17-095 substrate; only this comparator blocks.
- **Suggested API shape:** `play_cost_eq_binding: {binding: replacement_subject|event_target, offset: 1, op: eq|lte|gte}` mirroring `level_eq_binding` resolution.
- **Workaround:** None faithful — only the [All Turns] Delay rider is blocked; [Main] + Security are green.
- **Related:** `level_eq_binding` (extend); BT17-095 precedent; the binding-comparison family (EX11-046, BT8-107's level_lte_binding verify-note).

### Distinct-by-name count predicate over a filtered field set  [G-DSL-DISTINCT-NAMED-PERMANENT-COUNT]
- **Status: RESOLVED (2026-07-03).** `distinct_named_count_gte: {of, filter, n}` — distinct synth-identity-aware names among filtered battle-area permanents. BT21-040 unblocked.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** ShineGreymon (2026-07-02)
- **Card(s):** BT21-040 Agumon ("[Your Turn] While your opponent has a level 6 or higher Digimon or you have 3 or more [Hero] trait Tamers **with different names**, this Digimon may digivolve into [ShineGreymon] in the hand for a cost of 4, ignoring digivolution requirements.")
- **Effect text:** as above. NOTE: DCGO under-implements (counts raw Hero-trait Tamers, ignoring "different names") — the PRINTED text is the faithful target per source priority.
- **What's missing:** No boolean predicate counts DISTINCT card-names among a player's battle-area permanents matching a sub-filter. `count_gte`/`CountAggregate` counts raw matches (over-counts duplicates); `distinct_tamer_colors_gte` counts colors, not names, and is filter-less; `DistinctByMode::Name` is a *selection* uniqueness mode, not a board predicate.
- **Suggested API shape:** `distinct_named_count_gte: {of, filter, n}` (+ `_lte`), deduping by synth-identity-aware effective name; model the collector on `distinct_tamer_colors_gte` + inner filter.
- **Workaround:** authoring only the first disjunct (opp Lv6+) under-approximates a legal digivolve path — do NOT ship partial; BLOCKED until the leaf lands. Recurs across Hero/DATA SQUAD tribal gates.
- **Related:** `distinct_tamer_colors_gte` (closest sibling); `select_materials {uniqueness: name}` (selection-side precedent).

## Store-champs June-2026 audit — TS support + cross-deck staples (2026-07-02, final wave)

Third and final wave (`qa/archetype-qa/store-champs-june-2026-scoping.md`). Fix plan:
`.claude/plans/rust-engine-gaps-ts-support-staples.md`.

**Existing entries reconfirmed as drivers (cite, no re-file):**
- BT25-020 Marsmon → `G-DSL-BATTLE-WINNER-BOARDWIDE` (qa/dsl-vocab-gaps.md §3164, open)
- BT25-073 Dragomon → `G-DSL-LINK-TRASH-AS-COST` (qa/dsl-vocab-gaps.md §2897, open — activation-cost variant; replacement-cost variant landed)
- BT25-075 Vulcanusmon → `G-DSL-FORMULA-OWN-LINK-CARD-COUNT` (qa/dsl-vocab-gaps.md §3339, open; subsumes `G-DSL-LINK-N-CARDS-PER-HOST` §3328)
- BT25-102 Factorial Area → `G-ENGINE-SECURITY-ZONE-SOURCED-FIELD-AURA` (qa/archetype-qa/engine-gaps.md §909, open)
- BT25-039 Sirenmon → `G-DSL-PROTECT-OTHER-BY-SELF-DELETE` (§3185) + `G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER` (§3209), both re-verified against DCGO
- BT24-092 Shock Plasma → **append to `G-LINK-INHERITED-ESS` Card(s)** (link ESS `[When Attacking][OPT] -6000`; self-link via `link_cards source: self_option` is expressible)

**Tracker hygiene from this wave (apply on next sweep):**
- "Digivolution-stack source extraction (`pop_top_digivolution_source`)" (~line 790) is STALE → RESOLVED by `security_place_top_stacked_card` with a bound `carrier` (BT25-038 exemplar); unblocks BT24-093 Temple of Beginnings.
- Remove BT14-033 from the `play_from_security_at(index)` card list — its security operation is a *digivolve* (closed primitive), not a play; only BT13-012 genuinely plays.
- `<Training>` entry (~line 534) STALE → resolved (Phase F; EX9-060 confirms).
- BT15-037's spec comment claiming `G-DSL-ON-DISCARD-SECURITY-TRIGGER` blocked is stale — `on_discard_security` ships end-to-end.
- Data reconciliation: `(Rule) Trait: Has [Dragonkin] Type` for BT24-014 + P-213; `(Rule) Trait: Has [Composite]` for BT18-073 + BT19-065 → `card_overrides.json`.
- Verify-at-implementation: Scapegoat-via-aura `grant_keyword` (BT25-097) — confirm aura grant installs the WhenWouldBeDeleted replacement; P-213's effect-driven optional attack step; BT8-107's `level_lte_binding` widening.

### Grant a triggered effect whose body declares an attack (selection-driving granted body)  [G-ENGINE-GRANTED-EFFECT-SELECTION-BODY]
- **Status: RESOLVED as ALREADY-CLOSED (2026-07-03).** The v1 limitation note was stale — QueuedEffect::granted_effect_id already composes granted-body selections. BT23-032's shape proven: granted force_attack parks a mandatory attack at the opponent's start-of-main via the BeginAttack resume frame (clone-safe test), expiry drains the grant. Stale comment in grant_triggered.rs replaced. Tests: tests/dsl/granted_effect_selection_body.rs (4).
- **Severity:** 🔴 BLOCKING
- **Discovered in:** TS Angels (2026-07-02)
- **Card(s):** BT23-032 Shakkoumon ("[When Digivolving] Until your opponent's turn ends, give 1 of their Digimon '[Start of Your Main Phase] This Digimon attacks.'")
- **Effect text:** as above.
- **What's missing:** `grant_triggered_effect` (dsl_cards/step/grant_triggered.rs) has an explicit v1 limitation: granted bodies that install a `PendingSelection` don't compose with the firing sequence ("requires extending `QueuedEffect` with a granted-effect discriminator"). This grant's body must run `force_attack` on the carrier — a `SelectAttack` pending selection — on the OPPONENT's turn (carrier = opponent Digimon), with `on_start_main_phase` as a valid grant timing. DCGO: `SetEffectSourcePermanent(selected)` + `UntilOwnerTurnEndEffects` + `SelectAttackEffect(SetCanNotSelectNotAttack)`.
- **Suggested API shape:** extend `QueuedEffect` with a granted-effect discriminator so granted bodies may park selections; DSL `grant_triggered_effect { timing: on_start_main_phase, expiry: end_of_opponents_turn, body: [force_attack: {attacker: source}] }` honoring the carrier's controller for timing scope.
- **Workaround:** None faithful — a must-attack flag hides the target choice from the RL action space (§17). BLOCKED.
- **Related:** grant_triggered.rs v1 limitation note; `force_attack` step; G-ENGINE-GRANTED-ONBLOCK-CARRIER-GATE (adjacent granted-effect scoping issue).

### Combined both-players security-count predicate  [G-DSL-TOTAL-SECURITY-COUNT-PREDICATE]
- **Status: RESOLVED (2026-07-03).** `total_security_count_lte/gte/eq` sums both players (both-players reading pinned by test). BT13-106 unblocked.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** cross-deck staples (2026-07-02)
- **Card(s):** BT13-106 Odin's Breath ("if there're 6 or fewer total cards in both players' security stacks, all of your opponent's Digimon gain <Security Attack -1>…")
- **Effect text:** as above.
- **What's missing:** Only per-player `security_count_{lte,gte,eq}` / `opponent_security_count_{lte,gte}` exist. A SUM bound is not decomposable into per-player bounds (4+3=7 counterexample). DCGO: `owner.SecurityCards.Count + enemy.SecurityCards.Count <= 6`.
- **Suggested API shape:** `total_security_count_lte/gte/eq` predicate leaves summing both sides.
- **Workaround:** None — per-player conjunction is logically unfaithful. BLOCKED. (Everything else on the card ships: `on_discard_security` timing exists end-to-end; mass `SecurityAttackChange continuous: true` covers "all".)
- **Related:** per-player security predicates; resolved `OnDiscardSecurity` timing.

### Memory-count as a scalar formula leaf  [G-DSL-MEMORY-COUNT-FORMULA]
- **Status: RESOLVED (2026-07-03).** `PerSelector::PlayerMemory{of}` - `per: {player_memory: {of: opponent}}` (opponent clamped >=0 per DCGO Math.Max). BT25-086's clause now expressible; card to author.
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Time Strangers support (2026-07-02)
- **Card(s):** BT25-086 Dan Yuki ("[End of Your Turn] By suspending this Tamer, 1 of your [TS] trait Digimon gets +1000 DP for the turn **for each memory your opponent has**. After, that Digimon may attack.")
- **Effect text:** as above.
- **What's missing:** No `FormulaSpec` leaf reads the memory gauge as an integer — memory exists only as predicate thresholds (`memory_lte`/`own_memory_gte`) and the boolean `use_cost_lte_opponent_memory`. DCGO: `Math.Max(0, enemy.MemoryForPlayer) * 1000` via `ChangeDigimonDP(UntilEachTurnEnd)`.
- **Suggested API shape:** formula leaves `own_memory` / `opponent_memory` (opponent side clamped at 0), consumable by `dp_modifier_fn`/`base_per_delta`.
- **Workaround:** None faithful (live scalar). Only the DP amount is blocked; the suspend-cost + select + `may_attack_now` scaffolding is green.
- **Related:** `own_memory_lte` predicate perspective handling; "Dynamic DP scaling modifier" (delivery vehicle).

### Predicate on a bound card's printed color  [G-DSL-BINDING-CARD-COLOR]
- **Status: RESOLVED (2026-07-03).** `binding_card_color: {binding, color_is}` leaf (sibling of binding_card_kind); fails closed on unset binding. BT1-087 unblocked. Tests: tests/dsl/predicate_leaves_ii.rs.
- **Severity:** 🟡 PARTIAL
- **Discovered in:** cross-deck staples (2026-07-02)
- **Card(s):** BT1-087 T.K. Takaishi ("reveal 1 card in [security] and add it to your hand. **If that card is yellow**, <Recovery +1 (Deck)>. Then shuffle…")
- **Effect text:** as above.
- **What's missing:** `binding_card_kind` exists but no color analogue; `color_matches_binding` needs a subject card while `if` conditions run subject-less. So "Recover only if the added card is yellow" is unexpressible; the search itself is unfiltered (DCGO adds ANY card).
- **Suggested API shape:** `binding_card_color: {binding, color_is}` — sibling of `binding_card_kind` (predicate.rs:667).
- **Workaround:** color-filtering the security search is UNFAITHFUL (wrongly forbids adding non-yellow cards). No faithful workaround.
- **Related:** `binding_card_kind`; BT11-042 Angewomon idiom (unconditional Recovery sibling).

### Substring/root trait match on the event-target permanent  [G-DSL-EVENT-TARGET-TRAIT-CONTAINS]
- **Status: RESOLVED (2026-07-03).** `event_target_trait_contains: <token>` — case-insensitive substring over the event-target's traits (snapshot + live split); pluralization/compound tolerant per Q&A. BT11-089 unblocked.
- **Severity:** 🟡 PARTIAL
- **Discovered in:** cross-deck staples (2026-07-02)
- **Card(s):** BT11-089 Akiho Rindou ("When an effect plays any of your red Digimon with [Avian], [Bird], [Beast], [Animal] or [Sovereign] in **any of their traits**, other than [Sea Animal]…" — official Q&A: "regardless of other words or pluralizations")
- **Effect text:** as above.
- **What's missing:** Observer gates expose only exact `event_target_trait_has`; the substring sibling `trait_contains` (predicate.rs:72) has no event-target counterpart, so compound/pluralized traits ("Holy Beast", "Beasts") under-fire the trigger. DCGO `HasAvianBeastAnimalTraits`.
- **Suggested API shape:** `event_target_trait_contains: <str>` sharing the event-target resolver; compose with `not: {event_target_trait_has: "Sea Animal"}`.
- **Workaround:** exact-token `any_of` enumeration misses Q&A-included forms; ship-with-fidelity-note at most.
- **Related:** `trait_contains` (card-subject sibling); `event_target_trait_has`; multi-Digimon single-effect play nuance (DCGO grants all with one suspend — note at implementation).

## DigiXros material `distinct_by` (card number / name / level) is compiled but never enforced at runtime  [G-ENGINE-DIGIXROS-DISTINCT-BY]
**Status: RESOLVED (2026-07-03).** Enforced in `DigiXrosTransaction::resolve_material_origin` (single choke-point -> mask-level exclusion); `MaterialIdentity{card_number,name,level}` captured at commit (clone-safe data); slot-scoped, wildcards exempt; new error `DistinctnessViolation`. Provers: digixros.rs unit tests incl. `bt19_065_digixros_rejects_duplicate_card_number` + the behavioral prover in bt19_065.rs (un-ignored, green). Fixes shipped BT12-112/EX3-014 semantics.
Surfaced by: BT19-065 Machinedramon (batch-implement-cards-rust-dsl, 2026-07-02); PRE-EXISTING, shared by every card whose `alt_paths: kind: digixros` material sets `distinct_by` (BT12-112 Shoutmon X7: Superior Mode `distinct_by: card_number`, EX3-014 Dorbickmon `distinct_by: name`, and the review-added BT19-070 DigiXros -1 path).
`compiled_path_transaction` (`code/digimon-engine/src/digixros.rs:923-933`) correctly threads `material.distinct_by` into `DigiXrosRecipeSlot.distinct_by`, but that field is **never read anywhere else in the crate**. The runtime candidate filter (`slot_accepts_card`, `validate_material_origin` -> `resolve_material_origin`) checks only name/trait requirements and exact-handle dedup — so a recipe printed "5 cards w/different card numbers" currently accepts two copies of the identical card as two materials (no-approximations violation for decks running duplicates of a qualifying material).
No existing behavioral test caught this: EX3-014's and BT12-112's DigiXros tests supply already-distinct materials.
Fix: when `slot.distinct_by` is `Some(mode)`, reject candidates whose printed card number / name / level matches an already-committed material (mirror DCGO `CanTargetCondition_ByPreSelecetedList` deduping on `CardSource.CardID`); exclude conflicts in `pending_digixros_material_candidates` up front so the MASK stays faithful, not just commit-time validation.
First test: `code/digimon-engine/tests/cards_behavioral/bt19/bt19_065.rs::bt19_065_digixros_rejects_duplicate_card_number` (currently `#[ignore = "pending: G-ENGINE-DIGIXROS-DISTINCT-BY"]`).
Status: open.

### Shared [Once Per Turn] counter across face_up+inherited self cost-reducer (`scope: both` for CostReduction)  [G-ENGINE-SHARED-OPT-SCOPE-BOTH-REDUCER]
- **Status: RESOLVED (2026-07-03, small-batch round 2); BT18-060 mis-migration reverted (2026-07-04).** Finding: `scope: both` previously COLLAPSED to face-up-only for cost reducers (latent authoring bug, unreachable double-fire). Fixed properly: Both expands to [FaceUp, Inherited] sharing one OPT group (0x80|clause_index); cost path routes OPT accounting through Game::cost_reducer_opt_slot. BT18-060/BT11-061 author scope: inherited (DCGO SetIsInheritedEffect). Provers: tests/cards_behavioral/scope_both_shared_opt_reducer.rs (3).
- **Severity:** 🟢 RESOLVED (engine machinery valid + tested; no current card uses `scope: both` for CostReduction)
- **Discovered in:** store-champs follow-up review (2026-07-03)
- **Card(s):** BT18-060 Vemmon, BT11-061 Vemmon — both print an inherited `[Your Turn][Once Per Turn]` "reduce by 1 when this Digimon digivolves into a [Vemmon]-text card" whose printed OPT is ONE counter for the card, while the DSL `scope: both` expansion (face_up + inherited copies) gives each copy its OWN OPT counter — a latent double-reduction when the card is both a top card and, post-digivolve in the same turn, a source. DCGO shares one hash across both registrations.
- **What's missing:** an OPT counter keyed to the CARD (hash-shared across the expanded scope copies), not to each expanded clause instance.
- **Suggested API shape:** `once_per_turn_key: <shared>` or make the `scope: both` expansion register one shared OPT id for both lowered copies (mirroring DCGO SetHashString sharing).
- **Update (2026-07-04) — BT18-060 mis-migrated onto `scope: both`, reverted:** the round-2 work also re-authored BT18-060's reducer as `scope: both` on the belief that DCGO's `evoRootTops` were the NEW tops produced by the digivolution (making the reducer position-agnostic). Refuted by the DCGO source: `evoRootTops` are the PRE-digivolution tops (CardController.cs:1394-1397 captures `_targetPermanent.TopCard` BEFORE `AddCardSource`, which inserts the new top at index 0; jogress path 1487/1502), so `!evoRootTops.Contains(card)` means "was already buried" — and decisively, DCGO's cost pipeline (`CardSource.GetChangedPayingCost` → `Permanent.EffectList_ForCard`, Permanent.cs:1520-1541) EXCLUDES a top card's `SetIsInheritedEffect(true)` reducer entirely, matching general_rule.pdf §15-3-1 (an inherited effect is gained by a Digimon FROM a digivolution card). The face-up discount `scope: both` granted was an OVER-application; BT18-060 reverted to `scope: inherited` (pin: tests/cards_behavioral/bt18/bt18_060.rs::bt18_060_face_up_digivolve_pays_full_cost_over_application_pin; cross-position proof: `..._cross_position_face_up_pays_full_then_buried_reduces_once`). The `scope: both` + shared-OPT machinery itself remains valid and tested (synthetic provers unchanged) for future cards that genuinely print a reducer in both boxes.
- **Related:** GrantKeyword `scope: both` expansion (2026-07-02); DCGO shared-hash OPT semantics.


## Gap-closure round 3 — final resolutions (2026-07-03)

> **G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST — RESOLVED (round 3).** DSL cost verb
> `trash_option_from_own_stacks: {of, optional}`: two RL-visible selections (which own Digimon
> whose digivolution OR link cards hold >=1 Option, then which Option); per-zone trash
> (`trash_specific_source_card` fires OnDigivolutionCardTrashed; `trash_specific_link_card` fires
> OnLinkedCardTrashed); tail cost-gated; unpayable aborts. Clone-safe resume frames
> (FieldPermanent{SelectAndTrashStackOption} -> TrashOptionFromStackSelection). Combined with the
> round-1/2 use_option_from_sources, BT25-085 is fully UNBLOCKED. Tests:
> tests/dsl/trash_option_from_own_stacks.rs (7).

> **G-DSL-PROTECT-OTHER-BY-SELF-DELETE (BT25-039) + EX7-048 protect-others — RESOLVED (round 3).**
> The cross-permanent-subject would-leave path already existed; the two missing COST variants
> landed: `cost: {delete_self: true}` (dedicated DeleteSelfAndCancelLeave step, kept distinct from
> the Delay recognizer) and `cost: {trash_option_from_own_digivolution_cards: true}` (carrier-scoped
> Option pick via LinkCardLeaveMode::TrashDigivolutionOptionAndCancel), both with the
> `none_of: [replacement_cause: own_effect]` cause gate ("other than by your effects").
> BT25-039 clause 3 and EX7-048 clause 2 UNBLOCKED. Tests:
> tests/dsl/protect_others_leave_replacement.rs (11).

With round 3, every gap filed by the store-champs June-2026 assessment is RESOLVED, adjudicated
NOT-A-GAP, or explicitly deferred (BT15-102 Apocalymon's two pre-campaign architectural entries:
cast-time stack-construction + cross-card re-firing source-card variant).

## G-OPT-RESET-ONLY-TURN-PLAYER — [Once Per Turn] counters reset only at the carrier controller's turn start (OPEN 2026-07-03)

Surfaced by BT19-075 MoonMillenniummon clause 3 ("[All Turns] [Once Per Turn] When other Digimon or Tamers are deleted, trash your opponent's top security card") while authoring the OPT re-arm test.

- **What's broken:** `Permanent::effect_activations` (the per-carrier OPT counter map) is cleared only by `Permanent::new_turn`, which `begin_turn` calls for the NEW TURN PLAYER's permanents alone (`code/digimon-engine/src/game_phases.rs:88` → `self.player_mut(tp).new_turn()`; `code/digimon-engine/src/player.rs:284`). An **[All Turns][Once Per Turn]** effect consumed during the controller's turn therefore stays locked through the opponent's entire next turn and only re-arms at the controller's following turn — one full turn later than the rules allow.
- **Rules authority (general_rule.pdf §15-14, p.29):** 15-14-1-1/-2 — "[X Per Turn] means that an effect can be activated a number of times **during 1 turn** ... If an [X Per Turn] effect is used X number of times during 1 turn, it won't trigger again **during that turn**." The lockout is scoped to the single turn in which the uses occurred; a new turn (either player's) re-arms it.
- **DCGO authority:** `TurnStateMachine` end phase runs `cardSource.cEntity_EffectController.InitUseCountThisTurn()` over **`gameContext.ActiveCardList`** — every card of BOTH players — at every turn end (`CEntity_EffectController.cs:172-176`, reset loop at `TurnStateMachine.cs:3207-3210`). So in DCGO an [All Turns][OPT] observer consumed on your turn fires again on the opponent's very next turn.
- **Blast radius:** only effects that can trigger on BOTH players' turns ([All Turns] observers, opponent-turn-active inherited effects) with `once_per_turn`. [Your Turn]-only / [Main]-only OPT effects are unaffected (their next legal firing window is the controller's next turn anyway, which is exactly when the current reset happens). The prior `G-OPT-RESET-VIA-ATTACK-CYCLE` closure (qa/resolved-gaps.md, Track C) only validated controller-turn-cycle re-arm and explicitly documented the current single-player reset — the opponent-turn re-arm case was never exercised.
- **Suggested fix:** clear `effect_activations` for ALL players' permanents (battle + breeding areas) at the turn boundary — either in `begin_turn` (loop `self.players`) or at `end_turn`, matching DCGO's end-phase reset. Keep `attacks_this_turn` reset scoped to the turn player if desired (only the turn player attacks), or reset both for all players (DCGO-equivalent; attacks reset for the non-turn player is a no-op).
- **Repro / pinned test:** `code/digimon-engine/tests/cards_behavioral/bt19/bt19_075.rs` → `bt19_075_opt_rearms_on_opponents_next_turn`, `#[ignore]`'d with this gap id. The same-turn lockout and controller-turn re-arm legs (`bt19_075_opt_locks_second_deletion_same_turn`, `bt19_075_opt_rearms_by_controllers_next_turn`) pass today.
- **Verdict:** BT19-075 YAML is faithful (all three printed clauses authored); the delayed re-arm is engine-side only.

## G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-LEAVE-REPLACEMENT — Delay self-trash + union-zone free play inside a would-leave replacement (RESOLVED 2026-07-04)

Surfaced by BT19-099 The Wicked God Descends! clause 2 ("[All Turns] When any of your Digimon with [Millenniummon] in their names would leave the battle area, ＜Delay＞. ・You may play 1 [Wicked God] trait Digimon card with a play cost 1 higher than that Digimon from your hand or trash without paying the cost").

- **What was broken:** a `when_would_leave_battle_area` replacement whose body is `delete_permanent {target: source}` (the <Delay> cost) + `select_union_zone` + `play_union_bound_free` fell into the **generic replacement step runner**, which cannot thread this shape: the accepted replacement stays parked while the reward selection is pending, and after the selection chain completes the parked **Proceed commit deletes the subject by its stale battle-area index** — by then the self-trash has shifted the subject down one slot and the free play has pushed the reward card into the subject's old index. Net effect (reviewer-verified repro): the freshly played [Wicked God] card landed in the TRASH and the leaving Millenniummon SURVIVED on the field. The hand-only sibling shape was already recognised (`DelaySelfTrashPlayFromHandFlow`, ST20-14) precisely because "the generic path can't thread the reward's select → play tail across the mid-process source self-trash"; the union-zone variant had no recogniser.
- **Fix:** new recognised flow `DelaySelfTrashPlayFromUnionFlow` (`dsl_cards/lower_replacement.rs`) for `[delete_permanent{source}, select_union_zone{zones ⊆ [hand,trash], optional}, play_union_bound_free]`. It marks the replacement `handled()` (owning the leave), pays the self-trash, **finishes the owned leave by the subject's stable card handle** (the leave is NOT prevented — resolved before the reward play so the play cannot collide with the leaver's slot), then runs the reward steps verbatim through the generic step runner with **`replacement_subject` re-bound as the subject's stable top-card snapshot (a card binding)** so a `play_cost_eq_binding` reward filter reads the pre-leave printed cost — DCGO BT19_099.cs:261 evaluates `permanent.TopCard.GetCostItself + 1 == candidate.GetCostItself` against the trigger-hashtable snapshot, never a live post-deletion lookup. Clone-safe: the reward pick is the ordinary `select_union_zone` install (data-driven `RunTail` frame); the rare cost-Pending case parks the new data-driven `ResumeFrame::DelayPlayFromUnionAfterSelection`.
- **Offer gating (DCGO-verified):** the accept prompt installs **even with zero eligible reward candidates** — DCGO's `CanUseCondition` (BT19_099.cs:205-219) checks only on-battle-area + subject-match + Delay-declarable; the candidate scan (BT19_099.cs:275-277) is post-self-trash inside `ActivateCoroutine`. Accepting with zero candidates trashes the Option, plays nothing, and the leave still completes. (Note: ST20-14's engine preflight gates its offer on a hand candidate; its DCGO `CanUseCondition` (ST20_14.cs:83-87) likewise has NO candidate check — that pre-existing divergence is ST20-14-scoped and untouched here.)
- **Tests:** `tests/dsl/delay_union_play_flow.rs` (3 — engine contract with synthetic cards) + `tests/cards_behavioral/bt19/bt19_099.rs` (Clause B suite; `bt19_099_replacement_delay_then_plays_wicked_god_at_cost_plus_one_free` is the defect regression).
- **Verdict:** RESOLVED; BT19-099 fully authorable via the DSL (no raw_rust).

## G-ENGINE-SECURITY-STACK-END-OF-YOUR-TURN — `EndOfYourTurn` never fanned out to the persistent security stack (RESOLVED 2026-07-04)

Surfaced by BT25-039 Sirenmon clause 2 ("{Security} [End of Your Turn] You may play 1 [Ceresmon] from your hand with the cost reduced by 7. If this effect played, you may place this card as the played Digimon's bottom digivolution card") — a turn-boundary `[Security]` effect on a card that STAYS in the persistent security stack (DCGO `BT25_039.cs` `OnEndTurn` gated by `IsExistInSecurity(card, false) && IsOwnerTurn`). NOT an `on_discard_security` dispatch: the card is a live (face-down is fine) security card observing its owner's own End of Turn.

- **What was broken:** `Game::fire_end_of_your_turn` (game_phases.rs) enqueued `EffectTiming::EndOfYourTurn` only for `TriggerSource::PlayerBattleArea(player)` — a `scope: security` + `when: end_of_your_turn` clause could never fire, because no scan visited the security stack at that boundary. The sibling boundary already had the scan: `rotate_turn_player` fans `EndOfOpponentsTurn` out to every non-ending player's security stack via `TriggerSource::SecurityStackCard` (the `[Security] [End of Opponent's Turn]` shape).
- **Fix:** `fire_end_of_your_turn` now mirrors that scan for the ENDING player: after the battle-area enqueue and before the drain, it enqueues `EndOfYourTurn` with `TriggerSource::SecurityStackCard { player, card }` for every card in the ending player's security stack. `enqueue_from_security_stack_card` (effect_queue.rs) already filters collection to `effect.security` (the DSL `scope: security` lowering → `EffectBuilder::security_zone`), so battle-area-only `EndOfYourTurn` observers are unaffected.
- **Tests:** `tests/cards_behavioral/bt25/bt25_039.rs` Section 2/3 (fires while in security; does NOT fire from hand; plays Ceresmon at 12−7=5 real memory; place-self-under vs decline; clean no-op without a Ceresmon).
- **Verdict:** RESOLVED; consumed by `cards/bt25/BT25-039.yaml` (with the DSL-side `play_from_hand.bind_as` — see `qa/resolved-gaps.md` G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER).

## G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN — turn-scheduled <Delay> options fire as a mandatory scan (OPEN 2026-07-09)

Surfaced: 2026-07-09 bug-list faithfulness campaign, while fixing "Delay trigger is mandatory when it should be optional (observed on P-228)" (§16-16-2: "The processing from <Delay> is optional", general_rule.pdf p.35).

- **What's fixed already:** event-gated `<Delay>` (`DelayTrigger::OnEvent`) now lowers with `.optional().needs_outer_optional_prompt()` — P-228, P-229, BT22-098, BT24-089, EX10-069, BT23-096 all surface an accept/decline `pending_selection`. Main-phase-activated delays (`MainPhaseActivated`) are already player-initiated actions (no prompt needed; double-prompt guarded by tests/dsl/delay.rs).
- **What's still wrong:** turn-scheduled delays (`start_of_your_turn` / `end_of_your_next_turn` triggers: LM-027/029/030/031/032/034, BT15-096, BT21-093, BT22-099, BT24-100, P-206, ST23-15, ST24-15, LM-055) resolve through the legacy `resolve_delayed_options_matching` scan (game_phases.rs), which fires them WITHOUT an accept/decline choice — a §16-16-2 violation (the player may keep the Delay parked) and a missing RL action-space choice (no-approximations).
- **Suggested fix:** route the scheduled scan through the same outer-optional prompt machinery (`install_outer_optional_trigger_selection`) the OnEvent path now uses; decline leaves the Delay parked and re-armable at the next matching boundary.
- **Blast radius:** any scheduled-delay card where declining is strategically meaningful (e.g. keeping the option in the battle area as Decode/Partition fodder or to dodge a punish window).

## G-ENGINE-DELAY-BODY-BEFORE-TRASH — [Main]-activated Delay runs its body before trashing the Option (OPEN 2026-07-10)

- **Found by:** buglist faithfulness campaign, BT25-098 audit.
- **What's wrong:** `activate_delayed_option_main` resolves the Delay BODY first, then trashes the Option card (cause=Cost). DCGO trashes the Option FIRST and only runs the body on trash success (§16-16: "trash this card to activate the linked effect" — the trash is the cost). Divergence surfaces when the trash can be replaced/prevented (e.g. an effect protecting Options in the battle area) or when the body cares about the Option's zone.
- **Scope:** shared machinery for all MainPhaseActivated Delay options (P-035/037/039/103–107/193/205/235/236, LM-033/035/037/047/049/054/056, BT13-110, BT21-097, BT25-098, ST12-15).
- **Fix shape:** reorder in `activate_delayed_option_main`: pay the trash cost (through the replacement pipeline) first; abort the body if the Option did not actually leave.
