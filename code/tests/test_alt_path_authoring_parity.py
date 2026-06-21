"""Authoring-parity guard: every implemented card's YAML alt-digivolve paths must match
the official Bandai DB's printed special-digivolution conditions (gate + cost).

This is the exhaustive backbone of "alt paths are respected in game": it re-runs the
`audit_alt_evo.py` heuristic over EVERY implemented DSL card covered by the official mirror
(`data/card_official.json`) and asserts the set of divergences is exactly the documented
ALLOWLIST below. Any NEW divergence — a card whose YAML alt_paths drift from its printed
routes — fails this test, forcing either a fix or a justified allowlist entry.

Companion guards:
  - `code/digimon-engine/tests/alt_path_reachability.rs` — every simple trait-gated alt is
    reachable in-engine by a base carrying the trait (the route fires).
  - `code/digimon-engine/tests/dsl/trait_parity.rs` — production CardData carries every DSL
    trait (the data feeding the gates is correct).
  - `code/digimon-engine/tests/digivolve_cost_choice_archetypes.rs` — the cost CHOICE
    surfaces on real archetype cards (incl. a production-CardData capstone).

The ALLOWLIST holds two kinds of entries, each justified:
  * false_positive — the YAML is correct; the audit's regex heuristic misparses the printed
    text (combined "[Name]: Cost A / trait: Cost B" lines, formula costs, Assembly-class
    alts). These are NOT bugs.
  * engine_gap — the printed alt needs a `from:` primitive the DSL can't express yet
    (board-state conditionals, negative tri-colour gates, compound multi-card gates).
    Tracked in docs/RUST_ENGINE_GAPS.md / qa/dsl-vocab-gaps.md.

When you legitimately fix an allowlisted card, REMOVE it here (the test prints stale
entries to remind you).
"""
import importlib.util
import json
import os

import pytest

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.abspath(os.path.join(_HERE, "..", ".."))
_AUDIT_PY = os.path.join(_ROOT, "code", "tools", "audit_digivolve", "audit_alt_evo.py")


def _load_audit():
    spec = importlib.util.spec_from_file_location("audit_alt_evo", _AUDIT_PY)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# card_id -> (kind, reason). kind ∈ {"false_positive", "engine_gap"}.
ALLOWLIST = {
    "BT15-101": (
        "engine_gap",
        "Printed alt is a conditional name gate (Tamer presence + opponent DP threshold) "
        "the DSL `from:` predicate can't express.",
    ),
    "BT18-102": (
        "false_positive",
        "Printed alt is an Assembly-class digivolution (play with 10 [Hybrid] cards under), "
        "not a plain digivolve route; the cost-6 heuristic misfires.",
    ),
    "BT19-012": (
        "false_positive",
        "YAML correctly encodes both printed routes ([Shoutmon] name: Cost 4 AND Lv.4 "
        "w/[Xros Heart] trait: Cost 3). The audit regex misparses the combined "
        "'[Shoutmon]: Cost 4/Lv.4 w/[Xros Heart] trait: Cost 3' line as a single "
        "trait-gated cost-4 alt.",
    ),
    "BT21-021": (
        "false_positive",
        "Same combined-text misparse as BT19-012 (sibling): YAML correctly encodes "
        "[Shoutmon] name Cost 4 + Lv.4 w/[Xros Heart]/[Hero] trait Cost 3.",
    ),
    "BT23-013": (
        "engine_gap",
        "The cost-5 alt is conditional ('[Huckmon] while opponent has a 10000+ DP Digimon') "
        "— a board-state gate the DSL can't express. The cost-3 [SaviorHuckmon]/CS-trait "
        "route IS encoded.",
    ),
    "BT24-101": (
        "false_positive",
        "The '[Aegiochusmon] in name: Cost 1 for each security card' alt IS encoded as a "
        "formula cost; the audit can only match literal costs, so it flags a phantom "
        "missing literal-1 alt.",
    ),
    "BT25-084": (
        "engine_gap",
        "Printed alt '[Titamon] w/o 3 colors: Cost 2' combines a name gate with a NEGATIVE "
        "tri-colour condition the DSL can't express.",
    ),
    "EX11-074": (
        "engine_gap",
        "Printed alt requires a compound board-state gate (you have [Shoto Kazama] AND the "
        "base is [GrandGalemon]); the DSL `from:` supports only single gates.",
    ),
}


def test_alt_path_authoring_matches_official_db():
    audit = _load_audit()
    official = json.load(open(audit.OFFICIAL, encoding="utf-8"))["cards"]
    findings = audit.audit(official)
    flagged = sorted({f["card"] for f in findings})

    # Group findings per card for diagnostics.
    by_card = {}
    for f in findings:
        by_card.setdefault(f["card"], []).append(f)

    new_drift = [c for c in flagged if c not in ALLOWLIST]
    if new_drift:
        detail = []
        for c in new_drift:
            for f in by_card[c]:
                detail.append(
                    f"  {c} [{f['type']}]: {f.get('detail', '')} | official: {f.get('official', '')}"
                )
        pytest.fail(
            "Alt-digivolve authoring drift vs the official Bandai DB "
            f"({len(new_drift)} card(s) not in the allowlist):\n"
            + "\n".join(detail)
            + "\n\nEither fix the card's YAML alt_paths to match its printed special-digivolution "
            "(see code/tools/audit_digivolve/audit_alt_evo.py and data/card_bundles/<ID>.md), "
            "or — if the divergence is a heuristic false-positive or a documented engine gap — "
            "add a justified entry to ALLOWLIST in this file."
        )

    # Surface stale allowlist entries (card got fixed) so they can be pruned. Not a failure.
    stale = [c for c in ALLOWLIST if c not in flagged]
    if stale:
        print(
            "\n[alt-path-authoring-parity] allowlist entries no longer flagged (prune them): "
            + ", ".join(stale)
        )
