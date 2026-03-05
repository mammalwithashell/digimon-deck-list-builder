from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_206(CardScript):
    """P-206 Digital Gate Open"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Ignore color requirements (passive) ──────────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("P-206 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Ignores color requirement for playing Options — not modeled in engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── [Main] Reveal top 3, add 1 Digimon + 1 Tamer to hand ────
        # Then, place this card in the battle area.
        # NOTE: The "place this card in the battle area" part is handled
        # by the engine via the _is_delay flag on the Delay marker below.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-206 Reveal 3")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 Digimon card "
            "and 1 Tamer card among them to the hand. Return the rest to the "
            "bottom of the deck. Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_digimon(c):
                return getattr(c, 'is_digimon', False)

            def reveal_filter_tamer(c):
                return getattr(c, 'is_tamer', False)

            game.effect_reveal_and_select_multi(
                player, 3,
                [(reveal_filter_digimon, 'hand'), (reveal_filter_tamer, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Delay marker ─────────────────────────────────────────────
        # Causes the option to stay on field after resolution.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-206 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # ── Delay effect (must be immediately after _is_delay marker) ─
        # You may play 1 Tamer card with the same color as any of your
        # Digimon on the field from your hand with the play cost reduced by 4.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name(
            "P-206 Play 1 color-matched Tamer from hand with cost -4"
        )
        effect3.set_effect_description(
            "[Main] <Delay>. You may play 1 Tamer card with the same color "
            "as any of your Digimon on the field from your hand with the play "
            "cost reduced by 4."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Collect all colors from Digimon on the field
            field_colors = set()
            for p in player.battle_area:
                if p.is_digimon and p.top_card:
                    for col in getattr(p.top_card, 'card_colors', []):
                        field_colors.add(col)

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                # Must share at least one color with any of your Digimon on field
                tamer_colors = set(getattr(c, 'card_colors', []))
                return bool(tamer_colors & field_colors)

            game.effect_play_from_zone(
                player, 'hand', play_filter,
                free=False, manual_reduction=4, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ── [Security] ───────────────────────────────────────────────
        # You may play 1 Digimon card with a play cost of 3 or less from
        # your hand or trash without paying the cost.
        # Then, add this card to the hand.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name(
            "P-206 Security: play 3-cost Digimon, add this to hand"
        )
        effect4.set_effect_description(
            "[Security] You may play 1 Digimon card with a play cost of 3 "
            "or less from your hand or trash without paying the cost. Then, "
            "add this card to the hand."
        )
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                return getattr(c, 'get_cost_itself', 0) <= 3

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter,
                free=True, is_optional=True)

            # "Then, add this card to the hand."
            # The security callback fires before the engine trashes the
            # security card. We pre-add the card reference to hand; the
            # engine will also append it to trash (known engine limitation
            # shared by all security "add to hand" scripts).
            if player and card:
                player.hand_cards.append(card)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
