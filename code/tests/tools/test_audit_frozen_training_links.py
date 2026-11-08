import json
import pytest
from tools.cloud_search.audit_frozen_training_links import run
from tools.cloud_search.audit_frozen_training_lineage import sha


def fixture(root):
    def write(name,value):
        p=root/name;p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(json.dumps(value) if isinstance(value,(dict,list)) else value)
    write('audit/audit.json',dict(checks_passed=True,generations=[dict(generation=1,models={'zip':'z','onnx':'o'})]))
    write('campaign/inputs/heldout_snapshot.json',{'decks':[{'cards':['h']}]})
    for prefix in ['campaign/generation/gen0/model','campaign/inputs/seed','campaign/bootstrap/seed']:
        write(prefix+'.zip','zip');write(prefix+'.onnx','onnx')
    write('campaign/inputs/eval.json',{'decks':[{'cards':['h']}]})
    write('campaign/campaign-plan.json',{'evaluation_panels':[['evaluation','eval.json']]})
    return {'campaigns':[dict(root=str(root/'campaign'),first=1,last=1)]}


def test_links_exposure_but_not_admission(tmp_path):
    config=fixture(tmp_path)
    result=run(config,tmp_path/'audit',tmp_path/'links.json')
    assert result['checks_passed'] and not result['verified']
    assert result['campaigns'][0]['evaluation_inputs'][0]['heldout_exact_list_overlap']==1


def test_bootstrap_copy_mismatch_rejected(tmp_path):
    config=fixture(tmp_path);(tmp_path/'campaign/inputs/seed.zip').write_text('changed')
    with pytest.raises(ValueError,match='bootstrap ZIP'):run(config,tmp_path/'audit',tmp_path/'links.json')
