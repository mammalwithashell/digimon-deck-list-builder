# LLM Transpilation Lane + Pattern Learner — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add LLM retranspilation for low-confidence cards and a feedback loop that learns transpiler patterns from successful autofixes.

**Architecture:** Two new task types (`llm_transpile`, `transpiler_learn`) plug into the existing dispatcher/worker/orchestrator system. A deterministic scoring module gates which cards need LLM retranspilation. A pattern learner clusters autofix diffs and proposes transpiler improvements.

**Tech Stack:** Python, SQLAlchemy, Alembic, Pydantic, FastAPI, existing AI dispatcher + worker infrastructure.

**Design doc:** `docs/plans/2026-02-25-llm-transpiler-and-pattern-learner-design.md`

---

### Task 1: Confidence Scoring Module

**Files:**
- Create: `tools/transpiler/scoring.py`
- Create: `tests/test_transpiler_scoring.py`
- Modify: `tools/transpiler/__init__.py` (add export)

**Step 1: Write failing tests**

```python
# tests/test_transpiler_scoring.py
from dataclasses import dataclass
from tools.transpiler.scoring import score_card, TranspileScore


def _make_effects(count, actions_per=1, has_unmapped=False):
    """Build minimal EffectBlock-like objects for scoring."""
    from tools.transpiler.models import EffectBlock
    effects = []
    for i in range(count):
        eb = EffectBlock()
        eb.actions = [f"action_{j}" for j in range(actions_per)]
        if has_unmapped and i == 0:
            eb.actions = []  # Simulate unmapped coroutine
        effects.append(eb)
    return effects


def _make_validation(forward=0, reverse=0, timing=0):
    """Build minimal ValidationResult-like object."""
    from tools.transpiler.validation import ValidationResult
    vr = ValidationResult()
    vr.card_id = "TEST-001"
    vr.forward_issues = [f"issue_{i}" for i in range(forward)]
    vr.reverse_issues = [f"issue_{i}" for i in range(reverse)]
    vr.timing_issues = [f"issue_{i}" for i in range(timing)]
    return vr


def _make_card_meta(effect_count=3):
    """Build minimal card metadata with expected effect count."""
    # Card text with effect_count worth of keyword lines
    lines = ["[On Play] Draw 1." for _ in range(effect_count)]
    return {"card_id": "TEST-001", "effect": "\n".join(lines)}


class TestScoreCard:
    def test_perfect_score(self):
        effects = _make_effects(3, actions_per=2)
        vr = _make_validation(forward=0)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert isinstance(result, TranspileScore)
        assert result.score >= 0.9
        assert result.below_threshold is False

    def test_zero_effects_scores_low(self):
        effects = []
        vr = _make_validation(forward=3)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert result.score < 0.3
        assert result.below_threshold is True

    def test_partial_extraction(self):
        effects = _make_effects(1, actions_per=1)
        vr = _make_validation(forward=2)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert 0.2 < result.score < 0.7

    def test_unmapped_coroutines_penalize(self):
        effects_clean = _make_effects(3, actions_per=2)
        effects_unmapped = _make_effects(3, actions_per=2, has_unmapped=True)
        vr = _make_validation(forward=0)
        meta = _make_card_meta(effect_count=3)
        clean = score_card("TEST-001", effects_clean, vr, meta)
        unmapped = score_card("TEST-001", effects_unmapped, vr, meta)
        assert clean.score > unmapped.score

    def test_custom_threshold(self):
        effects = _make_effects(2, actions_per=1)
        vr = _make_validation(forward=1)
        meta = _make_card_meta(effect_count=3)
        low_bar = score_card("TEST-001", effects, vr, meta, threshold=0.3)
        high_bar = score_card("TEST-001", effects, vr, meta, threshold=0.95)
        assert low_bar.below_threshold is False or high_bar.below_threshold is True

    def test_card_with_no_expected_effects(self):
        """Vanilla cards (no effects in text) should score 1.0."""
        effects = []
        vr = _make_validation(forward=0)
        meta = {"card_id": "TEST-001", "effect": ""}
        result = score_card("TEST-001", effects, vr, meta)
        assert result.score == 1.0

    def test_result_dataclass_fields(self):
        effects = _make_effects(2, actions_per=1)
        vr = _make_validation(forward=1)
        meta = _make_card_meta(effect_count=3)
        result = score_card("TEST-001", effects, vr, meta)
        assert hasattr(result, "card_id")
        assert hasattr(result, "score")
        assert hasattr(result, "effects_ratio")
        assert hasattr(result, "actions_ratio")
        assert hasattr(result, "forward_match_ratio")
        assert hasattr(result, "has_unmapped_coroutines")
        assert hasattr(result, "below_threshold")
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_transpiler_scoring.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'tools.transpiler.scoring'`

**Step 3: Implement scoring module**

```python
# tools/transpiler/scoring.py
"""Deterministic confidence scoring for transpiled card scripts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, List

if TYPE_CHECKING:
    from tools.transpiler.models import EffectBlock
    from tools.transpiler.validation import ValidationResult

# Weights for scoring formula
W_EFFECTS = 0.40
W_ACTIONS = 0.30
W_FORWARD = 0.20
W_COROUTINE = 0.10

DEFAULT_THRESHOLD = 0.7


@dataclass
class TranspileScore:
    card_id: str
    score: float
    effects_ratio: float
    actions_ratio: float
    forward_match_ratio: float
    has_unmapped_coroutines: bool
    below_threshold: bool


def _count_expected_effects(card_meta: dict) -> int:
    """Estimate expected effect count from card text.

    Counts distinct effect lines/keywords in the card's effect field.
    Returns 0 for vanilla cards with no effect text.
    """
    effect_text = (card_meta.get("effect") or "").strip()
    if not effect_text:
        return 0
    # Count lines that contain timing keywords or effect text
    lines = [ln.strip() for ln in effect_text.split("\n") if ln.strip()]
    return max(len(lines), 1)


def score_card(
    card_id: str,
    effects: List["EffectBlock"],
    validation_result: "ValidationResult",
    card_meta: dict,
    threshold: float = DEFAULT_THRESHOLD,
) -> TranspileScore:
    """Score a transpiled card for completeness.

    Returns a TranspileScore with a 0.0-1.0 score.
    Cards scoring below *threshold* have below_threshold=True.
    """
    expected = _count_expected_effects(card_meta)

    # Vanilla card — no effects expected, nothing to transpile
    if expected == 0:
        return TranspileScore(
            card_id=card_id,
            score=1.0,
            effects_ratio=1.0,
            actions_ratio=1.0,
            forward_match_ratio=1.0,
            has_unmapped_coroutines=False,
            below_threshold=False,
        )

    # Effects ratio: how many timing blocks did the regex find?
    extracted = len(effects)
    effects_ratio = min(extracted / expected, 1.0)

    # Actions ratio: of found effects, how many have mapped actions?
    total_actions = sum(len(eb.actions) for eb in effects)
    detected_slots = max(extracted, 1)  # avoid division by zero
    actions_ratio = min(total_actions / detected_slots, 1.0) if extracted > 0 else 0.0

    # Forward match ratio: 1 - (forward mismatches / expected)
    forward_issues = len(validation_result.forward_issues)
    forward_match_ratio = max(1.0 - (forward_issues / expected), 0.0)

    # Unmapped coroutines: any effect with zero actions despite being non-factory?
    has_unmapped = any(
        len(eb.actions) == 0 and not eb.is_factory
        for eb in effects
    )
    coroutine_score = 0.0 if has_unmapped else 1.0

    score = (
        W_EFFECTS * effects_ratio
        + W_ACTIONS * actions_ratio
        + W_FORWARD * forward_match_ratio
        + W_COROUTINE * coroutine_score
    )
    score = round(max(0.0, min(1.0, score)), 4)

    return TranspileScore(
        card_id=card_id,
        score=score,
        effects_ratio=round(effects_ratio, 4),
        actions_ratio=round(actions_ratio, 4),
        forward_match_ratio=round(forward_match_ratio, 4),
        has_unmapped_coroutines=has_unmapped,
        below_threshold=score < threshold,
    )
```

