from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_206(CardScript):
    """P-206 Digital Gate Open | Option (White, Cost 4)

    You can ignore this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 Digimon card and 1 Tamer
    card among them to the hand. Return the rest to the bottom of the deck.
    Then, place this card in the battle area.
    [Main] <Delay> You may play 1 Tamer card with the same color as any of
    your Digimon on the field from your hand with the play cost reduced by 4.
    [Security] You may play 1 Digimon card with a play cost of 3 or less from
    your hand or trash without paying the cost. Then, add this card to the hand.

    C# ref: P_206.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements ---
        card._match_color_requirement = False

        # --- Effect 1: [Main] Reveal top 3, add 1 Digimon + 1 Tamer ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name(
            "P-206 Reveal top 3, add 1 Digimon and 1 Tamer to hand"
        )
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
                is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay marker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("P-206 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Main] <Delay> Play Tamer matching Digimon color ---
        # NOTE: _is_field_main must NOT be set here. The delay mechanism has
        # its own action slot (effectIdx=1 on the option permanent). Adding
        # _is_field_main would expose the effect at effectIdx=2 as well,
        # creating a duplicate action that fires without trashing the Option.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name(
            "P-206 Play 1 color-matched Tamer from hand with cost -4"
        )
        effect3.set_effect_description(
            "[Main] <Delay> You may play 1 Tamer card with the same color as "
            "any of your Digimon on the field from your hand with the play "
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

            # Collect all colors from player's Digimon on the field
            field_colors = set()
            for p in player.battle_area:
                if p.is_digimon and p.top_card:
                    for col in (getattr(p.top_card, 'card_colors', []) or []):
                        field_colors.add(col)

            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                tamer_colors = set(getattr(c, 'card_colors', []) or [])
                return bool(tamer_colors & field_colors)

            game.effect_play_from_zone(
                player, 'hand', play_filter,
                free=False, manual_reduction=4, is_optional=True
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Play Digimon cost<=3 from hand/trash free ---
        # C# ref: SecuritySkill timing, play from hand or trash,
        #   then AddThisCardToHand
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name(
            "P-206 Security: play Digimon cost<=3 free, add this to hand"
        )
        effect4.set_effect_description(
            "[Security] You may play 1 Digimon card with a play cost of 3 or "
            "less from your hand or trash without paying the cost. Then, add "
            "this card to the hand."
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
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    return False
                return cost <= 3

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter,
                free=True, is_optional=True
            )

            # "Then, add this card to the hand."
            # Mark as security_played to prevent the engine from trashing
            # this card after the security check resolves.
            if card:
                card._security_played = True
                # Move card to hand (it might be floating after security pop)
                if player and card not in player.hand_cards:
                    if card in player.trash_cards:
                        player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
