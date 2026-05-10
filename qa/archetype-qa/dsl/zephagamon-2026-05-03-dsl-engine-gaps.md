# Zephagamon Rust DSL/Engine Gap Source Document

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


Date: 2026-05-03
Archetype: Zephagamon / Vortex Warriors
Assessment source: `data/deck_library.json` archetype `Zephagamon`
Rust target: `code/digimon-engine/` plus YAML DSL under `code/digimon-engine/cards/`
Verdict: blocked

This document captures reusable Rust engine and DSL gaps surfaced by the Zephagamon archetype so they can be folded into a cross-archetype gap spec. It is intentionally stricter than `qa/archetype-qa/Zephaga.md`, which is a legacy Python-lane faithfulness report from 2026-03-17.

The controlling rule is the repository no-approximations policy: every gameplay choice must flow through engine actions or `PendingSelection`. Do not close these gaps with hidden auto-selection, no-op raw Rust placeholders, or UI-only decisions.

## Assessment Target

`data/deck_library.json` has 54 local `Zephagamon` decklists. The current archetype aliases include `Vortex`, `Vortex Warriors`, and `Zephaga`.

High-frequency cards across those lists:

| Card | Name | Frequency | Core role |
|---|---:|---:|---|
| `EX7-064` | Shoto Kazama | 54/54 | memory, end-turn keyword grant, unsuspend |
| `P-166` | Galemon | 52/54 | suspend, effect digivolve with suspended-count cost reduction |
| `ST18-04` | Pteromon | 52/54 | reveal search |
| `LM-030` | Green Scramble | 48/54 | resource option / Delay-style support |
| `P-106` | Agility Training | 48/54 | search and reduced-cost digivolve |
| `BT20-101` | Zephagamon | 47/54 | ACE, Blast Digivolve, Vortex, bottom-deck payoff |
| `EX7-031` | Pteromon | 46/54 | cost reduction / inherited memory |
| `ST22-13` | GrandGalemon | 46/54 | Fortitude, Vortex, suspend/DP, inherited unsuspend |
| `EX8-074` | MedievalGallantmon | 38/54 | Alliance/Vortex, suspend-count deletion |
| `BT20-085` | Shoto Kazama | 35/54 | self-return, free Shoto/Lv3 play, end-turn suspend/DP |
| `BT24-047` | Kokatorimon | implemented Track D slice | suspend-result branch + result-bound may-attack covered; inherited battle-delete memory remains card-body follow-up |
| `EX7-034` | GrandGalemon | 32/54 | Vortex, suspend/self-immunity |
| `BT24-044` | Muchomon | 31/54 | suspend-then-search |
| `EX7-032` | Galemon | 31/54 | Shoto free-play |
| `EX11-035` | Zephagamon | 27/54 | Vortex, suspend-trigger play formula |
| `EX11-062` | Shoto Kazama | 27/54 | memory setter, suspend observer, Vortex-to-player aura |
| `EX11-074` | Vortexdramon | 27/54 | level 7 payoff, effect battle, immunity |
| `EX11-072` | Unique Emblem: Guardian Vortex | 26/54 | option placement, Delay, effect digivolve |
| `ST18-12` | Zephagamon | 24/54 | Vortex, suspend/unsuspend, protection on unsuspend |
| `EX11-026` | Pteromon | 21/54 | moving/on-play suspend and DP grant |
| `BT3-103` | Hidden Potential Discovered! | 19/54 | one-shot digivolve cost hook tech |
| `EX7-036` | Zephagamon | 18/54 | Vortex, suspend, bottom-deck payoff |
| `ST18-14` | Shoto Kazama | 27/54 | attack-target change |

## Current Implementation Evidence

- Production YAML exists only for `EX7-074`, `EX8-074`, and `EX11-074` among the Zephagamon-adjacent cards checked for this pass.
- `EX11-074` is explicitly a readiness slice, not a full implementation. `code/digimon-engine/cards/ex11/EX11-074.yaml` covers static `<Piercing>`, `<Vortex>`, `<Blocker>`, and a focused `battle:` path.
- `code/digimon-engine/tests/cards_behavioral/ex11/ex11_074.rs` proves the `battle:` rule boundary: effect battle deletes through DP battle, does not trigger Piercing security checks, and does not leave `pending_attack`.
- `EX7-074` and `EX8-074` have YAML plus behavioral tests, but their card files still document ignored or partial cases around Option/security integration, formula/selection details, and triggered once-per-turn behavior.
- Most Zephagamon core cards are metadata-only for Rust execution today: `EX7-064`, `P-166`, `ST18-04`, `BT20-101`, `EX7-031`, `ST22-13`, `BT20-085`, `EX11-035`, `EX11-062`, `EX11-072`, `ST18-12`, and others have no production YAML under `code/digimon-engine/cards/`. `BT24-047` now has production YAML and focused Track D coverage for its suspend-result may-attack branch.

