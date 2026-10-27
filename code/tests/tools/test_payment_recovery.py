"""Fitted-checkpoint repair preserves lineage and never reuses affected replay."""
import copy
import json
from pathlib import Path
import shutil
import sys
from types import SimpleNamespace

import pytest

from tools.cloud_search import prepare_payment_recovery as prep
from tools.cloud_search import resume_payment_recovery as resume
from tools.cloud_search.replay_window import bounded_window
from tools.selfplay_scheduler import sha256

REPO = Path(__file__).resolve().parents[3]
IMAGE = 'registry/search@sha256:' + 'a' * 64


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding='utf-8')


def hashes(root):
    return {p.relative_to(root).as_posix(): sha256(p) for p in root.rglob('*') if p.is_file()}


@pytest.fixture
def args(tmp_path, monkeypatch):
    source, root = tmp_path / 'archive', tmp_path / 'recovery'
    inputs, gen1 = source / 'inputs', source / 'generation/gen1'
    inputs.mkdir(parents=True)
    gen1.mkdir(parents=True)
    pool = REPO / 'data/training_pools/ex13-20261026'
    for name in prep.POOL_FILES:
        shutil.copyfile(pool / name, inputs / name)
    shutil.copyfile(inputs / 'pet_regression_snapshot.json', inputs / 'pet_evaluation_snapshot.json')
    for name in ('seed.zip', 'seed.onnx', 'seed.onnx.meta.json', 'development.zip'):
        (inputs / name).write_text('original frozen anchor ' + name)
    for name in ('model.zip', 'model.onnx', 'model.onnx.meta.json'):
        (gen1 / name).write_text('fitted CP1 ' + name)
    write(inputs / 'anchors.json', dict(version=1, champions=[dict(name=name,
        weights_path='/experiment/inputs/' + filename, tensor_layout_hash='test-layout', algorithm='mlp')
        for name, filename in prep.ANCHOR_FILES.items()]))
    cards = tmp_path / 'cards.json'
    write(cards, {'runtime': 'unchanged card database'})
    cli = tmp_path / 'native.exe'
    cli.write_bytes(b'pinned repaired native engine')
    write(source / 'corpora/pilot/provenance.json', {'cards_sha256': sha256(cards)})
    write(source / 'experiment_state.json', dict(stage='failed', generation_index=1))
    write(source / 'generation/state.json', dict(next_gen=2, prev=dict(
        model_zip='/experiment/generation/gen1/model.zip', model_onnx='/experiment/generation/gen1/model.onnx')))
    write(source / 'evaluations/window-000001/result.json', {'status': 'failed'})
    write(source / 'started.json', {'run_id': 'ex13-repaired-4x1100-20261027'})
    write(gen1 / 'summary.json', {'generation': 1})
    write(gen1 / 'scheduler_progress.json', {'committed_games': 1100})
    write(gen1 / 'shards/proc00000000/manifest.json', dict(games=1100, rows=105563, truncated_games=0))
    (gen1 / 'shards/proc00000000/old-replay.npy').write_bytes(b'never reusable')
    source_plan = dict(generations=4, games=1100, workers=8, seed=20261027,
        image='registry/search@sha256:' + 'b' * 64, evaluation_seed=22261027,
        evaluation_protocol=prep.PROTOCOL, evaluation_panels=prep.PANELS,
        evaluation_games_per_archetype_per_anchor=200,
        search=prep.SEARCH, replay_window=3, replay_cap_games=1100,
        pool_denominators=prep.summarize_pool(inputs, inputs / 'pet_evaluation_snapshot.json'),
        files={**{f'inputs/{name}': digest for name, digest in hashes(inputs).items()},
               'corpora/pilot/provenance.json': sha256(source / 'corpora/pilot/provenance.json')},
        meta_archetypes=['Chronicle'], pet_archetypes=['Light Hybrid'])
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        write(source / name, source_plan)
    result = SimpleNamespace(source=source, root=root, source_commit='c' * 40, ci_run='123',
        source_plan_sha256=sha256(source / 'campaign-plan.json'), checkpoint_sha256=sha256(gen1 / 'model.zip'),
        onnx_sha256=sha256(gen1 / 'model.onnx'), onnx_metadata_sha256=sha256(gen1 / 'model.onnx.meta.json'),
        cards_json=cards, code_root=REPO / 'code', cli_bin=str(cli))
    monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', IMAGE)
    monkeypatch.setattr(prep, 'validate_models', lambda *a: dict(contract={
        'observation_profile': 'standard_lite_v2', 'tensor_layout_hash': 'test-layout'}, card_eval_event_version=2))
    monkeypatch.setattr(prep, 'run_health', lambda *a: [dict(start=start, games=4, rows=100, truncated_games=0)
                                                    for start in (0, 4)])
    return result


