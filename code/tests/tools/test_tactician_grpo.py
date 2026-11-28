"""Comprehensive offline unit & engine-integration tests for `tools.tactician_grpo` (Track D)."""

from __future__ import annotations

import ast
import copy
import json
from pathlib import Path
import subprocess
import sys
from typing import Any

import pytest

from tools.puzzle_lab import PUZZLE_SCHEMA, resolve_repo_root
from tools.puzzle_lab.mcp_client import MockMcpTransport, StdioMcpClient
from tools.tactician_grpo import (
    DEFAULT_BASE_MODEL,
    TACTICIAN_SOLVE_SCHEMA,
    TASK_COMPOSE_COUNTER_PUZZLE,
    TASK_SOLVE_POSITION,
    DigimonVerifiableReward,
    OllamaExportConfig,
    TacticianTrainConfig,
    export_ollama_bundle,
    export_tactician_datasets,
    render_ollama_modelfile,
    run_sft_and_grpo,
    score_single_completion,
    strip_thinking_blocks,
    validate_train_config,
)
from tools.tactician_grpo.cli import main as cli_main
from tools.tactician_grpo.verifiable_reward import _convert_search_exam_to_puzzle_dict

REPO_ROOT = resolve_repo_root()
EX12_PUZZLE_PATH = REPO_ROOT / "qa" / "puzzles" / "ex12-erlangmon-saneiketsu-invitation-trap.json"
EX13_EXAM_PATH = REPO_ROOT / "qa" / "search-exams" / "ex13-imperialdramon-dna-paildramon-jamming-lethal.json"


# ── 1. Dataset Exporter Tests (`sft_train.jsonl` & `grpo_prompts.jsonl`) ──────


def test_export_dataset_real_puzzles_and_search_exams_enriches_cards_and_rules(tmp_path: Path):
    summary = export_tactician_datasets(out_dir=tmp_path)

    assert summary.sft_path.exists()
    assert summary.grpo_path.exists()
    assert summary.puzzles_exported >= 5
    assert summary.solve_rows >= 5
    assert summary.compose_rows >= 4
    assert summary.sft_rows == summary.solve_rows + summary.compose_rows
    assert summary.grpo_rows == summary.sft_rows
    # Duplicate ID `ex12-erlangmon-saneiketsu-invitation-trap` deduplicated; unlabeled `3-muskets-der-blitz` skipped
    assert "ex12-erlangmon-saneiketsu-invitation-trap" in summary.skipped_duplicate_ids
    assert "3-muskets-der-blitz" in summary.skipped_unlabeled_ids

    sft_rows = [
        json.loads(ln)
        for ln in summary.sft_path.read_text(encoding="utf-8").splitlines()
        if ln.strip()
    ]
    grpo_rows = [
        json.loads(ln)
        for ln in summary.grpo_path.read_text(encoding="utf-8").splitlines()
        if ln.strip()
    ]
    assert len(sft_rows) == summary.sft_rows
    assert len(grpo_rows) == summary.grpo_rows

    sft_by_id = {r["id"]: r for r in sft_rows}
    grpo_by_id = {r["id"]: r for r in grpo_rows}

    for expected_pid in (
        "ex12-erlangmon-saneiketsu-invitation-trap",
        "ex13-imperialdramon-dna-paildramon-jamming-lethal",
    ):
        assert f"{expected_pid}::solve" in sft_by_id
        assert f"{expected_pid}::compose" in sft_by_id
        assert f"{expected_pid}::solve" in grpo_by_id
        assert f"{expected_pid}::compose" in grpo_by_id

    # Inspect card-text enrichment & rules excerpts on EX12 solve prompt
    ex12_solve_sft = sft_by_id["ex12-erlangmon-saneiketsu-invitation-trap::solve"]
    msgs = ex12_solve_sft["messages"]
    assert [m["role"] for m in msgs] == ["system", "user", "assistant"]
    assert "<think>" in msgs[2]["content"] and "</think>" in msgs[2]["content"]

    user_json_start = msgs[1]["content"].index("{")
    user_payload = json.loads(msgs[1]["content"][user_json_start:])
    card_texts_by_id = {c["card_id"]: c for c in user_payload["card_texts"]}

    for must_have_cid in ("EX12-034", "EX12-071", "BT20-042", "TOKEN_KOTENKEN"):
        assert must_have_cid in card_texts_by_id
        cinfo = card_texts_by_id[must_have_cid]
        for req_field in (
            "effect_text",
            "inherited_text",
            "security_text",
            "link_text",
            "traits",
            "evo_costs",
        ):
            assert req_field in cinfo
    assert "Kotenken" in card_texts_by_id["EX12-034"]["effect_text"]
    assert "Delay" in card_texts_by_id["EX12-071"]["effect_text"]
    assert any("Delay" in ex or "Blocker" in ex for ex in user_payload["rules_excerpts"])

    # Inspect EX13 Imperialdramon solve prompt enrichment
    ex13_solve_sft = sft_by_id["ex13-imperialdramon-dna-paildramon-jamming-lethal::solve"]
    u13 = json.loads(ex13_solve_sft["messages"][1]["content"][ex13_solve_sft["messages"][1]["content"].index("{") :])
    c13 = {c["card_id"]: c for c in u13["card_texts"]}
    assert "BT16-025" in c13 and "BT12-022" in c13 and "EX13-024" in c13
    assert "Jamming" in c13["BT12-022"]["inherited_text"]
    assert len(c13["BT16-025"]["evo_costs"]) >= 1
    assert any("DNA" in ex or "Jamming" in ex for ex in u13["rules_excerpts"])

    # Every single exported SFT ground-truth completion across all solve & compose rows
    # must score a full 1.00 under DigimonVerifiableReward!
    reward_fn = DigimonVerifiableReward(adversary_seeds=1)
    for row_key, sft_r in sft_by_id.items():
        grpo_r = grpo_by_id[row_key]
        scores = reward_fn(
            prompts=[grpo_r["prompt"]],
            completions=[[sft_r["messages"][2]]],
            ground_truth_meta=[grpo_r["ground_truth_meta"]],
            task_kind=[grpo_r["task_kind"]],
        )
        assert scores == [1.0], f"{row_key} failed 1.00 check: {reward_fn.last_breakdowns[0].to_dict()}"