## Gap Summary

| Gap ID | Type | Status | Blocks | Canonical tracker |
|---|---|---|---|---|
| `ZEPH-G001` | dsl-gap / test-gap | open | Most Zephagamon core cards | none; archetype-local authoring backlog |
| `ZEPH-G002` | dsl-gap | open | `EX11-074`, `BT20-101`, `EX7-036`, `ST18-12`, `EX11-035` | `qa/dsl-vocab-gaps.md` |
| `ZEPH-G003` | dsl-gap / engine-gap | open | `BT20-101`, `EX7-036`, `EX8-074`, `EX11-035` | `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md` |
| `ZEPH-G004` | dsl-gap | open | `EX11-062` | `qa/dsl-vocab-gaps.md` |
| `ZEPH-G005` | engine-gap / dsl-gap | partially resolved | `ST18-14` and `BT24-047` covered; related may-attack cards remain card-authored as encountered | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `ZEPH-G006` | engine-gap / dsl-gap | partially resolved | `BT20-101`, other ACE/Counter cards | `docs/DCGO_KEYWORD_PARITY.md`, `docs/RUST_ENGINE_GAPS.md` |
| `ZEPH-G007` | engine-gap / dsl-gap / test-gap | open | `EX11-072`, `LM-030`, `P-106`, `P-038`, `EX7-074` | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `ZEPH-G008` | engine-gap / dsl-gap | open | `EX7-064`, `BT20-085`, `P-133`, `EX11-062`, `EX11-028` | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `ZEPH-G009` | engine-gap / dsl-gap | open | `BT3-103`, `P-166`, `P-106`, `EX11-072` | `docs/RUST_ENGINE_GAPS.md` |
| `ZEPH-G010` | dsl-gap / test-gap | open | `ST22-13`, `ST18-12`, `EX7-034`, `EX11-074` | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |

## Detailed Gaps

### ZEPH-G001: Production YAML Missing for the Core Vortex Warriors Package

- **Type:** `dsl-gap` / `test-gap`
- **Status:** narrowed 2026-05-10 — runtime controller landed; DSL predicate-to-modifier lowering remains open.
- **Blocks:** `EX7-064`, `P-166`, `ST18-04`, `BT20-101`, `EX7-031`, `ST22-13`, `BT20-085`, `BT24-047`, `EX7-034`, `BT24-044`, `EX7-032`, `EX11-028`, `EX11-035`, `EX11-062`, `EX11-072`, `ST18-12`, `EX11-026`, `EX7-036`, and most starter/promotional line pieces.
- **Why it matters:** The Rust runtime only executes production YAML or registered Rust effects. A deck can appear in metadata and legacy QA while still being unusable as a faithful Rust training/evaluation target.
- **Evidence:** Only `EX7-074`, `EX8-074`, and `EX11-074` have Zephagamon-adjacent production YAML. Only those three have matching card-level behavioral test files.
- **First test:** Add a structural registration test for `BT20-101` that asserts compiled metadata includes ACE Overflow, `<Blast Digivolve>`, `<Piercing>`, `<Vortex>`, `<Blocker>`, and the two printed triggered clauses before implementing behavior.
- **Implementation hint:** Start with small production slices only when the reusable primitives below exist. Keep omitted text explicit in YAML comments and tests.

### ZEPH-G002: Result-Bound Branches for "If This Effect Suspended Your Digimon"

