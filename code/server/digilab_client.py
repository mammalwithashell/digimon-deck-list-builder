"""DigiLab PostgreSQL client for scoped meta queries.

Standalone module using raw psycopg2 — completely independent of the app's
SQLAlchemy database.  Connects to the same DigiLab DB used by meta_loader.
"""

from __future__ import annotations

import json
import math
import os
import statistics
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

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


@dataclass
class PlayerThreatProfile:
    """A player's threat profile for a store: how good they are and what they play."""

    player_name: str
    likely_archetype: str
    archetype_win_rate: float = 0.0
    overall_win_rate: float = 0.0
    total_games: int = 0
    event_count: int = 0
    threat_score: float = 0.0


@dataclass
class AttendanceProfile:
    """A player's attendance regularity at a store."""

    player_name: str
    event_count: int = 0
    first_seen: Optional[str] = None
    last_seen: Optional[str] = None
    regularity_score: float = 0.0
    is_regular: bool = False


@dataclass
class ColorDistribution:
    """Color pair frequency in a meta."""

    primary_color: str
    secondary_color: Optional[str] = None
    count: int = 0
    share: float = 0.0


@dataclass
class DecklistRecord:
    """A single decklist fetched from DigiLab with context."""

    result_id: int
    archetype_name: str
    decklist_json: Dict[str, Any]
    placement: Optional[int] = None
    wins: int = 0
    losses: int = 0
    player_name: Optional[str] = None
    event_date: Optional[str] = None
    player_count: Optional[int] = None


@dataclass
class PeriodMeta:
    """Meta snapshot for a time period."""

    period_label: str
    period_start: str
    period_end: str
    archetypes: Dict[str, ScopedArchetypeStats] = field(default_factory=dict)
    total_results: int = 0


from data_paths import ARCHETYPE_ALIASES as _ARCHETYPE_ALIASES_PATH

ARCHETYPE_ALIASES_PATH = str(_ARCHETYPE_ALIASES_PATH)

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
    event_type: Optional[str] = None,
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

    if event_type is not None:
        conditions.append("t.event_type ILIKE %s")
        params.append(f"%{event_type}%")

    where_clause = "WHERE " + " AND ".join(conditions)
    return where_clause, tuple(params)


def get_scoped_meta(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
    event_type: Optional[str] = None,
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
                store_ids, scene_id, since_date, event_type
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
    event_type: Optional[str] = None,
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
                store_ids, scene_id, since_date, event_type
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


# ---------------------------------------------------------------------------
# Feature 11: Color distribution
# ---------------------------------------------------------------------------

def get_color_distribution(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
    event_type: Optional[str] = None,
) -> List[ColorDistribution]:
    """Query color pair frequencies from DigiLab."""
    conn = _connect()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date, event_type
            )
            cur.execute(
                f"""
                SELECT da.primary_color, da.secondary_color, COUNT(*) AS cnt
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                GROUP BY da.primary_color, da.secondary_color
                ORDER BY cnt DESC
                """,
                params,
            )
            rows = cur.fetchall()
            total = sum(r[2] for r in rows) or 1
            return [
                ColorDistribution(
                    primary_color=row[0] or "Unknown",
                    secondary_color=row[1],
                    count=row[2],
                    share=row[2] / total,
                )
                for row in rows
            ]
    finally:
        conn.close()


# ---------------------------------------------------------------------------
# Feature 8: Size-normalized meta
# ---------------------------------------------------------------------------

def get_scoped_meta_normalized(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
    event_type: Optional[str] = None,
) -> ScopedMetaResult:
    """Like get_scoped_meta but weights conversion rates by tournament size.

    Top-4 at a 32-player event counts more than top-4 at an 8-player event.
    Weight = sqrt(player_count) for each result.
    """
    conn = _connect()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date, event_type
            )
            cur.execute(
                f"""
                SELECT da.archetype_name,
                       r.result_id, r.placement, r.wins, r.losses,
                       t.player_count
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                """,
                params,
            )
            rows = cur.fetchall()

            # Group by canonical archetype
            by_arch: Dict[str, list] = defaultdict(list)
            for row in rows:
                canonical = canonicalize_archetype(row[0])
                by_arch[canonical].append(row)

            total_played = len(rows)
            archetypes: Dict[str, ScopedArchetypeStats] = {}
            play_counts: List[int] = []

            for name, arch_rows in by_arch.items():
                times_played = len(arch_rows)
                total_wins = sum(r[3] or 0 for r in arch_rows)
                total_losses = sum(r[4] or 0 for r in arch_rows)

                # Size-weighted conversion: weight each top-4 by sqrt(player_count)
                weighted_top4 = 0.0
                total_weight = 0.0
                for r in arch_rows:
                    pc = r[5] or 8  # default to 8 if unknown
                    w = math.sqrt(pc)
                    total_weight += w
                    if r[2] is not None and r[2] <= 4:
                        weighted_top4 += w

                total_games = total_wins + total_losses
                win_rate = total_wins / total_games if total_games > 0 else 0.0
                conversion_rate = (
                    weighted_top4 / total_weight if total_weight > 0 else 0.0
                )
                meta_share = (
                    times_played / total_played if total_played > 0 else 0.0
                )

                archetypes[name] = ScopedArchetypeStats(
                    archetype_name=name,
                    meta_share=meta_share,
                    conversion_rate=conversion_rate,
                    win_rate=win_rate,
                    times_played=times_played,
                )
                play_counts.append(times_played)

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


