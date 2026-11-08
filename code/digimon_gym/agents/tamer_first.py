"""Opt-in native heuristic opponents. No model retraining or hidden-state input."""
import json

VERSIONS = ("greedy_tamer_first_v1", "development_tamer_first_v1")


class TamerFirstOpponent:
    """Delegate to the native teacher; fail clearly on an old binding.

    The runner owns independent per-seat development state and is replaced by
    DigimonEnv.reset(). Counters are observational and persist across games.
    """
    def __init__(self, version):
        if version not in VERSIONS:
            raise ValueError(f"Unknown Tamer-first baseline: {version}")
        self.version = version
        self.decisions = 0
        self.opportunities = 0
        self.last_decision = None

    def __call__(self, env):
        runner = env.runner
        mask = env.get_action_mask()
        try:
            result = json.loads(runner.teacher_decision(self.version))
        except AttributeError as exc:
            raise RuntimeError(f"{self.version} needs a qualified native binding with Tamer-first teacher support: {exc}") from exc
        except ValueError as exc:
            if "unknown bootstrap teacher" in str(exc):
                raise RuntimeError(f"{self.version} needs a qualified native binding with Tamer-first teacher support: {exc}") from exc
            raise RuntimeError(f"{self.version} native decision failed: {exc}") from exc
        if result.get("version") != self.version:
            raise RuntimeError("native teacher version mismatch")
        action = result["action"]
        routes = result["direct_tamer_routes"]
        if not isinstance(action, int) or not 0 <= action < len(mask) or mask[action] <= 0.5:
            raise RuntimeError("native teacher returned a masked action")
        if any(not isinstance(a, int) or not 0 <= a < len(mask) or mask[a] <= 0.5 for a in routes):
            raise RuntimeError("native teacher declared a masked Tamer route")
        if bool(routes) != result["tamer_priority_applied"] or (routes and action not in routes):
            raise RuntimeError("native teacher violated Tamer-first priority")
        self.decisions += 1
        self.opportunities += bool(routes)
        self.last_decision = result
        return action

    def reset_state(self):
        # Per-game native teacher state belongs to the newly reset runner.
        self.last_decision = None
