import numpy as np

from tools.cloud_search.audit_evolution_imitation import offer, route_masks


def test_phase_aware_routes_do_not_count_selection_action_ids():
    actions = np.array([0, 61, 62, 63, 92, 100, 400, 414, 429, 999, 1002])
    routes = route_masks(actions, 3)
    assert actions[routes["raising"]].tolist() == [414, 429, 999]
    assert actions[routes["battle"]].tolist() == [400]
    assert actions[routes["dna"]].tolist() == [63, 92]
    assert not routes["promotion"].any()
    for phase in [0, 1, 4, 13, 15]:
        assert not any(mask.any() for mask in route_masks(actions, phase).values())
    assert actions[route_masks(actions, 2)["promotion"]].tolist() == [61]


def test_reservoir_is_bounded_reproducible_and_counts_full_population():
    from collections import Counter
    results = []
    for _ in range(2):
        samples, seen, rng = {}, Counter(), np.random.default_rng(42)
        for i in range(100):
            offer(samples, seen, "battle", i, 8, rng)
        assert seen["battle"] == 100
        assert len(set(samples["battle"])) == 8
        assert any(i >= 8 for i in samples["battle"])
        results.append(samples)
    assert results[0] == results[1]
