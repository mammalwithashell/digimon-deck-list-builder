"""Explicit recovery after audited quarantine; preserve every original shard.

Requires a separately prepared, hash-bound recovery plan. No implicit retries,
model overwrites, unreviewed exclusions or search-budget changes.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from tools.cloud_search.card_eval_worker import evaluate_window
from tools.cloud_search.replay_window import bounded_window
from tools.cloud_search.run_small_generations import evaluation_options, generation_command
from tools.selfplay_scheduler import _read_commit, atomic_json, controller_lock, sha256


def read(path):
    return json.loads(path.read_text())


def validate_recovery(root, recovery):
    if recovery['image'] != os.environ.get('DIGIMON_DEPLOYED_IMAGE'):
        raise ValueError('Recovery image differs')
    required = {'campaign-plan.json', 'experiment_state.json', 'generation/state.json',
                f"generation/gen{recovery['generation']-1}/model.zip",
                f"generation/gen{recovery['generation']-1}/model.onnx"}
    if not required <= set(recovery['files']) or not recovery['runtime_files']:
        raise ValueError('Missing frozen recovery identities')
    for relative, digest in recovery['files'].items():
        path = root / relative
        if not path.resolve().is_relative_to(root.resolve()) or sha256(path) != digest:
            raise ValueError(f'Recovery evidence changed: {relative}')
    for name, digest in recovery['runtime_files'].items():
        if sha256(Path(name)) != digest:
            raise ValueError(f'Recovery runtime changed: {name}')
    plan = read(root / 'campaign-plan.json')
    for relative, digest in plan['files'].items():
        if sha256(root / relative) != digest:
            raise ValueError(f'Original input changed: {relative}')
    index = recovery['generation']
    if type(index) is not int or not 1 <= index <= plan['generations'] <= 4:
        raise ValueError('Invalid recovery scope')
    state = read(root / 'generation/state.json')
    previous = root / 'generation' / f'gen{index-1}'
    if state != dict(next_gen=index, prev=dict(model_zip=str(previous/'model.zip'),
                                               model_onnx=str(previous/'model.onnx'))):
        raise ValueError('Unexpected generation state')
    if read(root / 'experiment_state.json')['stage'] != 'failed':
        raise ValueError('Only explicitly failed campaigns may recover')
    for i in range(index, plan['generations'] + 1):
        folder = root / 'generation' / f'gen{i}'
        if i > index and folder.exists():
            raise ValueError('Later output exists')
        if any((folder / n).exists() for n in ('model.zip', 'model.onnx', 'summary.json')):
            raise ValueError('Refusing to overwrite fitted output')
    if recovery['evaluation_window_base'] < 1000:
        raise ValueError('Recovery evaluations need distinct window IDs')
    for i in range(index-1, plan['generations']+1):
        if (root/'evaluations'/f"window-{recovery['evaluation_window_base']+i:06d}").exists():
            raise ValueError('Recovery evaluation already exists')
    evaluation_options(plan)
    return plan


def verify_replay(root, plan, recovery):
    from digimon_gym.agents.azero_trainer import load_shards
    index = recovery['generation']
    for i in range(max(0, index-2), index+1):
        directories = sorted((root/'generation'/f'gen{i}'/'shards').glob('proc*'))
        games = 0
        for directory in directories:
            m = read(directory/'manifest.json')
            _read_commit(directory, m['game_start'], m['games'], m['seed'])
            if m['game_start'] != games:
                raise ValueError('Replay has gaps or overlapping games')
            games += m['games']
            exclusion = directory/'training_exclusions.json'
            if exclusion.exists() and str(exclusion.relative_to(root)) not in recovery['files']:
                raise ValueError('Unreviewed quarantine')
        if games != plan['games']:
            raise ValueError('Incomplete source generation')
    dirs, record = bounded_window(root/'generation', index, 3, plan['games'], plan['seed'])
    ds = load_shards(dirs)
    if not ds.terminated.all() or sum(len(q['games']) for q in ds.exclusions) != recovery['excluded_games']:
        raise ValueError('Unquarantined truncation or unexpected exclusions')
    rows = len(ds.z)
    del ds
    return dirs, record, rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    root = args.root
    recovery = read(root/'recovery-plan.json')
    with controller_lock(root/'campaign.lock'):
        plan = validate_recovery(root, recovery)
        dirs, record, rows = verify_replay(root, plan, recovery)
        if args.validate_only:
            print(json.dumps(dict(status='passed', rows=rows, excluded_games=recovery['excluded_games'])))
            return
        with (root/'recovery-started.json').open('x') as stream:
            json.dump(dict(time_unix=time.time(), recovery=recovery), stream, indent=2)
        out = root/'evaluations'
        index = recovery['generation']

        def stage(name, generation, **extra):
            atomic_json(root/'experiment_state.json', dict(stage=name, generation_index=generation,
                image=recovery['image'], time_unix=time.time(), automatic_restart=False,
                automatic_promotion=False, recovery=True, **extra))

        def evaluate(i):
            stage('checkpoint_evaluation', i)
            window = recovery['evaluation_window_base'] + i
            model = root/'generation'/f'gen{i}'/'model.zip'
            checkpoint = dict(path=str(model), sha256=sha256(model), step=i)
            atomic_json(out/'status.json', dict(status='running', checkpoint=checkpoint, window=window))
            result = evaluate_window(root, out, checkpoint, window, 'engine_fix_recovery', evaluation_options(plan))
            atomic_json(out/'status.json', dict(result, worker_status='waiting'))
            if result['status'] != 'complete':
                raise RuntimeError(f'Evaluation failed: {result}')
            for panel, _ in evaluation_options(plan).panels:
                shutil.copyfile(out/f'window-{window:06d}'/f'{panel}.log',
                                root/'generation'/f'gen{i}'/f'{panel}.txt')

        try:
            # Refresh the last saved model on the fixed engine, keeping the old
            # engine's evaluation windows intact for an honest comparison.
            evaluate(index-1)
            stage('generation', index, substage='fitting_audited_collection')
            print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
            folder = root/'generation'/f'gen{index}'
            atomic_json(folder/'replay_selection.json', record)
            from digimon_gym.agents.azero_trainer import train
            start = time.time()
            summary = train(init_from=root/'generation'/f'gen{index-1}'/'model.zip',
                shard_dirs=dirs, out=folder/'model.zip', epochs=2, batch_size=512,
                lr=.0001, value_coef=1., device='cpu', seed=plan['seed']+index,
                export_onnx=folder/'model.onnx')
            elapsed = time.time()-start
            atomic_json(folder/'summary.json', dict(generation=index, train=summary,
                window_dirs=[str(p) for p in dirs], recovery=recovery,
                timings_sec=dict(train=elapsed, selfplay=read(folder/'scheduler_progress.json')['elapsed_seconds_this_invocation'])))
            atomic_json(root/'generation/state.json', dict(next_gen=index+1,
                prev=dict(model_zip=str(folder/'model.zip'), model_onnx=str(folder/'model.onnx'))))
            evaluate(index)
            for index in range(index+1, plan['generations']+1):
                stage('generation', index)
                print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
                subprocess.run(generation_command(root, plan), check=True)
                evaluate(index)
            stage('complete', index)
            atomic_json(out/'status.json', dict(status='stopped', pipeline='complete'))
        except BaseException as error:
            stage('failed', index, error=str(error))
            atomic_json(out/'status.json', dict(status='stopped', pipeline='failed',
                last_result=dict(status='failed', error=str(error))))
            raise


if __name__ == '__main__':
    main()
