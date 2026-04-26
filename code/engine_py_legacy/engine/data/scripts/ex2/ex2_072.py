from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_072(CardScript):
    """EX2-072 Blue Card | Option"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # While you have a Tamer, you can ignore this card's color requirements.
        effect_color = ICardEffect()
        effect_color.set_effect_name("EX2-072 Ignore color requirements with Tamer")
        effect_color.set_effect_description(
            "While you have a Tamer, you can ignore this card's color requirements."
        )
        effect_color.is_declarative = True
        effect_color._ignore_color_requirement = True

        def condition_color(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            return any(p.is_tamer for p in owner.battle_area)

        effect_color.set_can_use_condition(condition_color)
        effects.append(effect_color)

        # [Main] Reveal the top 5 cards of your deck. You may digivolve 1 of
        # your Digimon into 1 non-white Digimon card among them without paying
        # its memory cost. If you don't, add 1 Digimon card among them to your
        # hand. Place the remaining cards at the bottom of your deck in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("EX2-072 Reveal top 5, digivolve or add to hand")
        effect0.set_effect_description(
            "[Main] Reveal the top 5 cards of your deck. You may digivolve 1 "
            "of your Digimon into 1 non-white Digimon card among them without "
            "paying its memory cost. If you don't, add 1 Digimon card among "
            "them to your hand. Place the remaining cards at the bottom of "
            "your deck in any order."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import CardColor

            def non_white_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.White not in colors

            def on_digivolve_selected(selected, remaining):
                """Player chose a non-white Digimon to digivolve into."""
                # Remove selected from library
                if selected in player.library_cards:
                    player.library_cards.remove(selected)

                # Let player choose which of their Digimon to digivolve
                has_digimon = any(p.is_digimon for p in player.battle_area)
                if has_digimon:
                    def on_target_digimon(target_perm):
                        target_perm.add_card_source(selected)
                        if game:
                            target_perm.turn_digivolved = game.turn_count
                            game.logger.log(
                                f"[EX2-072] {player.player_name} digivolved "
                                f"{target_perm.top_card.card_names[0]} from "
                                f"revealed cards (free)."
                            )
                            game.execute_effects(
                                EffectTiming.OnEnterFieldAnyone,
                                {
                                    "played_card": selected,
                                    "played_permanent": target_perm,
                                    "event_permanent": target_perm,
                                    "event_player": player,
                                    "is_digivolving": True,
                                },
                            )

                    game.effect_select_own_permanent(
                        player, on_target_digimon,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                        prompt="Select 1 of your Digimon to digivolve."
                    )
                else:
                    # No Digimon to digivolve onto — card goes to deck bottom
                    player.library_cards.append(selected)

                # Remaining go to deck bottom
                for c in remaining:
                    if c in player.library_cards:
                        player.library_cards.remove(c)
                    player.library_cards.append(c)

            # Use effect_reveal_and_select with optional=True
            # If player declines, the on_decline callback offers adding
            # a Digimon to hand instead (via a second reveal pass)
            game.effect_reveal_and_select(
                player, 5,
                filter_fn=non_white_digimon_filter,
                on_selected=on_digivolve_selected,
                is_optional=True,
                prompt="Select a non-white Digimon to digivolve into (free). Decline to add a Digimon to hand instead."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect: [Security] You may play 1 Tamer card from your hand
        # without paying its memory cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX2-072 Security: Play Tamer from hand free")
        effect1.set_effect_description(
            "[Security] You may play 1 Tamer card from your hand without "
            "paying its memory cost."
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

            def tamer_filter(c):
                return getattr(c, 'is_tamer', False)

            game.effect_play_from_zone(
                player, 'hand', tamer_filter,
                free=True, is_optional=True,
                prompt="You may play 1 Tamer card from your hand without paying the cost."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
