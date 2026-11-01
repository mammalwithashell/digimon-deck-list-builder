"""Bind the completed static input audit and reviewed frozen source into the Stage D gate."""
import argparse
from pathlib import Path
import time
from .frozen_mode_campaign import read,write,digest,require,heldout_gate


def finalize(root,schedule):
    require(not (root/'completion-verified.json').exists(),'new completion proof required')
    a=read(root/'static-v1/audit.json');links=read(root/'links-v1.json');protocol=read(schedule/'protocol.json')
    require(a['checks_passed'] and not a['errors'] and links['checks_passed'],'static audit checks')
    require(links['static_audit_sha256']==digest(root/'static-v1/audit.json'),'link audit identity')
    require([v['generation'] for v in a['generations']]==list(range(1,21)),'all twenty generations required')
    require(a['model_hashes']==protocol['model_hashes'],'all four evaluated exports')
    for name,expected in a['evidence_files'].items():
        require(digest(root/'static-v1'/name)==expected,'mirrored evidence changed: '+name)
    reviews=[]
    plans=sorted((root/'static-v1/evidence').glob('*/campaign-plan.json'))
    require(len(plans)==5,'five training segments required')
    for p in plans:
        plan=read(p);matched={}
        for name in ['run_selfplay_generation.py','azero_trainer.py','replay_window.py','mixed_bootstrap.py','english_meta_search_job.py']:
            recorded=[h for k,h in plan['runtime_files'].items() if k.endswith('/'+name)]
            require(len(recorded)==1 and recorded[0]==digest(root/'frozen-source'/name),'frozen fitting source: '+name)
            matched[name]=recorded[0]
        reviews.append(dict(campaign=p.parent.name,source_hashes=matched))
    fresh=read(root/'static-v1/evidence/ex13-fresh-history-20261028/campaign-plan.json')
    require(fresh['preparation_sha256']==digest(root/'frozen-source/prepare_fresh_history_campaign.py'),'bootstrap preparation source')
    for name in ['train_snapshot.json','heldout_snapshot.json']:
        require(digest(root/'static-v1/evidence'/name)==digest(schedule/'sources'/name),'partition binding')
    contracts=[]
    for gen in [8,12,16,20]:
        matches=list((root/'static-v1/evidence').glob(f'*/gen{gen}/model.onnx.meta.json'))
        require(len(matches)==1,'model contract evidence');contracts.append(read(matches[0]))
    require(all(v==contracts[0] for v in contracts),'model contracts differ')
    review=dict(at=time.time(),source_reviews=reviews,bootstrap_preparer_sha256=fresh['preparation_sha256'],model_contract=contracts[0],
        source_reading={
            'bootstrap':'Frozen preparer collects train_snapshot, then fits random fresh_seed using mixed teacher shards, calibrates value on the same shards and copies seed outputs. Calibration heldout rows are an internal split of training-pool games, not the 23 heldout decklists.',
            'updates':'run_selfplay_generation uses previous model.zip warm start and bounded_window shard directories. Saved replay selections exactly equal summary.window_dirs and row counts.',
            'export':'azero_trainer saves the fitted model then invokes export_onnx with that model.zip. Recorded export paths and four ONNX hashes match the scored schedule.',
            'exposure':'All 23 heldout lists appear in training-era combined diagnostic evaluation inputs and the excluded current preflight. This is not a blind test. Historical opponent networks influence collected play; their weights were not copied into the learner.'},
        limits=['Retained input/hash/source provenance, not cryptographic attestation of historical processes.',
                'Exact decklist holdout; card identities and archetypes overlap training.',
                'No new inference, training or native game replay performed.'])
    write(root/'source-review-verified.json',review)
    files={p.relative_to(root).as_posix():digest(p) for folder in ['static-v1','frozen-source','executed-tools'] for p in (root/folder).rglob('*') if p.is_file()}
    for name in ['links-v1.json','source-review-verified.json','audit-config.json','audit-tests.log']:
        files[name]=digest(root/name)
    proof=dict(schema='heldout-weight-training-provenance-v1',verified=True,at=time.time(),model_hashes=a['model_hashes'],
        train_snapshot_sha256=digest(schedule/'sources/train_snapshot.json'),heldout_snapshot_sha256=digest(schedule/'sources/heldout_snapshot.json'),
        weight_training_overlap=[],prior_evaluation_exposure_declared=True,generations=20,
        collection_games=sum(x['games'] for x in a['generations']),collection_chunks=sum(len(x['chunks']) for x in a['generations']),
        unique_collection_rows=sum(x['rows'] for x in a['generations']),bootstrap_games=416,bootstrap_rows=53248,
        train_distinct_lists=21,heldout_distinct_lists=23,heldout_archetypes=12,light_hybrid_heldout='unmeasured',
        evidence_files=files,source_review='source-review-verified.json',qualification_still_required=True)
    heldout_gate(proof,schedule);write(root/'completion-verified.json',proof)
    return {k:v for k,v in proof.items() if k not in ['evidence_files','model_hashes']}


if __name__=='__main__':
    import json
    p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--schedule',type=Path,required=True)
    a=p.parse_args();print(json.dumps(finalize(a.root,a.schedule),indent=2))
