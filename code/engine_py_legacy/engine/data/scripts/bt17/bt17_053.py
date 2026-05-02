from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_053(CardScript):
    """BT17-053 Keramon | Lv.3 Rookie | Black | Unidentified

    [Opponent's Turn] When an opponent's Digimon is played or digivolves,
        if that Digimon is level 5 or higher, you may digivolve this Digimon
        into a [Infermon] from your hand without paying the cost, ignoring
        digivolution requirements.
    Inherited: [On Deletion] If this Digimon had [Unidentified] trait,
        you may play 1 [Diaboromon] Token without paying the cost.
        (Digimon/Cost 14/Lv.6/White/Mega/Unknown/Unidentified/3000 DP)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Opponent's Turn] Free digivolve into Infermon ---
        # When opponent plays/digivolves a Lv.5+ Digimon, digivolve this into
        # Infermon from hand without paying cost, ignoring requirements.
        # This is a reactive digivolution trigger that the engine handles
        # via interrupt timing. We model it as a conditional effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-053 Free digivolve into Infermon")
        effect0.set_effect_description(
            "[Opponent's Turn] When an opponent's Digimon is played or "
            "digivolves, if that Digimon is level 5 or higher, you may "
            "digivolve this Digimon into a [Infermon] from your hand "
            "without paying the cost, ignoring digivolution requirements."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            if not (card and card.owner and not card.owner.is_my_turn):
                return False
            # Check if the triggering permanent is opponent's and Lv.5+
            trigger_perm = context.get('permanent')
            if trigger_perm and trigger_perm.top_card:
                tc = trigger_perm.top_card
                if tc.owner != card.owner and tc.c_entity_base and (tc.c_entity_base.level or 0) >= 5:
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Digivolve this Digimon into Infermon from hand for free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Find an Infermon in hand
            infermon_card = None
            for hc in player.hand_cards:
                if hc.c_entity_base and 'Infermon' in (hc.c_entity_base.card_name_eng or ''):
                    infermon_card = hc
                    break
            if infermon_card:
                player.hand_cards.remove(infermon_card)
                perm.card_sources.append(infermon_card)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [On Deletion] Play Diaboromon Token ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("BT17-053 Inherited: On Deletion play Diaboromon Token")
        effect1.set_effect_description(
            "[On Deletion] If this Digimon had [Unidentified] trait, you may "
            "play 1 [Diaboromon] Token without paying the cost."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_deletion = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                # On deletion, the permanent may already be removed,
                # so we check via context
                pass
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play a Diaboromon Token if the deleted Digimon had Unidentified trait."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and game):
                return
            # Check if the deleted Digimon had [Unidentified] trait
            had_unidentified = False
            if perm and perm.top_card:
                had_unidentified = 'Unidentified' in (perm.top_card.card_traits or [])
            if not had_unidentified and perm:
                # Check all card sources
                for cs in perm.card_sources:
                    if 'Unidentified' in (cs.card_traits or []):
                        had_unidentified = True
                        break
            if had_unidentified:
                game.effect_play_token(player, 'diaboromon')
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
