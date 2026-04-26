from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_073(CardScript):
    """EX5-073 GraceNovamon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-073 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-073 Security Attack +1")
        effect1.set_effect_description("Security Attack +1")
        effect1._security_attack_modifier = 1

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-073 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [When Attacking] If DNA digivolving, trash any 8 digivolution cards from your opponent's Digimon. Then, delete 1 of their Digimon with as many or fewer digivolution cards as this Digimon.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-073 Trash digivolution cards and delete 1 Digimon")
        effect3.set_effect_description("[When Digivolving] [When Attacking] If DNA digivolving, trash any 8 digivolution cards from your opponent's Digimon. Then, delete 1 of their Digimon with as many or fewer digivolution cards as this Digimon.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete, Trash Digivolution Cards"""
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
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If DNA digivolving, trash any 8 digivolution cards from your opponent's Digimon. Then, delete 1 of their Digimon with as many or fewer digivolution cards as this Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX5-073 Delete 1 Digimon with as many or fewer digivolution cards as this Digimon")
        effect4.set_effect_description("[When Attacking] If DNA digivolving, trash any 8 digivolution cards from your opponent's Digimon. Then, delete 1 of their Digimon with as many or fewer digivolution cards as this Digimon.")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
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

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area by an opponent's effect, by trashing 2 same-level cards in this Digimon's digivolution cards, prevent it from leaving.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX5-073 Prevent this Digimon from being leaving the battle area by opponent's effect")
        effect5.set_effect_description("[All Turns] When this Digimon would leave the battle area by an opponent's effect, by trashing 2 same-level cards in this Digimon's digivolution cards, prevent it from leaving.")
        effect5.is_optional = True
        effect5.set_hash_string("Substitute_EX5_073")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
