from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_058(CardScript):
    """BT24-058 Blimpmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 with [TS] trait for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-058 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Shared process for On Play / When Digivolving:
        # Reveal top 3 of deck. Among them, add 1 [Machine], [Cyborg] or [TS]
        # trait Digimon or Tamer to hand OR place as bottom digivolution card of
        # any of your [Machine], [Cyborg] or [TS] trait Digimon. Return the rest
        # to the bottom of the deck.
        def _card_qualifies(c) -> bool:
            if not (getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)):
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any(t in ('Machine', 'Cyborg', 'TS') for t in traits)

        def _perm_qualifies(p, owner) -> bool:
            if not getattr(p, 'is_digimon', False):
                return False
            if getattr(p, 'owner', None) != owner:
                return False
            top = getattr(p, 'top_card', None)
            if top is None:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any(t in ('Machine', 'Cyborg', 'TS') for t in traits)

        def make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Reveal top 3 cards; on_selected provides (selected, remaining)
                def reveal_filter(c):
                    return _card_qualifies(c)

                def on_revealed(selected, remaining):
                    # Put remaining at deck bottom
                    for c in remaining:
                        player.library_cards.append(c)
                    if selected is None:
                        return
                    # Player chooses: add to hand or place as bottom divi card
                    def on_choice(choice_index):
                        if choice_index == 0:
                            # Add to hand
                            player.hand_cards.append(selected)
                        else:
                            # Place as bottom digivolution card of a qualifying Digimon
                            def divi_filter(p):
                                return _perm_qualifies(p, player)

                            def on_divi_selected(target_perm):
                                if target_perm and hasattr(target_perm, 'digivolution_cards'):
                                    target_perm.digivolution_cards.append(selected)

                            game.effect_select_own_permanent(
                                player, on_divi_selected,
                                filter_fn=divi_filter,
                                is_optional=False
                            )

                    game.effect_choose_branch(
                        player, 2, on_choice,
                        branch_labels=["Add to hand", "Add as digivolution card"]
                    )

                game.effect_reveal_and_select(
                    player, 3, reveal_filter, on_revealed, is_optional=False
                )

            return process

        # Timing: EffectTiming.OnEnterFieldAnyone — On Play
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-058 Reveal 3, add or place under sources 1, return rest to deck")
        effect1.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Among them, add 1 [Machine], [Cyborg] or [TS] trait Digimon card or Tamer card to the hand or place it as any of your [Machine], [Cyborg] or [TS] trait Digimon's bottom digivolution card. Return the rest to the bottom of the deck.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(make_shared_process())
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone — When Digivolving
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-058 Reveal 3, add or place under sources 1, return rest to deck")
        effect2.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. Among them, add 1 [Machine], [Cyborg] or [TS] trait Digimon card or Tamer card to the hand or place it as any of your [Machine], [Cyborg] or [TS] trait Digimon's bottom digivolution card. Return the rest to the bottom of the deck.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(make_shared_process())
        effects.append(effect2)

        # Factory effect: reboot (inherited / ESS)
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-058 Reboot")
        effect3.set_effect_description("Reboot")
        effect3.is_inherited_effect = True
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
