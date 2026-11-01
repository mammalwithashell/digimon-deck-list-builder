"""Opt-in real native startup/receipt boundary checks; no model or search runs.

Requires a freshly built frozen_checkpoint_modes executable and an evidence
directory. Invalid fixtures deliberately stop before model loading. This does
not qualify the scored protocol or prove failures after gameplay starts.
"""
import json
import os
from pathlib import Path
import subprocess

import pytest

from tools.cloud_search import frozen_mode_campaign as campaign

BINARY = os.environ.get("DIGIMON_NATIVE_STARTUP_BINARY")
EVIDENCE = os.environ.get("DIGIMON_NATIVE_STARTUP_EVIDENCE")
pytestmark = pytest.mark.skipif(
    not BINARY or not EVIDENCE, reason="explicit native startup boundary fixture"
)
SCHEMA = "frozen-information-regime-eval-v1"
CASES = [
    ("request_schema", "fixture-request-schema"),
    ("plan_schema", "fixture-plan-schema"),
    ("candidate_budget", "319"),
    ("opponent_noise", "Some(0.25)"),
    ("runtime_binding", "fixture-runtime-mismatch"),
    ("frozen_cards", "frozen cards_json mismatch"),
    ("closed_mechanics", "mechanics training gate is closed"),
    ("existing_output", "preserve original result"),
    ("malformed_json", "key must be a string"),
]


def capacity_before_native(name):
    """Each actual native allocation gets fresh local host evidence."""
    assert os.name == "nt", "this evidence lane records Windows capacity"
    script = r"""
$taskProcesses=@(Get-CimInstance Win32_Process | Where-Object {
 $_.Name -match '^(cargo|rustc|digimon_engine|frozen_checkpoint_modes|replay_closed_audit).*\.exe$' -or
 ($_.Name -match '^python.*\.exe$' -and $_.CommandLine -match 'p3-constructs|frozen_mode_campaign|frozen_mode_queue')
} | Select-Object ProcessId,ParentProcessId,Name,CommandLine,WorkingSetSize,PeakWorkingSetSize)
$taskOs=Get-CimInstance Win32_OperatingSystem
$taskCpu=(Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average
$taskDisk=Get-CimInstance Win32_LogicalDisk -Filter "DeviceID='D:'"
[ordered]@{at=[DateTime]::UtcNow.ToString('o');cpu_percent=$taskCpu;free_ram_bytes=([int64]$taskOs.FreePhysicalMemory*1024);free_disk_bytes=$taskDisk.FreeSpace;processes=$taskProcesses} | ConvertTo-Json -Depth 5
"""
    raw = subprocess.run(
        ["powershell", "-NoProfile", "-Command", script],
        check=True, capture_output=True, text=True, timeout=30,
    ).stdout
    evidence = json.loads(raw)
    campaign.write(Path(EVIDENCE) / f"capacity-{name}.json", evidence)
    assert evidence["cpu_percent"] <= 90
    assert evidence["free_ram_bytes"] >= 8 * 1024**3 + 1.5 * 2831593472
    assert evidence["free_disk_bytes"] >= 3 * 1024**3
    native = [p for p in evidence["processes"] if p["Name"].lower().startswith(
        ("cargo", "rustc", "digimon_engine", "frozen_checkpoint_modes", "replay_closed_audit")
    )]
    assert not native, native