- **Type:** `dsl-gap`
- **Status:** partially resolved
- **Blocks:** `EX11-074`, `EX7-034`, `BT24-044`, `ST18-10`, `BT20-085`, `EX11-026`, `EX11-035`, `ST18-12`, `BT20-101`. `BT24-047` is covered for the unsuspended-target branch.
- **Effect text examples:** "If this effect suspended your Digimon..." and "If this effect suspended your Digimon, this Digimon isn't affected..." / "return 1 of your opponent's suspended Digimon..."
- **Why it matters:** The DSL can select and suspend, but Zephagamon cards repeatedly branch based on whether the suspend step actually suspended a friendly Digimon. This cannot be modeled by blindly executing the tail, because opponent/self target choice changes legal outcomes.
- **Evidence:** `binding_owner: { binding, of }` now supports the owner branch used by BT24-047 after an optional `select_any_permanent` choice. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- binding_owner_predicate_matches_bound_permanent_controller` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_047`. `EX11-074` and related cards still need richer mutation-result payloads where protection/already-suspended cases must be distinguished.
- **First test:** For `EX11-074`, resolve the When Digivolving/When Attacking effect twice: once choosing an own Digimon and once choosing an opponent Digimon. Only the own-Digimon suspension should grant +6000 DP and opponent-effect immunity.
- **Implementation hint:** For BT24-047-style branches, bind the selected target and use `binding_owner`; for stricter "this mutation actually changed state" cards, add a DSL result binding such as `suspend: { target: picked, bind_result_as: suspended_by_this_effect }`, then allow `if` conditions to test that binding and target controller.

### ZEPH-G003: Suspended-Count and Formula-Driven Multi-Selection

- **Type:** `dsl-gap` / `engine-gap`
- **Status:** open
- **Blocks:** `BT20-101`, `EX7-036`, `EX8-074`, `EX11-035`, and related suspend-count cards.
- **Effect text examples:** `BT20-101`: "for every 2 suspended Digimon, you may return 1 of your opponent's suspended Digimon to the bottom of the deck." `EX11-035`: "For each suspended Digimon, add 2000 to this effect's DP maximum."
- **Why it matters:** These effects need live board formulas, floor division, and formula results that drive either target count or target-card DP ceilings. A static target count or fixed DP predicate hides legal choices.
- **Evidence:** `qa/dsl-vocab-gaps.md` lists the `BT20-101` suspended-count / divide-by-2 / count-capped bottom-deck formula and `EX11-035` formula DP cap as open Zephagamon gaps. `EX8-074` currently carries raw formula and ignored-test comments for dynamic DP cap behavior.
- **First test:** Put four suspended Digimon across both battle areas, resolve `BT20-101`, and assert exactly two opponent suspended Digimon can be selected for bottom-decking, with PASS only legal after the minimum/optional selection rule is satisfied.
- **Implementation hint:** Reuse existing formula primitives (`floor_div`, aggregate scopes, count-capped selection), but add a suspended-permanent count selector and a way to bind a formula result into `select_count_capped_multi` over opponent battle-area permanents.

### ZEPH-G004: Conditional `VortexCanAttackPlayer` Aura

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `EX11-062` Shoto Kazama.
- **Effect text:** "[Your Turn] While your opponent has no unsuspended Digimon, your <Vortex> can also attack players."
- **Why it matters:** The engine has `ModifierType::VortexCanAttackPlayer`, Vortex masks already distinguish Digimon targets from player/security targets, and `Expiry::UntilCondition` can now evict condition-gated modifiers after board mutations. The missing piece is DSL aura lowering that grants the modifier with an attached BoolPredicate while the opponent has no unsuspended Digimon.
- **Evidence:** `qa/dsl-vocab-gaps.md` records this as a Zephagamon gap. `code/digimon-engine/tests/mask_and_tensor/mask_end_of_turn_parity.rs` already proves base Vortex does not attack players unless `VortexCanAttackPlayer` is present.
- **First test:** With `EX11-062` in battle, a Vortex Digimon, and no unsuspended opponent Digimon, the EndOfTurnAction mask should include the security/player target. Adding an unsuspended opponent Digimon should remove that target while retaining legal Digimon-target Vortex attacks.
- **Implementation hint:** Add aura/flood-gate syntax that grants `VortexCanAttackPlayer` to matching own Vortex Digimon with `expiry: until_condition` and an attached `until_condition` predicate checking opponent unsuspended Digimon count equals zero.

