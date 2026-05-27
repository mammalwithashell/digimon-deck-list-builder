"""Profile loader, inheritance resolver, and hash + hot-reload manager
for `reward-profiles`.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`

Pipeline (top-down):

    YAML text
       v
    parse + validate top-level schema (profiles{}, assignments{})
       v
    expand each profile's `key_cards:` into synthetic component decls
       v
    resolve inheritance per-profile (single-parent, cycle-detected)
       v
    apply child-replaces-parent override semantics on (kind, key_params)
       v
    materialize each component decl → RewardComponent + ComponentBudget
       v
    Profile(name, components, profile_budget, boss_cards)

Public API:

- `ProfileLoader`        — owns the loaded set; supports hot-reload.
- `Profile`              — resolved profile (use this in the wrapper).
- `Profiles`             — loaded snapshot (`{name: Profile}` + assignments + hash).
- exceptions: `ProfileConfigError` (load-time), `InheritanceCycleError`.
"""

from __future__ import annotations

import hashlib
import json
import os
import warnings
from dataclasses import dataclass, field
from pathlib import Path
from typing import (
    Any,
    Dict,
    FrozenSet,
    Iterable,
    List,
    Mapping,
    MutableMapping,
    Optional,
    Sequence,
    Set,
    Tuple,
)

import yaml

from .budget import ComponentBudget, ProfileBudget
from .component import RewardComponent
from .registry import KIND_KEY_PARAMETERS, UnknownComponentKindError, get_component_class


# ============================================================================
# Errors
# ============================================================================


class ProfileConfigError(ValueError):
    """Raised on any structural error in profiles.yaml that prevents
    loading. Always carries a single human-readable message; the
    `RewardProfileWrapper` surfaces this through `ConfigError` at
    run-start.
    """


class InheritanceCycleError(ProfileConfigError):
    """Raised when a profile's `inherits:` chain forms a cycle. The
    message names every profile in the cycle so the operator can fix
    it without grepping.
    """

    def __init__(self, cycle: Sequence[str]) -> None:
        path = " -> ".join(cycle)
        super().__init__(f"Profile inheritance cycle detected: {path}")
        self.cycle = list(cycle)


# ============================================================================
# Resolved shapes (what the wrapper consumes)
# ============================================================================


@dataclass(frozen=True)
class MaterializedComponent:
    """A component declaration after inheritance + key_cards expansion,
    paired with its budget and a stable lookup key.
    """

    kind: str
    name: str             # `info["reward_breakdown"]` key
    component_key: str    # unique within (profile, kind) — for budget state
    component: RewardComponent
    budget: ComponentBudget


@dataclass(frozen=True)
class Profile:
    """A fully-resolved profile ready for the wrapper.

    `legacy_terminal_exclusivity` (default False) suppresses every
    non-terminal_outcome component on the terminal step — replicates
    the legacy `DigimonEnv._compute_reward` short-circuit where the
    terminal-magnitude return skipped both step_penalty AND security
    delta. Set True ONLY on profiles that need byte-identical legacy
    behavior (the shipped `_default` and `_digivolve_shaped`).
    Custom profiles should leave this False — shaping components
    firing on the terminal step is the correct behavior.
    """

    name: str
    components: Tuple[MaterializedComponent, ...]
    profile_budget: ProfileBudget
    boss_cards: FrozenSet[str]
    legacy_terminal_exclusivity: bool = False


@dataclass(frozen=True)
class Profiles:
    """Snapshot of `profiles.yaml` after load + resolve."""

    by_name: Mapping[str, Profile]
    assignments: Mapping[str, str]   # archetype_name -> profile_name
    content_hash: str                # "sha256:<hex>"
    source_path: Path                # for error messages + reload mtime

    def resolve_active(
        self,
        archetype: Optional[str],
        override: Optional[str] = None,
    ) -> Profile:
        """Pick the active profile for an episode.

        Resolution order:
          1. Explicit override (validated at load — must exist).
          2. `archetype` (when not None and present in assignments).
          3. `assignments["_default"]` (always present; validated at load).
        """
        if override is not None:
            return self.by_name[override]
        if archetype is not None and archetype in self.assignments:
            return self.by_name[self.assignments[archetype]]
        return self.by_name[self.assignments["_default"]]


