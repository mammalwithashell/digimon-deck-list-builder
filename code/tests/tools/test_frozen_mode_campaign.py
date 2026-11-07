import copy
import pytest
from tools.cloud_search import frozen_mode_campaign as c
from test_frozen_mode_receipts import fixture


def full_fixture():
    req,out,plan=fixture()
    for game in out['games']:
        for search in game['search_diagnostics']:
            route=search['selected_plan']
            search['opportunity']=dict(schema='search-opportunities-v1',actor=0,sampling='argmax_tau0',simulations_run=320,
                selected_plan=route,executed_actions=route,commitment_executed=True,
                plans=[dict(plan=p,visits=n,q=q if n else None,prior_before_noise=.5,prior_after_noise=.5,
                    selected=p==route,selection_probability=float(p==route)) for p,n,q in search['root_plans']])
    return req,out,plan,dict(expected_first_player=0)


def test_valid_scored_output_requires_opener_and_complete_telemetry():
    assert c.validate_output(*full_fixture())['games']==2


@pytest.mark.parametrize('mutation',[
    lambda o:o['games'][0].update(first_player=1),
    lambda o:o['games'][0]['search_diagnostics'][0].pop('opportunity'),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity'].update(executed_actions=[]),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity'].update(sampling='visit_sample_tau1'),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity']['plans'][0].update(prior_before_noise=None),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity']['plans'][0].update(prior_after_noise=.6),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity']['plans'][0].update(selection_probability=.7),
    lambda o:o['games'][0]['search_diagnostics'][0]['opportunity']['plans'][0].update(q=.3),
])
def test_rejects_wrong_opener_or_untruthful_opportunity_rows(mutation):
    req,out,plan,job=full_fixture();mutation(out)
    with pytest.raises((ValueError,KeyError)):c.validate_output(req,out,plan,job)


@pytest.mark.parametrize('proof',[{},dict(passed=True),dict(schema='frozen-mode-preflight-parity-v1',passed=False,
    differences=[],games_compared=268,requests_compared=134),dict(schema='frozen-mode-preflight-parity-v1',passed=True,
    differences=[],games_compared=8,requests_compared=4)])
def test_incomplete_failed_or_wrong_qualification_blocks_before_copy(tmp_path,proof):
    with pytest.raises(ValueError,match='qualification'):c.qualification_gate(tmp_path,tmp_path,proof)


def test_missing_heldout_training_provenance_blocks(tmp_path):
    c.write(tmp_path/'protocol.json',dict(model_hashes={'8':'abc'}))
    with pytest.raises(ValueError,match='provenance'):c.heldout_gate(None,tmp_path)


def test_mutated_inputs_fail_integrity_check(tmp_path):
    (tmp_path/'input').write_text('original')
    c.write(tmp_path/'manifest.json',dict(files={'input':c.digest(tmp_path/'input')}))
    (tmp_path/'input').write_text('changed')
    with pytest.raises(ValueError,match='changed frozen file'):c.frozen(tmp_path)


@pytest.mark.parametrize('expected',[240,260])
def test_incomplete_receipt_denominator_is_never_completed(tmp_path,monkeypatch,expected):
    (tmp_path/'results').mkdir()
    jobs=[{'id':str(i)} for i in range(expected)]
    monkeypatch.setattr(c,'validate_inputs',lambda r:({},jobs,{}))
    for job in jobs[:-1]:c.write(tmp_path/f"results/{job['id']}.done.json",{})
    with pytest.raises(ValueError,match='incomplete receipt'):c.verify(tmp_path)
    assert not (tmp_path/'completion-verified.json').exists()


def test_native_rejection_preserves_identity_and_cannot_retry(tmp_path,monkeypatch):
    for d in ('attempts','results','requests','logs'):(tmp_path/d).mkdir()
    c.write(tmp_path/'requests/test.json',{})
    calls=[]
    def fail(*a,**kw):calls.append(1);raise RuntimeError('preserved native error')
    monkeypatch.setattr(c.subprocess,'run',fail)
    p=dict(binary='native',deadline_seconds=1);plan=dict(model_sha256='cp20',opponent_model_sha256='cp8')
    job=dict(id='test',game_seed=42,deck_indices=[1,2],checkpoint=20)
    assert c.execute(tmp_path,p,plan,job) is False
    record=c.read(tmp_path/'results/test.rejected.json')
    assert record['request']==job and record['model_sha256']=='cp20'
    with pytest.raises(FileExistsError):c.execute(tmp_path,p,plan,job)
    assert len(calls)==1


