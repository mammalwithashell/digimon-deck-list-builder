import json
from pathlib import Path

from tools.cloud_search.audit_main_activations import audit

ROOT = Path(__file__).resolve().parents[3]


def test_detects_nested_main_refund_but_not_unrelated_trigger_refund(tmp_path):
    folder=tmp_path/'test'; folder.mkdir()
    (folder/'TEST-1.yaml').write_text('''card: TEST-1
name: Test
effects:
  - when: on_play
    process: [{refund_opt: {}}]
  - when: main_on_field
    once_per_turn: true
    process:
      - if:
          condition: {binding_absent: picked}
          then: [{refund_opt: {}}]
''')
    result=audit(['TEST-1','TEST-2'],tmp_path)
    assert result['main_clause_count']==1
    assert result['refund_clause_count']==1
    assert result['missing_sources']==['TEST-2']


def test_frozen_training_and_evaluation_pool_has_no_authored_main_refunds():
    report=json.loads((ROOT/'docs/superpowers/research/2026-10-26-main-activation-sweep.json').read_text())
    fresh=audit(report['card_ids'],ROOT/'code/digimon-engine/cards')
    assert fresh['missing_sources']==[]
    assert fresh['refund_clause_count']==0
    assert fresh['main_clause_count']==report['main_clause_count']


def test_includes_option_faces_of_dual_cards(tmp_path):
    folder=tmp_path/'test'; folder.mkdir()
    (folder/'TEST-1.yaml').write_text('''card: TEST-1
name: Dual
faces:
  option:
    effects:
      - when: main
        process: [{draw: {count: 1}}]
''')
    result=audit(['TEST-1'],tmp_path)
    assert result['main_clause_count']==1
    assert result['clauses'][0]['location']=='$.faces.option.effects[0]'
