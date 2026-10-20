"""Reviewed execution epochs preserve the original source generation/history."""
import copy
import json
from pathlib import Path
import pytest
from test_card_loop_source_snapshot import snapshot
from test_card_loop_revalidation import revalidation_ctx, create
from tools.card_loop import revalidation as r
from tools.card_loop.stages.base import StageDeferred

ITEM='clause:BT1-001#effect#0'

def stopped(ctx):
    (Path(ctx.run_dir)/'STOP').write_text('review transition while drained')
    root=Path(ctx.config.harness_root);root.mkdir(exist_ok=True)
    (root/'harness.heartbeat').write_text('idle')


def proposal(ctx):
    from tools.card_loop import revalidation_epochs as e
    return e.prepare_transition(ctx,{ITEM:['s.yaml']},reviewer='independent-root',evidence='reviewed new binary and repaired scenario')


def test_new_epoch_preserves_flat_bytes_history_and_remeasures_done(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    old_items=copy.deepcopy(ctx.items)
    with r.locked(ctx):
        r._append(ctx,ITEM,'start',{'status':'STARTED'},stage='SIM',attempt_id='old-sim')
        r._append(ctx,ITEM,'result',{'status':'DONE','data':{'oracle_results':{'s.yaml':{'job_id':'historical-job'}}}})
    flat={p.name:p.read_bytes() for p in [r.directory(ctx)/'queue.json',r.directory(ctx)/'events.jsonl']}
    Path(ctx.harness_bin).write_bytes(b'reviewed updated engine')
    p=proposal(ctx);e.activate_transition(ctx,p)
    q=r.read_queue(ctx)
    assert q['definition']['digest']==p['digest']
    assert q['items'][ITEM]['status']=='SIM'
    assert q['cumulative_attempts'][ITEM]=={'SIM':1}
    assert q['items'][ITEM]['previous_attempts']=={'author_clause':3}
    assert ctx.items==old_items and q['items'][ITEM]['previous_data']['human_question']=='retain'
    assert {name:(r.directory(ctx)/name).read_bytes() for name in flat}==flat
    assert r.completion_blocker(ctx)


@pytest.mark.parametrize('violation',['stop','busy','started','tip','runtime','scenario','drop'])
def test_activation_refuses_changed_or_unreviewed_preconditions(revalidation_ctx,violation):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);p=proposal(ctx)
    if violation=='stop':(Path(ctx.run_dir)/'STOP').unlink()
    elif violation=='busy':(Path(ctx.config.harness_root)/'harness.heartbeat').write_text('active-job')
    elif violation in {'started','tip'}:
        with r.locked(ctx):r._append(ctx,ITEM,'start' if violation=='started' else 'review',{'status':'STARTED' if violation=='started' else 'REVIEW'},stage='SIM',attempt_id='other')
    elif violation=='runtime':Path(ctx.harness_bin).write_bytes(b'unreviewed engine')
    elif violation=='scenario':(Path(ctx.repo)/'s.yaml').write_text('changed')
    else:
        p['entries']={};p['digest']=r._digest({k:v for k,v in p.items() if k!='digest'})
    with pytest.raises(StageDeferred):e.activate_transition(ctx,p)
    assert not (r.directory(ctx)/'transitions.jsonl').exists()


def test_orphan_definition_cannot_activate_and_retry_is_idempotent(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);p=proposal(ctx)
    old=r.read_queue(ctx)['definition']['digest'];publish=e._publish_activation
    monkeypatch.setattr(e,'_publish_activation',lambda *a:(_ for _ in ()).throw(RuntimeError('crash before activation')))
    with pytest.raises(RuntimeError):e.activate_transition(ctx,p)
    assert (r.directory(ctx)/'epochs'/p['digest']/'queue.json').exists()
    assert r.read_queue(ctx)['definition']['digest']==old
    monkeypatch.setattr(e,'_publish_activation',publish)
    e.activate_transition(ctx,p)
    before=(r.directory(ctx)/'transitions.jsonl').read_bytes()
    e.activate_transition(ctx,p)
    assert (r.directory(ctx)/'transitions.jsonl').read_bytes()==before