**Step 4: Update `tools/transpiler/__init__.py`**

Add `from .scoring import score_card, TranspileScore` to the existing exports.

**Step 5: Run tests to verify they pass**

Run: `python -m pytest tests/test_transpiler_scoring.py -v`
Expected: All 7 tests PASS

**Step 6: Commit**

```bash
git add tools/transpiler/scoring.py tests/test_transpiler_scoring.py tools/transpiler/__init__.py
git commit -m "feat: add deterministic confidence scoring for transpiled cards"
```

---

### Task 2: LLMTranspileOutput Contract

**Files:**
- Modify: `digimon_gym/ai/contracts.py` (add new Pydantic model)
- Modify: `tests/test_ai_pipeline.py` (add contract validation test)

**Step 1: Write failing test**

```python
# Add to tests/test_ai_pipeline.py or create tests/test_contracts.py
from digimon_gym.ai.contracts import LLMTranspileOutput


class TestLLMTranspileOutput:
    def test_valid_output(self):
        out = LLMTranspileOutput(
            script_content="class BT24_001(CardScript): ...",
            effects_implemented=["OnPlay Draw 1"],
            effects_skipped=[],
            engine_gaps=[],
            reasoning="All effects mapped.",
        )
        assert out.script_content.startswith("class")

    def test_rejects_extra_fields(self):
        import pytest
        with pytest.raises(Exception):
            LLMTranspileOutput(
                script_content="...",
                effects_implemented=[],
                effects_skipped=[],
                engine_gaps=[],
                reasoning="ok",
                rogue_field="bad",
            )
```

**Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_contracts.py::TestLLMTranspileOutput -v`
Expected: FAIL — `ImportError: cannot import name 'LLMTranspileOutput'`

**Step 3: Add model to contracts.py**

Add after the existing `ScriptAutofixOutput` class in `digimon_gym/ai/contracts.py`:

```python
class LLMTranspileOutput(BaseModel):
    """Output from LLM-based retranspilation of a card script."""

    model_config = ConfigDict(extra="forbid")

    script_content: str
    effects_implemented: list[str]
    effects_skipped: list[str]
    engine_gaps: list[str]
    reasoning: str
```

**Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_contracts.py::TestLLMTranspileOutput -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/contracts.py tests/test_contracts.py
git commit -m "feat: add LLMTranspileOutput contract for retranspilation tasks"
```

---

### Task 3: LLM Transpile Prompt Builder

**Files:**
- Modify: `digimon_gym/ai/prompts.py` (add `build_llm_transpile_messages`)
- Create: `tests/test_prompts_llm_transpile.py`

**Step 1: Write failing test**

```python
# tests/test_prompts_llm_transpile.py
from digimon_gym.ai.prompts import build_llm_transpile_messages


class TestBuildLLMTranspileMessages:
    def test_returns_system_and_user(self):
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2 cards.",
            cs_source="public class BT24_042 : CEntity_Effect { ... }",
            regex_output="class BT24_042(CardScript): ...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert isinstance(system, str)
        assert isinstance(user, str)
        assert len(system) > 50
        assert "BT24-042" in user

    def test_includes_cs_source_in_user(self):
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2.",
            cs_source="public class BT24_042 : CEntity_Effect { MARKER }",
            regex_output="class BT24_042(CardScript): ...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert "MARKER" in user

    def test_includes_few_shot_examples(self):
        examples = [{"card_id": "BT24-001", "script": "class BT24_001(CardScript): pass"}]
        system, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="[On Play] Draw 2.",
            cs_source="...",
            regex_output="...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=examples,
        )
        assert "BT24-001" in user

    def test_includes_regex_score(self):
        _, user = build_llm_transpile_messages(
            card_id="BT24-042",
            card_text="effect",
            cs_source="...",
            regex_output="...",
            regex_score=0.45,
            context_chunks=[],
            pinned_engine_chunks=None,
            few_shot_examples=[],
        )
        assert "0.45" in user
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_prompts_llm_transpile.py -v`
Expected: FAIL — `ImportError: cannot import name 'build_llm_transpile_messages'`

**Step 3: Implement prompt builder**

Add to `digimon_gym/ai/prompts.py`:

```python
def build_llm_transpile_messages(
    *,
    card_id: str,
    card_text: str,
    cs_source: str,
    regex_output: str,
    regex_score: float,
    context_chunks: list[dict],
    pinned_engine_chunks: list[dict] | None,
    few_shot_examples: list[dict],
) -> tuple[str, str]:
    """Build system + user prompts for LLM retranspilation."""
    context_block = _join_context_chunks(context_chunks)
    pinned_block = _join_pinned_engine_chunks(pinned_engine_chunks) if pinned_engine_chunks else ""

    examples_block = ""
    if few_shot_examples:
        parts = []
        for ex in few_shot_examples:
            parts.append(f"### {ex['card_id']}\n```python\n{ex['script']}\n```")
        examples_block = "## Working Examples from This Set\n\n" + "\n\n".join(parts)

    system = (
        "You are a Digimon TCG card script transpiler. You convert C# card effect "
        "classes from DCGO into Python CardScript subclasses for a headless game engine.\n\n"
        "You will receive:\n"
        "1. The original C# source file\n"
        "2. The official card text\n"
        "3. A partial Python output from a regex-based transpiler (with a confidence score)\n"
        "4. Engine API documentation for available methods\n"
        "5. Working examples of similar cards from the same set\n\n"
        "Your job: produce a COMPLETE, CORRECT Python CardScript module that faithfully "
        "implements the card's effects using the available engine API. Build on the regex "
        "output where it is correct. Fix or rewrite parts that are wrong or missing.\n\n"
        "Rules:\n"
        "- Output the complete Python module (imports through class definition)\n"
        "- Use only engine methods shown in the API reference\n"
        "- If an effect requires an engine mechanic that doesn't exist, list it in engine_gaps and skip that effect\n"
        "- Follow the exact CardScript structure shown in the working examples\n"
        "- Do not invent engine methods\n"
    )

    user = (
        f"## Card: {card_id}\n\n"
        f"**Official card text:**\n{card_text}\n\n"
        f"## C# Source\n```csharp\n{cs_source}\n```\n\n"
        f"## Regex Transpiler Output (score: {regex_score})\n```python\n{regex_output}\n```\n\n"
    )
    if examples_block:
        user += f"{examples_block}\n\n"
    if pinned_block:
        user += f"## Engine API Reference\n{pinned_block}\n\n"
    if context_block:
        user += f"## Rules & Context\n{context_block}\n\n"
    user += (
        "Produce the complete corrected Python module. List any effects you "
        "skipped and any engine gaps you identified."
    )

    return system, user
```

**Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_prompts_llm_transpile.py -v`
Expected: All 4 tests PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/prompts.py tests/test_prompts_llm_transpile.py
git commit -m "feat: add prompt builder for LLM retranspilation"
```

---

### Task 4: LLM Transpile Dispatcher Branch

**Files:**
- Modify: `digimon_gym/ai/dispatcher.py` (add `_run_llm_transpile` method + route)
- Modify: `tests/test_dispatcher.py` (add dispatch routing test)

**Step 1: Write failing test**

```python
# Add to tests/test_dispatcher.py
class TestLLMTranspileDispatch:
    def test_rejects_missing_card_id(self):
        from digimon_gym.ai.dispatcher import TaskDispatcher
        d = TaskDispatcher(rag_index=None, client=None)
        with pytest.raises(ValueError, match="card_id"):
            d.run("llm_transpile", {"set_id": "bt24", "module_name": "bt24_042"})

    def test_unknown_task_type_raises(self):
        from digimon_gym.ai.dispatcher import TaskDispatcher
        d = TaskDispatcher(rag_index=None, client=None)
        with pytest.raises(ValueError, match="Unsupported"):
            d.run("nonexistent_type", {})
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_dispatcher.py::TestLLMTranspileDispatch -v`
Expected: FAIL — `test_rejects_missing_card_id` fails because `llm_transpile` is not a recognized task type yet (raises "Unsupported" instead of "card_id")

**Step 3: Add dispatch branch**

In `digimon_gym/ai/dispatcher.py`, add to the `run()` method:

```python
if task_type == "llm_transpile":
    return self._run_llm_transpile(payload, model_name=model_name)
```

Add new method (follow the pattern of `_run_script_autofix`):

```python
def _run_llm_transpile(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
    card_id = str(payload.get("card_id", "")).strip().upper()
    set_id = str(payload.get("set_id", "")).strip().lower()
    module_name = str(payload.get("module_name", "")).strip().lower()
    cs_source = str(payload.get("cs_source", "")).strip()
    regex_output = str(payload.get("regex_output", "")).strip()
    regex_score = float(payload.get("regex_score", 0.0))

    if not card_id or not set_id or not module_name:
        raise ValueError("llm_transpile payload must include card_id, set_id, module_name")

    card_meta = self.cards_index.get(card_id, {})
    card_text = _card_text_from_meta(card_meta)

    # Pin engine methods from the regex output
    engine_method_names = extract_engine_calls(regex_output) if regex_output else []
    pinned_chunks = (
        lookup_pinned_engine_methods(self.rag_index, engine_method_names)
        if self.rag_index and engine_method_names
        else []
    )

    # RAG context
    query = f"{card_id} {card_text[:300]}"
    context = (
        self.rag_index.retrieve(query, k=6, source_types=["engine_api", "rules"])
        if self.rag_index
        else []
    )

    # Few-shot: load frozen scripts from same set that exist
    few_shot_examples = _load_few_shot_examples(set_id, limit=5)

    system_prompt, user_prompt = build_llm_transpile_messages(
        card_id=card_id,
        card_text=card_text,
        cs_source=cs_source,
        regex_output=regex_output,
        regex_score=regex_score,
        context_chunks=context,
        pinned_engine_chunks=pinned_chunks or None,
        few_shot_examples=few_shot_examples,
    )

    run = self.client.run_structured(
        task_type="llm_transpile",
        system_prompt=system_prompt,
        user_prompt=user_prompt,
        schema_model=LLMTranspileOutput,
        model_name=model_name,
    )

    all_refs = [{"source": c.get("source", ""), "chunk_id": c.get("chunk_id", "")} for c in context]
    return DispatchOutcome(
        model_name=run.model_name,
        result=run.output,
        sanitized_input={"card_id": card_id, "set_id": set_id, "module_name": module_name, "regex_score": regex_score},
        retrieval_refs=all_refs,
        input_tokens=run.input_tokens,
        output_tokens=run.output_tokens,
        cost_actual=self._cost_from_usage(run.model_name, run.input_tokens, run.output_tokens),
    )
```

Add helper function:

```python
def _load_few_shot_examples(set_id: str, limit: int = 5) -> list[dict]:
    """Load frozen scripts from the same set as few-shot examples."""
    import json
    manifest_path = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts" / "_frozen_manifest.json"
    if not manifest_path.exists():
        return []
    manifest = json.loads(manifest_path.read_text())
    examples = []
    for card in manifest.get("cards", {}).values():
        if card.get("set_id") != set_id:
            continue
        if not card.get("frozen_hash"):
            continue
        frozen_path = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts" / card["frozen_relpath"]
        if frozen_path.exists() and len(examples) < limit:
            examples.append({
                "card_id": card["card_id"],
                "script": frozen_path.read_text(),
            })
    return examples
```

**Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_dispatcher.py::TestLLMTranspileDispatch -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/dispatcher.py tests/test_dispatcher.py
git commit -m "feat: add llm_transpile dispatch branch with few-shot examples"
```

---

### Task 5: Database Migration — Retranspile Columns + Learn Run Table

**Files:**
- Modify: `digimon_gym/db/models.py` (add columns + new table)
- Modify: `digimon_gym/db/schemas.py` (update request/response schemas)
- Create: `alembic/versions/20260225_0010_retranspile_and_learn_run.py`

**Step 1: Add columns to AISetRun in models.py**

After the existing `max_fix_tasks` column (~line 440):

```python
# Retranspile configuration and counters
score_threshold = Column(Float, nullable=True)
retranspile_total = Column(Integer, nullable=False, default=0)
retranspile_completed = Column(Integer, nullable=False, default=0)
retranspile_failed = Column(Integer, nullable=False, default=0)
```

**Step 2: Add columns to AISetRunItem in models.py**

After the existing `review_faithful` column (~line 490):

```python
transpile_score = Column(Float, nullable=True)
retranspile_task_id = Column(String, ForeignKey("ai_tasks.id", ondelete="SET NULL"), nullable=True)
```

Add relationship:

```python
retranspile_task = relationship("AITask", foreign_keys=[retranspile_task_id])
```

**Step 3: Add AITranspilerLearnRun table in models.py**

After the AIFixApplyAudit class:

```python
class AITranspilerLearnRun(Base):
    __tablename__ = "ai_transpiler_learn_runs"

    id = Column(String, primary_key=True, default=_new_uuid)
    source_set_run_id = Column(String, ForeignKey("ai_set_runs.id", ondelete="SET NULL"), nullable=True)
    status = Column(String, nullable=False, default="clustering")
    clusters_found = Column(Integer, nullable=False, default=0)
    patches_proposed = Column(Integer, nullable=False, default=0)
    pr_url = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    completed_at = Column(DateTime(timezone=True), nullable=True)

    source_set_run = relationship("AISetRun")
