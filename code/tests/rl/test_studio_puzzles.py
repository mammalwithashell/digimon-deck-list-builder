"""Tests for RL Studio Puzzle loader, permutation augmentation, scheduler, and PuzzleResetWrapper."""

from __future__ import annotations

import json
import random
from pathlib import Path
from types import SimpleNamespace

import numpy as np
import pytest
from sb3_contrib.common.wrappers import ActionMasker

from digimon_gym.agents.studio_puzzles import (
    AdaptivePuzzleScheduler,
    PuzzleResetWrapper,
    StudioPuzzle,
    _find_wrapper_by_name,
    _make_staged_headless_game,
    load_studio_puzzle,
    load_studio_puzzles_dir,
    permute_staged_puzzle,
    select_refutation_action,
)
from digimon_gym.digimon_gym import DigimonEnv

FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


def _write_puzzle(
    path: Path,
    puzzle_id: str,
    folder: str,
    tags: list[str],
    rl_goal: dict,
    *,
    zones: dict | None = None,
    lines: list[dict] | None = None,
    refutation_triggers: list[dict] | None = None,
) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema": "studio-puzzle-v1",
        "id": puzzle_id,
        "title": f"Puzzle {puzzle_id}",
        "description": "test",
        "folder": folder,
        "tags": tags,
        "source": {"kind": "composed"},
        "decks": {"1": FILLER_DECK, "2": FILLER_DECK},
        "seed": 11,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": zones
            or {
                "1": {"hand": ["ST1-03"], "field": [], "security": ["ST1-03"]},
                "2": {"hand": ["ST1-03"], "field": [], "security": ["ST1-03"]},
            },
            "prefix_actions": [],
        },
        "lines": lines or [],
        "expectations": [],
        "assertions": {"engine": [], "ui": []},
        "rl_goal": rl_goal,
    }
    if refutation_triggers is not None:
        doc["refutation_triggers"] = refutation_triggers
    path.write_text(json.dumps(doc), encoding="utf-8")
    return path


def test_recursive_puzzle_loading_with_folder_and_tag_filters(tmp_path: Path):
    _write_puzzle(
        tmp_path / "examon" / "lethal" / "p1.json",
        "p1",
        "examon/lethal",
        ["lethal", "examon"],
        {"mode": "playout_to_win"},
        lines=[{"id": "l1", "name": "main-lethal", "role": "target", "actions": [0, 62]}],
        refutation_triggers=[
            {
                "source_card_id": "EX12-034",
                "phase": None,
                "accept_optional": True,
                "prefer_label_contains": "Play",
            }
        ],
    )
    _write_puzzle(
        tmp_path / "control" / "p2.json",
        "p2",
        "control",
        ["defense"],
        {"mode": "playout_to_win"},
    )
    (tmp_path / "_library.json").write_text(json.dumps({"version": 1, "folders": [], "items": {}}))

    all_p = load_studio_puzzles_dir(tmp_path)
    assert [p.id for p in all_p] == ["p2", "p1"] or {p.id for p in all_p} == {"p1", "p2"}

    ex_only = load_studio_puzzles_dir(tmp_path, folder="examon")
    assert [p.id for p in ex_only] == ["p1"]
    assert ex_only[0].lines == [
        {"id": "l1", "name": "main-lethal", "role": "target", "actions": [0, 62]}
    ]
    assert ex_only[0].refutation_triggers[0]["source_card_id"] == "EX12-034"

    tag_only = load_studio_puzzles_dir(tmp_path, tags=["defense"])
    assert [p.id for p in tag_only] == ["p2"]


