"""Ingest engine docs, card scripts, and metadata into Pinecone for sub-agent retrieval.

Uses Pinecone integrated inference — text is auto-embedded on upsert.
Reuses chunking helpers from digimon_gym/ai/retrieval.py.

Usage:
    python tools/ingest_pinecone.py --all
    python tools/ingest_pinecone.py --namespace card-scripts
    python tools/ingest_pinecone.py --namespace card-scripts --set bt10
    python tools/ingest_pinecone.py --all --dry-run
"""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import os
import re
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Bootstrap: ensure repo root is on sys.path so retrieval imports work
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from server.ai.retrieval import (
    chunk_cards_json,
    chunk_markdown_by_section,
    chunk_python_by_ast,
    chunk_text,
)

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
ENGINE_API_DOC = REPO_ROOT / "qa" / "archetype-qa" / "engine-api-reference.md"
ENGINE_GAME_DIR = REPO_ROOT / "digimon_gym" / "engine" / "game"
ENGINE_CORE_DIR = REPO_ROOT / "digimon_gym" / "engine" / "core"
ENGINE_INTERFACES_DIR = REPO_ROOT / "digimon_gym" / "engine" / "interfaces"
ENGINE_VALIDATION_DIR = REPO_ROOT / "digimon_gym" / "engine" / "validation"
SCRIPTS_DIR = REPO_ROOT / "digimon_gym" / "engine" / "data" / "scripts"
GENERATED_DIR = SCRIPTS_DIR / "generated"
CSHARP_DIR = REPO_ROOT / "DCGO" / "Assets" / "Scripts" / "CardEffect"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
from data_paths import CARDS_JSON  # noqa: E402
FROZEN_MANIFEST = SCRIPTS_DIR / "_frozen_manifest.json"

RULES_DOCS = [
    REPO_ROOT / "docs" / "RULES_CONTEXT.md",
    REPO_ROOT / "docs" / "ACTION_SPEC.md",
    REPO_ROOT / "docs" / "TENSOR_SPEC.md",
    REPO_ROOT / "qa" / "archetype-qa" / "engine-gaps.md",
]

ENGINE_SOURCE_FILES = [
    # game/ decomposed modules
    ENGINE_GAME_DIR / "__init__.py",
    ENGINE_GAME_DIR / "constants.py",
    ENGINE_GAME_DIR / "action_decoder.py",
    ENGINE_GAME_DIR / "action_mask.py",
    ENGINE_GAME_DIR / "combat.py",
    ENGINE_GAME_DIR / "effects.py",
    ENGINE_GAME_DIR / "tensor.py",
    ENGINE_GAME_DIR / "serialization.py",
    # core modules
    ENGINE_CORE_DIR / "permanent.py",
    ENGINE_CORE_DIR / "player.py",
    ENGINE_CORE_DIR / "card_source.py",
    # interfaces
    ENGINE_INTERFACES_DIR / "card_effect.py",
    ENGINE_INTERFACES_DIR / "modifiers.py",
    # validation
    ENGINE_VALIDATION_DIR / "digivolve_validator.py",
]

# Keywords regex for card metadata enrichment
KEYWORD_PATTERN = re.compile(
    r"\b(Blocker|Rush|Jamming|Piercing|Security Attack|Recovery|Reboot|"
    r"De-Digivolve|Retaliation|Armor Purge|Blitz|Decoy|Fortitude|"
    r"Mind Link|Save|Material Save|Raid|Alliance|Blast Digivolve|"
    r"Overflow|Ice Clad|Partition|Collision|ACE|Barrier|Evade)\b",
    re.IGNORECASE,
)

INDEX_NAME = "digimon-engine"
BATCH_SIZE = 96  # Pinecone upsert batch limit for integrated inference


def _normalize_text(text: str) -> str:
    """Normalize CRLF to LF for consistent hashing across platforms."""
    return text.replace("\r\n", "\n")


def _content_hash(text: str) -> str:
    """SHA256 of normalized text, truncated to 12 chars."""
    return hashlib.sha256(_normalize_text(text).encode("utf-8")).hexdigest()[:12]


def _read_file(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="latin-1")


def _rel_path(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT)).replace("\\", "/")
    except ValueError:
        return str(path).replace("\\", "/")


def _extract_card_id_from_path(path: Path) -> str | None:
    """Extract card ID like BT10-001 from script filename like bt10_001.py."""
    stem = path.stem  # e.g., bt10_001
    parts = stem.rsplit("_", 1)
    if len(parts) == 2:
        set_part, num_part = parts
        return f"{set_part.upper()}-{num_part}".replace("_", "-")
    return None


