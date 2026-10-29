"""Continuation must be bound to the parent, inputs, validation and image."""
import json
from pathlib import Path

import pytest
from tools.cloud_search.continue_experiment import validate
from tools.selfplay_scheduler import sha256


@pytest.fixture
def prepared(tmp_path, monkeypatch):
    paths = ['generation/gen0/model.zip', 'generation/gen0/model.onnx',
             'inputs/decks.json', 'inputs/anchors.json', 'inputs/heldout_snapshot.json',
             'inputs/pet_evaluation_snapshot.json']
    for name in paths:
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(name)
    parent = tmp_path / 'generation/gen0'
    (tmp_path / 'generation/state.json').write_text(json.dumps(dict(next_gen=1,
        prev=dict(model_zip=str(parent / 'model.zip'), model_onnx=str(parent / 'model.onnx')))))
    image = 'image@sha256:fixed'
    report = tmp_path / 'validation.json'
    report.write_text(json.dumps(dict(status='passed', image=image,
        checkpoint_sha256=sha256(parent / 'model.zip'))))
    paths.append('validation.json')
    plan = dict(generation_index=1, games=3300, workers=8, image=image,
                validation_report='validation.json', files={p:sha256(tmp_path / p) for p in paths})
    monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', image)
    return tmp_path, plan


def test_approved_continuation(prepared):
    assert validate(*prepared) == 1


@pytest.mark.parametrize('change', ['checkpoint', 'image', 'started', 'gate', 'omitted_hash', 'runtime'])
def test_invalid_continuation_rejected(prepared, monkeypatch, change):
    root, plan = prepared
    if change == 'checkpoint':
        (root / 'generation/gen0/model.zip').write_text('changed')
    elif change == 'image':
        monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', 'different-image')
    elif change == 'started':
        (root / 'generation/gen1').mkdir()
    elif change == 'gate':
        path = root / 'validation.json'
        path.write_text(json.dumps(dict(status='failed', image=plan['image'])))
        plan['files']['validation.json'] = sha256(path)
    elif change == 'runtime':
        plan['runtime_files'] = {str(root / 'inputs/decks.json'): 'wrong'}
    else:
        del plan['files']['inputs/decks.json']
    with pytest.raises(ValueError):
        validate(root, plan)
