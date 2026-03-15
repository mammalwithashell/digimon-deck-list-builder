"""DigiLab PostgreSQL client for scoped meta queries.

Standalone module using raw psycopg2 — completely independent of the app's
SQLAlchemy database.  Connects to the same DigiLab DB used by meta_loader.
"""

from __future__ import annotations

import json
import os
import statistics
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

# Reuse the same connection string as meta_loader
DIGILAB_CONN_STR = (
    "postgresql://neondb_owner:npg_S85ItviFhMPE@"
    "ep-tiny-morning-ajc7jhvm-pooler.c-3.us-east-2.aws.neon.tech/"
    "digilab-db?sslmode=require"
)


@dataclass
class StoreInfo:
    store_id: int
    name: str
    city: str
    state: str
    scene_id: Optional[int] = None
    tournament_count: int = 0


@dataclass
class SceneInfo:
    scene_id: int
    name: str
    display_name: str
    tournament_count: int = 0


@dataclass
class ScopedArchetypeStats:
    archetype_name: str
    meta_share: float = 0.0
    conversion_rate: float = 0.0
    win_rate: float = 0.0
    times_played: int = 0


@dataclass
class ScopedMetaResult:
    """Result from get_scoped_meta with aggregate stats alongside per-archetype data."""

    archetypes: Dict[str, ScopedArchetypeStats]
    total_results: int = 0
    median_times_played: float = 0.0
    mean_times_played: float = 0.0


@dataclass
class PlayerResult:
    """A single tournament result for a player."""

    player_name: str
    archetype_name: str
    store_name: str
    store_id: int
    event_date: Optional[str] = None
    placement: Optional[int] = None
    wins: int = 0
    losses: int = 0


@dataclass
class PlayerSummary:
    """Aggregated player profile across tournaments."""

    player_name: str
    archetypes_played: Dict[str, int] = field(default_factory=dict)
    stores_attended: Dict[str, int] = field(default_factory=dict)
    total_results: int = 0
    last_seen: Optional[str] = None
    results: List[PlayerResult] = field(default_factory=list)


ARCHETYPE_ALIASES_PATH = os.path.join(
    os.path.dirname(__file__), "engine", "data", "archetype_aliases.json"
)

_ALIAS_MAP: Optional[Dict[str, str]] = None


def _load_alias_map() -> Dict[str, str]:
    path = os.environ.get("ARCHETYPE_ALIASES_PATH", ARCHETYPE_ALIASES_PATH)
    try:
        with open(path, "r", encoding="utf-8") as f:
            raw = json.load(f)
    except FileNotFoundError:
        return {}
    result: Dict[str, str] = {}
    for canonical, aliases in raw.items():
        if canonical.startswith("_"):
            continue
        for alias in aliases:
            result[alias.lower()] = canonical
    return result


def canonicalize_archetype(name: str) -> str:
    """Resolve an archetype name to its canonical form via the alias index."""
    global _ALIAS_MAP
    if _ALIAS_MAP is None:
        _ALIAS_MAP = _load_alias_map()
    return _ALIAS_MAP.get(name.lower(), name)


def _connect():
    """Connect to DigiLab PostgreSQL database."""
    import psycopg2
    conn_str = os.environ.get("DIGILAB_CONN_STR", DIGILAB_CONN_STR)
    return psycopg2.connect(conn_str)


