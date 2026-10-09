"""Router (tasks 4.4): defaults, seeded exploration, forced adversarial authoring,
availability fallback, two-family termination, scorecard-driven switching."""
import pytest

from tools.card_loop.config import LoopConfig
from tools.card_loop.contracts import FAMILIES, STAGES, TERMINATING_STAGES, Usage
from tools.card_loop.ledger import Attempt
from tools.card_loop.router import (
    RoutingUnavailable,
    SingleFamilyTermination,
    exploration_draw,
    is_terminating,
    other_family,
    route,
    scorecard_choice,
    second_opinion_family,
)
from tools.card_loop.scorecard import Scorecard

CFG = LoopConfig()
ITEMS = [f"clause:BT{s}-{n:03d}#effect#0" for s in range(1, 21) for n in range(1, 101)]


def _routed_item(stage, cfg=CFG):
    """An item whose seeded draw does NOT explore."""
    return next(i for i in ITEMS if exploration_draw(cfg.seed, stage, i) >= cfg.exploration_share)


def _explore_item(stage, cfg=CFG):
    return next(i for i in ITEMS if exploration_draw(cfg.seed, stage, i) < cfg.exploration_share)


def test_default_routes_from_config():
    for stage in STAGES:
        fam, assignment = route(stage, _routed_item(stage), config=CFG)
        assert (fam, assignment) == (CFG.routes[stage], "routed")


def test_exploration_sends_the_item_to_the_other_family():
    stage = "implement"
    fam, assignment = route(stage, _explore_item(stage), config=CFG)
    assert (fam, assignment) == ("codex", "explore")


def test_spec_reproducible_assignment():
    for item in ITEMS[:200]:
        assert route("triage", item, config=CFG) == route("triage", item, config=LoopConfig())
    # the draw is keyed on the seed: another seed reshuffles which items explore
    other = LoopConfig(seed=1)
    explore0 = {i for i in ITEMS if route("triage", i, config=CFG)[1] == "explore"}
    explore1 = {i for i in ITEMS if route("triage", i, config=other)[1] == "explore"}
    assert explore0 != explore1


def test_exploration_share_is_about_twenty_percent_per_stage():
    for stage in ("implement", "review", "author_clause"):
        n = sum(route(stage, i, config=CFG)[1] == "explore" for i in ITEMS)
        assert 0.17 < n / len(ITEMS) < 0.23, (stage, n)


def test_exploration_share_bounds():
    never = LoopConfig(exploration_share=0.0)
    always = LoopConfig(exploration_share=1.0)
    assert all(route("implement", i, config=never)[1] == "routed" for i in ITEMS[:300])
    assert all(route("implement", i, config=always) == ("codex", "explore") for i in ITEMS[:300])


def test_spec_implementer_excluded_outside_exploration():
    # Spec: a card implemented by Claude has its interaction exams routed to Codex,
    # for every item — including ones whose draw would otherwise explore.
    for item in ITEMS[:500]:
        assert route("author_interaction", item, config=CFG, implementer_family="claude") == \
            ("codex", "forced")
        assert route("author_interaction", item, config=CFG, implementer_family="codex") == \
            ("claude", "forced")
    # unknown implementer: ordinary routing
    assert route("author_interaction", _routed_item("author_interaction"), config=CFG) == \
        ("claude", "routed")


def test_forced_route_never_falls_back_to_the_implementer():
    with pytest.raises(RoutingUnavailable):
        route("author_interaction", ITEMS[0], config=CFG, implementer_family="claude",
              available=("claude",))


def test_implementer_family_is_ignored_for_other_stages():
    item = _routed_item("implement")
    assert route("implement", item, config=CFG, implementer_family="claude") == ("claude", "routed")


def test_unavailable_family_reroutes_to_the_remaining_one():
    item = _routed_item("implement")
    assert route("implement", item, config=CFG, available=("codex",)) == ("codex", "routed")
    explore = _explore_item("implement")          # would explore to codex
    assert route("implement", explore, config=CFG, available=("claude",)) == ("claude", "routed")
    assert route("implement", explore, config=CFG, available=iter(["claude"])) == ("claude", "routed")
    with pytest.raises(RoutingUnavailable):
        route("implement", item, config=CFG, available=())


