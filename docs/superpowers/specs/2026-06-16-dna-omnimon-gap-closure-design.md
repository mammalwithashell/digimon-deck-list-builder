# DNA Omnimon engine/DSL gap closure — design

- **Date:** 2026-06-16
- **Branch:** `claude/infallible-goldwasser-83f65c`
- **Origin:** Brainstorm following the 2026-06-14/16 DNA Omnimon family faithfulness audit. Scope was revalidated against `origin/main` (merged 2026-06-16), which had already closed 2 of the original 4 gaps.

## Context

The faithfulness audit (`qa/qa-reports/2026-06-14-archetype-faithfulness-audit.md`) found DNA Omnimon combos A–D faithful, combo **E BLOCKED**, and combo **B reliant on scaffolding**; Omnimon ACE combo **1 BLOCKED**. The blockers traced to two live engine/DSL gaps. This change closes them so the affected DNA Omnimon / Omnimon-family cards can be authored faithfully (CLAUDE.md §17–18 no-approximations).

## Revalidation outcome (post-`origin/main` merge)

Merging 70 commits of `main` closed two of the four originally-scoped gaps:

| Gap | Status | Evidence |
|---|---|---|
| `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` | ✅ RESOLVED upstream (2026-05-17, Phase 2 Track F) | `qa/resolved-gaps.md:905`; no live `#[ignore]` cites it |
| `G-DSL-SOURCE-NAME-CONTAINS` | ✅ RESOLVED upstream | gone from open trackers; evaluated at `dsl_cards/predicate.rs:440` |
| `G-PRED-DP-LTE` | ✅ RESOLVED upstream | BT22 headers updated to "now RESOLVED" |
| `G-ACTIVATED-DIGIVOLVE-EXECUTION` | 🔴 OPEN (this change) | `dna_omnimon.rs:712` still `#[ignore]`'d |
| `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` | 🔴 OPEN (this change) | `omnimon_ace.rs:203,309` still `#[ignore]`'d |

Net: **2 real phases (A, B) + 1 verification pass (C).**

## Goal

Close `G-ACTIVATED-DIGIVOLVE-EXECUTION` and `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH`, and verify the upstream-closed gaps are genuinely exercised for the DNA Omnimon pool. DSL-first and TDD (rules 18/28); no approximations (rule 17). **No `action/space.rs` change anywhere → no DCGO `ActionSpace.cs` regen (rule 27).**

## Phase A — Retire `G-ACTIVATED-DIGIVOLVE-EXECUTION` via DSL re-model (zero engine code)

Three residual cards model an unreachable `kind: activated_digivolve` alt-path (no engine execution route). Re-model each onto existing, working machinery — the same move that unblocked BT24-016 Lamiamon with zero engine code.

### A1 — BT22-013 WarGreymon & BT22-026 MetalGarurumon (`[Hand][Main]` self-target)

Printed: *"[Hand][Main] If you have [Nokia Shiramine], 1 of your [Agumon|Gabumon] digivolves into **this card** for a digivolution cost of 6, ignoring digivolution requirements."*

- **Remove** the `kind: activated_digivolve` alt-path.
- **Add** a `when: main_from_hand` clause (BT24-016 precedent):
  - `condition:` `all_of` → `any_permanent { of: you, zone: [battle_area], name_is: "Nokia Shiramine" }` **AND** `any_permanent { of: you, zone: [battle_area], name_contains: "Agumon"|"Gabumon" }`. This restores the Nokia precondition that `G-ALT-PATH-CONDITION` could not express on an alt-path — a faithfulness *gain*.
  - `process:` `select_own_permanent` (the Agumon/Gabumon) → `effect_initiated_digivolve { target: <picked>, from_hand: self, cost: 6, ignore_requirements: true }`.
- **Keep** the printed standard digivolution alt-path (`kind: digivolve, from: { level_eq: 5 }, cost: 4`).

### A2 — BT16-027 Imperialdramon: Fighter Mode (static add-source)

DCGO models this as `AddSelfDigivolutionRequirementStaticEffect(permanentCondition: TopCard EqualsCardName("Imperialdramon: Dragon Mode"), digivolutionCost: 2)` — an **additional standard digivolution source**, *not* a `[Main]` activated effect. It is currently mis-modelled as `kind: activated_digivolve`.

