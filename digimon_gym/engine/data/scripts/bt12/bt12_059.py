from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_059(CardScript):
    """BT12-059 Agumon (Black) | Lv.3 Black Rookie

    Alt digivolve: from [Koromon] for 0.
    [On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with
        [Greymon] or [Omnimon] in its name and 1 Tamer card with [Tai Kamiya]
        in its name among them to your hand. Place the remaining cards at the
        bottom of your deck in any order.
    Inherited: [All Turns] While this Digimon has [Greymon] or [Omnimon] in
        its name, it gets +1000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Koromon] for 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT12-059 Alt digivolve from Koromon")
        effect0.set_effect_description("Alt digivolve: from [Koromon] for 0")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Koromon"
        effect0._alt_digi_exact = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Reveal top 4, add Greymon/Omnimon Digimon +
        #     Tai Kamiya Tamer, rest to deck bottom ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT12-059 On Play: Reveal 4, add Greymon/Omnimon + Tai Kamiya")
        effect1.set_effect_description(
            "[On Play] Reveal top 4 cards. Add 1 Digimon with [Greymon]/"
            "[Omnimon] and 1 Tamer with [Tai Kamiya] to hand. Rest to deck bottom."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner):
                return False
            return len(card.owner.library_cards) >= 1
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Pass 1: Digimon with [Greymon] or [Omnimon]
            def filter_greymon_omnimon(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return (c.contains_card_name('Greymon') or
                        c.contains_card_name('Omnimon'))

            # Pass 2: Tamer with [Tai Kamiya]
            def filter_tai_kamiya(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return c.contains_card_name('Tai Kamiya')

            game.effect_reveal_and_select_multi(
                player, 4,
                passes=[
                    (filter_greymon_omnimon, 'hand'),
                    (filter_tai_kamiya, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited +1000 DP if Greymon or Omnimon ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT12-059 Inherited: +1000 DP if Greymon/Omnimon")
        effect2.set_effect_description(
            "[All Turns] While this Digimon has [Greymon] or [Omnimon] in "
            "its name, it gets +1000 DP."
        )
        effect2.is_inherited_effect = True
        effect2._dp_modifier = 1000

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not perm.top_card:
                return False
            return (perm.top_card.contains_card_name('Greymon') or
                    perm.top_card.contains_card_name('Omnimon'))
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
