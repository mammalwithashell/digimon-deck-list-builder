"""Emit `selfplay-shards-v1` supervised warm-start shards from Studio puzzles & search exams.

Accepts both canonical `studio-puzzle-v1` files (`qa/puzzles/`) and `search-exam-v1`
files (`qa/search-exams/`), reconstructs each board plus `K` slot-permuted variants via
`permute_staged_puzzle`, verifies every step in the `target` line in `RustHeadlessGame`,
and writes `selfplay-shards-v1` arrays consumable out-of-the-box by both
`digimon_gym.agents.azero_trainer` and `digimon_gym.agents.azero_lstm_distill`
(including `forced_selection.npy`, `chain_anchor_idx.npy`, optional `avoid_*.npy`,
and multi-world Bayes-averaged `policy_visits.npy` + continuous `z.npy` for belief puzzles).
"""

from __future__ import annotations

from collections import Counter
import argparse
import copy
import json
import random
import sys
from pathlib import Path
from typing import Any, Optional, Sequence

import numpy as np

_CODE_DIR = Path(__file__).resolve().parents[1]
if str(_CODE_DIR) not in sys.path:
    sys.path.insert(0, str(_CODE_DIR))

from digimon_gym.agents.studio_puzzles import (  # noqa: E402
    StudioPuzzle,
    _advance_interrupts,
    _evaluate_goal_assertion,
    _get_card_db,
    _is_digi_egg,
    _make_staged_headless_game,
    _normalize_lines,
    load_studio_puzzle,
    permute_staged_puzzle,
    select_refutation_action,
)
from model_contract import current_contract  # noqa: E402

SCHEMA = "selfplay-shards-v1"
DEFAULT_FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50
RNG_EVENT_TYPES = frozenset({"Draw", "Reveal", "SecurityReveal", "Mill"})


def _is_token_card_id(card_id: str) -> bool:
    if not card_id or "TOKEN" in card_id.upper():
        return True
    db = _get_card_db()
    if db is None:
        return False
    try:
        return db.get_card(card_id) is None
    except Exception:
        return False


def _attach_belief_metadata(puzzle: StudioPuzzle, doc: dict[str, Any]) -> StudioPuzzle:
    prior = doc.get("prior")
    setattr(puzzle, "prior", copy.deepcopy(prior) if isinstance(prior, dict) else None)
    search_block = doc.get("search") if isinstance(doc.get("search"), dict) else {}
    regime = doc.get("information_regime") or search_block.get("regime")
    setattr(puzzle, "regime", str(regime) if regime is not None else None)
    return puzzle


def _copy_belief_metadata(src: StudioPuzzle, dst: StudioPuzzle) -> StudioPuzzle:
    setattr(dst, "prior", copy.deepcopy(getattr(src, "prior", None)))
    setattr(dst, "regime", getattr(src, "regime", None))
    return dst


def load_puzzle_or_exam_doc(doc: dict[str, Any], path: Optional[Path] = None) -> StudioPuzzle:
    """Parse a `studio-puzzle-v1` or `search-exam-v1` dict into a `StudioPuzzle`."""
    schema = doc.get("schema")
    stem = path.stem if path is not None else "inline-puzzle"
    if schema == "studio-puzzle-v1":
        decks = doc.get("decks") or {}
        if "1" not in decks or "2" not in decks:
            raise ValueError(f"{path or stem} missing '1' or '2' deck")
        expectations = [dict(e) for e in (doc.get("expectations") or []) if isinstance(e, dict)]
        lines = _normalize_lines(doc.get("lines"), expectations=expectations)
        refutation_triggers = [
            dict(t) for t in (doc.get("refutation_triggers") or []) if isinstance(t, dict)
        ]
        puzzle = StudioPuzzle(
            id=str(doc["id"]),
            title=str(doc.get("title") or doc["id"]),
            folder=str(doc.get("folder") or ""),
            tags=[str(t) for t in doc.get("tags", [])],
            decks={"1": list(decks["1"]), "2": list(decks["2"])},
            seed=int(doc.get("seed", 0)),
            seat=int(doc.get("seat", 1)),
            setup=copy.deepcopy(dict(doc.get("setup") or {})),
            rl_goal=copy.deepcopy(dict(doc.get("rl_goal") or {"mode": "playout_to_win"})),
            path=path,
            lines=lines,
            expectations=expectations,
            refutation_triggers=refutation_triggers,
        )
        return _attach_belief_metadata(puzzle, doc)

    if schema != "search-exam-v1":
        raise ValueError(
            f"{path or stem}: unsupported schema {schema!r} (expected studio-puzzle-v1 or search-exam-v1)"
        )

    setup_raw = doc.get("setup") or {}
    decks_raw = setup_raw.get("decks") or []
    if isinstance(decks_raw, dict):
        deck1 = list(decks_raw.get("1") or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw.get("2") or DEFAULT_FILLER_DECK)
    elif isinstance(decks_raw, list) and len(decks_raw) >= 2:
        deck1 = list(decks_raw[0] or DEFAULT_FILLER_DECK)
        deck2 = list(decks_raw[1] or DEFAULT_FILLER_DECK)
    else:
        deck1, deck2 = list(DEFAULT_FILLER_DECK), list(DEFAULT_FILLER_DECK)

    viewer = int((doc.get("search") or {}).get("viewer", 0))
    seat = 2 if viewer == 1 else 1
    game_seed = int(setup_raw.get("game_seed", 0))
    prefix_actions = [int(a) for a in (setup_raw.get("actions") or [])]

    if isinstance(setup_raw.get("staged"), dict):
        staged = setup_raw["staged"]
        setup: dict[str, Any] = {
            "kind": "staged",
            "state": copy.deepcopy(
                staged.get("state") or {"memory": 0, "phase": "Main", "turn": 1, "first_player": seat}
            ),
            "zones": copy.deepcopy(staged.get("zones") or {}),
            "prefix_actions": prefix_actions,
        }
    else:
        setup = {
            "kind": "replay_prefix",
            "prefix_actions": prefix_actions,
        }

    expectations = [dict(e) for e in (doc.get("expectations") or []) if isinstance(e, dict)]
    lines = _normalize_lines(doc.get("lines"), expectations=expectations)
    if not any(ln.get("role") == "target" and ln.get("actions") for ln in lines):
        for exp in expectations:
            if exp.get("kind") in ("chosen", "choose", "visit_share_at_least", "visits"):
                plan = [int(a) for a in (exp.get("plan") or [])]
                if plan:
                    lines.insert(
                        0,
                        {
                            "id": "target-expectation",
                            "name": "target-expectation",
                            "role": "target",
                            "actions": plan,
                        },
                    )
                    break

    refutation_triggers = [
        dict(t) for t in (doc.get("refutation_triggers") or []) if isinstance(t, dict)
    ]
    puzzle = StudioPuzzle(
        id=str(doc.get("id") or stem),
        title=str(doc.get("title") or doc.get("id") or stem),
        folder="",
        tags=[str(t) for t in doc.get("tags", [])],
        decks={"1": deck1, "2": deck2},
        seed=game_seed,
        seat=seat,
        setup=setup,
        rl_goal=copy.deepcopy(dict(doc.get("rl_goal") or {"mode": "playout_to_win"})),
        path=path,
        lines=lines,
        expectations=expectations,
        refutation_triggers=refutation_triggers,
    )
    return _attach_belief_metadata(puzzle, doc)


