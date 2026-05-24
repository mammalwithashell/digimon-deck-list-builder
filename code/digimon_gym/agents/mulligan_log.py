"""Per-game mulligan log writer + wrapper.

Captures starting hand + mulligan choice from the pilot seat during
training, appended live to `models/<run>/mulligan_log_env_<NNN>.jsonl`
(one file per env_index for SubprocVecEnv safety). See
`docs/superpowers/specs/2026-05-23-mulligan-log-design.md`.
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

import gymnasium

from data_paths import CARDS_JSON
from digimon_gym.agents.env_utils import unwrap_to_digimon_env


SCHEMA_VERSION = 1


def _load_card_metadata() -> Dict[str, Dict[str, Any]]:
    """Load cards.json once at module import; used by helpers below.

    On any I/O or parse failure, log once to stderr and return an empty
    dict so callers degrade gracefully (helpers return zero histograms
    and False for tamer) rather than crash training.
    """
    try:
        return json.loads(Path(CARDS_JSON).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(
            f"[mulligan_log] cards.json unavailable; hand features will be empty: {exc!r}",
            file=sys.stderr,
            flush=True,
        )
        return {}


_CARDS = _load_card_metadata()


def _derive_lvl_counts(card_ids: List[str]) -> Dict[str, int]:
    """Return a histogram of levels 3..7 for the given card IDs.

    Unknown card IDs and cards without a level field contribute 0 to every
    bucket. Only Digimon levels 3-7 are bucketed; eggs (level 2) and
    Options/Tamers are ignored here (use `_derive_has_tamer` for tamers).
    """
    buckets = {str(lvl): 0 for lvl in range(3, 8)}
    for cid in card_ids:
        lvl = _CARDS.get(cid, {}).get("level")
        if isinstance(lvl, int) and 3 <= lvl <= 7:
            buckets[str(lvl)] += 1
    return buckets


def _derive_has_tamer(card_ids: List[str]) -> bool:
    """True if any card in the list is a Tamer.

    cards.json encodes card type as ``card_kind`` (int): 0=Digimon, 1=Tamer,
    2=Option, 3=DigiEgg.  A value of 1 means Tamer.
    """
    for cid in card_ids:
        if _CARDS.get(cid, {}).get("card_kind") == 1:
            return True
    return False


class MulliganLogWriter:
    """Owns the JSONL file handle for a training run's mulligan log.

    One writer instance per env_index. Under SubprocVecEnv, each subprocess
    holds its own writer pointing at its own per-env-index file (e.g.
    ``mulligan_log_env_000.jsonl``, ``mulligan_log_env_001.jsonl``, ...)
    so concurrent appends never contend on the same file. Analysis tools
    glob ``mulligan_log_env_*.jsonl`` to recover the cross-env dataset.

    A single header line is written lazily on the first ``append()`` per
    writer instance. Subsequent appends write one JSON record per line.
    Failures (disk full, permission denied) flip ``enabled`` to ``False``
    and log once to stderr so training is never killed by observability
    code.
    """

    def __init__(
        self,
        output_dir: str | Path,
        *,
        env_index: int = 0,
        enabled: bool = True,
        run_metadata: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.output_dir = Path(output_dir)
        self.env_index = int(env_index)
        self.enabled = bool(enabled)
        self.run_metadata = dict(run_metadata or {})
        self._path: Path = self.output_dir / f"mulligan_log_env_{self.env_index:03d}.jsonl"
        self._wrote_header = False
        self._failed = False

    @property
    def path(self) -> Path:
        return self._path

    def _header_record(self) -> Dict[str, Any]:
        return {
            "kind": "mulligan_log_header",
            "schema_version": SCHEMA_VERSION,
            **self.run_metadata,
        }

    def append(self, record: Dict[str, Any]) -> None:
        """Append one JSONL record. No-op if disabled."""
        if not self.enabled or self._failed:
            return
        try:
            self.output_dir.mkdir(parents=True, exist_ok=True)
            with self._path.open("a", encoding="utf-8") as fh:
                if not self._wrote_header:
                    fh.write(json.dumps(self._header_record()) + "\n")
                    self._wrote_header = True
                fh.write(json.dumps(record) + "\n")
        except OSError as exc:
            self._failed = True
            self.enabled = False
            print(
                f"[mulligan_log] disabled after write failure: {exc!r}",
                file=sys.stderr,
                flush=True,
            )


class MulliganLogWrapper(gymnasium.Wrapper):
    """Capture pilot's starting-hand + mulligan choice per game.

    Sits in the env stack outside OpponentWrapper / GeneralistDeckPoolWrapper
    / TrainingRecordingWrapper. On ``reset()`` it stashes a pending record
    with the pilot's hand snapshot. On ``step()`` it finalizes the record
    with the action if the pre-step state shows pilot is the mulligan
    decider.
    """

    def __init__(
        self,
        env: gymnasium.Env,
        writer: MulliganLogWriter,
        *,
        source: str = "train",
        env_index: int = 0,
    ) -> None:
        super().__init__(env)
        self._writer = writer
        self.source = source
        self.env_index = env_index
        self._inner = unwrap_to_digimon_env(env)
        self._pending: Optional[Dict[str, Any]] = None
        self._game_counter = 0

    # ─── Gymnasium API ───────────────────────────────────────────

    def reset(self, **kwargs):
        obs, info = self.env.reset(**kwargs)
        self._pending = None  # drop any unfinalized record from a crashed game
        if not self._writer.enabled:
            return obs, info
        runner = self._inner.runner
        if runner is None:
            return obs, info
        # Only snapshot if pilot is about to face a mulligan decision.
        if runner.mulligan_current_player != 1:
            return obs, info
        try:
            ui = runner.to_ui_json()
        except Exception:
            return obs, info
        hand_ids = list(ui.get("player1", {}).get("handIds", []) or [])
        # `current_player_id` from _rl_state() is always 1 here (OpponentWrapper
        # advanced to P1's decision). Use `currentPlayer` from to_ui_json() instead:
        # that field tracks game.turn_player() = turn_order[0], which is the actual
        # first player and doesn't change during mulligan steps.
        first_player_id: Optional[int] = None
        try:
            first_player_id = ui.get("currentPlayer")
            if first_player_id is not None:
                first_player_id = int(first_player_id)
        except Exception:
            first_player_id = None
        # If recording is enabled, prefer the recording's initial_state which is
        # the canonical source and validates our derivation.
        try:
            rec = runner.get_recording()
            if rec is not None:
                fp = rec.get("initial_state", {}).get("first_player_id")
                if fp is not None:
                    first_player_id = int(fp)
        except Exception:
            pass
        self._pending = {
            "schema_version": SCHEMA_VERSION,
            "wall_time": time.time(),
            "iso_time": datetime.now(timezone.utc).isoformat(),
            "global_step": self._infer_global_step(),
            "source": self.source,
            "env_index": self.env_index,
            "game_index": self._game_counter,
            "agent_archetype": info.get("deck1_archetype"),
            "opp_archetype": info.get("opponent_archetype"),
            "hand_card_ids": hand_ids,
            "hand_lvl_counts": _derive_lvl_counts(hand_ids),
            "hand_has_tamer": _derive_has_tamer(hand_ids),
            "hand_size": len(hand_ids),
            "first_player_id": first_player_id,
        }
        return obs, info

    def step(self, action):
        # Snapshot pre-step state so we know whether this step resolves a
        # pilot mulligan.
        pre_player: Optional[int] = None
        runner = self._inner.runner if self._inner else None
        if runner is not None and self._pending is not None:
            pre_player = runner.mulligan_current_player
        obs, reward, terminated, truncated, info = self.env.step(action)
        if self._pending is not None and pre_player == 1:
            self._pending["action"] = int(action)
            self._writer.append(self._pending)
            self._pending = None
            self._game_counter += 1
        return obs, reward, terminated, truncated, info

    # ─── Internals ───────────────────────────────────────────────

    def _infer_global_step(self) -> Optional[int]:
        """Best-effort: SB3 attaches `num_timesteps` to the env in some setups."""
        return getattr(self.unwrapped, "num_timesteps", None)
