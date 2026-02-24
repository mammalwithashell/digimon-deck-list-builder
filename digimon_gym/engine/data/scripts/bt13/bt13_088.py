from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_088(CardScript):
    """BT13-088 Belphemon: Sleep Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-088 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Belphemon: Rage Mode] from your trash as this Digimon's top digivolution card, until the end of your opponent's turn, this Digimon can't attack and isn't affected by your opponent's effects.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-088 Place 1 card from trash to digivolution cards so that this Digimon gets effects")
        effect1.set_effect_description("[On Play] By placing 1 [Belphemon: Rage Mode] from your trash as this Digimon's top digivolution card, until the end of your opponent's turn, this Digimon can't attack and isn't affected by your opponent's effects.")
        effect1.is_optional = True
        effect1.is_on_play = True
        effect1._is_cannot_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
            # Grant effect immunity (CanNotAffectedClass) — not yet in engine
            pass  # descriptive-tagged: effect_immunity

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 1 [Belphemon: Rage Mode] from your trash as this Digimon's top digivolution card, until the end of your opponent's turn, this Digimon can't attack and isn't affected by your opponent's effects.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-088 Place 1 card from trash to digivolution cards so that this Digimon gets effects")
        effect2.set_effect_description("[When Digivolving] By placing 1 [Belphemon: Rage Mode] from your trash as this Digimon's top digivolution card, until the end of your opponent's turn, this Digimon can't attack and isn't affected by your opponent's effects.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True
        effect2._is_cannot_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_cannot_attack')
            # Grant effect immunity (CanNotAffectedClass) — not yet in engine
            pass  # descriptive-tagged: effect_immunity

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by trashing 2 cards in your hand, end the attack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-088 Trash 2 cards from hand to end the attack")
        effect3.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by trashing 2 cards in your hand, end the attack.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Discard_BT13_088")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
