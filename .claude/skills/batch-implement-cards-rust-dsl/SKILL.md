---
name: batch-implement-cards-rust-dsl
description: Archetype- or pool-scoped TDD pipeline that authors YAML card specs in `code/digimon-engine/cards/<set>/` plus DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/<set>/`. Three-wave architecture (Sonnet scout → Sonnet implementer/auditor → Opus reviewer) with per-card mode auto-detection (IMPLEMENT for new YAML / AUDIT for existing YAML / SKIP for prior verdicts). Engine-gap and DSL-vocab-gap routed to separate trackers. Verdicts in `validated_cards_dsl.json`.
argument-hint: <ARCHETYPE_NAME|--pool> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--implementer-model {sonnet,opus}] [--no-audit] [--skip-tests]
---

# Batch Implement Cards (Rust DSL) — Archetype/Pool-Scoped Test-First DSL Pipeline

Author YAML card specs and behavioral tests for an entire archetype (or the curated DSL test pool) using the engine's declarative DSL. Cards are processed in batches of 4 with parallel sub-agents in isolated git worktrees: a Sonnet scout pre-curates context, a Sonnet implementer (or Sonnet auditor for existing YAML) writes tests-first then YAML, and an Opus reviewer audits each batch. **The orchestrator wires test-discovery `mod.rs` files** after merging — agents never touch shared registration.

This is the DSL-flavored sibling of `/batch-implement-cards-rust`. Cards already implemented as hand-written Rust `CardEffect` structs are not affected; cards already shipping YAML are routed to AUDIT mode.

## When to Use

- Authoring DSL YAML for a new archetype on the Rust engine.
- Running the curated `qa/dsl-test-pool.md` end-to-end (pattern-coverage smoke).
- Auditing existing YAML for drift after a DSL phase lands new vocabulary.

**Not for:** hand-written Rust `CardEffect` work (use `/batch-implement-cards-rust`). Not for Python scripts (use `/batch-fix-cards`). Not for gameplay testing (use `/gameplay-qa`). Not for full rewrites of drifted YAML (v1 is audit-only — emits diff proposals, does not modify shipping YAML).

## Flags

| Flag | Default | Purpose |
|---|---|---|
| `--cards CARD1,CARD2,...` | unset | Explicit comma-separated card list. Wins over both archetype and `--pool`. |
| `--batch-size N` | `4` | Per-batch worker count. |
| `--report-only` | off | Phases 1–2 only — resolve, classify, plan, exit. No agents dispatched. |
| `--implementer-model {sonnet,opus}` | `sonnet` | Escape hatch for hard archetypes. Scout stays Sonnet, reviewer stays Opus regardless. |
| `--no-audit` | off | Skip AUDIT-mode dispatch for cards already shipping YAML. They become `SKIP`. |
| `--skip-tests` | off | Emit YAML without behavioral tests. Strongly discouraged — breaks TDD discipline. |

The positional argument is either an archetype name from `data/deck_library.json` or the literal token `--pool` (which targets every card in `qa/dsl-test-pool.md`).

## Per-Card Mode (auto-detected)

- **IMPLEMENT** — no YAML at `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`. Full scout → implementer → reviewer pipeline.
- **AUDIT** — YAML exists. Single Sonnet auditor → reviewer. No scout wave.
- **SKIP** — prior `IMPLEMENTED` or `AUDITED-OK` verdict in `validated_cards_dsl.json`, or `--no-audit` is set and YAML exists.

## Quick Reference

| Resource | Path |
|----------|------|
| DSL test API (canonical for test patterns) | `docs/RUST_DSL_TEST_API.md` |
| DSL syntax + compile pipeline | `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` |
| Engine API (`EffectContext`) | `docs/RUST_ENGINE_API.md` |
| Production YAML directory | `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` |
| Card test directory | `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` |
| Test discovery root | `code/digimon-engine/tests/cards_behavioral/main.rs` |
| Test discovery per set | `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` |
| Card metadata | `data/cards.json` |
| Deck library | `data/deck_library.json` |
| DSL test pool (curated) | `qa/dsl-test-pool.md` |
| C# reference | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID_UNDERSCORE}.cs` |
| Verdict tracker | `qa/qa-reports/validated_cards_dsl.json` |
| Engine-gap tracker | `qa/archetype-qa/engine-gaps.md` |
| DSL-vocab-gap tracker | `qa/dsl-vocab-gaps.md` |
| Per-archetype QA artifact | `qa/archetype-qa/dsl/<archetype_slug>.md` |
| Pool-progress artifact | `qa/dsl-test-pool-progress.md` |

