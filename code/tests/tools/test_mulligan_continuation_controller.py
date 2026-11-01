import copy
import json
import pytest
from tools.cloud_search import mulligan_continuation_controller as c


def fixture():
    node = dict(id="root", mover=0, first=True, observation_sha256="obs", hand=["A"])
    plan = dict(schema=c.SCHEMA, checkpoint=12, opponent_checkpoint=8, model_sha256="cp12",
        opponent_model_sha256="cp8", runtime_sha256="runtime", protocol_sha256="protocol",
        information_regime="known-list", simulations=320, worlds=4, batch=4, root_noise=0,
        decision_cap=2000, deadline_seconds=3600)
    request = dict(schema=c.SCHEMA, checkpoint=12, opponent_checkpoint=8, root="root",
        plan_sha256="plan", runtime_sha256="runtime", protocol_sha256="protocol",
        forced_mulligan_action=0, outcome_seed=100, follow_on_search_seed=200)
    steps = [dict(mover=1, phase="Mulligan", action=0, legal_actions=[0, 1],
                  forced_intervention=True, selection_seconds=0)]
    searches = []
    for start, actor, model, action in [(1, 0, "cp12", 7), (2, 1, "cp8", 8)]:
        steps.append(dict(mover=actor+1, phase="Main", action=action, legal_actions=[action, 9], selection_seconds=.1))
        route = [action]
        opportunity = dict(schema="search-opportunities-v1", actor=actor, sampling="argmax_tau0",
            simulations_run=320, selected_plan=route, executed_actions=route, commitment_executed=True,
            plans=[dict(plan=route, visits=320, q=.5, prior_before_noise=1., prior_after_noise=1.,
                        selected=True, selection_probability=1.)])
        searches.append(dict(decision=start, actor=actor, model_sha256=model, simulations=320, worlds=4,
            batch=4, root_noise=0, information_regime="known-list", selected_plan=route,
            root_plans=[[route, 320, .5]], search_seed=(200+start*0x9e3779b97f4a7c15+actor)%(1<<64),
            opportunity=opportunity))
    game = dict(candidate_seat=0, first_player=0, winner=0, score=1., truncated=False,
        terminal_reason="Some(SecurityAttack)", actions=[0, 7, 8], decisions=3, searched=2,
        search_diagnostics=searches, seconds=1., card_metrics=dict(steps=steps, initial_ui={"player1":{"handIds":["A"]}}))
    output = dict(schema=c.SCHEMA, request=copy.deepcopy(request), plan_sha256="plan", runtime_sha256="runtime",
        model_sha256="cp12", opponent_model_sha256="cp8", root_observation_sha256="obs", game=game)
    return request, output, plan, node


def test_valid_forced_then_both_actor_searches():
    assert c.validate_output(*fixture()) == dict(games=1, score=1., decisions=3)


@pytest.mark.parametrize("mutate", [
    lambda r,o,p,n:o.update(runtime_sha256="other"),
    lambda r,o,p,n:o.update(plan_sha256="other"),
    lambda r,o,p,n:o.update(root_observation_sha256="other"),
    lambda r,o,p,n:o["game"].update(truncated=True),
    lambda r,o,p,n:o["game"].update(terminal_reason="Some(StepLimit)"),
    lambda r,o,p,n:o["game"].update(score=0),
    lambda r,o,p,n:o["game"].update(seconds=3601),
    lambda r,o,p,n:o["game"].update(first_player=1),
    lambda r,o,p,n:o["game"]["actions"].__setitem__(0,1),
    lambda r,o,p,n:o["game"]["card_metrics"]["steps"][1].update(forced_intervention=True),
    lambda r,o,p,n:o["game"]["card_metrics"]["steps"][1].update(legal_actions=[9]),
    lambda r,o,p,n:o["game"]["search_diagnostics"][1].update(model_sha256="cp12"),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0].update(simulations=64),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0].update(search_seed=100),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0].update(decision=0),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0]["opportunity"].update(commitment_executed=False),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0]["opportunity"]["plans"][0].update(prior_before_noise=None),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0]["opportunity"]["plans"][0].update(visits=1),
    lambda r,o,p,n:o["game"]["search_diagnostics"][0]["opportunity"]["plans"][0].update(q=float("nan")),
])
def test_rejects_wrong_branch_routing_budget_and_telemetry(mutate):
    args=fixture(); mutate(*args)
    with pytest.raises(ValueError): c.validate_output(*args)


def test_unsearched_nonforced_choice_is_not_hidden_by_intervention():
    r,o,p,n=fixture(); o["game"]["search_diagnostics"].pop(); o["game"]["searched"]=1
    with pytest.raises(ValueError,match="unsearched"): c.validate_output(r,o,p,n)


def test_second_player_bool_is_not_a_physical_seat_id():
    r,o,p,n=fixture(); n["first"]=False; o["game"]["first_player"]=1
    assert c.validate_output(r,o,p,n)["games"]==1


def test_native_failure_is_rejected_and_never_reinvoked(tmp_path, monkeypatch):
    for folder in ("requests","results","attempts","logs"): (tmp_path/folder).mkdir()
    (tmp_path/'requests/job.json').write_text('{}')
    called=[]
    def fail(*args,**kwargs):
        called.append(args)
        return type('Exit',(),{'returncode':1})()
    monkeypatch.setattr(c.subprocess,'run',fail)
    protocol=dict(binary='runtime/native',deadline_seconds=3600)
    assert c.execute(tmp_path,protocol,'job') is False
    assert (tmp_path/'results/job.rejected.json').exists()
    with pytest.raises(FileExistsError): c.execute(tmp_path,protocol,'job')
    assert len(called)==1


def test_queue_drains_active_jobs_without_dispatch_after_rejection(tmp_path, monkeypatch):
    (tmp_path/'attempts').mkdir(); (tmp_path/'manifest.json').write_text('{}')
    monkeypatch.setattr(c,'validate_inputs',lambda root:dict(jobs=['a','b','c'],workers=1))
    calls=[]
    monkeypatch.setattr(c,'execute',lambda root,protocol,job:calls.append(job) or False)
    assert c.run(tmp_path) is False
    assert calls==['a']
    assert json.loads((tmp_path/'state.json').read_text())['state']=='failed'