```

**Step 4: Update schemas.py**

Update `AISetRunCreateRequest` — add:

```python
score_threshold: Optional[float] = Field(None, ge=0.0, le=1.0)
```

Update `AISetRunResponse` — add:

```python
score_threshold: Optional[float] = None
retranspile_total: int = 0
retranspile_completed: int = 0
retranspile_failed: int = 0
```

Update `AISetRunItemResponse` — add:

```python
transpile_score: Optional[float] = None
retranspile_task_id: Optional[str] = None
```

Add new schemas:

```python
class AITranspilerLearnCreateRequest(BaseModel):
    source_set_run_id: str
    min_cluster_size: int = Field(3, ge=1, le=50)

class AITranspilerLearnResponse(BaseModel):
    id: str
    source_set_run_id: Optional[str] = None
    status: str
    clusters_found: int
    patches_proposed: int
    pr_url: Optional[str] = None
    created_at: datetime
    completed_at: Optional[datetime] = None
```

**Step 5: Create migration**

Create `alembic/versions/20260225_0010_retranspile_and_learn_run.py`:

```python
"""Add retranspile columns and learn run table."""

revision = "20260225_0010"
down_revision = "20260225_0009"
branch_labels = None
depends_on = None

import sqlalchemy as sa
from alembic import op


