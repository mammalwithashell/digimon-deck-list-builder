from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_044(CardScript):
    """BT14-044 Palmon | Lv.3

    [Start of Your Main Phase] 1 of your opponent's Digimon gains
    "[All Turns] When this Digimon becomes suspended, lose 2 memory."
    until the end of their turn.

    [Inherited] [Your Turn] [Once Per Turn] When this Digimon would digivolve,
    if you have a green Tamer, reduce the cost by 1.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: [Start of Your Main Phase] Grant suspend penalty to opponent's Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name(
            "BT14-044 Opponent's Digimon gains 'suspend -> lose 2 memory' until end of their turn"
        )
        effect0.set_effect_description(
            "[Start of Your Main Phase] 1 of your opponent's Digimon gains "
            "[All Turns] When this Digimon becomes suspended, lose 2 memory. "
            "until the end of their turn."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            source_perm = card.permanent_of_this_card() if card else None

            def target_filter(p):
                return p.is_digimon

            def on_target(target_perm):
                # Create a temporary OnTappedAnyone effect: "lose 2 memory"
                granted = ICardEffect()
                granted.set_timing(EffectTiming.OnTappedAnyone)
                granted.set_effect_name(
                    "BT14-044 Granted: When suspended, lose 2 memory"
                )
                granted._source_permanent = source_perm

                def granted_condition(context: Dict[str, Any]) -> bool:
                    perm_ctx = context.get('permanent')
                    return perm_ctx is target_perm

                granted.set_can_use_condition(granted_condition)

                def granted_process(ctx2: Dict[str, Any]):
                    # The opponent of the player who granted this effect loses memory
                    # (i.e., the owner of the targeted Digimon loses 2 memory)
                    owner = target_perm.top_card.owner if target_perm.top_card else None
                    if owner:
                        owner.lose_memory(2)
                        if game:
                            game.logger.log(
                                f"[BT14-044] {target_perm.top_card.card_names[0] if target_perm.top_card.card_names else 'Unknown'} "
                                f"suspended — {owner.player_name} loses 2 memory."
                            )

                granted.set_on_process_callback(granted_process)

                # Grant until end of opponent's next turn
                # expiry_turn = current turn + 1 (expires at start of our next turn)
                target_perm.grant_temp_effect(granted, expiry_turn=game.turn_count + 1)

            game.effect_select_opponent_permanent(
                player, on_target,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to gain the suspend penalty."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1 (Inherited): [Your Turn] [Once Per Turn] When this Digimon would digivolve,
        # if you have a green Tamer, reduce the cost by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenWouldDigivolve)
        effect1.set_effect_name(
            "BT14-044 Inherited: When digivolving, if green Tamer, cost -1"
        )
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon would digivolve, "
            "if you have a green Tamer, reduce the cost by 1."
        )
        effect1.is_inherited_effect = True
        effect1.cost_reduction = 1

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a green Tamer on field
            player_owner = card.owner if card else None
            if player_owner is None:
                return False
            from ....data.enums import CardColor
            has_green_tamer = any(
                p.is_tamer and CardColor.Green in getattr(
                    p.top_card, 'card_colors', []
                )
                for p in player_owner.battle_area
            )
            return has_green_tamer

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -1 — handled via cost_reduction property"""
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
