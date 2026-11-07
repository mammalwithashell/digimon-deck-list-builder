import json
from types import SimpleNamespace
import pytest
from digimon_gym.agents.tamer_first import TamerFirstOpponent


def env(result):
    return SimpleNamespace(get_action_mask=lambda:[1,0,1],runner=SimpleNamespace(teacher_decision=lambda name:json.dumps(result)))


def decision(**kwargs):
    return dict(version="greedy_tamer_first_v1",action=2,direct_tamer_routes=[2],tamer_priority_applied=True,**kwargs)


def test_prioritized_action_and_observational_counters():
    policy=TamerFirstOpponent("greedy_tamer_first_v1")
    assert policy(env(decision()))==2
    assert (policy.decisions,policy.opportunities)==(1,1)
    policy.reset_state();assert policy.last_decision is None
    assert policy.opportunities==1


@pytest.mark.parametrize("field,value",[("action",1),("action",0),("direct_tamer_routes",[1]),("version","greedy"),("tamer_priority_applied",False)])
def test_rejects_wrong_contract(field,value):
    row=decision();row[field]=value
    with pytest.raises(RuntimeError):TamerFirstOpponent("greedy_tamer_first_v1")(env(row))


def test_no_opportunity_uses_native_fallback():
    row=decision();row.update(action=0,direct_tamer_routes=[],tamer_priority_applied=False)
    policy=TamerFirstOpponent("greedy_tamer_first_v1")
    assert policy(env(row))==0
    assert policy.opportunities==0


def test_old_binding_fails_instead_of_silently_using_old_bot():
    def old(name):raise ValueError("unknown bootstrap teacher")
    game=env(decision());game.runner.teacher_decision=old
    with pytest.raises(RuntimeError,match="qualified native binding"):
        TamerFirstOpponent("greedy_tamer_first_v1")(game)


def test_native_nonprogress_failure_is_preserved_not_mislabeled_as_old_binding():
    def failed(name):raise ValueError("repeated visible state exhausted legal progress choices")
    game=env(decision());game.runner.teacher_decision=failed
    with pytest.raises(RuntimeError,match="native decision failed: repeated visible state"):
        TamerFirstOpponent("development_tamer_first_v1")(game)


def test_anchored_evaluation_addition_is_opt_in(monkeypatch):
    from digimon_gym.agents import anchored_eval
    calls=[]
    def matchup(candidate,opponent,*args,**kwargs):
        calls.append(opponent);return anchored_eval.MatchRecord(wins=1)
    monkeypatch.setattr(anchored_eval,"play_matchup",matchup)
    registry=SimpleNamespace(compatible=lambda _:[])
    baseline=anchored_eval.evaluate_against_anchors(None,"layout",registry,None)
    assert set(baseline)=={"greedy"}
    expanded=anchored_eval.evaluate_against_anchors(None,"layout",registry,None,include_tamer_first=True)
    assert set(expanded)=={"greedy","greedy_tamer_first_v1","development_tamer_first_v1"}
    assert calls[-2].version=="greedy_tamer_first_v1"
    assert calls[-1].version=="development_tamer_first_v1"


def test_baseline_name_collision_fails_before_games(monkeypatch):
    from digimon_gym.agents import anchored_eval
    monkeypatch.setattr(anchored_eval,"play_matchup",lambda *a,**k:pytest.fail("must reject before games"))
    registry=SimpleNamespace(compatible=lambda _:[SimpleNamespace(name="development_tamer_first_v1")])
    with pytest.raises(ValueError,match="collides"):
        anchored_eval.evaluate_against_anchors(None,"layout",registry,None,include_tamer_first=True)
