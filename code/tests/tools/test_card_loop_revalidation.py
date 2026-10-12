"""Revalidation is a new evidence generation, never a rewrite of old state."""
import copy
import json
from pathlib import Path
from types import SimpleNamespace

import pytest
from test_card_loop_source_snapshot import snapshot
from tools.card_loop.driver_contracts import ItemRecord, StageOutcome
from tools.card_loop.stages.base import StageDeferred


@pytest.fixture
def revalidation_ctx(snapshot):
    ctx, _, _, _ = snapshot
    from dataclasses import replace
    ctx.config = replace(ctx.config, harness_root=str(Path(ctx.repo)/"shared-oracle"))
    ctx.run_id = "same-run"
    ctx.run_dir = str(Path(ctx.repo)/"run")
    Path(ctx.run_dir).mkdir()
    ctx.harness_bin = str(Path(ctx.repo)/"harness.exe")
    Path(ctx.harness_bin).write_bytes(b"frozen harness")
    (Path(ctx.repo)/"deck.json").write_text("{}")
    ctx.run_command = lambda *a: pytest.fail("unexpected command")
    ctx.now = lambda: "2026-10-12T04:00:00Z"
    ctx.items = {"clause:BT1-001#effect#0": ItemRecord("clause:BT1-001#effect#0", "CONFIRMED", {"author_clause": 3}, {"human_question": "retain", "cost_usd": 7})}
    (Path(ctx.repo)/"s.yaml").write_text("card: BT1-001\nclause: BT1-001#effect#0\n")
    return ctx


def create(ctx):
    from tools.card_loop import revalidation as r
    return r.create_queue(ctx, {"clause:BT1-001#effect#0": ["s.yaml"]}, reviewer="independent-root")