# ── 2. 5-Stage Progressive Verifiable Engine Reward Tests ─────────────────────


def test_verifiable_reward_all_five_progressive_tiers_on_ex12_and_ex13():
    ex12_doc = json.loads(EX12_PUZZLE_PATH.read_text(encoding="utf-8"))
    ex13_raw = json.loads(EX13_EXAM_PATH.read_text(encoding="utf-8"))
    ex13_doc = _convert_search_exam_to_puzzle_dict(ex13_raw)

    # Tier 0 (0.00): Malformed JSON & missing schema fields (even with <think> block)
    bd_malformed = score_single_completion("<think>Reasoning...</think>\n{not valid json")
    assert bd_malformed.total == 0.0
    assert bd_malformed.stage_reached == 0
    assert bd_malformed.failure_stage == 1

    missing_lines = copy.deepcopy(ex12_doc)
    missing_lines["lines"] = []
    bd_schema = score_single_completion(json.dumps(missing_lines))
    assert bd_schema.total == 0.0
    assert bd_schema.failure_stage == 1

    # Tier 1 (0.15): Valid JSON & schema, but unknown / inadmissible card or deck copy purity violation
    bad_card_doc = copy.deepcopy(ex12_doc)
    bad_card_doc["decks"]["1"][0] = "FAKE-999"
    bd_bad_card = score_single_completion(f"<think>Try fake card</think>\n{json.dumps(bad_card_doc)}")
    assert bd_bad_card.total == 0.15
    assert bd_bad_card.stage_reached == 1
    assert bd_bad_card.failure_stage == 2
    assert "FAKE-999" in bd_bad_card.reason

    excess_copy_doc = copy.deepcopy(ex13_doc)
    # Place 5 copies of BT16-025 in hand when deck only has 3 copies
    excess_copy_doc["setup"]["zones"]["1"]["hand"] = ["BT16-025"] * 5
    bd_purity = score_single_completion(json.dumps(excess_copy_doc))
    assert bd_purity.total == 0.15
    assert bd_purity.failure_stage == 2

    # Tier 2 (0.30): Valid JSON + admissible cards/purity, but invalid stage / illegal prefix_actions
    bad_phase_doc = copy.deepcopy(ex13_doc)
    bad_phase_doc["setup"]["state"]["phase"] = "BogusPhase"
    bd_bad_phase = score_single_completion(json.dumps(bad_phase_doc))
    assert bd_bad_phase.total == 0.30
    assert bd_bad_phase.stage_reached == 2
    assert bd_bad_phase.failure_stage == 3

    bad_prefix_doc = copy.deepcopy(ex12_doc)
    bad_prefix_doc["setup"]["prefix_actions"] = [999]
    bd_bad_prefix = score_single_completion(json.dumps(bad_prefix_doc))
    assert bd_bad_prefix.total == 0.30
    assert bd_bad_prefix.stage_reached == 2
    assert bd_bad_prefix.failure_stage == 3

    egg_in_hand_doc = copy.deepcopy(ex13_doc)
    egg_in_hand_doc["setup"]["zones"]["1"]["egg_deck"].pop()
    egg_in_hand_doc["setup"]["zones"]["1"]["hand"].append("BT12-002")
    bd_egg_hand = score_single_completion(json.dumps(egg_in_hand_doc))
    assert bd_egg_hand.total == 0.30
    assert bd_egg_hand.failure_stage == 3

    # Tier 3 (0.50): Legal board (`RustDebugGame.validate()` passes), but `target` line gets refuted / fails!
    # On EX12: proposing Oblivion Bird [0, 101] as `target` springs Erlangmon + Saneiketsu Invitation!
    ex12_refuted_target = copy.deepcopy(ex12_doc)
    ex12_refuted_target["lines"] = [
        {
            "id": "oblivion_bird_into_trap",
            "name": "Oblivion Bird into Erlangmon trap",
            "role": "target",
            "actions": [0, 101],
        },
        {
            "id": "avoid_swing",
            "name": "Swing into Kotenken",
            "role": "avoid",
            "actions": [114, 62],
        },
    ]
    bd_refuted_12 = score_single_completion(json.dumps(ex12_refuted_target), adversary_seeds=1)
    assert bd_refuted_12.total == 0.50
    assert bd_refuted_12.stage_reached == 3
    assert bd_refuted_12.failure_stage == 4

    # On EX13: proposing normal digivolve [400] (costs 4, passes turn) as `target` fails to win!
    ex13_failed_target = copy.deepcopy(ex13_doc)
    ex13_failed_target["lines"] = [
        {"id": "normal_digivolve", "name": "Normal digivolve pass", "role": "target", "actions": [400]},
        {"id": "swing_blocker", "name": "Swing into Slayerdramon", "role": "avoid", "actions": [114]},
    ]
    bd_failed_13 = score_single_completion(json.dumps(ex13_failed_target), adversary_seeds=1)
    assert bd_failed_13.total == 0.50
    assert bd_failed_13.stage_reached == 3
    assert bd_failed_13.failure_stage == 4

    # Tier 4 (0.85): `target` solves cleanly, but `avoid` is absent or is a prefix of `target`
    ex13_no_avoid = copy.deepcopy(ex13_doc)
    ex13_no_avoid["lines"] = [ln for ln in ex13_doc["lines"] if ln["role"] == "target"]
    bd_no_avoid_13 = score_single_completion(json.dumps(ex13_no_avoid), adversary_seeds=1)
    assert bd_no_avoid_13.total == 0.85
    assert bd_no_avoid_13.stage_reached == 4
    assert bd_no_avoid_13.failure_stage == 5

    # Reward-hack guard: truncating `target` ([63, 0, 1, 114, 114]) to [63] as `avoid` stays at 0.85!
    ex13_prefix_hack = copy.deepcopy(ex13_no_avoid)
    ex13_prefix_hack["lines"].append(
        {"id": "fake_avoid_prefix", "name": "Truncated target prefix", "role": "avoid", "actions": [63]}
    )
    bd_prefix_hack = score_single_completion(json.dumps(ex13_prefix_hack), adversary_seeds=1)
    assert bd_prefix_hack.total == 0.85
    assert bd_prefix_hack.failure_stage == 5

    # Tier 5 (1.00): Full verified solve + refuted `avoid` on BOTH EX12 and EX13 with `<think>...</think>` + fences!
    wrapped_ex12 = (
        "<think>\n"
        "BT1-094 Oblivion Bird triggers EX12-034 WhenWouldLeave -> EX12-015 Gokuumon -> bounce + EX12-071 Delay.\n"
        "Instead digivolve BT20-042 Groundramon (415) to suspend TOKEN_KOTENKEN without removing it, then attack (114)!\n"
        "</think>\n"
        f"```json\n{json.dumps(ex12_doc, indent=2)}\n```"
    )
    bd_full_12 = score_single_completion(wrapped_ex12, adversary_seeds=2)
    assert bd_full_12.total == 1.00
    assert bd_full_12.stage_reached == 5
    assert bd_full_12.failure_stage is None
    assert bd_full_12.reason == "verified"

    wrapped_ex13 = (
        "<think>\n"
        "DNA digivolve BT16-025 Paildramon (63 -> 0 -> 1) for 0 cost (+2 mem), suspending 2-source EX13-024 Blocker,\n"
        "swing security with Jamming (114), unsuspend, and swing for lethal (114).\n"
        "</think>\n"
        + json.dumps(ex13_doc)
    )
    bd_full_13 = score_single_completion(wrapped_ex13, adversary_seeds=2)
    assert bd_full_13.total == 1.00
    assert bd_full_13.stage_reached == 5
    assert bd_full_13.failure_stage is None

    # Verify TRL batch callable contract (`__name__`, conversational completions, mixed batch)
    reward_callable = DigimonVerifiableReward(adversary_seeds=1)
    assert reward_callable.__name__ == "digimon_verifiable_reward"
    batch_scores = reward_callable(
        prompts=[[{"role": "user", "content": "p"}]] * 6,
        completions=[
            [{"role": "assistant", "content": "<think>bad</think>nope"}],
            [{"role": "assistant", "content": json.dumps(bad_card_doc)}],
            [{"role": "assistant", "content": json.dumps(bad_phase_doc)}],
            [{"role": "assistant", "content": json.dumps(ex12_refuted_target)}],
            [{"role": "assistant", "content": json.dumps(ex13_no_avoid)}],
            [{"role": "assistant", "content": wrapped_ex12}],
        ],
    )
    assert batch_scores == [0.0, 0.15, 0.30, 0.50, 0.85, 1.0]


