"""Qualification-gated, single-attempt scored panels on the frozen A-D schedule.

No resume or retry path. Queue orchestration can start successive new panels,
but a failed panel must remain terminal and cannot acquire a completed score.
"""
import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

try:
    from . import frozen_mode_receipts as receipts
    from .prepare_frozen_mode_campaign import validate_blocks
except ImportError:
    import frozen_mode_receipts as receipts
    from schedule_validation import validate_blocks

require=receipts.require


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def digest(path):
    import hashlib
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path,value):
    path=Path(path);tmp=path.with_suffix(path.suffix+'.tmp')
    tmp.write_text(json.dumps(value,indent=2,allow_nan=False),encoding='utf-8');tmp.replace(path)


def frozen(root):
    manifest=read(root/'manifest.json')
    for name,expected in manifest['files'].items():
        path=(root/name).resolve()
        require(path.is_relative_to(root.resolve()) and digest(path)==expected,'changed frozen file: '+name)
    return manifest


def qualification_gate(windows,linux,parity):
    require(parity.get('schema')=='frozen-mode-preflight-parity-v1' and parity.get('passed') is True
            and parity.get('differences')==[] and parity.get('games_compared')==268
            and parity.get('requests_compared')==134,'exact complete qualification required')
    require(len(parity.get('outputs',{}))==134,'parity coverage')
    for host,root in (('windows',windows),('linux',linux)):
        frozen(root)
        require(digest(root/'manifest.json')==parity['manifests'][host],'parity manifest binding')
        complete=read(root/'completion-verified.json')
        require(complete['manifest_sha256']==parity['manifests'][host] and complete['games']==268
                and complete['requests']==134 and complete['heldout_fallback_worlds']>0,'complete fallback qualification')
        require(not list((root/'results').glob('*.rejected.json')),'rejected qualification')
        require(len(complete['receipt_hashes'])==134,'receipt coverage')
        for job,hashes in parity['outputs'].items():
            require(digest(root/f'results/{job}.json')==hashes[host],'qualification output changed')
            require(digest(root/f'results/{job}.done.json')==complete['receipt_hashes'][job],'qualification receipt changed')
    return True


def heldout_gate(proof,schedule):
    protocol=read(schedule/'protocol.json')
    require(proof is not None and proof.get('verified') is True,'held-out weight-training provenance required')
    require(proof.get('model_hashes')==protocol['model_hashes'],'held-out model lineage coverage')
    require(proof.get('train_snapshot_sha256')==digest(schedule/'sources/train_snapshot.json')
            and proof.get('heldout_snapshot_sha256')==digest(schedule/'sources/heldout_snapshot.json'),'held-out partition binding')
    require(proof.get('weight_training_overlap')==[] and proof.get('prior_evaluation_exposure_declared') is True,
            'held-out overlap/exposure evidence')
    require(bool(proof.get('evidence_files')),'held-out evidence files missing')


