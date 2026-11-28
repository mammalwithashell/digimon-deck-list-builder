"""Board-State Studio backend router (add-board-state-studio).

Provides the full Studio HTTP API over Rust (`RustDebugGame`, `RustReplaySession`,
`inspect_position`):
- Session lifecycle: Compose <-> Dual-Seat Play & Record <-> Stepped Pilot Model Playout
- Studio Library with hierarchical folders and tags (`qa/puzzles/_library.json`)
- Multi-source Replay Loader (DB recordings, /watch queues, native JSON, DCGO JSONL,
  auto-converted MelonLoader `raw-*.jsonl`) + step-to-Studio forking
- Canonical `studio-puzzle-v1` CRUD under `qa/puzzles/` + exporters to `qa/scenarios/`
  and `qa/search-exams/`
- Single-position ONNX policy/value & MCTS search inspection
"""

from __future__ import annotations

import importlib.util
import json
import os
import random
import re
import subprocess
import sys
import tempfile
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Literal, Optional
from uuid import uuid4

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

import digimon_engine
from digimon_engine import RustDebugGame, load_implemented_card_ids

from server.routers.debug_games import _apply_scalar_state, _apply_zone, _debug_events
from server.routers.games import GameMeta, game_meta
from server.routers.schemas import DebugZoneSpec
from server.routers.state import active_games

try:
    from sqlalchemy import select as _sa_select
    from server.db.database import async_session_factory as _async_session_factory
    from server.db.models import GameRecording as _GameRecording
except Exception:  # pragma: no cover - engine-only env without DB deps
    _sa_select = None
    _async_session_factory = None
    _GameRecording = None

router = APIRouter(prefix="/studio", tags=["studio"])

# ── Repository paths ──────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parents[3]
QA_ROOT = REPO_ROOT / "qa"
PUZZLES_DIR = QA_ROOT / "puzzles"
LIBRARY_INDEX_PATH = PUZZLES_DIR / "_library.json"
REPLAY_ANALYSIS_CACHE_PATH = PUZZLES_DIR / "_replay_analysis_cache.json"
SCENARIOS_DIR = QA_ROOT / "scenarios"
SEARCH_EXAMS_DIR = QA_ROOT / "search-exams"
DCGO_RECORDINGS_DIR = QA_ROOT / "dcgo-mod-recordings"
DEFAULT_DCGO_LIVE_DIR = Path.home() / "Documents" / "DCGO_Application" / "Game" / "UserData" / "DcgoRecorder"
DEFAULT_DCGO_PRIVATE_DIR = Path(r"D:\dcgo-recordings-private")
DECK_LIBRARY_PATH = REPO_ROOT / "data" / "deck_library.json"
CARDS_JSON_PATH = REPO_ROOT / "data" / "cards.json"
CONVERT_RAW_SCRIPT = REPO_ROOT / "code" / "dcgo-melon-recorder" / "convert_raw.py"
MODELS_DIR = REPO_ROOT / "models"

SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,95}$")
FOLDER_SEG_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,47}$")

DEFAULT_FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


# ── Path & slug validation ────────────────────────────────────────────────────

def _validate_slug(slug: str, label: str = "id") -> str:
    s = slug.strip()
    if not s or not SLUG_RE.match(s):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid {label} {slug!r}: must be kebab-case alphanumeric (a-z, 0-9, -, _)",
        )
    return s


def _validate_folder(folder: Optional[str]) -> str:
    if not folder:
        return ""
    cleaned = folder.strip().strip("/").replace("\\", "/")
    if not cleaned:
        return ""
    parts = [p for p in cleaned.split("/") if p]
    for p in parts:
        if p in (".", "..") or not FOLDER_SEG_RE.match(p):
            raise HTTPException(
                status_code=400,
                detail=f"Invalid folder segment {p!r} in {folder!r}",
            )
    return "/".join(parts)


def _safe_under(root: Path, rel: str) -> Path:
    root_res = root.resolve()
    target = (root_res / rel).resolve()
    try:
        target.relative_to(root_res)
    except ValueError:
        raise HTTPException(status_code=400, detail="Path escapes allowlisted root")
    return target


def _models_dir() -> Path:
    env_dir = os.environ.get("DIGIMON_MODELS_DIR")
    if env_dir:
        return Path(env_dir)
    return MODELS_DIR


def _dcgo_recording_roots() -> dict[str, Path]:
    """Allowlisted named roots for DCGO recordings (repo + local MelonLoader output).

    Override or extend via `DIGIMON_DCGO_RECORDER_DIRS` (semicolon-separated paths,
    optionally prefixed with `<root_id>=<path>`).
    """
    roots: dict[str, Path] = {"repo": DCGO_RECORDINGS_DIR}
    if DEFAULT_DCGO_LIVE_DIR.exists() and DEFAULT_DCGO_LIVE_DIR.is_dir():
        roots["dcgo_live"] = DEFAULT_DCGO_LIVE_DIR
    if DEFAULT_DCGO_PRIVATE_DIR.exists() and DEFAULT_DCGO_PRIVATE_DIR.is_dir():
        roots["dcgo_private"] = DEFAULT_DCGO_PRIVATE_DIR

    extra = os.environ.get("DIGIMON_DCGO_RECORDER_DIRS", "").strip()
    if extra:
        for idx, raw_entry in enumerate(extra.split(";"), 1):
            entry = raw_entry.strip()
            if not entry:
                continue
            if "=" in entry:
                rid, rpath = entry.split("=", 1)
                rid = rid.strip()
                p = Path(rpath.strip())
            else:
                rid = f"dcgo_extra_{idx}"
                p = Path(entry)
            if rid and SLUG_RE.match(rid) and p.exists() and p.is_dir():
                roots[rid] = p
    return roots


_RAW_SCAN_CACHE: dict[tuple[str, int, int], list[dict[str, Any]]] = {}
_V1_SCAN_CACHE: dict[tuple[str, int, int], dict[str, Any]] = {}
_GAME_SLICE_RE = re.compile(r"^(.*)#g(\d+)$")

_DECK_CLASSIFIER_LOCK = threading.Lock()
_DECK_CLASSIFIER_LIB: Any = None
_DECK_CLASSIFIER_LOADED = False

_CONVERT_RAW_LOCK = threading.Lock()
_CONVERT_RAW_MOD: Any = None
_CONVERT_RAW_CARDS: Any = None

_ANALYSIS_CACHE_LOCK = threading.Lock()
_ANALYSIS_CACHE: Optional[dict[str, dict[str, Any]]] = None
_BG_SCAN_LOCK = threading.Lock()
_BG_SCAN_RUNNING = False


def _slugify_tag(text: Optional[str]) -> Optional[str]:
    if not text:
        return None
    slug = re.sub(r"[^a-z0-9]+", "-", str(text).strip().lower()).strip("-")
    return slug[:40] if slug else None


def _get_deck_classifier() -> Any:
    global _DECK_CLASSIFIER_LIB, _DECK_CLASSIFIER_LOADED
    if _DECK_CLASSIFIER_LOADED:
        return _DECK_CLASSIFIER_LIB
    with _DECK_CLASSIFIER_LOCK:
        if not _DECK_CLASSIFIER_LOADED:
            try:
                from server.classifier.meta_tier import load_library_from_path

                if DECK_LIBRARY_PATH.exists():
                    _DECK_CLASSIFIER_LIB = load_library_from_path(DECK_LIBRARY_PATH)
            except Exception:
                _DECK_CLASSIFIER_LIB = None
            _DECK_CLASSIFIER_LOADED = True
    return _DECK_CLASSIFIER_LIB


def _classify_deck_cards(cards: list[str]) -> Optional[str]:
    clean = [str(c) for c in cards if isinstance(c, str) and c and not c.startswith("?")]
    if len(clean) < 10:
        return None
    lib = _get_deck_classifier()
    if lib is None:
        return None
    try:
        from server.classifier.meta_tier import classify_deck

        arch, _tier, _score = classify_deck(clean, lib)
        return str(arch) if arch else None
    except Exception:
        return None


def _extract_snapshot_cards(pl: dict[str, Any]) -> list[str]:
    out: list[str] = []
    for zone in ("library", "hand", "security", "egg_library", "trash"):
        vals = pl.get(zone)
        if isinstance(vals, list):
            for c in vals:
                if isinstance(c, str) and c:
                    out.append(c)
    for zone in ("battle", "breeding"):
        perms = pl.get(zone)
        if isinstance(perms, list):
            for perm in perms:
                if isinstance(perm, list):
                    for c in perm:
                        if isinstance(c, str) and c:
                            out.append(c)
    return out


def _format_iso_short(ts: Optional[str]) -> str:
    if not ts:
        return ""
    m = re.match(r"^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2})", str(ts))
    if m:
        return f"{m.group(1)} {m.group(2)}"
    return str(ts)[:16]


def _format_iso_date(ts: Optional[str]) -> str:
    if not ts:
        return ""
    m = re.match(r"^(\d{4}-\d{2}-\d{2})", str(ts))
    return m.group(1) if m else ""


def _build_matchup_title(
    start_ts: Optional[str],
    my_deck: Optional[str],
    opp_deck: Optional[str],
    fallback_stem: str,
    game_index: Optional[int] = None,
    turns: int = 0,
    ended: bool = True,
) -> str:
    date_part = _format_iso_date(start_ts) or _format_iso_short(start_ts)
    matchup = ""
    if my_deck or opp_deck:
        matchup = f"{my_deck or 'Unknown'} vs {opp_deck or 'Unknown'}"
    parts: list[str] = []
    if date_part:
        parts.append(date_part)
    if matchup:
        parts.append(matchup)
    elif not date_part:
        parts.append(fallback_stem)
    if game_index is not None:
        turns_str = f"T{turns}" if turns > 0 else "opening"
        status_str = "" if ended else " · live/trunc"
        parts.append(f"#{game_index} ({turns_str}{status_str})")
    elif not matchup:
        parts.append(fallback_stem)
    return " · ".join(parts)


def _load_analysis_cache() -> dict[str, dict[str, Any]]:
    global _ANALYSIS_CACHE
    with _ANALYSIS_CACHE_LOCK:
        if _ANALYSIS_CACHE is not None:
            return _ANALYSIS_CACHE
        if REPLAY_ANALYSIS_CACHE_PATH.exists():
            try:
                raw = json.loads(REPLAY_ANALYSIS_CACHE_PATH.read_text(encoding="utf-8"))
                if isinstance(raw, dict):
                    _ANALYSIS_CACHE = raw
                    return _ANALYSIS_CACHE
            except Exception:
                pass
        _ANALYSIS_CACHE = {}
        return _ANALYSIS_CACHE


def _save_analysis_cache() -> None:
    with _ANALYSIS_CACHE_LOCK:
        if _ANALYSIS_CACHE is None:
            return
        try:
            PUZZLES_DIR.mkdir(parents=True, exist_ok=True)
            REPLAY_ANALYSIS_CACHE_PATH.write_text(
                json.dumps(_ANALYSIS_CACHE, indent=2) + "\n", encoding="utf-8"
            )
        except Exception:
            pass


def _get_cached_analysis(ref: str, mtime_ns: int, size: int) -> Optional[dict[str, Any]]:
    cache = _load_analysis_cache()
    with _ANALYSIS_CACHE_LOCK:
        entry = cache.get(ref)
        if (
            isinstance(entry, dict)
            and entry.get("mtime_ns") == mtime_ns
            and entry.get("size") == size
        ):
            return dict(entry)
    return None


def _set_cached_analysis(ref: str, mtime_ns: int, size: int, result: dict[str, Any], *, flush: bool = True) -> None:
    cache = _load_analysis_cache()
    with _ANALYSIS_CACHE_LOCK:
        cache[ref] = {"mtime_ns": mtime_ns, "size": size, **result}
    if flush:
        _save_analysis_cache()


