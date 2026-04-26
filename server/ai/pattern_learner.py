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
        ast.parse(before)
        ast.parse(after)
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