def test_generation_reuses_scenario_preserves_history_and_requires_review(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    before = copy.deepcopy(ctx.items)
    create(ctx)
    calls = []
    class Sim:
        def run(self, c, item):
            calls.append("SIM")
            assert item.data == {"scenario_paths": ["s.yaml"]}
            return StageOutcome(next_state="ORACLE", data={"deck_books": {"s.yaml": "deck.json"}})
    class Oracle:
        def run(self, c, item):
            calls.append("ORACLE")
            return StageOutcome(next_state="CONFIRMED", data={"oracle_results": {"s.yaml": {"job_id": "job-new", "verdict": "confirmed"}}})
    monkeypatch.setattr(r, "SimExecutor", Sim)
    monkeypatch.setattr(r, "ReviewOracleExecutor", Oracle)
    monkeypatch.setattr(r, "require_oracle_idle", lambda c: None)
    assert r.advance(ctx)
    assert r.advance(ctx)
    assert not r.advance(ctx)
    assert calls == ["SIM", "ORACLE"]
    assert ctx.items == before
    row = r.read_queue(ctx)["items"]["clause:BT1-001#effect#0"]
    assert row["status"] == "REVIEW"
    assert row["attempts"] == {"SIM": 1, "ORACLE": 1}
    assert r.completion_blocker(ctx)
    with pytest.raises(StageDeferred):
        r.accept(ctx, row["item"], reviewer="root")


def test_unknown_outcome_is_not_retried_on_resume(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    class Crash:
        def run(self, c, item):
            raise RuntimeError("interrupted")
    monkeypatch.setattr(r, "SimExecutor", Crash)
    with pytest.raises(RuntimeError):
        r.advance(ctx)
    assert not r.advance(ctx)
    row = next(iter(r.read_queue(ctx)["items"].values()))
    assert row["status"] == "STARTED"
    assert row["attempts"] == {"SIM": 1}
    with pytest.raises(StageDeferred, match="unknown"):
        r.authorize_retry(ctx, row["item"], reviewer="root")


@pytest.mark.parametrize("state", ["TERMINAL", "UNAVAILABLE", "ESCALATED"])
def test_old_terminal_decisions_and_questions_are_not_automatically_converted(revalidation_ctx, state):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    next(iter(ctx.items.values())).state = state
    create(ctx)
    assert not r.advance(ctx)
    assert next(iter(r.read_queue(ctx)["items"].values()))["status"] == "HOLD"
    assert next(iter(ctx.items.values())).data["human_question"] == "retain"


def test_manifest_and_scenario_drift_refuse_before_execution(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    (Path(ctx.repo)/"s.yaml").write_text("card: BT1-001\nclause: BT1-001#effect#0\n# changed\n")
    with pytest.raises(StageDeferred):
        r.advance(ctx)
    assert not (r.directory(ctx)/"events.jsonl").exists()


def test_lock_blocks_duplicate_dispatch_and_create_is_idempotent(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    before = (r.directory(ctx)/"queue.json").read_bytes()
    create(ctx)
    assert (r.directory(ctx)/"queue.json").read_bytes() == before
    with r.locked(ctx):
        with pytest.raises(StageDeferred):
            r.advance(ctx)


def test_driver_completion_rejects_legacy_terminal_only_state(revalidation_ctx):
    from tools.card_loop.driver import Driver
    ctx = revalidation_ctx
    d = object.__new__(Driver)
    d.ctx = ctx
    d.config = ctx.config
    d.state = SimpleNamespace(records=ctx.items)
    assert d._idle_stop().reason != "complete"


def test_opt_in_default_disabled_requires_manifest():
    from tools.card_loop.config import LoopConfig
    assert LoopConfig().revalidation_enabled is False
    with pytest.raises(ValueError, match="manifest"):
        LoopConfig(revalidation_enabled=True)



def test_runtime_changes_between_stages_are_refused(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    Path(ctx.harness_bin).write_bytes(b"new build")
    with pytest.raises(StageDeferred, match="runtime changed"):
        r.advance(ctx)
    assert not (r.directory(ctx)/"events.jsonl").exists()


def test_oracle_recording_context_never_writes_legacy_verdicts_or_restarts(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    calls = []
    ctx.run_command = lambda *a: calls.append(a)
    wrapped = r._review_context(ctx)
    wrapped.run_command(["harness", "exam", "--oracle", "--verdicts", "--backfill", "--scenario", "s.yaml"], "repo", 900)
    assert calls == [(["harness", "exam", "--oracle", "--scenario", "s.yaml"], "repo", 900)]
    assert "not restarted" in r.ReviewOracleExecutor()._restart_if_still_stalled(ctx,{})


def test_sim_failure_goes_to_review_without_authoring_or_retry(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    class FailedSim:
        def run(self,c,item):
            return StageOutcome(next_state="AUTHORING", reason="invalid scenario")
    monkeypatch.setattr(r,"SimExecutor",FailedSim)
    assert r.advance(ctx)
    assert not r.advance(ctx)
    old = copy.deepcopy(next(iter(ctx.items.values())).attempts)
    r.authorize_retry(ctx,"clause:BT1-001#effect#0",reviewer="independent repair review")
    assert next(iter(ctx.items.values())).attempts == old
    assert next(iter(r.read_queue(ctx)["items"].values()))["attempts"] == {"SIM":1}


def test_corrupt_or_truncated_event_log_cannot_dispatch(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    (r.directory(ctx)/"events.jsonl").write_text('{"incomplete":')
    with pytest.raises(StageDeferred, match="history"):
        r.advance(ctx)


def test_stop_defers_queue_without_starting_attempt(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx = revalidation_ctx
    create(ctx)
    (Path(ctx.run_dir)/"STOP").write_text("keep stopped")
    assert not r.advance(ctx)
    assert not (r.directory(ctx)/"events.jsonl").exists()


def test_nonterminal_record_cannot_be_double_scheduled(revalidation_ctx):
    from tools.card_loop import revalidation as r
    next(iter(revalidation_ctx.items.values())).state="AUTHORING"
    with pytest.raises(StageDeferred,match="ordinary live"):
        create(revalidation_ctx)


def test_full_completion_binding_cannot_shrink_the_manifest(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    from tools.clause_coverage import exam_binding
    monkeypatch.setattr(exam_binding,"bind",lambda *a,**k: {"cards":{}})
    with pytest.raises(StageDeferred,match="full clause/gating"):
        r.current_binding(revalidation_ctx)


def test_missing_or_unmeasured_gating_rows_prevent_completion(revalidation_ctx):
    from tools.card_loop import revalidation as r
    assert "incomplete" in r.completion_blocker(revalidation_ctx)
    assert not r.directory(revalidation_ctx).exists()


def test_enabled_dispatch_is_explicit_and_only_serial(revalidation_ctx):
    from dataclasses import replace
    from tools.card_loop.driver import Driver
    d=object.__new__(Driver)
    d.config=revalidation_ctx.config
    assert d._revalidation_step() is False
    d.config=replace(d.config,revalidation_enabled=True)
    d.serial=False
    with pytest.raises(ValueError,match="serial"):
        d._on_start()



def test_deck_book_change_after_sim_refuses_oracle(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx
    create(ctx)
    class Sim:
        def run(self,c,item):
            return StageOutcome(next_state="ORACLE",data={"deck_books":{"s.yaml":"deck.json"}})
    monkeypatch.setattr(r,"SimExecutor",Sim)
    monkeypatch.setattr(r,"require_oracle_idle",lambda c:None)
    assert r.advance(ctx)
    (Path(ctx.repo)/"deck.json").write_text('{"changed":true}')
    with pytest.raises(StageDeferred,match="deck book"):
        r.advance(ctx)
    assert next(iter(r.read_queue(ctx)["items"].values()))["attempts"] == {"SIM":1}


def test_accept_requires_current_same_job_and_keeps_history(revalidation_ctx, monkeypatch):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx
    create(ctx)
    item="clause:BT1-001#effect#0"
    before=copy.deepcopy(ctx.items)
    with r.locked(ctx):
        r._append(ctx,item,"result",{"status":"REVIEW","outcome":"CONFIRMED","data":{"oracle_results":{"s.yaml":{"job_id":"new-job"}}}})
    current={item:{"verdict":"confirmed","stored_record":{"job_id":"old-job"}}}
    monkeypatch.setattr(r,"current_binding",lambda c:current)
    with pytest.raises(StageDeferred):
        r.accept(ctx,item,reviewer="root")
    current[item]["stored_record"]["job_id"]="new-job"
    r.accept(ctx,item,reviewer="independent-root")
    assert r.read_queue(ctx)["items"][item]["status"] == "DONE"
    assert ctx.items == before
    assert not r.advance(ctx)


def test_interruption_recovery_requires_review_and_never_resets_attempts(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx
    create(ctx)
    item="clause:BT1-001#effect#0"
    with r.locked(ctx):
        r._append(ctx,item,"start",{"status":"STARTED"},stage="ORACLE",attempt_id="historical-attempt")
    monkeypatch.setattr(r,"require_oracle_idle",lambda c:None)
    r.review_interruption(ctx,item,reviewer="root",evidence="reviewed recording has no complete result; queue drained")
    assert not r.advance(ctx)
    r.authorize_retry(ctx,item,reviewer="root explicit retry")
    assert r.read_queue(ctx)["items"][item]["attempts"] == {"ORACLE":1}
    assert len(r.read_queue(ctx)["events"]) == 3


def test_held_question_measurement_needs_explicit_review_and_preserves_question(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx
    rec=next(iter(ctx.items.values()));rec.state="ESCALATED"
    create(ctx)
    with pytest.raises(StageDeferred):
        r.review_hold(ctx,rec.item,reviewer="root",evidence="")
    r.review_hold(ctx,rec.item,reviewer="root",evidence="reviewed fixed scenario; measurement only")
    assert rec.state == "ESCALATED" and rec.data["human_question"] == "retain"
    assert r.read_queue(ctx)["items"][rec.item]["status"] == "SIM"
    assert r.completion_blocker(ctx)


def test_oracle_idle_requires_fresh_heartbeat_and_empty_jobs_and_claims(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;root=Path(ctx.config.harness_root);root.mkdir()
    with pytest.raises(StageDeferred):r.require_oracle_idle(ctx)
    (root/"harness.heartbeat").write_text("idle")
    r.require_oracle_idle(ctx)
    (root/"jobs").mkdir();(root/"jobs/a.json").write_text("{}")
    with pytest.raises(StageDeferred,match="occupied"):r.require_oracle_idle(ctx)


def test_disabled_start_does_not_create_revalidation_files(revalidation_ctx):
    from tools.card_loop.driver import Driver
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;d=object.__new__(Driver);d.config=ctx.config
    assert d._revalidation_step() is False
    assert not r.directory(ctx).exists()



def test_reviewed_preserved_claim_inventory_allows_idle_without_touching_claims(revalidation_ctx):
    import hashlib
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;root=Path(ctx.config.harness_root)
    (root/"claimed").mkdir(parents=True);(root/"harness.heartbeat").write_text("idle")
    claim=root/"claimed/unrelated-historical.json";claim.write_text('{"job_id":"unrelated-historical"}')
    before=claim.read_bytes();ctx.revalidation_preserved_claims={claim.name:hashlib.sha256(before).hexdigest()}
    r.require_oracle_idle(ctx)
    assert claim.read_bytes()==before


@pytest.mark.parametrize("change",["new","changed","missing","active","queued"])
def test_preserved_claim_exception_rejects_unreviewed_changes(revalidation_ctx,change):
    import hashlib
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;root=Path(ctx.config.harness_root)
    (root/"claimed").mkdir(parents=True);(root/"harness.heartbeat").write_text("idle")
    claim=root/"claimed/old.json";claim.write_text("{}")
    ctx.revalidation_preserved_claims={claim.name:hashlib.sha256(claim.read_bytes()).hexdigest()}
    if change=="new":(root/"claimed/new.json").write_text("{}")
    elif change=="changed":claim.write_text('{"changed":true}')
    elif change=="missing":claim.unlink()
    elif change=="active":(root/"harness.heartbeat").write_text("old")
    else:
        (root/"jobs").mkdir();(root/"jobs/new.json").write_text("{}")
    with pytest.raises(StageDeferred):r.require_oracle_idle(ctx)

def test_claim_inventory_is_frozen_with_queue(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;root=Path(ctx.config.harness_root)
    (root/'claimed').mkdir(parents=True)
    claim=root/'claimed/old.json';claim.write_text('{}')
    inventory={claim.name:r._file_hash(claim)}
    selected={'clause:BT1-001#effect#0':['s.yaml']}
    r.create_queue(ctx,selected,reviewer='root inspected unrelated recording',preserved_claims=inventory)
    assert r.read_queue(ctx)['definition']['preserved_claims']==inventory
    r.create_queue(ctx,selected,reviewer='root inspected unrelated recording',preserved_claims=inventory)
    with pytest.raises(StageDeferred,match='immutable'):
        r.create_queue(ctx,selected,reviewer='root inspected unrelated recording')
    with pytest.raises(StageDeferred,match='invalid'):
        r._claim_inventory(ctx,{'../old.json':r._file_hash(claim)})


def test_mid_attempt_runtime_change_holds_unknown_result(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;create(ctx)
    class Sim:
        def run(self,c,item):
            Path(c.harness_bin).write_bytes(b'changed during attempt')
            return StageOutcome(next_state='ORACLE')
    monkeypatch.setattr(r,'SimExecutor',Sim)
    with pytest.raises(StageDeferred,match='during attempt'):
        r.advance(ctx)
    row=next(iter(r.read_queue(ctx)['items'].values()))
    assert row['status']=='STARTED' and row['attempts']=={'SIM':1}

def test_each_oracle_command_rechecks_idle_even_inside_one_stage(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx;create(ctx)
    root=Path(ctx.config.harness_root);root.mkdir();(root/'harness.heartbeat').write_text('idle')
    calls=[]
    ctx.run_command=lambda *a:calls.append(a)
    wrapped=r._review_context(ctx,r.read_queue(ctx))
    wrapped.run_command(['harness','--oracle','--verdicts'],ctx.repo,720)
    (root/'harness.heartbeat').write_text('previous-job-still-running')
    with pytest.raises(StageDeferred,match='idle'):
        wrapped.run_command(['harness','--oracle'],ctx.repo,720)
    assert len(calls)==1


def test_stop_between_oracle_commands_prevents_second_dispatch(revalidation_ctx):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx; create(ctx)
    root=Path(ctx.config.harness_root); root.mkdir(); (root/'harness.heartbeat').write_text('idle')
    calls=[]
    ctx.run_command=lambda *a:calls.append(a)
    wrapped=r._review_context(ctx,r.read_queue(ctx))
    wrapped.run_command(['harness','--oracle'],ctx.repo,720)
    (Path(ctx.run_dir)/'STOP').write_text('drain active only')
    with pytest.raises(StageDeferred):
        wrapped.run_command(['harness','--oracle'],ctx.repo,720)
    assert len(calls)==1

@pytest.mark.parametrize('changed',['scenario','deck'])
def test_input_change_between_oracle_commands_prevents_dispatch(revalidation_ctx,changed):
    from tools.card_loop import revalidation as r
    ctx=revalidation_ctx; create(ctx)
    root=Path(ctx.config.harness_root); root.mkdir(); (root/'harness.heartbeat').write_text('idle')
    item='clause:BT1-001#effect#0'
    with r.locked(ctx):
        r._append(ctx,item,'result',{'status':'ORACLE','deck_book_pins':{'deck.json':r._file_hash(Path(ctx.repo)/'deck.json')},'data':{'deck_books':{'s.yaml':'deck.json'}}})
    calls=[]; ctx.run_command=lambda *a:calls.append(a)
    wrapped=r._review_context(ctx,r.read_queue(ctx))
    argv=['harness','--oracle','--scenario','s.yaml','--decks','deck.json']
    wrapped.run_command(argv,ctx.repo,720)
    target=Path(ctx.repo)/('s.yaml' if changed=='scenario' else 'deck.json')
    target.write_bytes(target.read_bytes()+b'\n# changed')
    with pytest.raises(StageDeferred):
        wrapped.run_command(argv,ctx.repo,720)
    assert len(calls)==1
