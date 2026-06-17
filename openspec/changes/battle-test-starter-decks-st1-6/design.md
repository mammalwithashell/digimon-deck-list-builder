## Context

ST-1 … ST-6 are the six original color starter decks (Gaia Red, Cocytus Blue, Heaven's Yellow, Giga Green, Machine Black, Venomous Violet). All 96 unique cards (`st1`–`st6`, 16 each) are authored as Rust DSL YAML specs under `code/digimon-engine/cards/st{1..6}/`, with JSON metadata siblings. Each deck has an existing per-deck coverage spec under `openspec/specs/` and an archetype test file `code/digimon-engine/tests/archetypes/st{N}.rs` (6–10 fns each).

Current state established during brainstorming:
- All 96 cards have YAML + JSON. All 96 carry a `validated_cards_dsl.json` entry from report `audit-starter-decks-st1-6` (2026-05-29) marked `AUDITED-OK` — but every note is the identical string "Faithful to printed text + DCGO.", a smell that the audit was templated/shallow.
- Per-card behavioral coverage is uneven: `st4` has **zero** per-card test files despite its spec requiring full coverage; `st1`/`st5` have several; `st2`/`st3`/`st6` have a handful.
- The six lists are in `data/deck_library.json` as `starter_st{1..6}_*` (format `"starter"`, 54 cards each: 4 Digitama + 50 main).

Constraints:
- **No-approximations policy** (CLAUDE.md rule 17): every printed clause faithfully implemented; every choice exposed via `pending_selection`; no stubs/auto-selection.
- **Source priority** (CLAUDE.md): card image (printed text) → DCGO C# (`$BASE_DCGO`, behavior) → `general_rule.pdf` (rules/timing) → fandom; `cards.json` is API-ingested and lowest trust.
- **DSL-first / rule 28**: widen DSL vocab or engine primitives rather than routing around a missing capability; log to `qa/dsl-vocab-gaps.md` / `docs/RUST_ENGINE_GAPS.md`.
- **Rule 31**: this may be a pre-restart session, so cargo commands must use an explicit per-worktree `CARGO_TARGET_DIR` to avoid shared-target contamination.
- **Worktree write-pinning** (memory): authoring sub-agents must write relative to their worktree cwd, never `cd` into the base repo; `$BASE_DCGO` is read-only.

## Goals / Non-Goals

**Goals:**
- Re-derive a trustworthy faithfulness verdict for each of the 96 cards against authoritative sources, replacing the templated 2026-05-29 verdicts.
- Correct every faithfulness bug found, TDD-first, widening the substrate where needed.
- Achieve behavioral-test coverage for every non-vanilla effect, and the four static archetype tests + key interaction tests per deck.
- Demonstrate, via `digimon-scenario-mcp` staged scenarios and full played games (mirror + cross-matchup), that each deck resolves effects faithfully, exposes every choice, and runs full games without panics / soft-locks / mask violations.
- Confirm the six lists drive `DigimonEnv` reset/step and resolve through deck-pool/archetype wiring, so training can launch.

**Non-Goals:**
- No new gameplay features or new cards; this is verification + correction + hardening of existing implementations.
- No launching of a full/cloud RL training run (only the readiness check and, optionally, a tiny local smoke step).
- No changes to the tensor/action spec or training algorithms.
- No reworking of decks not in ST-1…ST-6.

## Decisions

### D1 — Treat the 2026-05-29 verdicts as untrusted; re-derive every card
The identical templated notes provide no evidence of per-card scrutiny. We re-audit all 96 from the card image down, rather than spot-checking, because a shallow prior pass gives false confidence precisely where bugs hide. *Alternative considered:* trust prior `AUDITED-OK` and only spot-check — rejected; the user explicitly asked to review the effects and wants real confidence.

### D2 — Parallel Opus sub-agents, one per deck, for the audit (Phase 1)
Six decks are independent; fan out one Opus sub-agent per deck (16 cards each), each emitting a structured per-card verdict (`OK` / `BUG{detail,source}` / `GAP{detail}`). Audit-only: sub-agents read the card image (`/digimon-card-lookup`), DCGO C# (`$BASE_DCGO`, read-only), `general_rule.pdf`, fandom, and the YAML, and *propose* fixes; the orchestrator applies them so changes are serialized and TDD-gated. *Alternative:* one mega-agent over 96 cards — rejected (context dilution, weaker per-card rigor). *Alternative:* let sub-agents edit YAML directly — rejected (parallel writes to the shared cards dir + the card-pack build is a global gate; serializing fixes through the orchestrator avoids the concurrent-worktree hazards in memory).

### D3 — Verification is MCP-driven, with two complementary modes (Phase 4)
The user's chosen confidence mechanism is playing games + staging scenarios with the real DSL cards.
- **Microscope** — `digimon-scenario-mcp` (browser target): `stage_scenario` a precise board for a tricky effect/combo → `step` to the decision point → `get_pending_selection`/`get_mask` to confirm the choice is exposed (no auto-select) → `evaluate` engine assertions on the outcome → `save_fixture` to `qa/scenarios/`. Browser target chosen because `evaluate` is browser-only and it needs no desktop build.
- **Volume** — full games per deck (mirror + the 15 cross-matchups): drive complete games to catch panics, soft-locks, and illegal-mask states that only emerge in real play. Use a headless self-play/greedy loop for throughput; use `digimon-engine-mcp` recording forensics to localize any crash.
*Alternative:* desktop target — rejected for this pass (needs a Tauri `debug-bridge` build and can't be driven headlessly), though it remains the only way to exercise the desktop DTO wire and can be a follow-up.

### D4 — Browser target requires PyO3 bindings + uvicorn (Phase 0)
`digimon-scenario-mcp` browser target talks to the hosted-API `/debug` surface, which drives the Rust engine through `digimon_engine` (PyO3). So Phase 0 runs `maturin develop` in `code/digimon-engine-py` and starts `python -m uvicorn server.api:app`. The MCP is launched with `--browser-url http://127.0.0.1:8000`.

### D5 — Tests: per-card behavioral + four static + interaction, TDD
Per-card behavioral tests go in `code/digimon-engine/tests/cards_behavioral/st{N}/` (DebugRunner). The four static archetype tests (deck-legality, coverage gate, smoke games, combo-presence) come from the `archetype-static-tests` crate; interaction tests live in `code/digimon-engine/tests/archetypes/st{N}.rs`. Where a deck's archetype/interaction coverage is thin, drive it with `/archetype-interaction-test-author`. Bug fixes are TDD: failing behavioral test first, then fix. *Alternative:* MCP-only verification without durable Rust tests — rejected; MCP confidence is not regression-proof, and the existing specs already require behavioral coverage.

### D6 — Fix policy follows rule 28
When a card can't be expressed, widen DSL vocabulary (lower in `code/digimon-dsl/`, log `qa/dsl-vocab-gaps.md`) or add an engine primitive (log `docs/RUST_ENGINE_GAPS.md`); hand-written `raw_rust` is last resort. This keeps the compounding-coverage flywheel intact rather than special-casing starter cards.

### D7 — Verdict tracking + report
Re-derived verdicts overwrite the templated entries in `qa/qa-reports/validated_cards_dsl.json` under a new report id (e.g. `battle-test-starter-decks-st1-6`), with honest per-card notes. A human-readable summary lands under `qa/` (battle-test report) recording: cards audited, bugs found+fixed, tests added, scenarios staged, full-game counts/results per deck, and the final go/no-go for training.

## Risks / Trade-offs

- **Scale (96 cards) inflates effort** → Parallelize the audit by deck; batch fixes; reserve deep MCP scenario work for cards the audit flags as non-trivial, while still smoke-playing every deck.
- **Build/runtime friction (maturin, uvicorn, rule-31 target dir)** → Phase 0 is a hard gate: stand up bindings + server + an isolated `CARGO_TARGET_DIR` and confirm a baseline test run before audit/fix work, so environment failures don't masquerade as card bugs.
- **Shared-worktree / concurrent-write hazards** (memory: stash loss, concurrent agents, base-repo write drift) → Serialize all writes through the orchestrator; sub-agents are read-only auditors; stage/commit early; never stash in the shared worktree.
- **An audit finding may require widening DSL/engine (rule 28), which is larger than a YAML tweak** → Triage by blast radius: if a primitive gap blocks a card and is out of scope to build now, mark the card BLOCKED, log the gap, and exclude it from the training-ready claim rather than stubbing it.
- **MCP confidence is sampling, not proof** → Pair every MCP-confirmed behavior with a durable Rust test so the guarantee survives as a regression gate; record exactly which decks/matchups were played and how many games, with no silent caps.
- **Browser DTO ≠ desktop DTO** (memory) → This pass verifies engine + browser wire; desktop-DTO verification is explicitly a possible follow-up, noted so the go/no-go isn't overclaimed.

## Migration Plan

Execution order (each phase gates the next):
1. **Phase 0** — bindings + uvicorn up; isolated `CARGO_TARGET_DIR`; baseline run of `st1–6` `cards_behavioral` + `archetypes` tests; record red/green + coverage gaps.
2. **Phase 1** — parallel per-deck audit → consolidated bug/gap list + re-derived draft verdicts.
3. **Phase 2** — TDD fixes (YAML / substrate), gaps logged.
4. **Phase 3** — fill behavioral + static + interaction test gaps; all green.
5. **Phase 4** — MCP scenario microscope + full-game volume per deck; iterate fixes; save fixtures.
6. **Phase 5** — deck-pool/archetype wiring + `DigimonEnv` reset/step check.
7. Finalize verdicts, report, go/no-go.

Rollback: all changes are additive tests + card-spec corrections + QA artifacts on the change branch; reverting the branch restores prior state. No data migrations, no schema changes, no production deploy.

## Open Questions

- Will any ST-1…ST-6 card surface a genuine DSL/engine primitive gap (rule 28) large enough to defer? Resolved during Phase 1; deferred cards are marked BLOCKED and excluded from the training-ready set rather than stubbed.
- Does the user want a tiny local smoke-train at the end, or stop at the wiring + `DigimonEnv` reset/step check? Default: stop at the readiness check (per the brainstorming answer favoring MCP games over a smoke train); revisit if requested.
