from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_021(CardScript):
    """BT10-021 MailBirdramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have a [Kiriha Aonuma] in play, you may return 1 [MetalGreymon] from your trash to your hand. If you don't have a [Kiriha Aonuma] in play, you may play 1 [Kiriha Aonuma] from your hand without paying its memory cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT10-021 Return a [MetalGreymon] from trash to hand or play [Kiriha Aonuma] from hand")
        effect0.set_effect_description("[On Play] If you have a [Kiriha Aonuma] in play, you may return 1 [MetalGreymon] from your trash to your hand. If you don't have a [Kiriha Aonuma] in play, you may play 1 [Kiriha Aonuma] from your hand without paying its memory cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: save
        # Save
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-021 Save")
        effect1.set_effect_description("Save")
        effect1.is_on_deletion = True
        effect1._is_save = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once Per Turn] If this Digimon has [Blue Flare] in its traits and your opponent has 2 or more Digimon in play, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT10-021 Opponent's 1 Digimon can't attack and block")
        effect2.set_effect_description("[When Attacking][Once Per Turn] If this Digimon has [Blue Flare] in its traits and your opponent has 2 or more Digimon in play, 1 of your opponent's Digimon can't attack or block until the end of your opponent's turn.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Can'tAttackBlock_BT10_021")
        effect2.is_on_attack = True
        effect2._is_cannot_attack = True
        effect2._is_cannot_block = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Gain Keyword Cannot Block, Grant Cannot Block"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
                perm.grant_keyword('_is_cannot_block')
            # Prevent target from blocking
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_BLOCK, target_perm,
                    value_fn=lambda: True, expiry='end_of_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