def test_puzzle_reset_wrapper_playout_and_bounded_horizon(tmp_path: Path):
    p_playout = load_studio_puzzle(
        _write_puzzle(
            tmp_path / "playout.json",
            "playout-1",
            "",
            ["test"],
            {"mode": "playout_to_win"},
        )
    )
    p_bounded = load_studio_puzzle(
        _write_puzzle(
            tmp_path / "bounded.json",
            "bounded-1",
            "",
            ["test"],
            {
                "mode": "bounded_horizon",
                "max_actions": 1,
                "max_turns": None,
                "win_assertions": [{"kind": "zone_count", "player": 1, "zone": "field", "value": 99}],
            },
        )
    )

    env = DigimonEnv()
    wrapped = PuzzleResetWrapper(env, [p_playout], puzzle_prob=1.0, seed=0)
    obs, info = wrapped.reset(seed=0)
    assert obs.shape == env.observation_space.shape
    assert info["studio_puzzle_id"] == "playout-1"
    mask = wrapped.action_mask()
    assert mask.shape == (env.action_space.n,)
    assert int(mask[62]) == 1  # PASS is legal in Main phase

    # Bounded horizon: unsatisfied win_assertions expires at max_actions=1 -> -1.0
    wrapped_b = PuzzleResetWrapper(DigimonEnv(), [p_bounded], puzzle_prob=1.0, seed=0)
    obs_b, info_b = wrapped_b.reset(seed=0)
    assert info_b["studio_puzzle_goal_mode"] == "bounded_horizon"
    _, reward, terminated, truncated, step_info = wrapped_b.step(62)
    assert terminated is True
    assert truncated is False
    assert reward == pytest.approx(-1.0)
    assert step_info.get("studio_puzzle_horizon_expired") is True
    stats = wrapped_b.get_puzzle_stats()
    assert stats["bounded-1"]["attempts"] == 1
    assert stats["bounded-1"]["solves"] == 0


def test_select_refutation_action_matching_rules():
    mask = np.zeros(2192, dtype=np.int8)
    for aid in (62, 1000, 1001, 1010):
        mask[aid] = 1

    class FakeRunner:
        def get_rl_state(self):
            return {"phase": "EffectChoice"}

        def get_pending_selection(self):
            return {
                "kind": "EffectChoice",
                "phase": "Main",
                "selectingPlayer": 2,
                "validIndices": [62, 1000, 1001, 1010],
                "isOptional": True,
                "prompt": "Resolve EX12-034 Erlangmon interrupt",
            }

        def legal_decoded_actions(self):
            return [
                {"action_id": 62, "card_id": None, "label": "Pass / decline", "phase": "EffectChoice"},
                {"action_id": 1000, "card_id": "EX12-034", "label": "Return 1 Digimon to hand", "phase": "EffectChoice"},
                {"action_id": 1001, "card_id": "EX12-034", "label": "Play 1 Digimon from trash", "phase": "EffectChoice"},
                {"action_id": 1010, "card_id": "EX12-071", "label": "Activate Delay", "phase": "EffectChoice"},
            ]

    runner = FakeRunner()

    # 1. prefer_label_contains selects matching label case-insensitively on EX12-034.
    act = select_refutation_action(
        runner,
        mask,
        [{"source_card_id": "EX12-034", "phase": None, "accept_optional": True, "prefer_label_contains": "play"}],
    )
    assert act == 1001

    # 2. accept_optional=True without prefer_label_contains chooses first legal non-pass for that card.
    act2 = select_refutation_action(
        runner,
        mask,
        [{"source_card_id": "EX12-071", "phase": "EffectChoice", "accept_optional": True, "prefer_label_contains": None}],
    )
    assert act2 == 1010

    # 3. accept_optional=False declines via action 62 when legal.
    act3 = select_refutation_action(
        runner,
        mask,
        [{"source_card_id": "EX12-034", "phase": None, "accept_optional": False, "prefer_label_contains": None}],
    )
    assert act3 == 62

    # 4. Non-matching phase or card returns None so caller falls back to opponent policy.
    assert (
        select_refutation_action(
            runner,
            mask,
            [{"source_card_id": "EX12-034", "phase": "Breeding", "accept_optional": True}],
        )
        is None
    )
    assert (
        select_refutation_action(
            runner,
            mask,
            [{"source_card_id": "BT99-999", "phase": None, "accept_optional": True}],
        )
        is None
    )


