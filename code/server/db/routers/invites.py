"""Admin invite-code management for alpha registration gating.

Alpha invites are a curated roster mechanism — a registration only succeeds
when `settings.invite_codes_required` is true and the user supplies an unused
code. These endpoints mint and inspect those codes; redemption happens in
`routers/auth.py::register`.
"""

from datetime import datetime, timezone
from secrets import token_urlsafe
from typing import List

from fastapi import APIRouter, Body, Depends, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.orm import aliased

from server.db.auth import ROLE_ADMIN, require_roles
from server.db.database import get_db
from server.db.models import InviteCode, User
from server.db.schemas import InviteCodeInfo, MintInviteCodesRequest


router = APIRouter(prefix="/admin/invite-codes", tags=["admin", "invites"])


def _generate_code() -> str:
    # 12 URL-safe chars ≈ 72 bits of entropy; short enough to type, long
    # enough that a random-guess attack is infeasible at alpha volumes.
    return token_urlsafe(9)


@router.post("", response_model=List[InviteCodeInfo], status_code=status.HTTP_201_CREATED)
async def mint_invite_codes(
    body: MintInviteCodesRequest = Body(...),
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    now = datetime.now(timezone.utc)
    rows = [
        InviteCode(
            code=_generate_code(),
            created_by_user_id=user.id,
            created_at=now,
            note=body.note,
        )
        for _ in range(body.count)
    ]
    db.add_all(rows)
    await db.commit()
    return [
        InviteCodeInfo(
            code=row.code,
            created_at=row.created_at,
            redeemed_at=None,
            redeemed_by_username=None,
            note=row.note,
        )
        for row in rows
    ]


@router.get("", response_model=List[InviteCodeInfo])
async def list_invite_codes(
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    redeemer = aliased(User)
    result = await db.execute(
        select(InviteCode, redeemer.username)
        .outerjoin(redeemer, InviteCode.redeemed_by_user_id == redeemer.id)
        .order_by(InviteCode.created_at.desc())
    )
    out: list[InviteCodeInfo] = []
    for row, redeemer_username in result.all():
        out.append(
            InviteCodeInfo(
                code=row.code,
                created_at=row.created_at,
                redeemed_at=row.redeemed_at,
                redeemed_by_username=redeemer_username,
                note=row.note,
            )
        )
    return out
