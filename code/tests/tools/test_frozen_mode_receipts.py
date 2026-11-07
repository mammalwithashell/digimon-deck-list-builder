import copy
import pytest
from tools.cloud_search.frozen_mode_receipts import SCHEMA, validate


def fixture():
    settings = dict(kind="search", information_regime="closed-list", simulations=320, worlds=4, batch=4, root_noise=0)
    req = dict(schema=SCHEMA, runtime_sha256="runtime", prior_snapshot_hash="prior", search_seed=7,
               candidate=settings, opponent=dict(settings, information_regime="known-list"))
    plan = dict(schema=SCHEMA, native_sha256="runtime", prior_snapshot_hash="prior", model_sha256="cp12",
                opponent_model_sha256="cp8", decision_cap=100)
    games = []
    for seat in (0, 1):
        info = dict(regime="closed-list" if seat == 0 else "known-list", prior_snapshot_hash="prior",
                    sampled_worlds=4, world_seeds=[1,2,3,4], fallback_worlds=0, fallback_mass=.1,
                    evidence_sha256="e"*64, sampling_seconds=.1)
        games.append(dict(candidate_seat=seat, first_player=0, winner=0, truncated=False,
            terminal_reason="Some(SecurityAttack)", score=float(seat==0), seconds=.2, decisions=1,
            actions=[2], searched=1, card_metrics=dict(steps=[dict(mover=1,action=2,legal_actions=[2,3],selection_seconds=.1)]),
            search_diagnostics=[dict(decision=0,selected_plan=[2],root_plans=[[[2],200,.1],[[3],120,.2]],
                diagnostics=dict(viewer=0,search_seed=7,simulations=320,worlds=4,batch=4,seconds=.1,information=info))]))
    out=dict(schema=SCHEMA,request=copy.deepcopy(req),runtime_sha256="runtime",prior_snapshot_hash="prior",
             model_sha256="cp12",opponent_model_sha256="cp8",games=games)
    return req,out,plan


def test_valid_swapped_mode_receipt_semantics():
    assert validate(*fixture()) == dict(games=2,wins=1,draws=0,losses=1)


@pytest.mark.parametrize("mutate", [
    lambda r,o,p:o.pop("schema"),
    lambda r,o,p:o.update(runtime_sha256="other"),
    lambda r,o,p:o.update(prior_snapshot_hash="other"),
    lambda r,o,p:o["games"][0].update(truncated=True),
    lambda r,o,p:o["games"][0].update(terminal_reason="Some(StepLimit)"),
    lambda r,o,p:o["games"][0].update(score=0),
    lambda r,o,p:o["games"][1].update(candidate_seat=0),
    lambda r,o,p:o["games"][0]["search_diagnostics"][0]["diagnostics"].update(simulations=64),
    lambda r,o,p:o["games"][0]["search_diagnostics"][0]["diagnostics"]["information"].update(regime="known-list"),
    lambda r,o,p:o["games"][0]["card_metrics"]["steps"][0].update(legal_actions=[3]),
    lambda r,o,p:o["games"][0]["search_diagnostics"][0].update(selected_plan=[3]),
    lambda r,o,p:o["games"][0]["search_diagnostics"][0]["diagnostics"].update(search_seed=8),
])
def test_rejects_invalid_evidence(mutate):
    r,o,p=fixture(); mutate(r,o,p)
    with pytest.raises(ValueError): validate(r,o,p)


def test_rejects_unsearched_nonforced_decision():
    r,o,p=fixture(); o["games"][0].update(searched=0,search_diagnostics=[])
    with pytest.raises(ValueError,match="unsearched"): validate(r,o,p)
