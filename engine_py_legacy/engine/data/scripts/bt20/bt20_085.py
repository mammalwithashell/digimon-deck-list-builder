from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_085(CardScript):
    """BT20-085 Shoto Kazama"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
        # you may play 1 [Shoto Kazama] from your hand without paying the cost.
        # Then, if you don't have a Digimon, you may play 1 level 3 Digimon card
        # with [Avian] or [Bird] in any of its traits from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT20-085 Return self to deck to play [Shoto Kazama] from hand, then play lv3 Avian/Bird from trash if no Digimon")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Shoto Kazama] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 level 3 Digimon card with [Avian] or [Bird] in any of its traits from your trash without paying the cost.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Require that this tamer is on the field
            if card and card.permanent_of_this_card() is None:
                return False
            # Require at least 1 [Shoto Kazama] in hand to make the cost worthwhile
            owner = card.owner if card else None
            if owner:
                has_shoto = any(
                    any('Shoto Kazama' in _n for _n in getattr(c, 'card_names', []))
                    for c in (owner.hand_cards or [])
                )
                if not has_shoto:
                    return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: return this Tamer to deck bottom.
            Action: play 1 [Shoto Kazama] from hand free.
            Then if no Digimon: play lv3 Avian/Bird Digimon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: return this tamer to the bottom of the deck.
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)

            # Play 1 [Shoto Kazama] from hand without paying the cost.
            def shoto_filter(c):
                return any('Shoto Kazama' in _n for _n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', shoto_filter, free=True, is_optional=True,
                prompt="Play 1 [Shoto Kazama] from your hand without paying the cost.")

            # Then: if you don't have a Digimon on the field, play lv3 Avian/Bird from trash.
            has_digimon = any(p.is_digimon for p in (player.battle_area or []))
            if not has_digimon:
                def avian_bird_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    lv = getattr(c, 'level', None)
                    if lv != 3:
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Avian' in t or 'Bird' in t for t in traits)

                game.effect_play_from_zone(
                    player, 'trash', avian_bird_filter, free=True, is_optional=True,
                    prompt="Play 1 level 3 Digimon with [Avian] or [Bird] in its traits from trash.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] By suspending this Tamer, suspend 1 of your opponent's Digimon
        # and, until the end of their turn, 1 of your Digimon with the [Vortex Warriors] trait
        # gets +2000 DP.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT20-085 Suspend self to suspend opponent Digimon; 1 Vortex Warriors Digimon +2000 DP until end of opponent's turn")
        effect1.set_effect_description("[End of Your Turn] By suspending this Tamer, suspend 1 of your opponent's Digimon and, until the end of their turn, 1 of your Digimon with the [Vortex Warriors] trait gets +2000 DP.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Cost: suspending this Tamer — it must be unsuspended and on the field.
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return False
            if tamer_perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Cost: suspend this Tamer.
            Action: suspend 1 opponent Digimon; 1 Vortex Warriors Digimon +2000 DP until end of opponent's turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: suspend this tamer.
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                tamer_perm.suspend()

            # Suspend 1 of the opponent's Digimon.
            def on_suspend_opp(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend_opp,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Suspend 1 of your opponent's Digimon.")

            # 1 of your Digimon with [Vortex Warriors] trait gets +2000 DP
            # until the end of their (opponent's) turn.
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType

            def on_select_vortex(target_perm):
                game.register_modifier(
                    ModifierType.CHANGE_DP, target_perm,
                    value_fn=lambda: 2000,
                    expiry='end_of_opponent_turn')

            game.effect_select_own_permanent(
                player, on_select_vortex,
                filter_fn=lambda p: p.is_digimon and any(
                    'Vortex Warriors' in t
                    for t in (getattr(p.top_card, 'card_traits', []) or [])
                    if p.top_card
                ),
                is_optional=False,
                prompt="Choose 1 of your Digimon with the [Vortex Warriors] trait to get +2000 DP.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-085 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
