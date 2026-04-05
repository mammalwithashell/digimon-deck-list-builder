"""Resolve archetype deck lists into enriched card manifests.

Primary consumer: skill agents (implement-archetype, batch-fix-cards,
review-archetype). Also usable as a CLI tool for human inspection.

Usage as library:
    from tools.resolve_deck import resolve_archetype, resolve_cards
    manifest = resolve_archetype("Royal Knights")
    cards = resolve_cards(["BT24-017", "BT24-018"])

Usage as CLI:
    python tools/resolve_deck.py "Royal Knights"
    python tools/resolve_deck.py "Royal Knights" --json
    python tools/resolve_deck.py --cards BT24-017,BT24-018
    python tools/resolve_deck.py --list-archetypes
"""

from __future__ import annotations

import glob
import json
import os
import sys
from collections import Counter
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from digimon_gym.engine.data.card_database import CardDatabase
from digimon_gym.engine.data.enums import CardKind

_DATA_DIR = _PROJECT_ROOT / "digimon_gym" / "engine" / "data"
_MANIFEST_PATH = _DATA_DIR / "scripts" / "_frozen_manifest.json"
_DECK_LIBRARY_PATH = _DATA_DIR / "deck_library.json"
_SCRIPTS_DIR = _DATA_DIR / "scripts"
_DCGO_DIR = _PROJECT_ROOT / "DCGO" / "Assets" / "Scripts" / "CardEffect"
_QA_DIR = _PROJECT_ROOT / "qa" / "archetype-qa"

_KIND_NAMES = {
    CardKind.Digimon: "Digimon",
    CardKind.Tamer: "Tamer",
    CardKind.Option: "Option",
    CardKind.DigiEgg: "DigiEgg",
}


@dataclass
class CardEntry:
    """Enriched metadata for a single card in an archetype's pool."""

    card_id: str
    card_name: str
    card_kind: str
    level: Optional[int]
    colors: list[str]
    traits: str
    dp: Optional[int]
    play_cost: Optional[int]
    evo_costs: list[dict]
    effect_text: str
    inherited_text: str
    security_text: str
    script_status: str
    script_path: Optional[str]
    csharp_path: Optional[str]
    deck_frequency: int


@dataclass
class ArchetypeManifest:
    """Full resolved manifest for an archetype's card pool."""
    archetype_name: str
    input_name: str
    total_decklists: int
    unique_cards: list[CardEntry]
    meta_share: float
    coverage_pct: float
    frozen_count: int
    generated_count: int
    missing_count: int
    missing_cards: list[str]
    best_decklist: list[str]
    deck_pool_path: str


def _load_frozen_manifest() -> dict:
    """Load _frozen_manifest.json and return the 'cards' dict."""
    try:
        with open(_MANIFEST_PATH, "r", encoding="utf-8") as f:
            return json.load(f).get("cards", {})
    except FileNotFoundError:
        return {}


def _card_id_to_script_parts(card_id: str) -> tuple[str, str]:
    """Convert 'BT10-001' to ('bt10', 'bt10_001') for path construction."""
    parts = card_id.split("-")
    set_id = parts[0].lower()
    num = parts[1] if len(parts) > 1 else ""
    module = f"{set_id}_{num}"
    return set_id, module


def _find_csharp_path(card_id: str) -> Optional[str]:
    """Glob for the C# reference script. Returns path relative to project root or None."""
    parts = card_id.split("-")
    set_upper = parts[0].upper()
    cs_name = card_id.replace("-", "_")
    pattern = str(_DCGO_DIR / set_upper / "*" / f"{cs_name}.cs")
    matches = glob.glob(pattern)
    if matches:
        return str(Path(matches[0]).relative_to(_PROJECT_ROOT))
    return None


def _resolve_script_status(
    card_id: str, manifest: dict
) -> tuple[str, Optional[str]]:
    """Determine script status and path for a card."""
    entry = manifest.get(card_id)
    if entry:
        if entry.get("frozen_relpath"):
            rel = entry["frozen_relpath"]
            full = _SCRIPTS_DIR / rel
            if full.exists():
                return "frozen", f"digimon_gym/engine/data/scripts/{rel}"
        if entry.get("generated_relpath"):
            rel = entry["generated_relpath"]
            full = _SCRIPTS_DIR / rel
            if full.exists():
                return "generated", f"digimon_gym/engine/data/scripts/{rel}"

    set_id, module = _card_id_to_script_parts(card_id)
    gen_path = _SCRIPTS_DIR / "generated" / set_id / f"{module}.py"
    if gen_path.exists():
        return "generated", f"digimon_gym/engine/data/scripts/generated/{set_id}/{module}.py"

    direct_path = _SCRIPTS_DIR / set_id / f"{module}.py"
    if direct_path.exists():
        return "frozen", f"digimon_gym/engine/data/scripts/{set_id}/{module}.py"

    return "missing", None


