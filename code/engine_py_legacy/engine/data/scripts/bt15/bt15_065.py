from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_065(CardScript):
    """BT15-065 WaruMonzaemon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-065 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_name = "Numemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By placing 1 card with [Numemon] in it's name from your trash as this Digimon's bottom digivolution card, all of your opponent's Digimon with a play cost of 5 or less can't attack players until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT15-065 Place a card under this Digimon to gain effects.")
        effect1.set_effect_description("[On Play] By placing 1 card with [Numemon] in it's name from your trash as this Digimon's bottom digivolution card, all of your opponent's Digimon with a play cost of 5 or less can't attack players until the end of your opponent's turn.")
        effect1.is_optional = True
        effect1.set_hash_string("CantAttack_BT15_065")
        effect1.is_on_play = True
        effect1._is_cannot_attack_player = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Restrict Attack, Gain Keyword Cannot Attack Player, Grant Cannot Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Attack restriction via modifier system
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def target_filter(p):
                return p.is_digimon
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=target_filter, is_optional=True)
            if perm:
                perm.grant_keyword('_is_cannot_attack_player')
            # Prevent target from attacking
            if not (player and game):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            def on_restrict(target_perm):
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target_perm,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_opponent_permanent(
                player, on_restrict, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By trashing 1 card with [Numemon] in it's name in your hand or from the bottom of this Digimon's digivolution cards, [De-Digivolve 1] 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT15-065 Trash 1 card from hand to DeDigivolve an opponent's Digimon.")
        effect2.set_effect_description("[On Play] By trashing 1 card with [Numemon] in it's name in your hand or from the bottom of this Digimon's digivolution cards, [De-Digivolve 1] 1 of your opponent's Digimon.")
        effect2.is_optional = True
        effect2.set_hash_string("DeDigivolve_BT15_065")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Trash From Hand, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Numemon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By trashing 1 card with [Numemon] in it's name in your hand or from the bottom of this Digimon's digivolution cards, [De-Digivolve 1] 1 of your opponent's Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT15-065 Trash 1 card from hand to DeDigivolve an opponent's Digimon.")
        effect3.set_effect_description("[When Digivolving] By trashing 1 card with [Numemon] in it's name in your hand or from the bottom of this Digimon's digivolution cards, [De-Digivolve 1] 1 of your opponent's Digimon.")
        effect3.is_optional = True
        effect3.set_hash_string("DeDigivolve_BT15_065")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Numemon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect4 = ICardEffect()
        effect4.set_effect_name("BT15-065 Security Attack +1")
        effect4.set_effect_description("Security Attack +1")
        effect4.is_inherited_effect = True
        effect4._security_attack_modifier = 1

        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
