# Archetype DSL Implementation: EX13 — Dracomon / dragon
Date: 2026-10-01
Total cards in pool: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl (author-set EX13 slice)

## Summary
- IMPLEMENTED: 4
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 2
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-008 | Dracomon | IMPLEMENT | IMPLEMENTED | self-reviewed | 12/12 | [When Moving][On Play] reveal-3 text pick, rest to bottom; [Bebydomon] alt; inherited EoT may-DNA |
| EX13-018 | Coredramon | IMPLEMENT | IMPLEMENTED | self-reviewed | 15/15 | trash-text-card → Draw 2; [Your Turn] ally-played → digivolve into [Examon]-text −2; inherited +2000 |
| EX13-039 | Coredramon | IMPLEMENT | IMPLEMENTED | self-reviewed | 15/15 | may return non-Egg text card from trash; same observer; inherited +2000 |
| EX13-021 | Wingdramon | IMPLEMENT | IMPLEMENTED (2026-10-01) | — | 18/18 | "treated as Lv.6 [Slayerdramon] for [Examon]'s DNA" via `dna_material_identity` |
| EX13-041 | Groundramon | IMPLEMENT | IMPLEMENTED (2026-10-01) | — | 18/18 | `dna_material_identity`; `CannotUnsuspendInUnsuspendPhase`; `event_battle_deleter` |
| EX13-044 | Breakdramon | IMPLEMENT | IMPLEMENTED | self-reviewed | 20/20 | Assembly -5 (3 per-level text materials); up-to-2 any-side suspend + 2-target CannotUnsuspend; face+inherited OPT suspend→battle |

## Engine-Gap Blocked Cards
### EX13-021 Wingdramon / EX13-041 Groundramon (hybrid) — RESOLVED 2026-10-01 (see engine-gaps.md)
- Effect text: "[All Turns] This Digimon is also treated as Lv.6 [Slayerdramon]/[Breakdramon] for [Examon]'s DNA digivolution."
- Missing engine API: result-scoped DNA-material identity override (DCGO `AddJogressLevelsClass` + scoped `ChangeCardNamesClass`); all DNA / Blast-DNA material matchers read printed level/name only.
- Suggested addition: `Game::dna_material_identity(material, result)` consulted by every DNA material predicate. See `qa/archetype-qa/engine-gaps.md` §G-DNA-MATERIAL-TREATED-AS-FOR-TARGET.

## DSL-Vocab-Gap Blocked Cards
### EX13-021 / EX13-041 (hybrid — same gap)
- Missing DSL verb: `kind: dna_material_identity { for_result, treated_as: { level, name } }`.
- Lowers to engine API: none yet (see above).
- Suggested DSL syntax: see `qa/dsl-vocab-gaps.md` §G-DNA-MATERIAL-TREATED-AS-FOR-TARGET.

## New Patterns Discovered
- `select_any_permanent` silently ignores `continue_on_decline` (G-SELECT-ANY-PERMANENT-CONTINUE-ON-DECLINE): "you may [act on] up to N Digimon (either side). Then, <mandatory>" needs effect-choice gates + mandatory picks. Logged in `qa/dsl-vocab-gaps.md`.
- Test-env note: DSL-loaded cards carry empty `effect_text` in DebugRunner, so "[X] in its text" tests must use synthetic cards with text (or a matching name).
- A digivolve onto a base that satisfies two routes (colour circle + named alt) installs a route-choice prompt; per-route cost tests need a base that matches exactly one route.
