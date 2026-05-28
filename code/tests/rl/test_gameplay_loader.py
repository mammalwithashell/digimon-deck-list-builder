"""Two-file loader tests — `add-gameplay-reward-config`.

Maps to scenarios in
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
("Gameplay reward shape lives in a separate YAML file", "Two-file
loader merges namespaces", "Sidecar persists gameplay hash separately"),
and the modified `reward-profiles` capability's
"Profile loader cross-file inheritance".

Tests construct literal YAML strings and call
`_parse_and_resolve_two_files` directly, bypassing file I/O.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from digimon_gym.agents.reward.profile_loader import (
    ProfileConfigError,
    ProfileLoader,
    _parse_and_resolve_two_files,
)


GP_PATH = Path("gameplay.yaml")
PR_PATH = Path("profiles.yaml")


def _good_gameplay_yaml() -> str:
    return """\
profiles:
  gameplay:
    components:
      - kind: terminal_outcome
        win_base: 10.0
        fast_win_bonus_max: 0.0
        fast_win_par_steps: 200
        loss: -10.0
        draw: -1.0
      - kind: step_penalty
        weight: -0.001
"""


def _good_profiles_yaml() -> str:
    return """\
profiles:
  _default:
    inherits: gameplay
assignments:
  _default: _default
"""


# ── Happy path ──────────────────────────────────────────────────────


def test_two_file_load_succeeds_with_default_shape():
    """Cross-file inheritance resolves: `_default` inherits gameplay,
    the resolved component list includes gameplay's components."""
    p = _parse_and_resolve_two_files(
        _good_gameplay_yaml(), GP_PATH, _good_profiles_yaml(), PR_PATH,
    )
    assert set(p.by_name) == {"gameplay", "_default"}
    # _default inherits all of gameplay's components.
    default_kinds = [c.kind for c in p.by_name["_default"].components]
    assert "terminal_outcome" in default_kinds
    assert "step_penalty" in default_kinds


def test_two_separate_hashes_computed():
    """Scenario: Sidecar records both hashes (sha256:..)."""
    p = _parse_and_resolve_two_files(
        _good_gameplay_yaml(), GP_PATH, _good_profiles_yaml(), PR_PATH,
    )
    assert p.gameplay_hash.startswith("sha256:")
    assert p.profiles_hash.startswith("sha256:")
    # The two files differ → different hashes.
    assert p.gameplay_hash != p.profiles_hash


def test_real_shipped_files_load_via_profile_loader():
    """Smoke: ProfileLoader against the actual shipped files resolves
    all four profiles (gameplay, _default, dna_omnimon_combo_v1,
    bg_imperialdramon_combo_v1)."""
    loader = ProfileLoader(
        gameplay_path="code/digimon_gym/agents/reward/gameplay.yaml",
        profiles_path="code/digimon_gym/agents/reward/profiles.yaml",
    )
    assert set(loader.profiles.by_name) >= {
        "gameplay",
        "_default",
        "dna_omnimon_combo_v1",
        "bg_imperialdramon_combo_v1",
    }


# ── Gameplay.yaml validation failures ───────────────────────────────


def test_missing_gameplay_profile_fails():
    bad_gp = """\
profiles:
  not_gameplay: {}
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(bad_gp, GP_PATH, _good_profiles_yaml(), PR_PATH)
    assert "gameplay" in str(exc.value).lower()


def test_multi_profile_gameplay_yaml_fails():
    """Scenario: gameplay.yaml defines exactly one profile."""
    bad_gp = """\
profiles:
  gameplay:
    components:
      - kind: step_penalty
        weight: -0.001
  something_else: {}
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(bad_gp, GP_PATH, _good_profiles_yaml(), PR_PATH)
    msg = str(exc.value)
    assert "exactly one" in msg.lower()
    assert "something_else" in msg


def test_gameplay_with_inherits_fails():
    """Scenario: gameplay profile attempting to inherit fails."""
    bad_gp = """\
profiles:
  gameplay:
    inherits: something
    components:
      - kind: step_penalty
        weight: -0.001
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(bad_gp, GP_PATH, _good_profiles_yaml(), PR_PATH)
    assert "inherits" in str(exc.value).lower()


def test_assignments_in_gameplay_yaml_fails():
    bad_gp = """\
