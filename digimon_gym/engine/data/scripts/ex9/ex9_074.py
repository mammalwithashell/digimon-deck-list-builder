from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_074(CardScript):
    """EX9-074 Kimeramon | Lv.5 | Composite/DM/Ver.3
    <Rush>
    <Security A. +1>
    [On Play][When Digivolving] Place 1 Lv.4 or lower [DM] Digimon from trash
    as top digi card. Then, delete 1 opponent Digimon with same color as any
    digi card. If 6+ colors in digi cards, delete 1 of each opponent's Digimon
    with different colors.
    [All Turns] +1000 DP per color in digi cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # <Rush>
        effect_rush = ICardEffect()
        effect_rush.set_effect_name("EX9-074 Rush")
        effect_rush.set_effect_description("Rush")
        effect_rush._is_rush = True

        def cond_rush(context: Dict[str, Any]) -> bool:
            return True
        effect_rush.set_can_use_condition(cond_rush)
        effects.append(effect_rush)

        # <Security A. +1>
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("EX9-074 Security A. +1")
        effect_sa.set_effect_description("Security A. +1")
        effect_sa._security_attack_modifier = 1

        def cond_sa(context: Dict[str, Any]) -> bool:
            return True
        effect_sa.set_can_use_condition(cond_sa)
        effects.append(effect_sa)

        def _get_digi_colors(perm):
            """Get all colors from digivolution cards."""
            colors = set()
            for src in perm.digivolution_cards:
                card_colors = getattr(src, 'card_colors', []) or []
                for col in card_colors:
                    colors.add(col)
            # Also include top card colors
            if perm.top_card:
                for col in (getattr(perm.top_card, 'card_colors', []) or []):
                    colors.add(col)
            return colors

        def _place_and_delete(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Place 1 Lv.4 or lower DM Digimon from trash as top digi card
            for c in list(player.trash_cards):
                if getattr(c, 'is_digimon', False):
                    level = getattr(c, 'level', None)
                    if level is not None and level <= 4:
                        traits = getattr(c, 'card_traits', []) or []
                        if any('DM' in t for t in traits):
                            player.trash_cards.remove(c)
                            perm.add_card_source(c)
                            break

            # Get all colors in digi cards
            colors = _get_digi_colors(perm)

            if len(colors) >= 6:
                # Delete 1 of each opponent's Digimon with different colors
                deleted_colors = set()
                for opp_perm in list(enemy.battle_area):
                    if opp_perm.is_digimon:
                        opp_colors = getattr(opp_perm.top_card, 'card_colors', []) or [] if opp_perm.top_card else []
                        for col in opp_colors:
                            if col not in deleted_colors:
                                enemy.delete_permanent(opp_perm)
                                deleted_colors.add(col)
                                break
            else:
                # Delete 1 opponent Digimon with same color as any digi card
                def color_filter(p):
                    if not p.is_digimon:
                        return False
                    opp_colors = getattr(p.top_card, 'card_colors', []) or [] if p.top_card else []
                    return any(col in colors for col in opp_colors)

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=color_filter, is_optional=False)

        # [On Play]
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-074 On Play: Place DM from trash, color delete")
        effect2.set_effect_description("[On Play] Place DM from trash, delete by color.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_place_and_delete)
        effects.append(effect2)

        # [When Digivolving]
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX9-074 When Digivolving: Place DM from trash, color delete")
        effect3.set_effect_description("[When Digivolving] Place DM from trash, delete by color.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_place_and_delete)
        effects.append(effect3)

        # [All Turns] +1000 DP per color in digi cards
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.NoTiming)
        effect4.set_effect_name("EX9-074 All Turns: +1000 DP per color")
        effect4.set_effect_description("[All Turns] +1000 DP per color in digi cards.")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        # DP modifier is dynamic based on colors, handled by _dp_modifier attribute
        effects.append(effect4)

        return effects