def _scan_raw_melon_games(path: Path) -> list[dict[str, Any]]:
    """Split a MelonLoader `raw-*.jsonl` session file into individual games at
    `TurnStateMachine.Init`, skipping menu-only slices with no Snapshot rows.
    Extracts timestamps, DCGO `deck_name` (v0.2.1+), and classified archetypes
    for both players. Cached by (path, mtime_ns, size)."""
    try:
        st = path.stat()
    except OSError:
        return []
    cache_key = (str(path), st.st_mtime_ns, st.st_size)
    cached = _RAW_SCAN_CACHE.get(cache_key)
    if cached is not None:
        return cached

    slices: list[dict[str, Any]] = []
    cur: Optional[dict[str, Any]] = None
    game_idx = 0
    try:
        with path.open("r", encoding="utf-8", errors="replace") as f:
            for raw_line in f:
                s = raw_line.strip()
                if not s:
                    continue
                try:
                    row = json.loads(s)
                except ValueError:
                    continue
                if not isinstance(row, dict):
                    continue
                method = row.get("method")
                if method == "TurnStateMachine.Init":
                    game_idx += 1
                    cur = {
                        "game_index": game_idx,
                        "start_ts": row.get("t") or row.get("ts") or row.get("timestamp") or "",
                        "turns": 0,
                        "has_snapshot": False,
                        "line_count": 0,
                        "ended": False,
                        "surrendered": False,
                        "dcgo_deck_name": None,
                        "my_cards": None,
                        "opp_cards": None,
                        "is_ai": None,
                    }
                    slices.append(cur)
                if cur is None:
                    continue
                cur["line_count"] += 1
                if not cur["start_ts"]:
                    cur["start_ts"] = row.get("t") or row.get("ts") or row.get("timestamp") or ""
                if isinstance(method, str):
                    if method.startswith("Snapshot."):
                        cur["has_snapshot"] = True
                        if not cur["dcgo_deck_name"] and row.get("deck_name"):
                            cur["dcgo_deck_name"] = str(row["deck_name"]).strip()
                        if cur["is_ai"] is None and "is_ai" in row:
                            cur["is_ai"] = bool(row["is_ai"])
                        if cur["my_cards"] is None and isinstance(row.get("players"), list):
                            you_id = row.get("you", 0)
                            for pl in row["players"]:
                                if not isinstance(pl, dict):
                                    continue
                                cards = _extract_snapshot_cards(pl)
                                if pl.get("id") == you_id:
                                    cur["my_cards"] = cards
                                else:
                                    cur["opp_cards"] = cards
                    elif method == "TurnStateMachine.EndGame":
                        cur["ended"] = True
                        args = row.get("args")
                        if isinstance(args, list) and len(args) > 1 and args[1] is True:
                            cur["surrendered"] = True
                    elif method == "TurnStateMachine.Surrender":
                        cur["ended"] = True
                        cur["surrendered"] = True
                turn = row.get("turn")
                if isinstance(turn, int) and turn > cur["turns"]:
                    cur["turns"] = turn
    except OSError:
        return []

    playable: list[dict[str, Any]] = []
    for g in slices:
        if not g["has_snapshot"]:
            continue
        my_classified = _classify_deck_cards(g["my_cards"] or [])
        opp_classified = _classify_deck_cards(g["opp_cards"] or [])
        dcgo_name = g["dcgo_deck_name"] or None
        # Prioritize user's DCGO deck name over classifier label
        my_deck = dcgo_name or my_classified
        opp_deck = opp_classified
        if g["surrendered"]:
            finish_type = "concede"
        elif g["ended"]:
            finish_type = "natural"
        else:
            finish_type = "unfinished"
        playable.append(
            {
                "game_index": g["game_index"],
                "start_ts": g["start_ts"],
                "turns": g["turns"],
                "line_count": g["line_count"],
                "ended": g["ended"],
                "finish_type": finish_type,
                "is_ai": g["is_ai"],
                "dcgo_deck_name": dcgo_name,
                "my_deck": my_deck,
                "opp_deck": opp_deck,
            }
        )

    if len(_RAW_SCAN_CACHE) > 256:
        _RAW_SCAN_CACHE.clear()
    _RAW_SCAN_CACHE[cache_key] = playable
    return playable


def _scan_v1_dcgo_meta(path: Path) -> dict[str, Any]:
    """Read just the header rows (`game_start`, `initial_state`) of a schema-v1
    DCGO JSONL file to extract timestamp, DCGO deck name, and classified archetypes."""
    try:
        st = path.stat()
    except OSError:
        return {}
    cache_key = (str(path), st.st_mtime_ns, st.st_size)
    cached = _V1_SCAN_CACHE.get(cache_key)
    if cached is not None:
        return cached

    start_ts = ""
    dcgo_deck_name: Optional[str] = None
    my_cards: list[str] = []
    opp_cards: list[str] = []
    try:
        with path.open("r", encoding="utf-8", errors="replace") as f:
            for _ in range(4):
                line = f.readline()
                if not line:
                    break
                s = line.strip()
                if not s:
                    continue
                try:
                    row = json.loads(s)
                except ValueError:
                    continue
                if not isinstance(row, dict):
                    continue
                rtype = row.get("type")
                if rtype == "game_start":
                    start_ts = str(row.get("timestamp") or "")
                    if row.get("my_deck_name"):
                        dcgo_deck_name = str(row["my_deck_name"]).strip()
                    my_cards = list(row.get("my_deck_post_shuffle") or []) + list(row.get("my_egg_deck") or [])
                    opp_cards = (
                        list(row.get("opp_deck_post_shuffle") or row.get("opp_decklist_composition") or [])
                        + list(row.get("opp_egg_deck") or [])
                    )
                elif rtype == "initial_state":
                    if not my_cards and isinstance(row.get("my"), dict):
                        for z in ("deck", "hand", "security", "egg_deck"):
                            my_cards.extend(row["my"].get(z) or [])
                    if not opp_cards and isinstance(row.get("opp"), dict):
                        for z in ("deck", "hand", "security", "egg_deck"):
                            opp_cards.extend(row["opp"].get(z) or [])
    except OSError:
        return {}

    my_classified = _classify_deck_cards(my_cards)
    opp_classified = _classify_deck_cards(opp_cards)

    finish_type = "unfinished"
    try:
        with path.open("rb") as fb:
            fb.seek(0, os.SEEK_END)
            end_pos = fb.tell()
            fb.seek(max(0, end_pos - 4096))
            tail = fb.read().decode("utf-8", errors="replace").splitlines()
        for raw_ln in reversed(tail):
            s = raw_ln.strip()
            if not s:
                continue
            last_row = json.loads(s)
            if isinstance(last_row, dict) and last_row.get("type") == "game_end":
                reason = str(last_row.get("reason") or "").lower()
                winner = last_row.get("winner")
                if "concede" in reason or "surrender" in reason:
                    finish_type = "concede"
                elif reason.startswith("truncat") or winner in (-1, None):
                    finish_type = "unfinished"
                else:
                    finish_type = "natural"
            break
    except Exception:
        pass

    meta = {
        "start_ts": start_ts,
        "dcgo_deck_name": dcgo_deck_name,
        "my_deck": dcgo_deck_name or my_classified,
        "opp_deck": opp_classified,
        "finish_type": finish_type,
    }
    if len(_V1_SCAN_CACHE) > 512:
        _V1_SCAN_CACHE.clear()
    _V1_SCAN_CACHE[cache_key] = meta
    return meta


def _extract_raw_game_slice(path: Path, game_index: int) -> str:
    """Return the raw JSONL text for 1-based `game_index` in a MelonLoader raw file."""
    games: list[list[str]] = []
    cur: Optional[list[str]] = None
    with path.open("r", encoding="utf-8", errors="replace") as f:
        for raw_line in f:
            s = raw_line.strip()
            if not s:
                continue
            try:
                row = json.loads(s)
            except ValueError:
                continue
            if isinstance(row, dict) and row.get("method") == "TurnStateMachine.Init":
                cur = []
                games.append(cur)
            if cur is not None:
                cur.append(s)
    if game_index < 1 or game_index > len(games):
        raise HTTPException(
            status_code=404,
            detail=f"Game #{game_index} not found in {path.name} (found {len(games)} game(s))",
        )
    return "\n".join(games[game_index - 1]) + "\n"


def _resolve_dcgo_ref(ref: str) -> tuple[Path, Optional[int], str]:
    """Resolve a DCGO library ref (`<rel>`, `<root_id>:<rel>`, optionally `#g<n>`)
    strictly within allowlisted DCGO recording roots."""
    cleaned = ref.strip()
    if not cleaned:
        raise HTTPException(status_code=400, detail="Empty DCGO recording ref")
    game_index: Optional[int] = None
    m = _GAME_SLICE_RE.match(cleaned)
    if m:
        cleaned = m.group(1)
        game_index = int(m.group(2))
        if game_index < 1:
            raise HTTPException(status_code=400, detail="Game index must be >= 1")

    roots = _dcgo_recording_roots()
    root_id = "repo"
    rel = cleaned
    colon_pos = cleaned.find(":")
    slash_pos = min(
        [pos for pos in (cleaned.find("/"), cleaned.find("\\")) if pos != -1],
        default=-1,
    )
    if colon_pos != -1 and (slash_pos == -1 or colon_pos < slash_pos):
        root_id = cleaned[:colon_pos]
        rel = cleaned[colon_pos + 1 :]
        if root_id not in roots:
            raise HTTPException(status_code=400, detail=f"Unknown DCGO recording root: {root_id!r}")

    if not rel or rel.startswith(("/", "\\")):
        raise HTTPException(status_code=400, detail="Invalid DCGO recording path")

    target = _safe_under(roots[root_id], rel)
    return target, game_index, root_id


# ── Pydantic request/response models ──────────────────────────────────────────

class SeatConfig(BaseModel):
    type: Literal["human", "model", "greedy", "random"] = "human"
    model_id: Optional[str] = None
    simulations: int = Field(0, ge=0, le=2048)
    worlds: int = Field(1, ge=1, le=32)
    regime: Literal["open_list", "closed_list"] = "open_list"
    seed: int = 0


class StudioBoardPayload(BaseModel):
    deck1: list[str] = Field(default_factory=lambda: list(DEFAULT_FILLER_DECK))
    deck2: list[str] = Field(default_factory=lambda: list(DEFAULT_FILLER_DECK))
    seed: int = 0
    memory: int = Field(0, ge=-10, le=10)
    phase: str = "Main"
    turn: int = Field(1, ge=1)
    first_player: int = Field(1, ge=1, le=2)
    zones: dict[str, DebugZoneSpec] = Field(default_factory=dict)


class StudioSessionCreateRequest(BaseModel):
    board: Optional[StudioBoardPayload] = None
    blank: bool = False
    source: dict[str, Any] = Field(default_factory=lambda: {"kind": "composed"})
    seats: Optional[dict[str, SeatConfig]] = None


class StudioSeatsUpdateRequest(BaseModel):
    seats: dict[str, SeatConfig]


class StudioStepRequest(BaseModel):
    action_id: int


class StudioRewindRequest(BaseModel):
    target_step: int = Field(0, ge=0)


class StudioLineSaveRequest(BaseModel):
    id: Optional[str] = None
    name: str
    role: Literal["target", "avoid", "alternative"] = "target"
    actions: Optional[list[int]] = None


class StudioInspectRequest(BaseModel):
    model_id: str
    simulations: int = Field(0, ge=0, le=2048)
    worlds: int = Field(1, ge=1, le=32)
    regime: Literal["open_list", "closed_list"] = "open_list"
    seed: int = 0
    top_k: int = Field(12, ge=1, le=64)
    expectations: list[dict[str, Any]] = Field(default_factory=list)


