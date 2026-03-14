from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_102(CardScript):
    """BT21-102 Tai Kamiya

    [Start of Your Turn] If your memory is at 2 or less, it becomes 3.
    [Your Turn] When one of your Digimon attacks, by suspending this Tamer,
        <Draw 1>.
    [Main][Once Per Turn] You may play 1 [ADVENTURE]/[Hero] trait card with
        a play cost of 2 or less from your hand without paying the cost.
        For each of your Tamers' colors, add 1 to this effect's play cost
        maximum. Then, return this Tamer to the bottom of the deck.
    Security: Play this card without paying its cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT21-102 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When Digimon attacks, suspend this to Draw 1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT21-102 Suspend to draw 1")
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon attacks, by suspending this "
            "Tamer, <Draw 1>."
        )
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must not already be suspended (cost: suspend this Tamer)
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer as cost, then Draw 1."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return
            # Cost: suspend this Tamer
            perm.suspend()
            # Reward: draw 1
            player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Main][OPT] Play ADVENTURE/Hero card, cost max = 2 + tamer colors ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("BT21-102 Play Adventure/Hero card for reduced cost")
        effect2.set_effect_description(
            "[Main] [Once Per Turn] You may play 1 [ADVENTURE]/[Hero] trait "
            "card with a play cost of 2 or less from your hand without paying "
            "the cost. For each of your Tamers' colors, add 1 to this "
            "effect's play cost maximum. Then, return this Tamer to the bottom "
            "of the deck."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Play_BT21_102")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play ADVENTURE/Hero card, then return this Tamer to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Calculate max play cost: 2 + unique tamer colors
            tamer_colors = set()
            for p in player.battle_area:
                if p.is_tamer and p.top_card:
                    for c in getattr(p.top_card, 'card_colors', []):
                        tamer_colors.add(c)
            max_cost = 2 + len(tamer_colors)

            def play_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                has_adventure = any('ADVENTURE' in t or 'Adventure' in t for t in traits)
                has_hero = any('Hero' in t for t in traits)
                if not (has_adventure or has_hero):
                    return False
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    return False
                return cost <= max_cost

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

            # Then return this Tamer to the bottom of the deck
            perm = card.permanent_of_this_card() if card else None
            if perm:
                player.return_permanent_to_deck_bottom(perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Security: Play this card ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-102 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
