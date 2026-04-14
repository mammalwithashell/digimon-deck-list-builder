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
            from digimon_gym.engine.data.enums import GamePhase
            from digimon_gym.engine.game.constants import SOURCES_PER_FIELD, ACTION_SPACE_SIZE

            def target_filter(p):
                if not p.is_digimon:
                    return False
                if not (p.has_trait('Mineral') or p.has_trait('Rock')):
                    return False
                # Must have at least 1 digivolution card to trash
                return not p.has_no_digivolution_cards

            def _apply_grants(target_perm):
                """Grant keywords and DP boost until end of opponent's turn."""
                expiry_turn = game.turn_count + 1
                target_perm.grant_keyword('_is_collision', duration=expiry_turn)
                target_perm.grant_keyword('_is_piercing', duration=expiry_turn)
                target_perm.grant_keyword('_is_reboot', duration=expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_hand', duration=expiry_turn)
                target_perm.grant_keyword('_is_cannot_return_to_deck', duration=expiry_turn)

                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda base, perm, ctx: base + 3000,
                    expiry='end_of_opponent_turn',
                )

            def on_target(target_perm):
                # C# uses SelectTrashDigivolutionCards: player chooses which
                # digivolution card to trash. Use SelectSource phase.
                field_idx = None
                for fi, fp in enumerate(player.battle_area):
                    if fp is target_perm:
                        field_idx = fi
                        break
                if field_idx is None:
                    return

                base = 2000 + field_idx * SOURCES_PER_FIELD
                top = target_perm.top_card
                valid = []
                for i, cs in enumerate(target_perm.card_sources):
                    if cs is top:
                        continue
                    if (base + i) < ACTION_SPACE_SIZE:
                        valid.append(base + i)
                if not valid:
                    return

                def on_source_selected(action_id):
                    idx = action_id - base
                    if not (0 <= idx < len(target_perm.card_sources)):
                        return
                    selected = target_perm.card_sources[idx]
                    # Trash the selected digivolution card
                    target_perm.card_sources.remove(selected)
                    if selected not in player.trash_cards:
                        player.trash_cards.append(selected)
                    # Fire OnDigivolutionCardDiscarded timing
                    game.execute_effects(EffectTiming.OnDigivolutionCardDiscarded,
                                         {'trashed_cards': [selected],
                                          'permanent': target_perm})

                    # Apply grants after successful trash
                    _apply_grants(target_perm)

                game.request_selection(
                    GamePhase.SelectSource, player, on_source_selected,
                    valid, is_optional=True,
                    prompt="Select 1 digivolution card to trash.")

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
