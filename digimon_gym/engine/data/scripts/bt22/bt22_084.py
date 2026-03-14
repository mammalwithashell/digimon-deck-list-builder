from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_084(CardScript):
    """BT22-084 Nokia Shiramine

    [Start of Your Turn] If your memory is at 2 or less, it becomes 3.
    [Start of Your Main Phase] If you have 1 or fewer Digimon, you may play
        1 [Agumon] or [Gabumon] from your hand without paying the cost.
    [On Play] If you have 1 or fewer Digimon, you may play 1 [Agumon] or
        [Gabumon] from your hand without paying the cost.
    [All Turns] Your Digimon with [Greymon], [Garurumon], or [Omnimon] in
        their names get +1000 DP.
    Security: Play this card without paying its cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT22-084 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Shared play filter and process for Start of Main / On Play ---
        def _play_agumon_gabumon(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Check 1 or fewer Digimon condition
            digimon_count = sum(1 for p in player.battle_area if p.is_digimon)
            if digimon_count > 1:
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Agumon') or c.contains_card_name('Gabumon')

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        # --- Effect 1: [Start of Your Main Phase] Play Agumon/Gabumon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT22-084 Play 1 [Agumon] or [Gabumon]")
        effect1.set_effect_description(
            "[Start of Your Main Phase] If you have 1 or fewer Digimon, you "
            "may play 1 [Agumon] or [Gabumon] from your hand without paying "
            "the cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_play_agumon_gabumon)
        effects.append(effect1)

        # --- Effect 2: [On Play] Play Agumon/Gabumon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-084 Play 1 [Agumon] or [Gabumon]")
        effect2.set_effect_description(
            "[On Play] If you have 1 or fewer Digimon, you may play 1 "
            "[Agumon] or [Gabumon] from your hand without paying the cost."
        )
        effect2.is_optional = True
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_agumon_gabumon)
        effects.append(effect2)

        # --- Effect 3: [All Turns] DP +1000 to Greymon/Garurumon/Omnimon ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-084 Greymon/Garurumon/Omnimon DP +1000")
        effect3.set_effect_description(
            "Your Digimon with [Greymon], [Garurumon], or [Omnimon] in their "
            "names get +1000 DP."
        )
        effect3.dp_modifier = 1000
        effect3._applies_to_all_own_digimon = True

        def dp_perm_filter(perm):
            tc = perm.top_card if perm else None
            if not tc:
                return False
            return (tc.contains_card_name('Greymon') or
                    tc.contains_card_name('Garurumon') or
                    tc.contains_card_name('Omnimon'))
        effect3._dp_permanent_condition = dp_perm_filter

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # --- Effect 4: Security: Play this card ---
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-084 Security: Play this card")
        effect4.set_effect_description("Security: Play this card")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
