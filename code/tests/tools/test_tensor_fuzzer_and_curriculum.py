"""Tests for Track C (`play_in.py`, `tensor_fuzzer.py`, `auto_curriculum.py`, and `cli.py`).

Covers:
1. Live `run_tensor_completeness_fuzz` against the real `digimon_engine` Rust
   backend across all 5 core rule families (`standard_lite_v2` &
   `standard_lite_deck_v2`) + live `digimon_training_mcp.whitebox.plasticity_probe`
   integration.
2. `compile_play_in_with_fallback` (`play_in.py`): direct MCP `compile_play_in`
   hit AND multi-attempt `Role.AUTHOR` LLM fallback with real `RustHeadlessGame`
   seed-deck stacking, functional-equivalence checking, hand-shift line
   remapping, `verify_puzzle`, and symbolic DCGO YAML generation.
3. `audit_card_set_fidelity` (`tensor_fuzzer.py` Part B) with `MockMcpTransport`
   (`load_puzzle`, `step`, `events`, `close_game`) + scripted `LLMRouter`.
4. `run_auto_curriculum` (`auto_curriculum.py`) end-to-end over both
   `anchored_eval` and `specialist-gate-v1` inputs, proving `learnable=True`
   emits SHA256-stamped `selfplay-shards-v1` (`curriculum_manifest.json`) while
   `learnable=False` lands in `tensor_blindness_quarantine.json`.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Callable

import gymnasium as gym
import numpy as np
import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
CODE_ROOT = REPO_ROOT / "code"
TRAINING_MCP_SRC = CODE_ROOT / "digimon-training-mcp" / "src"
for _p in (str(CODE_ROOT), str(TRAINING_MCP_SRC)):
    if _p not in sys.path:
        sys.path.insert(0, _p)

from digimon_gym.agents.azero_trainer import load_shards  # noqa: E402
from digimon_gym.agents.studio_puzzles import load_studio_puzzle  # noqa: E402
from tools.puzzle_lab import PUZZLE_SCHEMA, validate_studio_puzzle_doc  # noqa: E402
from tools.puzzle_lab.auto_curriculum import (  # noqa: E402
    CURRICULUM_MANIFEST_SCHEMA,
    QUARANTINE_SCHEMA,
    ingest_eval_matchups,
    run_auto_curriculum,
    select_bottom_k_matchups,
)
from tools.puzzle_lab.cli import build_parser, main as cli_main  # noqa: E402
from tools.puzzle_lab.llm_router import (  # noqa: E402
    ChatMessage,
    LLMProvider,
    LLMResponse,
    LLMRouter,
    ToolCall,
)
from tools.puzzle_lab.mcp_client import MockMcpTransport, StdioMcpClient  # noqa: E402
from tools.puzzle_lab.play_in import (  # noqa: E402
    compile_play_in_with_fallback,
    stack_deck_for_seed,
)
from tools.puzzle_lab.tensor_fuzzer import (  # noqa: E402
    CORE_RULE_FAMILIES,
    audit_card_set_fidelity,
    resolve_implemented_audit_cards,
    run_tensor_completeness_fuzz,
)

FILLER_DECK = ["ST1-01"] * 4 + ["ST1-03"] * 50


class ScriptedLLMProvider(LLMProvider):
    """Deterministic scripted `LLMProvider` for offline multi-turn tests."""

    is_closed_weight = False

    def __init__(
        self,
        steps: list[Callable[[list[ChatMessage]], LLMResponse] | LLMResponse],
        *,
        model: str = "scripted-mock",
    ) -> None:
        self.model = model
        self._steps = list(steps)
        self.calls: list[dict[str, Any]] = []

    def complete(
        self,
        messages,
        *,
        tools=None,
        json_schema=None,
        schema_name="response",
        temperature=0.2,
        max_tokens=4096,
    ) -> LLMResponse:
        msg_list = list(messages)
        self.calls.append(
            {
                "messages": msg_list,
                "tools": list(tools or []),
                "json_schema": json_schema,
                "schema_name": schema_name,
            }
        )
        if not self._steps:
            raise AssertionError("ScriptedLLMProvider exhausted all scripted responses")
        nxt = self._steps.pop(0)
        return nxt(msg_list) if callable(nxt) else nxt


def _build_tiny_mlp_checkpoint(zip_path: Path, profile_id: str = "standard_lite_v2") -> Path:
    from digimon_engine import ACTION_SPACE_SIZE
    from digimon_gym.agents.features_extractor import CardEmbeddingExtractor
    from digimon_gym.tensor_profiles import get_tensor_profile
    from model_contract import current_contract
    from sb3_contrib import MaskablePPO

    profile = get_tensor_profile(profile_id)

    class _DummyMaskedEnv(gym.Env):
        def __init__(self):
            super().__init__()
            self.observation_space = gym.spaces.Box(
                low=-10.0, high=20001.0, shape=(profile.tensor_size,), dtype=np.float32
            )
            self.action_space = gym.spaces.Discrete(ACTION_SPACE_SIZE)

        def reset(self, *, seed=None, options=None):
            super().reset(seed=seed)
            return np.zeros(profile.tensor_size, dtype=np.float32), {}

        def step(self, action):
            return np.zeros(profile.tensor_size, dtype=np.float32), 0.0, True, False, {}

        def action_masks(self):
            m = np.zeros(ACTION_SPACE_SIZE, dtype=bool)
            m[62] = True
            return m

    model = MaskablePPO(
        "MlpPolicy",
        _DummyMaskedEnv(),
        policy_kwargs={
            "features_extractor_class": CardEmbeddingExtractor,
            "features_extractor_kwargs": {"features_dim": 64, "tensor_profile_id": profile_id},
            "net_arch": dict(pi=[32], vf=[32]),
        },
        n_steps=8,
        batch_size=8,
        device="cpu",
    )
    model.digimon_contract = current_contract(profile_id)
    model.save(str(zip_path))
    return zip_path


# ── 1. Live Zero-LLM Tensor Completeness Fuzzer Tests ─────────────────────────


def test_run_tensor_completeness_fuzz_live_across_all_5_families(tmp_path: Path):
    json_out = tmp_path / "fuzz_report.json"
    md_out = tmp_path / "fuzz_report.md"

    report = run_tensor_completeness_fuzz(
        profile="standard_lite_v2",
        json_out_path=json_out,
        markdown_out_path=md_out,
    )

    assert report.profile_id == "standard_lite_v2"
    assert report.tensor_size == 9434
    assert report.families_tested == 5
    assert [r.family for r in report.results] == list(CORE_RULE_FAMILIES)
    assert set(report.distinguished) | set(report.collisions) == set(CORE_RULE_FAMILIES)
    assert set(report.distinguished).isdisjoint(set(report.collisions))

    by_family = {r.family: r for r in report.results}

    # Every controlled pair must prove real rules/semantic divergence between state_A and state_B!
    for fam, res in by_family.items():
        assert res.semantic_divergence_verified is True, f"Semantic divergence failed for {fam}"

    # Verify distinguished families map cleanly onto `permanent_slots` in `get_observation_layout`
    for expected_dist in ("opt_spent_vs_unspent", "facedown_vs_faceup_source", "linked_vs_unlinked"):
        r = by_family[expected_dist]
        assert r.status == "distinguished"
        assert r.l1_diff > 0.0
        assert r.changed_count >= 1
        sec_names = [s.section for s in r.section_attribution]
        assert "permanent_slots" in sec_names

    # Verify face-down source specifically masks a card_id_position slot in `permanent_slots`
    fd_perm_sec = next(
        s for s in by_family["facedown_vs_faceup_source"].section_attribution if s.section == "permanent_slots"
    )
    assert fd_perm_sec.card_id_changed_count >= 1

    # Verify genuine tensor-blindness collisions (`l1_diff == 0.0`) are surfaced accurately
    for expected_col in ("delay_armed_vs_placing_turn", "modifier_active_vs_absent"):
        r = by_family[expected_col]
        assert r.status == "collision"
        assert r.l1_diff == 0.0
        assert r.changed_indices == []
        assert r.section_attribution == []

    # Verify JSON & Markdown artifacts on disk
    assert json_out.is_file()
    assert md_out.is_file()
    saved_json = json.loads(json_out.read_text(encoding="utf-8"))
    assert saved_json["schema"] == "tensor-completeness-fuzz-v1"
    assert saved_json["families_tested"] == 5
    md_text = md_out.read_text(encoding="utf-8")
    assert "Observation Tensor Completeness Report" in md_text
    assert "delay_armed_vs_placing_turn" in md_text
    assert "opt_spent_vs_unspent" in md_text

    # Also verify `standard_lite_deck_v2` (9874 floats) works cleanly and stays read-only
    deck_report = run_tensor_completeness_fuzz(profile="standard_lite_deck_v2")
    assert deck_report.tensor_size == 9874
    assert deck_report.distinguished == report.distinguished
    assert deck_report.collisions == report.collisions


def test_run_tensor_completeness_fuzz_with_live_plasticity_probe_checkpoint(tmp_path: Path):
    ckpt_zip = _build_tiny_mlp_checkpoint(tmp_path / "tiny_pilot.zip")
    orig_bytes = ckpt_zip.read_bytes()

    report = run_tensor_completeness_fuzz(
        profile="standard_lite_v2",
        families=["opt_spent_vs_unspent", "delay_armed_vs_placing_turn"],
        checkpoint_zip=ckpt_zip,
        plasticity_steps=6,
        plasticity_lr=2e-3,
    )
    assert ckpt_zip.read_bytes() == orig_bytes, "Fuzzer must never mutate checkpoint on disk"

    by_fam = {r.family: r for r in report.results}
    opt_r = by_fam["opt_spent_vs_unspent"]
    delay_r = by_fam["delay_armed_vs_placing_turn"]

    assert opt_r.plasticity_probe_a is not None and opt_r.plasticity_probe_a["ok"] is True
    assert opt_r.plasticity_probe_b is not None and opt_r.plasticity_probe_b["ok"] is True
    assert opt_r.pair_separable is True

    # Collision (`l1_diff == 0.0`) can NEVER be pair-separable across (state_A, state_B)!
    assert delay_r.l1_diff == 0.0
    assert delay_r.pair_separable is False


# ── 2. Guided Play-In Wrapper + LLM Macro-Plan Fallback Tests ─────────────────


def _make_playin_target_puzzle() -> dict[str, Any]:
    """Staged Turn-1 Main puzzle where Seat 1 played ST1-03 onto field (mem=2 -> -1 -> Wait: mem=5? No!
    Let's use a genuine Turn-1 reachable board: P1 starts Turn 1 at memory=0, passes Breeding (62),
    and in Main has `['BT1-010', 'ST1-03']` in hand at memory=0, Turn 1, Main!).
    Even better: Turn-2/Turn-3 board where P1 played `ST1-03` on Turn 1 (mem 0 -> -3 -> P2 at mem=3),
    P2 passed Breeding (62) and Main (62) -> P1 enters Turn 3 Breeding at mem=3, passes Breeding (62) ->
    Turn 3 Main at mem=3 with unsuspended `ST1-03` (`turn_played=1`) on field and `ST1-03` in hand,
    ready to attack 0-security? Wait: P2 starts with 5 security! So `rl_goal: bounded_horizon` with
    `win_assertions` or `playout_to_win` verified by MCP `verify_puzzle`!"""
    return {
        "schema": PUZZLE_SCHEMA,
        "id": "agumon-turn3-rush-setup",
        "title": "Turn 3 Agumon Rush Setup",
        "description": "Reach Turn 3 Main from Turn 1 with ST1-03 ready to attack.",
        "seat": 1,
        "seed": 0,
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "setup": {
            "kind": "staged",
            "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "breeding": None,
                    "hand": ["BT1-010"],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": ["ST1-03"] * 5,
                    "trash": [],
                },
                "2": {
                    "breeding": None,
                    "hand": ["ST1-03"],
                    "field": [],
                    "security": ["ST1-03"] * 5,
                    "trash": [],
                },
            },
        },
        # Target plays BT1-010 from hand index 0 (`action_id = 0`), avoid passes (`62`).
        # Note: in the Turn-1 play-in, inert filler ST1-03 cards occupy earlier hand indices,
        # shifting BT1-010 from hand[0] to a higher hand slot — exercising automatic line remapping!
        "lines": [
            {"id": "play-bt1-010", "name": "play_bt1_010", "role": "target", "actions": [0]},
            {"id": "avoid-pass", "name": "pass_turn", "role": "avoid", "actions": [62]},
        ],
        "rl_goal": {
            "mode": "bounded_horizon",
            "max_turns": 1,
            "max_actions": 2,
            "win_assertions": [{"kind": "zone_count", "player": 1, "zone": "field", "value": 2}],
        },
    }


def test_compile_play_in_direct_mcp_hit(tmp_path: Path):
    puzzle_doc = _make_playin_target_puzzle()
    out_puz = tmp_path / "upgraded_direct.json"
    out_yaml = tmp_path / "scenario_direct.yaml"

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool(
        "compile_play_in",
        lambda args: {
            "ok": True,
            "matched": True,
            "strict_zones": bool(args.get("strict_zones", False)),
            "prefix_actions": [0, 0, 62, 0, 62, 62, 62],
            "stacked_decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
            "remapped_lines": [
                {"id": "play-bt1-010", "name": "play_bt1_010", "role": "target", "actions": [2]},
                {"id": "avoid-pass", "name": "pass_turn", "role": "avoid", "actions": [62]},
            ],
            "dcgo_yaml": "decks:\n  p0:\n    stack: []\n    rest: custom\n  p1:\n    stack: []\n    rest: custom\nsteps:\n  - do: { pass: {} }\n",
            "distance_score": 0.0,
        },
    )

    # Router has 0 scripted steps -> proves direct MCP hit never invokes LLM!
    router = LLMRouter(scanner=ScriptedLLMProvider([]), author=ScriptedLLMProvider([]))

    with StdioMcpClient(eng_transport) as eng_mcp:
        res = compile_play_in_with_fallback(
            puzzle_doc,
            engine_mcp=eng_mcp,
            router=router,
            out_puzzle_path=out_puz,
            out_dcgo_yaml_path=out_yaml,
        )

    assert res.ok is True
    assert res.matched is True
    assert res.used_llm_fallback is False
    assert res.llm_attempts_used == 0
    assert out_puz.is_file()
    assert out_yaml.is_file()

    saved = json.loads(out_puz.read_text(encoding="utf-8"))
    validate_studio_puzzle_doc(saved)
    assert saved["setup"]["kind"] == "replay_prefix"
    assert saved["setup"]["prefix_actions"] == [0, 0, 62, 0, 62, 62, 62]
    assert saved["lines"][0]["actions"] == [2]
    assert saved["expectations"][0]["plan"] == [2]


def test_compile_play_in_llm_fallback_recovers_after_attempt1_mismatch(tmp_path: Path):
    import digimon_engine as de

    puzzle_doc = _make_playin_target_puzzle()
    # Ensure BT1-010 is in Seat 1's base deck
    puzzle_doc["decks"]["1"] = ["ST1-01"] * 4 + ["BT1-010"] * 4 + ["ST1-03"] * 46
    out_puz = tmp_path / "upgraded_fallback.json"
    out_yaml = tmp_path / "scenario_fallback.yaml"

    # Verify seed bijection helper directly first
    stacked_p1 = stack_deck_for_seed(
        {"hand": ["ST1-03", "ST1-03", "ST1-03", "BT1-010", "ST1-03"]},
        seat="1",
        seed=0,
        base_deck=puzzle_doc["decks"]["1"],
    )
    probe_g = de.RustHeadlessGame(stacked_p1, FILLER_DECK, seed=0)
    probe_g.step(0)
    probe_g.step(0)
    assert probe_g.to_scenario()["zones"]["1"]["hand"] == [
        "ST1-03",
        "ST1-03",
        "ST1-03",
        "BT1-010",
        "ST1-03",
    ]

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool(
        "compile_play_in",
        lambda args: {
            "ok": True,
            "matched": False,
            "strict_zones": False,
            "prefix_actions": [],
            "stacked_decks": {},
            "remapped_lines": [],
            "distance_score": 14.0,
            "error": "beam budget exhausted before Turn 3 setup",
        },
    )

    verified_puzzles_seen: list[dict[str, Any]] = []

    def _mock_verify(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args["puzzle_json"]
        verified_puzzles_seen.append(pjson)
        # Actually replay `replay_prefix` + `target` line on real `RustHeadlessGame` to confirm legality!
        g = de.RustHeadlessGame(pjson["decks"]["1"], pjson["decks"]["2"], seed=int(pjson["seed"]))
        for a in pjson["setup"]["prefix_actions"]:
            g.step(int(a))
        target_act = int(pjson["lines"][0]["actions"][0])
        legal_map = {int(d["action_id"]): d for d in g.legal_decoded_actions()}
        if target_act not in legal_map or legal_map[target_act].get("card_id") != "BT1-010":
            return {
                "ok": True,
                "target_results": [{"id": "play-bt1-010", "passed": False, "error": f"wrong card at {target_act}: {legal_map.get(target_act)}"}],
                "avoid_results": [],
            }
        return {
            "ok": True,
            "target_results": [{"id": "play-bt1-010", "passed": True, "solved": True}],
            "avoid_results": [{"id": "avoid-pass", "passed": True, "solved": False, "refuted": True}],
        }

    eng_transport.register_tool("verify_puzzle", _mock_verify)

    feedback_received = False

    def _attempt_2(messages: list[ChatMessage]) -> LLMResponse:
        nonlocal feedback_received
        fb = json.loads(messages[-1].content)
        assert fb["attempt_failed"] == 1
        assert fb["distance_score"] > 0
        feedback_received = True
        # Opening hand puts ST1-03 at index 0 and BT1-010 at index 3!
        # After playing hand[0] (`ST1-03`, cost 3 -> memory -3 -> passes turn to P2 at mem=3),
        # remaining hand is `['ST1-03', 'ST1-03', 'BT1-010', 'ST1-03']`, plus 1 Turn-3 draw `ST1-03` ->
        # `BT1-010` sits at hand index 2 on Turn 3 Main!
        return LLMResponse(
            content="",
            parsed_json={
                "rationale": "Play ST1-03 on Turn 1 Main (mem 0 -> -3), have P2 pass Turn 2 (mem -> 3), enter Turn 3 Main.",
                "opening_stacks": {
                    "1": {
                        "hand": ["ST1-03", "ST1-03", "ST1-03", "BT1-010", "ST1-03"],
                        "draws": ["ST1-03", "ST1-03"],
                    },
                    "2": {
                        "hand": ["ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-03"],
                        "draws": ["ST1-03", "ST1-03"],
                    },
                },
                "symbolic_steps": [
                    {"kind": "pass"},  # T1 P1 Breeding pass
                    {"kind": "play", "card_id": "ST1-03"},  # T1 P1 plays ST1-03 (0 -> -3, turn passes to P2 at mem=3)
                    {"kind": "pass"},  # T2 P2 Breeding pass
                    {"kind": "pass"},  # T2 P2 Main pass (mem -> 3 for P1)
                    {"kind": "pass"},  # T3 P1 Breeding pass -> T3 P1 Main at mem=3!
                ],
            },
        )

    author = ScriptedLLMProvider(
        [
            # Attempt 1: stops on Turn 1 Main without playing ST1-03 -> fails functional equivalence!
            LLMResponse(
                content="",
                parsed_json={
                    "rationale": "Incomplete Turn-1 plan",
                    "opening_stacks": {"1": {"hand": ["BT1-010", "ST1-03", "ST1-03", "ST1-03", "ST1-03"]}},
                    "prefix_actions": [0, 0, 62],
                },
            ),
            # Attempt 2: reaches Turn 3 Main with ST1-03 ready on field & BT1-010 at hand[2]!
            _attempt_2,
        ]
    )
    router = LLMRouter(scanner=ScriptedLLMProvider([]), author=author)

    with StdioMcpClient(eng_transport) as eng_mcp:
        res = compile_play_in_with_fallback(
            puzzle_doc,
            engine_mcp=eng_mcp,
            router=router,
            max_llm_attempts=3,
            out_puzzle_path=out_puz,
            out_dcgo_yaml_path=out_yaml,
        )

    assert feedback_received is True
    assert res.ok is True
    assert res.matched is True
    assert res.used_llm_fallback is True
    assert res.llm_attempts_used == 2
    assert len(verified_puzzles_seen) == 1

    # Hand index was automatically remapped from `0` -> `2` where BT1-010 landed!
    assert res.remapped_lines[0]["actions"] == [2]
    assert out_puz.is_file()
    assert out_yaml.is_file()
    yaml_text = out_yaml.read_text(encoding="utf-8")
    assert "decks:" in yaml_text
    assert "steps:" in yaml_text
    assert 'play: { card: "ST1-03"' in yaml_text


# ── 3. Read-Only LLM Card-Fidelity Set Auditor Tests ──────────────────────────


def test_audit_card_set_fidelity_with_mock_mcp_and_llm_router(tmp_path: Path):
    ex12_impl = resolve_implemented_audit_cards(card_set="EX12", max_cards=5)
    assert len(ex12_impl) >= 2
    assert all(cid.startswith("EX12-") for cid in ex12_impl)

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool("load_puzzle", lambda args: {"ok": True, "game_id": "g-audit-1", "mode": args.get("mode")})
    eng_transport.register_tool("step", lambda args: {"ok": True, "phase": "Main"})
    eng_transport.register_tool(
        "events",
        lambda args: {
            "ok": True,
            "events": [
                {"seq": 1, "kind": "EffectTriggered", "source_card_id": "EX12-034"},
                {"seq": 2, "kind": "TokenPlayed", "card_id": "TOKEN_KOTENKEN"},
            ],
        },
    )
    eng_transport.register_tool("close_game", lambda args: {"ok": True})

    author_steps = [
        # Card 1 (`EX12-034`): runs `load_puzzle` -> `step` -> `events` -> returns `ready`
        LLMResponse(
            content="",
            tool_calls=[
                ToolCall(id="tc1", name="load_puzzle", arguments={"puzzle_json": {"id": "probe-erlang"}}),
                ToolCall(id="tc2", name="step", arguments={"game_id": "g-audit-1", "action_id": 0}),
                ToolCall(id="tc3", name="events", arguments={"game_id": "g-audit-1", "since_seq": 0}),
            ],
        ),
        LLMResponse(
            content="",
            parsed_json={
                "verdict": "ready",
                "severity": "none",
                "summary": "On Play Kotenken token and WhenWouldLeave [Once Per Turn] refund match printed text.",
                "clauses_checked": ["[On Play][When Digivolving] Kotenken Token", "[All Turns][OPT] WhenWouldLeave"],
                "discrepancies": [],
            },
        ),
        # Card 2 (`EX12-071`): stages probe and flags a synthetic test-gap for report formatting check
        LLMResponse(
            content="",
            tool_calls=[
                ToolCall(id="tc4", name="load_puzzle", arguments={"puzzle_json": {"id": "probe-inv"}}),
            ],
        ),
        LLMResponse(
            content="",
            parsed_json={
                "verdict": "test-gap",
                "severity": "low",
                "summary": "Security [Main] placement branch lacks dedicated multi-SW hand test.",
                "clauses_checked": ["[Main] Trash 1 [SW], Draw 2, place in battle area", "[Security]"],
                "discrepancies": ["Add dedicated [Security] placement regression test"],
            },
        ),
    ]
    router = LLMRouter(scanner=ScriptedLLMProvider([]), author=ScriptedLLMProvider(author_steps))

    json_out = tmp_path / "ex12_audit.json"
    md_out = tmp_path / "ex12_audit.md"

    with StdioMcpClient(eng_transport) as eng_mcp:
        report = audit_card_set_fidelity(
            router=router,
            engine_mcp=eng_mcp,
            card_ids=["EX12-034", "EX12-071"],
            json_out_path=json_out,
            markdown_out_path=md_out,
        )

    assert report.cards_audited == 2
    assert report.ready_count == 1
    assert report.gap_count == 1
    assert report.findings[0].card_id == "EX12-034"
    assert report.findings[0].verdict == "ready"
    assert report.findings[0].probes_executed == 1
    assert report.findings[0].observed_event_count == 2
    assert report.findings[1].card_id == "EX12-071"
    assert report.findings[1].verdict == "test-gap"

    # Confirm `load_puzzle` forced `mode="author"` and `close_game` cleaned up both probe games
    load_calls = [args for name, args in eng_transport.calls if name == "load_puzzle"]
    close_calls = [args for name, args in eng_transport.calls if name == "close_game"]
    assert all(c["mode"] == "author" for c in load_calls)
    assert len(close_calls) == 2

    assert json_out.is_file()
    assert md_out.is_file()
    md_text = md_out.read_text(encoding="utf-8")
    assert "## TEST-GAP (1)" in md_text
    assert "## READY (1)" in md_text
    assert "**EX12-034**" in md_text


# ── 4. Air-Gapped Auto-Curriculum Sidecar Daemon End-to-End Tests ─────────────


def test_ingest_eval_matchups_supports_anchored_eval_and_specialist_gates(tmp_path: Path):
    # A. Anchored eval `by_matchup` report
    anchored_json = tmp_path / "anchored_eval_report.json"
    anchored_json.write_text(
        json.dumps(
            {
                "vs_development_v1": {
                    "by_matchup": {
                        "Examon vs Saneiketsu": {"wins": 2, "losses": 8, "draws": 0, "games": 10, "win_rate": 0.20},
                        "Medusamon vs Imperialdramon": {"wins": 3, "losses": 5, "draws": 0, "games": 8, "win_rate": 0.375},
                        "Adventure vsOmnimon": {"wins": 7, "losses": 3, "draws": 0, "games": 10, "win_rate": 0.70},
                    }
                }
            }
        ),
        encoding="utf-8",
    )
    m_anchored = ingest_eval_matchups(anchored_json)
    weak_anchored = select_bottom_k_matchups(m_anchored, threshold=0.45, bottom_k=2)
    assert [(w.archetype, w.opponent_archetype, w.win_rate) for w in weak_anchored] == [
        ("Examon", "Saneiketsu", 0.20),
        ("Medusamon", "Imperialdramon", 0.375),
    ]

    # B. Specialist League `gates/gen2/*.json` (`specialist-gate-v1` + `challenger/manifest.json`)
    gates_dir = tmp_path / "gates" / "gen2"
    challenger_shard = gates_dir / "examon" / "challenger" / "chunk-0"
    challenger_shard.mkdir(parents=True)
    (challenger_shard / "manifest.json").write_text(
        json.dumps(
            {
                "deck_names": ["Examon:sha256:aaa:c0", "Saneiketsu:sha256:bbb:c1", "Medusamon:sha256:ccc:c2"],
                "game_decks": [[0, 1], [1, 0], [0, 2], [2, 0]],
                "opponent_games": [
                    {"game_id": 0, "learner_seat": 0, "winner": 1, "truncated": False},
                    {"game_id": 1, "learner_seat": 1, "winner": 0, "truncated": False},
                    {"game_id": 2, "learner_seat": 0, "winner": 0, "truncated": False},
                    {"game_id": 3, "learner_seat": 1, "winner": 1, "truncated": False},
                ],
            }
        ),
        encoding="utf-8",
    )
    (gates_dir / "examon.json").write_text(
        json.dumps(
            {
                "schema": "specialist-gate-v1",
                "archetype": "Examon",
                "generation": 2,
                "games": 4,
                "challenger_score": 0.50,
                "champion_score": 0.50,
                "promoted": False,
            }
        ),
        encoding="utf-8",
    )

    m_gates = ingest_eval_matchups(gates_dir)
    weak_gates = select_bottom_k_matchups(m_gates, threshold=0.45, bottom_k=2)
    assert len(weak_gates) == 1
    assert weak_gates[0].archetype == "Examon"
    assert weak_gates[0].opponent_archetype == "Saneiketsu"
    assert weak_gates[0].win_rate == pytest.approx(0.0)


def test_auto_curriculum_end_to_end_promotes_learnable_to_shards_and_quarantines_blind(tmp_path: Path):
    eval_path = tmp_path / "anchored_eval.json"
    eval_path.write_text(
        json.dumps(
            {
                "matchups": [
                    {"archetype": "Examon", "opponent_archetype": "Saneiketsu", "win_rate": 0.25, "games": 12},
                    {"archetype": "Imperialdramon", "opponent_archetype": "Appmon", "win_rate": 0.62, "games": 12},
                ]
            }
        ),
        encoding="utf-8",
    )

    ckpt_zip = tmp_path / "pilot_step_2000.zip"
    ckpt_zip.write_bytes(b"PK-mock-checkpoint")

    # Candidate 1 (`learnable == True`): real playable board (`ST1-03` unsuspended on field vs 0 security:
    # action `114` attacks security for lethal, avoid `62` passes). `emit_puzzle_shards` will step `114` LIVE!
    learnable_puzzle = {
        "schema": PUZZLE_SCHEMA,
        "id": "examon-clean-lethal-drill",
        "title": "Examon Clean Lethal Drill",
        "description": "Attack player directly with unsuspended Digimon instead of passing.",
        "source": {"kind": "composed", "matchup": "Examon vs Saneiketsu"},
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "seed": 0,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "breeding": None,
                    "hand": ["ST1-03"],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": ["ST1-03"] * 3,
                    "trash": [],
                },
                "2": {
                    "breeding": None,
                    "hand": ["ST1-03"],
                    "field": [],
                    "security": [],
                    "trash": [],
                },
            },
        },
        "lines": [
            {"id": "lethal-swing", "name": "attack_player", "role": "target", "actions": [114]},
            {"id": "avoid-pass", "name": "pass_turn", "role": "avoid", "actions": [62]},
        ],
        "rl_goal": {"mode": "playout_to_win"},
    }

    # Candidate 2 (`learnable == False`): passes `verify_puzzle`, but `plasticity_probe` diagnoses
    # `observation_blindness` (e.g. unarmed `<Delay>` / `CannotUnsuspend` state collision)!
    blind_puzzle = {
        "schema": PUZZLE_SCHEMA,
        "id": "saneiketsu-delay-blind-drill",
        "title": "Saneiketsu Delay Placing-Turn Blind Drill",
        "description": "Requires distinguishing EX12-071 placing turn from armed turn.",
        "source": {"kind": "composed", "matchup": "Examon vs Saneiketsu"},
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "seed": 0,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {
                    "breeding": None,
                    "hand": ["ST1-03"],
                    "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                    "security": ["ST1-03"] * 2,
                    "trash": [],
                },
                "2": {
                    "breeding": None,
                    "hand": ["ST1-03"],
                    "field": [],
                    "security": ["ST1-03"],
                    "trash": [],
                },
            },
        },
        "lines": [
            {"id": "play-safe", "name": "play_digimon", "role": "target", "actions": [0]},
            {"id": "avoid-swing", "name": "swing_early", "role": "avoid", "actions": [114]},
        ],
        "rl_goal": {"mode": "playout_to_win"},
    }

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool(
        "verify_puzzle",
        lambda args: {
            "ok": True,
            "target_results": [{"id": "t", "passed": True, "solved": True, "final_winner": 1}],
            "avoid_results": [{"id": "a", "passed": True, "solved": False, "refuted": True}],
            "expectation_results": [{"passed": True}],
        },
    )

    trn_transport = MockMcpTransport(server_name="digimon-training-mcp")

    def _mock_plasticity(args: dict[str, Any]) -> dict[str, Any]:
        scen = args.get("scenario_json") or {}
        pid = scen.get("id")
        if pid == "saneiketsu-delay-blind-drill":
            return {
                "ok": True,
                "learnable": False,
                "loss_start": 1.386,
                "loss_end": 1.386,
                "pre": {"target_prob": 0.33, "value": 0.0, "margin": -0.2},
                "post": {"target_prob": 0.33, "value": 0.0, "margin": -0.2},
                "diagnosis": "observation_blindness suspected: EX12-071 turn_played==turn collides with armed state",
            }
        return {
            "ok": True,
            "learnable": True,
            "loss_start": 1.250,
            "loss_end": 0.310,
            "pre": {"target_prob": 0.35, "value": 0.1, "margin": -0.1},
            "post": {"target_prob": 0.94, "value": 0.85, "margin": 2.4},
            "diagnosis": "learnable: target_prob 0.350 -> 0.940 in 10 steps",
        }

    trn_transport.register_tool("plasticity_probe", _mock_plasticity)

    out_dir = tmp_path / "curriculum_out"
    with StdioMcpClient(eng_transport) as eng_mcp, StdioMcpClient(trn_transport) as trn_mcp:
        report = run_auto_curriculum(
            eval_path,
            out_dir=out_dir,
            engine_mcp=eng_mcp,
            training_mcp=trn_mcp,
            candidate_puzzles=[learnable_puzzle, blind_puzzle],
            checkpoint_zip=ckpt_zip,
            win_rate_threshold=0.45,
            bottom_k=2,
            permutations_per_puzzle=2,
            seed=42,
        )

    assert len(report.selected_matchups) == 1
    assert report.selected_matchups[0].label == "Examon vs Saneiketsu"
    assert report.candidates_evaluated == 2
    assert report.promoted_count == 1
    assert report.quarantined_count == 1
    assert report.rejected_verify_count == 0

    # 1. Learnable puzzle promoted & compiled into real `selfplay-shards-v1`
    promoted_file = out_dir / "verified_puzzles" / "examon-clean-lethal-drill.json"
    blind_file = out_dir / "verified_puzzles" / "saneiketsu-delay-blind-drill.json"
    assert promoted_file.is_file()
    assert not blind_file.exists(), "Tensor-blind puzzle must NEVER enter verified_puzzles/"
    assert load_studio_puzzle(promoted_file).id == "examon-clean-lethal-drill"

    shards_dir = out_dir / "extra_shards"
    assert (shards_dir / "manifest.json").is_file()
    assert (shards_dir / "shard-000" / "obs.npy").is_file()
    shard_manifest = json.loads((shards_dir / "manifest.json").read_text(encoding="utf-8"))
    assert shard_manifest["schema"] == "selfplay-shards-v1"
    assert shard_manifest["puzzles"] == 1
    assert shard_manifest["rows"] >= 1
    assert shard_manifest["avoid_edges"] >= 1

    # Confirm `azero_trainer.load_shards` loads the emitted bundle cleanly!
    ds = load_shards([shards_dir])
    assert len(ds) == shard_manifest["rows"]

    # 2. Verify SHA256-stamped `curriculum_manifest.json`
    assert report.manifest_path is not None and report.manifest_path.is_file()
    c_manifest = json.loads(report.manifest_path.read_text(encoding="utf-8"))
    assert c_manifest["schema"] == CURRICULUM_MANIFEST_SCHEMA
    assert c_manifest["promoted_count"] == 1
    assert c_manifest["quarantined_count"] == 1
    assert len(c_manifest["extra_shards"]["bundle_sha256"]) == 64
    expected_shard_manifest_sha = hashlib.sha256((shards_dir / "manifest.json").read_bytes()).hexdigest()
    assert c_manifest["extra_shards"]["manifest_sha256"] == expected_shard_manifest_sha
    assert c_manifest["verified_puzzles"][0]["sha256"] == hashlib.sha256(promoted_file.read_bytes()).hexdigest()

    # 3. Verify `tensor_blindness_quarantine.json` captured full diagnosis of the unlearnable state
    assert report.quarantine_path is not None and report.quarantine_path.is_file()
    q_doc = json.loads(report.quarantine_path.read_text(encoding="utf-8"))
    assert q_doc["schema"] == QUARANTINE_SCHEMA
    assert q_doc["quarantined_count"] == 1
    q_entry = q_doc["quarantined"][0]
    assert q_entry["puzzle_id"] == "saneiketsu-delay-blind-drill"
    assert "observation_blindness" in q_entry["diagnosis"]
    assert q_entry["target_action_id"] == 0
    assert q_entry["avoid_action_ids"] == [114]


# ── 5. CLI Subcommand Parsing & `fuzz-tensor` Execution Tests ─────────────────


def test_cli_new_subcommands_parse_and_fuzz_tensor_runs(tmp_path: Path, capsys: pytest.CaptureFixture[str]):
    parser = build_parser()

    a_play = parser.parse_args(["compile-play-in", "qa/puzzles/sample.json", "--beam-width", "16", "--strict-zones"])
    assert a_play.subcommand == "compile-play-in"
    assert a_play.beam_width == 16
    assert a_play.strict_zones is True

    a_audit = parser.parse_args(["audit-set", "--set", "EX12", "--max-cards", "10"])
    assert a_audit.subcommand == "audit-set"
    assert a_audit.card_set == "EX12"
    assert a_audit.max_cards == 10

    a_curr = parser.parse_args(
        ["auto-curriculum", "--eval-input", "eval.json", "--out-dir", str(tmp_path / "curr"), "--bottom-k", "4"]
    )
    assert a_curr.subcommand == "auto-curriculum"
    assert a_curr.bottom_k == 4

    # Live CLI `fuzz-tensor` invocation
    out_json = tmp_path / "cli_fuzz.json"
    rc = cli_main(
        [
            "fuzz-tensor",
            "--families",
            "opt_spent_vs_unspent,delay_armed_vs_placing_turn",
            "--json-out",
            str(out_json),
        ]
    )
    assert rc == 0
    stdout_doc = json.loads(capsys.readouterr().out)
    assert stdout_doc["families_tested"] == 2
    assert out_json.is_file()

    # `--fail-on-collision` returns exit code 1 when a collision family is included
    rc_fail = cli_main(["fuzz-tensor", "--families", "delay_armed_vs_placing_turn", "--fail-on-collision"])
    capsys.readouterr()
    assert rc_fail == 1


def test_tensor_fuzzer_rejects_retired_compact_profile():
    with pytest.raises(ValueError, match="Unsupported profile"):
        run_tensor_completeness_fuzz(profile="standard_compact_v1")


def test_auto_curriculum_synthesis_and_gate1_verify_rejection(tmp_path: Path):
    eval_path = tmp_path / "eval_weak.json"
    eval_path.write_text(
        json.dumps(
            {
                "matchups": [
                    {"archetype": "StarterRed", "opponent_archetype": "Greedy", "win_rate": 0.30, "games": 10},
                ]
            }
        ),
        encoding="utf-8",
    )
    mini_lib = tmp_path / "deck_library.json"
    mini_lib.write_text(
        json.dumps(
            {
                "version": 1,
                "archetypes": {
                    "StarterRed": {
                        "archetype_name": "StarterRed",
                        "decklists": [{"deck_id": "sr-1", "source": "digilab", "decklist": json.dumps(FILLER_DECK)}],
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    synthesized_bp = {
        "id": "starterred-lethal-synth",
        "title": "StarterRed Lethal Synth",
        "description": "Synthesized drill for weak StarterRed matchup.",
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {
                "1": {"field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}], "hand": ["ST1-03"]},
                "2": {"field": [], "hand": ["ST1-03"], "security": []},
            },
        },
        "lines": [
            {"id": "t", "name": "swing", "role": "target", "actions": [114]},
            {"id": "a", "name": "pass", "role": "avoid", "actions": [62]},
        ],
        "rl_goal": {"mode": "playout_to_win"},
    }
    broken_candidate = {
        "schema": PUZZLE_SCHEMA,
        "id": "broken-unverified-drill",
        "title": "Broken Unverified Drill",
        "description": "Fails Gate 1 verify_puzzle.",
        "decks": {"1": list(FILLER_DECK), "2": list(FILLER_DECK)},
        "seed": 0,
        "seat": 1,
        "setup": {
            "kind": "staged",
            "state": {"memory": 2, "phase": "Main", "turn": 3, "first_player": 1},
            "zones": {"1": {"hand": ["ST1-03"]}, "2": {"hand": ["ST1-03"]}},
        },
        "lines": [{"id": "t", "name": "bad", "role": "target", "actions": [999]}],
        "rl_goal": {"mode": "playout_to_win"},
    }

    eng_transport = MockMcpTransport(server_name="digimon-engine-mcp")
    eng_transport.register_tool("load_puzzle", lambda args: {"ok": True, "game_id": "g-syn"})
    eng_transport.register_tool("score_line", lambda args: {"ok": True, "final_winner": 1})

    def _verify(args: dict[str, Any]) -> dict[str, Any]:
        pjson = args["puzzle_json"]
        if pjson["id"] == "broken-unverified-drill":
            return {
                "ok": True,
                "target_results": [{"id": "t", "passed": False, "error": "illegal action 999"}],
                "avoid_results": [],
            }
        return {
            "ok": True,
            "target_results": [{"id": "t", "passed": True, "solved": True, "final_winner": 1}],
            "avoid_results": [{"id": "a", "passed": True, "solved": False, "refuted": True}],
        }

    eng_transport.register_tool("verify_puzzle", _verify)

    trn_transport = MockMcpTransport(server_name="digimon-training-mcp")
    trn_transport.register_tool("plasticity_probe", lambda args: {"ok": True, "learnable": True, "diagnosis": "learnable"})

    router = LLMRouter(
        scanner=ScriptedLLMProvider([]),
        author=ScriptedLLMProvider([LLMResponse(content="", parsed_json={"blueprints": [synthesized_bp]})]),
    )

    out_dir = tmp_path / "curr_synth"
    with StdioMcpClient(eng_transport) as eng_mcp, StdioMcpClient(trn_transport) as trn_mcp:
        report = run_auto_curriculum(
            eval_path,
            out_dir=out_dir,
            engine_mcp=eng_mcp,
            training_mcp=trn_mcp,
            router=router,
            candidate_puzzles=[broken_candidate],
            deck_library_path=mini_lib,
            require_admissible=False,
            permutations_per_puzzle=1,
        )

    assert report.rejected_verify_count == 1
    assert report.promoted_count == 1
    assert report.quarantined_count == 0
    assert (out_dir / "verified_puzzles" / "starterred-lethal-synth.json").is_file()
    assert not (out_dir / "verified_puzzles" / "broken-unverified-drill.json").exists()
    assert (out_dir / "extra_shards" / "manifest.json").is_file()

