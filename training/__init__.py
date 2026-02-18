"""Meta Gauntlet training pipeline for Digimon TCG RL agents.

Provides meta-weighted opponent selection, deck library management,
deck variant generation, and reward shaping for Pilot Agent training.
"""

from training.gauntlet import (
    MetaDistribution,
    DeckLibrary,
    VariantGenerator,
    MetaGauntlet,
    MetaGauntletEnv,
    calculate_reward_scale,
    parse_deck_any,
    parse_egman_text,
    parse_egman_url,
    parse_export_code,
)

__all__ = [
    "MetaDistribution",
    "DeckLibrary",
    "VariantGenerator",
    "MetaGauntlet",
    "MetaGauntletEnv",
    "calculate_reward_scale",
    "parse_deck_any",
    "parse_egman_text",
    "parse_egman_url",
    "parse_export_code",
]
