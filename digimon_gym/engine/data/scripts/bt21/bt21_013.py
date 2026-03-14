from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_013(CardScript):
    """BT21-013 Agunimon | Lv.4

    [When Digivolving] You may place 1 [Hybrid] or [Hero] trait Digimon card from
    your hand or trash as this Digimon's bottom digivolution card or under any of
    your red Tamers with inherited effects.
    [When Attacking] This Digimon may digivolve into a red [Hybrid] or [Hero] trait
    Digimon card in the hand with the digivolution cost reduced by 1.
    Inherited: [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Place 1 Hybrid/Hero Digimon from hand/trash under this or red Tamer
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-013 Place 1 Hybrid/Hero under this or red tamer")
        effect2.set_effect_description("[When Digivolving] Place 1 Hybrid/Hero Digimon from hand/trash as bottom digi card.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Place 1 Hybrid/Hero Digimon from hand or trash under this Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def hybrid_hero_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Hybrid' in t or 'Hero' in t for t in traits)

            # Check hand and trash for qualifying cards
            candidates = [c for c in player.hand_cards if hybrid_hero_filter(c)]
            candidates += [c for c in player.trash_cards if hybrid_hero_filter(c)]
            if not candidates:
                return

            # Auto-select first qualifying card
            chosen = candidates[0]
            if chosen in player.hand_cards:
                player.hand_cards.remove(chosen)
            elif chosen in player.trash_cards:
                player.trash_cards.remove(chosen)
            perm.card_sources.insert(0, chosen)  # bottom digi card

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [When Attacking] Digivolve into red Hybrid/Hero from hand with cost -1
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT21-013 Digivolve into red Hybrid/Hero")
        effect3.set_effect_description("[When Attacking] This Digimon may digivolve into a red Hybrid/Hero from hand with cost -1.")
        effect3.is_optional = True
        effect3.is_on_attack = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Digivolve into a red Hybrid/Hero Digimon from hand with cost -1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            from ....data.enums import CardColor

            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                if CardColor.Red not in colors:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Hybrid' in t or 'Hero' in t for t in traits)

            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_reduction=1, is_optional=True,
                prompt="Select a red Hybrid/Hero Digimon from hand to digivolve into (cost -1).")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Inherited: [Your Turn] This Digimon gets +2000 DP.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-013 DP modifier")
        effect4.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect4.is_inherited_effect = True
        effect4.dp_modifier = 2000

        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
