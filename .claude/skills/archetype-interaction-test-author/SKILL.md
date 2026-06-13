---
name: archetype-interaction-test-author
description: Capstone archetype-QA skill. Researches an archetype as a *system* (combos, digivolution lines, playstyle, win conditions), emits a durable archetype-model doc, then plans and authors multi-card DebugRunner interaction tests plus the four static archetype tests (deck-legality, coverage gate, smoke games, combo-presence). Runs AFTER the archetype's cards are implemented and per-card tests are green; composes with /assess-archetype-rust and /batch-implement-cards-rust-dsl and does NOT re-implement cards. Confirmed test failures route to the shared gap trackers; it does not edit engine code.
argument-hint: <ARCHETYPE_NAME | --cards CARD1,CARD2,...> [--top-n N] [--smoke-games N] [--threshold F]
---

# Archetype Interaction Test Author

You build a **system-level understanding** of an archetype and turn it into
**interaction tests** — multi-card combos exercised the way the deck actually
plays — plus the **static archetype tests** that gate them. This is the
**capstone** of the archetype-QA family: it runs *after* the cards are
implemented (`/batch-implement-cards-rust-dsl`) and their per-card behavioral
tests are green. It does **not** re-implement cards or author per-card tests.

It is the proactive, hypothesis-driven third mode of bug discovery, alongside
the two reactive replay modes in `/replay-bug-hunt` (differential + judge): here
you form hypotheses about how the archetype *should* behave and test them
deterministically. All three modes route confirmed findings to the same
trackers.

## When to use

