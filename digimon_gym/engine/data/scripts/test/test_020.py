"""TEST-020 — synthetic pilot card for RUST_PYTHON_PARITY §2.5.

Exercises the [Security] trigger-and-trash path: a revealed security card
fires an effect, then trashes after the check. Mirrors the Rust
`TEST-020` pilot in `digimon-engine/src/cards/test_cards.rs`.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class TEST_020(CardScript):
    """TEST-020: [Security] Draw 2 cards."""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_timing(EffectTiming.SecuritySkill)
        effect.set_effect_name("TEST-020 Security: Draw 2")
        effect.set_effect_description("[Security] Draw 2 cards.")
        effect.is_security_effect = True

        def process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player is None:
                return
            player.draw_cards(2)

        effect.set_on_process_callback(process)
        return [effect]