def _extract_set_from_path(path: Path) -> str | None:
    """Extract set ID from path like scripts/bt10/bt10_001.py → bt10."""
    parent = path.parent.name
    if parent and parent != "scripts" and parent != "generated":
        return parent.lower()
    return None


def _load_frozen_manifest() -> set[str]:
    """Return set of card IDs that are frozen."""
    if not FROZEN_MANIFEST.exists():
        return set()
    data = json.loads(FROZEN_MANIFEST.read_text(encoding="utf-8"))
    if isinstance(data, dict):
        return set(data.keys())
    return set()


def _get_pinecone_client():
    """Get Pinecone client. Requires PINECONE_API_KEY env var."""
    api_key = os.environ.get("PINECONE_API_KEY")
    if not api_key:
        logger.error("PINECONE_API_KEY environment variable not set")
        sys.exit(1)
    from pinecone import Pinecone
    return Pinecone(api_key=api_key)


# ---------------------------------------------------------------------------
# Namespace builders — each returns list of (vector_id, text, metadata) tuples
# ---------------------------------------------------------------------------

def build_engine_api_records() -> list[tuple[str, str, dict]]:
    """Build records for the engine-api namespace."""
    records = []

    # 1. Engine API reference doc (markdown)
    if ENGINE_API_DOC.exists():
        text = _read_file(ENGINE_API_DOC)
        chunks = chunk_markdown_by_section(text, _rel_path(ENGINE_API_DOC))
        for i, chunk in enumerate(chunks):
            content = chunk["text"]
            vec_id = f"engine-api:{_rel_path(ENGINE_API_DOC)}:{_content_hash(content)}"
            records.append((vec_id, content, {
                "source_type": "engine_api",
                "section_title": chunk.get("section_title", ""),
                "file_path": _rel_path(ENGINE_API_DOC),
            }))

    # 2. Engine source files (Python AST)
    for src_file in ENGINE_SOURCE_FILES:
        if not src_file.exists():
            logger.warning("Engine source not found: %s", src_file)
            continue
        text = _read_file(src_file)
        rel = _rel_path(src_file)
        chunks = chunk_python_by_ast(text, rel)
        for chunk in chunks:
            content = chunk["text"]
            fn = chunk.get("function_name", "")
            cn = chunk.get("class_name", "")
            label = f"{cn}.{fn}" if cn and fn else fn or "module"
            vec_id = f"engine-api:{rel}:{label}:{_content_hash(content)}"
            records.append((vec_id, content, {
                "source_type": "engine_api",
                "function_name": fn or "",
                "class_name": cn or "",
                "file_path": rel,
            }))

    return records


def build_card_script_records(set_filter: str | None = None) -> list[tuple[str, str, dict]]:
    """Build records for the card-scripts namespace."""
    records = []
    frozen = _load_frozen_manifest()

    # Python scripts (frozen lane — non-generated directories)
    script_dirs = sorted(p for p in SCRIPTS_DIR.iterdir()
                         if p.is_dir() and p.name != "generated"
                         and p.name != "__pycache__")
    for script_dir in script_dirs:
        set_id = script_dir.name.lower()
        if set_filter and set_id != set_filter.lower():
            continue
        for py_file in sorted(script_dir.glob("*.py")):
            if py_file.name == "__init__.py":
                continue
            card_id = _extract_card_id_from_path(py_file)
            text = _read_file(py_file)
            is_frozen = card_id in frozen if card_id else False

            # Split large scripts at method boundaries
            if len(text) > 2000:
                chunks = chunk_python_by_ast(text, _rel_path(py_file))
                for i, chunk in enumerate(chunks):
                    content = chunk["text"]
                    vec_id = f"card-scripts:{_rel_path(py_file)}:{i}:{_content_hash(content)}"
                    records.append((vec_id, content, {
                        "source_type": "python_script",
                        "card_id": card_id or "",
                        "set_id": set_id,
                        "is_frozen": is_frozen,
                        "file_path": _rel_path(py_file),
                        "function_name": chunk.get("function_name", ""),
                    }))
            else:
                vec_id = f"card-scripts:{_rel_path(py_file)}:{_content_hash(text)}"
                records.append((vec_id, text, {
                    "source_type": "python_script",
                    "card_id": card_id or "",
                    "set_id": set_id,
                    "is_frozen": is_frozen,
                    "file_path": _rel_path(py_file),
                }))

    # C# reference scripts
    if CSHARP_DIR.exists():
        for cs_file in sorted(CSHARP_DIR.rglob("*.cs")):
            card_id_match = re.match(r"([A-Za-z]+\d+)[_-](\d+)", cs_file.stem)
            if not card_id_match:
                continue
            card_id = f"{card_id_match.group(1).upper()}-{card_id_match.group(2)}"
            set_id_cs = card_id_match.group(1).lower()
            if set_filter and set_id_cs != set_filter.lower():
                continue

            text = _read_file(cs_file)
            if len(text) > 2000:
                # Simple split for C# — use text chunking
                chunks = chunk_text(text, chunk_size=1800, overlap=200)
                for i, content in enumerate(chunks):
                    vec_id = f"card-scripts:{_rel_path(cs_file)}:{i}:{_content_hash(content)}"
                    records.append((vec_id, content, {
                        "source_type": "csharp_script",
                        "card_id": card_id,
                        "set_id": set_id_cs,
                        "is_frozen": False,
                        "file_path": _rel_path(cs_file),
                    }))
            else:
                vec_id = f"card-scripts:{_rel_path(cs_file)}:{_content_hash(text)}"
                records.append((vec_id, text, {
                    "source_type": "csharp_script",
                    "card_id": card_id,
                    "set_id": set_id_cs,
                    "is_frozen": False,
                    "file_path": _rel_path(cs_file),
                }))

    return records


