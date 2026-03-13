from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX2_007(CardScript):
    """EX2-007 Mother D-Reaper | Digi-Egg/Token | White | Ability Synthesis Agent, D-Reaper

    [All Turns] This Digimon can't attack and isn't affected by your
        opponent's effects.
    [Main] [Once Per Turn] If you don't have another [Mother D-Reaper] in play,
        place 1 of your [ADR-02 Searcher]s from in play or your hand under this
        Digimon as its bottom digivolution card.
    [Your Turn] [Once Per Turn] When you would play a card with the [D-Reaper]
        trait from your hand, you may reduce its play cost by 1 for each of
        this Digimon's digivolution cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Can't attack ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX2-007 Can't attack")
        effect0.set_effect_description(
            "[All Turns] This Digimon can't attack."
        )
        effect0._is_cannot_attack = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] Isn't affected by opponent's effects ---
        # The engine's is_immune_to_opponent_effects only covers Progress.
        # We model this as a keyword-based immunity.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX2-007 Immune to opponent's effects")
        effect1.set_effect_description(
            "[All Turns] This Digimon isn't affected by your opponent's effects."
        )
        effect1._is_immune_to_opponent_effects = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Main] [Once Per Turn] Place ADR-02 Searcher as bottom digi ---
        # If no other Mother D-Reaper in play, place 1 [ADR-02 Searcher]
        # from field or hand under this Digimon as bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("EX2-007 Main: Place ADR-02 Searcher as digi card")
        effect2.set_effect_description(
            "[Main] [Once Per Turn] If you don't have another [Mother D-Reaper] "
            "in play, place 1 of your [ADR-02 Searcher]s from in play or your "
            "hand under this Digimon as its bottom digivolution card."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Main_EX2_007")

        def _is_adr02(c) -> bool:
            """Check if a card is ADR-02 Searcher."""
            names = getattr(c, 'card_names', []) or []
            return any('ADR-02 Searcher' in n or 'ADR-02Searcher' in n for n in names)

        def _has_another_mother_dreaper() -> bool:
            """Check if player has another Mother D-Reaper in play."""
            player = card.owner if card else None
            if not player:
                return False
            my_perm = card.permanent_of_this_card() if card else None
            for p in player.battle_area:
                if p is my_perm:
                    continue
                if p.top_card:
                    names = getattr(p.top_card, 'card_names', []) or []
                    if any('Mother D-Reaper' in n or 'MotherD-Reaper' in n for n in names):
                        return True
            return False

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must not have another Mother D-Reaper in play
            if _has_another_mother_dreaper():
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have ADR-02 Searcher on field or in hand
            for p in player.battle_area:
                my_perm = card.permanent_of_this_card() if card else None
                if p is my_perm:
                    continue
                if p.top_card and _is_adr02(p.top_card):
                    return True
            for c_hand in player.hand_cards:
                if _is_adr02(c_hand):
                    return True
            return False
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Place 1 ADR-02 Searcher from field or hand as bottom digi card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            # Try field first - find ADR-02 Searcher permanent
            field_target = None
            for p in list(player.battle_area):
                if p is my_perm:
                    continue
                if p.top_card and _is_adr02(p.top_card):
                    field_target = p
                    break

            if field_target:
                # Remove permanent from battle area and place all its cards
                # as bottom digivolution cards
                player.battle_area.remove(field_target)
                for cs in field_target.card_sources:
                    my_perm.add_card_source_bottom(cs)
                return

            # Try hand
            hand_target = None
            for c_hand in player.hand_cards:
                if _is_adr02(c_hand):
                    hand_target = c_hand
                    break

            if hand_target:
                player.hand_cards.remove(hand_target)
                my_perm.add_card_source_bottom(hand_target)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: BeforePayCost - D-Reaper cost reduction ---
        # [Your Turn] [Once Per Turn] When playing a D-Reaper trait card from hand,
        # reduce play cost by 1 per digivolution card count.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.BeforePayCost)
        effect3.set_effect_name("EX2-007 Cost reduction for D-Reaper")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When you would play a card with the "
            "[D-Reaper] trait from your hand, you may reduce its play cost by "
            "1 for each of this Digimon's digivolution cards."
        )
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Reduce_EX2_007")

        def condition3(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                # This effect triggers when ANY D-Reaper card is played,
                # not just this card. We need a different approach.
                pass

            # Check: must be on field, your turn
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False

            # The card being played must be a D-Reaper trait card from hand
            played_card = context.get('card_source')
            if not played_card:
                return False
            traits = getattr(played_card, 'card_traits', []) or []
            if not any('D-Reaper' in t for t in traits):
                return False
            # Must not be an Option card
            if getattr(played_card, 'is_option', False):
                return False

            # Must have digivolution cards
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm or len(my_perm.card_sources) <= 1:
                return False

            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Reduce play cost by 1 per digivolution card count."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return
            # Digivolution card count = total sources minus the top card
            digi_count = max(len(my_perm.card_sources) - 1, 0)
            if digi_count > 0:
                if hasattr(player, '_temp_play_cost_reduction'):
                    player._temp_play_cost_reduction += digi_count
                else:
                    player._temp_play_cost_reduction = digi_count
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
