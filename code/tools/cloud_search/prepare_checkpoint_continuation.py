"""Prepare a new EX13 campaign from an assessed saved checkpoint, never replay.

The parent may have failed its final evaluation: its state and failed windows
remain untouched. Mount it read-only at /archive and provide a separate complete
transition assessment made on this image. This command prepares and validates;
it neither starts training nor promotes a model.
"""
from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import json
import math
import os
from pathlib import Path
import re
import shutil
import subprocess
import time

from tools.selfplay_scheduler import atomic_json, sha256
from tools.cloud_search.evaluate_checkpoint_transition import validate_assessment

PROFILE = 'standard_lite_v2'
PROTOCOL = 'archetype-paired-v2'
SNAPSHOTS = ('train_snapshot.json', 'heldout_snapshot.json', 'pet_regression_snapshot.json',
             'combined_diagnostic_snapshot.json', 'supplemental_snapshot.json')
POOL_FILES = (*SNAPSHOTS, 'decks.json', 'distinct_training_decks.json', 'pool_report.json',
              'source_candidates.json', 'source_extras.json', 'source_baseline_train_snapshot.json', 'README.md')
RUNTIME_FILES = ('tools/cloud_search/run_small_generations.py', 'tools/cloud_search/card_eval_worker.py',
                 'tools/cloud_search/replay_window.py', 'tools/run_selfplay_generation.py',
                 'tools/selfplay_scheduler.py', 'digimon_gym/agents/azero_trainer.py',
                 'digimon_gym/agents/anchored_eval.py', 'digimon_gym/agents/card_eval.py',
                 'digimon_gym/agents/gauntlet.py', 'tools/anchored_eval_cli.py',
                 'tools/cloud_search/eval_progress.py', 'tools/cloud_search/wandb_monitor.py',
                 'tools/cloud_search/evaluate_checkpoint_transition.py',
                 'tools/cloud_search/run_seed_comparison.py',
                 'model_contract.py', 'tools/cloud_search/prepare_checkpoint_continuation.py')


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def contents(cards):
    return tuple(sorted(Counter(cards).items()))


def snapshot_rows(path):
    rows = []
    for deck in read(path)['decks']:
        cards = deck.get('card_ids') or [card for card, count in sorted(deck['card_counts'].items())
                                       for _ in range(count)]
        rows.append((deck['archetype'], contents(cards)))
    return rows


def summarize_pool(pool_dir, original_pets):
    """Keep distinct contents, weighting slots, and evaluation evidence separate."""
    rows = {name: snapshot_rows(pool_dir / name) for name in SNAPSHOTS}
    train, heldout, pets, combined = (set(rows[name]) for name in SNAPSHOTS[:4])
    if pets != set(snapshot_rows(original_pets)):
        raise ValueError('Prepared pool changed the original pet cohort')
    if train & heldout:
        raise ValueError('Held-out lists overlap refreshed training')
    if heldout & set(snapshot_rows(pool_dir / 'source_baseline_train_snapshot.json')):
        raise ValueError('Held-out lists overlap prior frozen training')
    native = read(pool_dir / 'decks.json')
    native_rows = [(row['name'].split(':sha256:', 1)[0], contents(row['cards'])) for row in native]
    if set(native_rows) != train:
        raise ValueError('Native sampling slots differ from the training snapshot')
    slots = Counter(arch for arch, _ in native_rows)
    if not slots or len(set(slots.values())) != 1:
        raise ValueError('Native pool must give each archetype equal sampling weight')
    same_list = sorted({arch for arch, _ in combined & train})
    if same_list != ['Light Hybrid'] or combined != heldout | {row for row in pets if row[0] == 'Light Hybrid'}:
        raise ValueError('Combined diagnostic panel lost its explicit Light Hybrid fallback')
    def counts(name):
        unique = set(rows[name])
        return dict(archetypes=len({arch for arch, _ in unique}), distinct_lists=len(unique),
                    lists_by_archetype=dict(sorted(Counter(arch for arch, _ in unique).items())))
    result = {label: counts(filename) for label, filename in (
        ('training', 'train_snapshot.json'), ('heldout', 'heldout_snapshot.json'),
        ('original_pet_regression', 'pet_regression_snapshot.json'),
        ('combined_diagnostic', 'combined_diagnostic_snapshot.json'),
        ('supplemental', 'supplemental_snapshot.json'))}
    result['training'].update(native_sampling_slots=len(native), slots_by_archetype=dict(sorted(slots.items())))
    result['heldout']['evidence'] = 'Unseen exact lists; competitive and lower-evidence tiers retained in pool_report.json'
    result['combined_diagnostic'].update(same_list_archetypes=same_list, all_heldout=False)
    result['original_pet_regression'].update(evidence='Five original exact pet lists; separate regression panel',
                                             overlap_with_combined_lists=len(pets & combined))
    result['supplemental']['evaluated_by_default'] = False
    return result


