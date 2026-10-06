"""Tests for digimon_gym/agents/gauntlet.py — MetaGauntlet and GauntletWrapper.

Tests use mock deck library JSON; no network calls or real game engine required
for most tests.

v2: Updated for survivorship-bias fix — TI now derived from DigiLab stats only,
    confidence threshold gates conversion_rate, deck pool routing prefers
    higher-quality sources (digimonmeta > egman > others).
"""

import json
import os
from unittest.mock import MagicMock, patch

import numpy as np
import gymnasium
import pytest

import digimon_gym.agents.gauntlet as gauntlet_module
from digimon_gym.agents.gauntlet import (
    ArchetypeStats,
    DeckEntry,
    GeneralistDeckPool,
    GeneralistDeckPoolWrapper,
    GauntletWrapper,
    MetaGauntlet,
    UnimplementedDeckError,
    _load_not_ready_card_ids,
    stable_deck_id,
    validate_implemented_deck,
)


# ─── Fixtures ────────────────────────────────────────────────────────

def _make_deck_library(archetypes_data: dict) -> dict:
    """Build a minimal deck_library.json structure."""
    return {
        "version": 2,
        "generated_at": "2026-02-19T00:00:00+00:00",
        "total_entries": sum(
            len(a.get("decklists", []))
            for a in archetypes_data.values()
        ),
        "archetypes": archetypes_data,
    }


def _make_archetype(
    name: str,
    n_decks: int = 1,
    *,
    # DigiLab stats (sole source of truth for TI)
    digilab_times_played: int = 10,
    digilab_conversion_rate: float = 0.20,
    digilab_win_rate: float = 0.50,
    digilab_top4_rate: float = 0.10,
    # Scraper stats (legacy, NOT used for TI — kept for data lineage)
    scraper_meta_share: float = 0.10,
    scraper_conversion_rate: float = 1.0,  # always 100% from DigimonMeta
    # Deck source mix
    sources: str = "test",
) -> dict:
    """Build a single archetype entry for the deck library.

    By default, generates a valid deck_library archetype with DigiLab stats
    that are DIFFERENT from scraper stats, to verify the gauntlet uses DigiLab
    and ignores scraper stats.
    """
    decklists = []
    for i in range(n_decks):
        # Minimal valid deck: 50 main + 5 eggs, stored as TTS format JSON array string
        card_ids = ["BT12-022"] * 50 + ["BT12-002"] * 5
        tts_decklist = json.dumps(card_ids)
        source = sources.split(",")[i % len(sources.split(","))].strip()
        decklists.append({
            "deck_id": f"{name.lower()}_{i:03d}",
            "source": source,
            "source_url": "",
            "decklist": tts_decklist,
            "format": "BT24",
            "placement": "1st Place" if i == 0 else f"{i + 2}",
            "is_top_cut": i == 0,
        })

    return {
        "archetype_name": name,
        "primary_color": "Red",
        "display_card_id": f"BT24-{0:03d}",
        # Scraper-derived stats (intentionally inflated — should be IGNORED by TI)
        "stats": {
            "times_played": n_decks,
            "meta_share": scraper_meta_share,
            "top_cut_count": 1,
            "conversion_rate": scraper_conversion_rate,
        },
        # DigiLab stats (SOLE source of truth for TI)
        "digilab_stats": {
            "times_played": digilab_times_played,
            "conversion_rate": digilab_conversion_rate,
            "win_rate": digilab_win_rate,
            "top4_rate": digilab_top4_rate,
        },
        "decklists": decklists,
    }


def _with_card(card_id: str) -> str:
    """A TTS decklist like `_make_archetype`'s, with one main-deck copy swapped for `card_id`."""
    return json.dumps(["BT12-022"] * 49 + [card_id] + ["BT12-002"] * 5)


def _write_dsl_ledger(tmp_path, rows: dict) -> str:
    """Write a minimal `validated_cards_dsl.json`; `rows` maps card ID -> (status, archetype label)."""
    ledger = {
        "version": 1,
        "cards": {
            card_id: {"status": status, "archetype": label}
            for card_id, (status, label) in rows.items()
        },
    }
    path = tmp_path / "validated_cards_dsl.json"
    path.write_text(json.dumps(ledger))
    return str(path)


@pytest.fixture
def default_library(tmp_path, monkeypatch):
    """Point the module's default deck library and DSL ledger at tmp files.

    Loading the default library is the production path: it is the load that
    reads the DSL ledger itself. Write the library to the returned path and the
    ledger with `_write_dsl_ledger(tmp_path, ...)`.
    """
    library_path = tmp_path / "deck_library.json"
    monkeypatch.setattr(gauntlet_module, "DECK_LIBRARY_PATH", str(library_path))
    monkeypatch.setattr(
        gauntlet_module, "_QA_DSL_STATUS_PATH", tmp_path / "validated_cards_dsl.json"
    )
    return library_path


@pytest.fixture
def basic_library(tmp_path):
    """A deck library with 3 archetypes of varying DigiLab threat."""
    lib = _make_deck_library({
        "MetaKing": _make_archetype(
            "MetaKing", n_decks=3,
            digilab_times_played=20, digilab_conversion_rate=0.40,
        ),
        "MidTier": _make_archetype(
            "MidTier", n_decks=2,
            digilab_times_played=10, digilab_conversion_rate=0.20,
        ),
        "Rogue": _make_archetype(
            "Rogue", n_decks=1,
            digilab_times_played=8, digilab_conversion_rate=0.60,
        ),
    })
    path = tmp_path / "deck_library.json"
    path.write_text(json.dumps(lib))
    return str(path)


@pytest.fixture
def sleeper_library(tmp_path):
    """Library where a rare deck has very high conversion rate in DigiLab."""
    lib = _make_deck_library({
        "Popular": _make_archetype(
            "Popular", n_decks=5,
            digilab_times_played=30, digilab_conversion_rate=0.10,
        ),
        "Sleeper": _make_archetype(
            "Sleeper", n_decks=1,
            digilab_times_played=6, digilab_conversion_rate=0.80,
        ),
    })
    path = tmp_path / "deck_library.json"
    path.write_text(json.dumps(lib))
    return str(path)


