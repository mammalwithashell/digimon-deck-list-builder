import json
from pathlib import Path

import pytest
from tools.cloud_search import run_seed_comparison as comparison


def games(winners):
    return [dict(seed=20 + 2*i + i % 2, winner_id=winner, terminal_reason="security_attack",
                 context=dict(panel="meta_validation", anchor="greedy", agent_archetype="a",
                              opponent_archetype="b", agent_deck_id="a1", opponent_deck_id="b1"))
            for i, winner in enumerate(winners)]


@pytest.fixture
def saved(tmp_path, monkeypatch):
    import model_contract
    from sb3_contrib import MaskablePPO
    write = comparison.write_json
    monkeypatch.setattr(model_contract, 'current_contract', lambda _: {'profile': 'fixture'})
    monkeypatch.setattr(model_contract, 'validate_checkpoint', lambda *a: {})
    monkeypatch.setattr(MaskablePPO, 'load', lambda *a, **k: object())
    cards = tmp_path / 'cards.json'
    cards.write_text('{}')
    monkeypatch.setenv('DIGIMON_CARDS_JSON', str(cards))
    inputs = {name: comparison.sha256(cards) for name in comparison.INPUTS}
    for name in inputs:
        write(tmp_path / 'inputs' / name, {})
        inputs[name] = comparison.sha256(tmp_path / 'inputs' / name)
    plan = dict(seed=7, image='old@sha256:a', inputs=inputs, arms=list(comparison.ARMS),
        bootstrap_games=416, search_games_per_arm=1100, simulations=320, worlds=4,
        evaluation_batch_size=4, evaluations_per_arm=2, games_per_evaluation=5200,
        automatic_restart=False, automatic_promotion=False)
    write(tmp_path / 'started.json', dict(plan=plan))
    write(tmp_path / 'comparison_state.json', dict(stage='failed'))
    for arm in comparison.ARMS:
        path, corpus = tmp_path / 'arms' / arm, tmp_path / 'corpora' / arm
        write(corpus / 'qualification.json', dict(qualified=True, games=[{}]*416))
        write(corpus / 'provenance.json', dict(seed=7, contract={'profile':'fixture'}, cards_sha256=comparison.sha256(cards)))
        write(path / 'deployment-plan.json', dict(image='old@sha256:a'))
        for name in inputs:
            write(path / 'inputs' / name, {})
        for suffix in ('.zip', '.onnx', '.onnx.meta.json', '.calibration.json'):
            for file in (corpus / f'seed{suffix}', path / 'inputs' / f'seed{suffix}', path / 'generation/gen0' / f'model{suffix}'):
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_bytes((arm+suffix).encode())
        write(path / 'generation/state.json', dict(next_gen=1, prev=dict(
            model_zip=str(path/'generation/gen0/model.zip'), model_onnx=str(path/'generation/gen0/model.onnx'))))
        write(path/'inputs/anchors.json', dict(champions=[dict(weights_path=str(tmp_path/'corpora/greedy/seed.zip'))]))
    window = tmp_path / 'evaluations/development/window-000000'
    write(window/'result.json', dict(status='failed'))
    (window/'evidence.txt').write_text('preserve this')
    return tmp_path


def test_explicit_resume_rejects_changed_seed_input_model_or_started_search(saved):
    for mutation, message in [('seed', 'initialization'), ('input','frozen input'), ('model','saved seed'), ('generation','pre-search')]:
        file = None
        if mutation == 'input': file = saved/'inputs/decks.json'
        if mutation == 'model': file = saved/'arms/mixed/generation/gen0/model.zip'
        if file:
            original = file.read_bytes(); file.write_bytes(b'tampered')
        if mutation == 'generation': (saved/'arms/greedy/generation/gen1').mkdir()
        with pytest.raises(ValueError, match=message):
            comparison.validate_resume(saved, seed=8 if mutation=='seed' else 7, image='new@sha256:b')
        if file: file.write_bytes(original)
        if mutation == 'generation': (saved/'arms/greedy/generation/gen1').rmdir()