def prepare(root,schedule,windows,linux,parity_path,profile_path,panel_id,platform,workers,heldout_proof=None):
    require(not root.exists(),'preserve existing panel')
    require(platform in ('windows','linux') and 1<=workers<=(3 if platform=='windows' else 4),'worker allocation')
    frozen(schedule);parity=read(parity_path);qualification_gate(windows,linux,parity)
    panels={p['id']:p for p in read(schedule/'panels.json')}
    require(panel_id in panels,'unknown panel');panel=panels[panel_id]
    if panel['stage']=='D':
        heldout_gate(read(heldout_proof) if heldout_proof else None,schedule)
        for path,expected in read(heldout_proof)['evidence_files'].items():
            require(digest(heldout_proof.parent/path)==expected,'held-out evidence changed')
    qualified=windows if platform=='windows' else linux
    profile=read(profile_path)
    require(profile.get('verified') is True and profile.get('qualification_manifest_sha256')==digest(qualified/'manifest.json'),
            'measured qualification runtime profile required')
    require(profile.get('observed_peak_worker_rss_bytes',0)>0 and profile.get('observed_completed_requests')==134,
            'complete timing/memory profile required')
    require(profile.get('decision_cap')==2000 and profile.get('request_deadline_seconds')==7200,'qualified execution limits')
    jobs=[j for j in read(schedule/'jobs.json') if j['panel']==panel_id]
    decks=read(schedule/'decks.json')[panel['partition']]
    validate_blocks(decks,jobs,12 if panel['stage']=='D' else 13)
    require(2*len(jobs)==panel['games'],'panel budget')
    root.mkdir(parents=True)
    for name in ('inputs','requests','results','attempts','logs'):(root/name).mkdir()
    shutil.copytree(qualified/'assets',root/'assets');shutil.copytree(qualified/'runtime',root/'runtime')
    shutil.copy2(__file__,root/'controller.py');shutil.copy2(Path(receipts.__file__),root/'frozen_mode_receipts.py')
    from inspect import getsourcefile
    shutil.copy2(getsourcefile(validate_blocks),root/'schedule_validation.py')
    for source,target in ((parity_path,'parity.json'),(profile_path,'profile.json'),
                          (qualified/'manifest.json','qualification-manifest.json'),
                          (qualified/'inputs/build-provenance.json','build-provenance.json'),
                          (schedule/'manifest.json','schedule-manifest.json'),(schedule/'protocol.json','scored-protocol.json')):
        shutil.copy2(source,root/'inputs'/target)
    if heldout_proof:shutil.copy2(heldout_proof,root/'inputs/heldout-provenance.json')
    write(root/'inputs/jobs.json',jobs);write(root/'inputs/panel.json',panel)
    build=read(qualified/'inputs/build-provenance.json');cp=panel['checkpoint'];opp=panel['opponent_checkpoint']
    reference=read(next((qualified/'plans').glob('*.json')))
    plan=dict(schema=receipts.SCHEMA,native_sha256=build['binary_sha256'],model=f'assets/checkpoint{cp}.onnx',
        opponent_model=f'assets/checkpoint{opp}.onnx',cards_json='assets/cards.json',prior='assets/prior.json',
        prior_snapshot_hash=reference['prior_snapshot_hash'],decks=decks,decision_cap=2000)
    for key,field in (('model','model_sha256'),('opponent_model','opponent_model_sha256'),('cards_json','cards_sha256'),('prior','prior_file_sha256')):
        plan[field]=digest(root/plan[key])
    for cp_key,hash_key in (('checkpoint','model_sha256'),('opponent_checkpoint','opponent_model_sha256')):
        require(plan[hash_key]==read(schedule/'protocol.json')['model_hashes'][str(panel[cp_key])],'scheduled model hash')
    require(plan['prior_file_sha256']==read(schedule/'protocol.json')['prior_file_sha256'],'scheduled prior')
    write(root/'plan.json',plan)
    for job in jobs:
        actor=lambda mode:dict(kind='search',information_regime=mode,simulations=320,worlds=4,batch=4,root_noise=0)
        request=dict(schema=receipts.SCHEMA,id=job['id'],plan='plan.json',runtime_sha256=build['binary_sha256'],
            prior_snapshot_hash=plan['prior_snapshot_hash'],deck_indices=job['deck_indices'],game_seed=job['game_seed'],
            search_seed=job['search_seed'],candidate=actor(panel['candidate_mode']),opponent=actor(panel['opponent_mode']))
        write(root/f"requests/{job['id']}.json",request)
    write(root/'protocol.json',dict(schema='frozen-mode-scored-panel-v1',platform=platform,panel=panel_id,stage=panel['stage'],
        games=panel['games'],requests=len(jobs),workers=workers,scored=True,no_retry=True,deadline_seconds=7200,
        binary=read(qualified/'protocol.json')['binary'],schedule_manifest_sha256=digest(schedule/'manifest.json'),
        qualification_manifest_sha256=digest(qualified/'manifest.json')))
    write(root/'manifest.json',dict(files={p.relative_to(root).as_posix():digest(p) for p in sorted(root.rglob('*')) if p.is_file()}))
    return dict(root=str(root),games=panel['games'],state='prepared_after_qualification',launched=False)


