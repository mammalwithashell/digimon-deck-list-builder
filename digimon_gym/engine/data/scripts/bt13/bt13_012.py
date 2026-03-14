from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_012(CardScript):
    """BT13-012 GeoGreymon | Lv.4

    Alt digivolve: from Lv.3 [Agumon] w/ Dinosaur trait for 2.
    [When Digivolving] Search your security stack, and you may play 1 red or
        yellow Tamer card among it without paying the cost. If you did,
        <Recovery +1 (Deck)>. Then, shuffle your security stack.
    Inherited: [Your Turn][Once Per Turn] When one of your red or yellow
        Tamers becomes suspended, you may delete 1 of your opponent's Digimon
        with 3000 DP or less.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.3 Agumon w/ Dinosaur for 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-012 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Agumon"
        effect0._alt_digi_trait = "Dinosaur"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Search security for red/yellow Tamer ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-012 Play 1 Tamer from security")
        effect1.set_effect_description(
            "[When Digivolving] Search your security stack, and you may play "
            "1 red or yellow Tamer card among it without paying the cost. If "
            "you did, <Recovery +1 (Deck)>. Then, shuffle your security stack."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Search security for red/yellow Tamer, play it, then Recovery +1, then shuffle."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return

            from ....data.enums import CardColor

            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Red in colors or CardColor.Yellow in colors

            # Search security: find qualifying tamers
            matching = [c for c in player.security_cards if tamer_filter(c)]
            played = False

            if matching:
                # Play the first matching tamer from security
                chosen = matching[0]
                player.security_cards.remove(chosen)
                played_perm = player.play_card_from_source(chosen, pay_cost=False)
                if played_perm:
                    played = True

            # If we played a tamer, Recovery +1
            if played:
                player.recovery(1)

            # Shuffle security stack
            if player.security_cards:
                import random
                random.shuffle(player.security_cards)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [Your Turn][OPT] When red/yellow Tamer suspends, delete <=3000 DP ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name("BT13-012 Delete 1 Digimon with 3000 DP or less")
        effect2.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your red or yellow Tamers "
            "becomes suspended, you may delete 1 of your opponent's Digimon "
            "with 3000 DP or less."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Delete_BT13_012")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the suspended permanent is a red/yellow tamer we own
            event_perm = context.get('event_permanent')
            if event_perm:
                from ....data.enums import CardColor
                if not event_perm.is_tamer:
                    return False
                tc = event_perm.top_card
                if not tc:
                    return False
                colors = getattr(tc, 'card_colors', []) or []
                if CardColor.Red not in colors and CardColor.Yellow not in colors:
                    return False
                # Must be own tamer
                if tc.owner != card.owner:
                    return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with 3000 DP or less."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if p.dp is None or p.dp > 3000:
                    return False
                return p.is_digimon

            def on_delete(target_perm):
                enemy = player.enemy
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
