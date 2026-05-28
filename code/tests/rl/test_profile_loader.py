"""Pure-Python tests for `digimon_gym.agents.reward.profile_loader`.

Most tests use `_parse_and_resolve` directly with literal YAML strings
to avoid filesystem I/O. A small block at the end exercises the
file-based `ProfileLoader` for hot-reload semantics.
"""

from __future__ import annotations

import textwrap
import time
import warnings
from pathlib import Path

import pytest

from digimon_gym.agents.reward.profile_loader import (
    InheritanceCycleError,
    ProfileConfigError,
    ProfileLoader,
    _parse_and_resolve,
)


SYNTH = Path("synthetic.yaml")


def _yaml(s: str) -> str:
    """De-indent a literal YAML block for readability in tests."""
    return textwrap.dedent(s).strip("\n") + "\n"


def _minimal_default_profile_block() -> str:
    """Reusable YAML block with a valid `_default` profile + assignment.
    Tests that need a baseline composable profile import this.
    """
    return _yaml(
        """
        profiles:
          _default:
            components:
              - kind: terminal_outcome
                win_base: 10.0
                fast_win_bonus_max: 5.0
                fast_win_par_steps: 200
                loss: -10.0
                draw: -1.0
              - kind: step_penalty
                weight: -0.001
        assignments:
          _default: _default
        """
    )


# ============================================================================
# Top-level schema validation
# ============================================================================


def test_missing_profiles_key_fails_with_clear_error():
    with pytest.raises(ProfileConfigError, match=r"profiles:"):
        _parse_and_resolve(_yaml("assignments: {}"), SYNTH)


def test_missing_assignments_key_fails():
    with pytest.raises(ProfileConfigError, match=r"assignments:"):
        _parse_and_resolve(_yaml("profiles: {x: {components: []}}"), SYNTH)


def test_missing_default_assignment_fails_at_load():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: step_penalty
                weight: -0.001
        assignments:
          "X": p
        """
    )
    with pytest.raises(ProfileConfigError, match=r"_default"):
        _parse_and_resolve(yaml, SYNTH)


def test_assignment_targeting_undefined_profile_fails():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: step_penalty
                weight: -0.001
        assignments:
          _default: p
          "X": does_not_exist
        """
    )
    with pytest.raises(ProfileConfigError, match=r"does_not_exist"):
        _parse_and_resolve(yaml, SYNTH)


def test_top_level_yaml_must_be_mapping():
    with pytest.raises(ProfileConfigError):
        _parse_and_resolve("- not a mapping\n- still not\n", SYNTH)


# ============================================================================
# Private-profile rules
# ============================================================================


