"""Fetch meta stats and scrape decklists from external sources, build deck_library.json.

Ingests tournament decklists from DigimonMeta.com, Egman Events, and DigimonCard.io.
Optionally enriches with meta stats from DigiLab MotherDuck database.
Computes meta share and conversion rate from scraped placement data.

Usage:
    python tools/meta_loader.py --scrape-digimonmeta URL   # Scrape BT24/EX11 decks
    python tools/meta_loader.py --scrape-egman URL          # Scrape Egman tournament decks
    python tools/meta_loader.py --scrape-digimoncard-io URL # Scrape DigimonCard.io tourney
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

# Source priority for deduplication (higher = preferred)
SOURCE_PRIORITY = {"digimonmeta": 3, "egman": 2, "digimoncard_io": 1, "manual": 0, "file": 0}

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

    # ── DigiLab MotherDuck (enrichment) ──────────────────────────

    def fetch_digilab_meta(self) -> int:
        """Fetch archetype stats from DigiLab MotherDuck database.

        Reads MOTHERDUCK_TOKEN from .env file.
        These stats supplement (not replace) the computed stats from scraped data.

        Returns number of archetypes loaded.
        """
        token = self._get_motherduck_token()
        if not token:
            logger.warning("No MOTHERDUCK_TOKEN found in .env — skipping DigiLab fetch")
            return 0

        try:
            import duckdb
        except ImportError:
            logger.warning("duckdb not installed — skipping DigiLab fetch (pip install duckdb)")
            return 0

        logger.info("Fetching meta stats from DigiLab MotherDuck...")
        conn_str = "md:_share/digilab-digimontcg/68ea21a1-6e57-4c50-9102-ae3d583e16c0"
        con = duckdb.connect(conn_str, config={"motherduck_token": token})
        con.sql('USE "digilab-digimontcg"')

        rows = con.sql("""
            SELECT archetype_name, primary_color, secondary_color,
                   display_card_id, times_played, conversion_rate,
                   win_rate, top4_rate, total_match_wins, total_match_losses
            FROM archetype_meta
            WHERE times_played > 0
            ORDER BY times_played DESC
        """).fetchall()
        con.close()

        count = 0
        for row in rows:
            name = row[0]
            self._digilab_meta[name] = {
                "primary_color": row[1],
                "secondary_color": row[2],
                "display_card_id": row[3],
                "times_played": row[4],
                "conversion_rate": (row[5] or 0.0) / 100.0,  # DB stores as percent
                "win_rate": (row[6] or 0.0) / 100.0,
                "top4_rate": (row[7] or 0.0) / 100.0,
                "total_match_wins": row[8] or 0,
                "total_match_losses": row[9] or 0,
            }

            # Ensure archetype exists
            if name not in self.archetypes:
                self.archetypes[name] = ArchetypeMeta(
                    archetype_name=name,
                    primary_color=row[1],
                    secondary_color=row[2],
                    display_card_id=row[3],
                )

            # Set digilab_stats as enrichment
            arch = self.archetypes[name]
            if arch.display_card_id is None and row[3]:
                arch.display_card_id = row[3]
            if arch.primary_color is None and row[1]:
                arch.primary_color = row[1]

            arch.digilab_stats = DigiLabStats(
                times_played=row[4] or 0,
                conversion_rate=(row[5] or 0.0) / 100.0,
                win_rate=(row[6] or 0.0) / 100.0,
                top4_rate=(row[7] or 0.0) / 100.0,
            )
            count += 1

        logger.info("DigiLab: loaded stats for %d archetypes", count)
        return count

    @staticmethod
    def _get_motherduck_token() -> Optional[str]:
        """Read MOTHERDUCK_TOKEN from .env file or environment."""
        # Try python-dotenv
        try:
            from dotenv import load_dotenv
            env_path = os.path.join(_PROJECT_ROOT, ".env")
            if os.path.exists(env_path):
                load_dotenv(env_path)
        except ImportError:
            pass

        return os.environ.get("MOTHERDUCK_TOKEN")

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

        for name, arch_data in data.get("archetypes", {}).items():
            if name not in self.archetypes:
                self.archetypes[name] = ArchetypeMeta(
                    archetype_name=name,
                    primary_color=arch_data.get("primary_color"),
                    secondary_color=arch_data.get("secondary_color"),
                    display_card_id=arch_data.get("display_card_id"),
                )

            arch = self.archetypes[name]

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

            # Load digilab stats
            digilab = arch_data.get("digilab_stats")
            if digilab and arch.digilab_stats is None:
                arch.digilab_stats = DigiLabStats(
                    times_played=digilab.get("times_played", 0),
                    conversion_rate=digilab.get("conversion_rate", 0.0),
                    win_rate=digilab.get("win_rate", 0.0),
                    top4_rate=digilab.get("top4_rate", 0.0),
                )

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
        "--import-file", metavar="FILE",
        help="Import a local deck file (TTS/text format)",
    )
    parser.add_argument(
        "--import-archetype", metavar="NAME",
        help="Archetype name for --import-file",
    )
    parser.add_argument(
        "--fetch-meta", action="store_true",
        help="Fetch meta stats from DigiLab MotherDuck (requires .env token)",
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

    if not actions_taken and not args.report:
        parser.print_help()


if __name__ == "__main__":
    main()
