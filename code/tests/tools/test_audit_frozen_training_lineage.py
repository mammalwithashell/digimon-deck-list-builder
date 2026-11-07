import pytest
from tools.cloud_search.audit_frozen_training_lineage import content,sha,checked_files,audit


def test_content_uses_counts_not_order_or_labels():
    assert content({'card_ids':['a','b','a']})==content({'cards':['b','a','a'],'name':'other'})
    assert content({'cards':['a','b']})!=content({'cards':['a','a','b']})


def test_files_hashes_and_path_scope(tmp_path):
    p=tmp_path/'x';p.write_text('one');expected=sha(p)
    checked_files(tmp_path,{'x':expected})
    p.write_text('different')
    with pytest.raises(ValueError,match='hash mismatch'):checked_files(tmp_path,{'x':expected})
    with pytest.raises(ValueError,match='path escape'):checked_files(tmp_path,{'../outside':'x'})


def test_missing_evidence_preserves_failed_audit_and_never_admits(tmp_path):
    import json
    out=tmp_path/'audit'
    with pytest.raises(ValueError,match='lineage audit failed'):
        audit({'campaigns':[{'root':str(tmp_path/'missing')}]},out)
    proof=json.loads((out/'audit.json').read_text())
    assert not proof['verified'] and not proof['checks_passed'] and proof['errors']
    with pytest.raises(ValueError,match='new audit output'):audit({},out)
