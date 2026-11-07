import json
from pathlib import Path
import pytest
from tools.cloud_search import mulligan_continuation_controller as c


def freeze(root):
    c.write(root/'manifest.json',{'files':{p.relative_to(root).as_posix():c.digest(p)
        for p in root.rglob('*') if p.is_file() and p.name!='manifest.json' and p.parent.name!='results'}})


@pytest.fixture
def inputs(tmp_path):
    """Synthetic input-gate evidence only; never invokes native games."""
    reg=tmp_path/'registration'; reg.mkdir()
    nodes=[dict(id=f'root{i}',mover=i%2,first=bool(i%2)) for i in range(52)]
    jobs=[dict(id=f'cp{cp}-root{i}-repeat{repeat}-action{branch}',checkpoint=cp,root=f'root{i}',
        forced_mulligan_action=branch,outcome_seed=1000+i*2+repeat,follow_on_search_seed=2000+i*2+repeat,
        opponent_checkpoint=8,repeat=repeat)
        for cp in c.CHECKPOINTS for i in range(52) for repeat in (0,1) for branch in (0,1)]
    c.write(reg/'protocol.json',dict(scored_games=832))
    c.write(reg/'selected-roots.json',nodes); c.write(reg/'jobs.json',jobs); freeze(reg)
    roots={}; manifests={}; outputs={}
    for host in ('windows','linux'):
        q=tmp_path/host; roots[host]=q
        for folder in ('inputs','plans','requests','assets','runtime','results'): (q/folder).mkdir(parents=True)
        c.write(q/'inputs/root.json',dict(id='excluded'))
        c.write(q/'inputs/scored-roots.json',nodes)
        c.write(q/'inputs/preregistered-protocol.json',dict(scored_games=832))
        c.write(q/'inputs/build-provenance.json',dict(platform=host))
        (q/'assets/model').write_text('same weights'); (q/'runtime/native').write_text(host)
        ids=[]
        for cp in c.CHECKPOINTS:
            c.write(q/f'plans/cp{cp}.json',dict(runtime_sha256=c.digest(q/'runtime/native'),
                protocol_sha256=c.digest(q/'inputs/preregistered-protocol.json')))
            for branch in (0,1):
                job=f'cp{cp}-action{branch}'; ids.append(job)
                c.write(q/f'requests/{job}.json',dict(stage='excluded_qualification',root='excluded',
                    checkpoint=cp,forced_mulligan_action=branch,root_path='inputs/root.json',
                    root_sha256=c.digest(q/'inputs/root.json'),plan=f'plans/cp{cp}.json',
                    plan_sha256=c.digest(q/f'plans/cp{cp}.json')))
                c.write(q/f'results/{job}.json',dict(synthetic=job))
                c.write(q/f'results/{job}.done.json',dict(synthetic=job))
                outputs.setdefault(job,{})[host]=c.digest(q/f'results/{job}.json')
        c.write(q/'protocol.json',dict(stage='excluded_qualification',scored=False,games=8,jobs=ids,
            workers=3,no_retry=True,binary='runtime/native',platform=host))
        freeze(q); manifests[host]=c.digest(q/'manifest.json')
        c.write(q/'completion-verified.json',dict(games=8,manifest_sha256=manifests[host],
            receipts={j:c.digest(q/f'results/{j}.done.json') for j in ids}))
    parity=tmp_path/'parity.json'
    c.write(parity,dict(passed=True,differences=[],games_compared=8,qualification_manifests=manifests,outputs=outputs))
    return dict(windows=roots['windows'],linux=roots['linux'],parity_path=parity,preregistration=reg)


def test_two_shards_have_exact_832_jobs_with_branches_on_same_host(tmp_path,inputs):
    job_sets=[]
    for host in ('windows','linux'):
        out=tmp_path/f'full-{host}'; result=c.prepare_full(root=out,platform=host,**inputs)
        assert result['games']==416
        protocol=c.validate_inputs(out); requests=[c.read(out/f'requests/{j}.json') for j in protocol['jobs']]
        pairs={}
        for q in requests:pairs.setdefault((q['checkpoint'],q['root'],q['outcome_seed']),set()).add(q['forced_mulligan_action'])
        assert len(pairs)==208 and all(v=={0,1} for v in pairs.values())
        job_sets.append(set(protocol['jobs']))
    assert not job_sets[0]&job_sets[1] and len(job_sets[0]|job_sets[1])==832


def test_failing_parity_or_missing_receipt_cannot_prepare_full(tmp_path,inputs):
    parity=c.read(inputs['parity_path']); parity['passed']=False; c.write(inputs['parity_path'],parity)
    with pytest.raises(ValueError,match='parity'): c.prepare_full(root=tmp_path/'full',platform='windows',**inputs)
    assert not (tmp_path/'full').exists()
    parity['passed']=True;c.write(inputs['parity_path'],parity)
    receipt=inputs['windows']/'results/cp8-action0.done.json'; receipt.write_text('changed')
    with pytest.raises(ValueError,match='receipt changed'): c.prepare_full(root=tmp_path/'full',platform='windows',**inputs)


@pytest.mark.parametrize('tamper',['schedule','runtime'])
def test_rehashing_tampered_shard_cannot_bypass_qualification_or_registration(tmp_path,inputs,tamper):
    out=tmp_path/'full'; c.prepare_full(root=out,platform='windows',**inputs)
    manifest=c.read(out/'manifest.json')
    if tamper=='schedule':
        name='requests/'+c.read(out/'protocol.json')['jobs'][0]+'.json'
        q=c.read(out/name);q['outcome_seed']+=1;c.write(out/name,q)
    else:
        name='runtime/native';(out/name).write_text('different native implementation')
    manifest['files'][name]=c.digest(out/name);c.write(out/'manifest.json',manifest)
    with pytest.raises(ValueError):c.validate_inputs(out)
