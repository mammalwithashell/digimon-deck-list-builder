from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_061(CardScript):
    """EX10-061 Apocalymon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-061 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, by placing 1 of each face-up [Dark Masters] trait card with different names from your security stack under this card, reduce the play cost by 4 for each card placed.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-061 Placing 1 [Dark Masters] to get Play Cost -4")
        effect1.set_effect_description("When this card would be played, by placing 1 of each face-up [Dark Masters] trait card with different names from your security stack under this card, reduce the play cost by 4 for each card placed.")
        effect1.set_hash_string("PlayCost-_EX10-061")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-061 Play Cost -4")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs. Then, all of your [Dark Masters] trait Digimon gain <Rush> for the turn. At turn end, delete the Digimon this effect played.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-061 Play 1 [Dark Masters] from sources, give <Rush>, Delete them at end of turn")
        effect3.set_effect_description("[On Play] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs. Then, all of your [Dark Masters] trait Digimon gain <Rush> for the turn. At turn end, delete the Digimon this effect played.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Dark Masters' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [End of Your Turn] Delete this Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-061 Delete")
        effect4.set_effect_description("[End of Your Turn] Delete this Digimon.")
        effect4.is_on_play = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs. Then, all of your [Dark Masters] trait Digimon gain <Rush> for the turn. (This Digimon can attack the turn it comes into play.) At turn end, delete the Digimon this effect played.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX10-061 Play 1 [Dark Masters] from sources, give <Rush>, Delete them at end of turn")
        effect5.set_effect_description("[When Digivolving] You may play 1 of each [Dark Masters] trait card with different names from this Digimon's digivolution cards without paying the costs. Then, all of your [Dark Masters] trait Digimon gain <Rush> for the turn. (This Digimon can attack the turn it comes into play.) At turn end, delete the Digimon this effect played.")
        effect5.is_when_digivolving = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Dark Masters' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [End of Your Turn] Delete this Digimon.
        effect6 = ICardEffect()
        effect6.set_effect_name("EX10-061 Delete the Digimon")
        effect6.set_effect_description("[End of Your Turn] Delete this Digimon.")
        effect6.is_on_play = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