def build_card_metadata_records() -> list[tuple[str, str, dict]]:
    """Build records for the card-metadata namespace."""
    if not CARDS_JSON.exists():
        logger.warning("cards.json not found at %s", CARDS_JSON)
        return []

    text = _read_file(CARDS_JSON)
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        logger.error("Failed to parse cards.json")
        return []

    records = []
    if not isinstance(data, dict):
        logger.error("Expected cards.json to be a dict, got %s", type(data).__name__)
        return []

    for card_id, card_data in data.items():
        # Build a rich text representation for embedding
        name = card_data.get("card_name_eng", "")
        kind_map = {0: "digimon", 1: "tamer", 2: "option", 3: "digitama"}
        card_kind = kind_map.get(card_data.get("card_kind"), str(card_data.get("card_kind", "")))
        level = card_data.get("level")
        colors_map = {0: "Red", 1: "Blue", 2: "Yellow", 3: "Green", 4: "Black", 5: "Purple", 6: "White"}
        colors = [colors_map.get(c, str(c)) for c in card_data.get("card_colors", [])]
        traits = card_data.get("type_eng", [])
        effect = card_data.get("effect_description_eng", "")
        inherited = card_data.get("inherited_effect_description_eng", "")
        security = card_data.get("security_effect_description_eng", "")
        dp = card_data.get("dp")
        play_cost = card_data.get("play_cost")

        # Extract keywords from effect text
        all_text = f"{effect} {inherited} {security}"
        keywords = sorted(set(m.group() for m in KEYWORD_PATTERN.finditer(all_text)))

        # Build searchable text
        text_parts = [
            f"Card: {card_id} — {name}",
            f"Kind: {card_kind} | Level: {level} | Colors: {', '.join(colors)}",
            f"DP: {dp} | Play Cost: {play_cost}",
            f"Traits: {', '.join(traits)}" if traits else "",
            f"Keywords: {', '.join(keywords)}" if keywords else "",
            f"Effect: {effect}" if effect else "",
            f"Inherited: {inherited}" if inherited else "",
            f"Security: {security}" if security else "",
        ]
        card_text = "\n".join(p for p in text_parts if p)

        # Extract set_id from card_id
        set_match = re.match(r"([A-Za-z]+\d+)", card_id)
        set_id = set_match.group(1).lower() if set_match else ""

        vec_id = f"card-metadata:{card_id}:{_content_hash(card_text)}"
        records.append((vec_id, card_text, {
            "source_type": "card_metadata",
            "card_id": card_id,
            "set_id": set_id,
            "card_kind": card_kind,
            "colors": colors,
            "level": level if level is not None else 0,
            "keywords": keywords,
            "file_path": "data/cards.json",
        }))

    return records


def build_rules_docs_records() -> list[tuple[str, str, dict]]:
    """Build records for the rules-docs namespace."""
    records = []
    for doc_path in RULES_DOCS:
        if not doc_path.exists():
            logger.warning("Rules doc not found: %s", doc_path)
            continue
        text = _read_file(doc_path)
        rel = _rel_path(doc_path)
        chunks = chunk_markdown_by_section(text, rel)
        for chunk in chunks:
            content = chunk["text"]
            vec_id = f"rules-docs:{rel}:{_content_hash(content)}"
            records.append((vec_id, content, {
                "source_type": "rules",
                "section_title": chunk.get("section_title", ""),
                "file_path": rel,
            }))
    return records


