"""Per-game mulligan log writer + wrapper.

Captures starting hand + mulligan choice from the pilot seat during
training, appended live to `models/<run>/mulligan_log.jsonl`. See
`docs/superpowers/specs/2026-05-23-mulligan-log-design.md`.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

from data_paths import CARDS_JSON


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
