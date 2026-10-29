import json
import subprocess

import pytest

from tools.cloud_search.audit_archetype_behavior import aggregate, build, outcome_rates, render
from tools.cloud_search.audit_search_piloting import completion_status, run_pair, summarize


def test_incomplete_mirror_pair_is_not_a_draw_or_two_independent_games():
    complete = dict(archetype='A', status='complete', seconds=1, games=[
        dict(candidate_seat=0, winner=0, score=1, searched=10, decisions=20),
        dict(candidate_seat=1, winner=0, score=0, searched=10, decisions=20),
    ])
    incomplete = dict(archetype='A', status='incomplete', seconds=1, games=[
        dict(candidate_seat=0, winner=None, score=.5, searched=10, decisions=1200),
        dict(candidate_seat=1, winner=1, score=1, searched=10, decisions=20),
    ])
    row = summarize([complete, incomplete], ['A'])['overall']
    assert (row['games'], row['wins'], row['losses'], row['draws']) == (2, 1, 1, 0)
    assert row['paired_scores'] == [.5]
    assert row['incomplete_pairs'] == 1


def test_failed_archetype_is_unmeasured_not_zero_strength():
    result = summarize([dict(archetype='A', status='failed', games=[])], ['A', 'B'])
    assert result['overall']['score'] is None
    assert result['archetypes']['A']['incomplete_pairs'] == 1
    assert result['archetypes']['B']['games'] == 0
    assert result['archetypes']['B']['score'] is None


def test_behavior_aggregation_keeps_archetype_anchor_and_player_scopes():
    rows = [dict(agent_archetype=a, anchor=b, player=p, route='digivolve_raising',
                 opportunities=n, choices=1)
            for a,b,p,n in [('A','parent',1,5), ('A','parent',1,6),
                            ('A','greedy',1,100), ('A','parent',2,200), ('B','parent',1,300)]]
    assert aggregate(rows, 'A', 'parent', 'route') == {
        'digivolve_raising': dict(opportunities=11, choices=2)}


def test_attempted_failed_pairs_cannot_satisfy_planned_coverage():
    rows = [dict(archetype='A', status='failed', games=[], process_wall_seconds=30)]
    result = summarize(rows, ['A', 'B'], expected_pairs_per_archetype=1)
    assert result['overall']['attempted_pairs'] == 1
    assert result['overall']['valid_pairs'] == 0
    assert result['overall']['expected_pairs'] == 2
    assert result['overall']['coverage_complete'] is False
    assert result['archetypes']['B']['coverage_complete'] is False
    assert completion_status(result) == 'finished_with_incomplete'
    assert result['overall']['process_wall_seconds'] == 30
    assert result['overall']['engine_gameplay_seconds'] == 0


def test_complete_pair_covers_both_seats_and_counts_genuine_draws():
    row = dict(archetype='A', status='complete', seconds=2, process_wall_seconds=7, games=[
        dict(candidate_seat=0, winner=0, score=1, searched=5, decisions=10),
        dict(candidate_seat=1, winner=None, score=.5, searched=5, decisions=10)])
    result = summarize([row], ['A'], expected_pairs_per_archetype=1)
    assert completion_status(result) == 'complete'
    assert result['overall']['games'] == 2
    assert result['overall']['score'] == .75
    assert result['overall']['paired_scores'] == [.75]
    assert result['overall']['engine_gameplay_seconds'] == 2
    assert result['overall']['process_wall_seconds'] == 7


def test_pair_timeout_records_wall_time_and_no_strength(monkeypatch, tmp_path):
    times = iter([100., 130.])
    monkeypatch.setattr('tools.cloud_search.audit_search_piloting.time.perf_counter', lambda:next(times))
    def timeout(*args, **kwargs):
        raise subprocess.TimeoutExpired(args[0], kwargs['timeout'])
    monkeypatch.setattr('tools.cloud_search.audit_search_piloting.subprocess.run', timeout)
    result = run_pair('engine', tmp_path/'request.json', tmp_path/'result.json', timeout=30)
    assert result['status'] == 'failed'
    assert result['games'] == []
    assert result['process_wall_seconds'] == 30


