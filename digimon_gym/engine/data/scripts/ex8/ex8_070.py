from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_070(CardScript):
    """EX8-070 Zofr Kabus | Option (Black, Cost 2)

    [Main] By trashing any 1 digivolution card of 1 of your Digimon with the
        [Mineral] or [Rock] trait, until the end of your opponent's turn, that
        Digimon gains <Collision>, <Piercing>, <Reboot>, and your opponent's
        effects can't return it to hands or decks, and it gets +3000 DP.
    [Security] Delete 1 of your opponent's Digimon with the lowest play cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "EX8-070 Trash 1 source of Mineral/Rock Digimon to grant keywords, "
            "+3000 DP, and protection until end of opponent's turn"
        )
        effect0.set_effect_description(
            "[Main] By trashing any 1 digivolution card of 1 of your Digimon "
            "with the [Mineral] or [Rock] trait, until the end of your opponent's "
            "turn, that Digimon gains <Collision>, <Piercing>, <Reboot>, and your "
            "opponent's effects can't return it to hands or decks, and it gets "
            "+3000 DP."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from digimon_gym.engine.interfaces.modifiers import ModifierType

            def target_filter(p):
                if not p.is_digimon:
                    return False
                if not (p.has_trait('Mineral') or p.has_trait('Rock')):
                    return False
                # Must have at least 1 digivolution card to trash
                return not p.has_no_digivolution_cards

            def on_target(target_perm):
                # Trash 1 digivolution card as the cost
                trashed = target_perm.trash_digivolution_cards(1)
                for c in trashed:
                    if c not in player.trash_cards:
                        player.trash_cards.append(c)

                # Grant keywords until end of opponent's turn.
                # duration = turn_count + 1 expires at start of granting player's
                # next turn, covering the full opponent turn.
                expiry_turn = game.turn_count + 1
                target_perm.grant_keyword('_is_collision', duration=expiry_turn)
                target_perm.grant_keyword('_is_piercing', duration=expiry_turn)
                target_perm.grant_keyword('_is_reboot', duration=expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_hand', duration=expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_deck', duration=expiry_turn)

                # +3000 DP until end of opponent's turn
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda base, perm, ctx: base + 3000,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_target, filter_fn=target_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Delete 1 of opponent's Digimon with lowest play cost ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX8-070 Delete opponent's Digimon with lowest play cost")
        effect1.set_effect_description(
            "[Security] Delete 1 of your opponent's Digimon with the lowest play cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            digimon_list = [p for p in enemy.battle_area if p.is_digimon]
            if not digimon_list:
                return
            min_cost = min(
                p.top_card.get_cost_itself if p.top_card else 0
                for p in digimon_list
            )

            def target_filter(p):
                return (
                    p.is_digimon
                    and (p.top_card.get_cost_itself if p.top_card else 0) == min_cost
                )

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
