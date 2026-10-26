import json
from collections import Counter
from pathlib import Path
from types import SimpleNamespace

import pytest

from tools.cloud_search.card_eval_worker import due_checkpoint
from tools.cloud_search.replay_window import appearance_counts, bounded_window
from digimon_gym.agents.anchored_eval import balanced_deck_pairs, _seat_balanced_seed
from tools.cloud_search.run_small_generations import validate, evaluation_options
from tools.selfplay_scheduler import sha256


def test_checkpoint_only_never_repeats_unchanged_weights():
    a, b = dict(sha256='a'), dict(sha256='b')
    state = dict(attempted=['a'], next_due_unix=0)
    for terminal in (False, True):
        assert due_checkpoint([a], state, 999999, 1, terminal, checkpoint_only=True) is None
        assert due_checkpoint([a, b], state, 999999, 1, terminal, checkpoint_only=True) == (b, 'new_checkpoint')


def test_balanced_cohort_has_exact_quotas_and_both_opening_orders():
    pool = SimpleNamespace(archetypes={
        name: [SimpleNamespace(archetype_name=name, deck_id=f'{name}{j}') for j in range(count)]
        for name, count in [('A', 2), ('B', 1), ('C', 3)]})
    cohort = list(balanced_deck_pairs(pool, 200, 47))
    assert len(cohort) == 600
    assert cohort == list(balanced_deck_pairs(pool, 200, 47))
    for name in pool.archetypes:
        selected = [(i, b) for i, (a, b) in enumerate(cohort) if a.archetype_name == name]
        assert len(selected) == 200
        assert Counter(_seat_balanced_seed(47, i) % 2 for i, _ in selected) == {0: 100, 1: 100}
        opponents = Counter(b.archetype_name for _, b in selected)
        assert max(opponents.values()) - min(opponents.values()) <= 2
    for i in range(0, len(cohort), 2):
        assert cohort[i] == cohort[i + 1]
    with pytest.raises(ValueError):
        list(balanced_deck_pairs(pool, 199, 1))


@pytest.mark.parametrize('counts', [[2] * 13, [2] * 12, [1, 2, 3]])
def test_balanced_cohort_visits_every_variant_matchup(counts):
    """Adding held-out variants must not silently omit half their matchups."""
    pool = SimpleNamespace(archetypes={
        f'A{i:02d}': [SimpleNamespace(archetype_name=f'A{i:02d}', deck_id=f'A{i:02d}-{j}')
                     for j in range(count)]
        for i, count in enumerate(counts)})
    quota = 2 * len(counts) * max(counts) ** 2
    cohort = list(balanced_deck_pairs(pool, quota, 47))
    seen = {(a.deck_id, b.deck_id) for a, b in cohort}
    expected = {(a.deck_id, b.deck_id)
                for first in pool.archetypes.values() for second in pool.archetypes.values()
                for a in first for b in second}
    assert seen == expected
    assert all(cohort[i] == cohort[i + 1] for i in range(0, len(cohort), 2))


def test_replay_caps_old_large_generation_and_is_stable(tmp_path):
    for generation, chunks in [(0, 825), (1, 250), (2, 250)]:
        for i in range(chunks):
            path = tmp_path / f'gen{generation}' / 'shards' / f'proc{i:08d}'
            path.mkdir(parents=True)
            (path / 'manifest.json').write_text(json.dumps(dict(games=4)))
    selected, record = bounded_window(tmp_path, 2, 3, 1000, 123)
    assert len(selected) == 750
    assert [g['games_before_quarantine'] for g in record['generations']] == [1000] * 3
    earlier, _ = bounded_window(tmp_path, 1, 3, 1000, 123)
    assert [p for p in selected if 'gen0' in p.parts] == [p for p in earlier if 'gen0' in p.parts]
    _, shifted = bounded_window(tmp_path, 3, 3, 1000, 123)
    assert [g['generation'] for g in shifted['generations']] == [1, 2, 3]
    assert len(list((tmp_path / 'gen0/shards').glob('*/manifest.json'))) == 825