### ZEPH-G005: Attack Target Change and Immediate May-Attack Flow

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** partially resolved by Track D. The shared `redirect_attack_target` and `may_attack_now` primitives exist; ST18-14 has a prompted retarget card-shaped fixture, and BT24-047 now has production YAML/tests for the suspend-result may-attack branch using `binding_owner`.
- **Blocks:** Other Zephagamon cards that say a Digimon may attack as part of effect resolution remain card-authoring/verification work as encountered. `ST18-14` is no longer blocked by the retarget prompt primitive, and `BT24-047` is no longer blocked by the result-bound may-attack slice.
- **Effect text examples:** `ST18-14`: "When one of your Digimon attacks your opponent's Digimon, by suspending this Tamer, you may change the attack target to another of your opponent's Digimon or the player." `BT24-047`: suspend/search tail that lets a selected Digimon attack.
- **Why it matters:** These are player-visible combat decisions. Modeling target change as up-front attack targeting, or omitting a "may attack" tail, changes the action surface.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` tracks remaining card-specific Zephagamon work. Shared primitives are covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- may_attack_now`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- binding_owner_predicate_matches_bound_permanent_controller`.
- **First test:** Implemented for `ST18-14`: attack an opponent Digimon while Shoto is unsuspended; the engine offers the Tamer suspend cost, then a target-redirect selection that includes another opponent Digimon and the player while excluding the current target.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- redirect_attack_target_prompt_yaml_lowers_to_compiled_step`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st18_14`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt24_047`.
- **Implementation hint:** Remaining adjacent work is richer mutation-result payloads for protected/already-suspended edge cases; BT24-047 is covered by filtering the initial suspend choice to unsuspended Digimon and testing the selected permanent's owner via `binding_owner`.

### ZEPH-G006: Native `<Blast Digivolve>` Auto-Install for Printed ACE Cards

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** partially resolved
- **Blocks:** `BT20-101` Zephagamon and other printed ACE/Counter cards when they are represented only by card metadata.
- **Why it matters:** Rust has Counter-window substrate and `EffectBuilder::blast_digivolve()`, but printed keyword parsing alone does not currently make a metadata-only hand card a Counter candidate. Zephagamon ACE needs to appear in the defender's action mask during Counter timing without hand-authored hidden logic.
- **Evidence:** `docs/DCGO_KEYWORD_PARITY.md` now records native `<Blast Digivolve>` auto-install via `Keyword::BlastDigivolve`, and the Counter substrate is live. `BT20-101` still needs production YAML/card-shaped tests for its other printed clauses and ACE Overflow behavior.
- **First test:** During an opponent attack, put `BT20-101` in defender hand with a legal level 5/6 target in battle. The CounterTiming mask must expose the Blast Digivolve candidate, execute the free digivolve, fire When Digivolving, and preserve attack-state continuation.
- **Implementation update (2026-05-08):** `Keyword::BlastDigivolve` now auto-installs the same `.blast_digivolve()` Counter marker from `CardData`, so metadata-only hand cards become Counter candidates when they have a legal ordinary digivolution route. Focused coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- native_blast_digivolve_keyword_installs_counter_candidate`. `BT20-101` still needs production YAML/card-shaped tests for its other printed clauses and ACE Overflow behavior.

### ZEPH-G007: Option, Delay, Search, and Security Disposition Completeness

- **Type:** `engine-gap` / `dsl-gap` / `test-gap`
- **Status:** open
- **Blocks:** `EX11-072`, `EX7-074`, `LM-030`, `P-106`, `P-038`, and other search/options used by the deck.
- **Why it matters:** Zephagamon lists heavily use Training/Scramble/Memory Boost style options and `Unique Emblem: Guardian Vortex`. These effects combine reveal selection, optional effect digivolve, placing an option in battle, event-gated Delay, security activation, and "add this card to hand" dispositions.
- **Evidence:** `EX7-074` YAML exists, but its behavioral test file still contains ignored cases for main-from-hand option integration and security integration. `qa/dsl-vocab-gaps.md` separately tracks self-to-hand security disposition and event-gated Delay shapes, though some narrow slices have since been resolved.
- **First test:** Resolve `EX11-072` from hand: choose `Pteromon`, `Muchomon`, or `Shoto Kazama` from hand/trash to play, place the option in battle, then on a later Shoto suspend expose the Delay digivolve choice with cost reduced by 3.
- **Implementation hint:** Reuse Group 5 Delay/option infrastructure, but add Zephagamon card-level tests proving the exact event predicate, delay placement-turn gate, hand/trash zone choice, and effect-digivolve target filters.

### ZEPH-G008: Event Context Fan-Out for Tamers, Hand/Trash Plays, and Suspend Observers

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `EX7-064`, `BT20-085`, `P-133`, `EX11-062`, `EX11-028`, `EX11-072`, `ST18-14`.
- **Effect text examples:** "When any Digimon suspend...", "When any of your [Shoto Kazama]s suspend...", "When one of your Digimon digivolves into...", "If effects suspended those Digimon..."
- **Why it matters:** Zephagamon is built around suspend events. The trigger context must identify the suspended permanent, its controller, whether an effect suspended it, and the source card that caused the event. Without that payload, filters either fire too broadly or not at all.
- **Evidence:** `qa/dsl-vocab-gaps.md` calls out Zephagamon result bindings and `EX11-062` conditional aura. `docs/RUST_ENGINE_GAPS.md` notes follow-up event paths remain open unless separately tested for effect-created plays and effect-initiated digivolve.
- **First test:** With `EX11-062` and an own Digimon in battle, resolve a card effect that suspends an own Digimon. The Shoto trigger should be optional, require suspending Shoto as cost, draw 1 only if effects suspended the Digimon, and apply +3000 DP to an eligible Avian/Bird/Vortex Warriors Digimon.
- **Implementation hint:** Extend suspend/unsuspend event context with `by_effect`, `event_permanent`, `event_card`, and `source_player`, then expose those predicates in DSL `active_when`.

### ZEPH-G009: One-Shot and Effect-Scoped Digivolve Cost Hooks

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT3-103`, `P-166`, `P-106`, `EX11-072`, `EX7-074`, and similar reduced-cost digivolve effects.
- **Why it matters:** Zephagamon uses both generic cost reducers and effect-scoped cost reductions. A global or lingering cost modifier can leak to unrelated digivolutions, while no reduction makes the card unplayable.
- **Evidence:** Legacy `qa/archetype-qa/Zephaga.md` marks `BT3-103` as deferred for a one-shot digivolve cost hook. `docs/RUST_ENGINE_GAPS.md` says `effect_initiated_digivolve` exists, but passive cost-reduction hooks, prompt-scoped reductions, and some specialized variants remain open sub-items.
- **First test:** Resolve `P-166` after three other Digimon are suspended. It should offer an effect-initiated digivolve from hand with cost reduced by the printed count only for that effect activation, then clear the reduction.
- **Implementation hint:** Prefer effect-scoped `CostDelta` on the digivolve primitive over installing broad `BeforePayCost` modifiers unless the printed text is genuinely passive.