def _build_card_entry(
    card_id: str,
    manifest: dict,
    db: CardDatabase,
    frequency: int = 0,
) -> CardEntry:
    """Build an enriched CardEntry for a single card ID."""
    card = db.get_card(card_id)

    if card:
        card_name = card.card_name_eng
        card_kind = _KIND_NAMES.get(card.card_kind, "Digimon")
        level = card.level if card.level and card.level > 0 else None
        colors = [c.name for c in card.card_colors]
        traits = ", ".join(card.type_eng) if card.type_eng else ""
        dp = card.dp if card.dp is not None and card.dp > 0 else None
        play_cost = card.play_cost if card.play_cost is not None else None
        evo_costs = [
            {"color": ec.card_color.name if hasattr(ec, "card_color") and ec.card_color else "",
             "cost": ec.memory_cost if hasattr(ec, "memory_cost") else 0,
             "level": ec.level if hasattr(ec, "level") else 0}
            for ec in (card.evo_costs or [])
        ]
        effect_text = card.effect_description_eng or ""
        inherited_text = card.inherited_effect_description_eng or ""
        security_text = card.security_effect_description_eng or ""
    else:
        card_name = ""
        card_kind = "Digimon"
        level = None
        colors = []
        traits = ""
        dp = None
        play_cost = None
        evo_costs = []
        effect_text = ""
        inherited_text = ""
        security_text = ""

    status, script_path = _resolve_script_status(card_id, manifest)
    csharp_path = _find_csharp_path(card_id)

    return CardEntry(
        card_id=card_id,
        card_name=card_name,
        card_kind=card_kind,
        level=level,
        colors=colors,
        traits=traits,
        dp=dp,
        play_cost=play_cost,
        evo_costs=evo_costs,
        effect_text=effect_text,
        inherited_text=inherited_text,
        security_text=security_text,
        script_status=status,
        script_path=script_path,
        csharp_path=csharp_path,
        deck_frequency=frequency,
    )


def resolve_cards(card_ids: list[str]) -> list[CardEntry]:
    """Enrich a raw list of card IDs without archetype context.

    Deduplicates input. Returns sorted by card_id.
    No deck_pool.json written. deck_frequency is 0 for all entries.
    """
    manifest = _load_frozen_manifest()
    db = CardDatabase()
    unique = sorted(set(card_ids))
    return [_build_card_entry(cid, manifest, db) for cid in unique]


def _slugify(name: str) -> str:
    """Convert archetype name to a filesystem-safe slug."""
    import re
    slug = name.lower().strip()
    slug = re.sub(r"[^a-z0-9]+", "-", slug)
    slug = slug.strip("-")
    return slug


def _write_deck_pool(slug: str, card_ids: list[str]) -> str:
    """Write sorted card IDs to qa/archetype-qa/{slug}/deck_pool.json."""
    pool_dir = _QA_DIR / slug
    pool_dir.mkdir(parents=True, exist_ok=True)
    pool_path = pool_dir / "deck_pool.json"
    pool_path.write_text(
        json.dumps(sorted(card_ids), indent=2) + "\n", encoding="utf-8"
    )
    try:
        return str(pool_path.relative_to(_PROJECT_ROOT))
    except ValueError:
        return str(pool_path)


def resolve_archetype(
    name: str,
    *,
    cards_override: list[str] | None = None,
) -> ArchetypeManifest:
    """Resolve an archetype to its full enriched card manifest."""
    from tools.meta_loader import canonicalize_archetype, SOURCE_PRIORITY

    canonical = canonicalize_archetype(name) if not cards_override else name
    slug = _slugify(canonical)

    if cards_override:
        unique_ids = sorted(set(cards_override))
        frequency: dict[str, int] = Counter(cards_override)
        total_decklists = 0
        meta_share = 0.0
        best_decklist: list[str] = []
    else:
        try:
            with open(_DECK_LIBRARY_PATH, "r", encoding="utf-8") as f:
                library = json.load(f)
        except FileNotFoundError:
            library = {"archetypes": {}}

        archetype_data = library.get("archetypes", {}).get(canonical, {})
        decklists = archetype_data.get("decklists", [])
        total_decklists = len(decklists)
        meta_share = archetype_data.get("stats", {}).get("meta_share", 0.0)

        frequency = Counter()
        for dl in decklists:
            try:
                cards = json.loads(dl.get("decklist", "[]"))
            except (json.JSONDecodeError, TypeError):
                continue
            for cid in set(cards):
                frequency[cid] += 1

        unique_ids = sorted(frequency.keys())

        best_decklist = []
        if decklists:
            sorted_dls = sorted(
                decklists,
                key=lambda d: SOURCE_PRIORITY.get(d.get("source", ""), 0),
                reverse=True,
            )
            for dl in sorted_dls:
                try:
                    parsed = json.loads(dl.get("decklist", "[]"))
                    if parsed:
                        best_decklist = parsed
                        break
                except (json.JSONDecodeError, TypeError):
                    continue

    manifest_data = _load_frozen_manifest()
    db = CardDatabase()
    entries = [
        _build_card_entry(cid, manifest_data, db, frequency.get(cid, 0))
        for cid in unique_ids
    ]

    frozen_count = sum(1 for e in entries if e.script_status == "frozen")
    generated_count = sum(1 for e in entries if e.script_status == "generated")
    missing_count = sum(1 for e in entries if e.script_status == "missing")
    missing_cards = [e.card_id for e in entries if e.script_status == "missing"]
    coverage_pct = frozen_count / len(entries) if entries else 0.0

    deck_pool_path = _write_deck_pool(slug, unique_ids) if unique_ids else ""

    return ArchetypeManifest(
        archetype_name=canonical,
        input_name=name,
        total_decklists=total_decklists,
        unique_cards=entries,
        meta_share=meta_share,
        coverage_pct=coverage_pct,
        frozen_count=frozen_count,
        generated_count=generated_count,
        missing_count=missing_count,
        missing_cards=missing_cards,
        best_decklist=best_decklist,
        deck_pool_path=deck_pool_path,
    )
