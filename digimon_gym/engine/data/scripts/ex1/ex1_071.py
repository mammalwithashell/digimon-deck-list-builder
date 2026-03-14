from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_071(CardScript):
    """EX1-071 Win Rate: 60%! | Option (Yellow, Cost 2)

    While you have a Tamer, you can ignore this card's color requirements.
    [Main] The next time one of your Digimon would digivolve this turn,
    you may trash 1 Digimon card in your hand of the same color as the
    digivolving Digimon to reduce the memory cost of the digivolution by 4.
    [Security] Add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Bypass color requirement (any deck running this will have a Tamer)
        card._match_color_requirement = False
        effects = []

        # [Main] One-shot digivolution cost -4 with color-matching trash
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX1-071 Reduce next digi cost by 4")
        effect0.set_effect_description(
            "[Main] The next time one of your Digimon would digivolve this "
            "turn, you may trash 1 Digimon card in your hand of the same "
            "color as the digivolving Digimon to reduce the memory cost of "
            "the digivolution by 4."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Collect all colors from Digimon on field to determine valid
            # hand cards (same color as any digivolving candidate)
            field_colors = set()
            for field_perm in player.battle_area:
                if field_perm.is_digimon and field_perm.top_card:
                    for c in (getattr(field_perm.top_card, 'colors', []) or []):
                        field_colors.add(c)

            if not field_colors:
                return

            def color_match_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                hc_colors = set(getattr(c, 'colors', []) or [])
                return bool(hc_colors & field_colors)

            # Check there is at least one valid hand card to trash
            if not any(color_match_filter(hc) for hc in player.hand_cards):
                return

            def on_hand_selected(trashed_card):
                player.hand_cards.remove(trashed_card)
                player.trash_cards.append(trashed_card)
                game.logger.log(
                    f"[EX1-071] Trashed {game._card_ref(trashed_card)} "
                    f"from hand to reduce next digivolution cost by 4.")

                # One-shot flag: consumed after first digivolve uses it
                consumed = [False]

                def digi_cost_condition(target, context):
                    return not consumed[0]

                def digi_cost_value(current, target, context):
                    consumed[0] = True
                    return current - 4

                # Register cost reduction on ALL current Digimon
                for field_perm in player.battle_area:
                    if field_perm.is_digimon:
                        game.register_modifier(
                            field_perm, ModifierType.CHANGE_DIGIVOLUTION_COST,
                            condition=digi_cost_condition,
                            value_fn=digi_cost_value,
                            expiry='end_of_turn',
                        )

            game.effect_select_hand_card(
                player, color_match_filter, on_hand_selected,
                is_optional=True,
                prompt="Select a same-color Digimon card from hand to trash (reduces next digi cost by 4)."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Security] Add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("EX1-071 Add to hand")
        effect1.set_effect_description("[Security] Add this card to the hand.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.hand_cards.append(card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