### ZEPH-G010: Protection, Immunity, Fortitude, and Modifier Enforcement

- **Type:** `engine-gap` / `dsl-gap` / `test-gap`
- **Status:** open
- **Blocks:** `ST22-13`, `ST18-12`, `EX7-034`, `EX11-074`, `BT12-057`, `BT14-044`.
- **Effect text examples:** "isn't affected by your opponent's Digimon's effects", `<Fortitude>`, "all of your opponent's Digimon can't unsuspend", and "grant triggered effect to opponent's permanents."
- **Why it matters:** These cards depend on modifiers being installed with correct scope and enforced in mutation paths, not just represented in compiled YAML. Cross-archetype specs should separate "modifier exists" from "all relevant engine operations consult it."
- **Evidence:** `qa/archetype-qa/Zephaga.md` deferred `BT12-057` and `BT14-044` for modifier/trigger-grant issues in the legacy lane. `EX11-074` readiness YAML deliberately omits the full immunity clause because result-bound branch support is missing.
- **First test:** Resolve `EX7-034` by suspending an own Digimon, then attempt to affect that GrandGalemon with an opponent Digimon effect before the opponent's turn ends. The effect must fail while same-side effects still behave normally.
- **Implementation hint:** Add card-level modifier enforcement tests after `ZEPH-G002` can express the branch that grants the modifier.

## Resolved or Partially Resolved Reusable Primitives To Avoid Reopening

| Primitive | Current note | Remaining Zephagamon work |
|---|---|---|
| Base Vortex end-of-turn attack mask | Covered by combat/mask tests; base Vortex attacks opponent Digimon, not players. | Use it in production YAML once card-level clauses are authored. |
| Effect battle via `battle:` | Proven by `EX11-074` readiness slice. | Full `EX11-074` still needs unsuspend branch, immunity, and suspend-result binding. |
| ACE Overflow engine application | Engine/card-data support exists and card-level tests exist for other ACE cards. | `BT20-101` production YAML/metadata and card-level movement tests still needed. |
| `play_cost_lte` predicates | Tracker says the reusable predicate is resolved for hand/trash selections. | Update stale `EX7-074` comments/tests when card integration is revisited. |
| `add_this_option_to_hand` security disposition | Narrow pending-security disposition slice is resolved. | Revisit `EX7-074` and option tech with current syntax rather than old raw-Rust placeholders. |
| `effect_initiated_digivolve` source-parametric primitive | Group 4 added source-parametric support. | Zephagamon still needs effect-scoped costs, filters, event context, and card-level coverage. |

