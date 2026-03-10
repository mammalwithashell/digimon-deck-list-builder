from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_053(CardScript):
    """EX11-053 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-053 Also treated as [X Antibody]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Draw 1
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-053 By placing a [Royal Knight] under any of your [King Drasil_7D6]s, Draw 1")
        effect1.set_effect_description("Draw 1")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check if player has any King Drasil_7D6 in battle area or breeding
            has_king_drasil = False
            for p in player.battle_area:
                if p.contains_card_name('King Drasil_7D6'):
                    has_king_drasil = True
                    break
            if not has_king_drasil and player.breeding_area:
                if player.breeding_area.contains_card_name('King Drasil_7D6'):
                    has_king_drasil = True
            if not has_king_drasil:
                return False
            # Check if player has a Royal Knight trait card in hand
            has_rk_in_hand = any(
                any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or []))
                for c in player.hand_cards
            )
            return has_rk_in_hand

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place Royal Knight from hand under King Drasil, then Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: select a Royal Knight trait card from hand and place under King Drasil
            def hand_filter(c):
                return any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or []))
            def on_select(selected_card):
                # Find King Drasil permanent
                king_drasil = None
                for p in player.battle_area:
                    if p.contains_card_name('King Drasil_7D6'):
                        king_drasil = p
                        break
                if not king_drasil and player.breeding_area:
                    if player.breeding_area.contains_card_name('King Drasil_7D6'):
                        king_drasil = player.breeding_area
                if king_drasil and selected_card:
                    player.hand_cards.remove(selected_card)
                    king_drasil.add_card_source(selected_card)
                    # Reward: draw 1
                    player.draw_cards(1)
            game.effect_select_hand_card(
                player, hand_filter, on_select, is_optional=True,
                prompt="Select a [Royal Knight] card to place under King Drasil.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Play Card
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX11-053 Play 1 [Omnimon (X Antibody)] and place this card under it.")
        effect2.set_effect_description("Play Card")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('King Drasil_7D6'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'has_play_cost', False):
                    return False
                if not (any('Omnimon (X Antibody)' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