# ============================================================================
# Raw (pre-resolution) shapes
# ============================================================================


# A raw component declaration after key_cards expansion but before
# inheritance resolution. Just the parsed YAML dict + a source marker
# for error messages.
@dataclass
class _RawComponent:
    kind: str
    params: Dict[str, Any]
    budget: Optional[Dict[str, Any]]
    # `"yaml"` for hand-written components; `"key_cards"` for synthetic
    # components expanded from a key_cards entry. Used in error messages.
    source: str


@dataclass
class _RawProfile:
    name: str
    parent_name: Optional[str]
    raw_components: List[_RawComponent]
    profile_budget: Optional[Dict[str, Any]]
    boss_cards: Set[str]   # populated by key_cards expansion
    legacy_terminal_exclusivity: Optional[bool] = None  # None = inherit; True/False = explicit


# ============================================================================
# Loader
# ============================================================================


class ProfileLoader:
    """Loads `profiles.yaml`, supports hot-reload, exposes the
    `Profiles` snapshot.

    Hot-reload semantics: `reload_if_changed()` stats the file and
    re-parses when mtime advances. The wrapper calls this at each
    `reset()` (when enabled in config). Parse failures retain the
    previously-loaded `Profiles` and log a warning.
    """

    def __init__(self, path: Path | str) -> None:
        self._path = Path(path)
        self._last_mtime_ns: Optional[int] = None
        self._profiles: Optional[Profiles] = None
        # Load eagerly so callers can rely on `profiles` being non-None
        # after construction (failures raise — operators should not
        # silently get an empty loader).
        self._load_now()

    @property
    def path(self) -> Path:
        return self._path

    @property
    def profiles(self) -> Profiles:
        assert self._profiles is not None, "ProfileLoader invariant violated"
        return self._profiles

    def reload_if_changed(self) -> bool:
        """Stat the file. When mtime is newer than the last load,
        re-parse and update the cached `Profiles`. On parse failure,
        log a warning and retain the previous load.

        Returns True iff a successful reload occurred (mtime advanced
        AND parse succeeded).
        """
        try:
            st = os.stat(self._path)
        except OSError as e:
            warnings.warn(
                f"reward profile reload: stat failed for {self._path}: {e}",
                stacklevel=2,
            )
            return False
        if self._last_mtime_ns is not None and st.st_mtime_ns <= self._last_mtime_ns:
            return False
        try:
            self._load_now()
            return True
        except ProfileConfigError as e:
            warnings.warn(
                f"reward profile reload failed (preserving previous load): {e}",
                stacklevel=2,
            )
            return False

    def _load_now(self) -> None:
        try:
            text = self._path.read_text(encoding="utf-8")
            st = os.stat(self._path)
        except OSError as e:
            raise ProfileConfigError(
                f"Failed to read reward profiles file {self._path}: {e}"
            ) from e
        self._profiles = _parse_and_resolve(text, self._path)
        self._last_mtime_ns = st.st_mtime_ns


# ============================================================================
# Top-level parse + resolve entry point
# ============================================================================


