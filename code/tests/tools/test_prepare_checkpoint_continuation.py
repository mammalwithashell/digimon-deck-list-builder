"""Checkpoint-only rollover must preserve failed evidence and exclude old replay."""
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from tools.cloud_search import prepare_checkpoint_continuation as prep
from tools.cloud_search.run_small_generations import validate
from tools.selfplay_scheduler import sha256

REPO = Path(__file__).resolve().parents[3]
IMAGE = 'registry/search@sha256:' + 'a' * 64


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding='utf-8')


def tree_hashes(path):
    return {p.relative_to(path).as_posix(): sha256(p) for p in path.rglob('*') if p.is_file()}


@pytest.fixture
def prepared_args(tmp_path, monkeypatch):
    source = tmp_path / 'archive'
    pool = REPO / 'data/training_pools/ex13-20261026'
    parent = source / 'generation/gen4'
    parent.mkdir(parents=True)
    (parent / 'model.zip').write_bytes(b'fitted checkpoint 4')
    (parent / 'model.onnx').write_bytes(b'onnx checkpoint 4')
    write(parent / 'model.onnx.meta.json', {'tensor_layout_hash': 'test-layout'})
    write(parent / 'shards/proc00000000/manifest.json', {'games': 1100})
    write(source / 'experiment_state.json', {'stage': 'failed', 'generation_index': 4,
                                            'error': 'Checkpoint evaluation failed'})
    write(source / 'evaluations/window-000004/result.json', {'status': 'failed'})
    write(source / 'started.json', {'run_id': 'old-diagnostic-run'})
    write(source / 'inputs/pet_evaluation_snapshot.json',
          json.loads((pool / 'pet_regression_snapshot.json').read_text()))
    (source / 'inputs/development.zip').write_bytes(b'frozen development anchor')
    write(source / 'campaign-plan.json', {'files': {
        f'inputs/{name}': sha256(source / 'inputs' / name)
        for name in ('development.zip', 'pet_evaluation_snapshot.json')},
        'meta_archetypes': ['Chronicle'], 'pet_archetypes': ['Light Hybrid']})
    cards = REPO / 'data/cards.json'
    write(source / 'corpora/pilot/provenance.json', {'cards_sha256': sha256(cards)})
    selection = tmp_path / 'assessment/result.json'
    write(selection, {'status': 'complete', 'image': IMAGE, 'protocol': 'archetype-paired-v2',
                      'games_per_archetype': 200, 'games_per_checkpoint': 7800,
                      'selection': {'checkpoint_index': 4, 'checkpoint_sha256': sha256(parent / 'model.zip'),
                                    'automatic_promotion': False}})
    write(selection.parent / 'plan.json', {'fixture': 'assessment validation is isolated below'})
    write(selection.parent / 'cohort.json', [])
    args = SimpleNamespace(source=source, root=tmp_path / 'new-campaign', pool_dir=pool,
                           parent_generation=4, parent_sha256=sha256(parent / 'model.zip'),
                           selection_report=selection, source_commit='b' * 40, ci_run='12345',
                           seed=123, evaluation_seed=456, games=None, workers=8,
                           cards_json=cards, code_root=REPO / 'code', cli_bin='digimon-engine-cli')
    monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', IMAGE)
    monkeypatch.setattr(prep, 'validate_assessment', lambda path, **kwargs: json.loads(path.read_text()))
    monkeypatch.setattr(prep, 'validate_runtime', lambda *a: {
        'contract': {'observation_profile': 'standard_lite_v2', 'tensor_layout_hash': 'test-layout'},
        'card_eval_event_version': 2, 'registered_cards': 1185})
    monkeypatch.setattr(prep, 'run_health', lambda *a: [
        {'start': start, 'games': 4, 'rows': 100, 'truncated_games': 0} for start in (0, 4)])
    return args


def test_failed_parent_becomes_weights_only_seed_without_changing_archive(prepared_args):
    args = prepared_args
    before = tree_hashes(args.source)
    plan = prep.prepare(args)
    assert tree_hashes(args.source) == before
    assert not list(args.root.glob('generation/gen0/shards'))
    assert sha256(args.root / 'generation/gen0/model.zip') == args.parent_sha256
    assert plan['parent_source_state']['stage'] == 'failed'
    assert plan['fresh_replay'] and plan['parent_replay_games'] == 0
    assert plan['games'] == 1100 and plan['generations'] == 4 and plan['replay_window'] == 3
    assert plan['search'] == {'simulations': 320, 'worlds': 4, 'evaluation_batch_size': 4, 'decision_cap': 1200}
    assert len(validate(args.root, plan)) == 13
    assert plan['games_per_evaluation'] == 10800
    assert plan['evaluation_panels'] == [
        ['combined_diagnostic', 'combined_diagnostic_snapshot.json'],
        ['original_pet_regression', 'pet_evaluation_snapshot.json']]
    assert plan['pool_denominators']['training']['distinct_lists'] == 21
    assert plan['pool_denominators']['training']['native_sampling_slots'] == 26
    assert plan['pool_denominators']['heldout']['archetypes'] == 12
    assert plan['pool_denominators']['combined_diagnostic']['same_list_archetypes'] == ['Light Hybrid']
    assert (args.root / 'inputs/supplemental_snapshot.json').exists()
    assert (args.root / 'inputs/pool_report.json').exists()
    assert plan['evaluation_protocol'] == 'archetype-paired-v2'
    with pytest.raises(ValueError, match='empty'):
        prep.prepare(args)


