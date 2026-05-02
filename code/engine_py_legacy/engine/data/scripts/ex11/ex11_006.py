from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_006(CardScript):
    """EX11-006 Flickmon | Lv.2

    --- Inherited ---
    [When Attacking] [Once Per Turn] This Digimon linked with [Maquinamon]
    may digivolve into a Digimon card with [Maquinamon] in its text in the
    hand with the digivolution cost reduced by 2.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Inherited: [When Attacking] [Once Per Turn]
        # This Digimon linked with [Maquinamon] may digivolve into a Digimon
        # card with [Maquinamon] in its text from hand, cost reduced by 2.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnUseAttack)
        effect0.set_effect_name("EX11-006 Digivolve this Digimon into a Digimon with [Maquinamon] in text.")
        effect0.set_effect_description(
            "[When Attacking] [Once Per Turn] This Digimon linked with "
            "[Maquinamon] may digivolve into a Digimon card with [Maquinamon] "
            "in its text in the hand with the digivolution cost reduced by 2.")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Digivolve_EX11_006")
        effect0.is_on_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            if not my_perm:
                return False
            # Must be linked with [Maquinamon]
            linked = getattr(my_perm, 'linked_cards', [])
            has_maquinamon_link = False
            for lc in linked:
                names = getattr(lc, 'card_names', []) or []
                if any('Maquinamon' in n for n in names):
                    has_maquinamon_link = True
                    break
            if not has_maquinamon_link:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Digivolve into a Digimon with [Maquinamon] in text, cost reduced by 2."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                text = getattr(c, 'card_text', '') or ''
                return getattr(c, 'is_digimon', False) and 'Maquinamon' in text

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True,
                cost_reduction=2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