def _parse_and_resolve(text: str, source_path: Path) -> Profiles:
    """Single-shot: YAML text → resolved `Profiles`. Tests bypass file
    I/O by calling this directly with literal YAML strings.
    """
    try:
        doc = yaml.safe_load(text)
    except yaml.YAMLError as e:
        raise ProfileConfigError(f"YAML parse error in {source_path}: {e}") from e
    if not isinstance(doc, dict):
        raise ProfileConfigError(
            f"{source_path}: top-level YAML must be a mapping with "
            "`profiles:` and `assignments:` keys"
        )

    raw_profiles_block = doc.get("profiles")
    if not isinstance(raw_profiles_block, dict) or not raw_profiles_block:
        raise ProfileConfigError(
            f"{source_path}: top-level `profiles:` must be a non-empty mapping"
        )

    raw_assignments = doc.get("assignments")
    if not isinstance(raw_assignments, dict):
        raise ProfileConfigError(
            f"{source_path}: top-level `assignments:` must be a mapping"
        )
    if "_default" not in raw_assignments:
        raise ProfileConfigError(
            f"{source_path}: `assignments:` SHALL contain a `_default` key "
            "(spec: 'Profile assignment is agent-archetype-keyed with required default fallback')"
        )

    # Parse each profile into raw form. key_cards expansion happens
    # here so subsequent inheritance resolution sees the expanded
    # components as if they were hand-written (matching spec D15).
    raw_profiles: Dict[str, _RawProfile] = {}
    for name, body in raw_profiles_block.items():
        if not isinstance(name, str) or not name:
            raise ProfileConfigError(
                f"{source_path}: profile name must be a non-empty string; "
                f"got {name!r}"
            )
        raw_profiles[name] = _parse_raw_profile(name, body, source_path)

    # Validate `inherits` references resolve and detect cycles.
    for prof in raw_profiles.values():
        if prof.parent_name is not None and prof.parent_name not in raw_profiles:
            raise ProfileConfigError(
                f"{source_path}: profile {prof.name!r} inherits from "
                f"{prof.parent_name!r} which is not defined"
            )
    _detect_cycles(raw_profiles)

    # Validate assignments target real profiles, and private profiles
    # don't appear as assignment values for real archetype keys. The
    # `_default` assignment key is special: it's REQUIRED by the spec
    # and exists specifically as the fallback when an archetype isn't
    # in the map, so it MAY target a private profile (typically the
    # shipped `_default` profile).
    for arch_name, profile_name in raw_assignments.items():
        if not isinstance(profile_name, str):
            raise ProfileConfigError(
                f"{source_path}: assignments[{arch_name!r}] must be a string; "
                f"got {profile_name!r}"
            )
        if profile_name not in raw_profiles:
            raise ProfileConfigError(
                f"{source_path}: assignments[{arch_name!r}] = {profile_name!r} "
                "does not reference a defined profile"
            )
        # Real-archetype assignments (everything except `_default`) cannot
        # point at private profiles. The `_default` carve-out exists
        # because the fallback assignment is the spec-required wiring
        # point for the shipped `_default` profile.
        if arch_name != "_default" and profile_name.startswith("_"):
            raise ProfileConfigError(
                f"{source_path}: profile names starting with `_` are private "
                f"and cannot be assigned to real archetypes; "
                f"got {arch_name!r} -> {profile_name!r}"
            )

    # Resolve inheritance + materialize components for each profile.
    resolved: Dict[str, Profile] = {}
    for name in raw_profiles:
        resolved[name] = _resolve_profile(name, raw_profiles, source_path)

    return Profiles(
        by_name=resolved,
        assignments=dict(raw_assignments),
        content_hash=_canonical_hash(doc),
        source_path=source_path,
    )


# ============================================================================
# Raw-profile parsing (incl. key_cards expansion)
# ============================================================================