def test_selected_gen3_can_seed_from_a_failed_gen4_campaign(prepared_args):
    args = prepared_args
    model = args.source / 'generation/gen3'
    model.mkdir()
    (model / 'model.zip').write_bytes(b'approved earlier checkpoint')
    (model / 'model.onnx').write_bytes(b'approved earlier export')
    write(model / 'model.onnx.meta.json', {'tensor_layout_hash': 'test-layout'})
    args.parent_generation = 3
    args.parent_sha256 = sha256(model / 'model.zip')
    report = json.loads(args.selection_report.read_text())
    report['selection'].update(checkpoint_index=3, checkpoint_sha256=args.parent_sha256)
    write(args.selection_report, report)
    before = tree_hashes(args.source)
    plan = prep.prepare(args)
    assert plan['parent_checkpoint'] == 3
    assert plan['parent_source_state']['generation_index'] == 4
    assert sha256(args.root / 'inputs/seed.zip') == args.parent_sha256
    assert tree_hashes(args.source) == before


@pytest.mark.parametrize('change, message', [
    ('running', 'inactive'), ('parent_hash', 'checkpoint'), ('selection_image', 'assessment'),
    ('selection_failed', 'assessment'), ('selection_other_checkpoint', 'assessment'),
    ('selection_small_cohort', '200-game'),
    ('development', 'Frozen'), ('cards', 'Card data'), ('pet', 'original pet'),
    ('inside_source', 'overlap'), ('existing_destination', 'empty'), ('too_few_games', '150'),
])
def test_preparation_refuses_drift_before_creating_output(prepared_args, change, message):
    args = prepared_args
    if change == 'running':
        write(args.source / 'experiment_state.json', {'stage': 'generation', 'generation_index': 4})
    elif change == 'parent_hash':
        args.parent_sha256 = 'f' * 64
    elif change.startswith('selection_'):
        report = json.loads(args.selection_report.read_text())
        if change == 'selection_image':
            report['image'] = 'other'
        elif change == 'selection_failed':
            report['status'] = 'failed'
        elif change == 'selection_small_cohort':
            report.update(games_per_archetype=2, games_per_checkpoint=78)
        else:
            report['selection']['checkpoint_index'] = 3
        write(args.selection_report, report)
    elif change == 'development':
        (args.source / 'inputs/development.zip').write_bytes(b'changed')
    elif change == 'cards':
        write(args.source / 'corpora/pilot/provenance.json', {'cards_sha256': 'f' * 64})
    elif change == 'pet':
        path = args.source / 'inputs/pet_evaluation_snapshot.json'
        value = json.loads(path.read_text())
        value['decks'] = value['decks'][:-1]
        write(path, value)
        plan_path = args.source / 'campaign-plan.json'
        plan = json.loads(plan_path.read_text())
        plan['files']['inputs/pet_evaluation_snapshot.json'] = sha256(path)
        write(plan_path, plan)
    elif change == 'inside_source':
        args.root = args.source / 'new-campaign'
    elif change == 'existing_destination':
        write(args.root / 'unrelated.json', {'preserve': True})
    else:
        args.games = 900
    before = tree_hashes(args.source)
    with pytest.raises(ValueError, match=message):
        prep.prepare(args)
    assert tree_hashes(args.source) == before
    assert not (args.root / 'campaign-plan.json').exists()


def test_a_status_only_assessment_cannot_authorize_preparation(prepared_args, monkeypatch):
    from tools.cloud_search.evaluate_checkpoint_transition import validate_assessment
    monkeypatch.setattr(prep, 'validate_assessment', validate_assessment)
    before = tree_hashes(prepared_args.source)
    with pytest.raises((ValueError, KeyError)):
        prep.prepare(prepared_args)
    assert not prepared_args.root.exists()
    assert tree_hashes(prepared_args.source) == before


def test_failed_health_is_recorded_without_usable_campaign(prepared_args, monkeypatch):
    def fail(*args):
        raise ValueError('Native full-budget health check failed')
    monkeypatch.setattr(prep, 'run_health', fail)
    before = tree_hashes(prepared_args.source)
    with pytest.raises(ValueError, match='health check'):
        prep.prepare(prepared_args)
    evidence = json.loads((prepared_args.root / 'validation/generation0-recovery.json').read_text())
    assert evidence['status'] == 'failed'
    assert not (prepared_args.root / 'campaign-plan.json').exists()
    assert tree_hashes(prepared_args.source) == before


def test_health_runs_eight_full_budget_games_and_rejects_truncation(tmp_path, monkeypatch):
    args = SimpleNamespace(cards_json=Path('/app/data/cards.json'), cli_bin='native', seed=40)
    seen = []
    def run(command, **kwargs):
        seen.append(command)
        out = Path(command[command.index('--out') + 1])
        write(out / 'manifest.json', {'games': 4, 'truncated_games': 0, 'rows': 20})
    monkeypatch.setattr(prep.subprocess, 'run', run)
    results = prep.run_health(args, tmp_path)
    assert sum(r['games'] for r in results) == 8
    for command in seen:
        assert command[command.index('--sims') + 1] == '320'
        assert command[command.index('--worlds') + 1] == '4'
        assert command[command.index('--eval-batch-size') + 1] == '4'
    def truncated(command, **kwargs):
        out = Path(command[command.index('--out') + 1])
        write(out / 'manifest.json', {'games': 4, 'truncated_games': 1, 'rows': 20})
    monkeypatch.setattr(prep.subprocess, 'run', truncated)
    with pytest.raises(ValueError, match='health check'):
        prep.run_health(args, tmp_path / 'bad')
