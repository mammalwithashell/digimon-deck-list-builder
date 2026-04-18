"""TEST-021 — synthetic pilot card for RUST_PYTHON_PARITY §2.5.

Exercises the [Security] play-from-security path: a revealed security card
is put on the defender's field instead of being trashed. Mirrors the Rust
`TEST-021` pilot in `digimon-engine/src/cards/test_cards.rs`.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class TEST_021(CardScript):
    """TEST-021: [Security] Play this card without paying its cost."""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_timing(EffectTiming.SecuritySkill)
        effect.set_effect_name("TEST-021 Security: Play free")
        effect.set_effect_description(
            "[Security] Play this card without paying its cost."
        )
        effect.is_security_effect = True

        def process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            security_card = ctx.get('card')
            if not (player and game and security_card):
                return
            game.effect_play_from_security(player, security_card)

        effect.set_on_process_callback(process)
        return [effect]
