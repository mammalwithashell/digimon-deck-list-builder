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
            """Place 1 Hybrid/Hero Digimon from hand or trash as bottom digi card
            under this Digimon or under a red Tamer with inherited effects."""
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

            # Step 1: Player selects a Hybrid/Hero Digimon card from hand or trash
            from ....game.constants import SEL_HAND_START, SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = []
            for i, c in enumerate(player.hand_cards):
                if hybrid_hero_filter(c):
                    valid.append(SEL_HAND_START + i)
            for i, c in enumerate(player.trash_cards):
                if hybrid_hero_filter(c):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_card_selected(action_id):
                chosen = None
                if action_id >= SEL_TRASH_START:
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        chosen = player.trash_cards[idx]
                else:
                    idx = action_id - SEL_HAND_START
                    if 0 <= idx < len(player.hand_cards):
                        chosen = player.hand_cards[idx]
                if not chosen:
                    return

                # Step 2: Player selects destination — this Digimon OR a red Tamer
                from ....data.enums import CardColor

                def dest_filter(p):
                    # This Digimon itself
                    if p is perm:
                        return True
                    # A red Tamer with inherited effects
                    if getattr(p, 'is_tamer', False):
                        top = p.top_card
                        if top:
                            colors = getattr(top, 'card_colors', []) or []
                            if CardColor.Red in colors:
                                return True
                    return False

                def on_dest_selected(dest_perm):
                    # Remove from source zone
                    if chosen in player.hand_cards:
                        player.hand_cards.remove(chosen)
                    elif chosen in player.trash_cards:
                        player.trash_cards.remove(chosen)
                    # Place as bottom digivolution card
                    dest_perm.card_sources.insert(0, chosen)

                game.effect_select_own_permanent(
                    player, on_dest_selected, filter_fn=dest_filter, is_optional=False,
                    prompt="Select destination: this Digimon or a red Tamer.")

            game.request_selection(
                GamePhase.SelectTarget, player, on_card_selected,
                valid, is_optional=True,
                prompt="Select 1 [Hybrid] or [Hero] trait Digimon from hand or trash to place.")

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