class StudioReplayLoadRequest(BaseModel):
    source: Literal["recording_id", "watch_queue", "native_json", "dcgo_jsonl"]
    ref: Optional[str] = None
    content: Optional[str] = None
    verify: bool = True


class StudioReplaySeekRequest(BaseModel):
    step: int = Field(0, ge=0)


class StudioPuzzleSaveRequest(BaseModel):
    id: str
    title: str
    description: str = ""
    folder: str = ""
    tags: list[str] = Field(default_factory=list)
    source: dict[str, Any] = Field(default_factory=lambda: {"kind": "composed"})
    decks: dict[str, list[str]]
    seed: int = 0
    seat: int = Field(1, ge=1, le=2)
    setup: dict[str, Any]
    lines: list[dict[str, Any]] = Field(default_factory=list)
    expectations: list[dict[str, Any]] = Field(default_factory=list)
    assertions: dict[str, Any] = Field(default_factory=lambda: {"engine": [], "ui": []})
    rl_goal: dict[str, Any] = Field(
        default_factory=lambda: {
            "mode": "playout_to_win",
            "max_turns": None,
            "max_actions": None,
            "win_assertions": [],
        }
    )


class LibraryFolderCreateRequest(BaseModel):
    folder: str


class LibraryFolderRenameRequest(BaseModel):
    old_folder: str
    new_folder: str


class LibraryItemUpdateRequest(BaseModel):
    item_key: str
    folder: Optional[str] = None
    tags: Optional[list[str]] = None
    title_override: Optional[str] = None


# ── In-memory Studio session & replay stores ──────────────────────────────────

@dataclass
class StudioSession:
    session_id: str
    game_id: str
    base_board: dict[str, Any]
    seats: dict[str, dict[str, Any]]
    source: dict[str, Any]
    timeline: list[dict[str, Any]] = field(default_factory=list)
    lines: dict[str, dict[str, Any]] = field(default_factory=dict)
    validation_error: Optional[str] = None


@dataclass
class StudioReplayEntry:
    replay_id: str
    runner: Any  # RustReplaySession
    source: dict[str, Any]
    title: str = ""
    item_key: Optional[str] = None
    truncation_warning: Optional[str] = None
    my_deck: Optional[str] = None
    opp_deck: Optional[str] = None
    divergence_status: Optional[str] = None
    auto_tags: list[str] = field(default_factory=list)
    user_tags: list[str] = field(default_factory=list)
    step_log: list[dict[str, Any]] = field(default_factory=list)


_studio_sessions: dict[str, StudioSession] = {}
_studio_replays: dict[str, StudioReplayEntry] = {}


# ── Helpers: build / rebuild staged RustDebugGame ─────────────────────────────

def _build_debug_game_from_board(board: dict[str, Any]) -> tuple[RustDebugGame, Optional[str]]:
    deck1 = list(board.get("deck1") or DEFAULT_FILLER_DECK)
    deck2 = list(board.get("deck2") or DEFAULT_FILLER_DECK)
    seed = int(board.get("seed") or 0)
    game = RustDebugGame(deck1, deck2, seed)
    game.skip_mulligan()

    zones = board.get("zones") or {}
    for pid_str, zraw in zones.items():
        pid = int(pid_str)
        zspec = zraw if isinstance(zraw, DebugZoneSpec) else DebugZoneSpec(**zraw)
        _apply_zone(game, pid, zspec)

    _apply_scalar_state(
        game,
        memory=int(board.get("memory", 0)),
        phase=str(board.get("phase") or "Main"),
        turn=int(board.get("turn", 1)),
        first_player=int(board.get("first_player", 1)),
    )

    validation_error: Optional[str] = None
    try:
        game.validate()
    except Exception as exc:
        validation_error = str(exc)
    return game, validation_error


def _register_game(game_id: str, game: RustDebugGame, board: dict[str, Any]) -> None:
    active_games[game_id] = game
    _debug_events[game_id] = []
    game_meta[game_id] = GameMeta(
        p1_human=True,
        p2_human=True,
        last_log=[],
        last_events=[],
        player_labels={1: "Player 1", 2: "Player 2"},
        deck1=list(board.get("deck1") or DEFAULT_FILLER_DECK),
        deck2=list(board.get("deck2") or DEFAULT_FILLER_DECK),
        seed=int(board.get("seed") or 0),
    )


def _session_payload(sess: StudioSession) -> dict[str, Any]:
    game = active_games.get(sess.game_id)
    if not isinstance(game, RustDebugGame):
        raise HTTPException(status_code=404, detail="Studio game instance missing")
    mask = game.get_action_mask().tolist()
    has_explain = hasattr(game, "explain_action")
    legal_actions = [
        {
            "action_id": idx,
            "label": game.explain_action(idx) if has_explain else f"Action {idx}",
        }
        for idx, v in enumerate(mask)
        if v > 0.5
    ]
    return {
        "session_id": sess.session_id,
        "game_id": sess.game_id,
        "board": sess.base_board,
        "seats": sess.seats,
        "source": sess.source,
        "timeline": sess.timeline,
        "lines": list(sess.lines.values()),
        "validation_error": sess.validation_error,
        "state": game.to_ui_json(),
        "internal_state": game.internal_state(),
        "action_mask": mask,
        "legal_actions": legal_actions,
        "is_game_over": game.is_game_over,
        "current_player": game.current_player_id,
        "pending_selection": game.get_pending_selection(),
    }


def _require_session(session_id: str) -> tuple[StudioSession, RustDebugGame]:
    sess = _studio_sessions.get(session_id)
    if sess is None:
        raise HTTPException(status_code=404, detail="Studio session not found")
    game = active_games.get(sess.game_id)
    if not isinstance(game, RustDebugGame):
        raise HTTPException(status_code=404, detail="Studio game instance not found")
    return sess, game


def _resolve_model_path(model_id: str) -> Path:
    if not model_id or ".." in model_id or model_id.startswith(("/", "\\")):
        raise HTTPException(status_code=400, detail="Invalid model_id")
    mdir = _models_dir()
    candidate = _safe_under(mdir, model_id)
    if not candidate.exists() or candidate.suffix.lower() != ".onnx":
        raise HTTPException(status_code=404, detail=f"Model not found: {model_id}")
    return candidate


def _record_step(
    sess: StudioSession,
    game: RustDebugGame,
    action_id: int,
    controller: str,
    eval_snapshot: Optional[dict[str, Any]] = None,
) -> dict[str, Any]:
    if game.is_game_over:
        raise HTTPException(status_code=400, detail="Game is already over")
    mask = game.get_action_mask().tolist()
    if action_id < 0 or action_id >= len(mask) or mask[action_id] < 0.5:
        raise HTTPException(status_code=400, detail=f"Action {action_id} is not legal")

    before_internal = game.internal_state()
    actor = int(game.current_player_id)
    label = (
        game.explain_action(int(action_id))
        if hasattr(game, "explain_action")
        else f"Action {action_id}"
    )
    game.step(int(action_id))
    events = game.get_events_since_last_step()
    _debug_events.setdefault(sess.game_id, []).extend(events)
    after_internal = game.internal_state()

    entry: dict[str, Any] = {
        "step_index": len(sess.timeline),
        "action_id": int(action_id),
        "actor": actor,
        "controller": controller,
        "label": label,
        "phase_before": before_internal.get("current_phase", ""),
        "phase_after": after_internal.get("current_phase", ""),
        "memory_after": after_internal.get("memory", 0),
        "turn_after": after_internal.get("turn_count", 1),
        "events": events,
    }
    if eval_snapshot is not None:
        entry["eval"] = eval_snapshot
    sess.timeline.append(entry)
    return entry


# ── Session endpoints ─────────────────────────────────────────────────────────

@router.post("/sessions")
def create_studio_session(request: StudioSessionCreateRequest):
    if request.blank or request.board is None:
        board_model = StudioBoardPayload(
            deck1=list(DEFAULT_FILLER_DECK),
            deck2=list(DEFAULT_FILLER_DECK),
            seed=0,
            memory=0,
            phase="Main",
            turn=1,
            first_player=1,
            zones={
                "1": DebugZoneSpec(hand=[], field=[], security=[], trash=[], breeding=[]),
                "2": DebugZoneSpec(hand=[], field=[], security=[], trash=[], breeding=[]),
            },
        )
    else:
        board_model = request.board

    board_dict = board_model.model_dump()
    try:
        game, validation_error = _build_debug_game_from_board(board_dict)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Failed to stage board: {exc}")

    session_id = str(uuid4())
    game_id = session_id
    _register_game(game_id, game, board_dict)

    default_seats = {
        "1": SeatConfig(type="human").model_dump(),
        "2": SeatConfig(type="human").model_dump(),
    }
    if request.seats:
        for k, v in request.seats.items():
            if k in ("1", "2"):
                default_seats[k] = v.model_dump()

    sess = StudioSession(
        session_id=session_id,
        game_id=game_id,
        base_board=board_dict,
        seats=default_seats,
        source=dict(request.source or {"kind": "composed"}),
        validation_error=validation_error,
    )
    _studio_sessions[session_id] = sess
    return _session_payload(sess)


@router.get("/sessions/{session_id}")
def get_studio_session(session_id: str):
    sess, _ = _require_session(session_id)
    return _session_payload(sess)


@router.put("/sessions/{session_id}/board")
def update_studio_board(session_id: str, board: StudioBoardPayload):
    sess, _ = _require_session(session_id)
    board_dict = board.model_dump()
    try:
        game, validation_error = _build_debug_game_from_board(board_dict)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Board update failed: {exc}")
    sess.base_board = board_dict
    sess.timeline = []
    sess.validation_error = validation_error
    _register_game(sess.game_id, game, board_dict)
    return _session_payload(sess)


@router.post("/sessions/{session_id}/seats")
def update_studio_seats(session_id: str, request: StudioSeatsUpdateRequest):
    sess, _ = _require_session(session_id)
    for k, v in request.seats.items():
        if k in ("1", "2"):
            sess.seats[k] = v.model_dump()
    return _session_payload(sess)


@router.post("/sessions/{session_id}/step")
def studio_human_step(session_id: str, request: StudioStepRequest):
    sess, game = _require_session(session_id)
    step_entry = _record_step(sess, game, request.action_id, controller="human")
    payload = _session_payload(sess)
    payload["last_step"] = step_entry
    return payload


@router.post("/sessions/{session_id}/model-step")
def studio_model_step(session_id: str, expectations: Optional[list[dict[str, Any]]] = None):
    """Execute exactly ONE decision for the active seat's assigned pilot model /
    greedy / random controller, returning full policy & search valuation."""
    sess, game = _require_session(session_id)
    if game.is_game_over:
        raise HTTPException(status_code=400, detail="Game is already over")

    actor_key = str(game.current_player_id)
    seat = sess.seats.get(actor_key) or {"type": "human"}
    seat_type = seat.get("type", "human")
    if seat_type == "human":
        raise HTTPException(
            status_code=400,
            detail=f"Seat {actor_key} is set to human; assign a pilot model or bot to model-step",
        )

    eval_snapshot: Optional[dict[str, Any]] = None
    if seat_type == "greedy":
        action_id = int(game.greedy_action())
        controller = "greedy"
    elif seat_type == "random":
        mask = game.get_action_mask().tolist()
        legal = [i for i, v in enumerate(mask) if v > 0.5]
        if not legal:
            raise HTTPException(status_code=400, detail="No legal actions")
        action_id = random.choice(legal)
        controller = "random"
    elif seat_type == "model":
        model_id = seat.get("model_id")
        if not model_id:
            raise HTTPException(status_code=400, detail=f"Seat {actor_key} model_id is not set")
        model_path = _resolve_model_path(model_id)
        if not hasattr(digimon_engine, "inspect_position"):
            raise HTTPException(
                status_code=501,
                detail="digimon_engine.inspect_position is unavailable; rebuild wheel",
            )
        try:
            eval_snapshot = digimon_engine.inspect_position(
                game,
                str(model_path),
                simulations=int(seat.get("simulations", 0)),
                worlds=int(seat.get("worlds", 1)),
                seed=int(seat.get("seed", 0)) + len(sess.timeline),
                regime=str(seat.get("regime", "open_list")),
                top_k=12,
                expectations_json=json.dumps(expectations or []),
            )
        except Exception as exc:
            raise HTTPException(status_code=400, detail=f"Model evaluation failed: {exc}")
        action_id = int(eval_snapshot["chosen_action_id"])
        controller = f"model:{model_id}"
    else:
        raise HTTPException(status_code=400, detail=f"Unknown seat type {seat_type!r}")

    step_entry = _record_step(sess, game, action_id, controller=controller, eval_snapshot=eval_snapshot)
    payload = _session_payload(sess)
    payload["last_step"] = step_entry
    payload["eval"] = eval_snapshot
    return payload


