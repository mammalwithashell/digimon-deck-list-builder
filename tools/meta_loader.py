"""Fetch meta stats and scrape decklists from external sources, build deck_library.json.

Ingests tournament decklists from DigimonMeta.com, Egman Events, DigimonCard.io,
and DigiLab PostgreSQL database.
Optionally enriches with meta stats from DigiLab.
Computes meta share and conversion rate from scraped placement data.

Usage:
    python tools/meta_loader.py --scrape-digimonmeta URL   # Scrape BT24/EX11 decks
    python tools/meta_loader.py --scrape-egman URL          # Scrape Egman tournament decks
    python tools/meta_loader.py --scrape-digimoncard-io URL # Scrape DigimonCard.io tourney
    python tools/meta_loader.py --scrape-digilab            # Scrape decklists from DigiLab DB
    python tools/meta_loader.py --import-file FILE          # Import local deck file
    python tools/meta_loader.py --fetch-meta                # DigiLab stats (optional)
    python tools/meta_loader.py --build                     # Resolve + dedup + stats + write
    python tools/meta_loader.py --report                    # Print summary
"""

from __future__ import annotations

import argparse
import hashlib
import json
import logging
import os
import re
import sys
import urllib.parse
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Set, Tuple

# Add project root to path for imports
_PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _PROJECT_ROOT not in sys.path:
    sys.path.insert(0, _PROJECT_ROOT)

from digimon_gym.engine.data.deck_loader import (
    RE_CARD_ID,
    expand_deck_dict,
    parse_deck,
)

logger = logging.getLogger(__name__)

DECK_LIBRARY_PATH = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "deck_library.json"
)

ARCHETYPE_ALIASES_PATH = os.path.join(
    os.path.dirname(__file__), "..", "digimon_gym", "engine", "data", "archetype_aliases.json"
)

# Source priority for deduplication (higher = preferred)
SOURCE_PRIORITY = {"digimonmeta": 3, "digilab": 2, "egman": 2, "digimoncard_io": 1, "manual": 0, "file": 0}


# ─── Archetype Alias Resolution ─────────────────────────────────────

def _load_alias_map() -> Dict[str, str]:
    """Load the archetype alias index, returning {lowercase_alias: canonical_name}."""
    path = os.environ.get("ARCHETYPE_ALIASES_PATH", ARCHETYPE_ALIASES_PATH)
    try:
        with open(path, "r", encoding="utf-8") as f:
            raw = json.load(f)
    except FileNotFoundError:
        logger.warning("Archetype aliases file not found: %s", path)
        return {}
    result: Dict[str, str] = {}
    for canonical, aliases in raw.items():
        if canonical.startswith("_"):
            continue
        for alias in aliases:
            result[alias.lower()] = canonical
    return result


_ALIAS_MAP: Optional[Dict[str, str]] = None


def canonicalize_archetype(name: str) -> str:
    """Resolve an archetype name to its canonical form via the alias index."""
    global _ALIAS_MAP
    if _ALIAS_MAP is None:
        _ALIAS_MAP = _load_alias_map()
    return _ALIAS_MAP.get(name.lower(), name)

# DigiLab PostgreSQL connection string
DIGILAB_CONN_STR = (
    "postgresql://neondb_owner:npg_S85ItviFhMPE@"
    "ep-tiny-morning-ajc7jhvm-pooler.c-3.us-east-2.aws.neon.tech/"
    "digilab-db?sslmode=require"
)

# Top-cut placement patterns
RE_PLACEMENT_NUM = re.compile(r"(\d+)")
TOP_CUT_PLACEMENTS = {"1st Place", "2nd Place", "3rd Place", "4th Place",
                      "1st", "2nd", "3rd", "4th"}

# DigimonMeta deck code parser: {qty}n{card_id}a
RE_DIGIMONMETA_ENTRY = re.compile(r"(\d)n([A-Z]{1,3}\d*-\d{2,3})a?")


# ─── Data Classes ────────────────────────────────────────────────────

@dataclass
class IngestedDeck:
    """A single decklist scraped from an external source."""
    deck_id: str
    source: str  # "digimonmeta", "egman", "digimoncard_io", "file", "manual"
    source_url: str = ""
    card_ids: List[str] = field(default_factory=list)
    card_counts: Dict[str, int] = field(default_factory=dict)
    archetype_name: Optional[str] = None
    format_tag: Optional[str] = None  # "BT24", "BT23", "EX11"
    placement: Optional[str] = None  # "1st Place", "5", etc.
    is_top_cut: bool = False
    event_date: Optional[str] = None
    event_players: Optional[int] = None


@dataclass
class SourceStats:
    """Per-source breakdown for an archetype."""
    count: int = 0
    top_cuts: int = 0


@dataclass
class ComputedStats:
    """Meta stats computed from scraped tournament data."""
    times_played: int = 0
    meta_share: float = 0.0
    top_cut_count: int = 0
    conversion_rate: float = 0.0
    avg_placement: float = 0.0
    sources: Dict[str, SourceStats] = field(default_factory=dict)


@dataclass
class DigiLabStats:
    """Optional enrichment stats from DigiLab MotherDuck DB."""
    times_played: int = 0
    conversion_rate: float = 0.0
    win_rate: float = 0.0
    top4_rate: float = 0.0


@dataclass
class ArchetypeMeta:
    """Full archetype entry with stats and decklists."""
    archetype_name: str
    primary_color: Optional[str] = None
    secondary_color: Optional[str] = None
    display_card_id: Optional[str] = None
    stats: ComputedStats = field(default_factory=ComputedStats)
    digilab_stats: Optional[DigiLabStats] = None
    decklists: List[IngestedDeck] = field(default_factory=list)


# ─── Helpers ─────────────────────────────────────────────────────────

def _deck_fingerprint(card_counts: Dict[str, int]) -> str:
    """Stable hash of a deck's card composition for deduplication."""
    items = sorted(card_counts.items())
    raw = "|".join(f"{k}:{v}" for k, v in items)
    return hashlib.md5(raw.encode()).hexdigest()


