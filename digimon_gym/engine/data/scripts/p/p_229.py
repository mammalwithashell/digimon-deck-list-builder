from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_229(CardScript):
    """P-229 Unique Emblem: Narrative Ronde"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 3 cards of your deck. Add 1 [Puppet] trait Digimon card
        # and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the
        # bottom of the deck. Then, place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-229 Reveal 3, add 1 [Puppet] Digimon and 1 [LIBERATOR], rest to deck bottom, place in battle area.")
        effect0.set_effect_description("[Main] Reveal the top 3 cards of your deck. Add 1 [Puppet] trait Digimon card and 1 [LIBERATOR] trait card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 Puppet Digimon and 1 LIBERATOR to hand, rest to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter_0(c):
                """Pass 1: 1 [Puppet] trait Digimon card."""
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            def reveal_filter_1(c):
                """Pass 2: 1 [LIBERATOR] trait card (any kind)."""
                traits = getattr(c, 'card_traits', []) or []
                return any('LIBERATOR' in t for t in traits)

            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay flag
        # Marks this option as a Delay card so it stays in battle area
        effect1 = ICardEffect()
        effect1.set_effect_name("P-229 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone (observer)
        # [Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay>.
        # 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait
        # card in the hand with the digivolution cost reduced by 3.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("P-229 Delay: Digivolve into Lv6- LIBERATOR for 3 less.")
        effect2.set_effect_description("[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay>. 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3.")
        effect2.is_optional = True
        # NOTE: is_on_play is intentionally NOT set here.
        # This is an observer that watches for OTHER permanents (Mirai Kinosaki)
        # being played, not for this card itself being played.

        def condition2(context: Dict[str, Any]) -> bool:
            # This card must still be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] restriction
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the PLAYED permanent is a [Mirai Kinosaki]
            played_perm = context.get('played_permanent')
            if played_perm is None:
                return False
            if not played_perm.contains_card_name('Mirai Kinosaki'):
                return False
            # Must be our own Mirai Kinosaki (not opponent's)
            event_player = context.get('event_player')
            if event_player is not None and card.owner is not None:
                if event_player is not card.owner:
                    return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash this delay card, then 1 of your Digimon may digivolve into
            a Lv6 or lower LIBERATOR from hand with cost -3."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash this delay card from battle area
            delay_perm = card.permanent_of_this_card() if card else None
            if delay_perm and delay_perm in player.battle_area:
                player.delete_permanent(delay_perm)

            def digi_filter(c):
                """Lv6 or lower, LIBERATOR trait."""
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv > 6:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('LIBERATOR' in t for t in traits)

            def own_filter(p):
                """Select one of your Digimon."""
                return p.is_digimon

            def on_select(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=3, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Activate this card's [Main] effects.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("P-229 Security: Activate Main effects")
        effect3.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Security: Activate the [Main] reveal-and-select effect."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Re-execute the Main effect logic (reveal top 3, add Puppet Digimon + LIBERATOR)
            process0(ctx)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