def list_stores(min_tournaments: int = 1) -> List[StoreInfo]:
    """List stores with at least *min_tournaments* tournaments."""
    conn = _connect()
    try:
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT s.store_id, s.name, s.city, s.state, s.scene_id,
                       COUNT(t.tournament_id) AS tournament_count
                FROM stores s
                JOIN tournaments t USING (store_id)
                GROUP BY s.store_id, s.name, s.city, s.state, s.scene_id
                HAVING COUNT(t.tournament_id) >= %s
                ORDER BY COUNT(t.tournament_id) DESC
                """,
                (min_tournaments,),
            )
            return [
                StoreInfo(
                    store_id=row[0],
                    name=row[1],
                    city=row[2] or "",
                    state=row[3] or "",
                    scene_id=row[4],
                    tournament_count=row[5],
                )
                for row in cur.fetchall()
            ]
    finally:
        conn.close()


def list_scenes(min_tournaments: int = 1) -> List[SceneInfo]:
    """List scenes with at least *min_tournaments* tournaments."""
    conn = _connect()
    try:
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT sc.scene_id, sc.name, sc.display_name,
                       COUNT(t.tournament_id) AS tournament_count
                FROM scenes sc
                JOIN stores s USING (scene_id)
                JOIN tournaments t USING (store_id)
                GROUP BY sc.scene_id, sc.name, sc.display_name
                HAVING COUNT(t.tournament_id) >= %s
                ORDER BY COUNT(t.tournament_id) DESC
                """,
                (min_tournaments,),
            )
            return [
                SceneInfo(
                    scene_id=row[0],
                    name=row[1],
                    display_name=row[2] or row[1],
                    tournament_count=row[3],
                )
                for row in cur.fetchall()
            ]
    finally:
        conn.close()


def _build_scope_clause(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
) -> Tuple[str, tuple]:
    """Build WHERE clause and params for scoped queries.

    Returns (where_clause, params) tuple.
    """
    conditions: List[str] = []
    params: list = []

    if scene_id is not None:
        conditions.append(
            "t.store_id IN (SELECT store_id FROM stores WHERE scene_id = %s)"
        )
        params.append(scene_id)
    elif store_ids:
        conditions.append("t.store_id = ANY(%s)")
        params.append(store_ids)
    else:
        raise ValueError("Must provide store_ids or scene_id")

    if since_date is not None:
        conditions.append("t.event_date >= %s")
        params.append(since_date)

    where_clause = "WHERE " + " AND ".join(conditions)
    return where_clause, tuple(params)


def get_scoped_meta(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
) -> ScopedMetaResult:
    """Query scoped archetype stats from DigiLab.

    Provide *store_ids* for store-level scope or *scene_id* for scene-level.
    Optionally filter to tournaments on or after *since_date* (ISO format,
    e.g. ``"2025-12-01"``).

    Returns a :class:`ScopedMetaResult` with per-archetype stats and
    aggregate median/mean play counts for sample-size-aware analysis.
    """
    conn = _connect()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date
            )

            cur.execute(
                f"""
                SELECT da.archetype_name,
                       COUNT(r.result_id) AS times_played,
                       SUM(r.wins) AS total_wins,
                       SUM(r.losses) AS total_losses,
                       COUNT(CASE WHEN r.placement <= 4 THEN 1 END) AS top4_count
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                GROUP BY da.archetype_id, da.archetype_name
                HAVING COUNT(r.result_id) > 0
                """,
                params,
            )

            rows = cur.fetchall()

            # Aggregate rows that alias to the same canonical name
            aggregated: Dict[str, List] = defaultdict(list)
            for row in rows:
                canonical = canonicalize_archetype(row[0])
                aggregated[canonical].append(row)

            # Compute total for meta_share denominator
            total_played = sum(row[1] for row in rows)

            archetypes: Dict[str, ScopedArchetypeStats] = {}
            play_counts: List[int] = []

            for name, agg_rows in aggregated.items():
                times_played = sum(r[1] for r in agg_rows)
                total_wins = sum(r[2] or 0 for r in agg_rows)
                total_losses = sum(r[3] or 0 for r in agg_rows)
                top4_count = sum(r[4] or 0 for r in agg_rows)

                total_games = total_wins + total_losses
                win_rate = total_wins / total_games if total_games > 0 else 0.0
                conversion_rate = top4_count / times_played if times_played > 0 else 0.0
                meta_share = times_played / total_played if total_played > 0 else 0.0

                archetypes[name] = ScopedArchetypeStats(
                    archetype_name=name,
                    meta_share=meta_share,
                    conversion_rate=conversion_rate,
                    win_rate=win_rate,
                    times_played=times_played,
                )
                play_counts.append(times_played)

            # Compute aggregate stats
            median_tp = statistics.median(play_counts) if play_counts else 0.0
            mean_tp = statistics.mean(play_counts) if play_counts else 0.0

            return ScopedMetaResult(
                archetypes=archetypes,
                total_results=total_played,
                median_times_played=median_tp,
                mean_times_played=mean_tp,
            )
    finally:
        conn.close()


