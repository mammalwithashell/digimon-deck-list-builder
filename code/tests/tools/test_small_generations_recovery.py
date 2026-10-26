import json
from pathlib import Path

import pytest

from tools.cloud_search.recover_small_generations import validate_recovery
from tools.cloud_search.run_small_generations import generation_command
from tools.selfplay_scheduler import sha256


@pytest.fixture
def prepared(tmp_path, monkeypatch):
    root = tmp_path
    parent = root/'generation/gen1'
    parent.mkdir(parents=True)
    (parent/'model.zip').write_bytes(b'checkpoint')
    (parent/'model.onnx').write_bytes(b'onnx')
    (root/'inputs').mkdir()
    for name in ('heldout_snapshot.json', 'pet_evaluation_snapshot.json'):
        (root/'inputs'/name).write_text('{}')
    plan = dict(generations=4, games=1100, workers=8, seed=123,
        files={f'inputs/{p.name}':sha256(p) for p in (root/'inputs').iterdir()})
    (root/'campaign-plan.json').write_text(json.dumps(plan))
    (root/'experiment_state.json').write_text(json.dumps(dict(stage='failed')))
    (root/'generation/state.json').write_text(json.dumps(dict(next_gen=2,
        prev=dict(model_zip=str(parent/'model.zip'), model_onnx=str(parent/'model.onnx')))))
    runtime = root/'runtime.py'
    runtime.write_text('runtime')
    recovery = dict(image='fixed-image', generation=2, excluded_games=1,
        evaluation_window_base=1000, runtime_files={str(runtime):sha256(runtime)},
        files={p.relative_to(root).as_posix():sha256(p) for p in root.rglob('*') if p.is_file()})
    monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', 'fixed-image')
    return root, recovery


def test_prepared_failed_campaign_is_recoverable(prepared):
    assert validate_recovery(*prepared)['generations'] == 4


@pytest.mark.parametrize('change', ['model', 'runtime', 'state', 'later', 'fitted', 'window', 'unbound', 'image'])
def test_recovery_refuses_changed_or_reused_evidence(prepared, monkeypatch, change):
    root, recovery = prepared
    if change == 'model':
        (root/'generation/gen1/model.zip').write_bytes(b'changed')
    elif change == 'runtime':
        (root/'runtime.py').write_text('changed')
    elif change == 'state':
        (root/'generation/state.json').write_text('{}')
    elif change == 'later':
        (root/'generation/gen3').mkdir()
    elif change == 'fitted':
        p=root/'generation/gen2/model.zip'; p.parent.mkdir(); p.write_bytes(b'existing')
    elif change == 'window':
        (root/'evaluations/window-001001').mkdir(parents=True)
    elif change == 'unbound':
        recovery['files'].pop('generation/gen1/model.zip')
    else:
        monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE','wrong')
    with pytest.raises(ValueError):
        validate_recovery(root,recovery)


def test_recovery_keeps_search_budget_and_truncation_guard():
    cmd=generation_command(Path('/experiment'), dict(games=1100,workers=8,seed=123))
    for flag, value in [('--sims','320'),('--worlds','4'),('--eval-batch-size','4'),
                        ('--decision-cap','1200'),('--window','3'),('--games','1100')]:
        assert cmd[cmd.index(flag)+1] == value
    assert '--fail-on-truncation' in cmd and '--resume-selfplay' not in cmd