def test_verifiable_reward_optional_stdio_mcp_backend_and_strip_thinking():
    assert strip_thinking_blocks("<think>\nstep 1\n</think>\n{\"ok\": true}") == '{"ok": true}'

    ex13_raw = json.loads(EX13_EXAM_PATH.read_text(encoding="utf-8"))
    ex13_doc = _convert_search_exam_to_puzzle_dict(ex13_raw)

    def _mock_verify_puzzle(args: dict[str, Any]) -> dict[str, Any]:
        pdoc = json.loads(args["puzzle_json"])
        t_lines = [ln for ln in pdoc["lines"] if ln["role"] == "target"]
        a_lines = [ln for ln in pdoc["lines"] if ln["role"] == "avoid"]
        return {
            "ok": True,
            "verified": True,
            "target_results": [
                {"id": ln["id"], "passed": True, "final_winner": 1} for ln in t_lines
            ],
            "avoid_results": [
                {"id": ln["id"], "passed": True, "refuted": True, "final_winner": None}
                for ln in a_lines
            ],
        }

    transport = MockMcpTransport(tools={"verify_puzzle": _mock_verify_puzzle})
    with StdioMcpClient(transport) as mcp_client:
        reward_fn = DigimonVerifiableReward(mcp_client=mcp_client, adversary_seeds=2)
        scores = reward_fn(
            prompts=["solve"],
            completions=[f"<think>check mcp</think>\n{json.dumps(ex13_doc)}"],
        )
        assert scores == [1.0]
        assert len(transport.calls) == 2  # Stage 4 target check + Stage 5 full avoid check


