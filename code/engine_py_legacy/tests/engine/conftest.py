"""Engine test conftest — fixtures for unit tests that use bare make_card() objects."""

import pytest
from engine_py_legacy.engine.data.card_registry import CardRegistry


@pytest.fixture
def init_test_registry():
    """Initialize CardRegistry with specific test card IDs.

    Use this in engine unit tests that create bare CardSource objects via make_card()
    and need the registry to recognize specific card IDs.

    Usage:
        def test_something(init_test_registry):
            init_test_registry(["BT14-001", "BT14-002"])
    """
    def _init(card_ids: list[str]):
        CardRegistry.initialize_from_list(card_ids)
    return _init