def _probe_player_columns() -> Optional[str]:
    """Probe the results table for player identity columns.

    Returns the column name if found, or None if no player column exists.
    """
    conn = _connect()
    try:
        with conn.cursor() as cur:
            cur.execute(
                """
                SELECT column_name
                FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'results'
                  AND column_name IN ('player_name', 'user_name', 'player',
                                      'username', 'player_id')
                ORDER BY
                    CASE column_name
                        WHEN 'player_name' THEN 1
                        WHEN 'player' THEN 2
                        WHEN 'user_name' THEN 3
                        WHEN 'username' THEN 4
                        WHEN 'player_id' THEN 5
                    END
                LIMIT 1
                """
            )
            row = cur.fetchone()
            return row[0] if row else None
    finally:
        conn.close()


# Cache the probe result
_PLAYER_COLUMN: Optional[str] = None
_PLAYER_COLUMN_PROBED: bool = False


def _get_player_column() -> Optional[str]:
    """Get the player column name, probing once and caching."""
    global _PLAYER_COLUMN, _PLAYER_COLUMN_PROBED
    if not _PLAYER_COLUMN_PROBED:
        try:
            _PLAYER_COLUMN = _probe_player_columns()
        except Exception:
            _PLAYER_COLUMN = None
        _PLAYER_COLUMN_PROBED = True
    return _PLAYER_COLUMN


def get_player_history(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
) -> List[PlayerSummary]:
    """Query per-player tournament history from DigiLab.

    Probes the results table for a player identity column. If none exists,
    returns an empty list.

    Returns a list of :class:`PlayerSummary` sorted by total results
    descending.
    """
    player_col = _get_player_column()
    if player_col is None:
        return []

    conn = _connect()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date
            )

            cur.execute(
                f"""
                SELECT r.{player_col},
                       da.archetype_name,
                       s.name AS store_name,
                       s.store_id,
                       t.event_date,
                       r.placement,
                       r.wins,
                       r.losses
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN stores s USING (store_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                  AND r.{player_col} IS NOT NULL
                  AND r.{player_col} != ''
                ORDER BY t.event_date DESC
                """,
                params,
            )

            rows = cur.fetchall()

            # Aggregate into PlayerSummary objects
            players: Dict[str, PlayerSummary] = {}
            for row in rows:
                pname = str(row[0]).strip()
                if not pname:
                    continue

                archetype = canonicalize_archetype(row[1])
                store_name = row[2] or ""
                store_id = row[3]
                event_date = row[4].strftime("%Y-%m-%d") if row[4] else None
                placement = row[5]
                wins = row[6] or 0
                losses = row[7] or 0

                result = PlayerResult(
                    player_name=pname,
                    archetype_name=archetype,
                    store_name=store_name,
                    store_id=store_id,
                    event_date=event_date,
                    placement=placement,
                    wins=wins,
                    losses=losses,
                )

                if pname not in players:
                    players[pname] = PlayerSummary(player_name=pname)

                summary = players[pname]
                summary.results.append(result)
                summary.total_results += 1
                summary.archetypes_played[archetype] = (
                    summary.archetypes_played.get(archetype, 0) + 1
                )
                summary.stores_attended[store_name] = (
                    summary.stores_attended.get(store_name, 0) + 1
                )
                if event_date and (
                    summary.last_seen is None or event_date > summary.last_seen
                ):
                    summary.last_seen = event_date

            # Sort by total results descending
            return sorted(
                players.values(),
                key=lambda p: p.total_results,
                reverse=True,
            )
    finally:
        conn.close()
