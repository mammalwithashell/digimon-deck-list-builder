"""Decompose a release set into archetype/evolution slices (Phase 3, task 6).

A booster set is a UNION of archetype slices plus orphan staples. The clusterer
recovers those slices from multiple signals:

- name-reference graph  -> PRIMARY connectivity. An in-text bracketed name that
  resolves to another in-set card links them (tamer<->Digimon, option<->Digimon,
  evo lines through named digivolve conditions).
- distinctive traits    -> membership reinforcement. Cards sharing a trait that
  is neither generic nor set-spanning get an edge.
- reference frequency   -> identifies each slice's marquee (most-referenced) card.
- color + level         -> intra-slice ordering (eggs -> Digimon by level -> tamers
  -> options) so prerequisites precede dependents during authoring.

Connected components at/above ``min_slice`` become slices; smaller fragments and
singletons fall into a labeled ``orphan`` bucket (never silently dropped).
Clustering is fuzzy by nature, so the Workflow presents the partition for human
approval (task 6.3) before dispatch.
"""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass, field

_TOKEN_RE = re.compile(r"[\[<]([^\]>]+)[\]>]")

# Digi-Egg generic + obvious non-archetype trait noise that must not create
# membership edges (would merge unrelated lines).
GENERIC_TRAITS = {"lesser", "in-training", "rookie", "champion", "ultimate", "mega"}

_KIND_RANK = {"Digi-Egg": 0, "Digimon": 1, "Tamer": 2, "Option": 3}


@dataclass
class Slice:
    name: str
    card_ids: list[str]
    marquee: str = ""
    traits: list[str] = field(default_factory=list)

    @property
    def size(self) -> int:
        return len(self.card_ids)


@dataclass
class Partition:
    set_prefix: str
    slices: list[Slice] = field(default_factory=list)
    orphans: list[str] = field(default_factory=list)

    def format(self) -> str:
        lines = [f"Set {self.set_prefix}: {len(self.slices)} slices, "
                 f"{len(self.orphans)} orphan staples"]
        for s in self.slices:
            lines.append(f"  • {s.name}  ({s.size} cards, marquee={s.marquee or '-'})")
            lines.append(f"      {', '.join(s.card_ids)}")
        if self.orphans:
            lines.append(f"  • [orphan staples] ({len(self.orphans)})")
            lines.append(f"      {', '.join(self.orphans)}")
        return "\n".join(lines)


# ---- signals ----------------------------------------------------------------

def _norm_name(s: str) -> str:
    return s.strip().lower()


def in_set_name_index(set_cards: dict) -> dict[str, set[str]]:
    """Map lowercased card name -> set of in-set card IDs bearing that name."""
    idx: dict[str, set[str]] = {}
    for cid, c in set_cards.items():
        nm = _norm_name(str(c.get("card_name_eng", "")))
        if nm:
            idx.setdefault(nm, set()).add(cid)
    return idx


def reference_edges(set_cards: dict, name_index: dict[str, set[str]]) -> set[tuple[str, str]]:
    """Edges from bracketed in-text names that resolve to OTHER in-set cards."""
    edges = set()
    for cid, c in set_cards.items():
        text = " ".join(str(c.get(k, "") or "") for k in
                        ("effect_description_eng", "inherited_effect_description_eng",
                         "security_effect_description_eng"))
        for m in _TOKEN_RE.finditer(text):
            tgt_name = _norm_name(m.group(1))
            for tgt in name_index.get(tgt_name, ()):  # noqa: B007
                if tgt != cid:
                    edges.add(tuple(sorted((cid, tgt))))
    return edges


def reference_frequency(set_cards: dict, name_index: dict[str, set[str]]) -> Counter:
    """How often each in-set card NAME is referenced across the set's text."""
    freq: Counter = Counter()
    for c in set_cards.values():
        text = " ".join(str(c.get(k, "") or "") for k in
                        ("effect_description_eng", "inherited_effect_description_eng",
                         "security_effect_description_eng"))
        for m in _TOKEN_RE.finditer(text):
            nm = _norm_name(m.group(1))
            if nm in name_index:
                freq[nm] += 1
    return freq


def distinctive_traits(set_cards: dict, max_fraction: float = 0.5) -> set[str]:
    """Traits shared by >=2 set cards but not generic or set-spanning."""
    n = max(1, len(set_cards))
    counts: Counter = Counter()
    for c in set_cards.values():
        for t in (c.get("type_eng") or []):
            counts[str(t).strip().lower()] += 1
    return {
        t for t, k in counts.items()
        if k >= 2 and k <= max_fraction * n and t not in GENERIC_TRAITS and not t.isdigit()
    }


def trait_edges(set_cards: dict, distinctive: set[str]) -> set[tuple[str, str]]:
    """Edges between cards sharing a distinctive trait."""
    by_trait: dict[str, list[str]] = {}
    for cid, c in set_cards.items():
        for t in (c.get("type_eng") or []):
            tl = str(t).strip().lower()
            if tl in distinctive:
                by_trait.setdefault(tl, []).append(cid)
    edges = set()
    for ids in by_trait.values():
        for i in range(len(ids)):
            for j in range(i + 1, len(ids)):
                edges.add(tuple(sorted((ids[i], ids[j]))))
    return edges


class _UF:
    def __init__(self, items):
        self.p = {i: i for i in items}

    def find(self, x):
        while self.p[x] != x:
            self.p[x] = self.p[self.p[x]]
            x = self.p[x]
        return x

    def union(self, a, b):
        self.p[self.find(a)] = self.find(b)


def _order_cards(card_ids, set_cards) -> list[str]:
    def key(cid):
        c = set_cards.get(cid, {})
        kind = _KIND_RANK.get(c.get("card_kind"), 9)
        lvl = c.get("level")
        lvl = lvl if isinstance(lvl, int) else 99
        return (kind, lvl, cid)
    return sorted(card_ids, key=key)