def _parse_raw_profile(name: str, body: Any, source_path: Path) -> _RawProfile:
    """Parse one profile block to `_RawProfile`. Expand `key_cards:`
    into synthetic component decls and prepend them to `raw_components`
    (so they participate in inheritance override as if hand-written).
    """
    if body is None:
        body = {}
    if not isinstance(body, dict):
        raise ProfileConfigError(
            f"{source_path}: profile {name!r} body must be a mapping"
        )

    parent_name = body.get("inherits")
    if parent_name is not None and not isinstance(parent_name, str):
        raise ProfileConfigError(
            f"{source_path}: profile {name!r}.inherits must be a string"
        )

    raw_components: List[_RawComponent] = []
    boss_cards: Set[str] = set()

    # key_cards expansion happens BEFORE hand-written components so the
    # override semantics treat synthetic + hand-written as a single
    # ordered list. The boss_cards set accumulates across all entries.
    for entry in body.get("key_cards", []) or []:
        synthetic, entry_cards = _expand_key_cards_entry(entry, name, source_path)
        raw_components.extend(synthetic)
        boss_cards.update(entry_cards)

    for raw_comp_dict in body.get("components", []) or []:
        if not isinstance(raw_comp_dict, dict):
            raise ProfileConfigError(
                f"{source_path}: profile {name!r}.components entries must be mappings"
            )
        raw_components.append(_parse_raw_component(raw_comp_dict, name, source_path, "yaml"))

    profile_budget = body.get("budget")
    if profile_budget is not None and not isinstance(profile_budget, dict):
        raise ProfileConfigError(
            f"{source_path}: profile {name!r}.budget must be a mapping"
        )

    legacy_terminal_exclusivity = body.get("legacy_terminal_exclusivity")
    if legacy_terminal_exclusivity is not None and not isinstance(
        legacy_terminal_exclusivity, bool
    ):
        raise ProfileConfigError(
            f"{source_path}: profile {name!r}.legacy_terminal_exclusivity "
            f"must be a boolean (true/false); got {legacy_terminal_exclusivity!r}"
        )

    return _RawProfile(
        name=name,
        parent_name=parent_name,
        raw_components=raw_components,
        profile_budget=profile_budget,
        boss_cards=boss_cards,
        legacy_terminal_exclusivity=legacy_terminal_exclusivity,
    )


def _parse_raw_component(
    raw: Dict[str, Any], profile_name: str, source_path: Path, source: str,
) -> _RawComponent:
    if "kind" not in raw:
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} component entry missing `kind`"
        )
    kind = raw["kind"]
    if not isinstance(kind, str):
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} component `kind` must be a string"
        )

    # Validate kind exists in registry now so the error message names
    # the profile + source rather than failing later in materialization.
    try:
        get_component_class(kind)
    except UnknownComponentKindError as e:
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} ({source} source): {e}"
        ) from None

    # Split out budget sub-key from params.
    budget_raw = raw.get("budget")
    if budget_raw is not None and not isinstance(budget_raw, dict):
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} component {kind!r} "
            f"`budget` must be a mapping"
        )

    # `once_per_episode: true` shorthand → budget.max_fires_per_episode=1.
    # Emit DeprecationWarning per spec; v2 removes the alias.
    params = {k: v for k, v in raw.items() if k not in ("kind", "budget", "once_per_episode")}
    if raw.get("once_per_episode"):
        warnings.warn(
            f"profile {profile_name!r} component {kind!r}: `once_per_episode: true` "
            "is deprecated; use `budget: {max_fires_per_episode: 1}` instead.",
            DeprecationWarning,
            stacklevel=4,
        )
        merged_budget = dict(budget_raw) if budget_raw else {}
        merged_budget.setdefault("max_fires_per_episode", 1)
        budget_raw = merged_budget

    return _RawComponent(kind=kind, params=params, budget=budget_raw, source=source)


