"""Tests for tools/resolve_deck.py."""

import json
import pytest
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))


class TestResolveCards:
    """Tests for resolve_cards() — enriching raw card IDs."""

    def test_single_frozen_card(self):
        """A card in the frozen manifest should have script_status='frozen'."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001"])
        assert len(entries) == 1
        entry = entries[0]
        assert entry.card_id == "BT10-001"
        assert entry.script_status == "frozen"
        assert entry.script_path is not None
        assert entry.card_name != ""
        assert entry.card_kind in ("Digimon", "Tamer", "Option", "DigiEgg")

    def test_missing_card(self):
        """A card ID not in manifest or on disk should be 'missing'."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["ZZ99-999"])
        assert len(entries) == 1
        assert entries[0].script_status == "missing"
        assert entries[0].script_path is None
        assert entries[0].card_id == "ZZ99-999"

    def test_multiple_cards_sorted(self):
        """Multiple cards should be returned sorted by card_id."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-003", "BT10-001", "BT10-002"])
        ids = [e.card_id for e in entries]
        assert ids == ["BT10-001", "BT10-002", "BT10-003"]

    def test_card_metadata_fields(self):
        """Card entries should have populated metadata fields from cards.json."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001"])
        entry = entries[0]
        assert isinstance(entry.colors, list)
        assert isinstance(entry.play_cost, int) or entry.play_cost is None
        assert isinstance(entry.effect_text, str)
        assert isinstance(entry.inherited_text, str)
        assert isinstance(entry.deck_frequency, int)
        assert entry.deck_frequency == 0  # no archetype context

    def test_deduplicates_input(self):
        """Duplicate card IDs in input should produce one entry."""
        from tools.resolve_deck import resolve_cards

        entries = resolve_cards(["BT10-001", "BT10-001", "BT10-001"])
        assert len(entries) == 1


class TestResolveArchetype:
    """Tests for resolve_archetype() — full archetype resolution."""

    def test_known_archetype(self):
        """Resolving a known archetype should return a populated manifest."""
        from tools.resolve_deck import resolve_archetype

        import json
        from pathlib import Path

        lib = json.loads(Path("digimon_gym/engine/data/deck_library.json").read_text(encoding="utf-8"))
        archetypes = lib.get("archetypes", {})
        arch_name = None
        for name, data in archetypes.items():
            if data.get("decklists"):
                arch_name = name
                break
        assert arch_name is not None, "No archetypes with decklists found"

        manifest = resolve_archetype(arch_name)
        assert manifest.archetype_name == arch_name
        assert len(manifest.unique_cards) > 0
        assert manifest.total_decklists > 0
        assert 0.0 <= manifest.coverage_pct <= 1.0
        assert manifest.frozen_count + manifest.generated_count + manifest.missing_count == len(manifest.unique_cards)

    def test_alias_resolution(self):
        """An aliased name should resolve to the canonical archetype."""
        from tools.resolve_deck import resolve_archetype

        import json
        from pathlib import Path

        aliases = json.loads(
            Path("digimon_gym/engine/data/archetype_aliases.json").read_text(encoding="utf-8")
        )
        lib = json.loads(
            Path("digimon_gym/engine/data/deck_library.json").read_text(encoding="utf-8")
        )
        archetypes = lib.get("archetypes", {})

        for canonical, alias_list in aliases.items():
            if canonical.startswith("_"):
                continue
            if canonical in archetypes and alias_list:
                alias = alias_list[0]
                manifest = resolve_archetype(alias)
                assert manifest.archetype_name == canonical
                assert manifest.input_name == alias
                return

        pytest.skip("No testable alias found")

    def test_cards_override(self):
        """cards_override should bypass deck_library lookup."""
        from tools.resolve_deck import resolve_archetype

        override = ["BT10-001", "BT10-002"]
        manifest = resolve_archetype("custom-test", cards_override=override)
        assert manifest.archetype_name == "custom-test"
        assert len(manifest.unique_cards) == 2
        assert manifest.total_decklists == 0
        assert manifest.meta_share == 0.0
        assert manifest.best_decklist == []

    def test_deck_pool_written(self, tmp_path, monkeypatch):
        """resolve_archetype should write deck_pool.json."""
        from tools import resolve_deck
        from tools.resolve_deck import resolve_archetype

        monkeypatch.setattr(resolve_deck, "_QA_DIR", tmp_path)

        manifest = resolve_archetype(
            "test-pool-write", cards_override=["BT10-001", "BT10-002"]
        )
        pool_path = tmp_path / "test-pool-write" / "deck_pool.json"
        assert pool_path.exists()
        pool = json.loads(pool_path.read_text(encoding="utf-8"))
        assert pool == ["BT10-001", "BT10-002"]

    def test_unknown_archetype_empty(self):
        """An unknown archetype with no cards_override should return empty manifest."""
        from tools.resolve_deck import resolve_archetype

        manifest = resolve_archetype("zzz-nonexistent-archetype-12345")
        assert len(manifest.unique_cards) == 0
        assert manifest.total_decklists == 0

    def test_best_decklist_populated(self):
        """best_decklist should contain a valid deck list from the archetype."""
        from tools.resolve_deck import resolve_archetype

        import json
        from pathlib import Path

        lib = json.loads(Path("digimon_gym/engine/data/deck_library.json").read_text(encoding="utf-8"))
        archetypes = lib.get("archetypes", {})
        for name, data in archetypes.items():
            if data.get("decklists"):
                manifest = resolve_archetype(name)
                assert len(manifest.best_decklist) > 0
                unique_ids = {c.card_id for c in manifest.unique_cards}
                for cid in set(manifest.best_decklist):
                    assert cid in unique_ids
                return

        pytest.skip("No archetypes with decklists found")
