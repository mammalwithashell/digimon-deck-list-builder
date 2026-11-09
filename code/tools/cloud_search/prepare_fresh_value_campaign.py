"""Prepare the fresh value-fixed EX13 campaign (2026-11-08 plan). Never trains.

Spec: docs/superpowers/plans/2026-11-07-ex13-fresh-value-fixed-run.md.

Fresh weights from a ``mixed_v2`` heuristic bootstrap (greedy/development,
Tamer-first on Light Hybrid seats, value-only random seats; tanh value head),
the old fresh campaign's frozen deck pool, snapshots and anchors, and a
hash-pinned opponent pool that phases CP8 in:

  gens 1-2  page-search CP2 (weight 1), self-play 0.5   -- the old recipe
  gens 3-4  + CP8 (weight 2), history window 2, self-play 0.6
  gens 5+   same pool, self-play 0.5

Run inside the campaign image with the root mounted at /experiment and the
copied old inputs at /experiment/inputs-from-fresh-history-20261028.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from tools.selfplay_scheduler import atomic_json, sha256

OLD = 'inputs-from-fresh-history-20261028'
POOL_FILES = ('train_snapshot.json', 'heldout_snapshot.json', 'pet_regression_snapshot.json',
              'combined_diagnostic_snapshot.json', 'supplemental_snapshot.json', 'decks.json',
              'distinct_training_decks.json', 'pool_report.json', 'source_candidates.json',
              'source_extras.json', 'source_baseline_train_snapshot.json', 'README.md',
              'pet_evaluation_snapshot.json', 'development.zip', 'anchor-parent-checkpoint4.zip',
              'anchors.json')
CP8_SHA = '4a026e27b7af5e88be35ace34819b847c035a87fe80e55589c33ef8dc99d5963'


def bootstrap(root: Path, seed: int, random_fraction: float):
    work = root / 'bootstrap'
    code = [sys.executable, '-m', 'tools.cloud_search.mixed_bootstrap']
    if not (work / 'qualification.json').exists():
        subprocess.run(code + ['collect', '--snapshot', str(root / 'inputs/train_snapshot.json'),
                               '--output', str(work), '--arm', 'mixed_v2', '--seed', str(seed),
                               '--rounds', '32', '--tamer-first-archetype', 'Light Hybrid',
                               '--random-fraction', str(random_fraction)], check=True)
    if not (work / 'seed.onnx').exists():
        subprocess.run(code + ['fit', '--output', str(work), '--seed', str(seed)], check=True)
    return work


def health(root: Path, image: str) -> dict:
    """Short native self-play with the gen-3+ pool: proves the seed, CP2 and
    CP8 all load and play on this engine before any long run starts."""
    from tools.run_selfplay_generation import run_selfplay_stage
    folder = root / 'validation/health-cp8-pool'
    started = time.monotonic()
    dirs = run_selfplay_stage(cli_bin='/usr/local/bin/digimon-engine-cli', gen_dir=folder,
        model_onnx=str(root / 'generation/gen0/model.onnx'), uniform=False,
        decks=root / 'inputs/decks.json', games=8, procs=4, sims=64, worlds=4, mode='pimc',
        seed_base=7, temperature_decisions=16, decision_cap=1200, shard_rows=4096,
        cards_json=Path('/app/data/cards.json'), pool='implemented', eval_batch_size=4,
        scheduling='dynamic', chunk_games=2, opponent_pool=root / 'inputs/opponent_pool_with_cp8.json')
    manifests = [json.loads((d / 'manifest.json').read_text()) for d in dirs]
    games = sum(m['games'] for m in manifests)
    truncated = sum(m['truncated_games'] for m in manifests)
    opponents = sorted({r['opponent_id'] for m in manifests for r in m.get('opponent_games', [])})
    if games != 8 or truncated:
        raise RuntimeError(f'Health check failed: {games} games, {truncated} truncated')
    return dict(games=games, truncated_games=truncated, opponents_seen=opponents,
                seconds=round(time.monotonic() - started), image=image)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--seed', type=int, default=80261108)
    parser.add_argument('--random-fraction', type=float, default=0.175)
    parser.add_argument('--generations', type=int, default=12)
    parser.add_argument('--source-commit', required=True)
    args = parser.parse_args()
    root = args.root
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    from tools.search_opponents import SCHEMA, checkpoint_entry, validate_pool

    inputs = root / 'inputs'
    inputs.mkdir(exist_ok=True)
    for name in POOL_FILES:
        if not (inputs / name).exists():
            shutil.copy2(root / OLD / name, inputs / name)
    opponents = inputs / 'opponents'
    opponents.mkdir(exist_ok=True)
    for name in ('checkpoint2.onnx', 'checkpoint2.onnx.meta.json'):
        if not (opponents / name).exists():
            shutil.copy2(root / OLD / 'opponents' / name, opponents / name)
    if sha256(opponents / 'ex13-lineage-checkpoint8.onnx') != CP8_SHA:
        raise ValueError('CP8 opponent must be the lineage checkpoint 8 export')

    work = bootstrap(root, args.seed, args.random_fraction)
    gen0 = root / 'generation/gen0'
    gen0.mkdir(parents=True, exist_ok=True)
    for suffix in ('.zip', '.onnx', '.onnx.meta.json', '.calibration.json'):
        shutil.copy2(work / f'seed{suffix}', inputs / f'seed{suffix}')
        shutil.copy2(work / f'seed{suffix}', gen0 / f'model{suffix}')
    atomic_json(root / 'generation/state.json', dict(next_gen=1, prev=dict(
        model_zip=str(gen0 / 'model.zip'), model_onnx=str(gen0 / 'model.onnx'))))
    from digimon_gym.agents.belief_contract import bind_prepared_run
    bind_prepared_run(root / 'generation')  # the driver launches open-decklist generations

    cp2 = checkpoint_entry('ex13-page-checkpoint2', opponents / 'checkpoint2.onnx')
    cp8 = checkpoint_entry('ex13-lineage-checkpoint8', opponents / 'ex13-lineage-checkpoint8.onnx', weight=2.0)
    for name, entries in (('opponent_pool.json', [cp2]), ('opponent_pool_with_cp8.json', [cp2, cp8])):
        pool = dict(schema=SCHEMA, selfplay_probability=0.5,
                    opponents=[dict(e, path=Path(e['path']).relative_to(inputs).as_posix()) for e in entries])
        validate_pool(pool, inputs, gen0 / 'model.onnx')
        atomic_json(inputs / name, pool)

    (root / 'validation').mkdir(exist_ok=True)
    evidence = dict(status='running', image=image, checkpoint_sha256=sha256(gen0 / 'model.zip'),
                    initialization='fresh_mixed_v2_bootstrap', source_commit=args.source_commit)
    try:
        evidence['health'] = health(root, image)
        evidence['bootstrap'] = json.loads((work / 'imitation_summary.json').read_text()).get('rows')
        evidence['status'] = 'passed'
    except BaseException as error:
        evidence.update(status='failed', error=repr(error))
        raise
    finally:
        atomic_json(root / 'validation/generation0-recovery.json', evidence)

    files = {p.relative_to(root).as_posix(): sha256(p)
             for directory in (inputs, gen0, root / 'validation')
             for p in directory.rglob('*') if p.is_file()}
    early = dict(pool='inputs/opponent_pool.json', history_window=0, selfplay_probability=0.5)
    phase_in = dict(pool='inputs/opponent_pool_with_cp8.json', history_window=2, selfplay_probability=0.6)
    steady = dict(phase_in, selfplay_probability=0.5)
    overrides = {str(g): (early if g <= 2 else phase_in if g <= 4 else steady)
                 for g in range(1, args.generations + 1)}
    plan = dict(schema='fresh-value-campaign-v1', image=image, controller_commit=args.source_commit,
                spec='docs/superpowers/plans/2026-11-07-ex13-fresh-value-fixed-run.md',
                first_generation=1, generations=args.generations, long_campaign=True,
                games=1300, workers=8, seed=args.seed, files=files, runtime_files={},
                training=dict(epochs=4, lr=0.0001, batch_size=512, value_holdout_fraction=0.2,
                              value_activation='tanh'),
                historical_opponents_by_generation=overrides,
                cp8_share_of_games={'1-2': 0.0, '3-4': '0.4 x 2/5 = 16% (20% at gen 3, one history entry)',
                                    '5+': '0.5 x 2/5 = 20%'},
                evaluation_panels=[['combined_diagnostic', 'combined_diagnostic_snapshot.json'],
                                   ['original_pet_regression', 'pet_evaluation_snapshot.json']],
                evaluation_games_per_archetype_per_anchor=200, evaluation_seed=22261027,
                search=dict(simulations=320, worlds=4, evaluation_batch_size=4, reduced_simulations=64,
                            full_search_fraction=0.25, cheap_simulations=64),
                replay_window=3, fresh_replay=True, automatic_restart=False, automatic_promotion=False,
                stop_rules='CP8 search-vs-search check every 2 generations from gen 4; two checks in a row '
                           'below 47% stop the run; held-out value MSE must beat the constant baseline')
    from tools.cloud_search.run_small_generations import validate
    validate(root, plan)
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        atomic_json(root / name, plan)
    atomic_json(root / 'experiment_state.json', dict(stage='prepared', generation_index=0, image=image))
    print(json.dumps(dict(status='prepared', generations=args.generations, health=evidence['health'])), flush=True)


if __name__ == '__main__':
    main()