def test_fail_stop_drain_no_controller_restart(tmp_path,monkeypatch):
    for d in ('results','attempts'):(tmp_path/d).mkdir()
    monkeypatch.setattr(c,'validate_inputs',lambda r:(dict(binary='native',workers=3),[dict(id=str(i)) for i in range(10)],{}))
    monkeypatch.setattr(c.os,'access',lambda *a:True)
    calls=[]
    def fail(*args):calls.append(args[-1]['id']);return False
    monkeypatch.setattr(c,'execute',fail)
    with pytest.raises(RuntimeError,match='panel failed'):c.run(tmp_path)
    assert 1<=len(calls)<=3
    assert c.read(tmp_path/'state.json')['state']=='failed'
    with pytest.raises(FileExistsError):c.run(tmp_path)
    assert len(calls)<=3


def prepare_fixture(tmp_path,monkeypatch):
    from tools.cloud_search.prepare_frozen_mode_campaign import freeze
    source=tmp_path/'source';source.mkdir();original=tmp_path/'original';(original/'assets').mkdir(parents=True)
    hashes={}
    for cp in (8,12,16,20):
        path=original/f'assets/checkpoint{cp}.onnx';path.write_bytes(str(cp).encode());hashes[f'assets/checkpoint{cp}.onnx']=c.digest(path)
    c.write(original/'campaign-plan.json',dict(files=hashes))
    for partition,n,extra in [('train',13,8),('heldout',12,11)]:
        rows=[dict(archetype=f'arch{i:02d}',card_ids=[partition+str(i)],deck_id=partition+str(i)) for i in range(n)]
        rows.extend(dict(archetype=f'arch{i:02d}',card_ids=[partition+'extra'+str(i)],deck_id=partition+'extra'+str(i)) for i in range(extra))
        c.write(source/f'{partition}.json',dict(decks=rows))
    c.write(source/'prior.json',{'fixture':True})
    schedule=tmp_path/'schedule';freeze(source/'train.json',source/'heldout.json',source/'prior.json',original,schedule)
    qual=tmp_path/'qualified';(qual/'runtime').mkdir(parents=True);(qual/'inputs').mkdir();(qual/'plans').mkdir()
    c.shutil.copytree(original/'assets',qual/'assets');c.shutil.copy2(source/'prior.json',qual/'assets/prior.json')
    c.write(qual/'assets/cards.json',{});(qual/'runtime/native').write_bytes(b'fixture-native')
    c.write(qual/'inputs/build-provenance.json',dict(binary_sha256=c.digest(qual/'runtime/native')))
    c.write(qual/'plans/reference.json',dict(prior_snapshot_hash='snapshot'))
    c.write(qual/'protocol.json',dict(binary='runtime/native'))
    c.write(qual/'manifest.json',dict(files={p.relative_to(qual).as_posix():c.digest(p) for p in qual.rglob('*') if p.is_file()}))
    c.write(source/'parity.json',dict(passed=True,differences=[],games_compared=268,manifests={'linux':c.digest(qual/'manifest.json')}))
    c.write(source/'profile.json',dict(verified=True,qualification_manifest_sha256=c.digest(qual/'manifest.json'),
        observed_peak_worker_rss_bytes=1024,observed_completed_requests=134,decision_cap=2000,request_deadline_seconds=7200))
    monkeypatch.setattr(c,'qualification_gate',lambda *a:True)  # separately tested above; no inference in these fixtures
    return schedule,qual,source


def test_prepare_panel_binds_full_schedule_runtime_and_receipts(tmp_path,monkeypatch):
    schedule,qual,source=prepare_fixture(tmp_path,monkeypatch);root=tmp_path/'panel'
    result=c.prepare(root,schedule,qual,qual,source/'parity.json',source/'profile.json','a-cp12-vs-cp8','linux',3)
    assert result['games']==520 and result['launched'] is False
    p,jobs,plan=c.validate_inputs(root)
    assert len(jobs)==260 and p['games']==520 and plan['model']=='assets/checkpoint12.onnx'
    with pytest.raises(ValueError,match='existing panel'):
        c.prepare(root,schedule,qual,qual,source/'parity.json',source/'profile.json','a-cp12-vs-cp8','linux',3)


def test_prepare_heldout_panel_requires_separate_weight_proof(tmp_path,monkeypatch):
    schedule,qual,source=prepare_fixture(tmp_path,monkeypatch);root=tmp_path/'panel'
    with pytest.raises(ValueError,match='weight-training provenance'):
        c.prepare(root,schedule,qual,qual,source/'parity.json',source/'profile.json','d-cp12-vs-cp8','linux',3)
    assert not root.exists()


def test_unmeasured_memory_blocks_new_allocation(tmp_path,monkeypatch):
    schedule,qual,source=prepare_fixture(tmp_path,monkeypatch);root=tmp_path/'panel'
    profile=c.read(source/'profile.json');profile['observed_peak_worker_rss_bytes']=0;c.write(source/'profile.json',profile)
    with pytest.raises(ValueError,match='timing/memory'):
        c.prepare(root,schedule,qual,qual,source/'parity.json',source/'profile.json','a-cp12-vs-cp8','linux',3)
    assert not root.exists()
