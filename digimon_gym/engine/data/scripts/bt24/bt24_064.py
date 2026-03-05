from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_064(CardScript):
    """BT24-064 Ouryumon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-064 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-064 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        effect1b = ICardEffect()
        effect1b.set_effect_name("BT24-064 Piercing")
        effect1b.set_effect_description("Piercing")
        effect1b._is_piercing = True

        def condition1b(context: Dict[str, Any]) -> bool:
            return True
        effect1b.set_can_use_condition(condition1b)
        effects.append(effect1b)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 7 or lower [Digi] or [SEEKERS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-064 Reveal the top 3 cards of deck, play 1")
        effect2.set_effect_description("[When Digivolving] Reveal the top 3 cards of your deck. You may play 1 play cost 7 or lower [Digi] or [SEEKERS] trait card among them without paying the cost. Return the rest to the top or bottom of the deck.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Reveal top 3, play 1 cost<=7 [DigiPolice]/[SEEKERS] card free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                if getattr(c, 'get_cost_itself', 0) > 7:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('DigiPolice' in t or 'SEEKERS' in t for t in traits)

            def on_revealed(selected, remaining):
                # Play the selected card without paying the cost
                played_perm = player.play_card_from_source(selected, pay_cost=False)
                game.logger.log(f"[Effect] {player.player_name} played {getattr(selected, 'card_name_eng', '?')} from revealed cards")
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": selected, "played_permanent": played_perm, "event_player": player},
                )
                if selected.is_option:
                    game._trash_option_after_resolution(player, played_perm)
                # Return remaining to bottom of deck
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] [Once Per Turn] When any Digimon or Tamers suspend, <De-Digivolve 2> 1 of your opponent's Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT24-064 De-Digivolve 2")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When any Digimon or Tamers suspend, <De-Digivolve 2> 1 of your opponent's Digimon.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT24_064_DeDigivolve")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