def test_terminating_stages_need_a_second_family():
    assert TERMINATING_STAGES and all(is_terminating(s) for s in TERMINATING_STAGES)
    assert not is_terminating("implement")
    for fam in FAMILIES:
        assert second_opinion_family(fam) == other_family(fam) != fam
    with pytest.raises(SingleFamilyTermination):
        second_opinion_family("claude", available=("claude",))
    # the first opinion is still routable (an `ours_wrong` triage needs no second opinion)
    assert route("triage", _routed_item("triage"), config=CFG, available=("claude",))[0] == "claude"
    assert issubclass(SingleFamilyTermination, RoutingUnavailable)


def test_unknown_stage_rejected():
    with pytest.raises(ValueError):
        route("vibes", "x", config=CFG)


# --- scorecard-driven switching -----------------------------------------------------------

def _attempts(stage, family, n, cost, *, model=None, pv="implement@1", start=0):
    model = model if model is not None else CFG.models[family]["model"]
    return [Attempt(attempt_id=f"{family}-{pv}-{start + i:04d}", run_id="r",
                    ts="2026-10-01T00:00:00Z", stage=stage, item=f"card:X-{i}", family=family,
                    model=model, effort=None, prompt_version=pv, assignment="routed",
                    outcome="accepted", usage=Usage(cost_usd=cost)) for i in range(n)]


def _card(claude_n, codex_n, claude_cost=1.0, codex_cost=0.2, pv="implement@1"):
    return Scorecard(_attempts("implement", "claude", claude_n, claude_cost, pv=pv)
                     + _attempts("implement", "codex", codex_n, codex_cost, pv=pv))


def test_spec_insufficient_evidence_keeps_the_default():
    card = _card(40, 29)        # codex is cheaper but has only 29 samples
    item = _routed_item("implement")
    assert route("implement", item, config=CFG, scorecard=card, prompt_version="implement@1") == \
        ("claude", "routed")


def test_spec_clear_winner_routes_to_the_cheaper_family_keeping_exploration():
    card = _card(40, 40)
    assert scorecard_choice("implement", config=CFG, scorecard=card,
                            prompt_version="implement@1") == "codex"
    assert route("implement", _routed_item("implement"), config=CFG, scorecard=card,
                 prompt_version="implement@1") == ("codex", "routed")
    # the exploration share now lands on the former default
    assert route("implement", _explore_item("implement"), config=CFG, scorecard=card,
                 prompt_version="implement@1") == ("claude", "explore")


def test_overlapping_intervals_keep_the_default():
    card = _card(40, 40, claude_cost=1.0, codex_cost=0.97)
    assert scorecard_choice("implement", config=CFG, scorecard=card,
                            prompt_version="implement@1") is None


def test_comparison_is_per_prompt_version_and_needs_one():
    card = _card(40, 40, pv="implement@1")
    assert scorecard_choice("implement", config=CFG, scorecard=card, prompt_version=None) is None
    assert scorecard_choice("implement", config=CFG, scorecard=card,
                            prompt_version="implement@2") is None


def test_unpriced_family_cannot_win_or_lose():
    unpriced = [Attempt(**{**a.__dict__, "usage": Usage(input_tokens=100, cost_usd=None)})
                for a in _attempts("implement", "codex", 40, 0.0)]
    card = Scorecard(_attempts("implement", "claude", 40, 1.0) + unpriced)
    assert scorecard_choice("implement", config=CFG, scorecard=card,
                            prompt_version="implement@1") is None


def test_configured_model_filters_the_comparison():
    # 40 claude attempts on a different model than the configured one do not count
    card = Scorecard(_attempts("implement", "claude", 40, 5.0, model="opus")
                     + _attempts("implement", "codex", 40, 0.2))
    assert scorecard_choice("implement", config=CFG, scorecard=card,
                            prompt_version="implement@1") is None