- **Change** alt-path 2 from `kind: activated_digivolve` → `kind: digivolve` with `from: { name_contains: "Imperialdramon: Dragon Mode" }`, `cost: 2`. (Confirm `from:` name-predicate support on a `digivolve` alt-path during impl — it shares `CompiledPredicate`, so expected to work; fall back to the A1 `main_from_hand` shape only if it doesn't.)

### Tests (write/​un-ignore first)

- Un-`#[ignore]` `dna_omnimon::combo_e_nokia_cost6_lv6_jump` (`dna_omnimon.rs:712`): Nokia + Agumon on field → activate BT22-013 from hand → the Agumon stack is topped by BT22-013 at cost 6 and `[When Digivolving]` fires. Add the **Nokia-absent negative path** (the `main_from_hand` action is masked OUT without Nokia).
- Per-card: the BT22-013/026 activated-digivolve `cards_behavioral` tests, and a BT16-027 add-source digivolve test.

### Stop point
Combo E green; per-card activated-digivolve tests green; gap closed for the 3 cards → entry moved to `qa/resolved-gaps.md`; `validated_cards_dsl.json` updated.

## Phase B — Fix `G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH` (engine)

BT17-095 Miraculous Mega Knight is a **Standard** Option whose `[Main]` body ends with `place_self_as_delay_option`. On the real `Game::play_option_from_hand` lifecycle, `play_option_core` moves the Option from hand/trash into the single-occupancy `pending_option` slot **before** running the body; the place-self step then scans only hand/trash/source, finds nothing, no-ops, and `dispose_option` trashes the (Standard-classified) Option. The printed "place this card in the battle area" is silently dropped on the path the deck actually uses.

### API surface
- `EffectContext::place_self_as_delay_option_permanent` (non-security branch): when `source_permanent`/hand/trash all miss, **claim the card from the in-flight `pending_option` slot** if its card matches `self.source_card`, and seat it as an `OptionState::Delayed` permanent. Mirrors how `dispose_option`'s `OptionSubtype::Delay` arm seats a `kind: delay` Option.
- `dispose_option` (`game_actions.rs`, `play_option_core` step 8): **skip the Standard trash** when the body already re-homed the card (clear `pending_option` / set a "self-placed" flag in the place-self step and check it here).
- Assess the sibling `add_this_option_to_hand` on the non-security on-play path; apply the same `pending_option`-aware claim only if a test requires it (the security path already special-cases `pending_security`).
- **Affected files:** `src/effect_context/` (the place-self helper), `src/game_actions.rs` (`dispose_option` / `play_option_core`). No `action/space.rs` change.

### Tests (un-ignore first)
- Un-`#[ignore]` `omnimon_ace::combo1_mega_knight_free_plays_agumon_from_trash_and_seats_as_delay` and `…_declining_recursion_still_seats_delay` (`omnimon_ace.rs:203,309`): driven through the real `play_option_from_hand`; assert BT17-095 ends as `OptionState::Delayed` (not trash) and trash count is correct.
- After the fix lands, drop the `seat_as_delay_option` scaffold in DNA Omnimon combo B so it runs through the real play path (audit `testIssue`).

### Stop point
Mega Knight seats as a Delay on the real play path; Omnimon ACE combo 1 and DNA Omnimon combo B run through real card effects; gap → `qa/resolved-gaps.md`.

## Phase C — Verify the upstream-closed gaps for the DNA Omnimon pool

`G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` and `G-DSL-SOURCE-NAME-CONTAINS` are resolved upstream, but the BT22-013/026 YAML headers still carry stale "blocked/will exhibit" comments. Verify, don't assume:

- Run the BT22-013/026 branch-0 (Gabumon→MetalGarurumon-in-hand) per-card tests; if any remain `#[ignore]`'d or fail, un-ignore / wire and fix.
- Run the inherited `[Omnimon]`-name negative tests (`source_name_contains`); confirm green (also aided by upstream fix `88b633e9` "[When Attacking] inherited effects firing for non-attacking Digimon").
- Scrub the now-stale "will exhibit … never installs" header comments in `BT22-013.yaml` / `BT22-026.yaml`.

## Cross-cutting requirements
- **TDD:** failing test first in each phase. Build in the per-worktree isolated `CARGO_TARGET_DIR` (memory `reference_cargo_target_per_worktree`). Green `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` and `--test archetypes`; lib parity intact.
- Update gap trackers (`qa/archetype-qa/engine-gaps.md`, `docs/RUST_ENGINE_GAPS.md`) and `qa/qa-reports/validated_cards_dsl.json` as each phase closes; move closed entries to `qa/resolved-gaps.md`.
- **No action-space change** → no DCGO `ActionSpace.cs` regen (rule 27).

## Out of scope
- DNA Omnimon coverage-gate line-connectors (BT14-014, BT15-024, EX9-014) — line context; no combo depends on them.
- Card-authoring of the broader BLOCKED tranche — that is `/batch-implement-cards-rust-dsl`, a separate change once this engine/DSL work lands.