def _has_table(name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    return name in inspector.get_table_names()


def _has_column(table_name: str, column_name: str) -> bool:
    bind = op.get_bind()
    inspector = sa.inspect(bind)
    columns = [col["name"] for col in inspector.get_columns(table_name)]
    return column_name in columns


def upgrade() -> None:
    # AISetRun columns
    if not _has_column("ai_set_runs", "score_threshold"):
        op.add_column("ai_set_runs", sa.Column("score_threshold", sa.Float, nullable=True))
    if not _has_column("ai_set_runs", "retranspile_total"):
        op.add_column("ai_set_runs", sa.Column("retranspile_total", sa.Integer, nullable=False, server_default="0"))
    if not _has_column("ai_set_runs", "retranspile_completed"):
        op.add_column("ai_set_runs", sa.Column("retranspile_completed", sa.Integer, nullable=False, server_default="0"))
    if not _has_column("ai_set_runs", "retranspile_failed"):
        op.add_column("ai_set_runs", sa.Column("retranspile_failed", sa.Integer, nullable=False, server_default="0"))

    # AISetRunItem columns
    if not _has_column("ai_set_run_items", "transpile_score"):
        op.add_column("ai_set_run_items", sa.Column("transpile_score", sa.Float, nullable=True))
    if not _has_column("ai_set_run_items", "retranspile_task_id"):
        op.add_column("ai_set_run_items", sa.Column("retranspile_task_id", sa.String, nullable=True))

    # AITranspilerLearnRun table
    if not _has_table("ai_transpiler_learn_runs"):
        op.create_table(
            "ai_transpiler_learn_runs",
            sa.Column("id", sa.String, primary_key=True),
            sa.Column("source_set_run_id", sa.String, sa.ForeignKey("ai_set_runs.id", ondelete="SET NULL"), nullable=True),
            sa.Column("status", sa.String, nullable=False, server_default="clustering"),
            sa.Column("clusters_found", sa.Integer, nullable=False, server_default="0"),
            sa.Column("patches_proposed", sa.Integer, nullable=False, server_default="0"),
            sa.Column("pr_url", sa.String, nullable=True),
            sa.Column("created_at", sa.DateTime(timezone=True), nullable=False),
            sa.Column("completed_at", sa.DateTime(timezone=True), nullable=True),
        )


def downgrade() -> None:
    op.drop_table("ai_transpiler_learn_runs")
    op.drop_column("ai_set_run_items", "retranspile_task_id")
    op.drop_column("ai_set_run_items", "transpile_score")
    op.drop_column("ai_set_runs", "retranspile_failed")
    op.drop_column("ai_set_runs", "retranspile_completed")
    op.drop_column("ai_set_runs", "retranspile_total")
    op.drop_column("ai_set_runs", "score_threshold")
```

**Step 6: Run migration**

Run: `python -m alembic upgrade head`
Expected: Migration applies cleanly

**Step 7: Commit**

```bash
git add digimon_gym/db/models.py digimon_gym/db/schemas.py alembic/versions/20260225_0010_retranspile_and_learn_run.py
git commit -m "feat: add retranspile columns and transpiler learn run table"
```

---

### Task 6: Set Run Orchestrator — Score + Retranspile Stages

**Files:**
- Modify: `digimon_gym/ai/set_run_orchestrator.py` (add score/retranspile stages)
- Create: `tests/test_set_run_retranspile.py`

**Step 1: Write failing test**

```python
# tests/test_set_run_retranspile.py
"""Tests for the retranspile stage in set run orchestrator."""
import pytest
from unittest.mock import patch, MagicMock
from digimon_gym.ai.set_run_orchestrator import AISetRunOrchestrator


class TestDiscoverAndScore:
    def test_score_stage_sets_transpile_scores(self):
        """After scoring, items should have transpile_score populated."""
        # This test will be fleshed out once we know the exact method signature.
        # For now, test that the orchestrator has a _score_cards method.
        orch = AISetRunOrchestrator()
        assert hasattr(orch, "_score_cards")

    def test_retranspile_stage_creates_tasks_for_low_scores(self):
        """Cards below threshold should get llm_transpile tasks."""
        orch = AISetRunOrchestrator()
        assert hasattr(orch, "_queue_retranspile_tasks")
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_set_run_retranspile.py -v`
Expected: FAIL — `_score_cards` attribute not found

**Step 3: Implement score and retranspile stages**

Modify `digimon_gym/ai/set_run_orchestrator.py`:

1. Update `create_set_run()` to accept `score_threshold` parameter and set `stage="score"` when a threshold is provided.

2. Add `_score_cards()` method:

```python
def _score_cards(
    self,
    items: list[AISetRunItem],
    set_id: str,
    threshold: float,
    cs_dir: str | None = None,
) -> list[AISetRunItem]:
    """Compute transpile scores for all items. Synchronous, no LLM calls."""
    from tools.transpiler.scoring import score_card
    from tools.transpiler.extractors import parse_cs_file
    from tools.transpiler.validation import validate_card
    import json

    cards_json = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "cards.json"
    card_db = json.loads(cards_json.read_text()) if cards_json.exists() else {}

    for item in items:
        gen_path = GENERATED_SCRIPTS_ROOT / item.set_id / f"{item.module_name}.py"
        if not gen_path.exists():
            item.transpile_score = 0.0
            continue

        card_meta = card_db.get(item.card_id, {})
        # Parse the generated script to get EffectBlock data
        # We need the original C# to get EffectBlocks, or re-parse the Python
        # For scoring, use validation results as the primary signal
        vr = validate_card(item.card_id, card_meta, gen_path.read_text())

        # Attempt to load EffectBlocks if C# source available
        effects = []
        if cs_dir:
            cs_path = _find_cs_file(cs_dir, item.module_name)
            if cs_path:
                _, effects = parse_cs_file(str(cs_path))

        result = score_card(item.card_id, effects, vr, card_meta, threshold=threshold)
        item.transpile_score = result.score

    return items
```

3. Add `_queue_retranspile_tasks()` method:

```python
async def _queue_retranspile_tasks(
    self, db: AsyncSession, run: AISetRun, items: list[AISetRunItem], cs_dir: str | None
) -> int:
    """Create llm_transpile tasks for items below score threshold."""
    threshold = run.score_threshold or 0.7
    count = 0
    for item in items:
        if item.transpile_score is not None and item.transpile_score < threshold:
            # Load C# source if available
            cs_source = ""
            if cs_dir:
                cs_path = _find_cs_file(cs_dir, item.module_name)
                if cs_path:
                    cs_source = cs_path.read_text()

            # Load regex output
            gen_path = GENERATED_SCRIPTS_ROOT / item.set_id / f"{item.module_name}.py"
            regex_output = gen_path.read_text() if gen_path.exists() else ""

            task = AITask(
                task_type="llm_transpile",
                status="queued",
                set_run_id=run.id,
                set_id=item.set_id,
                payload_json=json.dumps({
                    "card_id": item.card_id,
                    "set_id": item.set_id,
                    "module_name": item.module_name,
                    "cs_source": cs_source,
                    "regex_output": regex_output,
                    "regex_score": item.transpile_score,
                }),
                max_attempts=2,
            )
            db.add(task)
            await db.flush()
            item.retranspile_task_id = task.id
            count += 1

    run.retranspile_total = count
    run.stage = "retranspile" if count > 0 else "qa"
    await db.commit()
    return count
```

4. Add handler for `llm_transpile` task completion in `on_task_finished()`:

```python
# In on_task_finished(), add before the existing qa_analysis check:
if task.task_type == "llm_transpile":
    await self._handle_retranspile_task_finished(db, run, task)
```

```python
async def _handle_retranspile_task_finished(
    self, db: AsyncSession, run: AISetRun, task: AITask
) -> None:
    """Write retranspiled script to disk and advance stage if all done."""
    item = await db.execute(
        select(AISetRunItem).where(AISetRunItem.retranspile_task_id == task.id)
    )
    item = item.scalar_one_or_none()
    if not item:
        return

    if task.status == "completed" and task.result_json:
        result = json.loads(task.result_json)
        script_content = result.get("script_content", "")
        if script_content:
            out_path = GENERATED_SCRIPTS_ROOT / item.set_id / f"{item.module_name}.py"
            out_path.write_text(script_content)
        run.retranspile_completed += 1
    else:
        run.retranspile_failed += 1

    # Check if all retranspile tasks done
    done = run.retranspile_completed + run.retranspile_failed
    if done >= run.retranspile_total:
        await self._queue_qa_stage(db, run)

    await db.commit()
```

**Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_set_run_retranspile.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/set_run_orchestrator.py tests/test_set_run_retranspile.py
git commit -m "feat: add score and retranspile stages to set run orchestrator"
```

---

### Task 7: Admin API — Set Run Score Threshold + Transpiler Learn Endpoints

**Files:**
- Modify: `digimon_gym/db/routers/admin_ai.py` (update set-run create, add learn endpoints)
- Modify: `tests/test_ai_pipeline.py` (add endpoint tests)

**Step 1: Write failing test**

```python
# Add to tests/test_ai_pipeline.py
class TestTranspilerLearnEndpoints:
    def test_create_learn_run_requires_auth(self, client):
        resp = client.post("/admin/transpiler-learn", json={
            "source_set_run_id": "fake-id",
        })
        assert resp.status_code in (401, 403)

    def test_set_run_create_accepts_score_threshold(self, client, session_factory):
        # Register admin user, grant roles
        tokens = _register_and_login(client, "learn_admin")
        import asyncio
        asyncio.get_event_loop().run_until_complete(
            _grant_roles(session_factory, "learn_admin", "admin")
        )
        resp = client.post(
            "/admin/set-runs",
            json={"set_id": "bt24", "score_threshold": 0.8},
            headers={"Authorization": f"Bearer {tokens['access_token']}"},
        )
        # May fail if bt24 scripts don't exist in test env, but should not 422
        assert resp.status_code != 422
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_ai_pipeline.py::TestTranspilerLearnEndpoints -v`
Expected: FAIL or 422 (schema doesn't accept `score_threshold` yet)

**Step 3: Update admin_ai.py**

1. Update the set-run creation endpoint to pass `score_threshold` from request to orchestrator.

2. Add new endpoints:

```python
@router.post("/transpiler-learn", response_model=AITranspilerLearnResponse)
async def create_transpiler_learn_run(
    req: AITranspilerLearnCreateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role("admin")),
):
    """Trigger pattern learning from a completed set run's autofixes."""
    from digimon_gym.ai.pattern_learner import create_learn_run
    learn_run = await create_learn_run(
        db,
        source_set_run_id=req.source_set_run_id,
        min_cluster_size=req.min_cluster_size,
    )
    return _learn_run_to_response(learn_run)


@router.get("/transpiler-learn/{learn_run_id}", response_model=AITranspilerLearnResponse)
async def get_transpiler_learn_run(
    learn_run_id: str,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_role("admin")),
):
    """Get transpiler learn run status."""
    run = await db.get(AITranspilerLearnRun, learn_run_id)
    if not run:
        raise HTTPException(404, "Learn run not found")
    return _learn_run_to_response(run)
```

**Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_ai_pipeline.py::TestTranspilerLearnEndpoints -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/db/routers/admin_ai.py tests/test_ai_pipeline.py
git commit -m "feat: add score_threshold to set-run create and transpiler-learn endpoints"
```

---

### Task 8: Pattern Learner — Diff Clustering (Phase 1)

**Files:**
- Create: `digimon_gym/ai/pattern_learner.py`
- Create: `tests/test_pattern_learner.py`

**Step 1: Write failing tests**

```python
# tests/test_pattern_learner.py
import pytest
from digimon_gym.ai.pattern_learner import cluster_autofix_diffs, DiffCluster


class TestClusterAutofixDiffs:
    def test_empty_input(self):
        result = cluster_autofix_diffs([])
        assert result == []

    def test_single_diff_below_threshold(self):
        """A single diff doesn't form a cluster (min_size=3)."""
        diffs = [_make_audit_record(
            card_id="BT24-001",
            before="def condition0(ctx):\n    return True",
            after="def condition0(ctx):\n    if not card.owner.is_my_turn:\n        return False\n    return True",
        )]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        assert result == []

    def test_clusters_similar_diffs(self):
        """Three diffs with the same change type should form one cluster."""
        diffs = [
            _make_audit_record(
                card_id=f"BT24-{i:03d}",
                before=f"def condition{i}(ctx):\n    return True",
                after=f"def condition{i}(ctx):\n    if not card.owner.is_my_turn:\n        return False\n    return True",
            )
            for i in range(5)
        ]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        assert len(result) >= 1
        assert result[0].count >= 3

    def test_cluster_has_required_fields(self):
        diffs = [
            _make_audit_record(
                card_id=f"BT24-{i:03d}",
                before="player.draw_cards(1)",
                after="player.draw_cards(2)",
            )
            for i in range(4)
        ]
        result = cluster_autofix_diffs(diffs, min_cluster_size=3)
        if result:
            c = result[0]
            assert isinstance(c, DiffCluster)
            assert hasattr(c, "description")
            assert hasattr(c, "change_type")
            assert hasattr(c, "card_ids")
            assert hasattr(c, "representative_diffs")
            assert hasattr(c, "count")


def _make_audit_record(card_id: str, before: str, after: str):
    """Create a mock audit record with before/after script content."""
    import json
    class MockAudit:
        def __init__(self):
            self.card_id = card_id
            self.applied_files_json = json.dumps([{
                "path": f"digimon_gym/engine/data/scripts/generated/bt24/{card_id.lower().replace('-', '_')}.py",
                "before": before,
                "after": after,
            }])
            self.status = "applied"
    return MockAudit()
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_pattern_learner.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'digimon_gym.ai.pattern_learner'`

**Step 3: Implement pattern_learner.py**

```python
# digimon_gym/ai/pattern_learner.py
"""Cluster autofix diffs and orchestrate transpiler learning runs."""

from __future__ import annotations

import ast
import difflib
import json
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any


@dataclass
class DiffCluster:
    description: str
    change_type: str
    card_ids: list[str]
    representative_diffs: list[dict]
    count: int


def _extract_diffs(audit_record: Any) -> list[dict]:
    """Extract before/after pairs from an audit record."""
    try:
        files = json.loads(audit_record.applied_files_json)
    except (json.JSONDecodeError, AttributeError):
        return []
    return [f for f in files if f.get("before") and f.get("after")]


def _classify_diff(before: str, after: str) -> str:
    """Classify a diff into a change type based on structural analysis."""
    # Try AST-level comparison
    try:
        before_ast = ast.dump(ast.parse(before))
        after_ast = ast.dump(ast.parse(after))
    except SyntaxError:
        # Fall back to text-level diff classification
        return _classify_text_diff(before, after)

    diff_lines = list(difflib.unified_diff(
        before.splitlines(), after.splitlines(), lineterm=""
    ))
    added = [ln[1:] for ln in diff_lines if ln.startswith("+") and not ln.startswith("+++")]
    removed = [ln[1:] for ln in diff_lines if ln.startswith("-") and not ln.startswith("---")]

    added_text = "\n".join(added).lower()
    removed_text = "\n".join(removed).lower()

    if "return false" in added_text and "condition" in added_text:
        return "condition_guard"
    if "def process" in added_text or "def callback" in added_text:
        return "new_callback"
    if "effect" in added_text and "icard" in added_text.replace(" ", ""):
        return "new_effect"
    if any(kw in added_text for kw in ("draw_cards", "add_memory", "change_dp", "suspend")):
        return "action_call"
    if "filter" in added_text or "card_filter" in added_text:
        return "filter_fix"
    return "other"


def _classify_text_diff(before: str, after: str) -> str:
    """Classify based on raw text when AST parsing fails."""
    diff_lines = list(difflib.unified_diff(
        before.splitlines(), after.splitlines(), lineterm=""
    ))
    added = "\n".join(ln[1:] for ln in diff_lines if ln.startswith("+") and not ln.startswith("+++")).lower()
    if "condition" in added:
        return "condition_guard"
    if "effect" in added:
        return "new_effect"
    return "other"


def cluster_autofix_diffs(
    audit_records: list[Any],
    min_cluster_size: int = 3,
) -> list[DiffCluster]:
    """Cluster successful autofix diffs by change type."""
    if not audit_records:
        return []

    # Extract and classify all diffs
    classified: dict[str, list[dict]] = defaultdict(list)
    for record in audit_records:
        if getattr(record, "status", "") != "applied":
            continue
        for diff_pair in _extract_diffs(record):
            change_type = _classify_diff(diff_pair["before"], diff_pair["after"])
            classified[change_type].append({
                "card_id": record.card_id,
                "before": diff_pair["before"],
                "after": diff_pair["after"],
                "path": diff_pair.get("path", ""),
            })

    # Build clusters
    clusters = []
    for change_type, diffs in classified.items():
        if len(diffs) < min_cluster_size:
            continue
        card_ids = list({d["card_id"] for d in diffs})
        representatives = diffs[:3]  # First 3 as examples
        clusters.append(DiffCluster(
            description=f"{len(diffs)} cards: {change_type} change",
            change_type=change_type,
            card_ids=card_ids,
            representative_diffs=representatives,
            count=len(diffs),
        ))

    clusters.sort(key=lambda c: c.count, reverse=True)
    return clusters
```

**Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/test_pattern_learner.py -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/pattern_learner.py tests/test_pattern_learner.py
git commit -m "feat: add pattern learner with AST-based diff clustering"
```

---

### Task 9: TranspilerLearnOutput Contract + Dispatcher Branch

**Files:**
- Modify: `digimon_gym/ai/contracts.py` (add `TranspilerPatchSuggestion`, `TranspilerLearnOutput`)
- Modify: `digimon_gym/ai/prompts.py` (add `build_transpiler_learn_messages`)
- Modify: `digimon_gym/ai/dispatcher.py` (add `transpiler_learn` branch)
- Modify: `tests/test_contracts.py` (add contract test)

**Step 1: Write failing test**

```python
# Add to tests/test_contracts.py
from digimon_gym.ai.contracts import TranspilerLearnOutput, TranspilerPatchSuggestion


class TestTranspilerLearnOutput:
    def test_valid_output(self):
        patch = TranspilerPatchSuggestion(
            target_file="tools/transpiler/extractors.py",
            description="Add is_my_turn guard extraction",
            before_snippet="# existing code",
            after_snippet="# new code",
            cards_affected=["BT24-001", "BT24-002"],
        )
        out = TranspilerLearnOutput(
            cluster_summary="5 cards needed is_my_turn guards",
            patches=[patch],
            estimated_cards_fixed=5,
            confidence="medium",
        )
        assert len(out.patches) == 1
        assert out.confidence == "medium"

    def test_rejects_invalid_confidence(self):
        import pytest
        with pytest.raises(Exception):
            TranspilerLearnOutput(
                cluster_summary="...",
                patches=[],
                estimated_cards_fixed=0,
                confidence="maybe",
            )
```

**Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_contracts.py::TestTranspilerLearnOutput -v`
Expected: FAIL — `ImportError`

**Step 3: Add contracts**

Add to `digimon_gym/ai/contracts.py`:

```python
class TranspilerPatchSuggestion(BaseModel):
    """A suggested change to the transpiler codebase."""

    model_config = ConfigDict(extra="forbid")

    target_file: str
    description: str
    before_snippet: str
    after_snippet: str
    cards_affected: list[str]


class TranspilerLearnOutput(BaseModel):
    """Output from transpiler pattern learning."""

    model_config = ConfigDict(extra="forbid")

    cluster_summary: str
    patches: list[TranspilerPatchSuggestion]
    estimated_cards_fixed: int
    confidence: Literal["low", "medium", "high"]
```

**Step 4: Add prompt builder to prompts.py**

```python
def build_transpiler_learn_messages(
    *,
    cluster: dict,
    extractors_source: str,
    generators_source: str,
    patterns_source: str,
    cs_examples: list[dict],
) -> tuple[str, str]:
    """Build prompts for transpiler pattern learning."""
    system = (
        "You are a Python developer improving a regex-based C# to Python transpiler. "
        "You will receive a cluster of similar autofix diffs — changes that were needed "
        "to fix transpiled card scripts. Your job is to propose changes to the transpiler "
        "source code (extractors.py, generators.py, or patterns.py) that would produce "
        "the correct output in the first place, eliminating the need for these fixes.\n\n"
        "Rules:\n"
        "- Propose minimal, targeted changes\n"
        "- Show exact before/after snippets that can be applied to the transpiler\n"
        "- Each patch should reference actual code from the transpiler source\n"
        "- Estimate how many cards would benefit\n"
    )

    examples_block = ""
    if cs_examples:
        parts = [f"### {ex['card_id']}\n```csharp\n{ex['cs_source']}\n```" for ex in cs_examples]
        examples_block = "## Representative C# Sources\n\n" + "\n\n".join(parts)

    diffs_block = json.dumps(cluster.get("representative_diffs", []), indent=2)

    user = (
        f"## Diff Cluster\n\n"
        f"**Type:** {cluster.get('change_type', 'unknown')}\n"
        f"**Count:** {cluster.get('count', 0)} cards affected\n"
        f"**Description:** {cluster.get('description', '')}\n\n"
        f"### Representative Diffs\n```json\n{diffs_block}\n```\n\n"
        f"{examples_block}\n\n"
        f"## Transpiler Source: extractors.py\n```python\n{extractors_source}\n```\n\n"
        f"## Transpiler Source: generators.py\n```python\n{generators_source}\n```\n\n"
        f"## Transpiler Source: patterns.py\n```python\n{patterns_source}\n```\n\n"
        f"Propose transpiler changes to handle this pattern automatically."
    )

    return system, user
```

**Step 5: Add dispatch branch**

In `dispatcher.py`, add to `run()`:

```python
if task_type == "transpiler_learn":
    return self._run_transpiler_learn(payload, model_name=model_name)
```

Add method:

```python
def _run_transpiler_learn(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
    cluster = payload.get("cluster", {})
    if not cluster:
        raise ValueError("transpiler_learn payload must include cluster")

    # Load transpiler source files
    transpiler_dir = PROJECT_ROOT / "tools" / "transpiler"
    extractors_src = (transpiler_dir / "extractors.py").read_text()
    generators_src = (transpiler_dir / "generators.py").read_text()
    patterns_src = (transpiler_dir / "patterns.py").read_text()

    cs_examples = payload.get("cs_examples", [])

    system_prompt, user_prompt = build_transpiler_learn_messages(
        cluster=cluster,
        extractors_source=extractors_src,
        generators_source=generators_src,
        patterns_source=patterns_src,
        cs_examples=cs_examples,
    )

    run = self.client.run_structured(
        task_type="transpiler_learn",
        system_prompt=system_prompt,
        user_prompt=user_prompt,
        schema_model=TranspilerLearnOutput,
        model_name=model_name,
    )

    return DispatchOutcome(
        model_name=run.model_name,
        result=run.output,
        sanitized_input={"change_type": cluster.get("change_type"), "count": cluster.get("count")},
        retrieval_refs=[],
        input_tokens=run.input_tokens,
        output_tokens=run.output_tokens,
        cost_actual=self._cost_from_usage(run.model_name, run.input_tokens, run.output_tokens),
    )
```

**Step 6: Run tests to verify they pass**

Run: `python -m pytest tests/test_contracts.py::TestTranspilerLearnOutput -v`
Expected: PASS

**Step 7: Commit**

```bash
git add digimon_gym/ai/contracts.py digimon_gym/ai/prompts.py digimon_gym/ai/dispatcher.py tests/test_contracts.py
git commit -m "feat: add transpiler_learn contract, prompt builder, and dispatch branch"
```

---

### Task 10: Pattern Learner — Learn Run Orchestration (Phase 2)

**Files:**
- Modify: `digimon_gym/ai/pattern_learner.py` (add `create_learn_run` async function)
- Modify: `tests/test_pattern_learner.py` (add orchestration test)

**Step 1: Write failing test**

```python
# Add to tests/test_pattern_learner.py
class TestCreateLearnRun:
    def test_create_learn_run_exists(self):
        from digimon_gym.ai.pattern_learner import create_learn_run
        assert callable(create_learn_run)
```

**Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_pattern_learner.py::TestCreateLearnRun -v`
Expected: FAIL — `ImportError: cannot import name 'create_learn_run'`

**Step 3: Implement create_learn_run**

Add to `digimon_gym/ai/pattern_learner.py`:

```python
async def create_learn_run(
    db: "AsyncSession",
    *,
    source_set_run_id: str,
    min_cluster_size: int = 3,
) -> "AITranspilerLearnRun":
    """Create a transpiler learn run from a completed set run's autofixes."""
    from sqlalchemy import select
    from digimon_gym.db.models import AIFixApplyAudit, AITranspilerLearnRun, AITask

    # Fetch successful audits for this set run
    stmt = (
        select(AIFixApplyAudit)
        .join(AITask, AIFixApplyAudit.ai_task_id == AITask.id)
        .where(AITask.set_run_id == source_set_run_id)
        .where(AIFixApplyAudit.status == "applied")
    )
    result = await db.execute(stmt)
    audits = list(result.scalars().all())

    # Create learn run record
    learn_run = AITranspilerLearnRun(
        source_set_run_id=source_set_run_id,
        status="clustering",
    )
    db.add(learn_run)
    await db.flush()

    # Phase 1: cluster diffs (synchronous)
    clusters = cluster_autofix_diffs(audits, min_cluster_size=min_cluster_size)
    learn_run.clusters_found = len(clusters)

    if not clusters:
        learn_run.status = "completed"
        await db.commit()
        return learn_run

    # Phase 2: create transpiler_learn tasks for each cluster
    learn_run.status = "generating"
    for cluster in clusters:
        task = AITask(
            task_type="transpiler_learn",
            status="queued",
            payload_json=json.dumps({
                "cluster": {
                    "description": cluster.description,
                    "change_type": cluster.change_type,
                    "card_ids": cluster.card_ids,
                    "representative_diffs": cluster.representative_diffs,
                    "count": cluster.count,
                },
                "learn_run_id": learn_run.id,
            }),
            max_attempts=2,
        )
        db.add(task)

    await db.commit()
    return learn_run
```

**Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_pattern_learner.py::TestCreateLearnRun -v`
Expected: PASS

**Step 5: Commit**

```bash
git add digimon_gym/ai/pattern_learner.py tests/test_pattern_learner.py
git commit -m "feat: add learn run orchestration for transpiler pattern learning"
```

---

### Task 11: Admin Frontend — Score Column + Learn Button

**Files:**
- Modify: `frontend/src/pages/AdminTasksPage.tsx` (show `llm_transpile` and `transpiler_learn` task types)
- Modify: `frontend/src/api/adminApi.ts` (add transpiler-learn API calls)

**Step 1: Add API client functions**

In `frontend/src/api/adminApi.ts`:

```typescript
export async function createTranspilerLearnRun(sourceSetRunId: string, minClusterSize = 3) {
  const resp = await api.post('/admin/transpiler-learn', {
    source_set_run_id: sourceSetRunId,
    min_cluster_size: minClusterSize,
  });
  return resp.data;
}

export async function getTranspilerLearnRun(learnRunId: string) {
  const resp = await api.get(`/admin/transpiler-learn/${learnRunId}`);
  return resp.data;
}
```

**Step 2: Update AdminTasksPage to handle new task types**

The existing task list should already display `llm_transpile` and `transpiler_learn` tasks since it renders all tasks generically. Verify the task type column renders these new values. If there's a task type filter dropdown, add the new types to its options.

**Step 3: Add score column to set run detail view**

If there's a set run detail view that renders `AISetRunItemResponse` items, add a `transpile_score` column. If the set run is completed, show a "Learn from Fixes" button that calls `createTranspilerLearnRun`.

**Step 4: Verify in browser**

Run: `cd frontend && npm run dev`
Navigate to admin tasks page, verify no TypeScript errors.

**Step 5: Commit**

```bash
git add frontend/src/api/adminApi.ts frontend/src/pages/AdminTasksPage.tsx
git commit -m "feat: add transpiler-learn API client and admin UI support"
```

---

### Task 12: Integration Test — End-to-End Scoring + Retranspile Flow

**Files:**
- Create: `tests/test_retranspile_integration.py`

**Step 1: Write integration test**

```python
# tests/test_retranspile_integration.py
"""Integration test for the scoring + retranspile pipeline."""
import json
import pytest
from unittest.mock import patch, MagicMock, AsyncMock
from tools.transpiler.scoring import score_card, TranspileScore
from tools.transpiler.models import EffectBlock
from tools.transpiler.validation import ValidationResult


class TestScoringIntegration:
    """Test scoring against real-ish transpiler data structures."""

    def test_score_real_effect_blocks(self):
        """Score a card with realistic EffectBlock data."""
        eb1 = EffectBlock()
        eb1.timing = "EffectTiming.OnPlay"
        eb1.actions = ["draw"]
        eb1.is_factory = False

        eb2 = EffectBlock()
        eb2.timing = "EffectTiming.WhenDigivolving"
        eb2.actions = ["gain_memory"]
        eb2.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-042"
        vr.forward_issues = []
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-042", "effect": "[On Play] Draw 1.\n[When Digivolving] Gain 1 memory."}

        result = score_card("BT24-042", [eb1, eb2], vr, meta)
        assert result.score >= 0.8
        assert result.below_threshold is False

    def test_score_with_missing_effects(self):
        """Card with 3 expected effects but only 1 extracted."""
        eb1 = EffectBlock()
        eb1.actions = ["draw"]
        eb1.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-042"
        vr.forward_issues = ["missing_reveal", "missing_delete"]
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-042", "effect": "[On Play] Draw 1.\n[When Digivolving] Reveal top 3.\n[On Deletion] Delete 1."}

        result = score_card("BT24-042", [eb1], vr, meta)
        assert result.score < 0.7
        assert result.below_threshold is True


class TestScoringToRetranspileFlow:
    """Test that low scores correctly trigger retranspile task creation."""

    def test_low_score_card_gets_retranspile_task(self):
        """Verify the data flow from scoring to task creation."""
        # Score a low-confidence card
        eb = EffectBlock()
        eb.actions = []
        eb.is_factory = False

        vr = ValidationResult()
        vr.card_id = "BT24-099"
        vr.forward_issues = ["missing_x", "missing_y", "missing_z"]
        vr.reverse_issues = []
        vr.timing_issues = []

        meta = {"card_id": "BT24-099", "effect": "Line1\nLine2\nLine3"}

        result = score_card("BT24-099", [eb], vr, meta, threshold=0.7)
        assert result.below_threshold is True
        assert result.score < 0.5

        # This score would trigger a llm_transpile task in the orchestrator
        # (actual DB integration tested in test_ai_pipeline.py)
```

**Step 2: Run tests**

Run: `python -m pytest tests/test_retranspile_integration.py -v`
Expected: All PASS

**Step 3: Commit**

```bash
git add tests/test_retranspile_integration.py
git commit -m "test: add integration tests for scoring and retranspile flow"
```

---

## Summary

| Task | What it builds | Files touched |
|------|---------------|---------------|
| 1 | Confidence scoring module | `tools/transpiler/scoring.py`, test |
| 2 | LLMTranspileOutput contract | `contracts.py`, test |
| 3 | LLM transpile prompt builder | `prompts.py`, test |
| 4 | LLM transpile dispatcher branch | `dispatcher.py`, test |
| 5 | DB migration (columns + table) | `models.py`, `schemas.py`, migration |
| 6 | Orchestrator score + retranspile stages | `set_run_orchestrator.py`, test |
| 7 | Admin API endpoints | `admin_ai.py`, test |
| 8 | Pattern learner diff clustering | `pattern_learner.py`, test |
| 9 | TranspilerLearn contract + dispatch | `contracts.py`, `prompts.py`, `dispatcher.py`, test |
| 10 | Learn run orchestration | `pattern_learner.py`, test |
| 11 | Admin frontend updates | `adminApi.ts`, `AdminTasksPage.tsx` |
| 12 | Integration tests | `test_retranspile_integration.py` |

**Total: 12 tasks, ~12 commits, estimated 2-3 hours of implementation time.**
