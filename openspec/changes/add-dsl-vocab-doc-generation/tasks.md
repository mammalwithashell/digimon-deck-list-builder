# Tasks — DSL vocabulary doc generation

## 1. Spike: metadata extraction (D2)
- [x] 1.1 Confirm whether `schemars` surfaces enum-variant `///` doc-comments as schema `description` at the crate's version. **Result: yes** — schemars 0.8 captures `///` as `description` for predicate fields (38/145), step variants (22/152), and timing variants. No crate-local `variant_docs()` map needed.
- [x] 1.2 Confirm arg-struct names/shapes are recoverable per variant. **Result: yes** via `$ref`/inline type in each step variant subschema. **Caveat:** step YAML keys are NOT in the schema (PascalCase variant names only, due to `StepSpec`'s hand-written serde); they are joined from the authoritative `kv!(s, "key", v)` literals in `step.rs`. Predicate/timing/kind keys are in the schema directly. (Design D2 updated.)

## 2. `dsl-doc-export` formatter (Python, mirrors `action-space-export`)
- [x] 2.1 Reuse the existing `cargo run -p dsl-schema-export` JSON as the Rust-emit step (no new Cargo member needed — design D2). Add `code/tools/dsl-doc-export/emit_markdown.py` + a README.
- [x] 2.2 Parse the schema JSON → `StepSpec` (PascalCase variant + doc + arg-ref), `PredicateSpec`, `Timing`, `DeclarativeKind` rows; join step variants to snake keys via `step.rs` `kv!` serialize literals.
- [x] 2.3 Scan `code/digimon-engine/cards/**/*.yaml` → per-key usage count + first fixture path.
- [x] 2.4 Derive `unused` (0 uses) / `rare` (1–2) tags.
- [x] 2.5 Emit the grouped-by-family Markdown Vocabulary Reference (stable, one row per entry, deterministic ordering — sorted by key so count churn never reorders).
- [x] 2.6 In-place writer: replace content between `<!-- BEGIN GENERATED:dsl-vocab -->` / `<!-- END GENERATED:dsl-vocab -->`, leaving all other content untouched. Banner + structural-sha included.
- [x] 2.7 Verified idempotent (two runs → no diff) and `--check` passes when synced / fails (exit 1) on drift.

## 3. Restructure the guide
- [x] 3.1 Inserted the generated-block markers + populated the Vocabulary Reference (block at §11, after §10; guide 683→1208 lines).
- [x] 3.2 Reframed §5/§6 exhaustive lists: the authoritative index is now the generated table (pointers added at §5 head and §6 predicate-families intro). Kept the per-verb/per-predicate *nuance* prose deliberately — that curated judgment is the value the generated table can't encode (deleting it would lose, not gain).
- [x] 3.3 Refreshed §4 timings + declarative kinds (added Link family + `on_block`, `on_add_to_hand`, `on_check_face_up_security`, `before_pay_cost_observe`, `on_option_trashed`; `link_condition` kind; pointers to generated tables).
- [x] 3.4 Usage-aware: generated tables carry a `tag` column (`unused`/`rare`); §5 banner steers authors to live idioms and away from `unused` verbs.
- [x] 3.5 Replaced the "Last refreshed" line with a generated-reference + CI-gate note.

## 4. CI drift gate
- [x] 4.1 Added `.github/workflows/dsl-vocab-doc-drift.yml` mirroring `action-space-codegen-drift.yml` (runs `dsl-schema-export | emit_markdown.py --check`).
- [x] 4.2 Confirmed the gate ignores count/fixture churn (structural-sha excludes them; `--check` against a `bt1/`-only scan still passed) and goes green on the synced guide.

## 5. Verification
- [x] 5.1 Re-ran the inventory diff → 0 undocumented keys (152 steps, 145 predicates, 56 timings, 12 kinds all present).
- [x] 5.2 Proved the gate fails on a new enum member: committed sha == fresh sha, and `structural_sha` changes when a synthetic step variant is added (so `--check` would exit 1 until regeneration). Also verified `--check` exits 1 on a tampered sha.
- [x] 5.3 Removed `docs/_scratch-dsl-inventory.md` (and temp build artifacts).
- [x] 5.4 Confirmed the 3 inbound links (`implement-rust-dsl-archetype` SKILL.md + reference, `docs/INDEX.md`) still resolve (guide path unchanged) and the generated section headers are greppable.
