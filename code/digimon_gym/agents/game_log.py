"""Per-game eval-game-log writer.

Appended live during evaluation by `WinRateCallback._run_evaluation`. One
row per completed eval game, captured at the same point the callback
already extracts per-game digivolve counts, winner_id, archetypes, and
terminal_score. See `openspec/changes/add-per-game-eval-log/` for the
row schema and motivation.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict


class GameLogWriter:
    """Append-only JSONL writer for per-game eval rows.

    Opens in `"a"` mode so re-runs against an existing run directory
    append rather than truncate. Flushes after every row write so a
    SIGKILL leaves a parseable file. On any OSError, disables itself
    and logs once to stderr — training MUST NOT die from observability.
    """

    def __init__(self, path: str | Path) -> None:
        self._path = Path(path)
        self.enabled = True
        self._failed = False

    @property
    def path(self) -> Path:
        return self._path

    def append(self, row: Dict[str, Any]) -> None:
        if not self.enabled or self._failed:
            return
        try:
            self._path.parent.mkdir(parents=True, exist_ok=True)
            with self._path.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(row) + "\n")
                fh.flush()
        except OSError as exc:
            self._failed = True
            self.enabled = False
            print(
                f"[game_log] disabled after write failure at {self._path}: {exc!r}",
                file=sys.stderr,
                flush=True,
            )