def test_recovery_retains_fitted_cp1_and_original_anchors_but_no_replay(args):
    before = hashes(args.source)
    plan = prep.prepare(args)
    assert hashes(args.source) == before
    assert sha256(args.root / 'generation/gen1/model.zip') == args.checkpoint_sha256
    assert (args.root / 'inputs/seed.zip').read_bytes() == (args.source / 'inputs/seed.zip').read_bytes()
    assert sha256(args.root / 'inputs/seed.zip') != args.checkpoint_sha256
    assert not (args.root / 'generation/gen0').exists()
    assert not list((args.root / 'generation').rglob('shards'))
    assert not list((args.root / 'generation').rglob('*.npy'))
    assert plan['baseline_checkpoint'] == 1 and plan['first_generation'] == 2
    assert plan['final_generation'] == 4 and plan['remaining_generations'] == 3
    assert plan['parent_replay_games'] == 0 and plan['fresh_replay']
    assert plan['inherited_collection'] == dict(games=1100, rows=105563, truncated_games=0)
    assert plan['evaluation_seed'] == 22261027 and plan['games_per_evaluation'] == 10800
    assert plan['search'] == prep.SEARCH and plan['fit'] == prep.FIT
    for _, filename in prep.PANELS:
        assert (args.root / 'inputs' / filename).read_bytes() == (args.source / 'inputs' / filename).read_bytes()
    assert set(plan['anchor_model_hashes']) == {'parent-checkpoint4', 'development-fixed'}
    resume.validate(args.root, plan, args.source)
    with pytest.raises(ValueError, match='empty'):
        prep.prepare(args)


@pytest.mark.parametrize('change, message', [
    ('source_plan', 'source-plan'), ('model', 'checkpoint'), ('onnx', 'checkpoint'), ('sidecar', 'checkpoint'),
    ('running', 'stopped'), ('anchor', 'source input'), ('cards', 'card data'), ('old_image', 'new repaired'),
    ('truncated', 'collection evidence'), ('overlap', 'overlap'), ('existing', 'empty'),
])
def test_preparation_fails_before_output_for_source_or_runtime_drift(args, monkeypatch, change, message):
    if change == 'source_plan':
        args.source_plan_sha256 = 'f' * 64
    elif change in ('model', 'onnx', 'sidecar'):
        suffix = {'model': '.zip', 'onnx': '.onnx', 'sidecar': '.onnx.meta.json'}[change]
        (args.source / f'generation/gen1/model{suffix}').write_bytes(b'changed')
    elif change == 'running':
        write(args.source / 'experiment_state.json', dict(stage='generation', generation_index=1))
    elif change == 'anchor':
        (args.source / 'inputs/seed.zip').write_bytes(b'substituted CP1')
    elif change == 'cards':
        args.cards_json.write_bytes(b'changed')
    elif change == 'old_image':
        monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', prep.read(args.source / 'campaign-plan.json')['image'])
    elif change == 'truncated':
        write(args.source / 'generation/gen1/shards/proc00000000/manifest.json',
              dict(games=1100, rows=105563, truncated_games=1))
    elif change == 'overlap':
        args.root = args.source / 'recovery'
    else:
        write(args.root / 'evidence.json', {'preserve': True})
    before = hashes(args.source)
    with pytest.raises(ValueError, match=message):
        prep.prepare(args)
    assert hashes(args.source) == before
    assert not (args.root / 'campaign-plan.json').exists()


