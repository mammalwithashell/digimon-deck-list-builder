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

from digimon_gym.agents.gauntlet import (
    ArchetypeStats,
    DeckEntry,
    GauntletWrapper,
    MetaGauntlet,
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
        card_ids = [f"BT24-{i:03d}"] * 46 + ["ST1-03"] * 4 + ["ST1-01"] * 5
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
