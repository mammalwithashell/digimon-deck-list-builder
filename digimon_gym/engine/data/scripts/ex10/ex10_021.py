from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_021(CardScript):
    """EX10-021 Belphemon: Sleep Mode | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-021 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Belphemon: Rage Mode] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Belphemon: Rage Mode"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Belphemon: Rage Mode'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 [Belphemon: Rage Mode] from your trash as this Digimon's top digivolution card, until the end of your opponent's turn, this Digimon can't attack and isn't affected by your opponent's effects.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-021 Place 1 card from trash to digivolution cards so that this Digimon gets effects")
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
        effect2.set_effect_name("EX10-021 Place 1 card from trash to digivolution cards so that this Digimon gets effects")
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

        # Timing: EffectTiming.OnTappedAnyone
        # [Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon suspend, by trashing 2 cards in your hand, suspend 2 of their Digimon or Tamers.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-021 Trash 2, Suspend 2")
        effect3.set_effect_description("[Opponent's Turn] [Once Per Turn] When any of your opponent's Digimon suspend, by trashing 2 cards in your hand, suspend 2 of their Digimon or Tamers.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("AT_EX10-021")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Suspend"""
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
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