def test_failed_native_health_preserves_evidence_without_runnable_plan(args, monkeypatch):
    monkeypatch.setattr(prep, 'run_health', lambda *a: [dict(start=0, games=4, rows=10, truncated_games=1)])
    with pytest.raises(ValueError, match='Eight complete'):
        prep.prepare(args)
    assert prep.read(args.root / 'validation/checkpoint1-recovery.json')['status'] == 'failed'
    assert not (args.root / 'campaign-plan.json').exists()


def test_health_uses_cp1_and_full_budget_and_rejects_truncation(tmp_path, monkeypatch):
    args = SimpleNamespace(cli_bin='native', cards_json=Path('cards.json'))
    commands = []
    def native(command, **kwargs):
        commands.append(command)
        out = Path(command[command.index('--out') + 1])
        write(out / 'manifest.json', dict(games=4, rows=32, truncated_games=0))
    monkeypatch.setattr(prep.subprocess, 'run', native)
    result = prep.run_health(args, tmp_path / 'ok')
    assert sum(h['games'] for h in result) == 8
    for command in commands:
        assert Path(command[command.index('--model') + 1]).parts[-2:] == ('gen1', 'model.onnx')
        for flag, value in (('--sims', '320'), ('--worlds', '4'), ('--eval-batch-size', '4'), ('--decision-cap', '1200')):
            assert command[command.index(flag) + 1] == value
    def truncated(command, **kwargs):
        write(Path(command[command.index('--out') + 1]) / 'manifest.json', dict(games=4, rows=32, truncated_games=1))
    monkeypatch.setattr(prep.subprocess, 'run', truncated)
    with pytest.raises(ValueError, match='health check'):
        prep.run_health(args, tmp_path / 'bad')


@pytest.mark.parametrize('change', ['replay', 'state', 'runtime', 'source', 'scope', 'output', 'anchor'])
def test_controller_rejects_drift_and_existing_output(args, change):
    plan = prep.prepare(args)
    if change == 'replay':
        write(args.root / 'generation/gen1/shards/proc0/manifest.json', dict(games=4))
    elif change == 'state':
        write(args.root / 'generation/state.json', {'next_gen': 1})
    elif change == 'runtime':
        Path(args.cli_bin).write_bytes(b'wrong runtime')
    elif change == 'source':
        write(args.source / 'experiment_state.json', {'stage': 'running'})
    elif change == 'scope':
        plan['first_generation'] = 1
    elif change == 'anchor':
        shutil.copyfile(args.root / 'generation/gen1/model.zip', args.root / 'inputs/seed.zip')
    else:
        (args.root / 'generation/gen2').mkdir()
    with pytest.raises(ValueError):
        resume.validate(args.root, plan, args.source)


def fake_evaluator(calls, failed=False):
    def evaluate(root, out, checkpoint, window, reason, options):
        calls.append(('evaluate', window, reason, options.seed))
        for panel, _ in options.panels:
            path = out / f'window-{window:06d}' / f'{panel}.log'
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('evaluation complete')
        return {'status': 'failed' if failed else 'complete'}
    return evaluate


