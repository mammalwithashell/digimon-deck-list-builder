"""Tests for the multi-signal set clusterer (task 6 / 6.4)."""

import json
import io

from data_paths import CARDS_JSON

from tools.author_set.set_resolver import resolve_set
from tools.author_set.clusterer import (
    cluster,
    distinctive_traits,
    reference_edges,
    in_set_name_index,
)


def _c(cid, name, *, level=4, kind="Digimon", traits=None, eff=""):
    return {
        "card_id": cid,
        "card_name_eng": name,
        "card_kind": kind,
        "level": level,
        "type_eng": traits or [],
        "card_colors": ["Red"],
        "effect_description_eng": eff,
        "inherited_effect_description_eng": "",
        "security_effect_description_eng": "",
    }


def test_reference_edges_link_named_cards():
    cards = {
        "S-1": _c("S-1", "Pulsemon", level=3),
        "S-2": _c("S-2", "Tai", kind="Tamer", level=None, eff="play [Pulsemon] from hand"),
        "S-3": _c("S-3", "Lone", level=4),
    }
    idx = in_set_name_index(cards)
    edges = {tuple(sorted(e)) for e in reference_edges(cards, idx)}
    assert ("S-1", "S-2") in edges
    assert all("S-3" not in e for e in edges)


def test_distinctive_excludes_generic_and_spanning():
    cards = {
        "A": _c("A", "a", traits=["SoC", "Common"]),
        "B": _c("B", "b", traits=["SoC", "Common"]),
        "C": _c("C", "c", traits=["Lesser", "Common"]),
        "D": _c("D", "d", traits=["Common"]),
    }
    dist = distinctive_traits(cards, max_fraction=0.5)
    assert "soc" in dist            # on 2/4 = distinctive
    assert "lesser" not in dist     # generic
    assert "common" not in dist     # on 100% -> set-spanning


def test_cluster_forms_slice_via_references_and_orphans():
    # Connect via the PRIMARY signal (name references), not shared trait.
    cards = {
        "S-1": _c("S-1", "Eosmon", level=3),
        "S-2": _c("S-2", "MetalEosmon", level=5, eff="digivolve from [Eosmon]"),
        "S-3": _c("S-3", "Tamer", kind="Tamer", level=None,
                  eff="buff [Eosmon] and [MetalEosmon]"),
        "S-9": _c("S-9", "Random", level=5, traits=["Solo"]),
        "S-8": _c("S-8", "Other", level=6, traits=["Alone"]),
    }
    part = cluster(cards, "TEST", min_slice=3)
    assert len(part.slices) == 1
    assert set(part.slices[0].card_ids) == {"S-1", "S-2", "S-3"}
    assert sorted(part.orphans) == ["S-8", "S-9"]  # labeled, not dropped


def test_orphans_never_dropped_total_conserved():
    cards = {f"S-{i}": _c(f"S-{i}", f"n{i}", traits=[f"t{i}"]) for i in range(5)}
    part = cluster(cards, "TEST", min_slice=3)
    covered = sum(s.size for s in part.slices) + len(part.orphans)
    assert covered == len(cards)


def test_intra_slice_ordering_eggs_then_level_then_tamer_then_option():
    cards = {
        "S-opt": _c("S-opt", "Opt", kind="Option", level=None, traits=["Eos"]),
        "S-egg": _c("S-egg", "Egg", kind="Digi-Egg", level=2, traits=["Eos"]),
        "S-mega": _c("S-mega", "Mega", level=6, traits=["Eos"]),
        "S-rookie": _c("S-rookie", "Rookie", level=3, traits=["Eos"]),
    }
    # all four share the archetype trait; raise the fraction guard for the tiny set.
    part = cluster(cards, "TEST", min_slice=3, max_trait_fraction=1.0)
    assert part.slices[0].card_ids == ["S-egg", "S-rookie", "S-mega", "S-opt"]


def test_archetype_name_inherited_when_overlap():
    cards = {
        "S-1": _c("S-1", "Eosmon", level=3, traits=["Eos"]),
        "S-2": _c("S-2", "Eosmon2", level=4, traits=["Eos"]),
        "S-3": _c("S-3", "Eosmon3", level=5, traits=["Eos"]),
    }
    part = cluster(cards, "TEST", min_slice=3, max_trait_fraction=1.0,
                   archetype_map={"SoC Control": ["S-1", "S-2", "X-9"]})
    assert part.slices[0].name == "SoC Control"


def test_bt17_recovers_marquee_archetypes():
    # Integration: real BT17 should yield slices whose marquees include the set's
    # featured archetypes (Pulsemon / Eosmon / Argomon / Diaboromon).
    cards = json.load(io.open(CARDS_JSON, encoding="utf-8"))
    ids = resolve_set("BT17", cards)
    set_cards = {c: cards[c] for c in ids}
    part = cluster(set_cards, "BT17")
    marquees = {s.marquee for s in part.slices}
    expected = {"pulsemon", "eosmon", "argomon", "diaboromon"}
    assert len(expected & marquees) >= 3, f"got marquees={sorted(marquees)}"
    covered = sum(s.size for s in part.slices) + len(part.orphans)
    assert covered == len(set_cards)