def validate_runtime(parent, development, pool_dir, cards_json):
    """Recheck native legality, committed admission, and actual embedded contracts."""
    import digimon_engine
    from sb3_contrib import MaskablePPO
    from model_contract import validate_checkpoint, validate_contract, validate_onnx_file
    from digimon_gym.agents.gauntlet import GeneralistDeckPool, require_mechanics_ready, validate_admissible_deck

    require_mechanics_ready()
    if digimon_engine.CARD_EVAL_EVENT_VERSION < 2:
        raise ValueError('Decision telemetry is required')
    contract = validate_checkpoint(MaskablePPO.load(parent / 'model.zip', device='cpu'), PROFILE)
    if validate_contract(read(parent / 'model.onnx.meta.json'), PROFILE) != contract:
        raise ValueError('Checkpoint and ONNX sidecar contracts differ')
    validate_onnx_file(parent / 'model.onnx', contract)
    if validate_checkpoint(MaskablePPO.load(development, device='cpu'), PROFILE) != contract:
        raise ValueError('Development anchor contract mismatch')
    registered = set(digimon_engine.load_implemented_card_ids())
    seen = set()
    for filename in SNAPSHOTS:
        pool = GeneralistDeckPool.from_snapshot(pool_dir / filename, implemented_card_ids=registered)
        for decks in pool.archetypes.values():
            for deck in decks:
                key = contents(deck.card_ids)
                if key in seen:
                    continue
                validate_admissible_deck(deck.card_ids, label=deck.deck_id)
                if not digimon_engine.validate_deck(deck.card_ids).is_valid:
                    raise ValueError(f'Deck failed native admission: {deck.deck_id}')
                seen.add(key)
    if not Path(cards_json).is_file():
        raise ValueError('Missing runtime card data')
    return dict(contract=contract, card_eval_event_version=digimon_engine.CARD_EVAL_EVENT_VERSION,
                registered_cards=len(registered), validated_distinct_decks=len(seen))


def run_health(args, root):
    """Two independent four-game native shards, with the full search budget."""
    validation = root / 'validation'
    validation.mkdir(parents=True, exist_ok=True)
    def health(start):
        out = validation / f'health-{start}'
        command = [args.cli_bin, '--pool', 'implemented', '--cards-json', str(args.cards_json),
                   'selfplay', '--model', str(root / 'generation/gen0/model.onnx'),
                   '--decks', str(root / 'inputs/decks.json'), '--games', '4',
                   '--game-start', str(start), '--seed', str(args.seed + 10000000),
                   '--sims', '320', '--worlds', '4', '--eval-batch-size', '4',
                   '--temperature-decisions', '16', '--decision-cap', '1200',
                   '--tensor-profile', PROFILE, '--out', str(out)]
        with (validation / f'health-{start}.log').open('w', encoding='utf-8') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1800)
        manifest = read(out / 'manifest.json')
        if manifest['games'] != 4 or manifest['truncated_games']:
            raise ValueError('Native full-budget health check failed')
        return dict(start=start, games=4, rows=manifest['rows'], truncated_games=0)
    with ThreadPoolExecutor(max_workers=2) as executor:
        return list(executor.map(health, (0, 4)))


def copy_verified(source, target, expected):
    if sha256(source) != expected:
        raise ValueError(f'Source changed before copy: {source}')
    target.parent.mkdir(parents=True, exist_ok=True)
    with source.open('rb') as src, target.open('xb') as dst:
        shutil.copyfileobj(src, dst)
    if sha256(target) != expected or sha256(source) != expected:
        raise ValueError(f'Source changed during copy: {source}')


