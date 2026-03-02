from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_063(CardScript):
    """BT5-063 Kurisarimon | Lv.4 Champion | Black | Unidentified

    [When Digivolving] If you don't have an [Arata Sanada] card in play,
        you may play one from your hand without paying its memory cost.
    Inherited: [Your Turn] All of your other Digimon with the same name
        as this Digimon gain <Rush>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Play Arata Sanada from hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT5-063 When Digivolving: Play Arata Sanada from hand")
        effect0.set_effect_description(
            "[When Digivolving] If you don't have an [Arata Sanada] card in "
            "play, you may play one from your hand without paying its memory cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner):
                return False
            player = card.owner
            # Check that no Arata Sanada is in play
            for perm in player.battle_area:
                if perm.top_card and perm.top_card.c_entity_base:
                    if 'Arata Sanada' in (perm.top_card.c_entity_base.card_name_eng or ''):
                        return False
            # Check there's an Arata Sanada in hand
            for hc in player.hand_cards:
                if hc.c_entity_base and 'Arata Sanada' in (hc.c_entity_base.card_name_eng or ''):
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play Arata Sanada from hand without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Find Arata Sanada in hand
            for hc in player.hand_cards:
                if hc.c_entity_base and 'Arata Sanada' in (hc.c_entity_base.card_name_eng or ''):
                    player.hand_cards.remove(hc)
                    player.play_card_from_source(hc, pay_cost=False)
                    break
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [Your Turn] Same-name Digimon gain Rush ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT5-063 Inherited: Same-name Digimon gain Rush")
        effect1.set_effect_description(
            "[Your Turn] All of your other Digimon with the same name as this "
            "Digimon gain <Rush>."
        )
        effect1.is_inherited_effect = True
        effect1._is_rush = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
