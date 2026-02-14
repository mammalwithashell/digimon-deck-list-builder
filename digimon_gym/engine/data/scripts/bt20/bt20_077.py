from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_077(CardScript):
    """BT20-077 HeavyMetaldramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-077 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-077 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Dark Dragon] trait for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_trait = "Dark Dragon"

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Dark Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Evil Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash cards in your hand until it has 4 left. Then, play 1 8000 DP or lower Digimon card from your trash without paying the cost. For each card this effect trashed, remove 2000 from this effect's DP maximum.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-077 Trash until 4 left and play a Digimon")
        effect2.set_effect_description("[On Play] Trash cards in your hand until it has 4 left. Then, play 1 8000 DP or lower Digimon card from your trash without paying the cost. For each card this effect trashed, remove 2000 from this effect's DP maximum.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash cards in your hand until it has 4 left. Then, play 1 8000 DP or lower Digimon card from your trash without paying the cost. For each card this effect trashed, remove 2000 from this effect's DP maximum.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-077 Trash until 4 left and play a Digimon")
        effect3.set_effect_description("[When Digivolving] Trash cards in your hand until it has 4 left. Then, play 1 8000 DP or lower Digimon card from your trash without paying the cost. For each card this effect trashed, remove 2000 from this effect's DP maximum.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: blocker
        # Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-077 Blocker")
        effect4.set_effect_description("Blocker")
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Dark Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Evil Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-077 All your Digimon DP modifier")
        effect5.set_effect_description("All your Digimon DP modifier")
        effect5.dp_modifier = 2000
        effect5._applies_to_all_own_digimon = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Dark Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Evil Dragon' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
