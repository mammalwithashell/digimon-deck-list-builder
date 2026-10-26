"""Qualify a fresh four-update campaign from the winning frozen mixed model."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from tools.selfplay_scheduler import _read_commit, sha256


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--duel', type=Path, default=Path('/duel'))
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    args = parser.parse_args()
    root, source = args.root, args.source
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    if (root / 'campaign-plan.json').exists() or (root / 'campaign-started.json').exists():
        raise ValueError('Refusing to overwrite a prepared or started campaign')
    if read(source / 'comparison_state.json')['stage'] != 'complete':
        raise ValueError('Parent comparison is incomplete')
    duel = read(args.duel / 'head_to_head_result.json')
    duel_plan = read(args.duel / 'plan.json')
    if duel['status'] != 'complete' or duel['perspective'] != 'mixed' or duel['ci95'][0] <= .5:
        raise ValueError('Expected completed direct evidence for the selected mixed checkpoint')
    from sb3_contrib import MaskablePPO
    import digimon_engine
    from model_contract import validate_checkpoint, validate_contract
    from digimon_gym.agents.gauntlet import GeneralistDeckPool, validate_admissible_deck
    from digimon_gym.agents.azero_trainer import load_shards

    parent_source = source / 'arms/mixed/generation/gen1'
    inputs = root / 'inputs'
    inputs.mkdir(parents=True, exist_ok=True)
    parent = root / 'generation/gen0'
    parent.mkdir(parents=True, exist_ok=True)
    contract = validate_checkpoint(MaskablePPO.load(parent_source / 'model.zip', device='cpu'), 'standard_lite_v2')
    validate_contract(read(parent_source / 'model.onnx.meta.json'), 'standard_lite_v2')
    if read(source / 'evaluations/mixed/window-000001/context.json')['engine_image'] != image:
        raise ValueError('Native image must match the validated parent')
    for arm in ('mixed', 'development'):
        checkpoint = source / 'arms' / arm / 'generation/gen1/model.zip'
        if sha256(checkpoint) != duel_plan['model_hashes'][arm]:
            raise ValueError('Checkpoint differs from head-to-head evidence')
        if validate_checkpoint(MaskablePPO.load(checkpoint, device='cpu')) != contract:
            raise ValueError('Anchor contract mismatch')
    provenance = read(source / 'corpora/mixed/provenance.json')
    if sha256(Path('/app/data/cards.json')) != provenance['cards_sha256']:
        raise ValueError('Card data differs from parent experiment')
    registered = set(digimon_engine.load_implemented_card_ids())
    for name in ('decks.json', 'train_snapshot.json', 'heldout_snapshot.json', 'pet_evaluation_snapshot.json'):
        shutil.copy2(source / 'inputs' / name, inputs / name)
    decks = read(inputs / 'decks.json')
    for deck in decks:
        validate_admissible_deck(deck['cards'], label=deck['name'])
        if not set(deck['cards']) <= registered or not digimon_engine.validate_deck(deck['cards']).is_valid:
            raise ValueError('Training deck failed native admission')
    pools = [GeneralistDeckPool.from_snapshot(inputs / name, implemented_card_ids=registered)
             for name in ('train_snapshot.json', 'heldout_snapshot.json', 'pet_evaluation_snapshot.json')]
    if set(pools[1].archetypes) & set(pools[2].archetypes):
        raise ValueError('Meta and pet cohorts overlap')
    combined = GeneralistDeckPool({**pools[1].archetypes, **pools[2].archetypes})
    if len(combined.archetypes) != 13 or set(combined.archetypes) != set(pools[0].archetypes):
        raise ValueError('Expected the unchanged 13-archetype pool')
    combined.write_snapshot(inputs / 'combined_snapshot.json')
    for suffix in ('.zip', '.onnx', '.onnx.meta.json'):
        shutil.copy2(parent_source / f'model{suffix}', inputs / f'seed{suffix}')
        shutil.copy2(parent_source / f'model{suffix}', parent / f'model{suffix}')
        if sha256(parent / f'model{suffix}') != sha256(parent_source / f'model{suffix}'):
            raise ValueError('Parent model changed during snapshot')
    shutil.copy2(source / 'arms/development/generation/gen1/model.zip', inputs / 'development.zip')
    anchors = [dict(name=name, weights_path=str(inputs / filename), algorithm='mlp',
                    observation_profile=contract['observation_profile'], tensor_layout_hash=contract['tensor_layout_hash'],
                    source='frozen pre-continuation checkpoint 1')
               for name,filename in [('mixed-start','seed.zip'), ('development-fixed','development.zip')]]
    write(inputs / 'anchors.json', dict(version=1, champions=anchors))
    # Verify every retained native chunk, not just the aggregate progress file.
    schedule = read(parent_source / 'schedule.json')
    progress = read(parent_source / 'scheduler_progress.json')
    if progress['status'] != 'complete' or progress['truncated_games'] or schedule['games'] != 1100:
        raise ValueError('Parent search replay is not a completed 1100-game generation')
    chunks, commits = [], []
    for start in range(0, schedule['games'], schedule['chunk_games']):
        folder = parent_source / 'shards' / f'proc{start:08d}'
        count = min(schedule['chunk_games'], schedule['games'] - start)
        commit = _read_commit(folder, start, count, schedule['seed'])
        if commit['truncated_games'] or (folder / 'training_exclusions.json').exists():
            raise ValueError('Parent replay must contain only complete, non-quarantined games')
        chunks.append(folder)
        commits.append(dict(directory=str(folder), commit_sha256=sha256(folder / 'chunk_commit.json')))
    if set(chunks) != set((parent_source / 'shards').glob('proc*')):
        raise ValueError('Unexpected parent replay chunks')
    replay = load_shards(chunks)
    if not replay.terminated.all():
        raise ValueError('Parent replay contains incomplete value targets')
    replay_rows = len(replay.z)
    del replay
    (parent / 'shards').symlink_to(parent_source / 'shards', target_is_directory=True)
    write(root / 'generation/state.json', dict(next_gen=1, prev=dict(model_zip=str(parent / 'model.zip'),
                                                                   model_onnx=str(parent / 'model.onnx'))))
    write(root / 'corpora/pilot/provenance.json', provenance)
    validation = root / 'validation'
    validation.mkdir(exist_ok=True)
    evidence = dict(status='running', image=image, checkpoint_sha256=sha256(parent / 'model.zip'),
                    contract=contract, parent_replay_games=1100, parent_replay_rows=replay_rows,
                    source_commits=commits, cards_sha256=provenance['cards_sha256'])
    write(validation / 'generation0-recovery.json', evidence)
    # Confirm the selected exported model can drive unchanged full-budget search.
    def health(start):
        out = validation / f'health-{start}'
        command = ['digimon-engine-cli', '--pool', 'implemented', '--cards-json', '/app/data/cards.json',
                   'selfplay', '--model', str(parent / 'model.onnx'), '--decks', str(inputs / 'decks.json'),
                   '--games', '4', '--game-start', str(start), '--seed', '20261026', '--sims', '320',
                   '--worlds', '4', '--eval-batch-size', '4', '--temperature-decisions', '16',
                   '--decision-cap', '1200', '--tensor-profile', 'standard_lite_v2', '--out', str(out)]
        with (validation / f'health-{start}.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
        manifest = read(out / 'manifest.json')
        if manifest['games'] != 4 or manifest['truncated_games']:
            raise ValueError('Native continuation health check failed')
        return dict(start=start, games=4, rows=manifest['rows'], truncated_games=0)
    try:
        with ThreadPoolExecutor(max_workers=2) as executor:
            evidence['native_health'] = list(executor.map(health, (0,4)))
        evidence.update(status='passed', finished_unix=time.time())
    except BaseException as error:
        evidence.update(status='failed', error=str(error))
        raise
    finally:
        write(validation / 'generation0-recovery.json', evidence)
    runtime_paths = [Path('/app/code') / name for name in (
        'tools/cloud_search/run_small_generations.py', 'tools/cloud_search/prepare_mixed_continuation.py',
        'tools/cloud_search/card_eval_worker.py', 'tools/cloud_search/replay_window.py',
        'tools/run_selfplay_generation.py', 'digimon_gym/agents/azero_trainer.py',
        'digimon_gym/agents/anchored_eval.py', 'tools/anchored_eval_cli.py', 'tools/cloud_search/eval_progress.py')]
    files = {p.relative_to(root).as_posix(): sha256(p) for directory in (inputs,parent,validation)
             for p in directory.iterdir() if p.is_file()}
    plan = dict(generations=4, games=1100, workers=8, seed=10261026, image=image, files=files,
                runtime_files={str(p):sha256(p) for p in runtime_paths},
                controller_commit=os.environ['DIGIMON_CONTROLLER_COMMIT'],
                parent_run='english-meta-seed-mixed-20261025/checkpoint-1', replay_window=3,
                replay_cap_games=1100, expected_appearances_per_archetype=2200/13,
                evaluation_panels=[['combined_validation','combined_snapshot.json']],
                evaluation_seed=12261026, evaluation_games_per_archetype_per_anchor=200,
                games_per_evaluation=7800, automatic_restart=False, automatic_promotion=False,
                meta_archetypes=pools[1].archetype_names, pet_archetypes=pools[2].archetype_names)
    write(root / 'campaign-plan.json', plan)
    write(root / 'deployment-plan.json', plan)
    write(root / 'experiment_state.json', dict(stage='prepared', generation_index=0))
    from tools.cloud_search.run_small_generations import validate
    validate(root, plan)
    print(json.dumps(dict(status='prepared', plan=plan, native_health=evidence['native_health'])), flush=True)


if __name__ == '__main__':
    main()