def test_epoch_dispatch_uses_common_lock_and_new_event_file(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation_epochs as e
    from tools.card_loop.driver_contracts import StageOutcome
    ctx=revalidation_ctx;create(ctx);stopped(ctx);p=proposal(ctx)
    with r.locked(ctx):
        with pytest.raises(StageDeferred):e.activate_transition(ctx,p)
    e.activate_transition(ctx,p);(Path(ctx.run_dir)/'STOP').unlink()
    class Sim:
        def run(self,c,item):return StageOutcome('ORACLE')
    monkeypatch.setattr(r,'SimExecutor',Sim)
    assert r.advance(ctx)
    assert not (r.directory(ctx)/'events.jsonl').exists()
    assert (r.directory(ctx)/'epochs'/p['digest']/'events.jsonl').exists()
    assert r.read_queue(ctx)['cumulative_attempts'][ITEM]=={'SIM':1}


def test_transition_preserves_held_question_and_rejects_started_even_with_fresh_tip(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;ctx.items[ITEM].state='ESCALATED';create(ctx);stopped(ctx)
    e.activate_transition(ctx,proposal(ctx))
    assert r.read_queue(ctx)['items'][ITEM]['status']=='HOLD'
    assert ctx.items[ITEM].state=='ESCALATED' and r.completion_blocker(ctx)
    with r.locked(ctx):r._append(ctx,ITEM,'start',{'status':'STARTED'},stage='ORACLE',attempt_id='unknown')
    p=proposal(ctx)
    with pytest.raises(StageDeferred,match='STARTED'):e.activate_transition(ctx,p)


def test_corrupt_activation_chain_and_modified_ancestor_fail_closed(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);e.activate_transition(ctx,proposal(ctx))
    with (r.directory(ctx)/'events.jsonl').open('a') as out:out.write('{"truncated":')
    with pytest.raises(StageDeferred):r.read_queue(ctx)

def test_two_transitions_keep_cumulative_attempts_and_ancestor_tips(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    with r.locked(ctx):r._append(ctx,ITEM,'start',{'status':'REVIEW'},stage='SIM',attempt_id='epoch0')
    first=proposal(ctx);e.activate_transition(ctx,first)
    with r.locked(ctx):r._append(ctx,ITEM,'start',{'status':'REVIEW'},stage='SIM',attempt_id='epoch1')
    second=proposal(ctx);e.activate_transition(ctx,second)
    q=r.read_queue(ctx)
    assert len(q['lineage'])==3 and q['cumulative_attempts'][ITEM]=={'SIM':2}
    assert q['items'][ITEM]['status']=='SIM'
    assert len(q['transitions'])==2 and ctx.items[ITEM].data['cost_usd']==7
    with pytest.raises(StageDeferred):e.activate_transition(ctx,first)


def test_new_epoch_deck_candidate_addition_or_change_is_unreviewed(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation_epochs as e
    from tools.card_loop.stages import harness
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    monkeypatch.setattr(harness,'deck_books_for',lambda *a:['deck.json'])
    p=proposal(ctx);e.activate_transition(ctx,p)
    (Path(ctx.repo)/'deck.json').write_text('{"changed":true}')
    (Path(ctx.run_dir)/'STOP').unlink()
    with pytest.raises(StageDeferred,match='deck inputs'):r.advance(ctx)
    assert not (r.directory(ctx)/'epochs'/p['digest']/'events.jsonl').exists()


def test_activation_journal_corruption_refuses_completion(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);e.activate_transition(ctx,proposal(ctx))
    with (r.directory(ctx)/'transitions.jsonl').open('a') as out:out.write('{"partial":')
    with pytest.raises(StageDeferred,match='activation'):r.read_queue(ctx)
    assert 'refused' in r.completion_blocker(ctx)


def test_same_source_epoch_keeps_complete_clause_and_gating_membership(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    before=set(r.current_binding(ctx));e.activate_transition(ctx,proposal(ctx))
    assert set(r.current_binding(ctx))==before
    assert r.read_queue(ctx)['definition']['generation']==ctx.config.source_manifest_sha256
    p=proposal(ctx);p['generation']='changed-source';p['digest']=r._digest({k:v for k,v in p.items() if k!='digest'})
    with pytest.raises(StageDeferred,match='source generation'):e.activate_transition(ctx,p)


def test_transition_cannot_rewrite_historical_question_or_attempts(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);p=proposal(ctx)
    p['entries'][ITEM]['previous_data']={};p['digest']=r._digest({k:v for k,v in p.items() if k!='digest'})
    with pytest.raises(StageDeferred,match='historical'):e.activate_transition(ctx,p)


def test_prepare_is_read_only_and_missing_queue_does_not_create_one(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx
    with pytest.raises(StageDeferred):proposal(ctx)
    assert not r.directory(ctx).exists()
    create(ctx);before={str(p):p.read_bytes() for p in r.directory(ctx).rglob('*') if p.is_file()}
    proposal(ctx)
    assert {str(p):p.read_bytes() for p in r.directory(ctx).rglob('*') if p.is_file()}==before

@pytest.mark.parametrize('malformed',['version','entry_status','deck_shape'])
def test_malformed_sealed_proposal_cannot_publish_journal(revalidation_ctx,malformed):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx);p=proposal(ctx)
    if malformed=='version':p['version']=9
    elif malformed=='entry_status':p['entries'][ITEM]['status']='DONE'
    else:p['reviewed_deck_candidates']=[]
    p['digest']=r._digest({k:v for k,v in p.items() if k!='digest'})
    with pytest.raises(StageDeferred):e.activate_transition(ctx,p)
    assert not (r.directory(ctx)/'transitions.jsonl').exists()


def test_repaired_scenario_epoch_runs_sim_then_one_oracle_without_reauthoring(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation_epochs as e
    from tools.card_loop.driver_contracts import StageOutcome
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    flat=(r.directory(ctx)/'queue.json').read_bytes()
    with (Path(ctx.repo)/'s.yaml').open('a') as out:out.write('# reviewed repair\n')
    e.activate_transition(ctx,proposal(ctx));(Path(ctx.run_dir)/'STOP').unlink()
    calls=[]
    class Sim:
        def run(self,c,item):
            calls.append('SIM')
            return StageOutcome('ORACLE',data={'deck_books':{'s.yaml':'deck.json'}})
    class Oracle:
        def run(self,c,item):
            calls.append('ORACLE')
            return StageOutcome('CONFIRMED',data={'oracle_results':{'s.yaml':{'job_id':'new-epoch-job','verdict':'confirmed'}}})
    monkeypatch.setattr(r,'SimExecutor',Sim)
    monkeypatch.setattr(r,'ReviewOracleExecutor',Oracle)
    assert r.advance(ctx) and r.advance(ctx) and not r.advance(ctx)
    q=r.read_queue(ctx)
    assert calls==['SIM','ORACLE'] and q['items'][ITEM]['status']=='REVIEW'
    assert q['cumulative_attempts'][ITEM]=={'SIM':1,'ORACLE':1}
    assert (r.directory(ctx)/'queue.json').read_bytes()==flat
    assert ctx.items[ITEM].attempts=={'author_clause':3} and ctx.items[ITEM].data['cost_usd']==7
    assert r.completion_blocker(ctx)


def test_ancestor_escalation_blocks_successor_accept_until_exact_review(revalidation_ctx,monkeypatch):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    with r.locked(ctx):r._append(ctx,ITEM,'result',{'status':'REVIEW','outcome':'ESCALATED','data':{'human_question':'new unresolved question'},'reason':'needs decision'})
    ancestor=r.read_queue(ctx);event=ancestor['events'][-1]
    old_bytes=(r.directory(ctx)/'events.jsonl').read_bytes()
    e.activate_transition(ctx,proposal(ctx))
    with r.locked(ctx):r._append(ctx,ITEM,'result',{'status':'REVIEW','outcome':'CONFIRMED','data':{'oracle_results':{'s.yaml':{'job_id':'successor'}}}})
    monkeypatch.setattr(r,'current_binding',lambda c:{ITEM:{'verdict':'confirmed','stored_record':{'job_id':'successor'}}})
    r.accept(ctx,ITEM,reviewer='root')
    assert r.completion_blocker(ctx), 'a new accepted measurement must not close an ancestor question'
    q=r.read_queue(ctx);question=q['unresolved_escalations'][0]
    assert question['epoch']==ancestor['definition']['digest'] and question['event']==event['digest']
    assert question['data']['human_question']=='new unresolved question'
    with pytest.raises(StageDeferred):e.resolve_escalation(ctx,question,expected_tip='stale',reviewer='root',evidence='decision')
    with pytest.raises(StageDeferred):e.resolve_escalation(ctx,question,expected_tip=q['tip'],reviewer='root',evidence='')
    e.resolve_escalation(ctx,question,expected_tip=q['tip'],reviewer='root',evidence='explicit decision recorded')
    assert r.completion_blocker(ctx) is None
    assert (r.directory(ctx)/'events.jsonl').read_bytes()==old_bytes
    assert ctx.items[ITEM].state=='CONFIRMED'


def test_stage_escalation_full_payload_is_preserved(revalidation_ctx,monkeypatch):
    from tools.card_loop.driver_contracts import StageOutcome,Escalation
    ctx=revalidation_ctx;create(ctx)
    class Sim:
        def run(self,c,item):
            return StageOutcome('ESCALATED',reason='question',escalation=Escalation(ITEM,'precise question',({'citation':'official','reasoning':'alternatives'},),('prior-attempt',)))
    monkeypatch.setattr(r,'SimExecutor',Sim)
    assert r.advance(ctx)
    q=r.read_queue(ctx)
    assert q['events'][-1]['update']['escalation']['reason']=='precise question'
    assert q['unresolved_escalations'][0]['escalation']['arguments']==[{'citation':'official','reasoning':'alternatives'}]


def test_same_epoch_retry_cannot_erase_question_and_resolution_survives_transition(revalidation_ctx):
    from tools.card_loop import revalidation_epochs as e
    ctx=revalidation_ctx;create(ctx);stopped(ctx)
    with r.locked(ctx):r._append(ctx,ITEM,'result',{'status':'REVIEW','outcome':'ESCALATED','reason':'retain me'})
    r.authorize_retry(ctx,ITEM,reviewer='measurement reviewer')
    q=r.read_queue(ctx);question=q['unresolved_escalations'][0]
    altered=dict(question,reason='different decision')
    with pytest.raises(StageDeferred):e.resolve_escalation(ctx,altered,expected_tip=q['tip'],reviewer='root',evidence='decision')
    e.resolve_escalation(ctx,question,expected_tip=q['tip'],reviewer='root',evidence='explicit resolution')
    assert not r.read_queue(ctx)['unresolved_escalations']
    e.activate_transition(ctx,proposal(ctx))
    assert not r.read_queue(ctx)['unresolved_escalations']
    with pytest.raises(StageDeferred):e.resolve_escalation(ctx,question,expected_tip=r.read_queue(ctx)['tip'],reviewer='root',evidence='duplicate')