# ── 3. Train `--dry-run` (Qwen3-8B & Qwen3-14B) + Lazy Import Tests ───────────


def test_train_dry_run_qwen3_8b_default_and_qwen3_14b_plus_closed_model_guard(tmp_path: Path):
    ds_dir = tmp_path / "datasets"
    summary = export_tactician_datasets(out_dir=ds_dir)

    # 1. Default Qwen/Qwen3-8B via CLI `train --dry-run`
    out_8b = tmp_path / "run_8b"
    rc_8b = cli_main(
        [
            "train",
            "--dry-run",
            "--sft-dataset",
            str(summary.sft_path),
            "--grpo-dataset",
            str(summary.grpo_path),
            "--output-dir",
            str(out_8b),
        ]
    )
    assert rc_8b == 0
    plan_8b = json.loads((out_8b / "dry_run_plan.json").read_text(encoding="utf-8"))
    assert plan_8b["ok"] is True
    assert plan_8b["base_model"] == DEFAULT_BASE_MODEL == "Qwen/Qwen3-8B"
    assert plan_8b["qlora"]["lora_r"] == 32
    assert plan_8b["qlora"]["lora_alpha"] == 64
    assert plan_8b["qlora"]["load_in_4bit"] is True
    assert plan_8b["model_preset"]["co_resides_with_ppo_on_12gb_rtx4070"] is True
    assert plan_8b["model_preset"]["recommended_ollama_tag"] == "digimon-tactician:8b"
    assert plan_8b["dataset_validation"]["sft_rows"] == summary.sft_rows
    assert plan_8b["dataset_validation"]["grpo_rows"] == summary.grpo_rows
    assert plan_8b["dataset_validation"]["smoke_reward_score"] == 1.0

    # 2. Qwen/Qwen3-14B via CLI `train-dry-run`
    out_14b = tmp_path / "run_14b"
    rc_14b = cli_main(
        [
            "train-dry-run",
            "--base-model",
            "Qwen/Qwen3-14B",
            "--sft-dataset",
            str(summary.sft_path),
            "--grpo-dataset",
            str(summary.grpo_path),
            "--output-dir",
            str(out_14b),
        ]
    )
    assert rc_14b == 0
    plan_14b = json.loads((out_14b / "dry_run_plan.json").read_text(encoding="utf-8"))
    assert plan_14b["ok"] is True
    assert plan_14b["base_model"] == "Qwen/Qwen3-14B"
    assert plan_14b["qlora"]["lora_r"] == 32
    assert plan_14b["qlora"]["lora_alpha"] == 64
    assert plan_14b["model_preset"]["co_resides_with_ppo_on_12gb_rtx4070"] is False
    assert plan_14b["model_preset"]["recommended_ollama_tag"] == "digimon-tactician:14b"

    # 3. Closed-weight model IDs are strictly blocked
    for closed_id in ("gpt-4o", "claude-3-7-sonnet", "gemini-4-neon"):
        with pytest.raises(PermissionError):
            validate_train_config(TacticianTrainConfig(base_model=closed_id))

    # 4. Subprocess check: importing `tools.tactician_grpo` + running dry-run never imports heavy GPU libs
    probe_code = (
        "import sys; sys.path.insert(0, 'code'); "
        "import tools.tactician_grpo as tg; "
        "tg.run_sft_and_grpo(tg.TacticianTrainConfig(), dry_run=True, write_plan=False); "
        "heavy = [m for m in ('trl', 'peft', 'transformers', 'datasets', 'bitsandbytes') if m in sys.modules]; "
        "assert not heavy, f'Heavy modules imported eagerly: {heavy}'"
    )
    proc = subprocess.run(
        [sys.executable, "-c", probe_code],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"Lazy-import probe failed:\n{proc.stdout}\n{proc.stderr}"


# ── 4. RTX 4070 Ollama `Modelfile` Exporter & CLI Tests ───────────────────────


def test_export_ollama_modelfile_and_cli_score_completion(tmp_path: Path):
    ollama_dir = tmp_path / "ollama_out"
    rc = cli_main(
        [
            "export-ollama",
            "--from-source",
            "./digimon-tactician-qwen3-8b-q4_k_m.gguf",
            "--base-model",
            "Qwen/Qwen3-8B",
            "--out-dir",
            str(ollama_dir),
        ]
    )
    assert rc == 0

    modelfile_path = ollama_dir / "Modelfile"
    helper_path = ollama_dir / "deploy_ollama.sh"
    manifest_path = ollama_dir / "export_manifest.json"
    assert modelfile_path.exists()
    assert helper_path.exists()
    assert manifest_path.exists()

    mf_text = modelfile_path.read_text(encoding="utf-8")
    assert "FROM ./digimon-tactician-qwen3-8b-q4_k_m.gguf" in mf_text
    assert "PARAMETER temperature 0.20" in mf_text
    assert "PARAMETER num_ctx 16384" in mf_text
    assert 'SYSTEM """You are Digimon Tactician' in mf_text

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert manifest["model_tag"] == "digimon-tactician:8b"
    assert "Q4_K_M" in manifest["commands"]["llama_quantize"]
    assert "ollama create digimon-tactician:8b -f" in manifest["commands"]["ollama_create"]
    assert "--provider ollama" in manifest["commands"]["puzzle_lab_synthesize"]
    assert "digimon-tactician:8b" in manifest["commands"]["puzzle_lab_synthesize"]

    # Optional ADAPTER directive when exporting an unmerged LoRA directory
    mf_adapter = render_ollama_modelfile(
        OllamaExportConfig(
            from_source="qwen3:14b",
            base_model="Qwen/Qwen3-14B",
            adapter_dir=Path("./grpo_adapter"),
        )
    )
    assert "ADAPTER grpo_adapter" in mf_adapter

    # CLI `score-completion` smoke test against real EX12 puzzle file
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "tools.tactician_grpo",
            "score-completion",
            "--completion-file",
            str(EX12_PUZZLE_PATH),
            "--seeds",
            "1",
        ],
        cwd=str(REPO_ROOT),
        env={"PYTHONPATH": str(REPO_ROOT / "code")},
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, f"CLI score-completion failed:\n{proc.stderr}"
    cli_bd = json.loads(proc.stdout)
    assert cli_bd["total"] == 1.0
    assert cli_bd["stage_reached"] == 5


