"""Board tensor profile metadata used by RL feature extraction.

Rust owns the canonical profile registry. The fallback keeps imports working
before a local PyO3 wheel has been rebuilt, and it must match standard_v1.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable


@dataclass(frozen=True)
class TensorProfile:
    id: str
    game_mode: str
    version: int
    tensor_size: int
    field_slots: int
    slot_size: int
    max_sources: int
    card_id_slot_count: int
    scalar_slot_count: int
    card_id_positions: tuple[int, ...]
    scalar_positions: tuple[int, ...]


def get_tensor_profile(profile_id: str | None = None) -> TensorProfile:
    try:
        import digimon_engine
    except ImportError:
        if profile_id not in (None, "standard_v1"):
            raise ValueError(f"unknown tensor profile: {profile_id}") from None
        return _legacy_standard_v1()

    get_profile = getattr(digimon_engine, "get_tensor_profile", None)
    if get_profile is None:
        if profile_id not in (None, "standard_v1"):
            raise ValueError(f"unknown tensor profile: {profile_id}") from None
        return _legacy_standard_v1()

    raw = get_profile(profile_id)
    return TensorProfile(
        id=raw.id,
        game_mode=raw.game_mode,
        version=raw.version,
        tensor_size=raw.tensor_size,
        field_slots=raw.field_slots,
        slot_size=raw.slot_size,
        max_sources=raw.max_sources,
        card_id_slot_count=raw.card_id_slot_count,
        scalar_slot_count=raw.scalar_slot_count,
        card_id_positions=tuple(raw.card_id_positions),
        scalar_positions=tuple(raw.scalar_positions),
    )


def list_tensor_profiles() -> list[str]:
    try:
        import digimon_engine
    except ImportError:
        return ["standard_v1"]

    list_profiles = getattr(digimon_engine, "list_tensor_profiles", None)
    if list_profiles is None:
        return ["standard_v1"]

    return list(list_profiles())


def _legacy_standard_v1() -> TensorProfile:
    from engine_py_legacy.engine.data.tensor_layout import (
        CARD_ID_POSITIONS,
        NUM_CARD_SLOTS,
        NUM_SCALAR_SLOTS,
        SCALAR_POSITIONS,
    )
    from engine_py_legacy.engine.game import FIELD_SLOTS, MAX_SOURCES, SLOT_SIZE, TENSOR_SIZE

    return TensorProfile(
        id="standard_v1",
        game_mode="standard",
        version=1,
        tensor_size=TENSOR_SIZE,
        field_slots=FIELD_SLOTS,
        slot_size=SLOT_SIZE,
        max_sources=MAX_SOURCES,
        card_id_slot_count=NUM_CARD_SLOTS,
        scalar_slot_count=NUM_SCALAR_SLOTS,
        card_id_positions=_as_tuple(CARD_ID_POSITIONS),
        scalar_positions=_as_tuple(SCALAR_POSITIONS),
    )


def _as_tuple(values: Iterable[int]) -> tuple[int, ...]:
    return tuple(int(v) for v in values)
