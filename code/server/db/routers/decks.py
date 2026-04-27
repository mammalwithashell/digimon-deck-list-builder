"""Deck CRUD router with per-game-mode validation."""

from __future__ import annotations

import json
from typing import List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy import func, select, update
from sqlalchemy.exc import IntegrityError
from sqlalchemy.ext.asyncio import AsyncSession

from server.classifier.deck_tagger import tag_deck
from server.db.auth import get_current_user
from server.db.database import get_db
from server.db.models import Deck, DeckFolder, DeckVersion, User
from server.db.schemas import (
    CreateDeckRequest,
    CreateDeckFolderRequest,
    DeckFolderResponse,
    DeckResponse,
    DeckSummary,
    UpdateDeckRequest,
    UpdateDeckFolderRequest,
    UpdateDeckLibraryRequest,
)
# validate_deck + CardRestriction stay on Python: the no_restriction game-mode
# path passes an empty CardRestriction() to bypass restricted-list checks,
# and the Rust binding only exposes the official-ENG list path. Tracked in
# RUST_PYTHON_PARITY.md.
from engine_py_legacy.engine.data.deck_loader import validate_deck, RESTRICTED_LIST, CardRestriction
from digimon_engine import out_of_set_cards

router = APIRouter(prefix="/decks", tags=["decks"])

DEFAULT_FOLDER_NAMES = ("Tournament", "Experimental", "Casual")


def _reject_untested_cards(main_deck: list[str], egg_deck: list[str]) -> None:
    """Hard-reject any deck containing cards without behavioral test coverage.

    Alpha gate: prevents un-QA'd card scripts from being saved to the DB
    and later loaded into a game.
    """
    untested = out_of_set_cards(main_deck + egg_deck)
    if untested:
        raise HTTPException(
            status_code=422,
            detail={
                "message": (
                    "Deck contains cards that are not available in the "
                    "alpha release (no test coverage)."
                ),
                "untested_card_ids": untested,
            },
        )


def _validate_for_mode(card_ids: list[str], game_mode: str, titan_role: str | None) -> tuple[bool, list[str]]:
    """Run deck validation with mode-specific rules.

    Returns (is_valid, error_list).
    """
    if game_mode == "no_restriction":
        # Only check deck size and card existence — no restricted list
        result = validate_deck(card_ids, restricted_list=CardRestriction())
        return result.is_valid, result.errors

    if game_mode == "titan":
        expected_main = 80 if titan_role == "titan" else 50
        # Use standard restricted list but override size check via post-validation
        result = validate_deck(card_ids)
        # Filter out size errors and re-check with titan size
        errors = [e for e in result.errors if "Main deck must be exactly" not in e]
        # Count main deck cards (non-egg)
        from digimon_engine import CardDatabase, CardKind
        db = CardDatabase()
        main_count = sum(
            1 for cid in card_ids
            if (entity := db.get_card(cid)) is None or entity.card_kind != CardKind.DigiEgg
        )
        if main_count != expected_main:
            errors.append(f"Titan {titan_role} main deck must be exactly {expected_main} cards (got {main_count})")
        return len(errors) == 0, errors

    if game_mode == "edh_commander":
        # Singleton rule: no more than 1 copy of any card (except basic eggs)
        from collections import Counter
        counts = Counter(card_ids)
        errors = []
        for card_id, count in counts.items():
            if count > 1:
                errors.append(f"{card_id}: {count} copies (EDH Commander is singleton — max 1)")
        # Size: 70 main deck cards
        from digimon_engine import CardDatabase, CardKind
        db = CardDatabase()
        main_count = sum(
            1 for cid in card_ids
            if (entity := db.get_card(cid)) is None or entity.card_kind != CardKind.DigiEgg
        )
        egg_count = len(card_ids) - main_count
        if main_count != 70:
            errors.append(f"EDH Commander main deck must be exactly 70 cards (got {main_count})")
        if egg_count > 5:
            errors.append(f"Digi-Egg deck must be 0-5 cards (got {egg_count})")
        return len(errors) == 0, errors

    # Standard mode — use full validation
    result = validate_deck(card_ids)
    return result.is_valid, result.errors


def _normalize_alt_arts(flags: list[bool] | None, expected_len: int) -> list[bool]:
    """Pad/truncate an alt-art flag list to match its card-id list length.

    Accepts `None` or empty input and returns a matching-length list of
    `False` values, so older decks saved before alt-art support transparently
    become "all base art".
    """
    if not flags:
        return [False] * expected_len
    # Explicitly coerce to booleans in case JSON round-tripped as ints.
    out = [bool(x) for x in flags[:expected_len]]
    if len(out) < expected_len:
        out.extend([False] * (expected_len - len(out)))
    return out


