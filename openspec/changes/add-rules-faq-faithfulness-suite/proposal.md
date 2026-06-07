## Why

The official Digimon TCG **General Rules/FAQ** (digimoncardgame.fandom.com/wiki/General_Rules/FAQ) is a community-curated, citation-backed corpus of ~60 Q&A entries that pin the game's *foundational* rules — phase legality, mandatory-vs-optional actions, memory cap, 0-DP deletion, keyword-as-text vs keyword-in-play, multi-color identity, security-Digimon scoping. Each entry is a ready-made faithfulness oracle: "here is the situation, here is the correct outcome." Where the existing `add-judge-quiz-faithfulness-suite` pins 30 *adversarial multi-card* scenarios, this corpus pins the **bedrock underneath them** — and it is currently unpinned: we do not know which of these foundational rules the Rust engine reproduces. At least one (simultaneous +DP/−DP ending at end of turn) is a strong candidate to expose a known latent timing bug.

## What Changes

- **Resolve + triage the corpus (gating spike).** Freeze all ~60 FAQ entries into an authoritative ledger, each classified by **test surface**: ① runtime-behavioral (`DebugRunner`), ② deck-validation (`deck_tools`), ③ data/metadata (`CardData`), ④ not-modeled/N-A (documented, not tested — e.g. RPS first-player, which the engine resolves by `seed % 2`).
- **Audit existing coverage; cross-link, do not duplicate.** Inventory the current Rust test tree. For rules already pinned somewhere (0-DP deletion, memory cap, can't-attack-the-turn-played, etc.), the ledger cites the existing test (`XLINK`) rather than re-encoding it. New tests are written only for genuinely-uncovered rules.
- **Reuse real implemented DSL cards as vehicles.** The unit under test is the *rule*, not the card. Each test that needs a card with a specific property (a 2-color card, a `-` Level / `-` DP card, a conditional-keyword card) reuses an already-implemented DSL card that exhibits it; a card is authored only when no implemented card carries the property.
- **Discovery wave (discover-then-pin).** Encode each uncovered, in-scope FAQ entry as a test asserting the FAQ-correct outcome and run it. PASS → pinned permanent regression. FAIL → a discovered faithfulness gap logged to `qa/archetype-qa/engine-gaps.md` (or `qa/dsl-vocab-gaps.md`) and spun off as a scoped fix/chip — the assertion is never weakened to go green.
- **New `tests/rules_faq/` tree**, organized by the FAQ's own sections (deck-creation, phases, main-phase, effect-resolution, keyword-identity, multicolor, no-level/no-value, security-digimon, in-its-text), each test docstring quoting the FAQ question, its answer, and the `general_rule.pdf` / DCGO citation.
- **Reconcile trackers**: per-item verdict ledger at `qa/qa-reports/rules-faq.md`; gap entries routed to the shared gap trackers; any cards authored to fill a property gap recorded in `validated_cards_dsl.json`.

## Capabilities

### New Capabilities
- `rules-faq-faithfulness-suite`: A permanent test suite reproducing the official General Rules/FAQ corpus against the Rust engine. Every FAQ entry is triaged to a test surface and either newly pinned (asserting the FAQ-correct outcome with real implemented DSL cards), cross-linked to pre-existing coverage, or documented as not-modeled; foundational rules clusters (phase legality, mandatory/optional resolution, memory & DP arithmetic, keyword-as-text vs gained-keyword, multi-color identity, security-Digimon scoping, "X in its text/name" matching) are verified; and a per-item verdict ledger is reconciled to test reality.

### Modified Capabilities
<!-- None pre-emptively. The discovery wave may confirm gaps that change spec-level
     behavior in existing capabilities (candidate hot spots: permanent-deletion-semantics
     for the simultaneous +DP/−DP end-of-turn rule; security-card-effects for the
     security-Digimon-not-Digimon scoping). Any such requirement change will be added as
     a MODIFIED delta to that capability's spec when the specific gap is confirmed during
     the discovery wave — not pre-emptively. -->

## Impact

- **Tests:** new `code/digimon-engine/tests/rules_faq/` tree (one module per FAQ section) + additions to the `tests/deck_tools/` surface for deck-creation rules; cross-links (no new code) for already-covered rules.
- **Card content:** `code/digimon-engine/cards/<set>/*.yaml` — only the handful of property-carrying cards (e.g. a `-` Level, a `-` DP, a conditional-keyword card) authored when the repo lacks an implementable vehicle; biased toward reuse.
- **Engine / DSL (Rust):** `code/digimon-engine/src/` and `code/digimon-dsl/src/` — only where the discovery wave confirms a genuine gap (candidate hot spot: the simultaneous-end-of-turn DP-modifier rule vs the known latent 17-1-2-2 mid-effect-deletion timing bug).
- **Reference:** read-only use of `general_rule.pdf` (canonical) and the base-repo `DCGO/` submodule as behavioral tiebreakers per CLAUDE.md source priority; no DCGO edits.
- **Trackers:** new `qa/qa-reports/rules-faq.md` (per-item verdict ledger); `qa/qa-reports/validated_cards_dsl.json`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md` reconciled as gaps surface.
- **No RL contract change** is expected; if a gap fix requires a pending-selection sub-range it is handled under the existing additive action-space contract (`docs/ACTION_SPEC.md` updated if so).
- **Scope note:** the corpus spans four test surfaces and two harnesses; the change is phased (triage ledger → audit/cross-link → discovery wave → gap chips) so value lands incrementally even if the full corpus isn't completed in one pass.