def load_puzzle_or_exam(path: str | Path) -> StudioPuzzle:
    """Load either a `studio-puzzle-v1` or `search-exam-v1` JSON file as a `StudioPuzzle`."""
    p = Path(path)
    doc = json.loads(p.read_text(encoding="utf-8"))
    if doc.get("schema") == "studio-puzzle-v1":
        loaded = load_studio_puzzle(p)
        return _attach_belief_metadata(loaded, doc)
    return load_puzzle_or_exam_doc(doc, path=p)


def _resolve_inputs(
    puzzles_or_exams: Sequence[str | Path | StudioPuzzle | dict[str, Any]]
    | str
    | Path
    | StudioPuzzle
    | dict[str, Any],
) -> list[StudioPuzzle]:
    if isinstance(puzzles_or_exams, (str, Path, StudioPuzzle, dict)):
        items: list[Any] = [puzzles_or_exams]
    else:
        items = list(puzzles_or_exams)

    resolved: list[StudioPuzzle] = []
    for item in items:
        if isinstance(item, StudioPuzzle):
            if getattr(item, "prior", None) is None and item.path is not None and item.path.is_file():
                try:
                    raw = json.loads(item.path.read_text(encoding="utf-8"))
                    _attach_belief_metadata(item, raw)
                except Exception:
                    pass
            resolved.append(item)
            continue
        if isinstance(item, dict):
            resolved.append(load_puzzle_or_exam_doc(item))
            continue
        p = Path(item)
        if p.is_dir():
            for child in sorted(p.rglob("*.json")):
                if child.name == "_library.json":
                    continue
                try:
                    loaded = load_puzzle_or_exam(child)
                except Exception:
                    continue
                if any(ln.get("role") == "target" and ln.get("actions") for ln in loaded.lines):
                    resolved.append(loaded)
        else:
            resolved.append(load_puzzle_or_exam(p))
    return resolved


def _extract_root_avoid_actions(
    puzzle: StudioPuzzle,
    root_legal_set: set[int],
    target_root_action: int,
) -> list[int]:
    avoid_ids: set[int] = set()
    for ln in puzzle.lines:
        if str(ln.get("role")) == "avoid":
            acts = [int(a) for a in (ln.get("actions") or [])]
            if acts:
                avoid_ids.add(acts[0])
    for exp in puzzle.expectations:
        if str(exp.get("kind")) in ("not_chosen", "avoid"):
            plan = [int(a) for a in (exp.get("plan") or [])]
            if plan:
                avoid_ids.add(plan[0])
    return sorted(a for a in avoid_ids if a in root_legal_set and a != int(target_root_action))


def _drain_events_and_check_rng(game: Any) -> bool:
    if not hasattr(game, "get_events_since_last_step"):
        return False
    try:
        evs = list(game.get_events_since_last_step())
    except Exception:
        return False
    return any(isinstance(ev, dict) and str(ev.get("type")) in RNG_EVENT_TYPES for ev in evs)


def _record_target_trajectory(
    puzzle: StudioPuzzle,
    target_actions: list[int],
    *,
    observation_profile: str,
    target_visit: float,
    include_forced_selection: bool = True,
) -> tuple[list[dict[str, Any]], int, int]:
    """Step `target_actions` on `RustHeadlessGame`, emitting forced PendingSelection value rows
    and tracking single-mover, non-RNG `chain_anchor_idx` links."""
    game = _make_staged_headless_game(puzzle, profile=observation_profile)
    _drain_events_and_check_rng(game)

    rows: list[dict[str, Any]] = []
    forced_count = 0
    forced_emitted_count = 0
    active_chain_rows: list[int] = []
    prev_step_had_rng = False

    for step_idx, act_id in enumerate(target_actions):
        opp_interrupted = (not game.is_game_over) and (int(game.current_player_id) != int(puzzle.seat))
        if opp_interrupted:
            if not _advance_interrupts(game, puzzle.seat, puzzle.refutation_triggers):
                raise ValueError(f"{puzzle.id}: opponent interrupt loop before step {step_idx}")
            _drain_events_and_check_rng(game)
            active_chain_rows.clear()
            prev_step_had_rng = False
        elif prev_step_had_rng:
            active_chain_rows.clear()
            prev_step_had_rng = False

        if game.is_game_over:
            raise ValueError(f"{puzzle.id}: game ended prematurely before step {step_idx}")
        if int(game.current_player_id) != int(puzzle.seat):
            raise ValueError(
                f"{puzzle.id}: expected solver seat {puzzle.seat} at step {step_idx}, got {game.current_player_id}"
            )

        mask = np.asarray(game.get_action_mask())
        legal_acts = np.flatnonzero(mask > 0.5).astype(np.int32)
        legal_set = {int(a) for a in legal_acts.tolist()}
        if int(act_id) not in legal_set:
            raise ValueError(
                f"{puzzle.id}: target action {act_id} at step {step_idx} is illegal (legal={sorted(legal_set)})"
            )

        is_pending = isinstance(game.get_pending_selection(), dict)

        if len(legal_acts) == 1:
            forced_count += 1
            if is_pending and include_forced_selection:
                row_idx = len(rows)
                active_chain_rows.append(row_idx)
                obs = np.asarray(game.get_board_tensor(int(puzzle.seat)), dtype=np.float32)
                visits = np.asarray([float(target_visit)], dtype=np.float32)
                rows.append(
                    {
                        "obs": obs,
                        "actions": legal_acts,
                        "visits": visits,
                        "avoid_actions": np.zeros(0, dtype=np.int32),
                        "policy_weight": 0.0,
                        "forced_selection": 1,
                        "chain_anchor_idx": -1,
                        "z": 1.0,
                    }
                )
                forced_emitted_count += 1
            game.step(int(act_id))
            prev_step_had_rng = _drain_events_and_check_rng(game)
            continue

        row_idx = len(rows)
        if is_pending:
            active_chain_rows.append(row_idx)
        elif active_chain_rows:
            for prev_i in active_chain_rows:
                rows[prev_i]["chain_anchor_idx"] = row_idx
            active_chain_rows.clear()

        obs = np.asarray(game.get_board_tensor(int(puzzle.seat)), dtype=np.float32)
        visits = np.where(legal_acts == int(act_id), float(target_visit), 0.0).astype(np.float32)
        avoid_acts = (
            _extract_root_avoid_actions(puzzle, legal_set, int(act_id)) if step_idx == 0 else []
        )
        rows.append(
            {
                "obs": obs,
                "actions": legal_acts,
                "visits": visits,
                "avoid_actions": np.asarray(avoid_acts, dtype=np.int32),
                "policy_weight": 1.0,
                "forced_selection": 0,
                "chain_anchor_idx": -1,
                "z": 1.0,
            }
        )
        game.step(int(act_id))
        prev_step_had_rng = _drain_events_and_check_rng(game)

    if active_chain_rows and rows:
        last_idx = len(rows) - 1
        for prev_i in active_chain_rows:
            if prev_i < last_idx:
                rows[prev_i]["chain_anchor_idx"] = last_idx
        active_chain_rows.clear()

    if not game.is_game_over:
        _advance_interrupts(game, puzzle.seat, puzzle.refutation_triggers)

    return rows, forced_count, forced_emitted_count