def test_resume_archives_failed_window_without_collecting_or_refitting(saved, monkeypatch):
    immutable = {str(p): p.read_bytes() for p in saved.rglob('model.*')}
    started = (saved/'started.json').read_bytes()
    monkeypatch.setattr(comparison, 'collect', lambda *a,**k: pytest.fail('must not recollect'))
    monkeypatch.setattr(comparison, 'fit', lambda *a,**k: pytest.fail('must not refit'))
    called=[]
    def continuation(root, seed, stage, arm_stage, reused):
        assert not (root/'evaluations/development/window-000000').exists()
        assert next(root.glob('resume-history/*/development/window-000000/evidence.txt')).read_text()=='preserve this'
        called.append((seed,reused))
    monkeypatch.setattr(comparison,'continue_comparison',continuation)
    comparison.resume(saved,seed=7,image='new@sha256:b')
    assert called==[(7,set())]
    assert all(Path(p).read_bytes()==b for p,b in immutable.items())
    assert (saved/'started.json').read_bytes()==started
    assert json.loads((saved/'comparison_state.json').read_text())['stage']=='complete'


def test_validate_only_has_no_state_or_evaluation_mutations(saved):
    before={str(p):p.read_bytes() for p in saved.rglob('*') if p.is_file()}
    result=comparison.resume(saved,seed=7,image='new@sha256:b',validate_only=True)
    assert result['archived_evaluations']==['development']
    assert all(Path(p).read_bytes()==b for p,b in before.items())
    assert not (saved/'resume-history').exists()


def test_completed_window_reuse_requires_matching_provenance(saved, monkeypatch):
    write = comparison.write_json
    window=saved/'evaluations/greedy/window-000000'
    model_hash=comparison.sha256(saved/'arms/greedy/generation/gen0/model.zip')
    write(window/'result.json',dict(status='complete',checkpoint=dict(sha256=model_hash)))
    write(window/'context.json',dict(checkpoint_sha256=model_hash))
    provenance=dict(status='complete',candidate_sha256=model_hash,panel='meta_validation',base_seed=28261025,
        games_per_archetype=200,snapshot_sha256=comparison.sha256(saved/'inputs/heldout_snapshot.json'),
        anchors_sha256=comparison.sha256(saved/'arms/greedy/inputs/anchors.json'),
        anchor_model_hashes={'frozen-greedy-bootstrap':comparison.sha256(saved/'corpora/greedy/seed.zip')})
    write(window/'reports/test.json',dict(provenance=provenance))
    monkeypatch.setattr(comparison,'read_games',lambda _:games([1,2]))
    record=comparison.validate_resume(saved,seed=7,image='new@sha256:b')
    assert record['reused_evaluations']==['greedy']
    provenance['snapshot_sha256']='changed'
    write(window/'reports/test.json',dict(provenance=provenance))
    with pytest.raises(ValueError,match='cohort provenance'):
        comparison.validate_resume(saved,seed=7,image='new@sha256:b')


def test_continuation_skips_completed_window_but_runs_remaining_updates(saved,monkeypatch):
    calls=[]
    write=comparison.write_json
    for arm in comparison.ARMS:
        path=saved/'arms'/arm/'generation/gen1/model.zip'
        path.parent.mkdir(parents=True)
        path.write_text('new model')
    completed=saved/'evaluations/greedy/window-000000'
    write(completed/'result.json',dict(status='complete'))
    for panel in ('meta_validation','pet_same_list'):
        (completed/f'{panel}.log').write_text('preserved complete log')
    def evaluate(root,out,checkpoint,window,reason,args):
        calls.append(('eval',root.name,window))
        folder=out/f'window-{window:06d}'
        folder.mkdir(parents=True,exist_ok=True)
        for panel,_ in args.panels:
            (folder/f'{panel}.log').write_text('new report')
        return dict(status='complete')
    def search(command,**kwargs):
        assert command[command.index('--sims')+1]=='320'
        assert command[command.index('--worlds')+1]=='4'
        assert command[command.index('--eval-batch-size')+1]=='4'
        calls.append(('search',))
    monkeypatch.setattr(comparison,'evaluate_window',evaluate)
    monkeypatch.setattr(comparison.subprocess,'run',search)
    monkeypatch.setattr(comparison,'read_games',lambda _:games([1,2]))
    comparison.continue_comparison(saved,7,lambda *a,**k:None,lambda *a,**k:None,{('greedy',0)})
    assert ('eval','greedy',0) not in calls
    assert len([c for c in calls if c[0]=='eval'])==5
    assert calls.count(('search',))==3
    assert (completed/'meta_validation.log').read_text()=='preserved complete log'
