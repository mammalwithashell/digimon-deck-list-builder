import json
from pathlib import Path
import pytest
from test_card_loop_source_snapshot import snapshot
from test_card_loop_revalidation import revalidation_ctx
from tools.card_loop.driver_contracts import ItemRecord
from tools.card_loop.stages.oracle import OracleExecutor
from tools.card_loop.stages.base import StageDeferred
from tools.card_loop.stages.harness import exam_input_pin

@pytest.mark.parametrize("change", ["stop", "scenario", "deck", "source", "none"])
def test_standard_oracle_refuses_second_job_after_change(revalidation_ctx, change):
    ctx = revalidation_ctx
    root = Path(ctx.repo)
    (root/"s2.yaml").write_bytes((root/"s.yaml").read_bytes())
    calls=[]
    def fake(argv,cwd,timeout):
        calls.append(argv)
        if len(calls)==1:
            if change=="stop": (Path(ctx.run_dir)/"STOP").write_text("drain active only")
            elif change != "none":
                target=root/({"scenario":"s2.yaml","deck":"deck.json","source":"data/cards.json"}[change])
                target.write_bytes(target.read_bytes()+b"\n")
        return 0,json.dumps({"verdict":"confirmed","job_id":"fake-only"}),""
    ctx.run_command=fake
    item=ItemRecord("clause:BT1-001#effect#0","ORACLE",data={"scenario_paths":["s.yaml","s2.yaml"],"deck_books":{"s.yaml":"deck.json","s2.yaml":"deck.json"}})
    item.data["oracle_input_pins"]={p:exam_input_pin(ctx,p,"deck.json") for p in item.data["scenario_paths"]}
    try: OracleExecutor().run(ctx,item)
    except StageDeferred: pass
    assert len(calls)==(2 if change == "none" else 1), f"unsafe second dispatch after {change} change"

@pytest.mark.parametrize('change',['stop','scenario'])
def test_deferred_results_and_pins_survive_resume(revalidation_ctx,change):
    ctx=revalidation_ctx;root=Path(ctx.repo)
    (root/'s2.yaml').write_bytes((root/'s.yaml').read_bytes())
    calls=[]
    def fake(argv,cwd,timeout):
        calls.append(argv)
        if len(calls)==1:
            if change=='stop':(Path(ctx.run_dir)/'STOP').write_text('drain')
            else:(root/'s2.yaml').write_bytes((root/'s2.yaml').read_bytes()+b'\n')
            # Normal --backfill may update the completed scenario. It must
            # neither invalidate nor replay that already-finished job.
            (root/'s.yaml').write_bytes((root/'s.yaml').read_bytes()+b'\n# backfilled')
        return 0,json.dumps({'verdict':'confirmed','job_id':'fake'+str(len(calls))}),''
    ctx.run_command=fake
    item=ItemRecord('clause:BT1-001#effect#0','ORACLE',data={'scenario_paths':['s.yaml','s2.yaml'],'deck_books':{'s.yaml':'deck.json','s2.yaml':'deck.json'}})
    item.data["oracle_input_pins"]={p:exam_input_pin(ctx,p,"deck.json") for p in item.data["scenario_paths"]}
    with pytest.raises(StageDeferred) as stopped:OracleExecutor().run(ctx,item)
    outcome=stopped.value.outcome
    assert outcome.data['oracle_results']['s.yaml']['job_id']=='fake1'
    assert outcome.data['oracle_retry_paths']==['s2.yaml']
    resumed=ItemRecord(item.item,'ORACLE',data={**item.data,**outcome.data})
    if change=='stop':
        (Path(ctx.run_dir)/'STOP').unlink()
        result=OracleExecutor().run(ctx,resumed)
        assert result.next_state=='CONFIRMED' and len(calls)==2
    else:
        with pytest.raises(StageDeferred,match='changed'):OracleExecutor().run(ctx,resumed)
        assert len(calls)==1