@router.post("/sessions/{session_id}/rewind")
def studio_rewind(session_id: str, request: StudioRewindRequest):
    sess, _ = _require_session(session_id)
    target = min(request.target_step, len(sess.timeline))
    kept_steps = sess.timeline[:target]

    game, validation_error = _build_debug_game_from_board(sess.base_board)
    _register_game(sess.game_id, game, sess.base_board)
    sess.validation_error = validation_error

    replayed: list[dict[str, Any]] = []
    for old in kept_steps:
        aid = int(old["action_id"])
        game.step(aid)
        ev = game.get_events_since_last_step()
        _debug_events.setdefault(sess.game_id, []).extend(ev)
        replayed.append(old)
    sess.timeline = replayed
    return _session_payload(sess)


@router.post("/sessions/{session_id}/snapshot-as-start")
def studio_snapshot_as_start(session_id: str):
    sess, game = _require_session(session_id)
    snap = game.to_scenario()
    new_board = {
        "deck1": snap["decks"].get("1", sess.base_board.get("deck1", list(DEFAULT_FILLER_DECK))),
        "deck2": snap["decks"].get("2", sess.base_board.get("deck2", list(DEFAULT_FILLER_DECK))),
        "seed": int(sess.base_board.get("seed", 0)),
        "memory": int(snap["state"].get("memory", 0)),
        "phase": str(snap["state"].get("phase", "Main")),
        "turn": int(snap["state"].get("turn", 1)),
        "first_player": int(snap["state"].get("first_player", 1)),
        "zones": snap.get("zones", {}),
    }
    sess.base_board = new_board
    sess.timeline = []
    rebuilt, validation_error = _build_debug_game_from_board(new_board)
    sess.validation_error = validation_error
    _register_game(sess.game_id, rebuilt, new_board)
    return _session_payload(sess)


@router.post("/sessions/{session_id}/lines")
def studio_save_line(session_id: str, request: StudioLineSaveRequest):
    sess, _ = _require_session(session_id)
    line_id = _validate_slug(request.id or f"line-{len(sess.lines) + 1}", "line_id")
    actions = (
        list(request.actions)
        if request.actions is not None
        else [int(s["action_id"]) for s in sess.timeline]
    )
    line = {
        "id": line_id,
        "name": request.name.strip() or line_id,
        "role": request.role,
        "actions": actions,
    }
    sess.lines[line_id] = line
    return _session_payload(sess)


@router.delete("/sessions/{session_id}/lines/{line_id}")
def studio_delete_line(session_id: str, line_id: str):
    sess, _ = _require_session(session_id)
    sess.lines.pop(line_id, None)
    return _session_payload(sess)


@router.post("/sessions/{session_id}/lines/{line_id}/load")
def studio_load_line(session_id: str, line_id: str):
    sess, _ = _require_session(session_id)
    line = sess.lines.get(line_id)
    if line is None:
        raise HTTPException(status_code=404, detail="Line not found")
    game, validation_error = _build_debug_game_from_board(sess.base_board)
    _register_game(sess.game_id, game, sess.base_board)
    sess.validation_error = validation_error
    sess.timeline = []
    for aid in line.get("actions", []):
        _record_step(sess, game, int(aid), controller="line")
    return _session_payload(sess)


@router.post("/games/{session_id}/inspect-agent")
@router.post("/sessions/{session_id}/inspect-agent")
def studio_inspect_agent(session_id: str, request: StudioInspectRequest):
    """Single-position ONNX policy/value & optional MCTS inspection."""
    _, game = _require_session(session_id)
    model_path = _resolve_model_path(request.model_id)
    if not hasattr(digimon_engine, "inspect_position"):
        raise HTTPException(
            status_code=501,
            detail="digimon_engine.inspect_position is unavailable; rebuild wheel",
        )
    try:
        return digimon_engine.inspect_position(
            game,
            str(model_path),
            simulations=request.simulations,
            worlds=request.worlds,
            seed=request.seed,
            regime=request.regime,
            top_k=request.top_k,
            expectations_json=json.dumps(request.expectations),
        )
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Agent inspection failed: {exc}")


# ── Catalogs: deck templates, implemented cards, models ───────────────────────

@router.get("/deck-templates")
def list_deck_templates():
    templates: list[dict[str, Any]] = [
        {
            "id": "blank-filler",
            "name": "Blank / Filler Deck (ST1)",
            "source": "builtin",
            "cards": list(DEFAULT_FILLER_DECK),
        }
    ]
    if DECK_LIBRARY_PATH.exists():
        try:
            raw = json.loads(DECK_LIBRARY_PATH.read_text(encoding="utf-8"))
            archetypes = raw.get("archetypes", raw) if isinstance(raw, dict) else {}
            if isinstance(archetypes, dict):
                for arch_name, entry in sorted(archetypes.items()):
                    decklists = entry.get("decklists", []) if isinstance(entry, dict) else []
                    if not decklists:
                        continue
                    first = decklists[0]
                    cards = first.get("decklist") or first.get("cards") or []
                    if isinstance(cards, str):
                        try:
                            cards = json.loads(cards)
                        except Exception:
                            cards = []
                    if isinstance(cards, list) and cards:
                        templates.append(
                            {
                                "id": f"archetype:{arch_name}",
                                "name": arch_name,
                                "source": "deck_library",
                                "cards": [str(c) for c in cards],
                            }
                        )
        except Exception:
            pass
    return {"templates": templates}


@router.get("/implemented-cards")
def list_implemented_cards():
    return {"card_ids": sorted(load_implemented_card_ids())}


_CARD_CATALOG_CACHE: Optional[list[dict[str, Any]]] = None


@router.get("/cards")
def list_studio_cards():
    """Return full searchable card catalog (name, ID, colors, level, kind, DP,
    cost, traits, effect/inherited/security text, and Rust implementation flag)."""
    global _CARD_CATALOG_CACHE
    if _CARD_CATALOG_CACHE is None:
        impl = set(load_implemented_card_ids())
        raw = json.loads(CARDS_JSON_PATH.read_text(encoding="utf-8")) if CARDS_JSON_PATH.exists() else {}
        entries: list[dict[str, Any]] = []
        for cid, c in sorted(raw.items()):
            if not isinstance(c, dict):
                continue
            entries.append(
                {
                    "card_id": cid,
                    "name": c.get("card_name_eng") or cid,
                    "card_kind": int(c.get("card_kind", 0)),
                    "level": c.get("level"),
                    "colors": c.get("card_colors") or [],
                    "dp": c.get("dp"),
                    "play_cost": c.get("play_cost"),
                    "traits": c.get("type_eng") or [],
                    "effect_text": c.get("effect_description_eng") or "",
                    "inherited_text": c.get("inherited_effect_description_eng") or "",
                    "security_text": c.get("security_effect_description_eng") or "",
                    "implemented": cid in impl,
                }
            )
        _CARD_CATALOG_CACHE = entries
    return {"cards": _CARD_CATALOG_CACHE}


@router.get("/models")
def list_local_models():
    mdir = _models_dir()
    models: list[dict[str, Any]] = []
    if mdir.exists():
        for p in sorted(mdir.rglob("*.onnx")):
            rel = str(p.relative_to(mdir)).replace("\\", "/")
            models.append(
                {
                    "id": rel,
                    "name": p.stem,
                    "bytes": p.stat().st_size,
                }
            )
    return {"models": models}


# ── Studio Library (folders + tags across puzzles, exams, replays) ────────────

