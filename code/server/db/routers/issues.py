"""Issue queue router: create, list, triage/update."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import ROLE_ADMIN, ROLE_JUDGE, ROLE_PLAYER, require_roles
from server.db.database import get_db
from server.db.models import Issue, User
from server.db.schemas import IssueCreateRequest, IssueResponse, IssueUpdateRequest

router = APIRouter(prefix="/issues", tags=["issues"])


def _issue_to_response(issue: Issue) -> IssueResponse:
    return IssueResponse(
        id=issue.id,
        card_id=issue.card_id,
        description=issue.description,
        source=issue.source,
        severity=issue.severity,
        status=issue.status,
        created_by=issue.created_by,
        approved_by=issue.approved_by,
        triage_notes=issue.triage_notes or "",
        resolution_notes=issue.resolution_notes or "",
        created_at=issue.created_at,
        updated_at=issue.updated_at,
        resolved_at=issue.resolved_at,
    )


def _is_valid_transition(current: str, new: str) -> bool:
    if current == new:
        return True

    allowed = {
        "new": {"approved_for_ai", "rejected"},
        "approved_for_ai": {"resolved", "rejected"},
        "rejected": set(),
        "resolved": set(),
    }
    return new in allowed.get(current, set())


@router.post("", response_model=IssueResponse, status_code=status.HTTP_201_CREATED)
async def create_issue(
    request: IssueCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN, ROLE_JUDGE, ROLE_PLAYER)),
    db: AsyncSession = Depends(get_db),
):
    issue = Issue(
        card_id=request.card_id,
        description=request.description,
        source=request.source,
        severity=request.severity,
        status="new",
        created_by=user.id,
    )
    db.add(issue)
    await db.commit()
    await db.refresh(issue)
    return _issue_to_response(issue)


@router.get("", response_model=List[IssueResponse])
async def list_issues(
    status_filter: Optional[str] = Query(None, alias="status", pattern=r"^(new|approved_for_ai|rejected|resolved)$"),
    source: Optional[str] = Query(None, pattern=r"^(player|judge|system)$"),
    severity: Optional[str] = Query(None, pattern=r"^(low|medium|high|critical)$"),
    card_id: Optional[str] = Query(None, min_length=1, max_length=64),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN, ROLE_JUDGE)),
    db: AsyncSession = Depends(get_db),
):
    query = select(Issue).order_by(Issue.created_at.desc())
    if status_filter:
        query = query.where(Issue.status == status_filter)
    if source:
        query = query.where(Issue.source == source)
    if severity:
        query = query.where(Issue.severity == severity)
    if card_id:
        query = query.where(Issue.card_id == card_id)
    query = query.limit(limit)

    result = await db.execute(query)
    return [_issue_to_response(issue) for issue in result.scalars().all()]


@router.patch("/{issue_id}", response_model=IssueResponse)
async def update_issue(
    issue_id: str,
    request: IssueUpdateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN, ROLE_JUDGE)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Issue).where(Issue.id == issue_id))
    issue = result.scalar_one_or_none()
    if issue is None:
        raise HTTPException(status_code=404, detail="Issue not found")

    if request.status is not None and not _is_valid_transition(issue.status, request.status):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid status transition: {issue.status} -> {request.status}",
        )

    if request.status is not None:
        issue.status = request.status
        if request.status == "approved_for_ai":
            issue.approved_by = user.id
        if request.status == "resolved":
            issue.resolved_at = datetime.now(timezone.utc)

    if request.severity is not None:
        issue.severity = request.severity
    if request.triage_notes is not None:
        issue.triage_notes = request.triage_notes
    if request.resolution_notes is not None:
        issue.resolution_notes = request.resolution_notes

    await db.commit()
    await db.refresh(issue)
    return _issue_to_response(issue)
