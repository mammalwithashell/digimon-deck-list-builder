"""TEST-022 — synthetic pilot card for RUST_PYTHON_PARITY §2.5.

Exercises the [Security] memory-gain path (distinct from TEST-020 so the
parity harness can distinguish the two pilots from their side effects).
Mirrors the Rust `TEST-022` pilot in `digimon-engine/src/cards/test_cards.rs`.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class TEST_022(CardScript):
    """TEST-022: [Security] Gain 3 memory."""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effect = ICardEffect()
        effect.set_timing(EffectTiming.SecuritySkill)
        effect.set_effect_name("TEST-022 Security: Gain 3 memory")
        effect.set_effect_description("[Security] Gain 3 memory.")
        effect.is_security_effect = True

        def process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player is None:
                return
            # Mirrors the convention used by existing `[Security] Gain N
            # memory` cards (e.g. ST2-13) — `Player.add_memory` applies
            # the signed seesaw in the defender's favor.
            player.add_memory(3)

        effect.set_on_process_callback(process)
        return [effect]
