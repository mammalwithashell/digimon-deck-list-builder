# LLM Transpilation Lane + Pattern Learner

**Date:** 2026-02-25
**Status:** Approved

## Problem

The regex-based C# to Python transpiler (`tools/transpiler/`) handles common card effect patterns well but produces incomplete or incorrect output for complex cards. The current pipeline — transpile, QA review, autofix — patches individual scripts but doesn't improve the transpiler itself. Major engine mechanics (simultaneous effect ordering, mandatory vs optional activation) are also missing, compounding the gap.

Key friction: context limits when working with AI across engine internals, transpiler code, rules, and ~98 cards per set; coordination across files that must stay consistent.

## Goals

1. Get more card scripts working per set with less manual effort.
2. Use LLM selectively (only where regex fails) to control compute cost.
3. Create a feedback loop so that successful fixes improve the transpiler over time.
4. Build on the existing AI task pipeline — new task types, not new infrastructure.

## Architecture Overview

```
                         ┌──────────────────────────────────┐
                         │        SET RUN PIPELINE           │
                         │  (AISetRunOrchestrator stages)    │
                         └──────────────────────────────────┘
                                        │
  ┌──────────┐    ┌──────────────┐    ┌─┴──────────┐    ┌──────────┐    ┌──────────┐
  │ TRANSPILE │───→│ LLM RETRANS  │───→│  QA/REVIEW  │───→│  AUTOFIX  │───→│  FREEZE   │
  │  (regex)  │    │  (Approach A)│    │ (existing)  │    │ (existing)│    │ (existing)│
  └──────────┘    └──────────────┘    └────────────┘    └─────┬─────┘    └──────────┘
       │                 ▲                                     │
       │                 │                                     │
       └─ confidence ────┘                                     │
          scoring                              ┌───────────────┘
          (deterministic)                      ▼
                                     ┌──────────────────┐
                                     │ PATTERN LEARNER   │
                                     │ (Approach B)      │
                                     │ analyzes fix diffs │
                                     │ → transpiler PRs   │
                                     └──────────────────┘
```

**Approach A** adds LLM retranspilation for low-confidence cards between regex transpile and QA.
**Approach B** adds a post-run learning step that proposes transpiler improvements from successful autofixes.

Both integrate as new task types in the existing dispatcher/worker/orchestrator system.

---

## Approach A: LLM Retranspilation Lane

### Confidence Scoring

After regex transpilation, each card receives a deterministic completeness score. No LLM calls — purely computed from data the transpiler and validator already produce.

**New module:** `tools/transpiler/scoring.py`

```python
def score_card(
    card_id: str,
    effects: list[EffectBlock],
    validation_result: ValidationResult,
    card_meta: dict,
) -> TranspileScore:
```

**Inputs** (all available from existing transpile + validate steps):
- `EffectBlock` list from `parse_cs_file()` — effects extracted, actions mapped.
- `ValidationResult` from `validate_card()` — forward/reverse/timing mismatches.
- Card metadata from `cards.json` — expected effect count from card text.

**Scoring formula:**

| Signal | Weight | Meaning |
|--------|--------|---------|
| `effects_extracted / effects_expected` | 40% | Did the regex find all timing blocks? |
| `actions_mapped / actions_detected` | 30% | Of found effects, did actions resolve? |
| `1 - (forward_mismatches / expected)` | 20% | API says X, script has it? |
| `no_unmapped_coroutines` | 10% | No unresolved shared coroutine delegation? |

**Output:** Float 0.0-1.0. Cards below a configurable threshold (default 0.7) route to LLM retranspilation. Cards above pass straight through to QA.

**`TranspileScore` dataclass:**

```python
@dataclass
class TranspileScore:
    card_id: str
    score: float                    # 0.0-1.0
    effects_ratio: float            # effects_extracted / expected
    actions_ratio: float            # actions_mapped / detected
    forward_match_ratio: float      # 1 - mismatches / expected
    has_unmapped_coroutines: bool
    below_threshold: bool           # Convenience flag
```

### New Task Type: `llm_transpile`

**Contract** (added to `contracts.py`):

```python
class LLMTranspileOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    script_content: str             # Complete Python module source
    effects_implemented: list[str]  # Effect names/timings implemented
    effects_skipped: list[str]      # Effects intentionally skipped (engine gap)
    engine_gaps: list[str]          # Mechanics identified as missing from engine
    reasoning: str                  # Brief rationale for key decisions
```

**Dispatcher payload:**

```python
{
    "card_id": "BT24-042",
    "set_id": "bt24",
    "module_name": "bt24_042",
    "cs_source": "<full C# file contents>",
    "regex_output": "<what the regex transpiler produced>",
    "regex_score": 0.45,
    "score_breakdown": {
        "effects_ratio": 0.4,
        "actions_ratio": 0.5,
        "forward_match_ratio": 0.3,
        "has_unmapped_coroutines": true
    }
}
```

