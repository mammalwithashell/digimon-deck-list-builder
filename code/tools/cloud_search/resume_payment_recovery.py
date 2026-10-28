"""One-shot repaired-engine CP1 baseline, then fresh-replay generations 2--4.

Despite the name, this never adopts partial output. Any interruption requires a
new explicit recovery decision; neither training nor model promotion restarts.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from tools.cloud_search.card_eval_worker import evaluate_window
from tools.cloud_search.evaluate_checkpoint_transition import INCOMPLETE, expected_cohort
from tools.cloud_search.prepare_payment_recovery import (
    ANCHOR_FILES, FIT, PANELS, SEARCH, anchors_for_root, pinned_path, read,
)
from tools.cloud_search.run_small_generations import evaluation_options, generation_command
from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def validate_frozen(root, plan, source):
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image'] or plan['image'] == plan['source_image']:
        raise ValueError('Runtime image differs from the repaired engine')
    required = {'inputs/decks.json', 'inputs/anchors.json', 'inputs/seed.zip', 'inputs/development.zip',
                'inputs/seed.onnx', 'inputs/seed.onnx.meta.json', 'validation/checkpoint1-recovery.json',
                'lineage/source/campaign-plan.json', 'lineage/source/experiment_state.json',
                'lineage/source/inputs/anchors.json', 'corpora/pilot/provenance.json',
                *[f'generation/gen1/model{s}' for s in ('.zip', '.onnx', '.onnx.meta.json')],
                *[f'inputs/{filename}' for _, filename in PANELS]}
    if not required <= plan['files'].keys():
        raise ValueError('Missing hash-bound recovery artifacts')
    for name, digest in plan['files'].items():
        if sha256(pinned_path(root, name)) != digest:
            raise ValueError(f'Recovery input hash mismatch: {name}')
    for name, digest in plan['source_files'].items():
        if sha256(pinned_path(source, name)) != digest:
            raise ValueError(f'Archived source hash mismatch: {name}')
    for name, digest in plan['runtime_files'].items():
        if sha256(Path(name)) != digest:
            raise ValueError(f'Runtime hash mismatch: {name}')
    if (plan['source_files'].get('campaign-plan.json') != plan['source_plan_sha256'] or
        plan['files']['lineage/source/campaign-plan.json'] != plan['source_plan_sha256']):
        raise ValueError('Source plan binding differs')
    original = read(root / 'lineage/source/campaign-plan.json')
    for name, digest in original['files'].items():
        if name.startswith('inputs/') and name != 'inputs/anchors.json' and plan['files'].get(name) != digest:
            raise ValueError(f'Original frozen cohort or anchor substituted: {name}')
    if read(root / 'inputs/anchors.json') != anchors_for_root(
        read(root / 'lineage/source/inputs/anchors.json'), root
    ) or plan['anchor_model_hashes'] != {
        name: plan['files'][f'inputs/{filename}'] for name, filename in ANCHOR_FILES.items()
    }:
        raise ValueError('Original frozen anchor identities differ')
    evidence = read(root / 'validation/checkpoint1-recovery.json')
    health = evidence.get('native_health', [])
    if (evidence.get('status') != 'passed' or evidence.get('image') != plan['image']
        or evidence.get('checkpoint_sha256') != plan['parent_checkpoint_sha256']
        or evidence.get('contract') != plan['model_contract']
        or evidence.get('cards_sha256') != plan['runtime_cards_sha256']
        or plan['files']['generation/gen1/model.zip'] != plan['parent_checkpoint_sha256']
        or [h['start'] for h in health] != [0, 4]
        or any(h['games'] != 4 or h['truncated_games'] or h['rows'] <= 0 for h in health)):
        raise ValueError('Eight-game CP1 health evidence differs')


def validate(root, plan, source):
    expected = dict(schema='fitted-checkpoint-payment-recovery-v1', baseline_checkpoint=1,
        first_generation=2, final_generation=4, remaining_generations=3, generations=4,
        games=1100, workers=8, seed=20261027, replay_window=3, replay_cap_games=1100,
        fresh_replay=True, parent_replay_games=0, evaluation_protocol='archetype-paired-v2',
        evaluation_panels=PANELS, evaluation_seed=22261027,
        evaluation_games_per_archetype_per_anchor=200, games_per_evaluation=10800,
        automatic_restart=False, automatic_promotion=False, search=SEARCH, fit=FIT)
    if any(plan.get(key) != value for key, value in expected.items()):
        raise ValueError('Recovery scope differs from CP1 baseline plus generations 2--4')
    validate_frozen(root, plan, source)
    gen1 = root / 'generation/gen1'
    expected_state = dict(next_gen=2, prev=dict(model_zip=str(gen1 / 'model.zip'),
                                             model_onnx=str(gen1 / 'model.onnx')))
    if read(root / 'generation/state.json') != expected_state:
        raise ValueError('Expected fresh next_gen=2 state warm-starting fitted CP1')
    if {p.name for p in gen1.iterdir()} != {'model.zip', 'model.onnx', 'model.onnx.meta.json'}:
        raise ValueError('Inherited CP1 must contain weights only, never old replay')
    if any(p.name != 'gen1' for p in (root / 'generation').glob('gen*')) or (root / 'campaign-started.json').exists():
        raise ValueError('Existing generation output; partial recovery cannot be adopted')
    evaluation_options(plan)


def validate_evaluation(folder, root, plan, index):
    """A process exit is insufficient: verify every pinned seat/deck/anchor row."""
    reports = [read(path) for path in (folder / 'reports').glob('checkpoint_*.json')]
    if len(reports) != len(PANELS):
        raise ValueError('Incomplete checkpoint evaluation panels')
    by_panel = {r.get('provenance', {}).get('panel'): r for r in reports}
    if set(by_panel) != {name for name, _ in PANELS}:
        raise ValueError('Duplicate or unexpected evaluation panel')
    digest = sha256(root / f'generation/gen{index}/model.zip')
    for panel, filename in PANELS:
        report = by_panel[panel]
        _, cohort = expected_cohort(read(root / 'inputs' / filename), plan['evaluation_seed'], 200)
        expected_games = {(r['anchor'], r['seed']): dict(r, panel=panel) for r in cohort}
        provenance = report.get('provenance', {})
        expected = dict(status='complete', engine_image=plan['image'], cohort_scheme=plan['evaluation_protocol'],
            evaluation_mode='policy_only', panel=panel, checkpoint_index=index,
            candidate_sha256=digest, checkpoint_sha256=digest, base_seed=plan['evaluation_seed'],
            games_per_archetype=200, requested_games_per_anchor=len(cohort) // 3,
            snapshot_sha256=plan['files'][f'inputs/{filename}'],
            anchors_sha256=plan['files']['inputs/anchors.json'], anchor_model_hashes=plan['anchor_model_hashes'],
            deterministic=True, runtime_cards_sha256=plan['runtime_cards_sha256'], **plan['model_contract'])
        if (report.get('schema_version') != 1 or report.get('checkpoint_step') != index
            or any(provenance.get(k) != v for k, v in expected.items())
            or provenance.get('card_eval_event_version', 0) < 2
            or any(not isinstance(report.get(k), list) for k in ('cards', 'routes', 'route_decisions', 'effect_sequences'))):
            raise ValueError('Evaluation provenance differs from the repaired-engine contract')
        games, seen = report.get('games', []), set()
        if len(games) != len(expected_games):
            raise ValueError('Incomplete evaluation cohort')
        for game in games:
            context = game.get('context', {})
            key = (context.get('anchor'), game.get('seed'))
            if key in seen or key not in expected_games or type(game.get('seed')) is not int:
                raise ValueError('Duplicate or unexpected evaluation seed/seat')
            seen.add(key)
            if any(context.get(k) != v for k, v in expected_games[key].items() if k not in ('seed', 'opening_player')):
                raise ValueError('Evaluation cohort deck or anchor differs')
            winner = game.get('winner_id')
            if (not game.get('terminal_reason') or game['terminal_reason'] in INCOMPLETE
                or game.get('evaluation_error') or 'winner_id' not in game
                or not (winner is None or type(winner) is int and winner in (0, 1, 2))):
                raise ValueError('Incomplete or invalid terminal evaluation game')


def run(root, out, source, plan):
    if out == root or out == source or out.is_relative_to(source) or source.is_relative_to(out):
        raise ValueError('Evaluation output must not overlap the source archive')
    if out.exists() and (not out.is_dir() or any(out.iterdir())):
        raise ValueError('Evaluation output must be empty; existing reports cannot be adopted')
    with controller_lock(root / 'campaign.lock'):
        validate(root, plan, source)
        with (root / 'campaign-started.json').open('x', encoding='utf-8') as marker:
            json.dump(dict(time_unix=time.time(), plan=plan), marker, indent=2)
        out.mkdir(parents=True, exist_ok=True)
        index = 1
        def stage(name, **extra):
            atomic_json(root / 'experiment_state.json', dict(stage=name, generation_index=index,
                time_unix=time.time(), image=plan['image'], automatic_restart=False,
                automatic_promotion=False, **extra))
        def evaluate():
            validate_frozen(root, plan, source)
            checkpoint = dict(path=str(root / f'generation/gen{index}/model.zip'),
                              sha256=sha256(root / f'generation/gen{index}/model.zip'), step=index)
            stage('checkpoint_evaluation', repaired_baseline=index == 1)
            atomic_json(out / 'status.json', dict(status='running', checkpoint=checkpoint, window=index))
            options = evaluation_options(plan)
            result = evaluate_window(root, out, checkpoint, index,
                'repaired_engine_baseline' if index == 1 else 'new_checkpoint', options)
            if result['status'] != 'complete':
                raise RuntimeError(f'Checkpoint evaluation failed: {result}')
            validate_evaluation(out / f'window-{index:06d}', root, plan, index)
            for panel, _ in options.panels:
                shutil.copyfile(out / f'window-{index:06d}/{panel}.log',
                                root / f'generation/gen{index}/{panel}.txt')
            atomic_json(out / 'status.json', dict(result, worker_status='waiting'))
        try:
            evaluate()
            for index in range(plan['first_generation'], plan['final_generation'] + 1):
                validate_frozen(root, plan, source)
                stage('generation', games=plan['games'], workers=plan['workers'])
                print(f'=== GENERATION_MONITOR_INDEX={index} ===', flush=True)
                subprocess.run(generation_command(root, plan), check=True)
                expected_state = dict(next_gen=index + 1, prev=dict(
                    model_zip=str(root / f'generation/gen{index}/model.zip'),
                    model_onnx=str(root / f'generation/gen{index}/model.onnx')))
                if read(root / 'generation/state.json') != expected_state:
                    raise ValueError('Generation subprocess did not complete the expected logical update')
                evaluate()
            stage('complete')
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='complete'))
        except BaseException as error:
            stage('failed', error=str(error))
            atomic_json(out / 'status.json', dict(status='stopped', pipeline='failed',
                        last_result=dict(status='failed', error=str(error))))
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--evaluation-root', type=Path)
    parser.add_argument('--validate-only', action='store_true')
    args = parser.parse_args()
    root, source = args.root.resolve(), args.source.resolve()
    plan = read(root / 'campaign-plan.json')
    if read(root / 'deployment-plan.json') != plan:
        raise ValueError('Saved campaign and deployment plans differ')
    validate(root, plan, source)
    if args.validate_only:
        print('Fitted CP1 recovery inputs and runtime verified', flush=True)
        return
    run(root, (args.evaluation_root or root / 'evaluations').resolve(), source, plan)


if __name__ == '__main__':
    main()
