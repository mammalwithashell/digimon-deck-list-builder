from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_022(CardScript):
    """BT24-022 Ikkakumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 with [TS] trait for cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-022 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-022 Jamming")
        effect1.set_effect_description("Jamming")
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for On Play / When Digivolving:
        # Trash the top 2 digivolution cards of 1 of your opponent's Digimon.
        # Then, 1 of their Digimon with as many or fewer digivolution cards as
        # this Digimon can't suspend until their turn ends.
        def make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and perm and game):
                    return

                my_source_count = len(perm.digivolution_cards) if hasattr(perm, 'digivolution_cards') else 0

                # Step 1: select opponent Digimon to trash 2 divi cards from
                def strip_filter(p):
                    return (getattr(p, 'is_digimon', False) and
                            p.owner == player.enemy)

                def on_strip_selected(target_perm):
                    if target_perm and hasattr(target_perm, 'digivolution_cards'):
                        trashed = []
                        for _ in range(2):
                            if target_perm.digivolution_cards:
                                trashed.append(target_perm.digivolution_cards.pop(0))
                        if trashed:
                            player.enemy.trash_cards.extend(trashed)

                game.effect_select_opponent_permanent(
                    player, on_strip_selected,
                    filter_fn=strip_filter,
                    is_optional=False
                )

                # Step 2: select opponent Digimon with <= my source count to prevent suspend
                def stun_filter(p):
                    if not (getattr(p, 'is_digimon', False) and p.owner == player.enemy):
                        return False
                    opp_count = len(p.digivolution_cards) if hasattr(p, 'digivolution_cards') else 0
                    return opp_count <= my_source_count

                def on_stun_selected(target_perm):
                    if target_perm and game:
                        from digimon_gym.engine.interfaces.modifiers import ModifierType
                        game.register_modifier(
                            ModifierType.CANNOT_SUSPEND, target_perm,
                            value_fn=lambda: True, expiry='end_of_opponent_turn'
                        )

                game.effect_select_opponent_permanent(
                    player, on_stun_selected,
                    filter_fn=stun_filter,
                    is_optional=False
                )

            return process

        # Timing: EffectTiming.OnEnterFieldAnyone — On Play
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-022 Trash 2 divi cards, prevent suspend")
        effect2.set_effect_description("[On Play] Trash the top 2 digivolution cards of 1 of your opponent's Digimon. Then, 1 of their Digimon with as many or fewer digivolution cards as this Digimon can't suspend until their turn ends.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(make_shared_process())
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone — When Digivolving
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-022 Trash 2 divi cards, prevent suspend")
        effect3.set_effect_description("[When Digivolving] Trash the top 2 digivolution cards of 1 of your opponent's Digimon. Then, 1 of their Digimon with as many or fewer digivolution cards as this Digimon can't suspend until their turn ends.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(make_shared_process())
        effects.append(effect3)

        # Timing: EffectTiming.OnUnTappedAnyone — ESS (inherited)
        # [Your Turn] [Once Per Turn] When this Digimon unsuspends, if you have
        # 7 or fewer cards in your hand, <Draw 1>.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUnTappedAnyone)
        effect4.set_effect_name("BT24-022 If you have 7 or fewer cards in hand, <Draw 1>.")
        effect4.set_effect_description("[Your Turn] [Once Per Turn] When this Digimon unsuspends, if you have 7 or fewer cards in your hand, <Draw 1>.")
        effect4.is_inherited_effect = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_022_YT_Draw1")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be triggered by THIS Digimon unsuspending
            triggering_perm = context.get('permanent') if context else None
            my_perm = card.permanent_of_this_card() if card else None
            if triggering_perm is not None and my_perm is not None:
                if triggering_perm != my_perm:
                    return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Draw 1 if hand <= 7"""
            player = ctx.get('player')
            if player:
                hand_size = len(player.hand_cards) if hasattr(player, 'hand_cards') else 0
                if hand_size <= 7:
                    player.draw_cards(1)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
