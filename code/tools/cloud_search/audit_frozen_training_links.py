"""Supplement a static shard audit with saved warm-start and evaluation input links."""
import argparse
import json
from pathlib import Path
try:
    from .audit_frozen_training_lineage import read,sha,require,content
except ImportError:
    from audit_frozen_training_lineage import read,sha,require,content


def run(config,audit_root,out):
    require(not out.exists(),'new links output required')
    a=read(audit_root/'audit.json');require(a['checks_passed'],'static shard audit failed')
    models={r['generation']:r['models'] for r in a['generations']}
    first=Path(config['campaigns'][0]['root'])
    heldout={content(d) for d in read(first/'inputs/heldout_snapshot.json')['decks']}
    seed_zip=sha(first/'generation/gen0/model.zip');seed_onnx=sha(first/'generation/gen0/model.onnx')
    require(seed_onnx==sha(first/'inputs/seed.onnx')==sha(first/'bootstrap/seed.onnx'),'bootstrap ONNX copies')
    require(seed_zip==sha(first/'inputs/seed.zip'),'bootstrap ZIP copies')
    rows=[];previous=None
    for item in config['campaigns']:
        root=Path(item['root']);plan=read(root/'campaign-plan.json')
        links=[]
        for gen in range(item['first'],item['last']+1):
            expected=models[gen-1] if gen>1 else dict(zip=seed_zip,onnx=seed_onnx)
            for kind in ['zip','onnx']:
                require(sha(root/f'generation/gen{gen-1}/model.{kind}')==expected[kind],f'warm-start {kind} copy gen{gen}')
            links.append(dict(generation=gen,parent=expected))
        if previous is not None:
            require(plan['source_plan_sha256']==sha(previous/'campaign-plan.json'),'parent plan chain')
            require(plan['parent_checkpoint_sha256']==models[item['first']-1]['zip'],'parent checkpoint chain')
        # Only named evaluation inputs are inspected; no result or live state polling.
        exposures=[]
        for label,name in plan['evaluation_panels']:
            p=root/'inputs'/name
            d=read(p)
            exposures.append(dict(label=label,input=name,sha256=sha(p),
                heldout_exact_list_overlap=len({content(v) for v in d['decks']} & heldout)))
        rows.append(dict(campaign=root.name,warm_starts=links,evaluation_inputs=exposures))
        previous=root
    result=dict(checks_passed=True,verified=False,static_audit_sha256=sha(audit_root/'audit.json'),
        campaigns=rows,bootstrap_seed_zip_sha256=seed_zip,bootstrap_seed_onnx_sha256=seed_onnx,
        prior_evaluation_exposure_declared=True,
        exposure_note='Named training-era evaluation inputs overlap some heldout lists; all 23 heldout lists also appear in the separately excluded current model preflight. This is not a blind test. No evaluated outcomes are used by this audit to select panels or models.',
        pending='Local frozen source review and complete evidence hash binding before Stage D admission')
    out.write_text(json.dumps(result,indent=2)+'\n');return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--config',type=Path,required=True);p.add_argument('--audit',type=Path,required=True);p.add_argument('--out',type=Path,required=True)
    a=p.parse_args();result=run(read(a.config),a.audit,a.out)
    print(json.dumps(dict(checks_passed=result['checks_passed'],campaigns=len(result['campaigns']),verified=False)))