- An archetype's cards are implemented and per-card tests pass, and you want to
  catch the **cross-card** bugs per-card TDD misses (a trigger firing off
  another card's digivolve, a cost-reduction loop, a security-trigger chain).
- You want a durable, reviewable model of how an archetype works as a system.

Two saved workflows fan this skill out across archetypes — pick by intent:
`author-archetype-combo-tests` (greenfield: author suites for archetypes that
have none) and `audit-archetype-faithfulness` (audit: re-verify existing suites
against card text / `general_rule.pdf` / DCGO, gap-fill untested top combos,
and emit per-archetype faithfulness verdicts + a dated report).

If cards are still missing/red, run `/assess-archetype-rust` then
`/batch-implement-cards-rust-dsl` first — this skill will *report* what's missing
(precondition gating, Phase 4) but won't author tests that can't pass.

## Quick reference

- **Resolve a pool:** `python code/tools/resolve_deck.py "<archetype>" --json`
  (over `data/deck_library.json` + `data/archetype_aliases.json`).
- **Static-test harness:** `cargo run -p archetype-static-tests -- "<archetype>"
  [--threshold F --smoke-games N --combo "name=A,B" --combos-file f.json --json]`
  → the four invariants + writes `qa/qa-reports/archetype_interactions.json`.
- **Interaction-test home:** `code/digimon-engine/tests/archetypes/<slug>.rs`,
  fixtures in `tests/archetypes/support.rs` (`dsl_builder`, `BoardSnapshot` +
  `snapshot`, `run_actions`). Exemplar: `tests/archetypes/rocks.rs`. Run with
  `cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes`.
- **Model doc:** `qa/archetype-qa/<archetype>-model.md`.
- **Per-card verdicts (read-only here):** `qa/qa-reports/validated_cards_dsl.json`.
- **Source priority** (CLAUDE.md): `general_rule.pdf` (canonical) + DCGO C#
  (battle-tested) outrank the card-text JSON. Resolve base-repo DCGO:
  `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`
  (C# at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`,
  underscores: `BT17-102` → `BT17_102.cs`).
- **Retrieval:** sub-agents use the Pinecone `digimon-engine` index (card-scripts,
  engine-api, card-metadata, rules-docs) per the family convention.

---

## Phase 0 — Resolve the archetype

1. If `--cards CARD1,...` was given, use that list as the pool (skip the deck
   library). Otherwise resolve the name:
   `python code/tools/resolve_deck.py "<archetype>" --json` → the unique-card
   pool with per-card text + play frequency. (If the name is unknown, list
   candidates with `--list-archetypes` and stop.)
2. Note the canonical name and slug (`<archetype>` lowercased, spaces → `-`).

## Phase 1 — Research

Build the inputs for the model. For each meaningful card in the pool:

- Read the **printed text** (`effect_text` / `inherited_text` / `security_text`
  from the resolve output / `card_overrides.json` / `cards.json`).
- Read the **DCGO C#** at `$BASE_DCGO/.../<CARD_ID>.cs` for how it actually
  resolves (processing order, interaction edges).
- Read the relevant **`general_rule.pdf`** rules (keyword semantics in §16;
  timing) via the Read tool's `pages` arg — cite rule numbers.
- Retrieve prior context from Pinecone (`digimon-engine` index): existing card
  scripts, engine-API notes, the archetype's per-card QA doc if one exists.

Identify the digivolution lines, the payoffs (what wins), the enablers (search /
ramp / cost reduction), the engines (recurring value), and the named combos that
make the deck function.

## Phase 2 — Model (the durable artifact)

Emit `qa/archetype-qa/<archetype>-model.md` **before authoring any test**, with
this fixed structure (sources cited inline — DCGO C# path and/or `general_rule.pdf`
rule number):

```markdown
# <Archetype> — Model

## Card pool & roles
| Card | Role (payoff/enabler/engine/tech) | One-line function |

## Digivolution lines
- <egg → rookie → champion → ...>, with the cost/colour gates.

## Named combos
### <Combo name>
- Cards: <CARD_A>, <CARD_B>, ...
- Expected mechanical outcome: <precise board change the combo claims>
- Rules/keyword basis: <general_rule.pdf §16-xx ; DCGO C# ref>
- Rank: <play-frequency × payoff-centrality>

## Playstyle
- Class (aggro/control/combo), tempo, memory curve.

## Win conditions
- <how the deck closes games>

## Ranked interactions to test
1. <combo> — <why it's high-value to test>
...
```

The model is the input to planning and is reviewable (Phase 6's Opus reviewer
audits it against card text + rules before tests are trusted). A wrong model
yields wrong tests — cite sources so the review is concrete.

## Phase 3 — Plan

- Rank the candidate interactions (play frequency × payoff centrality).
- Select the top **N** (default from `--top-n`, else a sensible cap). **`log()`
  the interactions you did not select** — never silently truncate.
- Enumerate the static checks to run (all four; coverage-gate + combo-presence
  are the precondition gates below).

## Phase 4 — Precondition gating (before authoring)

Run the static harness for the archetype's targeted combos:

```bash
cargo run -p archetype-static-tests -- "<archetype>" \
  --combo "<combo1 name>=CARD_A,CARD_B" --combo "<combo2 name>=CARD_C,CARD_D" --json
```

- **Coverage gate** + **combo-presence** are the gates. If a combo names an
  **unimplemented** card, report that combo as **blocked on the missing card**
  and do **not** author its interaction test. Route the missing card to the
  implementation backlog (`/batch-implement-cards-rust-dsl`) and the gap
  trackers (`docs/RUST_ENGINE_GAPS.md` for a missing engine primitive,
  `qa/archetype-qa/engine-gaps.md` for a card-effect gap).
- **Deck-legality** + **smoke-games** establish the deck is constructible and
  live; a smoke panic is itself a finding to triage (Phase 6).
- Proceed to author only the combos whose pieces are all present.

## Phase 5 — Author interaction tests

For each surviving top-ranked combo, write a DebugRunner interaction test in
`code/digimon-engine/tests/archetypes/<slug>.rs`, using the shared fixtures
(`dsl_builder`, `snapshot` / `BoardSnapshot`, `run_actions`) — see
`tests/archetypes/rocks.rs` for the exemplar pattern. Each test:

- exercises **real implemented DSL cards for EVERY role** — not only the named
  combo pieces but also fillers, neutral targets, opponents, and stack bases —
  loaded by real card ID via `dsl_builder` / `dsl_card`. Synthetic
  `make_test_card` is a **last resort**, allowed only when NO real implemented
  DSL card can fill a role; when unavoidable, add a one-line comment naming the
  role and why no real card fit. Prefer effectless / vanilla real DSL Digimon
  for filler/target roles so their own effects don't perturb the assertion,
- asserts the combo's **claimed mechanical outcome** (a before/after
  `BoardSnapshot` diff: cards deleted/drawn/milled, memory swing, DP window),