## Design spec

`docs/superpowers/specs/2026-04-26-batch-implement-cards-rust-dsl-design.md`

---

## Phase 1: Resolve Card Pool

### 1a. Parse the positional argument

Three resolution paths, in priority order:

1. If `--cards` is set: use the explicit list verbatim. Skip both archetype and pool resolution.
2. If the positional argument is `--pool`: parse `qa/dsl-test-pool.md`. The card IDs are in the first column of the markdown table under `## Pool`. Use this Python one-liner via Bash:

   ```python
   import re, pathlib
   text = pathlib.Path("qa/dsl-test-pool.md").read_text()
   pool_section = text.split("## Pool", 1)[1].split("## ", 1)[0]
   card_ids = re.findall(r"\|\s*`([A-Z0-9-]+)`\s+", pool_section)
   print("\n".join(card_ids))
   ```

3. Otherwise: treat the positional as an archetype name and call `code/tools/resolve_deck.py`:

   ```python
   import sys; sys.path.insert(0, 'code')
   from tools.resolve_deck import resolve_archetype
   manifest = resolve_archetype('ARCHETYPE_NAME')
   for c in manifest.unique_cards:
       print(c.card_id)
   ```

   `manifest.unique_cards` yields `CardEntry` objects with `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`, `effect_text`, `inherited_text`, `security_text`, `csharp_path`, `deck_frequency`. `manifest.meta_share` and `manifest.best_decklist` come from scraped tournament data. The Python-side `script_status` / `script_path` fields are **not used** — they belong to the legacy Python pipeline and are out of scope here.

   If `resolve_archetype` raises `UnknownArchetype`, surface the message and tell the user to run:

   ```bash
   python code/tools/resolve_deck.py --list-archetypes --min-meta-share 0.01
   ```

   to find a valid archetype name. Exit cleanly.

### 1b. Classify each card by YAML existence

For each `card_id`, compute the lowercased set prefix and check whether YAML exists:

```python
def set_prefix(card_id: str) -> str:
    # 'BT17-001' -> 'bt17'; 'P-117' -> 'p'; 'LM-029' -> 'lm'; 'AD1-025' -> 'ad1'
    return card_id.split('-')[0].lower()
```

Then:

```python
import pathlib
yaml_path = pathlib.Path(f"code/digimon-engine/cards/{set_prefix(card_id)}/{card_id}.yaml")
mode = "AUDIT" if yaml_path.exists() else "IMPLEMENT"
```

If `--no-audit` is set and `mode == "AUDIT"`: change to `SKIP`.

### 1c. Cross-check the verdict tracker

Load `qa/qa-reports/validated_cards_dsl.json`. For each card:

- If the entry's `status` is `IMPLEMENTED` or `AUDITED-OK`: change `mode` to `SKIP` (idempotency). Other verdicts (`PARTIAL`, `AUDITED-MISSING-TESTS`, `AUDITED-DRIFT`, `BLOCKED`) do **not** trigger SKIP — those cards are re-attempted on the next run.

If the file is missing, treat as no prior verdicts (the file was created by the skeleton commit; if it has been deleted, recreate with `{"version":1,"last_updated":"YYYY-MM-DD","cards":{}}`).

### 1d. Build the cross-archetype reverse map

Scan `data/deck_library.json` to produce `{card_id: [archetype_name, ...]}`. This is used in the final report to surface "this card is also used by N other archetypes." Implementation:

```python
import json, pathlib
deck_lib = json.loads(pathlib.Path("data/deck_library.json").read_text())
reverse = {}
for archetype_name, archetype_data in deck_lib.items():
    for decklist in archetype_data.get("decklists", []):
        for entry in decklist.get("cards", []):
            reverse.setdefault(entry["card_id"], set()).add(archetype_name)
reverse = {k: sorted(v) for k, v in reverse.items()}
```

(Adjust the structural path if `deck_library.json` schema diverges; the actual schema lives in `code/tools/meta_loader.py`.)

---