@pytest.fixture
def confidence_library(tmp_path):
    """Library with archetypes above and below confidence threshold."""
    lib = _make_deck_library({
        "HighSample": _make_archetype(
            "HighSample", n_decks=2,
            digilab_times_played=10, digilab_conversion_rate=0.50,
        ),
        "LowSample": _make_archetype(
            "LowSample", n_decks=1,
            digilab_times_played=3, digilab_conversion_rate=0.90,
        ),
    })
    path = tmp_path / "deck_library.json"
    path.write_text(json.dumps(lib))
    return str(path)


# ─── MetaGauntlet Tests ─────────────────────────────────────────────

class TestMetaGauntlet:

    def test_load_from_json(self, basic_library):
        g = MetaGauntlet()
        g.load(basic_library)
        assert g.archetype_count == 3
        assert g.deck_count == 6  # 3 + 2 + 1

    def test_load_filters_decks_with_unimplemented_cards_by_default(self, tmp_path):
        lib = _make_deck_library({
            "Implemented": _make_archetype(
                "Implemented", n_decks=1,
                digilab_times_played=10, digilab_conversion_rate=0.20,
            ),
            "GapDeck": _make_archetype(
                "GapDeck", n_decks=1,
                digilab_times_played=10, digilab_conversion_rate=0.90,
            ),
        })
        gap_deck = ["BT12-022"] * 49 + ["BT99-999"] + ["BT12-002"] * 5
        lib["archetypes"]["GapDeck"]["decklists"][0]["decklist"] = json.dumps(gap_deck)
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(path))

        assert set(g.archetypes) == {"Implemented"}
        assert g.deck_count == 1
        assert g._deck_pool[0].archetype_name == "Implemented"

    def test_random_sampling_mode_uses_uniform_deck_weights(self, tmp_path):
        lib = _make_deck_library({
            "HighTI": _make_archetype(
                "HighTI", n_decks=3,
                digilab_times_played=100, digilab_conversion_rate=0.90,
            ),
            "LowTI": _make_archetype(
                "LowTI", n_decks=1,
                digilab_times_played=5, digilab_conversion_rate=0.01,
            ),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(
            sampling_mode="random",
            implemented_card_ids={"BT12-002", "BT12-022"},
        )
        g.load(str(path))

        assert g.deck_count == 4
        assert np.allclose(g._weights, np.ones(4) / 4)
        assert g.archetypes["HighTI"].sampling_probability == pytest.approx(0.75)
        assert g.archetypes["LowTI"].sampling_probability == pytest.approx(0.25)

    def test_loaded_decks_use_stable_content_addressed_ids(self, tmp_path):
        lib = _make_deck_library({
            "Ready": _make_archetype("Ready", n_decks=1),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(path))

        deck = g._deck_pool[0]
        assert deck.deck_id == stable_deck_id(deck.card_ids)
        assert deck.source_deck_id == "ready_000"

    def test_validate_implemented_deck_reports_missing_ids(self):
        with pytest.raises(UnimplementedDeckError, match="BT99-999"):
            validate_implemented_deck(
                ["BT12-022", "BT99-999", "BT99-999"],
                {"BT12-022"},
                label="test deck",
            )

    def test_generalist_snapshot_round_trip_and_hash(self, tmp_path):
        lib = _make_deck_library({
            "A": _make_archetype("A", n_decks=1),
            "B": _make_archetype("B", n_decks=2),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(path))
        pool = g.as_generalist_pool()
        snapshot = tmp_path / "pool.json"

        written_hash = pool.write_snapshot(snapshot)
        loaded = GeneralistDeckPool.from_snapshot(
            snapshot,
            implemented_card_ids={"BT12-002", "BT12-022"},
        )

        assert loaded.snapshot_hash == written_hash
        assert loaded.archetype_names == ["A", "B"]
        assert loaded.deck_count == 3

    def test_snapshot_reuse_is_independent_of_library_order(self, tmp_path):
        lib = _make_deck_library({
            "A": _make_archetype("A", n_decks=1),
            "B": _make_archetype("B", n_decks=2),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(path))
        snapshot = tmp_path / "pool.json"
        g.as_generalist_pool().write_snapshot(snapshot)

        pool1 = GeneralistDeckPool.from_snapshot(snapshot)
        pool2 = GeneralistDeckPool.from_snapshot(snapshot)
        rng1 = np.random.default_rng(123)
        rng2 = np.random.default_rng(123)

        seq1 = [pool1.sample_uniform_archetype(rng1).deck_id for _ in range(12)]
        seq2 = [pool2.sample_uniform_archetype(rng2).deck_id for _ in range(12)]

        assert seq1 == seq2

    def test_different_curriculum_seeds_change_deck_pair_schedule(self, tmp_path):
        lib = _make_deck_library({
            "A": _make_archetype("A", n_decks=3),
            "B": _make_archetype("B", n_decks=3),
        })
        variants = [
            ["BT12-022"] * 50 + ["BT12-002"] * 5,
            ["BT12-022"] * 49 + ["BT12-031"] + ["BT12-002"] * 5,
            ["BT12-022"] * 48 + ["BT12-031"] * 2 + ["BT12-002"] * 5,
        ]
        for archetype in lib["archetypes"].values():
            for idx, decklist in enumerate(archetype["decklists"]):
                decklist["decklist"] = json.dumps(variants[idx])
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022", "BT12-031"})
        g.load(str(path))
        pool = g.as_generalist_pool()

        seq1 = [
            pool.sample_uniform_archetype(np.random.default_rng(seed)).deck_id
            for seed in range(10, 20)
        ]
        seq2 = [
            pool.sample_uniform_archetype(np.random.default_rng(seed)).deck_id
            for seed in range(20, 30)
        ]

        assert seq1 != seq2
        all_deck_ids = {
            deck.deck_id
            for decks in pool.archetypes.values()
            for deck in decks
        }
        assert set(seq1).issubset(all_deck_ids)
        assert set(seq2).issubset(all_deck_ids)

    def test_uniform_archetype_sampling_ignores_deck_count_bias(self, tmp_path):
        lib = _make_deck_library({
            "ManyDecks": _make_archetype("ManyDecks", n_decks=10),
            "OneDeck": _make_archetype("OneDeck", n_decks=1),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(path))
        pool = g.as_generalist_pool()
        rng = np.random.default_rng(7)

        counts = {"ManyDecks": 0, "OneDeck": 0}
        for _ in range(2000):
            counts[pool.sample_uniform_archetype(rng).archetype_name] += 1

        assert abs(counts["ManyDecks"] / 2000 - 0.5) < 0.06
        assert abs(counts["OneDeck"] / 2000 - 0.5) < 0.06

    def test_ti_uses_digilab_stats_not_scraper(self, basic_library):
        """TI must be computed from DigiLab meta_share/conversion_rate,
        NOT from the scraper 'stats' block (which is survivorship-biased)."""
        g = MetaGauntlet(alpha=1.0, beta=2.0)
        g.load(basic_library)

        mk = g.archetypes["MetaKing"]
        # DigiLab meta_share = 20 / (20+10+8) = 20/38
        expected_ms = 20 / 38
        expected_ti = expected_ms * 1.0 + 0.40 * 2.0  # conv from digilab
        assert abs(mk.threat_index - expected_ti) < 1e-6

        # Verify scraper stats are NOT used
        assert abs(mk.threat_index - (0.10 * 1.0 + 1.0 * 2.0)) > 0.1

    def test_ti_formula_with_digilab(self, basic_library):
        """Verify the exact TI formula for all archetypes."""
        g = MetaGauntlet(alpha=1.0, beta=2.0)
        g.load(basic_library)

        total_tp = 20 + 10 + 8  # 38
        for name, tp, conv in [("MetaKing", 20, 0.40), ("MidTier", 10, 0.20), ("Rogue", 8, 0.60)]:
            ms = tp / total_tp
            expected_ti = ms * 1.0 + conv * 2.0
            actual_ti = g.archetypes[name].threat_index
            assert abs(actual_ti - expected_ti) < 1e-6, (
                f"{name}: expected TI={expected_ti:.6f}, got {actual_ti:.6f}"
            )

    def test_custom_alpha_beta(self, basic_library):
        g = MetaGauntlet(alpha=3.0, beta=1.0)
        g.load(basic_library)

        mk = g.archetypes["MetaKing"]
        ms = 20 / 38
        assert abs(mk.threat_index - (ms * 3.0 + 0.40 * 1.0)) < 1e-6

    def test_confidence_threshold_gates_conversion(self, confidence_library):
        """Archetypes below confidence_min_appearances use meta_share only."""
        g = MetaGauntlet(alpha=1.0, beta=2.0, confidence_min_appearances=5)
        g.load(confidence_library)

        total_tp = 10 + 3  # 13
        high = g.archetypes["HighSample"]
        low = g.archetypes["LowSample"]

        # HighSample (10 appearances >= 5): TI = meta_share + conv*beta
        hs_ms = 10 / total_tp
        assert abs(high.threat_index - (hs_ms * 1.0 + 0.50 * 2.0)) < 1e-6

        # LowSample (3 appearances < 5): TI = meta_share ONLY (no conversion)
        ls_ms = 3 / total_tp
        assert abs(low.threat_index - (ls_ms * 1.0)) < 1e-6

        # Verify the high-conv LowSample doesn't sneak in its 0.90 conversion
        assert low.threat_index < 0.5  # Would be ~2.03 with conv factored in

    def test_confidence_threshold_custom(self, confidence_library):
        """Custom confidence_min_appearances=2 lets LowSample use conversion."""
        g = MetaGauntlet(alpha=1.0, beta=2.0, confidence_min_appearances=2)
        g.load(confidence_library)

        total_tp = 13
        low = g.archetypes["LowSample"]
        ls_ms = 3 / total_tp
        # With threshold=2, LowSample (3 >= 2) gets conversion factored in
        expected = ls_ms * 1.0 + 0.90 * 2.0
        assert abs(low.threat_index - expected) < 1e-6

    def test_sleeper_rule_activates(self, sleeper_library):
        g = MetaGauntlet(sleeper_threshold=0.50, sleeper_floor=0.05,
                         confidence_min_appearances=5)
        g.load(sleeper_library)

        sleeper = g.archetypes["Sleeper"]
        # Sleeper has digilab_conversion_rate=0.80 > 0.50, times_played=6 >= 5
        assert sleeper.sampling_probability >= 0.05 - 1e-9

    def test_sleeper_rule_blocked_by_confidence(self, tmp_path):
        """Sleeper rule should NOT activate if below confidence threshold."""
        lib = _make_deck_library({
            "Popular": _make_archetype(
                "Popular", n_decks=5,
                digilab_times_played=30, digilab_conversion_rate=0.10,
            ),
            "LowDataSleeper": _make_archetype(
                "LowDataSleeper", n_decks=1,
                digilab_times_played=2, digilab_conversion_rate=0.90,
            ),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(sleeper_threshold=0.50, sleeper_floor=0.10,
                         confidence_min_appearances=5)
        g.load(str(path))

        # LowDataSleeper has conv=0.90 but only 2 appearances < 5
        # Sleeper rule should NOT fire — its probability should be < floor
        low = g.archetypes["LowDataSleeper"]
        assert low.sampling_probability < 0.10

    def test_sampling_weights_sum_to_one(self, basic_library):
        g = MetaGauntlet()
        g.load(basic_library)
        assert abs(g._weights.sum() - 1.0) < 1e-9

    def test_sampling_weights_all_positive(self, basic_library):
        g = MetaGauntlet()
        g.load(basic_library)
        assert np.all(g._weights > 0)

    def test_sample_opponent_returns_deck_entry(self, basic_library):
        g = MetaGauntlet(seed=42)
        g.load(basic_library)
        deck = g.sample_opponent()
        assert isinstance(deck, DeckEntry)
        assert len(deck.card_ids) > 0
        assert deck.threat_index > 0
        assert deck.archetype_name in ("MetaKing", "MidTier", "Rogue")

    def test_sample_opponent_empty_raises(self, tmp_path):
        path = tmp_path / "empty.json"
        path.write_text(json.dumps({"version": 2, "archetypes": {}}))
        g = MetaGauntlet()
        g.load(str(path))
        with pytest.raises(RuntimeError, match="no decks loaded"):
            g.sample_opponent()

    def test_sample_opponents_batch(self, basic_library):
        g = MetaGauntlet(seed=42)
        g.load(basic_library)
        decks = g.sample_opponents(10)
        assert len(decks) == 10
        assert all(isinstance(d, DeckEntry) for d in decks)

    def test_seed_reproducibility(self, basic_library):
        g1 = MetaGauntlet(seed=123)
        g1.load(basic_library)
        seq1 = [g1.sample_opponent().deck_id for _ in range(20)]

        g2 = MetaGauntlet(seed=123)
        g2.load(basic_library)
        seq2 = [g2.sample_opponent().deck_id for _ in range(20)]

        assert seq1 == seq2

    def test_sample_distribution_approximates_weights(self, basic_library):
        """Large sample roughly matches expected distribution."""
        g = MetaGauntlet(seed=42)
        g.load(basic_library)

        n_samples = 10000
        decks = g.sample_opponents(n_samples)
        counts = {}
        for d in decks:
            counts[d.archetype_name] = counts.get(d.archetype_name, 0) + 1

        for arch in g.archetypes.values():
            expected_frac = arch.sampling_probability
            actual_frac = counts.get(arch.archetype_name, 0) / n_samples
            # Allow 3% absolute tolerance
            assert abs(actual_frac - expected_frac) < 0.03, (
                f"{arch.archetype_name}: expected {expected_frac:.3f}, "
                f"got {actual_frac:.3f}"
            )

    def test_get_archetype_summary(self, basic_library):
        g = MetaGauntlet()
        g.load(basic_library)
        summary = g.get_archetype_summary()
        assert len(summary) == 3
        # Sorted by TI descending
        tis = [s["threat_index"] for s in summary]
        assert tis == sorted(tis, reverse=True)
        # v2 fields present
        assert "digilab_meta_share" in summary[0]
        assert "digilab_conversion_rate" in summary[0]
        assert "digilab_times_played" in summary[0]

    def test_zero_ti_archetypes_get_uniform(self, tmp_path):
        """When all TIs are 0 (no DigiLab data), sampling falls back to uniform."""
        lib = _make_deck_library({
            "A": _make_archetype("A", n_decks=1, digilab_times_played=0, digilab_conversion_rate=0.0),
            "B": _make_archetype("B", n_decks=1, digilab_times_played=0, digilab_conversion_rate=0.0),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet()
        g.load(str(path))
        assert abs(g._weights.sum() - 1.0) < 1e-9
        # Both should have roughly equal weight
        assert abs(g._weights[0] - 0.5) < 1e-9

    def test_no_digilab_stats_gives_zero_ti(self, tmp_path):
        """Archetypes with no DigiLab stats at all get TI=0."""
        lib = _make_deck_library({
            "HasDigiLab": _make_archetype(
                "HasDigiLab", n_decks=1,
                digilab_times_played=10, digilab_conversion_rate=0.30,
            ),
        })
        # Add archetype with NO digilab_stats
        lib["archetypes"]["NoDigiLab"] = {
            "archetype_name": "NoDigiLab",
            "primary_color": "Blue",
            "display_card_id": None,
            "stats": {"times_played": 5, "meta_share": 0.50, "conversion_rate": 1.0},
            "decklists": [{
                "deck_id": "no_dl_001",
                "source": "digimonmeta",
                "source_url": "",
                "card_ids": ["BT24-001"] * 55,
                "card_counts": {"BT24-001": 55},
            }],
        }
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(alpha=1.0, beta=2.0)
        g.load(str(path))

        no_dl = g.archetypes["NoDigiLab"]
        assert no_dl.threat_index == 0.0
        assert no_dl.digilab_meta_share == 0.0

    def test_deck_pool_routing_prefers_digimonmeta(self, tmp_path):
        """Within an archetype, decks should be sorted by source preference."""
        lib = _make_deck_library({
            "Mixed": _make_archetype(
                "Mixed", n_decks=3,
                digilab_times_played=10, digilab_conversion_rate=0.30,
                sources="egman,digimonmeta,file",
            ),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(seed=42)
        g.load(str(path))

        decks = g.archetypes["Mixed"].decks
        # Decks should be sorted: digimonmeta first, then egman, then file
        assert decks[0].source == "digimonmeta"
        assert decks[1].source == "egman"
        assert decks[2].source == "file"


# ─── Training-ready gate (per decklist) ─────────────────────────────


class TestTrainingReadyGate:
    """A decklist is admitted when every card is registered and none has a
    not-ready DSL ledger verdict. The ledger's `archetype` field is a batch
    label, so it plays no part in admission."""

    @pytest.mark.parametrize(
        "status", ["PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"],
    )
    def test_list_with_a_not_ready_card_is_rejected(self, tmp_path, status):
        lib = _make_deck_library({"Mixed": _make_archetype("Mixed", n_decks=2)})
        lib["archetypes"]["Mixed"]["decklists"][1]["decklist"] = _with_card("BT12-031")
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        ledger = _write_dsl_ledger(tmp_path, {
            "BT12-022": ("IMPLEMENTED", "Mixed"),
            "BT12-031": (status, "Mixed"),
        })

        g = MetaGauntlet(
            # Every card is registered, so only the ledger can reject a list.
            implemented_card_ids={"BT12-002", "BT12-022", "BT12-031"},
            not_ready_card_ids=_load_not_ready_card_ids(ledger),
        )
        g.load(str(path))

        assert g.deck_count == 1
        assert g._deck_pool[0].source_deck_id == "mixed_000"

    def test_default_library_admits_a_ready_list_whose_archetype_has_no_ledger_label(
        self, tmp_path, default_library,
    ):
        default_library.write_text(json.dumps(_make_deck_library({
            "Three Musketeers": _make_archetype("Three Musketeers", n_decks=1),
        })))
        # Ledger labels are batch names that never match the library's archetype
        # key, and BT12-002 has no ledger entry at all (a registered spec that was
        # never ledgered still counts as ready).
        _write_dsl_ledger(tmp_path, {"BT12-022": ("IMPLEMENTED", "store-champs-june-2026")})

        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022"})
        g.load(str(default_library))

        assert set(g.archetypes) == {"Three Musketeers"}
        assert g.deck_count == 1

    def test_default_library_drops_only_the_lists_that_play_a_not_ready_card(
        self, tmp_path, default_library,
    ):
        lib = _make_deck_library({"Toho Braves": _make_archetype("Toho Braves", n_decks=2)})
        lib["archetypes"]["Toho Braves"]["decklists"][1]["decklist"] = _with_card("BT12-031")
        default_library.write_text(json.dumps(lib))
        # One PARTIAL card under the archetype's own label: the list that never
        # plays it stays trainable.
        _write_dsl_ledger(tmp_path, {
            "BT12-022": ("IMPLEMENTED", "Toho Braves"),
            "BT12-031": ("PARTIAL", "Toho Braves"),
        })

        g = MetaGauntlet(implemented_card_ids={"BT12-002", "BT12-022", "BT12-031"})
        g.load(str(default_library))

        assert [d.source_deck_id for d in g._deck_pool] == ["toho braves_000"]

    def test_registered_card_missing_from_the_card_database_is_not_playable(
        self, tmp_path, monkeypatch,
    ):
        # P-241 once had a YAML spec (so the registry listed it) but no
        # cards.json entry, and building a game with it raised
        # "Card P-241 not found in card database".
        lib = _make_deck_library({"Mixed": _make_archetype("Mixed", n_decks=2)})
        lib["archetypes"]["Mixed"]["decklists"][1]["decklist"] = _with_card("P-241")
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        class _CardDatabase:
            def get_card(self, card_id):
                return None if card_id == "P-241" else object()

        monkeypatch.setattr(
            gauntlet_module, "load_implemented_card_ids",
            lambda: {"BT12-002", "BT12-022", "P-241"},
        )
        monkeypatch.setattr(gauntlet_module, "CardDatabase", _CardDatabase)

        g = MetaGauntlet()  # no injected card IDs: the engine is asked
        g.load(str(path))

        assert [d.source_deck_id for d in g._deck_pool] == ["mixed_000"]

    def test_missing_ledger_means_no_ledger_gate(self, tmp_path, caplog):
        with caplog.at_level("WARNING", logger="digimon_gym.agents.gauntlet"):
            assert _load_not_ready_card_ids(tmp_path / "absent.json") is None
        assert any("ledger not found" in r.getMessage() for r in caplog.records)


# ─── GauntletWrapper Tests ──────────────────────────────────────────

class _FakeEnv(gymnasium.Env):
    """Minimal Gymnasium env stub for wrapper tests."""

    def __init__(self):
        super().__init__()
        self.observation_space = gymnasium.spaces.Box(
            low=0, high=1, shape=(10,), dtype=np.float32
        )
        self.action_space = gymnasium.spaces.Discrete(5)
        self._reset_return = (np.zeros(10, dtype=np.float32), {"action_mask": np.ones(5)})
        self._step_return = (np.zeros(10, dtype=np.float32), 0.0, False, False, {})

    def reset(self, **kwargs):
        return self._reset_return

    def step(self, action):
        return self._step_return


class TestGauntletWrapper:

    def _make_mock_env(self):
        """Create a minimal Gymnasium env for wrapper tests."""
        return _FakeEnv()

    def _make_gauntlet(self, tmp_path):
        lib = _make_deck_library({
            "Strong": _make_archetype(
                "Strong", n_decks=1,
                digilab_times_played=20, digilab_conversion_rate=0.60,
            ),
            "Weak": _make_archetype(
                "Weak", n_decks=1,
                digilab_times_played=5, digilab_conversion_rate=0.05,
            ),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        g = MetaGauntlet(seed=42)
        g.load(str(path))
        return g

    def test_reset_injects_opponent_deck(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)
        player_deck = ["ST1-03"] * 50

        wrapper = GauntletWrapper(env, g, player_deck)

        # Capture what options are passed to the inner env
        captured_kwargs = {}
        original_reset = env.reset

        def spy_reset(**kwargs):
            captured_kwargs.update(kwargs)
            return original_reset(**kwargs)

        env.reset = spy_reset
        obs, info = wrapper.reset()

        options = captured_kwargs.get("options", {})
        assert options["deck1"] == player_deck
        assert len(options["deck2"]) > 0

        assert "opponent_archetype" in info
        assert "opponent_threat_index" in info

    def test_step_applies_bounty_on_win_vs_strong(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)

        wrapper = GauntletWrapper(
            env, g, ["ST1-03"] * 50,
            bounty_threshold=0.10,
            bounty_bonus=0.5,
        )
        wrapper.reset()
        # Force current opponent to Strong
        strong_deck = [d for d in g._deck_pool if d.archetype_name == "Strong"][0]
        wrapper._current_opponent = strong_deck

        # Simulate terminal win
        env._step_return = (np.zeros(10, dtype=np.float32), 1.0, True, False, {})
        _, reward, terminated, _, info = wrapper.step(0)

        assert terminated is True
        assert reward == 1.5  # 1.0 base + 0.5 bounty
        assert info.get("bounty_applied") is True

    def test_step_no_bounty_on_win_vs_weak(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)

        wrapper = GauntletWrapper(
            env, g, ["ST1-03"] * 50,
            bounty_threshold=0.50,  # Very high threshold
            bounty_bonus=0.5,
        )
        wrapper.reset()
        weak_deck = [d for d in g._deck_pool if d.archetype_name == "Weak"][0]
        wrapper._current_opponent = weak_deck

        env._step_return = (np.zeros(10, dtype=np.float32), 1.0, True, False, {})
        _, reward, _, _, info = wrapper.step(0)

        assert reward == 1.0  # No bounty
        assert "bounty_applied" not in info

    def test_step_no_bounty_on_loss(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)

        wrapper = GauntletWrapper(
            env, g, ["ST1-03"] * 50,
            bounty_threshold=0.0,
            bounty_bonus=0.5,
        )
        wrapper.reset()

        env._step_return = (np.zeros(10, dtype=np.float32), -1.0, True, False, {})
        _, reward, _, _, info = wrapper.step(0)

        assert reward == -1.0  # No bounty on loss
        assert "bounty_applied" not in info

    def test_step_no_bounty_on_nonterminal(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)

        wrapper = GauntletWrapper(
            env, g, ["ST1-03"] * 50,
            bounty_threshold=0.0,
        )
        wrapper.reset()

        env._step_return = (np.zeros(10, dtype=np.float32), 0.05, False, False, {})
        _, reward, _, _, _ = wrapper.step(0)

        assert reward == 0.05  # No bounty on non-terminal

    def test_current_opponent_property(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)
        wrapper = GauntletWrapper(env, g, ["ST1-03"] * 50)

        assert wrapper.current_opponent is None  # Before reset
        wrapper.reset()
        assert wrapper.current_opponent is not None
        assert isinstance(wrapper.current_opponent, DeckEntry)

    def test_generalist_wrapper_injects_both_decks(self, tmp_path):
        env = self._make_mock_env()
        g = self._make_gauntlet(tmp_path)
        pool = g.as_generalist_pool()
        wrapper = GeneralistDeckPoolWrapper(env, pool, seed=42)

        captured_kwargs = {}
        original_reset = env.reset

        def spy_reset(**kwargs):
            captured_kwargs.update(kwargs)
            return original_reset(**kwargs)

        env.reset = spy_reset
        _obs, info = wrapper.reset()

        options = captured_kwargs["options"]
        assert len(options["deck1"]) > 0
        assert len(options["deck2"]) > 0
        assert info["deck1_archetype"] in pool.archetype_names
        assert info["opponent_archetype"] in pool.archetype_names
        assert info["deck1_deck_id"] == wrapper.current_deck1.deck_id
        assert info["opponent_deck_id"] == wrapper.current_deck2.deck_id


# ─── Override Meta Shares ─────────────────────────────────────────────


class TestOverrideMetaShares:
    """Tests for MetaGauntlet.override_meta_shares()."""

    def test_override_recomputes_ti(self, basic_library):
        """After override, TI reflects new meta_share values."""
        mg = MetaGauntlet(seed=42)
        mg.load(basic_library)

        old_ti = {n: s.threat_index for n, s in mg.archetypes.items()}

        # Invert the meta: Rogue becomes dominant
        mg.override_meta_shares({"Rogue": 0.60, "MetaKing": 0.05, "MidTier": 0.10})

        # Rogue should now have higher TI than MetaKing
        assert mg.archetypes["Rogue"].threat_index > mg.archetypes["MetaKing"].threat_index
        # MetaKing's TI should have dropped
        assert mg.archetypes["MetaKing"].threat_index < old_ti["MetaKing"]

    def test_override_zeroes_missing_archetypes(self, basic_library):
        """Archetypes not in overrides dict get meta_share = 0."""
        mg = MetaGauntlet(seed=42)
        mg.load(basic_library)

        mg.override_meta_shares({"MetaKing": 0.80})

        assert mg.archetypes["MidTier"].digilab_meta_share == 0.0
        assert mg.archetypes["Rogue"].digilab_meta_share == 0.0
        assert mg.archetypes["MetaKing"].digilab_meta_share == 0.80

    def test_override_rebuilds_sampling_weights(self, basic_library):
        """After override, sampling weights are recomputed and sum to 1."""
        mg = MetaGauntlet(seed=42)
        mg.load(basic_library)

        mg.override_meta_shares({"Rogue": 0.90, "MetaKing": 0.05, "MidTier": 0.05})

        assert mg._weights is not None
        assert abs(mg._weights.sum() - 1.0) < 1e-6
        assert len(mg._weights) == len(mg._deck_pool)

    def test_override_preserves_conversion_in_ti(self, basic_library):
        """Override changes meta_share but conversion_rate still contributes to TI."""
        mg = MetaGauntlet(seed=42)
        mg.load(basic_library)

        # Give Rogue high meta share; it also has high conversion (0.60)
        mg.override_meta_shares({"Rogue": 0.50, "MetaKing": 0.25, "MidTier": 0.25})

        rogue_ti = mg.archetypes["Rogue"].threat_index
        # TI = meta_share * alpha + conversion * beta = 0.50 * 1.0 + 0.60 * 2.0 = 1.70
        expected_ti = 0.50 * mg.alpha + 0.60 * mg.beta
        assert abs(rogue_ti - expected_ti) < 1e-6


# ─── allowed_archetypes filter ──────────────────────────────────────


class TestAllowedArchetypes:
    """Filter behavior for the declared `allowed_archetypes` scope."""

    def test_filters_pool_to_declared_subset(self, tmp_path):
        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=2),
            "Yellow Hybrid": _make_archetype("Yellow Hybrid", n_decks=2),
            "Other": _make_archetype("Other", n_decks=3),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes={"Rocks", "Yellow Hybrid"},
        )
        g.load(str(path))

        assert set(g.archetypes) == {"Rocks", "Yellow Hybrid"}
        assert {d.archetype_name for d in g._deck_pool} == {"Rocks", "Yellow Hybrid"}

    def test_alias_canonicalizes_to_library_entry(self, tmp_path):
        """An alias in allowed_archetypes resolves to the canonical library name."""
        # "RockClose" is a known alias for "Rocks" in data/archetype_aliases.json.
        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=1),
            "Other": _make_archetype("Other", n_decks=1),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes={"RockClose"},  # alias for "Rocks"
        )
        g.load(str(path))

        assert set(g.archetypes) == {"Rocks"}
        # Snapshot recording (downstream) should use the canonical name.
        assert g._deck_pool[0].archetype_name == "Rocks"

    def test_unrecognized_archetype_logs_warning_and_continues(self, tmp_path, caplog):
        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=1),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        with caplog.at_level("WARNING", logger="digimon_gym.agents.gauntlet"):
            g = MetaGauntlet(
                implemented_card_ids={"BT12-002", "BT12-022"},
                allowed_archetypes={"Rocks", "Definitely Not A Real Archetype"},
            )
            g.load(str(path))

        # Recognized entry produces a pool; unrecognized entry produces a warning
        # but does NOT cause silent fallback to the full pool.
        assert set(g.archetypes) == {"Rocks"}
        warnings = [r for r in caplog.records if r.levelname == "WARNING"]
        assert any(
            "Definitely Not A Real Archetype" in r.getMessage()
            for r in warnings
        ), f"expected warning naming the unrecognized entry; got {[r.getMessage() for r in warnings]}"

    @pytest.mark.parametrize(
        "bad_card, implemented, not_ready",
        [
            ("BT12-031", {"BT12-002", "BT12-022", "BT12-031"}, {"BT12-031"}),
            ("BT99-999", {"BT12-002", "BT12-022"}, set()),
        ],
        ids=["not-ready-card", "unregistered-card"],
    )
    def test_safety_floor_overrides_allowed(
        self, tmp_path, caplog, bad_card, implemented, not_ready,
    ):
        """An allowed archetype with no training-ready decklist is excluded and logged."""
        lib = _make_deck_library({
            "Ready": _make_archetype("Ready", n_decks=1),
            "NotReady": _make_archetype("NotReady", n_decks=1),
        })
        lib["archetypes"]["NotReady"]["decklists"][0]["decklist"] = _with_card(bad_card)
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        with caplog.at_level("INFO", logger="digimon_gym.agents.gauntlet"):
            g = MetaGauntlet(
                implemented_card_ids=implemented,
                not_ready_card_ids=not_ready,
                allowed_archetypes={"Ready", "NotReady"},
            )
            g.load(str(path))

        assert set(g.archetypes) == {"Ready"}
        # Spec requires logging the exclusion reason.
        info_msgs = [r.getMessage() for r in caplog.records if r.levelname == "INFO"]
        assert any(
            "NotReady" in m and "no training-ready decklist" in m for m in info_msgs
        ), f"expected info log naming dropped archetype; got {info_msgs}"

    def test_allowed_archetypes_empty_set_produces_empty_pool(self, tmp_path):
        """An explicit empty set scopes to nothing — not 'all'."""
        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=1),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes=set(),
        )
        g.load(str(path))

        assert g.archetype_count == 0
        assert g.deck_count == 0

    def test_load_generalist_deck_pool_forwards_filter(self, tmp_path, monkeypatch):
        """The generalist loader passes allowed_archetypes through to MetaGauntlet."""
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool

        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=2),
            "Yellow Hybrid": _make_archetype("Yellow Hybrid", n_decks=2),
            "Other": _make_archetype("Other", n_decks=3),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        pool = load_generalist_deck_pool(
            str(path),
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes={"Rocks"},
        )

        assert pool.archetype_names == ["Rocks"]
        assert pool.deck_count == 2

    def test_load_generalist_deck_pool_forwards_not_ready_cards(self, tmp_path):
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool

        lib = _make_deck_library({"Rocks": _make_archetype("Rocks", n_decks=2)})
        lib["archetypes"]["Rocks"]["decklists"][1]["decklist"] = _with_card("BT12-031")
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        pool = load_generalist_deck_pool(
            str(path),
            implemented_card_ids={"BT12-002", "BT12-022", "BT12-031"},
            not_ready_card_ids={"BT12-031"},
        )

        assert pool.deck_count == 1

    def test_snapshot_roundtrip_preserves_filtered_pool(self, tmp_path):
        """Write a filtered snapshot, mutate library, reload, expect identical pool."""
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool

        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=2),
            "Yellow Hybrid": _make_archetype("Yellow Hybrid", n_decks=2),
            "Other": _make_archetype("Other", n_decks=3),
        })
        lib_path = tmp_path / "lib.json"
        lib_path.write_text(json.dumps(lib))

        pool = load_generalist_deck_pool(
            str(lib_path),
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes={"Rocks", "Yellow Hybrid"},
        )

        snapshot_path = tmp_path / "snap.json"
        pool.write_snapshot(snapshot_path)

        # Mutate the library out from under us: add new archetypes, drop Rocks.
        lib_path.write_text(json.dumps(_make_deck_library({
            "Other": _make_archetype("Other", n_decks=3),
            "NewArchetype": _make_archetype("NewArchetype", n_decks=2),
        })))

        reloaded = GeneralistDeckPool.from_snapshot(
            snapshot_path,
            implemented_card_ids={"BT12-002", "BT12-022"},
        )

        assert set(reloaded.archetype_names) == {"Rocks", "Yellow Hybrid"}
        assert reloaded.deck_count == pool.deck_count
        # Deck IDs are content-addressed and must round-trip identically.
        for arch in reloaded.archetype_names:
            orig_ids = sorted(d.deck_id for d in pool.archetypes[arch])
            new_ids = sorted(d.deck_id for d in reloaded.archetypes[arch])
            assert orig_ids == new_ids

    def test_gauntlet_opponent_sampling_honors_filter(self, tmp_path):
        """In meta sampling mode, sampled opponents stay inside the filtered subset."""
        lib = _make_deck_library({
            "Rocks": _make_archetype("Rocks", n_decks=2),
            "Yellow Hybrid": _make_archetype("Yellow Hybrid", n_decks=2),
            "Other": _make_archetype("Other", n_decks=3),
        })
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))

        g = MetaGauntlet(
            seed=42,
            sampling_mode="meta",
            implemented_card_ids={"BT12-002", "BT12-022"},
            allowed_archetypes={"Rocks", "Yellow Hybrid"},
        )
        g.load(str(path))

        sampled = g.sample_opponents(200)
        sampled_archetypes = {entry.archetype_name for entry in sampled}
        assert sampled_archetypes <= {"Rocks", "Yellow Hybrid"}
        # Verify both members are reachable (not stuck on one).
        assert sampled_archetypes == {"Rocks", "Yellow Hybrid"}


# ─── Oracle readiness gate (spec 2026-10-04 §4.7) ──────────────────────

@pytest.mark.oracle_gate
class TestOracleReadinessGate:
    REGISTERED = {"BT12-002", "BT12-022", "BT12-031"}

    def _lib(self, tmp_path):
        lib = _make_deck_library({"Mixed": _make_archetype("Mixed", n_decks=2)})
        lib["archetypes"]["Mixed"]["decklists"][1]["decklist"] = _with_card("BT12-031")
        path = tmp_path / "lib.json"
        path.write_text(json.dumps(lib))
        return path

    def test_list_with_a_not_ready_card_is_rejected(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids={"BT12-002", "BT12-022"},
        )
        g.load(str(self._lib(tmp_path)))
        assert g.deck_count == 1
        assert g._deck_pool[0].source_deck_id == "mixed_000"

    def test_gate_applies_to_a_non_default_library_path(self, tmp_path, monkeypatch):
        # The ledger gate only auto-loads for the default path; the oracle gate
        # must not inherit that hole. Nothing is injected here: the loader runs.
        readiness = tmp_path / "oracle_readiness.json"
        readiness.write_text(json.dumps({"version": 1, "cards": {
            "BT12-002": {"status": "ready"}, "BT12-022": {"status": "ready"},
            "BT12-031": {"status": "not_ready"},
        }}))
        monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", readiness)
        g = MetaGauntlet(implemented_card_ids=self.REGISTERED, not_ready_card_ids=set())
        g.load(str(self._lib(tmp_path)))
        assert g.deck_count == 1

    def test_missing_artifact_fails_at_load(self, tmp_path, monkeypatch):
        monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", tmp_path / "absent.json")
        g = MetaGauntlet(implemented_card_ids=self.REGISTERED, not_ready_card_ids=set())
        with pytest.raises(gauntlet_module.OracleReadinessMissingError) as exc:
            g.load(str(self._lib(tmp_path)))
        message = str(exc.value)
        # A cloud box has data/ but not qa/: say how to get the artifact there.
        assert "copy it from the repo" in message
        assert "qa/qa-reports/exam-verdicts" in message
        assert "tools.clause_coverage.readiness" in message

    def test_empty_pool_fails_fast_naming_the_blockers(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids={"BT12-002"},
        )
        with pytest.raises(gauntlet_module.EmptyTrainingPoolError) as exc:
            g.load(str(self._lib(tmp_path)))
        message = str(exc.value)
        assert "BT12-022" in message
        assert "BT12-022 ExVeemon (2)" in message  # card, name, decklists blocked
        assert "--plan" in message

    def test_allowed_archetypes_empty_set_still_produces_an_empty_pool_quietly(self, tmp_path):
        # No decklist reached the oracle gate, so this is not an oracle failure.
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids=set(),
            allowed_archetypes=set(),
        )
        g.load(str(self._lib(tmp_path)))
        assert g.deck_count == 0

    def test_generalist_pool_loader_passes_the_ready_set_through(self, tmp_path):
        pool = gauntlet_module.load_generalist_deck_pool(
            str(self._lib(tmp_path)),
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids={"BT12-002", "BT12-022"},
        )
        assert pool.deck_count == 1

    def test_starter_curriculum_archetypes_are_gated(self):
        # ST-1..6 live in the real deck library; with nothing oracle-ready the
        # starter archetype fails fast instead of training on nothing.
        with pytest.raises(gauntlet_module.EmptyTrainingPoolError):
            gauntlet_module.load_generalist_deck_pool(
                allowed_archetypes={"ST-1 Gaia Red"},
                oracle_ready_card_ids=set(),
            )

    def _snapshot(self, tmp_path):
        g = MetaGauntlet(
            implemented_card_ids=self.REGISTERED,
            not_ready_card_ids=set(),
            oracle_ready_card_ids=set(self.REGISTERED),
        )
        g.load(str(self._lib(tmp_path)))
        snapshot = tmp_path / "pool.json"
        written = g.as_generalist_pool().write_snapshot(snapshot)
        return snapshot, written

    def test_snapshot_with_a_not_ready_card_is_refused(self, tmp_path):
        snapshot, _ = self._snapshot(tmp_path)
        with pytest.raises(gauntlet_module.NotOracleReadyDeckError, match="BT12-031"):
            GeneralistDeckPool.from_snapshot(
                snapshot,
                implemented_card_ids=self.REGISTERED,
                oracle_ready_card_ids={"BT12-002", "BT12-022"},
            )

    def test_snapshot_of_ready_decks_still_loads(self, tmp_path):
        snapshot, written = self._snapshot(tmp_path)
        loaded = GeneralistDeckPool.from_snapshot(
            snapshot, implemented_card_ids=self.REGISTERED,
            oracle_ready_card_ids=set(self.REGISTERED),
        )
        assert loaded.snapshot_hash == written

    def test_run_training_job_library_deck_is_gated(self, tmp_path, monkeypatch):
        # `agent_deck: {source: deck_id}` reads a deck straight from the library;
        # it must pass the same gate as the pool (explicit file decks are the
        # operator's own choice and stay ungated).
        import data_paths
        from tools import run_training_job

        readiness = tmp_path / "oracle_readiness.json"
        readiness.write_text(json.dumps({"version": 1, "cards": {
            "BT12-002": {"status": "ready"}, "BT12-022": {"status": "ready"},
            "BT12-031": {"status": "not_ready"},
        }}))
        monkeypatch.setattr(data_paths, "DECK_LIBRARY", self._lib(tmp_path))
        monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", readiness)
        deck = run_training_job.load_deck({"source": "deck_id", "deck_id": "mixed_000"})
        assert len(deck) == 55
        with pytest.raises(gauntlet_module.NotOracleReadyDeckError, match="mixed_001.*BT12-031"):
            run_training_job.load_deck({"source": "deck_id", "deck_id": "mixed_001"})

    def test_snapshot_load_reads_the_artifact_when_nothing_is_injected(self, tmp_path, monkeypatch):
        # A snapshot taken before the gate must not smuggle decks back in, even
        # through callers (eval CLIs, --curriculum-pool) that inject nothing.
        snapshot, _ = self._snapshot(tmp_path)
        monkeypatch.setattr(gauntlet_module, "_ORACLE_READINESS_PATH", tmp_path / "absent.json")
        with pytest.raises(gauntlet_module.OracleReadinessMissingError):
            GeneralistDeckPool.from_snapshot(snapshot, implemented_card_ids=self.REGISTERED)