# ── 5. Production AST Import-Isolation Guard ──────────────────────────────────


def test_production_surfaces_never_import_tactician_grpo():
    """Verify `code/server/`, `code/digimon_gym/`, and `code/src-tauri/` never import `tactician_grpo`."""
    py_roots = [
        REPO_ROOT / "code" / "server",
        REPO_ROOT / "code" / "digimon_gym",
    ]
    violations: list[str] = []

    for root in py_roots:
        if not root.exists():
            continue
        for py_file in sorted(root.rglob("*.py")):
            tree = ast.parse(py_file.read_text(encoding="utf-8"), filename=str(py_file))
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    for alias in node.names:
                        if "tactician_grpo" in alias.name:
                            violations.append(f"{py_file}:{node.lineno} imports {alias.name}")
                elif isinstance(node, ast.ImportFrom):
                    mod = node.module or ""
                    if "tactician_grpo" in mod or any("tactician_grpo" in a.name for a in node.names):
                        violations.append(f"{py_file}:{node.lineno} imports from {mod}")

    tauri_root = REPO_ROOT / "code" / "src-tauri"
    if tauri_root.exists():
        for src_file in sorted(tauri_root.rglob("*")):
            if not src_file.is_file() or src_file.suffix not in (".rs", ".py", ".ts", ".tsx", ".json"):
                continue
            if "target" in src_file.parts:
                continue
            txt = src_file.read_text(encoding="utf-8", errors="ignore")
            if "tactician_grpo" in txt:
                violations.append(f"{src_file} references tactician_grpo")

    assert not violations, f"Production import-isolation violated:\n" + "\n".join(violations)


