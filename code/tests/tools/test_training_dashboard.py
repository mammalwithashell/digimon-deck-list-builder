import json
from pathlib import Path
import time
from types import SimpleNamespace

import pytest

from tools.cloud_search.dashboard_snapshot import collect, compact_status, latest_reports, merge_snapshot, scheduler_progress, search_progress, summarize_report
from tools.cloud_search.eval_progress import EvaluationProgress


def report(status="complete", started=1):
    context = dict(anchor="greedy", agent_archetype="DNA deck", agent_deck_id="dna-list")
    return dict(schema_version=1, checkpoint_step=0, window_id=str(started),
        provenance=dict(status=status, panel="meta_validation", evaluation_started_unix=started), coverage={},
        cards=[], routes=[], games=[
            dict(context=context, winner_id=1, terminal_reason="security_attack", steps=4,
                 cards=[dict(player=1, dna_digivolves=1)],
                 routes=[dict(player=1, route="dna_digivolve", opportunities=2, choices=1)]),
            dict(context=context, winner_id=None, terminal_reason="evaluation_end", steps=100, cards=[], routes=[]),
            dict(context={**context, "agent_deck_id":"not-dna"}, winner_id=2, terminal_reason="security_attack", steps=5, cards=[], routes=[])])


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value))


def test_outcomes_use_games_and_dna_uses_exact_eligible_lists():
    result = summarize_report(report(), "r", "mixed", [1, 2], {"dna-list":{"BT-X":4}})
    assert result["outcomes"] == [dict(anchor="greedy", archetype="DNA deck", wins=1, losses=1, draws=0, incomplete=1, decisions=109)]
    assert result["dna"][0]["games"] == 1
    assert result["dna"][0]["choices"] == result["dna"][0]["entries"] == 1
    assert result["dna"][0]["opportunities"] == 2


def test_snapshot_cache_excludes_archives_and_invalidates_changed_reports(tmp_path):
    write(tmp_path/"started.json", dict(plan={}))
    write(tmp_path/"arms/mixed/experiment_state.json", dict(stage="checkpoint_evaluation"))
    file=tmp_path/"evaluations/mixed/window-000000/reports/checkpoint_0_test.json"
    write(file, report())
    write(tmp_path/"resume-history/old/window-000000/reports/checkpoint_0_old.json", report("failed"))
    first=collect(tmp_path)
    assert len(first["reports"])==1
    second=collect(tmp_path,first["report_index"])
    assert second["reports"]==[]
    assert len(merge_snapshot(first,second)["reports"])==1
    file.write_text('{"broken":')
    changed=collect(tmp_path,first["report_index"])
    assert changed["errors"] and not merge_snapshot(first,changed)["reports"]
    file.unlink()
    assert not merge_snapshot(first,collect(tmp_path))["reports"]


def test_latest_windows_do_not_pool_repeated_or_failed_samples():
    rows=[summarize_report(report(started=i,status=s),str(i),"mixed",[i,i])
          for i,s in [(1,"complete"),(2,"complete"),(3,"failed")]]
    assert [r["window_id"] for r in latest_reports(rows)]==["2"]


def test_eta_is_withheld_for_stale_or_insufficient_progress():
    p=dict(committed_games=100,requested_games=1100,elapsed_seconds_this_invocation=300,
           heartbeat_unix=1000,status="running",measured_games_this_invocation=100)
    assert search_progress(p,1010)["eta_seconds"]==3000
    assert search_progress(p,1201)["eta_seconds"] is None
    assert search_progress({**p,"measured_games_this_invocation":2},1010)["eta_seconds"] is None
    assert search_progress({**p,"status":"failed"},1010)["eta_seconds"] is None


def test_progress_heartbeat_records_only_finished_games_and_marks_failure(tmp_path):
    path=tmp_path/"progress.json"
    p=EvaluationProgress(path,total=20,panel="meta",checkpoint=1,interval=60)
    event=dict(anchor="greedy",archetype="A",opponent_archetype="B",index=0)
    try:
        p.event(dict(event,event="game_started"))
        p.publish()
        assert json.loads(path.read_text())["games_completed"]==0
        p.event(dict(event,event="game_completed"))
        p.event(dict(event,event="game_started",index=1))
    finally:
        p.finish("failed","native panic")
    data=json.loads(path.read_text())
    assert data["games_completed"]==1 and data["status"]=="failed"
    assert data["error"]=="native panic" and data["eta_seconds"] is None


def test_unwritable_progress_does_not_stop_evaluation(tmp_path, caplog):
    blocked=tmp_path/"not_a_directory"
    blocked.write_text("x")
    p=EvaluationProgress(blocked/"progress.json",total=2,panel="meta",checkpoint=0,interval=60)
    p.finish("complete")
    assert caplog.text.count("progress output unavailable") == 1


def test_resumed_search_rate_does_not_count_previously_committed_games(tmp_path):
    write(tmp_path/"scheduler_progress.json", dict(committed_games=200, requested_games=1100,
        heartbeat_unix=1000, elapsed_seconds_this_invocation=100, status="running"))
    events = [dict(event="chunk_committed", unix=t, games=n) for t,n in [(800,180),(950,20),(1001,8)]]
    (tmp_path/"scheduler_events.jsonl").write_text("\n".join(json.dumps(e) for e in events))
    result = search_progress(scheduler_progress(tmp_path), 1010)
    assert result["games_per_hour"] == 720
    assert result["eta_seconds"] == 4500
    (tmp_path/"scheduler_events.jsonl").unlink()
    assert search_progress(scheduler_progress(tmp_path), 1010)["eta_seconds"] is None