def prepare(args):
    source, root, pool = args.source.resolve(), args.root.resolve(), args.pool_dir.resolve()
    if source == root or source in root.parents or root in source.parents:
        raise ValueError('Source and destination must not overlap')
    if pool == root or pool in root.parents or root in pool.parents:
        raise ValueError('Pool and destination must not overlap')
    if root.exists() and (not root.is_dir() or any(root.iterdir())):
        raise ValueError('Preparation destination must be empty')
    image = os.environ.get('DIGIMON_DEPLOYED_IMAGE', '')
    if not re.fullmatch(r'.+@sha256:[0-9a-f]{64}', image):
        raise ValueError('An immutable image digest is required')
    if not re.fullmatch(r'[0-9a-f]{40}', args.source_commit):
        raise ValueError('An exact image source commit is required')
    state, original = read(source / 'experiment_state.json'), read(source / 'campaign-plan.json')
    if state.get('stage') not in ('failed', 'complete'):
        raise ValueError('Parent campaign must be inactive (failed or complete)')
    if args.parent_generation not in (3, 4) or state.get('generation_index', -1) < args.parent_generation:
        raise ValueError('Requested saved checkpoint is outside the parent campaign')
    parent = source / f'generation/gen{args.parent_generation}'
    if sha256(parent / 'model.zip') != args.parent_sha256:
        raise ValueError('Parent checkpoint differs from approved evidence')
    assessment = validate_assessment(args.selection_report, source=source, image=image)
    selection = assessment.get('selection', {})
    if (assessment.get('status') != 'complete' or assessment.get('image') != image
            or assessment.get('protocol') != PROTOCOL
            or selection.get('checkpoint_index') != args.parent_generation
            or selection.get('checkpoint_sha256') != args.parent_sha256):
        raise ValueError('Transition assessment does not select this checkpoint on this image')
    if assessment.get('games_per_archetype') != 200 or assessment.get('games_per_checkpoint') != 7800:
        raise ValueError('Transition assessment must contain the approved 200-game, 13-archetype cohort')
    for name in ('development.zip', 'pet_evaluation_snapshot.json'):
        if sha256(source / 'inputs' / name) != original['files'].get(f'inputs/{name}'):
            raise ValueError(f'Frozen parent input differs: {name}')
    provenance = read(source / 'corpora/pilot/provenance.json')
    if sha256(args.cards_json) != provenance['cards_sha256']:
        raise ValueError('Card data changed across the continuation')
    report = read(pool / 'pool_report.json')
    for name, expected in report['artifacts'].items():
        if Path(name).name != name or sha256(pool / name) != expected:
            raise ValueError(f'Prepared pool artifact differs: {name}')
    denominators = summarize_pool(pool, source / 'inputs/pet_evaluation_snapshot.json')
    count = denominators['training']['archetypes']
    games = args.games if args.games is not None else max(1100, 4 * math.ceil(75 * count / 4))
    if games < 1000 or games % 4 or games * 2 / count < 150:
        raise ValueError('Use complete four-game chunks and >=150 expected appearances per archetype')
    if args.workers not in range(1, 9):
        raise ValueError('Use between one and eight workers')

    sources = {f'inputs/{name}': pool / name for name in POOL_FILES}
    sources.update({
        'inputs/pet_evaluation_snapshot.json': pool / 'pet_regression_snapshot.json',
        'inputs/development.zip': source / 'inputs/development.zip',
        'inputs/transition_assessment.json': args.selection_report,
        'inputs/transition_assessment_plan.json': args.selection_report.parent / 'plan.json',
        'inputs/transition_assessment_cohort.json': args.selection_report.parent / 'cohort.json',
        'inputs/parent_experiment_state.json': source / 'experiment_state.json',
        'inputs/parent_campaign_plan.json': source / 'campaign-plan.json',
        'inputs/parent_started.json': source / 'started.json',
        'inputs/parent_provenance.json': source / 'corpora/pilot/provenance.json',
    })
    for suffix in ('.zip', '.onnx', '.onnx.meta.json'):
        sources[f'inputs/seed{suffix}'] = parent / f'model{suffix}'
        sources[f'generation/gen0/model{suffix}'] = parent / f'model{suffix}'
    digests = {name: sha256(path) for name, path in sources.items()}
    runtime = validate_runtime(parent, source / 'inputs/development.zip', pool, args.cards_json)
    runtime_hashes = {str(args.code_root / name): sha256(args.code_root / name) for name in RUNTIME_FILES}
    root.mkdir(parents=True, exist_ok=True)
    with (root / 'preparation-started.json').open('x', encoding='utf-8') as marker:
        json.dump(dict(time_unix=time.time(), image=image, checkpoint_sha256=args.parent_sha256), marker)
    for name, path in sources.items():
        copy_verified(path, root / name, digests[name])
    gen0 = root / 'generation/gen0'
    atomic_json(root / 'generation/state.json', dict(next_gen=1, prev=dict(
        model_zip=str(gen0 / 'model.zip'), model_onnx=str(gen0 / 'model.onnx'))))
    contract = runtime['contract']
    champions = [dict(name=name, weights_path=str(root / 'inputs' / filename), algorithm='mlp',
                      observation_profile=contract['observation_profile'], tensor_layout_hash=contract['tensor_layout_hash'],
                      source=description) for name, filename, description in (
        (f'parent-checkpoint{args.parent_generation}', 'seed.zip', 'Assessed frozen parent; not promoted'),
        ('development-fixed', 'development.zip', 'Original frozen development anchor'))]
    atomic_json(root / 'inputs/anchors.json', dict(version=1, champions=champions))
    (root / 'corpora/pilot').mkdir(parents=True)
    (root / 'validation').mkdir()
    atomic_json(root / 'corpora/pilot/provenance.json', dict(
        cards_sha256=sha256(args.cards_json), source='checkpoint-only EX13 continuation',
        parent_provenance=provenance, parent_replay_games=0))
    evidence = dict(status='running', image=image, source_commit=args.source_commit,
                    checkpoint_sha256=args.parent_sha256, **runtime, fresh_replay=True,
                    parent_replay_games=0, cards_sha256=sha256(args.cards_json), started_unix=time.time())
    try:
        evidence['native_health'] = run_health(args, root)
        for name, path in sources.items():
            if sha256(path) != digests[name]:
                raise ValueError(f'Frozen source changed during preparation: {path}')
        evidence['status'] = 'passed'
    except BaseException as error:
        evidence.update(status='failed', error=str(error))
        raise
    finally:
        evidence['finished_unix'] = time.time()
        atomic_json(root / 'validation/generation0-recovery.json', evidence)
    files = {p.relative_to(root).as_posix(): sha256(p)
             for directory in (root / 'inputs', gen0, root / 'validation', root / 'corpora')
             for p in directory.rglob('*') if p.is_file()}
    # Both panels intentionally have their own denominators. Overlapping pet
    # rows are regression evidence, never pooled as independent held-out games.
    evaluation_panels = [['combined_diagnostic', 'combined_diagnostic_snapshot.json'],
                         ['original_pet_regression', 'pet_evaluation_snapshot.json']]
    evaluation_games = 200 * (1 + len(champions)) * sum(
        denominators[label]['archetypes'] for label, _ in evaluation_panels)
    plan = dict(generations=4, games=games, workers=args.workers, seed=args.seed, image=image,
                files=files, runtime_files=runtime_hashes, controller_commit=args.source_commit,
                ci_run=args.ci_run, preparation_sha256=sha256(Path(__file__)),
                parent_run=read(root / 'inputs/parent_started.json')['run_id'], parent_checkpoint=args.parent_generation,
                parent_checkpoint_sha256=args.parent_sha256, parent_source_state=state,
                parent_replay_games=0, fresh_replay=True, replay_window=3, replay_cap_games=games,
                expected_appearances_per_archetype=games * 2 / count,
                pool_denominators=denominators, pool_source_commit=report.get('source_commit'),
                evaluation_protocol=PROTOCOL, evaluation_panels=evaluation_panels,
                evaluation_seed=args.evaluation_seed, evaluation_games_per_archetype_per_anchor=200,
                games_per_evaluation=evaluation_games, automatic_restart=False, automatic_promotion=False,
                meta_archetypes=original['meta_archetypes'], pet_archetypes=original['pet_archetypes'],
                search=dict(simulations=320, worlds=4, evaluation_batch_size=4, decision_cap=1200))
    from tools.cloud_search.run_small_generations import validate
    validate(root, plan)
    for name in ('campaign-plan.json', 'deployment-plan.json'):
        atomic_json(root / name, plan)
    atomic_json(root / 'experiment_state.json', dict(stage='prepared', generation_index=0, image=image))
    return plan


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path('/archive'))
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    parser.add_argument('--pool-dir', type=Path, default=Path('/app/data/training_pools/ex13-20261026'))
    parser.add_argument('--parent-generation', type=int, choices=(3, 4), required=True)
    parser.add_argument('--parent-sha256', required=True)
    parser.add_argument('--selection-report', type=Path, required=True)
    parser.add_argument('--source-commit', required=True)
    parser.add_argument('--ci-run', required=True)
    parser.add_argument('--seed', type=int, required=True)
    parser.add_argument('--evaluation-seed', type=int, required=True)
    parser.add_argument('--games', type=int)
    parser.add_argument('--workers', type=int, default=8)
    parser.add_argument('--cards-json', type=Path, default=Path('/app/data/cards.json'))
    parser.add_argument('--code-root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--cli-bin', default='digimon-engine-cli')
    args = parser.parse_args()
    plan = prepare(args)
    print(json.dumps(dict(status='prepared', image=plan['image'], games=plan['games'],
                          checkpoint_sha256=plan['parent_checkpoint_sha256'],
                          games_per_evaluation=plan['games_per_evaluation'])), flush=True)


if __name__ == '__main__':
    main()