def _load_library_index() -> dict[str, Any]:
    if not LIBRARY_INDEX_PATH.exists():
        return {"version": 1, "folders": [], "items": {}}
    try:
        data = json.loads(LIBRARY_INDEX_PATH.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            return {"version": 1, "folders": [], "items": {}}
        data.setdefault("version", 1)
        data.setdefault("folders", [])
        data.setdefault("items", {})
        return data
    except Exception:
        return {"version": 1, "folders": [], "items": {}}


def _save_library_index(data: dict[str, Any]) -> None:
    PUZZLES_DIR.mkdir(parents=True, exist_ok=True)
    data["folders"] = sorted(set(f for f in data.get("folders", []) if f))
    LIBRARY_INDEX_PATH.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def _find_puzzle_file(puzzle_id: str) -> Optional[Path]:
    _validate_slug(puzzle_id, "puzzle_id")
    if not PUZZLES_DIR.exists():
        return None
    for p in PUZZLES_DIR.rglob(f"{puzzle_id}.json"):
        if p.name == f"{puzzle_id}.json" and p.name != "_library.json":
            return p
    return None


@router.get("/library")
async def get_studio_library():
    index = _load_library_index()
    meta_items: dict[str, dict[str, Any]] = index.get("items", {})
    folders: set[str] = set(index.get("folders", []))
    items: list[dict[str, Any]] = []

    # 1. Studio puzzles (qa/puzzles/**/*.json)
    if PUZZLES_DIR.exists():
        for p in sorted(PUZZLES_DIR.rglob("*.json")):
            if p.name == "_library.json":
                continue
            try:
                doc = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            rel_folder = str(p.parent.relative_to(PUZZLES_DIR)).replace("\\", "/")
            if rel_folder == ".":
                rel_folder = ""
            if rel_folder:
                folders.add(rel_folder)
            pid = str(doc.get("id") or p.stem)
            key = f"puzzle:{pid}"
            m = meta_items.get(key, {})
            tags = sorted(set(doc.get("tags", [])) | set(m.get("tags", [])))
            items.append(
                {
                    "item_key": key,
                    "kind": "puzzle",
                    "id": pid,
                    "title": m.get("title_override") or doc.get("title") or pid,
                    "folder": rel_folder,
                    "tags": tags,
                    "ref": str(p.relative_to(REPO_ROOT)).replace("\\", "/"),
                    "seat": doc.get("seat", 1),
                    "lines_count": len(doc.get("lines", [])),
                    "expectations_count": len(doc.get("expectations", [])),
                    "rl_goal_mode": (doc.get("rl_goal") or {}).get("mode", "playout_to_win"),
                }
            )

    # 2. Search exams (qa/search-exams/*.json)
    if SEARCH_EXAMS_DIR.exists():
        for p in sorted(SEARCH_EXAMS_DIR.glob("*.json")):
            try:
                doc = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            eid = str(doc.get("id") or p.stem)
            key = f"search_exam:{eid}"
            m = meta_items.get(key, {})
            folder = _validate_folder(m.get("folder")) if m.get("folder") else ""
            if folder:
                folders.add(folder)
            items.append(
                {
                    "item_key": key,
                    "kind": "search_exam",
                    "id": eid,
                    "title": m.get("title_override") or doc.get("title") or eid,
                    "folder": folder,
                    "tags": sorted(set(m.get("tags", []))),
                    "ref": str(p.relative_to(REPO_ROOT)).replace("\\", "/"),
                }
            )

    # 3. Scenario fixtures (qa/scenarios/*.json)
    if SCENARIOS_DIR.exists():
        for p in sorted(SCENARIOS_DIR.glob("*.json")):
            try:
                doc = json.loads(p.read_text(encoding="utf-8"))
            except Exception:
                continue
            sid = str(doc.get("id") or p.stem)
            key = f"scenario:{sid}"
            m = meta_items.get(key, {})
            folder = _validate_folder(m.get("folder")) if m.get("folder") else ""
            if folder:
                folders.add(folder)
            items.append(
                {
                    "item_key": key,
                    "kind": "scenario",
                    "id": sid,
                    "title": m.get("title_override") or doc.get("title") or sid,
                    "folder": folder,
                    "tags": sorted(set(m.get("tags", []))),
                    "ref": str(p.relative_to(REPO_ROOT)).replace("\\", "/"),
                }
            )

    # 4. Local DCGO JSONL recordings (repo + live MelonLoader & private roots)
    dcgo_entries: list[tuple[str, int, dict[str, Any]]] = []
    pending_bg_scan: list[tuple[str, Path, Optional[int], int, int]] = []
    dcgo_total_count = 0
    dcgo_analyzed_count = 0

    for root_id, root_path in _dcgo_recording_roots().items():
        if not root_path.exists():
            continue
        source_tags = [] if root_id == "repo" else [root_id.replace("_", "-")]
        for p in root_path.rglob("*.jsonl"):
            try:
                st = p.stat()
            except OSError:
                continue
            rel = str(p.relative_to(root_path)).replace("\\", "/")
            base_ref = rel if root_id == "repo" else f"{root_id}:{rel}"
            mtime_key = f"{st.st_mtime:020.3f}"

            if p.name.startswith("raw-"):
                games = _scan_raw_melon_games(p)
                for g in games:
                    dcgo_total_count += 1
                    gidx = int(g["game_index"])
                    ref = f"{base_ref}#g{gidx}"
                    key = f"dcgo_jsonl:{ref}"
                    m = meta_items.get(key, {})
                    folder = _validate_folder(m.get("folder")) if m.get("folder") else ""
                    if folder:
                        folders.add(folder)

                    my_deck = g.get("my_deck")
                    opp_deck = g.get("opp_deck")
                    turns = int(g.get("turns") or 0)
                    default_title = _build_matchup_title(
                        g.get("start_ts"),
                        my_deck,
                        opp_deck,
                        p.stem,
                        game_index=gidx,
                        turns=turns,
                        ended=bool(g.get("ended")),
                    )

                    cached_div = _get_cached_analysis(ref, st.st_mtime_ns, st.st_size)
                    div_status: Optional[str] = None
                    if cached_div is not None:
                        dcgo_analyzed_count += 1
                        div_status = str(cached_div.get("divergence_status") or "")
                    else:
                        pending_bg_scan.append((ref, p, gidx, st.st_mtime_ns, st.st_size))

                    auto_tags: list[str] = list(source_tags)
                    finish_type = str(g.get("finish_type") or "unfinished")
                    if finish_type == "natural":
                        auto_tags.append("no-concede")
                    elif finish_type == "concede":
                        auto_tags.append("concede")
                    else:
                        auto_tags.append("unfinished")
                    if div_status in ("diverged", "clean", "error"):
                        auto_tags.append(div_status)
                    for dname in (my_deck, opp_deck):
                        slug = _slugify_tag(dname)
                        if slug:
                            auto_tags.append(slug)
                    auto_tags = sorted(set(auto_tags))
                    user_tags = sorted(set(m.get("tags", [])))
                    merged_tags = sorted(set(auto_tags) | set(user_tags))

                    sort_ts = str(g.get("start_ts") or "") or mtime_key
                    dcgo_entries.append(
                        (
                            sort_ts,
                            gidx,
                            {
                                "item_key": key,
                                "kind": "dcgo_jsonl",
                                "id": ref,
                                "title": m.get("title_override") or default_title,
                                "folder": folder,
                                "tags": merged_tags,
                                "auto_tags": auto_tags,
                                "user_tags": user_tags,
                                "ref": ref,
                                "source_root": root_id,
                                "my_deck": my_deck,
                                "opp_deck": opp_deck,
                                "finish_type": finish_type,
                                "divergence_status": div_status,
                                "turns": turns,
                                "bytes": st.st_size,
                            },
                        )
                    )
            else:
                dcgo_total_count += 1
                ref = base_ref
                key = f"dcgo_jsonl:{ref}"
                m = meta_items.get(key, {})
                folder = _validate_folder(m.get("folder")) if m.get("folder") else ""
                if folder:
                    folders.add(folder)

                v1_meta = _scan_v1_dcgo_meta(p)
                my_deck = v1_meta.get("my_deck")
                opp_deck = v1_meta.get("opp_deck")
                default_title = (
                    _build_matchup_title(v1_meta.get("start_ts"), my_deck, opp_deck, p.stem)
                    if (my_deck or opp_deck)
                    else p.name
                )

                cached_div = _get_cached_analysis(ref, st.st_mtime_ns, st.st_size)
                div_status = None
                if cached_div is not None:
                    dcgo_analyzed_count += 1
                    div_status = str(cached_div.get("divergence_status") or "")
                else:
                    pending_bg_scan.append((ref, p, None, st.st_mtime_ns, st.st_size))

                auto_tags = list(source_tags)
                finish_type = str(v1_meta.get("finish_type") or "unfinished")
                if finish_type == "natural":
                    auto_tags.append("no-concede")
                elif finish_type == "concede":
                    auto_tags.append("concede")
                else:
                    auto_tags.append("unfinished")
                if div_status in ("diverged", "clean", "error"):
                    auto_tags.append(div_status)
                for dname in (my_deck, opp_deck):
                    slug = _slugify_tag(dname)
                    if slug:
                        auto_tags.append(slug)
                auto_tags = sorted(set(auto_tags))
                user_tags = sorted(set(m.get("tags", [])))
                merged_tags = sorted(set(auto_tags) | set(user_tags))

                sort_ts = str(v1_meta.get("start_ts") or "") or mtime_key
                dcgo_entries.append(
                    (
                        sort_ts,
                        0,
                        {
                            "item_key": key,
                            "kind": "dcgo_jsonl",
                            "id": ref,
                            "title": m.get("title_override") or default_title,
                            "folder": folder,
                            "tags": merged_tags,
                            "auto_tags": auto_tags,
                            "user_tags": user_tags,
                            "ref": ref,
                            "source_root": root_id,
                            "my_deck": my_deck,
                            "opp_deck": opp_deck,
                            "finish_type": finish_type,
                            "divergence_status": div_status,
                            "bytes": st.st_size,
                        },
                    )
                )

    dcgo_entries.sort(key=lambda tup: (tup[0], tup[1]), reverse=True)
    for _, _, entry_item in dcgo_entries:
        items.append(entry_item)

    if pending_bg_scan and os.environ.get("DIGIMON_STUDIO_AUTO_SCAN", "1") != "0":
        # Prioritize dcgo_live and repo newest-first
        order_refs = [entry_item["ref"] for _, _, entry_item in dcgo_entries]
        order_map = {r: idx for idx, r in enumerate(order_refs)}
        pending_bg_scan.sort(key=lambda t: order_map.get(t[0], 999999))
        _schedule_background_divergence_scan(pending_bg_scan)

    # 5. DB recordings (optional when running with hosted DB stack)
    if _async_session_factory is not None and _sa_select is not None and _GameRecording is not None:
        try:
            async with _async_session_factory() as db:
                res = await db.execute(
                    _sa_select(_GameRecording).order_by(_GameRecording.created_at.desc()).limit(100)
                )
                for rec in res.scalars().all():
                    key = f"recording:{rec.id}"
                    m = meta_items.get(key, {})
                    folder = _validate_folder(m.get("folder")) if m.get("folder") else ""
                    if folder:
                        folders.add(folder)
                    items.append(
                        {
                            "item_key": key,
                            "kind": "recording",
                            "id": str(rec.id),
                            "title": m.get("title_override") or f"Recording {str(rec.id)[:8]} ({rec.total_steps} steps)",
                            "folder": folder,
                            "tags": sorted(set(m.get("tags", []))),
                            "ref": str(rec.id),
                            "total_steps": rec.total_steps,
                        }
                    )
        except Exception:
            pass

    return {
        "folders": sorted(folders),
        "items": items,
        "analysis_status": {
            "analyzed": dcgo_analyzed_count,
            "total": dcgo_total_count,
            "running": _BG_SCAN_RUNNING,
        },
    }


@router.post("/library/folders")
def create_library_folder(request: LibraryFolderCreateRequest):
    folder = _validate_folder(request.folder)
    if not folder:
        raise HTTPException(status_code=400, detail="Folder path cannot be empty")
    index = _load_library_index()
    if folder not in index["folders"]:
        index["folders"].append(folder)
    _save_library_index(index)
    target = _safe_under(PUZZLES_DIR, folder)
    target.mkdir(parents=True, exist_ok=True)
    return {"folder": folder, "folders": index["folders"]}


@router.patch("/library/folders")
def rename_library_folder(request: LibraryFolderRenameRequest):
    old_f = _validate_folder(request.old_folder)
    new_f = _validate_folder(request.new_folder)
    if not old_f or not new_f:
        raise HTTPException(status_code=400, detail="Both old_folder and new_folder are required")
    index = _load_library_index()
    index["folders"] = [
        new_f if f == old_f else (new_f + f[len(old_f):] if f.startswith(old_f + "/") else f)
        for f in index.get("folders", [])
    ]
    for meta in index.get("items", {}).values():
        mf = meta.get("folder", "")
        if mf == old_f:
            meta["folder"] = new_f
        elif mf.startswith(old_f + "/"):
            meta["folder"] = new_f + mf[len(old_f):]
    old_dir = _safe_under(PUZZLES_DIR, old_f)
    new_dir = _safe_under(PUZZLES_DIR, new_f)
    if old_dir.exists() and old_dir.is_dir():
        new_dir.parent.mkdir(parents=True, exist_ok=True)
        old_dir.rename(new_dir)
    _save_library_index(index)
    return {"folders": index["folders"]}


@router.delete("/library/folders")
def delete_library_folder(folder: str = Query(...)):
    target_f = _validate_folder(folder)
    if not target_f:
        raise HTTPException(status_code=400, detail="Folder path cannot be empty")
    index = _load_library_index()
    index["folders"] = [
        f for f in index.get("folders", []) if f != target_f and not f.startswith(target_f + "/")
    ]
    for meta in index.get("items", {}).values():
        mf = meta.get("folder", "")
        if mf == target_f or mf.startswith(target_f + "/"):
            meta["folder"] = ""
    _save_library_index(index)
    return {"folders": index["folders"]}


@router.patch("/library/items")
def update_library_item(request: LibraryItemUpdateRequest):
    index = _load_library_index()
    key = request.item_key.strip()
    if ":" not in key:
        raise HTTPException(status_code=400, detail="item_key must be '<kind>:<id>'")
    kind, item_id = key.split(":", 1)

    folder = _validate_folder(request.folder) if request.folder is not None else None
    tags = (
        sorted(set(t.strip().lower() for t in request.tags if t.strip()))
        if request.tags is not None
        else None
    )

    entry = index["items"].setdefault(key, {})
    if folder is not None:
        entry["folder"] = folder
        if folder and folder not in index["folders"]:
            index["folders"].append(folder)
    if tags is not None:
        entry["tags"] = tags
    if request.title_override is not None:
        entry["title_override"] = request.title_override.strip()

    # If this is a Studio-owned puzzle, relocate its JSON file into the folder on disk.
    if kind == "puzzle":
        pfile = _find_puzzle_file(item_id)
        if pfile is not None:
            doc = json.loads(pfile.read_text(encoding="utf-8"))
            if tags is not None:
                doc["tags"] = tags
            if folder is not None:
                doc["folder"] = folder
                dest_dir = _safe_under(PUZZLES_DIR, folder) if folder else PUZZLES_DIR
                dest_dir.mkdir(parents=True, exist_ok=True)
                dest_file = dest_dir / f"{item_id}.json"
                dest_file.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
                if dest_file.resolve() != pfile.resolve():
                    pfile.unlink(missing_ok=True)
            else:
                pfile.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")

    _save_library_index(index)
    # Keep any open replay session with this item_key in sync
    for rep_entry in _studio_replays.values():
        if rep_entry.item_key == key:
            if tags is not None:
                rep_entry.user_tags = list(tags)
            if request.title_override is not None and request.title_override.strip():
                rep_entry.title = request.title_override.strip()
    return {"item_key": key, "meta": entry}


# ── Replay Loader & Forking ───────────────────────────────────────────────────

def _is_raw_melon_jsonl(text: str) -> bool:
    for line in text.splitlines():
        s = line.strip()
        if not s:
            continue
        try:
            obj = json.loads(s)
        except Exception:
            return False
        return "method" in obj and obj.get("type") != "game_start"
    return False


def _get_convert_raw_inprocess() -> tuple[Any, Any]:
    global _CONVERT_RAW_MOD, _CONVERT_RAW_CARDS
    if _CONVERT_RAW_MOD is not None and _CONVERT_RAW_CARDS is not None:
        return _CONVERT_RAW_MOD, _CONVERT_RAW_CARDS
    if not CONVERT_RAW_SCRIPT.exists():
        return None, None
    try:
        mod_dir = str(CONVERT_RAW_SCRIPT.parent)
        if mod_dir not in sys.path:
            sys.path.insert(0, mod_dir)
        spec = importlib.util.spec_from_file_location("_dcgo_convert_raw", str(CONVERT_RAW_SCRIPT))
        if spec is None or spec.loader is None:
            return None, None
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        cards = mod.load_cards(str(CARDS_JSON_PATH))
        _CONVERT_RAW_MOD = mod
        _CONVERT_RAW_CARDS = cards
        return mod, cards
    except Exception:
        return None, None


def _convert_raw_melon_jsonl(raw_text: str) -> tuple[str, Optional[str]]:
    if not CONVERT_RAW_SCRIPT.exists():
        raise HTTPException(status_code=500, detail="convert_raw.py not found in repository")

    # Fast in-process path (caches cards.json; ~15ms vs ~600ms subprocess)
    with _CONVERT_RAW_LOCK:
        mod, cards = _get_convert_raw_inprocess()
        if mod is not None and cards is not None:
            try:
                rows = [json.loads(ln) for ln in raw_text.splitlines() if ln.strip()]
                out_rows, _stats = mod.convert(rows, cards, allow_truncated=True)
                converted = "\n".join(json.dumps(r, separators=(",", ":")) for r in out_rows) + "\n"
                trunc = getattr(mod, "LAST_TRUNCATION", None)
                return converted, str(trunc) if trunc else None
            except SystemExit as exc:
                raise HTTPException(status_code=400, detail=f"Failed to convert raw MelonLoader JSONL: {exc}")
            except Exception:
                pass  # fall through to subprocess fallback

    with tempfile.TemporaryDirectory() as tmp:
        raw_path = Path(tmp) / "raw.jsonl"
        out_path = Path(tmp) / "converted.jsonl"
        raw_path.write_text(raw_text, encoding="utf-8")
        cmd = [
            sys.executable,
            str(CONVERT_RAW_SCRIPT),
            str(raw_path),
            str(out_path),
            "--allow-truncated",
            "--cards",
            str(CARDS_JSON_PATH),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        if proc.returncode != 0 or not out_path.exists():
            raise HTTPException(
                status_code=400,
                detail=f"Failed to convert raw MelonLoader JSONL: {proc.stderr or proc.stdout}",
            )
        converted = out_path.read_text(encoding="utf-8")
        trunc_warn = None
        combined = (proc.stdout + "\n" + proc.stderr).strip()
        if "truncat" in combined.lower():
            trunc_warn = combined
        return converted, trunc_warn


def _verify_dcgo_v1_divergence(v1_text: str) -> dict[str, Any]:
    """Step a schema-v1 DCGO JSONL replay in Rust with verification enabled to
    determine whether it diverges in our simulator."""
    if not hasattr(digimon_engine, "RustReplaySession"):
        return {"divergence_status": "error", "error": "RustReplaySession unavailable"}
    try:
        runner = digimon_engine.RustReplaySession.from_dcgo_jsonl(v1_text, verify=True)
        limit = int(runner.total_steps) + 50
        steps = 0
        while not runner.is_complete and not runner.is_paused and steps < limit:
            before = runner.current_step
            runner.step()
            steps += 1
            if runner.current_step == before:
                break
        divs = runner.get_divergences() or []
        diverged = len(divs) > 0 or bool(runner.is_paused)
        return {
            "divergence_status": "diverged" if diverged else "clean",
            "divergence_count": len(divs),
            "first_divergence_step": int(runner.current_step) if diverged else None,
            "total_steps": int(runner.total_steps),
        }
    except Exception as exc:
        return {"divergence_status": "error", "error": str(exc)[:240]}


def _schedule_background_divergence_scan(
    targets: list[tuple[str, Path, Optional[int], int, int]],
) -> None:
    global _BG_SCAN_RUNNING
    with _BG_SCAN_LOCK:
        if _BG_SCAN_RUNNING or not targets:
            return
        _BG_SCAN_RUNNING = True

    def _worker() -> None:
        global _BG_SCAN_RUNNING
        dirty = 0
        try:
            for ref, path, game_index, mtime_ns, size in targets:
                if _get_cached_analysis(ref, mtime_ns, size) is not None:
                    continue
                try:
                    if game_index is not None:
                        text = _extract_raw_game_slice(path, game_index)
                    else:
                        text = path.read_text(encoding="utf-8", errors="replace")
                    if _is_raw_melon_jsonl(text):
                        text, _ = _convert_raw_melon_jsonl(text)
                    text, _ = _ensure_dcgo_game_end(text)
                    result = _verify_dcgo_v1_divergence(text)
                except Exception as exc:
                    result = {"divergence_status": "error", "error": str(exc)[:240]}
                _set_cached_analysis(ref, mtime_ns, size, result, flush=False)
                dirty += 1
                if dirty >= 5:
                    _save_analysis_cache()
                    dirty = 0
            if dirty > 0:
                _save_analysis_cache()
        finally:
            with _BG_SCAN_LOCK:
                _BG_SCAN_RUNNING = False

    t = threading.Thread(target=_worker, name="studio-divergence-scan", daemon=True)
    t.start()


def _ensure_dcgo_game_end(text: str) -> tuple[str, Optional[str]]:
    lines = [ln for ln in text.splitlines() if ln.strip()]
    if not lines:
        return text, None
    try:
        last = json.loads(lines[-1])
    except Exception:
        return text, None
    if last.get("type") == "game_end":
        return text, None
    synthetic = json.dumps(
        {
            "type": "game_end",
            "winner": -1,
            "total_steps": max(0, len(lines) - 1),
            "turn_count": 0,
            "reason": "truncated",
        }
    )
    return "\n".join(lines + [synthetic]) + "\n", "DCGO recording truncated (no game_end row); padded synthetic game_end for stepping."


def _describe_replay_action(action_id: int, phase_before: str) -> str:
    if phase_before.startswith("Select") or phase_before in ("BlockTiming", "CounterTiming", "AllianceTiming"):
        if action_id == 62:
            return "Decline / Pass Selection (62)"
        return f"Select option ({action_id})"
    if action_id == 60:
        return "Hatch Digi-Egg (60)"
    if action_id == 61:
        return "Move from Breeding (61)"
    if action_id == 62:
        return f"Pass {phase_before or 'Phase'} (62)"
    if action_id == 93:
        return "Concede (93)"
    if 0 <= action_id < 30:
        return f"Play Hand[{action_id}]"
    if 30 <= action_id < 60:
        return f"Hand Effect / Select[{action_id - 30}]"
    if 63 <= action_id < 93:
        return f"DNA Digivolve Hand[{action_id - 63}]"
    if 100 <= action_id < 400:
        slot = (action_id - 100) // 15
        tgt = (action_id - 100) % 15
        return f"Attack Slot[{slot}] -> {'Security' if tgt == 14 else f'Opp[{tgt}]'}"
    if 400 <= action_id < 1000:
        hand_idx = (action_id - 400) // 15
        slot = (action_id - 400) % 15
        return f"Digivolve Hand[{hand_idx}] -> {'Breeding' if slot == 14 else f'Slot[{slot}]'}"
    if 1000 <= action_id < 1150:
        return f"Field Effect ({action_id})"
    if 1150 <= action_id < 1195:
        return f"Trash Effect ({action_id})"
    if action_id >= 2000:
        return f"Source Select ({action_id})"
    return f"Action {action_id}"


def _append_replay_log(entry: StudioReplayEntry, step_res: dict[str, Any]) -> None:
    pb = str(step_res.get("phase_before", ""))
    aid = int(step_res.get("action_id", 0))
    entry.step_log.append(
        {
            "step_number": int(step_res.get("step_number", len(entry.step_log) + 1)),
            "player_id": step_res.get("player_id"),
            "action_id": aid,
            "label": _describe_replay_action(aid, pb),
            "phase_before": pb,
            "phase_after": str(step_res.get("phase_after", "")),
            "memory_before": int(step_res.get("memory_before", 0)),
            "memory_after": int(step_res.get("memory_after", 0)),
            "turn_number": int(step_res.get("turn_number", 1)),
        }
    )


def _replay_payload(entry: StudioReplayEntry, step_result: Optional[dict[str, Any]] = None) -> dict[str, Any]:
    r = entry.runner
    merged_tags = sorted(set(entry.auto_tags) | set(entry.user_tags))
    return {
        "replay_id": entry.replay_id,
        "title": entry.title,
        "item_key": entry.item_key,
        "my_deck": entry.my_deck,
        "opp_deck": entry.opp_deck,
        "divergence_status": entry.divergence_status,
        "auto_tags": list(entry.auto_tags),
        "user_tags": list(entry.user_tags),
        "tags": merged_tags,
        "source": entry.source,
        "total_steps": r.total_steps,
        "current_step": r.current_step,
        "is_complete": r.is_complete,
        "is_game_over": r.is_game_over,
        "is_paused": r.is_paused,
        "winner_id": r.winner_id,
        "skipped_incomplete_plays": r.skipped_incomplete_plays,
        "has_opaque_placeholders": r.has_opaque_placeholders,
        "opaque_placeholder_zones": r.opaque_placeholder_zones(),
        "truncation_warning": entry.truncation_warning,
        "divergences": r.get_divergences(),
        "step_log": entry.step_log,
        "state": r.get_state(),
        "step_result": step_result,
    }


@router.post("/replays/load")
async def studio_load_replay(request: StudioReplayLoadRequest):
    if not hasattr(digimon_engine, "RustReplaySession"):
        raise HTTPException(
            status_code=501,
            detail="digimon_engine.RustReplaySession is unavailable; rebuild wheel",
        )
    trunc_warn: Optional[str] = None
    source_meta: dict[str, Any] = {"kind": request.source, "ref": request.ref}
    default_title: Optional[str] = None
    my_deck: Optional[str] = None
    opp_deck: Optional[str] = None
    divergence_status: Optional[str] = None
    auto_tags: list[str] = []

    if request.source == "recording_id":
        if not request.ref:
            raise HTTPException(status_code=400, detail="ref (recording_id) is required")
        if _async_session_factory is None or _sa_select is None or _GameRecording is None:
            raise HTTPException(status_code=503, detail="Database backend unavailable")
        async with _async_session_factory() as db:
            res = await db.execute(_sa_select(_GameRecording).where(_GameRecording.id == request.ref))
            rec = res.scalar_one_or_none()
        if rec is None:
            raise HTTPException(status_code=404, detail="Recording not found")
        runner = digimon_engine.RustReplaySession.from_native_json(
            rec.recording_json, verify=request.verify
        )
    elif request.source == "native_json":
        if not request.content:
            raise HTTPException(status_code=400, detail="content is required for native_json")
        runner = digimon_engine.RustReplaySession.from_native_json(
            request.content, verify=request.verify
        )
    elif request.source == "watch_queue":
        if not request.content:
            raise HTTPException(
                status_code=400,
                detail="content (watch game JSON payload) is required for watch_queue",
            )
        runner = digimon_engine.RustReplaySession.from_native_json(
            request.content, verify=request.verify
        )
    elif request.source == "dcgo_jsonl":
        text = request.content
        resolved_path: Optional[Path] = None
        root_id = "repo"
        if not text and request.ref:
            p, game_index, root_id = _resolve_dcgo_ref(request.ref)
            if not p.exists():
                raise HTTPException(status_code=404, detail=f"DCGO recording not found: {request.ref}")
            resolved_path = p
            if root_id != "repo":
                auto_tags.append(root_id.replace("_", "-"))
            if game_index is not None:
                text = _extract_raw_game_slice(p, game_index)
                matched = next(
                    (g for g in _scan_raw_melon_games(p) if g["game_index"] == game_index),
                    None,
                )
                if matched:
                    my_deck = matched.get("my_deck")
                    opp_deck = matched.get("opp_deck")
                    ft = str(matched.get("finish_type") or "unfinished")
                    auto_tags.append("no-concede" if ft == "natural" else ft)
                    default_title = _build_matchup_title(
                        matched.get("start_ts"),
                        my_deck,
                        opp_deck,
                        p.stem,
                        game_index=game_index,
                        turns=int(matched.get("turns") or 0),
                        ended=bool(matched.get("ended")),
                    )
                else:
                    default_title = f"{p.stem} · #{game_index}"
            else:
                text = p.read_text(encoding="utf-8", errors="replace")
                v1_meta = _scan_v1_dcgo_meta(p)
                my_deck = v1_meta.get("my_deck")
                opp_deck = v1_meta.get("opp_deck")
                ft = str(v1_meta.get("finish_type") or "unfinished")
                auto_tags.append("no-concede" if ft == "natural" else ft)
                default_title = (
                    _build_matchup_title(v1_meta.get("start_ts"), my_deck, opp_deck, p.stem)
                    if (my_deck or opp_deck)
                    else p.name
                )
        if not text:
            raise HTTPException(status_code=400, detail="Provide ref or content for dcgo_jsonl")
        if _is_raw_melon_jsonl(text):
            text, trunc_warn = _convert_raw_melon_jsonl(text)
        text, pad_warn = _ensure_dcgo_game_end(text)
        if pad_warn:
            trunc_warn = f"{trunc_warn} | {pad_warn}" if trunc_warn else pad_warn
        try:
            runner = digimon_engine.RustReplaySession.from_dcgo_jsonl(text, verify=request.verify)
        except Exception as exc:
            raise HTTPException(status_code=400, detail=f"Failed to load DCGO JSONL: {exc}")

        # Verify divergence status on load (uses cache if already verified)
        if request.ref and resolved_path is not None:
            try:
                st = resolved_path.stat()
                cached_div = _get_cached_analysis(request.ref, st.st_mtime_ns, st.st_size)
                if cached_div is None:
                    cached_div = _verify_dcgo_v1_divergence(text)
                    _set_cached_analysis(request.ref, st.st_mtime_ns, st.st_size, cached_div, flush=True)
                divergence_status = str(cached_div.get("divergence_status") or "")
            except Exception:
                pass
        else:
            res_div = _verify_dcgo_v1_divergence(text)
            divergence_status = str(res_div.get("divergence_status") or "")

        if divergence_status in ("diverged", "clean", "error"):
            auto_tags.append(divergence_status)
        for dname in (my_deck, opp_deck):
            slug = _slugify_tag(dname)
            if slug:
                auto_tags.append(slug)
        auto_tags = sorted(set(auto_tags))
    else:
        raise HTTPException(status_code=400, detail=f"Unsupported replay source {request.source!r}")

    replay_id = str(uuid4())
    item_key: Optional[str] = None
    if request.source == "dcgo_jsonl" and request.ref:
        item_key = f"dcgo_jsonl:{request.ref}"
    elif request.source == "recording_id" and request.ref:
        item_key = f"recording:{request.ref}"
    title = (
        default_title
        if request.source == "dcgo_jsonl" and default_title
        else (request.ref or f"{request.source} replay")
    )
    user_tags: list[str] = []
    if item_key:
        idx = _load_library_index()
        meta = idx.get("items", {}).get(item_key, {})
        if meta.get("title_override"):
            title = str(meta["title_override"])
        if isinstance(meta.get("tags"), list):
            user_tags = sorted(set(str(t) for t in meta["tags"] if str(t).strip()))
    entry = StudioReplayEntry(
        replay_id=replay_id,
        runner=runner,
        source=source_meta,
        title=title,
        item_key=item_key,
        truncation_warning=trunc_warn,
        my_deck=my_deck,
        opp_deck=opp_deck,
        divergence_status=divergence_status,
        auto_tags=auto_tags,
        user_tags=user_tags,
    )
    _studio_replays[replay_id] = entry
    return _replay_payload(entry)


@router.post("/replays/{replay_id}/step")
def studio_replay_step(replay_id: str):
    entry = _studio_replays.get(replay_id)
    if entry is None:
        raise HTTPException(status_code=404, detail="Replay session not found")
    before = entry.runner.current_step
    step_res = entry.runner.step()
    if entry.runner.current_step > before:
        _append_replay_log(entry, step_res)
    return _replay_payload(entry, step_result=step_res)


@router.post("/replays/{replay_id}/step-back")
def studio_replay_step_back(replay_id: str):
    entry = _studio_replays.get(replay_id)
    if entry is None:
        raise HTTPException(status_code=404, detail="Replay session not found")
    try:
        entry.runner.step_back()
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    entry.step_log = entry.step_log[: entry.runner.current_step]
    return _replay_payload(entry)


@router.post("/replays/{replay_id}/seek")
def studio_replay_seek(replay_id: str, request: StudioReplaySeekRequest):
    entry = _studio_replays.get(replay_id)
    if entry is None:
        raise HTTPException(status_code=404, detail="Replay session not found")
    target = min(request.step, int(entry.runner.total_steps))
    try:
        if target < entry.runner.current_step:
            entry.runner.seek(target)
            entry.step_log = entry.step_log[: entry.runner.current_step]
        else:
            while entry.runner.current_step < target and not entry.runner.is_complete and not entry.runner.is_paused:
                before = entry.runner.current_step
                res = entry.runner.step()
                if entry.runner.current_step > before:
                    _append_replay_log(entry, res)
                else:
                    break
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return _replay_payload(entry)


@router.post("/replays/{replay_id}/fork")
def studio_replay_fork(replay_id: str):
    entry = _studio_replays.get(replay_id)
    if entry is None:
        raise HTTPException(status_code=404, detail="Replay session not found")
    r = entry.runner
    snap = r.to_scenario()
    opaque_zones = r.opaque_placeholder_zones()
    opaque_warning: Optional[str] = None
    if opaque_zones:
        opaque_warning = (
            "Forked from an opaque PvP recording. Unrevealed opponent cards in "
            f"{opaque_zones} were materialized as editable cards — inspect and fill "
            "hidden zones before saving."
        )

    board_dict = {
        "deck1": snap["decks"].get("1") or list(DEFAULT_FILLER_DECK),
        "deck2": snap["decks"].get("2") or list(DEFAULT_FILLER_DECK),
        "seed": 0,
        "memory": int(snap["state"].get("memory", 0)),
        "phase": str(snap["state"].get("phase", "Main")),
        "turn": int(snap["state"].get("turn", 1)),
        "first_player": int(snap["state"].get("first_player", 1)),
        "zones": snap.get("zones", {}),
    }
    game, validation_error = _build_debug_game_from_board(board_dict)
    session_id = str(uuid4())
    _register_game(session_id, game, board_dict)
    sess = StudioSession(
        session_id=session_id,
        game_id=session_id,
        base_board=board_dict,
        seats={
            "1": SeatConfig(type="human").model_dump(),
            "2": SeatConfig(type="human").model_dump(),
        },
        source={**entry.source, "step": int(r.current_step)},
        validation_error=validation_error,
    )
    _studio_sessions[session_id] = sess
    payload = _session_payload(sess)
    payload["opaque_warning"] = opaque_warning
    return payload


# ── Puzzle CRUD & Exporters ───────────────────────────────────────────────────

@router.get("/puzzles")
def list_studio_puzzles():
    puzzles: list[dict[str, Any]] = []
    if PUZZLES_DIR.exists():
        for p in sorted(PUZZLES_DIR.rglob("*.json")):
            if p.name == "_library.json":
                continue
            try:
                doc = json.loads(p.read_text(encoding="utf-8"))
                puzzles.append(doc)
            except Exception:
                continue
    return {"puzzles": puzzles}


@router.get("/puzzles/{puzzle_id}")
def get_studio_puzzle(puzzle_id: str):
    pfile = _find_puzzle_file(puzzle_id)
    if pfile is None:
        raise HTTPException(status_code=404, detail=f"Puzzle not found: {puzzle_id}")
    return json.loads(pfile.read_text(encoding="utf-8"))


@router.post("/puzzles")
def save_studio_puzzle(request: StudioPuzzleSaveRequest):
    pid = _validate_slug(request.id, "puzzle_id")
    folder = _validate_folder(request.folder)
    dest_dir = _safe_under(PUZZLES_DIR, folder) if folder else PUZZLES_DIR
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest_file = dest_dir / f"{pid}.json"

    # Remove old location if moved between folders
    existing = _find_puzzle_file(pid)
    if existing is not None and existing.resolve() != dest_file.resolve():
        existing.unlink(missing_ok=True)

    doc = {
        "schema": "studio-puzzle-v1",
        "id": pid,
        "title": request.title.strip() or pid,
        "description": request.description,
        "folder": folder,
        "tags": sorted(set(t.strip().lower() for t in request.tags if t.strip())),
        "source": request.source,
        "decks": request.decks,
        "seed": request.seed,
        "seat": request.seat,
        "setup": request.setup,
        "lines": request.lines,
        "expectations": request.expectations,
        "assertions": request.assertions,
        "rl_goal": request.rl_goal,
    }
    dest_file.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")

    if folder:
        index = _load_library_index()
        if folder not in index["folders"]:
            index["folders"].append(folder)
            _save_library_index(index)

    return {
        "puzzle": doc,
        "path": str(dest_file.relative_to(REPO_ROOT)).replace("\\", "/"),
    }


@router.delete("/puzzles/{puzzle_id}")
def delete_studio_puzzle(puzzle_id: str):
    pfile = _find_puzzle_file(puzzle_id)
    if pfile is None:
        raise HTTPException(status_code=404, detail=f"Puzzle not found: {puzzle_id}")
    pfile.unlink(missing_ok=True)
    return {"status": "deleted", "id": puzzle_id}


@router.post("/puzzles/{puzzle_id}/export/scenario")
def export_puzzle_to_scenario(puzzle_id: str, line_id: Optional[str] = None):
    pfile = _find_puzzle_file(puzzle_id)
    if pfile is None:
        raise HTTPException(status_code=404, detail=f"Puzzle not found: {puzzle_id}")
    doc = json.loads(pfile.read_text(encoding="utf-8"))
    setup = doc.get("setup") or {}

    action_script: list[int] = []
    if line_id:
        for ln in doc.get("lines", []):
            if ln.get("id") == line_id:
                action_script = [int(a) for a in ln.get("actions", [])]
                break
    else:
        for ln in doc.get("lines", []):
            if ln.get("role") == "target":
                action_script = [int(a) for a in ln.get("actions", [])]
                break

    scenario = {
        "schema_version": 1,
        "id": doc["id"],
        "title": doc.get("title", doc["id"]),
        "readiness": "expected_pass",
        "blocked_reason": "",
        "decks": doc["decks"],
        "seed": doc.get("seed", 0),
        "state": setup.get("state", {"memory": 0, "phase": "Main", "turn": 1, "first_player": 1}),
        "zones": setup.get("zones", {}),
        "action_script": action_script,
        "assertions": doc.get("assertions", {"engine": [], "ui": []}),
    }
    SCENARIOS_DIR.mkdir(parents=True, exist_ok=True)
    out = _safe_under(SCENARIOS_DIR, f"{doc['id']}.json")
    out.write_text(json.dumps(scenario, indent=2) + "\n", encoding="utf-8")
    return {"path": str(out.relative_to(REPO_ROOT)).replace("\\", "/"), "scenario": scenario}


@router.post("/puzzles/{puzzle_id}/export/search-exam")
def export_puzzle_to_search_exam(puzzle_id: str):
    pfile = _find_puzzle_file(puzzle_id)
    if pfile is None:
        raise HTTPException(status_code=404, detail=f"Puzzle not found: {puzzle_id}")
    doc = json.loads(pfile.read_text(encoding="utf-8"))
    setup = doc.get("setup") or {}
    seat_0based = max(0, int(doc.get("seat", 1)) - 1)

    exam_expectations = []
    for exp in doc.get("expectations", []):
        kind = exp.get("kind", "")
        plan = [int(a) for a in exp.get("plan", [])]
        if kind in ("visits", "visit_share_at_least"):
            exam_expectations.append(
                {
                    "kind": "visit_share_at_least",
                    "plan": plan,
                    "share": float(exp.get("min_share", exp.get("share", 0.1))),
                    "label": exp.get("label", "target line"),
                }
            )
        elif kind in ("avoid", "not_chosen"):
            exam_expectations.append(
                {"kind": "not_chosen", "plan": plan, "label": exp.get("label", "avoid line")}
            )
        elif kind in ("choose", "chosen"):
            exam_expectations.append(
                {"kind": "chosen", "plan": plan, "label": exp.get("label", "expected line")}
            )

    lines_map = {
        str(ln.get("name") or ln.get("id")): [int(a) for a in ln.get("actions", [])]
        for ln in doc.get("lines", [])
    }

    decks_list = [doc["decks"].get("1", list(DEFAULT_FILLER_DECK)), doc["decks"].get("2", list(DEFAULT_FILLER_DECK))]
    exam_setup: dict[str, Any] = {
        "decks": decks_list,
        "game_seed": int(doc.get("seed", 0)),
        "actions": [int(a) for a in setup.get("prefix_actions", [])],
    }
    if setup.get("kind", "staged") == "staged":
        exam_setup["staged"] = {
            "state": setup.get("state", {"memory": 0, "phase": "Main", "turn": 1, "first_player": 1}),
            "zones": setup.get("zones", {}),
        }

    exam = {
        "schema": "search-exam-v1",
        "id": doc["id"],
        "title": doc.get("title", doc["id"]),
        "why": doc.get("description", ""),
        "source": doc.get("source", {"kind": "studio"}),
        "model": {"name": "studio-authored", "sha256": ""},
        "setup": exam_setup,
        "search": {
            "viewer": seat_0based,
            "regime": "open-list",
            "simulations": 128,
            "worlds": 4,
            "batch": 4,
            "search_seed": int(doc.get("seed", 0)),
        },
        "expectations": exam_expectations,
        "lines": lines_map,
    }
    SEARCH_EXAMS_DIR.mkdir(parents=True, exist_ok=True)
    out = _safe_under(SEARCH_EXAMS_DIR, f"{doc['id']}.json")
    out.write_text(json.dumps(exam, indent=2) + "\n", encoding="utf-8")
    return {"path": str(out.relative_to(REPO_ROOT)).replace("\\", "/"), "exam": exam}


@router.post("/search-exams/{exam_id}/open")
def open_search_exam_in_studio(exam_id: str):
    """Load a `qa/search-exams/<exam_id>.json` fixture into a live Studio session
    with its board state, named lines, and search expectations pre-populated."""
    eid = _validate_slug(exam_id, "exam_id")
    exam_path = _safe_under(SEARCH_EXAMS_DIR, f"{eid}.json")
    if not exam_path.exists():
        raise HTTPException(status_code=404, detail=f"Search exam not found: {eid}")
    doc = json.loads(exam_path.read_text(encoding="utf-8"))
    setup = doc.get("setup") or {}
    decks_raw = setup.get("decks") or []
    if isinstance(decks_raw, dict):
        deck1 = list(decks_raw.get("1") or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw.get("2") or DEFAULT_FILLER_DECK)
    elif isinstance(decks_raw, list) and len(decks_raw) >= 2:
        deck1 = list(decks_raw[0] or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw[1] or DEFAULT_FILLER_DECK)
    else:
        deck1, deck2 = list(DEFAULT_FILLER_DECK), list(DEFAULT_FILLER_DECK)

    game_seed = int(setup.get("game_seed") or 0)
    prefix_actions = [int(a) for a in (setup.get("actions") or [])]

    try:
        if isinstance(setup.get("staged"), dict):
            staged = setup["staged"]
            st = staged.get("state") or {}
            board_init = {
                "deck1": deck1,
                "deck2": deck2,
                "seed": game_seed,
                "memory": int(st.get("memory", 0)),
                "phase": str(st.get("phase") or "Main"),
                "turn": int(st.get("turn", 1)),
                "first_player": int(st.get("first_player", 1)),
                "zones": staged.get("zones") or {},
            }
            game, validation_error = _build_debug_game_from_board(board_init)
            for aid in prefix_actions:
                game.step(aid)
        else:
            game = RustDebugGame(deck1, deck2, game_seed)
            if not prefix_actions:
                game.skip_mulligan()
            else:
                for aid in prefix_actions:
                    game.step(aid)
            validation_error = None
            try:
                game.validate()
            except Exception as exc:
                validation_error = str(exc)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Failed to load search exam {eid}: {exc}")

    snap = game.to_scenario()
    board_dict = {
        "deck1": snap["decks"].get("1") or deck1,
        "deck2": snap["decks"].get("2") or deck2,
        "seed": game_seed,
        "memory": int(snap["state"].get("memory", 0)),
        "phase": str(snap["state"].get("phase", "Main")),
        "turn": int(snap["state"].get("turn", 1)),
        "first_player": int(snap["state"].get("first_player", 1)),
        "zones": snap.get("zones", {}),
    }

    session_id = str(uuid4())
    _register_game(session_id, game, board_dict)

    lines: dict[str, dict[str, Any]] = {}
    raw_lines = doc.get("lines") or {}
    if isinstance(raw_lines, dict):
        for idx, (lname, acts) in enumerate(raw_lines.items(), 1):
            if isinstance(acts, list):
                lid = _slugify_tag(lname) or f"line-{idx}"
                lines[lid] = {
                    "id": lid,
                    "name": str(lname),
                    "role": "target",
                    "actions": [int(a) for a in acts],
                }

    norm_expectations: list[dict[str, Any]] = []
    for exp in doc.get("expectations") or []:
        if not isinstance(exp, dict):
            continue
        kind = str(exp.get("kind") or "")
        plan = [int(a) for a in (exp.get("plan") or [])]
        if kind in ("visit_share_at_least", "visits"):
            norm_expectations.append(
                {
                    "kind": "visits",
                    "plan": plan,
                    "min_share": float(exp.get("share", exp.get("min_share", 0.1))),
                    "label": exp.get("label") or "target line",
                }
            )
        elif kind in ("not_chosen", "avoid"):
            norm_expectations.append(
                {"kind": "avoid", "plan": plan, "label": exp.get("label") or "avoid line"}
            )
        elif kind in ("chosen", "choose"):
            norm_expectations.append(
                {"kind": "choose", "plan": plan, "label": exp.get("label") or "expected line"}
            )

    search_cfg = doc.get("search") or {}
    seat = 2 if int(search_cfg.get("viewer", 0)) == 1 else 1

    sess = StudioSession(
        session_id=session_id,
        game_id=session_id,
        base_board=board_dict,
        seats={
            "1": SeatConfig(type="human").model_dump(),
            "2": SeatConfig(type="human").model_dump(),
        },
        source={"kind": "search_exam", "ref": str(exam_path.relative_to(REPO_ROOT)).replace("\\", "/")},
        lines=lines,
        validation_error=validation_error,
    )
    _studio_sessions[session_id] = sess
    return {
        "session": _session_payload(sess),
        "meta": {
            "id": str(doc.get("id") or eid),
            "title": str(doc.get("title") or eid),
            "description": str(doc.get("why") or doc.get("description") or ""),
            "seat": seat,
            "expectations": norm_expectations,
            "assertions": [],
            "search": search_cfg,
        },
    }


@router.post("/scenarios/{scenario_id}/open")
def open_scenario_in_studio(scenario_id: str):
    """Load a `qa/scenarios/<scenario_id>.json` fixture into a live Studio session
    with its staged board, action_script line, and engine assertions pre-populated."""
    sid = _validate_slug(scenario_id, "scenario_id")
    scen_path = _safe_under(SCENARIOS_DIR, f"{sid}.json")
    if not scen_path.exists():
        raise HTTPException(status_code=404, detail=f"Scenario not found: {sid}")
    doc = json.loads(scen_path.read_text(encoding="utf-8"))
    decks = doc.get("decks") or {}
    st = doc.get("state") or {}
    board_dict = {
        "deck1": list(decks.get("1") or DEFAULT_FILLER_DECK),
        "deck2": list(decks.get("2") or DEFAULT_FILLER_DECK),
        "seed": int(doc.get("seed") or 0),
        "memory": int(st.get("memory", 0)),
        "phase": str(st.get("phase") or "Main"),
        "turn": int(st.get("turn", 1)),
        "first_player": int(st.get("first_player", 1)),
        "zones": doc.get("zones") or {},
    }
    try:
        game, validation_error = _build_debug_game_from_board(board_dict)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Failed to stage scenario {sid}: {exc}")

    session_id = str(uuid4())
    _register_game(session_id, game, board_dict)

    lines: dict[str, dict[str, Any]] = {}
    script = [int(a) for a in (doc.get("action_script") or [])]
    if script:
        lines["scenario-script"] = {
            "id": "scenario-script",
            "name": "Scenario Action Script",
            "role": "target",
            "actions": script,
        }

    sess = StudioSession(
        session_id=session_id,
        game_id=session_id,
        base_board=board_dict,
        seats={
            "1": SeatConfig(type="human").model_dump(),
            "2": SeatConfig(type="human").model_dump(),
        },
        source={"kind": "scenario", "ref": str(scen_path.relative_to(REPO_ROOT)).replace("\\", "/")},
        lines=lines,
        validation_error=validation_error,
    )
    _studio_sessions[session_id] = sess
    return {
        "session": _session_payload(sess),
        "meta": {
            "id": str(doc.get("id") or sid),
            "title": str(doc.get("title") or sid),
            "description": str(doc.get("notes") or doc.get("description") or ""),
            "seat": 2 if int(st.get("first_player", 1)) == 2 else 1,
            "expectations": [],
            "assertions": list((doc.get("assertions") or {}).get("engine") or []),
        },
    }
