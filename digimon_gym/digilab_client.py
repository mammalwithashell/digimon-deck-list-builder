"""DigiLab PostgreSQL client for scoped meta queries.

Standalone module using raw psycopg2 — completely independent of the app's
SQLAlchemy database.  Connects to the same DigiLab DB used by meta_loader.
"""

from __future__ import annotations

import json
import os
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, List, Optional

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


def get_scoped_meta(
    store_ids: Optional[List[int]] = None,
    scene_id: Optional[int] = None,
) -> Dict[str, ScopedArchetypeStats]:
    """Query scoped archetype stats from DigiLab.

    Provide *store_ids* for store-level scope or *scene_id* for scene-level.
    Returns a dict keyed by archetype name.
    """
    conn = _connect()
    try:
        with conn.cursor() as cur:
            if scene_id is not None:
                where_clause = """
                    WHERE t.store_id IN (
                        SELECT store_id FROM stores WHERE scene_id = %s
                    )
                """
                params = (scene_id,)
            elif store_ids:
                where_clause = "WHERE t.store_id = ANY(%s)"
                params = (store_ids,)
            else:
                raise ValueError("Must provide store_ids or scene_id")

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

            result: Dict[str, ScopedArchetypeStats] = {}
            for name, agg_rows in aggregated.items():
                times_played = sum(r[1] for r in agg_rows)
                total_wins = sum(r[2] or 0 for r in agg_rows)
                total_losses = sum(r[3] or 0 for r in agg_rows)
                top4_count = sum(r[4] or 0 for r in agg_rows)

                total_games = total_wins + total_losses
                win_rate = total_wins / total_games if total_games > 0 else 0.0
                conversion_rate = top4_count / times_played if times_played > 0 else 0.0
                meta_share = times_played / total_played if total_played > 0 else 0.0

                result[name] = ScopedArchetypeStats(
                    archetype_name=name,
                    meta_share=meta_share,
                    conversion_rate=conversion_rate,
                    win_rate=win_rate,
                    times_played=times_played,
                )

            return result
    finally:
        conn.close()