def _is_top_cut(placement: Optional[str], event_players: Optional[int] = None) -> bool:
    """Determine if a placement qualifies as top cut."""
    if placement is None:
        return False

    placement_str = placement.strip()

    # Check known top-cut strings
    if placement_str in TOP_CUT_PLACEMENTS:
        return True

    # Try numeric placement
    m = RE_PLACEMENT_NUM.match(placement_str)
    if m:
        place_num = int(m.group(1))
        if place_num <= 4:
            return True
        # If we know event size, top 25% is top cut
        if event_players and event_players > 0:
            return place_num <= max(4, event_players // 4)

    return False


def _card_counts_diff(a: Dict[str, int], b: Dict[str, int]) -> int:
    """Count total card differences between two decklists."""
    all_ids = set(a.keys()) | set(b.keys())
    return sum(abs(a.get(cid, 0) - b.get(cid, 0)) for cid in all_ids)


# ─── DeckIngestor ────────────────────────────────────────────────────

class DeckIngestor:
    """Fetches decklists and meta data from multiple sources, builds deck_library.json."""

    _SESSION_HEADERS = {
        "User-Agent": (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
            "AppleWebKit/537.36 (KHTML, like Gecko) "
            "Chrome/131.0.0.0 Safari/537.36"
        ),
    }

    def __init__(self) -> None:
        self.archetypes: Dict[str, ArchetypeMeta] = {}
        self.unresolved_decks: List[IngestedDeck] = []
        self._digilab_meta: Dict[str, Dict[str, Any]] = {}  # archetype_name -> raw row

    # ── DigimonMeta.com ──────────────────────────────────────────

    # Regions whose Best-of-1 rules and mulligan logic differ from Western/EN
    JP_REGIONS = {"JP", "Japan", "Korea"}

    def scrape_digimonmeta(
        self,
        url: str,
        exclude_regions: Optional[Set[str]] = None,
    ) -> int:
        """Scrape decklists from a DigimonMeta.com deck list page.

        Extracts deck codes from deckinfo2 links, along with archetype name,
        placement, and date metadata from query params.

        Args:
            url: DigimonMeta deck list page URL.
            exclude_regions: Country codes to skip (default: JP_REGIONS).
                Set to empty set() to disable filtering.

        Returns number of decklists scraped.
        """
        if exclude_regions is None:
            exclude_regions = self.JP_REGIONS
        import requests
        from bs4 import BeautifulSoup

        logger.info("Scraping DigimonMeta: %s", url)
        response = requests.get(url, headers=self._SESSION_HEADERS, timeout=30)
        response.raise_for_status()
        soup = BeautifulSoup(response.content, "html.parser")

        count = 0
        seen_urls: Set[str] = set()

        for a_tag in soup.find_all("a", href=True):
            href = a_tag["href"]
            if "deckinfo2" not in href:
                continue

            # Normalize URL
            if href.startswith("/"):
                parsed_base = urllib.parse.urlparse(url)
                href = f"{parsed_base.scheme}://{parsed_base.netloc}{href}"

            if href in seen_urls:
                continue
            seen_urls.add(href)

            parsed = urllib.parse.urlparse(href)
            params = urllib.parse.parse_qs(parsed.query)

            deck_code = params.get("dg", [""])[0]
            if not deck_code:
                continue

            # Region filter (e.g., exclude JP Best-of-1 format decks)
            if exclude_regions:
                region = params.get("cn", [""])[0]
                if region in exclude_regions:
                    logger.debug("Skipping %s region deck: %s", region, href)
                    continue

            # Parse deck code: {qty}n{card_id}a
            card_counts: Dict[str, int] = {}
            card_ids: List[str] = []
            for match in RE_DIGIMONMETA_ENTRY.finditer(deck_code):
                qty = int(match.group(1))
                card_id = match.group(2)
                card_counts[card_id] = card_counts.get(card_id, 0) + qty
                card_ids.extend([card_id] * qty)

            # Cap at 4 copies per card (DigimonMeta sometimes repeats blocks)
            for card_id in list(card_counts):
                if card_counts[card_id] > 4:
                    card_counts[card_id] = 4
            card_ids = expand_deck_dict(card_counts)

            if not card_ids:
                logger.debug("Empty deck code in %s", href)
                continue

            archetype_name = params.get("dn", [None])[0]
            placement = params.get("pl", [None])[0]
            event_date = params.get("date", [None])[0]

            deck_id = f"digimonmeta_{_deck_fingerprint(card_counts)[:12]}"

            deck = IngestedDeck(
                deck_id=deck_id,
                source="digimonmeta",
                source_url=href,
                card_ids=card_ids,
                card_counts=card_counts,
                archetype_name=archetype_name,
                placement=placement,
                is_top_cut=_is_top_cut(placement),
                event_date=event_date,
            )

            if archetype_name:
                self._add_deck_to_archetype(archetype_name, deck)
            else:
                self.unresolved_decks.append(deck)
            count += 1

        logger.info("DigimonMeta: scraped %d decklists", count)
        return count

    # ── Egman Events ─────────────────────────────────────────────

    def scrape_egman(self, url: str) -> int:
        """Scrape decklists from an Egman Events tournament decks page.

        Parses the deck table for archetype, placement, event info,
        and deck links in deckbuilder.egmanevents.com format.

        Returns number of decklists scraped.
        """
        import requests
        from bs4 import BeautifulSoup

        logger.info("Scraping Egman Events: %s", url)
        response = requests.get(url, headers=self._SESSION_HEADERS, timeout=30)
        response.raise_for_status()
        soup = BeautifulSoup(response.content, "html.parser")

        count = 0

        # Find all deck links on the page
        for a_tag in soup.find_all("a", href=True):
            href = a_tag["href"]
            if "deckbuilder.egmanevents.com" not in href or "deck=" not in href:
                continue

            # Parse the deck URL
            parsed = urllib.parse.urlparse(href)
            query = urllib.parse.parse_qs(parsed.query)
            deck_str = query.get("deck", [""])[0]

            card_counts: Dict[str, int] = {}
            for item in deck_str.split(","):
                if ":" in item:
                    card_id, count_str = item.split(":", 1)
                    try:
                        card_counts[card_id] = int(count_str)
                    except ValueError:
                        continue

            if not card_counts:
                continue

            card_ids = expand_deck_dict(card_counts)
            deck_id = f"egman_{_deck_fingerprint(card_counts)[:12]}"

            # Try to extract archetype and placement from surrounding table row
            archetype_name = None
            placement = None
            event_date = None
            event_players = None

            # Walk up to find the parent <tr>
            row = a_tag.find_parent("tr")
            if row:
                cells = row.find_all("td")
                archetype_name, placement, event_date, event_players = (
                    self._parse_egman_row(cells)
                )

            deck = IngestedDeck(
                deck_id=deck_id,
                source="egman",
                source_url=href,
                card_ids=card_ids,
                card_counts=card_counts,
                archetype_name=archetype_name,
                placement=placement,
                is_top_cut=_is_top_cut(placement, event_players),
                event_date=event_date,
                event_players=event_players,
            )

            if archetype_name:
                self._add_deck_to_archetype(archetype_name, deck)
            else:
                self.unresolved_decks.append(deck)
            count += 1

        logger.info("Egman: scraped %d decklists", count)
        return count

    @staticmethod
    def _parse_egman_row(cells) -> Tuple[Optional[str], Optional[str],
                                         Optional[str], Optional[int]]:
        """Extract metadata from an Egman table row's <td> cells.

        Known Egman column layout (8 cells):
          0: Icon (img, often empty alt)
          1: Archetype name (plain text)
          2: Player name
          3: Placement (ordinal like "1st", "2nd", etc.)
          4: Format (BT23, etc.)
          5: Event type
          6: Event name with player count in parens, e.g. "Store Name (16)"
          7: Date (M/D/YY)
        """
        archetype_name = None
        placement = None
        event_date = None
        event_players = None

        if not cells:
            return archetype_name, placement, event_date, event_players

        # Positional extraction for known 8-column layout
        if len(cells) >= 8:
            # Cell 1: Archetype name
            arch_text = cells[1].get_text(strip=True)
            if arch_text:
                archetype_name = arch_text

            # Cell 3: Placement
            place_text = cells[3].get_text(strip=True)
            if place_text:
                placement = place_text

            # Cell 6: Event name with player count
            event_text = cells[6].get_text(strip=True)
            player_match = re.search(r"\((\d+)\)", event_text)
            if player_match:
                event_players = int(player_match.group(1))

            # Cell 7: Date
            date_text = cells[7].get_text(strip=True)
            if re.match(r"\d{1,2}/\d{1,2}/\d{2,4}", date_text):
                event_date = date_text

        else:
            # Fallback: scan all cells heuristically
            for cell in cells:
                text = cell.get_text(strip=True)

                img = cell.find("img")
                if img and img.get("alt"):
                    archetype_name = img["alt"].strip()

                if re.match(r"^\d+(st|nd|rd|th)?$", text, re.IGNORECASE):
                    placement = text

                player_match = re.search(r"\((\d+)\)", text)
                if player_match:
                    event_players = int(player_match.group(1))

                if re.match(r"\d{1,2}/\d{1,2}/\d{2,4}", text):
                    event_date = text

        return archetype_name, placement, event_date, event_players

    # ── DigimonCard.io ───────────────────────────────────────────

    def scrape_digimoncard_io(self, url: str) -> int:
        """Scrape tournament decklists from DigimonCard.io.

        Fetches a tournament page and parses individual deck links.

        Returns number of decklists scraped.
        """
        import requests
        from bs4 import BeautifulSoup

        logger.info("Scraping DigimonCard.io: %s", url)
        response = requests.get(url, headers=self._SESSION_HEADERS, timeout=30)
        response.raise_for_status()
        soup = BeautifulSoup(response.content, "html.parser")

        count = 0

        # Find deck links (pattern: /deck/{name}-by-{user}-{id})
        for a_tag in soup.find_all("a", href=True):
            href = a_tag["href"]
            if "/deck/" not in href:
                continue

            # Normalize to full URL
            if href.startswith("/"):
                href = f"https://digimoncard.io{href}"

            deck_data = self._fetch_digimoncard_io_deck(href)
            if deck_data is None:
                continue

            deck_data.source = "digimoncard_io"
            if deck_data.archetype_name:
                self._add_deck_to_archetype(deck_data.archetype_name, deck_data)
            else:
                self.unresolved_decks.append(deck_data)
            count += 1

        logger.info("DigimonCard.io: scraped %d decklists", count)
        return count

    def _fetch_digimoncard_io_deck(self, deck_url: str) -> Optional[IngestedDeck]:
        """Fetch and parse a single deck page from DigimonCard.io."""
        import requests
        from bs4 import BeautifulSoup

        try:
            response = requests.get(deck_url, headers=self._SESSION_HEADERS, timeout=15)
            response.raise_for_status()
        except Exception as e:
            logger.debug("Failed to fetch %s: %s", deck_url, e)
            return None

        soup = BeautifulSoup(response.content, "html.parser")

        # Extract card IDs from the page — look for card ID patterns in text
        card_counts: Dict[str, int] = {}
        card_ids: List[str] = []

        # DigimonCard.io pages have card entries with IDs
        for elem in soup.find_all(string=re.compile(r"[A-Z]{1,3}\d*-\d{2,3}")):
            text = elem.strip()
            # Try text format parsing: "4 CardName BT24-017"
            tokens = text.split()
            if len(tokens) >= 2:
                try:
                    qty = int(tokens[0])
                    card_id = tokens[-1]
                    if RE_CARD_ID.match(card_id):
                        card_counts[card_id] = card_counts.get(card_id, 0) + qty
                        card_ids.extend([card_id] * qty)
                except ValueError:
                    pass

        if not card_ids:
            return None

        # Try to extract deck name from title
        title = soup.find("title")
        archetype_name = None
        if title:
            title_text = title.get_text(strip=True)
            # Pattern: "DeckName by Username - DigimonCard.io"
            if " by " in title_text:
                archetype_name = title_text.split(" by ")[0].strip()

        deck_id = f"digimoncard_io_{_deck_fingerprint(card_counts)[:12]}"

        return IngestedDeck(
            deck_id=deck_id,
            source="digimoncard_io",
            source_url=deck_url,
            card_ids=card_ids,
            card_counts=card_counts,
            archetype_name=archetype_name,
        )

    # ── DigiLab PostgreSQL ───────────────────────────────────────

    @staticmethod
    def _digilab_connect():
        """Connect to DigiLab PostgreSQL database.

        Uses DIGILAB_CONN_STR env var if set, otherwise falls back to default.
        """
        import psycopg2
        conn_str = os.environ.get("DIGILAB_CONN_STR", DIGILAB_CONN_STR)
        return psycopg2.connect(conn_str)

    @staticmethod
    def _digilab_json_to_card_ids(decklist_json: Dict[str, Any]) -> Tuple[List[str], Dict[str, int]]:
        """Convert DigiLab structured decklist JSON to flat card ID list.

        DigiLab format:
            {"digimon": [{"count": 4, "name": "...", "set": "BT12", "number": "073"}, ...],
             "tamer": [...], "option": [...], "egg": [...]}

        Returns (card_ids, card_counts).
        """
        card_ids: List[str] = []
        card_counts: Dict[str, int] = {}
        for category in ("egg", "digimon", "tamer", "option"):
            for entry in decklist_json.get(category, []):
                set_code = entry.get("set", "")
                number = entry.get("number", "")
                count = entry.get("count", 0)
                if not set_code or not number or not count:
                    continue
                card_id = f"{set_code}-{number}"
                card_ids.extend([card_id] * count)
                card_counts[card_id] = card_counts.get(card_id, 0) + count
        return card_ids, card_counts

    def scrape_digilab(self, min_placement: Optional[int] = None) -> int:
        """Scrape decklists from DigiLab PostgreSQL database.

        Fetches tournament results that have decklist_json, joined with
        deck_archetypes for archetype names and tournaments for event metadata.

        Args:
            min_placement: Only include results with placement <= this value.
                If None, includes all results with decklists.

        Returns number of decklists scraped.
        """
        try:
            import psycopg2
        except ImportError:
            logger.warning("psycopg2 not installed — pip install psycopg2-binary")
            return 0

        logger.info("Scraping decklists from DigiLab PostgreSQL...")
        conn = self._digilab_connect()
        try:
            cur = conn.cursor()
            query = """
                SELECT r.result_id, r.decklist_json, r.placement,
                       r.wins, r.losses, r.decklist_url,
                       da.archetype_name, da.primary_color, da.secondary_color,
                       da.display_card_id,
                       t.event_date, t.player_count, t.event_type
                FROM results r
                JOIN deck_archetypes da ON da.archetype_id = r.archetype_id
                JOIN tournaments t ON t.tournament_id = r.tournament_id
                WHERE r.decklist_json IS NOT NULL
                  AND r.decklist_json != ''
            """
            params: list = []
            if min_placement is not None:
                query += " AND r.placement <= %s"
                params.append(min_placement)
            query += " ORDER BY t.event_date DESC"

            cur.execute(query, params)
            rows = cur.fetchall()
        finally:
            conn.close()

        count = 0
        for row in rows:
            result_id, decklist_str, placement, wins, losses, decklist_url, \
                raw_archetype_name, primary_color, secondary_color, display_card_id, \
                event_date, player_count, event_type = row

            archetype_name = canonicalize_archetype(raw_archetype_name)

            # Parse the decklist JSON
            try:
                decklist_json = json.loads(decklist_str)
            except (json.JSONDecodeError, TypeError):
                logger.debug("Invalid decklist JSON for result %s", result_id)
                continue

            card_ids, card_counts = self._digilab_json_to_card_ids(decklist_json)
            if not card_ids:
                continue

            deck_id = f"digilab_{_deck_fingerprint(card_counts)[:12]}"
            placement_str = str(placement) if placement is not None else None
            event_date_str = event_date.strftime("%Y-%m-%d") if event_date else None

            deck = IngestedDeck(
                deck_id=deck_id,
                source="digilab",
                source_url=decklist_url or "",
                card_ids=card_ids,
                card_counts=card_counts,
                archetype_name=archetype_name,
                placement=placement_str,
                is_top_cut=_is_top_cut(placement_str, player_count),
                event_date=event_date_str,
                event_players=player_count,
            )

            # Ensure archetype exists with metadata from DB
            if archetype_name not in self.archetypes:
                self.archetypes[archetype_name] = ArchetypeMeta(
                    archetype_name=archetype_name,
                    primary_color=primary_color,
                    secondary_color=secondary_color,
                    display_card_id=display_card_id,
                )
            else:
                arch = self.archetypes[archetype_name]
                if arch.display_card_id is None and display_card_id:
                    arch.display_card_id = display_card_id
                if arch.primary_color is None and primary_color:
                    arch.primary_color = primary_color
                if arch.secondary_color is None and secondary_color:
                    arch.secondary_color = secondary_color

            self._add_deck_to_archetype(archetype_name, deck)
            count += 1

        logger.info("DigiLab: scraped %d decklists", count)
        return count

    def fetch_digilab_meta(self) -> int:
        """Fetch archetype stats from DigiLab PostgreSQL database.

        Computes stats from deck_archetypes, results, and matches tables.
        These stats supplement (not replace) the computed stats from scraped data.

        Returns number of archetypes loaded.
        """
        try:
            import psycopg2
        except ImportError:
            logger.warning("psycopg2 not installed — pip install psycopg2-binary")
            return 0

        logger.info("Fetching meta stats from DigiLab PostgreSQL...")
        conn = self._digilab_connect()
        try:
            cur = conn.cursor()

            # Get archetype metadata and computed stats from results
            cur.execute("""
                SELECT da.archetype_name, da.primary_color, da.secondary_color,
                       da.display_card_id,
                       COUNT(r.result_id) AS times_played,
                       SUM(r.wins) AS total_wins,
                       SUM(r.losses) AS total_losses,
                       COUNT(CASE WHEN r.placement <= 4 THEN 1 END) AS top4_count
                FROM deck_archetypes da
                LEFT JOIN results r ON r.archetype_id = da.archetype_id
                WHERE da.is_active = true
                GROUP BY da.archetype_id, da.archetype_name, da.primary_color,
                         da.secondary_color, da.display_card_id
                HAVING COUNT(r.result_id) > 0
                ORDER BY COUNT(r.result_id) DESC
            """)
            rows = cur.fetchall()
        finally:
            conn.close()

        # Pre-aggregate rows that alias to the same canonical name
        aggregated: Dict[str, list] = defaultdict(list)
        for row in rows:
            raw_name = row[0]
            canonical = canonicalize_archetype(raw_name)
            aggregated[canonical].append(row)

        count = 0
        for name, agg_rows in aggregated.items():
            # Merge stats across aliased rows
            primary_color = secondary_color = display_card_id = None
            times_played = total_wins = total_losses = top4_count = 0
            for row in agg_rows:
                _, pc, sc, dci, tp, tw, tl, t4 = row
                times_played += tp or 0
                total_wins += tw or 0
                total_losses += tl or 0
                top4_count += t4 or 0
                if primary_color is None and pc:
                    primary_color = pc
                if secondary_color is None and sc:
                    secondary_color = sc
                if display_card_id is None and dci:
                    display_card_id = dci

            total_games = total_wins + total_losses
            win_rate = total_wins / total_games if total_games > 0 else 0.0
            top4_rate = top4_count / times_played if times_played > 0 else 0.0
            conversion_rate = top4_rate  # top4 = conversion in this context

            self._digilab_meta[name] = {
                "primary_color": primary_color,
                "secondary_color": secondary_color,
                "display_card_id": display_card_id,
                "times_played": times_played,
                "conversion_rate": conversion_rate,
                "win_rate": win_rate,
                "top4_rate": top4_rate,
                "total_match_wins": total_wins or 0,
                "total_match_losses": total_losses or 0,
            }

            # Ensure archetype exists
            if name not in self.archetypes:
                self.archetypes[name] = ArchetypeMeta(
                    archetype_name=name,
                    primary_color=primary_color,
                    secondary_color=secondary_color,
                    display_card_id=display_card_id,
                )

            arch = self.archetypes[name]
            if arch.display_card_id is None and display_card_id:
                arch.display_card_id = display_card_id
            if arch.primary_color is None and primary_color:
                arch.primary_color = primary_color
            if arch.secondary_color is None and secondary_color:
                arch.secondary_color = secondary_color

            arch.digilab_stats = DigiLabStats(
                times_played=times_played or 0,
                conversion_rate=conversion_rate,
                win_rate=win_rate,
                top4_rate=top4_rate,
            )
            count += 1

        logger.info("DigiLab: loaded stats for %d archetypes", count)
        return count

    # ── File Import ──────────────────────────────────────────────

    def import_file(self, filepath: str, archetype: Optional[str] = None) -> int:
        """Import decklists from a local file (TTS/text format).

        Uses deck_loader.parse_deck() for format auto-detection.

        Returns number of decklists imported.
        """
        logger.info("Importing file: %s", filepath)
        with open(filepath, "r", encoding="utf-8") as f:
            raw = f.read()

        card_ids = parse_deck(raw)
        card_counts = dict(Counter(card_ids))

        deck_id = f"file_{_deck_fingerprint(card_counts)[:12]}"
        deck = IngestedDeck(
            deck_id=deck_id,
            source="file",
            source_url=filepath,
            card_ids=card_ids,
            card_counts=card_counts,
            archetype_name=archetype,
        )

        if archetype:
            self._add_deck_to_archetype(archetype, deck)
        else:
            self.unresolved_decks.append(deck)

        logger.info("Imported 1 decklist from %s", filepath)
        return 1

    # ── Archetype Management ─────────────────────────────────────

    def _add_deck_to_archetype(self, archetype_name: str, deck: IngestedDeck) -> None:
        """Add a deck to an archetype, creating the archetype if needed."""
        archetype_name = canonicalize_archetype(archetype_name)
        deck.archetype_name = archetype_name
        if archetype_name not in self.archetypes:
            self.archetypes[archetype_name] = ArchetypeMeta(
                archetype_name=archetype_name,
            )
        self.archetypes[archetype_name].decklists.append(deck)

    def resolve_archetypes(self) -> None:
        """Match unresolved decks to archetypes using display_card_id heuristic.

        For each unresolved deck:
        1. Check if deck contains any archetype's display_card_id
        2. If match found, assign to archetype with highest copy count
        3. If no match, assign to "Unclassified" archetype
        """
        if not self.unresolved_decks:
            return

        # Build display_card_id -> archetype mapping
        display_map: Dict[str, str] = {}
        for name, arch in self.archetypes.items():
            if arch.display_card_id:
                display_map[arch.display_card_id] = name

        still_unresolved: List[IngestedDeck] = []

        for deck in self.unresolved_decks:
            best_archetype = None
            best_count = 0

            for card_id, count in deck.card_counts.items():
                if card_id in display_map and count > best_count:
                    best_archetype = display_map[card_id]
                    best_count = count

            if best_archetype:
                self._add_deck_to_archetype(best_archetype, deck)
                deck.archetype_name = best_archetype
            else:
                still_unresolved.append(deck)

        # Put remaining into "Unclassified"
        for deck in still_unresolved:
            deck.archetype_name = "Unclassified"
            self._add_deck_to_archetype("Unclassified", deck)

        resolved = len(self.unresolved_decks) - len(still_unresolved)
        logger.info(
            "Resolved %d/%d decks to archetypes, %d unclassified",
            resolved, len(self.unresolved_decks), len(still_unresolved),
        )
        self.unresolved_decks = []

    # ── Deduplication ────────────────────────────────────────────

    def deduplicate(self) -> int:
        """Remove cross-source duplicate decklists.

        Deduplication strategy:
        1. Exact fingerprint match: identical card compositions → keep preferred source
        2. Near-duplicate: same archetype + ≤3 card diff → keep richer metadata

        Returns number of duplicates removed.
        """
        removed = 0

        for arch in self.archetypes.values():
            if len(arch.decklists) <= 1:
                continue

            # Group by fingerprint
            fp_groups: Dict[str, List[IngestedDeck]] = defaultdict(list)
            for deck in arch.decklists:
                fp = _deck_fingerprint(deck.card_counts)
                fp_groups[fp].append(deck)

            deduped: List[IngestedDeck] = []
            used_fps: Set[str] = set()

            for fp, group in fp_groups.items():
                if fp in used_fps:
                    continue
                used_fps.add(fp)

                if len(group) == 1:
                    deduped.append(group[0])
                else:
                    # Keep the one from the highest-priority source
                    group.sort(
                        key=lambda d: SOURCE_PRIORITY.get(d.source, 0),
                        reverse=True,
                    )
                    deduped.append(group[0])
                    removed += len(group) - 1

            # Near-duplicate pass: same archetype, ≤3 card diff
            final: List[IngestedDeck] = []
            skip_indices: Set[int] = set()

            for i, deck_a in enumerate(deduped):
                if i in skip_indices:
                    continue
                for j in range(i + 1, len(deduped)):
                    if j in skip_indices:
                        continue
                    diff = _card_counts_diff(deck_a.card_counts, deduped[j].card_counts)
                    if diff <= 3:
                        # Same event date check (if available)
                        if deck_a.event_date and deduped[j].event_date:
                            if deck_a.event_date == deduped[j].event_date:
                                # Merge: keep higher priority
                                prio_a = SOURCE_PRIORITY.get(deck_a.source, 0)
                                prio_j = SOURCE_PRIORITY.get(deduped[j].source, 0)
                                if prio_j > prio_a:
                                    skip_indices.add(i)
                                    break
                                else:
                                    skip_indices.add(j)
                                    removed += 1

                if i not in skip_indices:
                    final.append(deck_a)

            arch.decklists = final

        logger.info("Deduplication: removed %d duplicates", removed)
        return removed

    # ── Stats Computation ────────────────────────────────────────

    def compute_meta_stats(self) -> None:
        """Derive meta_share and conversion_rate from scraped placement data."""
        total_entries = sum(len(a.decklists) for a in self.archetypes.values())
        if total_entries == 0:
            return

        for arch in self.archetypes.values():
            n = len(arch.decklists)
            top_cuts = sum(1 for d in arch.decklists if d.is_top_cut)

            # Per-source breakdown
            source_stats: Dict[str, SourceStats] = defaultdict(SourceStats)
            placements: List[int] = []

            for deck in arch.decklists:
                ss = source_stats[deck.source]
                ss.count += 1
                if deck.is_top_cut:
                    ss.top_cuts += 1

                # Collect numeric placements for average
                if deck.placement:
                    m = RE_PLACEMENT_NUM.match(deck.placement.strip())
                    if m:
                        placements.append(int(m.group(1)))

            arch.stats = ComputedStats(
                times_played=n,
                meta_share=n / total_entries,
                top_cut_count=top_cuts,
                conversion_rate=top_cuts / n if n > 0 else 0.0,
                avg_placement=sum(placements) / len(placements) if placements else 0.0,
                sources=dict(source_stats),
            )

        logger.info(
            "Computed meta stats: %d archetypes, %d total entries",
            len(self.archetypes), total_entries,
        )

    # ── Persistence ──────────────────────────────────────────────

    def save(self, path: str = DECK_LIBRARY_PATH) -> None:
        """Write the deck library to JSON."""
        data = {
            "version": 1,
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "total_entries": sum(len(a.decklists) for a in self.archetypes.values()),
            "archetypes": {},
        }

        for name, arch in sorted(self.archetypes.items()):
            arch_data: Dict[str, Any] = {
                "archetype_name": arch.archetype_name,
                "primary_color": arch.primary_color,
                "secondary_color": arch.secondary_color,
                "display_card_id": arch.display_card_id,
                "stats": {
                    "times_played": arch.stats.times_played,
                    "meta_share": round(arch.stats.meta_share, 6),
                    "top_cut_count": arch.stats.top_cut_count,
                    "conversion_rate": round(arch.stats.conversion_rate, 4),
                    "avg_placement": round(arch.stats.avg_placement, 2),
                    "sources": {
                        src: {"count": ss.count, "top_cuts": ss.top_cuts}
                        for src, ss in arch.stats.sources.items()
                    },
                },
                "decklists": [],
            }

            if arch.digilab_stats:
                arch_data["digilab_stats"] = {
                    "times_played": arch.digilab_stats.times_played,
                    "conversion_rate": round(arch.digilab_stats.conversion_rate, 4),
                    "win_rate": round(arch.digilab_stats.win_rate, 4),
                    "top4_rate": round(arch.digilab_stats.top4_rate, 4),
                }

            for deck in arch.decklists:
                # Store decklist as TTS format (JSON array string) for easy
                # copy-paste from digimoncard.io's "Export Deck → TTS" feature
                tts_decklist = json.dumps(deck.card_ids)
                arch_data["decklists"].append({
                    "deck_id": deck.deck_id,
                    "source": deck.source,
                    "source_url": deck.source_url,
                    "decklist": tts_decklist,
                    "format": deck.format_tag,
                    "placement": deck.placement,
                    "is_top_cut": deck.is_top_cut,
                    "event_date": deck.event_date,
                })

            data["archetypes"][name] = arch_data

        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        logger.info("Saved deck library to %s (%d archetypes)", path, len(data["archetypes"]))

    def load(self, path: str = DECK_LIBRARY_PATH) -> None:
        """Load an existing deck library JSON for incremental merge."""
        if not os.path.exists(path):
            logger.info("No existing deck library at %s", path)
            return

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        for raw_name, arch_data in data.get("archetypes", {}).items():
            name = canonicalize_archetype(raw_name)
            if name not in self.archetypes:
                self.archetypes[name] = ArchetypeMeta(
                    archetype_name=name,
                    primary_color=arch_data.get("primary_color"),
                    secondary_color=arch_data.get("secondary_color"),
                    display_card_id=arch_data.get("display_card_id"),
                )

            arch = self.archetypes[name]
            # Backfill metadata from aliases
            if arch.display_card_id is None and arch_data.get("display_card_id"):
                arch.display_card_id = arch_data["display_card_id"]
            if arch.primary_color is None and arch_data.get("primary_color"):
                arch.primary_color = arch_data["primary_color"]
            if arch.secondary_color is None and arch_data.get("secondary_color"):
                arch.secondary_color = arch_data["secondary_color"]

            # Load existing decklists
            existing_ids = {d.deck_id for d in arch.decklists}
            for dl in arch_data.get("decklists", []):
                if dl.get("deck_id") in existing_ids:
                    continue
                # Parse TTS decklist format (JSON array string) back to card_ids
                tts_str = dl.get("decklist", "")
                if tts_str:
                    card_ids = json.loads(tts_str)
                    # Filter to valid card IDs (mirrors parse_tts behaviour)
                    card_ids = [cid for cid in card_ids if isinstance(cid, str)]
                else:
                    # Backward compat: fall back to legacy card_ids field
                    card_ids = dl.get("card_ids", [])
                card_counts = {}
                for cid in card_ids:
                    card_counts[cid] = card_counts.get(cid, 0) + 1
                deck = IngestedDeck(
                    deck_id=dl.get("deck_id", ""),
                    source=dl.get("source", ""),
                    source_url=dl.get("source_url", ""),
                    card_ids=card_ids,
                    card_counts=card_counts,
                    archetype_name=name,
                    format_tag=dl.get("format"),
                    placement=dl.get("placement"),
                    is_top_cut=dl.get("is_top_cut", False),
                    event_date=dl.get("event_date"),
                )
                arch.decklists.append(deck)

            # Load digilab stats (merge if alias brings in additional stats)
            digilab = arch_data.get("digilab_stats")
            if digilab:
                new_tp = digilab.get("times_played", 0)
                if arch.digilab_stats is None:
                    arch.digilab_stats = DigiLabStats(
                        times_played=new_tp,
                        conversion_rate=digilab.get("conversion_rate", 0.0),
                        win_rate=digilab.get("win_rate", 0.0),
                        top4_rate=digilab.get("top4_rate", 0.0),
                    )
                elif new_tp > 0:
                    # Merge: combine times_played, weighted-average rates
                    old = arch.digilab_stats
                    combined_tp = old.times_played + new_tp
                    if combined_tp > 0:
                        old.conversion_rate = (
                            old.conversion_rate * old.times_played
                            + digilab.get("conversion_rate", 0.0) * new_tp
                        ) / combined_tp
                        old.win_rate = (
                            old.win_rate * old.times_played
                            + digilab.get("win_rate", 0.0) * new_tp
                        ) / combined_tp
                        old.top4_rate = (
                            old.top4_rate * old.times_played
                            + digilab.get("top4_rate", 0.0) * new_tp
                        ) / combined_tp
                    old.times_played = combined_tp

        total = sum(len(a.decklists) for a in self.archetypes.values())
        logger.info("Loaded deck library: %d archetypes, %d decklists", len(self.archetypes), total)

    # ── Reporting ────────────────────────────────────────────────

    def report(self) -> str:
        """Generate a summary report of the deck library."""
        lines = ["=" * 60, "Deck Library Report", "=" * 60]

        total_decks = sum(len(a.decklists) for a in self.archetypes.values())
        lines.append(f"Archetypes: {len(self.archetypes)}")
        lines.append(f"Total decklists: {total_decks}")
        lines.append(f"Unresolved decks: {len(self.unresolved_decks)}")
        lines.append("")

        # Sort by meta_share descending
        sorted_archs = sorted(
            self.archetypes.values(),
            key=lambda a: a.stats.meta_share,
            reverse=True,
        )

        lines.append(
            f"{'Archetype':<25s} {'Decks':>5s} {'Meta%':>7s} "
            f"{'Conv%':>7s} {'TopCut':>7s} {'Sources'}"
        )
        lines.append("-" * 80)

        for arch in sorted_archs:
            if not arch.decklists:
                continue
            sources = ", ".join(
                f"{src}:{ss.count}"
                for src, ss in arch.stats.sources.items()
            )
            lines.append(
                f"{arch.archetype_name:<25s} "
                f"{arch.stats.times_played:>5d} "
                f"{arch.stats.meta_share * 100:>6.1f}% "
                f"{arch.stats.conversion_rate * 100:>6.1f}% "
                f"{arch.stats.top_cut_count:>7d} "
                f"{sources}"
            )

        # Orphaned archetypes (have DigiLab stats but no decks)
        orphaned = [
            name for name, arch in self.archetypes.items()
            if not arch.decklists and arch.digilab_stats
        ]
        if orphaned:
            lines.append("")
            lines.append(f"Orphaned archetypes (DigiLab stats, no decks): {len(orphaned)}")
            for name in orphaned[:10]:
                lines.append(f"  - {name}")
            if len(orphaned) > 10:
                lines.append(f"  ... and {len(orphaned) - 10} more")

        return "\n".join(lines)


# ─── Local Meta Report ────────────────────────────────────────────────

def _print_local_meta_report(args):
    """Print scoped meta report comparing local vs global meta shares."""
    # Lazy import to avoid requiring psycopg2 for other commands
    from digimon_gym.digilab_client import (
        get_scoped_meta,
        list_stores as digilab_list_stores,
        list_scenes as digilab_list_scenes,
    )

    # Resolve store names to IDs, or scene name to ID
    store_ids = None
    scene_id = None
    scope_label = ""

    if args.stores:
        store_names = [n.strip() for n in args.stores.split(",")]
        all_stores = digilab_list_stores(min_tournaments=0)
        store_map = {s.name.lower(): s for s in all_stores}
        store_ids = []
        for name in store_names:
            match = store_map.get(name.lower())
            if match:
                store_ids.append(match.store_id)
            else:
                logger.warning("Store not found: %s", name)
        if not store_ids:
            print("ERROR: No matching stores found.")
            return
        scope_label = ", ".join(store_names)
    elif args.scene:
        all_scenes = digilab_list_scenes(min_tournaments=0)
        scene_match = None
        for s in all_scenes:
            if s.display_name.lower() == args.scene.lower() or s.name.lower() == args.scene.lower():
                scene_match = s
                break
        if not scene_match:
            print(f"ERROR: Scene not found: {args.scene}")
            print("Available scenes:")
            for s in all_scenes:
                print(f"  {s.display_name} (ID: {s.scene_id}, {s.tournament_count} events)")
            return
        scene_id = scene_match.scene_id
        scope_label = scene_match.display_name
    else:
        print("ERROR: --local-meta requires --stores or --scene")
        return

    # Get scoped meta from DigiLab
    scoped = get_scoped_meta(store_ids=store_ids, scene_id=scene_id)

    # Load deck library for global comparison and decklist availability
    # Canonicalize names so aliases merge correctly
    library_path = args.output if hasattr(args, "output") else DECK_LIBRARY_PATH
    with open(library_path, "r", encoding="utf-8") as f:
        library = json.load(f)

    # Aggregate library entries under canonical names
    global_archetypes: Dict[str, dict] = {}
    for raw_name, arch_data in library.get("archetypes", {}).items():
        canon = canonicalize_archetype(raw_name)
        if canon not in global_archetypes:
            global_archetypes[canon] = {"digilab_tp": 0, "decklists": 0}
        entry = global_archetypes[canon]
        entry["digilab_tp"] += arch_data.get("digilab_stats", {}).get("times_played", 0)
        entry["decklists"] += len(arch_data.get("decklists", []))

    # Compute global total for meta_share
    global_total = sum(e["digilab_tp"] for e in global_archetypes.values())

    # Merge: all archetypes from both sources
    all_names = set(scoped.keys()) | set(global_archetypes.keys())

    rows = []
    for name in sorted(all_names):
        local = scoped.get(name)
        gdata = global_archetypes.get(name, {"digilab_tp": 0, "decklists": 0})
        global_tp = gdata["digilab_tp"]
        global_ms = global_tp / global_total if global_total > 0 else 0.0
        has_decklists = gdata["decklists"] > 0

        local_ms = local.meta_share if local else 0.0
        local_tp = local.times_played if local else 0

        if local_tp == 0 and not has_decklists:
            continue  # skip if no local data and no decklists

        rows.append((name, local_ms, global_ms, local_tp, has_decklists))

    # Sort by local meta share descending
    rows.sort(key=lambda r: r[1], reverse=True)

    # Print report
    print(f"\n{'=' * 80}")
    print(f"  Local Meta Report: {scope_label}")
    print(f"  {len(scoped)} archetypes found locally")
    print(f"{'=' * 80}\n")
    print(f"  {'Archetype':<30} {'Local%':>7} {'Global%':>8} {'Plays':>6} {'Decks':>6}")
    print(f"  {'-' * 30} {'-' * 7} {'-' * 8} {'-' * 6} {'-' * 6}")

    for name, local_ms, global_ms, local_tp, has_decks in rows:
        deck_flag = "Y" if has_decks else "N"
        print(f"  {name:<30} {local_ms * 100:>6.1f}% {global_ms * 100:>7.1f}% {local_tp:>6} {deck_flag:>6}")

    # Summary
    trainable = sum(1 for _, lms, _, _, hd in rows if lms > 0 and hd)
    local_only = sum(1 for _, lms, _, _, hd in rows if lms > 0 and not hd)
    print(f"\n  Trainable (local meta + decklists): {trainable}")
    print(f"  Local-only (no decklists): {local_only}")
    print()


# ─── CLI ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Fetch meta stats and scrape decklists into deck_library.json"
    )
    parser.add_argument(
        "--scrape-digimonmeta", metavar="URL",
        help="Scrape decklists from a DigimonMeta.com page",
    )
    parser.add_argument(
        "--scrape-egman", metavar="URL",
        help="Scrape decklists from an Egman Events tournament decks page",
    )
    parser.add_argument(
        "--scrape-digimoncard-io", metavar="URL",
        help="Scrape tournament decklists from DigimonCard.io",
    )
    parser.add_argument(
        "--scrape-digilab", action="store_true",
        help="Scrape decklists from DigiLab PostgreSQL database",
    )
    parser.add_argument(
        "--digilab-min-placement", metavar="N", type=int, default=None,
        help="Only import DigiLab decklists with placement <= N",
    )
    parser.add_argument(
        "--import-file", metavar="FILE",
        help="Import a local deck file (TTS/text format)",
    )
    parser.add_argument(
        "--import-archetype", metavar="NAME",
        help="Archetype name for --import-file",
    )
    parser.add_argument(
        "--fetch-meta", action="store_true",
        help="Fetch meta stats from DigiLab PostgreSQL database",
    )
    parser.add_argument(
        "--build", action="store_true",
        help="Resolve archetypes, deduplicate, compute stats, write deck_library.json",
    )
    parser.add_argument(
        "--report", action="store_true",
        help="Print summary report",
    )
    parser.add_argument(
        "--no-jp-filter", action="store_true",
        help="Include JP/Korea format decks from DigimonMeta (excluded by default)",
    )
    parser.add_argument(
        "--output", metavar="PATH", default=DECK_LIBRARY_PATH,
        help=f"Output path for deck_library.json (default: {DECK_LIBRARY_PATH})",
    )
    parser.add_argument(
        "--local-meta", action="store_true",
        help="Print local meta report for specified stores or scene",
    )
    parser.add_argument(
        "--stores", metavar="NAMES",
        help="Comma-separated store names for --local-meta (e.g. 'Card Haven,Boardwalk Games')",
    )
    parser.add_argument(
        "--scene", metavar="NAME",
        help="Scene display name for --local-meta (e.g. 'Texas (Dallas-Fort Worth)')",
    )
    parser.add_argument(
        "-v", "--verbose", action="store_true",
        help="Enable verbose logging",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(levelname)s: %(message)s",
    )

    ingestor = DeckIngestor()

    # Load existing library for merge
    ingestor.load(args.output)

    actions_taken = False

    if args.scrape_digimonmeta:
        exclude = set() if args.no_jp_filter else None  # None = default JP_REGIONS
        ingestor.scrape_digimonmeta(args.scrape_digimonmeta, exclude_regions=exclude)
        actions_taken = True

    if args.scrape_egman:
        ingestor.scrape_egman(args.scrape_egman)
        actions_taken = True

    if args.scrape_digimoncard_io:
        ingestor.scrape_digimoncard_io(args.scrape_digimoncard_io)
        actions_taken = True

    if args.scrape_digilab:
        ingestor.scrape_digilab(min_placement=args.digilab_min_placement)
        actions_taken = True

    if args.import_file:
        ingestor.import_file(args.import_file, archetype=args.import_archetype)
        actions_taken = True

    if args.fetch_meta:
        ingestor.fetch_digilab_meta()
        actions_taken = True

    if args.build or actions_taken:
        ingestor.resolve_archetypes()
        removed = ingestor.deduplicate()
        ingestor.compute_meta_stats()
        ingestor.save(args.output)
        print(f"Saved deck library to {args.output}")
        if removed:
            print(f"  Removed {removed} duplicate decklists")

    if args.report or actions_taken:
        print(ingestor.report())

    if args.local_meta:
        _print_local_meta_report(args)
        actions_taken = True

    if not actions_taken and not args.report:
        parser.print_help()


if __name__ == "__main__":
    main()
