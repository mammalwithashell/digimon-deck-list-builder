"""Up to four bounded search updates with one fixed-cohort evaluation per model.

Starts from an explicitly prepared, hash-bound checkpoint and archive. No
automatic restart, promotion, partial-run adoption or source-data mutation.
"""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from types import SimpleNamespace

from tools.cloud_search.card_eval_worker import evaluate_window
from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def evaluation_options(plan):
    panels = plan.get('evaluation_panels', [('meta_validation', 'heldout_snapshot.json'),
                                          ('pet_same_list', 'pet_evaluation_snapshot.json')])
    for panel, filename in panels:
        if any(not value or Path(value).name != value or '/' in value or '\\' in value
               for value in (panel, filename)):
            raise ValueError('Evaluation panels must use local labels and filenames')
        if f'inputs/{filename}' not in plan['files']:
            raise ValueError('Evaluation snapshot must be hash-bound')
    games = plan.get('evaluation_games_per_archetype_per_anchor', 200)
    if not panels or games <= 0 or games % 2:
        raise ValueError('Use nonempty panels and positive even evaluation quotas')
    return SimpleNamespace(panels=panels, games=games, games_per_archetype=games,
                           seed=plan.get('evaluation_seed', 28261025),
                           profile='standard_lite_v2', timeout=10800)


def validate(root, plan):
    first = plan.get('first_generation', 1)
    last = plan['generations']
    if type(first) is not int or type(last) is not int or not 1 <= first <= last:
        raise ValueError('Invalid first generation')
    # Campaigns are bounded at four generations unless the plan explicitly
    # opts into a longer, still bounded, run (at most twelve).
    limit = 12 if plan.get('long_campaign') is True else 4
    if last - first + 1 not in range(1, limit + 1) or plan['games'] < 1000 or plan['workers'] not in range(1, 9):
        raise ValueError('Unexpected campaign scope')
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image']:
        raise ValueError('Image differs from approved engine')
    parent = first - 1
    required = {'inputs/decks.json', 'inputs/seed.zip', 'inputs/seed.onnx',
                'inputs/heldout_snapshot.json', 'inputs/pet_evaluation_snapshot.json',
                'inputs/anchors.json', f'generation/gen{parent}/model.zip', f'generation/gen{parent}/model.onnx',
                'validation/generation0-recovery.json'}
    if not required <= set(plan['files']):
        raise ValueError('Missing required input hashes')
    for name, digest in plan['files'].items():
        path = root / name
        if not path.resolve().is_relative_to(root.resolve()) or sha256(path) != digest:
            raise ValueError(f'Input hash mismatch: {name}')
    for name, digest in plan['runtime_files'].items():
        if sha256(Path(name)) != digest:
            raise ValueError(f'Runtime hash mismatch: {name}')
    report = json.loads((root / 'validation/generation0-recovery.json').read_text())
    if (report.get('status') != 'passed' or report.get('image') != plan['image']
            or report.get('checkpoint_sha256') != sha256(root / f'generation/gen{parent}/model.zip')):
        raise ValueError('Parent checkpoint validation differs')
    if plan.get('reuse_parent_evaluation', False):
        receipt_path = 'validation/parent-evaluation.json'
        if receipt_path not in plan['files']:
            raise ValueError('Reusing evaluation requires a hash-bound receipt')
        receipt = json.loads((root / receipt_path).read_text())
        if (receipt.get('status') != 'complete'
                or receipt.get('checkpoint_sha256') != report['checkpoint_sha256']
                or receipt.get('evaluation_panels') != plan.get('evaluation_panels')
                or receipt.get('evaluation_seed') != plan.get('evaluation_seed')
                or receipt.get('games_per_archetype') != plan.get('evaluation_games_per_archetype_per_anchor')
                or not receipt.get('policy_evaluation_unchanged')):
            raise ValueError('Parent evaluation reuse evidence differs')
    slots = Counter(d['name'].split(':sha256:', 1)[0] for d in json.loads((root / 'inputs/decks.json').read_text()))
    if len(set(slots.values())) != 1 or plan['games'] * 2 / len(slots) < 150:
        raise ValueError('Pool must have equal archetype weight and >=150 expected appearances')
    state = json.loads((root / 'generation/state.json').read_text())
    if state != dict(next_gen=first, prev=dict(model_zip=str(root / f'generation/gen{parent}/model.zip'),
                                          model_onnx=str(root / f'generation/gen{parent}/model.onnx'))):
        raise ValueError('Expected prepared parent state')
    if any(p.name[3:].isdigit() and int(p.name[3:]) >= first
           for p in (root / 'generation').glob('gen*')):
        raise ValueError('Existing campaign output; explicit recovery required')
    evaluation_options(plan)
    training_options(plan)
    overrides = plan.get('historical_opponents_by_generation', {})
    if not isinstance(overrides, dict) or any(
            not key.isdigit() or not first <= int(key) <= last for key in overrides):
        raise ValueError('Per-generation opponent overrides must name campaign generations')
    for key, config in sorted(overrides.items()):
        if config is not None:
            validate_historical(root, plan, slots, config, int(key), parent, resolve_history=False)
    config = generation_config(plan, first)
    if config is not None:
        validate_historical(root, plan, slots, config, first, parent, resolve_history=True)
    return slots