def _expand_key_cards_entry(
    entry: Any, profile_name: str, source_path: Path,
) -> Tuple[List[_RawComponent], Set[str]]:
    """Expand one key_cards entry into 1-3 synthetic `_RawComponent`s
    + return the cards-set contribution for boss_cards.

    Per spec D15:
      - reward (required) → digivolve_into_named_card
      - hardcast_penalty + hardcast_max_per_episode → play_named_card
        with cost_paid_gte_printed=true
      - alt_path_reward + alt_paths (both required when either set) →
        play_named_card with via_alt_path matcher
    """
    if not isinstance(entry, dict):
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} key_cards entry must be a mapping"
        )

    cards = entry.get("cards")
    if not isinstance(cards, list) or not cards or not all(isinstance(c, str) for c in cards):
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} key_cards entry requires "
            "`cards: <non-empty list[str]>`"
        )
    cards_list: List[str] = list(cards)
    cards_set: Set[str] = set(cards_list)

    if "reward" not in entry:
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} key_cards entry requires `reward: <float>`"
        )
    reward = float(entry["reward"])
    diminishing_factor = float(entry.get("diminishing_factor", 0.4))
    max_per_episode = entry.get("max_per_episode")  # None default; can be int
    hardcast_penalty = entry.get("hardcast_penalty")  # None default
    hardcast_max_per_episode = int(entry.get("hardcast_max_per_episode", 1))
    alt_path_reward = entry.get("alt_path_reward")  # None default
    alt_paths = entry.get("alt_paths")  # None default

    # Spec: alt_path_reward + alt_paths must come together (or neither).
    if (alt_path_reward is None) != (alt_paths is None):
        raise ProfileConfigError(
            f"{source_path}: profile {profile_name!r} key_cards entry sets "
            "only one of (alt_path_reward, alt_paths) — both required together"
        )

    synthetic: List[_RawComponent] = []

    # Always: reward component (digivolve_into_named_card).
    synthetic.append(
        _RawComponent(
            kind="digivolve_into_named_card",
            params={
                "match": {"card_id": cards_list},
                "weight": reward,
            },
            budget={
                "max_fires_per_episode": max_per_episode,
                "diminishing_returns_factor": diminishing_factor,
            },
            source="key_cards",
        )
    )

    # Optional: hardcast penalty component (play_named_card with cost gate).
    if hardcast_penalty is not None:
        synthetic.append(
            _RawComponent(
                kind="play_named_card",
                params={
                    "match": {"card_id": cards_list},
                    "weight": float(hardcast_penalty),
                    "cost_paid_gte_printed": True,
                },
                budget={"max_fires_per_episode": hardcast_max_per_episode},
                source="key_cards",
            )
        )

    # Optional: alt-path reward component (play_named_card with via_alt_path gate).
    if alt_path_reward is not None:
        if not isinstance(alt_paths, list) or not all(isinstance(p, str) for p in alt_paths):
            raise ProfileConfigError(
                f"{source_path}: profile {profile_name!r} key_cards entry "
                "`alt_paths` must be a list[str]"
            )
        synthetic.append(
            _RawComponent(
                kind="play_named_card",
                params={
                    "match": {"card_id": cards_list},
                    "weight": float(alt_path_reward),
                    "via_alt_path": list(alt_paths),
                },
                budget={
                    "max_fires_per_episode": max_per_episode,
                    "diminishing_returns_factor": diminishing_factor,
                },
                source="key_cards",
            )
        )

    return synthetic, cards_set


# ============================================================================
# Inheritance + override resolution
# ============================================================================


def _detect_cycles(raw_profiles: Mapping[str, _RawProfile]) -> None:
    """Walk every profile's inherits chain. Raise on a cycle."""
    for start in raw_profiles:
        visited: List[str] = []
        cur: Optional[str] = start
        while cur is not None:
            if cur in visited:
                visited.append(cur)
                # Trim to just the cycle portion for readability.
                cycle_start = visited.index(cur)
                raise InheritanceCycleError(visited[cycle_start:])
            visited.append(cur)
            cur = raw_profiles[cur].parent_name


