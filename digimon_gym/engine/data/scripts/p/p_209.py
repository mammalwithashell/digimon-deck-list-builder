from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_209(CardScript):
    """P-209 Titamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("P-209 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 with [Demon] trait for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Demon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Demon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("P-209 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card in your hand, suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-209 By trashing 1 card from your hand, suspend 1 digimon or tamer, then 1 digimon or tamer cant unsuspend")
        effect2.set_effect_description("[When Digivolving] By trashing 1 card in your hand, suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends.")
        effect2.is_on_play = True
        effect2._is_cannot_unsuspend = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Suspend, Gain Keyword Cannot Unsuspend, Grant Cannot Unsuspend"""
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
                player, hand_filter, on_trashed, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')
            # Prevent target from unsuspending
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card in your hand, suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("P-209 By trashing 1 card from your hand, suspend 1 digimon or tamer, then 1 digimon or tamer cant unsuspend")
        effect3.set_effect_description("[When Digivolving] By trashing 1 card in your hand, suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends.")
        effect3.is_when_digivolving = True
        effect3._is_cannot_unsuspend = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Suspend, Gain Keyword Cannot Unsuspend, Grant Cannot Unsuspend"""
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
                player, hand_filter, on_trashed, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            if perm:
                perm.grant_keyword('_is_cannot_unsuspend')
            # Prevent target from unsuspending
            if not (player and game):
                return
            from digimon_gym.engine.interfaces.modifiers import ModifierType
            def on_freeze(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_UNSUSPEND, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_freeze, filter_fn=lambda p: p.is_suspended, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDiscardHand
        # [All Turns] [Once Per Turn] When your hand is trashed from, you may play 1 level 4 or lower [Demon] or [Titan] trait card from your trash without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("P-209 Play 1 level 4 or lower [Demon] or [Titan] card")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When your hand is trashed from, you may play 1 level 4 or lower [Demon] or [Titan] trait card from your trash without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("P_209_AT")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not (any('Demon' in _t or 'Titan' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