def test_controller_evaluates_repaired_cp1_then_only_remaining_updates(args, monkeypatch):
    plan = prep.prepare(args)
    calls = []
    monkeypatch.setattr(resume, 'evaluate_window', fake_evaluator(calls))
    monkeypatch.setattr(resume, 'validate_evaluation', lambda *a: None)
    def generation(command, **kwargs):
        state = prep.read(args.root / 'generation/state.json')
        index = state['next_gen']
        calls.append(('generation', index))
        assert '--fail-on-truncation' in command
        for flag, expected in (('--sims', '320'), ('--worlds', '4'), ('--eval-batch-size', '4'),
                               ('--games', '1100'), ('--procs', '8'), ('--window', '3'), ('--epochs', '2')):
            assert command[command.index(flag) + 1] == expected
        path = args.root / f'generation/gen{index}'
        path.mkdir()
        (path / 'model.zip').write_bytes(b'new weights')
        write(args.root / 'generation/state.json', dict(next_gen=index + 1, prev=dict(
            model_zip=str(path / 'model.zip'), model_onnx=str(path / 'model.onnx'))))
    monkeypatch.setattr(resume.subprocess, 'run', generation)
    resume.run(args.root, args.root / 'evaluations', args.source, plan)
    assert [(c[0], c[1]) for c in calls] == [('evaluate', 1), ('generation', 2), ('evaluate', 2),
        ('generation', 3), ('evaluate', 3), ('generation', 4), ('evaluate', 4)]
    assert calls[0][2:] == ('repaired_engine_baseline', 22261027)
    assert prep.read(args.root / 'experiment_state.json')['stage'] == 'complete'
    with pytest.raises(ValueError, match='empty'):
        resume.run(args.root, args.root / 'evaluations', args.source, plan)


def test_failed_repaired_baseline_does_not_collect_any_new_games(args, monkeypatch):
    plan, calls = prep.prepare(args), []
    monkeypatch.setattr(resume, 'evaluate_window', fake_evaluator(calls, failed=True))
    monkeypatch.setattr(resume.subprocess, 'run', lambda *a, **kw: pytest.fail('Must not collect after failed baseline'))
    with pytest.raises(RuntimeError, match='evaluation failed'):
        resume.run(args.root, args.root / 'evaluations', args.source, plan)
    assert len(calls) == 1 and calls[0][1] == 1
    assert prep.read(args.root / 'experiment_state.json')['stage'] == 'failed'
    assert not (args.root / 'generation/gen2').exists()


def test_native_generation_driver_starts_two_from_cp1_without_old_shards(args, monkeypatch):
    from tools import run_selfplay_generation as job
    prep.prepare(args)
    run_dir = args.root / 'generation'
    assert bounded_window(run_dir, 2, 3, 1100, 20261027)[0] == []
    seen = {}
    def collect(**kwargs):
        seen['selfplay_model'] = kwargs['model_onnx']
        path = kwargs['gen_dir'] / 'shards/proc00000000'
        write(path / 'manifest.json', dict(games=4, rows=32, truncated_games=0))
        return [path]
    def train(**kwargs):
        seen.update(kwargs)
        kwargs['out'].write_bytes(b'new fitted weights')
        kwargs['export_onnx'].write_bytes(b'new export')
        return {}
    monkeypatch.setattr(job, 'run_selfplay_stage', collect)
    monkeypatch.setitem(sys.modules, 'digimon_gym.agents.azero_trainer', SimpleNamespace(train=train))
    command = resume.generation_command(args.root, prep.read(args.root / 'campaign-plan.json'))[3:]
    command[command.index('--cli-bin') + 1] = args.cli_bin
    assert job.main(command) == 0
    assert seen['selfplay_model'] == str(run_dir / 'gen1/model.onnx')
    assert seen['init_from'] == run_dir / 'gen1/model.zip'
    assert seen['shard_dirs'] == [run_dir / 'gen2/shards/proc00000000']
    assert prep.read(run_dir / 'state.json')['next_gen'] == 3
    record = prep.read(run_dir / 'gen2/replay_selection.json')
    assert [g['games_before_quarantine'] for g in record['generations']] == [0, 0, 4]