@pytest.mark.parametrize("case,expected_error", CASES, ids=[x[0] for x in CASES])
def test_native_startup_failure_cannot_publish_done_or_retry(tmp_path, case, expected_error):
    binary = Path(BINARY).resolve()
    assert binary.is_file()
    for directory in ("requests", "results", "attempts", "logs", "assets"):
        (tmp_path / directory).mkdir()
    # Deliberately invalid model bytes. Every selected failure precedes model loading.
    for filename, value in [
        ("cards.json", {}), ("prior.json", {}),
        ("mechanics_readiness.json", {"ready": False, "blocking": ["startup fixture"]}),
        ("card_admissibility.json", {"cards": {}}),
    ]:
        campaign.write(tmp_path / "assets" / filename, value)
    (tmp_path / "assets" / "candidate.onnx").write_bytes(b"never load fixture candidate")
    (tmp_path / "assets" / "opponent.onnx").write_bytes(b"never load fixture opponent")
    plan = {"schema": SCHEMA, "native_sha256": campaign.digest(binary), "decks": []}
    for pathkey, hashkey, filename in [
        ("cards_json", "cards_sha256", "cards.json"),
        ("model", "model_sha256", "candidate.onnx"),
        ("opponent_model", "opponent_model_sha256", "opponent.onnx"),
        ("prior", "prior_file_sha256", "prior.json"),
    ]:
        asset = tmp_path / "assets" / filename
        plan[pathkey] = str(asset.resolve())
        plan[hashkey] = campaign.digest(asset)
    settings = dict(kind="search", simulations=320, worlds=4, batch=4,
                    root_noise=0.0, information_regime="closed-list")
    request = dict(schema=SCHEMA, id=case, plan=str((tmp_path / "plan.json").resolve()),
                   candidate=dict(settings), opponent=dict(settings),
                   runtime_sha256=plan["native_sha256"])
    job = dict(id=case, game_seed=42, deck_indices=[0, 1], checkpoint=12)
    if case == "request_schema":
        request["schema"] = "fixture-request-schema"
    elif case == "plan_schema":
        plan["schema"] = "fixture-plan-schema"
    elif case == "candidate_budget":
        request["candidate"]["simulations"] = 319
    elif case == "opponent_noise":
        request["opponent"]["root_noise"] = 0.25
    elif case == "runtime_binding":
        plan["native_sha256"] = "fixture-runtime-mismatch"
    elif case == "frozen_cards":
        plan["cards_sha256"] = "fixture-cards-mismatch"
    campaign.write(tmp_path / "plan.json", plan)
    reqpath = tmp_path / "requests" / f"{case}.json"
    campaign.write(reqpath, request)
    if case == "malformed_json":
        reqpath.write_bytes(b"{broken")
    outpath = tmp_path / "results" / f"{case}.json"
    if case == "existing_output":
        outpath.write_bytes(b'{"fixture":"preserve existing output byte for byte"}')
    original_output = outpath.read_bytes() if outpath.exists() else None
    # Diagnostic artifacts are not completed results, even when left by earlier work.
    active = outpath.with_suffix(".active.json")
    progress = outpath.with_suffix(".progress.json")
    active.write_bytes(b'{"fixture":"retained active"}')
    progress.write_bytes(b'{"fixture":"retained progress"}')
    diagnostics = {p: p.read_bytes() for p in (active, progress)}
    before_request = reqpath.read_bytes()
    capacity_before_native(case)
    protocol = dict(binary=str(binary), deadline_seconds=120)
    assert campaign.execute(tmp_path, protocol, plan, job) is False
    rejected = tmp_path / "results" / f"{case}.rejected.json"
    done = tmp_path / "results" / f"{case}.done.json"
    record = campaign.read(rejected)
    assert not done.exists()
    assert not (tmp_path / "completion-verified.json").exists()
    assert record["request"] == job
    assert record["request_sha256"] == campaign.digest(reqpath)
    assert record["model_sha256"] == plan["model_sha256"]
    assert record["opponent_model_sha256"] == plan["opponent_model_sha256"]
    assert "returned non-zero exit status" in record["error"]
    stderr = (tmp_path / "logs" / f"{case}.err.log").read_text(encoding="utf8")
    assert "panicked at" in stderr and expected_error in stderr, stderr
    assert (outpath.read_bytes() if outpath.exists() else None) == original_output
    assert not outpath.with_suffix(".tmp").exists()
    assert all(p.read_bytes() == value for p, value in diagnostics.items())
    assert reqpath.read_bytes() == before_request
    preserved = {p: p.read_bytes() for p in [
        tmp_path / "attempts" / f"{case}.json", rejected,
        tmp_path / "logs" / f"{case}.err.log", tmp_path / "logs" / f"{case}.out.log",
    ]}
    with pytest.raises(FileExistsError):
        campaign.execute(tmp_path, protocol, plan, job)
    assert all(p.read_bytes() == value for p, value in preserved.items())
    assert not done.exists()
    campaign.write(Path(EVIDENCE) / f"case-{case}.json", dict(
        case=case, fixture_root=str(tmp_path), native_sha256=campaign.digest(binary),
        expected_error=expected_error, actual_rejection=record,
        request_sha256=campaign.digest(reqpath), rejected_sha256=campaign.digest(rejected),
        stderr_sha256=campaign.digest(tmp_path / "logs" / f"{case}.err.log"),
        done_absent=True, prior_output_preserved=original_output is not None,
        diagnostic_bytes_preserved=True, repeat_attempt_blocked=True,
        scope="native startup before model loading; no gameplay, search or model qualification",
    ))