@pytest.mark.parametrize('change',['scenario','deck'])
def test_changed_sim_input_before_first_oracle_is_refused(revalidation_ctx,monkeypatch,change):
    from tools.card_loop.stages.sim import SimExecutor
    from tools.card_loop.stages import harness
    ctx=revalidation_ctx;root=Path(ctx.repo)
    monkeypatch.setattr(harness,'deck_books_for',lambda *a:['deck.json'])
    calls=[]
    def fake(argv,cwd,timeout):
        calls.append(argv)
        if '--sim-only' in argv:return 0,'exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0\n',''
        return 0,json.dumps({'verdict':'confirmed','job_id':'should-not-dispatch'}),''
    ctx.run_command=fake
    item=ItemRecord('clause:BT1-001#effect#0','SIM',data={'scenario_paths':['s.yaml']})
    simulated=SimExecutor().run(ctx,item)
    assert simulated.next_state=='ORACLE'
    target=root/('s.yaml' if change=='scenario' else 'deck.json')
    target.write_bytes(target.read_bytes()+b'\n')
    with pytest.raises(StageDeferred):
        OracleExecutor().run(ctx,ItemRecord(item.item,'ORACLE',data={**item.data,**simulated.data}))
    assert len(calls)==1


@pytest.mark.parametrize('change',['scenario','deck'])
def test_input_mutation_during_sim_cannot_create_valid_pin(revalidation_ctx,monkeypatch,change):
    from tools.card_loop.stages.sim import SimExecutor
    from tools.card_loop.stages import harness
    ctx=revalidation_ctx;root=Path(ctx.repo)
    monkeypatch.setattr(harness,'deck_books_for',lambda *a:['deck.json'])
    def fake(argv,cwd,timeout):
        target=root/('s.yaml' if change=='scenario' else 'deck.json')
        target.write_bytes(target.read_bytes()+b'\n')
        return 0,'exam: scenarios seen 1 / lowered 1 / run 1 / diffed 0 / failed 0\n',''
    ctx.run_command=fake
    with pytest.raises(StageDeferred):
        SimExecutor().run(ctx,ItemRecord('clause:BT1-001#effect#0','SIM',data={'scenario_paths':['s.yaml']}))


def test_legacy_oracle_without_sim_pins_returns_sim_without_command(revalidation_ctx):
    from tools.card_loop.driver_contracts import check_transition
    ctx=revalidation_ctx;calls=[]
    ctx.run_command=lambda *a:(calls.append(a) or (0,json.dumps({'verdict':'confirmed'}),''))
    item=ItemRecord('clause:BT1-001#effect#0','ORACLE',data={'scenario_paths':['s.yaml'],'deck_books':{'s.yaml':'deck.json'},'oracle_results':{'old.yaml':{'job_id':'historical'}}})
    result=OracleExecutor().run(ctx,item)
    assert result.next_state=='SIM' and not calls
    check_transition(item.kind,item.state,result.next_state)
    assert item.data['oracle_results']['old.yaml']['job_id']=='historical'

def test_driver_resims_legacy_without_author_or_billed_call(revalidation_ctx):
    from test_card_loop_driver import Harness,seed
    from tools.card_loop.driver_contracts import StageOutcome
    ctx=revalidation_ctx
    class StopAfterSim:
        state='SIM'
        def run(self,c,item):
            (Path(c.run_dir)/'STOP').write_text('test stops after observing SIM route')
            return StageOutcome('ORACLE',reason='fresh SIM would establish byte pins')
    h=Harness(Path(ctx.repo)/'driver-legacy',[seed('clause:BT1-001#effect#0','ORACLE',data={'scenario_paths':['s.yaml'],'deck_books':{'s.yaml':'deck.json'}})],[OracleExecutor(),StopAfterSim()])
    h.driver.ctx.run_command=lambda *a:pytest.fail('missing SIM pins must not dispatch')
    h.run()
    assert ('ORACLE','SIM') in [(e['src'],e['dst']) for e in h.events()]
    assert h.attempts()==[]