def generation_config(plan, index):
    """Opponent config for one generation: a per-generation override wins
    (``None`` there means pure self-play), else the campaign-wide config."""
    overrides = plan.get('historical_opponents_by_generation', {})
    if str(index) in overrides:
        return overrides[str(index)]
    return plan.get('historical_opponents')


def training_options(plan):
    """Fitter settings; the defaults are the historical hard-coded recipe."""
    config = plan.get('training', {})
    allowed = {'epochs', 'lr', 'batch_size', 'value_holdout_fraction', 'value_activation'}
    if not isinstance(config, dict) or set(config) - allowed:
        raise ValueError('Invalid training options')
    options = dict(epochs=2, lr=0.0001, batch_size=512, value_holdout_fraction=0.0, value_activation=None)
    options.update(config)
    if (type(options['epochs']) is not int or options['epochs'] < 1
            or not 0.0 <= options['value_holdout_fraction'] < 1.0
            or options['value_activation'] not in (None, 'identity', 'tanh')):
        raise ValueError('Invalid training options')
    return options


def validate_historical(root, plan, slots, config, index, parent, resolve_history):
    from tools.search_opponents import resolve_pool, metadata_path
    options = historical_options(root, config)
    current = root / f'generation/gen{parent}/model.onnx'
    if not resolve_history:
        # Later generations' history does not exist yet; validate the
        # pinned pool and mixture now, history entries at freeze time.
        options = dict(options, history_window=0)
        if options['source'] is None:
            if plan['games'] * (1 + (options['selfplay_probability'] or 0.5)) / len(slots) < 150:
                raise ValueError('Historical mixture needs >=150 expected learner appearances per archetype')
            return
    pool = resolve_pool(root / 'generation', index, current, **options)
    required_assets = [current, metadata_path(current)]
    if options['source']:
        required_assets.append(options['source'])
    for entry in pool['opponents']:
        model = Path(entry['path'])
        required_assets.extend([model, metadata_path(model)])
        if (model.parent.parent == (root / 'generation').resolve()
                and model.parent.name.startswith('gen') and model.parent.name[3:].isdigit()
                and entry['id'] == f'generation-{int(model.parent.name[3:])}'):
            required_assets.extend([model.with_suffix('.zip'), model.with_name('summary.json')])
    for path in required_assets:
        resolved = path.resolve()
        if not resolved.is_relative_to(root.resolve()):
            raise ValueError('Historical opponents must be contained in the campaign root')
        name = resolved.relative_to(root.resolve()).as_posix()
        if plan['files'].get(name) != sha256(resolved):
            raise ValueError(f'Missing historical opponent input hash: {name}')
    if plan['games'] * (1 + pool['selfplay_probability']) / len(slots) < 150:
        raise ValueError('Historical mixture needs >=150 expected learner appearances per archetype')


def historical_options(root, config):
    if not isinstance(config, dict) or set(config) - {'pool', 'history_window', 'selfplay_probability'}:
        raise ValueError('Invalid historical_opponents plan options')
    source = root / config['pool'] if config.get('pool') else None
    if source and not source.resolve().is_relative_to(root.resolve()):
        raise ValueError('Historical pool must be inside the campaign root')
    if source is None and not config.get('history_window'):
        raise ValueError('Historical opponents require a pool or history_window')
    return dict(source=source, history_window=config.get('history_window', 0),
                selfplay_probability=config.get('selfplay_probability'))


