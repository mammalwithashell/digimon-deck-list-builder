"""Observation-Tensor Completeness & Card-Fidelity Fuzzer (`tensor_fuzzer`).

Strictly READ-ONLY over active tensor contracts (`standard_lite_v2`,
`standard_lite_deck_v2`, `standard_full_v2`): never mutates Rust tensor layouts,
`ACTION_SPACE_SIZE`, card specs, or model checkpoints.

Part A — Deterministic Zero-LLM State-Pair Differential Fuzzer (`run_tensor_completeness_fuzz`):
  Constructs controlled `(state_A, state_B)` pairs in `RustHeadlessGame` /
  `RustDebugGame` across 5 core rule families where optimal play / engine
  resolution differs ONLY by one subtle mechanic:
    1. `opt_spent_vs_unspent` (`EX12-034` Erlangmon `[Once Per Turn]`)
    2. `delay_armed_vs_placing_turn` (`EX12-071` Saneiketsu Invitation `<Delay>`)
    3. `modifier_active_vs_absent` (`BT20-042` Groundramon `CannotUnsuspend`)
    4. `facedown_vs_faceup_source` (`ST23-14` Reina Sakuya & Makoto Kuonji)
    5. `linked_vs_unlinked` (`BT21-009` Gatchmon + `BT24-067` Hackmon `<Link>`)
  Computes `l1_diff = ||obs_A - obs_B||_1`, attributes changed indices to
  `digimon_engine.get_observation_layout(profile)` sections, verifies semantic
  rules divergence on cloned games, optionally runs `plasticity_probe`, and
  emits structured JSON + Markdown gap reports (`collisions` vs `distinguished`).

Part B — LLM Card-Fidelity Set Auditor (`audit_card_set_fidelity`):
  Filters a set (`--set EX12`) or explicit `--card-ids` to implemented cards,
  grounds `Role.AUTHOR` on printed text from `CardTextCatalog`, stages minimal
  boundary probes via `digimon-engine-mcp` (`load_puzzle`, `step`, `events`),
  and writes a read-only Markdown + JSON audit report compatible with `qa/archetype-qa/`.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Mapping, Optional, Sequence

import numpy as np

from . import CardTextCatalog, PrintedCardInfo, resolve_repo_root, slugify_puzzle_id
from .llm_router import ChatMessage, LLMRouter, Role, ToolCall, ToolSpec
from .mcp_client import StdioMcpClient

CORE_RULE_FAMILIES: tuple[str, ...] = (
    "opt_spent_vs_unspent",
    "delay_armed_vs_placing_turn",
    "modifier_active_vs_absent",
    "facedown_vs_faceup_source",
    "linked_vs_unlinked",
)

SUPPORTED_ACTIVE_PROFILES: frozenset[str] = frozenset(
    {"standard_lite_v2", "standard_lite_deck_v2", "standard_full_v2"}
)

VALID_AUDIT_VERDICTS: frozenset[str] = frozenset(
    {"ready", "dsl-gap", "engine-gap", "rules-gap", "data-gap", "test-gap"}
)

CARD_FIDELITY_VERDICT_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "verdict": {
            "type": "string",
            "enum": ["ready", "dsl-gap", "engine-gap", "rules-gap", "data-gap", "test-gap"],
        },
        "severity": {
            "type": "string",
            "enum": ["none", "low", "medium", "high"],
        },
        "summary": {"type": "string"},
        "clauses_checked": {"type": "array", "items": {"type": "string"}},
        "discrepancies": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["verdict", "severity", "summary", "clauses_checked", "discrepancies"],
    "additionalProperties": False,
}


def _layout_get(layout: Any, key: str, default: Any = None) -> Any:
    if isinstance(layout, Mapping):
        return layout.get(key, default)
    return getattr(layout, key, default)


def _ensure_whitebox_importable() -> None:
    root = resolve_repo_root()
    candidates = [
        root / "code",
        root / "code" / "digimon-training-mcp" / "src",
    ]
    for p in candidates:
        s = str(p)
        if p.exists() and s not in sys.path:
            sys.path.insert(0, s)


@dataclass
class SectionDiffSummary:
    """Per-section attribution of `|obs_A - obs_B|` against `get_observation_layout(profile)`."""

    section: str
    offset: int
    size: int
    changed_count: int
    card_id_changed_count: int
    scalar_changed_count: int
    l1_diff: float
    linf_diff: float
    sample_indices: list[int] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "section": self.section,
            "offset": self.offset,
            "size": self.size,
            "changed_count": self.changed_count,
            "card_id_changed_count": self.card_id_changed_count,
            "scalar_changed_count": self.scalar_changed_count,
            "l1_diff": self.l1_diff,
            "linf_diff": self.linf_diff,
            "sample_indices": list(self.sample_indices),
        }


@dataclass
class FamilyFuzzResult:
    """Differential observation-tensor probe outcome for one rule family."""

    family: str
    description: str
    focus_cards: list[str]
    viewer: int
    status: str  # "distinguished" | "collision"
    l1_diff: float
    linf_diff: float
    changed_count: int
    changed_indices: list[int]
    section_attribution: list[SectionDiffSummary]
    legal_actions_differ: bool
    semantic_divergence_verified: bool
    semantic_divergence_detail: str
    plasticity_probe_a: Optional[dict[str, Any]] = None
    plasticity_probe_b: Optional[dict[str, Any]] = None
    pair_separable: Optional[bool] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "family": self.family,
            "description": self.description,
            "focus_cards": list(self.focus_cards),
            "viewer": self.viewer,
            "status": self.status,
            "l1_diff": self.l1_diff,
            "linf_diff": self.linf_diff,
            "changed_count": self.changed_count,
            "changed_indices": list(self.changed_indices),
            "section_attribution": [s.to_dict() for s in self.section_attribution],
            "legal_actions_differ": self.legal_actions_differ,
            "semantic_divergence_verified": self.semantic_divergence_verified,
            "semantic_divergence_detail": self.semantic_divergence_detail,
            "plasticity_probe_a": self.plasticity_probe_a,
            "plasticity_probe_b": self.plasticity_probe_b,
            "pair_separable": self.pair_separable,
        }


@dataclass
class TensorFuzzReport:
    """Structured report from `run_tensor_completeness_fuzz`."""

    profile_id: str
    tensor_size: int
    feature_schema_version: str
    layout_hash: str
    viewer: int
    families_tested: int
    distinguished: list[str]
    collisions: list[str]
    results: list[FamilyFuzzResult]
    json_out_path: Optional[Path] = None
    markdown_out_path: Optional[Path] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema": "tensor-completeness-fuzz-v1",
            "profile_id": self.profile_id,
            "tensor_size": self.tensor_size,
            "feature_schema_version": self.feature_schema_version,
            "layout_hash": self.layout_hash,
            "viewer": self.viewer,
            "families_tested": self.families_tested,
            "distinguished_count": len(self.distinguished),
            "collision_count": len(self.collisions),
            "distinguished": list(self.distinguished),
            "collisions": list(self.collisions),
            "results": [r.to_dict() for r in self.results],
        }

    def to_markdown(self) -> str:
        lines = [
            f"# Observation Tensor Completeness Report (`{self.profile_id}`)",
            "",
            f"- **Feature Schema**: `{self.feature_schema_version}` (`tensor_size={self.tensor_size}`)",
            f"- **Layout Hash**: `{self.layout_hash}`",
            f"- **Viewer Seat**: `P{self.viewer}`",
            f"- **Summary**: `{len(self.distinguished)}` distinguished, `{len(self.collisions)}` collisions across `{self.families_tested}` core rule families.",
            "",
            "| Family | Status | `||obs_A - obs_B||_1` | Changed Slots | Sections | Semantic Divergence |",
            "|---|---|---:|---:|---|---|",
        ]
        for r in self.results:
            secs = ", ".join(f"`{s.section}` ({s.changed_count})" for s in r.section_attribution) or "*(none)*"
            lines.append(
                f"| `{r.family}` | **{r.status.upper()}** | `{r.l1_diff:.4f}` | "
                f"`{r.changed_count}` | {secs} | {r.semantic_divergence_detail} |"
            )

        if self.collisions:
            lines.extend(["", "## Tensor Blindness Collisions (`l1_diff == 0.0`)", ""])
            for r in self.results:
                if r.status == "collision":
                    lines.append(
                        f"- **`{r.family}`** ({', '.join(r.focus_cards)}): {r.description} — "
                        f"{r.semantic_divergence_detail}"
                    )

        if self.distinguished:
            lines.extend(["", "## Distinguished Families (`l1_diff > 0.0`)", ""])
            for r in self.results:
                if r.status == "distinguished":
                    secs = ", ".join(
                        f"`{s.section}` (L1={s.l1_diff:.4f}, indices={s.sample_indices[:6]})"
                        for s in r.section_attribution
                    )
                    lines.append(
                        f"- **`{r.family}`** ({', '.join(r.focus_cards)}): {secs}"
                    )
        return "\n".join(lines) + "\n"


@dataclass
class _StatePairSpec:
    family: str
    description: str
    focus_cards: list[str]
    game_a: Any
    game_b: Any
    scenario_a: dict[str, Any]
    scenario_b: dict[str, Any]
    target_action_a: int
    avoid_actions_a: list[int]
    target_action_b: int
    avoid_actions_b: list[int]
    semantic_divergence_verified: bool
    semantic_divergence_detail: str


def attribute_tensor_diff_to_layout(
    obs_a: np.ndarray,
    obs_b: np.ndarray,
    layout: Any,
) -> tuple[float, float, list[int], list[SectionDiffSummary]]:
    """Compute `l1_diff`, `linf_diff`, `changed_indices`, and per-section summaries."""
    diff = np.abs(np.asarray(obs_a, dtype=np.float32) - np.asarray(obs_b, dtype=np.float32))
    l1_diff = float(diff.sum())
    linf_diff = float(diff.max()) if diff.size > 0 else 0.0
    changed_indices = [int(i) for i in np.flatnonzero(diff > 0.0)]

    if not changed_indices:
        return l1_diff, linf_diff, [], []

    raw_sections = _layout_get(layout, "sections", []) or []
    card_id_set = {int(i) for i in (_layout_get(layout, "card_id_positions", []) or [])}

    summaries: list[SectionDiffSummary] = []
    assigned: set[int] = set()

    for sec in raw_sections:
        name = str(_layout_get(sec, "name", _layout_get(sec, "id", "unknown")))
        offset = int(_layout_get(sec, "offset", _layout_get(sec, "start", 0)))
        size = int(_layout_get(sec, "size", _layout_get(sec, "len", 0)))
        sec_indices = [idx for idx in changed_indices if offset <= idx < offset + size]
        if not sec_indices:
            continue
        assigned.update(sec_indices)
        sec_vals = diff[sec_indices]
        cid_cnt = sum(1 for idx in sec_indices if idx in card_id_set)
        summaries.append(
            SectionDiffSummary(
                section=name,
                offset=offset,
                size=size,
                changed_count=len(sec_indices),
                card_id_changed_count=cid_cnt,
                scalar_changed_count=len(sec_indices) - cid_cnt,
                l1_diff=float(sec_vals.sum()),
                linf_diff=float(sec_vals.max()),
                sample_indices=sec_indices[:12],
            )
        )

    unassigned = [idx for idx in changed_indices if idx not in assigned]
    if unassigned:
        u_vals = diff[unassigned]
        cid_cnt = sum(1 for idx in unassigned if idx in card_id_set)
        summaries.append(
            SectionDiffSummary(
                section="unmapped",
                offset=unassigned[0],
                size=unassigned[-1] - unassigned[0] + 1,
                changed_count=len(unassigned),
                card_id_changed_count=cid_cnt,
                scalar_changed_count=len(unassigned) - cid_cnt,
                l1_diff=float(u_vals.sum()),
                linf_diff=float(u_vals.max()),
                sample_indices=unassigned[:12],
            )
        )

    return l1_diff, linf_diff, changed_indices, summaries


# ── 5 Core Rule-Family Controlled Pair Builders ───────────────────────────────


def _build_pair_opt_spent_vs_unspent(profile: str) -> _StatePairSpec:
    """Family 1: `EX12-034` Erlangmon `[All Turns][Once Per Turn]` unspent (`A`) vs spent (`B`)."""
    import digimon_engine as de

    deck1 = ["ST1-01"] * 4 + ["EX12-034"] * 4 + ["ST1-03"] * 46
    deck2 = ["ST1-01"] * 4 + ["ST1-03"] * 50

    pre_b_setup = {
        "state": {"memory": 5, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["ST1-03", "ST1-03"],
                "field": [{"stack": ["EX12-034"], "is_suspended": False, "turn_played": 1}],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [
                    {"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1},
                    {"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1},
                ],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 29,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    def _make_b() -> tuple[Any, list[int]]:
        gb = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        gb.skip_mulligan()
        gb.apply_scenario(json.dumps(pre_b_setup))
        prefix = [0]
        gb.step(0)  # Play first ST1-03 -> triggers Erlangmon [All Turns][OPT] bounce
        while gb.get_pending_selection():
            pick = int(gb.legal_decoded_actions()[0]["action_id"])
            gb.step(pick)
            prefix.append(pick)
        return gb, prefix

    game_b, prefix_b = _make_b()
    scen_post_b = game_b.to_scenario()
    staged_a_payload = {"state": scen_post_b["state"], "zones": scen_post_b["zones"]}

    def _make_a() -> Any:
        ga = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        ga.skip_mulligan()
        ga.apply_scenario(json.dumps(staged_a_payload))
        return ga

    game_a = _make_a()

    # Verify semantic rules divergence on fresh clones: playing second ST1-03 triggers bounce in A, not in B.
    probe_a = _make_a()
    probe_b, _ = _make_b()
    probe_a.step(0)
    probe_b.step(0)
    diverged = (probe_a.get_pending_selection() is not None) and (probe_b.get_pending_selection() is None)

    return _StatePairSpec(
        family="opt_spent_vs_unspent",
        description="EX12-034 Erlangmon [All Turns][Once Per Turn] bounce unspent (A) vs already triggered once this turn via prefix action (B)",
        focus_cards=["EX12-034", "ST1-03"],
        game_a=game_a,
        game_b=game_b,
        scenario_a={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": staged_a_payload["state"],
            "zones": staged_a_payload["zones"],
            "prefix_actions": [],
        },
        scenario_b={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": pre_b_setup["state"],
            "zones": pre_b_setup["zones"],
            "prefix_actions": prefix_b,
        },
        target_action_a=0,
        avoid_actions_a=[62],
        target_action_b=114,
        avoid_actions_b=[0],
        semantic_divergence_verified=diverged,
        semantic_divergence_detail=(
            "Playing ST1-03 opens EX12-034 bounce prompt in state_A (OPT available) vs no trigger in state_B (OPT spent)"
        ),
    )


def _build_pair_delay_armed_vs_placing_turn(profile: str) -> _StatePairSpec:
    """Family 2: `EX12-071` Saneiketsu Invitation `<Delay>` placed earlier turn (`A`) vs placing turn (`B`)."""
    import digimon_engine as de

    deck1 = (
        ["ST1-01"] * 4
        + ["EX12-071"] * 4
        + ["EX12-006"] * 8
        + ["EX12-015"] * 4
        + ["EX12-019"] * 4
        + ["ST1-03"] * 30
    )
    deck2 = ["ST1-01"] * 4 + ["ST1-03"] * 50

    setup_a = {
        "state": {"memory": 2, "phase": "Main", "turn": 1, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["EX12-071", "EX12-006", "EX12-006", "EX12-019"],
                "field": [{"stack": ["EX12-015"], "is_suspended": False, "turn_played": 1}],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 33,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": [],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 31,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    setup_b = {
        "state": {"memory": 6, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["EX12-071", "EX12-006", "EX12-006", "EX12-019", "ST1-03"],
                "field": [{"stack": ["EX12-015"], "is_suspended": False, "turn_played": 1}],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 32,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    def _make_a() -> tuple[Any, list[int]]:
        ga = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        ga.skip_mulligan()
        ga.apply_scenario(json.dumps(setup_a))
        ga.step(0)  # Play EX12-071 on Turn 1 (cost 3 -> mem=-1)
        discard_act = int(ga.legal_decoded_actions()[0]["action_id"])
        ga.step(discard_act)  # Trash EX12-006 -> draw 2 -> place EX12-071 -> P2 Turn 2 Breeding
        ga.step(62)  # P2 Breeding pass
        ga.step(62)  # P2 Main pass -> P1 Turn 3 Breeding (mem=3)
        ga.step(62)  # P1 Breeding pass -> P1 Turn 3 Main (mem=3, turn_played=1 < turn=3)
        return ga, [0, discard_act, 62, 62, 62]

    def _make_b() -> tuple[Any, list[int]]:
        gb = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        gb.skip_mulligan()
        gb.apply_scenario(json.dumps(setup_b))
        gb.step(0)  # Play EX12-071 on Turn 3 (cost 3 -> mem=3)
        discard_act = int(gb.legal_decoded_actions()[0]["action_id"])
        gb.step(discard_act)  # Trash EX12-006 -> draw 2 -> place EX12-071 (turn_played=3 == turn=3)
        return gb, [0, discard_act]

    game_a, prefix_a = _make_a()
    game_b, prefix_b = _make_b()

    probe_a, _ = _make_a()
    probe_b, _ = _make_b()
    probe_a.step(0)  # Play EX12-006 [SW]
    probe_b.step(0)
    diverged = (probe_a.get_pending_selection() is not None) and (probe_b.get_pending_selection() is None)

    return _StatePairSpec(
        family="delay_armed_vs_placing_turn",
        description="EX12-071 Saneiketsu Invitation <Delay> placed on Turn 1 (turn_played < turn, armed) vs placed on Turn 3 (turn_played == turn, unarmed)",
        focus_cards=["EX12-071", "EX12-015", "EX12-019", "EX12-006"],
        game_a=game_a,
        game_b=game_b,
        scenario_a={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": setup_a["state"],
            "zones": setup_a["zones"],
            "prefix_actions": prefix_a,
        },
        scenario_b={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": setup_b["state"],
            "zones": setup_b["zones"],
            "prefix_actions": prefix_b,
        },
        target_action_a=0,
        avoid_actions_a=[62],
        target_action_b=62,
        avoid_actions_b=[0],
        semantic_divergence_verified=diverged,
        semantic_divergence_detail=(
            "Playing [SW] EX12-006 triggers EX12-071 <Delay> free digivolve in state_A (armed) vs no trigger in state_B (placing turn)"
        ),
    )


def _build_pair_modifier_active_vs_absent(profile: str) -> _StatePairSpec:
    """Family 3: `BT20-042` Groundramon `CannotUnsuspend` lock active (`B`) vs absent (`A`)."""
    import digimon_engine as de

    deck1 = ["ST1-01"] * 4 + ["BT20-042"] * 4 + ["ST1-03"] * 46
    deck2 = ["ST1-01"] * 4 + ["ST1-03"] * 50

    pre_b_setup = {
        "state": {"memory": 9, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["BT20-042"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [{"stack": ["ST1-03"], "is_suspended": False, "turn_played": 1}],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    def _make_b() -> tuple[Any, list[int]]:
        gb = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        gb.skip_mulligan()
        gb.apply_scenario(json.dumps(pre_b_setup))
        prefix = [0]
        gb.step(0)  # Play BT20-042 -> suspend + CannotUnsuspend lock on P2's ST1-03
        while gb.get_pending_selection():
            pick = int(gb.legal_decoded_actions()[0]["action_id"])
            gb.step(pick)
            prefix.append(pick)
        return gb, prefix

    game_b, prefix_b = _make_b()
    scen_post_b = game_b.to_scenario()
    staged_a_payload = {"state": scen_post_b["state"], "zones": scen_post_b["zones"]}

    def _make_a() -> Any:
        ga = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        ga.skip_mulligan()
        ga.apply_scenario(json.dumps(staged_a_payload))
        return ga

    game_a = _make_a()

    # Passing turn (62) unsuspends P2's ST1-03 in state_A, while CannotUnsuspend keeps it suspended in state_B.
    probe_a = _make_a()
    probe_b, _ = _make_b()
    probe_a.step(62)
    probe_b.step(62)
    susp_a = bool(probe_a.to_scenario()["zones"]["2"]["field"][0]["is_suspended"])
    susp_b = bool(probe_b.to_scenario()["zones"]["2"]["field"][0]["is_suspended"])
    diverged = (susp_a is False) and (susp_b is True)

    return _StatePairSpec(
        family="modifier_active_vs_absent",
        description="Opponent ST1-03 merely suspended (A) vs suspended with BT20-042 Groundramon CannotUnsuspend modifier active (B) at identical zones/memory/phase",
        focus_cards=["BT20-042", "ST1-03"],
        game_a=game_a,
        game_b=game_b,
        scenario_a={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": staged_a_payload["state"],
            "zones": staged_a_payload["zones"],
            "prefix_actions": [],
        },
        scenario_b={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": pre_b_setup["state"],
            "zones": pre_b_setup["zones"],
            "prefix_actions": prefix_b,
        },
        target_action_a=62,
        avoid_actions_a=[],
        target_action_b=62,
        avoid_actions_b=[],
        semantic_divergence_verified=diverged,
        semantic_divergence_detail=(
            f"After passing turn, opponent ST1-03 unsuspends in state_A (is_suspended={susp_a}) vs stays locked in state_B (is_suspended={susp_b})"
        ),
    )


def _build_pair_facedown_vs_faceup_source(profile: str) -> _StatePairSpec:
    """Family 4: `ST23-14` Tamer with face-up bottom source (`A`) vs face-down bottom source (`B`)."""
    import digimon_engine as de

    deck1 = ["ST1-01"] * 4 + ["ST23-14"] * 4 + ["ST23-06"] * 4 + ["ST1-03"] * 42
    deck2 = ["ST1-01"] * 4 + ["ST1-03"] * 50

    pre_b_setup = {
        "state": {"memory": 6, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["ST23-14", "ST1-03"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    def _make_b() -> tuple[Any, list[int]]:
        gb = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
        gb.skip_mulligan()
        gb.apply_scenario(json.dumps(pre_b_setup))
        prefix = [0]
        gb.step(0)  # Play ST23-14 -> choose index 0 ("Place the top card of your deck face down under this Tamer")
        while gb.get_pending_selection():
            pick = int(gb.legal_decoded_actions()[0]["action_id"])
            gb.step(pick)
            prefix.append(pick)
        return gb, prefix

    game_b, prefix_b = _make_b()
    scen_post_b = game_b.to_scenario()
    staged_a_payload = {"state": scen_post_b["state"], "zones": scen_post_b["zones"]}

    game_a = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
    game_a.skip_mulligan()
    game_a.apply_scenario(json.dumps(staged_a_payload))

    ui_a = game_a.to_ui_json()
    ui_b = game_b.to_ui_json()
    src_a = ((ui_a.get("player1") or {}).get("battleArea") or [{}])[0].get("sources") or []
    src_b = ((ui_b.get("player1") or {}).get("battleArea") or [{}])[0].get("sources") or []
    diverged = (src_a != src_b) or (
        float(np.abs(np.asarray(game_a.get_board_tensor(1)) - np.asarray(game_b.get_board_tensor(1))).sum()) > 0.0
    )

    return _StatePairSpec(
        family="facedown_vs_faceup_source",
        description="ST23-14 Reina Sakuya & Makoto Kuonji with face-up ST1-03 digivolution card (A) vs face-down ST1-03 digivolution card placed via [On Play] (B)",
        focus_cards=["ST23-14", "ST1-03"],
        game_a=game_a,
        game_b=game_b,
        scenario_a={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": staged_a_payload["state"],
            "zones": staged_a_payload["zones"],
            "prefix_actions": [],
        },
        scenario_b={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": pre_b_setup["state"],
            "zones": pre_b_setup["zones"],
            "prefix_actions": prefix_b,
        },
        target_action_a=0,
        avoid_actions_a=[62],
        target_action_b=62,
        avoid_actions_b=[0],
        semantic_divergence_verified=diverged,
        semantic_divergence_detail=(
            "Bottom digivolution source under ST23-14 is face-up in state_A vs face-down (card identity hidden & eligible for Beatbreak face-down trash costs) in state_B"
        ),
    )


def _build_pair_linked_vs_unlinked(profile: str) -> _StatePairSpec:
    """Family 5: `BT21-009` Gatchmon linked with `BT24-067` Hackmon (`A`) vs unlinked (`B`)."""
    import digimon_engine as de

    deck1 = ["ST1-01"] * 4 + ["BT21-009"] * 4 + ["BT24-067"] * 4 + ["ST1-03"] * 42
    deck2 = ["ST1-01"] * 4 + ["ST1-03"] * 50

    setup_a = {
        "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [
                    {
                        "stack": ["BT21-009"],
                        "is_suspended": False,
                        "turn_played": 1,
                        "linked_cards": ["BT24-067"],
                    }
                ],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    setup_b = {
        "state": {"memory": 3, "phase": "Main", "turn": 3, "first_player": 1},
        "zones": {
            "1": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [
                    {
                        "stack": ["BT21-009"],
                        "is_suspended": False,
                        "turn_played": 1,
                    }
                ],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
            "2": {
                "breeding": None,
                "hand": ["ST1-03"],
                "field": [],
                "security": ["ST1-03"] * 3,
                "trash": [],
                "deck_top": ["ST1-03"] * 30,
                "egg_deck": ["ST1-01"] * 4,
            },
        },
    }

    game_a = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
    game_a.skip_mulligan()
    game_a.apply_scenario(json.dumps(setup_a))

    game_b = de.RustHeadlessGame(deck1, deck2, seed=0, observation_profile=profile)
    game_b.skip_mulligan()
    game_b.apply_scenario(json.dumps(setup_b))

    linked_a = game_a.to_scenario()["zones"]["1"]["field"][0].get("linked_cards") or []
    linked_b = game_b.to_scenario()["zones"]["1"]["field"][0].get("linked_cards") or []

    return _StatePairSpec(
        family="linked_vs_unlinked",
        description="BT21-009 Gatchmon with linked Appmon BT24-067 Hackmon (A) vs unlinked BT21-009 (B)",
        focus_cards=["BT21-009", "BT24-067"],
        game_a=game_a,
        game_b=game_b,
        scenario_a={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": setup_a["state"],
            "zones": setup_a["zones"],
            "prefix_actions": [],
        },
        scenario_b={
            "decks": {"1": deck1, "2": deck2},
            "seed": 0,
            "state": setup_b["state"],
            "zones": setup_b["zones"],
            "prefix_actions": [],
        },
        target_action_a=114,
        avoid_actions_a=[62],
        target_action_b=62,
        avoid_actions_b=[114],
        semantic_divergence_verified=(len(linked_a) == 1 and len(linked_b) == 0),
        semantic_divergence_detail=(
            f"Host permanent carries linked_cards={linked_a} in state_A vs linked_cards={linked_b} in state_B"
        ),
    )


_FAMILY_BUILDERS: dict[str, Callable[[str], _StatePairSpec]] = {
    "opt_spent_vs_unspent": _build_pair_opt_spent_vs_unspent,
    "delay_armed_vs_placing_turn": _build_pair_delay_armed_vs_placing_turn,
    "modifier_active_vs_absent": _build_pair_modifier_active_vs_absent,
    "facedown_vs_faceup_source": _build_pair_facedown_vs_faceup_source,
    "linked_vs_unlinked": _build_pair_linked_vs_unlinked,
}


def run_tensor_completeness_fuzz(
    *,
    profile: str = "standard_lite_v2",
    families: Optional[Sequence[str]] = None,
    viewer: int = 1,
    checkpoint_zip: Optional[Path | str] = None,
    plasticity_steps: int = 10,
    plasticity_lr: float = 1e-3,
    json_out_path: Optional[Path] = None,
    markdown_out_path: Optional[Path] = None,
) -> TensorFuzzReport:
    """Run deterministic zero-LLM observation-tensor completeness fuzzing across core rule families."""
    if profile not in SUPPORTED_ACTIVE_PROFILES:
        raise ValueError(
            f"Unsupported profile {profile!r}; expected one of {sorted(SUPPORTED_ACTIVE_PROFILES)}"
        )
    if int(viewer) not in (1, 2):
        raise ValueError(f"viewer must be 1 or 2, got {viewer!r}")

    import digimon_engine as de

    wanted_families = list(families) if families else list(CORE_RULE_FAMILIES)
    unknown = [f for f in wanted_families if f not in _FAMILY_BUILDERS]
    if unknown:
        raise ValueError(
            f"Unknown rule families {unknown!r}; valid families: {list(CORE_RULE_FAMILIES)}"
        )

    layout = de.get_observation_layout(profile)
    tensor_size = int(_layout_get(layout, "tensor_size"))
    feature_schema = str(_layout_get(layout, "feature_schema_version", ""))
    layout_hash = str(_layout_get(layout, "layout_hash", ""))

    whitebox_probe_fn: Optional[Callable[..., dict[str, Any]]] = None
    if checkpoint_zip is not None:
        _ensure_whitebox_importable()
        from digimon_training_mcp.whitebox import plasticity_probe as _wb_probe

        whitebox_probe_fn = _wb_probe

    results: list[FamilyFuzzResult] = []
    distinguished: list[str] = []
    collisions: list[str] = []

    for fam_name in wanted_families:
        spec = _FAMILY_BUILDERS[fam_name](profile)
        obs_a = np.asarray(spec.game_a.get_board_tensor(int(viewer)), dtype=np.float32)
        obs_b = np.asarray(spec.game_b.get_board_tensor(int(viewer)), dtype=np.float32)

        l1_diff, linf_diff, changed_indices, sec_summaries = attribute_tensor_diff_to_layout(
            obs_a, obs_b, layout
        )

        legal_a = [int(d["action_id"]) for d in spec.game_a.legal_decoded_actions()]
        legal_b = [int(d["action_id"]) for d in spec.game_b.legal_decoded_actions()]
        legal_differ = legal_a != legal_b

        status = "collision" if l1_diff == 0.0 else "distinguished"
        if status == "collision":
            collisions.append(fam_name)
        else:
            distinguished.append(fam_name)

        probe_a_res: Optional[dict[str, Any]] = None
        probe_b_res: Optional[dict[str, Any]] = None
        pair_sep: Optional[bool] = None
        if whitebox_probe_fn is not None and checkpoint_zip is not None:
            probe_a_res = whitebox_probe_fn(
                checkpoint_path=Path(checkpoint_zip),
                scenario_json=spec.scenario_a,
                target_action_id=spec.target_action_a,
                avoid_action_ids=spec.avoid_actions_a,
                steps=int(plasticity_steps),
                lr=float(plasticity_lr),
            )
            probe_b_res = whitebox_probe_fn(
                checkpoint_path=Path(checkpoint_zip),
                scenario_json=spec.scenario_b,
                target_action_id=spec.target_action_b,
                avoid_action_ids=spec.avoid_actions_b,
                steps=int(plasticity_steps),
                lr=float(plasticity_lr),
            )
            pair_sep = bool(
                l1_diff > 0.0
                and probe_a_res.get("ok") is True
                and probe_a_res.get("learnable") is True
                and probe_b_res.get("ok") is True
                and probe_b_res.get("learnable") is True
            )

        results.append(
            FamilyFuzzResult(
                family=fam_name,
                description=spec.description,
                focus_cards=spec.focus_cards,
                viewer=int(viewer),
                status=status,
                l1_diff=l1_diff,
                linf_diff=linf_diff,
                changed_count=len(changed_indices),
                changed_indices=changed_indices,
                section_attribution=sec_summaries,
                legal_actions_differ=legal_differ,
                semantic_divergence_verified=spec.semantic_divergence_verified,
                semantic_divergence_detail=spec.semantic_divergence_detail,
                plasticity_probe_a=probe_a_res,
                plasticity_probe_b=probe_b_res,
                pair_separable=pair_sep,
            )
        )

    report = TensorFuzzReport(
        profile_id=profile,
        tensor_size=tensor_size,
        feature_schema_version=feature_schema,
        layout_hash=layout_hash,
        viewer=int(viewer),
        families_tested=len(results),
        distinguished=distinguished,
        collisions=collisions,
        results=results,
        json_out_path=json_out_path,
        markdown_out_path=markdown_out_path,
    )

    if json_out_path is not None:
        json_out_path.parent.mkdir(parents=True, exist_ok=True)
        json_out_path.write_text(json.dumps(report.to_dict(), indent=2) + "\n", encoding="utf-8")
    if markdown_out_path is not None:
        markdown_out_path.parent.mkdir(parents=True, exist_ok=True)
        markdown_out_path.write_text(report.to_markdown(), encoding="utf-8")

    return report


# ── Part B: Read-Only LLM Card-Fidelity Set Auditor (`audit_card_set_fidelity`) ─


@dataclass
class CardFidelityFinding:
    card_id: str
    card_name: str
    verdict: str  # "ready" | "dsl-gap" | "engine-gap" | "rules-gap" | "data-gap" | "test-gap"
    severity: str  # "none" | "low" | "medium" | "high"
    summary: str
    clauses_checked: list[str] = field(default_factory=list)
    discrepancies: list[str] = field(default_factory=list)
    probes_executed: int = 0
    observed_event_count: int = 0
    tool_turns_used: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "card_id": self.card_id,
            "card_name": self.card_name,
            "verdict": self.verdict,
            "severity": self.severity,
            "summary": self.summary,
            "clauses_checked": list(self.clauses_checked),
            "discrepancies": list(self.discrepancies),
            "probes_executed": self.probes_executed,
            "observed_event_count": self.observed_event_count,
            "tool_turns_used": self.tool_turns_used,
        }


@dataclass
class CardSetFidelityReport:
    scope_label: str
    cards_audited: int
    ready_count: int
    gap_count: int
    findings: list[CardFidelityFinding]
    json_out_path: Optional[Path] = None
    markdown_out_path: Optional[Path] = None

    def to_dict(self) -> dict[str, Any]:
        by_verdict: dict[str, int] = {}
        for f in self.findings:
            by_verdict[f.verdict] = by_verdict.get(f.verdict, 0) + 1
        return {
            "schema": "card-set-fidelity-audit-v1",
            "scope": self.scope_label,
            "cards_audited": self.cards_audited,
            "ready_count": self.ready_count,
            "gap_count": self.gap_count,
            "counts_by_verdict": by_verdict,
            "findings": [f.to_dict() for f in self.findings],
        }

    def to_markdown(self) -> str:
        lines = [
            f"# Card-Fidelity Audit Report — {self.scope_label}",
            "",
            f"{self.cards_audited} implemented card(s) audited; **{self.ready_count} ready**, "
            f"**{self.gap_count} potential gap(s)**.",
            "",
            "> Read-only boundary probe report (`tools.puzzle_lab.tensor_fuzzer`). Verify against printed card text (`data/cards.json`) and `docs/RULES_CONTEXT.md`.",
            "",
        ]
        grouped: dict[str, list[CardFidelityFinding]] = {}
        for f in self.findings:
            grouped.setdefault(f.verdict, []).append(f)

        for verdict_key in ("engine-gap", "dsl-gap", "rules-gap", "data-gap", "test-gap", "ready"):
            bucket = grouped.get(verdict_key)
            if not bucket:
                continue
            lines.append(f"## {verdict_key.upper()} ({len(bucket)})")
            for item in bucket:
                disc = f" Discrepancies: {'; '.join(item.discrepancies)}" if item.discrepancies else ""
                lines.append(
                    f"- **{item.card_id}** ({item.card_name}) [{item.severity}]: {item.summary}{disc}"
                )
            lines.append("")
        return "\n".join(lines).rstrip() + "\n"


def _build_audit_probe_tools() -> list[ToolSpec]:
    return [
        ToolSpec(
            name="load_puzzle",
            description="Stage a minimal boundary-probe studio-puzzle-v1 object into digimon-engine-mcp in author mode and return game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "puzzle_json": {"type": "object"},
                    "mode": {"type": "string", "enum": ["author"]},
                },
                "required": ["puzzle_json"],
            },
        ),
        ToolSpec(
            name="legal_actions",
            description="Inspect legal actions for player (0 or 1) on game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "player": {"type": "integer"},
                },
                "required": ["game_id", "player"],
            },
        ),
        ToolSpec(
            name="step",
            description="Execute action_id on game_id and observe resulting phase / pending selection.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "action_id": {"type": "integer"},
                },
                "required": ["game_id", "action_id"],
            },
        ),
        ToolSpec(
            name="events",
            description="Fetch structured engine events emitted on game_id since optional since_seq.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "since_seq": {"type": "integer"},
                },
                "required": ["game_id"],
            },
        ),
        ToolSpec(
            name="inspect_stack",
            description="Inspect top card, inherited sources, linked cards, DP, and modifiers on a battle/breeding slot.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "player": {"type": "integer"},
                    "zone": {"type": "string", "enum": ["battle", "breeding"]},
                    "slot": {"type": "integer"},
                },
                "required": ["game_id", "player"],
            },
        ),
        ToolSpec(
            name="field",
            description="Read full board zones, memory, turn, and phase on game_id.",
            parameters={
                "type": "object",
                "properties": {"game_id": {"type": "string"}},
                "required": ["game_id"],
            },
        ),
        ToolSpec(
            name="modifiers",
            description="Inspect active modifiers on handle {player, index} for game_id.",
            parameters={
                "type": "object",
                "properties": {
                    "game_id": {"type": "string"},
                    "handle": {"type": "object"},
                },
                "required": ["game_id", "handle"],
            },
        ),
        ToolSpec(
            name="pending_selection",
            description="Inspect the currently parked PendingSelection prompt on game_id.",
            parameters={
                "type": "object",
                "properties": {"game_id": {"type": "string"}},
                "required": ["game_id"],
            },
        ),
    ]


def resolve_implemented_audit_cards(
    *,
    card_set: Optional[str] = None,
    card_ids: Optional[Sequence[str]] = None,
    max_cards: Optional[int] = None,
) -> list[str]:
    """Filter requested `--set` prefix and/or explicit `card_ids` to implemented Rust cards."""
    import digimon_engine as de

    implemented = {str(c) for c in de.load_implemented_card_ids() if not str(c).startswith("TOKEN_")}
    selected: list[str] = []

    if card_ids:
        for raw in card_ids:
            cid = str(raw).strip().upper()
            if cid and cid in implemented and cid not in selected:
                selected.append(cid)

    if card_set:
        prefix = str(card_set).strip().upper().rstrip("-") + "-"
        for cid in sorted(implemented):
            if cid.upper().startswith(prefix) and cid not in selected:
                selected.append(cid)

    if not card_set and not card_ids:
        raise ValueError("Provide either card_set (e.g. 'EX12') or explicit card_ids")

    if max_cards is not None and int(max_cards) > 0:
        selected = selected[: int(max_cards)]
    return selected


def audit_card_set_fidelity(
    *,
    router: LLMRouter,
    engine_mcp: StdioMcpClient,
    card_set: Optional[str] = None,
    card_ids: Optional[Sequence[str]] = None,
    max_cards: int = 25,
    max_tool_turns: int = 6,
    cards_json_path: Optional[Path] = None,
    json_out_path: Optional[Path] = None,
    markdown_out_path: Optional[Path] = None,
    card_catalog: Optional[CardTextCatalog] = None,
) -> CardSetFidelityReport:
    """Audit implemented cards against printed text using `Role.AUTHOR` + MCP boundary probes."""
    catalog = card_catalog or CardTextCatalog(cards_json_path=cards_json_path)
    targets = resolve_implemented_audit_cards(
        card_set=card_set,
        card_ids=card_ids,
        max_cards=max_cards,
    )
    scope_label = (
        str(card_set).strip().upper()
        if card_set
        else (", ".join(targets[:4]) + ("..." if len(targets) > 4 else ""))
    ) or "custom"

    avail_tools = [t for t in _build_audit_probe_tools() if engine_mcp.has_tool(t.name)]

    findings: list[CardFidelityFinding] = []
    for cid in targets:
        info: Optional[PrintedCardInfo] = catalog.get(cid)
        card_dict = info.to_prompt_dict() if info is not None else {"card_id": cid, "card_name": cid}
        card_name = str(card_dict.get("card_name") or cid)

        opened_games: list[str] = []
        probes_run = 0
        events_seen = 0

        def _exec_probe(tc: ToolCall) -> dict[str, Any]:
            nonlocal probes_run, events_seen
            args = dict(tc.arguments)
            if tc.name == "load_puzzle":
                args["mode"] = "author"
                res = engine_mcp.call_tool("load_puzzle", args)
                if res.get("game_id"):
                    opened_games.append(str(res["game_id"]))
                    probes_run += 1
                return res
            res = engine_mcp.call_tool(tc.name, args)
            if tc.name == "events":
                ev_list = res.get("events") if isinstance(res.get("events"), list) else res.get("value")
                if isinstance(ev_list, list):
                    events_seen += len(ev_list)
            return res

        prompt_payload = {
            "card": card_dict,
            "instructions": [
                "Inspect printed `effect_text`, `inherited_text`, `security_text`, `link_text`, and `evo_costs`.",
                "Stage 1-2 minimal boundary probes via `load_puzzle` (`mode: \"author\"`), `step`, and `events` to verify triggers, optionality, cost payment, and target filters.",
                "Never approximate: report `ready` only when observed behavior matches printed text; otherwise classify using capability-centric categories (`dsl-gap`, `engine-gap`, `rules-gap`, `data-gap`, `test-gap`).",
            ],
        }
        messages = [
            ChatMessage(
                role="system",
                content=(
                    "You are Role.AUTHOR running a read-only Card-Fidelity Audit in `tools.puzzle_lab`. "
                    "Use `load_puzzle`, `legal_actions`, `step`, `events`, and `inspect_stack` to test "
                    "printed clauses on `digimon-engine-mcp`, then return strict JSON matching the verdict schema."
                ),
            ),
            ChatMessage(role="user", content=json.dumps(prompt_payload, indent=2)),
        ]

        try:
            if avail_tools and max_tool_turns > 0:
                loop_res = router.run_tool_loop(
                    Role.AUTHOR,
                    messages,
                    tools=avail_tools,
                    tool_executor=_exec_probe,
                    max_turns=int(max_tool_turns),
                    final_json_schema=CARD_FIDELITY_VERDICT_SCHEMA,
                    schema_name="card_fidelity_verdict",
                )
                parsed = loop_res.final_response.parsed_json or {}
                turns_used = loop_res.turns_used
            else:
                parsed = router.complete_structured(
                    Role.AUTHOR,
                    messages,
                    json_schema=CARD_FIDELITY_VERDICT_SCHEMA,
                    schema_name="card_fidelity_verdict",
                )
                turns_used = 1
        finally:
            if engine_mcp.has_tool("close_game"):
                for gid in opened_games:
                    try:
                        engine_mcp.call_tool("close_game", {"game_id": gid})
                    except Exception:
                        pass

        verdict = str(parsed.get("verdict") or "test-gap").strip().lower()
        if verdict not in VALID_AUDIT_VERDICTS:
            verdict = "test-gap"
        severity = str(parsed.get("severity") or ("none" if verdict == "ready" else "medium")).lower()

        findings.append(
            CardFidelityFinding(
                card_id=cid,
                card_name=card_name,
                verdict=verdict,
                severity=severity,
                summary=str(parsed.get("summary") or f"Audited {cid}"),
                clauses_checked=[str(x) for x in (parsed.get("clauses_checked") or [])],
                discrepancies=[str(x) for x in (parsed.get("discrepancies") or [])],
                probes_executed=probes_run,
                observed_event_count=events_seen,
                tool_turns_used=turns_used,
            )
        )

    ready_count = sum(1 for f in findings if f.verdict == "ready")
    report = CardSetFidelityReport(
        scope_label=scope_label,
        cards_audited=len(findings),
        ready_count=ready_count,
        gap_count=len(findings) - ready_count,
        findings=findings,
        json_out_path=json_out_path,
        markdown_out_path=markdown_out_path,
    )

    if json_out_path is not None:
        json_out_path.parent.mkdir(parents=True, exist_ok=True)
        json_out_path.write_text(json.dumps(report.to_dict(), indent=2) + "\n", encoding="utf-8")
    if markdown_out_path is not None:
        markdown_out_path.parent.mkdir(parents=True, exist_ok=True)
        markdown_out_path.write_text(report.to_markdown(), encoding="utf-8")

    return report


__all__ = [
    "CORE_RULE_FAMILIES",
    "CardFidelityFinding",
    "CardSetFidelityReport",
    "FamilyFuzzResult",
    "SectionDiffSummary",
    "TensorFuzzReport",
    "attribute_tensor_diff_to_layout",
    "audit_card_set_fidelity",
    "resolve_implemented_audit_cards",
    "run_tensor_completeness_fuzz",
]
