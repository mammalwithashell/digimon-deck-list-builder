"""Deck parsing and validation utility endpoints."""

from __future__ import annotations

from fastapi import APIRouter, HTTPException

from digimon_engine import (
    CardDatabase,
    CardKind,
    load_tested_cards,
    out_of_set_cards,
    parse_deck,
    summarize_deck,
    validate_deck,
)
from digimon_gym.routers.schemas import DeckParseRequest, DeckValidateRequest

router = APIRouter(tags=["deck-tools"])


def _alpha_pool_error(card_id: str) -> str:
    return (
        f"Card {card_id} is not available in the alpha release "
        "(no test coverage)"
    )


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

    # Alpha gate: reject decks with any card that lacks behavioral test
    # coverage. Keeps out-of-scope card scripts from reaching the engine.
    errors = list(result.errors)
    out_of_pool = out_of_set_cards(card_ids)
    is_valid = result.is_valid
    if out_of_pool:
        errors.extend(_alpha_pool_error(cid) for cid in out_of_pool)
        is_valid = False

    return {
        "is_valid": is_valid,
        "errors": errors,
        "warnings": result.warnings,
        "summary": summary,
        "total_cards": len(card_ids),
    }


@router.get("/decks/tested-cards")
def list_tested_cards():
    """Return the allowlist of card IDs available in the alpha deck builder.

    The list is derived from per-card behavioral tests under
    ``tests/behavioral/`` at build time and committed as
    ``data/tested_cards.json``.
    """
    card_ids = sorted(load_tested_cards())
    return {"card_ids": card_ids, "card_count": len(card_ids)}

