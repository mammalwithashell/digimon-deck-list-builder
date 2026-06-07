"""Board tensor profile metadata used by RL feature extraction.

Rust owns the canonical profile registry; this module is a thin adapter over
the `digimon_engine` PyO3 wheel, which the training env now requires. The
`standard_compact_v1` profile is retired from the training env (the Rust engine
keeps its own compact tensor builder for offline parity use); requesting it
raises a clear error pointing at `standard_lite_v2`.
"""

from __future__ import annotations

import importlib
import importlib.util
import sys
from dataclasses import dataclass
from typing import Any, Iterable


@dataclass(frozen=True)
class TensorSection:
    name: str
    offset: int
    size: int
    shape: tuple[int, ...]


@dataclass(frozen=True)
class TensorProfile:
    id: str
    game_mode: str
    version: int
    tensor_version: int
    feature_schema_version: str
    layout_hash: str
    tensor_size: int
    field_slots: int
    slot_size: int
    max_sources: int
    card_id_slot_count: int
    scalar_slot_count: int
    card_id_positions: tuple[int, ...]
    scalar_positions: tuple[int, ...]
    sections: tuple[TensorSection, ...] = ()


_CANONICAL_STANDARD_COMPACT_V1 = "standard_compact_v1"
_STANDARD_COMPACT_V1_ALIASES = {
    _CANONICAL_STANDARD_COMPACT_V1,
    "standard_v1",
    "compact_v1",
}


def get_tensor_profile(profile_id: str | None = None) -> TensorProfile:
    _reject_retired_profile(profile_id)
    digimon_engine = _require_digimon_engine()

    get_layout = getattr(digimon_engine, "get_observation_layout", None)
    if get_layout is not None:
        return _profile_from_observation_layout(get_layout(profile_id))

    get_profile = getattr(digimon_engine, "get_tensor_profile", None)
    if get_profile is None:
        raise RuntimeError(
            "digimon_engine is installed but exposes neither "
            "get_observation_layout nor get_tensor_profile; rebuild the wheel "
            "(`cd code/digimon-engine-py && maturin develop --release`)."
        )

    raw = get_profile(profile_id)
    return TensorProfile(
        id=_canonicalize_tensor_profile_id(raw.id),
        game_mode=raw.game_mode,
        version=raw.version,
        tensor_version=getattr(raw, "tensor_version", raw.version),
        feature_schema_version=getattr(
            raw,
            "feature_schema_version",
            f"{_canonicalize_tensor_profile_id(raw.id)}.1",
        ),
        layout_hash=getattr(raw, "layout_hash", ""),
        tensor_size=raw.tensor_size,
        field_slots=raw.field_slots,
        slot_size=raw.slot_size,
        max_sources=raw.max_sources,
        card_id_slot_count=raw.card_id_slot_count,
        scalar_slot_count=raw.scalar_slot_count,
        card_id_positions=tuple(raw.card_id_positions),
        scalar_positions=tuple(raw.scalar_positions),
    )


def get_tensor_profile_for_observation_shape(shape: tuple[int, ...]) -> TensorProfile:
    """Resolve a unique tensor profile from a one-dimensional observation shape."""
    if len(shape) != 1:
        raise ValueError(
            f"observation space shape {shape} is not one-dimensional; "
            "pass tensor_profile_id or observation_layout explicitly"
        )
    return get_tensor_profile_for_tensor_size(int(shape[0]))


def get_tensor_profile_for_tensor_size(tensor_size: int) -> TensorProfile:
    """Resolve a unique registered tensor profile by tensor size."""
    digimon_engine = _require_digimon_engine()

    list_profiles = getattr(digimon_engine, "list_observation_profiles", None)
    if list_profiles is None:
        list_profiles = getattr(digimon_engine, "list_tensor_profiles", None)
    if list_profiles is None:
        raise RuntimeError(
            "digimon_engine is installed but exposes no profile registry; "
            "rebuild the wheel (`cd code/digimon-engine-py && maturin develop --release`)."
        )

    matches: dict[str, TensorProfile] = {}
    for profile_id in list_profiles():
        try:
            profile = get_tensor_profile(str(profile_id))
        except ValueError:
            # Skip profiles retired from the training env (e.g. standard_compact_v1).
            continue
        if profile.tensor_size == tensor_size:
            matches[profile.id] = profile

    if len(matches) == 1:
        return next(iter(matches.values()))
    if len(matches) > 1:
        profile_ids = ", ".join(sorted(matches))
        raise ValueError(
            f"multiple tensor profiles match observation tensor size {tensor_size}: "
            f"{profile_ids}; pass tensor_profile_id or observation_layout explicitly"
        )
    raise ValueError(
        f"no registered tensor profile matches observation tensor size {tensor_size}; "
        "pass tensor_profile_id or observation_layout explicitly"
    )