def generation_command(root, plan, index=None):
    training = training_options(plan)
    command = [sys.executable, '-m', 'tools.run_selfplay_generation',
        '--run-dir', str(root / 'generation'), '--seed-model', str(root / 'inputs/seed.zip'),
        '--seed-model-onnx', str(root / 'inputs/seed.onnx'), '--decks', str(root / 'inputs/decks.json'),
        '--cli-bin', '/usr/local/bin/digimon-engine-cli', '--cards-json', '/app/data/cards.json',
        '--generations', '1', '--games', str(plan['games']), '--procs', str(plan['workers']),
        '--chunk-games', '4', '--worker-timeout-seconds', '7200', '--sims', '320',
        '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
        '--fail-on-truncation', '--window', '3', '--replay-games-per-generation', str(plan['games']),
        '--epochs', str(training['epochs']), '--batch-size', str(training['batch_size']),
        '--lr', str(training['lr']), '--train-device', 'cpu',
        '--seed', str(plan['seed']), '--skip-eval']
    if training['value_holdout_fraction']:
        command += ['--value-holdout-fraction', str(training['value_holdout_fraction'])]
    if training['value_activation'] is not None:
        command += ['--value-activation', training['value_activation']]
    config = generation_config(plan, plan.get('first_generation', 1) if index is None else index)
    if config is not None:
        options = historical_options(root, config)
        if options['source']:
            command += ['--opponent-pool', str(options['source'])]
        command += ['--history-window', str(options['history_window'])]
        if options['selfplay_probability'] is not None:
            command += ['--selfplay-probability', str(options['selfplay_probability'])]
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--evaluation-root', type=Path, default=Path('/card-evaluation'))
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    root, out = args.root, args.evaluation_root
    plan = json.loads((root / 'campaign-plan.json').read_text())
    validate(root, plan)
    if args.validate_only:
        print('Campaign inputs and runtime hashes verified', flush=True)
        return
    with controller_lock(root / 'campaign.lock'):
        with (root / 'campaign-started.json').open('x') as f:
            json.dump(dict(time_unix=time.time(), plan=plan), f, indent=2)
        out.mkdir(parents=True, exist_ok=True)

        def stage(name, index, **extra):
            atomic_json(root / 'experiment_state.json', dict(stage=name, generation_index=index,
                time_unix=time.time(), image=plan['image'], automatic_restart=False,
                automatic_promotion=False, **extra))

        def evaluate(index):
            model = root / 'generation' / f'gen{index}' / 'model.zip'
            checkpoint = dict(path=str(model), sha256=sha256(model), step=index)
            atomic_json(out / 'status.json', dict(status='running', checkpoint=checkpoint, window=index))
            options = evaluation_options(plan)
            result = evaluate_window(root, out, checkpoint, index, 'new_checkpoint', options)
            directory = root / 'generation' / f'gen{index}'
            # Keep monitor-compatible scalar summaries alongside full card reports.
            import shutil
            for panel, _ in options.panels:
                target = {'meta_validation': 'anchored_eval.txt', 'pet_same_list': 'pet_anchored_eval.txt'}.get(panel, f'{panel}.txt')
                shutil.copyfile(out / f'window-{index:06d}' / f'{panel}.log', directory / target)
            atomic_json(out / 'status.json', dict(result, worker_status='waiting'))
            if result['status'] != 'complete':
                raise RuntimeError(f'Checkpoint evaluation failed: {result}')

        first = plan.get('first_generation', 1)
        index = first - 1
        try:
            if not plan.get('reuse_parent_evaluation', False):
                stage('checkpoint_evaluation', index)
                evaluate(index)
            for index in range(first, plan['generations'] + 1):
                stage('generation', index, games=plan['games'], workers=plan['workers'])
                print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
                subprocess.run(generation_command(root, plan, index), check=True)
                stage('checkpoint_evaluation', index)
                evaluate(index)
            stage('complete', index)
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='complete'))
        except BaseException as error:
            stage('failed', index, error=str(error))
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='failed',
                        last_result=dict(status='failed', error=str(error))))
            raise


if __name__ == '__main__':
    main()