def test_pair_rejects_unbalanced_opening_seats(monkeypatch, tmp_path):
    target = tmp_path/'result.json'
    def run(*args, **kwargs):
        target.write_text(json.dumps(dict(seconds=1,games=[
            dict(candidate_seat=0,first_player=0,truncated=False),
            dict(candidate_seat=1,first_player=1,truncated=False)])))
    monkeypatch.setattr('tools.cloud_search.audit_search_piloting.subprocess.run', run)
    result = run_pair('engine', tmp_path/'request.json', target)
    assert result['status'] == 'failed'
    assert 'balanced' in result['error']


@pytest.fixture
def baseline_inputs():
    anchors = ['greedy', 'parent-checkpoint4', 'development-fixed']
    report = dict(id='baseline', checkpoint=0, status='complete',
        provenance=dict(candidate_sha256='model', anchor_model_hashes={'parent-checkpoint4':'model'},
                        evaluation_mode='policy_only', card_eval_event_version=2,
                        snapshot_sha256='pool', engine_image='fixed-image'),
        outcomes=[dict(anchor=a, archetype='A', wins=90, losses=90, draws=20, incomplete=0)
                  for a in anchors], cards=[], route_decisions=[], effect_sequences=[])
    lists = dict(hashes={'runs/audit/inputs/combined_snapshot.json':'pool'}, deployed_image='fixed-image',
        archetypes=[dict(archetype='A', training_distinct_lists=2, evaluation_scope='heldout',
                         signature_cards=[dict(card_id='CARD', name='Signature')])])
    meta = dict(archetypes=[dict(archetype='A', format_row=['EX13','10','9-9-2','50%'], url='https://example.test')])
    return dict(reports=[report]), lists, meta


@pytest.mark.parametrize('field,value', [('snapshot_sha256','wrong-pool'), ('engine_image','wrong-image')])
def test_behavior_rejects_evidence_from_different_pool_or_engine(baseline_inputs, field, value):
    snapshot, lists, meta = baseline_inputs
    snapshot['reports'][0]['provenance'][field] = value
    with pytest.raises(ValueError, match='differs'):
        build(snapshot, lists, meta)


def test_behavior_retains_draw_denominators(baseline_inputs):
    result = build(*baseline_inputs)
    row = result['summary'][0]
    assert row['same_model_games'] == 200
    assert row['same_model_draws'] == 20
    assert row['same_model_decided_games'] == 180
    assert row['same_model_percent'] == 45
    assert row['same_model_score_percent'] == 50
    assert row['same_model_decided_win_percent'] == 50
    assert result['archetypes']['A']['anchor_rates']['greedy']['draws'] == 20
    assert outcome_rates(dict(wins=0, losses=0, draws=2))['decided_win_percent'] is None


def test_missing_behavior_is_unmeasured_while_measured_zero_remains_zero(baseline_inputs, tmp_path):
    result = build(*baseline_inputs)
    assert result['summary'][0]['raising_choices'] is None
    assert result['summary'][0]['raising_opportunities'] is None
    assert result['archetypes']['A']['signatures'][0]['metrics'] is None
    render(result, tmp_path)
    html = (tmp_path/'behavior-audit.html').read_text()
    assert '<td>Unmeasured</td>' in html
    assert '<td>None</td>' not in html
    snapshot, lists, meta = baseline_inputs
    snapshot['reports'][0]['route_decisions'].append(dict(agent_archetype='A',
        anchor='parent-checkpoint4', player=1, route='digivolve_raising', opportunities=0, choices=0))
    row = build(snapshot, lists, meta)['summary'][0]
    assert row['raising_choices'] == 0
    assert row['raising_opportunities'] == 0


def test_behavior_rejects_incomplete_evaluation_denominator(baseline_inputs):
    snapshot, lists, meta = baseline_inputs
    snapshot['reports'][0]['outcomes'][0]['incomplete'] = 1
    with pytest.raises(ValueError, match='denominator'):
        build(snapshot, lists, meta)
