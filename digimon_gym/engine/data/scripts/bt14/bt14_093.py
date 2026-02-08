from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_093(CardScript):
    """Auto-transpiled from DCGO BT14_093.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-093 Recovery +1, Play Card")
        effect0.set_effect_description("[Main] Search your security stack. 1 of your Digimon may digivolve into 1 yellow level 6 or lower Digimon card with the [Vaccine] trait among them without paying the cost. Then, shuffle your security stack. If digivolved by this effect, and you have a Tamer with [T.K. Takaishi] in its name, <Recovery +1 (Deck)>.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Search security, digivolve 1 of your Digimon into yellow Lv6-or-lower Vaccine from security. If digivolved and have T.K. Takaishi, Recovery +1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return

            def is_yellow_vaccine_lv6_or_lower(c):
                """Filter: yellow Digimon with [Vaccine] trait, level 6 or lower."""
                if not c.is_digimon:
                    return False
                from ....data.enums import CardColor
                if c.color != CardColor.Yellow:
                    return False
                if getattr(c, 'level', 99) > 6:
                    return False
                traits = getattr(c, 'traits', []) or []
                return any('Vaccine' in t for t in traits)

            # First, select target Digimon on own field to digivolve
            def on_field_selected(target_perm):
                # Now select security card to digivolve into
                def on_security_selected(sec_card):
                    player.remove_from_security(sec_card)
                    target_perm.add_card_source(sec_card)
                    game.logger.log(
                        f"[Effect] {player.player_name} digivolved "
                        f"{target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'} "
                        f"from security (no cost)")
                    # Draw 1 (digivolution bonus)
                    player.draw()
                    # Shuffle security stack
                    import random
                    random.shuffle(player.security_cards)
                    # If have T.K. Takaishi tamer, Recovery +1
                    has_tk = any(
                        any('T.K. Takaishi' in n for n in p.top_card.card_names)
                        for p in player.battle_area
                        if p.top_card and p.top_card.is_tamer
                    )
                    if has_tk:
                        player.recovery(1)
                        game.logger.log(
                            f"[Effect] Recovery +1 (T.K. Takaishi present)")

                game.effect_select_own_security(
                    player, is_yellow_vaccine_lv6_or_lower,
                    on_security_selected, is_optional=True)

            # Select one of your Digimon to digivolve
            game.effect_select_own_permanent(
                player, on_field_selected,
                filter_fn=lambda p: p.is_digimon,
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-093 Play Card, Trash From Hand, Add To Hand")
        effect1.set_effect_description("[Security] You may play 1 [Patamon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play 1 [Patamon] from hand or trash without cost, then add this to hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def is_patamon(c):
                return any('Patamon' in n for n in c.card_names)

            # Check hand first, then trash for Patamon
            has_in_hand = any(is_patamon(c) for c in player.hand_cards)
            has_in_trash = any(is_patamon(c) for c in player.trash_cards)
            if has_in_hand:
                game.effect_play_from_zone(
                    player, 'hand', is_patamon, free=True, is_optional=True)
            elif has_in_trash:
                game.effect_play_from_zone(
                    player, 'trash', is_patamon, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