def _dominant_trait(card_ids, set_cards, distinctive) -> str:
    counts: Counter = Counter()
    for cid in card_ids:
        for t in (set_cards.get(cid, {}).get("type_eng") or []):
            tl = str(t).strip().lower()
            if tl in distinctive:
                counts[tl] += 1
    return counts.most_common(1)[0][0] if counts else ""


def _name_slice(card_ids, set_cards, marquee_name, distinctive, archetype_map) -> str:
    # 1. inherit a deck_library archetype name if this component overlaps one well.
    if archetype_map:
        best, best_overlap = None, 0
        ids = set(card_ids)
        for arch, arch_ids in archetype_map.items():
            ov = len(ids & set(arch_ids))
            if ov > best_overlap:
                best, best_overlap = arch, ov
        if best and best_overlap >= 2:
            return best
    # 2. synthesize from marquee + dominant trait.
    trait = _dominant_trait(card_ids, set_cards, distinctive)
    marquee_title = next(
        (str(set_cards[c].get("card_name_eng")) for c in card_ids
         if _norm_name(str(set_cards[c].get("card_name_eng", ""))) == marquee_name),
        "",
    )
    if marquee_title and trait:
        return f"{marquee_title} / {trait}"
    return marquee_title or trait or "unnamed"


def cluster(
    set_cards: dict,
    set_prefix: str = "",
    *,
    min_slice: int = 3,
    max_trait_fraction: float = 0.5,
    archetype_map: dict | None = None,
) -> Partition:
    """Partition ``set_cards`` (id -> card dict) into slices + an orphan bucket.

    ``max_trait_fraction`` gates which traits create membership edges: a trait on
    more than this fraction of the set is treated as set-spanning (a color/broad
    type) and excluded so it does not merge unrelated lines. The default suits
    real ~100-card sets; tiny synthetic fixtures may raise it.
    """
    name_index = in_set_name_index(set_cards)
    distinctive = distinctive_traits(set_cards, max_fraction=max_trait_fraction)
    freq = reference_frequency(set_cards, name_index)

    # Community sizes per distinctive trait (membership counts).
    trait_size: Counter = Counter()
    for c in set_cards.values():
        for t in (c.get("type_eng") or []):
            tl = str(t).strip().lower()
            if tl in distinctive:
                trait_size[tl] += 1

    # Per-card reference targets (in-set), for attaching trait-less satellites.
    ref_targets: dict[str, set[str]] = {}
    for cid, c in set_cards.items():
        text = " ".join(str(c.get(k, "") or "") for k in
                        ("effect_description_eng", "inherited_effect_description_eng",
                         "security_effect_description_eng"))
        tgts: set[str] = set()
        for m in _TOKEN_RE.finditer(text):
            for tgt in name_index.get(_norm_name(m.group(1)), ()):
                if tgt != cid:
                    tgts.add(tgt)
        ref_targets[cid] = tgts

    # Pass 1 — assign each trait-bearing card to its STRONGEST single community
    # (its distinctive trait with the largest membership). This avoids the
    # transitive chaining that a connected-components merge produces.
    label: dict[str, str] = {}
    satellites: list[str] = []
    for cid, c in set_cards.items():
        cand = [t for t in (str(x).strip().lower() for x in (c.get("type_eng") or []))
                if t in distinctive]
        if cand:
            label[cid] = max(cand, key=lambda t: (trait_size[t], t))
        else:
            satellites.append(cid)

    # Pass 2 — attach trait-less satellites (tamers/options) to the community
    # their references point at most; unresolved satellites become orphans.
    for cid in satellites:
        votes: Counter = Counter()
        for tgt in ref_targets[cid]:
            if tgt in label:
                votes[label[tgt]] += 1
        if votes:
            label[cid] = votes.most_common(1)[0][0]

    # Pass 3 — sub-cluster the still-unlabeled (trait-less, reference-less to any
    # community) cards AMONG THEMSELVES via name references. This recovers pure
    # name-only evolution lines without letting references bridge trait
    # communities (the edges are restricted to unlabeled<->unlabeled).
    unlabeled = [cid for cid in set_cards if cid not in label]
    if unlabeled:
        uf = _UF(unlabeled)
        uset = set(unlabeled)
        for cid in unlabeled:
            for tgt in ref_targets[cid]:
                if tgt in uset:
                    uf.union(cid, tgt)
        for cid in unlabeled:
            label[cid] = f"__ref__{uf.find(cid)}"

    comps: dict[str, list[str]] = {}
    for cid in set_cards:
        comps.setdefault(label[cid], []).append(cid)

    part = Partition(set_prefix=set_prefix)
    for key, ids in comps.items():
        if len(ids) < min_slice:
            part.orphans.extend(ids)
            continue
        # marquee = most-referenced in-set name among this component's cards.
        comp_names = {_norm_name(str(set_cards[c].get("card_name_eng", ""))) for c in ids}
        marquee = max(comp_names, key=lambda nm: freq.get(nm, 0), default="")
        if freq.get(marquee, 0) == 0:
            marquee = ""
        traits = sorted({t for c in ids for t in
                         ((set_cards[c].get("type_eng") or [])) if str(t).strip().lower() in distinctive})
        part.slices.append(Slice(
            name=_name_slice(ids, set_cards, marquee, distinctive, archetype_map),
            card_ids=_order_cards(ids, set_cards),
            marquee=marquee,
            traits=traits,
        ))
    part.slices.sort(key=lambda s: (-s.size, s.name))
    part.orphans = _order_cards(part.orphans, set_cards)
    return part