## Card-Local YAML/Test Backlog

These items should remain card migration work unless they expose a new reusable primitive:

| Card | Needed next step |
|---|---|
| `BT20-101` Zephagamon | Author production YAML after Blast auto-install and suspended-count bottom-deck selection are proven. |
| `EX7-064` Shoto Kazama | Test start-main memory and EOT suspend-as-cost keyword grant/unsuspend targeting. |
| `P-166` Galemon | Test optional suspend and effect-scoped digivolve cost reduction from suspended count. |
| `ST18-04` Pteromon | Author reveal search and inherited DP/memory coverage. |
| `ST22-13` GrandGalemon | Test Fortitude, Vortex, suspend/DP, and inherited Vortex Warriors unsuspend condition. |
| `EX11-035` Zephagamon | Test unsuspend-then-suspend and formula DP cap play from hand. |
| `EX11-062` Shoto Kazama | Test memory setter, effect-suspend observer, and conditional Vortex-to-player aura. |
| `EX11-072` Unique Emblem: Guardian Vortex | Test option play from hand/trash, battle-area placement, event-gated Delay, and reduced-cost digivolve. |
| `ST18-14` Shoto Kazama | Implemented 2026-05-08; production YAML and 3 focused redirect tests pass. |
| `EX11-074` Vortexdramon | Expand beyond readiness slice to full suspend-result immunity and unsuspend-then-battle flow. |

## Cross-Archetype Spec Tags

Use these tags when normalizing Zephagamon gaps with other archetype gap source documents:

- `missing-production-yaml`
- `suspend-result-binding`
- `effect-result-conditional`
- `suspended-count-formula`
- `formula-driven-count-select`
- `bottom-deck-selected-permanents`
- `conditional-vortex-player-aura`
- `attack-target-redirect`
- `may-attack-now`
- `blast-digivolve-auto-install`
- `option-delay-disposition`
- `effect-initiated-event-context`
- `effect-scoped-cost-reduction`
- `modifier-enforcement`

## Suggested Spec Grouping

For a future cross-archetype gap spec, group remaining work by reusable capability rather than by Zephagamon card:

1. Effect result bindings: suspend-result, play-result, and "by this effect" predicates.
2. Formula-backed selections: suspended-count formulas, count-capped battle-area target selection, and formula DP caps.
3. Combat decision surfaces: attack-target redirect, immediate may-attack, and Vortex player-target extension.
4. Counter/ACE bridge: native `<Blast Digivolve>` auto-install plus ACE Overflow card metadata coverage.
5. Option and Delay integration: hand/trash zone choice, option placement, security disposition, event-gated Delay.
6. Event context fan-out: suspend/unsuspend observers, effect-created plays, effect-initiated digivolve, Tamer costs.
7. Cost hooks: one-shot and prompt-scoped digivolve reductions.
8. Modifier enforcement: opponent-effect immunity, Fortitude/protection, CannotUnsuspend, and cross-side trigger grants.
9. Production YAML pass: migrate Zephagamon core only after the relevant primitives have failing tests and native lowering.

## Spec Input Checklist

A future implementation spec should require:

- one failing Rust behavioral test for each reusable primitive before implementation;
- one DSL parser/lowering test for every new YAML vocabulary shape;
- action-mask or `PendingSelection` assertions for every player-visible choice, including optional one-card choices;
- no `ACTION_SPACE_SIZE` or active tensor contract changes unless `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, RL wrappers, frontend constants, and model metadata are updated together;
- tracker updates in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and this document when a gap closes, splits, or is downgraded to card-local authoring.

## Suggested First Slice

Start with `BT20-101` if the goal is maximum Zephagamon identity: it forces Blast Digivolve, ACE Overflow, base Vortex, suspend-result branching, and suspended-count bottom-deck selection.

Start with `EX11-062` if the goal is maximum action-mask reuse: it isolates the conditional `VortexCanAttackPlayer` aura and proves Vortex-to-player targeting without also needing ACE or bottom-deck formulas.

For a cross-archetype gap spec, the best first anchor is the `suspend-result-binding` primitive from `EX11-074` or `EX7-034`, because it unlocks several Zephagamon clauses and gives a clean, narrow acceptance test before the broader formula-selection work.
