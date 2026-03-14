from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_069(CardScript):
    """BT20-069 Punkmon | Lv.4

    Alt digivolve: Lv.3 with [Evil] trait for cost 2
    [On Play] [When Digivolving] Trash 1 card in your hand. Then, until
        the end of your opponent's turn, 1 of your Digimon gains <Blocker>
        and <Retaliation>.
    Inherited: [Your Turn] This Digimon gets +2000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.3 [Evil] trait for cost 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-069 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_trait = "Evil"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and
                    any('Evil' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or []))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared On Play / When Digivolving logic ---
        def _shared_process(ctx: Dict[str, Any]):
            """Trash 1 card from hand. Then, player selects 1 of their own
            Digimon to gain Blocker and Retaliation until end of opponent's turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Trash 1 card from hand (mandatory)
            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

                # Step 2: Select 1 of your Digimon to gain Blocker + Retaliation
                def perm_filter(p):
                    return p.is_digimon

                def on_select_perm(target_perm):
                    if target_perm is None:
                        return
                    target_perm.grant_keyword('_is_blocker')
                    target_perm.grant_keyword('_is_retaliation')

                if any(perm_filter(p) for p in player.battle_area):
                    game.effect_select_own_permanent(
                        player, on_select_perm,
                        filter_fn=perm_filter,
                        is_optional=False,
                        prompt="Select 1 of your Digimon to gain Blocker and Retaliation.")

            if player.hand_cards:
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False)

        # --- Effect 1: [On Play] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT20-069 Trash 1 card, grant Blocker+Retaliation")
        effect1.set_effect_description(
            "[On Play] Trash 1 card in your hand. Then, until the end of your "
            "opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_process)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-069 Trash 1 card, grant Blocker+Retaliation")
        effect2.set_effect_description(
            "[When Digivolving] Trash 1 card in your hand. Then, until the end of "
            "your opponent's turn, 1 of your Digimon gains <Blocker> and <Retaliation>.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # --- Effect 3: Inherited [Your Turn] +2000 DP ---
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-069 DP modifier")
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
