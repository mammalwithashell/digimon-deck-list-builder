# Split Cards JSON Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deterministic CLI that splits `data/cards.json` into one JSON file per card beside Rust DSL YAML files.

**Architecture:** Add one focused Python tool under `code/tools` with pure helper functions for bucketing, JSON rendering, writing, and checking. Add pytest coverage under `code/tests/tools` using temp directories and injected card dictionaries so tests avoid rewriting the real card tree.

**Tech Stack:** Python 3.11 standard library, pytest, existing `code/tools` CLI pattern.

---

### Task 1: CLI Helper Behavior

**Files:**
- Create: `code/tools/split_cards_json.py`
- Create: `code/tests/tools/test_split_cards_json.py`

- [ ] **Step 1: Write failing helper tests**

Create tests that import `set_id_from_card_id`, `render_card_json`, `write_card_json`, and `cmd_check`. Assert standard set bucketing, `_misc` fallback, stable pretty JSON with a trailing newline, LF-only file output, successful check for matching files, and failed check for missing or stale files.

- [ ] **Step 2: Run helper tests to verify failure**

Run: `python -m pytest code/tests/tools/test_split_cards_json.py -v`

Expected: import failure because `tools.split_cards_json` does not exist yet.

- [ ] **Step 3: Implement minimal helper functions**

Create `code/tools/split_cards_json.py` with:

```python
from __future__ import annotations

import json
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
CARDS_JSON_PATH = _PROJECT_ROOT / "data" / "cards.json"
CARD_OUTPUT_ROOT = _PROJECT_ROOT / "code" / "digimon-engine" / "cards"

def set_id_from_card_id(card_id: str) -> str:
    if "-" not in card_id:
        return "_misc"
    return card_id.split("-", 1)[0].lower()

def render_card_json(card: dict) -> str:
    return json.dumps(card, indent=2, ensure_ascii=False) + "\n"

def write_card_json(card_id: str, card: dict, root: Path) -> Path:
    set_dir = root / set_id_from_card_id(card_id)
    set_dir.mkdir(parents=True, exist_ok=True)
    out = set_dir / f"{card_id}.json"
    out.write_text(render_card_json(card), encoding="utf-8", newline="\n")
    return out
```

Add `cmd_check(cards, root)` to compare rendered content with disk and return `0` or `1`.

- [ ] **Step 4: Run helper tests to verify pass**

Run: `python -m pytest code/tests/tools/test_split_cards_json.py -v`

Expected: all tests pass.

### Task 2: Full CLI Flow

**Files:**
- Modify: `code/tools/split_cards_json.py`
- Modify: `code/tests/tools/test_split_cards_json.py`

- [ ] **Step 1: Write failing CLI-flow tests**

Add tests for filtering by card ID, filtering by set ID, building every injected card, and rejecting an unknown card or empty set with exit code `2`.

- [ ] **Step 2: Run tests to verify failure**

Run: `python -m pytest code/tests/tools/test_split_cards_json.py -v`

Expected: failures for missing build and argument handling functions.

- [ ] **Step 3: Implement CLI flow**

Add `_load_cards_json`, `_all_card_ids`, `_filter_by_set`, `cmd_build`, `main`, and argparse options `--card`, `--set`, and `--check`. Process card IDs in sorted order and print a concise write/check summary.

- [ ] **Step 4: Run focused tests**

Run: `python -m pytest code/tests/tools/test_split_cards_json.py -v`

Expected: all tests pass.

### Task 3: Generate Card JSON Files

**Files:**
- Generate: `code/digimon-engine/cards/<set>/<CARD-ID>.json`

- [ ] **Step 1: Run the generator**

Run: `python code/tools/split_cards_json.py`

Expected: writes one JSON file for every card in `data/cards.json`.

- [ ] **Step 2: Verify generated tree is current**

Run: `python code/tools/split_cards_json.py --check`

Expected: check passes.

- [ ] **Step 3: Run regression tests**

Run: `python -m pytest code/tests/tools/test_split_cards_json.py -v`

Expected: all tests pass.