def test_compose_counter_puzzle_compact_and_defender_tamper_guard(tmp_path: Path):
    summary = export_tactician_datasets(out_dir=tmp_path)
    grpo_rows = {
        r["id"]: r
        for r in (json.loads(ln) for ln in summary.grpo_path.read_text(encoding="utf-8").splitlines() if ln.strip())
    }
    ex13_comp_meta = grpo_rows["ex13-imperialdramon-dna-paildramon-jamming-lethal::compose"]["ground_truth_meta"]
    base_puzzle = ex13_comp_meta["base_puzzle"]

    # 1. Compact counter-puzzle completion (`challenger_zones` + `lines`) merges with locked defender_zones -> 1.00
    compact_proposal = {
        "title": "Imperialdramon DNA Bypass",
        "description": "DNA Paildramon suspends Slayerdramon Blocker and swings twice.",
        "challenger_zones": base_puzzle["setup"]["zones"]["1"],
        "lines": base_puzzle["lines"],
    }
    bd_compact = score_single_completion(
        json.dumps(compact_proposal),
        ground_truth_meta=ex13_comp_meta,
        task_kind=TASK_COMPOSE_COUNTER_PUZZLE,
        adversary_seeds=1,
    )
    assert bd_compact.total == 1.00

    # 2. Tampering with Defender's Boss board (`zones.2.field = []`) is caught at Stage 3 (`0.30`)!
    tampered_doc = copy.deepcopy(base_puzzle)
    tampered_doc["setup"]["zones"]["2"]["field"] = []
    bd_tamper = score_single_completion(
        json.dumps(tampered_doc),
        ground_truth_meta=ex13_comp_meta,
        task_kind=TASK_COMPOSE_COUNTER_PUZZLE,
        adversary_seeds=1,
    )
    assert bd_tamper.total == 0.30
    assert bd_tamper.failure_stage == 3