# ---------------------------------------------------------------------------
# Upsert logic
# ---------------------------------------------------------------------------

def upsert_records(
    pc,
    namespace: str,
    records: list[tuple[str, str, dict]],
    dry_run: bool = False,
):
    """Upsert records to Pinecone and clean up stale vectors."""
    logger.info("Namespace '%s': %d records to upsert", namespace, len(records))

    if dry_run:
        logger.info("  [DRY RUN] Would upsert %d records", len(records))
        for vec_id, text, meta in records[:3]:
            logger.info("    Sample: id=%s, text=%d chars, meta_keys=%s",
                         vec_id, len(text), list(meta.keys()))
        return

    index = pc.Index(INDEX_NAME)

    # Upsert in batches with rate-limit retry
    new_ids = set()
    for i in range(0, len(records), BATCH_SIZE):
        batch = records[i : i + BATCH_SIZE]
        upsert_data = []
        for vec_id, text, meta in batch:
            new_ids.add(vec_id)
            upsert_data.append({
                "_id": vec_id,
                "text": text,
                **meta,
            })
        # Retry with exponential backoff on rate limits
        for attempt in range(6):
            try:
                index.upsert_records(namespace=namespace, records=upsert_data)
                break
            except Exception as e:
                if "429" in str(e) or "RESOURCE_EXHAUSTED" in str(e):
                    wait = 2 ** attempt * 5  # 5, 10, 20, 40, 80, 160s
                    logger.warning("  Rate limited, waiting %ds before retry...", wait)
                    time.sleep(wait)
                else:
                    raise
        else:
            logger.error("  Failed after 6 retries for batch %d-%d", i + 1, min(i + BATCH_SIZE, len(records)))
            raise RuntimeError("Rate limit retries exhausted")
        logger.info("  Upserted batch %d-%d / %d",
                     i + 1, min(i + BATCH_SIZE, len(records)), len(records))
        # Pace requests to stay under 250K tokens/min on free tier
        time.sleep(2)

    # Clean up stale vectors by listing and deleting IDs not in new set
    logger.info("  Checking for stale vectors in namespace '%s'...", namespace)
    try:
        # List all vector IDs in namespace via pagination
        stale_ids = []
        for id_list in index.list(namespace=namespace):
            for vid in id_list:
                if vid not in new_ids:
                    stale_ids.append(vid)
        if stale_ids:
            # Delete in batches of 1000
            for i in range(0, len(stale_ids), 1000):
                batch = stale_ids[i : i + 1000]
                index.delete(ids=batch, namespace=namespace)
            logger.info("  Deleted %d stale vectors", len(stale_ids))
        else:
            logger.info("  No stale vectors found")
    except Exception as e:
        logger.warning("  Could not clean stale vectors: %s", e)


# ---------------------------------------------------------------------------
# Namespace dispatch
# ---------------------------------------------------------------------------

NAMESPACE_BUILDERS = {
    "engine-api": lambda set_filter: build_engine_api_records(),
    "card-scripts": build_card_script_records,
    "card-metadata": lambda set_filter: build_card_metadata_records(),
    "rules-docs": lambda set_filter: build_rules_docs_records(),
}


def run_ingestion(
    namespaces: list[str],
    set_filter: str | None = None,
    dry_run: bool = False,
):
    """Run ingestion for specified namespaces."""
    pc = None if dry_run else _get_pinecone_client()

    total = 0
    for ns in namespaces:
        builder = NAMESPACE_BUILDERS.get(ns)
        if not builder:
            logger.error("Unknown namespace: %s", ns)
            continue

        logger.info("Building records for namespace '%s'...", ns)
        records = builder(set_filter)
        total += len(records)

        if pc or dry_run:
            upsert_records(pc, ns, records, dry_run=dry_run)

    logger.info("Total records across all namespaces: %d", total)


def main():
    parser = argparse.ArgumentParser(
        description="Ingest data into Pinecone for sub-agent retrieval"
    )
    parser.add_argument("--all", action="store_true", help="Rebuild all namespaces")
    parser.add_argument("--namespace", type=str, help="Single namespace to rebuild",
                        choices=list(NAMESPACE_BUILDERS.keys()))
    parser.add_argument("--set", type=str, help="Filter card-scripts to a specific set (e.g., bt10)")
    parser.add_argument("--dry-run", action="store_true", help="Preview without upserting")

    args = parser.parse_args()

    if not args.all and not args.namespace:
        parser.error("Specify --all or --namespace")

    if args.all:
        namespaces = list(NAMESPACE_BUILDERS.keys())
    else:
        namespaces = [args.namespace]

    run_ingestion(namespaces, set_filter=args.set, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
