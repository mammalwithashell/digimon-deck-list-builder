from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_112(CardScript):
    """BT10-112 Jesmon GX | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-112 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "Royal Knight"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT10-112 Place 1 card to digivolution cards to activate effects and Blitz")
        effect1.set_effect_description("[When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Place 1 Royal Knight from hand/trash under self, activate WhenDigivolving, Blitz."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            def hand_filter(c):
                if not any('Royal Knight' in tr for tr in (getattr(c, 'card_traits', []) or [])):
                    return False
                play_cost = getattr(c, 'get_cost_itself', 0)
                if callable(play_cost):
                    play_cost = play_cost
                else:
                    play_cost = getattr(c, 'play_cost', 0) or 0
                pc = getattr(c, 'play_cost', 0) or 0
                return pc <= 13

            def on_card_selected(selected_card):
                if selected_card is None:
                    return
                if selected_card in player.hand_cards:
                    player.hand_cards.remove(selected_card)
                elif selected_card in player.trash_cards:
                    player.trash_cards.remove(selected_card)
                perm.add_card_source_bottom(selected_card)
                # Grant Blitz (Rush on digivolve turn)
                perm.grant_keyword('_is_rush')

            game.effect_select_hand_card(
                player, hand_filter, on_card_selected,
                is_optional=True,
                prompt="Select 1 [Royal Knight] card (cost 13 or less) from hand to place under this Digimon.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-112 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect3 = ICardEffect()
        effect3.set_effect_name("BT10-112 Security Attack +1")
        effect3.set_effect_description("Security Attack +1")
        effect3._security_attack_modifier = 1

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
