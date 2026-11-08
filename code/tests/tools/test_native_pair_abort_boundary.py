"""Real subprocess tests of the shared pair loop, using diagnostic-only schema."""
import json
import os
from pathlib import Path
import subprocess
import pytest
from tools.cloud_search import frozen_mode_campaign as campaign

BINARY=os.environ.get("DIGIMON_PAIR_ABORT_BINARY")
EVIDENCE=os.environ.get("DIGIMON_PAIR_ABORT_EVIDENCE")
pytestmark=pytest.mark.skipif(not BINARY or not EVIDENCE,reason="explicit native pair boundary fixture")
SCHEMA="native-pair-abort-fixture-v1"
def capacity_before_native(name):
    """Each actual native allocation gets fresh local host evidence."""
    assert os.name == "nt", "this evidence lane records Windows capacity"
    script = r"""
$taskProcesses=@(Get-CimInstance Win32_Process | Where-Object {
 $_.Name -match '^(cargo|rustc|digimon_engine|frozen_checkpoint_modes|frozen_request_abort_probe|replay_closed_audit).*\.exe$' -or
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
        ("cargo", "rustc", "digimon_engine", "frozen_checkpoint_modes", "frozen_request_abort_probe", "replay_closed_audit")
    )]
    assert not native, native


@pytest.mark.parametrize("actor",[0,1])
@pytest.mark.parametrize("failure",["control","first-panic","second-panic","second-incomplete"])
def test_real_pair_work_never_publishes_partial_success(tmp_path,actor,failure):
    for name in ("requests","results","attempts","logs"):
        (tmp_path/name).mkdir()
    name=f"{failure}-{actor}"
    request=dict(schema=SCHEMA,id=name,actor=actor,failure=failure,search_seed=143,
                 runtime_sha256=campaign.digest(BINARY))
    job=dict(id=name,game_seed=101,deck_indices=[0,1],checkpoint="uniform-fixture")
    plan=dict(model_sha256="uniform-fixture-not-a-model",opponent_model_sha256="uniform-fixture-not-a-model")
    reqpath=tmp_path/f"requests/{name}.json"
    outpath=tmp_path/f"results/{name}.json"
    campaign.write(reqpath,request)
    protocol=dict(binary=str(Path(BINARY).resolve()),deadline_seconds=120)
    capacity_before_native("pair-"+name)
    assert campaign.execute(tmp_path,protocol,plan,job) is False
    rejected=tmp_path/f"results/{name}.rejected.json"
    record=campaign.read(rejected)
    assert record["request"]==job
    assert record["request_sha256"]==campaign.digest(reqpath)
    assert not (tmp_path/f"results/{name}.done.json").exists()
    assert not (tmp_path/"completion-verified.json").exists()
    trace_path=outpath.with_suffix(".trace.jsonl")
    events=[json.loads(line) for line in trace_path.read_text(encoding="utf8").splitlines()]
    starts=[e["game"] for e in events if e["event"]=="game_start"]
    stderr=(tmp_path/f"logs/{name}.err.log").read_text(encoding="utf8")
    if failure=="control":
        assert record["error"]=="untagged schema"
        assert "panicked at" not in stderr
        output=campaign.read(outpath)
        assert output["schema"]==SCHEMA and output["request"]==request
        assert starts==[0,1] and events[-1]["event"]=="pair_returned"
        assert len(output["games"])==2
        for seat,game in enumerate(output["games"]):
            assert game["candidate_seat"]==seat
            assert game["first_player"]==actor and game["winner"]==actor
            assert game["score"]==float(actor==seat) and game["truncated"] is False
            assert game["decisions"]>0 and game["searched"]>0
            assert game["terminal_reason"]=="Some(SecurityAttack)"
            for step in game["card_metrics"]["steps"]:
                assert step["action"] in step["legal_actions"]
            for search in game["search_diagnostics"]:
                assert search["diagnostics"]["simulations"]==320
                assert search["diagnostics"]["worlds"]==4 and search["diagnostics"]["batch"]==4
                assert search["diagnostics"]["information"]["regime"]=="known-list"
                assert search["opportunity"]["commitment_executed"] is True
    else:
        assert "returned non-zero exit status" in record["error"]
        assert not outpath.exists() and not outpath.with_suffix(".tmp").exists()
        assert not any(e["event"]=="pair_returned" for e in events)
        assert starts==([0] if failure=="first-panic" else [0,1])
        if failure.endswith("panic"):
            failed_game=0 if failure=="first-panic" else 1
            assert f"native pair fixture evaluator failure: game={failed_game}, call=2" in stderr
            assert any(e["event"]=="eval_complete" and e["game"]==failed_game and e["call"]==1 for e in events)
            assert not any(e["event"]=="eval_complete" and e["game"]==failed_game and e["call"]==2 for e in events)
            active=campaign.read(outpath.with_suffix(".active.json"))
            assert active["candidate_seat"]==failed_game and active["viewer"]==actor
            assert not outpath.with_suffix(".failure.json").exists()
        else:
            assert "incomplete game; no completed score" in stderr
            failed=campaign.read(outpath.with_suffix(".failure.json"))
            assert failed["candidate_seat"]==1 and len(failed["actions"])==1
            assert failed["error"]=="incomplete game"
        if failure!="first-panic":
            assert any(e["event"]=="eval_complete" and e["game"]==0 for e in events)
    preserved={p:p.read_bytes() for p in tmp_path.rglob("*") if p.is_file()}
    with pytest.raises(FileExistsError):
        campaign.execute(tmp_path,protocol,plan,job)
    assert all(p.read_bytes()==data for p,data in preserved.items())
    campaign.write(Path(EVIDENCE)/f"pair-case-{name}.json",dict(
        case=name,fixture_root=str(tmp_path),native_sha256=campaign.digest(BINARY),
        trace_sha256=campaign.digest(trace_path),rejected_sha256=campaign.digest(rejected),
        native_completed_pair=failure=="control",successful_scored_receipt=False,
        starts=starts,failed_after_completed_first_game=failure in ("second-panic","second-incomplete"),
        first_game_completion_evidence="Second factory invocation is after prior terminal check and games.push in shared exact loop.",
        output_present=outpath.exists(),error=record["error"],
    ))