**Context assembled by dispatcher:**

1. **C# source** — the original file (from payload).
2. **Official card text** — from `cards.json`.
3. **Regex transpiler output** — what it managed to produce, so the LLM builds on existing structure rather than starting from scratch.
4. **Engine API reference** — pinned methods extracted from the regex output + card text keywords (using existing `extract_engine_calls()` + `lookup_pinned_engine_methods()`).
5. **Few-shot examples** — 3-5 frozen scripts from the same set that scored > 0.9 (high-confidence regex output that passed QA). Selected by similarity: same keywords, same timings.
6. **Rules context** — RAG retrieval for relevant keywords mentioned in card text.

**Design decision:** The LLM receives the regex output as a starting point. It completes/corrects rather than generating from scratch. This improves quality and reduces hallucination because the structure is partially there.

### Set Run Orchestrator Integration

New stages added to `AISetRunOrchestrator`:

```
Current:  discover → qa → review → fix
Proposed: discover → score → retranspile → qa → review → fix
```

The `score` step is synchronous (no tasks, computation only). The `retranspile` stage creates `llm_transpile` tasks only for cards below threshold. Cards above threshold skip to QA.

**Stage values on `AISetRun`:**

```python
stage: "score" | "retranspile" | "qa" | "review" | "fix" | "completed" | "canceled"
```

**When `llm_transpile` completes:**
- New script replaces the generated file at `scripts/generated/{set}/{module}.py`.
- Score is recalculated for the report.
- Card proceeds to QA/review like any other.

### Cost Profile

Per set (~98 cards), using batch API with a Sonnet-class model:

| Threshold | Cards retranspiled | Estimated cost |
|-----------|-------------------|----------------|
| 0.7 (default) | ~30 cards | $1.50-3.00 |
| 0.5 (conservative) | ~15 cards | $0.75-1.50 |
| 0.9 (aggressive) | ~60 cards | $3.00-6.00 |

Regex-only cards: $0. The threshold is the cost dial.

---

## Approach B: Pattern Learner (Autofix Teaches the Transpiler)

### Trigger

After a set run completes with successful autofixes. Triggered manually via admin UI ("Learn from fixes" button) or API call.

### Phase 1: Diff Clustering (Deterministic)

**New module:** `digimon_gym/ai/pattern_learner.py`

```python
def cluster_autofix_diffs(
    audit_records: list[AIFixApplyAudit],
) -> list[DiffCluster]
```

Reads before/after for each successful autofix and groups by structural similarity:

- **AST-level diffing** — parse old and new Python, diff AST nodes rather than raw text.
- **Cluster by change type** — "added condition guard," "changed action call," "added new effect block," "fixed filter logic."
- **Minimum cluster size** — only surface clusters with 3+ instances. A pattern that repeats is worth automating.

**`DiffCluster` dataclass:**

```python
@dataclass
class DiffCluster:
    description: str                # e.g., "12 cards: added is_my_turn guard on OnPlay"
    change_type: str                # "condition_guard" | "action_call" | "new_effect" | "filter_fix" | ...
    card_ids: list[str]             # Cards in this cluster
    representative_diffs: list[dict]  # 2-3 example before/after pairs
    count: int
```

**Cost:** $0 — pure Python AST diffing.

### Phase 2: Transpiler Patch Generation (LLM)

**New task type:** `transpiler_learn`

**Contract:**

```python
class TranspilerPatchSuggestion(BaseModel):
    target_file: str                # e.g., "tools/transpiler/extractors.py"
    description: str                # What this change does
    before_snippet: str             # Existing code to change
    after_snippet: str              # Proposed replacement
    cards_affected: list[str]       # Card IDs this would fix

class TranspilerLearnOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    cluster_summary: str
    patches: list[TranspilerPatchSuggestion]
    estimated_cards_fixed: int
    confidence: Literal["low", "medium", "high"]
```

**Dispatcher context:**

1. The diff cluster (representative examples, not all N diffs).
2. Current `extractors.py` and `generators.py` source.
3. Current `patterns.py` (regex patterns).
4. C# source for 2-3 representative cards from the cluster.

**Output:** Concrete code suggestions for `patterns.py`, `extractors.py`, or `generators.py`.

### Integration

Does **not** auto-apply. Produces a transpiler improvement PR for human review:

```
Set run completes
  → N successful autofixes recorded in AIFixApplyAudit
  → Admin triggers "Learn from fixes"
  → cluster_autofix_diffs() runs (instant)
  → For each cluster >= 3: create transpiler_learn task
  → Results assembled into draft PR with transpiler patches
  → Human reviews and merges
  → Re-transpile set → fewer cards need LLM/autofix next time
```