class _OnnxBeliefEvaluator:
    """Optional ONNX policy + value evaluator for multi-world belief rollouts."""

    def __init__(self, onnx_path: str | Path):
        from digimon_gym.inference.onnx_policy import _detect_model_type, _get_session, load_onnx_policy

        self.path = Path(onnx_path)
        self.session = _get_session(str(self.path))
        self.kind = _detect_model_type(self.session)
        self.policy = load_onnx_policy(str(self.path), model_type=self.kind)
        self.output_names = {o.name for o in self.session.get_outputs()}
        self.has_value = "value" in self.output_names
        val_companion = self.path.with_suffix(".value.onnx")
        self.value_session = _get_session(str(val_companion)) if val_companion.exists() else None

    def root_logits(self, obs: np.ndarray) -> np.ndarray:
        obs_b = obs.reshape(1, -1).astype(np.float32)
        if self.kind == "lstm":
            h = np.zeros((1, 1, self.policy.hidden_size), dtype=np.float32)
            c = np.zeros((1, 1, self.policy.hidden_size), dtype=np.float32)
            (logits,) = self.session.run(["logits"], {"obs": obs_b, "h_in": h, "c_in": c})
            return np.asarray(logits[0], dtype=np.float32)
        (logits,) = self.session.run(["logits"], {"obs": obs_b})
        return np.asarray(logits[0], dtype=np.float32)

    def predict_action(self, game: Any, player_id: int, mask: np.ndarray) -> int:
        obs = np.asarray(game.get_board_tensor(int(player_id)), dtype=np.float32)
        logits = self.root_logits(obs)
        masked = logits.copy()
        masked[np.asarray(mask) <= 0.5] = -1e9
        if masked.shape[0] > 93 and np.any((np.asarray(mask) > 0.5) & (np.arange(len(mask)) != 93)):
            masked[93] = -1e9
        return int(np.argmax(masked))

    def predict_value(self, game: Any, player_id: int) -> Optional[float]:
        obs = np.asarray(game.get_board_tensor(int(player_id)), dtype=np.float32).reshape(1, -1)
        if self.has_value and self.kind == "mlp":
            (val,) = self.session.run(["value"], {"obs": obs})
            return float(np.clip(np.asarray(val).reshape(-1)[0], -1.0, 1.0))
        if self.value_session is not None:
            h = np.zeros((1, 1, self.policy.hidden_size), dtype=np.float32)
            c = np.zeros((1, 1, self.policy.hidden_size), dtype=np.float32)
            (val,) = self.value_session.run(["value"], {"obs": obs, "h_in": h, "c_in": c})
            return float(np.clip(np.asarray(val).reshape(-1)[0], -1.0, 1.0))
        return None


def _is_belief_puzzle(puzzle: StudioPuzzle, belief_worlds: int) -> bool:
    prior = getattr(puzzle, "prior", None)
    if isinstance(prior, dict) and bool(prior.get("candidates")):
        return True
    if belief_worlds <= 1:
        return False
    regime = str(getattr(puzzle, "regime", "") or "").lower()
    if regime in ("closed-list", "closed_decklist"):
        return True
    if "belief" in {str(t).strip().lower() for t in (puzzle.tags or [])}:
        return True
    opp_str = "2" if int(puzzle.seat) == 1 else "1"
    opp_zones = ((puzzle.setup or {}).get("zones") or {}).get(opp_str)
    if isinstance(opp_zones, dict):
        if opp_zones.get("hand_unpinned") is True or opp_zones.get("unpinned_hand") is True:
            return True
        if "hand_count" in opp_zones and "hand" not in opp_zones:
            return True
    return False


