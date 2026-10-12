"""Archive/body replay proves old public contracts survive private extraction."""
from pathlib import Path
import pytest
from test_runtime_rule_actual_snapshot import snapshot,actual_inputs
from tools.clause_coverage import runtime_projection_proof as p
from tools.clause_coverage import runtime_rule_composition as w

F=Path(__file__).parents[1]/'fixtures/card_data/runtime_empty_condition'


def before(module,name):
    ns={'__name__':module.__name__,'__package__':module.__package__,'__file__':module.__file__}
    # Both bodies see identical current producer files/pins. Only old vs new
    # function logic differs; this is not a fabricated historical source proof.
    exec(compile((F/name).read_bytes(),module.__file__,'exec'),ns)
    return ns


@pytest.mark.parametrize('cid',['BT24-003','BT24-031','BT24-083','BT24-090'])
def test_original_public_proof_and_wrapper_byte_parity(snapshot,cid):
    a=actual_inputs(snapshot,cid)
    args={k:v for k,v in a.items()if k not in ('snapshot','expected_extraction_sha256','scopes','runtime_binding')}
    args['binding']=a['runtime_binding']
    old_proof=before(p,'runtime_projection_before.py.txt')['prove_runtime_projection'](**args)
    assert p.enc(old_proof)==p.enc(p.prove_runtime_projection(**args))
    old_wrapper=before(w,'runtime_rule_before.py.txt')['compose_runtime_rule_card'](**a)
    assert p.enc(old_wrapper)==p.enc(w.compose_runtime_rule_card(**a))