def _deck_to_response(deck: Deck) -> DeckResponse:
    main_ids = json.loads(deck.main_deck)
    egg_ids = json.loads(deck.egg_deck) if deck.egg_deck else []
    main_alts = json.loads(deck.main_deck_alt_arts) if deck.main_deck_alt_arts else []
    egg_alts = json.loads(deck.egg_deck_alt_arts) if deck.egg_deck_alt_arts else []
    return DeckResponse(
        id=deck.id,
        owner_id=deck.owner_id,
        folder_id=deck.folder_id,
        name=deck.name,
        description=deck.description or "",
        game_mode=deck.game_mode,
        titan_role=deck.titan_role,
        main_deck=main_ids,
        egg_deck=egg_ids,
        main_deck_alt_arts=_normalize_alt_arts(main_alts, len(main_ids)),
        egg_deck_alt_arts=_normalize_alt_arts(egg_alts, len(egg_ids)),
        commander_id=deck.commander_id,
        is_valid=bool(deck.is_valid),
        validation_errors=json.loads(deck.validation_errors) if deck.validation_errors else [],
        is_public=bool(deck.is_public),
        is_pinned=bool(deck.is_pinned),
        tags=json.loads(deck.tags) if deck.tags else [],
        meta_tier=deck.meta_tier,
        meta_archetype=deck.meta_archetype,
        created_at=deck.created_at,
        updated_at=deck.updated_at,
    )


def _deck_to_summary(deck: Deck) -> DeckSummary:
    main_ids = json.loads(deck.main_deck)
    egg_ids = json.loads(deck.egg_deck) if deck.egg_deck else []
    tags = json.loads(deck.tags) if deck.tags else []
    return DeckSummary(
        id=deck.id,
        name=deck.name,
        description=deck.description or "",
        game_mode=deck.game_mode,
        is_valid=bool(deck.is_valid),
        is_public=bool(deck.is_public),
        is_pinned=bool(deck.is_pinned),
        folder_id=deck.folder_id,
        card_count=len(main_ids) + len(egg_ids),
        main_count=len(main_ids),
        egg_count=len(egg_ids),
        tags=tags,
        meta_tier=deck.meta_tier,
        meta_archetype=deck.meta_archetype,
        colors=[],
        highest_level=None,
        created_at=deck.created_at,
        updated_at=deck.updated_at,
    )


async def _ensure_default_folders(user_id: str, db: AsyncSession) -> None:
    result = await db.execute(select(func.count()).select_from(DeckFolder).where(DeckFolder.owner_id == user_id))
    if (result.scalar() or 0) > 0:
        return
    for idx, name in enumerate(DEFAULT_FOLDER_NAMES):
        db.add(DeckFolder(owner_id=user_id, name=name, sort_order=idx))
    await db.commit()


async def _get_owned_folder(folder_id: str, user_id: str, db: AsyncSession) -> DeckFolder:
    result = await db.execute(
        select(DeckFolder).where(DeckFolder.id == folder_id, DeckFolder.owner_id == user_id)
    )
    folder = result.scalar_one_or_none()
    if not folder:
        raise HTTPException(status_code=404, detail="Folder not found")
    return folder