def list_tensor_profiles() -> list[str]:
    digimon_engine = _require_digimon_engine()

    list_profiles = getattr(digimon_engine, "list_observation_profiles", None)
    if list_profiles is None:
        list_profiles = getattr(digimon_engine, "list_tensor_profiles", None)
    if list_profiles is None:
        raise RuntimeError(
            "digimon_engine is installed but exposes no profile registry; "
            "rebuild the wheel (`cd code/digimon-engine-py && maturin develop --release`)."
        )

    result: list[str] = []
    for profile_id in list_profiles():
        canonical = _canonicalize_tensor_profile_id(profile_id)
        if canonical in _STANDARD_COMPACT_V1_ALIASES:
            continue  # retired from the training env
        result.append(canonical)
    return result


def _profile_from_observation_layout(raw: Any) -> TensorProfile:
    profile_id = _raw_get(raw, "profile_id", _raw_get(raw, "id", None))
    canonical_id = _canonicalize_tensor_profile_id(profile_id)
    card_id_positions = _as_tuple(_raw_get(raw, "card_id_positions"))
    scalar_positions = _as_tuple(_raw_get(raw, "scalar_positions"))
    sections = tuple(
        TensorSection(
            name=str(_raw_get(section, "name", _raw_get(section, "id", ""))),
            offset=int(_raw_get(section, "offset", _raw_get(section, "start", 0))),
            size=int(_raw_get(section, "size", _raw_get(section, "len", 0))),
            shape=tuple(int(v) for v in _raw_get(section, "shape", ())),
        )
        for section in _raw_get(raw, "sections", ())
    )

    return TensorProfile(
        id=canonical_id,
        game_mode=str(_raw_get(raw, "game_mode", canonical_id.split("_", 1)[0])),
        version=int(_raw_get(raw, "version", _raw_get(raw, "tensor_version", 1))),
        tensor_version=int(_raw_get(raw, "tensor_version")),
        feature_schema_version=str(_raw_get(raw, "feature_schema_version")),
        layout_hash=str(_raw_get(raw, "layout_hash")),
        tensor_size=int(_raw_get(raw, "tensor_size")),
        field_slots=int(_raw_get(raw, "field_slots")),
        slot_size=int(_raw_get(raw, "slot_size")),
        max_sources=int(_raw_get(raw, "max_sources")),
        card_id_slot_count=int(_raw_get(raw, "card_id_slot_count", len(card_id_positions))),
        scalar_slot_count=int(_raw_get(raw, "scalar_slot_count", len(scalar_positions))),
        card_id_positions=card_id_positions,
        scalar_positions=scalar_positions,
        sections=sections,
    )


def _raw_get(raw: Any, key: str, default: Any = ...):
    if isinstance(raw, dict):
        if default is ...:
            return raw[key]
        return raw.get(key, default)
    if default is ...:
        return getattr(raw, key)
    return getattr(raw, key, default)


def _load_digimon_engine():
    loaded = sys.modules.get("digimon_engine")
    if loaded is not None:
        return loaded

    if importlib.util.find_spec("digimon_engine") is None:
        return None

    return importlib.import_module("digimon_engine")


def _require_digimon_engine():
    digimon_engine = _load_digimon_engine()
    if digimon_engine is None:
        raise RuntimeError(
            "The Rust engine wheel (digimon_engine) is not installed, but the "
            "training env requires it. Build it with "
            "`cd code/digimon-engine-py && maturin develop --release` "
            "(or install the prebuilt wheel)."
        )
    return digimon_engine


def _reject_retired_profile(profile_id: str | None) -> None:
    if profile_id is not None and profile_id in _STANDARD_COMPACT_V1_ALIASES:
        raise ValueError(
            "standard_compact_v1 is retired from the training env; use "
            "standard_lite_v2 (the training default). The Rust engine retains "
            "its own compact tensor builder for offline parity use."
        )


def _canonicalize_tensor_profile_id(profile_id: str) -> str:
    if profile_id in _STANDARD_COMPACT_V1_ALIASES:
        return _CANONICAL_STANDARD_COMPACT_V1
    return profile_id


def _as_tuple(values: Iterable[int]) -> tuple[int, ...]:
    return tuple(int(v) for v in values)