def test_permute_staged_puzzle_remaps_hand_field_lines_and_assertions():
    deck1 = ["ST1-01"] * 4 + ["ST1-03"] * 25 + ["ST1-07"] * 25
    deck2 = list(FILLER_DECK)
    puzzle = StudioPuzzle(
        id="perm-test",
        title="Permute Test",
        folder="",
        tags=[],
        decks={"1": deck1, "2": deck2},
        seed=7,
        seat=1,
        setup={
            "kind": "staged",
            "state": {"memory": 5, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "hand": ["ST1-07", "ST1-03"],  # Greymon at 0, Agumon at 1
                    "field": [
                        {"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1},  # unsuspended Agumon at 0
                        {"stack": ["ST1-07"], "is_suspended": True, "turn_played": 1},   # suspended Greymon at 1
                    ],
                    "security": ["ST1-03"],
                    "trash": [],
                },
                "2": {
                    "hand": [],
                    "field": [],
                    "security": [],
                    "trash": [],
                },
            },
            "prefix_actions": [],
        },
        rl_goal={
            "mode": "bounded_horizon",
            "max_actions": 3,
            "win_assertions": [{"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "ST1-07"}],
        },
        # Target: digivolve Greymon (hand 0) onto Agumon (slot 0) -> 400 + 0*15 + 0 = 400, then swing lethal 114.
        # Avoid: play Agumon from hand 1 -> action 1.
        lines=[
            {"id": "win", "name": "evo-and-lethal", "role": "target", "actions": [400, 114]},
            {"id": "bad", "name": "hard-play-agumon", "role": "avoid", "actions": [1]},
        ],
    )

    seen_target_roots: set[int] = set()
    seen_field_orders: set[tuple[str, ...]] = set()
    rng = random.Random(42)

    for _ in range(16):
        permuted = permute_staged_puzzle(puzzle, rng, max_distractors=2)
        z1 = permuted.setup["zones"]["1"]
        # Digi-Eggs must never leak into hand or trash.
        assert "ST1-01" not in z1.get("hand", [])
        assert "ST1-01" not in z1.get("trash", [])

        seen_field_orders.add(tuple(st["stack"][-1] for st in z1["field"]))
        tgt_actions = next(ln["actions"] for ln in permuted.lines if ln["role"] == "target")
        avoid_actions = next(ln["actions"] for ln in permuted.lines if ln["role"] == "avoid")
        seen_target_roots.add(tgt_actions[0])

        # Verify every remapped action executes legally to lethal win on RustHeadlessGame.
        g = _make_staged_headless_game(permuted)
        root_mask = g.get_action_mask()
        assert root_mask[avoid_actions[0]] > 0.5
        for aid in tgt_actions:
            assert g.get_action_mask()[aid] > 0.5
            g.step(aid)
        assert g.is_game_over is True
        assert g.winner_id == 1

    assert len(seen_field_orders) == 2
    assert len(seen_target_roots) >= 2


def test_adaptive_puzzle_scheduler_prioritizes_unsolved_with_floor():
    p_easy = StudioPuzzle(
        id="easy",
        title="Easy",
        folder="",
        tags=[],
        decks={"1": FILLER_DECK, "2": FILLER_DECK},
        seed=0,
        seat=1,
        setup={"kind": "staged", "zones": {"1": {}, "2": {}}},
    )
    p_hard = StudioPuzzle(
        id="hard",
        title="Hard",
        folder="",
        tags=[],
        decks={"1": FILLER_DECK, "2": FILLER_DECK},
        seed=0,
        seat=1,
        setup={"kind": "staged", "zones": {"1": {}, "2": {}}},
    )
    sched = AdaptivePuzzleScheduler([p_easy, p_hard], ema_alpha=0.2, gamma=2.0, floor=0.05, seed=123)
    assert sched.weight_for("easy") == pytest.approx(1.05)
    assert sched.weight_for("hard") == pytest.approx(1.05)

    for _ in range(25):
        sched.record_outcome("easy", solved=True)
        sched.record_outcome("hard", solved=False)

    stats = sched.get_stats()
    assert stats["easy"]["ema_solve_rate"] > 0.95
    assert stats["hard"]["ema_solve_rate"] == pytest.approx(0.0)
    assert stats["hard"]["weight"] > stats["easy"]["weight"]
    assert stats["easy"]["weight"] >= 0.05

    rng = random.Random(99)
    draws = [sched.sample(rng).id for _ in range(400)]
    hard_share = draws.count("hard") / len(draws)
    assert hard_share > 0.85
    # Mastered puzzle retains its maintenance floor (>0 samples).
    assert draws.count("easy") > 0


def test_opponent_wrapper_uses_refutation_triggers_and_pilot_training_wiring(monkeypatch, tmp_path: Path):
    from digimon_gym.agents import pilot_training

    # Stage a puzzle where P1 passes turn (62 -> P2 gets 3 memory) and P2 acts in Main with
    # ST1-07 Greymon (play cost 5) in hand; refutation_triggers instructs P2 to play ST1-07
    # (3 - 5 = -2 memory, immediately returning turn to P1 with 1 Digimon on P2's field),
    # whereas fallback opponent_fn (62) would immediately pass without playing anything.
    deck_with_greymon = ["ST1-01"] * 4 + ["ST1-03"] * 45 + ["ST1-07"] * 5
    p_path = tmp_path / "puzzles" / "refute.json"
    p_path.parent.mkdir(parents=True, exist_ok=True)
    p_path.write_text(
        json.dumps(
            {
                "schema": "studio-puzzle-v1",
                "id": "refute-1",
                "title": "Refute 1",
                "description": "test",
                "folder": "",
                "tags": ["interrupt"],
                "source": {"kind": "composed"},
                "decks": {"1": deck_with_greymon, "2": deck_with_greymon},
                "seed": 11,
                "seat": 1,
                "setup": {
                    "kind": "staged",
                    "state": {"memory": 1, "phase": "Main", "turn": 3, "first_player": 1},
                    "zones": {
                        "1": {"hand": ["ST1-03"], "field": [], "security": ["ST1-03"]},
                        "2": {"hand": ["ST1-07"], "field": [], "security": ["ST1-03"]},
                    },
                    "prefix_actions": [],
                },
                "lines": [],
                "expectations": [],
                "assertions": {"engine": [], "ui": []},
                "rl_goal": {
                    "mode": "bounded_horizon",
                    "max_actions": 1,
                    "win_assertions": [{"kind": "zone_count", "player": 2, "zone": "field", "value": 1}],
                },
                "refutation_triggers": [
                    {
                        "source_card_id": "ST1-07",
                        "phase": "Main",
                        "accept_optional": True,
                        "prefer_label_contains": "Play",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    puzzles = load_studio_puzzles_dir(tmp_path / "puzzles")
    assert len(puzzles) == 1

    # Fallback opponent would always pass (62), never playing ST1-07 onto P2's field.
    base_env = DigimonEnv()
    opp_env = pilot_training.OpponentWrapper(base_env, opponent_fn=lambda _e: 62)
    wrapped = PuzzleResetWrapper(opp_env, puzzles, puzzle_prob=1.0, permute=False, adaptive=True, seed=0)
    obs, info = wrapped.reset(seed=0)
    assert info["studio_puzzle_id"] == "refute-1"
    _, reward, terminated, _, step_info = wrapped.step(62)
    assert terminated is True
    assert reward == pytest.approx(1.0)
    assert step_info.get("studio_puzzle_solved") is True
    assert wrapped.get_puzzle_stats()["refute-1"]["solves"] == 1

    # Verify pilot_training.make_env wraps PuzzleResetWrapper inside ActionMasker (even under match_format="bo3").
    train_env = pilot_training.make_env(
        opponent="greedy",
        match_format="bo3",
        studio_puzzles=puzzles,
        puzzle_prob=1.0,
        puzzle_permute=True,
    )
    assert isinstance(train_env, ActionMasker)
    puzzle_wrapper = _find_wrapper_by_name(train_env, "PuzzleResetWrapper")
    assert puzzle_wrapper is not None
    obs_t, info_t = train_env.reset(seed=5)
    assert obs_t.shape == train_env.observation_space.shape
    assert info_t["studio_puzzle_id"] == "refute-1"

    # Verify CLI flags --studio-puzzles-dir / --puzzle-prob / --no-puzzle-permute reach train_pilot / train.
    cfg_yaml = tmp_path / "train.yaml"
    cfg_yaml.write_text("timesteps: 1\neval_freq: 0\neval_episodes: 1\n", encoding="utf-8")
    captured = {}

    def fake_train(**kwargs):
        captured.update(kwargs)
        return SimpleNamespace()

    monkeypatch.setattr(pilot_training, "train", fake_train)
    monkeypatch.setattr(
        "sys.argv",
        [
            "pilot_training",
            "--config",
            str(cfg_yaml),
            "--studio-puzzles-dir",
            str(tmp_path / "puzzles"),
            "--puzzle-prob",
            "0.25",
            "--no-puzzle-permute",
        ],
    )
    pilot_training.main()
    assert captured["studio_puzzles_dir"] == str(tmp_path / "puzzles")
    assert captured["puzzle_prob"] == pytest.approx(0.25)
    assert captured["puzzle_permute"] is False
    assert pilot_training.train_pilot is fake_train or callable(pilot_training.train_pilot)

