from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_092(CardScript):
    """BT5-092 Nokia Shiramine | Tamer | White | Cost 3

    [On Play] You may play 1 [Agumon] or [Gabumon] from your hand without
        paying the cost.
    [Your Turn] When one of your Digimon would digivolve into a Digimon card
        with [Greymon], [Garurumon] or [Omnimon] in its name, by suspending
        this Tamer, reduce the digivolution cost by 1.
    Security Effect: [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [On Play] Play 1 [Agumon] or [Gabumon] from hand free ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT5-092 On Play: Play Agumon or Gabumon from hand")
        effect0.set_effect_description(
            "[On Play] You may play 1 [Agumon] or [Gabumon] from your hand "
            "without paying its memory cost."
        )
        effect0.is_on_play = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check if there's an Agumon or Gabumon in hand
            for hc in player.hand_cards:
                if hc.contains_card_name('Agumon') or hc.contains_card_name('Gabumon'):
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play 1 [Agumon] or [Gabumon] from hand without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def agumon_gabumon_filter(c):
                return (c.contains_card_name('Agumon')
                        or c.contains_card_name('Gabumon'))

            game.effect_play_from_zone(
                player, 'hand', agumon_gabumon_filter, free=True,
                is_optional=True)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Digisorption-like: suspend to reduce evo cost by 1 ---
        # [Main] When digivolving into a Digimon with [Garurumon], [Omnimon],
        # or [Greymon] in its name, suspend this Tamer to reduce cost by 1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT5-092 Reduce digivolution cost by 1")
        effect1.set_effect_description(
            "[Main] When digivolving one of your Digimon into a Digimon card "
            "in your hand with [Garurumon], [Omnimon], or [Greymon] in its "
            "name, you may suspend this Tamer to reduce the memory cost of "
            "the digivolution by 1."
        )
        effect1.is_optional = True
        effect1.cost_reduction = 1
        effect1.set_hash_string("Digisorption-1_BT5_092")

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on field and not suspended
            if card and card.permanent_of_this_card() is None:
                return False
            own_perm = card.permanent_of_this_card()
            if own_perm and own_perm.is_suspended:
                return False
            # Leak guard: the permanent being digivolved must NOT be this tamer
            card_source = context.get('card_source')
            if card_source is card:
                return False
            # Must be digivolving own Digimon
            if card_source and card_source.owner is not card.owner:
                return False
            # Check the digivolving card has Garurumon/Omnimon/Greymon in name
            if card_source:
                if (card_source.contains_card_name('Garurumon')
                        or card_source.contains_card_name('Omnimon')
                        or card_source.contains_card_name('Greymon')):
                    return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer to reduce digivolution cost by 1."""
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm and not own_perm.is_suspended:
                own_perm.suspend()
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT5-092 Security: Play free")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
