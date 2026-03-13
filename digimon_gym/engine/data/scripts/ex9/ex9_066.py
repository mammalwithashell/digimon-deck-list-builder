from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_066(CardScript):
    """EX9-066 Tai Kamiya & Matt Ishida | Tamer | Red/Blue | Cost 4

    [On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or
        [Omnimon] in its name from your trash to the hand. If this effect
        didn't return, <Draw 1>.

    [All Turns] When any of your Digimon are played or digivolve, by
        suspending this Tamer, gain 1 memory if you have a Digimon with
        [Greymon] in its name. Then, gain 1 memory if you have a Digimon
        with [Garurumon] in its name.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_target_digimon(c) -> bool:
            """Digimon card with Greymon, Garurumon, or Omnimon in its name."""
            if not getattr(c, 'is_digimon', False):
                return False
            if c.contains_card_name('Greymon'):
                return True
            if c.contains_card_name('Garurumon'):
                return True
            if c.contains_card_name('Omnimon'):
                return True
            return False

        # --- Effect 0: [On Play] Return 1 Greymon/Garurumon/Omnimon Digimon
        #     from trash to hand, or Draw 1 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-066 On Play: Return from trash or Draw 1")
        effect0.set_effect_description(
            "[On Play] You may return 1 Digimon card with [Greymon], "
            "[Garurumon] or [Omnimon] in its name from your trash to the "
            "hand. If this effect didn't return, <Draw 1>."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Try to return a matching Digimon from trash to hand
            qualifying = [c for c in player.trash_cards if _is_target_digimon(c)]
            if qualifying:
                chosen = qualifying[0]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)
            else:
                # Didn't return, so Draw 1
                player.draw(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When any of your Digimon are played or
        #     digivolve, suspend this Tamer to gain memory based on field ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-066 Suspend for memory on play/digivolve")
        effect1.set_effect_description(
            "[All Turns] When any of your Digimon are played or digivolve, "
            "by suspending this Tamer, gain 1 memory if you have a Digimon "
            "with [Greymon] in its name. Then, gain 1 memory if you have a "
            "Digimon with [Garurumon] in its name."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Trigger: one of our Digimon was played or digivolved
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != player:
                return False
            if not trigger_perm.is_digimon:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: suspend this Tamer
            perm.suspend()

            # Gain 1 memory if you have a Digimon with [Greymon] in its name
            memory_gain = 0
            has_greymon = any(
                p.is_digimon and p.contains_card_name('Greymon')
                for p in player.battle_area
            )
            if has_greymon:
                memory_gain += 1

            # Then, gain 1 memory if you have a Digimon with [Garurumon]
            has_garurumon = any(
                p.is_digimon and p.contains_card_name('Garurumon')
                for p in player.battle_area
            )
            if has_garurumon:
                memory_gain += 1

            if memory_gain > 0:
                player.add_memory(memory_gain)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX9-066 Security: Play this card free")
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
