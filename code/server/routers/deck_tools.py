"""Deck parsing and validation utility endpoints."""

from __future__ import annotations

import json
from functools import lru_cache

from fastapi import APIRouter, HTTPException

from data_paths import CARDS_JSON
from digimon_engine import (
    CardDatabase,
    CardKind,
    load_tested_cards,
    out_of_set_cards,
    parse_deck,
    summarize_deck,
    validate_deck,
    validate_deck_for_game_mode,
)
from server.routers.schemas import DeckParseRequest, DeckValidateRequest

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

    if request.game_mode in {"standard", "edh_commander", "titan"}:
        result = validate_deck(card_ids)
    else:
        result = validate_deck_for_game_mode(card_ids, request.game_mode)
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

    The list is derived from the Rust engine's implemented-card registry
    (``code/tools/build_tested_cards.py``) and committed as
    ``data/tested_cards.json``.
    """
    card_ids = sorted(load_tested_cards())
    return {"card_ids": card_ids, "card_count": len(card_ids)}


_COLOR_NAMES = ["Red", "Blue", "Yellow", "Green", "White", "Black", "Purple"]
_KIND_NAMES = ["Digimon", "Tamer", "Option", "Digi-Egg"]
_RARITY_NAMES = ["C", "U", "R", "SR", "SEC", "P"]


def _name_at(names: list[str], index) -> str:
    if isinstance(index, int) and 0 <= index < len(names):
        return names[index]
    return ""


@lru_cache(maxsize=1)
def _card_database_payload() -> list[dict]:
    """Display metadata for every allowlisted card, sorted by card ID.

    Mirrors the desktop's ``rust_card_database`` Tauri command
    (``digimon_engine::deck_tools::tested_card_metadata``) — keep the
    field set and value mappings in sync with that DTO.
    """
    with CARDS_JSON.open(encoding="utf-8") as f:
        cards: dict[str, dict] = json.load(f)
    tested = load_tested_cards()

    out = []
    for card_id in sorted(tested):
        entry = cards.get(card_id)
        if entry is None:
            continue
        evo_costs = entry.get("evo_costs") or []
        out.append(
            {
                "card_id": card_id,
                "name": entry.get("card_name_eng") or "",
                "card_type": _name_at(_KIND_NAMES, entry.get("card_kind")),
                "colors": [
                    name
                    for c in (entry.get("card_colors") or [])
                    if (name := _name_at(_COLOR_NAMES, c))
                ],
                "level": entry.get("level"),
                "play_cost": entry.get("play_cost"),
                "evolution_cost": (evo_costs[0].get("memory_cost") if evo_costs else None),
                "rarity": _name_at(_RARITY_NAMES, entry.get("rarity")),
                "dp": entry.get("dp"),
                "stage": (entry.get("form_eng") or [""])[0],
                "digi_type": "/".join(entry.get("type_eng") or []),
                "attribute": "/".join(entry.get("attribute_eng") or []),
                "main_effect": entry.get("effect_description_eng") or "",
                "inherited_effect": entry.get("inherited_effect_description_eng") or "",
                "security_effect": entry.get("security_effect_description_eng") or "",
            }
        )
    return out


@router.get("/decks/card-database")
def card_database_endpoint():
    """Full display metadata for every implemented (allowlisted) card.

    The deck builder seeds its browse pool from this so the whole
    implemented pool is visible without depending on the external
    digimoncard.io search API.
    """
    return _card_database_payload()

