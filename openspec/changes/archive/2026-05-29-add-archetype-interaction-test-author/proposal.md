## Why

Every existing archetype skill (`/assess-archetype-rust`, `/batch-implement-cards-rust-dsl`, `/review-archetype`, `/implement-archetype`) is **per-card**: it audits, implements, or tests one card in isolation. Nothing builds a system-level understanding of an archetype — its combos, digivolution lines, playstyle, win conditions — and nothing authors **interaction tests** that exercise multiple cards working together as the deck actually plays. The bugs that per-card TDD misses are precisely the cross-card interaction bugs (a triggered ability firing off another card's digivolve, a cost-reduction loop, a security-trigger chain). This change adds a proactive, hypothesis-driven bug-discovery mode: Claude researches an archetype, builds a durable model of how it works, and from that model plans and executes DebugRunner interaction tests plus static archetype-level invariant tests. It is the third mode alongside the two replay modes in `add-interactive-replay-bug-hunter` (differential + judge); all three route confirmed findings to the same trackers.

## What Changes

- **New `/archetype-interaction-test-author` skill** — a capstone in the archetype-QA family that runs *after* cards are implemented and per-card tests are green. Pipeline: **research** (resolve the archetype's card pool via `resolve_deck.py`; read card text + DCGO C# + `general_rule.pdf`) → **model** (emit a durable archetype-model doc: combos, digivolution lines, playstyle, memory curve, win conditions, key interactions) → **plan** (enumerate testable interactions + the static invariants) → **author** (write DebugRunner multi-card interaction tests + the static tests) → **execute + triage** (run; failures become engine-bug findings routed to the shared trackers).
- **Archetype-model artifact** — a reviewable `qa/archetype-qa/<archetype>-model.md` capturing Claude's understanding of the archetype as a system. It is both the research output and the input to test planning, so authored tests are principled, not arbitrary.
- **Interaction-test home** — a new location for multi-card / combo tests (which span sets and cards and have no home today, since `tests/cards_behavioral/<set>/` is strictly per-card), e.g. `code/digimon-engine/tests/archetypes/<archetype_slug>.rs`, with shared multi-card fixture helpers.
- **Static archetype-test harness** — four invariant test types runnable without stepping a full game: (1) **deck legality / construction** (the archetype's meta decklist builds into a legal deck and the engine constructs a game from it), (2) **coverage gate** (all / a threshold of the archetype's cards are implemented and have passing per-card tests), (3) **per-archetype smoke games** (N self-play games on the archetype's deck run to completion without panic / illegal state), (4) **combo-presence assertions** (the specific cards forming the archetype's key combos are all implemented, so interaction tests can be written at all).
- **Verdict tracking** — record interaction-test + static-test outcomes per archetype (e.g. `qa/qa-reports/archetype_interactions.json`) so progress is visible across the campaign.

## Capabilities

### New Capabilities

- `archetype-interaction-test-author`: the `/archetype-interaction-test-author` skill — the research → archetype-model → plan → author → execute → triage workflow, the archetype-model artifact contract, the interaction-test home + fixture conventions, capstone positioning relative to the per-card skills, and findings routing.
- `archetype-static-tests`: the four static archetype-level invariant tests (deck legality / construction, coverage gate, per-archetype smoke games, combo-presence) as a reusable harness with per-archetype verdict tracking, independent of the skill that authors them.

### Modified Capabilities

None. The existing per-card archetype skills are unchanged; this adds a new skill and new test surfaces on top of them.

## Impact

- **Skill** (`.claude/skills/archetype-interaction-test-author/`): new skill + sub-agent prompts; composes with (does not replace) `/assess-archetype-rust` and `/batch-implement-cards-rust-dsl`.
- **Tests** (`code/digimon-engine/tests/`): new `archetypes/` (or `interactions/`) test tree with multi-card fixture helpers; new static-test harness module.
- **Tooling**: reuse `code/tools/resolve_deck.py` (`resolve_archetype`) for pool resolution; possibly a small static-test runner/CLI for the deck-legality / coverage / smoke checks.
- **QA artifacts**: new `qa/archetype-qa/<archetype>-model.md` per archetype; new `qa/qa-reports/archetype_interactions.json` verdict tracker; findings routed to `docs/RUST_ENGINE_GAPS.md` (engine primitives) and `qa/archetype-qa/engine-gaps.md` (card effects).
- **Docs**: `docs/INDEX.md` pointer; `docs/RUST_DSL_TEST_API.md` extended with the multi-card / interaction fixture pattern.
- **Coordination (not a hard dependency)**: shares the finding-routing convention with `add-interactive-replay-bug-hunter`; the skill MAY optionally use the engine MCP (`new_game_debug` / `legal_actions` / `step`) to prototype combos before committing them as Rust tests, but can author DebugRunner tests directly without it.
- **Not changed**: existing per-card skills, the DSL, card schemas, the action space, trained models.
