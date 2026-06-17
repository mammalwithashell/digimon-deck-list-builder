# dsl-doc-export

Generates the **DSL Vocabulary Reference** block inside
[`docs/RUST_DSL_AGENT_GUIDE.md`](../../../docs/RUST_DSL_AGENT_GUIDE.md) from the
`digimon-dsl` enums, so the guide's exhaustive list of step verbs / predicates /
timings / declarative kinds can never drift from the code.

This is the Python *formatter* half of a Rust-emit + Python-format codegen pair,
the same shape as [`action-space-export`](../action-space-export/) (CLAUDE.md
rule 27): the Rust binary emits machine-readable data, the Python script formats
it and writes the doc.

## Regenerate (after changing any DSL enum)

```bash
cargo run -q -p dsl-schema-export | python code/tools/dsl-doc-export/emit_markdown.py
```

The Rust half is the existing `dsl-schema-export` binary (emits the schemars
JSON for `CardSpec`); no new Cargo member is needed.

## Drift gate

```bash
cargo run -q -p dsl-schema-export | python code/tools/dsl-doc-export/emit_markdown.py --check
```

Exits non-zero if the committed block's structural signature
(`<!-- vocab-structural-sha: … -->`) differs from what the current enums
produce. CI runs this via `.github/workflows/dsl-vocab-doc-drift.yml`.

The signature covers only the **structural set** — keys, families, argument
shapes, and doc-comments. Usage counts and fixture paths are refreshed on every
run but excluded from the signature, so ordinary card authoring never trips the
gate; only adding/renaming/removing an enum variant does.

## How keys are resolved

| Enum | Key source |
|------|------------|
| `PredicateSpec`, `Timing`, `DeclarativeKind` | schemars JSON (real YAML keys) |
| `StepSpec` | PascalCase variant + doc + arg-ref from schemars, **joined to the snake YAML key by parsing the `kv!(s, "<key>", v)` serialize literals in `code/digimon-dsl/src/step.rs`** — `StepSpec` has a hand-written serde impl, so schemars alone doesn't expose its keys |

Doc-comments come from schemars (`///` → schema `description`); coverage is
partial and that's fine — a row shows a description only when the enum has one.

## Flags

- `--schema PATH` — read schema JSON from a file instead of stdin.
- `--guide PATH` — target guide (default `docs/RUST_DSL_AGENT_GUIDE.md`).
- `--cards-dir PATH` — corpus to scan for usage (default `code/digimon-engine/cards`).
- `--check` — drift-gate mode (no writes).
