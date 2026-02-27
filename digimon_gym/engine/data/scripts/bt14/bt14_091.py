from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_091(CardScript):
    """BT14-091 Wave of Reliability"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Trash 2 digivolution cards from your opponent's Digimon.
        # Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon.
        # If your opponent has no Digimon with as many or more digivolution cards as that Digimon, unsuspend it.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-091 Trash Digivolution Cards, Unsuspend")
        effect0.set_effect_description("[Main] Trash 2 digivolution cards from your opponent's Digimon. Then, if you have a Tamer with [Joe Kido] in its name, choose 1 of your Digimon. If your opponent has no Digimon with as many or more digivolution cards as that Digimon, unsuspend it.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash exactly 2 digivolution cards from opponent's Digimon in total.
            trashed_total = 0
            opponent = game.get_opponent_player(player)
            if opponent:
                for opp_perm in list(getattr(opponent, 'battle_area', [])):
                    if trashed_total >= 2:
                        break
                    if getattr(opp_perm, 'has_no_digivolution_cards', True):
                        continue
                    can_trash = min(2 - trashed_total, len(getattr(opp_perm, 'digivolution_cards', [])))
                    if can_trash > 0:
                        trashed = opp_perm.trash_digivolution_cards(can_trash)
                        player.trash_cards.extend(trashed)
                        trashed_total += len(trashed)

            # Continue only if player has a Tamer with [Joe Kido] in its name.
            has_joe_kido = False
            for tamer in getattr(player, 'tamers', []):
                name = getattr(tamer, 'name', '')
                if 'Joe Kido' in name:
                    has_joe_kido = True
                    break
            if not has_joe_kido:
                return

            def on_choose_own(target_perm):
                target_count = len(getattr(target_perm, 'digivolution_cards', []))
                opponent_has_as_many_or_more = False
                if opponent:
                    for opp_perm in getattr(opponent, 'battle_area', []):
                        if len(getattr(opp_perm, 'digivolution_cards', [])) >= target_count:
                            opponent_has_as_many_or_more = True
                            break
                if not opponent_has_as_many_or_more:
                    target_perm.unsuspend()

            game.effect_select_own_permanent(
                player,
                on_choose_own,
                filter_fn=lambda p: True,
                is_optional=False,
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
