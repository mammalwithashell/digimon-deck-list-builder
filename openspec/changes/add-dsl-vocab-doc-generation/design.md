# Design — DSL vocabulary doc generation

## Context

The guide drift is a *process* failure, not a content failure: there is no mechanism keeping the exhaustive reference in step with the enums. The fix is to make completeness a build artifact (generated + gated) and let humans own only what generation can't produce (judgment). This mirrors the existing `action-space-codegen-drift.yml` pattern (rule 27), which keeps DCGO's `ActionSpace.cs` in sync with `code/digimon-engine/src/action/space.rs`.

## D1 — Where does the generated reference live?

**Decision: an in-file generated block inside `RUST_DSL_AGENT_GUIDE.md`, delimited by HTML comment markers.**

Alternatives considered:
- *Separate `RUST_DSL_VOCABULARY.md`.* Cleaner separation, but ~10 skills/docs link to `RUST_DSL_AGENT_GUIDE.md` as the one DSL doc; a second file fragments discovery and risks agents reading only the narrative half. Rejected for now (noted as a future option if the block grows unwieldy).
- *Generated rustdoc only.* Doc-comment density is uneven (~25% of variants have `///`); rustdoc also isn't where authoring agents look. Rejected.

The block is regenerated in place by the exporter (read file → replace between markers → write). Everything outside the markers is hand-owned and never touched by the tool.

## D2 — How does the exporter get variant metadata?

**Decision (refined after the spike): Rust-emit + Python-format, mirroring the `action-space-export` → `emit_csharp.py` precedent (rule 27). Metadata comes from the existing schemars JSON, joined with the authoritative serialize-literal key map for steps.**

Spike findings (2026-06-14, against schemars 0.8 / `dsl-schema-export` output):
- schemars **does** capture `///` doc-comments as schema `description` — for predicate struct fields (38/145 documented), step enum variants (22/152), and timing variants. Partial coverage is expected and fine; the reference shows a doc when one exists. **No crate-local `variant_docs()` map is needed.**
- Arg shapes are recoverable per variant: each `StepSpec` variant subschema's single property is either an inline type or a `$ref` to its arg struct (e.g. `FormulaStepArgs`).
- **Key gap:** `StepSpec` has a hand-written `Serialize`/`Deserialize` (not derived), so schemars only exposes the **PascalCase variant names** for steps — not the snake_case YAML keys. The authoritative `Variant → "yaml_key"` map lives in the `serialize` impl as `kv!(s, "key", v)` literals. `PredicateSpec` (derived serde), `Timing`, and `DeclarativeKind` expose their real YAML keys directly in the schema.

Resulting pipeline (matches `action-space-export` exactly — that tool is Rust `src/` emit piped into `emit_csharp.py`):
1. `cargo run -p dsl-schema-export` emits the schema JSON (already exists; no Rust changes needed for predicates/timings/kinds + step docs/args).
2. `code/tools/dsl-doc-export/emit_markdown.py` consumes that JSON, **joins step PascalCase variants to their snake keys by parsing the `kv!(s, "<key>", …)` literals in `code/digimon-dsl/src/step.rs`** (authoritative contract, not a snake_case guess), scans the card corpus for usage, and rewrites the guide's generated block.

The only "source read" is the serialize-literal join for steps, which reads the *contract itself* (the literals that define the wire keys), so it is authoritative rather than heuristic — distinct from the throwaway scratch inventory's full regex parse of the enum body.

## D3 — Usage counts and fixtures

**Decision: the exporter scans `code/digimon-engine/cards/**/*.yaml` for each YAML key and records (count, first-fixture-path).**

This is the signal that drove the usage-aware reorg and is cheap (one corpus pass). It also gives each row a real card to open — the highest-value field for an authoring agent. Counts are advisory metadata, not a contract; the drift gate keys on the *set of keys* and arg-shapes, not the counts (counts change every set without signaling a doc problem). Consideration: to keep the gate from churning on every card addition, either (a) round counts to buckets, or (b) exclude counts/fixtures from the drift hash and regenerate them opportunistically. **Lean (b):** gate on structure (keys + args + docs), refresh counts/fixtures on every run but don't fail CI on count drift alone.

## D4 — What exactly does the drift gate compare?

**Decision: CI runs the exporter into a temp copy and diffs the structural portion of the generated block against the committed guide; non-empty diff fails.**

Structural portion = the set of (key, family, arg-type, doc) rows. A new enum variant → new row → diff → fail until the dev regenerates and commits. This is the same shape as `action-space-codegen-drift.yml`. The job is fast (cargo run + diff), no engine build of the full workspace required beyond the dsl crate + tool.

## D5 — Usage-aware curation shape

Two mechanisms, both in the generated table (so they self-maintain):
- A `uses` column on every row.
- A derived tag: `unused` (0 corpus uses) and optionally `rare` (1–2). Authoring agents can grep `unused` to avoid dead API; the curated idioms in §5–7 cite only live verbs.

The ~15 currently-documented zero-use verbs are not deleted from docs (they're real API) but are demoted out of the narrative idioms into the generated table where the `unused` tag warns about them.

## Risks

- **schemars doc-comment coverage (D2)** is the main unknown — resolved by the spike; fallback is a crate-local metadata map.
- **Marker discipline:** if a human edits inside the generated block, the next regen silently overwrites it. Mitigation: a visible "DO NOT EDIT — generated by dsl-doc-export" banner inside the block, plus the CI gate catches divergence.
- **Count churn (D3):** mitigated by excluding counts from the gate hash.

## Migration / rollout

1. Land the exporter + generate the block (guide now complete).
2. Land the curation pass (narrative foregrounds live verbs).
3. Land the CI gate last, so it goes green on an already-synced guide.
4. Remove `docs/_scratch-dsl-inventory.md`.
