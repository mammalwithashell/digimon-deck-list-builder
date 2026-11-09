"""Checkpoint-panel schedules: the historical one is frozen; no_mirrors drops every mirror."""
from collections import Counter

import pytest

from tools.cloud_search.local_checkpoint_search import make_schedule

NAMES = [f"arch{i:02d}" for i in range(13)]
# 21 lists: eight archetypes with two exact lists, five with one (the frozen pool's shape).
DECKS = [dict(name=n, source_deck_id=f"{n}-{k}", cards=[]) for i, n in enumerate(NAMES)
         for k in range(2 if i < 8 else 1)]


def blocks(schedule):
    return {r["block"]: r["deck_indices"] for r in schedule}


def appearances(schedule):
    count = Counter()
    for a, b in blocks(schedule).values():
        count[DECKS[a]["name"]] += 2
        count[DECKS[b]["name"]] += 2
    return count


def test_default_schedule_unchanged_with_thirteen_mirror_blocks():
    schedule = make_schedule(DECKS, seed=8926203100)
    assert schedule == make_schedule(DECKS, seed=8926203100, schedule="with_mirrors")
    mirrors = [b for b in blocks(schedule).values() if DECKS[b[0]]["name"] == DECKS[b[1]]["name"]]
    assert len(schedule) == 260 and len(mirrors) == 13


def test_no_mirrors_keeps_balance_and_coverage():
    schedule = make_schedule(DECKS, seed=8926203100, schedule="no_mirrors")
    pairs = blocks(schedule).values()
    assert len(schedule) == 260
    assert all(DECKS[a]["name"] != DECKS[b]["name"] for a, b in pairs)
    # 40 candidate games per archetype = 20 block endpoints x 2 games, as summary() requires.
    assert set(appearances(schedule).values()) == {40}
    assert {i for r in schedule for i in r["deck_indices"]} == set(range(len(DECKS)))
    # Every unordered cross-archetype matchup appears at least once.
    seen = {frozenset((DECKS[a]["name"], DECKS[b]["name"])) for a, b in pairs}
    assert len(seen) == 13 * 12 // 2
    # Seeds stay identical to the historical schedule block for block.
    old = make_schedule(DECKS, seed=8926203100)
    assert [r["game_seed"] for r in schedule] == [r["game_seed"] for r in old]


def test_unknown_schedule_rejected():
    with pytest.raises(ValueError):
        make_schedule(DECKS, seed=8926203100, schedule="mirrors_only")
