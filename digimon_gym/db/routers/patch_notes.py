"""Patch notes router: public read, admin write for releases and known issues."""

from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.auth import ROLE_ADMIN, require_roles
from digimon_gym.db.database import get_db
from digimon_gym.db.models import KnownIssue, Release, User
from digimon_gym.db.schemas import (
    KnownIssueCreateRequest,
    KnownIssueResponse,
    KnownIssueUpdateRequest,
    PatchNotesResponse,
    ReleaseCreateRequest,
    ReleaseResponse,
    ReleaseUpdateRequest,
)

router = APIRouter(prefix="/patch-notes", tags=["patch-notes"])


def _release_to_response(release: Release) -> ReleaseResponse:
    return ReleaseResponse(
        id=release.id,
        version=release.version,
        release_date=release.release_date,
        title=release.title,
        added=list(release.added or []),
        changed=list(release.changed or []),
        fixed=list(release.fixed or []),
        created_at=release.created_at,
        updated_at=release.updated_at,
    )


def _known_issue_to_response(issue: KnownIssue) -> KnownIssueResponse:
    return KnownIssueResponse(
        id=issue.id,
        title=issue.title,
        description=issue.description,
        created_at=issue.created_at,
        updated_at=issue.updated_at,
    )


@router.get("", response_model=PatchNotesResponse)
async def get_patch_notes(db: AsyncSession = Depends(get_db)):
    """Public endpoint: return all known issues and releases for the public page."""
    issues_result = await db.execute(
        select(KnownIssue).order_by(KnownIssue.created_at.desc())
    )
    releases_result = await db.execute(
        select(Release).order_by(Release.release_date.desc())
    )
    return PatchNotesResponse(
        known_issues=[_known_issue_to_response(i) for i in issues_result.scalars().all()],
        releases=[_release_to_response(r) for r in releases_result.scalars().all()],
    )


# ── Known Issues (admin write) ─────────────────────────────────────────

@router.post(
    "/known-issues",
    response_model=KnownIssueResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_known_issue(
    request: KnownIssueCreateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    issue = KnownIssue(title=request.title, description=request.description)
    db.add(issue)
    await db.commit()
    await db.refresh(issue)
    return _known_issue_to_response(issue)


@router.patch("/known-issues/{issue_id}", response_model=KnownIssueResponse)
async def update_known_issue(
    issue_id: str,
    request: KnownIssueUpdateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(KnownIssue).where(KnownIssue.id == issue_id))
    issue = result.scalar_one_or_none()
    if issue is None:
        raise HTTPException(status_code=404, detail="Known issue not found")

    if request.title is not None:
        issue.title = request.title
    if request.description is not None:
        issue.description = request.description

    await db.commit()
    await db.refresh(issue)
    return _known_issue_to_response(issue)


@router.delete("/known-issues/{issue_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_known_issue(
    issue_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(KnownIssue).where(KnownIssue.id == issue_id))
    issue = result.scalar_one_or_none()
    if issue is None:
        raise HTTPException(status_code=404, detail="Known issue not found")

    await db.delete(issue)
    await db.commit()
    return None


# ── Releases (admin write) ─────────────────────────────────────────────

@router.post(
    "/releases",
    response_model=ReleaseResponse,
    status_code=status.HTTP_201_CREATED,
)
async def create_release(
    request: ReleaseCreateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    release = Release(
        version=request.version,
        release_date=request.release_date,
        title=request.title,
        added=list(request.added),
        changed=list(request.changed),
        fixed=list(request.fixed),
    )
    db.add(release)
    try:
        await db.commit()
    except IntegrityError:
        await db.rollback()
        raise HTTPException(
            status_code=409,
            detail=f"Release with version '{request.version}' already exists",
        )
    await db.refresh(release)
    return _release_to_response(release)


@router.patch("/releases/{release_id}", response_model=ReleaseResponse)
async def update_release(
    release_id: str,
    request: ReleaseUpdateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Release).where(Release.id == release_id))
    release = result.scalar_one_or_none()
    if release is None:
        raise HTTPException(status_code=404, detail="Release not found")

    if request.version is not None:
        release.version = request.version
    if request.release_date is not None:
        release.release_date = request.release_date
    if request.title is not None:
        release.title = request.title
    if request.added is not None:
        release.added = list(request.added)
    if request.changed is not None:
        release.changed = list(request.changed)
    if request.fixed is not None:
        release.fixed = list(request.fixed)

    try:
        await db.commit()
    except IntegrityError:
        await db.rollback()
        raise HTTPException(
            status_code=409,
            detail=f"Release with version '{request.version}' already exists",
        )
    await db.refresh(release)
    return _release_to_response(release)


@router.delete("/releases/{release_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_release(
    release_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Release).where(Release.id == release_id))
    release = result.scalar_one_or_none()
    if release is None:
        raise HTTPException(status_code=404, detail="Release not found")

    await db.delete(release)
    await db.commit()
    return None
