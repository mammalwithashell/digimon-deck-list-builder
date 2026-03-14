from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST22_08(CardScript):
    """ST22-08 Offensive Plug-In V | Option | Red | Cost:4

    While you have a Tamer, you can ignore this card's color requirements.
    [Security] Delete 1 of your opponent's Digimon with the lowest DP.
    Then, add this card to the hand.
    [Main] You may link this card to 1 of your Digimon without paying the cost.
    Then, delete 1 of your opponent's Digimon with as much or less DP as 1 of
    your Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Ignore color requirements while you have a Tamer (descriptive — engine handles)
        effect_color = ICardEffect()
        effect_color.set_effect_name("ST22-08 Ignore color requirements")
        effect_color.set_effect_description("While you have a Tamer, you can ignore this card's color requirements.")

        def condition_color(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            return any(getattr(p, 'is_tamer', False) for p in owner.battle_area)
        effect_color.set_can_use_condition(condition_color)
        effects.append(effect_color)

        # [Security] Delete lowest DP opponent Digimon. Then add this card to hand.
        effect_sec = ICardEffect()
        effect_sec.set_timing(EffectTiming.SecuritySkill)
        effect_sec.set_effect_name("ST22-08 Security: Delete lowest DP, add to hand")
        effect_sec.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, add this card to the hand.")
        effect_sec.is_security_effect = True

        def condition_sec(context: Dict[str, Any]) -> bool:
            return True
        effect_sec.set_can_use_condition(condition_sec)

        def process_sec(ctx: Dict[str, Any]):
            """Delete lowest DP opponent Digimon, then add this card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
            if opp_digimon:
                min_dp = min(p.dp for p in opp_digimon)
                lowest = [p for p in opp_digimon if p.dp == min_dp]
                if lowest:
                    target = lowest[0]
                    enemy.delete_permanent(target)

            # Add this card to hand
            if card:
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect_sec.set_on_process_callback(process_sec)
        effects.append(effect_sec)

        # [Main] You may link this card to 1 of your Digimon without paying the cost.
        # Then, delete 1 of your opponent's Digimon with DP <= 1 of your Digimon's DP.
        effect_main = ICardEffect()
        effect_main.set_timing(EffectTiming.OptionSkill)
        effect_main.set_effect_name("ST22-08 Link and delete")
        effect_main.set_effect_description("[Main] Link this card, then delete 1 opponent Digimon with DP <= 1 of your Digimon.")

        def condition_main(context: Dict[str, Any]) -> bool:
            return True
        effect_main.set_can_use_condition(condition_main)

        def process_main(ctx: Dict[str, Any]):
            """Link this card, then delete 1 opponent Digimon with DP <= 1 of yours."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Find max DP among own Digimon for the delete threshold
            own_digimon = [p for p in player.battle_area if p.is_digimon and p.dp is not None]
            if not own_digimon:
                return
            max_own_dp = max(p.dp for p in own_digimon)

            # Delete 1 of opponent's Digimon with DP <= max_own_dp
            def dp_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= max_own_dp

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=dp_filter, is_optional=False,
                prompt="Delete 1 of your opponent's Digimon with DP equal to or less than 1 of your Digimon.")

        effect_main.set_on_process_callback(process_main)
        effects.append(effect_main)

        return effects
