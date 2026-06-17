"""Local-file deck access for the browser-dev workflow.

The desktop client stores decks as JSON files under Tauri's
`app_data_dir()` — on Windows that's
`%APPDATA%\\com.digimon-tcg.desktop\\decks\\<uuid>.json`. The desktop
client reads them via the `decks_list` / `decks_get` Tauri commands
(see `code/src-tauri/src/deck_storage.rs`).

Running the same frontend bundle in a plain browser (the test workflow)
can't reach Tauri commands, so it needs an HTTP route that reads from
the *same* file location. This router provides exactly that: a
read-only mirror of the deck-storage Tauri commands. The path is
resolved via `platformdirs` using the same identifier Tauri uses so
both paths land on the same files.

Read-only by design — write operations stay on the desktop side. The
browser is a test harness, not a deckbuilder.
"""

from __future__ import annotations

import json
import uuid as uuid_module
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import platformdirs
from fastapi import APIRouter, Body, HTTPException

from digimon_engine import validate_deck_for_game_mode

router = APIRouter(tags=["desktop-decks"])

# Matches `identifier` in `code/src-tauri/tauri.conf.json`. If the
# desktop bundle ever moves to a new identifier, change here too.
TAURI_BUNDLE_IDENTIFIER = "com.digimon-tcg.desktop"


def _app_data_dir() -> Path:
    """Resolve the Tauri `app_data_dir()` root. `platformdirs` with
    `appauthor=False, roaming=True` reproduces Tauri's Windows scheme
    (`%APPDATA%/<identifier>`); on macOS / Linux it lands on the same
    location Tauri uses too (`~/Library/Application Support/<id>` and
    `~/.local/share/<id>` respectively)."""
    return Path(
        platformdirs.user_data_dir(
            TAURI_BUNDLE_IDENTIFIER, appauthor=False, roaming=True
        )
    )


def _decks_dir() -> Path:
    """Path to the deck-files directory under the Tauri app data root."""
    return _app_data_dir() / "decks"


def _library_metadata_path() -> Path:
    """The `deck_library.json` sibling file that holds folders. Created
    on first folder write; reads tolerate its absence."""
    return _app_data_dir() / "deck_library.json"


def _read_library() -> dict[str, Any]:
    """Load `deck_library.json` or return an empty stub. The Tauri side
    seeds three default folders (Tournament / Experimental / Casual)
    on first launch; in browser-only sessions the file may not exist."""
    path = _library_metadata_path()
    if not path.exists():
        return {"folders": []}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {"folders": []}


def _read_deck(path: Path) -> dict[str, Any]:
    """Load and validate a deck JSON. Defaults the optional fields the
    `Deck` struct in `deck_storage.rs` flags with `#[serde(default)]`
    so the frontend's `DeckResponse` type sees a complete shape."""
    raw = json.loads(path.read_text(encoding="utf-8"))
    raw.setdefault("folder_id", None)
    raw.setdefault("description", "")
    raw.setdefault("main_deck_alt_arts", [])
    raw.setdefault("egg_deck_alt_arts", [])
    raw.setdefault("commander_id", None)
    raw.setdefault("is_valid", False)
    raw.setdefault("validation_errors", [])
    raw.setdefault("is_public", False)
    raw.setdefault("is_pinned", False)
    raw.setdefault("tags", [])
    raw.setdefault("meta_tier", None)
    raw.setdefault("meta_archetype", None)
    return raw


def _summarize(deck: dict[str, Any]) -> dict[str, Any]:
    """Convert a full deck dict to the `DeckSummary` shape the frontend's
    listing UI expects. Mirrors `DeckSummary` in `deck_storage.rs`."""
    main = deck.get("main_deck") or []
    eggs = deck.get("egg_deck") or []
    commander_id = deck.get("commander_id")
    all_ids = set(main) | set(eggs)
    deck_icon_card_id = (
        commander_id
        if commander_id and commander_id in all_ids
        else sorted(main)[0]
        if main
        else sorted(eggs)[0]
        if eggs
        else None
    )
    return {
        "id": deck["id"],
        "name": deck.get("name", "Untitled"),
        "description": deck.get("description", ""),
        "game_mode": deck.get("game_mode", "standard"),
        "is_valid": deck.get("is_valid", False),
        "is_public": deck.get("is_public", False),
        "is_pinned": deck.get("is_pinned", False),
        "folder_id": deck.get("folder_id"),
        "card_count": len(main) + len(eggs),
        "main_count": len(main),
        "egg_count": len(eggs),
        "tags": deck.get("tags", []),
        "commander_id": commander_id,
        "deck_icon_card_id": deck_icon_card_id,
        "meta_tier": deck.get("meta_tier"),
        "meta_archetype": deck.get("meta_archetype"),
        # The desktop's summary computes `colors` / `highest_level` from
        # card data; in browser-dev we skip those (the UI tolerates
        # missing values).
        "colors": [],
        "highest_level": None,
        "created_at": deck.get("created_at", ""),
        "updated_at": deck.get("updated_at", ""),
    }


@router.get("/desktop-decks")
def list_desktop_decks() -> list[dict[str, Any]]:
    """List deck summaries from the Tauri local-file location.
    Returns `[]` if the directory doesn't exist yet (fresh install)."""
    decks_dir = _decks_dir()
    if not decks_dir.exists():
        return []
    summaries: list[dict[str, Any]] = []
    for entry in sorted(decks_dir.glob("*.json")):
        try:
            summaries.append(_summarize(_read_deck(entry)))
        except (json.JSONDecodeError, KeyError, OSError):
            # Skip broken files; matches the desktop's tolerant listing.
            continue
    return summaries


