"""Prepare a fresh-replay continuation from a completed, hash-bound checkpoint.

Run inside the pinned training image with the old campaign mounted read-only
at /archive and a new, empty campaign mounted at /experiment. This prepares
and validates inputs; it never starts the campaign or copies old replay.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from tools.selfplay_scheduler import sha256


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--parent-generation', type=int, required=True)
    parser.add_argument('--parent-sha256', required=True)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--ci-run', required=True)
    parser.add_argument('--seed', type=int, required=True)
    parser.add_argument('--evaluation-seed', type=int, required=True)
    args = parser.parse_args()
    source, root = args.source, args.root
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    if '@sha256:' not in image:
        raise ValueError('An immutable image is required')
    if any((root / name).exists() for name in ('inputs', 'generation', 'campaign-plan.json')):
        raise ValueError('Refusing to overwrite an existing preparation')
    state = read(source / 'experiment_state.json')
    if state['stage'] != 'complete' or state['generation_index'] != args.parent_generation:
        raise ValueError('Expected the final checkpoint of a completed campaign')
    parent_source = source / f'generation/gen{args.parent_generation}'
    if sha256(parent_source / 'model.zip') != args.parent_sha256:
        raise ValueError('Parent checkpoint differs from approved evidence')
    original = read(source / 'campaign-plan.json')
    from sb3_contrib import MaskablePPO
    import digimon_engine
    from model_contract import validate_checkpoint, validate_contract
    from digimon_gym.agents.gauntlet import (
        GeneralistDeckPool, require_mechanics_ready, validate_admissible_deck,
    )
    require_mechanics_ready()
    if digimon_engine.CARD_EVAL_EVENT_VERSION < 2:
        raise ValueError('Decision telemetry is required')
    contract = validate_checkpoint(MaskablePPO.load(parent_source / 'model.zip', device='cpu'), 'standard_lite_v2')
    validate_contract(read(parent_source / 'model.onnx.meta.json'), 'standard_lite_v2')
    provenance = read(source / 'corpora/pilot/provenance.json')
    if sha256(Path('/app/data/cards.json')) != provenance['cards_sha256']:
        raise ValueError('Card data changed across the continuation')
    inputs, parent = root / 'inputs', root / 'generation/gen0'
    inputs.mkdir(parents=True)
    parent.mkdir(parents=True)
    for name in ('decks.json', 'train_snapshot.json', 'heldout_snapshot.json',
                 'pet_evaluation_snapshot.json', 'combined_snapshot.json', 'development.zip'):
        if sha256(source / 'inputs' / name) != original['files'][f'inputs/{name}']:
            raise ValueError(f'Frozen input differs: {name}')
        shutil.copy2(source / 'inputs' / name, inputs / name)
    registered = set(digimon_engine.load_implemented_card_ids())
    decks = read(inputs / 'decks.json')
    for deck in decks:
        validate_admissible_deck(deck['cards'], label=deck['name'])
        if not set(deck['cards']) <= registered or not digimon_engine.validate_deck(deck['cards']).is_valid:
            raise ValueError(f'Deck failed native admission: {deck["name"]}')
    pools = [GeneralistDeckPool.from_snapshot(inputs / name, implemented_card_ids=registered)
             for name in ('train_snapshot.json', 'combined_snapshot.json')]
    if len(pools[0].archetypes) != 13 or set(pools[0].archetypes) != set(pools[1].archetypes):
        raise ValueError('Expected unchanged 13-archetype training and evaluation cohorts')
    if validate_checkpoint(MaskablePPO.load(inputs / 'development.zip', device='cpu')) != contract:
        raise ValueError('Development anchor contract mismatch')
    for suffix in ('.zip', '.onnx', '.onnx.meta.json'):
        for target in (inputs / f'seed{suffix}', parent / f'model{suffix}'):
            shutil.copy2(parent_source / f'model{suffix}', target)
            if sha256(target) != sha256(parent_source / f'model{suffix}'):
                raise ValueError('Checkpoint changed during snapshot')
    # gen0 contains weights only. bounded_window skips the absent shards folder.
    write(root / 'generation/state.json', dict(next_gen=1, prev=dict(
        model_zip=str(parent / 'model.zip'), model_onnx=str(parent / 'model.onnx'))))
    anchors = [dict(name=name, weights_path=str(inputs / filename), algorithm='mlp',
                    observation_profile=contract['observation_profile'], tensor_layout_hash=contract['tensor_layout_hash'],
                    source=description)
               for name, filename, description in (
                   ('parent-checkpoint4', 'seed.zip', 'Frozen pre-continuation checkpoint 4; not promoted'),
                   ('development-fixed', 'development.zip', 'Original frozen development anchor'))]
    write(inputs / 'anchors.json', dict(version=1, champions=anchors))
    write(root / 'corpora/pilot/provenance.json', provenance)
    validation = root / 'validation'
    validation.mkdir()
    evidence = dict(status='running', image=image, source_commit=args.source_commit,
        checkpoint_sha256=args.parent_sha256, contract=contract, fresh_replay=True,
        parent_replay_games=0, validated_deck_slots=len(decks), cards_sha256=provenance['cards_sha256'],
        card_eval_event_version=digimon_engine.CARD_EVAL_EVENT_VERSION, started_unix=time.time())

    def health(start):
        out = validation / f'health-{start}'
        command = ['digimon-engine-cli', '--pool', 'implemented', '--cards-json', '/app/data/cards.json',
                   'selfplay', '--model', str(parent / 'model.onnx'), '--decks', str(inputs / 'decks.json'),
                   '--games', '4', '--game-start', str(start), '--seed', str(args.seed + 10000000),
                   '--sims', '320', '--worlds', '4', '--eval-batch-size', '4',
                   '--temperature-decisions', '16', '--decision-cap', '1200',
                   '--tensor-profile', 'standard_lite_v2', '--out', str(out)]
        with (validation / f'health-{start}.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
        manifest = read(out / 'manifest.json')
        if manifest['games'] != 4 or manifest['truncated_games']:
            raise ValueError('Native full-budget health check failed')
        return dict(start=start, games=4, rows=manifest['rows'], truncated_games=0)

    try:
        with ThreadPoolExecutor(max_workers=2) as executor:
            evidence['native_health'] = list(executor.map(health, (0, 4)))
        evidence['status'] = 'passed'
    except BaseException as error:
        evidence.update(status='failed', error=str(error))
        raise
    finally:
        evidence['finished_unix'] = time.time()
        write(validation / 'generation0-recovery.json', evidence)
    runtime_paths = [Path('/app/code') / name for name in (
        'tools/cloud_search/run_small_generations.py', 'tools/cloud_search/card_eval_worker.py',
        'tools/cloud_search/replay_window.py', 'tools/run_selfplay_generation.py',
        'digimon_gym/agents/azero_trainer.py', 'digimon_gym/agents/anchored_eval.py',
        'digimon_gym/agents/card_eval.py', 'tools/anchored_eval_cli.py',
        'tools/cloud_search/eval_progress.py', 'tools/cloud_search/wandb_monitor.py')]
    files = {p.relative_to(root).as_posix(): sha256(p) for directory in (inputs, parent, validation)
             for p in directory.iterdir() if p.is_file()}
    plan = dict(generations=4, games=1100, workers=8, seed=args.seed, image=image, files=files,
        runtime_files={str(p): sha256(p) for p in runtime_paths}, controller_commit=args.source_commit,
        ci_run=args.ci_run, preparation_sha256=sha256(Path(__file__)),
        parent_run='english-meta-mixed-4x1100-20261026', parent_checkpoint=args.parent_generation,
        parent_checkpoint_sha256=args.parent_sha256, fresh_replay=True, replay_window=3,
        replay_cap_games=1100, expected_appearances_per_archetype=2200 / 13,
        evaluation_panels=[['combined_validation', 'combined_snapshot.json']],
        evaluation_seed=args.evaluation_seed, evaluation_games_per_archetype_per_anchor=200,
        games_per_evaluation=7800, automatic_restart=False, automatic_promotion=False,
        meta_archetypes=original['meta_archetypes'], pet_archetypes=original['pet_archetypes'],
        search=dict(simulations=320, worlds=4, evaluation_batch_size=4, decision_cap=1200))
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        write(root / name, plan)
    write(root / 'experiment_state.json', dict(stage='prepared', generation_index=0))
    from tools.cloud_search.run_small_generations import validate
    validate(root, plan)
    print(json.dumps(dict(status='prepared', evidence=evidence)), flush=True)


if __name__ == '__main__':
    main()
