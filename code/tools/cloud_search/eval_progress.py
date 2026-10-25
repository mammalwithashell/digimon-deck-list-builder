"""Optional evaluation telemetry: heartbeat and counters, never game decisions."""
import json
from pathlib import Path
import threading
import time
import logging


class EvaluationProgress:
    def __init__(self, path, *, total, panel, checkpoint, interval=5):
        self.path = Path(path)
        self.interval = interval
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.started = time.monotonic()
        self.warning_emitted = False
        self.state = dict(status="running", panel=panel, checkpoint=checkpoint, total_games=total,
                          games_completed=0, started_unix=time.time(), last_game_completed_unix=None)
        self.publish()
        self.thread = threading.Thread(target=self._heartbeat, daemon=True)
        self.thread.start()

    def event(self, event):
        with self.lock:
            self.state.update(anchor=event["anchor"], archetype=event["archetype"],
                              opponent_archetype=event["opponent_archetype"], game_index=event["index"])
            if event["event"] == "game_started":
                self.state["game_started_unix"] = time.time()
            else:
                self.state["games_completed"] += 1
                self.state["last_game_completed_unix"] = time.time()

    def publish(self):
        with self.lock:
            payload = dict(self.state)
        now, elapsed = time.time(), time.monotonic() - self.started
        count = payload["games_completed"]
        payload.update(heartbeat_unix=now, elapsed_seconds=elapsed,
                       games_per_minute=60*count/elapsed if elapsed > 0 else None, eta_seconds=None)
        if count >= 10 and payload["status"] == "running" and now - (payload["last_game_completed_unix"] or now) < 120:
            payload["eta_seconds"] = max(0, payload["total_games"] - count)*elapsed/count
        try:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            temporary = self.path.with_suffix(".tmp")
            temporary.write_text(json.dumps(payload), encoding="utf-8")
            temporary.replace(self.path)
        except OSError as error:
            if not self.warning_emitted:
                logging.getLogger(__name__).warning("Evaluation progress output unavailable: %s", error)
                self.warning_emitted = True

    def _heartbeat(self):
        while not self.stop.wait(self.interval):
            self.publish()

    def finish(self, status, error=None):
        self.stop.set()
        self.thread.join()
        with self.lock:
            self.state.update(status=status, finished_unix=time.time())
            if error:
                self.state["error"] = str(error)
        self.publish()