@router.post("", response_model=DeckResponse, status_code=status.HTTP_201_CREATED)
async def create_deck(
    request: CreateDeckRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    # Validate mode-specific constraints on request
    if request.game_mode == "edh_commander" and not request.commander_id:
        raise HTTPException(status_code=400, detail="EDH Commander mode requires a commander_id")
    if request.game_mode == "titan" and not request.titan_role:
        raise HTTPException(status_code=400, detail="Titan mode requires a titan_role (titan or team)")
    if request.game_mode != "edh_commander" and request.commander_id:
        raise HTTPException(status_code=400, detail="commander_id only valid for edh_commander mode")
    if request.game_mode != "titan" and request.titan_role:
        raise HTTPException(status_code=400, detail="titan_role only valid for titan mode")

    # Alpha gate: reject cards without behavioral test coverage.
    _reject_untested_cards(request.main_deck, request.egg_deck)

    # Run validation
    all_cards = request.main_deck + request.egg_deck
    is_valid, errors = _validate_for_mode(all_cards, request.game_mode, request.titan_role)

    main_alts = _normalize_alt_arts(request.main_deck_alt_arts, len(request.main_deck))
    egg_alts = _normalize_alt_arts(request.egg_deck_alt_arts, len(request.egg_deck))

    archetype, tier = tag_deck(request.main_deck + request.egg_deck)

    deck = Deck(
        owner_id=user.id,
        name=request.name,
        description=request.description,
        game_mode=request.game_mode,
        titan_role=request.titan_role,
        main_deck=json.dumps(request.main_deck),
        egg_deck=json.dumps(request.egg_deck),
        main_deck_alt_arts=json.dumps(main_alts),
        egg_deck_alt_arts=json.dumps(egg_alts),
        commander_id=request.commander_id,
        is_valid=1 if is_valid else 0,
        validation_errors=json.dumps(errors),
        is_public=1 if request.is_public else 0,
        tags=json.dumps(request.tags),
        meta_archetype=archetype,
        meta_tier=tier,
    )
    db.add(deck)
    await db.commit()
    await db.refresh(deck)
    return _deck_to_response(deck)


@router.get("", response_model=List[DeckSummary])
async def list_my_decks(
    game_mode: Optional[str] = Query(None, pattern=r"^(standard|edh_commander|titan|no_restriction)$"),
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    query = select(Deck).where(Deck.owner_id == user.id)
    if game_mode:
        query = query.where(Deck.game_mode == game_mode)
    query = query.order_by(Deck.updated_at.desc())

    result = await db.execute(query)
    decks = result.scalars().all()
    return [_deck_to_summary(d) for d in decks]


@router.get("/public", response_model=List[DeckSummary])
async def list_public_decks(
    game_mode: Optional[str] = Query(None, pattern=r"^(standard|edh_commander|titan|no_restriction)$"),
    db: AsyncSession = Depends(get_db),
):
    query = select(Deck).where(Deck.is_public == 1)
    if game_mode:
        query = query.where(Deck.game_mode == game_mode)
    query = query.order_by(Deck.updated_at.desc()).limit(50)

    result = await db.execute(query)
    decks = result.scalars().all()
    return [_deck_to_summary(d) for d in decks]


@router.get("/folders", response_model=List[DeckFolderResponse])
async def list_deck_folders(
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    await _ensure_default_folders(user.id, db)
    result = await db.execute(
        select(DeckFolder)
        .where(DeckFolder.owner_id == user.id)
        .order_by(DeckFolder.sort_order.asc(), DeckFolder.name.asc())
    )
    return result.scalars().all()


@router.post("/folders", response_model=DeckFolderResponse, status_code=status.HTTP_201_CREATED)
async def create_deck_folder(
    request: CreateDeckFolderRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    folder = DeckFolder(owner_id=user.id, name=request.name.strip(), sort_order=request.sort_order)
    db.add(folder)
    try:
        await db.commit()
    except IntegrityError as exc:
        await db.rollback()
        raise HTTPException(status_code=409, detail="Folder name already exists") from exc
    await db.refresh(folder)
    return folder


@router.put("/folders/{folder_id}", response_model=DeckFolderResponse)
async def update_deck_folder(
    folder_id: str,
    request: UpdateDeckFolderRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    folder = await _get_owned_folder(folder_id, user.id, db)
    if request.name is not None:
        folder.name = request.name.strip()
    if request.sort_order is not None:
        folder.sort_order = request.sort_order
    try:
        await db.commit()
    except IntegrityError as exc:
        await db.rollback()
        raise HTTPException(status_code=409, detail="Folder name already exists") from exc
    await db.refresh(folder)
    return folder


@router.delete("/folders/{folder_id}")
async def delete_deck_folder(
    folder_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    folder = await _get_owned_folder(folder_id, user.id, db)
    await db.execute(
        update(Deck)
        .where(Deck.owner_id == user.id, Deck.folder_id == folder.id)
        .values(folder_id=None)
    )
    await db.delete(folder)
    await db.commit()
    return {"status": "deleted"}


@router.patch("/{deck_id}/library", response_model=DeckResponse)
async def update_deck_library_fields(
    deck_id: str,
    request: UpdateDeckLibraryRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Deck).where(Deck.id == deck_id))
    deck = result.scalar_one_or_none()
    if not deck:
        raise HTTPException(status_code=404, detail="Deck not found")
    if deck.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Access denied")

    if "folder_id" in request.model_fields_set:
        if request.folder_id is not None:
            await _get_owned_folder(request.folder_id, user.id, db)
        deck.folder_id = request.folder_id
    if request.is_pinned is not None:
        deck.is_pinned = 1 if request.is_pinned else 0

    await db.commit()
    await db.refresh(deck)
    return _deck_to_response(deck)


@router.get("/{deck_id}", response_model=DeckResponse)
async def get_deck(
    deck_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Deck).where(Deck.id == deck_id))
    deck = result.scalar_one_or_none()
    if not deck:
        raise HTTPException(status_code=404, detail="Deck not found")
    # Allow access if owner or public
    if deck.owner_id != user.id and not deck.is_public:
        raise HTTPException(status_code=403, detail="Access denied")
    return _deck_to_response(deck)


@router.put("/{deck_id}", response_model=DeckResponse)
async def update_deck(
    deck_id: str,
    request: UpdateDeckRequest,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Deck).where(Deck.id == deck_id))
    deck = result.scalar_one_or_none()
    if not deck:
        raise HTTPException(status_code=404, detail="Deck not found")
    if deck.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Access denied")

    # Alpha gate: reject cards without behavioral test coverage. Use the
    # submitted lists if provided, otherwise fall back to the current deck.
    proposed_main = (
        request.main_deck if request.main_deck is not None else json.loads(deck.main_deck)
    )
    proposed_egg = (
        request.egg_deck
        if request.egg_deck is not None
        else json.loads(deck.egg_deck or "[]")
    )
    _reject_untested_cards(proposed_main, proposed_egg)

    # Apply updates
    if request.name is not None:
        deck.name = request.name
    if request.description is not None:
        deck.description = request.description
    if request.main_deck is not None:
        deck.main_deck = json.dumps(request.main_deck)
        # Normalize whatever alt-art list the client sent against the new
        # card list length, or reset to all-base-art if the client didn't
        # send one alongside the card update.
        main_alts = _normalize_alt_arts(request.main_deck_alt_arts, len(request.main_deck))
        deck.main_deck_alt_arts = json.dumps(main_alts)
    elif request.main_deck_alt_arts is not None:
        current_len = len(json.loads(deck.main_deck))
        deck.main_deck_alt_arts = json.dumps(
            _normalize_alt_arts(request.main_deck_alt_arts, current_len)
        )
    if request.egg_deck is not None:
        deck.egg_deck = json.dumps(request.egg_deck)
        egg_alts = _normalize_alt_arts(request.egg_deck_alt_arts, len(request.egg_deck))
        deck.egg_deck_alt_arts = json.dumps(egg_alts)
    elif request.egg_deck_alt_arts is not None:
        current_len = len(json.loads(deck.egg_deck or "[]"))
        deck.egg_deck_alt_arts = json.dumps(
            _normalize_alt_arts(request.egg_deck_alt_arts, current_len)
        )
    if request.commander_id is not None:
        deck.commander_id = request.commander_id
    if request.is_public is not None:
        deck.is_public = 1 if request.is_public else 0
    if request.tags is not None:
        deck.tags = json.dumps(request.tags)

    # Re-validate
    all_cards = json.loads(deck.main_deck) + json.loads(deck.egg_deck or "[]")
    is_valid, errors = _validate_for_mode(all_cards, deck.game_mode, deck.titan_role)
    deck.is_valid = 1 if is_valid else 0
    deck.validation_errors = json.dumps(errors)

    # Re-tag meta classification if the card list changed.
    if request.main_deck is not None or request.egg_deck is not None:
        archetype, tier = tag_deck(all_cards)
        deck.meta_archetype = archetype
        deck.meta_tier = tier

    # Save version snapshot if card list changed
    if request.main_deck is not None or request.egg_deck is not None:
        # Get next version number
        max_ver = await db.execute(
            select(func.max(DeckVersion.version_number)).where(DeckVersion.deck_id == deck_id)
        )
        current_max = max_ver.scalar() or 0
        version = DeckVersion(
            deck_id=deck_id,
            version_number=current_max + 1,
            main_deck=deck.main_deck,
            egg_deck=deck.egg_deck or "[]",
            main_deck_alt_arts=deck.main_deck_alt_arts or "[]",
            egg_deck_alt_arts=deck.egg_deck_alt_arts or "[]",
            commander_id=deck.commander_id,
            change_note=request.change_note,
        )
        db.add(version)

    await db.commit()
    await db.refresh(deck)
    return _deck_to_response(deck)


@router.delete("/{deck_id}")
async def delete_deck(
    deck_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Deck).where(Deck.id == deck_id))
    deck = result.scalar_one_or_none()
    if not deck:
        raise HTTPException(status_code=404, detail="Deck not found")
    if deck.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Access denied")
    await db.delete(deck)
    await db.commit()
    return {"status": "deleted"}


@router.post("/{deck_id}/validate", response_model=DeckResponse)
async def validate_existing_deck(
    deck_id: str,
    user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Deck).where(Deck.id == deck_id))
    deck = result.scalar_one_or_none()
    if not deck:
        raise HTTPException(status_code=404, detail="Deck not found")
    if deck.owner_id != user.id:
        raise HTTPException(status_code=403, detail="Access denied")

    all_cards = json.loads(deck.main_deck) + json.loads(deck.egg_deck or "[]")
    is_valid, errors = _validate_for_mode(all_cards, deck.game_mode, deck.titan_role)
    deck.is_valid = 1 if is_valid else 0
    deck.validation_errors = json.dumps(errors)
    await db.commit()
    await db.refresh(deck)
    return _deck_to_response(deck)