def test_coverage_counts_both_seats(tmp_path):
    (tmp_path / 'manifest.json').write_text(json.dumps(dict(
        deck_names=['A:sha256:a:slot0', 'B:sha256:b:slot1'], game_decks=[[0, 1], [0, 0]])))
    result = appearance_counts([tmp_path])
    assert result['appearances'] == {'A': 3, 'B': 1}
    assert result['by_seat'] == [{'A': 2}, {'A': 1, 'B': 1}]


@pytest.fixture
def campaign(tmp_path, monkeypatch):
    names = ['inputs/decks.json', 'inputs/seed.zip', 'inputs/seed.onnx',
             'inputs/heldout_snapshot.json', 'inputs/pet_evaluation_snapshot.json',
             'inputs/anchors.json', 'generation/gen0/model.zip', 'generation/gen0/model.onnx',
             'validation/generation0-recovery.json']
    for name in names:
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(name)
    (tmp_path / 'inputs/decks.json').write_text(json.dumps([
        dict(name=f'A{i}:sha256:hash:slot0', cards=[]) for i in range(12)]))
    (tmp_path / 'validation/generation0-recovery.json').write_text(json.dumps(dict(
        status='passed', image='pinned', checkpoint_sha256=sha256(tmp_path / 'generation/gen0/model.zip'))))
    parent = tmp_path / 'generation/gen0'
    (tmp_path / 'generation/state.json').write_text(json.dumps(dict(next_gen=1,
        prev=dict(model_zip=str(parent / 'model.zip'), model_onnx=str(parent / 'model.onnx')))))
    plan = dict(generations=4, games=1000, workers=8, image='pinned', runtime_files={},
                files={name: sha256(tmp_path / name) for name in names})
    monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', 'pinned')
    return tmp_path, plan


def test_campaign_accepts_validated_parent(campaign):
    assert len(validate(*campaign)) == 12


def test_combined_evaluation_requires_hash_bound_snapshot(campaign):
    root, plan = campaign
    plan.update(evaluation_panels=[['combined_validation', 'combined_snapshot.json']], evaluation_seed=7)
    with pytest.raises(ValueError, match='hash-bound'):
        validate(root, plan)
    path = root / 'inputs/combined_snapshot.json'
    path.write_text('{}')
    plan['files']['inputs/combined_snapshot.json'] = sha256(path)
    validate(root, plan)
    options = evaluation_options(plan)
    assert options.seed == 7 and options.games_per_archetype == 200
    plan['evaluation_panels'] = [['../escape', 'combined_snapshot.json']]
    with pytest.raises(ValueError, match='local labels'):
        validate(root, plan)


def test_campaign_accepts_two_remaining_updates_with_fresh_replay(campaign):
    root, plan = campaign
    plan['generations'] = 2
    assert len(validate(root, plan)) == 12
    selected, record = bounded_window(root / 'generation', 1, 3, 1000, 123)
    assert selected == []  # A warm-start checkpoint alone is not replay data.
    assert all(g['games_before_quarantine'] == 0 for g in record['generations'])


@pytest.mark.parametrize('change', ['parent', 'image', 'hash', 'scope', 'already_started', 'pool_size'])
def test_campaign_refuses_drift(campaign, monkeypatch, change):
    root, plan = campaign
    if change == 'parent':
        path = root / 'validation/generation0-recovery.json'
        path.write_text(json.dumps(dict(status='failed', image='pinned')))
        plan['files']['validation/generation0-recovery.json'] = sha256(path)
    elif change == 'image':
        monkeypatch.setenv('DIGIMON_DEPLOYED_IMAGE', 'other')
    elif change == 'hash':
        (root / 'inputs/seed.zip').write_text('different')
    elif change == 'scope':
        plan['generations'] = 8
    elif change == 'already_started':
        (root / 'generation/gen1').mkdir()
    else:
        path = root / 'inputs/decks.json'
        path.write_text(json.dumps([dict(name=f'A{i}:sha256:x') for i in range(18)]))
        plan['files']['inputs/decks.json'] = sha256(path)
    with pytest.raises(ValueError):
        validate(root, plan)
