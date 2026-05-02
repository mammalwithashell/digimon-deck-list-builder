# Phase 3: Cut Callers Over to `digimon_engine` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate every production caller of `digimon_gym.engine.data.*` and adjacent surfaces to the Phase-2 `digimon_engine` PyO3 bindings. Anything that hits a binding gap stays on the Python engine import for now and is logged in `docs/RUST_PYTHON_PARITY.md` so Phase 4 (move Python engine to `engine_py_legacy/`) knows what's still depending on it.

**Architecture:** Mechanical import-rewrite per file. The Rust bindings expose drop-in replacements for the import groups Phase 2 wrapped (CardDatabase, deck_loader, enums, tested_cards, CardRegistry, get_models_dir, load_implemented_card_ids). For each call site, swap the dotted path and re-run the relevant tests. Files that import surfaces Phase 2 didn't wrap (`HeadlessGame`, `InteractiveGame`, `ScenarioRunner`, `tensor_layout`, engine internals like `Permanent`/`Player`, `onnx_policy`, `script_promotion`, `card_features`) keep their existing imports — they migrate in later phases.

**Tech Stack:** Python 3.11, pytest, FastAPI smoke, the `digimon_engine` PyO3 module from Phase 2.

**Spec:** `docs/superpowers/specs/2026-04-25-server-digimon-gym-split-design.md` (Phase 3).

---

## Migration Coverage Matrix

| Old import | New import | Files affected |
|---|---|---|
| `digimon_gym.engine.data.card_database.CardDatabase` | `digimon_engine.CardDatabase` | api.py, deck_tools, deck_optimizer, debug_games, architect_env, architect_explain, architect_pool, deck_pool, generate_scenarios, resolve_deck |
| `digimon_gym.engine.data.card_registry.CardRegistry` | `digimon_engine.CardRegistry` | api.py, generate_scenarios |
| `digimon_gym.engine.data.card_registry.{REGISTRY_CAPACITY,EMBEDDING_DIM}` | `digimon_engine.{REGISTRY_CAPACITY,EMBEDDING_DIM}` | features_extractor |
| `digimon_gym.engine.data.deck_loader.{parse_deck,parse_tts,parse_text,validate_deck,expand_deck_dict,RESTRICTED_LIST}` | `digimon_engine.{parse_deck,parse_tts,parse_text,validate_deck,expand_deck_dict,restricted_list}` | deck_tools, matchmaking, gauntlet, pilot_training, deck_pool, architect_pool, run_training_job, meta_loader, games |
| `digimon_gym.engine.data.enums.{CardKind,GamePhase}` | `digimon_engine.{CardKind,GamePhase}` | architect_env, architect_pool, deck_pool, deck_optimizer, deck_tools, generate_scenarios, resolve_deck, debug_games (partial) |
| `digimon_gym.engine.data.tested_cards.{load_tested_cards,out_of_set_cards}` | `digimon_engine.{load_tested_cards,out_of_set_cards}` | deck_tools |
| `digimon_gym.engine.data.deck_finder.load_implemented_card_ids` | `digimon_engine.load_implemented_card_ids` | architect_explain, architect_optimizer, architect_pool, store_night |
| `digimon_gym.engine.model_utils.get_models_dir` | `digimon_engine.get_models_dir` | admin_models, games (partial — `list_onnx_models`/`resolve_model_path` stay Python) |
| `digimon_gym.engine.game.ACTION_SPACE_SIZE` (or `engine.game.constants`) | `digimon_engine.ACTION_SPACE_SIZE` (added in Task 2) | pilot_training, export_onnx, export_random_onnx |
| `digimon_gym.engine.game.constants.TENSOR_SIZE` | `digimon_engine.TENSOR_SIZE` (added in Task 2) | export_onnx, export_random_onnx |

## Stays on Python Engine (Logged in Parity Doc, Task 11)

These surfaces are not covered by Phase 2 and are not added in this phase. Their callers keep importing from `digimon_gym.engine.*` and the parity doc tracks the gap.

| Surface | Reason | Rust counterpart? |
|---|---|---|
| `runners.headless_game.HeadlessGame` (Python class) | Different class shape from `RustHeadlessGame`; callers that rely on Python state inspection (e.g., `state.py` router, `recordings.py` router) keep using it. The RL gym (`digimon_gym.py`) already swaps via `DIGIMON_BACKEND`. | `digimon_engine.RustHeadlessGame` (already exposed; per-caller migration is non-trivial state-shape work). |
| `runners.interactive_game.InteractiveGame` | Server-side PvP/debug game driver. The PvP bindings plan ([2026-04-18-pyo3-pvp-bindings.md](2026-04-18-pyo3-pvp-bindings.md)) covers `RustHeadlessGame` with PvP-friendly methods, but the `InteractiveGame` adapter shape isn't 1:1. | Pending PvP plan completion. |
| `runners.replay_runner.ReplayRunner` | Server replay-playback driver. | Pending future Rust replay runner. |
| `runners.scenario_runner.ScenarioRunner` | Tooling — `run_scenario.py`, `run_qa_batch.py`, behavioral test infrastructure. Sunset alongside the Python engine. | Not planned (DebugRunner-based scenarios are a Rust-side parallel). |
| `data.tensor_layout.*` | Tensor offset module used by `features_extractor.py`. | Could be added later if RL trainer survives the Rust-only world; for now an explicit gap. |
| `data.enums.{PendingAction, PlayerType}` | `PendingAction` is vestigial; `PlayerType` is server orchestration. | Out-of-scope per Phase 2. |
| `data.card_features.CardFeatureVectorizer` | Used by `tools/train_card_autoencoder.py`. Tool is RL-training-side and may stay Python-only. | Not planned. |
| `data.script_promotion.*` | Python card-script lane. Sunset alongside Python engine. | Not planned (Rust card scripts are hand-written, no promotion flow). |
| `engine.onnx_policy.load_onnx_policy` | ONNX policy loader. Phase 5 moves it to `digimon_gym/inference/onnx_policy.py`. | Stays Python-side (numpy/onnxruntime). |
| `engine.core.{permanent.Permanent, player.Player, card_source.CardSource}` | Engine internal types used by `engine.debug.state_injection`. | Engine-internal — not appropriate to expose. |
| `engine.events.GameEvent` (Python) | Used by `engine/loggers.py` (engine-internal). | `digimon_engine` exposes events via `RustHeadlessGame.get_events_since_last_step` already. |
| `data.card_database.parse_xros_req, parse_digixros_req` | Used by `tools/ingest_cards.py`. Could be wrapped if needed. | Could add later. |

