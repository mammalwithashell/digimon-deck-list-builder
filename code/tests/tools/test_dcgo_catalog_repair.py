import copy
import importlib.util
from pathlib import Path
import pytest

MODULE = Path(__file__).parents[2] / 'tools/dcgo_catalog_repair/generate.py'
spec = importlib.util.spec_from_file_location('dcgo_catalog_repair', MODULE)
repair = importlib.util.module_from_spec(spec)
spec.loader.exec_module(repair)


def card():
    return dict(card_id='EX13-018', name='Coredramon', rarity='C', card_type='Digimon',
                level=4, colors=['Blue', 'Green'], play_cost=5, dp=6000,
                form='Champion', attribute='Vaccine', type='Dragon', layout='single',
                evolution_routes=[dict(colors=['Blue', 'Green'], from_level=3, cost=2)],
                rule_grants=dict(traits=[], attributes=[]), text_sections=[
                    dict(label='Effect', text='[On Play] Draw 1.'),
                    dict(label='Inherited Effect', text='[Your Turn] +1000 DP.')],
                printing_field_gaps=[], special_evolution_routes=[],
                grade_evolution_routes=[], tamer_evolution_routes=[])


def test_maps_all_evolution_colors_and_exact_class():
    row = repair.map_card(card())
    assert row['CardEffectClassName'] == 'EX13_018'
    assert row['cardColors'] == [1, 3]
    assert row['EvoCosts'] == [dict(CardColor=1, Level=3, MemoryCost=2), dict(CardColor=3, Level=3, MemoryCost=2)]
    assert row['InheritedEffectDiscription_ENG'] == '[Your Turn] +1000 DP.'


def test_egg_no_play_cost_is_negative_one():
    raw = card(); raw.update(card_type='Digi-Egg', play_cost=None, level=2, dp=None, evolution_routes=[])
    assert repair.map_card(raw)['PlayCost'] == -1


@pytest.mark.parametrize('change', [dict(layout='dual'), dict(colors=['Orange']), dict(printing_field_gaps=['level']), dict(name='')])
def test_incomplete_or_unsupported_authority_rejected(change):
    raw = card(); raw.update(change)
    with pytest.raises(ValueError): repair.map_card(raw)


def test_indices_preserve_existing_and_append_without_collision():
    existing = {'BT1-001': 2, 'BT1-002': 90}
    assert repair.allocate_indices(existing, ['BT1-001', 'EX13-005', 'EX13-008']) == {'BT1-001': 2, 'EX13-005': 91, 'EX13-008': 92}
    assert existing == {'BT1-001': 2, 'BT1-002': 90}


def test_duplicate_existing_indices_refused():
    with pytest.raises(ValueError): repair.allocate_indices({'a': 1, 'b': 1}, ['EX13-005'])


def test_payload_requires_exact_pool_and_hash_bound_evidence(tmp_path):
    with pytest.raises(ValueError): repair.load_evidence(tmp_path)


def test_checked_in_evidence_roundtrips_and_matches_image_metadata():
    rows = repair.load_evidence(MODULE.parent/'evidence')
    assert [r['CardID'] for r in rows] == list(repair.IDS)
    by_id = {r['CardID']: r for r in rows}
    assert by_id['EX13-018']['DP'] == 5000
    assert by_id['EX13-018']['cardColors'] == [1,0]
    assert by_id['EX13-018']['EvoCosts'] == [dict(CardColor=1,Level=3,MemoryCost=3),dict(CardColor=0,Level=3,MemoryCost=3)]
    assert by_id['EX13-045']['Type_ENG'] == ['Holy Warrior','Royal Knight']
    assert by_id['EX13-045']['Level'] == 7
    assert by_id['EX13-005']['PlayCost'] == -1
    assert 'Assembly -5:' in by_id['EX13-044']['EffectDiscription_ENG']


def test_evidence_tampering_is_rejected(tmp_path):
    import shutil
    shutil.copytree(MODULE.parent/'evidence', tmp_path/'evidence')
    (tmp_path/'evidence'/'EX13-005.json').write_text('{}')
    with pytest.raises(ValueError, match='hash mismatch'): repair.load_evidence(tmp_path/'evidence')


def test_default_cli_does_not_write_and_explicit_write_refuses_overwrite(tmp_path, capsys):
    output = tmp_path/'payload.json'
    assert repair.main([]) == 0
    assert 'EX13_045' in capsys.readouterr().out
    assert not output.exists()
    assert repair.main(['--write',str(output)]) == 0
    original = output.read_bytes()
    with pytest.raises(FileExistsError): repair.main(['--write',str(output)])
    assert output.read_bytes() == original
