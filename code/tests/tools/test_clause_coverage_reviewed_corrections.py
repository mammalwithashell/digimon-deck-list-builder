"""Reviewed printed-zone corrections must drive clause and interaction denominators."""
import json
from pathlib import Path

from tools.clause_coverage.card_sources import load_official_index, extract_card_clauses
from tools.clause_coverage.extract import run
from tools.card_loop.interactions.denominator import extract_clauses


def fixture(tmp_path):
    effect = "While you have a Digimon in play, you may use this card without meeting its color requirements."
    main = "[Main] Place this card under 1 of your Digimon."
    security = "[Security] Gain 1 memory, and add this card to its owner's hand."
    inherited = "[All Turns] Effects can't trash [X Antibody] in this Digimon's digivolution cards.\n[When Attacking] This Digimon can digivolve."
    official = tmp_path / 'card_official.json'
    official.write_text(json.dumps({'cards': {'BT9-109': {'colors': ['White'], 'text_sections': [
        {'label': 'Effect', 'text': effect+'\n'+security+'\n'+main},
        {'label': 'Inherited Effect', 'text': inherited},
        {'label': 'Security Effect', 'text': inherited},
    ]}}}), encoding='utf-8')
    corrections = {'schema_version': 1, 'corrections': [
        {'card_id':'BT9-109','field_path':field,'value':value,'reason':'printed image review','reviewer':'reviewer','reviewed_at':'2026-07-13','evidence_observation_ids':['observation'], 'correction_id':field}
        for field,value in [('effect_description_eng',effect+'\n'+main),('security_effect_description_eng',security)]
    ]}
    empty = tmp_path/'empty.json'; empty.write_text('{}')
    return official, corrections, empty


def test_reviewed_sections_reduce_seven_to_five_preserving_option_inherited(tmp_path):
    official, corrections, _ = fixture(tmp_path)
    def clauses():
        return extract_card_clauses('BT9-109', cards_index={}, overrides_index={}, official_index=load_official_index(official), dcgo_root=None)
    assert len(clauses()) == 7
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    actual = clauses()
    assert [c.id for c in actual] == ['BT9-109#effect#0','BT9-109#effect#1','BT9-109#inherited#0','BT9-109#inherited#1','BT9-109#security#0']
    assert actual[-1].text == "Gain 1 memory, and add this card to its owner's hand."
    assert len(json.loads(official.read_text())['cards']['BT9-109']['text_sections']) == 3


def test_clause_and_interaction_extract_use_identical_reviewed_text(tmp_path):
    official, corrections, empty = fixture(tmp_path)
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    coverage = run(['BT9-109'],'fixture',cards_json_path=empty,overrides_path=empty,official_json_path=official)['clauses']
    interactions = extract_clauses(['BT9-109'],cards_json=empty,overrides=empty,official=official)
    assert len(coverage) == 5
    assert coverage == interactions


def test_reviewed_empty_scrape_and_missing_section_are_not_lost(tmp_path):
    official, corrections, _ = fixture(tmp_path)
    official.write_text(json.dumps({'cards': {'LM-057': {'text_sections': []}}}))
    corrections['corrections']=[dict(corrections['corrections'][0],card_id='LM-057',value='[Main] Draw 1.'),dict(corrections['corrections'][1],card_id='LM-057')]
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    actual=extract_card_clauses('LM-057',cards_index={},overrides_index={},official_index=load_official_index(official),dcgo_root=None)
    assert [(c.zone,c.text) for c in actual]==[('effect','Draw 1.'),('security',"Gain 1 memory, and add this card to its owner's hand.")]


def test_explicit_empty_security_correction_suppresses_image_fallback(tmp_path):
    official, corrections, _ = fixture(tmp_path)
    official.write_text(json.dumps({'cards': {'BT9-109': {'text_sections': []}}}))
    corrections['corrections'] = [dict(corrections['corrections'][1], value='')]
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    actual = extract_card_clauses('BT9-109',cards_index={'BT9-109': {'card_kind':2,'effect_description_eng':'[Main] Draw 1.'}},overrides_index={},official_index=load_official_index(official),image_cache={'BT9-109':{'security':'[Security] Draw 2.'}},dcgo_root=None)
    assert [(c.zone,c.text) for c in actual] == [('effect','Draw 1.')]


def test_base_fallback_only_for_this_checkout_standard_dataset(tmp_path, monkeypatch):
    from tools.clause_coverage import card_sources
    base = tmp_path/'base'; (base/'data').mkdir(parents=True)
    ledger = base/'data/card_corrections.json'; ledger.write_text('{}')
    monkeypatch.setattr(card_sources, '_base_repo_root', lambda: base)
    monkeypatch.setattr(card_sources, '__file__', str(tmp_path/'project/code/tools/clause_coverage/card_sources.py'))
    standard = Path(card_sources.__file__).resolve().parents[3]/'data/card_official.json'
    assert card_sources._reviewed_corrections_path(standard) == ledger
    assert card_sources._reviewed_corrections_path(tmp_path/'unrelated/card_official.json') is None
    (tmp_path/'card_corrections.json').write_text('{}')
    assert card_sources._reviewed_corrections_path(tmp_path/'card_official.json') == tmp_path/'card_corrections.json'