---

## File Map

**Files modified (production code):**

Server-side:
- `digimon_gym/api.py` — CardDatabase, CardRegistry init in lifespan.
- `digimon_gym/routers/deck_tools.py` — full migration (CardDatabase, deck_loader, enums, tested_cards).
- `digimon_gym/routers/deck_optimizer.py` — CardDatabase, CardKind.
- `digimon_gym/routers/matchmaking.py` — partial: RESTRICTED_LIST, validate_deck migrate; InteractiveGame stays.
- `digimon_gym/routers/games.py` — partial: parse_deck, get_models_dir migrate; HeadlessGame/InteractiveGame/PlayerType/list_onnx_models/resolve_model_path stay.
- `digimon_gym/db/routers/admin_models.py` — get_models_dir.

RL-side:
- `digimon_gym/digimon_gym.py` — partial: HeadlessGame, GamePhase migrate; PendingAction usage redesigned around Rust selection state.
- `digimon_gym/agents/architect_env.py` — CardDatabase, CardKind.
- `digimon_gym/agents/architect_explain.py` — CardDatabase, load_implemented_card_ids.
- `digimon_gym/agents/architect_optimizer.py` — load_implemented_card_ids.
- `digimon_gym/agents/architect_pool.py` — CardDatabase, load_implemented_card_ids, CardKind, RESTRICTED_LIST.
- `digimon_gym/agents/deck_pool.py` — CardDatabase, expand_deck_dict, validate_deck, CardKind.
- `digimon_gym/agents/gauntlet.py` — parse_tts.
- `digimon_gym/agents/pilot_training.py` — ACTION_SPACE_SIZE, parse_deck.
- `digimon_gym/agents/features_extractor.py` — partial: REGISTRY_CAPACITY/EMBEDDING_DIM migrate; tensor_layout stays.
- `digimon_gym/agents/architect_simulator.py` — partial: HeadlessGame, onnx_policy stay.

Tools:
- `tools/generate_scenarios.py` — CardDatabase, CardRegistry, CardKind.
- `tools/resolve_deck.py` — CardDatabase, CardKind.
- `tools/run_training_job.py` — parse_tts, parse_text.
- `tools/store_night.py` — load_implemented_card_ids.
- `tools/meta_loader.py` — partial (depends on which deck_loader exports it uses; check at task time).
- `tools/export_onnx.py` — ACTION_SPACE_SIZE, TENSOR_SIZE.
- `tools/export_random_onnx.py` — ACTION_SPACE_SIZE, TENSOR_SIZE.

**Files modified (bindings + docs):**
- `digimon-engine-py/src/lib.rs` — add `ACTION_SPACE_SIZE` + `TENSOR_SIZE` constants in Task 2.
- `tests/engine/test_rust_bindings_surface.py` — add tests for the Task 2 additions.
- `docs/RUST_PYTHON_PARITY.md` — append a Phase-3-residue section with the gaps listed above.

**Files NOT modified:**
- `digimon_gym/routers/{state,recordings,debug_games}.py` — heavy users of `HeadlessGame`/`InteractiveGame`/`ReplayRunner`/`state_injection` (parity doc).
- `digimon_gym/engine/*` — internal Python engine, sunset in Phase 4.
- `tools/{run_scenario,run_qa_batch,promote_script,train_card_autoencoder,ingest_cards}.py` — heavy users of Python-engine surfaces with no Phase 2 binding.
- All tests under `tests/` — none migrate in this phase. Tests for the Python engine stay on Python imports until Phase 4 moves them to `engine_py_legacy/`.

---

## Task 1: Pre-flight + parity-doc skeleton

**Files:**
- Create or modify: `docs/RUST_PYTHON_PARITY.md` — append the Phase-3-residue section header with empty body (filled in Task 11).

- [ ] **Step 1: Confirm bindings still load**

Run: `python -c "import digimon_engine; print(sorted(n for n in dir(digimon_engine) if not n.startswith('_')))"`

