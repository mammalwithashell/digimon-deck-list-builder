from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_074(CardScript):
    """EX7-074 Vortex Resonance | Option, Green/Yellow, LIBERATOR, Cost 3

    While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this
        card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 card with the
        [LIBERATOR] trait among them to the hand. Return the rest to the
        bottom of the deck. Then, 1 of your Digimon may digivolve into a
        Digimon card in your hand with the digivolution cost reduced by 4.
    [Security] You may play 1 card with the [LIBERATOR] trait with a play
        cost of 4 or less from your hand or trash without paying the cost.
        Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements while you have LIBERATOR ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX7-074 Ignore color requirements")
        effect0.set_effect_description(
            "While you have [LIBERATOR] trait Digimon or Tamer, you can ignore "
            "this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            has_liberator = any(
                (p.is_digimon or p.is_tamer) and p.top_card and
                any('LIBERATOR' in t or 'Liberator' in t
                    for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in owner.battle_area
            )
            return has_liberator

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # Color requirement bypass handled by condition check

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Main] Reveal top 3, add 1 LIBERATOR, then digivolve -4 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("EX7-074 Reveal top 3 and digivolve -4")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 card with the "
            "[LIBERATOR] trait among them to the hand. Return the rest to the "
            "bottom of the deck. Then, 1 of your Digimon may digivolve into a "
            "Digimon card in your hand with the digivolution cost reduced by 4."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 LIBERATOR to hand, rest to deck bottom.
            Then digivolve 1 Digimon from hand with cost -4."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def liberator_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any('LIBERATOR' in t or 'Liberator' in t for t in traits)

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)  # bottom of deck

            game.effect_reveal_and_select(
                player, 3, liberator_filter, on_revealed, is_optional=True)

            # Then, 1 of your Digimon may digivolve from hand with cost -4
            def digimon_hand_filter(c):
                return getattr(c, 'is_digimon', False)

            def own_digimon_filter(p):
                if not p.is_digimon:
                    return False
                # Must have at least one Digimon in hand that can digivolve onto it
                return any(digimon_hand_filter(c) for c in player.hand_cards)

            own_digimon = [p for p in player.battle_area if own_digimon_filter(p)]
            if own_digimon:
                def on_select_digimon(target_perm):
                    game.effect_digivolve_from_hand(
                        player, target_perm,
                        filter_fn=digimon_hand_filter,
                        cost_reduction=4,
                        is_optional=True,
                        prompt="Select a Digimon card from hand to digivolve into (cost -4).",
                    )

                game.effect_select_own_permanent(
                    player, on_select_digimon,
                    filter_fn=own_digimon_filter,
                    is_optional=True,
                    prompt="Select 1 of your Digimon to digivolve.",
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play 1 LIBERATOR (cost 4 or less) from hand/trash free.
        #     Then add this card to hand. ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX7-074 Security: Play LIBERATOR then add to hand")
        effect2.set_effect_description(
            "[Security] You may play 1 card with the [LIBERATOR] trait with a "
            "play cost of 4 or less from your hand or trash without paying the "
            "cost. Then, add this card to the hand."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 LIBERATOR (cost <=4) from hand or trash free, then add this to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                has_liberator = any('LIBERATOR' in t or 'Liberator' in t for t in traits)
                if not has_liberator:
                    return False
                try:
                    cost = c.get_cost_itself
                except Exception:
                    cost = getattr(c, 'play_cost', None)
                if cost is None or cost > 4:
                    return False
                return not getattr(c, 'is_digi_egg', False)

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

            # Then, add this card to hand
            if card:
                # Move from wherever this card currently is to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)
                elif card in player.security_cards:
                    player.security_cards.remove(card)
                    player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