# ---------------------------------------------------------------------------
# Feature 7: Meta over time
# ---------------------------------------------------------------------------

def get_meta_over_time(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
    event_type: Optional[str] = None,
    periods: int = 3,
) -> List[PeriodMeta]:
    """Query meta broken into time periods for trend analysis."""
    conn = _connect()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date, event_type
            )
            cur.execute(
                f"""
                SELECT da.archetype_name,
                       r.result_id, r.placement, r.wins, r.losses,
                       t.event_date
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                ORDER BY t.event_date
                """,
                params,
            )
            rows = cur.fetchall()
            if not rows:
                return []

            # Determine date range and bucket boundaries
            dates = [r[5] for r in rows if r[5] is not None]
            if not dates:
                return []

            from datetime import timedelta
            min_date = min(dates)
            max_date = max(dates)
            span = (max_date - min_date).days + 1
            bucket_days = max(1, span // periods)

            # Assign rows to period buckets
            buckets: List[list] = [[] for _ in range(periods)]
            for row in rows:
                if row[5] is None:
                    continue
                idx = min((row[5] - min_date).days // bucket_days, periods - 1)
                buckets[idx].append(row)

            result: List[PeriodMeta] = []
            for i, bucket_rows in enumerate(buckets):
                start = min_date + timedelta(days=i * bucket_days)
                end = min_date + timedelta(days=(i + 1) * bucket_days - 1)
                if i == periods - 1:
                    end = max_date

                # Compute per-archetype stats for this period
                by_arch: Dict[str, list] = defaultdict(list)
                for r in bucket_rows:
                    canonical = canonicalize_archetype(r[0])
                    by_arch[canonical].append(r)

                total = len(bucket_rows)
                archetypes: Dict[str, ScopedArchetypeStats] = {}
                for name, arch_rows in by_arch.items():
                    tp = len(arch_rows)
                    tw = sum(r[3] or 0 for r in arch_rows)
                    tl = sum(r[4] or 0 for r in arch_rows)
                    top4 = sum(1 for r in arch_rows if r[2] and r[2] <= 4)
                    tg = tw + tl
                    archetypes[name] = ScopedArchetypeStats(
                        archetype_name=name,
                        meta_share=tp / total if total else 0.0,
                        conversion_rate=top4 / tp if tp else 0.0,
                        win_rate=tw / tg if tg else 0.0,
                        times_played=tp,
                    )

                result.append(PeriodMeta(
                    period_label=f"{start.strftime('%m/%d')}-{end.strftime('%m/%d')}",
                    period_start=start.strftime("%Y-%m-%d"),
                    period_end=end.strftime("%Y-%m-%d"),
                    archetypes=archetypes,
                    total_results=total,
                ))

            return result
    finally:
        conn.close()


# ---------------------------------------------------------------------------
# Features 4-6: Decklist fetching
# ---------------------------------------------------------------------------

def get_decklists_for_archetype(
    archetype_name: str,
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
    since_date: Optional[str] = None,
    event_type: Optional[str] = None,
) -> List[DecklistRecord]:
    """Fetch full decklists for an archetype from DigiLab."""
    conn = _connect()
    player_col = _get_player_column()
    try:
        with conn.cursor() as cur:
            where_clause, params = _build_scope_clause(
                store_ids, scene_id, since_date, event_type
            )
            player_select = f", r.{player_col}" if player_col else ""
            cur.execute(
                f"""
                SELECT r.result_id, da.archetype_name, r.decklist_json,
                       r.placement, r.wins, r.losses,
                       t.event_date, t.player_count
                       {player_select}
                FROM results r
                JOIN tournaments t USING (tournament_id)
                JOIN deck_archetypes da USING (archetype_id)
                {where_clause}
                  AND da.archetype_name ILIKE %s
                  AND r.decklist_json IS NOT NULL
                  AND r.decklist_json != ''
                ORDER BY t.event_date DESC
                """,
                params + (archetype_name,),
            )
            records: List[DecklistRecord] = []
            for row in cur.fetchall():
                try:
                    dl_json = (
                        json.loads(row[2]) if isinstance(row[2], str) else row[2]
                    )
                except (json.JSONDecodeError, TypeError):
                    continue
                if not dl_json:
                    continue
                pname = str(row[8]).strip() if player_col and len(row) > 8 and row[8] else None
                records.append(DecklistRecord(
                    result_id=row[0],
                    archetype_name=canonicalize_archetype(row[1]),
                    decklist_json=dl_json,
                    placement=row[3],
                    wins=row[4] or 0,
                    losses=row[5] or 0,
                    event_date=row[6].strftime("%Y-%m-%d") if row[6] else None,
                    player_count=row[7],
                    player_name=pname,
                ))
            return records
    finally:
        conn.close()