def test_invalid_or_duplicate_reviewed_text_fails_closed(tmp_path):
    import pytest
    official, corrections, _ = fixture(tmp_path)
    corrections['corrections'][0]['evidence_observation_ids'] = []
    ledger = tmp_path/'card_corrections.json'
    ledger.write_text(json.dumps(corrections))
    with pytest.raises(ValueError, match='invalid reviewed'):
        load_official_index(official)
    corrections['corrections'][0]['evidence_observation_ids'] = ['observation']
    corrections['corrections'].append(corrections['corrections'][0])
    ledger.write_text(json.dumps(corrections))
    with pytest.raises(ValueError, match='duplicate reviewed'):
        load_official_index(official)


def test_security_alias_and_canonical_duplicate_replaced_as_one_zone(tmp_path):
    official, corrections, _ = fixture(tmp_path)
    raw = json.loads(official.read_text())
    sections = raw['cards']['BT9-109']['text_sections']
    sections[2] = {'label': 'Security', 'text': '[Security] Draw 1.'}
    sections.append({'label': 'Security Effect', 'text': '[Security] Draw 2.'})
    official.write_text(json.dumps(raw))
    corrections['corrections'] = [dict(corrections['corrections'][1], value='[Security] Gain 2 memory.')]
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    actual = extract_card_clauses('BT9-109',cards_index={},overrides_index={},official_index=load_official_index(official),dcgo_root=None)
    assert [(c.id,c.text) for c in actual if c.zone=='security'] == [('BT9-109#security#0','Gain 2 memory.')]
    assert len([c for c in actual if c.zone=='inherited']) == 2


def test_empty_security_correction_removes_legacy_alias(tmp_path):
    official, corrections, _ = fixture(tmp_path)
    raw = json.loads(official.read_text())
    raw['cards']['BT9-109']['text_sections'][2]['label'] = ' Security '
    official.write_text(json.dumps(raw))
    corrections['corrections'] = [dict(corrections['corrections'][1], value='')]
    (tmp_path/'card_corrections.json').write_text(json.dumps(corrections))
    actual = extract_card_clauses('BT9-109',cards_index={},overrides_index={},official_index=load_official_index(official),dcgo_root=None)
    assert not [c for c in actual if c.zone=='security']
    assert len([c for c in actual if c.zone=='inherited']) == 2


def test_committed_reviewed_ledger_has_portable_observation_closure():
    """Every imported reviewed correction remains auditable without a base checkout."""
    root = Path(__file__).resolve().parents[3]
    ledger = json.loads((root / "data/card_corrections.json").read_text(encoding="utf-8"))
    paths = list((root / "data/card_sources/observations").glob("*/*.json"))
    by_id = {path.stem: path for path in paths}
    assert len(by_id) == len(paths), "observation IDs must not be ambiguous across providers"
    for correction in ledger["corrections"]:
        for observation_id in correction["evidence_observation_ids"]:
            assert observation_id in by_id, (correction["correction_id"], observation_id)
            observation = json.loads(by_id[observation_id].read_text(encoding="utf-8"))
            assert observation["observation_id"] == observation_id
            assert observation["source"] == by_id[observation_id].parent.name


def test_examon_denominators_reproduce_without_base_checkout(monkeypatch):
    """No developer-local ledger may silently alter committed campaign identities."""
    from tools.clause_coverage import card_sources
    from tools.card_loop.interactions.denominator import (
        build_denominator, load_card_qa, load_promotion, DEFAULT_PROMOTION,
    )
    root = Path(__file__).resolve().parents[3]
    canonical = json.loads((root / "qa/archetype-qa/dsl/examon-2026-10-10-clauses.json").read_text(encoding="utf-8"))
    monkeypatch.setattr(card_sources, "_base_repo_root", lambda: None)
    monkeypatch.setenv("DIGIMON_DCGO_ROOT", "")
    official = root / "data/card_official.json"
    assert card_sources._reviewed_corrections_path(official) == root / "data/card_corrections.json"
    actual = extract_clauses(canonical["cards"], cards_json=root / "data/cards.json",
                             overrides=root / "data/card_overrides.json", official=official)
    assert {c["id"]: c for c in actual} == {c["id"]: c for c in canonical["clauses"]}
    generated = build_denominator(canonical["cards"], actual,
                                  load_card_qa(root / "data/card_qa.json"), load_promotion(DEFAULT_PROMOTION))
    committed = json.loads((root / "data/interaction_denominator.json").read_text(encoding="utf-8"))
    pool = set(canonical["cards"])
    expected = {iid: record for iid, record in committed["interactions"].items()
                if pool.intersection(record["card_ids"]) and record["gating"]}
    assert {iid: record for iid, record in generated["interactions"].items()
            if record["gating"]} == expected