- includes the **unhappy path** where useful (the combo breaks if an enabler is
  absent or the opponent disrupts it — the system-level fact a per-card test
  can't express),
- carries a **doc-comment traceable to the model combo** (combo name, cards,
  expected outcome, sources), so a reader can map test ⇄ model ⇄ card text.

Register the new file in `tests/archetypes/main.rs` (`mod <slug>;`).

## Phase 6 — Execute, triage, record

1. Run the interaction tests
   (`cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes`)
   and the static harness.
2. Treat each **failure as a candidate engine bug** — not a test to weaken.
   Before filing, **confirm the discrepancy** against the card's printed text,
   `general_rule.pdf`, and DCGO C# (`$BASE_DCGO`), exactly like a replay
   divergence. Distinguish a genuine engine bug from a wrong model/test (fix the
   test only if the *model* was wrong, and update the model doc).
3. **Route confirmed findings:** engine-primitive gap → `docs/RUST_ENGINE_GAPS.md`;
   card-effect faithfulness gap → `qa/archetype-qa/engine-gaps.md`. Each entry
   cites the combo, the test, and the source consulted. **Do not edit engine
   code as part of a run.**
4. **Record the run** in `qa/qa-reports/archetype_interactions.json` (the harness
   CLI writes the static-gate results; add the combos tested + their pass/fail +
   findings filed). A clean pass is also recorded.

---

## Sub-agent structure (three waves, mirroring `/batch-implement-cards-rust-dsl`)

Orchestrate the heavy phases with parallel sub-agents; the orchestrator owns
shared registration (`tests/archetypes/main.rs`) — agents never touch it.

1. **Scout (Sonnet)** — drafts the archetype-model + the ranked interaction plan
   (Phases 1–3). Pinecone-retrieves card scripts / engine API / rules; reads
   card text + DCGO C# + `general_rule.pdf`. Output: the model doc + a ranked
   combo list with the cards + expected outcomes + sources.
2. **Implementer (Sonnet)** — authors the interaction tests for the approved
   combos (Phase 5), one combo per test, each traceable to the model. Runs them
   locally and reports pass/fail + any candidate findings.
3. **Reviewer (Opus)** — audits the **model and the tests** against card text +
   `general_rule.pdf` + DCGO C# before they are trusted/committed. Catches a
   wrong combo claim, a test that asserts the wrong outcome, an
   over-/under-specified DP window, a missed "may"/"by-cost"/"or" nuance. Gate
   findings here before they're filed.

Print the plan (model summary + ranked combos + the cap with dropped
interactions logged) and **require approval before the implementer wave**, per
the family's plan-approve-execute convention.

## Optional — MCP-assisted combo prototyping

When the engine MCP (`digimon-engine-mcp`) is registered, prototype a combo
interactively before writing the Rust test, to confirm the action sequence and
reduce authoring churn:

- `new_game_debug` to set up the board (hands / field stacks / security),
- `legal_actions` to find the action IDs,
- `step` to drive the sequence and read the resulting events / state.

This is an optimization, **not** a dependency — author the Rust test directly
when the MCP is absent.

## Guardrails

- Real DSL cards for every role: load all bodies (combo pieces, fillers,
  targets, opponents, stack bases) by real card ID via `dsl_builder`/`dsl_card`.
  Synthetic `make_test_card` is a last resort only when no real implemented DSL
  card fits — comment the role + reason when you must. A real card pulled in as
  filler must be effectless/vanilla (or its effect accounted for) so it can't
  silently change the asserted outcome.
- Capstone only: assume cards implemented; never re-implement a card or author a
  per-card test here.
- Model before tests; cite sources inline so review is concrete.
- One test ⇄ one named combo; assert the *mechanical* outcome a human can check.
- Cap interactions by rank and **log what was dropped**.
- Confirm a failure against card text + rules + DCGO C# before filing; never
  edit engine code as part of a hunt.
- "unknown"-status cards are not "implemented" — report them, don't assume pass.
- Smoke games are a liveness gate, not correctness (a green smoke ≠ correct).
