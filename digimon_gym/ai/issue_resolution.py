"""Helpers for resolving issue rows after successful fix apply."""

from __future__ import annotations

from datetime import datetime, timezone

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.models import Issue


def build_apply_resolution_note(
    *,
    task_id: str,
    card_id: str,
    run_mode: str,
    commit_sha: str | None = None,
    pr_url: str | None = None,
) -> str:
    parts = [
        f"[auto-resolved] script_autofix apply succeeded for {card_id}",
        f"task={task_id}",
        f"run_mode={run_mode}",
    ]
    if commit_sha:
        parts.append(f"commit={commit_sha}")
    if pr_url:
        parts.append(f"pr={pr_url}")
    return " | ".join(parts)


async def resolve_open_issues_for_card(
    db: AsyncSession,
    *,
    card_id: str,
    resolution_note: str,
) -> list[str]:
    rows = (
        await db.execute(
            select(Issue).where(
                Issue.card_id == card_id,
                Issue.status.in_(["new", "approved_for_ai"]),
            )
        )
    ).scalars().all()

    if not rows:
        return []

    now = datetime.now(timezone.utc)
    normalized_note = resolution_note.strip()
    resolved_ids: list[str] = []
    for issue in rows:
        issue.status = "resolved"
        issue.resolved_at = now
        if normalized_note:
            current = (issue.resolution_notes or "").strip()
            issue.resolution_notes = (
                f"{current}\n{normalized_note}".strip() if current else normalized_note
            )
        resolved_ids.append(issue.id)
    return resolved_ids