def test_bounded_horizon_win_assertions_verifiable_reward():
    ex13_raw = json.loads(EX13_EXAM_PATH.read_text(encoding="utf-8"))
    ex13_doc = _convert_search_exam_to_puzzle_dict(ex13_raw)

    # Just perform the 0-cost DNA digivolve (63 -> 0 -> 1): memory moves from 2 to 4, P1 field has 1 stack (BT16-025)
    bounded_doc = copy.deepcopy(ex13_doc)
    bounded_doc["lines"] = [
        {"id": "dna_setup", "name": "DNA into Paildramon at 4 memory", "role": "target", "actions": [63, 0, 1]},
        {"id": "bad_normal_evo", "name": "Normal digivolve for 4 memory", "role": "avoid", "actions": [400]},
    ]
    bounded_doc["rl_goal"] = {
        "mode": "bounded_horizon",
        "max_turns": 1,
        "max_actions": 4,
        "win_assertions": [
            {"kind": "memory_equals", "value": 4},
            {"kind": "stack_top", "player": 1, "field_index": 0, "card_id": "BT16-025"},
            {"kind": "zone_count", "player": 1, "zone": "field", "value": 1},
        ],
    }
    bd_ok = score_single_completion(json.dumps(bounded_doc), adversary_seeds=1)
    assert bd_ok.total == 1.00

    # Exceeding `max_actions` (setting max_actions=2 on a 3-step target line) fails Stage 4 -> 0.50
    bounded_expired = copy.deepcopy(bounded_doc)
    bounded_expired["rl_goal"]["max_actions"] = 2
    bd_exp = score_single_completion(json.dumps(bounded_expired), adversary_seeds=1)
    assert bd_exp.total == 0.50
    assert bd_exp.failure_stage == 4


