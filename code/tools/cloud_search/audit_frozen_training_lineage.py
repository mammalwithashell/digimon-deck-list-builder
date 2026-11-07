"""Read-only audit of saved training inputs. Never imports a trainer or runs inference."""
import argparse
from collections import Counter
import hashlib
from functools import lru_cache
import json
from pathlib import Path
import time


def read(p):
    return json.loads(p.read_text())


def sha(p):
    p=p.resolve();s=p.stat()
    return _sha(p,s.st_size,s.st_mtime_ns)


@lru_cache(maxsize=None)
def _sha(p,size,mtime):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
    return h.hexdigest()


def require(ok,reason):
    if not ok:raise ValueError(reason)


def content(deck):
    return tuple(sorted(Counter(deck.get('card_ids',deck.get('cards',[]))).items()))


def checked_files(root,files):
    for name,expected in files.items():
        p=(root/name).resolve()
        require(p.is_relative_to(root.resolve()),'input path escape')
        require(sha(p)==expected,'input hash mismatch: '+str(p))


def audit(config,out):
    require(not out.exists(),'new audit output required')
    out.mkdir(parents=True)
    records=[];by_generation={};models={};errors=[]
    def save(source,relative):
        dest=out/'evidence'/relative;dest.parent.mkdir(parents=True,exist_ok=True)
        dest.write_bytes(source.read_bytes())
        return sha(source)
    try:
        first=Path(config['campaigns'][0]['root'])
        train=read(first/'inputs/train_snapshot.json')['decks']
        heldout=read(first/'inputs/heldout_snapshot.json')['decks']
        train_contents={content(d) for d in train};heldout_contents={content(d) for d in heldout}
        require(len(train_contents)==21 and len(heldout_contents)==23,'partition list coverage')
        require(not train_contents & heldout_contents,'training/heldout content overlap')
        train_ids={d['deck_id'] for d in train}
        train_sha=sha(first/'inputs/train_snapshot.json');heldout_sha=sha(first/'inputs/heldout_snapshot.json')
        save(first/'inputs/train_snapshot.json','train_snapshot.json')
        save(first/'inputs/heldout_snapshot.json','heldout_snapshot.json')
        for campaign in config['campaigns']:
            root=Path(campaign['root']);tag=root.name;plan=read(root/'campaign-plan.json')
            save(root/'campaign-plan.json',tag+'/campaign-plan.json')
            require(sha(root/'inputs/train_snapshot.json')==train_sha and sha(root/'inputs/heldout_snapshot.json')==heldout_sha,'partition drift')
            decks=read(root/'inputs/decks.json')
            require({content(d) for d in decks}==train_contents,'collection pool differs from training partition')
            deck_sha=save(root/'inputs/decks.json',tag+'/decks.json')
            for gen in range(campaign['first'],campaign['last']+1):
                g=root/f'generation/gen{gen}';schedule=read(g/'schedule.json');summary=read(g/'summary.json')
                require(schedule['identity']['assets']['decks']==deck_sha,'schedule deck hash mismatch')
                require(schedule['identity']['assets']['model']==models[gen-1]['onnx'] if gen>1 else schedule['identity']['assets']['model']==sha(first/'generation/gen0/model.onnx'),'collector parent export mismatch')
                chunks=[];games=rows=0
                for start in range(0,schedule['games'],schedule['chunk_games']):
                    c=g/f'shards/proc{start:08d}';commit=read(c/'chunk_commit.json')
                    require(commit['game_start']==start and commit['seed']==schedule['seed'] and commit['games']==min(schedule['chunk_games'],schedule['games']-start) and commit['truncated_games']==0,'chunk coverage/identity')
                    checked_files(c,commit['files'])
                    m=read(c/'manifest.json')
                    require(m['deck_names']==[d['name'] for d in decks],'manifest deck pool mismatch')
                    require(m['games']==commit['games'] and m['rows']==commit['rows'] and len(m['game_decks'])==m['games'],'manifest game count')
                    require(all(len(pair)==2 and all(isinstance(i,int) and 0<=i<len(decks) for i in pair) for pair in m['game_decks']),'game deck index')
                    save(c/'chunk_commit.json',f'{tag}/gen{gen}/{c.name}/chunk_commit.json')
                    save(c/'manifest.json',f'{tag}/gen{gen}/{c.name}/manifest.json')
                    chunks.append(dict(name=c.name,commit_sha256=sha(c/'chunk_commit.json'),manifest_sha256=sha(c/'manifest.json')))
                    games+=m['games'];rows+=m['rows']
                require(games==schedule['games'],'generation completeness')
                replay=read(g/'replay_selection.json');selected=[]
                for old in replay['generations']:
                    for item in old['selected']:
                        rel=Path(item['directory']).relative_to('/experiment')
                        c=root/rel
                        require(sha(c/'manifest.json')==item['manifest_sha256'],'replay manifest changed')
                        require(item['exclusions_sha256'] is None,'unexpected replay exclusion requires separate audit')
                        source_gen=int(rel.parts[1][3:])
                        if source_gen==gen:
                            canonical=next(x for x in chunks if x['name']==c.name)
                        else:
                            canonical=next(x for x in by_generation[source_gen]['chunks'] if x['name']==c.name)
                        require(sha(c/'chunk_commit.json')==canonical['commit_sha256'],'inherited replay differs')
                        checked_files(c,read(c/'chunk_commit.json')['files'])
                        selected.append(item['directory'])
                require(selected==summary['window_dirs'],'fitter input directories differ from replay selection')
                require(summary['train']['excluded_rows']==0 and summary['train']['truncated_rows']==0,'unexpected fitting exclusions')
                require(sum(read(root/Path(s).relative_to('/experiment')/'manifest.json')['rows'] for s in selected)==summary['train']['rows'],'fit row count mismatch')
                require(summary['train']['out']==f'/experiment/generation/gen{gen}/model.zip' and summary['train']['export_onnx']==f'/experiment/generation/gen{gen}/model.onnx','fitted export routing')
                for name in ['schedule.json','summary.json','replay_selection.json','model.onnx.meta.json']:
                    save(g/name,f'{tag}/gen{gen}/{name}')
                models[gen]=dict(zip=sha(g/'model.zip'),onnx=sha(g/'model.onnx'))
                record=dict(generation=gen,campaign=tag,games=games,rows=rows,fitted_rows=summary['train']['rows'],models=models[gen],chunks=chunks)
                records.append(record);by_generation[gen]=record
                print(json.dumps(dict(generation=gen,checked_games=games,checked_chunks=len(chunks))),flush=True)
        bootstrap=first/'bootstrap';provenance=read(bootstrap/'provenance.json');manifest=read(bootstrap/'bootstrap_shards/manifest.json')
        require(len(provenance['schedule'])==manifest['games']==416,'bootstrap coverage')
        require(all(set(game['decks'])<=train_ids for game in provenance['schedule']),'bootstrap deck outside training')
        for shard in manifest['shards']:checked_files(bootstrap/'bootstrap_shards'/shard['dir'],shard['sha256'])
        init=read(first/'inputs/initialization.json')
        require(init['inherited_weights'] is False and init['inherited_replay_games']==0,'bootstrap inherited state')
        require(sha(bootstrap/'fresh_seed.zip')==init['random_initial_weights_sha256'],'random initializer identity')
        require(sha(bootstrap/'seed.zip')==sha(first/'generation/gen0/model.zip')==init['seed_sha256'],'bootstrap seed routing')
        for name in ['provenance.json','imitation_summary.json','seed.calibration.json','bootstrap_shards/manifest.json']:
            save(bootstrap/name,'bootstrap/'+name)
        save(first/'inputs/initialization.json','bootstrap/initialization.json')
    except Exception as e:
        errors.append(str(e))
    result=dict(at=time.time(),schema='saved-training-lineage-audit-v1',verified=False,
        checks_passed=not errors,errors=errors,generations=records,model_hashes={str(g):m['onnx'] for g,m in models.items() if g in [8,12,16,20]},
        pending=['Review hash-bound original fitting/export/bootstrap source and prior evaluation exposure before Stage D admission'],
        scope='Static retained artifacts only; no inference, training, game replay, service status or completed-run polling',
        evidence_files={str(p.relative_to(out)):sha(p) for p in (out/'evidence').rglob('*') if p.is_file()})
    (out/'audit.json').write_text(json.dumps(result,indent=2)+'\n')
    require(not errors,'lineage audit failed: '+'; '.join(errors))
    return {k:v for k,v in result.items() if k not in ['generations','evidence_files']}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--config',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    a=p.parse_args();print(json.dumps(audit(read(a.config),a.out)))