@router.get("/desktop-decks/folders")
def list_desktop_folders_first() -> list[dict[str, Any]]:
    """List deck folders from `deck_library.json`. Declared *before*
    `/{deck_id}` so the catch-all path param doesn't swallow "folders"
    as a deck id and 404. FastAPI matches in declaration order."""
    lib = _read_library()
    folders = lib.get("folders") or []
    return sorted(folders, key=lambda f: f.get("sort_order", 0))


@router.get("/desktop-decks/{deck_id}")
def get_desktop_deck(deck_id: str) -> dict[str, Any]:
    """Load a single full deck by id from the local-file location."""
    path = _decks_dir() / f"{deck_id}.json"
    if not path.exists():
        raise HTTPException(status_code=404, detail="Deck not found")
    try:
        return _read_deck(path)
    except (json.JSONDecodeError, OSError) as exc:
        raise HTTPException(status_code=500, detail=f"Deck read failed: {exc}")


def _validate(main_deck: list[str], egg_deck: list[str], game_mode: str) -> tuple[bool, list[str]]:
    """Re-validate the saved deck via the Rust engine. Mirrors what the
    desktop's `deckStore.putDeck()` does before invoking `decks_put`."""
    try:
        result = validate_deck_for_game_mode(list(main_deck) + list(egg_deck), game_mode)
        return result.is_valid, list(result.errors)
    except Exception as exc:  # pragma: no cover — engine errors are caller-visible
        return False, [f"validation backend errored: {exc}"]


@router.post("/desktop-decks")
def save_desktop_deck(deck: dict[str, Any] = Body(...)) -> dict[str, Any]:
    """Save a deck JSON file under the Tauri app-data deck dir. Mirrors
    the desktop's `decks_put` Tauri command: re-validates, fills
    defaults, writes `<id>.json`, returns the full deck.

    Accepts a partial deck — generates an id and timestamps if missing.
    Saving an existing id overwrites that file. Owner is fixed to
    `'guest'` since browser-dev has no auth."""
    if not isinstance(deck, dict):
        raise HTTPException(status_code=400, detail="Body must be a deck object")

    main_deck = list(deck.get("main_deck") or [])
    egg_deck = list(deck.get("egg_deck") or [])
    game_mode = deck.get("game_mode") or "standard"
    if not deck.get("name"):
        raise HTTPException(status_code=400, detail="Deck name is required")
    if not main_deck:
        raise HTTPException(status_code=400, detail="main_deck cannot be empty")
    commander_id = deck.get("commander_id")
    if commander_id and commander_id not in (set(main_deck) | set(egg_deck)):
        raise HTTPException(status_code=400, detail="commander_id must reference a card in the deck")

    # Always re-validate. The desktop client does this in deckStore.ts;
    # the server must do the same so the saved file's `is_valid` flag
    # reflects current engine semantics and the DeckSelectPage's
    # `canUseDeckForFormat` gate works.
    is_valid, errors = _validate(main_deck, egg_deck, game_mode)

    now = datetime.now(timezone.utc).isoformat()
    deck_id = deck.get("id") or str(uuid_module.uuid4())
    decks_dir = _decks_dir()
    decks_dir.mkdir(parents=True, exist_ok=True)

    full: dict[str, Any] = {
        "id": deck_id,
        "folder_id": deck.get("folder_id"),
        # No auth in browser-dev; stamp 'guest' to match the desktop's
        # bootstrap/guest.ts placeholder convention.
        "owner_id": deck.get("owner_id") or "guest",
        "name": deck["name"],
        "description": deck.get("description") or "",
        "game_mode": game_mode,
        "main_deck": main_deck,
        "egg_deck": egg_deck,
        "main_deck_alt_arts": list(deck.get("main_deck_alt_arts") or []),
        "egg_deck_alt_arts": list(deck.get("egg_deck_alt_arts") or []),
        "commander_id": commander_id,
        "is_valid": is_valid,
        "validation_errors": errors,
        "is_public": bool(deck.get("is_public", False)),
        "is_pinned": bool(deck.get("is_pinned", False)),
        "tags": list(deck.get("tags") or []),
        "meta_tier": deck.get("meta_tier"),
        "meta_archetype": deck.get("meta_archetype"),
        "created_at": deck.get("created_at") or now,
        "updated_at": now,
    }

    path = decks_dir / f"{deck_id}.json"
    try:
        path.write_text(json.dumps(full, indent=2), encoding="utf-8")
    except OSError as exc:
        raise HTTPException(status_code=500, detail=f"Deck write failed: {exc}")
    return full


@router.delete("/desktop-decks/{deck_id}")
def delete_desktop_deck(deck_id: str) -> bool:
    """Delete the deck file. Returns `true` if the file existed and was
    removed, `false` if it didn't (no-op delete, matches Tauri behavior).
    The desktop's `decks_delete` returns a bool with the same semantics."""
    path = _decks_dir() / f"{deck_id}.json"
    if not path.exists():
        return False
    try:
        path.unlink()
    except OSError as exc:
        raise HTTPException(status_code=500, detail=f"Deck delete failed: {exc}")
    return True
