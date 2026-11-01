import copy
from tools.cloud_search.compare_mulligan_continuations import comparable, differences


def output():
    return dict(runtime_sha256="w",plan_sha256="wp", request=dict(runtime_sha256="w",plan_sha256="wp",outcome_seed=10),
        game=dict(seconds=1.,actions=[0,7],card_metrics=dict(steps=[dict(selection_seconds=.1,action=7)]),
                  search_diagnostics=[dict(q=.25,prior=.5)]))


def test_only_declared_clocks_and_platform_identities_are_removed():
    a=output(); b=copy.deepcopy(a)
    b.update(runtime_sha256="l",plan_sha256="lp"); b['request'].update(runtime_sha256="l",plan_sha256="lp")
    b['game']['seconds']=10.; b['game']['card_metrics']['steps'][0]['selection_seconds']=5.
    assert differences(comparable(a),comparable(b))==[]
    assert a['game']['seconds']==1.


def test_one_float_step_is_not_rounded_away():
    a=output(); b=copy.deepcopy(a)
    b['game']['search_diagnostics'][0]['prior']+=2.9802322387695312e-08
    assert differences(comparable(a),comparable(b))==['/game/search_diagnostics/0/prior']


def test_rng_action_and_new_fields_remain_significant():
    a=output()
    for change in (lambda b:b['request'].update(outcome_seed=11),
                   lambda b:b['game']['actions'].__setitem__(1,8),
                   lambda b:b['game'].update(extra_seconds=2)):
        b=copy.deepcopy(a); change(b)
        assert differences(comparable(a),comparable(b))
