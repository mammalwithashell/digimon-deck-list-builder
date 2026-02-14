from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_083(CardScript):
    """BT20-083 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-083 Also treated as [X Antibody]")
        effect0.set_effect_description("Effect")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-083 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 1 or fewer security cards, this Digimon may digivolve into [Omnimon (X Antibody)] in the hand, ignoring digivolution requirements and without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-083 Digivolve this Digimon into [Omnimon (X Antibody)]")
        effect2.set_effect_description("[On Play] If you have 1 or fewer security cards, this Digimon may digivolve into [Omnimon (X Antibody)] in the hand, ignoring digivolution requirements and without paying the cost.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-083 Place under [King Drasil_7D6] in the breeding area.")
        effect3.set_effect_description("[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('King Drasil_7D6'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [Breeding] [Opponent's Turn] When your security stack is removed from, by suspending this Digimon, play 1 [Omekamon] from this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-083 play 1 [Omekamon]")
        effect4.set_effect_description("[Breeding] [Opponent's Turn] When your security stack is removed from, by suspending this Digimon, play 1 [Omekamon] from this Digimon's digivolution cards without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend, Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
