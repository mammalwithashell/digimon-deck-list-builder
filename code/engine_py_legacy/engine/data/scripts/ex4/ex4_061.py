from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_061(CardScript):
    """EX4-061 Matt Ishida & Tai Kamiya | Tamer | Blue/Red | Cost 3

    [Your Turn] When you play a [Gabumon] or [Agumon], you may suspend this
        Tamer to gain 1 memory.

    [Your Turn][Once Per Turn] When one of your Digimon digivolves, if it
        has [Omnimon] in its name, <Draw 2>.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] When you play a [Gabumon] or [Agumon],
        #     you may suspend this Tamer to gain 1 memory ---
        # Uses _is_play_observer pattern: engine fires this via
        # _fire_play_observers when any permanent is played by the owner.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX4-061 Suspend to gain memory on Agumon/Gabumon play")
        effect0.set_effect_description(
            "[Your Turn] When you play a [Gabumon] or [Agumon], you may suspend "
            "this Tamer to gain 1 memory."
        )
        effect0.is_optional = True
        effect0._is_play_observer = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if not player.is_my_turn:
                return False
            # Trigger: a permanent with [Agumon] or [Gabumon] in name was played
            played_perm = context.get('played_permanent')
            if not played_perm:
                return False
            if not played_perm.is_digimon:
                return False
            if not (played_perm.contains_card_name('Agumon')
                    or played_perm.contains_card_name('Gabumon')):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            perm.suspend()
            player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn][Once Per Turn] When one of your Digimon
        #     digivolves, if it has [Omnimon] in its name, <Draw 2>. ---
        # Uses _is_digivolve_observer pattern: engine fires this via
        # _fire_digivolve_observers. Note: this triggers on ANY of your
        # Digimon (including the one carrying this tamer's effects, though
        # tamers don't typically digivolve). The observer skips the
        # permanent carrying the effect, but since this is a tamer, the
        # observer naturally covers all Digimon.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX4-061 Draw 2 on Omnimon digivolve")
        effect1.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your Digimon digivolves, "
            "if it has [Omnimon] in its name, <Draw 2>."
        )
        effect1._is_digivolve_observer = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Draw2_EX4_061")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if not player.is_my_turn:
                return False
            # The digivolved Digimon must have [Omnimon] in its name
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if not digivolved.contains_card_name('Omnimon'):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(2)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX4-061 Security: Play this card free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
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