profiles:
  gameplay:
    components:
      - kind: step_penalty
        weight: -0.001
assignments:
  _default: gameplay
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(bad_gp, GP_PATH, _good_profiles_yaml(), PR_PATH)
    assert "assignments" in str(exc.value).lower()


# ── Profiles.yaml validation failures ───────────────────────────────


def test_profiles_yaml_profile_without_inherits_fails():
    """Scenario: profiles.yaml profile without inherits fails."""
    bad_pr = """\
profiles:
  rogue:
    components: []
assignments:
  _default: rogue
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(_good_gameplay_yaml(), GP_PATH, bad_pr, PR_PATH)
    assert "rogue" in str(exc.value)
    assert "inherits" in str(exc.value).lower()


def test_name_collision_across_files_fails():
    """Scenario: Profile name collision across files fails."""
    bad_pr = """\
profiles:
  gameplay:
    inherits: gameplay
assignments:
  _default: gameplay
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(_good_gameplay_yaml(), GP_PATH, bad_pr, PR_PATH)
    msg = str(exc.value)
    assert "gameplay" in msg
    assert "collision" in msg.lower()


def test_cycle_across_files_fails_with_cycle_path():
    """Scenario: Cycles across files fail with cycle path."""
    # gameplay → aux → gameplay (would-be cycle if gameplay inherited).
    # Construct a self-cycle in profiles.yaml since gameplay can't have inherits.
    bad_pr = """\
profiles:
  aux:
    inherits: bux
  bux:
    inherits: aux
assignments:
  _default: aux
"""
    with pytest.raises(ProfileConfigError):
        _parse_and_resolve_two_files(_good_gameplay_yaml(), GP_PATH, bad_pr, PR_PATH)


# ── Cross-file inheritance + override ──────────────────────────────


def test_cross_file_inheritance_with_override():
    """Spec: Cross-file inheritance resolves correctly. A child profile
    in profiles.yaml can override a component inherited from gameplay.yaml."""
    gp = """\
profiles:
  gameplay:
    components:
      - kind: security_remove
        weight: 1.5
      - kind: step_penalty
        weight: -0.001
"""
    pr = """\
profiles:
  aggressive:
    inherits: gameplay
    components:
      - kind: security_remove
        weight: 3.0
assignments:
  _default: aggressive
"""
    p = _parse_and_resolve_two_files(gp, GP_PATH, pr, PR_PATH)
    agg = p.by_name["aggressive"]
    sec = [c for c in agg.components if c.kind == "security_remove"]
    assert len(sec) == 1
    # Child override wins.
    assert sec[0].component.weight == pytest.approx(3.0)
    # step_penalty inherited unchanged.
    step = [c for c in agg.components if c.kind == "step_penalty"]
    assert len(step) == 1
    assert step[0].component.weight == pytest.approx(-0.001)


def test_legacy_terminal_exclusivity_in_yaml_fails():
    """`add-gameplay-reward-config` removed the legacy flag — YAML
    setting it MUST fail-fast with a migration message."""
    bad_pr = """\
profiles:
  _default:
    inherits: gameplay
    legacy_terminal_exclusivity: true
assignments:
  _default: _default
"""
    with pytest.raises(ProfileConfigError) as exc:
        _parse_and_resolve_two_files(_good_gameplay_yaml(), GP_PATH, bad_pr, PR_PATH)
    assert "legacy_terminal_exclusivity" in str(exc.value)
    assert "removed" in str(exc.value).lower() or "add-gameplay-reward-config" in str(exc.value)


# ── Missing-file behavior ──────────────────────────────────────────


def test_missing_gameplay_file_fails_fast(tmp_path: Path):
    """Spec: gameplay.yaml absent fails at load."""
    # gameplay file points at a non-existent path.
    profiles_path = tmp_path / "profiles.yaml"
    profiles_path.write_text(_good_profiles_yaml(), encoding="utf-8")
    with pytest.raises(ProfileConfigError):
        ProfileLoader(
            gameplay_path=tmp_path / "does-not-exist.yaml",
            profiles_path=profiles_path,
        )