def make_report(root, plan, context, panel, cohort, index=1):
    """Mirror the CLI's context propagation while keeping engine games out of unit tests."""
    filename = dict(prep.PANELS)[panel]
    digest = sha256(root / f'generation/gen{index}/model.zip')
    provenance = dict(context, status='complete', cohort_scheme=plan['evaluation_protocol'],
        evaluation_mode='policy_only', panel=panel, candidate_sha256=digest,
        base_seed=plan['evaluation_seed'], games_per_archetype=200,
        requested_games_per_anchor=len(cohort) // 3,
        snapshot_sha256=plan['files'][f'inputs/{filename}'],
        anchors_sha256=plan['files']['inputs/anchors.json'], anchor_model_hashes=plan['anchor_model_hashes'],
        deterministic=True, card_eval_event_version=2, **plan['model_contract'])
    games = [dict(seed=r['seed'], context={k: v for k, v in dict(r, panel=panel).items()
                                        if k not in ('seed', 'opening_player')},
                  winner_id=1, terminal_reason='security') for r in cohort]
    return dict(schema_version=1, checkpoint_step=index, provenance=provenance, games=games,
                cards=[], routes=[], route_decisions=[], effect_sequences=[])


@pytest.mark.parametrize('change', [None, 'cards_digest', 'terminal', 'duplicate', 'missing_game', 'deck', 'anchor'])
def test_real_evaluation_worker_context_passes_strict_report_gate(args, monkeypatch, change):
    import data_paths
    from tools.cloud_search import card_eval_worker

    plan = prep.prepare(args)
    cohort = [dict(anchor=name, panel='combined_validation', evaluation_mode='policy_only',
        seed=12 + i, agent_archetype='Chronicle', opponent_archetype='Medusamon',
        agent_deck_id='chronicle-one', opponent_deck_id='medusa-one', opening_player=1 + i % 2)
        for name in ('greedy', 'parent-checkpoint4', 'development-fixed') for i in range(2)]
    monkeypatch.setattr(resume, 'expected_cohort', lambda *a: ('archetype-paired-v2', cohort))
    monkeypatch.setattr(data_paths, 'CARDS_JSON', args.cards_json)
    def cli(command, **kwargs):
        context = prep.read(Path(command[command.index('--eval-context') + 1]))
        assert context['runtime_cards_sha256'] == sha256(args.cards_json)
        panel = command[command.index('--eval-panel') + 1]
        report = make_report(args.root, plan, context, panel, cohort)
        report_dir = Path(command[command.index('--card-eval-dir') + 1])
        write(report_dir / f'checkpoint_1_{panel}.json', report)
    monkeypatch.setattr(card_eval_worker.subprocess, 'run', cli)
    out = args.root / 'evaluations'
    checkpoint = dict(path=str(args.root / 'generation/gen1/model.zip'), sha256=args.checkpoint_sha256, step=1)
    result = card_eval_worker.evaluate_window(args.root, out, checkpoint, 1, 'repaired_engine_baseline',
                                             resume.evaluation_options(plan))
    assert result['status'] == 'complete'
    folder = out / 'window-000001'
    report_path = folder / 'reports/checkpoint_1_combined_diagnostic.json'
    report = prep.read(report_path)
    if change == 'cards_digest':
        report['provenance']['runtime_cards_sha256'] = 'f' * 64
    elif change == 'terminal':
        report['games'][0]['terminal_reason'] = 'step_limit'
    elif change == 'duplicate':
        report['games'][0] = copy.deepcopy(report['games'][1])
    elif change == 'missing_game':
        report['games'].pop()
    elif change == 'deck':
        report['games'][0]['context']['agent_deck_id'] = 'substituted list'
    elif change == 'anchor':
        report['provenance']['anchor_model_hashes']['parent-checkpoint4'] = args.checkpoint_sha256
    write(report_path, report)
    if change:
        with pytest.raises(ValueError):
            resume.validate_evaluation(folder, args.root, plan, 1)
    else:
        resume.validate_evaluation(folder, args.root, plan, 1)