Expected: 22 exports including CardDatabase, CardRegistry, parse_deck, validate_deck, CardKind, GamePhase, etc. If any expected export is missing, Phase 2 wasn't built into the active env — re-run `maturin build --release` + `pip install --force-reinstall --no-deps target/wheels/digimon_engine-*.whl`.

- [ ] **Step 2: Capture pre-Phase-3 baseline**

Run: `python -m pytest tests --ignore=tests/ai_pipeline --ignore=tests/api/test_admin_models.py 2>&1 | tail -5`

Expected: same failure count as the Phase 1/2 baseline (39 pre-existing Windows-charmap and rust-backend-parity failures). Record this number — Phase 3's verification compares to it.

- [ ] **Step 3: Append parity-doc skeleton**

Open `docs/RUST_PYTHON_PARITY.md`. Append (or update if a similar section exists):

```markdown
## Phase 3 residue (callers still on Python engine)

These imports survived the Phase 3 cutover because the Rust counterpart
isn't in `digimon_engine` yet. Each entry is a checklist: when the
binding lands, remove the Python import and the row.

| Surface | Caller(s) | Status |
|---|---|---|

(filled in Task 11 once all Phase 3 migrations land)
```

- [ ] **Step 4: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md
git commit -m "$(cat <<'EOF'
docs(parity): scaffold Phase 3 residue tracker

Empty section that Task 11 of the Phase 3 plan fills with the
gap inventory once migrations land.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Phase 2 follow-on bindings — ACTION_SPACE_SIZE + TENSOR_SIZE

The Rust crate exports both constants (`pub use action::build_action_mask;` and `pub use tensor::{build_tensor, TENSOR_SIZE};`). They're not yet exposed at the Python module level. Add them so `pilot_training`, `export_onnx`, `export_random_onnx` can stop importing from `digimon_gym.engine.game.constants`.

**Files:**
- Modify: `digimon-engine-py/src/lib.rs`.
- Modify: `tests/engine/test_rust_bindings_surface.py`.

- [ ] **Step 1: Find the Rust constants**

Run: `grep -E "ACTION_SPACE_SIZE|^pub const TENSOR_SIZE" digimon-engine/src/action/*.rs digimon-engine/src/tensor.rs digimon-engine/src/lib.rs`

Expected: confirms both constants exist as `pub const` in the Rust crate. Record their types (likely `u16` or `usize` for `ACTION_SPACE_SIZE`, `usize` for `TENSOR_SIZE`).

- [ ] **Step 2: Write failing tests**

Append to `tests/engine/test_rust_bindings_surface.py`:

```python
class TestActionAndTensorConstants:
    def test_action_space_size(self):
        from digimon_engine import ACTION_SPACE_SIZE
        # Per ACTION_SPEC.md: 2168 entries.
        assert ACTION_SPACE_SIZE == 2168

    def test_tensor_size(self):
        from digimon_engine import TENSOR_SIZE
        # Per TENSOR_SPEC.md: 1375 floats.
        assert TENSOR_SIZE == 1375
```

If `ACTION_SPACE_SIZE` or `TENSOR_SIZE` differ from the values above (the spec docs may have shifted), update the assertions to match what `digimon-engine/src/{action,tensor}.rs` actually defines. The point is the constant matches the Rust source of truth.

- [ ] **Step 3: Run to confirm failure**

Run: `python -m pytest tests/engine/test_rust_bindings_surface.py::TestActionAndTensorConstants -v`

Expected: ImportError on each.

- [ ] **Step 4: Add the bindings**

In `digimon-engine-py/src/lib.rs`, add the imports near the existing crate imports:

```rust
use ::digimon_engine::action::ACTION_SPACE_SIZE;
use ::digimon_engine::tensor::TENSOR_SIZE;
```

