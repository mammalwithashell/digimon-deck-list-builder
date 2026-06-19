## Why

The DSL authoring guide ([`docs/RUST_DSL_AGENT_GUIDE.md`](../../../docs/RUST_DSL_AGENT_GUIDE.md), "Last refreshed 2026-05-15") is hand-transcribed from the `digimon-dsl` enums, which are the real source of truth. It has drifted measurably:

| Vocabulary | In source | Undocumented in guide |
|---|---|---|
| Step verbs (`StepSpec`) | 152 | 41 (27%) |
| Predicate fields (`PredicateSpec`) | 146 | 44 (30%) |
| Timings (`Timing`) | 55 | 8 |
| Declarative kinds (`DeclarativeKind`) | 12 | 1 (`link_condition`) |

The gaps are not fringe — they are **whole capability families** added after the last refresh: DigiXros (5/5 verbs missing), Link/AppFuse (4/5 missing, incl. timings `when_linked`, `on_any_link`), Under-Tamer sources (6/6 missing), plus heavily-used singletons like `activation_cost` (28 card uses) and the predicate `event_target_owner` (92 uses). An author scanning §5 concludes those mechanics don't exist in the DSL and reaches for `raw_rust` — the exact anti-pattern §10 forbids.

The drift maps cleanly onto post-2026-05-15 substrate work and **will recur with every new set** under the current hand-maintenance model. Two structural problems compound it:

1. **One document does two jobs.** §1–4/§7/§10 are curated *judgment* (workflow, idioms, red flags) that ages well; §5–6 are an exhaustive *reference* that rots the instant an enum variant lands.
2. **The guide documents dead API while omitting live API.** ~15 documented verbs have **zero** uses in the entire card corpus (`open_counter_window`, `shuffle_deck`, `play_from_trash`, `add_to_hand_from_deck`, `bounce_self`, `handle_replacement`, `redirect_replacement`, `select_ordered_permutation`, `source_is_tamer`, `trashed_source_trait_has`, …), while the live-but-hidden verbs above carry real usage. Prominence is inverted.

The repo already has the raw material to fix this structurally: [`code/tools/dsl-schema-export/`](../../../code/tools/dsl-schema-export/) emits a complete JSON schema from the enums, and `.github/workflows/action-space-codegen-drift.yml` (rule 27) is a proven precedent for a "generated artifact stays in sync or CI fails" gate.

Primary audience is the authoring sub-agents (`/batch-implement-cards-rust-dsl`, `/implement-rust-dsl-archetype`); humans are secondary. So the generated reference must be maximally machine-greppable: one stable row per verb with arg-type and a real fixture card to open.

## What Changes

- **New `dsl-doc-export` tool.** A Cargo workspace member (extending or alongside `dsl-schema-export`) that introspects the `digimon-dsl` enums (`StepSpec`, `PredicateSpec`, `Timing`, `DeclarativeKind`) and the YAML card corpus, and emits a Markdown **Vocabulary Reference**: every step / predicate / timing / kind → YAML key, arg-type/struct, `///` doc-comment, card-usage count, and a fixture card path. Stable one-row-per-entry layout, grouped by family.
- **Generated block in the guide.** The Vocabulary Reference is written between `<!-- BEGIN GENERATED:dsl-vocab -->` / `<!-- END GENERATED:dsl-vocab -->` markers inside `RUST_DSL_AGENT_GUIDE.md` (single file preserves the ~10 skills/docs that link to it). The hand-written §5–6 exhaustive lists are replaced by curated *pattern* prose that points into the generated table.
- **CI drift gate.** A workflow mirroring `action-space-codegen-drift.yml` re-runs the exporter and fails if the generated block (or a committed reference artifact) is out of date, so a new enum variant cannot merge without a doc row.
- **Usage-aware curation pass.** The curated narrative foregrounds high-usage vocabulary in its idioms, and zero/low-usage verbs are flagged (e.g. a `rare` / `unused` column or a "rarely needed" callout) so authors aren't steered toward dead API. The "Last refreshed" line is replaced by the generator stamp.
- **Refresh of stale prose.** Bring §4 timings, §4 declarative kinds, and §5–6 narrative in line with current reality (Link/AppFuse, DigiXros, under-tamer sources, granular event-payload predicates, `activation_cost`, `link_condition`).

## Capabilities

### New Capabilities
- `dsl-authoring-guide`: The DSL authoring guide carries a machine-generated vocabulary reference covering every step verb, predicate, timing, and declarative kind exposed by the `digimon-dsl` enums — each with its YAML key, argument shape, doc-comment, card-usage count, and a fixture card — kept in sync with the enums by a drift-gating CI check. The curated narrative is usage-aware: high-usage vocabulary is foregrounded and zero/low-usage vocabulary is flagged.

## Impact

- **Tooling:** new `code/tools/dsl-doc-export/` workspace member (or extension of `dsl-schema-export`); registered in the root `Cargo.toml` workspace.
- **DSL crate:** may add introspection helpers in `code/digimon-dsl/src/schema.rs` (e.g. expose variant metadata) if rustdoc/serde introspection is insufficient. No behavior change to lowering.
- **Docs:** `docs/RUST_DSL_AGENT_GUIDE.md` restructured (generated block + usage-aware curation); the scratch `docs/_scratch-dsl-inventory.md` is superseded and removed.
- **CI:** new `.github/workflows/dsl-vocab-doc-drift.yml`.
- **No engine / action-space / tensor / PyO3 / frontend impact** — this is documentation + tooling only.

## Non-Goals

- Adding, renaming, or removing any DSL vocabulary. This change only documents what exists; vocabulary changes belong to `dsl-card-scripting-vocabulary` work.
- Auto-generating the curated narrative (§1–4/§7/§10). Those stay hand-written; only the exhaustive reference is generated.
- Pruning the zero-usage verbs from the engine. This change flags them in docs; deciding whether to retire them is separate.
- A standalone reference file. The reference lives inside the existing guide to preserve inbound links (revisit only if size becomes unwieldy).