def validate_inputs(root):
    manifest=frozen(root);protocol=read(root/'protocol.json');panel=read(root/'inputs/panel.json');jobs=read(root/'inputs/jobs.json');plan=read(root/'plan.json')
    require(protocol['schema']=='frozen-mode-scored-panel-v1' and protocol['scored'] is True,'scored panel schema')
    expected=480 if panel['stage']=='D' else 520
    require(protocol['games']==panel['games']==expected and protocol['requests']==len(jobs)==expected//2,'panel coverage')
    require(protocol['no_retry'] is True and protocol['deadline_seconds']==7200 and plan['decision_cap']==2000,'execution limits')
    require(1<=protocol['workers']<=(3 if protocol['platform']=='windows' else 4),'workers')
    parity=read(root/'inputs/parity.json');qual=read(root/'inputs/qualification-manifest.json')['files']
    require(parity['passed'] is True and parity['differences']==[] and parity['games_compared']==268,'qualification gate')
    require(digest(root/'inputs/qualification-manifest.json')==protocol['qualification_manifest_sha256']==parity['manifests'][protocol['platform']],
            'qualification identity')
    require(all(manifest['files'].get(k)==h for k,h in qual.items() if k.startswith(('assets/','runtime/'))),'qualified runtime/assets changed')
    require(digest(root/'inputs/schedule-manifest.json')==protocol['schedule_manifest_sha256'],'schedule identity')
    if panel['stage']=='D':
        proof=read(root/'inputs/heldout-provenance.json')
        require(proof['verified'] is True and proof['weight_training_overlap']==[],'held-out provenance')
    validate_blocks(plan['decks'],jobs,12 if panel['stage']=='D' else 13)
    for job in jobs:
        req=read(root/f"requests/{job['id']}.json")
        require(job['panel']==protocol['panel'] and req['id']==job['id'],'panel request identity')
        require(all(req[k]==job[k] for k in ('deck_indices','game_seed','search_seed')),'scheduled request')
        for role in ('candidate','opponent'):
            require(req[role]['information_regime']==job[role+'_mode']==panel[role+'_mode'],'scheduled information regime')
    return protocol,jobs,plan


def validate_output(request,output,plan,job):
    counts=receipts.validate(request,output,plan)
    for game in output['games']:
        require(game['first_player']==job['expected_first_player'],'scheduled opener')
        for search in game['search_diagnostics']:
            op=search['opportunity'];route=search['selected_plan'];plans=search['root_plans']
            require(op['schema']=='search-opportunities-v1' and op['actor']==search['diagnostics']['viewer']
                    and op['sampling']=='argmax_tau0' and op['simulations_run']==320,'opportunity context')
            require(op['selected_plan']==op['executed_actions']==route and op['commitment_executed'] is True,'opportunity actual execution')
            require(len(op['plans'])==len(plans),'opportunity coverage')
            for row,native in zip(op['plans'],plans):
                require(row['plan']==native[0] and row['visits']==native[1] and row['q']==(native[2] if native[1] else None),'opportunity plans')
                require(receipts.finite_nonnegative(row['prior_before_noise']) and row['prior_before_noise']==row['prior_after_noise'],'raw/effective priors')
                require(row['selected']==(row['plan']==route) and row['selection_probability']==float(row['selected']),'opportunity selection')
    return counts


def execute(root,protocol,plan,job):
    name=job['id'];reqpath=root/f'requests/{name}.json';outpath=root/f'results/{name}.json'
    with (root/f'attempts/{name}.json').open('x') as stream:json.dump(dict(at=time.time(),request_sha256=digest(reqpath)),stream)
    try:
        with (root/f'logs/{name}.out.log').open('xb') as out,(root/f'logs/{name}.err.log').open('xb') as err:
            subprocess.run([str((root/protocol['binary']).resolve()),str(reqpath.resolve()),str(outpath.resolve())],cwd=root,
                stdout=out,stderr=err,check=True,timeout=protocol['deadline_seconds'])
        counts=validate_output(read(reqpath),read(outpath),plan,job)
        write(root/f'results/{name}.done.json',dict(schema=receipts.SCHEMA,request_sha256=digest(reqpath),output_sha256=digest(outpath),
            runtime_sha256=plan['native_sha256'],prior_snapshot_hash=plan['prior_snapshot_hash'],counts=counts,committed_at=time.time()))
        return True
    except Exception as error:
        write(root/f'results/{name}.rejected.json',dict(at=time.time(),error=str(error),request=job,request_sha256=digest(reqpath),
            model_sha256=plan['model_sha256'],opponent_model_sha256=plan['opponent_model_sha256']))
        return False


def run(root):
    protocol,jobs,plan=validate_inputs(root)
    require(os.access(root/protocol['binary'],os.X_OK),'native executable permission')
    require(not any((root/'results').iterdir()) and not any((root/'attempts').iterdir()),'existing attempts/results')
    with (root/'started.json').open('x') as stream:json.dump(dict(at=time.time(),pid=os.getpid()),stream)
    cursor=committed=0;failed=False
    with ThreadPoolExecutor(max_workers=protocol['workers']) as pool:
        active=set()
        while active or (cursor<len(jobs) and not failed):
            while not failed and len(active)<protocol['workers'] and cursor<len(jobs):
                active.add(pool.submit(execute,root,protocol,plan,jobs[cursor]));cursor+=1
            done,active=wait(active,return_when=FIRST_COMPLETED)
            for future in done:
                try:success=future.result()
                except Exception:success=False
                failed |= not success;committed+=int(success)
            write(root/'state.json',dict(state='draining_failed' if failed else 'running',committed_games=2*committed,at=time.time()))
    if failed:
        write(root/'state.json',dict(state='failed',committed_games=2*committed,at=time.time()))
        raise RuntimeError('panel failed; no completed score or retries')
    verify(root);write(root/'state.json',dict(state='complete_verified',committed_games=protocol['games'],at=time.time()))


def verify(root):
    protocol,jobs,plan=validate_inputs(root)
    require(not list((root/'results').glob('*.rejected.json')),'rejected native evaluation')
    require({p.name for p in (root/'results').glob('*.done.json')}=={j['id']+'.done.json' for j in jobs},'incomplete receipt coverage')
    hashes={}
    for job in jobs:
        name=job['id'];req=read(root/f'requests/{name}.json');out=read(root/f'results/{name}.json')
        receipts.validate_receipt(root,f'requests/{name}.json',f'results/{name}.json',f'results/{name}.done.json',plan)
        validate_output(req,out,plan,job);hashes[name]=digest(root/f'results/{name}.done.json')
    proof=dict(at=time.time(),panel=protocol['panel'],games=protocol['games'],receipts=len(hashes),receipt_hashes=hashes,
        manifest_sha256=digest(root/'manifest.json'),scored=True,complete=True)
    write(root/'completion-verified.json',proof)
    return {k:v for k,v in proof.items() if k!='receipt_hashes'}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('command',choices=('prepare','run','verify'))
    parser.add_argument('--root',type=Path,required=True)
    for name in ('schedule','windows','linux','parity-path','profile-path','heldout-proof'):parser.add_argument('--'+name,type=Path)
    parser.add_argument('--panel-id');parser.add_argument('--platform',choices=('windows','linux'));parser.add_argument('--workers',type=int)
    a=parser.parse_args()
    if a.command=='prepare':result=prepare(a.root,a.schedule,a.windows,a.linux,a.parity_path,a.profile_path,a.panel_id,a.platform,a.workers,a.heldout_proof)
    elif a.command=='run':result=run(a.root.resolve())
    else:result=verify(a.root.resolve())
    print(json.dumps(result))
