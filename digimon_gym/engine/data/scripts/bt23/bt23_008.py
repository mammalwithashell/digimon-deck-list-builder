from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_008(CardScript):
    """BT23-008 Greymon | Lv.4

    Alt digivolve: from Lv.3 w/ [Agumon] name or CS trait for 2.
    Raid.
    [Main][Once Per Turn] By placing this Digimon's top stacked card as its
        bottom digivolution card, you may play 1 [Gabumon] or
        [Nokia Shiramine] from your hand with the play cost reduced by 2.
    Inherited: [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.3 w/ Agumon or CS trait for 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Raid ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-008 Raid")
        effect1.set_effect_description("Raid")
        effect1.is_on_attack = True
        effect1._is_raid = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Main][OPT] Stack-shift + play Gabumon/Nokia for -2 cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name(
            "BT23-008 Stack-shift, play [Gabumon]/[Nokia Shiramine] for -2"
        )
        effect2.set_effect_description(
            "[Main] [Once Per Turn] By placing this Digimon's top stacked "
            "card as its bottom digivolution card, you may play 1 [Gabumon] "
            "or [Nokia Shiramine] from your hand with the play cost reduced by 2."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT23_008_Main")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have at least 1 digivolution card (stack_cards)
            perm = card.permanent_of_this_card()
            if perm and len(perm.card_sources) < 2:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Stack-shift top to bottom, then play Gabumon/Nokia for -2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm or len(perm.card_sources) < 2:
                return

            # Cost: move top stacked card to bottom of digivolution stack
            # top_card is the topmost; the "top stacked card" is the one
            # just below top_card in the stack (the highest digivolution card)
            # Per C#: topCard = perm.TopCard, then AddDigivolutionCardsBottom
            # In the engine, card_sources is bottom-to-top, so card_sources[-1]
            # is top_card. The "top stacked card" means the top card of the
            # digivolution stack = card_sources[-1] = top_card itself per C# ref.
            # C# does: TopCard -> AddDigivolutionCardsBottom({topCard})
            # which moves the current top card to the bottom of the stack.
            top = perm.card_sources[-1]
            perm.card_sources.remove(top)
            perm.card_sources.insert(0, top)
            # Update top_card reference
            if hasattr(perm, '_refresh_top_card'):
                perm._refresh_top_card()

            # Reward: play 1 [Gabumon] or [Nokia Shiramine] with -2 cost
            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                is_gabumon = getattr(c, 'is_digimon', False) and 'Gabumon' in names
                is_nokia = getattr(c, 'is_tamer', False) and 'Nokia Shiramine' in names
                return is_gabumon or is_nokia

            game.effect_play_from_zone(
                player, 'hand', play_filter,
                free=False, manual_reduction=2, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [Your Turn] +2000 DP ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT23-008 DP modifier")
        effect3.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 2000

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