def test_private_profile_can_be_inherited():
    yaml = _yaml(
        """
        profiles:
          _base:
            components:
              - kind: terminal_outcome
                win_base: 10.0
                fast_win_bonus_max: 5.0
                fast_win_par_steps: 200
                loss: -10.0
                draw: -1.0
              - kind: step_penalty
                weight: -0.001
          public:
            inherits: _base
            components: []
        assignments:
          _default: public
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    assert "public" in profs.by_name


def test_private_profile_cannot_be_assigned_to_real_archetype():
    yaml = _yaml(
        """
        profiles:
          _base:
            components:
              - kind: step_penalty
                weight: -0.001
          regular:
            inherits: _base
            components: []
        assignments:
          _default: regular
          "Rocks": _base
        """
    )
    with pytest.raises(ProfileConfigError, match=r"private"):
        _parse_and_resolve(yaml, SYNTH)


def test_default_assignment_may_target_private_profile():
    # Spec carve-out: the `_default` assignment key is allowed to point
    # at a private profile (typically the shipped `_default` profile).
    yaml = _minimal_default_profile_block()
    profs = _parse_and_resolve(yaml, SYNTH)
    assert profs.assignments["_default"] == "_default"


# ============================================================================
# Inheritance
# ============================================================================


def test_child_inherits_parent_components():
    yaml = _yaml(
        """
        profiles:
          _base:
            components:
              - kind: terminal_outcome
                win_base: 10.0
                fast_win_bonus_max: 5.0
                fast_win_par_steps: 200
                loss: -10.0
                draw: -1.0
              - kind: step_penalty
                weight: -0.001
          child:
            inherits: _base
            components: []
        assignments:
          _default: child
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    kinds = [c.kind for c in profs.by_name["child"].components]
    assert "terminal_outcome" in kinds
    assert "step_penalty" in kinds


def test_child_overrides_parent_component_weight():
    # Spec scenario: parent `security_remove { weight: 1.5 }` + child
    # `security_remove { weight: 3.0 }` → resolved weight is 3.0, not 4.5.
    yaml = _yaml(
        """
        profiles:
          _parent:
            components:
              - kind: security_remove
                weight: 1.5
          child:
            inherits: _parent
            components:
              - kind: security_remove
                weight: 3.0
        assignments:
          _default: child
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    components = profs.by_name["child"].components
    sec = [c for c in components if c.kind == "security_remove"]
    assert len(sec) == 1
    assert sec[0].component.weight == 3.0


def test_multiple_play_named_card_with_different_matches_coexist():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: play_named_card
                match:
                  card_id: ["BT17-078"]
                weight: 3.0
              - kind: play_named_card
                match:
                  card_id: ["BT17-015"]
                weight: 2.0
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    plays = [c for c in profs.by_name["p"].components if c.kind == "play_named_card"]
    assert len(plays) == 2


def test_inheritance_cycle_fails_at_load_with_cycle_path():
    yaml = _yaml(
        """
        profiles:
          a:
            inherits: b
            components: []
          b:
            inherits: a
            components: []
        assignments:
          _default: a
        """
    )
    with pytest.raises(InheritanceCycleError) as exc:
        _parse_and_resolve(yaml, SYNTH)
    # Both profiles named in the cycle path.
    assert "a" in exc.value.cycle
    assert "b" in exc.value.cycle


def test_inherits_undefined_profile_fails():
    yaml = _yaml(
        """
        profiles:
          child:
            inherits: not_a_profile
            components: []
        assignments:
          _default: child
        """
    )
    with pytest.raises(ProfileConfigError, match=r"not_a_profile"):
        _parse_and_resolve(yaml, SYNTH)


# ============================================================================
# Unknown component kind
# ============================================================================


def test_unknown_component_kind_fails_at_load_with_registered_kinds_list():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: not_a_real_component
                weight: 1.0
        assignments:
          _default: p
        """
    )
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve(yaml, SYNTH)
    # Spec scenario: error names the unknown kind AND lists registered kinds.
    msg = str(exc.value)
    assert "not_a_real_component" in msg
    assert "step_penalty" in msg


# ============================================================================
# Canonical hash
# ============================================================================


def test_hash_is_sha256_prefixed():
    profs = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    assert profs.content_hash.startswith("sha256:")
    assert len(profs.content_hash) == len("sha256:") + 64  # hex of 32 bytes


def test_hash_invariant_under_key_order_change():
    yaml_a = _yaml(
        """
        profiles:
          _default:
            components:
              - kind: step_penalty
                weight: -0.001
              - kind: terminal_outcome
                win_base: 10.0
                fast_win_bonus_max: 5.0
                fast_win_par_steps: 200
                loss: -10.0
                draw: -1.0
        assignments:
          _default: _default
        """
    )
    yaml_b = _yaml(
        """
        assignments:
          _default: _default
        profiles:
          _default:
            components:
              - weight: -0.001
                kind: step_penalty
              - draw: -1.0
                loss: -10.0
                kind: terminal_outcome
                win_base: 10.0
                fast_win_par_steps: 200
                fast_win_bonus_max: 5.0
        """
    )
    a = _parse_and_resolve(yaml_a, SYNTH)
    b = _parse_and_resolve(yaml_b, SYNTH)
    assert a.content_hash == b.content_hash


def test_hash_differs_when_weight_changes():
    a = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    b = _parse_and_resolve(
        _minimal_default_profile_block().replace("-0.001", "-0.002"), SYNTH,
    )
    assert a.content_hash != b.content_hash


def test_hash_invariant_under_whitespace_change():
    a = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    extra_blank = _minimal_default_profile_block().replace(
        "assignments:", "\n\nassignments:",
    )
    b = _parse_and_resolve(extra_blank, SYNTH)
    assert a.content_hash == b.content_hash


# ============================================================================
# once_per_episode deprecation
# ============================================================================


def test_once_per_episode_emits_deprecation_warning_and_rewrites_budget():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: play_named_card
                match:
                  card_id: ["X"]
                weight: 3.0
                once_per_episode: true
        assignments:
          _default: p
        """
    )
    with pytest.warns(DeprecationWarning, match=r"once_per_episode"):
        profs = _parse_and_resolve(yaml, SYNTH)
    play = [c for c in profs.by_name["p"].components if c.kind == "play_named_card"][0]
    assert play.budget.max_fires_per_episode == 1


# ============================================================================
# Per-component + per-profile budget parsing
# ============================================================================


def test_per_component_budget_parses_three_knobs():
    yaml = _yaml(
        """
        profiles:
          p:
            components:
              - kind: block_event
                weight: 0.15
                budget:
                  max_fires_per_episode: 4
                  max_total_per_episode: 0.5
                  diminishing_returns_factor: 0.6
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    block = profs.by_name["p"].components[0]
    assert block.budget.max_fires_per_episode == 4
    assert block.budget.max_total_per_episode == 0.5
    assert block.budget.diminishing_returns_factor == 0.6


def test_per_profile_budget_cap_and_floor_parse():
    yaml = _yaml(
        """
        profiles:
          p:
            budget:
              per_episode_cap: 12.0
              per_episode_floor: -3.0
            components:
              - kind: step_penalty
                weight: -0.001
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    p = profs.by_name["p"]
    assert p.profile_budget.per_episode_cap == 12.0
    assert p.profile_budget.per_episode_floor == -3.0


# ============================================================================
# key_cards expansion (spec D15)
# ============================================================================


def test_key_cards_minimal_entry_expands_to_one_digivolve_into_component():
    yaml = _yaml(
        """
        profiles:
          p:
            key_cards:
              - cards: [BT17-078]
                reward: 6.0
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    p = profs.by_name["p"]
    # Exactly one synthetic component, no hardcast / alt-path.
    assert len(p.components) == 1
    syn = p.components[0]
    assert syn.kind == "digivolve_into_named_card"
    assert syn.component.weight == 6.0
    assert syn.component.match_card_id == ["BT17-078"]
    # Defaults: diminishing 0.4, no max_fires.
    assert syn.budget.diminishing_returns_factor == 0.4
    assert syn.budget.max_fires_per_episode is None


def test_key_cards_full_entry_expands_to_three_components():
    yaml = _yaml(
        """
        profiles:
          p:
            key_cards:
              - cards: [BT17-078, AD1-025]
                reward: 6.0
                hardcast_penalty: -1.5
                alt_path_reward: 2.0
                alt_paths: [assembly]
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    p = profs.by_name["p"]
    assert len(p.components) == 3
    kinds = [c.kind for c in p.components]
    assert kinds.count("digivolve_into_named_card") == 1
    assert kinds.count("play_named_card") == 2

    # Identify the three by their distinguishing fields.
    dig = [c for c in p.components if c.kind == "digivolve_into_named_card"][0]
    plays = [c for c in p.components if c.kind == "play_named_card"]
    hardcast = next(c for c in plays if c.component.weight == -1.5)
    altpath = next(c for c in plays if c.component.weight == 2.0)

    assert dig.component.weight == 6.0
    assert hardcast.component.cost_paid_gte_printed is True
    assert hardcast.budget.max_fires_per_episode == 1
    assert altpath.component.via_alt_path == ["assembly"]


def test_key_cards_alt_path_reward_without_alt_paths_fails():
    yaml = _yaml(
        """
        profiles:
          p:
            key_cards:
              - cards: [X]
                reward: 6.0
                alt_path_reward: 2.0
        assignments:
          _default: p
        """
    )
    with pytest.raises(ProfileConfigError, match=r"alt_paths"):
        _parse_and_resolve(yaml, SYNTH)


def test_key_cards_alt_paths_without_alt_path_reward_fails():
    yaml = _yaml(
        """
        profiles:
          p:
            key_cards:
              - cards: [X]
                reward: 6.0
                alt_paths: [assembly]
        assignments:
          _default: p
        """
    )
    with pytest.raises(ProfileConfigError, match=r"alt_path"):
        _parse_and_resolve(yaml, SYNTH)


def test_boss_cards_set_equals_union_of_key_cards_lists():
    yaml = _yaml(
        """
        profiles:
          p:
            key_cards:
              - cards: [A, B]
                reward: 6.0
              - cards: [C, A]
                reward: 4.0
        assignments:
          _default: p
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    assert profs.by_name["p"].boss_cards == frozenset({"A", "B", "C"})


def test_profile_without_key_cards_has_empty_boss_cards_set():
    profs = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    assert profs.by_name["_default"].boss_cards == frozenset()


def test_child_key_cards_replaces_parent_key_cards_wholesale():
    yaml = _yaml(
        """
        profiles:
          _parent:
            key_cards:
              - cards: [PARENT_CARD]
                reward: 6.0
          child:
            inherits: _parent
            key_cards:
              - cards: [CHILD_CARD]
                reward: 8.0
        assignments:
          _default: child
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    child = profs.by_name["child"]
    # Boss cards reflect child only (parent's wholesale replaced).
    assert child.boss_cards == frozenset({"CHILD_CARD"})
    # Component list has the child's synthetic digivolve_into (the
    # parent's synthetic was REPLACED by the child's at the same
    # (kind, key_params) key since both use `match.card_id`).
    digs = [c for c in child.components if c.kind == "digivolve_into_named_card"]
    assert len(digs) == 1
    assert digs[0].component.weight == 8.0


# ============================================================================
# Resolve_active
# ============================================================================


def test_resolve_active_uses_default_when_archetype_absent():
    profs = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    p = profs.resolve_active(archetype=None)
    assert p.name == "_default"


def test_resolve_active_uses_default_when_archetype_unmapped():
    profs = _parse_and_resolve(_minimal_default_profile_block(), SYNTH)
    p = profs.resolve_active(archetype="UnmappedArchetype")
    assert p.name == "_default"


def test_resolve_active_uses_archetype_when_mapped():
    yaml = _minimal_default_profile_block() + _yaml(
        """
        profiles:
          aggressive:
            components:
              - kind: step_penalty
                weight: -0.003
        """,
    )
    # Need to re-build the YAML so both blocks merge correctly.
    yaml = _yaml(
        """
        profiles:
          _default:
            components:
              - kind: step_penalty
                weight: -0.001
          aggressive:
            components:
              - kind: step_penalty
                weight: -0.003
        assignments:
          _default: _default
          "Rocks": aggressive
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    assert profs.resolve_active("Rocks").name == "aggressive"
    assert profs.resolve_active("DNA Omnimon").name == "_default"


def test_resolve_active_override_supersedes_archetype():
    yaml = _yaml(
        """
        profiles:
          _default:
            components:
              - kind: step_penalty
                weight: -0.001
          override_target:
            components:
              - kind: step_penalty
                weight: -0.5
          arch_target:
            components:
              - kind: step_penalty
                weight: -0.2
        assignments:
          _default: _default
          "X": arch_target
        """
    )
    profs = _parse_and_resolve(yaml, SYNTH)
    # Without override: archetype mapping wins.
    assert profs.resolve_active("X").name == "arch_target"
    # With override: archetype is ignored.
    assert profs.resolve_active("X", override="override_target").name == "override_target"


# ============================================================================
# Hot-reload (ProfileLoader file-based path)
# ============================================================================


def test_loader_reload_if_changed_reloads_on_mtime_advance(tmp_path: Path):
    p = tmp_path / "profiles.yaml"
    p.write_text(_minimal_default_profile_block(), encoding="utf-8")
    loader = ProfileLoader(p)
    first_hash = loader.profiles.content_hash

    # Wait briefly + write modified content with different weight.
    time.sleep(0.05)
    p.write_text(
        _minimal_default_profile_block().replace("-0.001", "-0.002"), encoding="utf-8",
    )

    reloaded = loader.reload_if_changed()
    assert reloaded is True
    assert loader.profiles.content_hash != first_hash


def test_loader_reload_no_op_when_mtime_unchanged(tmp_path: Path):
    p = tmp_path / "profiles.yaml"
    p.write_text(_minimal_default_profile_block(), encoding="utf-8")
    loader = ProfileLoader(p)
    # No mtime advance → returns False, no reload.
    assert loader.reload_if_changed() is False


def test_loader_parse_failure_preserves_previous_load_with_warning(tmp_path: Path):
    p = tmp_path / "profiles.yaml"
    p.write_text(_minimal_default_profile_block(), encoding="utf-8")
    loader = ProfileLoader(p)
    pre_hash = loader.profiles.content_hash

    time.sleep(0.05)
    p.write_text("invalid: ::: yaml :::\n", encoding="utf-8")

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        result = loader.reload_if_changed()
    assert result is False
    assert any("reward profile reload failed" in str(wi.message) for wi in w)
    # Previously-loaded profiles preserved.
    assert loader.profiles.content_hash == pre_hash


def test_loader_subsequent_reload_succeeds_after_temporary_failure(tmp_path: Path):
    p = tmp_path / "profiles.yaml"
    p.write_text(_minimal_default_profile_block(), encoding="utf-8")
    loader = ProfileLoader(p)

    time.sleep(0.05)
    p.write_text("invalid: ::: yaml :::\n", encoding="utf-8")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        loader.reload_if_changed()  # fails, preserves

    time.sleep(0.05)
    p.write_text(
        _minimal_default_profile_block().replace("-0.001", "-0.005"), encoding="utf-8",
    )
    assert loader.reload_if_changed() is True