def _collect_public_opp_cards(opp_zones: dict[str, Any]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for cid in opp_zones.get("trash") or []:
        if isinstance(cid, str) and not _is_token_card_id(cid):
            counts[cid] += 1
    br = opp_zones.get("breeding")
    if isinstance(br, list):
        for cid in br:
            if isinstance(cid, str) and not _is_token_card_id(cid):
                counts[cid] += 1
    for st in opp_zones.get("field") or []:
        if not isinstance(st, dict):
            continue
        for cid in st.get("stack") or []:
            if isinstance(cid, str) and not _is_token_card_id(cid):
                counts[cid] += 1
        for cid in st.get("linked_cards") or []:
            if isinstance(cid, str) and not _is_token_card_id(cid):
                counts[cid] += 1
    for cid in opp_zones.get("known_hand") or []:
        if isinstance(cid, str) and not _is_token_card_id(cid):
            counts[cid] += 1
    return counts


def _sample_belief_worlds(
    puzzle: StudioPuzzle,
    root_scen: dict[str, Any],
    belief_worlds: int,
    rng: random.Random,
) -> list[StudioPuzzle]:
    seat = int(puzzle.seat)
    opp_seat = 2 if seat == 1 else 1
    seat_str, opp_str = str(seat), str(opp_seat)

    orig_opp_cfg = ((puzzle.setup or {}).get("zones") or {}).get(opp_str) or {}
    scen_zones = root_scen.get("zones") or {}
    opp_scen = copy.deepcopy(scen_zones.get(opp_str) or {})
    solver_scen = copy.deepcopy(scen_zones.get(seat_str) or {})

    public_counts = _collect_public_opp_cards(opp_scen)
    known_hand = [str(c) for c in (orig_opp_cfg.get("known_hand") or []) if isinstance(c, str)]
    for cid in known_hand:
        if not _is_token_card_id(cid):
            public_counts[cid] = max(public_counts[cid], known_hand.count(cid))

    security_pinned = bool(orig_opp_cfg.get("security_pinned", False))
    pinned_sec: list[str] = []
    if security_pinned:
        for sec in opp_scen.get("security") or []:
            cid = sec.get("card_id") if isinstance(sec, dict) else sec
            if isinstance(cid, str):
                pinned_sec.append(cid)
                if not _is_token_card_id(cid):
                    public_counts[cid] += 1

    if orig_opp_cfg.get("hand_count") is not None:
        hand_size = int(orig_opp_cfg["hand_count"])
    else:
        hand_size = len(opp_scen.get("hand") or [])

    if orig_opp_cfg.get("security_count") is not None:
        security_size = int(orig_opp_cfg["security_count"])
    else:
        security_size = len(opp_scen.get("security") or [])

    egg_target_len = len(opp_scen.get("egg_deck") or [])
    known_egg_ids = set(str(c) for c in (opp_scen.get("egg_deck") or []) if isinstance(c, str))

    prior = getattr(puzzle, "prior", None)
    raw_candidates = (prior.get("candidates") if isinstance(prior, dict) else None) or []
    compatible: list[tuple[dict[str, Any], float]] = []
    for cand in raw_candidates:
        if not isinstance(cand, dict):
            continue
        w = float(cand.get("weight", 1.0))
        cards = [str(c) for c in (cand.get("cards") or [])]
        if w <= 0.0 or not cards:
            continue
        cand_counts = Counter(cards)
        if all(cand_counts[cid] >= req for cid, req in public_counts.items()):
            compatible.append((cand, w))

    if not compatible:
        fallback_cards = list(puzzle.decks.get(opp_str) or DEFAULT_FILLER_DECK)
        compatible = [({"cards": fallback_cards, "weight": 1.0}, 1.0)]

    total_w = sum(w for _, w in compatible)
    cum_probs: list[float] = []
    acc = 0.0
    for _, w in compatible:
        acc += w / total_w
        cum_probs.append(acc)
    cum_probs[-1] = 1.0

    # Stratified low-variance resampling across W worlds so candidate proportions match weights faithfully.
    u0 = rng.random()
    strata = [(u0 + i) / float(belief_worlds) for i in range(belief_worlds)]
    rng.shuffle(strata)

    worlds: list[StudioPuzzle] = []
    for w_idx, u in enumerate(strata):
        chosen_cand = compatible[-1][0]
        for (cand, _), cp in zip(compatible, cum_probs):
            if u <= cp:
                chosen_cand = cand
                break

        opp_deck = [str(c) for c in (chosen_cand.get("cards") or puzzle.decks.get(opp_str) or DEFAULT_FILLER_DECK)]
        rem = Counter(opp_deck) - public_counts
        egg_pool: list[str] = []
        main_pool: list[str] = []
        for cid, cnt in rem.items():
            if cnt <= 0:
                continue
            if cid in known_egg_ids or _is_digi_egg(cid):
                egg_pool.extend([cid] * cnt)
            else:
                main_pool.extend([cid] * cnt)
        rng.shuffle(egg_pool)
        rng.shuffle(main_pool)

        if isinstance(chosen_cand.get("hand"), list):
            world_hand = [str(c) for c in chosen_cand["hand"]]
            for c in world_hand:
                if c in main_pool:
                    main_pool.remove(c)
        else:
            need_h = max(0, hand_size - len(known_hand))
            drawn_h = main_pool[:need_h]
            main_pool = main_pool[need_h:]
            world_hand = list(known_hand) + drawn_h

        if isinstance(chosen_cand.get("security"), list):
            world_sec: list[Any] = list(chosen_cand["security"])
            for sec in world_sec:
                c = sec.get("card_id") if isinstance(sec, dict) else sec
                if isinstance(c, str) and c in main_pool:
                    main_pool.remove(c)
        elif security_pinned:
            world_sec = copy.deepcopy(opp_scen.get("security") or [])
        else:
            world_sec = main_pool[:security_size]
            main_pool = main_pool[security_size:]

        world_opp_zones = copy.deepcopy(opp_scen)
        world_opp_zones["hand"] = world_hand
        world_opp_zones["security"] = world_sec
        world_opp_zones["deck_top"] = main_pool
        if egg_target_len > 0 or egg_pool:
            world_opp_zones["egg_deck"] = egg_pool[:egg_target_len] if egg_target_len > 0 else egg_pool

        w_puz = copy.deepcopy(puzzle)
        w_puz.seed = int(puzzle.seed) + w_idx
        w_puz.decks = {
            seat_str: list(puzzle.decks[seat_str]),
            opp_str: opp_deck,
        }
        w_puz.setup = {
            "kind": "staged",
            "state": copy.deepcopy(root_scen.get("state") or {}),
            "zones": {
                seat_str: copy.deepcopy(solver_scen),
                opp_str: world_opp_zones,
            },
            "prefix_actions": [],
        }
        worlds.append(w_puz)
    return worlds


def _check_puzzle_assertion(game: Any, assertion: dict[str, Any], default_seat: int) -> bool:
    if assertion.get("kind") == "winner":
        return bool(game.is_game_over) and int(game.winner_id or 0) == int(assertion.get("player", default_seat))
    return _evaluate_goal_assertion(game, assertion)


def _advance_interrupts_with_eval(
    game: Any,
    seat: int,
    triggers: Sequence[dict[str, Any]],
    onnx_eval: Optional[_OnnxBeliefEvaluator],
) -> bool:
    guard = 0
    while not game.is_game_over and int(game.current_player_id) != int(seat):
        if guard >= 64:
            return False
        guard += 1
        mask = game.get_action_mask()
        act = select_refutation_action(game, mask, triggers)
        if act is None:
            if onnx_eval is not None:
                act = onnx_eval.predict_action(game, int(game.current_player_id), mask)
            else:
                act = int(game.greedy_action())
        if mask[int(act)] <= 0.5:
            valid = np.flatnonzero(np.asarray(mask) > 0.5)
            if len(valid) == 0:
                return False
            act = int(valid[0])
        game.step(int(act))
    return True


def _board_heuristic_value(game: Any, seat: int) -> float:
    opp = 2 if int(seat) == 1 else 1
    ui = game.to_ui_json()
    p_me = ui.get(f"player{seat}", {})
    p_op = ui.get(f"player{opp}", {})
    sec_diff = float(p_me.get("securityCount", 0)) - float(p_op.get("securityCount", 0))
    me_ba = p_me.get("battleArea") or []
    op_ba = p_op.get("battleArea") or []
    dp_me = sum(float(x.get("dp") or 2000.0) for x in me_ba) / 10000.0
    dp_op = sum(float(x.get("dp") or 2000.0) for x in op_ba) / 10000.0
    return float(np.tanh(0.35 * sec_diff + 0.30 * (dp_me - dp_op)))


def _evaluate_line_in_world(
    world_puzzle: StudioPuzzle,
    line_actions: list[int],
    *,
    observation_profile: str,
    onnx_eval: Optional[_OnnxBeliefEvaluator],
    max_rollout_steps: int = 100,
) -> float:
    seat = int(world_puzzle.seat)
    game = _make_staged_headless_game(world_puzzle, profile=observation_profile)
    goal = world_puzzle.rl_goal or {}
    win_assertions = [a for a in (goal.get("win_assertions") or []) if isinstance(a, dict)]
    non_winner_assertions = [a for a in win_assertions if a.get("kind") != "winner"]

    start_turn = int((game.get_rl_state() or {}).get("turn_count", 1))
    solver_steps = 0
    aborted_mid_line = False

    for act_id in line_actions:
        if not _advance_interrupts_with_eval(game, seat, world_puzzle.refutation_triggers, onnx_eval):
            return -1.0
        if game.is_game_over:
            break
        mask = np.asarray(game.get_action_mask())
        if int(game.current_player_id) != seat or int(act_id) >= len(mask) or mask[int(act_id)] <= 0.5:
            aborted_mid_line = True
            break
        game.step(int(act_id))
        solver_steps += 1
        if non_winner_assertions and all(_check_puzzle_assertion(game, a, seat) for a in win_assertions):
            return 1.0

    if not game.is_game_over:
        _advance_interrupts_with_eval(game, seat, world_puzzle.refutation_triggers, onnx_eval)

    if win_assertions and all(_check_puzzle_assertion(game, a, seat) for a in win_assertions):
        return 1.0
    if game.is_game_over:
        if game.winner_id == seat:
            return 1.0
        if game.winner_id in (None, 0):
            return 0.0
        return -1.0

    max_turns = goal.get("max_turns")
    max_actions = goal.get("max_actions")
    is_bounded = goal.get("mode") == "bounded_horizon"

    for _ in range(max_rollout_steps):
        if game.is_game_over:
            break
        if is_bounded:
            cur_turn = int((game.get_rl_state() or {}).get("turn_count", start_turn))
            if (max_actions is not None and solver_steps >= int(max_actions)) or (
                max_turns is not None and (cur_turn - start_turn) >= int(max_turns)
            ):
                return -1.0
        cur_pid = int(game.current_player_id)
        mask = np.asarray(game.get_action_mask())
        valid = np.flatnonzero(mask > 0.5)
        if len(valid) == 0:
            break
        if cur_pid != seat:
            act = select_refutation_action(game, mask, world_puzzle.refutation_triggers)
            if act is None:
                act = (
                    onnx_eval.predict_action(game, cur_pid, mask)
                    if onnx_eval is not None
                    else int(game.greedy_action())
                )
        else:
            act = (
                onnx_eval.predict_action(game, seat, mask)
                if onnx_eval is not None
                else int(game.greedy_action())
            )
            solver_steps += 1
        if mask[int(act)] <= 0.5:
            act = int(valid[0])
        game.step(int(act))
        if win_assertions and all(_check_puzzle_assertion(game, a, seat) for a in win_assertions):
            return 1.0

    if game.is_game_over:
        if game.winner_id == seat:
            return 1.0
        if game.winner_id in (None, 0):
            return 0.0
        return -1.0

    if onnx_eval is not None:
        v = onnx_eval.predict_value(game, seat)
        if v is not None:
            return v
    if aborted_mid_line or is_bounded or any(a.get("kind") == "winner" for a in win_assertions):
        return -1.0
    return _board_heuristic_value(game, seat)


def _record_belief_trajectory(
    puzzle: StudioPuzzle,
    *,
    observation_profile: str,
    target_visit: float,
    belief_worlds: int,
    rng: random.Random,
    include_forced_selection: bool = True,
    onnx_eval: Optional[_OnnxBeliefEvaluator] = None,
) -> tuple[list[dict[str, Any]], int, int]:
    """Evaluate candidate plans across `belief_worlds` determinized worlds, emitting Bayes-averaged
    `policy_visits` and continuous expected outcome `z_bar in [-1.0, +1.0]`."""
    seat = int(puzzle.seat)
    root_game = _make_staged_headless_game(puzzle, profile=observation_profile)
    if not _advance_interrupts(root_game, seat, puzzle.refutation_triggers):
        raise ValueError(f"{puzzle.id}: opponent interrupt loop before belief root")
    if root_game.is_game_over or int(root_game.current_player_id) != seat:
        raise ValueError(f"{puzzle.id}: invalid belief root state")

    root_scen = root_game.to_scenario()
    if isinstance(root_scen, str):
        root_scen = json.loads(root_scen)

    root_mask = np.asarray(root_game.get_action_mask())
    root_legal = np.flatnonzero(root_mask > 0.5).astype(np.int32)
    root_legal_set = {int(a) for a in root_legal.tolist()}
    if len(root_legal) == 0:
        raise ValueError(f"{puzzle.id}: no legal actions at belief root")

    worlds = _sample_belief_worlds(puzzle, root_scen, max(1, int(belief_worlds)), rng)

    # Collect candidate lines from puzzle.lines, expectations, and optional ONNX top proposals.
    cand_lines: list[dict[str, Any]] = []
    covered_roots: set[int] = set()
    for ln in puzzle.lines:
        acts = [int(a) for a in (ln.get("actions") or [])]
        if acts and acts[0] in root_legal_set:
            cand_lines.append({"role": str(ln.get("role") or "alternative"), "actions": acts})
            covered_roots.add(acts[0])
    for exp in puzzle.expectations:
        plan = [int(a) for a in (exp.get("plan") or [])]
        if plan and plan[0] in root_legal_set and plan[0] not in covered_roots:
            role = "avoid" if str(exp.get("kind")) in ("not_chosen", "avoid") else "alternative"
            cand_lines.append({"role": role, "actions": plan})
            covered_roots.add(plan[0])

    root_obs = np.asarray(root_game.get_board_tensor(seat), dtype=np.float32)
    if onnx_eval is not None:
        logits = onnx_eval.root_logits(root_obs)
        ranked = sorted(
            (a for a in root_legal.tolist() if int(a) != 93),
            key=lambda a: float(logits[int(a)]),
            reverse=True,
        )
        for top_a in ranked[:2]:
            if int(top_a) not in covered_roots:
                cand_lines.append({"role": "onnx", "actions": [int(top_a)]})
                covered_roots.add(int(top_a))

    if not cand_lines:
        raise ValueError(f"{puzzle.id}: no legal candidate lines at belief root")

    n_worlds = len(worlds)
    n_lines = len(cand_lines)
    outcomes = np.zeros((n_worlds, n_lines), dtype=np.float64)
    for w_i, w_puz in enumerate(worlds):
        for m_i, ln in enumerate(cand_lines):
            outcomes[w_i, m_i] = _evaluate_line_in_world(
                w_puz,
                ln["actions"],
                observation_profile=observation_profile,
                onnx_eval=onnx_eval,
            )

    cand_roots = sorted({int(ln["actions"][0]) for ln in cand_lines})
    action_world_q = np.zeros((n_worlds, len(cand_roots)), dtype=np.float64)
    for r_i, root_a in enumerate(cand_roots):
        matching = [m_i for m_i, ln in enumerate(cand_lines) if int(ln["actions"][0]) == root_a]
        action_world_q[:, r_i] = outcomes[:, matching].max(axis=1)

    mean_q_by_idx = action_world_q.mean(axis=0)
    legal_index = {int(a): idx for idx, a in enumerate(root_legal.tolist())}
    visits_per_world = np.zeros((n_worlds, len(root_legal)), dtype=np.float64)
    if len(cand_roots) == 1:
        visits_per_world[:, legal_index[cand_roots[0]]] = float(target_visit)
    else:
        for w_i in range(n_worlds):
            w_best = float(action_world_q[w_i].max())
            winners = [r_i for r_i in range(len(cand_roots)) if action_world_q[w_i, r_i] >= w_best - 1e-6]
            share = float(target_visit) / float(len(winners))
            for r_i in winners:
                visits_per_world[w_i, legal_index[cand_roots[r_i]]] = share

    visits_belief = visits_per_world.mean(axis=0).astype(np.float32)
    vis_sum = float(visits_belief.sum())
    if vis_sum > 0.0:
        cand_probs = np.asarray(
            [float(visits_belief[legal_index[a]]) / vis_sum for a in cand_roots],
            dtype=np.float64,
        )
        z_bar = float(np.clip(np.sum(cand_probs * mean_q_by_idx), -1.0, 1.0))
    else:
        z_bar = float(np.clip(mean_q_by_idx.max(), -1.0, 1.0))

    target_roots = {int(ln["actions"][0]) for ln in cand_lines if ln["role"] == "target"}
    best_cand_idx = max(
        range(len(cand_roots)),
        key=lambda r_i: (
            float(visits_belief[legal_index[cand_roots[r_i]]]),
            1 if cand_roots[r_i] in target_roots else 0,
            float(mean_q_by_idx[r_i]),
        ),
    )
    best_root_action = cand_roots[best_cand_idx]
    best_mean_q = float(mean_q_by_idx[best_cand_idx])

    raw_avoids = _extract_root_avoid_actions(puzzle, root_legal_set, best_root_action)
    q_lookup = {cand_roots[r_i]: float(mean_q_by_idx[r_i]) for r_i in range(len(cand_roots))}
    filtered_avoids = [
        a for a in raw_avoids if q_lookup.get(a, -1.0) <= best_mean_q + 1e-6
    ]

    is_root_pending = isinstance(root_game.get_pending_selection(), dict)
    is_root_forced = len(root_legal) == 1
    rows: list[dict[str, Any]] = []
    forced_count = 1 if is_root_forced else 0
    forced_emitted = 0
    active_chain_rows: list[int] = []

    if not is_root_forced or (is_root_pending and include_forced_selection):
        if is_root_pending:
            active_chain_rows.append(0)
        if is_root_forced:
            forced_emitted += 1
        rows.append(
            {
                "obs": root_obs,
                "actions": root_legal,
                "visits": visits_belief,
                "avoid_actions": (
                    np.zeros(0, dtype=np.int32)
                    if is_root_forced
                    else np.asarray(filtered_avoids, dtype=np.int32)
                ),
                "policy_weight": 0.0 if is_root_forced else 1.0,
                "forced_selection": 1 if is_root_forced else 0,
                "chain_anchor_idx": -1,
                "z": z_bar,
            }
        )

    # Lockstep continuation for deterministic world-invariant steps of the winning line.
    best_lines = [ln for ln in cand_lines if int(ln["actions"][0]) == best_root_action]
    best_lines.sort(key=lambda ln: (1 if ln["role"] == "target" else 0, len(ln["actions"])), reverse=True)
    chosen_actions = best_lines[0]["actions"]

    if len(chosen_actions) > 1:
        live_games = [_make_staged_headless_game(w_puz, profile=observation_profile) for w_puz in worlds]
        for g in live_games:
            _drain_events_and_check_rng(g)
            g.step(int(chosen_actions[0]))
        step_rng = any(_drain_events_and_check_rng(g) for g in live_games)

        for cont_act in chosen_actions[1:]:
            if step_rng:
                active_chain_rows.clear()
            if any(g.is_game_over or int(g.current_player_id) != seat for g in live_games):
                break
            masks = [np.asarray(g.get_action_mask()) for g in live_games]
            legals = [np.flatnonzero(m > 0.5).astype(np.int32) for m in masks]
            l0 = legals[0]
            if int(cont_act) not in {int(x) for x in l0.tolist()}:
                break
            if any(not np.array_equal(l0, lk) for lk in legals[1:]):
                break
            obs_list = [np.asarray(g.get_board_tensor(seat), dtype=np.float32) for g in live_games]
            obs0 = obs_list[0]
            if any(float(np.max(np.abs(obs0 - ok))) > 1e-6 for ok in obs_list[1:]):
                break

            is_pend = isinstance(live_games[0].get_pending_selection(), dict)
            if len(l0) == 1:
                forced_count += 1
                if is_pend and include_forced_selection:
                    r_idx = len(rows)
                    active_chain_rows.append(r_idx)
                    rows.append(
                        {
                            "obs": obs0,
                            "actions": l0,
                            "visits": np.asarray([float(target_visit)], dtype=np.float32),
                            "avoid_actions": np.zeros(0, dtype=np.int32),
                            "policy_weight": 0.0,
                            "forced_selection": 1,
                            "chain_anchor_idx": -1,
                            "z": z_bar,
                        }
                    )
                    forced_emitted += 1
            else:
                r_idx = len(rows)
                if is_pend:
                    active_chain_rows.append(r_idx)
                elif active_chain_rows:
                    for prev_i in active_chain_rows:
                        rows[prev_i]["chain_anchor_idx"] = r_idx
                    active_chain_rows.clear()
                rows.append(
                    {
                        "obs": obs0,
                        "actions": l0,
                        "visits": np.where(l0 == int(cont_act), float(target_visit), 0.0).astype(np.float32),
                        "avoid_actions": np.zeros(0, dtype=np.int32),
                        "policy_weight": 1.0,
                        "forced_selection": 0,
                        "chain_anchor_idx": -1,
                        "z": z_bar,
                    }
                )

            for g in live_games:
                g.step(int(cont_act))
            step_rng = any(_drain_events_and_check_rng(g) for g in live_games)

    if active_chain_rows and rows:
        last_idx = len(rows) - 1
        for prev_i in active_chain_rows:
            if prev_i < last_idx:
                rows[prev_i]["chain_anchor_idx"] = last_idx
        active_chain_rows.clear()

    return rows, forced_count, forced_emitted


def emit_puzzle_shards(
    puzzles_or_exams: Sequence[str | Path | StudioPuzzle | dict[str, Any]]
    | str
    | Path
    | StudioPuzzle
    | dict[str, Any],
    out_dir: str | Path,
    permutations_per_puzzle: int = 16,
    observation_profile: str = "standard_lite_v2",
    seed: int = 0,
    target_visit: float = 100.0,
    max_distractors: int = 2,
    include_forced_selection: bool = True,
    belief_worlds: int = 16,
    onnx_model: str | Path | None = None,
) -> dict[str, Any]:
    """Emit `selfplay-shards-v1` supervised shards (with `forced_selection.npy`,
    `chain_anchor_idx.npy`, optional `avoid_*.npy`, and multi-world belief targets) into `out_dir`."""
    if permutations_per_puzzle < 0:
        raise ValueError(f"permutations_per_puzzle must be >= 0, got {permutations_per_puzzle}")
    if belief_worlds < 1:
        raise ValueError(f"belief_worlds must be >= 1, got {belief_worlds}")

    contract = current_contract(observation_profile)
    obs_dim = int(contract["tensor_size"])
    action_dim = int(contract["action_space_size"])

    puzzles = _resolve_inputs(puzzles_or_exams)
    if not puzzles:
        raise ValueError("No puzzles or search exams with target lines found")

    onnx_eval = _OnnxBeliefEvaluator(onnx_model) if onnx_model is not None else None

    rng = random.Random(seed)
    obs_rows: list[np.ndarray] = []
    policy_offsets: list[int] = [0]
    policy_actions: list[np.ndarray] = []
    policy_visits: list[np.ndarray] = []
    avoid_offsets: list[int] = [0]
    avoid_actions: list[np.ndarray] = []
    z_list: list[float] = []
    terminated_list: list[int] = []
    game_ids: list[int] = []
    movers: list[int] = []
    weights: list[float] = []
    forced_list: list[int] = []
    anchor_list: list[int] = []

    next_game_id = 0
    puzzles_emitted = 0
    belief_puzzles_emitted = 0
    forced_decisions_total = 0
    forced_selection_rows = 0

    for puzzle in puzzles:
        is_belief = _is_belief_puzzle(puzzle, belief_worlds)
        target_lines = [
            ln for ln in puzzle.lines if str(ln.get("role")) == "target" and ln.get("actions")
        ]
        if not target_lines and not (is_belief and puzzle.lines):
            continue
        puzzles_emitted += 1
        if is_belief:
            belief_puzzles_emitted += 1

        can_permute = (
            (puzzle.setup or {}).get("kind", "staged") == "staged"
            and not (puzzle.setup or {}).get("prefix_actions")
            and permutations_per_puzzle > 0
        )
        variants: list[StudioPuzzle] = [puzzle]
        if can_permute:
            for _ in range(permutations_per_puzzle):
                perm_p = permute_staged_puzzle(puzzle, rng, max_distractors=max_distractors)
                variants.append(_copy_belief_metadata(puzzle, perm_p))

        for var_idx, var_puzzle in enumerate(variants):
            mover_id = max(0, min(1, int(var_puzzle.seat) - 1))
            if is_belief:
                try:
                    traj_rows, forced_cnt, forced_emit_cnt = _record_belief_trajectory(
                        var_puzzle,
                        observation_profile=observation_profile,
                        target_visit=target_visit,
                        belief_worlds=belief_worlds,
                        rng=rng,
                        include_forced_selection=include_forced_selection,
                        onnx_eval=onnx_eval,
                    )
                except Exception:
                    if var_idx == 0:
                        raise
                    continue
                if not traj_rows:
                    continue
                forced_decisions_total += forced_cnt
                forced_selection_rows += forced_emit_cnt
                row_base = len(obs_rows)
                for step_row in traj_rows:
                    obs_rows.append(step_row["obs"])
                    acts_arr = step_row["actions"]
                    vis_arr = step_row["visits"]
                    av_arr = step_row["avoid_actions"]
                    policy_actions.append(acts_arr)
                    policy_visits.append(vis_arr)
                    policy_offsets.append(policy_offsets[-1] + int(len(acts_arr)))
                    avoid_actions.append(av_arr)
                    avoid_offsets.append(avoid_offsets[-1] + int(len(av_arr)))
                    z_list.append(float(step_row["z"]))
                    terminated_list.append(1)
                    game_ids.append(next_game_id)
                    movers.append(mover_id)
                    weights.append(float(step_row["policy_weight"]))
                    forced_list.append(int(step_row["forced_selection"]))
                    local_anc = int(step_row["chain_anchor_idx"])
                    anchor_list.append((row_base + local_anc) if local_anc >= 0 else -1)
                next_game_id += 1
                continue

            var_targets = [
                ln for ln in var_puzzle.lines if str(ln.get("role")) == "target" and ln.get("actions")
            ]
            for t_ln in var_targets:
                t_acts = [int(a) for a in t_ln["actions"]]
                try:
                    traj_rows, forced_cnt, forced_emit_cnt = _record_target_trajectory(
                        var_puzzle,
                        t_acts,
                        observation_profile=observation_profile,
                        target_visit=target_visit,
                        include_forced_selection=include_forced_selection,
                    )
                except Exception:
                    if var_idx == 0:
                        raise
                    continue
                if not traj_rows:
                    continue
                forced_decisions_total += forced_cnt
                forced_selection_rows += forced_emit_cnt
                row_base = len(obs_rows)
                for step_row in traj_rows:
                    obs_rows.append(step_row["obs"])
                    acts_arr = step_row["actions"]
                    vis_arr = step_row["visits"]
                    av_arr = step_row["avoid_actions"]
                    policy_actions.append(acts_arr)
                    policy_visits.append(vis_arr)
                    policy_offsets.append(policy_offsets[-1] + int(len(acts_arr)))
                    avoid_actions.append(av_arr)
                    avoid_offsets.append(avoid_offsets[-1] + int(len(av_arr)))
                    z_list.append(float(step_row["z"]))
                    terminated_list.append(1)
                    game_ids.append(next_game_id)
                    movers.append(mover_id)
                    weights.append(float(step_row["policy_weight"]))
                    forced_list.append(int(step_row["forced_selection"]))
                    local_anc = int(step_row["chain_anchor_idx"])
                    anchor_list.append((row_base + local_anc) if local_anc >= 0 else -1)
                next_game_id += 1

    total_rows = len(obs_rows)
    if total_rows == 0:
        raise ValueError("No valid trajectory rows emitted from supplied puzzles")

    out_path = Path(out_dir)
    shard_dir = out_path / "shard-000"
    shard_dir.mkdir(parents=True, exist_ok=True)

    obs_np = np.stack(obs_rows, axis=0).astype(np.float32)
    if obs_np.shape != (total_rows, obs_dim):
        raise RuntimeError(f"Unexpected obs shape {obs_np.shape}, expected {(total_rows, obs_dim)}")

    pol_off_np = np.asarray(policy_offsets, dtype=np.int64)
    pol_act_np = np.concatenate(policy_actions).astype(np.int32)
    pol_vis_np = np.concatenate(policy_visits).astype(np.float32)
    z_np = np.asarray(z_list, dtype=np.float32)
    term_np = np.asarray(terminated_list, dtype=np.uint8)
    gid_np = np.asarray(game_ids, dtype=np.int32)
    mover_np = np.asarray(movers, dtype=np.uint8)
    weight_np = np.asarray(weights, dtype=np.float32)
    forced_np = np.asarray(forced_list, dtype=np.uint8)
    anchor_np = np.asarray(anchor_list, dtype=np.int64)

    np.save(shard_dir / "obs.npy", obs_np)
    np.save(shard_dir / "policy_offsets.npy", pol_off_np)
    np.save(shard_dir / "policy_actions.npy", pol_act_np)
    np.save(shard_dir / "policy_visits.npy", pol_vis_np)
    np.save(shard_dir / "z.npy", z_np)
    np.save(shard_dir / "terminated.npy", term_np)
    np.save(shard_dir / "game_id.npy", gid_np)
    np.save(shard_dir / "mover.npy", mover_np)
    np.save(shard_dir / "policy_weight.npy", weight_np)
    np.save(shard_dir / "forced_selection.npy", forced_np)
    np.save(shard_dir / "chain_anchor_idx.npy", anchor_np)

    av_off_np = np.asarray(avoid_offsets, dtype=np.int64)
    av_act_np = (
        np.concatenate(avoid_actions).astype(np.int32)
        if avoid_offsets[-1] > 0
        else np.zeros(0, dtype=np.int32)
    )
    if len(av_act_np) > 0:
        np.save(shard_dir / "avoid_offsets.npy", av_off_np)
        np.save(shard_dir / "avoid_actions.npy", av_act_np)

    manifest = {
        "schema": SCHEMA,
        "source": "emit_puzzle_shards",
        "evaluation_only": False,
        "information": {
            "regime": "open_decklist",
            "knowledge_schema": 2,
            "prior_hash": None,
            "legality_hash": None,
            "prior_provenance": None,
        },
        "obs_dim": obs_dim,
        "action_dim": action_dim,
        "observation_profile": observation_profile,
        "model_contract": contract,
        "value_target": "terminal_outcome",
        "seed": int(seed),
        "puzzles": int(puzzles_emitted),
        "belief_puzzles": int(belief_puzzles_emitted),
        "belief_worlds": int(belief_worlds),
        "permutations_per_puzzle": int(permutations_per_puzzle),
        "games": int(next_game_id),
        "rows": int(total_rows),
        "avoid_edges": int(len(av_act_np)),
        "forced_decisions_total": int(forced_decisions_total),
        "forced_selection_rows": int(forced_selection_rows),
        "chain_anchored_rows": int(np.sum(anchor_np >= 0)),
        "shards": [{"dir": "shard-000", "rows": int(total_rows)}],
    }
    manifest_path = out_path / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    p.add_argument(
        "--inputs",
        type=Path,
        nargs="+",
        required=True,
        help="studio-puzzle-v1 / search-exam-v1 JSON files or directories",
    )
    p.add_argument("--out", type=Path, required=True, help="Output selfplay-shards-v1 directory")
    p.add_argument("--permutations-per-puzzle", type=int, default=16)
    p.add_argument("--observation-profile", type=str, default="standard_lite_v2")
    p.add_argument("--seed", type=int, default=0)
    p.add_argument("--target-visit", type=float, default=100.0)
    p.add_argument("--max-distractors", type=int, default=2)
    p.add_argument(
        "--belief-worlds",
        type=int,
        default=16,
        help="Number of determinized opponent worlds to sample for belief puzzles with priors",
    )
    p.add_argument(
        "--onnx-model",
        type=Path,
        default=None,
        help="Optional ONNX policy/value model for multi-world belief evaluation",
    )
    p.add_argument(
        "--include-forced-selection",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Emit choiceless PendingSelection steps with policy_weight=0 and forced_selection=1",
    )
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    summary = emit_puzzle_shards(
        puzzles_or_exams=list(args.inputs),
        out_dir=args.out,
        permutations_per_puzzle=args.permutations_per_puzzle,
        observation_profile=args.observation_profile,
        seed=args.seed,
        target_visit=args.target_visit,
        max_distractors=args.max_distractors,
        include_forced_selection=args.include_forced_selection,
        belief_worlds=args.belief_worlds,
        onnx_model=args.onnx_model,
    )
    print(
        f"emit_puzzle_shards: wrote {summary['rows']} rows across {summary['games']} trajectories "
        f"({summary['avoid_edges']} avoid edges, {summary['forced_selection_rows']} forced-selection rows, "
        f"{summary['chain_anchored_rows']} anchored rows) -> {args.out}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
