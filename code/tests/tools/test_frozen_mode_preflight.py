import copy
import json
from pathlib import Path
import pytest
from tools.cloud_search import frozen_mode_preflight as c
from tools.cloud_search.compare_frozen_mode_preflight import comparable


def test_schedule_covers_all_models_lists_seats_and_modes():
    jobs=c.schedule()
    assert len(jobs)==len({j['id'] for j in jobs})==134
    assert len({j['game_seed'] for j in jobs})==134
    for partition,count,modes in [('train',21,['known-list','closed-list']),('heldout',23,['closed-list'])]:
        coverage=set()
        for j in jobs:
            if j['partition']!=partition or j['checkpoint']==j['opponent_checkpoint']: continue
            for candidate_seat in (0,1):
                for seat,index in enumerate(j['deck_indices']):
                    cp=j['checkpoint'] if seat==candidate_seat else j['opponent_checkpoint']
                    mode=j['candidate_mode'] if seat==candidate_seat else j['opponent_mode']
                    coverage.add((cp,index,seat,mode))
        assert coverage=={(cp,i,seat,mode) for cp in c.CHECKPOINTS for i in range(count) for seat in (0,1) for mode in modes}
    assert {j['checkpoint'] for j in jobs if j['candidate_mode']!=j['opponent_mode']}==set(c.CHECKPOINTS)


def fixture(root):
    for folder in ('attempts','results','requests','logs','plans'):(root/folder).mkdir()
    job=c.schedule()[0]
    c.write(root/f"requests/{job['id']}.json",dict(id=job['id'],plan='plans/test.json'))
    return job


def test_crashed_attempt_is_preserved_and_never_retried(tmp_path,monkeypatch):
    job=fixture(tmp_path); calls=[]
    def fail(*a,**kw): calls.append(1); raise RuntimeError('native failure')
    monkeypatch.setattr(c.subprocess,'run',fail)
    protocol=dict(binary='absent',deadline_seconds=1)
    assert c.execute(tmp_path,protocol,job) is False
    rejection=c.read(tmp_path/f"results/{job['id']}.rejected.json")
    assert rejection['game_seed']==job['game_seed'] and rejection['checkpoint']==8
    with pytest.raises(FileExistsError):c.execute(tmp_path,protocol,job)
    assert len(calls)==1


def test_controller_stops_new_launches_drains_and_cannot_restart(tmp_path,monkeypatch):
    (tmp_path/'attempts').mkdir();(tmp_path/'results').mkdir()
    jobs=c.schedule();called=[]
    monkeypatch.setattr(c,'validate_inputs',lambda r:({'workers':3,'binary':'fake'},jobs))
    monkeypatch.setattr(c.os,'access',lambda *a:True)
    def execute(root,p,j): called.append(j['id']);return False
    monkeypatch.setattr(c,'execute',execute)
    with pytest.raises(RuntimeError,match='preflight failed'):c.run(tmp_path)
    assert 1<=len(called)<=3
    assert c.read(tmp_path/'state.json')['state']=='failed'
    with pytest.raises(FileExistsError):c.run(tmp_path)
    assert len(called)<=3


def test_missing_execute_permission_fails_before_attempt(tmp_path,monkeypatch):
    monkeypatch.setattr(c,'validate_inputs',lambda r:({'binary':'fake'},c.schedule()))
    monkeypatch.setattr(c.os,'access',lambda *a:False)
    with pytest.raises(ValueError,match='permission'):c.run(tmp_path)
    assert not (tmp_path/'started.json').exists()


def test_archive_preserves_executable_and_empty_queue_dirs(tmp_path,monkeypatch):
    root=tmp_path/'root';root.mkdir()
    for name in ('results','attempts','logs','runtime'):(root/name).mkdir()
    (root/'runtime/native').write_bytes(b'fake')
    monkeypatch.setattr(c,'validate_inputs',lambda r:({'binary':'runtime/native'},[]))
    target=tmp_path/'deployment.tar.gz'
    c.archive(root,target)
    with c.tarfile.open(target) as bundle:
        assert bundle.getmember('runtime/native').mode==0o755
        assert bundle.getmember('results').isdir()


def test_incomplete_or_rejected_cannot_verify(tmp_path,monkeypatch):
    (tmp_path/'results').mkdir()
    monkeypatch.setattr(c,'validate_inputs',lambda r:({},c.schedule()))
    with pytest.raises(ValueError,match='incomplete'):c.verify(tmp_path)
    c.write(tmp_path/'results/bad.rejected.json',{})
    with pytest.raises(ValueError,match='rejected'):c.verify(tmp_path)


def output():
    return dict(runtime_sha256='win',request=dict(runtime_sha256='win',search_seed=7),games=[dict(seconds=1,
        actions=[2],card_metrics=dict(steps=[dict(selection_seconds=1,action=2)]),search_diagnostics=[dict(
        selected_plan=[2],opportunity=dict(raw_prior=.25),diagnostics=dict(seconds=1,information=dict(
        regime='closed-list',sampling_seconds=1,fallback_mass=.1)))])])


def test_parity_ignores_only_declared_clocks_and_platform_binary():
    a=output();b=copy.deepcopy(a);b['runtime_sha256']='linux';b['request']['runtime_sha256']='linux'
    b['games'][0]['seconds']=2
    b['games'][0]['search_diagnostics'][0]['diagnostics']['information']['sampling_seconds']=2
    assert comparable(a)==comparable(b)
    assert a['runtime_sha256']=='win'


@pytest.mark.parametrize('field',['prior','action','seed','fallback'])
def test_parity_keeps_numeric_and_semantic_differences(field):
    a=output();b=copy.deepcopy(a)
    if field=='prior':b['games'][0]['search_diagnostics'][0]['opportunity']['raw_prior']+=2**-25
    elif field=='action':b['games'][0]['actions']=[3]
    elif field=='seed':b['request']['search_seed']=8
    else:b['games'][0]['search_diagnostics'][0]['diagnostics']['information']['fallback_mass']=.2
    assert comparable(a)!=comparable(b)