def _resolve_profile(
    name: str, raw_profiles: Mapping[str, _RawProfile], source_path: Path,
) -> Profile:
    """Walk parent chain (deepest first), build component list with
    override semantics, materialize, accumulate boss_cards.
    """
    chain: List[_RawProfile] = []
    cur: Optional[str] = name
    while cur is not None:
        chain.append(raw_profiles[cur])
        cur = raw_profiles[cur].parent_name
    chain.reverse()  # root-first so child overrides parent

    # Per-component override key → MaterializedComponent. Ordered dict
    # so iteration order is "first declared wins position" with later
    # overrides REPLACING but not reordering. This matches the spec's
    # intent of "child replaces parent's matching tuple".
    resolved_by_key: "Dict[Tuple[str, str], _RawComponent]" = {}
    profile_budget_raw: Optional[Dict[str, Any]] = None
    # boss_cards is REPLACED (not merged) when a descendant declares
    # its own key_cards. Per spec D15: "A child profile that declares
    # key_cards: SHALL replace its parent's key_cards: wholesale".
    # Implementation: as soon as a profile in the chain (other than
    # the root) declares key_cards (== boss_cards is non-empty), drop
    # every earlier `_RawComponent` whose `source == "key_cards"` from
    # the override map. This honors "wholesale replacement" of the
    # key_cards BLOCK, not just per-(kind, key_params) override.
    boss_cards_winner: FrozenSet[str] = frozenset()
    # legacy_terminal_exclusivity inherits explicitly: child's value
    # (if set) wins; otherwise parent's; otherwise False.
    legacy_terminal_exclusivity = False

    for prof in chain:
        if prof.profile_budget is not None:
            profile_budget_raw = prof.profile_budget
        if prof.legacy_terminal_exclusivity is not None:
            legacy_terminal_exclusivity = prof.legacy_terminal_exclusivity
        if prof.boss_cards:
            # This profile declared key_cards. Drop EVERY earlier-chain
            # key_cards-sourced component from the override map (the
            # wholesale-replace semantic from spec D15). Then accumulate
            # this profile's key_cards components and replace the
            # boss_cards set.
            to_drop = [
                k for k, v in resolved_by_key.items() if v.source == "key_cards"
            ]
            for k in to_drop:
                del resolved_by_key[k]
            boss_cards_winner = frozenset(prof.boss_cards)
        for raw in prof.raw_components:
            override_key = (raw.kind, _override_key_for(raw))
            resolved_by_key[override_key] = raw

    # Materialize.
    materialized: List[MaterializedComponent] = []
    for (kind, _key_tuple), raw in resolved_by_key.items():
        materialized.append(_materialize_component(raw, name, len(materialized)))

    profile_budget = _materialize_profile_budget(profile_budget_raw)

    return Profile(
        name=name,
        components=tuple(materialized),
        profile_budget=profile_budget,
        boss_cards=boss_cards_winner,
        legacy_terminal_exclusivity=legacy_terminal_exclusivity,
    )


def _override_key_for(raw: _RawComponent) -> str:
    """Canonical override key for a component. Per spec D3 + D15:
    - Single-instance kinds (key params = empty set) → same key always,
      so child REPLACES parent.
    - Multi-instance kinds (e.g., play_named_card) → key includes all
      key_param values, so distinct instances coexist.

    Implemented as a JSON-serialized sorted dict of the key params'
    actual values (or `""` for single-instance kinds).
    """
    key_params: FrozenSet[str] = KIND_KEY_PARAMETERS.get(raw.kind, frozenset())
    if not key_params:
        return ""
    payload: Dict[str, Any] = {}
    for p in sorted(key_params):
        if p in raw.params:
            payload[p] = raw.params[p]
    return json.dumps(payload, sort_keys=True, default=str)


def _materialize_component(
    raw: _RawComponent, profile_name: str, position: int,
) -> MaterializedComponent:
    """Instantiate the RewardComponent + ComponentBudget from a raw decl."""
    cls = get_component_class(raw.kind)
    # `name` for telemetry: kind plus a disambiguating suffix when there
    # are multiple instances of the same kind in a profile (positional).
    # The position-based suffix is stable for a given resolution order.
    component_name = raw.kind if position == 0 or not _kind_allows_multiple(raw.kind) else f"{raw.kind}#{position}"

    # Convert params dict to constructor kwargs. play_named_card and
    # digivolve_into_named_card take `match` as a nested dict; we
    # translate that to flat kwargs.
    init_kwargs = _component_kwargs(raw.kind, raw.params)
    component = cls(name=component_name, **init_kwargs)

    budget = _materialize_component_budget(raw.budget)
    component_key = f"{raw.kind}|{position}"
    return MaterializedComponent(
        kind=raw.kind,
        name=component_name,
        component_key=component_key,
        component=component,
        budget=budget,
    )


def _kind_allows_multiple(kind: str) -> bool:
    return bool(KIND_KEY_PARAMETERS.get(kind, frozenset()))