**Scope profile:** `script_engine_transpiler` — already exists, allows edits to `tools/transpiler/*.py`.

### The Compound Effect

Each cycle improves the regex transpiler:

```
Set 1: 40% regex pass → 60% LLM retranspile → 20% autofix → learn 5 patterns
Set 2: 55% regex pass → 45% LLM retranspile → 15% autofix → learn 3 patterns
Set 3: 65% regex pass → 35% LLM retranspile → 10% autofix → learn 2 patterns
```

### Cost Profile

- Phase 1 (clustering): $0
- Phase 2 (patch generation): ~$0.50-1.00 per cluster
- Typical run: 3-6 clusters, $1.50-6.00
- Triggered manually, not per-card

---

## Data Model Changes

### New columns on `AISetRun`

```python
score_threshold: Float          # e.g., 0.7
retranspile_total: Integer      # Cards below threshold
retranspile_completed: Integer
retranspile_failed: Integer
```

### New column on `AISetRunItem`

```python
transpile_score: Float          # 0.0-1.0, computed after regex transpile
retranspile_task_id: String     # Links to llm_transpile task (null if above threshold)
```

### New table: `AITranspilerLearnRun`

```python
class AITranspilerLearnRun:
    id: String (UUID)
    source_set_run_id: String       # Which set run's fixes we're learning from
    status: String                  # "clustering" | "generating" | "completed" | "failed"
    clusters_found: Integer
    patches_proposed: Integer
    pr_url: String                  # Draft PR with transpiler changes
    created_at: DateTime
    completed_at: DateTime
```

---

## Admin API Additions

### Set run creation (modified)

```
POST /admin/set-runs
{
    "set_id": "bt25",
    "run_mode": "pr",
    "scope_profile": "script",
    "score_threshold": 0.7,          // NEW - default 0.7
    "model_name": null,
    ...
}
```

### Trigger transpiler learning (new)

```
POST /admin/transpiler-learn
{
    "source_set_run_id": "<uuid>",
    "min_cluster_size": 3
}

Response: { "learn_run_id": "...", "clusters_found": 5 }
```

### View learning results (new)

```
GET /admin/transpiler-learn/{learn_run_id}

Response: {
    "status": "completed",
    "clusters": [
        { "description": "12 cards: missing is_my_turn guard", "patch_count": 1 },
        ...
    ],
    "pr_url": "https://github.com/.../pull/55"
}
```

---

## Admin UI Changes

**AdminSetRunPage** (set run detail view):
- Score distribution column on the items table (per-card transpile score).
- Count badge: "32 retranspiled / 98 total".
- After run completes: "Learn from fixes" button triggers `POST /admin/transpiler-learn`.

**AdminTasksPage** — no changes. `llm_transpile` and `transpiler_learn` tasks appear like any other task type.

**AdminPromotionsPage** — no changes. Transpiler patches go through normal PR review.

---

## New Files

| File | Purpose |
|------|---------|
| `tools/transpiler/scoring.py` | Deterministic confidence scoring |
| `digimon_gym/ai/pattern_learner.py` | Diff clustering + learn run orchestration |
| `digimon_gym/ai/contracts.py` | +`LLMTranspileOutput`, +`TranspilerLearnOutput`, +`TranspilerPatchSuggestion` |
| `digimon_gym/ai/dispatcher.py` | +`llm_transpile` and `transpiler_learn` dispatch branches |
| `digimon_gym/ai/set_run_orchestrator.py` | +`score` and `retranspile` stages |
| `digimon_gym/db/models.py` | +columns on AISetRun/AISetRunItem, +AITranspilerLearnRun |
| `alembic/versions/...` | Migration for schema changes |

## Unchanged Files

- `tools/transpiler/extractors.py`, `generators.py`, `patterns.py` — Approach B proposes changes via PR, doesn't auto-edit.
- `digimon_gym/ai/autofix_apply.py` — existing apply logic works as-is.
- `digimon_gym/ai/worker.py` — no changes; new task types picked up via dispatcher routing.
- Frozen manifest / promotion flow — unchanged.

---

## End-to-End Flow for a New Set

```
1. Ingest card metadata                    (existing)
2. Regex transpile all ~98 cards           (existing)
3. Score each card                         (NEW - deterministic, instant)
4. LLM retranspile cards below threshold   (NEW - ~30 cards, ~$2-3)
5. QA → Review → Autofix                  (existing pipeline)
6. Freeze passing cards                    (existing)
7. "Learn from fixes" → transpiler PR      (NEW - manual trigger, ~$3)
8. Merge transpiler PR                     (human review)
9. Next set benefits from improved regex
```
