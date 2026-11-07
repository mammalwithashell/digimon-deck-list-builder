"""Actual checked-reference reads through the shared native pair boundary.

The native fixture obtains the value from a separate, actually materialized
starter position. It reads that retained value during Uniform evaluation after
earlier pair work; this is not a learned model or a scored request.
"""
import json
from pathlib import Path

import pytest
from tools.cloud_search import frozen_mode_campaign as campaign
from test_native_pair_abort_boundary import BINARY, EVIDENCE, SCHEMA, capacity_before_native

pytestmark = pytest.mark.skipif(
    not BINARY or not EVIDENCE, reason="explicit native reference boundary fixture"
)


@pytest.mark.parametrize("actor", [0, 1])
@pytest.mark.parametrize("failure", ["first-reference", "second-reference", "reference-control"])
def test_checked_reference_read_cannot_publish_partial_pair(tmp_path, actor, failure):
    for name in ("requests", "results", "attempts", "logs"):
        (tmp_path / name).mkdir()
    name = f"{failure}-{actor}"
    request = dict(schema=SCHEMA, id=name, actor=actor, failure=failure,
                   search_seed=143, runtime_sha256=campaign.digest(BINARY))
    job = dict(id=name, game_seed=101, deck_indices=[0, 1], checkpoint="uniform-fixture")
    plan = dict(model_sha256="uniform-fixture-not-a-model",
                opponent_model_sha256="uniform-fixture-not-a-model")
    reqpath = tmp_path / f"requests/{name}.json"
    outpath = tmp_path / f"results/{name}.json"
    campaign.write(reqpath, request)
    protocol = dict(binary=str(Path(BINARY).resolve()), deadline_seconds=120)
    capacity_before_native("reference-" + name)
    assert campaign.execute(tmp_path, protocol, plan, job) is False
    rejected = tmp_path / f"results/{name}.rejected.json"
    record = campaign.read(rejected)
    assert record["request"] == job
    assert record["request_sha256"] == campaign.digest(reqpath)
    assert not (tmp_path / f"results/{name}.done.json").exists()
    assert not (tmp_path / "completion-verified.json").exists()
    trace_path = outpath.with_suffix(".trace.jsonl")
    events = [json.loads(line) for line in trace_path.read_text(encoding="utf8").splitlines()]
    starts = [e["game"] for e in events if e["event"] == "game_start"]
    reads = [e["game"] for e in events if e["event"] == "reference_read_begin"]
    completed_reads = [e["game"] for e in events if e["event"] == "reference_read_complete"]
    stderr = (tmp_path / f"logs/{name}.err.log").read_text(encoding="utf8")
    if failure == "reference-control":
        assert record["error"] == "untagged schema"
        assert "panicked at" not in stderr
        output = campaign.read(outpath)
        assert output["schema"] == SCHEMA and output["request"] == request
        assert starts == [0, 1] and reads == [0, 1] and completed_reads == [0, 1]
        assert events[-1]["event"] == "pair_returned"
        assert len(output["games"]) == 2
        for seat, game in enumerate(output["games"]):
            assert game["candidate_seat"] == seat
            assert game["first_player"] == actor and game["winner"] == actor
            assert game["score"] == float(actor == seat) and game["truncated"] is False
            assert game["decisions"] > 0 and game["searched"] > 0
            assert game["terminal_reason"] == "Some(SecurityAttack)"
            for step in game["card_metrics"]["steps"]:
                assert step["action"] in step["legal_actions"]
            for search in game["search_diagnostics"]:
                assert search["diagnostics"]["simulations"] == 320
                assert search["diagnostics"]["worlds"] == 4
                assert search["diagnostics"]["batch"] == 4
                assert search["diagnostics"]["information"]["regime"] == "known-list"
                assert search["opportunity"]["commitment_executed"] is True
    else:
        failed_game = int(failure == "second-reference")
        assert "returned non-zero exit status" in record["error"]
        assert "closed-list unavailable event-card reference read" in stderr
        assert not outpath.exists() and not outpath.with_suffix(".tmp").exists()
        assert not any(e["event"] == "pair_returned" for e in events)
        assert starts == ([0] if failed_game == 0 else [0, 1])
        assert reads == [failed_game] and completed_reads == []
        assert any(e["event"] == "eval_complete" and e["game"] == failed_game
                   and e["call"] == 1 for e in events)
        assert not any(e["event"] == "eval_complete" and e["game"] == failed_game
                       and e["call"] == 2 for e in events)
        active = campaign.read(outpath.with_suffix(".active.json"))
        assert active["candidate_seat"] == failed_game and active["viewer"] == actor
        assert not outpath.with_suffix(".failure.json").exists()
        if failed_game == 1:
            assert any(e["event"] == "eval_complete" and e["game"] == 0 for e in events)
    preserved = {p: p.read_bytes() for p in tmp_path.rglob("*") if p.is_file()}
    with pytest.raises(FileExistsError):
        campaign.execute(tmp_path, protocol, plan, job)
    assert all(p.read_bytes() == data for p, data in preserved.items())
    campaign.write(Path(EVIDENCE) / f"reference-case-{name}.json", dict(
        case=name, fixture_root=str(tmp_path), native_sha256=campaign.digest(BINARY),
        trace_sha256=campaign.digest(trace_path), rejected_sha256=campaign.digest(rejected),
        native_completed_pair=failure == "reference-control", successful_scored_receipt=False,
        starts=starts, checked_reads=reads, completed_reads=completed_reads,
        failed_after_completed_first_game=failure == "second-reference",
        first_game_completion_evidence="Second factory invocation follows the prior terminal check and games.push in the shared loop.",
        reference_source="Actual closed materialization of a separate synthetic starter position.",
        read_site="Uniform evaluator call two; not a learned model or engine callback.",
        process_memory_units="WorkingSetSize bytes; Win32_Process PeakWorkingSetSize KiB.",
        output_present=outpath.exists(), error=record["error"],
    ))
