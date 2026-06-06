## Why

Card authoring today is **archetype-scoped**: every skill (`batch-implement-cards-rust-dsl`, `implement-rust-dsl-archetype`, `assess-archetype-rust`, `archetype-interaction-test-author`) resolves its pool through `resolve_deck.py` → `deck_library.json` (scraped tournament decklists). There is no way to author an entire **release set** (a booster like BT17 with ~102 cards, or an EX set with ~74). The team has been authoring starter decks set-at-a-time (`implement-st1`…`implement-st6`), but those work only because a starter is small, single-color, and self-contained — effectively an archetype that happens to be a product. A booster set is not: it fans across 6 colors and a dozen evolution lines, its cards reference cards in other sets, and **each new set introduces ~1–3 genuinely new keywords/mechanics** that the engine does not yet support.

Two concrete facts from a live `digimoncard.io` pull motivate this change:

1. **The local snapshot drifts on the newest set.** `data/cards.json` matches the live API exactly for settled sets (BT17: 102 = 102), but EX12 is `3` locally vs `43` live — a 40-card gap. The set you most want to author is the one most likely to be stale, so "just filter `cards.json`" is insufficient; authoring must begin with a pull→diff→ingest.
2. **New keywords recur and the engine lags them.** Scanning BT17/BT22/BT24/BT25/EX11 effect text against the engine's 34-variant `Keyword` enum surfaces a small, high-signal residual of new mechanics (App Fusion, Link, Petrification, Assembly/DigiXros-variant, Burst Digivolve, Unchained). `App Fusion` appears across BT22, BT24, **and** BT25 — once ingested it pays back across three sets (rule 28 flywheel). Without a per-set gate, these are discovered ad hoc, mid-implementation, after dependent cards have already been authored on a missing primitive.

A release-set authoring workflow makes "author all of BT17" a single, ordered, gated operation that (a) refreshes set data from the source of truth, (b) detects and resolves new-keyword substrate **before** mass implementation, (c) reuses the existing archetype skills for the bulk work, and (d) gates on complete, tested coverage.

## What Changes

- Add a **`/author-set <SET>` Workflow** (`code/digimon-engine`-adjacent tooling + a Workflow script) that orchestrates release-set authoring in six phases: ingest-diff → keyword gate → cluster → mass-implement → combo-test → set gate. Phases 4–5 invoke the existing `batch-implement-cards-rust-dsl` and `archetype-interaction-test-author` skills as stages; the workflow owns the new connective tissue.
- Add a **set resolver + ingest-diff** step: pull `?card=<SET>` from `digimoncard.io` (reusing `code/tools/ingest_cards.py`), diff against `data/cards.json`, and merge missing/changed cards before authoring. Surface the diff to the user; never silently author against a stale snapshot.
- Add a **DCGO-oracle keyword-ingestion gate**: detect candidate new keywords by lexicon set-subtraction (card-names ∪ traits ∪ timings ∪ grammar ∪ existing Rust keywords), then triage each against DCGO. Keywords DCGO implements are **auto-ingested** (port the C# behavior to a Rust `Keyword` variant + DSL lowering + wiring + behavioral test, TDD against DCGO); keywords DCGO does **not** implement are **flagged-for-human** (halt, emit to `docs/RUST_ENGINE_GAPS.md` + a `.claude/plans/` stub, await context/direction). The gate is a hard barrier before mass-implementation.
- Add a checked-in **DCGO keyword manifest** (`data/dcgo_keyword_manifest.json`) plus **trait/keyword lexicons** that the gate consults and maintains. DCGO has no central keyword enum — keyword behavior is scattered across `I…Effect` interfaces (`CardEffectInterfaces.cs`), `CardEffectFactory/Add*.cs` factory methods, and core mechanism files — so the manifest is extracted once and refreshed on DCGO rebase (pairs with the rule-27 rebase checklist).
- Add a **multi-signal set→slice clusterer**: decompose the set's ~100 cards into archetype/evolution slices (trait → membership, name-reference graph → connectivity, color+level → intra-slice order, reference-frequency → marquee-theme identification) plus a labeled **orphan-staples** bucket. Slices drive mass-implement ordering and combo-test targeting; orphan staples are authored but skip combo-testing.
- Add **lazy, test-driven cross-set dependency pull**: authoring needs ~zero cross-set pull (effects are self-contained; evolution prerequisites are generic and satisfied by synthesized DebugRunner fixtures). A cross-set card's *implementation* is pulled only when a slice's interaction test exercises that card's behavior and it is not already implemented — never an eager transitive closure.
- Add a **set-level coverage gate + verdict tracker**: roll up per-card verdicts; assert every set card reaches `IMPLEMENTED` with green behavioral tests; emit a set-level report and tracker entry.

## Capabilities

### New Capabilities

- `release-set-authoring-workflow`: End-to-end orchestration of release-set authoring — set resolution + ingest-diff, multi-signal clustering into slices, dependency-ordered mass-implementation via existing skills, per-slice combo-testing with lazy cross-set pull, and a set coverage gate.
- `dcgo-keyword-ingestion-gate`: Per-set detection of new keywords/mechanics, DCGO-oracle triage (covered / auto-ingest / flag-for-human), the checked-in DCGO keyword manifest + lexicons, and the auto-ingest barrier sub-pipeline.

### Modified Capabilities

- None. Existing card-authoring skills are invoked unchanged as workflow stages.

## Impact

- Affected code: a new Workflow script + supporting tooling (set resolver/differ around `code/tools/ingest_cards.py`, the keyword-gate detector/manifest extractor, the clusterer, the set gate); new data files (`data/dcgo_keyword_manifest.json`, trait/keyword lexicons); a new set-level verdict tracker under `qa/qa-reports/`. Keyword auto-ingestion modifies core engine surface (`code/digimon-engine/src/enums.rs`, `digimon-dsl`, `keyword_effects.rs`) on demand, each change TDD-gated against DCGO.
- Runtime behavior: no change to engine semantics except when a new keyword is ingested (additive `Keyword` variants + wiring). The workflow is a development/authoring tool, not a runtime path.
- API/contracts: no action-space, tensor-profile, or PyO3 API changes from the workflow itself. Auto-ingested keywords may add action-space entries if a keyword exposes a new player choice; such changes follow the existing rule-27 `ActionSpace.cs` codegen + drift-CI process.
- Source-of-truth dependency: requires network access to `digimoncard.io` for the ingest-diff phase, and a populated base-repo DCGO checkout (rule 29) for the keyword oracle.