(If the path is different — e.g., `::digimon_engine::ACTION_SPACE_SIZE` because it's re-exported at crate root — use that.)

In the `#[pymodule]` registration block:

```rust
m.add("ACTION_SPACE_SIZE", ACTION_SPACE_SIZE)?;
m.add("TENSOR_SIZE", TENSOR_SIZE)?;
```

- [ ] **Step 5: Build, install, run**

```bash
cd digimon-engine-py && python -m maturin build --release && cd ..
pip install --force-reinstall --no-deps target/wheels/digimon_engine-0.1.0-cp311-abi3-win_amd64.whl
python -m pytest tests/engine/test_rust_bindings_surface.py::TestActionAndTensorConstants -v
```

Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add digimon-engine-py/src/lib.rs tests/engine/test_rust_bindings_surface.py
git commit -m "$(cat <<'EOF'
feat(rust-bindings): expose ACTION_SPACE_SIZE + TENSOR_SIZE

Phase 3 follow-on: pilot_training, export_onnx, export_random_onnx
need these constants to migrate off digimon_gym.engine.game.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Migrate `digimon_gym/api.py` (server bootstrap)

**Files:**
- Modify: `digimon_gym/api.py`.

- [ ] **Step 1: Update imports**

Open `digimon_gym/api.py`. Replace lines 15–16:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.card_registry import CardRegistry
```

with:

```python
from digimon_engine import CardDatabase, CardRegistry
```

- [ ] **Step 2: Audit lifespan usage**

The lifespan function calls `CardDatabase()` and `CardRegistry.ensure_initialized()` (per the existing api.py). The Rust `CardDatabase` constructor takes no args, matching. The Rust `CardRegistry` doesn't have `ensure_initialized()` — its constructor `CardRegistry()` IS the initialization. Replace:

```python
CardRegistry.ensure_initialized()
```

with:

```python
CardRegistry()  # touches the static loader, mirrors Python's ensure_initialized
```

- [ ] **Step 3: Smoke-import the module**

Run: `PYTHONIOENCODING=utf-8 python -c "from digimon_gym.api import app; print('app loads OK')"`

Expected: `app loads OK` with no errors.

- [ ] **Step 4: Commit**

```bash
git add digimon_gym/api.py
git commit -m "$(cat <<'EOF'
refactor(api): cut CardDatabase + CardRegistry over to digimon_engine

Bootstrap calls now go through the PyO3 bindings.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Migrate engine-only routers

**Files:**
- Modify: `digimon_gym/routers/deck_tools.py`.
- Modify: `digimon_gym/routers/deck_optimizer.py`.
- Modify: `digimon_gym/routers/matchmaking.py`.
- Modify: `digimon_gym/routers/games.py`.
- Modify: `digimon_gym/db/routers/admin_models.py`.

- [ ] **Step 1: `deck_tools.py`**

Replace the four imports at the top of `digimon_gym/routers/deck_tools.py`:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.deck_loader import parse_deck, summarize_deck, validate_deck
from digimon_gym.engine.data.enums import CardKind
from digimon_gym.engine.data.tested_cards import load_tested_cards, out_of_set_cards
```

with:

```python
from digimon_engine import (
    CardDatabase,
    CardKind,
    load_tested_cards,
    out_of_set_cards,
    parse_deck,
    summarize_deck,
    validate_deck,
)
```

If the file uses `validate_deck` and accesses `.is_valid`/`.errors`/`.warnings` on the result, the new `PyDeckValidationResult` exposes the same fields — no behavior change. If it accesses any other field, audit and adjust.

Run: `python -c "from digimon_gym.routers import deck_tools; print('ok')"`

Expected: `ok`.

- [ ] **Step 2: `deck_optimizer.py`**

Lines 154–155 (deferred imports inside a function):

```python
    from digimon_gym.engine.data.card_database import CardDatabase
    from digimon_gym.engine.data.enums import CardKind
```

Replace with:

```python
    from digimon_engine import CardDatabase, CardKind
```

- [ ] **Step 3: `matchmaking.py`**

Line 40:

```python
from digimon_gym.engine.data.deck_loader import RESTRICTED_LIST, validate_deck
```

Replace with:

```python
from digimon_engine import restricted_list, validate_deck
```

Then update every usage of `RESTRICTED_LIST` in the file. The Rust binding exposes `restricted_list()` as a function (not a constant), and the returned object has `.card_limits` / `.choice_groups` like the Python `CardRestriction`. Update each call site:

```python
# Before
RESTRICTED_LIST.card_limits.get(card_id)

# After
restricted_list().card_limits.get(card_id)
```

If `RESTRICTED_LIST` is referenced more than ~3 times, store the result once: `_restricted = restricted_list()` near the imports, and reference `_restricted.card_limits` etc.

Line 41 (`from digimon_gym.engine.runners.interactive_game import InteractiveGame`) **stays** — it's an `# noqa: F401` re-export pattern and `InteractiveGame` is in the parity-doc residue.

- [ ] **Step 4: `games.py`**

Lines 14–18:

```python
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.data.deck_loader import parse_deck
from digimon_gym.engine.model_utils import get_models_dir, list_onnx_models, resolve_model_path
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
```

Replace `parse_deck` and `get_models_dir`:

```python
from digimon_gym.engine.data.enums import PlayerType
from digimon_gym.engine.model_utils import list_onnx_models, resolve_model_path
from digimon_gym.engine.runners.headless_game import HeadlessGame
from digimon_gym.engine.runners.interactive_game import InteractiveGame
from digimon_engine import get_models_dir, parse_deck
```

`PlayerType`, `list_onnx_models`, `resolve_model_path`, `HeadlessGame`, `InteractiveGame` stay on Python — parity-doc residue. `get_models_dir` migrates because Phase 2 covered it.

- [ ] **Step 5: `admin_models.py`**

Line 31 (in `digimon_gym/db/routers/admin_models.py`):

```python
from digimon_gym.engine.model_utils import get_models_dir
```

Replace with:

```python
from digimon_engine import get_models_dir
```

If the router also uses `list_onnx_models` or `resolve_model_path` from `model_utils`, those stay on Python (parity-doc) — split the import:

```python
from digimon_gym.engine.model_utils import list_onnx_models, resolve_model_path
from digimon_engine import get_models_dir
```

Run a grep to confirm the file's actual usage before committing:
```bash
grep -E "list_onnx_models|resolve_model_path|get_models_dir" digimon_gym/db/routers/admin_models.py
```

- [ ] **Step 6: Smoke + commit**

```bash
PYTHONIOENCODING=utf-8 python -c "from digimon_gym.api import app; print('app loads OK')"
python -m pytest tests/api -v --ignore=tests/api/test_admin_models.py 2>&1 | tail -10
```

Expected: app loads, API tests pass (or fail in pre-existing patterns only — compare against Task 1 baseline).

```bash
git add digimon_gym/routers/deck_tools.py digimon_gym/routers/deck_optimizer.py \
        digimon_gym/routers/matchmaking.py digimon_gym/routers/games.py \
        digimon_gym/db/routers/admin_models.py
git commit -m "$(cat <<'EOF'
refactor(routers): migrate engine-only routers to digimon_engine

CardDatabase, deck_loader (parse_deck, validate_deck, summarize_deck,
restricted_list), enums (CardKind), tested_cards, get_models_dir all
go through the PyO3 bindings now. PlayerType, HeadlessGame,
InteractiveGame, list_onnx_models, resolve_model_path stay on the
Python engine (parity-doc residue).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Migrate architect agents

**Files:**
- Modify: `digimon_gym/agents/architect_env.py`.
- Modify: `digimon_gym/agents/architect_explain.py`.
- Modify: `digimon_gym/agents/architect_optimizer.py`.
- Modify: `digimon_gym/agents/architect_pool.py`.
- (`digimon_gym/agents/architect_simulator.py` is **partial** — it imports `HeadlessGame` and `onnx_policy`, both staying on Python. Skip this file.)

- [ ] **Step 1: `architect_env.py`**

Lines 23–24:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import CardKind
```

→

```python
from digimon_engine import CardDatabase, CardKind
```

- [ ] **Step 2: `architect_explain.py`**

Line 23 (top-level) and line 437 (in-function deferred):

```python
from digimon_gym.engine.data.card_database import CardDatabase
# ...
        from digimon_gym.engine.data.deck_finder import load_implemented_card_ids
```

→

```python
from digimon_engine import CardDatabase
# ...
        from digimon_engine import load_implemented_card_ids
```

- [ ] **Step 3: `architect_optimizer.py`**

Line 29:

```python
from digimon_gym.engine.data.deck_finder import load_implemented_card_ids
```

→

```python
from digimon_engine import load_implemented_card_ids
```

- [ ] **Step 4: `architect_pool.py`**

Lines 26–28 (top) plus lines 94 and 132 (deferred imports of `RESTRICTED_LIST`):

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.deck_finder import load_implemented_card_ids
from digimon_gym.engine.data.enums import CardKind
```

→

```python
from digimon_engine import CardDatabase, CardKind, load_implemented_card_ids
```

For the deferred `RESTRICTED_LIST` imports (lines 94 and 132):

```python
            from digimon_gym.engine.data.deck_loader import RESTRICTED_LIST
```

→

```python
            from digimon_engine import restricted_list as _restricted_list_fn
            RESTRICTED_LIST = _restricted_list_fn()
```

(Wrap the function call once at the call site so the local variable `RESTRICTED_LIST` keeps its same shape — `.card_limits` / `.choice_groups`.)

- [ ] **Step 5: Smoke**

Run: `python -c "from digimon_gym.agents import architect_env, architect_explain, architect_optimizer, architect_pool; print('ok')"`

Expected: `ok`.

- [ ] **Step 6: Commit**

```bash
git add digimon_gym/agents/architect_env.py digimon_gym/agents/architect_explain.py \
        digimon_gym/agents/architect_optimizer.py digimon_gym/agents/architect_pool.py
git commit -m "$(cat <<'EOF'
refactor(agents): migrate architect_* to digimon_engine

CardDatabase, CardKind, load_implemented_card_ids, restricted_list
all go through the PyO3 bindings.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Migrate `deck_pool.py` and `gauntlet.py`

**Files:**
- Modify: `digimon_gym/agents/deck_pool.py`.
- Modify: `digimon_gym/agents/gauntlet.py`.

- [ ] **Step 1: `deck_pool.py`**

Lines 29–31:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.deck_loader import expand_deck_dict, validate_deck
from digimon_gym.engine.data.enums import CardKind
```

→

```python
from digimon_engine import CardDatabase, CardKind, expand_deck_dict, validate_deck
```

- [ ] **Step 2: `gauntlet.py`**

Line 45:

```python
from digimon_gym.engine.data.deck_loader import parse_tts
```

→

```python
from digimon_engine import parse_tts
```

- [ ] **Step 3: Smoke + commit**

```bash
python -c "from digimon_gym.agents import deck_pool, gauntlet; print('ok')"
git add digimon_gym/agents/deck_pool.py digimon_gym/agents/gauntlet.py
git commit -m "$(cat <<'EOF'
refactor(agents): migrate deck_pool + gauntlet to digimon_engine

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Migrate `pilot_training.py` and `features_extractor.py` (partial)

**Files:**
- Modify: `digimon_gym/agents/pilot_training.py`.
- Modify: `digimon_gym/agents/features_extractor.py`.

- [ ] **Step 1: `pilot_training.py`**

Line 33:

```python
from digimon_gym.engine.game import ACTION_SPACE_SIZE
```

→

```python
from digimon_engine import ACTION_SPACE_SIZE
```

Line 798 (deferred import):

```python
        from digimon_gym.engine.data.deck_loader import parse_deck
```

→

```python
        from digimon_engine import parse_deck
```

- [ ] **Step 2: `features_extractor.py`** (partial migration)

Line 25:

```python
from digimon_gym.engine.data.card_registry import REGISTRY_CAPACITY, EMBEDDING_DIM
```

→

```python
from digimon_engine import REGISTRY_CAPACITY, EMBEDDING_DIM
```

Line 22 (`from digimon_gym.engine.data.tensor_layout import (...)`) **stays** on the Python engine — parity-doc residue.

- [ ] **Step 3: Smoke + commit**

```bash
python -c "from digimon_gym.agents import pilot_training; print('ok')"
python -c "from digimon_gym.agents import features_extractor; print('ok')"
git add digimon_gym/agents/pilot_training.py digimon_gym/agents/features_extractor.py
git commit -m "$(cat <<'EOF'
refactor(agents): migrate pilot_training + features_extractor (partial)

ACTION_SPACE_SIZE, parse_deck, REGISTRY_CAPACITY, EMBEDDING_DIM go
through digimon_engine. tensor_layout stays on the Python engine
(parity-doc residue).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Migrate `digimon_gym.py` (gym env)

The gym env is the largest single migration. It currently dispatches between Python `HeadlessGame` and `RustHeadlessGame` via the `DIGIMON_BACKEND` env var. The migration here is narrow: replace direct enum imports (`PendingAction`, `GamePhase`) with the bindings versions where possible.

**Files:**
- Modify: `digimon_gym/digimon_gym.py`.

- [ ] **Step 1: Inspect the file's enum usage**

Run: `grep -nE "PendingAction\.|GamePhase\." digimon_gym/digimon_gym.py | head -30`

Confirm every usage of `GamePhase.X`. The Phase 2 `digimon_engine.GamePhase` covers all 22 standard variants. `PendingAction.TRASH_CARD` is the only non-trivial Python-engine-specific enum value (see Phase 2 deferral notes).

- [ ] **Step 2: Decide what to do with `PendingAction`**

Read the surrounding code at the `PendingAction.TRASH_CARD` reference:

```bash
grep -nB 5 -A 10 "PendingAction\." digimon_gym/digimon_gym.py
```

If the check is `if game.pending_action == PendingAction.TRASH_CARD`, the Rust backend exposes selection state via `RustHeadlessGame.get_pending_selection()` instead. The migration:

- The gym env runs in headless mode; trash-card is a sub-selection inside an effect resolution. With the Rust backend, `pending_selection['kind']` will identify the prompt (e.g. `"SelectTrash"`), not a `PendingAction` enum.
- For the env's reward/transition logic, the *only* thing that matters is whether the next step is a regular phase action vs. an effect-driven prompt. That's already encoded in `get_action_mask()` — the mask is empty in non-decision states and otherwise valid.

**Decision:** drop the `PendingAction` import and the conditional that uses it. If keeping the conditional is necessary because removing it changes env transitions for the Python-backend path, gate the conditional behind `if not _USING_RUST_BACKEND:` so the Python path keeps working through Phase 4.

A safer alternative: keep the import for the Python path, but only on that path:

```python
# Before
from digimon_gym.engine.data.enums import PendingAction, GamePhase

# After
from digimon_engine import GamePhase
# PendingAction is Python-engine only; imported lazily on the Python path.
```

Then inside the function that uses `PendingAction.TRASH_CARD`:

```python
if not _USING_RUST_BACKEND:
    from digimon_gym.engine.data.enums import PendingAction
    if game.pending_action == PendingAction.TRASH_CARD:
        # ... existing Python-path-only logic
```

- [ ] **Step 3: Replace the imports**

```python
# Before (line 16, 44, 48)
from digimon_gym.engine.runners.headless_game import HeadlessGame
# ...
from digimon_gym.engine.game import (
    # ... a multi-line import of game-level helpers
)
# ...
from digimon_gym.engine.data.enums import PendingAction, GamePhase
```

The `HeadlessGame` import stays — the env still falls back to it when `DIGIMON_BACKEND` is unset. The `digimon_gym.engine.game import (...)` block depends on what's inside; most likely engine-internal helpers that stay on Python. Keep them.

Replace just the enum line:

```python
from digimon_engine import GamePhase
```

(Drop `PendingAction` from the top-level import — handle it lazily on the Python path per Step 2.)

- [ ] **Step 4: Run the env smoke**

```bash
python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

Expected: `(1375,) (2168,)` (or whatever the current shapes are — they should not change).

Then run with the Rust backend:

```bash
DIGIMON_BACKEND=rust python -c "from digimon_gym.digimon_gym import DigimonEnv; env=DigimonEnv(); obs,info=env.reset(); print(obs.shape, info['action_mask'].shape)"
```

Expected: same shapes.

- [ ] **Step 5: Commit**

```bash
git add digimon_gym/digimon_gym.py
git commit -m "$(cat <<'EOF'
refactor(gym): migrate GamePhase to digimon_engine

Top-level GamePhase imports go through the PyO3 bindings.
PendingAction stays on the Python engine — it's only used on the
Python backend path and is gated behind _USING_RUST_BACKEND.
HeadlessGame import stays for the Python fallback path.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Migrate retained `tools/*` scripts

**Files:**
- Modify: `tools/generate_scenarios.py`.
- Modify: `tools/resolve_deck.py`.
- Modify: `tools/run_training_job.py`.
- Modify: `tools/store_night.py`.
- Modify: `tools/meta_loader.py`.
- Modify: `tools/export_onnx.py`.
- Modify: `tools/export_random_onnx.py`.

- [ ] **Step 1: `generate_scenarios.py`**

Lines 23–25:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.card_registry import CardRegistry
from digimon_gym.engine.data.enums import CardKind
```

→

```python
from digimon_engine import CardDatabase, CardKind, CardRegistry
```

- [ ] **Step 2: `resolve_deck.py`**

Lines 34–35:

```python
from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import CardKind
```

→

```python
from digimon_engine import CardDatabase, CardKind
```

- [ ] **Step 3: `run_training_job.py`**

Lines 27 and 41 (deferred imports):

```python
    from digimon_gym.engine.data.deck_loader import parse_tts, parse_text
# ...
    from digimon_gym.engine.data.deck_loader import parse_tts
```

→

```python
    from digimon_engine import parse_tts, parse_text
# ...
    from digimon_engine import parse_tts
```

- [ ] **Step 4: `store_night.py`**

Line 165 (deferred):

```python
    from digimon_gym.engine.data.deck_finder import load_implemented_card_ids
```

→

```python
    from digimon_engine import load_implemented_card_ids
```

- [ ] **Step 5: `meta_loader.py`**

Line 43 (multi-name import from `deck_loader`):

```bash
grep -A 6 "from digimon_gym.engine.data.deck_loader import" tools/meta_loader.py
```

For each imported name, check coverage. Phase 2 covers `parse_tts`, `parse_text`, `parse_deck`, `summarize_deck`, `validate_deck`, `expand_deck_dict`, `restricted_list`. Anything else in the import list (e.g., a Python-only helper) stays — split the import:

```python
# After (example — actual list depends on what meta_loader imports)
from digimon_engine import parse_deck, summarize_deck
from digimon_gym.engine.data.deck_loader import _python_only_helper  # parity-doc
```

- [ ] **Step 6: `export_onnx.py` and `export_random_onnx.py`**

In both files, line 20/27:

```python
from digimon_gym.engine.game.constants import ACTION_SPACE_SIZE, TENSOR_SIZE
```

→

```python
from digimon_engine import ACTION_SPACE_SIZE, TENSOR_SIZE
```

The `from digimon_gym.engine.onnx_policy import load_onnx_policy` lines stay (parity-doc — `onnx_policy` moves to `digimon_gym/inference/onnx_policy.py` in Phase 5, not here).

- [ ] **Step 7: Smoke each tool's import**

```bash
for f in tools/generate_scenarios.py tools/resolve_deck.py tools/run_training_job.py \
         tools/store_night.py tools/meta_loader.py tools/export_onnx.py \
         tools/export_random_onnx.py; do
    python -c "import ast; ast.parse(open('$f').read()); print('$f: parses')"
done
```

Each should print `<file>: parses`. (We don't actually run the tools — many require argparse args. Parsing is enough to catch import-line typos.)

- [ ] **Step 8: Commit**

```bash
git add tools/generate_scenarios.py tools/resolve_deck.py tools/run_training_job.py \
        tools/store_night.py tools/meta_loader.py tools/export_onnx.py \
        tools/export_random_onnx.py
git commit -m "$(cat <<'EOF'
refactor(tools): migrate retained tools/* to digimon_engine

CardDatabase, CardRegistry, CardKind, parse_tts/parse_text/parse_deck,
load_implemented_card_ids, ACTION_SPACE_SIZE, TENSOR_SIZE all go
through the PyO3 bindings. Tools using ScenarioRunner, script_promotion,
CardFeatureVectorizer, parse_xros_req, or onnx_policy are unchanged
(parity-doc residue).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Migrated-caller verification

**Files:** none (verification only).

- [ ] **Step 1: Repo-wide check that migrated paths are gone**

For each migrated symbol, confirm zero remaining production-code matches:

```bash
grep -rn "from digimon_gym\.engine\.data\.card_database import" --include="*.py" \
     digimon_gym/api.py digimon_gym/routers/ digimon_gym/agents/ \
     digimon_gym/db/routers/admin_models.py tools/
```

Expected: zero matches.

Repeat for each migrated module path:
- `digimon_gym\.engine\.data\.card_registry` (in non-features-extractor code)
- `digimon_gym\.engine\.data\.deck_loader` (any of the migrated names)
- `digimon_gym\.engine\.data\.enums` (CardKind, GamePhase only — PendingAction/PlayerType still allowed)
- `digimon_gym\.engine\.data\.tested_cards`
- `digimon_gym\.engine\.data\.deck_finder`
- `digimon_gym\.engine\.model_utils.*get_models_dir`

If any match remains in a file the plan said should migrate, fix it now and amend the relevant task's commit (or add a follow-up commit).

- [ ] **Step 2: Server boot smoke**

```bash
PYTHONIOENCODING=utf-8 python -c "from digimon_gym.api import app; print('app loads OK')"
```

- [ ] **Step 3: Default pytest**

```bash
python -m pytest tests --ignore=tests/ai_pipeline --ignore=tests/api/test_admin_models.py 2>&1 | tail -5
```

Expected: failure count matches the Task 1 baseline (39 pre-existing). No new failures introduced by Phase 3.

- [ ] **Step 4: ai_pipeline pytest**

```bash
python -m pytest tests/ai_pipeline 2>&1 | tail -3
```

Expected: same 7 baseline-existing failures from Phase 1.

- [ ] **Step 5: Cargo tests still green**

```bash
cargo test --manifest-path digimon-engine/Cargo.toml --lib 2>&1 | tail -3
```

Expected: `67 passed` (or whatever the current count is — must match pre-Phase-3 number).

---

## Task 11: Update parity doc

**Files:**
- Modify: `docs/RUST_PYTHON_PARITY.md`.

- [ ] **Step 1: Fill in the residue table**

Open `docs/RUST_PYTHON_PARITY.md`. Replace the empty table from Task 1 with:

```markdown
## Phase 3 residue (callers still on Python engine)

These imports survived the Phase 3 cutover because the Rust counterpart
isn't in `digimon_engine` yet. Each entry is a checklist: when the
binding lands, remove the Python import and the row.

| Surface | Caller(s) | Rust counterpart? |
|---|---|---|
| `engine.runners.headless_game.HeadlessGame` | `routers/state.py`, `routers/recordings.py`, `routers/games.py`, `digimon_gym.py` (Python fallback path), `agents/architect_simulator.py` | `RustHeadlessGame` — different state shape; per-caller migration is non-trivial. |
| `engine.runners.interactive_game.InteractiveGame` | `routers/games.py`, `routers/debug_games.py`, `routers/matchmaking.py` | Pending — covered by the PvP bindings plan ([2026-04-18-pyo3-pvp-bindings.md](2026-04-18-pyo3-pvp-bindings.md)). |
| `engine.runners.replay_runner.ReplayRunner` | `routers/recordings.py` | Not planned. |
| `engine.runners.scenario_runner.ScenarioRunner` | `tools/run_scenario.py`, `tools/run_qa_batch.py` | Not planned (DebugRunner is the Rust-side parallel). |
| `engine.data.tensor_layout.*` | `agents/features_extractor.py` | Not planned in scope. Fold-in if RL trainer survives. |
| `engine.data.enums.PendingAction` | `digimon_gym.py` (Python fallback path) | Vestigial. Will be removed when the Python backend is retired. |
| `engine.data.enums.PlayerType` | `routers/games.py`, `routers/debug_games.py` | Server orchestration, not engine. Stays Python-side. |
| `engine.data.card_features.CardFeatureVectorizer` | `tools/train_card_autoencoder.py` | Not planned. RL-training-side tool; may stay Python-only. |
| `engine.data.script_promotion.*` | `tools/promote_script.py`, `tools/archive/bootstrap_frozen_manifest.py` | Sunset — Python script lane is going away. Tools delete in Phase 4. |
| `engine.onnx_policy.load_onnx_policy` | `routers/games.py`, `agents/architect_simulator.py`, `tools/export_random_onnx.py` | Stays Python-side. Phase 5 moves it to `digimon_gym/inference/onnx_policy.py`. |
| `engine.core.{permanent,player,card_source}` | `engine/debug/state_injection.py` | Engine internals. Don't expose. |
| `engine.events.GameEvent` (Python class) | `engine/loggers.py` | Python-engine internal. `digimon_engine` exposes events via `RustHeadlessGame.get_events_since_last_step`. |
| `engine.data.card_database.parse_xros_req, parse_digixros_req` | `tools/ingest_cards.py` | Could be wrapped if needed. Low priority. |
| `engine.debug.state_injection.*` | `routers/debug_games.py` | Engine-internal scenario builder. Not appropriate to expose. |
```

- [ ] **Step 2: Commit**

```bash
git add docs/RUST_PYTHON_PARITY.md
git commit -m "$(cat <<'EOF'
docs(parity): fill in Phase 3 residue tracker

Inventory of every digimon_gym.engine.* import that survived the
Phase 3 cutover, with caller list and disposition.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Out-of-Scope

Explicitly **not** done in this phase:

- Migrating `routers/state.py`, `routers/recordings.py`, `routers/debug_games.py`. They use `HeadlessGame`/`InteractiveGame`/`ReplayRunner`/`state_injection`. PvP-plan-adjacent or engine-internal.
- Migrating any `tests/*` file. Tests for the Python engine stay on Python imports until Phase 4 moves them to `engine_py_legacy/`.
- Touching `digimon_gym/engine/*` itself.
- Removing `digimon_gym.engine.data.enums.PendingAction` or `PlayerType`. Those stay until Phase 4 / Phase 5.
- Adding a `tensor_layout` PyO3 binding. That's its own design decision — RL trainer-specific.
- Renaming any module. Phase 5 does the `server/` extract.

## Risks and Mitigations

- **`restricted_list()` is now a function call, not a constant.** Mitigation: every caller migration includes a "find usages" grep step. Over-zealous string replace could call the function many times in a tight loop; cache the result at the module level if needed.
- **Behavior drift between Python `validate_deck` and Rust `validate_deck`.** Both should produce identical error messages and `is_valid` flags per the Rust port doc. If a caller depends on a specific error string format that differs, the test will catch it. Address per case.
- **`PendingAction` gating is fragile.** The lazy-import + `_USING_RUST_BACKEND` flag keeps both paths working. If the gym env is ever exercised on the Rust path during a `PendingAction.TRASH_CARD` check at runtime, it'll silently no-op the conditional. Acceptable in alpha; flagged in the parity doc.
- **CI cache poisoning.** Phase 2's wheel install was local. CI environments will rebuild from `digimon-engine-py/` source. Verify that CI's existing `maturin build` step is still in `.github/workflows/`. If not, add it as a Phase 3 follow-up.

## Plan complete

After Task 11 ships green: open the Phase 3 PR. Phase 4 (move Python engine + tests to `engine_py_legacy/`) follows.
