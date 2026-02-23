"""Deck parsing and validation utility endpoints."""

from __future__ import annotations

from fastapi import APIRouter, HTTPException

from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.deck_loader import parse_deck, summarize_deck, validate_deck
from digimon_gym.engine.data.enums import CardKind
from digimon_gym.routers.schemas import DeckParseRequest, DeckValidateRequest

router = APIRouter(tags=["deck-tools"])


@router.post("/decks/parse")
@router.post("/deck/parse", include_in_schema=False)
def deck_parse(request: DeckParseRequest):
    """Parse a deck string and classify cards into main and egg deck."""
    try:
        card_ids = parse_deck(request.deck)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    db = CardDatabase()
    main_deck: list[str] = []
    egg_deck: list[str] = []
    warnings: list[str] = []
    seen_unknown: set[str] = set()

    for card_id in card_ids:
        entity = db.get_card(card_id)
        if entity is None:
            main_deck.append(card_id)
            if card_id not in seen_unknown:
                warnings.append(f"Unknown card: {card_id} (not in card database)")
                seen_unknown.add(card_id)
        elif entity.card_kind == CardKind.DigiEgg:
            egg_deck.append(card_id)
        else:
            main_deck.append(card_id)

    return {
        "main_deck": main_deck,
        "egg_deck": egg_deck,
        "warnings": warnings,
    }


@router.post("/decks/validate")
@router.post("/deck/validate", include_in_schema=False)
def deck_validate(request: DeckValidateRequest):
    """Validate a deck from raw string input or card-id arrays."""
    if request.deck is not None:
        try:
            card_ids = parse_deck(request.deck)
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=str(exc))
    else:
        card_ids = request.main_deck + request.egg_deck
        if not card_ids:
            raise HTTPException(status_code=400, detail="Provide deck or main_deck/egg_deck")

    result = validate_deck(card_ids)
    summary = summarize_deck(card_ids)
    return {
        "is_valid": result.is_valid,
        "errors": result.errors,
        "warnings": result.warnings,
        "summary": summary,
        "total_cards": len(card_ids),
    }