def _component_kwargs(kind: str, params: Mapping[str, Any]) -> Dict[str, Any]:
    """Translate raw YAML params to constructor kwargs per component kind.

    Most components accept `weight` + variant-specific scalars directly.
    Named-card components flatten the `match:` sub-dict into `match_*`
    kwargs.
    """
    out: Dict[str, Any] = {}
    if kind in ("play_named_card", "digivolve_into_named_card"):
        match = params.get("match", {})
        if not isinstance(match, dict):
            raise ProfileConfigError(
                f"component {kind!r}: `match` must be a mapping"
            )
        if "card_id" in match:
            out["match_card_id"] = _ensure_str_list(match["card_id"])
        if "card_name" in match:
            out["match_card_name"] = _ensure_str_list(match["card_name"])
        if "trait" in match:
            out["match_trait"] = _ensure_str_list(match["trait"])
        if not any(k in out for k in ("match_card_id", "match_card_name", "match_trait")):
            raise ProfileConfigError(
                f"component {kind!r}: `match` must set at least one of "
                "card_id / card_name / trait"
            )
        for k, v in params.items():
            if k == "match":
                continue
            if k in (
                "cost_paid_lt",
                "cost_paid_eq",
                "cost_paid_gte",
                "cost_paid_lt_printed",
                "cost_paid_gte_printed",
                "was_dna",
                "was_blast_dna",
                "min_result_level",
            ):
                out[k] = v
            elif k == "via_alt_path":
                out["via_alt_path"] = _ensure_str_list(v) if v is not None else None
            elif k == "weight":
                out["weight"] = float(v)
            else:
                raise ProfileConfigError(
                    f"component {kind!r}: unrecognized parameter {k!r}"
                )
        return out

    # Generic single-param components: just pass everything through.
    for k, v in params.items():
        if k in (
            "weight",
            "win_base",
            "fast_win_bonus_max",
            "loss",
            "draw",
        ):
            out[k] = float(v)
        elif k == "fast_win_par_steps":
            out[k] = int(v)
        else:
            raise ProfileConfigError(
                f"component {kind!r}: unrecognized parameter {k!r}"
            )
    return out


def _ensure_str_list(v: Any) -> List[str]:
    if isinstance(v, str):
        return [v]
    if isinstance(v, list) and all(isinstance(x, str) for x in v):
        return list(v)
    raise ProfileConfigError(
        f"expected string or list of strings, got {v!r}"
    )


def _materialize_component_budget(raw: Optional[Mapping[str, Any]]) -> ComponentBudget:
    if not raw:
        return ComponentBudget()
    return ComponentBudget(
        max_fires_per_episode=raw.get("max_fires_per_episode"),
        max_total_per_episode=raw.get("max_total_per_episode"),
        diminishing_returns_factor=float(raw.get("diminishing_returns_factor", 1.0)),
    )


def _materialize_profile_budget(raw: Optional[Mapping[str, Any]]) -> ProfileBudget:
    if not raw:
        return ProfileBudget()
    return ProfileBudget(
        per_episode_cap=raw.get("per_episode_cap"),
        per_episode_floor=raw.get("per_episode_floor"),
    )


# ============================================================================
# Canonical content hash
# ============================================================================


def _canonical_hash(doc: Any) -> str:
    """SHA-256 over the parsed-and-canonicalized YAML document.

    Canonicalization:
    - Dict keys sorted lexicographically (recursive).
    - List order preserved (component order is semantically meaningful).
    - Floats normalized to Python `repr()` to avoid 0.1 vs 0.10 drift.
    - Booleans normalized to lowercase `true`/`false` (json.dumps default).

    Whitespace and key-order changes in the source YAML do NOT alter
    the hash. Re-saving the file with a YAML formatter is a no-op.
    """
    canonical_text = json.dumps(_canonicalize(doc), sort_keys=True, separators=(",", ":"))
    digest = hashlib.sha256(canonical_text.encode("utf-8")).hexdigest()
    return f"sha256:{digest}"


def _canonicalize(node: Any) -> Any:
    if isinstance(node, dict):
        return {k: _canonicalize(v) for k, v in sorted(node.items(), key=lambda kv: str(kv[0]))}
    if isinstance(node, list):
        return [_canonicalize(v) for v in node]
    if isinstance(node, float):
        # Use repr to preserve precise float identity.
        return repr(node)
    return node
