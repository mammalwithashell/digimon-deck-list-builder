"""Strict new-mode result validation; never accepts historical untagged output.

This validates recorded semantics, not a replay of engine legality. A separate
qualification suite must establish the driver's truthful action-mask recording.
"""
import hashlib
import json
import math
from pathlib import Path

SCHEMA = "frozen-information-regime-eval-v1"
MODES = {"known-list", "closed-list"}
TERMINALS = {"Some(SecurityAttack)", "Some(DeckOut)", "Some(EngineDeclared)",
             "Some(Concede)", "Some(InfiniteLoopDraw)"}


def require(value, message):
    if not value:
        raise ValueError(message)


def finite_nonnegative(value):
    return isinstance(value, (int, float)) and math.isfinite(value) and value >= 0


def validate(request, output, plan):
    require(all(x.get("schema") == SCHEMA for x in (request, output, plan)), "untagged schema")
    require(output["request"] == request, "request echo")
    require(request["runtime_sha256"] == output["runtime_sha256"] == plan["native_sha256"], "runtime")
    require(request["prior_snapshot_hash"] == output["prior_snapshot_hash"] == plan["prior_snapshot_hash"], "prior")
    for key in ("model_sha256", "opponent_model_sha256"):
        require(output[key] == plan[key], key)
    for actor in ("candidate", "opponent"):
        settings = request[actor]
        require(settings["kind"] == "search" and settings["information_regime"] in MODES, "actor mode")
        require([settings[k] for k in ("simulations", "worlds", "batch", "root_noise")] == [320, 4, 4, 0], "requested budget")
    games = output["games"]
    require(len(games) == 2 and [g["candidate_seat"] for g in games] == [0, 1], "swapped model seats")
    for game in games:
        require(game["truncated"] is False and game["terminal_reason"] in TERMINALS, "incomplete terminal")
        require(game["first_player"] in (0, 1), "opener")
        winner = game["winner"]
        require(winner is None or (type(winner) is int and winner in (0, 1)), "winner")
        require((winner is None) == (game["terminal_reason"] == "Some(InfiniteLoopDraw)"), "draw semantics")
        require(game["score"] == (0.5 if winner is None else float(winner == game["candidate_seat"])), "score")
        actions, steps = game["actions"], game["card_metrics"]["steps"]
        require(0 < len(actions) == game["decisions"] == len(steps) <= plan["decision_cap"], "trace length")
        require(finite_nonnegative(game["seconds"]), "game timing")
        for action, step in zip(actions, steps):
            require(type(action) is int and action == step["action"] and action in step["legal_actions"], "recorded legality")
            require(step["mover"] in (1, 2) and finite_nonnegative(step["selection_seconds"]), "step actor/timing")
        searches = game["search_diagnostics"]
        require(game["searched"] == len(searches), "search count")
        covered = set()
        for search in searches:
            start, route, d = search["decision"], search["selected_plan"], search["diagnostics"]
            require(type(start) is int and 0 <= start < len(actions) and route, "search position")
            require(start + len(route) <= len(actions) and actions[start:start+len(route)] == route, "executed plan")
            require(not covered.intersection(range(start, start+len(route))), "overlapping plans")
            covered.update(range(start, start+len(route)))
            viewer = d["viewer"]
            require(viewer in (0, 1) and all(s["mover"] == viewer+1 for s in steps[start:start+len(route)]), "plan actor")
            require([d[k] for k in ("simulations", "worlds", "batch")] == [320, 4, 4], "actual budget")
            expected_seed = (request["search_seed"] + start * 0x9e3779b97f4a7c15 + viewer) % (1 << 64)
            require(d["search_seed"] == expected_seed and finite_nonnegative(d["seconds"]), "search seed/timing")
            mode = request["candidate" if viewer == game["candidate_seat"] else "opponent"]["information_regime"]
            info = d["information"]
            require(info["regime"] == mode, "actual information regime")
            if mode == "closed-list":
                require(info["prior_snapshot_hash"] == plan["prior_snapshot_hash"], "actual prior")
                require(info["sampled_worlds"] == 4 and len(set(info["world_seeds"])) == 4, "world coverage")
                require(0 <= info["fallback_worlds"] <= 4 and 0 <= info["fallback_mass"] <= 1, "fallback")
                require(len(info["evidence_sha256"]) == 64 and finite_nonnegative(info["sampling_seconds"]), "belief evidence")
            plans = search["root_plans"]
            require(plans and route == max(enumerate(plans), key=lambda p: (p[1][1], -p[0]))[1][0], "selected plan")
        require(all(len(s["legal_actions"]) == 1 or i in covered for i, s in enumerate(steps)), "unsearched nonforced root")
    return {"games": 2, "wins": sum(g["score"] == 1 for g in games),
            "draws": sum(g["score"] == .5 for g in games), "losses": sum(g["score"] == 0 for g in games)}


def validate_receipt(root, request_path, output_path, receipt_path, plan):
    root = Path(root).resolve()
    def load(path):
        path = (root / path).resolve()
        require(path.is_relative_to(root), "receipt path outside root")
        data = path.read_bytes()
        return json.loads(data), hashlib.sha256(data).hexdigest()
    request, rh = load(request_path)
    output, oh = load(output_path)
    receipt, _ = load(receipt_path)
    require(receipt["request_sha256"] == rh and receipt["output_sha256"] == oh, "receipt hash")
    require(receipt["schema"] == SCHEMA and receipt["runtime_sha256"] == plan["native_sha256"], "receipt runtime")
    require(receipt["prior_snapshot_hash"] == plan["prior_snapshot_hash"], "receipt prior")
    return validate(request, output, plan)
