from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_136(CardScript):
    """P-136 Arisa Kinosaki

    [On Play] You may play 1 [Shoemon] from your hand without paying the cost.
    [Your Turn][Once Per Turn] When one of your Digimon digivolves into a Digimon
        with the [Puppet] trait, by suspending this Tamer, gain 1 memory.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Play 1 [Shoemon] from hand free ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("P-136 Play 1 [Shoemon] from your hand")
        effect0.set_effect_description("[On Play] You may play 1 [Shoemon] from your hand without paying the cost.")
        effect0.is_optional = True
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def _is_exact_shoemon(c) -> bool:
            """C# uses CardNames.Contains('Shoemon') — exact list membership."""
            return 'Shoemon' in c.card_names

        def process0(ctx: Dict[str, Any]):
            """Action: Play 1 [Shoemon] from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_play_from_zone(
                player, 'hand', _is_exact_shoemon, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn][OPT] Digivolve observer ---
        # "When one of your Digimon digivolves into a Digimon with the [Puppet] trait,
        #  by suspending this Tamer, gain 1 memory."
        # Uses _is_digivolve_observer (NOT is_when_digivolving) since this fires
        # on the tamer when a DIFFERENT permanent digivolves.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-136 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When one of your Digimon digivolves into a Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Digivoles_P_136")
        effect1._is_digivolve_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Cost: tamer must not already be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and getattr(tamer_perm, 'is_suspended', False):
                return False
            # Check the digivolved permanent has Puppet trait
            trigger_perm = context.get('digivolved_permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != card.owner:
                return False
            if not trigger_perm.is_digimon:
                return False
            if trigger_perm.top_card:
                traits = getattr(trigger_perm.top_card, 'card_traits', []) or []
                if any('Puppet' in t for t in traits):
                    return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            if tamer_perm.is_suspended:
                return
            tamer_perm.suspend()
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("P-136 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