def test_dna_catalog_requires_matching_frozen_card_data(tmp_path):
    write(tmp_path/"started.json", dict(plan={}))
    write(tmp_path/"arms/mixed/experiment_state.json", {})
    write(tmp_path/"corpora/mixed/provenance.json", dict(cards_sha256="frozen"))
    write(tmp_path/"inputs/heldout_snapshot.json", dict(decks=[dict(deck_id="dna-list", card_counts={"BT-X":4})]))
    write(tmp_path/"evaluations/mixed/window-000000/reports/checkpoint_0_test.json", report())
    invalid=collect(tmp_path,dna_catalog=dict(cards_sha256="different",card_ids=["BT-X"]))
    assert not invalid["reports"][0]["dna"]
    valid=collect(tmp_path,dna_catalog=dict(cards_sha256="frozen",card_ids=["BT-X"]))
    assert valid["reports"][0]["dna"][0]["games"] == 1
    summary=compact_status(valid, now=valid["collected_at"]+151)
    assert not summary["fresh"] and summary["results"][0]["completed_games"] == 2


def test_sync_reuses_reports_and_preserves_snapshot_on_connection_failure(tmp_path, monkeypatch):
    from tools.cloud_search import sync_dashboard as sync
    root, cache = tmp_path/"experiment", tmp_path/"cache"
    write(root/"started.json", dict(plan={}))
    write(root/"arms/mixed/experiment_state.json", {})
    write(root/"evaluations/mixed/window-000000/reports/checkpoint_0_test.json", report())
    config = dict(root=str(root), cards_json=str(tmp_path/"missing.json"))
    sync.synchronize(config, cache)
    assert json.loads((cache/"sync_status.json").read_text())["changed_reports"] == 1
    sync.synchronize(config, cache)
    assert json.loads((cache/"sync_status.json").read_text())["changed_reports"] == 0
    previous = (cache/"snapshot.json").read_bytes()
    def disconnect(*args):
        raise OSError("connection unavailable")
    monkeypatch.setattr(sync, "synchronize", disconnect)
    with pytest.raises(SystemExit):
        sync.run_loop(SimpleNamespace(out=cache, once=True), config)
    assert (cache/"snapshot.json").read_bytes() == previous
    assert json.loads((cache/"sync_status.json").read_text())["status"] == "error"


def test_sync_rejects_ssh_option_injection(tmp_path):
    from tools.cloud_search.sync_dashboard import synchronize
    with pytest.raises(ValueError, match="Invalid SSH host"):
        synchronize(dict(host="-oProxyCommand=bad", root="/example", cards_json=str(tmp_path/"missing.json")), tmp_path)


def test_progress_observer_preserves_matchup_seeds_and_decks(monkeypatch):
    from digimon_gym.agents import anchored_eval as evaluation
    decks=[SimpleNamespace(card_ids=[x],archetype_name=x,deck_id=x) for x in ["A","B"]]
    pool=SimpleNamespace(archetypes={d.archetype_name:[d] for d in decks})
    calls=[]
    def play(_agent,_opponent,first,second,seed,_profile,**kwargs):
        calls.append((first,second,seed))
        return 1 if seed%2 else 2
    monkeypatch.setattr(evaluation,"play_one",play)
    first=evaluation.play_matchup(None,None,pool,8,17,"standard_lite_v2",games_per_archetype=4)
    baseline=list(calls);calls.clear();events=[]
    second=evaluation.play_matchup(None,None,pool,8,17,"standard_lite_v2",games_per_archetype=4,on_progress=events.append)
    assert calls==baseline and first==second
    assert len(events)==16
    assert [e["event"] for e in events]==["game_started","game_completed"]*8


def test_dashboard_renders_stale_status_and_empty_filters(tmp_path,monkeypatch):
    pytest.importorskip("streamlit")
    from streamlit.testing.v1 import AppTest
    from digimon_gym.agents.card_eval import CardEvalCollector
    collector=CardEvalCollector(0,0)
    collector.context=dict(anchor="greedy",panel="meta_validation",agent_archetype="A",opponent_archetype="B")
    collector.start_game({},(["A"],["B"]),1)
    collector.finish_game(1,"security_attack")
    raw=collector.report();raw["provenance"]={"status":"complete","panel":"meta_validation"}
    summary=summarize_report(raw,"one","mixed",[1,1])
    write(tmp_path/"snapshot.json",dict(schema_version=1,collected_at=1,root="fixture",state={"stage":"failed","error":"fixture error"},
        plan={"games_per_evaluation":5200},arms=[dict(name="mixed",state={"stage":"failed"},generations=[],windows=[])],
        reports=[summary],comparisons={},errors=[]))
    monkeypatch.setenv("DIGIMON_DASHBOARD_CACHE",str(tmp_path))
    app=AppTest.from_file(str(Path(__file__).parents[2]/"tools/training_dashboard.py"),default_timeout=30).run()
    assert not app.exception
    assert any("stale" in w.value for w in app.warning)
    assert any("fixture error" in e.value for e in app.error)
    next(m for m in app.multiselect if m.label=="Seeds").set_value([]).run()
    assert not app.exception
    assert any("No completed card telemetry" in i.value for i in app.info)