def test_run_sft_and_grpo_wires_sft_trainer_to_grpo_trainer_with_verifiable_reward(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
):
    from tools.tactician_grpo import train_grpo as tg_mod

    ds_dir = tmp_path / "datasets"
    summary = export_tactician_datasets(out_dir=ds_dir)
    captured: dict[str, Any] = {}

    class _FakeTokenizer:
        pad_token = None
        eos_token = "<eos>"

        def save_pretrained(self, path: str) -> None:
            Path(path).mkdir(parents=True, exist_ok=True)

    class _FakeModel:
        pass

    class _FakeAutoTokenizer:
        @staticmethod
        def from_pretrained(model_id: str, **kwargs: Any) -> _FakeTokenizer:
            captured["tokenizer_model"] = model_id
            return _FakeTokenizer()

    class _FakeAutoModel:
        @staticmethod
        def from_pretrained(model_id: str, **kwargs: Any) -> _FakeModel:
            captured.setdefault("loaded_models", []).append((model_id, kwargs))
            return _FakeModel()

    class _FakeLoraConfig:
        def __init__(self, **kwargs: Any) -> None:
            self.kwargs = kwargs
            captured["lora_config"] = kwargs

    class _FakePeftModel:
        @staticmethod
        def from_pretrained(base_model: Any, adapter_dir: str, is_trainable: bool = False) -> Any:
            captured["peft_loaded"] = (adapter_dir, is_trainable)
            return base_model

    class _FakeSFTConfig:
        def __init__(self, output_dir: str, max_length: int = 8192, **kwargs: Any) -> None:
            self.output_dir = output_dir
            self.max_length = max_length
            self.kwargs = kwargs

    class _FakeSFTTrainer:
        def __init__(self, model: Any, args: Any, train_dataset: Any, peft_config: Any = None, processing_class: Any = None) -> None:
            captured["sft_init"] = {"args": args, "rows": len(train_dataset), "peft": peft_config}
            self.out = args.output_dir

        def train(self) -> None:
            captured["sft_trained"] = True

        def save_model(self, path: str) -> None:
            Path(path).mkdir(parents=True, exist_ok=True)
            (Path(path) / "adapter_config.json").write_text("{}", encoding="utf-8")

    class _FakeGRPOConfig:
        def __init__(self, output_dir: str, num_generations: int = 8, **kwargs: Any) -> None:
            self.output_dir = output_dir
            self.num_generations = num_generations
            self.kwargs = kwargs

    class _FakeGRPOTrainer:
        def __init__(self, model: Any, reward_funcs: list[Any], args: Any, train_dataset: Any, processing_class: Any = None, peft_config: Any = None) -> None:
            captured["grpo_init"] = {
                "reward_funcs": reward_funcs,
                "num_generations": args.num_generations,
                "rows": len(train_dataset),
                "peft": peft_config,
            }
            # Exercise the wired reward callable directly on the loaded HF dataset row!
            row0 = train_dataset[0]
            scores = reward_funcs[0](
                prompts=[row0["prompt"]],
                completions=['{"not_valid_lines": true}'],
                ground_truth_meta=[row0["ground_truth_meta"]],
                task_kind=[row0["task_kind"]],
            )
            captured["grpo_probe_scores"] = scores

        def train(self) -> None:
            captured["grpo_trained"] = True

        def save_model(self, path: str) -> None:
            Path(path).mkdir(parents=True, exist_ok=True)
            (Path(path) / "adapter_config.json").write_text("{}", encoding="utf-8")

    class _FakeDatasetMod:
        class Dataset:
            @staticmethod
            def from_list(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
                return list(rows)

    class _FakeTorch:
        bfloat16 = "bf16"

        class cuda:
            @staticmethod
            def is_available() -> bool:
                return False

            @staticmethod
            def empty_cache() -> None:
                pass

    class _FakeBnbConfig:
        def __init__(self, **kwargs: Any) -> None:
            captured["bnb_config"] = kwargs

    monkeypatch.setattr(
        tg_mod,
        "_load_training_libraries",
        lambda: {
            "torch": _FakeTorch,
            "transformers": type("TF", (), {"AutoTokenizer": _FakeAutoTokenizer, "AutoModelForCausalLM": _FakeAutoModel}),
            "peft": type("PEFT", (), {"LoraConfig": _FakeLoraConfig, "PeftModel": _FakePeftModel}),
            "trl": type("TRL", (), {"SFTConfig": _FakeSFTConfig, "SFTTrainer": _FakeSFTTrainer, "GRPOConfig": _FakeGRPOConfig, "GRPOTrainer": _FakeGRPOTrainer}),
            "datasets": _FakeDatasetMod,
            "BitsAndBytesConfig": _FakeBnbConfig,
        },
    )

    run_out = tmp_path / "full_mock_run"
    manifest = run_sft_and_grpo(
        TacticianTrainConfig(
            base_model="Qwen/Qwen3-8B",
            sft_dataset_path=summary.sft_path,
            grpo_dataset_path=summary.grpo_path,
            output_dir=run_out,
        ),
        dry_run=False,
    )

    assert manifest["executed"] is True
    assert Path(manifest["manifest_path"]).exists()
    assert captured["lora_config"]["r"] == 32
    assert captured["lora_config"]["lora_alpha"] == 64
    assert captured["bnb_config"]["load_in_4bit"] is True
    assert captured["sft_trained"] is True
    assert captured["grpo_trained"] is True
    assert captured["peft_loaded"][1] is True  # SFT adapter loaded with is_trainable=True for GRPO
    assert isinstance(captured["grpo_init"]["reward_funcs"][0], DigimonVerifiableReward)
    assert captured["grpo_probe_scores"] == [0.0]

