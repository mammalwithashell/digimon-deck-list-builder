# `author_set` — release-set authoring tooling

Deterministic primitives behind the `/author-set <SET>` Workflow. See the spec +
design in `openspec/changes/add-author-set-workflow/`.

## Modules

| Module | Phase | What |
|---|---|---|
| `set_resolver.py` | 1 | set prefix (`BT17`) → distinct card-ID list (exact-prefix, alt-art collapsed) |
| `ingest_diff.py` | 1 | pull `?card=<PREFIX>` → diff vs `cards.json` → merge; loud offline fallback |
| `dcgo_manifest.py` | 2 | extract DCGO keyword surface + diff vs Rust `Keyword` enum |
| `lexicons.py` | 2 | complete trait + card-name lexicons (whole DB, never a sample) |
| `keyword_gate.py` | 2 | bracket scan → set-subtraction → positional trait denoise → DCGO triage |
| `../impact_scope.py` | verify | default scoped-test source after authoring merge/YAML/DSL edits |

Tests: `code/tests/tools/test_author_set_*.py`.

## Generated artifacts (checked in)

- `data/dcgo_keyword_manifest.json` — DCGO keyword registry (union of the two
  `KeyWordEffects/` dirs), interface list, core-modeled allowlist, Rust-enum
  keywords, and the `auto_ingest_candidates` / `rust_only_core_modeled` diffs.
- `data/author_set_lexicons.json` — complete `traits` + `card_names` lexicons.

## Refresh procedure (run on DCGO rebase and after card-data ingest)

The manifest is the DCGO-oracle for the keyword gate; it must track the DCGO
checkout. Per CLAUDE.md rule 27 (DCGO recorder maintenance) and rule 29 (DCGO
lives in the base repo), regenerate after rebasing the DCGO submodule:

```bash
# from repo root, with the base-repo DCGO populated (rule 29)
PYTHONPATH=code python -m tools.author_set.dcgo_manifest   # -> data/dcgo_keyword_manifest.json
PYTHONPATH=code python -m tools.author_set.lexicons        # -> data/author_set_lexicons.json
```

The manifest extractor anchors on the UNION of both keyword directories
(`CardEffectFactory/KeyWordEffects/` ∪ `CardEffectCommons/KeyWordEffects/`) —
neither is complete alone — plus the `I…Effect` interfaces and a hand-curated
core-modeled allowlist (`SecurityAttack±`, `DrawX`, `DeDigivolve`, `DigiBurst`).
If a DCGO rebase moves a keyword's representation, re-audit per
`openspec/changes/add-author-set-workflow/design.md` § "Fidelity audit" and
update `CORE_MODELED_ALLOWLIST` / `DCGO_TO_RUST_ALIAS` as needed.

## Scoped verification after authoring merge

After Phase 1 ingest merges card-data drift and after any generated YAML/DSL
authoring output is merged into the worktree, use the verification-ladder impact
scope as the default scoped-test source:

```bash
python code/tools/impact_scope.py
```

The command reads `git diff` by default and emits `cards_behavioral_filter`,
`side_binaries`, and `full_suite_required`. Use those fields to choose the
targeted behavioral test filter and side binaries. If `full_suite_required` is
true, the authoring flow must run or report the broader suite requirement
instead of narrowing manually.

## Known limitations

- **Name/trait collision**: a would-be keyword that spells like an existing card
  name or trait is classified as that, not flagged. The manifest's
  `auto_ingest_candidates` (proactive DCGO-vs-Rust diff) is the backstop.
- **DCGO core-modeled keywords**: keywords DCGO models in core processing rather
  than a `KeyWordEffects/` file (e.g. Petrification) are not in the registry and
  will `flag_for_human` until added to a curated DCGO-core list. Resolve during
  the dry-run triage.
