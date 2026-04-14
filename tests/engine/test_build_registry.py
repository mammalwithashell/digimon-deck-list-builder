"""Tests for tools/build_registry.py — card registry builder logic."""

import pytest

from tools.build_registry import card_sort_key, build_registry, REGISTRY_CAPACITY


class TestCardSortKey:
    """Verify natural sort key parsing for all set types."""

    def test_booster_set(self):
        assert card_sort_key("BT14-001") == ("BT", 14, 1)

    def test_booster_set_single_digit(self):
        assert card_sort_key("BT1-001") == ("BT", 1, 1)

    def test_extra_booster(self):
        assert card_sort_key("EX8-050") == ("EX", 8, 50)

    def test_starter_set(self):
        assert card_sort_key("ST1-01") == ("ST", 1, 1)

    def test_starter_set_double_digit(self):
        assert card_sort_key("ST17-09") == ("ST", 17, 9)

    def test_advanced_booster(self):
        assert card_sort_key("AD1-001") == ("AD", 1, 1)

    def test_resurgence_booster(self):
        assert card_sort_key("RB1-001") == ("RB", 1, 1)

    def test_limited_pack(self):
        assert card_sort_key("LM-001") == ("LM", 0, 1)

    def test_promo(self):
        assert card_sort_key("P-001") == ("P", 0, 1)

    def test_natural_sort_order(self):
        """BT1 < BT2 < BT10 (not BT1 < BT10 < BT2 like string sort)."""
        ids = ["BT10-001", "BT2-001", "BT1-001"]
        sorted_ids = sorted(ids, key=card_sort_key)
        assert sorted_ids == ["BT1-001", "BT2-001", "BT10-001"]

    def test_cross_set_sort_order(self):
        """AD < BT < EX < LM < P < RB < ST (alphabetical by prefix letters)."""
        ids = ["ST1-01", "BT1-001", "P-001", "EX1-001", "AD1-001", "RB1-001", "LM-001"]
        sorted_ids = sorted(ids, key=card_sort_key)
        assert sorted_ids == [
            "AD1-001", "BT1-001", "EX1-001", "LM-001", "P-001", "RB1-001", "ST1-01"
        ]


class TestBuildRegistry:
    """Verify the append-only registry building algorithm."""

    def test_fresh_build(self):
        """No existing mappings -> assigns indices 1..N in natural sort order."""
        api_ids = ["BT1-002", "BT1-001", "BT2-001"]
        registry = build_registry(api_ids, {})
        assert registry["BT1-001"] == 1
        assert registry["BT1-002"] == 2
        assert registry["BT2-001"] == 3

    def test_preserves_existing_indices(self):
        """Existing mappings must never change."""
        existing = {"BT1-001": 1, "BT1-003": 2}
        api_ids = ["BT1-001", "BT1-002", "BT1-003"]
        registry = build_registry(api_ids, existing)
        # Existing preserved
        assert registry["BT1-001"] == 1
        assert registry["BT1-003"] == 2
        # New card gets next index
        assert registry["BT1-002"] == 3

    def test_removed_cards_preserved(self):
        """Cards in existing but not in API keep their index."""
        existing = {"BT1-001": 1, "BT1-099": 2}
        api_ids = ["BT1-001"]  # BT1-099 "removed" from API
        registry = build_registry(api_ids, existing)
        assert registry["BT1-001"] == 1
        assert registry["BT1-099"] == 2  # Still preserved
        assert len(registry) == 2

    def test_capacity_exceeded(self):
        """Should raise ValueError when total exceeds capacity."""
        api_ids = [f"BT1-{i:03d}" for i in range(1, 12)]
        with pytest.raises(ValueError, match="exceeds registry capacity"):
            build_registry(api_ids, {}, capacity=10)

    def test_index_zero_never_assigned(self):
        """Index 0 is reserved for padding."""
        api_ids = ["P-001", "P-002", "P-003"]
        registry = build_registry(api_ids, {})
        assert 0 not in registry.values()
        assert min(registry.values()) == 1

    def test_deterministic(self):
        """Same inputs always produce same outputs."""
        api_ids = ["EX1-001", "BT1-001", "ST1-01"]
        r1 = build_registry(api_ids, {})
        r2 = build_registry(api_ids, {})
        assert r1 == r2

    def test_deduplication(self):
        """Duplicate card IDs in API should produce one entry."""
        api_ids = ["BT1-001", "BT1-001", "BT1-002", "BT1-002"]
        registry = build_registry(api_ids, {})
        assert len(registry) == 2

    def test_new_cards_append_after_max(self):
        """New cards always get indices after the current max, not filling gaps."""
        existing = {"BT1-001": 1, "BT1-005": 100}  # Gap between 1 and 100
        api_ids = ["BT1-001", "BT1-002", "BT1-005"]
        registry = build_registry(api_ids, existing)
        assert registry["BT1-001"] == 1
        assert registry["BT1-005"] == 100
        assert registry["BT1-002"] == 101  # After max, not filling gap

    def test_natural_sort_for_new_cards(self):
        """New cards sorted by natural sort, not alphabetical."""
        existing = {}
        api_ids = ["BT10-001", "BT2-001", "BT1-001"]
        registry = build_registry(api_ids, existing)
        # Natural sort: BT1 < BT2 < BT10
        assert registry["BT1-001"] == 1
        assert registry["BT2-001"] == 2
        assert registry["BT10-001"] == 3

    def test_norm_id_not_computed_by_build(self):
        """build_registry only returns card_id -> index mapping, no norm_ids."""
        api_ids = ["BT1-001"]
        registry = build_registry(api_ids, {})
        # Registry values are plain integers
        assert isinstance(registry["BT1-001"], int)
