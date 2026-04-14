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

            import random
            from ....data.enums import CardColor, GamePhase
            from ....core.permanent import Permanent
            from ....game.constants import SEL_MY_SECURITY_START

            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return CardColor.Red in colors or CardColor.Yellow in colors

            # Build valid selection indices
            valid = []
            for i, c in enumerate(player.security_cards):
                if tamer_filter(c):
                    valid.append(SEL_MY_SECURITY_START + i)

            if not valid:
                # No valid tamers — just shuffle security
                random.shuffle(player.security_cards)
                return

            def on_select(action_id: int):
                idx = action_id - SEL_MY_SECURITY_START
                if 0 <= idx < len(player.security_cards):
                    chosen = player.security_cards[idx]
                    # Remove from security
                    player.remove_from_security(chosen)
                    # Play as new permanent (free)
                    played_perm = player.play_card_from_source(chosen, pay_cost=False)
                    if played_perm:
                        played_perm.turn_played = game.turn_count
                        played_perm._owner_game = game
                        # Fire OnEnterFieldAnyone for the played tamer
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {
                                'played_card': chosen,
                                'played_permanent': played_perm,
                                'event_permanent': played_perm,
                                'event_player': player,
                            },
                        )
                        game._fire_play_observers(played_perm, player)
                        # Recovery +1
                        player.recovery(1)
                    # Shuffle security
                    random.shuffle(player.security_cards)

            def on_decline():
                # Player declined — just shuffle security (no recovery)
                random.shuffle(player.security_cards)

            game.request_selection(
                GamePhase.SelectSecurity, player, on_select, valid,
                is_optional=True,
                prompt="You may play 1 red or yellow Tamer card from your security stack without paying its cost.",
                on_decline=on_decline)

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
