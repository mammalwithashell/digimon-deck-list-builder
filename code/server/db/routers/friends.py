"""Friends router: request, accept, block, list."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import List

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy import and_, select
from sqlalchemy.ext.asyncio import AsyncSession

from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import Friendship, User
from server.db.schemas import FriendRequestResponse, FriendshipResponse, UserPublic

router = APIRouter(prefix="/friends", tags=["friends"])


@router.post("/request/{user_id}")
async def send_friend_request(
    user_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if user_id == user.id:
        raise HTTPException(status_code=400, detail="Cannot send a friend request to yourself")

    # Check target user exists
    target = await db.execute(select(User).where(User.id == user_id))
    if not target.scalar_one_or_none():
        raise HTTPException(status_code=404, detail="User not found")

    # Check if blocked by target
    blocked = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user_id,
            Friendship.friend_id == user.id,
            Friendship.status == "blocked",
        )
    )
    if blocked.scalar_one_or_none():
        raise HTTPException(status_code=403, detail="Cannot send request to this user")

    # Check if already exists
    existing = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user.id,
            Friendship.friend_id == user_id,
        )
    )
    record = existing.scalar_one_or_none()
    if record:
        if record.status == "accepted":
            raise HTTPException(status_code=409, detail="Already friends")
        if record.status == "pending":
            raise HTTPException(status_code=409, detail="Friend request already sent")
        if record.status == "blocked":
            raise HTTPException(status_code=400, detail="You have blocked this user. Unblock first.")

    # Check if target already sent us a request — auto-accept
    incoming = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user_id,
            Friendship.friend_id == user.id,
            Friendship.status == "pending",
        )
    )
    incoming_record = incoming.scalar_one_or_none()
    if incoming_record:
        # Auto-accept: they already sent us a request
        incoming_record.status = "accepted"
        incoming_record.updated_at = datetime.now(timezone.utc)
        new_record = Friendship(
            user_id=user.id,
            friend_id=user_id,
            status="accepted",
        )
        db.add(new_record)
        await db.commit()
        return {"status": "accepted", "message": "Friend request auto-accepted (mutual request)"}

    # Create pending request (one row — sender side only until accepted)
    friendship = Friendship(
        user_id=user.id,
        friend_id=user_id,
        status="pending",
    )
    db.add(friendship)
    await db.commit()
    return {"status": "pending", "message": "Friend request sent"}


@router.post("/accept/{user_id}")
async def accept_friend_request(
    user_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    # Find the incoming pending request
    result = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user_id,
            Friendship.friend_id == user.id,
            Friendship.status == "pending",
        )
    )
    incoming = result.scalar_one_or_none()
    if not incoming:
        raise HTTPException(status_code=404, detail="No pending friend request from this user")

    # Update incoming to accepted
    incoming.status = "accepted"
    incoming.updated_at = datetime.now(timezone.utc)

    # Create symmetric row
    my_row = Friendship(
        user_id=user.id,
        friend_id=user_id,
        status="accepted",
    )
    db.add(my_row)
    await db.commit()
    return {"status": "accepted"}


@router.delete("/{user_id}")
async def remove_friend(
    user_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    # Delete both directions
    result1 = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user.id,
            Friendship.friend_id == user_id,
        )
    )
    row1 = result1.scalar_one_or_none()

    result2 = await db.execute(
        select(Friendship).where(
            Friendship.user_id == user_id,
            Friendship.friend_id == user.id,
        )
    )
    row2 = result2.scalar_one_or_none()

    if not row1 and not row2:
        raise HTTPException(status_code=404, detail="No friendship or request found")

    if row1:
        await db.delete(row1)
    if row2:
        await db.delete(row2)
    await db.commit()
    return {"status": "removed"}


@router.get("", response_model=List[UserPublic])
async def list_friends(
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(
        select(Friendship.friend_id).where(
            Friendship.user_id == user.id,
            Friendship.status == "accepted",
        )
    )
    friend_ids = [row[0] for row in result.all()]

    if not friend_ids:
        return []

    users_result = await db.execute(select(User).where(User.id.in_(friend_ids)))
    friends = users_result.scalars().all()
    return [
        UserPublic(
            id=f.id,
            username=f.username,
            display_name=f.display_name,
            avatar_url=f.avatar_url,
        )
        for f in friends
    ]


@router.get("/requests", response_model=List[FriendRequestResponse])
async def list_pending_requests(
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    # Find pending requests where I am the target (friend_id)
    result = await db.execute(
        select(Friendship).where(
            Friendship.friend_id == user.id,
            Friendship.status == "pending",
        )
    )
    requests = result.scalars().all()

    if not requests:
        return []

    sender_ids = [r.user_id for r in requests]
    users_result = await db.execute(select(User).where(User.id.in_(sender_ids)))
    senders = {u.id: u for u in users_result.scalars().all()}

    return [
        FriendRequestResponse(
            from_user=UserPublic(
                id=senders[r.user_id].id,
                username=senders[r.user_id].username,
                display_name=senders[r.user_id].display_name,
                avatar_url=senders[r.user_id].avatar_url,
            ),
            created_at=r.created_at,
        )
        for r in requests
        if r.user_id in senders
    ]


@router.post("/block/{user_id}")
async def block_user(
    user_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if user_id == user.id:
        raise HTTPException(status_code=400, detail="Cannot block yourself")

    # Check target exists
    target = await db.execute(select(User).where(User.id == user_id))
    if not target.scalar_one_or_none():
        raise HTTPException(status_code=404, detail="User not found")

    # Remove any existing friendship in both directions
    for uid, fid in [(user.id, user_id), (user_id, user.id)]:
        result = await db.execute(
            select(Friendship).where(
                Friendship.user_id == uid,
                Friendship.friend_id == fid,
            )
        )
        row = result.scalar_one_or_none()
        if row:
            await db.delete(row)

    # Create blocked row (only my side)
    block = Friendship(
        user_id=user.id,
        friend_id=user_id,
        status="blocked",
    )
    db.add(block)
    await db.commit()
    return {"status": "blocked"}
