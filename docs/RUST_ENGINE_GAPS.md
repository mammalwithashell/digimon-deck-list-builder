# Rust Engine Gaps

Capability gaps in the Rust engine's scripting surface (`code/digimon-engine/`), discovered during archetype audits by `assess-rust-engine-archetype`. Distinct from [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md), which tracks Rust↔Python divergences in shared subsystems — this document catalogs **net-new primitives** the Rust scripting API needs before a given archetype can be implemented under the no-approximations policy (CLAUDE.md §17–18).

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
| DNA Omnimon | 2026-04-17 | 64 | 1 | 4 | 59 |
| TS Olympos | 2026-04-18 | 105 | 1 | 4 | 100 |
| Rocks | 2026-04-18; refreshed 2026-04-28 | 47 | 0 | 0 | 47 |
| Dark Masters | 2026-04-18 | 58 | 0 | 0 | 58 |

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
| [Selection: aggregate-sum residual sub-shapes (self-stack material / cost-time placement)](#selection-aggregate-sum-residual-sub-shapes) | 🟡 | 2+ | `effect_context.rs`, `action/` |
| [Selection: `select_any_permanent` curated helper + `select_dna_pair` plumbing audit](#selection-select_any_permanent-curated-helper--select_dna_pair-plumbing-audit) | 🟡 | 4+ | `effect_context.rs`, `dsl_cards/step/selections.rs` |
| [`play_from_revealed_free` (EX8-050 Gogmamon)](#play_from_revealed_free-ex8-050-gogmamon) | 🟡 | 1 | `effect_context.rs` |
| [`play_from_security_at(index)` (BT13-012 GeoGreymon, BT14-033 Patamon)](#play_from_security_atindex-bt13-012-geogreymon-bt14-033-patamon) | 🟡 | 2 | `effect_context.rs` |
| [Zone-manipulation: return-to-deck-top / self-return-as-cost](#zone-manipulation-return-to-deck-top--self-return-as-cost) | 🟡 | 4+ | `effect_context.rs`, `permanent.rs` |
| [Zone-manipulation: reveal-top-N residual (`play_from_revealed_free`)](#zone-manipulation-reveal-top-n-residual-play_from_revealed_free) | 🟡 | 1 | `effect_context.rs`, `game.rs` |
| [Zone-manipulation: top-N security trash + face-up security flip/extraction](#zone-manipulation-top-n-security-trash--face-up-security-flipextraction) | 🟡 | 3+ | `effect_context.rs`, `combat.rs` |
| [Alt-digivolve with override-cost + ignore-reqs + face-down placement](#alt-digivolve-with-override-cost--ignore-reqs--face-down-placement) | 🟡 | 4+ | `effect_context.rs`, `permanent.rs`, `game.rs` |
| [`<Training>` keyword](#training-keyword) | 🔴 | 1 | `enums.rs`, `card_source.rs`, `effect_context.rs`, `action/` |
| [Dynamic DP scaling residual (non-aura temporary dynamic DP grants)](#dynamic-dp-scaling-residual-non-aura-temporary-dynamic-dp-grants) | 🟡 | 1 | `effect.rs`, `tensor.rs` |
| [Condition-gated modifier residual: filter-aura + `while_condition` lazy-filter rewrite](#condition-gated-modifier-residual-filter-aura--while_condition-lazy-filter-rewrite) | 🟡 | 1 | `modifiers.rs`, `effect.rs` |
| [Player-scoped modifier registry residual: bilateral `UntilLeaveField` delivery (BT14-009)](#player-scoped-modifier-registry-residual-bilateral-untilleavefield-delivery-bt14-009) | 🟡 | 1 | `modifiers.rs`, `enums.rs` |
| [Option card play flow residual: place-Option-in-battle-area + [Hand][Main] Plug-In flow](#option-card-play-flow-residual-place-option-in-battle-area--handmain-plug-in-flow) | 🟡 | 11 | `game.rs`, `effect.rs`, `effect_context.rs`, `action/` |
| [Standard Delay main-phase activation action](#standard-delay-main-phase-activation-action) | 🟡 | 3+ | `game_actions.rs`, `action/mask.rs`, `effect_context.rs` |
| [Trait-filter helpers on `CardSource` / `Permanent`](#trait-filter-helpers-on-cardsource--permanent) | 🟡 | pervasive | `card_source.rs`, `permanent.rs` |
| [Digivolution-stack name overlay ("has all names of materials")](#digivolution-stack-name-overlay-has-all-names-of-materials) | 🔴 | 1 | `effect.rs`, `card_source.rs`, `permanent.rs` |
| [Decode residual: native `Keyword::Decode` sugar](#decode-keyword-play-from-own-digivolution-stack-without-paying-cost-on-non-battle-leave) | 🟡 | 1 | `effect.rs` |
| [Ergonomics partials](#ergonomics-partials) | 🟡 | pervasive | `effect.rs`, `effect_context.rs` |
| [Grant Security A. ±N modifier — targeted typed sugar](#grant-security-a-n-modifier-to-a-targeted-permanent-parametric-securityattackchange) | 🟡 | 3+ | `effect_context.rs` |
| [Play / digivolve origin context flag — effect-spawned cleanup token half](#play--digivolve-origin-context-flag-if-played-by-effects-if-digivolved-by-this-effect) | 🟡 | 4+ | `effect.rs`, `effect_context.rs` |
| [Generic `pop_top_digivolution_source` for arbitrary re-routing (BT24-093)](#digivolution-stack-source-extraction-pop_top_source-from-named-permanent) | 🟡 | 1 | `effect_context.rs`, `permanent.rs` |
| [Conditional digivolve-target restriction (filter on candidate top-card name/trait/level/color)](#conditional-digivolve-target-restriction-filter-on-candidate-top-card-nametraitlevelcolor) | 🔴 | 7+ | `modifiers.rs`, `effect.rs` |
| [Effect-spawned permanent with end-of-turn deletion rider](#effect-spawned-permanent-with-end-of-turn-deletion-rider-delete-the-digimon-this-effect-played) | 🔴 | 7+ | `game.rs`, `effect_context.rs` |
| [Cast-time stack-construction for cost reduction (BT15-102 Apocalymon)](#cast-time-stack-construction-for-cost-reduction-place-n-differently-named-cards-from-battle-areatrash-under-the-played-card) | 🔴 | 1 | `game.rs`, `effect_context.rs` |
| [Cross-card effect re-firing — foreign-card source-card variant (BT15-102)](#cross-card-effect-re-firing--activate-a-foreign-cards-on-play-effect-attributed-to-the-source) | 🟡 | 1 | `effect_context.rs` |
| [Reveal-zone overlay (declarative type/level synthesized while card is in deck or being revealed)](#reveal-zone-overlay-declarative-typelevel-synthesized-while-card-is-in-deck-or-being-revealed) | 🔴 | 1 | `effect.rs`, `card_source.rs` |
| [Effect-initiated play from face-up security stack (search-then-play-free)](#effect-initiated-play-from-face-up-security-stack-search-then-play-free) | 🔴 | 5+ | `effect_context.rs` |
| ~~Generic `.activation_cost(...)` builder hook for triggered abilities~~ — RESOLVED 2026-05-17 (Phase 2 Track B) | ✅ | — | — |
| ~~Once-per-turn enforcement for triggered effects (`G-OPT-TRIGGERED`)~~ — RESOLVED 2026-05-17 (Phase 2 Track C: diagnosed as already-closed; 23 stale `#[ignore]` annotations removed, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~OPT slot reset across turn cycle (`G-OPT-RESET-VIA-ATTACK-CYCLE`)~~ — RESOLVED 2026-05-17 (Phase 2 Track C: misdiagnosis; test-setup-only fix, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Inherited triggered-effect dispatch (`enqueue_from_permanent` digivolution-stack walk)~~ — RESOLVED 2026-05-17 (Phase 2 Track D: substrate completion + regression test + 18 tests un-ignored, see `qa/resolved-gaps.md`) | ✅ | — | — |
| ~~Standard Delay main-phase activation action (`PUPPETS-G009`)~~ — RESOLVED 2026-05-17 (Phase 2 Track I, see `qa/resolved-gaps.md`) | ✅ | — | — |
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
| [Costed self-digivolve stable source binding](#costed-self-digivolve-stable-source-binding) | 🔴 | 1 | `effect_context.rs`, `binding_ref.rs` |
| [Narrow opponent-effect protection for DP reduction and De-Digivolve](#narrow-opponent-effect-protection-for-dp-reduction-and-de-digivolve) | 🔴 | 1 | `modifiers.rs`, `effect.rs` |
| ~~Effect play with played-Digimon On Play suppression~~ | ✅ | — | RESOLVED 2026-05-19 (Phase 2 Track J Task S1.1) — see [qa/resolved-gaps.md](../qa/resolved-gaps.md#engine--dsl-gap-effect-play-with-played-digimon-on-play-suppression--resolved-2026-05-19-phase-2-track-j-task-s11-puppets-g030) |
| ~~End-of-attack mandatory self-delete chain (EX4-074)~~ | ✅ | — | RESOLVED 2026-05-17 (Track I first-test confirmed existing primitives suffice) — see [qa/resolved-gaps.md](../qa/resolved-gaps.md#engine-gap-end-of-attack-mandatory-self-delete-chain-with-recovery-and-conditional-hatch--resolved-2026-05-17-track-i) |

**Group 5 contract note (2026-05-02):** Group 5 did not change ACTION_SPACE_SIZE or TENSOR_SIZE. New Link/Delay choices reuse existing pending-selection masks.

**Zephagamon prep note (2026-05-03):** Task 4 added an EX11-074/Vortexdramon readiness slice in `code/digimon-engine/cards/ex11/EX11-074.yaml` and `code/digimon-engine/tests/cards_behavioral/ex11/ex11_074.rs`. The slice confirms the rule boundary that an effect battle resolves DP battle and `EndOfBattle`, but is not an attack: even if the attacker has `<Piercing>`, the `battle:` step must not trigger Piercing security checks and must not leave `pending_attack` populated. Remaining Zephagamon-specific blockers are documented in `qa/dsl-vocab-gaps.md`: conditional "if this effect suspended your Digimon" branch/binding support for EX11-074, BT20-101 suspended-Digimon count / divide-by-2 / capped multi-select bottom-deck formula, EX11-035 formula DP cap for green Avian/Bird play, and EX11-062 conditional `VortexCanAttackPlayer` aura while the opponent has no unsuspended Digimon.

**Track J formula/result substrate slice (2026-05-10):** Formula-valued `play_cost_lte` is now wired for selection filters, including `binding_play_cost` for a previously selected card/permanent and `distinct_colors_count` for BT21-102's Tamer-color cap. The same formula-threshold shape now covers the existing level, DP, stack/material-count, memory, security-count, and general count aggregate predicate leaves. Runtime bindings also carry an append-only per-effect result log for result-bound predicates such as `effect_suspended_any_own_digimon` and `effect_returned_any_card`, and formulas can count suspended battle-area permanents through `suspended_count`. The validator rejects `binding_dp` / `binding_play_cost` formulas that reference bindings before their declaring step. This closes the BT15-096 / BT21-102 play-cost-threshold gap and activates BT15-096's six behavioral tests. Coverage: `cargo test --manifest-path code/digimon-dsl/Cargo.toml`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt21_102 -- --nocapture`. Remaining Track J work is card authoring/fixture expansion for the Zephagamon, TS Olympos, and BG Imperial cards that need these primitives in full production YAML.

**Validated 2026-05-14 (PR #470):** BT15-096 Supreme Connection! and BT21-102 Undine behavioral tests now ship as card-shaped proof that the Track J substrate landed correctly on real cards. No new substrate surfaced; the slice remains closed.

## Open gaps

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
- **Severity:** 🟡 PARTIAL — sub-shape spun off from "Zone-manipulation: play-from-hand / trash without paying cost" headline closure (2026-05-15)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX8-050 Gogmamon
- **Effect text:** "play from revealed" — picks one of the just-revealed top-N deck cards and plays it without paying cost.
- **What's missing:** A `play_from_revealed_free(player, reveal_index)` curated helper. Reveal-zone is wired (`reveal_top_deck` / `revealed_cards`) but no helper consumes a chosen reveal index, removes it from `Game.revealed_cards`, and routes the card through `play_from_hand_with_cost(..., CostDelta::Reduce(printed_cost))`-equivalent semantics without first detouring through the hand. Naïve `add_to_hand_from_deck` + `play_from_hand_free` puts the card briefly in hand and fires OnAddToHand observers that the printed text does not authorize.
- **Suggested API shape:** `ctx.play_from_revealed_free(player, reveal_index) -> Option<PermanentHandle>` analogous to the existing `play_from_hand_with_cost`, but consuming from the reveal pool.
- **Workaround:** None faithful; hand-transit fix-up violates §17 no-approximations.
- **Related:** "Zone-manipulation: play-from-hand / trash without paying cost" (closed headline — see [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-zone-manipulation-play-from-hand--trash-without-paying-cost--cost-override--resolved-2026-05-15-phase-2-pr-track-a-2026-05-08)).

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
- **Severity:** 🟡 PARTIAL (residual — headline reveal-top-N + add-to-hand + hatch closed by Phase 2)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** EX8-050 Gogmamon (reveal 3 + play-from-revealed-free with cost/trait filter)
- **Effect text:** "Reveal the top 3 cards of your deck. You may play 1 [Rock] trait Digimon with cost 4 or less from them without paying the cost."
- **What's missing:** Headline reveal-top-N primitives shipped (see [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) — `EffectContext::reveal_top_deck`, `add_to_hand_from_deck`, `add_to_hand_from_trash`, `hatch`). Residual: `play_from_revealed_free` (consume a reveal index, route through the play pipeline without hand transit). Tracked under "[`play_from_revealed_free` (EX8-050 Gogmamon)](#play_from_revealed_free-ex8-050-gogmamon)" above.
- **Related:** "[`play_from_revealed_free` (EX8-050 Gogmamon)](#play_from_revealed_free-ex8-050-gogmamon)" — same sub-shape.

### Zone-manipulation: top-N security trash + face-up security flip/extraction
- **Severity:** 🟡 PARTIAL (residual — headline security-stack operations closed by Phase 2 + Track A/E)
- **Discovered in:** DNA Omnimon (2026-04-17); Rocks (2026-04-18); Dark Masters (2026-04-18)
- **Card(s):** EX4-073 Omnimon Alter-B (trash top 2 opp security — multi-N variant), BT20-055 Invisimon (flip opp top face-down security face-up), EX10-061 Apocalymon (security-card extraction with face-up filter)
- **Effect text:** "trash top 2 opp security" / "flip opp top face-down security face-up" / "place 1 of each face-up [Dark Masters] trait card with different names from your security stack under this card"
- **What's missing:** Headline security primitives shipped — `place_on_security` (Top/Bottom/Random), `trash_top_security` (single-card), `add_top_security_to_hand`, `recover_from_deck`, `place_self_at_security`, `place_self_option_at_security`, `security_place_stacked_card`, `security_place_top_stacked_card`, `place_permanent_on_security`, `search_own_security_stack`. Residual: a multi-N `trash_top_security(player, N)` form (today's helper trashes exactly 1), face-up security extraction with filter, and face-down → face-up flip primitive.
- **Suggested API shape:** Generalize `trash_top_security(player, count)` to handle N>1; add `extract_face_up_security(filter, callback)`; add `flip_security_face_up(player, index)`.
- **Workaround:** Loop single-card `trash_top_security` for the multi-N case where order is irrelevant; face-up flip and extraction have no faithful workaround.
- **Related:** Closed core in [`qa/resolved-gaps.md`](../qa/resolved-gaps.md).

### Zone-manipulation: security stack operations (trash top, place top/bottom, trash N, Recovery +N, shuffle security)
> Core security-stack primitives moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md) by the 2026-05-15 hygiene sweep. Residual top-N security trash + face-up extraction/flip tracked above as "[Zone-manipulation: top-N security trash + face-up security flip/extraction](#zone-manipulation-top-n-security-trash--face-up-security-flipextraction)".

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

### Standard Delay main-phase activation action — RESOLVED 2026-05-17 (Phase 2 Track I)
- **Status:** Closed. PUPPETS-G009 closure shipped in Track I (commit `26e27ccc`). Standard `<Delay>` Options on the field now expose a `[Main]` activation action through the normal main-phase action mask after the placing turn — the action trashes the Option as cost, then runs the stored Delay body. Pass/decline leaves the Option in the battle area for later legal activation. No `ACTION_SPACE_SIZE` change (Working Rule §1). Full closure details in `qa/resolved-gaps.md` § "Phase 2 Track I closure".
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
- **Severity:** 🔴 BLOCKING
- **Discovered in:** DNA Omnimon (2026-04-17)
- **Card(s):** BT17-102 Greymon ("[All Turns] This Digimon has all the names of level 3 and lower cards in its digivolution cards.")
- **Effect text:** As above.
- **What's missing:** `Permanent::contains_card_name` already walks the stack for self-checks, but external name lookups on this permanent from other cards see only the top card's printed name. No "virtual name overlay" mechanism that synthesizes additional names for external queries (e.g., another Tamer's aura that checks "[Koromon]" should see the overlay names).
- **Suggested API shape:** `Effect::declarative(card).name_overlay_from_sources(|src, data| src.level(data).map_or(false, |l| l <= 3))`; update all name-lookup surfaces (aura filters, inherited-effect name checks, trait-from-name derivations) to union overlays into the lookup set.
- **Workaround:** None — BLOCKED for external observers that query names on this permanent.
- **Related:** "Named-target declarative aura".

### Decode keyword (play from own digivolution stack without paying cost on non-battle leave)
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; BT22-015 Red/Black Decode, EX4-060 BlitzGreymon/CresGarurumon ladder, and EX9-021 End-of-Attack source-play all close. **Updated 2026-05-19 (Track J substrate S1.2):** the batch / different-name source-play DSL sugar is now CLOSED — see "What's closed" below. Residual is the native `Keyword::Decode` parsing sugar only.)
- **Discovered in:** DNA Omnimon (2026-04-17); Dark Masters (2026-04-18)
- **Card(s):** BT22-015 Omnimon ("＜Decode (Red/Black Lv.3)＞ — When this Digimon would leave the battle area other than in battle, you may play 1 Red or Black Level 3 Digimon card from its digivolution cards without paying the cost.") — Dark Masters adds: EX10-061 Apocalymon ("[On Play] [When Digivolving] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs").
- **Effect text:** As above.
- **Status:** Partially closed 2026-05-07 for BT22-015; narrowed again 2026-05-08 for EX4-060 and EX9-021. `select_material` now honors card predicates over the source stack, and `play_from_materials.source_index` may consume the selected `CardHandle` binding. `play_from_materials.bind_as` records a successful source play for follow-up gates. BT22-015's Red/Black Lv.3 and Blue/Yellow Lv.3 Decode clauses are faithful optional non-cancelling replacement subscribers. EX4-060's mandatory BlitzGreymon + CresGarurumon sequence is authored through sequential material selections, with its self-to-security tail handled by `place_permanent_on_security_and_handle_replacement`. EX9-021's End of Attack sequence is authored through the same source-play steps plus `binding_exists` and `place_permanent_on_security`. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_015`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex4_060`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_021`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement`.
- **What's closed (2026-05-19, Track J S1.2):** the batch / different-name source-play DSL sugar landed. A new `select_materials` DSL step picks *up to N* digivolution sources of a carrier permanent in ONE count-capped multi-pick, optionally constrained by `uniqueness: name` ("1 of each different name"). It lowers to `EffectContext::select_count_capped_multi` with `CountCappedZone::Material` + `DistinctByMode` — REUSING the existing count-capped action mask (no `ACTION_SPACE_SIZE` change). `play_from_materials` now consumes a `CardList` binding as a batch (each picked source becomes a fresh permanent), composing with the S1.1 `suppress_on_play` flag. Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- **What's missing:** Only the native `Keyword::Decode(Vec<Color>, u8)` printed-keyword parsing sugar + auto-emission in the leave-field replacement path. The batch / different-name source play is fully expressible today via `select_materials` + `play_from_materials`. EX10-061 Apocalymon's remaining blocker is its *cast-time* stack-construction half (see "Cast-time stack-construction for cost reduction"), not the source-play extraction.
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
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** EX10-012 MetalSeadramon, EX10-020 Puppetmon, EX10-035 Machinedramon, EX10-057 Piedmon, EX10-061 Apocalymon, EX10-072 Spiral Mountain, P-216 WaruMonzaemon — Puppets adds: P-165 ShoeShoemon ("delete that token" at the end of the opponent's turn)
- **Effect text:** "At turn end, delete the Digimon this effect played." / "The Digimon this effect played can't digivolve and is deleted at turn end." / "At the end of your opponent's turn, delete that token."
- **What's missing:** No mechanism for an Effect to (a) capture the `PermanentHandle` of the card it just played via a free-play helper and (b) schedule a deferred end-of-turn cleanup tied to that specific permanent (no-op if the card was already deleted earlier in the turn). `Permanent` has no `played_by_effect: Option<{source_card, effect_slot, expiry}>` provenance field. The end-of-turn drain (existing for transient Options under "Scheduled end-of-turn effect queue") does not key on per-permanent identity. Sibling-but-distinct from that scheduled-EOT entry: that gap covers arbitrary closures from trash; this gap covers per-permanent provenance-anchored cleanup that survives stack shifts.
- **Suggested API shape:** `ctx.play_from_X_free_then(...)` variants returning the resulting `PermanentHandle`, paired with `ctx.schedule_delete_at_end_of_turn(handle: PermanentHandle, source: CardHandle)` that enqueues a closure surviving stack-shift (snapshot the card_index, look it up at EOT, no-op if absent). Backed by `Game.scheduled_eot_deletions: Vec<{card_index, source}>` drained inside `end_turn` after standard `EndOfYourTurn` triggers but before memory reset. Alternative: `ModifierType::DeleteAtEndOfTurn` permanent-scoped modifier consumed by the EOT pass.
- **Workaround:** "None — BLOCKED." Hand-rolling a `Vec<PermanentHandle>` snapshot in a closure desyncs after stack shifts; an unconditional EOT scan would over-delete unrelated permanents.
- **Related:** "Scheduled end-of-turn effect queue (for transient Options)" (sibling — generic closure scheduling vs. provenance-anchored deletion); "Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)" (the play-free helpers must return a handle for this to chain); "Token creation + `CardKind::Token` + Petrification Token definition" (P-165 needs the token sibling: `play_token` must bind the newly created token handle before the cleanup can target exactly "that token").

### Effect-driven play of a Digimon from hand to an empty breeding-area slot (without paying cost)
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-driven-play-of-a-digimon-from-hand-to-an-empty-breeding-area-slot-without-paying-cost--resolved-2026-05-15-group-4) by the 2026-05-15 hygiene sweep.

### Cast-time stack-construction for cost reduction (place N differently-named cards from battle-area/trash UNDER the played card)
- **Severity:** 🔴 BLOCKING (deferred — Track E 2026-05-08)
- **Discovered in:** Dark Masters (2026-04-18)
- **Card(s):** BT15-102 Apocalymon
- **Effect text:** "When this card would be played, by placing up to 3 [Dark Masters] trait cards with different names from your battle area or trash under it, reduce the play cost by 4 for each one."
- **What's missing:** Mechanically resembles DigiXros (parity §4.7e) but with player-driven multi-select of UP-TO-N from a UNION of source zones (battle area + trash), under a different-name uniqueness constraint, with the placed cards becoming the new permanent's digivolution stack. None of `select_multiple_*` helpers, `place_under_played_card_at_cast_time`, or different-name selection filters exist. Distinct from EX10-061 Apocalymon's "from your security stack" cast-time variant (sibling primitive, different source zone).
- **Suggested API shape:** `Effect::before_pay_cost(card).with_optional_under_placement(max_count: u8, source_zones: &[Zone], filter, uniqueness: UniquenessFilter::Name, cost_per_placed: i16, callback)` — surfaces a multi-select at cost-time; for each chosen card, removes from source zone, queues for stack-attachment after the play resolves; reduces effective `play_cost` by `cost_per_placed * count`.
- **Workaround:** "None — BLOCKED." Pre-deciding the placement count auto-selects on the player's behalf (violates §17); skipping the reduction makes Apocalymon unplayable.
- **Related:** "Dynamic cost reduction at `BeforePayCost`"; RUST_PYTHON_PARITY.md §4.7e (DigiXros cost-reduction); "Place card at a specific stack position".
- **Track E (2026-05-08) deferred — implementation strategy for follow-up:** The work requires surgery on `Game::play_from_hand_with_cost_result` (`code/digimon-engine/src/game_actions.rs`) to splice a pre-`OnPlay` assembly hook between the cost calculation step and the `OnPlay` drain. Current flow returns `Played(field_index)` only after `OnPlay` drains; a cast-time-assembly hook must run after the permanent enters battle area but before `OnPlay` triggers fire. Suggested implementation phases:
  1. Carve out an internal `commit_play_to_battle_area_without_on_play(player, hand_index, cost_delta) -> Option<usize>` from the existing `play_from_hand_with_cost_result` so the placement and the `OnPlay` drain become separable.
  2. Add `EffectContext::play_with_cast_time_assembly(player, hand_index, cost_delta, max_count, source_zones, filter, cost_per_placed)` that calls the inner placement, installs a count-capped multi-select over `source_zones` with `is_optional_zero=true`, and on resolve: (a) installs each chosen card under the new permanent via `place_as_bottom_source` (top-down), (b) reduces memory cost retroactively by `cost_per_placed * count`, (c) drains `OnPlay`.
  3. DSL verb `cast_time_assembly:` block within the `play:` step.
  Until this lands, BT15-102 Apocalymon's [Main] play is OMITTED from any compiled YAML.

### Cross-card effect re-firing — activate a foreign card's [On Play] effect attributed to the source
- **Severity:** 🟡 PARTIAL
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
- **Severity:** 🟡 PARTIAL (audit 2026-05-15: narrowed; [Security] [End of Opponent's Turn] self-play closed by BT20-055. Residual: Start-of-turn / Start-of-opponent-turn security-stack timing variants need boundary-iteration extension to `begin_turn` / `rotate_turn_player` plus face-up security lifecycle/visibility.)
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** BT20-055 Invisimon (`[Security] [End of Opponent's Turn] Play this card without paying the cost.`)
- **Effect text:** As above.
- **What's missing:** Current security-effect plumbing (RUST_PYTHON_PARITY §2.5a) fires `SecuritySkill` effects only when a security card is revealed during an attack's security check. A subset of cards carry security-slot effects that gate on **global turn-phase timings** while the card remains face-down in the stack. No scheduling pass iterates each security card's effects at turn boundaries. A dedicated `play_from_security_at(player, security_index)` path is required (distinct from the attack-time `play_from_security()` which reads `pending_security`).
- **Suggested API shape:** Add `EffectTiming::SecurityOnStartYourTurn` / `SecurityOnEndYourTurn` / `SecurityOnStartOpponentsTurn` / `SecurityOnEndOpponentsTurn` variants (or extend `SecuritySkill` with a turn-boundary gate). Iterate each player's security stack at `begin_turn` / `end_turn`, enqueue matching effects (the iterator must include face-down cards; the card text explicitly activates from security without being revealed by an attack). Add `ctx.play_from_security_at(player, index)` popping the indexed security card and playing it without paying cost.
- **Status (2026-05-08):** Narrowed for `[Security] [End of Opponent's Turn]` self-play. DSL `scope: security` now compiles to security-zone effects; `rotate_turn_player` scans the non-ending player's persistent security stack for `EndOfOpponentsTurn`; `play_from_security` removes the exact source card rather than blindly popping the top. Coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- security_scope_end_of_opponents_turn_plays_this_security_card`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_055_security_end_of_opponents_turn_plays_self_from_security`.
- **Remaining:** start-of-turn/start-of-opponent-turn security-stack timing variants, face-up security lifecycle/visibility, and BT20-055's security-flip rider from its On Play/When Digivolving branch.
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

### `ctx.move_from_breeding()` EffectContext helper
- **Severity:** 🟡 PARTIAL — Group 4 primitive landed (`move_from_breeding_by_effect`, `play_to_breeding_from_hand`, `place_as_bottom_source` with BREEDING_TARGET). Residual: optional level-filtered prompt wrapper for P-130-style "you may move 1 of your level 3 or higher Digimon" text + broader breeding-area trigger fan-out.
- **Discovered in:** Rocks (2026-04-18)
- **Card(s):** P-130 Lui Ohwada (`[On Play] You may move 1 of your level 3 or higher Digimon from the breeding area to the battle area.`)
- **Effect text:** As above.
- **What's missing:** `Game::move_from_breeding(player_id)` exists only as an action-decoder entry point (invoked by the action space's `MOVE_FROM_BREEDING = 61` bit). Scripts cannot initiate the move from a `process` closure. Additionally, P-130 imposes a level filter (level 3 or higher in breeding) that isn't a standard move-from-breeding rule and must be enforced by the effect-initiated variant. Optional-prompt wrapping is also needed ("You may move…" → the mask must surface a choose/decline so the RL action space sees the choice).
- **Suggested API shape:** `ctx.move_from_breeding(player: PlayerId) -> bool` delegating to the existing `Game::move_from_breeding`. Optional-prompt variant: `ctx.offer_move_from_breeding(player, filter: Fn(&Permanent, &[CardData]) -> bool, is_optional: bool, callback: Fn(&mut EffectContext, bool))` — installs an `EffectChoice` (Yes, move / No, skip) gated on the filter, runs the move inside the Yes branch. The move must fire `OnEnterField` + `OnEnterFieldAnyone` + the new `[When Moving]` observer so downstream observers see the event (including P-130's own second effect).
- **Workaround:** `ctx.game.move_from_breeding(player)` via the escape hatch (`ctx.game: &mut Game`) bypasses the curated API (RUST_ENGINE_API §2/§3), skips the optional-prompt gate (violates §17 no-auto-selections), and doesn't enforce the level-3 filter. Not ship-worthy.
- **Related:** Existing "Observer timings tied to specific events" (`[When Moving]` observer side); existing "Zone-manipulation: reveal-top-N deck + add-to-hand + hatch" (parallel `ctx.hatch` helper gap).
- **Updated 2026-05-02:** Group 4 added `EffectContext::move_from_breeding_by_effect`, `EffectContext::play_to_breeding_from_hand`, and `BREEDING_TARGET` support in `place_as_bottom_source`. The real breeding slot is used, source stacks stay intact, and movement observers fire. Covered by `breeding_zone_movement::{move_from_breeding_by_effect_moves_real_breeding_stack_and_fires_move_observers,play_to_breeding_from_hand_uses_real_breeding_slot_and_rejects_occupied_slot,place_as_bottom_source_with_breeding_target_tucks_under_real_breeding_stack}`. Remaining work is the optional level-filtered prompt wrapper for P-130-style "you may move..." text and broader breeding-area trigger fan-out.

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
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets Batch 5 (2026-05-04)
- **Card(s):** `EX9-032` Karakurumon
- **Effect text:** "[On Play] [When Digivolving] By deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon may digivolve into a [Puppet] trait Digimon card in your hand without paying the cost."
- **What's missing:** Effect resolution needs a stable binding for the resolving source permanent across mid-body deletions. `EffectContext::source_permanent` stores a `PermanentHandle { player, index: u8 }` — an index into the controller's battle area. When a mid-body step deletes a lower-indexed permanent, `Player::delete_permanent` does `battle_area.remove(index)` (no handle adjustment), shifting all later permanents down by one, but `ctx.source_permanent` keeps the original (now stale) index. The subsequent `effect_initiated_digivolve { target: source }` resolves the stale handle and either targets the wrong slot or none. Preflight also needs to prove that a legal Token/Puppet cost body exists while excluding the source itself.
- **Suggested API shape:** Either (a) refactor `PermanentHandle` to a stable id (e.g. `CardHandle` of the base card) with lookup helpers, or (b) maintain a `source_permanent_card: CardHandle` snapshot alongside `source_permanent`, refreshed by `binding_ref.rs::Source` resolution by searching for the carrier card-handle in the live battle area. Approach (b) is the lower-blast-radius option and matches the audit footer's "existing `ctx.source_permanent` snapshot semantics" intent.
- **Workaround:** None faithful. Omitting the active slice is safer than hidden auto-costing or index-based self-digivolve.
- **Updated 2026-05-17 (Track I first-test):** First test [`code/digimon-engine/tests/cards_behavioral/ex9/ex9_032.rs`](../code/digimon-engine/tests/cards_behavioral/ex9/ex9_032.rs) `ex9_032_on_play_deletes_token_or_other_puppet_then_free_digivolves_into_puppet` written and confirmed failing. Repro: PUPPET-COST / PLAIN-COST / TOKEN-COST seeded into battle area, EX9-032 played to index 3; selecting PUPPET-COST (index 0) as cost deletes idx 0 — Karakurumon's live index drops to 2, but `ctx.source_permanent.index` is still 3, and the `effect_initiated_digivolve { target: source }` never lands the digivolve. **What now works without engine change:** the `when_digivolving` half (`ex9_032_when_digivolving_uses_same_cost_and_free_puppet_hand_digivolve_flow`) and the `optional`-decline path (`ex9_032_declining_on_play_cost_does_not_delete_or_digivolve`) pass against the current YAML, because they avoid the index-shift scenario (Karakurumon at idx 0, cost body at idx 1). **Still BLOCKING:** the index-shift case (above) and the no-legal-cost-preflight case (separate G-COSTED-SELF-DIGIVOLVE-PREFLIGHT).

### Inherited Token/Puppet leave-prevention replacement dispatch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-inherited-tokenpuppet-leave-prevention-replacement-dispatch--resolved-2026-05-15-track-b-2026-05-08) by the 2026-05-15 hygiene sweep.

### Effect-played permanent cleanup provenance
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-effect-played-permanent-cleanup-provenance--resolved-2026-05-15-track-a-pr-451) by the 2026-05-15 hygiene sweep.

### Suspend-this-Tamer deletion observer with Overclock cause branch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-suspend-this-tamer-deletion-observer-with-overclock-cause-branch--resolved-2026-05-15-2026-05-06) by the 2026-05-15 hygiene sweep.

### Narrow opponent-effect protection for DP reduction and De-Digivolve
- **Severity:** 🔴 BLOCKING
- **Discovered in:** Puppets Batch 8 (2026-05-04)
- **Card(s):** `BT16-055` Namakemon
- **Effect text:** "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects."
- **What's missing:** A category-scoped protection modifier that blocks only opponent DP reduction and opponent De-Digivolve effects under a live security-count predicate. Existing broad immunity would over-block legal effects; source-scoped zone-return immunity does not cover DP reduction.
- **Suggested API shape:** Add effect-category protection entries such as `ModifierType::ImmuneToOpponentDpReduction` and `ModifierType::ImmuneToOpponentDeDigivolve`, or a parametric `EffectCategoryProtection { source_player, categories, predicate }`, and consult them at DP-reduction and De-Digivolve effect sites.
- **Workaround:** None faithful. Do not model this as `CannotBeAffected`.
- **First test:** With `BT16-055` in battle and 3 security, resolve opponent DP reduction and De-Digivolve effects against it and assert both are blocked; repeat at 2 security and assert both apply.

### Trash-resident observer with effect digivolve from trash
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-trash-resident-observer-with-effect-digivolve-from-trash--resolved-2026-05-15-2026-05-06) by the 2026-05-15 hygiene sweep.

### Effect play with played-Digimon On Play suppression
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine--dsl-gap-effect-play-with-played-digimon-on-play-suppression--resolved-2026-05-19-phase-2-track-j-task-s11-puppets-g030) by Phase 2 Track J Task S1.1 (2026-05-19). `game::PlayOptions { suppress_on_play }` threads through the play pipeline; `commit_play_from_hand_card_no_replace` skips `fire_on_play` for the just-played permanent only. DSL `suppress_on_play: true` flag on the `play_from_*` steps. BT5-106's [Security] slice is now authored in `code/digimon-engine/cards/bt5/BT5-106.yaml`.

### End-of-attack mandatory self-delete chain with recovery and conditional hatch
> Moved to [`qa/resolved-gaps.md`](../qa/resolved-gaps.md#engine-gap-end-of-attack-mandatory-self-delete-chain-with-recovery-and-conditional-hatch--resolved-2026-05-17-track-i) by the 2026-05-17 Track I first-test confirmation. Existing primitives (`delete_permanent { target: source }`, `select_opponent_permanent { optional: true }`, `recover`, `if { any_field_permanent + can_hatch } then hatch`) compose into a faithful chain — see `code/digimon-engine/cards/ex4/EX4-074.yaml` Clause 2 and `code/digimon-engine/tests/cards_behavioral/ex4/ex4_074.rs::ex4_074_end_of_attack_self_deletes_opponent_delete_recovers_and_hatches_with_tamer`.

## Resolved gaps

Resolved Rust engine group summaries have been moved to [qa/resolved-gaps.md](../qa/resolved-gaps.md#rust-engine-gap-group-summaries).
